use crate::board_ir::{AnalogNetlistSource, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_DISTORTION_ANALYSIS;
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_distortion_assertions::{
    evaluate_distortion_assertions, validate_distortion_assertion_contract,
};
use super::analog_distortion_runner::{NgspiceDistortionRunOptions, run_ngspice_distortion};
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

pub(super) struct AnalogDistortionSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_distortion_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogDistortionSinks<'_>,
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
        "Preparing distortion analysis",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion scenario requires an analog block.",
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
                    "SPICE model file {} is required for physical distortion analysis.",
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
            "analog_distortion requires node_bindings and pin_bindings.",
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
                    "Analog distortion node binding {} references unknown board net {}.",
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
                    "Analog distortion pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "disto" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only disto is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let mode = analog
        .analysis
        .distortion_mode
        .as_deref()
        .unwrap_or("harmonic");
    if !matches!(mode, "harmonic" | "intermodulation") {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion requires distortion_mode to be harmonic or intermodulation when provided.",
        );
        return;
    }
    let Some(start_frequency_hz) = analog.analysis.distortion_start_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion requires distortion_start_frequency_hz.",
        );
        return;
    };
    let Some(stop_frequency_hz) = analog.analysis.distortion_stop_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion requires distortion_stop_frequency_hz.",
        );
        return;
    };
    if !start_frequency_hz.is_finite() || start_frequency_hz <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion requires positive finite distortion_start_frequency_hz.",
        );
        return;
    }
    if !stop_frequency_hz.is_finite() || stop_frequency_hz <= start_frequency_hz {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion requires distortion_stop_frequency_hz greater than distortion_start_frequency_hz.",
        );
        return;
    }
    let Some(points_per_decade) = analog.analysis.distortion_points_per_decade else {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion requires distortion_points_per_decade.",
        );
        return;
    };
    if !(1..=1000).contains(&points_per_decade) {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion requires distortion_points_per_decade in 1..=1000.",
        );
        return;
    }
    let Some(output_expression) = nonempty(analog.analysis.distortion_output_expression.as_deref())
    else {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion requires distortion_output_expression.",
        );
        return;
    };
    if let Err(message) = validate_output_expression(output_expression, &bound_nodes, scenario) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if analog.analysis.distortion_f1_sources.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion requires at least one distortion_f1_sources entry.",
        );
        return;
    }
    if let Err(message) = validate_sources(
        "distortion_f1_sources",
        &analog.analysis.distortion_f1_sources,
        &analog.netlist_source,
        bound,
    ) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if let Err(message) = validate_sources(
        "distortion_f2_sources",
        &analog.analysis.distortion_f2_sources,
        &analog.netlist_source,
        bound,
    ) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    match (mode, analog.analysis.distortion_f2_over_f1) {
        ("harmonic", None) => {}
        ("harmonic", Some(_)) => {
            validation_input_missing(
                findings,
                scenario,
                "analog_distortion harmonic mode must not set distortion_f2_over_f1.",
            );
            return;
        }
        ("intermodulation", Some(ratio)) if ratio.is_finite() && ratio > 0.0 && ratio < 1.0 => {
            if analog.analysis.distortion_f2_sources.is_empty() {
                validation_input_missing(
                    findings,
                    scenario,
                    "analog_distortion intermodulation mode requires distortion_f2_sources.",
                );
                return;
            }
        }
        ("intermodulation", _) => {
            validation_input_missing(
                findings,
                scenario,
                "analog_distortion intermodulation mode requires finite distortion_f2_over_f1 in 0..1.",
            );
            return;
        }
        _ => {}
    }
    if let Err(message) = validate_distortion_assertion_contract(analog) {
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
    if run_plans.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_distortion requires at least one run plan.",
        );
        return;
    }

    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Err(error) = fs::create_dir_all(&run_dir) {
        findings.push(Finding::critical(
            SPICE_DISTORTION_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create distortion planning directory {}: {error}",
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
            let mut finding = Finding::critical(SPICE_DISTORTION_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    };

    on_progress(
        "Planning distortion backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::Distortion);
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

    if backend == "ngspice" {
        let mut sweep_measurements = Vec::new();
        for run_plan in run_plans {
            if should_cancel() {
                push_canceled_finding(findings, scenario);
                return;
            }
            on_progress("Running distortion input corner", run_plan.progress_label());
            let parameter_overrides = run_plan.parameter_overrides_for_solver();
            let run_result = run_ngspice_distortion(
                bound,
                scenario,
                backend,
                &source_netlist,
                NgspiceDistortionRunOptions {
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
                    push_artifact(artifacts, &run.spectrum);
                    push_artifact(artifacts, &run.summary);
                    push_artifact(artifacts, &run.convergence);
                    let assertion_measurements =
                        evaluate_distortion_assertions(scenario, &run.summary, findings);
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
                        Finding::critical(SPICE_DISTORTION_ANALYSIS, &scenario.name, error.message);
                    finding
                        .measured
                        .insert("selected_backend".to_string(), json!(backend));
                    finding.limit.insert(
                        "required_evidence".to_string(),
                        json!("ngspice_distortion_spectrum_and_summary_csv"),
                    );
                    tag_corner_finding(&mut finding, &run_plan);
                    finding.suggested_fixes.push(
                        "Inspect the generated ngspice .DISTO wrapper deck and solver log artifacts."
                            .to_string(),
                    );
                    findings.push(finding);
                }
            }
        }
        push_sweep_margin_summaries(findings, scenario, &sweep_measurements);
        return;
    }

    let mut finding = unsupported_backend_plan_finding(
        scenario,
        UnsupportedBackendPlan {
            check_id: SPICE_DISTORTION_ANALYSIS,
            selected_backend: backend,
            implemented_backend: "ngspice",
            analysis_kind: "distortion",
            required_normalized_outputs: &[
                "distortion_spectrum",
                "distortion_summary",
                "distortion_convergence",
            ],
        },
    );
    finding
        .measured
        .insert("distortion_mode".to_string(), json!(mode));
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
    finding.measured.insert(
        "f1_sources".to_string(),
        json!(analog.analysis.distortion_f1_sources),
    );
    finding.measured.insert(
        "f2_sources".to_string(),
        json!(analog.analysis.distortion_f2_sources),
    );
    finding.measured.insert(
        "f2_over_f1".to_string(),
        json!(analog.analysis.distortion_f2_over_f1),
    );
    finding.measured.insert(
        "backend_research_status".to_string(),
        json!({
            "ngspice": "manual_and_source_document_disto_plots_and_circuitci_ngspice_adapter_is_enabled",
            "xyce": "reference_guide_search_found_no_disto_or_distortion_analysis_command",
            "qucs_s": "homepage_lists_disto_support_via_spice_backends_but_not_a_distinct_adapter_contract",
            "spice_opus": "release_notes_document_distortion_parameters_but_no_circuitci_adapter_or_conformance_contract"
        }),
    );
    finding.measured.insert(
        "source_notes".to_string(),
        json!({
            "ngspice_manual": "sources/ngspice_manual.xhtml",
            "ngspice_disto_manual_page": "sources/ngspice_manual_disto_distortionanalysis.html",
            "ngspice_disto_source": "sources/ngspice_source_distoan.c.gz",
            "ngspice_analyses": "sources/ngspice_ANALYSES",
            "xyce_reference": "sources/Xyce_Reference_Guide_7.8.txt",
            "qucs_s_home": "sources/qucs_s_home.html",
            "spiceopus_release": "sources/spiceopus_release.html"
        }),
    );
    finding.limit.insert(
        "required_evidence".to_string(),
        json!("distortion_spectrum_distortion_summary_distortion_convergence_solver_manifest"),
    );
    finding.limit.insert(
        "current_limitation".to_string(),
        json!("CircuitCI enables ngspice .DISTO only; other backends do not have a trusted distortion adapter."),
    );
    finding.suggested_fixes.push(
        "Use backend: ngspice for .DISTO or keep this scenario blocked until the selected backend emits normalized distortion spectrum, summary, raw-output, and solver-manifest artifacts with conformance coverage."
            .to_string(),
    );
    findings.push(finding);
}

fn validate_sources(
    field: &str,
    sources: &[String],
    netlist_source: &AnalogNetlistSource,
    bound: &BoundBoard<'_>,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for source in sources {
        let source = source.trim();
        if source.is_empty() {
            return Err(format!(
                "analog_distortion {field} entries must be non-empty."
            ));
        }
        if !seen.insert(source) {
            return Err(format!(
                "analog_distortion declares duplicate {field} entry {source}."
            ));
        }
        if netlist_source == &AnalogNetlistSource::GeneratedFromBoard
            && !bound.project.board.components.contains_key(source)
        {
            return Err(format!(
                "analog_distortion {field} entry {source} is not a generated board component."
            ));
        }
    }
    Ok(())
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
                    "analog_distortion output expression references unbound node {node}."
                ));
            }
        }
        return Ok(());
    }
    if lower.starts_with("i(") && expression.ends_with(')') {
        let component = expression[2..expression.len() - 1].trim();
        if component.is_empty() {
            return Err("analog_distortion current expression requires a component.".to_string());
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
                "analog_distortion output expression references unbound component {component}."
            ));
        }
        return Ok(());
    }
    Err(
        "analog_distortion output expression must be V(node), V(node,reference), or I(source)."
            .to_string(),
    )
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
