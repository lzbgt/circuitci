use crate::board_ir::{AnalogNetlistSource, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_POLE_ZERO_ANALYSIS;
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_pole_zero_assertions::{
    evaluate_pole_zero_assertions, validate_pole_zero_assertion_contract,
};
use super::analog_pole_zero_runner::{NgspicePoleZeroRunOptions, run_ngspice_pole_zero};
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
use super::analog_util::{file_sha256_hex, push_artifact, safe_artifact_name};
use super::common::validation_input_missing;

pub(super) struct AnalogPoleZeroSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_pole_zero_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogPoleZeroSinks<'_>,
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
        "Preparing analog pole-zero analysis",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_pole_zero scenario requires an analog block.",
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
                    "SPICE model file {} is required for physical analog pole-zero analysis.",
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
            "analog_pole_zero requires node_bindings and pin_bindings.",
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
                    "Analog pole-zero node binding {} references unknown board net {}.",
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
                    "Analog pole-zero pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "pz" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only pz is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let Some(output_node) = nonempty(analog.analysis.pole_zero_output_node.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_pole_zero requires pole_zero_output_node.",
        );
        return;
    };
    let Some(reference_node) = nonempty(analog.analysis.pole_zero_reference_node.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_pole_zero requires pole_zero_reference_node.",
        );
        return;
    };
    let Some(input_source) = nonempty(analog.analysis.pole_zero_input_source.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_pole_zero requires pole_zero_input_source.",
        );
        return;
    };
    let Some(mode) = nonempty(analog.analysis.pole_zero_mode.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_pole_zero requires pole_zero_mode.",
        );
        return;
    };
    if !matches!(mode, "poles" | "zeros" | "poles_and_zeros") {
        validation_input_missing(
            findings,
            scenario,
            "analog_pole_zero requires pole_zero_mode to be poles, zeros, or poles_and_zeros.",
        );
        return;
    }
    if let Err(message) = validate_pole_zero_assertion_contract(analog) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if !bound_nodes.contains(output_node) {
        validation_input_missing(
            findings,
            scenario,
            format!("analog_pole_zero output node {output_node} is not bound to a board net."),
        );
        return;
    }
    if !bound_nodes.contains(reference_node) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "analog_pole_zero reference node {reference_node} is not bound to a board net."
            ),
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
                "analog_pole_zero input source {input_source} is not a generated board component."
            ),
        );
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
            SPICE_POLE_ZERO_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create analog pole-zero run directory {}: {error}",
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
            let mut finding = Finding::critical(SPICE_POLE_ZERO_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    };

    on_progress(
        "Planning analog pole-zero backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::PoleZero);
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
                check_id: SPICE_POLE_ZERO_ANALYSIS,
                selected_backend: backend,
                implemented_backend: "ngspice",
                analysis_kind: "pole_zero",
                required_normalized_outputs: &["pole_zero_summary"],
            },
        );
        finding
            .measured
            .insert("output_node".to_string(), json!(output_node));
        finding
            .measured
            .insert("reference_node".to_string(), json!(reference_node));
        finding
            .measured
            .insert("input_source".to_string(), json!(input_source));
        finding.measured.insert("mode".to_string(), json!(mode));
        finding.limit.insert(
            "required_evidence".to_string(),
            json!("pole_zero_summary_csv_or_json"),
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
            "Running analog pole-zero input corner",
            run_plan.progress_label(),
        );
        let parameter_overrides = run_plan.parameter_overrides_for_solver();
        let run_result = run_ngspice_pole_zero(
            bound,
            scenario,
            backend,
            &source_netlist,
            NgspicePoleZeroRunOptions {
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
                    evaluate_pole_zero_assertions(scenario, &run.summary, findings);
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
                let mut finding =
                    Finding::critical(SPICE_POLE_ZERO_ANALYSIS, &scenario.name, error.message);
                finding
                    .measured
                    .insert("selected_backend".to_string(), json!(backend));
                finding.limit.insert(
                    "required_evidence".to_string(),
                    json!("ngspice_pole_zero_summary_csv"),
                );
                tag_corner_finding(&mut finding, &run_plan);
                finding.suggested_fixes.push(
                    "Inspect the generated ngspice .PZ wrapper deck and solver log artifacts."
                        .to_string(),
                );
                findings.push(finding);
            }
        }
    }
    push_sweep_margin_summaries(findings, scenario, &sweep_measurements);
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
