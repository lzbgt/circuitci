use crate::board_ir::{
    AnalogBackend, AnalogMeasureAssertion, AnalogMeasureTemplate, AnalogRelation, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::SPICE_MEASURE_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_measure_runner::{NgspiceMeasureRunOptions, run_ngspice_measure};
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
use super::analog_util::{
    executable_on_path, file_sha256_hex, normalize_artifact_path, push_artifact, safe_artifact_name,
};
use super::analog_xyce_measure_runner::{XyceMeasureRunOptions, run_xyce_measure};
use super::common::validation_input_missing;

pub(super) struct AnalogMeasureSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_measure_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogMeasureSinks<'_>,
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
        "Preparing analog measure analysis",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_measure scenario requires an analog block.",
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
                    "SPICE model file {} is required for physical analog measure analysis.",
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
            "analog_measure requires node_bindings and pin_bindings.",
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
                    "Analog measure node binding {} references unknown board net {}.",
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
                    "Analog measure pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "measure" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only measure is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let Some(mode) = nonempty(analog.analysis.measure_mode.as_deref()) else {
        validation_input_missing(findings, scenario, "analog_measure requires measure_mode.");
        return;
    };
    if !matches!(mode, "tran" | "ac") {
        validation_input_missing(
            findings,
            scenario,
            "analog_measure requires measure_mode to be tran or ac.",
        );
        return;
    }
    if mode == "tran" {
        if !analog.analysis.stop_time_us.is_finite() || analog.analysis.stop_time_us <= 0.0 {
            validation_input_missing(
                findings,
                scenario,
                "analog_measure transient mode requires positive finite stop_time_us.",
            );
            return;
        }
        if !analog.analysis.max_step_us.is_finite() || analog.analysis.max_step_us <= 0.0 {
            validation_input_missing(
                findings,
                scenario,
                "analog_measure transient mode requires positive finite max_step_us.",
            );
            return;
        }
        if analog.analysis.max_step_us >= analog.analysis.stop_time_us {
            validation_input_missing(
                findings,
                scenario,
                "analog_measure transient mode requires max_step_us smaller than stop_time_us.",
            );
            return;
        }
    } else {
        let Some(start_hz) = analog.analysis.start_frequency_hz else {
            validation_input_missing(
                findings,
                scenario,
                "analog_measure AC mode requires start_frequency_hz.",
            );
            return;
        };
        let Some(stop_hz) = analog.analysis.stop_frequency_hz else {
            validation_input_missing(
                findings,
                scenario,
                "analog_measure AC mode requires stop_frequency_hz.",
            );
            return;
        };
        if !start_hz.is_finite() || !stop_hz.is_finite() || start_hz <= 0.0 || start_hz >= stop_hz {
            validation_input_missing(
                findings,
                scenario,
                "analog_measure AC mode requires 0 < start_frequency_hz < stop_frequency_hz.",
            );
            return;
        }
        if !matches!(analog.analysis.points_per_decade, Some(1..=1000)) {
            validation_input_missing(
                findings,
                scenario,
                "analog_measure AC mode requires points_per_decade in 1..=1000.",
            );
            return;
        }
    }
    if analog.analysis.measure_statements.is_empty() && analog.analysis.measure_templates.is_empty()
    {
        validation_input_missing(
            findings,
            scenario,
            "analog_measure requires at least one measure statement or template.",
        );
        return;
    }
    let mut names = BTreeSet::new();
    for statement in &analog.analysis.measure_statements {
        let Some(name) = nonempty(Some(&statement.name)) else {
            validation_input_missing(
                findings,
                scenario,
                "analog_measure statement name is empty.",
            );
            return;
        };
        if !names.insert(name.to_ascii_lowercase()) {
            validation_input_missing(
                findings,
                scenario,
                format!("analog_measure duplicate statement name {name}."),
            );
            return;
        }
        if let Err(message) =
            validate_measure_statement(mode, name, &statement.statement, &bound_nodes, scenario)
        {
            validation_input_missing(findings, scenario, message);
            return;
        }
    }
    for template in &analog.analysis.measure_templates {
        let Some(name) = nonempty(Some(&template.name)) else {
            validation_input_missing(findings, scenario, "analog_measure template name is empty.");
            return;
        };
        if !names.insert(name.to_ascii_lowercase()) {
            validation_input_missing(
                findings,
                scenario,
                format!("analog_measure duplicate measurement name {name}."),
            );
            return;
        }
        if let Err(message) = validate_measure_template(mode, template, &bound_nodes, scenario) {
            validation_input_missing(findings, scenario, message);
            return;
        }
    }
    if let Err(message) = validate_measure_assertions(analog, &names) {
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
            SPICE_MEASURE_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create analog measure run directory {}: {error}",
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
            let mut finding = Finding::critical(SPICE_MEASURE_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    };

    on_progress(
        "Selecting analog measure backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_measure_backend(
        &analog.backend,
        !analog.analysis.measure_statements.is_empty(),
        !analog.analysis.measure_templates.is_empty(),
    );
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

    if backend != "ngspice" && !is_xyce_backend(backend) {
        let mut finding = unsupported_backend_plan_finding(
            scenario,
            UnsupportedBackendPlan {
                check_id: SPICE_MEASURE_ANALYSIS,
                selected_backend: backend,
                implemented_backend: "ngspice",
                analysis_kind: "measure",
                required_normalized_outputs: &["measure_summary"],
            },
        );
        finding
            .measured
            .insert("measure_mode".to_string(), json!(mode));
        finding.measured.insert(
            "measurements".to_string(),
            json!(
                analog.analysis.measure_statements.len() + analog.analysis.measure_templates.len()
            ),
        );
        finding.limit.insert(
            "required_evidence".to_string(),
            json!("measure_summary_csv_or_json"),
        );
        findings.push(finding);
        return;
    }
    if is_xyce_backend(backend) && !analog.analysis.measure_statements.is_empty() {
        let mut finding = unsupported_backend_plan_finding(
            scenario,
            UnsupportedBackendPlan {
                check_id: SPICE_MEASURE_ANALYSIS,
                selected_backend: backend,
                implemented_backend: "ngspice_or_xyce_templates",
                analysis_kind: "measure",
                required_normalized_outputs: &["measure_summary"],
            },
        );
        finding
            .measured
            .insert("measure_mode".to_string(), json!(mode));
        finding.measured.insert(
            "raw_measure_statements".to_string(),
            json!(analog.analysis.measure_statements.len()),
        );
        finding.measured.insert(
            "adapter_blocker".to_string(),
            json!("Raw measure_statements[] are backend-specific text and are not rewritten into Xyce .MEASURE syntax; use portable measure_templates[] for Xyce until a reviewed raw-statement translator and real-solver conformance contract exists."),
        );
        finding.measured.insert(
            "evidence_sources".to_string(),
            json!([
                "docs/research/circuit_simulation_full_featured/measure_backend_evidence.md",
                "docs/research/circuit_simulation_full_featured/sources/ngspice_manual.xhtml",
                "docs/research/circuit_simulation_full_featured/sources/Xyce_Reference_Guide_7.8.txt"
            ]),
        );
        finding.limit.insert(
            "required_evidence".to_string(),
            json!("measure_templates_or_ngspice_raw_measure_summary"),
        );
        finding.suggested_fixes.push(
            "Use measure_templates[] for portable Xyce scalar extraction, or run raw measure_statements[] with backend: ngspice."
                .to_string(),
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
            "Running analog measure input corner",
            run_plan.progress_label(),
        );
        let parameter_overrides = run_plan.parameter_overrides_for_solver();
        let run_result = if backend == "ngspice" {
            run_ngspice_measure(
                bound,
                scenario,
                backend,
                &source_netlist,
                NgspiceMeasureRunOptions {
                    output,
                    run_subdir: run_plan.run_subdir.as_deref(),
                    parameter_overrides: &parameter_overrides,
                    model_section_overrides: &run_plan.model_section_overrides,
                    on_progress: &mut on_progress,
                    should_cancel: &should_cancel,
                },
            )
        } else {
            run_xyce_measure(
                bound,
                scenario,
                backend,
                &source_netlist,
                XyceMeasureRunOptions {
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
                push_artifact(artifacts, &run.summary);
                let assertion_measurements =
                    evaluate_measure_assertions(scenario, &run.summary, findings);
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
                    Finding::critical(SPICE_MEASURE_ANALYSIS, &scenario.name, error.message);
                finding
                    .measured
                    .insert("selected_backend".to_string(), json!(backend));
                finding.limit.insert(
                    "required_evidence".to_string(),
                    json!(if backend == "ngspice" {
                        "ngspice_measure_summary_csv"
                    } else {
                        "xyce_measure_summary_csv"
                    }),
                );
                tag_corner_finding(&mut finding, &run_plan);
                finding.suggested_fixes.push(format!(
                    "Inspect the generated {} .MEASURE wrapper deck and solver log artifacts.",
                    if backend == "ngspice" {
                        "ngspice"
                    } else {
                        "Xyce"
                    }
                ));
                findings.push(finding);
            }
        }
    }
    push_sweep_margin_summaries(findings, scenario, &sweep_measurements);
}

fn select_measure_backend(
    requested: &AnalogBackend,
    has_raw_statements: bool,
    has_templates: bool,
) -> BackendSelection {
    match select_backend_for_feature(requested, AnalogRuntimeFeature::Measure) {
        BackendSelection::Unavailable
            if *requested == AnalogBackend::Auto && !has_raw_statements && has_templates =>
        {
            if executable_on_path("Xyce") {
                BackendSelection::Selected("Xyce")
            } else if executable_on_path("xyce") {
                BackendSelection::Selected("xyce")
            } else {
                BackendSelection::Unavailable
            }
        }
        selected => selected,
    }
}

fn validate_measure_assertions(
    analog: &crate::board_ir::AnalogScenario,
    measurement_names: &BTreeSet<String>,
) -> Result<(), String> {
    let mut assertion_names = BTreeSet::new();
    for assertion in &analog.analysis.measure_assertions {
        let Some(name) = nonempty(Some(&assertion.name)) else {
            return Err("analog_measure assertion name is empty.".to_string());
        };
        if !assertion_names.insert(name.to_ascii_lowercase()) {
            return Err(format!("analog_measure duplicate assertion name {name}."));
        }
        let Some(measurement) = nonempty(Some(&assertion.measurement)) else {
            return Err(format!(
                "analog_measure assertion {name} is missing measurement."
            ));
        };
        if !measurement_names.contains(&measurement.to_ascii_lowercase()) {
            return Err(format!(
                "analog_measure assertion {name} references unknown measurement {measurement}."
            ));
        }
        if !assertion.threshold.is_finite() {
            return Err(format!(
                "analog_measure assertion {name} requires finite threshold."
            ));
        }
        if matches!(assertion.unit.as_deref(), Some(unit) if unit.trim().is_empty()) {
            return Err(format!(
                "analog_measure assertion {name} has an empty unit."
            ));
        }
    }
    Ok(())
}

fn evaluate_measure_assertions(
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before measure assertions");
    if analog.analysis.measure_assertions.is_empty() {
        return Vec::new();
    }
    let rows = match read_measure_summary(summary) {
        Ok(rows) => rows,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_MEASURE_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.measure_assertions {
        let Some(measured) = rows
            .get(&assertion.measurement.to_ascii_lowercase())
            .copied()
        else {
            let mut finding = Finding::critical(
                SPICE_MEASURE_ANALYSIS,
                &scenario.name,
                format!(
                    "Measure assertion {} references missing normalized measurement {}.",
                    assertion.name, assertion.measurement
                ),
            );
            finding
                .measured
                .insert("assertion".to_string(), json!(&assertion.name));
            finding.measured.insert(
                "measure_summary".to_string(),
                json!(normalize_artifact_path(summary)),
            );
            findings.push(finding);
            continue;
        };
        let evaluation = evaluate_measure_assertion_value(assertion, measured);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: assertion.measurement.clone(),
            measured,
            limit: assertion.threshold,
            margin: evaluation.margin,
            relation: evaluation.relation,
            unit: measure_assertion_unit(assertion).to_string(),
            quantity: "measure".to_string(),
            passed: evaluation.passed,
        });
        push_measure_assertion_finding(
            scenario, assertion, measured, summary, evaluation, findings,
        );
    }
    measurements
}

#[derive(Debug, Clone, Copy)]
struct MeasureAssertionEvaluation {
    relation: &'static str,
    margin: f64,
    passed: bool,
}

fn evaluate_measure_assertion_value(
    assertion: &AnalogMeasureAssertion,
    measured: f64,
) -> MeasureAssertionEvaluation {
    match assertion.relation {
        AnalogRelation::Above => MeasureAssertionEvaluation {
            relation: "above",
            margin: measured - assertion.threshold,
            passed: measured > assertion.threshold,
        },
        AnalogRelation::Below => MeasureAssertionEvaluation {
            relation: "below",
            margin: assertion.threshold - measured,
            passed: measured < assertion.threshold,
        },
    }
}

fn push_measure_assertion_finding(
    scenario: &Scenario,
    assertion: &AnalogMeasureAssertion,
    measured: f64,
    summary: &Path,
    evaluation: MeasureAssertionEvaluation,
    findings: &mut Vec<Finding>,
) {
    if evaluation.passed {
        return;
    }
    let unit = measure_assertion_unit(assertion);
    let mut finding = Finding::critical(
        SPICE_MEASURE_ANALYSIS,
        &scenario.name,
        format!(
            "Measure assertion {} failed: measurement {} was {:.6} {}, expected {} {:.6} {}.",
            assertion.name,
            assertion.measurement,
            measured,
            unit,
            evaluation.relation,
            assertion.threshold,
            unit
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding
        .measured
        .insert("measurement".to_string(), json!(&assertion.measurement));
    finding
        .measured
        .insert("value".to_string(), json!(measured));
    finding.measured.insert("unit".to_string(), json!(unit));
    finding
        .measured
        .insert("margin".to_string(), json!(evaluation.margin));
    finding.measured.insert(
        "measure_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding.limit.insert(
        format!("{}_threshold", evaluation.relation),
        json!(assertion.threshold),
    );
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Adjust the circuit, stimulus, model, or measurement template so the normalized scalar result meets the declared design limit."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.clone());
    }
    findings.push(finding);
}

fn measure_assertion_unit(assertion: &AnalogMeasureAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or("scalar")
}

fn read_measure_summary(summary: &Path) -> Result<BTreeMap<String, f64>, String> {
    let text = fs::read_to_string(summary).map_err(|error| {
        format!(
            "Failed to read normalized measure summary {}: {error}",
            summary.display()
        )
    })?;
    let mut rows = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if index == 0 {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let mut columns = line.splitn(4, ',');
        let measurement = columns.next().unwrap_or("").trim();
        let _mode = columns.next();
        let value = columns.next().unwrap_or("").trim();
        let parsed = value.parse::<f64>().map_err(|error| {
            format!(
                "Measure summary {} row {} has invalid numeric value {value}: {error}",
                summary.display(),
                index + 1
            )
        })?;
        rows.insert(measurement.to_ascii_lowercase(), parsed);
    }
    Ok(rows)
}

fn is_xyce_backend(backend: &str) -> bool {
    backend.eq_ignore_ascii_case("xyce")
}

fn validate_measure_template(
    mode: &str,
    template: &AnalogMeasureTemplate,
    bound_nodes: &BTreeSet<&str>,
    scenario: &Scenario,
) -> Result<(), String> {
    if !matches!(
        template.operation.as_str(),
        "avg" | "max" | "min" | "rms" | "find" | "delay" | "slew" | "threshold_time"
    ) {
        return Err(format!(
            "analog_measure template {} uses unsupported operation {}.",
            template.name, template.operation
        ));
    }
    validate_expression_references(&template.expression, bound_nodes, scenario)?;
    if matches!(template.operation.as_str(), "delay" | "slew") {
        return validate_delay_template(mode, template, bound_nodes, scenario);
    }
    if template.operation == "threshold_time" {
        return validate_threshold_time_template(mode, template);
    }
    if mode == "tran" {
        validate_optional_window_us(template)?;
        if template.from_hz.is_some() || template.to_hz.is_some() || template.at_hz.is_some() {
            return Err(format!(
                "analog_measure transient template {} may not use frequency window fields.",
                template.name
            ));
        }
    } else {
        validate_optional_window_hz(template)?;
        if template.from_us.is_some() || template.to_us.is_some() || template.at_us.is_some() {
            return Err(format!(
                "analog_measure AC template {} may not use time window fields.",
                template.name
            ));
        }
    }
    if template.operation == "find" && template.at_us.is_none() && template.at_hz.is_none() {
        return Err(format!(
            "analog_measure find template {} requires at_us or at_hz.",
            template.name
        ));
    }
    Ok(())
}

fn validate_delay_template(
    mode: &str,
    template: &AnalogMeasureTemplate,
    bound_nodes: &BTreeSet<&str>,
    scenario: &Scenario,
) -> Result<(), String> {
    if mode != "tran" {
        return Err(format!(
            "analog_measure {} template {} is only supported for transient mode.",
            template.operation, template.name
        ));
    }
    if template.operation == "delay" {
        let Some(trigger_expression) = nonempty(template.trigger_expression.as_deref()) else {
            return Err(format!(
                "analog_measure delay template {} requires trigger_expression.",
                template.name
            ));
        };
        validate_expression_references(trigger_expression, bound_nodes, scenario)?;
    } else if let Some(trigger_expression) = nonempty(template.trigger_expression.as_deref())
        && trigger_expression != template.expression
    {
        return Err(format!(
            "analog_measure slew template {} uses expression for both trigger and target; use operation: delay for two expressions.",
            template.name
        ));
    }
    let Some(trigger_value) = template.trigger_value else {
        return Err(format!(
            "analog_measure {} template {} requires trigger_value.",
            template.operation, template.name
        ));
    };
    if !trigger_value.is_finite() {
        return Err(format!(
            "analog_measure {} template {} requires finite trigger_value.",
            template.operation, template.name
        ));
    }
    let Some(target_value) = template.target_value else {
        return Err(format!(
            "analog_measure {} template {} requires target_value.",
            template.operation, template.name
        ));
    };
    if !target_value.is_finite() {
        return Err(format!(
            "analog_measure {} template {} requires finite target_value.",
            template.operation, template.name
        ));
    }
    validate_measure_edge("trigger_edge", template.trigger_edge.as_deref(), template)?;
    validate_measure_edge("target_edge", template.target_edge.as_deref(), template)?;
    validate_measure_count("trigger_count", template.trigger_count, template)?;
    validate_measure_count("target_count", template.target_count, template)?;
    if template.from_us.is_some()
        || template.to_us.is_some()
        || template.at_us.is_some()
        || template.from_hz.is_some()
        || template.to_hz.is_some()
        || template.at_hz.is_some()
    {
        return Err(format!(
            "analog_measure {} template {} may not use at/from/to window fields.",
            template.operation, template.name
        ));
    }
    Ok(())
}

fn validate_threshold_time_template(
    mode: &str,
    template: &AnalogMeasureTemplate,
) -> Result<(), String> {
    if mode != "tran" {
        return Err(format!(
            "analog_measure threshold_time template {} is only supported for transient mode.",
            template.name
        ));
    }
    let Some(target_value) = template.target_value else {
        return Err(format!(
            "analog_measure threshold_time template {} requires target_value.",
            template.name
        ));
    };
    if !target_value.is_finite() {
        return Err(format!(
            "analog_measure threshold_time template {} requires finite target_value.",
            template.name
        ));
    }
    validate_measure_edge("target_edge", template.target_edge.as_deref(), template)?;
    validate_measure_count("target_count", template.target_count, template)?;
    validate_optional_window_us(template)?;
    if template.at_us.is_some() {
        return Err(format!(
            "analog_measure threshold_time template {} may not use at_us.",
            template.name
        ));
    }
    if template.from_hz.is_some() || template.to_hz.is_some() || template.at_hz.is_some() {
        return Err(format!(
            "analog_measure threshold_time template {} may not use frequency window fields.",
            template.name
        ));
    }
    if template.trigger_expression.is_some()
        || template.trigger_value.is_some()
        || template.trigger_edge.is_some()
        || template.trigger_count.is_some()
    {
        return Err(format!(
            "analog_measure threshold_time template {} may not use trigger fields.",
            template.name
        ));
    }
    Ok(())
}

fn validate_measure_edge(
    field: &str,
    edge: Option<&str>,
    template: &AnalogMeasureTemplate,
) -> Result<(), String> {
    if let Some(edge) = edge
        && !matches!(edge, "rise" | "fall" | "cross")
    {
        return Err(format!(
            "analog_measure {} template {} requires {field} to be rise, fall, or cross.",
            template.operation, template.name
        ));
    }
    Ok(())
}

fn validate_measure_count(
    field: &str,
    count: Option<u32>,
    template: &AnalogMeasureTemplate,
) -> Result<(), String> {
    if let Some(0) = count {
        return Err(format!(
            "analog_measure {} template {} requires {field} >= 1.",
            template.operation, template.name
        ));
    }
    Ok(())
}

fn validate_optional_window_us(template: &AnalogMeasureTemplate) -> Result<(), String> {
    if let Some(value) = template.from_us
        && (!value.is_finite() || value < 0.0)
    {
        return Err(format!(
            "analog_measure template {} requires finite non-negative from_us.",
            template.name
        ));
    }
    if let Some(value) = template.to_us
        && (!value.is_finite() || value <= 0.0)
    {
        return Err(format!(
            "analog_measure template {} requires positive finite to_us.",
            template.name
        ));
    }
    if let Some(value) = template.at_us
        && (!value.is_finite() || value < 0.0)
    {
        return Err(format!(
            "analog_measure template {} requires finite non-negative at_us.",
            template.name
        ));
    }
    if let (Some(from_us), Some(to_us)) = (template.from_us, template.to_us)
        && from_us >= to_us
    {
        return Err(format!(
            "analog_measure template {} requires from_us < to_us.",
            template.name
        ));
    }
    Ok(())
}

fn validate_optional_window_hz(template: &AnalogMeasureTemplate) -> Result<(), String> {
    if let Some(value) = template.from_hz
        && (!value.is_finite() || value <= 0.0)
    {
        return Err(format!(
            "analog_measure template {} requires positive finite from_hz.",
            template.name
        ));
    }
    if let Some(value) = template.to_hz
        && (!value.is_finite() || value <= 0.0)
    {
        return Err(format!(
            "analog_measure template {} requires positive finite to_hz.",
            template.name
        ));
    }
    if let Some(value) = template.at_hz
        && (!value.is_finite() || value <= 0.0)
    {
        return Err(format!(
            "analog_measure template {} requires positive finite at_hz.",
            template.name
        ));
    }
    if let (Some(from_hz), Some(to_hz)) = (template.from_hz, template.to_hz)
        && from_hz >= to_hz
    {
        return Err(format!(
            "analog_measure template {} requires from_hz < to_hz.",
            template.name
        ));
    }
    Ok(())
}

fn validate_measure_statement(
    mode: &str,
    expected_name: &str,
    statement: &str,
    bound_nodes: &BTreeSet<&str>,
    scenario: &Scenario,
) -> Result<(), String> {
    if statement.contains('\n') || statement.contains('\r') {
        return Err(
            "analog_measure statements must be single-line ngspice .MEASURE commands.".to_string(),
        );
    }
    let normalized = statement.trim().trim_start_matches('.').trim_start();
    let fields: Vec<&str> = normalized.split_whitespace().collect();
    if fields.len() < 4 {
        return Err(
            "analog_measure statement must include measure mode, name, operation, and expression."
                .to_string(),
        );
    }
    if !fields[0].eq_ignore_ascii_case("meas") && !fields[0].eq_ignore_ascii_case("measure") {
        return Err("analog_measure statement must start with meas or .meas.".to_string());
    }
    if !fields[1].eq_ignore_ascii_case(mode) {
        return Err(format!(
            "analog_measure statement mode {} does not match measure_mode {mode}.",
            fields[1]
        ));
    }
    if !fields[2].eq_ignore_ascii_case(expected_name) {
        return Err(format!(
            "analog_measure statement name {} does not match declared name {expected_name}.",
            fields[2]
        ));
    }
    let lower = normalized.to_ascii_lowercase();
    if lower.contains(".control") || lower.contains(".end") || lower.contains("quit") {
        return Err(
            "analog_measure statement may not include control-block terminators.".to_string(),
        );
    }
    validate_expression_references(normalized, bound_nodes, scenario)
}

fn validate_expression_references(
    statement: &str,
    bound_nodes: &BTreeSet<&str>,
    scenario: &Scenario,
) -> Result<(), String> {
    for expression in extract_call_arguments(statement, 'v') {
        for node in expression
            .split(',')
            .map(str::trim)
            .filter(|node| !node.is_empty())
        {
            if !bound_nodes.contains(node) {
                return Err(format!(
                    "analog_measure statement references unbound node {node}."
                ));
            }
        }
    }
    let Some(analog) = &scenario.analog else {
        return Ok(());
    };
    for component in extract_call_arguments(statement, 'i') {
        let component = component.trim();
        if component.is_empty() {
            return Err("analog_measure current expression requires a component.".to_string());
        }
        let bound = analog
            .pin_bindings
            .iter()
            .any(|binding| binding.endpoint.component.eq_ignore_ascii_case(component));
        if !bound {
            return Err(format!(
                "analog_measure statement references unbound component {component}."
            ));
        }
    }
    Ok(())
}

fn extract_call_arguments(statement: &str, designator: char) -> Vec<&str> {
    let bytes = statement.as_bytes();
    let mut arguments = Vec::new();
    let needle = designator.to_ascii_lowercase() as u8;
    let mut index = 0;
    while index + 2 <= bytes.len() {
        if bytes[index].to_ascii_lowercase() == needle && bytes.get(index + 1) == Some(&b'(') {
            let start = index + 2;
            if let Some(end_offset) = statement[start..].find(')') {
                arguments.push(&statement[start..start + end_offset]);
                index = start + end_offset + 1;
                continue;
            }
        }
        index += 1;
    }
    arguments
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
