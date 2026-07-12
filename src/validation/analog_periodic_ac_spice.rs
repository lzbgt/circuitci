use crate::board_ir::{AnalogNetlistSource, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_PERIODIC_AC_ANALYSIS;
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_runner::{
    AnalogRuntimeFeature, BackendSelection, backend_name, embedded_solver_unavailable,
    external_backend_unavailable, select_backend_for_feature,
};
use super::analog_spice::{
    analog_run_plans, prepare_source_netlist, push_canceled_finding, validate_netlist_source,
};
use super::analog_util::{push_artifact, safe_artifact_name};
use super::common::validation_input_missing;

pub(super) struct AnalogPeriodicAcSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_periodic_ac_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogPeriodicAcSinks<'_>,
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
        "Preparing periodic AC analysis",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac scenario requires an analog block.",
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
            "analog_periodic_ac requires node_bindings and pin_bindings.",
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
                    "Analog periodic AC node binding {} references unknown board net {}.",
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
                    "Analog periodic AC pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "pac" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only pac is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let mode = analog.analysis.pac_mode.as_deref().unwrap_or("pac");
    if !matches!(mode, "pac" | "pxf") {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires pac_mode to be pac or pxf when provided.",
        );
        return;
    }
    let Some(carrier_frequency_hz) = analog.analysis.pac_carrier_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires pac_carrier_frequency_hz.",
        );
        return;
    };
    if !carrier_frequency_hz.is_finite() || carrier_frequency_hz <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires positive finite pac_carrier_frequency_hz.",
        );
        return;
    }
    let Some(start_frequency_hz) = analog.analysis.pac_start_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires pac_start_frequency_hz.",
        );
        return;
    };
    let Some(stop_frequency_hz) = analog.analysis.pac_stop_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires pac_stop_frequency_hz.",
        );
        return;
    };
    if !start_frequency_hz.is_finite() || start_frequency_hz <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires positive finite pac_start_frequency_hz.",
        );
        return;
    }
    if !stop_frequency_hz.is_finite() || stop_frequency_hz <= start_frequency_hz {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires pac_stop_frequency_hz greater than pac_start_frequency_hz.",
        );
        return;
    }
    let Some(points_per_decade) = analog.analysis.pac_points_per_decade else {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires pac_points_per_decade.",
        );
        return;
    };
    if !(1..=1000).contains(&points_per_decade) {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires pac_points_per_decade in 1..=1000.",
        );
        return;
    }
    let sidebands = analog.analysis.pac_sidebands.unwrap_or(1);
    if sidebands > 1024 {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires pac_sidebands in 0..=1024 when provided.",
        );
        return;
    }
    let Some(output_expression) = nonempty(analog.analysis.pac_output_expression.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires pac_output_expression.",
        );
        return;
    };
    if let Err(message) = validate_output_expression(output_expression, &bound_nodes, scenario) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    let Some(input_source) = nonempty(analog.analysis.pac_input_source.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_periodic_ac requires pac_input_source.",
        );
        return;
    };
    if analog.netlist_source == AnalogNetlistSource::GeneratedFromBoard
        && !bound.project.board.components.contains_key(input_source)
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "analog_periodic_ac pac_input_source {input_source} is not a generated board component."
            ),
        );
        return;
    }

    let mut drive_sources = BTreeSet::new();
    for source in &analog.analysis.pac_drive_sources {
        let source = source.trim();
        if source.is_empty() {
            validation_input_missing(
                findings,
                scenario,
                "analog_periodic_ac pac_drive_sources entries must be non-empty.",
            );
            return;
        }
        if !drive_sources.insert(source) {
            validation_input_missing(
                findings,
                scenario,
                format!("analog_periodic_ac declares duplicate drive source {source}."),
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
                    "analog_periodic_ac drive source {source} is not a generated board component."
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
            "analog_periodic_ac requires at least one run plan.",
        );
        return;
    }

    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Err(error) = fs::create_dir_all(&run_dir) {
        findings.push(Finding::critical(
            SPICE_PERIODIC_AC_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create periodic AC planning directory {}: {error}",
                run_dir.display()
            ),
        ));
        return;
    }
    match prepare_source_netlist(bound, scenario, &run_dir) {
        Ok(source_netlist) => push_artifact(artifacts, &source_netlist),
        Err(message) => {
            let mut finding =
                Finding::critical(SPICE_PERIODIC_AC_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    }

    on_progress(
        "Planning periodic AC backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::PeriodicAc);
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
            check_id: SPICE_PERIODIC_AC_ANALYSIS,
            selected_backend: backend,
            implemented_backend: "none_yet",
            analysis_kind: "periodic_ac",
            required_normalized_outputs: &[
                "pac_response",
                "pac_sidebands",
                "pac_convergence",
                "pss_convergence",
            ],
        },
    );
    finding.measured.insert("pac_mode".to_string(), json!(mode));
    finding.measured.insert(
        "carrier_frequency_hz".to_string(),
        json!(carrier_frequency_hz),
    );
    finding
        .measured
        .insert("start_frequency_hz".to_string(), json!(start_frequency_hz));
    finding
        .measured
        .insert("stop_frequency_hz".to_string(), json!(stop_frequency_hz));
    finding
        .measured
        .insert("points_per_decade".to_string(), json!(points_per_decade));
    finding
        .measured
        .insert("output_expression".to_string(), json!(output_expression));
    finding
        .measured
        .insert("input_source".to_string(), json!(input_source));
    finding
        .measured
        .insert("sidebands".to_string(), json!(sidebands));
    finding.measured.insert(
        "drive_sources".to_string(),
        json!(analog.analysis.pac_drive_sources),
    );
    finding.measured.insert(
        "backend_research_status".to_string(),
        json!({
            "xyce": "reference_guide_and_genext_note_document_hb_but_no_pac_or_pxf_command_or_output_contract",
            "ngspice": "manual_mentions_pac_as_future_pss_downstream_analysis_but_pss_is_experimental_autonomous_only_and_no_pac_command_contract_is_documented",
            "qucs_copen": "papers_document_periodic_noise_flow_after_psssolver_but_no_public_source_repository_or_adapter_contract_found",
            "qucsator_rf": "qucs_technical_material_documents_hb_topics_but_no_usable_circuitci_pac_pxf_backend_contract",
            "commercial_reference": "spectre_like_pac_pxf_requires_periodic_operating_point_convergence_and_sideband_response_artifacts"
        }),
    );
    finding.measured.insert(
        "source_notes".to_string(),
        json!({
            "xyce_reference": "sources/Xyce_Reference_Guide_7.8.txt",
            "xyce_genext": "sources/Xyce_AppNote_GenExt.txt",
            "ngspice_manual": "sources/ngspice_manual.xhtml",
            "ngspice_pss_page": "sources/ngspice_pss_periodic_steady_state.html",
            "qucs_copen_part2": "sources/arxiv_2603.07828_qucs_phase_noise_part2.txt",
            "qucs_technical": "sources/qucs_technical.html"
        }),
    );
    finding.measured.insert(
        "adapter_blocker".to_string(),
        json!("No trusted open-source PAC/PXF backend path with periodic operating-point convergence, sideband response, raw-output, and solver-manifest artifacts is available in this runtime."),
    );
    finding.measured.insert(
        "evidence_sources".to_string(),
        json!([
            "docs/research/circuit_simulation_full_featured/periodic_ac_backend_evidence.md",
            "docs/research/circuit_simulation_full_featured/sources/Xyce_Reference_Guide_7.8.txt",
            "docs/research/circuit_simulation_full_featured/sources/Xyce_AppNote_GenExt.txt",
            "docs/research/circuit_simulation_full_featured/sources/ngspice_manual.xhtml"
        ]),
    );
    finding.limit.insert(
        "required_evidence".to_string(),
        json!("pac_response_pac_sidebands_pac_convergence_pss_convergence"),
    );
    finding.limit.insert(
        "current_limitation".to_string(),
        json!("CircuitCI records PAC/PXF intent and sideband-sweep requirements but has no trusted periodic operating-point linearization solver chain or normalized output contract yet."),
    );
    finding.limit.insert(
        "trusted_backend_status".to_string(),
        json!("none_available"),
    );
    finding.suggested_fixes.push(
        "Keep periodic small-signal sign-off blocked until a backend emits normalized PAC/PXF response, sideband, periodic-convergence, raw solver output, and solver-manifest artifacts with real-solver conformance coverage."
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
                    "analog_periodic_ac output expression references unbound node {node}."
                ));
            }
        }
        return Ok(());
    }
    if lower.starts_with("i(") && expression.ends_with(')') {
        let component = expression[2..expression.len() - 1].trim();
        if component.is_empty() {
            return Err("analog_periodic_ac current expression requires a component.".to_string());
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
                "analog_periodic_ac output expression references unbound component {component}."
            ));
        }
        return Ok(());
    }
    Err(
        "analog_periodic_ac output expression must be V(node), V(node,reference), or I(source)."
            .to_string(),
    )
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
