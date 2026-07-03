use crate::board_ir::{AnalogModelFile, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::path::Path;
use std::process::Command;

use super::analog_util::{executable_on_path, file_sha256_hex, push_artifact};

const OPENVAF_BUILD_ENV: &str = "CIRCUITCI_RUN_OPENVAF_BUILDS";

pub(super) fn validate_model_compiler_provenance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    artifacts: &mut Vec<String>,
) -> Option<Finding> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before model provenance validation");
    for model_file in &analog.model_files {
        if !model_file_requires_compiler_provenance(model_file) {
            continue;
        }
        if model_file.artifact_format.as_deref() != Some("osdi_shared_object") {
            return Some(model_compiler_metadata_missing(
                scenario,
                model_file,
                "artifact_format",
                "OpenVAF model provenance requires artifact_format: osdi_shared_object.",
            ));
        }
        if model_file.sha256.as_deref().is_none_or(str::is_empty) {
            return Some(model_compiler_metadata_missing(
                scenario,
                model_file,
                "sha256",
                "OpenVAF/OSDI model artifacts require a SHA-256 pin for the compiled shared object.",
            ));
        }
        let Some(source_path) = nonempty(model_file.source_path.as_deref()) else {
            return Some(model_compiler_metadata_missing(
                scenario,
                model_file,
                "source_path",
                "OpenVAF/OSDI model artifacts require the Verilog-A source path.",
            ));
        };
        let Some(source_sha256) = nonempty(model_file.source_sha256.as_deref()) else {
            return Some(model_compiler_metadata_missing(
                scenario,
                model_file,
                "source_sha256",
                "OpenVAF/OSDI model artifacts require a SHA-256 pin for the Verilog-A source.",
            ));
        };
        if model_file.compiler.as_deref() != Some("openvaf") {
            return Some(model_compiler_metadata_missing(
                scenario,
                model_file,
                "compiler",
                "OpenVAF/OSDI model artifacts require compiler: openvaf.",
            ));
        }
        if nonempty(model_file.compiler_version.as_deref()).is_none() {
            return Some(model_compiler_metadata_missing(
                scenario,
                model_file,
                "compiler_version",
                "OpenVAF/OSDI model artifacts require compiler_version provenance.",
            ));
        }
        let Some(compiler_command) = nonempty(model_file.compiler_command.as_deref()) else {
            return Some(model_compiler_metadata_missing(
                scenario,
                model_file,
                "compiler_command",
                "OpenVAF/OSDI model artifacts require the reproducible compiler command.",
            ));
        };
        if let Some(message) = validate_openvaf_compiler_command(model_file, compiler_command) {
            let mut finding = Finding::critical(
                "ANALOG_MODEL_COMPILER_COMMAND_MISMATCH",
                &scenario.name,
                message,
            );
            insert_model_compiler_plan(&mut finding, model_file, source_path, compiler_command);
            return Some(finding);
        }

        let source = bound.project.source_dir.join(source_path);
        if !source.is_file() {
            let mut finding = Finding::critical(
                "ANALOG_MODEL_SOURCE_UNAVAILABLE",
                &scenario.name,
                format!(
                    "Verilog-A source file {} is required for OpenVAF/OSDI model provenance.",
                    source.display()
                ),
            );
            finding.limit.insert(
                "required_artifact".to_string(),
                json!("verilog_a_source_file"),
            );
            finding
                .limit
                .insert("model_file".to_string(), json!(model_file.path));
            finding
                .limit
                .insert("source_path".to_string(), json!(source_path));
            finding.suggested_fixes.push(
                "Add the Verilog-A source artifact or remove OpenVAF/OSDI provenance metadata until the source can be audited."
                    .to_string(),
            );
            return Some(finding);
        }
        match file_sha256_hex(&source) {
            Ok(actual) if actual.eq_ignore_ascii_case(source_sha256) => {}
            Ok(actual) => {
                let mut finding = Finding::critical(
                    "ANALOG_MODEL_SOURCE_HASH_MISMATCH",
                    &scenario.name,
                    format!(
                        "Verilog-A source file {} does not match the declared SHA-256.",
                        source.display()
                    ),
                );
                finding.measured.insert("sha256".to_string(), json!(actual));
                finding
                    .limit
                    .insert("expected_sha256".to_string(), json!(source_sha256));
                finding
                    .limit
                    .insert("model_file".to_string(), json!(model_file.path));
                finding.suggested_fixes.push(
                    "Rebuild the OSDI artifact from the declared Verilog-A source or update the provenance pins together."
                        .to_string(),
                );
                return Some(finding);
            }
            Err(message) => {
                let mut finding =
                    Finding::critical("ANALOG_MODEL_SOURCE_UNAVAILABLE", &scenario.name, message);
                finding.limit.insert(
                    "required_artifact".to_string(),
                    json!("verilog_a_source_file"),
                );
                return Some(finding);
            }
        }
        push_artifact(artifacts, &source);

        let artifact = bound.project.source_dir.join(&model_file.path);
        if !artifact.is_file() && openvaf_builds_enabled() {
            match run_openvaf_build(bound, scenario, model_file, source_path, compiler_command) {
                Ok(()) => {}
                Err(finding) => return Some(*finding),
            }
        }
        if !artifact.is_file() {
            let mut finding = Finding::critical(
                "ANALOG_MODEL_COMPILER_ARTIFACT_UNAVAILABLE",
                &scenario.name,
                format!(
                    "Compiled OSDI artifact {} is missing for OpenVAF model provenance.",
                    artifact.display()
                ),
            );
            insert_model_compiler_plan(&mut finding, model_file, source_path, compiler_command);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("osdi_shared_object"));
            finding.suggested_fixes.push(
                "Run the declared OpenVAF compiler command from the project directory, then commit the compiled OSDI artifact and matching SHA-256 pin."
                    .to_string(),
            );
            return Some(finding);
        }
        let expected_artifact_sha = model_file
            .sha256
            .as_deref()
            .expect("model compiler provenance requires sha256");
        match file_sha256_hex(&artifact) {
            Ok(actual) if actual.eq_ignore_ascii_case(expected_artifact_sha) => {}
            Ok(actual) => {
                if openvaf_builds_enabled() {
                    if let Err(finding) = run_openvaf_build(
                        bound,
                        scenario,
                        model_file,
                        source_path,
                        compiler_command,
                    ) {
                        return Some(*finding);
                    }
                    match file_sha256_hex(&artifact) {
                        Ok(rebuilt) if rebuilt.eq_ignore_ascii_case(expected_artifact_sha) => {}
                        Ok(rebuilt) => {
                            return Some(artifact_hash_mismatch_finding(
                                scenario,
                                model_file,
                                source_path,
                                compiler_command,
                                &artifact,
                                expected_artifact_sha,
                                &rebuilt,
                            ));
                        }
                        Err(message) => {
                            let mut finding = Finding::critical(
                                "ANALOG_MODEL_COMPILER_ARTIFACT_UNAVAILABLE",
                                &scenario.name,
                                message,
                            );
                            insert_model_compiler_plan(
                                &mut finding,
                                model_file,
                                source_path,
                                compiler_command,
                            );
                            finding.limit.insert(
                                "required_artifact".to_string(),
                                json!("osdi_shared_object"),
                            );
                            return Some(finding);
                        }
                    }
                } else {
                    return Some(artifact_hash_mismatch_finding(
                        scenario,
                        model_file,
                        source_path,
                        compiler_command,
                        &artifact,
                        expected_artifact_sha,
                        &actual,
                    ));
                }
            }
            Err(message) => {
                let mut finding = Finding::critical(
                    "ANALOG_MODEL_COMPILER_ARTIFACT_UNAVAILABLE",
                    &scenario.name,
                    message,
                );
                insert_model_compiler_plan(&mut finding, model_file, source_path, compiler_command);
                finding
                    .limit
                    .insert("required_artifact".to_string(), json!("osdi_shared_object"));
                return Some(finding);
            }
        }
        push_artifact(artifacts, &artifact);
    }
    None
}

fn run_openvaf_build(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    model_file: &AnalogModelFile,
    source_path: &str,
    compiler_command: &str,
) -> Result<(), Box<Finding>> {
    let tokens = split_compiler_command(compiler_command).map_err(|message| {
        let mut finding = Finding::critical(
            "ANALOG_MODEL_COMPILER_COMMAND_MISMATCH",
            &scenario.name,
            message,
        );
        insert_model_compiler_plan(&mut finding, model_file, source_path, compiler_command);
        Box::new(finding)
    })?;
    let Some((program, args)) = tokens.split_first() else {
        let mut finding = Finding::critical(
            "ANALOG_MODEL_COMPILER_COMMAND_MISMATCH",
            &scenario.name,
            "OpenVAF compiler_command is empty.",
        );
        insert_model_compiler_plan(&mut finding, model_file, source_path, compiler_command);
        return Err(Box::new(finding));
    };
    let output = Command::new(program)
        .args(args)
        .current_dir(&bound.project.source_dir)
        .output()
        .map_err(|error| {
            let mut finding = Finding::critical(
                "ANALOG_MODEL_COMPILER_BUILD_FAILED",
                &scenario.name,
                format!("Failed to run OpenVAF compiler command: {error}"),
            );
            insert_model_compiler_plan(&mut finding, model_file, source_path, compiler_command);
            Box::new(finding)
        })?;
    if !output.status.success() {
        let mut finding = Finding::critical(
            "ANALOG_MODEL_COMPILER_BUILD_FAILED",
            &scenario.name,
            format!(
                "OpenVAF compiler command exited with status {}.",
                output.status
            ),
        );
        insert_model_compiler_plan(&mut finding, model_file, source_path, compiler_command);
        finding
            .measured
            .insert("stdout".to_string(), json!(lossy_prefix(&output.stdout)));
        finding
            .measured
            .insert("stderr".to_string(), json!(lossy_prefix(&output.stderr)));
        return Err(Box::new(finding));
    }
    Ok(())
}

fn artifact_hash_mismatch_finding(
    scenario: &Scenario,
    model_file: &AnalogModelFile,
    source_path: &str,
    compiler_command: &str,
    artifact: &Path,
    expected_artifact_sha: &str,
    actual: &str,
) -> Finding {
    let mut finding = Finding::critical(
        "ANALOG_MODEL_COMPILER_ARTIFACT_HASH_MISMATCH",
        &scenario.name,
        format!(
            "Compiled OSDI artifact {} does not match the declared SHA-256.",
            artifact.display()
        ),
    );
    finding.measured.insert("sha256".to_string(), json!(actual));
    finding
        .limit
        .insert("expected_sha256".to_string(), json!(expected_artifact_sha));
    insert_model_compiler_plan(&mut finding, model_file, source_path, compiler_command);
    finding.suggested_fixes.push(
        "Re-run the declared OpenVAF compiler command from the pinned Verilog-A source, or update the compiled artifact and SHA-256 pin together."
            .to_string(),
    );
    finding
}

fn validate_openvaf_compiler_command(
    model_file: &AnalogModelFile,
    compiler_command: &str,
) -> Option<String> {
    let tokens = match split_compiler_command(compiler_command) {
        Ok(tokens) => tokens,
        Err(message) => return Some(message),
    };
    let binary = tokens.first().map(String::as_str).unwrap_or_default();
    let binary_name = Path::new(binary)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(binary);
    if binary_name != "openvaf" {
        return Some(
            "OpenVAF/OSDI compiler_command must invoke the openvaf executable.".to_string(),
        );
    }
    let source_path = model_file
        .source_path
        .as_deref()
        .expect("model compiler provenance requires source_path");
    if !compiler_command.contains(source_path) {
        return Some(format!(
            "OpenVAF compiler_command must reference Verilog-A source {source_path}."
        ));
    }
    if !compiler_command.contains(&model_file.path) {
        return Some(format!(
            "OpenVAF compiler_command must write declared OSDI artifact {}.",
            model_file.path
        ));
    }
    None
}

fn split_compiler_command(command: &str) -> Result<Vec<String>, String> {
    if command.contains(['|', ';', '&', '>', '<', '`', '$', '\n', '\r']) {
        return Err(
            "OpenVAF compiler_command may not contain shell metacharacters; CircuitCI executes openvaf directly, not through a shell."
                .to_string(),
        );
    }
    let tokens: Vec<_> = command
        .split_whitespace()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if tokens.is_empty() {
        return Err("OpenVAF compiler_command is empty.".to_string());
    }
    Ok(tokens)
}

fn openvaf_builds_enabled() -> bool {
    std::env::var(OPENVAF_BUILD_ENV)
        .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn lossy_prefix(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.chars().take(2048).collect()
}

fn insert_model_compiler_plan(
    finding: &mut Finding,
    model_file: &AnalogModelFile,
    source_path: &str,
    compiler_command: &str,
) {
    finding
        .measured
        .insert("model_file".to_string(), json!(model_file.path));
    finding
        .measured
        .insert("source_path".to_string(), json!(source_path));
    finding
        .measured
        .insert("compiler".to_string(), json!("openvaf"));
    finding.measured.insert(
        "compiler_version".to_string(),
        json!(model_file.compiler_version),
    );
    finding
        .measured
        .insert("compiler_command".to_string(), json!(compiler_command));
    finding.measured.insert(
        "compiler_available_on_path".to_string(),
        json!(executable_on_path("openvaf")),
    );
    finding.limit.insert(
        "required_build_step".to_string(),
        json!("openvaf_compile_osdi_shared_object"),
    );
    finding
        .limit
        .insert("output_path".to_string(), json!(model_file.path));
}

fn model_file_requires_compiler_provenance(model_file: &AnalogModelFile) -> bool {
    model_file.artifact_format.as_deref() == Some("osdi_shared_object")
        || model_file.compiler.is_some()
        || model_file.compiler_version.is_some()
        || model_file.compiler_command.is_some()
        || model_file.source_path.is_some()
        || model_file.source_sha256.is_some()
}

fn model_compiler_metadata_missing(
    scenario: &Scenario,
    model_file: &AnalogModelFile,
    field: &str,
    message: &str,
) -> Finding {
    let mut finding = Finding::critical(
        "ANALOG_MODEL_COMPILER_PROVENANCE_MISSING",
        &scenario.name,
        message,
    );
    finding
        .measured
        .insert("model_file".to_string(), json!(model_file.path));
    finding
        .limit
        .insert("required_field".to_string(), json!(field));
    finding.limit.insert(
        "required_artifact".to_string(),
        json!("openvaf_osdi_model_provenance"),
    );
    finding.suggested_fixes.push(
        "Declare the compiled OSDI artifact, Verilog-A source, SHA-256 pins, OpenVAF version, and compiler command before using this compact model in simulation sign-off."
            .to_string(),
    );
    finding
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
