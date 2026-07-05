use crate::board_ir::{AnalogModelFile, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::path::Path;

use super::super::analog_util::push_artifact;
use super::{
    insert_optional_model_package_measured, nonempty, split_compiler_command, validate_pinned_file,
};

pub(super) fn validate_xyce_adms_plugin_contract(
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
    insert_optional_model_package_measured(finding, model_file);
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
