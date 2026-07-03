use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;

use super::SPICE_S_PARAMETER_ANALYSIS;
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_runner::{
    AnalogRuntimeFeature, BackendSelection, backend_name, embedded_solver_unavailable,
    external_backend_unavailable, select_backend_for_feature,
};
use super::analog_spice::{push_canceled_finding, validate_netlist_source};
use super::analog_util::{file_sha256_hex, push_artifact};
use super::common::validation_input_missing;

pub(super) struct AnalogSParameterSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_sparameter_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogSParameterSinks<'_>,
    mut on_progress: F,
    should_cancel: C,
) where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let findings = &mut *sinks.findings;
    let artifacts = &mut *sinks.artifacts;
    on_progress(
        "Preparing analog S-parameter sweep",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_sparameter scenario requires an analog block.",
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
                    "SPICE model file {} is required for physical analog S-parameter simulation.",
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
            "analog_sparameter requires node_bindings and pin_bindings.",
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
                    "Analog S-parameter node binding {} references unknown board net {}.",
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
                    "Analog S-parameter pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "sparam" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only sparam is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let Some(start_hz) = analog.analysis.start_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_sparameter requires start_frequency_hz.",
        );
        return;
    };
    let Some(stop_hz) = analog.analysis.stop_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_sparameter requires stop_frequency_hz.",
        );
        return;
    };
    if !start_hz.is_finite() || !stop_hz.is_finite() || start_hz <= 0.0 || stop_hz <= start_hz {
        validation_input_missing(
            findings,
            scenario,
            "analog_sparameter requires 0 < start_frequency_hz < stop_frequency_hz.",
        );
        return;
    }
    if !matches!(analog.analysis.points_per_decade, Some(1..=1000)) {
        validation_input_missing(
            findings,
            scenario,
            "analog_sparameter requires points_per_decade in 1..=1000.",
        );
        return;
    }
    if analog.analysis.s_parameter_ports.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_sparameter requires at least one s_parameter_ports entry.",
        );
        return;
    }
    let mut port_names = BTreeSet::new();
    for port in &analog.analysis.s_parameter_ports {
        if !port_names.insert(port.name.as_str()) {
            validation_input_missing(
                findings,
                scenario,
                format!("analog_sparameter declares duplicate port {}.", port.name),
            );
            return;
        }
        if !bound_nodes.contains(port.positive_node.as_str()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "analog_sparameter port {} positive_node {} is not bound to a board net.",
                    port.name, port.positive_node
                ),
            );
            return;
        }
        if !bound_nodes.contains(port.negative_node.as_str()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "analog_sparameter port {} negative_node {} is not bound to a board net.",
                    port.name, port.negative_node
                ),
            );
            return;
        }
        if !port.reference_impedance_ohm.is_finite() || port.reference_impedance_ohm <= 0.0 {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "analog_sparameter port {} requires positive finite reference_impedance_ohm.",
                    port.name
                ),
            );
            return;
        }
    }

    on_progress(
        "Planning analog S-parameter backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::SParameter);
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
            check_id: SPICE_S_PARAMETER_ANALYSIS,
            selected_backend: backend,
            implemented_backend: "none_yet",
            analysis_kind: "s_parameter",
            required_normalized_outputs: &["s_parameters"],
        },
    );
    finding.measured.insert(
        "port_count".to_string(),
        json!(analog.analysis.s_parameter_ports.len()),
    );
    finding
        .measured
        .insert("frequency_start_hz".to_string(), json!(start_hz));
    finding
        .measured
        .insert("frequency_stop_hz".to_string(), json!(stop_hz));
    finding.limit.insert(
        "required_evidence".to_string(),
        json!("s_parameters_csv_or_touchstone"),
    );
    findings.push(finding);
}
