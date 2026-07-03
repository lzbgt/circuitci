use crate::board_ir::{AnalogNetlistSource, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_PSS_ANALYSIS;
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

pub(super) struct AnalogPssSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_pss_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogPssSinks<'_>,
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
        "Preparing periodic steady-state analysis",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss scenario requires an analog block.",
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
                    "SPICE model file {} is required for physical periodic steady-state analysis.",
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
            "analog_pss requires node_bindings and pin_bindings.",
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
                    "Analog PSS node binding {} references unknown board net {}.",
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
                    "Analog PSS pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "pss" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only pss is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let mode = analog.analysis.pss_mode.as_deref().unwrap_or("driven");
    if !matches!(mode, "driven" | "autonomous") {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires pss_mode to be driven or autonomous when provided.",
        );
        return;
    }
    let Some(frequency_guess_hz) = analog.analysis.pss_frequency_guess_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires pss_frequency_guess_hz.",
        );
        return;
    };
    if !frequency_guess_hz.is_finite() || frequency_guess_hz <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires positive finite pss_frequency_guess_hz.",
        );
        return;
    }
    let Some(stabilization_time_us) = analog.analysis.pss_stabilization_time_us else {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires pss_stabilization_time_us.",
        );
        return;
    };
    if !stabilization_time_us.is_finite() || stabilization_time_us <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires positive finite pss_stabilization_time_us.",
        );
        return;
    }
    let Some(output_expression) = nonempty(analog.analysis.pss_output_expression.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires pss_output_expression.",
        );
        return;
    };
    if let Err(message) = validate_output_expression(output_expression, &bound_nodes, scenario) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if !matches!(analog.analysis.pss_periods, None | Some(1..=4096)) {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires pss_periods in 1..=4096 when provided.",
        );
        return;
    }
    if let Some(tolerance) = analog.analysis.pss_residual_tolerance
        && (!tolerance.is_finite() || tolerance <= 0.0)
    {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires positive finite pss_residual_tolerance when provided.",
        );
        return;
    }
    if let Some(tolerance) = analog.analysis.pss_state_error_tolerance
        && (!tolerance.is_finite() || tolerance <= 0.0)
    {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires positive finite pss_state_error_tolerance when provided.",
        );
        return;
    }
    if !matches!(analog.analysis.pss_max_iterations, None | Some(1..=10000)) {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires pss_max_iterations in 1..=10000 when provided.",
        );
        return;
    }
    if mode == "driven" && analog.analysis.pss_drive_sources.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss driven mode requires at least one pss_drive_sources entry.",
        );
        return;
    }
    let mut drive_sources = BTreeSet::new();
    for source in &analog.analysis.pss_drive_sources {
        let source = source.trim();
        if source.is_empty() {
            validation_input_missing(
                findings,
                scenario,
                "analog_pss pss_drive_sources entries must be non-empty.",
            );
            return;
        }
        if !drive_sources.insert(source) {
            validation_input_missing(
                findings,
                scenario,
                format!("analog_pss declares duplicate drive source {source}."),
            );
            return;
        }
        if analog.netlist_source == AnalogNetlistSource::GeneratedFromBoard
            && !bound.project.board.components.contains_key(source)
        {
            validation_input_missing(
                findings,
                scenario,
                format!("analog_pss drive source {source} is not a generated board component."),
            );
            return;
        }
    }

    let run_plans = match analog_run_plans(analog) {
        Ok(run_plans) => run_plans,
        Err(message) => {
            validation_input_missing(findings, scenario, message);
            return;
        }
    };
    if run_plans.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_pss requires at least one run plan.",
        );
        return;
    }

    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Err(error) = fs::create_dir_all(&run_dir) {
        findings.push(Finding::critical(
            SPICE_PSS_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create periodic steady-state planning directory {}: {error}",
                run_dir.display()
            ),
        ));
        return;
    }
    match prepare_source_netlist(bound, scenario, &run_dir) {
        Ok(source_netlist) => push_artifact(artifacts, &source_netlist),
        Err(message) => {
            let mut finding = Finding::critical(SPICE_PSS_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    }

    on_progress(
        "Planning periodic steady-state backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected =
        select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::PeriodicSteadyState);
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
            check_id: SPICE_PSS_ANALYSIS,
            selected_backend: backend,
            implemented_backend: "none_yet",
            analysis_kind: "periodic_steady_state",
            required_normalized_outputs: &["pss_waveform", "pss_spectrum", "pss_convergence"],
        },
    );
    finding.measured.insert("pss_mode".to_string(), json!(mode));
    finding
        .measured
        .insert("frequency_guess_hz".to_string(), json!(frequency_guess_hz));
    finding.measured.insert(
        "stabilization_time_us".to_string(),
        json!(stabilization_time_us),
    );
    finding.measured.insert(
        "periods".to_string(),
        json!(analog.analysis.pss_periods.unwrap_or(1)),
    );
    finding
        .measured
        .insert("output_expression".to_string(), json!(output_expression));
    finding.measured.insert(
        "drive_sources".to_string(),
        json!(analog.analysis.pss_drive_sources),
    );
    finding.measured.insert(
        "residual_tolerance".to_string(),
        json!(analog.analysis.pss_residual_tolerance),
    );
    finding.measured.insert(
        "state_error_tolerance".to_string(),
        json!(analog.analysis.pss_state_error_tolerance),
    );
    finding.measured.insert(
        "max_iterations".to_string(),
        json!(analog.analysis.pss_max_iterations),
    );
    finding.measured.insert(
        "backend_research_status".to_string(),
        json!({
            "ngspice": "experimental_potential_autonomous_pss_only_requires_enable_pss_and_no_stable_normalized_output_contract",
            "xyce": "no_distinct_pss_command_in_xyce_7_8_docs_hb_only_for_current_runtime",
            "spice_opus": "ssse_shooting_exists_but_no_circuitci_runtime_adapter_or_conformance_contract"
        }),
    );
    finding.limit.insert(
        "required_evidence".to_string(),
        json!("pss_waveform_pss_spectrum_pss_convergence"),
    );
    finding.limit.insert(
        "current_limitation".to_string(),
        json!("CircuitCI records PSS/oscillator intent and convergence requirements but has no trusted PSS solver adapter or normalized output contract yet."),
    );
    finding.limit.insert(
        "trusted_backend_status".to_string(),
        json!("none_available"),
    );
    finding.suggested_fixes.push(
        "Keep oscillator sign-off blocked until a backend emits normalized PSS waveform, spectrum, convergence, and solver-manifest artifacts with real-solver conformance coverage."
            .to_string(),
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
                    "analog_pss output expression references unbound node {node}."
                ));
            }
        }
        return Ok(());
    }
    if lower.starts_with("i(") && expression.ends_with(')') {
        let component = expression[2..expression.len() - 1].trim();
        if component.is_empty() {
            return Err("analog_pss current expression requires a component.".to_string());
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
                "analog_pss output expression references unbound component {component}."
            ));
        }
        return Ok(());
    }
    Err(
        "analog_pss output expression must be V(node), V(node,reference), or I(source)."
            .to_string(),
    )
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
