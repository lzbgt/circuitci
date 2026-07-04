use crate::board_ir::{AnalogNetlistSource, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_PHASE_NOISE_ANALYSIS;
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

pub(super) struct AnalogPhaseNoiseSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_phase_noise_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogPhaseNoiseSinks<'_>,
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
        "Preparing phase-noise analysis",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise scenario requires an analog block.",
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
                    "SPICE model file {} is required for physical phase-noise analysis.",
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
            "analog_phase_noise requires node_bindings and pin_bindings.",
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
                    "Analog phase-noise node binding {} references unknown board net {}.",
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
                    "Analog phase-noise pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "phase_noise" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only phase_noise is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let mode = analog
        .analysis
        .phase_noise_mode
        .as_deref()
        .unwrap_or("autonomous");
    if !matches!(mode, "driven" | "autonomous") {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise requires phase_noise_mode to be driven or autonomous when provided.",
        );
        return;
    }
    let Some(carrier_frequency_hz) = analog.analysis.phase_noise_carrier_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise requires phase_noise_carrier_frequency_hz.",
        );
        return;
    };
    if !carrier_frequency_hz.is_finite() || carrier_frequency_hz <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise requires positive finite phase_noise_carrier_frequency_hz.",
        );
        return;
    }
    let Some(offset_start_hz) = analog.analysis.phase_noise_offset_start_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise requires phase_noise_offset_start_hz.",
        );
        return;
    };
    let Some(offset_stop_hz) = analog.analysis.phase_noise_offset_stop_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise requires phase_noise_offset_stop_hz.",
        );
        return;
    };
    if !offset_start_hz.is_finite() || offset_start_hz <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise requires positive finite phase_noise_offset_start_hz.",
        );
        return;
    }
    if !offset_stop_hz.is_finite() || offset_stop_hz <= offset_start_hz {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise requires phase_noise_offset_stop_hz greater than phase_noise_offset_start_hz.",
        );
        return;
    }
    let Some(points_per_decade) = analog.analysis.phase_noise_points_per_decade else {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise requires phase_noise_points_per_decade.",
        );
        return;
    };
    if !(1..=1000).contains(&points_per_decade) {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise requires phase_noise_points_per_decade in 1..=1000.",
        );
        return;
    }
    let Some(output_expression) =
        nonempty(analog.analysis.phase_noise_output_expression.as_deref())
    else {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise requires phase_noise_output_expression.",
        );
        return;
    };
    if let Err(message) = validate_output_expression(output_expression, &bound_nodes, scenario) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if let Err(message) = validate_integration_window(analog, offset_start_hz, offset_stop_hz) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if mode == "driven" && analog.analysis.phase_noise_drive_sources.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_phase_noise driven mode requires at least one phase_noise_drive_sources entry.",
        );
        return;
    }
    let mut drive_sources = BTreeSet::new();
    for source in &analog.analysis.phase_noise_drive_sources {
        let source = source.trim();
        if source.is_empty() {
            validation_input_missing(
                findings,
                scenario,
                "analog_phase_noise phase_noise_drive_sources entries must be non-empty.",
            );
            return;
        }
        if !drive_sources.insert(source) {
            validation_input_missing(
                findings,
                scenario,
                format!("analog_phase_noise declares duplicate drive source {source}."),
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
                    "analog_phase_noise drive source {source} is not a generated board component."
                ),
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
            "analog_phase_noise requires at least one run plan.",
        );
        return;
    }

    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Err(error) = fs::create_dir_all(&run_dir) {
        findings.push(Finding::critical(
            SPICE_PHASE_NOISE_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create phase-noise planning directory {}: {error}",
                run_dir.display()
            ),
        ));
        return;
    }
    match prepare_source_netlist(bound, scenario, &run_dir) {
        Ok(source_netlist) => push_artifact(artifacts, &source_netlist),
        Err(message) => {
            let mut finding =
                Finding::critical(SPICE_PHASE_NOISE_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    }

    on_progress(
        "Planning phase-noise backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::PhaseNoise);
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
            check_id: SPICE_PHASE_NOISE_ANALYSIS,
            selected_backend: backend,
            implemented_backend: "none_yet",
            analysis_kind: "phase_noise",
            required_normalized_outputs: &[
                "phase_noise_spectrum",
                "phase_noise_integrated_jitter",
                "phase_noise_convergence",
                "pss_convergence",
            ],
        },
    );
    finding
        .measured
        .insert("phase_noise_mode".to_string(), json!(mode));
    finding.measured.insert(
        "carrier_frequency_hz".to_string(),
        json!(carrier_frequency_hz),
    );
    finding
        .measured
        .insert("offset_start_hz".to_string(), json!(offset_start_hz));
    finding
        .measured
        .insert("offset_stop_hz".to_string(), json!(offset_stop_hz));
    finding
        .measured
        .insert("points_per_decade".to_string(), json!(points_per_decade));
    finding
        .measured
        .insert("output_expression".to_string(), json!(output_expression));
    finding.measured.insert(
        "drive_sources".to_string(),
        json!(analog.analysis.phase_noise_drive_sources),
    );
    finding.measured.insert(
        "integration_start_hz".to_string(),
        json!(analog.analysis.phase_noise_integration_start_hz),
    );
    finding.measured.insert(
        "integration_stop_hz".to_string(),
        json!(analog.analysis.phase_noise_integration_stop_hz),
    );
    finding.measured.insert(
        "backend_research_status".to_string(),
        json!({
            "qucs_copen": "papers_document_pnsolver_after_psssolver_but_no_public_source_repository_or_adapter_contract_found",
            "xyce": "hb_supported_but_no_trusted_phase_noise_or_autonomous_pss_adapter_in_current_runtime",
            "ngspice": "experimental_pss_is_build_gated_and_no_trusted_pnoise_output_contract_is_available",
            "spice_opus": "ssse_shooting_exists_but_no_circuitci_phase_noise_adapter_or_conformance_contract"
        }),
    );
    finding.measured.insert(
        "adapter_blocker".to_string(),
        json!("No trusted open-source PSS/PNOISE solver chain with normalized phase-noise spectrum, integrated jitter, convergence, raw-output, and solver-manifest artifacts is available in this runtime."),
    );
    finding.measured.insert(
        "evidence_sources".to_string(),
        json!([
            "docs/research/circuit_simulation_full_featured/pss_backend_evidence.md",
            "docs/research/circuit_simulation_full_featured/sources/arxiv_2512.10373_qucs_phase_noise.txt",
            "docs/research/circuit_simulation_full_featured/sources/arxiv_2603.07828_qucs_phase_noise_part2.txt"
        ]),
    );
    finding.limit.insert(
        "required_evidence".to_string(),
        json!("phase_noise_spectrum_phase_noise_integrated_jitter_phase_noise_convergence_pss_convergence"),
    );
    finding.limit.insert(
        "current_limitation".to_string(),
        json!("CircuitCI records phase-noise intent and offset-sweep requirements but has no trusted PSS/PNOISE solver chain or normalized output contract yet."),
    );
    finding.limit.insert(
        "trusted_backend_status".to_string(),
        json!("none_available"),
    );
    finding.suggested_fixes.push(
        "Keep oscillator phase-noise sign-off blocked until a backend emits normalized phase-noise spectrum, integrated jitter, PSS convergence, phase-noise convergence, and solver-manifest artifacts with real-solver conformance coverage."
            .to_string(),
    );
    findings.push(finding);
}

fn validate_integration_window(
    analog: &crate::board_ir::AnalogScenario,
    offset_start_hz: f64,
    offset_stop_hz: f64,
) -> Result<(), String> {
    match (
        analog.analysis.phase_noise_integration_start_hz,
        analog.analysis.phase_noise_integration_stop_hz,
    ) {
        (None, None) => Ok(()),
        (Some(start), Some(stop)) => {
            if !start.is_finite() || start <= 0.0 {
                return Err("analog_phase_noise requires positive finite phase_noise_integration_start_hz when provided.".to_string());
            }
            if !stop.is_finite() || stop <= start {
                return Err("analog_phase_noise requires phase_noise_integration_stop_hz greater than phase_noise_integration_start_hz.".to_string());
            }
            if start < offset_start_hz || stop > offset_stop_hz {
                return Err("analog_phase_noise integration range must stay inside the phase-noise offset sweep range.".to_string());
            }
            Ok(())
        }
        _ => Err("analog_phase_noise requires both phase_noise_integration_start_hz and phase_noise_integration_stop_hz, or neither.".to_string()),
    }
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
                    "analog_phase_noise output expression references unbound node {node}."
                ));
            }
        }
        return Ok(());
    }
    if lower.starts_with("i(") && expression.ends_with(')') {
        let component = expression[2..expression.len() - 1].trim();
        if component.is_empty() {
            return Err("analog_phase_noise current expression requires a component.".to_string());
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
                "analog_phase_noise output expression references unbound component {component}."
            ));
        }
        return Ok(());
    }
    Err(
        "analog_phase_noise output expression must be V(node), V(node,reference), or I(source)."
            .to_string(),
    )
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
