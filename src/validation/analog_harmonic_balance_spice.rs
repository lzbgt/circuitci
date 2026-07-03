use crate::board_ir::{AnalogNetlistSource, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_HARMONIC_BALANCE_ANALYSIS;
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_runner::{
    AnalogRuntimeFeature, BackendSelection, backend_name, embedded_solver_unavailable,
    external_backend_unavailable, select_backend_for_feature,
};
use super::analog_spice::{
    analog_run_plans, prepare_source_netlist, push_canceled_finding, validate_netlist_source,
};
use super::analog_util::{file_sha256_hex, push_artifact, safe_artifact_name};
use super::common::validation_input_missing;

pub(super) struct AnalogHarmonicBalanceSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_harmonic_balance_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogHarmonicBalanceSinks<'_>,
    output: &Path,
    mut on_progress: F,
    should_cancel: C,
) where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let findings = &mut *sinks.findings;
    let artifacts = &mut *sinks.artifacts;
    on_progress(
        "Preparing harmonic-balance analysis",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_harmonic_balance scenario requires an analog block.",
        );
        return;
    };
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }

    if let Some(finding) = validate_netlist_source(bound, scenario, artifacts) {
        findings.push(finding);
        return;
    }
    for model_file in &analog.model_files {
        let path = bound.project.source_dir.join(&model_file.path);
        if !path.is_file() {
            let mut finding = Finding::critical(
                "ANALOG_MODEL_UNAVAILABLE",
                &scenario.name,
                format!(
                    "SPICE model file {} is required for physical harmonic-balance analysis.",
                    path.display()
                ),
            );
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_model_file"));
            findings.push(finding);
            return;
        }
        if let Some(expected) = &model_file.sha256 {
            match file_sha256_hex(&path) {
                Ok(actual) if actual.eq_ignore_ascii_case(expected) => {}
                Ok(actual) => {
                    let mut finding = Finding::critical(
                        "ANALOG_MODEL_HASH_MISMATCH",
                        &scenario.name,
                        format!(
                            "SPICE model file {} does not match the declared SHA-256.",
                            path.display()
                        ),
                    );
                    finding.measured.insert("sha256".to_string(), json!(actual));
                    finding
                        .limit
                        .insert("expected_sha256".to_string(), json!(expected));
                    findings.push(finding);
                    return;
                }
                Err(message) => {
                    validation_input_missing(findings, scenario, message);
                    return;
                }
            }
        }
        push_artifact(artifacts, &path);
    }

    if analog.node_bindings.is_empty() || analog.pin_bindings.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_harmonic_balance requires node_bindings and pin_bindings.",
        );
        return;
    }
    let mut bound_nodes = BTreeSet::new();
    for binding in &analog.node_bindings {
        if !bound.project.board.nets.contains_key(&binding.net) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog harmonic-balance node binding {} references unknown board net {}.",
                    binding.node, binding.net
                ),
            );
            return;
        }
        bound_nodes.insert(binding.node.as_str());
    }
    for binding in &analog.pin_bindings {
        if !bound_nodes.contains(binding.node.as_str()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog harmonic-balance pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "hb" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only hb is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let Some(fundamental_hz) = analog.analysis.hb_fundamental_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_harmonic_balance requires hb_fundamental_frequency_hz.",
        );
        return;
    };
    if !fundamental_hz.is_finite() || fundamental_hz <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "analog_harmonic_balance requires positive finite hb_fundamental_frequency_hz.",
        );
        return;
    }
    let Some(output_expression) = nonempty(analog.analysis.hb_output_expression.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_harmonic_balance requires hb_output_expression.",
        );
        return;
    };
    if let Err(message) = validate_output_expression(output_expression, &bound_nodes, scenario) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if !matches!(analog.analysis.hb_harmonics, None | Some(1..=1024)) {
        validation_input_missing(
            findings,
            scenario,
            "analog_harmonic_balance requires hb_harmonics in 1..=1024 when provided.",
        );
        return;
    }
    if analog.analysis.hb_drive_sources.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_harmonic_balance requires at least one hb_drive_sources entry.",
        );
        return;
    }
    let mut drive_sources = BTreeSet::new();
    for source in &analog.analysis.hb_drive_sources {
        let source = source.trim();
        if source.is_empty() {
            validation_input_missing(
                findings,
                scenario,
                "analog_harmonic_balance hb_drive_sources entries must be non-empty.",
            );
            return;
        }
        if !drive_sources.insert(source) {
            validation_input_missing(
                findings,
                scenario,
                format!("analog_harmonic_balance declares duplicate drive source {source}."),
            );
            return;
        }
        if analog.netlist_source == AnalogNetlistSource::GeneratedFromBoard
            && !bound.project.board.components.contains_key(source)
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "analog_harmonic_balance drive source {source} is not a generated board component."
                ),
            );
            return;
        }
    }

    if let Err(message) = analog_run_plans(analog) {
        validation_input_missing(findings, scenario, message);
        return;
    }

    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Err(error) = fs::create_dir_all(&run_dir) {
        findings.push(Finding::critical(
            SPICE_HARMONIC_BALANCE_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create harmonic-balance planning directory {}: {error}",
                run_dir.display()
            ),
        ));
        return;
    }
    match prepare_source_netlist(bound, scenario, &run_dir) {
        Ok(source_netlist) => push_artifact(artifacts, &source_netlist),
        Err(message) => {
            let mut finding =
                Finding::critical(SPICE_HARMONIC_BALANCE_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    }

    on_progress(
        "Planning harmonic-balance backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected =
        select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::HarmonicBalance);
    let BackendSelection::Selected(backend) = selected else {
        let mut finding = match selected {
            BackendSelection::EmbeddedUnavailable => embedded_solver_unavailable(&scenario.name),
            BackendSelection::Unavailable => {
                external_backend_unavailable(&scenario.name, &analog.backend)
            }
            BackendSelection::Selected(_) => unreachable!("handled by let-else pattern"),
        };
        finding.measured.insert(
            "requested_backend".to_string(),
            json!(backend_name(&analog.backend)),
        );
        findings.push(finding);
        return;
    };

    let mut finding = unsupported_backend_plan_finding(
        scenario,
        UnsupportedBackendPlan {
            check_id: SPICE_HARMONIC_BALANCE_ANALYSIS,
            selected_backend: backend,
            implemented_backend: "none_yet",
            analysis_kind: "harmonic_balance",
            required_normalized_outputs: &["hb_spectrum"],
        },
    );
    finding
        .measured
        .insert("output_expression".to_string(), json!(output_expression));
    finding.measured.insert(
        "fundamental_frequency_hz".to_string(),
        json!(fundamental_hz),
    );
    finding.measured.insert(
        "harmonics".to_string(),
        json!(analog.analysis.hb_harmonics.unwrap_or(10)),
    );
    finding.measured.insert(
        "drive_sources".to_string(),
        json!(analog.analysis.hb_drive_sources),
    );
    finding
        .limit
        .insert("preferred_backend".to_string(), json!("xyce"));
    finding.limit.insert(
        "required_evidence".to_string(),
        json!("hb_spectrum_csv_or_json"),
    );
    findings.push(finding);
}

fn validate_output_expression(
    expression: &str,
    bound_nodes: &BTreeSet<&str>,
    scenario: &Scenario,
) -> Result<(), String> {
    let expression = expression.trim();
    let lower = expression.to_ascii_lowercase();
    if lower.starts_with("v(") && expression.ends_with(')') {
        let inner = &expression[2..expression.len() - 1];
        for node in inner
            .split(',')
            .map(str::trim)
            .filter(|node| !node.is_empty())
        {
            if !bound_nodes.contains(node) {
                return Err(format!(
                    "analog_harmonic_balance output expression references unbound node {node}."
                ));
            }
        }
        return Ok(());
    }
    if lower.starts_with("i(") && expression.ends_with(')') {
        let component = expression[2..expression.len() - 1].trim();
        if component.is_empty() {
            return Err(
                "analog_harmonic_balance current expression requires a component.".to_string(),
            );
        }
        let Some(analog) = &scenario.analog else {
            return Ok(());
        };
        let bound = analog
            .pin_bindings
            .iter()
            .any(|binding| binding.endpoint.component == component);
        if !bound {
            return Err(format!(
                "analog_harmonic_balance output expression references unbound component {component}."
            ));
        }
        return Ok(());
    }
    Err(
        "analog_harmonic_balance output expression must be V(node), V(node,reference), or I(source)."
            .to_string(),
    )
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
