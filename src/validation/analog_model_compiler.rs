use crate::board_ir::{AnalogBackend, AnalogModelFile, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use super::analog_util::{executable_on_path, file_sha256_hex, push_artifact};

const OPENVAF_BUILD_ENV: &str = "CIRCUITCI_RUN_OPENVAF_BUILDS";

#[derive(Clone)]
struct ModelCompilerManifestRecord {
    scenario: String,
    model_file: String,
    source_path: String,
    source_sha256_declared: String,
    source_sha256_actual: String,
    artifact_sha256_declared: String,
    artifact_sha256_actual: String,
    compiler: String,
    compiler_version: Option<String>,
    compiler_command: String,
    compiler_available_on_path: bool,
    build_env_enabled: bool,
    rebuild_mode: &'static str,
}

static MODEL_COMPILER_MANIFEST_RECORDS: OnceLock<Mutex<Vec<ModelCompilerManifestRecord>>> =
    OnceLock::new();

pub(super) fn validate_model_compiler_provenance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    artifacts: &mut Vec<String>,
) -> Option<Finding> {
    clear_model_compiler_manifest_records(&scenario.name);
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before model provenance validation");
    for model_file in &analog.model_files {
        if !model_file_requires_compiler_provenance(model_file) {
            continue;
        }
        if model_file.artifact_format.as_deref() == Some("xyce_adms_plugin") {
            return Some(validate_xyce_adms_plugin_contract(
                bound, scenario, model_file, artifacts,
            ));
        }
        if model_file.artifact_format.as_deref() != Some("osdi_shared_object") {
            return Some(model_compiler_metadata_missing(
                scenario,
                model_file,
                "artifact_format",
                "OpenVAF model provenance requires artifact_format: osdi_shared_object.",
            ));
        }
        if matches!(
            analog.backend,
            AnalogBackend::Xyce | AnalogBackend::EmbeddedNgspice
        ) {
            return Some(osdi_backend_unsupported(scenario, model_file));
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
        let source_sha256_actual = match file_sha256_hex(&source) {
            Ok(actual) if actual.eq_ignore_ascii_case(source_sha256) => actual,
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
        };
        push_artifact(artifacts, &source);

        let artifact = bound.project.source_dir.join(&model_file.path);
        let mut rebuild_mode = "prebuilt_verified";
        if !artifact.is_file() && openvaf_builds_enabled() {
            match run_openvaf_build(bound, scenario, model_file, source_path, compiler_command) {
                Ok(()) => rebuild_mode = "rebuilt_missing_artifact",
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
            Ok(actual) if actual.eq_ignore_ascii_case(expected_artifact_sha) => {
                record_model_compiler_manifest(ModelCompilerManifestRecord {
                    scenario: scenario.name.clone(),
                    model_file: model_file.path.clone(),
                    source_path: source_path.to_string(),
                    source_sha256_declared: source_sha256.to_string(),
                    source_sha256_actual: source_sha256_actual.clone(),
                    artifact_sha256_declared: expected_artifact_sha.to_string(),
                    artifact_sha256_actual: actual,
                    compiler: "openvaf".to_string(),
                    compiler_version: model_file.compiler_version.clone(),
                    compiler_command: compiler_command.to_string(),
                    compiler_available_on_path: executable_on_path("openvaf"),
                    build_env_enabled: openvaf_builds_enabled(),
                    rebuild_mode,
                });
            }
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
                    rebuild_mode = "rebuilt_hash_stale_artifact";
                    match file_sha256_hex(&artifact) {
                        Ok(rebuilt) if rebuilt.eq_ignore_ascii_case(expected_artifact_sha) => {
                            record_model_compiler_manifest(ModelCompilerManifestRecord {
                                scenario: scenario.name.clone(),
                                model_file: model_file.path.clone(),
                                source_path: source_path.to_string(),
                                source_sha256_declared: source_sha256.to_string(),
                                source_sha256_actual: source_sha256_actual.clone(),
                                artifact_sha256_declared: expected_artifact_sha.to_string(),
                                artifact_sha256_actual: rebuilt,
                                compiler: "openvaf".to_string(),
                                compiler_version: model_file.compiler_version.clone(),
                                compiler_command: compiler_command.to_string(),
                                compiler_available_on_path: executable_on_path("openvaf"),
                                build_env_enabled: openvaf_builds_enabled(),
                                rebuild_mode,
                            });
                        }
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

pub(super) fn solver_manifest_model_file_provenance(scenario: &Scenario) -> Vec<serde_json::Value> {
    model_compiler_manifest_records()
        .lock()
        .expect("model compiler manifest records mutex is poisoned")
        .iter()
        .filter(|record| record.scenario == scenario.name)
        .map(|record| {
            json!({
                "model_file": &record.model_file,
                "artifact_format": "osdi_shared_object",
                "source_path": &record.source_path,
                "source_sha256_declared": &record.source_sha256_declared,
                "source_sha256_actual": &record.source_sha256_actual,
                "artifact_sha256_declared": &record.artifact_sha256_declared,
                "artifact_sha256_actual": &record.artifact_sha256_actual,
                "compiler": &record.compiler,
                "compiler_version": &record.compiler_version,
                "compiler_command": &record.compiler_command,
                "compiler_available_on_path": record.compiler_available_on_path,
                "build_env_enabled": record.build_env_enabled,
                "rebuild_mode": record.rebuild_mode,
                "produced_by_circuitci": record.rebuild_mode != "prebuilt_verified",
            })
        })
        .collect()
}

fn record_model_compiler_manifest(record: ModelCompilerManifestRecord) {
    model_compiler_manifest_records()
        .lock()
        .expect("model compiler manifest records mutex is poisoned")
        .push(record);
}

fn clear_model_compiler_manifest_records(scenario: &str) {
    model_compiler_manifest_records()
        .lock()
        .expect("model compiler manifest records mutex is poisoned")
        .retain(|record| record.scenario != scenario);
}

fn model_compiler_manifest_records() -> &'static Mutex<Vec<ModelCompilerManifestRecord>> {
    MODEL_COMPILER_MANIFEST_RECORDS.get_or_init(|| Mutex::new(Vec::new()))
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

fn validate_xyce_adms_plugin_contract(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    model_file: &AnalogModelFile,
    artifacts: &mut Vec<String>,
) -> Finding {
    if model_file.sha256.as_deref().is_none_or(str::is_empty) {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "sha256",
            "Xyce/ADMS plugin artifacts require a SHA-256 pin for the compiled plugin library.",
        );
    }
    let Some(source_path) = nonempty(model_file.source_path.as_deref()) else {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "source_path",
            "Xyce/ADMS plugin artifacts require the Verilog-A source path.",
        );
    };
    let Some(source_sha256) = nonempty(model_file.source_sha256.as_deref()) else {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "source_sha256",
            "Xyce/ADMS plugin artifacts require a SHA-256 pin for the Verilog-A source.",
        );
    };
    if model_file.compiler.as_deref() != Some("xyce_adms") {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "compiler",
            "Xyce/ADMS plugin artifacts require compiler: xyce_adms.",
        );
    }
    if nonempty(model_file.compiler_version.as_deref()).is_none() {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "compiler_version",
            "Xyce/ADMS plugin artifacts require compiler_version provenance.",
        );
    }
    let Some(compiler_command) = nonempty(model_file.compiler_command.as_deref()) else {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "compiler_command",
            "Xyce/ADMS plugin artifacts require the reproducible compiler command.",
        );
    };
    let Some(plugin_load_command) = nonempty(model_file.plugin_load_command.as_deref()) else {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "plugin_load_command",
            "Xyce/ADMS plugin artifacts require the Xyce -plugin load command.",
        );
    };
    if nonempty(model_file.xyce_version.as_deref()).is_none() {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "xyce_version",
            "Xyce/ADMS plugin artifacts require the qualified Xyce version.",
        );
    }
    if nonempty(model_file.xyce_adms_template_revision.as_deref()).is_none() {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "xyce_adms_template_revision",
            "Xyce/ADMS plugin artifacts require the ADMS template revision.",
        );
    }
    for required_option in ["--enable-shared", "--enable-xyce-shareable"] {
        if !model_file
            .xyce_configure_options
            .iter()
            .any(|option| option == required_option)
        {
            let mut finding = xyce_adms_metadata_missing(
                scenario,
                model_file,
                "xyce_configure_options",
                &format!(
                    "Xyce/ADMS plugin artifacts require Xyce configure option {required_option}."
                ),
            );
            finding.limit.insert(
                "required_configure_option".to_string(),
                json!(required_option),
            );
            return finding;
        }
    }
    let Some(conformance_artifact) = nonempty(model_file.conformance_artifact.as_deref()) else {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "conformance_artifact",
            "Xyce/ADMS plugin artifacts require a retained conformance artifact.",
        );
    };
    let Some(conformance_sha256) = nonempty(model_file.conformance_sha256.as_deref()) else {
        return xyce_adms_metadata_missing(
            scenario,
            model_file,
            "conformance_sha256",
            "Xyce/ADMS plugin artifacts require a SHA-256 pin for the retained conformance artifact.",
        );
    };
    if let Some(message) = validate_xyce_adms_compiler_command(model_file, compiler_command) {
        let mut finding = Finding::critical(
            "ANALOG_MODEL_COMPILER_COMMAND_MISMATCH",
            &scenario.name,
            message,
        );
        insert_xyce_adms_plan(&mut finding, model_file, source_path, compiler_command);
        return finding;
    }
    if let Some(message) = validate_xyce_plugin_load_command(model_file, plugin_load_command) {
        let mut finding = Finding::critical(
            "ANALOG_MODEL_COMPILER_COMMAND_MISMATCH",
            &scenario.name,
            message,
        );
        insert_xyce_adms_plan(&mut finding, model_file, source_path, compiler_command);
        finding.measured.insert(
            "plugin_load_command".to_string(),
            json!(plugin_load_command),
        );
        return finding;
    }

    let source = bound.project.source_dir.join(source_path);
    if let Some(finding) = validate_pinned_file(
        scenario,
        model_file,
        &source,
        source_sha256,
        "verilog_a_source_file",
        "ANALOG_MODEL_SOURCE_UNAVAILABLE",
        "ANALOG_MODEL_SOURCE_HASH_MISMATCH",
    ) {
        return finding;
    }
    push_artifact(artifacts, &source);

    let plugin = bound.project.source_dir.join(&model_file.path);
    if let Some(finding) = validate_pinned_file(
        scenario,
        model_file,
        &plugin,
        model_file
            .sha256
            .as_deref()
            .expect("Xyce/ADMS contract requires sha256"),
        "xyce_adms_plugin",
        "ANALOG_MODEL_COMPILER_ARTIFACT_UNAVAILABLE",
        "ANALOG_MODEL_COMPILER_ARTIFACT_HASH_MISMATCH",
    ) {
        return finding;
    }
    push_artifact(artifacts, &plugin);

    let conformance = bound.project.source_dir.join(conformance_artifact);
    if let Some(mut finding) = validate_pinned_file(
        scenario,
        model_file,
        &conformance,
        conformance_sha256,
        "xyce_adms_conformance_artifact",
        "ANALOG_MODEL_COMPILER_ARTIFACT_UNAVAILABLE",
        "ANALOG_MODEL_COMPILER_ARTIFACT_HASH_MISMATCH",
    ) {
        finding.limit.insert(
            "conformance_artifact".to_string(),
            json!(conformance_artifact),
        );
        return finding;
    }
    push_artifact(artifacts, &conformance);

    xyce_adms_plugin_unsupported(scenario, model_file, source_path, compiler_command)
}

fn validate_pinned_file(
    scenario: &Scenario,
    model_file: &AnalogModelFile,
    path: &Path,
    expected_sha256: &str,
    required_artifact: &str,
    unavailable_id: &str,
    mismatch_id: &str,
) -> Option<Finding> {
    if !path.is_file() {
        let mut finding = Finding::critical(
            unavailable_id,
            &scenario.name,
            format!("Required model artifact {} is missing.", path.display()),
        );
        finding
            .limit
            .insert("required_artifact".to_string(), json!(required_artifact));
        finding
            .limit
            .insert("model_file".to_string(), json!(model_file.path));
        return Some(finding);
    }
    match file_sha256_hex(path) {
        Ok(actual) if actual.eq_ignore_ascii_case(expected_sha256) => None,
        Ok(actual) => {
            let mut finding = Finding::critical(
                mismatch_id,
                &scenario.name,
                format!(
                    "Model artifact {} does not match the declared SHA-256.",
                    path.display()
                ),
            );
            finding.measured.insert("sha256".to_string(), json!(actual));
            finding
                .limit
                .insert("expected_sha256".to_string(), json!(expected_sha256));
            finding
                .limit
                .insert("model_file".to_string(), json!(model_file.path));
            Some(finding)
        }
        Err(message) => {
            let mut finding = Finding::critical(unavailable_id, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!(required_artifact));
            finding
                .limit
                .insert("model_file".to_string(), json!(model_file.path));
            Some(finding)
        }
    }
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

fn validate_xyce_adms_compiler_command(
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
    if binary_name != "buildxyceplugin" {
        return Some(
            "Xyce/ADMS compiler_command must invoke the buildxyceplugin executable.".to_string(),
        );
    }
    let source_path = model_file
        .source_path
        .as_deref()
        .expect("Xyce/ADMS contract requires source_path");
    if !compiler_command.contains(source_path) {
        return Some(format!(
            "Xyce/ADMS compiler_command must reference Verilog-A source {source_path}."
        ));
    }
    if !xyce_build_command_references_plugin_output(model_file, &tokens, compiler_command) {
        return Some(format!(
            "Xyce/ADMS compiler_command must write declared plugin artifact {}.",
            model_file.path
        ));
    }
    None
}

fn xyce_build_command_references_plugin_output(
    model_file: &AnalogModelFile,
    tokens: &[String],
    compiler_command: &str,
) -> bool {
    if compiler_command.contains(&model_file.path) {
        return true;
    }
    let Some(plugin_stem) = Path::new(&model_file.path)
        .file_stem()
        .and_then(|stem| stem.to_str())
    else {
        return false;
    };
    tokens.windows(2).any(|pair| {
        pair[0] == "-o" && (pair[1] == plugin_stem || format!("{}.so", pair[1]) == model_file.path)
    })
}

fn validate_xyce_plugin_load_command(
    model_file: &AnalogModelFile,
    plugin_load_command: &str,
) -> Option<String> {
    if let Err(message) = split_compiler_command(plugin_load_command) {
        return Some(message.replace("compiler_command", "plugin_load_command"));
    }
    if !plugin_load_command
        .split_whitespace()
        .any(|token| token == "-plugin")
    {
        return Some("Xyce/ADMS plugin_load_command must include -plugin.".to_string());
    }
    if !plugin_load_command.contains(&model_file.path) {
        return Some(format!(
            "Xyce/ADMS plugin_load_command must reference declared plugin artifact {}.",
            model_file.path
        ));
    }
    None
}

fn split_compiler_command(command: &str) -> Result<Vec<String>, String> {
    if command.contains(['|', ';', '&', '>', '<', '`', '$', '\n', '\r']) {
        return Err(
            "Model compiler_command may not contain shell metacharacters; CircuitCI executes compiler commands directly, not through a shell."
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
        return Err("Model compiler_command is empty.".to_string());
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

fn insert_xyce_adms_plan(
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
        .insert("artifact_format".to_string(), json!("xyce_adms_plugin"));
    finding
        .measured
        .insert("source_path".to_string(), json!(source_path));
    finding
        .measured
        .insert("compiler".to_string(), json!("xyce_adms"));
    finding.measured.insert(
        "compiler_version".to_string(),
        json!(model_file.compiler_version),
    );
    finding
        .measured
        .insert("compiler_command".to_string(), json!(compiler_command));
    finding.measured.insert(
        "plugin_load_command".to_string(),
        json!(model_file.plugin_load_command),
    );
    finding
        .measured
        .insert("xyce_version".to_string(), json!(model_file.xyce_version));
    finding.measured.insert(
        "xyce_adms_template_revision".to_string(),
        json!(model_file.xyce_adms_template_revision),
    );
    finding.measured.insert(
        "xyce_configure_options".to_string(),
        json!(model_file.xyce_configure_options),
    );
    finding.measured.insert(
        "conformance_artifact".to_string(),
        json!(model_file.conformance_artifact),
    );
    finding.measured.insert(
        "research_evidence".to_string(),
        json!("docs/research/circuit_simulation_full_featured/xyce_openvaf_osdi_compatibility.md"),
    );
    finding.limit.insert(
        "required_build_step".to_string(),
        json!("xyce_adms_buildxyceplugin"),
    );
    finding.limit.insert(
        "required_backend_adapter".to_string(),
        json!("xyce_adms_plugin_loader"),
    );
    finding.limit.insert(
        "required_conformance".to_string(),
        json!("real_xyce_plugin_load"),
    );
    finding
        .limit
        .insert("output_path".to_string(), json!(model_file.path));
}

fn model_file_requires_compiler_provenance(model_file: &AnalogModelFile) -> bool {
    model_file.artifact_format.as_deref() == Some("osdi_shared_object")
        || model_file.artifact_format.as_deref() == Some("xyce_adms_plugin")
        || model_file.compiler.is_some()
        || model_file.compiler_version.is_some()
        || model_file.compiler_command.is_some()
        || model_file.source_path.is_some()
        || model_file.source_sha256.is_some()
        || model_file.plugin_load_command.is_some()
        || model_file.xyce_version.is_some()
        || model_file.xyce_adms_template_revision.is_some()
        || !model_file.xyce_configure_options.is_empty()
        || model_file.conformance_artifact.is_some()
        || model_file.conformance_sha256.is_some()
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

fn xyce_adms_metadata_missing(
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
        .measured
        .insert("artifact_format".to_string(), json!("xyce_adms_plugin"));
    finding
        .limit
        .insert("required_field".to_string(), json!(field));
    finding.limit.insert(
        "required_artifact".to_string(),
        json!("xyce_adms_plugin_provenance"),
    );
    finding.suggested_fixes.push(
        "Declare the Xyce/ADMS plugin artifact, Verilog-A source, SHA-256 pins, Xyce build options, plugin load command, and retained conformance artifact before using this compact model in Xyce sign-off."
            .to_string(),
    );
    finding
}

fn xyce_adms_plugin_unsupported(
    scenario: &Scenario,
    model_file: &AnalogModelFile,
    source_path: &str,
    compiler_command: &str,
) -> Finding {
    let mut finding = Finding::critical(
        "ANALOG_MODEL_COMPILER_XYCE_PLUGIN_UNSUPPORTED",
        &scenario.name,
        format!(
            "Xyce/ADMS plugin artifact {} has pinned provenance, but CircuitCI does not yet execute Xyce plugin model loading.",
            model_file.path
        ),
    );
    insert_xyce_adms_plan(&mut finding, model_file, source_path, compiler_command);
    finding.suggested_fixes.push(
        "Keep this as planning evidence until CircuitCI adds a real-Xyce -plugin loader, solver manifest contract, and conformance fixture for the specific generated model."
            .to_string(),
    );
    finding
}

fn osdi_backend_unsupported(scenario: &Scenario, model_file: &AnalogModelFile) -> Finding {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before OSDI backend compatibility checks");
    let backend = match analog.backend {
        AnalogBackend::Xyce => "xyce",
        AnalogBackend::EmbeddedNgspice => "embedded_ngspice",
        AnalogBackend::Auto | AnalogBackend::Ngspice => "ngspice",
    };
    let mut finding = Finding::critical(
        "ANALOG_MODEL_COMPILER_BACKEND_UNSUPPORTED",
        &scenario.name,
        format!(
            "Compiled OSDI artifact {} is not a supported model-loading format for backend {backend}.",
            model_file.path
        ),
    );
    finding
        .measured
        .insert("model_file".to_string(), json!(model_file.path));
    finding
        .measured
        .insert("artifact_format".to_string(), json!("osdi_shared_object"));
    finding
        .measured
        .insert("requested_backend".to_string(), json!(backend));
    finding.measured.insert(
        "research_evidence".to_string(),
        json!("docs/research/circuit_simulation_full_featured/xyce_openvaf_osdi_compatibility.md"),
    );
    finding.limit.insert(
        "supported_backend".to_string(),
        json!("external_ngspice_with_pre_osdi"),
    );
    finding.limit.insert(
        "xyce_required_model_path".to_string(),
        json!("Xyce/ADMS-generated C++ device linked into Xyce or loaded with -plugin from a shareable Xyce build"),
    );
    finding.suggested_fixes.push(
        "Use backend: ngspice for OpenVAF/OSDI artifacts, or replace the model artifact with a separately qualified Xyce/ADMS plugin flow."
            .to_string(),
    );
    finding
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
