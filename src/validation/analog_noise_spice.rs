use crate::board_ir::{AnalogRelation, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_NOISE_ANALYSIS;
use super::analog_assertions::validate_probe_contract;
use super::analog_noise_assertions::{
    evaluate_noise_assertions, validate_noise_assertion_contract,
};
use super::analog_noise_runner::{NgspiceNoiseRunOptions, run_ngspice_noise};
use super::analog_runner::{
    AnalogRuntimeFeature, BackendSelection, backend_name, embedded_solver_unavailable,
    external_backend_unavailable, select_backend_for_feature,
};
use super::analog_spice::{
    analog_run_plans, prepare_source_netlist, push_canceled_finding, validate_netlist_source,
};
use super::analog_sweep_reports::{
    monte_carlo_criteria_enabled, push_sweep_margin_summaries, record_sweep_measurements,
    tag_corner_finding, tag_corner_findings,
};
use super::analog_util::{file_sha256_hex, push_artifact, safe_artifact_name};
use super::common::validation_input_missing;

pub(super) struct AnalogNoiseSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
    pub(super) waveforms: &'a mut Vec<String>,
}

pub(super) fn validate_spice_noise_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogNoiseSinks<'_>,
    output: &Path,
    mut on_progress: F,
    should_cancel: C,
) where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let findings = &mut *sinks.findings;
    let artifacts = &mut *sinks.artifacts;
    let waveforms = &mut *sinks.waveforms;
    on_progress(
        "Preparing analog noise observation",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_noise scenario requires an analog block.",
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
                    "SPICE model file {} is required for physical analog noise simulation.",
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
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }

    if analog.node_bindings.is_empty() || analog.pin_bindings.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_noise requires node_bindings and pin_bindings.",
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
                    "Analog noise node binding {} references unknown board net {}.",
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
                    "Analog noise pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "noise" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only noise is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let Some(start_hz) = analog.analysis.start_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_noise requires start_frequency_hz.",
        );
        return;
    };
    let Some(stop_hz) = analog.analysis.stop_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog_noise requires stop_frequency_hz.",
        );
        return;
    };
    if !start_hz.is_finite() || !stop_hz.is_finite() || start_hz <= 0.0 || stop_hz <= start_hz {
        validation_input_missing(
            findings,
            scenario,
            "analog_noise requires 0 < start_frequency_hz < stop_frequency_hz.",
        );
        return;
    }
    if !matches!(analog.analysis.points_per_decade, Some(1..=1000)) {
        validation_input_missing(
            findings,
            scenario,
            "analog_noise requires points_per_decade in 1..=1000.",
        );
        return;
    }
    let Some(output_node) = analog.analysis.noise_output_node.as_deref() else {
        validation_input_missing(
            findings,
            scenario,
            "analog_noise requires noise_output_node.",
        );
        return;
    };
    if !bound_nodes.contains(output_node) {
        validation_input_missing(
            findings,
            scenario,
            format!("analog_noise output node {output_node} is not bound to a board net."),
        );
        return;
    }
    if let Some(reference_node) = analog.analysis.noise_reference_node.as_deref()
        && !bound_nodes.contains(reference_node)
    {
        validation_input_missing(
            findings,
            scenario,
            format!("analog_noise reference node {reference_node} is not bound to a board net."),
        );
        return;
    }
    if analog
        .analysis
        .noise_input_source
        .as_deref()
        .is_none_or(str::is_empty)
    {
        validation_input_missing(
            findings,
            scenario,
            "analog_noise requires noise_input_source.",
        );
        return;
    }
    if analog.probes.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SPICE_NOISE_ANALYSIS requires at least one noise probe alias.",
        );
        return;
    }
    for probe in &analog.probes {
        if let Err(message) = validate_probe_contract(probe) {
            validation_input_missing(
                findings,
                scenario,
                format!("Analog noise probe {} {message}.", probe.name),
            );
            return;
        }
    }
    for assertion in &analog.assertions {
        if !analog
            .probes
            .iter()
            .any(|probe| probe.name == assertion.probe)
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog noise assertion {} references unknown probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            return;
        }
        if let Err(message) = validate_noise_assertion_contract(assertion, start_hz, stop_hz) {
            validation_input_missing(
                findings,
                scenario,
                format!("Analog noise assertion {} {message}.", assertion.name),
            );
            return;
        }
        match assertion.relation {
            AnalogRelation::Below | AnalogRelation::Above => {}
        }
    }
    if analog.assertions.is_empty() {
        let mut finding = Finding::info(
            "ANALOG_ASSERTIONS_ABSENT",
            &scenario.name,
            "SPICE noise analysis exported output/input noise, but no quantitative noise assertions were declared.",
        );
        finding.limit.insert(
            "required_for_signoff".to_string(),
            json!("Add noise-density or integrated RMS-noise assertions for the sensitive analog path being verified."),
        );
        findings.push(finding);
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
            SPICE_NOISE_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create analog noise run directory {}: {error}",
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
            let mut finding = Finding::critical(SPICE_NOISE_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    };

    on_progress(
        "Selecting analog noise backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::Noise);
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
        let mut finding = Finding::critical(
            SPICE_NOISE_ANALYSIS,
            &scenario.name,
            format!(
                "Backend {backend} was detected, but noise analysis export is currently implemented for external ngspice."
            ),
        );
        finding
            .measured
            .insert("selected_backend".to_string(), json!(backend));
        finding
            .limit
            .insert("implemented_backend".to_string(), json!("ngspice"));
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
            "Running analog noise input corner",
            run_plan.progress_label(),
        );
        let parameter_overrides = run_plan.parameter_overrides_for_solver();
        match run_ngspice_noise(
            bound,
            scenario,
            backend,
            &source_netlist,
            NgspiceNoiseRunOptions {
                output,
                run_subdir: run_plan.run_subdir.as_deref(),
                parameter_overrides: &parameter_overrides,
                model_section_overrides: &run_plan.model_section_overrides,
                on_progress: &mut on_progress,
                should_cancel: &should_cancel,
            },
        ) {
            Ok(run) => {
                let finding_start = findings.len();
                for artifact in &run.artifacts {
                    push_artifact(artifacts, artifact);
                }
                push_artifact(waveforms, &run.noise_spectrum);
                let assertion_measurements = evaluate_noise_assertions(
                    scenario,
                    &run.noise_spectrum,
                    &run.noise_total,
                    findings,
                );
                record_sweep_measurements(
                    &mut sweep_measurements,
                    &run_plan,
                    assertion_measurements,
                );
                tag_corner_findings(
                    findings,
                    finding_start,
                    &run_plan,
                    monte_carlo_criteria_enabled(scenario, &run_plan),
                );
            }
            Err(error) => {
                for artifact in &error.artifacts {
                    push_artifact(artifacts, artifact);
                }
                let mut finding =
                    Finding::critical(SPICE_NOISE_ANALYSIS, &scenario.name, error.message);
                finding
                    .measured
                    .insert("selected_backend".to_string(), json!(backend));
                finding
                    .limit
                    .insert("required_evidence".to_string(), json!("ngspice_noise_csv"));
                tag_corner_finding(&mut finding, &run_plan);
                finding.suggested_fixes.push(
                    "Inspect the generated ngspice noise wrapper deck and solver log artifacts."
                        .to_string(),
                );
                findings.push(finding);
            }
        }
    }
    push_sweep_margin_summaries(findings, scenario, &sweep_measurements);
}
