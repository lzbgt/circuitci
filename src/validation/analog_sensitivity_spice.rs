use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_SENSITIVITY_ANALYSIS;
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

pub(super) struct AnalogSensitivitySinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_sensitivity_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogSensitivitySinks<'_>,
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
        "Preparing analog sensitivity analysis",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_sensitivity scenario requires an analog block.",
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
                    "SPICE model file {} is required for physical analog sensitivity analysis.",
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
            "analog_sensitivity requires node_bindings and pin_bindings.",
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
                    "Analog sensitivity node binding {} references unknown board net {}.",
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
                    "Analog sensitivity pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "sens" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only sens is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let Some(output_expression) =
        nonempty(analog.analysis.sensitivity_output_expression.as_deref())
    else {
        validation_input_missing(
            findings,
            scenario,
            "analog_sensitivity requires sensitivity_output_expression.",
        );
        return;
    };
    let Some(mode) = nonempty(analog.analysis.sensitivity_mode.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_sensitivity requires sensitivity_mode.",
        );
        return;
    };
    if !matches!(mode, "dc" | "ac") {
        validation_input_missing(
            findings,
            scenario,
            "analog_sensitivity requires sensitivity_mode to be dc or ac.",
        );
        return;
    }
    if let Err(message) = validate_output_expression(output_expression, &bound_nodes, scenario) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if mode == "ac" {
        let Some(start_hz) = analog.analysis.start_frequency_hz else {
            validation_input_missing(
                findings,
                scenario,
                "analog_sensitivity AC mode requires start_frequency_hz.",
            );
            return;
        };
        let Some(stop_hz) = analog.analysis.stop_frequency_hz else {
            validation_input_missing(
                findings,
                scenario,
                "analog_sensitivity AC mode requires stop_frequency_hz.",
            );
            return;
        };
        if !start_hz.is_finite() || !stop_hz.is_finite() || start_hz <= 0.0 || stop_hz <= start_hz {
            validation_input_missing(
                findings,
                scenario,
                "analog_sensitivity AC mode requires 0 < start_frequency_hz < stop_frequency_hz.",
            );
            return;
        }
        if !matches!(analog.analysis.points_per_decade, Some(1..=1000)) {
            validation_input_missing(
                findings,
                scenario,
                "analog_sensitivity AC mode requires points_per_decade in 1..=1000.",
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
            SPICE_SENSITIVITY_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create analog sensitivity run directory {}: {error}",
                run_dir.display()
            ),
        ));
        return;
    }
    match prepare_source_netlist(bound, scenario, &run_dir) {
        Ok(source_netlist) => push_artifact(artifacts, &source_netlist),
        Err(message) => {
            let mut finding =
                Finding::critical(SPICE_SENSITIVITY_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    }

    on_progress(
        "Planning analog sensitivity backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::Sensitivity);
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
            check_id: SPICE_SENSITIVITY_ANALYSIS,
            selected_backend: backend,
            implemented_backend: "none_yet",
            analysis_kind: "sensitivity",
            required_normalized_outputs: &["sensitivity_summary"],
        },
    );
    finding
        .measured
        .insert("output_expression".to_string(), json!(output_expression));
    finding.measured.insert("mode".to_string(), json!(mode));
    finding.measured.insert(
        "filters".to_string(),
        json!(analog.analysis.sensitivity_filters),
    );
    if mode == "ac" {
        finding.measured.insert(
            "frequency_start_hz".to_string(),
            json!(analog.analysis.start_frequency_hz),
        );
        finding.measured.insert(
            "frequency_stop_hz".to_string(),
            json!(analog.analysis.stop_frequency_hz),
        );
        finding.measured.insert(
            "points_per_decade".to_string(),
            json!(analog.analysis.points_per_decade),
        );
    }
    finding.limit.insert(
        "required_evidence".to_string(),
        json!("sensitivity_summary_csv_or_json"),
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
                    "analog_sensitivity output expression references unbound node {node}."
                ));
            }
        }
        return Ok(());
    }
    if lower.starts_with("i(") && expression.ends_with(')') {
        let component = expression[2..expression.len() - 1].trim();
        if component.is_empty() {
            return Err("analog_sensitivity current expression requires a component.".to_string());
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
                "analog_sensitivity output expression references unbound component {component}."
            ));
        }
        return Ok(());
    }
    Err(
        "analog_sensitivity output expression must be V(node), V(node,reference), or I(source)."
            .to_string(),
    )
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
