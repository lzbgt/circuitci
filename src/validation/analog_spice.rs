use crate::board_ir::{AnalogAggregation, AnalogRelation, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::analog_ac_assertions::{evaluate_ac_assertions, validate_ac_assertion_contract};
use super::analog_assertions::{
    assertion_reference_contract_is_complete, evaluate_waveform_assertions,
    validate_assertion_contract, validate_probe_contract,
};
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_operating_limits::{
    evaluate_operating_limits, operating_limit_probes, operating_probe_expressions,
};
use super::analog_runner::{
    AnalogRuntimeFeature, BackendSelection, NgspiceAcRunOptions, NgspiceRunOptions, backend_name,
    embedded_solver_unavailable, external_backend_unavailable, normalized_frequency_sweep_type,
    run_ngspice, run_ngspice_ac, select_backend_for_feature,
};
use super::analog_soa::evaluate_soa_limits;
pub(super) use super::analog_spice_run_plan::{
    AnalogRunPlan, ComponentValueOverride, analog_run_plans, netlist_source_name,
    prepare_source_netlist, validate_netlist_source,
};
use super::analog_sweep_reports::{
    monte_carlo_criteria_enabled, push_sweep_margin_summaries, record_sweep_measurements,
    tag_corner_finding, tag_corner_findings,
};
use super::analog_util::{push_artifact, safe_artifact_name};
use super::analog_xyce_runner::{
    XyceAcRunOptions, XyceTransientRunOptions, run_xyce_ac, run_xyce_transient,
};
use super::common::validation_input_missing;
use super::{SPICE_AC_ANALYSIS, SPICE_TRANSIENT_ANALYSIS};

pub(super) struct AnalogTransientSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
    pub(super) waveforms: &'a mut Vec<String>,
}

pub(super) struct AnalogAcSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
    pub(super) waveforms: &'a mut Vec<String>,
}

pub(super) fn validate_spice_transient_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogTransientSinks<'_>,
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
        "Preparing analog transient",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_transient scenario requires an analog block.",
        );
        return;
    };
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }
    on_progress(
        "Checking analog model evidence",
        format!(
            "{} model file(s), {} node binding(s), {} pin binding(s).",
            analog.model_files.len(),
            analog.node_bindings.len(),
            analog.pin_bindings.len()
        ),
    );
    if let Some(finding) = validate_netlist_source(bound, scenario, artifacts) {
        findings.push(finding);
        return;
    }

    if analog.node_bindings.is_empty() || analog.pin_bindings.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_transient requires node_bindings and pin_bindings.",
        );
        return;
    }
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }
    let mut bound_nodes = BTreeSet::new();
    for binding in &analog.node_bindings {
        if !bound.project.board.nets.contains_key(&binding.net) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog node binding {} references unknown board net {}.",
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
                    "Analog pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
        let Some(component) = bound
            .project
            .board
            .components
            .get(&binding.endpoint.component)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog pin binding references unknown component {}.",
                    binding.endpoint.component
                ),
            );
            return;
        };
        let Some(pin_net) = component.pins.get(&binding.endpoint.pin) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog pin binding references unknown pin {}.{}.",
                    binding.endpoint.component, binding.endpoint.pin
                ),
            );
            return;
        };
        if !analog
            .node_bindings
            .iter()
            .any(|node| node.node == binding.node && node.net == *pin_net)
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog pin binding {}.{} maps to node {}, but the board pin is on net {}.",
                    binding.endpoint.component, binding.endpoint.pin, binding.node, pin_net
                ),
            );
            return;
        }
    }
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }

    if analog.analysis.analysis_type != "tran" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only tran is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    if !analog.analysis.stop_time_us.is_finite()
        || !analog.analysis.max_step_us.is_finite()
        || analog.analysis.stop_time_us <= 0.0
        || analog.analysis.max_step_us <= 0.0
        || analog.analysis.max_step_us > analog.analysis.stop_time_us
    {
        validation_input_missing(
            findings,
            scenario,
            "analog.analysis stop_time_us and max_step_us must be finite, positive, and max_step_us must not exceed stop_time_us.",
        );
        return;
    }
    if analog.probes.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SPICE_TRANSIENT_ANALYSIS requires at least one waveform probe.",
        );
        return;
    }
    for probe in &analog.probes {
        if let Err(message) = validate_probe_contract(probe) {
            validation_input_missing(
                findings,
                scenario,
                format!("Analog probe {} {message}.", probe.name),
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
                    "Analog assertion {} references unknown probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            return;
        }
        if let Err(message) = validate_assertion_contract(assertion, analog.analysis.stop_time_us) {
            validation_input_missing(
                findings,
                scenario,
                format!("Analog assertion {} {message}.", assertion.name),
            );
            return;
        }
        if matches!(
            assertion.aggregation,
            AnalogAggregation::RisingPhaseDelay
                | AnalogAggregation::FallingPhaseDelay
                | AnalogAggregation::RisingSetupTime
                | AnalogAggregation::RisingHoldTime
                | AnalogAggregation::FallingSetupTime
                | AnalogAggregation::FallingHoldTime
        ) {
            let Some(reference_probe) = assertion.reference_probe.as_deref() else {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "Analog assertion {} is missing a reference probe.",
                        assertion.name
                    ),
                );
                return;
            };
            if analog
                .probes
                .iter()
                .all(|probe| probe.name != reference_probe)
            {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "Analog assertion {} references unknown reference probe {}.",
                        assertion.name, reference_probe
                    ),
                );
                return;
            }
        }
        let probe = analog
            .probes
            .iter()
            .find(|probe| probe.name == assertion.probe)
            .expect("probe existence was checked above");
        if !assertion_reference_contract_is_complete(assertion, probe) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog assertion {} is missing finite reference fields for probe {}.",
                    assertion.name, probe.name
                ),
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
            "SPICE transient solved and exported probes, but no quantitative waveform assertions were declared.",
        );
        finding.limit.insert(
            "required_for_signoff".to_string(),
            json!("Add voltage/current/power assertions for the board behavior being verified."),
        );
        findings.push(finding);
    }
    if should_cancel() {
        push_canceled_finding(findings, scenario);
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
    on_progress(
        "Preparing analog run directory",
        format!("Creating {}.", run_dir.to_string_lossy()),
    );
    if let Err(error) = fs::create_dir_all(&run_dir) {
        findings.push(Finding::critical(
            SPICE_TRANSIENT_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create analog run directory {}: {error}",
                run_dir.display()
            ),
        ));
        return;
    }
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }
    on_progress(
        "Preparing analog deck",
        format!(
            "Resolving {} netlist source.",
            netlist_source_name(&analog.netlist_source)
        ),
    );
    let source_netlist = match prepare_source_netlist(bound, scenario, &run_dir) {
        Ok(source_netlist) => {
            push_artifact(artifacts, &source_netlist);
            source_netlist
        }
        Err(message) => {
            let mut finding = Finding::critical(SPICE_TRANSIENT_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            finding.suggested_fixes.push(
                "Fix the generated Board IR to SPICE contract before selecting a solver backend."
                    .to_string(),
            );
            findings.push(finding);
            return;
        }
    };
    on_progress(
        "Preparing analog operating probes",
        format!("Collecting operating-limit probes for {}.", scenario.name),
    );
    let operating_limits = operating_limit_probes(bound, scenario);
    if !operating_limits.metadata_findings.is_empty() {
        findings.extend(operating_limits.metadata_findings);
        return;
    }

    on_progress(
        "Selecting analog backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::Transient);
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

    if !matches!(backend, "ngspice" | "embedded_ngspice" | "Xyce" | "xyce") {
        findings.push(unsupported_backend_plan_finding(
            scenario,
            UnsupportedBackendPlan {
                check_id: SPICE_TRANSIENT_ANALYSIS,
                selected_backend: backend,
                implemented_backend: "ngspice_or_embedded_ngspice",
                analysis_kind: "transient",
                required_normalized_outputs: &["transient_waveform"],
            },
        ));
        return;
    }

    let operating_expressions = operating_probe_expressions(&operating_limits);
    let mut sweep_measurements = Vec::new();
    for run_plan in run_plans {
        if should_cancel() {
            push_canceled_finding(findings, scenario);
            return;
        }
        on_progress("Running analog input corner", run_plan.progress_label());
        let parameter_overrides = run_plan.parameter_overrides_for_solver();
        let run_result = if matches!(backend, "Xyce" | "xyce") {
            run_xyce_transient(
                bound,
                scenario,
                backend,
                &source_netlist,
                XyceTransientRunOptions {
                    output,
                    run_subdir: run_plan.run_subdir.as_deref(),
                    parameter_overrides: &parameter_overrides,
                    model_section_overrides: &run_plan.model_section_overrides,
                    operating_probe_expressions: &operating_expressions,
                    on_progress: &mut on_progress,
                    should_cancel: &should_cancel,
                },
            )
        } else {
            run_ngspice(
                bound,
                scenario,
                backend,
                &source_netlist,
                NgspiceRunOptions {
                    output,
                    run_subdir: run_plan.run_subdir.as_deref(),
                    parameter_overrides: &parameter_overrides,
                    model_section_overrides: &run_plan.model_section_overrides,
                    operating_probe_expressions: &operating_expressions,
                    on_progress: &mut on_progress,
                    should_cancel: &should_cancel,
                },
            )
        };
        match run_result {
            Ok(run) => {
                let finding_start = findings.len();
                on_progress(
                    "Recording analog artifacts",
                    format!(
                        "{} artifact(s), waveform {}.",
                        run.artifacts.len(),
                        run.waveform.to_string_lossy()
                    ),
                );
                for artifact in &run.artifacts {
                    push_artifact(artifacts, artifact);
                }
                push_artifact(waveforms, &run.waveform);
                on_progress(
                    "Evaluating analog assertions",
                    format!(
                        "{} user probe(s), {} assertion(s).",
                        run.user_probe_count,
                        analog.assertions.len()
                    ),
                );
                let assertion_measurements = evaluate_waveform_assertions(scenario, &run, findings);
                record_sweep_measurements(
                    &mut sweep_measurements,
                    &run_plan,
                    assertion_measurements,
                );
                on_progress(
                    "Evaluating analog limits",
                    format!("{} operating probe(s).", operating_limits.probes.len()),
                );
                evaluate_operating_limits(scenario, &run, &operating_limits.probes, findings);
                evaluate_soa_limits(scenario, &run, &operating_limits, findings);
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
                    Finding::critical(SPICE_TRANSIENT_ANALYSIS, &scenario.name, error.message);
                finding
                    .measured
                    .insert("selected_backend".to_string(), json!(backend));
                finding.limit.insert(
                    "required_evidence".to_string(),
                    json!(if matches!(backend, "Xyce" | "xyce") {
                        "xyce_transient_waveform_csv"
                    } else {
                        "ngspice_waveform_csv"
                    }),
                );
                tag_corner_finding(&mut finding, &run_plan);
                let solver_name = if matches!(backend, "Xyce" | "xyce") {
                    "Xyce"
                } else {
                    "ngspice"
                };
                finding.suggested_fixes.push(format!(
                    "Inspect the generated {solver_name} wrapper deck and solver log artifacts."
                ));
                findings.push(finding);
            }
        }
    }
    push_sweep_margin_summaries(findings, scenario, &sweep_measurements);
}

pub(super) fn validate_spice_ac_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogAcSinks<'_>,
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
        "Preparing analog AC sweep",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_ac scenario requires an analog block.",
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
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }

    if analog.node_bindings.is_empty() || analog.pin_bindings.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_ac requires node_bindings and pin_bindings.",
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
                    "Analog AC node binding {} references unknown board net {}.",
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
                    "Analog AC pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "ac" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only ac is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let Some(start_hz) = analog.analysis.start_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog.analysis.start_frequency_hz is required for analog_ac.",
        );
        return;
    };
    let Some(stop_hz) = analog.analysis.stop_frequency_hz else {
        validation_input_missing(
            findings,
            scenario,
            "analog.analysis.stop_frequency_hz is required for analog_ac.",
        );
        return;
    };
    let points_per_decade = analog.analysis.points_per_decade.unwrap_or(20);
    if !start_hz.is_finite()
        || !stop_hz.is_finite()
        || start_hz <= 0.0
        || stop_hz <= start_hz
        || points_per_decade == 0
        || points_per_decade > 1000
    {
        validation_input_missing(
            findings,
            scenario,
            "analog_ac frequency sweep requires finite positive start/stop frequencies, stop_frequency_hz greater than start_frequency_hz, and points_per_decade in 1..=1000.",
        );
        return;
    }
    if let Err(message) = normalized_frequency_sweep_type(analog.analysis.sweep_type.as_deref()) {
        validation_input_missing(findings, scenario, format!("analog_ac {message}"));
        return;
    }
    if analog.probes.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SPICE_AC_ANALYSIS requires at least one AC response probe.",
        );
        return;
    }
    for probe in &analog.probes {
        if let Err(message) = validate_probe_contract(probe) {
            validation_input_missing(
                findings,
                scenario,
                format!("Analog AC probe {} {message}.", probe.name),
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
                    "Analog AC assertion {} references unknown probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            return;
        }
        if let Err(message) = validate_ac_assertion_contract(assertion, start_hz, stop_hz) {
            validation_input_missing(
                findings,
                scenario,
                format!("Analog AC assertion {} {message}.", assertion.name),
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

    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Err(error) = fs::create_dir_all(&run_dir) {
        findings.push(Finding::critical(
            SPICE_AC_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create analog AC run directory {}: {error}",
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
            let mut finding = Finding::critical(SPICE_AC_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    };

    on_progress(
        "Selecting analog AC backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::Ac);
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
    if !matches!(backend, "ngspice" | "Xyce" | "xyce") {
        findings.push(unsupported_backend_plan_finding(
            scenario,
            UnsupportedBackendPlan {
                check_id: SPICE_AC_ANALYSIS,
                selected_backend: backend,
                implemented_backend: "ngspice_or_xyce",
                analysis_kind: "ac",
                required_normalized_outputs: &["ac_bode"],
            },
        ));
        return;
    }

    let mut sweep_measurements = Vec::new();
    for run_plan in run_plans {
        if should_cancel() {
            push_canceled_finding(findings, scenario);
            return;
        }
        on_progress("Running analog AC input corner", run_plan.progress_label());
        let parameter_overrides = run_plan.parameter_overrides_for_solver();
        let run_result = if matches!(backend, "Xyce" | "xyce") {
            run_xyce_ac(
                bound,
                scenario,
                backend,
                &source_netlist,
                XyceAcRunOptions {
                    output,
                    run_subdir: run_plan.run_subdir.as_deref(),
                    parameter_overrides: &parameter_overrides,
                    model_section_overrides: &run_plan.model_section_overrides,
                    on_progress: &mut on_progress,
                    should_cancel: &should_cancel,
                },
            )
        } else {
            run_ngspice_ac(
                bound,
                scenario,
                backend,
                &source_netlist,
                NgspiceAcRunOptions {
                    output,
                    run_subdir: run_plan.run_subdir.as_deref(),
                    parameter_overrides: &parameter_overrides,
                    model_section_overrides: &run_plan.model_section_overrides,
                    on_progress: &mut on_progress,
                    should_cancel: &should_cancel,
                },
            )
        };
        match run_result {
            Ok(run) => {
                let finding_start = findings.len();
                for artifact in &run.artifacts {
                    push_artifact(artifacts, artifact);
                }
                push_artifact(waveforms, &run.bode);
                let assertion_measurements = evaluate_ac_assertions(scenario, &run.bode, findings);
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
                    Finding::critical(SPICE_AC_ANALYSIS, &scenario.name, error.message);
                finding
                    .measured
                    .insert("selected_backend".to_string(), json!(backend));
                finding.limit.insert(
                    "required_evidence".to_string(),
                    json!(if matches!(backend, "Xyce" | "xyce") {
                        "xyce_ac_bode_csv"
                    } else {
                        "ngspice_bode_csv"
                    }),
                );
                tag_corner_finding(&mut finding, &run_plan);
                let solver_name = if matches!(backend, "Xyce" | "xyce") {
                    "Xyce"
                } else {
                    "ngspice"
                };
                finding.suggested_fixes.push(format!(
                    "Inspect the generated {solver_name} AC wrapper deck and solver log artifacts."
                ));
                findings.push(finding);
            }
        }
    }
    push_sweep_margin_summaries(findings, scenario, &sweep_measurements);
}

pub(super) fn push_canceled_finding(findings: &mut Vec<Finding>, scenario: &Scenario) {
    findings.push(Finding::critical(
        "VALIDATION_CANCELED",
        &scenario.name,
        "Analog transient validation was canceled before completion.",
    ));
}
