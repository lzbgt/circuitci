use crate::board_ir::{
    AnalogAggregation, AnalogMonteCarloCriteria, AnalogNetlistSource, AnalogRelation,
    AnalogSweepComponentField, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::analog_ac_assertions::{evaluate_ac_assertions, validate_ac_assertion_contract};
use super::analog_assertions::{
    assertion_reference_contract_is_complete, evaluate_waveform_assertions,
    validate_assertion_contract, validate_probe_contract,
};
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_model_compiler::validate_model_compiler_provenance;
use super::analog_operating_limits::{
    evaluate_operating_limits, operating_limit_probes, operating_probe_expressions,
};
use super::analog_runner::{
    AnalogRuntimeFeature, BackendSelection, ModelSectionOverride, NgspiceAcRunOptions,
    NgspiceRunOptions, ParameterOverride, backend_name, embedded_solver_unavailable,
    external_backend_unavailable, run_ngspice, run_ngspice_ac, select_backend_for_feature,
};
use super::analog_soa::evaluate_soa_limits;
use super::analog_sweep_reports::{
    monte_carlo_criteria_enabled, push_sweep_margin_summaries, record_sweep_measurements,
    tag_corner_finding, tag_corner_findings,
};
use super::analog_sweep_sampling::monte_carlo_component_value_samples;
use super::analog_util::{
    component_value_parameter_name, file_sha256_hex, push_artifact, safe_artifact_name,
};
use super::analog_xyce_runner::{
    XyceAcRunOptions, XyceTransientRunOptions, run_xyce_ac, run_xyce_transient,
};
use super::common::validation_input_missing;
use super::spice_netlist::generate_board_netlist;
use super::{SPICE_AC_ANALYSIS, SPICE_TRANSIENT_ANALYSIS};

const MAX_ANALOG_SWEEP_CORNERS: usize = 64;

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
    for model_file in &analog.model_files {
        let path = bound.project.source_dir.join(&model_file.path);
        if !path.is_file() {
            let mut finding = Finding::critical(
                "ANALOG_MODEL_UNAVAILABLE",
                &scenario.name,
                format!(
                    "SPICE model file {} is required for physical analog simulation.",
                    path.display()
                ),
            );
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_model_file"));
            finding.suggested_fixes.push(
                "Add sourced or bench-calibrated SPICE model files for the simulated devices."
                    .to_string(),
            );
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
                    finding.suggested_fixes.push(
                        "Update the model file provenance or use the exact model artifact declared by the scenario.".to_string(),
                    );
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
    for model_file in &analog.model_files {
        let path = bound.project.source_dir.join(&model_file.path);
        if !path.is_file() {
            let mut finding = Finding::critical(
                "ANALOG_MODEL_UNAVAILABLE",
                &scenario.name,
                format!(
                    "SPICE model file {} is required for physical analog AC simulation.",
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

#[derive(Debug, Clone)]
pub(super) struct AnalogRunPlan {
    pub(super) sweep_name: Option<String>,
    pub(super) corner_name: String,
    pub(super) run_subdir: Option<String>,
    pub(super) parameter_overrides: Vec<ParameterOverride>,
    pub(super) component_value_overrides: Vec<ComponentValueOverride>,
    pub(super) model_section_overrides: Vec<ModelSectionOverride>,
}

#[derive(Debug, Clone)]
pub(super) struct ComponentValueOverride {
    pub(super) component: String,
    pub(super) field: AnalogSweepComponentField,
    pub(super) parameter_name: String,
    pub(super) value: f64,
}

impl AnalogRunPlan {
    fn nominal() -> Self {
        Self {
            sweep_name: None,
            corner_name: "nominal".to_string(),
            run_subdir: None,
            parameter_overrides: Vec::new(),
            component_value_overrides: Vec::new(),
            model_section_overrides: Vec::new(),
        }
    }

    pub(super) fn progress_label(&self) -> String {
        if let Some(sweep_name) = &self.sweep_name {
            let override_count = self.parameter_overrides.len()
                + self.component_value_overrides.len()
                + self.model_section_overrides.len();
            format!(
                "{} / {} with {} override(s).",
                sweep_name, self.corner_name, override_count
            )
        } else {
            "nominal input set.".to_string()
        }
    }

    pub(super) fn parameter_overrides_for_solver(&self) -> Vec<ParameterOverride> {
        let mut overrides = self.parameter_overrides.clone();
        overrides.extend(self.component_value_overrides.iter().map(|override_| {
            ParameterOverride {
                name: override_.parameter_name.clone(),
                value: override_.value,
            }
        }));
        overrides
    }
}

pub(super) fn analog_run_plans(
    analog: &crate::board_ir::AnalogScenario,
) -> Result<Vec<AnalogRunPlan>, String> {
    if analog.sweeps.is_empty() {
        return Ok(vec![AnalogRunPlan::nominal()]);
    }
    let mut plans = Vec::new();
    for sweep in &analog.sweeps {
        if sweep.name.trim().is_empty() {
            return Err("analog sweep names must not be empty.".to_string());
        }
        if sweep.parameters.is_empty()
            && sweep.component_values.is_empty()
            && sweep.model_sections.is_empty()
            && sweep.monte_carlo.is_none()
        {
            return Err(format!(
                "Analog sweep {} must declare at least one parameter, component value, model section, or Monte Carlo block.",
                sweep.name
            ));
        }
        let mut seen = BTreeSet::new();
        let mut parameter_values = Vec::new();
        for parameter in &sweep.parameters {
            if !valid_spice_parameter_name(&parameter.name) {
                return Err(format!(
                    "Analog sweep {} has invalid SPICE parameter name {}.",
                    sweep.name, parameter.name
                ));
            }
            if !seen.insert(parameter.name.clone()) {
                return Err(format!(
                    "Analog sweep {} declares parameter {} more than once.",
                    sweep.name, parameter.name
                ));
            }
            if parameter.values.is_empty() {
                return Err(format!(
                    "Analog sweep {} parameter {} must declare at least one value.",
                    sweep.name, parameter.name
                ));
            }
            if parameter.values.iter().any(|value| !value.is_finite()) {
                return Err(format!(
                    "Analog sweep {} parameter {} contains a non-finite value.",
                    sweep.name, parameter.name
                ));
            }
            parameter_values.push((parameter.name.clone(), parameter.values.clone()));
        }
        let mut component_value_seen = BTreeSet::new();
        let mut component_value_values = Vec::new();
        for component_value in &sweep.component_values {
            let component = component_value.component.trim();
            if component.is_empty() {
                return Err(format!(
                    "Analog sweep {} has a component value entry with an empty component.",
                    sweep.name
                ));
            }
            let key = (component.to_string(), component_value.field);
            if !component_value_seen.insert(key.clone()) {
                return Err(format!(
                    "Analog sweep {} declares component value {}.{} more than once.",
                    sweep.name,
                    key.0,
                    key.1.as_str()
                ));
            }
            if component_value.values.is_empty() {
                return Err(format!(
                    "Analog sweep {} component value {}.{} must declare at least one value.",
                    sweep.name,
                    component,
                    component_value.field.as_str()
                ));
            }
            if component_value.values.iter().any(|value| {
                !value.is_finite()
                    || component_value.field.requires_positive_value() && *value <= 0.0
            }) {
                return Err(format!(
                    "Analog sweep {} component value {}.{} contains a non-finite or out-of-range value.",
                    sweep.name,
                    component,
                    component_value.field.as_str()
                ));
            }
            component_value_values.push((
                component.to_string(),
                component_value.field,
                component_value.values.clone(),
            ));
        }
        let mut model_section_values = Vec::new();
        for model_section in &sweep.model_sections {
            if model_section.path.trim().is_empty() {
                return Err(format!(
                    "Analog sweep {} has a model section entry with an empty path.",
                    sweep.name
                ));
            }
            if model_section.sections.is_empty() {
                return Err(format!(
                    "Analog sweep {} model file {} must declare at least one section.",
                    sweep.name, model_section.path
                ));
            }
            for section in &model_section.sections {
                if !valid_spice_model_section_name(section) {
                    return Err(format!(
                        "Analog sweep {} model file {} has invalid section name {}.",
                        sweep.name, model_section.path, section
                    ));
                }
            }
            model_section_values.push((model_section.path.clone(), model_section.sections.clone()));
        }
        let mut parameter_combinations = Vec::new();
        expand_parameter_combinations(
            &parameter_values,
            0,
            Vec::new(),
            &mut parameter_combinations,
        )?;
        let mut model_section_combinations = Vec::new();
        expand_model_section_combinations(
            &model_section_values,
            0,
            Vec::new(),
            &mut model_section_combinations,
        )?;
        let mut component_value_combinations = Vec::new();
        expand_component_value_combinations(
            &component_value_values,
            0,
            Vec::new(),
            &mut component_value_combinations,
        )?;
        let monte_carlo_combinations = if let Some(monte_carlo) = &sweep.monte_carlo {
            validate_monte_carlo_criteria(&sweep.name, monte_carlo.criteria.as_ref())?;
            monte_carlo_component_value_samples(&sweep.name, monte_carlo, &component_value_seen)?
                .into_iter()
                .map(|sample| {
                    sample
                        .into_iter()
                        .map(|entry| ComponentValueOverride {
                            parameter_name: component_value_parameter_name(
                                &entry.component,
                                entry.field.as_str(),
                            ),
                            component: entry.component,
                            field: entry.field,
                            value: entry.value,
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        } else {
            vec![Vec::new()]
        };
        for (
            index,
            (
                parameter_overrides,
                component_value_overrides,
                monte_carlo_component_value_overrides,
                model_section_overrides,
            ),
        ) in
            parameter_combinations
                .into_iter()
                .flat_map(|parameter_overrides| {
                    component_value_combinations.iter().cloned().map(
                        move |component_value_overrides| {
                            (parameter_overrides.clone(), component_value_overrides)
                        },
                    )
                })
                .flat_map(|(parameter_overrides, component_value_overrides)| {
                    monte_carlo_combinations.iter().cloned().map(
                        move |monte_carlo_component_value_overrides| {
                            (
                                parameter_overrides.clone(),
                                component_value_overrides.clone(),
                                monte_carlo_component_value_overrides,
                            )
                        },
                    )
                })
                .flat_map(
                    |(
                        parameter_overrides,
                        component_value_overrides,
                        monte_carlo_component_value_overrides,
                    )| {
                        model_section_combinations.iter().cloned().map(
                            move |model_section_overrides| {
                                (
                                    parameter_overrides.clone(),
                                    component_value_overrides.clone(),
                                    monte_carlo_component_value_overrides.clone(),
                                    model_section_overrides,
                                )
                            },
                        )
                    },
                )
                .enumerate()
        {
            if plans.len() >= MAX_ANALOG_SWEEP_CORNERS {
                return Err(format!(
                    "Analog sweeps exceed the {MAX_ANALOG_SWEEP_CORNERS}-corner execution cap."
                ));
            }
            let corner_name = format!("corner_{:03}", index + 1);
            plans.push(AnalogRunPlan {
                sweep_name: Some(sweep.name.clone()),
                run_subdir: Some(format!("{}_{}", sweep.name, corner_name)),
                corner_name,
                parameter_overrides,
                component_value_overrides: component_value_overrides
                    .into_iter()
                    .chain(monte_carlo_component_value_overrides)
                    .collect(),
                model_section_overrides,
            });
        }
    }
    Ok(plans)
}

fn validate_monte_carlo_criteria(
    sweep_name: &str,
    criteria: Option<&AnalogMonteCarloCriteria>,
) -> Result<(), String> {
    let Some(criteria) = criteria else {
        return Ok(());
    };
    if let Some(min_yield_percent) = criteria.min_yield_percent
        && (!min_yield_percent.is_finite() || !(0.0..=100.0).contains(&min_yield_percent))
    {
        return Err(format!(
            "Analog sweep {sweep_name} Monte Carlo min_yield_percent must be between 0 and 100."
        ));
    }
    for (field, value) in [
        ("min_p1_margin", criteria.min_p1_margin),
        ("min_p5_margin", criteria.min_p5_margin),
        ("min_p50_margin", criteria.min_p50_margin),
        ("min_p95_margin", criteria.min_p95_margin),
    ] {
        if value.is_some_and(|value| !value.is_finite()) {
            return Err(format!(
                "Analog sweep {sweep_name} Monte Carlo {field} must be finite."
            ));
        }
    }
    Ok(())
}

fn expand_component_value_combinations(
    component_values: &[(String, AnalogSweepComponentField, Vec<f64>)],
    index: usize,
    current: Vec<ComponentValueOverride>,
    output: &mut Vec<Vec<ComponentValueOverride>>,
) -> Result<(), String> {
    if output.len() >= MAX_ANALOG_SWEEP_CORNERS {
        return Err(format!(
            "Analog sweeps exceed the {MAX_ANALOG_SWEEP_CORNERS}-corner execution cap."
        ));
    }
    let Some((component, field, values)) = component_values.get(index) else {
        output.push(current);
        return Ok(());
    };
    for value in values {
        let mut next = current.clone();
        next.push(ComponentValueOverride {
            component: component.clone(),
            field: *field,
            parameter_name: component_value_parameter_name(component, field.as_str()),
            value: *value,
        });
        expand_component_value_combinations(component_values, index + 1, next, output)?;
    }
    Ok(())
}

fn expand_parameter_combinations(
    parameters: &[(String, Vec<f64>)],
    index: usize,
    current: Vec<ParameterOverride>,
    output: &mut Vec<Vec<ParameterOverride>>,
) -> Result<(), String> {
    if output.len() >= MAX_ANALOG_SWEEP_CORNERS {
        return Err(format!(
            "Analog sweeps exceed the {MAX_ANALOG_SWEEP_CORNERS}-corner execution cap."
        ));
    }
    let Some((name, values)) = parameters.get(index) else {
        output.push(current);
        return Ok(());
    };
    for value in values {
        let mut next = current.clone();
        next.push(ParameterOverride {
            name: name.clone(),
            value: *value,
        });
        expand_parameter_combinations(parameters, index + 1, next, output)?;
    }
    Ok(())
}

fn expand_model_section_combinations(
    model_sections: &[(String, Vec<String>)],
    index: usize,
    current: Vec<ModelSectionOverride>,
    output: &mut Vec<Vec<ModelSectionOverride>>,
) -> Result<(), String> {
    if output.len() >= MAX_ANALOG_SWEEP_CORNERS {
        return Err(format!(
            "Analog sweeps exceed the {MAX_ANALOG_SWEEP_CORNERS}-corner execution cap."
        ));
    }
    let Some((path, sections)) = model_sections.get(index) else {
        output.push(current);
        return Ok(());
    };
    for section in sections {
        let mut next = current.clone();
        next.push(ModelSectionOverride {
            path: path.clone(),
            section: section.clone(),
        });
        expand_model_section_combinations(model_sections, index + 1, next, output)?;
    }
    Ok(())
}

fn valid_spice_parameter_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn valid_spice_model_section_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
}

fn netlist_source_name(source: &AnalogNetlistSource) -> &'static str {
    match source {
        AnalogNetlistSource::File => "file-backed",
        AnalogNetlistSource::GeneratedFromBoard => "generated-from-Board",
    }
}

pub(super) fn validate_netlist_source(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    artifacts: &mut Vec<String>,
) -> Option<Finding> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before netlist source validation");
    let netlist_finding = match analog.netlist_source {
        AnalogNetlistSource::File => {
            let Some(netlist) = &analog.netlist else {
                let mut finding = Finding::critical(
                    "ANALOG_NETLIST_UNAVAILABLE",
                    &scenario.name,
                    "analog.netlist is required when analog.netlist_source is file.",
                );
                finding
                    .limit
                    .insert("required_artifact".to_string(), json!("spice_netlist"));
                return Some(finding);
            };
            let netlist = bound.project.source_dir.join(netlist);
            if !netlist.is_file() {
                let mut finding = Finding::critical(
                    "ANALOG_NETLIST_UNAVAILABLE",
                    &scenario.name,
                    format!(
                        "SPICE netlist {} is required for physical analog simulation.",
                        netlist.display()
                    ),
                );
                finding
                    .limit
                    .insert("required_artifact".to_string(), json!("spice_netlist"));
                finding.suggested_fixes.push(
                    "Add a SPICE-compatible deck with device models for this board region."
                        .to_string(),
                );
                return Some(finding);
            }
            push_artifact(artifacts, &netlist);
            None
        }
        AnalogNetlistSource::GeneratedFromBoard => {
            if analog.generated.is_none() {
                return Some(Finding::critical(
                    "ANALOG_NETLIST_UNAVAILABLE",
                    &scenario.name,
                    "analog.generated is required when analog.netlist_source is generated_from_board.",
                ));
            }
            None
        }
    };
    if netlist_finding.is_some() {
        return netlist_finding;
    }
    validate_model_compiler_provenance(bound, scenario, artifacts)
}

pub(super) fn prepare_source_netlist(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    run_dir: &Path,
) -> Result<PathBuf, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before source netlist preparation");
    match analog.netlist_source {
        AnalogNetlistSource::File => {
            let netlist = analog
                .netlist
                .as_ref()
                .ok_or_else(|| "analog.netlist is required for file netlist source.".to_string())?;
            Ok(bound.project.source_dir.join(netlist))
        }
        AnalogNetlistSource::GeneratedFromBoard => {
            let path = run_dir.join("generated_board.cir");
            generate_board_netlist(bound, analog, &path)?;
            Ok(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_ANALOG_SWEEP_CORNERS, analog_run_plans};
    use crate::board_ir::BoardProject;

    fn rc_sweep_project_yaml(parameter_yaml: &str) -> String {
        format!(
            r#"
project:
  name: sweep_test
  version: 0.1.0
board:
  name: sweep_test
  components: {{}}
  nets: {{}}
scenarios:
  - name: analog_sweep
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: file
      netlist: deck.cir
      model_files: []
      node_bindings:
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: "0", endpoint: {{ component: U1, pin: GND }} }}
      analysis:
        type: tran
        stop_time_us: 1000.0
        max_step_us: 1.0
      stimuli: []
      sweeps:
        - name: rc_tolerance
          parameters:
{parameter_yaml}
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"#
        )
    }

    fn model_section_sweep_project_yaml(sweep_body_yaml: &str) -> String {
        format!(
            r#"
project:
  name: model_corner_test
  version: 0.1.0
board:
  name: model_corner_test
  components: {{}}
  nets: {{}}
scenarios:
  - name: analog_sweep
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: file
      netlist: deck.cir
      model_files:
        - path: models/vendor.lib
      node_bindings:
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: "0", endpoint: {{ component: U1, pin: GND }} }}
      analysis:
        type: tran
        stop_time_us: 1000.0
        max_step_us: 1.0
      stimuli: []
      sweeps:
        - name: model_corner
{sweep_body_yaml}
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"#
        )
    }

    #[test]
    fn analog_run_plans_expand_parameter_sweeps() {
        let yaml = rc_sweep_project_yaml(
            r#"            - name: RIN_VALUE
              values: [950.0, 1000.0]
            - name: COUT_VALUE
              values: [0.000000095, 0.0000001]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 4);
        assert_eq!(plans[0].sweep_name.as_deref(), Some("rc_tolerance"));
        assert_eq!(plans[0].corner_name, "corner_001");
        assert_eq!(
            plans[0].run_subdir.as_deref(),
            Some("rc_tolerance_corner_001")
        );
        assert_eq!(plans[0].parameter_overrides[0].name, "RIN_VALUE");
        assert_eq!(plans[0].parameter_overrides[0].value, 950.0);
        assert_eq!(plans[0].parameter_overrides[1].name, "COUT_VALUE");
        assert_eq!(plans[0].parameter_overrides[1].value, 0.000000095);
        assert_eq!(plans[3].parameter_overrides[0].value, 1000.0);
        assert_eq!(plans[3].parameter_overrides[1].value, 0.0000001);
    }

    #[test]
    fn analog_run_plans_expand_model_section_sweeps() {
        let yaml = model_section_sweep_project_yaml(
            r#"          model_sections:
            - path: models/vendor.lib
              sections: [typ, slow, fast]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 3);
        assert_eq!(plans[0].sweep_name.as_deref(), Some("model_corner"));
        assert_eq!(plans[0].corner_name, "corner_001");
        assert_eq!(
            plans[0].run_subdir.as_deref(),
            Some("model_corner_corner_001")
        );
        assert!(plans[0].parameter_overrides.is_empty());
        assert_eq!(
            plans[0].model_section_overrides[0].path,
            "models/vendor.lib"
        );
        assert_eq!(plans[0].model_section_overrides[0].section, "typ");
        assert_eq!(plans[2].model_section_overrides[0].section, "fast");
    }

    #[test]
    fn analog_run_plans_combine_parameter_and_model_section_sweeps() {
        let yaml = model_section_sweep_project_yaml(
            r#"          parameters:
            - name: R_LOAD
              values: [900.0, 1100.0]
          model_sections:
            - path: models/vendor.lib
              sections: [slow, fast]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 4);
        assert_eq!(plans[0].parameter_overrides[0].value, 900.0);
        assert_eq!(plans[0].model_section_overrides[0].section, "slow");
        assert_eq!(plans[1].parameter_overrides[0].value, 900.0);
        assert_eq!(plans[1].model_section_overrides[0].section, "fast");
        assert_eq!(plans[2].parameter_overrides[0].value, 1100.0);
        assert_eq!(plans[2].model_section_overrides[0].section, "slow");
    }

    #[test]
    fn analog_run_plans_expand_component_value_sweeps() {
        let yaml = model_section_sweep_project_yaml(
            r#"          component_values:
            - component: RLOAD
              field: value_ohm
              values: [900.0, 1000.0]
            - component: ILOAD
              field: dc_a
              values: [0.01, 0.02]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 4);
        assert!(plans[0].parameter_overrides.is_empty());
        assert_eq!(
            plans[0].component_value_overrides[0].parameter_name,
            "CCI_RLOAD_VALUE_OHM"
        );
        assert_eq!(plans[0].component_value_overrides[0].value, 900.0);
        assert_eq!(
            plans[0].component_value_overrides[1].parameter_name,
            "CCI_ILOAD_DC_A"
        );
        assert_eq!(plans[3].component_value_overrides[0].value, 1000.0);
        assert_eq!(plans[3].component_value_overrides[1].value, 0.02);
        let solver_overrides = plans[0].parameter_overrides_for_solver();
        assert_eq!(solver_overrides.len(), 2);
        assert_eq!(solver_overrides[0].name, "CCI_RLOAD_VALUE_OHM");
    }

    #[test]
    fn analog_run_plans_expand_monte_carlo_component_values() {
        let yaml = model_section_sweep_project_yaml(
            r#"          monte_carlo:
            samples: 4
            seed: 42
            component_values:
              - component: RLOAD
                field: value_ohm
                nominal: 1000.0
                tolerance_percent: 5.0
              - component: CLOAD
                field: value_f
                nominal: 0.0000001
                tolerance_percent: 10.0
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 4);
        assert_eq!(plans[0].sweep_name.as_deref(), Some("model_corner"));
        assert_eq!(
            plans[0].run_subdir.as_deref(),
            Some("model_corner_corner_001")
        );
        assert_eq!(plans[0].component_value_overrides.len(), 2);
        assert_eq!(
            plans[0].component_value_overrides[0].parameter_name,
            "CCI_RLOAD_VALUE_OHM"
        );
        assert!((950.0..=1050.0).contains(&plans[0].component_value_overrides[0].value));
        assert!((0.00000009..=0.00000011).contains(&plans[3].component_value_overrides[1].value));
    }

    #[test]
    fn analog_run_plans_reject_invalid_monte_carlo_criteria() {
        let yaml = model_section_sweep_project_yaml(
            r#"          monte_carlo:
            samples: 4
            seed: 42
            criteria:
              min_yield_percent: 101.0
            component_values:
              - component: RLOAD
                field: value_ohm
                nominal: 1000.0
                tolerance_percent: 5.0
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let error = analog_run_plans(analog).unwrap_err();

        assert!(error.contains("min_yield_percent must be between 0 and 100"));
    }

    #[test]
    fn analog_run_plans_reject_invalid_parameter_names() {
        let yaml = rc_sweep_project_yaml(
            r#"            - name: 1BAD
              values: [1.0]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let error = analog_run_plans(analog).unwrap_err();

        assert!(error.contains("invalid SPICE parameter name"));
    }

    #[test]
    fn analog_run_plans_enforce_corner_cap() {
        let values = (0..=MAX_ANALOG_SWEEP_CORNERS)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let yaml = rc_sweep_project_yaml(&format!(
            r#"            - name: RIN_VALUE
              values: [{values}]
"#
        ));
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let error = analog_run_plans(analog).unwrap_err();

        assert!(error.contains("64-corner execution cap"));
    }
}
