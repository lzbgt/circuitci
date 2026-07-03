use crate::board_ir::{AnalogNetlistSource, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;

use super::SPICE_TRANSFER_FUNCTION_ANALYSIS;
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_runner::{
    AnalogRuntimeFeature, BackendSelection, backend_name, embedded_solver_unavailable,
    external_backend_unavailable, select_backend_for_feature,
};
use super::analog_spice::{push_canceled_finding, validate_netlist_source};
use super::analog_util::{file_sha256_hex, push_artifact};
use super::common::validation_input_missing;

pub(super) struct AnalogTransferFunctionSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_transfer_function_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogTransferFunctionSinks<'_>,
    mut on_progress: F,
    should_cancel: C,
) where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let findings = &mut *sinks.findings;
    let artifacts = &mut *sinks.artifacts;
    on_progress(
        "Preparing analog transfer-function analysis",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_transfer_function scenario requires an analog block.",
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
                    "SPICE model file {} is required for physical analog transfer-function simulation.",
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
            "analog_transfer_function requires node_bindings and pin_bindings.",
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
                    "Analog transfer-function node binding {} references unknown board net {}.",
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
                    "Analog transfer-function pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "tf" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only tf is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let Some(output_expression) = analog.analysis.transfer_output_expression.as_deref() else {
        validation_input_missing(
            findings,
            scenario,
            "analog_transfer_function requires transfer_output_expression.",
        );
        return;
    };
    let Some(input_source) = analog.analysis.transfer_input_source.as_deref() else {
        validation_input_missing(
            findings,
            scenario,
            "analog_transfer_function requires transfer_input_source.",
        );
        return;
    };
    if output_expression.trim().is_empty() || input_source.trim().is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_transfer_function requires non-empty transfer output expression and input source.",
        );
        return;
    }
    if analog.netlist_source == AnalogNetlistSource::GeneratedFromBoard
        && !bound.project.board.components.contains_key(input_source)
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "analog_transfer_function input source {} is not a generated board component.",
                input_source
            ),
        );
        return;
    }

    on_progress(
        "Planning analog transfer-function backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected =
        select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::TransferFunction);
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
            check_id: SPICE_TRANSFER_FUNCTION_ANALYSIS,
            selected_backend: backend,
            implemented_backend: "none_yet",
            analysis_kind: "transfer_function",
            required_normalized_outputs: &["transfer_function_summary"],
        },
    );
    finding
        .measured
        .insert("output_expression".to_string(), json!(output_expression));
    finding
        .measured
        .insert("input_source".to_string(), json!(input_source));
    finding.limit.insert(
        "required_evidence".to_string(),
        json!("transfer_function_summary_csv_or_json"),
    );
    findings.push(finding);
}
