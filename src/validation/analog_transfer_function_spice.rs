use crate::board_ir::{AnalogNetlistSource, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_TRANSFER_FUNCTION_ANALYSIS;
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_runner::{
    AnalogRuntimeFeature, BackendSelection, backend_name, embedded_solver_unavailable,
    external_backend_unavailable, select_backend_for_feature,
};
use super::analog_spice::{
    analog_run_plans, prepare_source_netlist, push_canceled_finding, validate_netlist_source,
};
use super::analog_sweep_reports::{
    push_sweep_margin_summaries, record_sweep_measurements, tag_corner_finding, tag_corner_findings,
};
use super::analog_transfer_function_assertions::{
    evaluate_transfer_function_assertions, validate_transfer_function_assertion_contract,
};
use super::analog_transfer_function_runner::{
    NgspiceTransferFunctionRunOptions, run_ngspice_transfer_function,
};
use super::analog_util::{push_artifact, safe_artifact_name};
use super::common::validation_input_missing;

pub(super) struct AnalogTransferFunctionSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_transfer_function_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogTransferFunctionSinks<'_>,
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
    if let Err(message) = validate_transfer_function_assertion_contract(analog) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    let run_plans = match analog_run_plans(analog) {
        Ok(run_plans) => run_plans,
        Err(message) => {
            validation_input_missing(findings, scenario, message);
            return;
        }
    };

    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Err(error) = fs::create_dir_all(&run_dir) {
        findings.push(Finding::critical(
            SPICE_TRANSFER_FUNCTION_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create analog transfer-function run directory {}: {error}",
                run_dir.display()
            ),
        ));
        return;
    }
    let source_netlist = match prepare_source_netlist(bound, scenario, &run_dir) {
        Ok(source_netlist) => {
            push_artifact(artifacts, &source_netlist);
            source_netlist
        }
        Err(message) => {
            let mut finding =
                Finding::critical(SPICE_TRANSFER_FUNCTION_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    };

    on_progress(
        "Selecting analog transfer-function backend",
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

    if backend != "ngspice" {
        let mut finding = unsupported_backend_plan_finding(
            scenario,
            UnsupportedBackendPlan {
                check_id: SPICE_TRANSFER_FUNCTION_ANALYSIS,
                selected_backend: backend,
                implemented_backend: "ngspice",
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
        if backend.eq_ignore_ascii_case("xyce") {
            finding.measured.insert(
                "adapter_blocker".to_string(),
                json!("Xyce 7.8 primary documentation in this repository does not document a native .TF command or transfer-function result artifact."),
            );
            finding.measured.insert(
                "evidence_sources".to_string(),
                json!(["docs/research/circuit_simulation_full_featured/sources/Xyce_Reference_Guide_7.8.txt"]),
            );
        }
        finding.limit.insert(
            "required_evidence".to_string(),
            json!("transfer_function_summary_csv_or_json"),
        );
        findings.push(finding);
        return;
    }

    let mut sweep_measurements = Vec::new();
    for run_plan in run_plans {
        if should_cancel() {
            push_canceled_finding(findings, scenario);
            return;
        }
        on_progress(
            "Running analog transfer-function input corner",
            run_plan.progress_label(),
        );
        let parameter_overrides = run_plan.parameter_overrides_for_solver();
        let run_result = run_ngspice_transfer_function(
            bound,
            scenario,
            backend,
            &source_netlist,
            NgspiceTransferFunctionRunOptions {
                output,
                run_subdir: run_plan.run_subdir.as_deref(),
                parameter_overrides: &parameter_overrides,
                model_section_overrides: &run_plan.model_section_overrides,
                on_progress: &mut on_progress,
                should_cancel: &should_cancel,
            },
        );
        match run_result {
            Ok(run) => {
                let finding_start = findings.len();
                for artifact in &run.artifacts {
                    push_artifact(artifacts, artifact);
                }
                push_artifact(artifacts, &run.summary);
                let assertion_measurements =
                    evaluate_transfer_function_assertions(scenario, &run.summary, findings);
                record_sweep_measurements(
                    &mut sweep_measurements,
                    &run_plan,
                    assertion_measurements,
                );
                tag_corner_findings(findings, finding_start, &run_plan, false);
            }
            Err(error) => {
                for artifact in &error.artifacts {
                    push_artifact(artifacts, artifact);
                }
                let mut finding = Finding::critical(
                    SPICE_TRANSFER_FUNCTION_ANALYSIS,
                    &scenario.name,
                    error.message,
                );
                finding
                    .measured
                    .insert("selected_backend".to_string(), json!(backend));
                finding.limit.insert(
                    "required_evidence".to_string(),
                    json!("ngspice_transfer_function_summary_csv"),
                );
                tag_corner_finding(&mut finding, &run_plan);
                finding.suggested_fixes.push(
                    "Inspect the generated ngspice .TF wrapper deck and solver log artifacts."
                        .to_string(),
                );
                findings.push(finding);
            }
        }
    }
    push_sweep_margin_summaries(findings, scenario, &sweep_measurements);
}
