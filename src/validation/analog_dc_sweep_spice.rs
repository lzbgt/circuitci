use crate::board_ir::{
    AnalogDcSweepAggregation, AnalogDcSweepAssertion, AnalogNetlistSource, AnalogQuantity, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::SPICE_DC_SWEEP_ANALYSIS;
use super::analog_assertions::{AnalogAssertionMeasurement, validate_probe_contract};
use super::analog_backend_plan::{UnsupportedBackendPlan, unsupported_backend_plan_finding};
use super::analog_dc_sweep_runner::{NgspiceDcSweepRunOptions, run_ngspice_dc_sweep};
use super::analog_dc_sweep_xyce_runner::{XyceDcSweepRunOptions, run_xyce_dc_sweep};
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
    file_sha256_hex, normalize_artifact_path, push_artifact, safe_artifact_name,
};
use super::common::validation_input_missing;

pub(super) struct AnalogDcSweepSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
}

pub(super) fn validate_spice_dc_sweep_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogDcSweepSinks<'_>,
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
        "Preparing analog DC sweep",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_dc_sweep scenario requires an analog block.",
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
                    "SPICE model file {} is required for physical analog DC sweep simulation.",
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
            "analog_dc_sweep requires node_bindings and pin_bindings.",
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
                    "Analog DC sweep node binding {} references unknown board net {}.",
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
                    "Analog DC sweep pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "dc_sweep" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only dc_sweep is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    let Some(source) = nonempty(analog.analysis.dc_sweep_source.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_dc_sweep requires dc_sweep_source.",
        );
        return;
    };
    if analog.netlist_source == AnalogNetlistSource::GeneratedFromBoard
        && !bound.project.board.components.contains_key(source)
    {
        validation_input_missing(
            findings,
            scenario,
            format!("analog_dc_sweep source {source} is not a generated board component."),
        );
        return;
    }
    let Some((start, stop, step)) = validate_sweep_range(scenario, findings) else {
        return;
    };
    if (stop - start).abs() < f64::EPSILON || step > (stop - start).abs() {
        validation_input_missing(
            findings,
            scenario,
            "analog_dc_sweep requires dc_sweep_start and dc_sweep_stop to differ with a step no larger than the sweep span.",
        );
        return;
    }
    if analog.probes.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SPICE_DC_SWEEP_ANALYSIS requires at least one sweep probe.",
        );
        return;
    }
    for probe in &analog.probes {
        if let Err(message) = validate_probe_contract(probe) {
            validation_input_missing(
                findings,
                scenario,
                format!("Analog DC sweep probe {} {message}.", probe.name),
            );
            return;
        }
    }
    for assertion in &analog.analysis.dc_sweep_assertions {
        if !analog
            .probes
            .iter()
            .any(|probe| probe.name == assertion.probe)
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog DC sweep assertion {} references unknown probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            return;
        }
        if let Err(message) = validate_dc_sweep_assertion_contract(assertion) {
            validation_input_missing(
                findings,
                scenario,
                format!("Analog DC sweep assertion {} {message}.", assertion.name),
            );
            return;
        }
    }
    if analog.analysis.dc_sweep_assertions.is_empty() {
        let mut finding = Finding::info(
            "ANALOG_ASSERTIONS_ABSENT",
            &scenario.name,
            "SPICE DC sweep solved and exported probes, but no quantitative DC sweep assertions were declared.",
        );
        finding.limit.insert(
            "required_for_signoff".to_string(),
            json!(
                "Add min/max/mean/sample DC sweep assertions for the sweep behavior being verified."
            ),
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
            SPICE_DC_SWEEP_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create analog DC sweep run directory {}: {error}",
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
            let mut finding = Finding::critical(SPICE_DC_SWEEP_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            findings.push(finding);
            return;
        }
    };

    on_progress(
        "Selecting analog DC sweep backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
    let selected = select_backend_for_feature(&analog.backend, AnalogRuntimeFeature::DcSweep);
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
                check_id: SPICE_DC_SWEEP_ANALYSIS,
                selected_backend: backend,
                implemented_backend: "ngspice_or_xyce",
                analysis_kind: "dc_sweep",
                required_normalized_outputs: &["dc_sweep"],
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
        on_progress(
            "Running analog DC sweep input corner",
            run_plan.progress_label(),
        );
        let parameter_overrides = run_plan.parameter_overrides_for_solver();
        let run_result = if matches!(backend, "Xyce" | "xyce") {
            run_xyce_dc_sweep(
                bound,
                scenario,
                backend,
                &source_netlist,
                XyceDcSweepRunOptions {
                    output,
                    run_subdir: run_plan.run_subdir.as_deref(),
                    parameter_overrides: &parameter_overrides,
                    model_section_overrides: &run_plan.model_section_overrides,
                    on_progress: &mut on_progress,
                    should_cancel: &should_cancel,
                },
            )
        } else {
            run_ngspice_dc_sweep(
                bound,
                scenario,
                backend,
                &source_netlist,
                NgspiceDcSweepRunOptions {
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
                let assertion_measurements =
                    evaluate_dc_sweep_assertions(scenario, &run.sweep, findings);
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
                    Finding::critical(SPICE_DC_SWEEP_ANALYSIS, &scenario.name, error.message);
                finding
                    .measured
                    .insert("selected_backend".to_string(), json!(backend));
                finding.limit.insert(
                    "required_evidence".to_string(),
                    json!(if matches!(backend, "Xyce" | "xyce") {
                        "xyce_dc_sweep_csv"
                    } else {
                        "ngspice_dc_sweep_csv"
                    }),
                );
                tag_corner_finding(&mut finding, &run_plan);
                let solver_name = if matches!(backend, "Xyce" | "xyce") {
                    "Xyce"
                } else {
                    "ngspice"
                };
                finding.suggested_fixes.push(format!(
                    "Inspect the generated {solver_name} DC sweep wrapper deck and solver log artifacts."
                ));
                findings.push(finding);
            }
        }
    }
    push_sweep_margin_summaries(findings, scenario, &sweep_measurements);
}

fn validate_sweep_range(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<(f64, f64, f64)> {
    let analog = scenario.analog.as_ref()?;
    let start = analog.analysis.dc_sweep_start;
    let stop = analog.analysis.dc_sweep_stop;
    let step = analog.analysis.dc_sweep_step;
    let Some(start) = start.filter(|value| value.is_finite()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_dc_sweep requires finite dc_sweep_start.",
        );
        return None;
    };
    let Some(stop) = stop.filter(|value| value.is_finite()) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_dc_sweep requires finite dc_sweep_stop.",
        );
        return None;
    };
    let Some(step) = step.filter(|value| value.is_finite() && *value > 0.0) else {
        validation_input_missing(
            findings,
            scenario,
            "analog_dc_sweep requires positive finite dc_sweep_step.",
        );
        return None;
    };
    Some((start, stop, step))
}

fn validate_dc_sweep_assertion_contract(assertion: &AnalogDcSweepAssertion) -> Result<(), String> {
    if !assertion.threshold.is_finite() {
        return Err("requires a finite threshold.".to_string());
    }
    match assertion.aggregation {
        AnalogDcSweepAggregation::Sample => {
            if !assertion
                .at_sweep_value
                .is_some_and(|value| value.is_finite())
            {
                return Err("sample aggregation requires finite at_sweep_value.".to_string());
            }
        }
        AnalogDcSweepAggregation::Min
        | AnalogDcSweepAggregation::Max
        | AnalogDcSweepAggregation::Mean => {
            if assertion.at_sweep_value.is_some() {
                return Err(
                    "min/max/mean aggregations must not declare at_sweep_value.".to_string()
                );
            }
        }
    }
    Ok(())
}

fn evaluate_dc_sweep_assertions(
    scenario: &Scenario,
    sweep: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before DC sweep assertions");
    if analog.analysis.dc_sweep_assertions.is_empty() {
        return Vec::new();
    }
    let rows = match read_dc_sweep_csv(sweep) {
        Ok(rows) => rows,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_DC_SWEEP_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.dc_sweep_assertions {
        let Some(probe) = analog
            .probes
            .iter()
            .find(|probe| probe.name == assertion.probe)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog DC sweep assertion {} references unknown probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            continue;
        };
        let probe_rows = rows
            .get(&assertion.probe)
            .cloned()
            .or_else(|| rows.get(&sanitize_key(&assertion.probe)).cloned());
        let Some(probe_rows) = probe_rows else {
            push_missing_probe_finding(scenario, assertion, sweep, findings);
            continue;
        };
        let Some(measured) = aggregate_dc_sweep(assertion, &probe_rows) else {
            push_missing_probe_finding(scenario, assertion, sweep, findings);
            continue;
        };
        let evaluation = evaluate_sweep_value(assertion, measured.value);
        let unit = assertion_unit(assertion, &probe.quantity);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: assertion.probe.clone(),
            measured: measured.value,
            limit: assertion.threshold,
            margin: evaluation.margin,
            relation: evaluation.relation,
            unit: unit.to_string(),
            quantity: format!("dc sweep {}", aggregation_name(assertion.aggregation)),
            passed: evaluation.passed,
        });
        if !evaluation.passed {
            let mut finding = Finding::critical(
                SPICE_DC_SWEEP_ANALYSIS,
                &scenario.name,
                format!(
                    "DC sweep assertion {} failed: {} probe {} measured {:.6} {}, expected {} {:.6} {}.",
                    assertion.name,
                    aggregation_name(assertion.aggregation),
                    assertion.probe,
                    measured.value,
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
                .insert("probe".to_string(), json!(&assertion.probe));
            finding
                .measured
                .insert("value".to_string(), json!(measured.value));
            finding
                .measured
                .insert("sweep_value".to_string(), json!(measured.sweep_value));
            finding.measured.insert("unit".to_string(), json!(unit));
            finding.measured.insert(
                "dc_sweep".to_string(),
                json!(normalize_artifact_path(sweep)),
            );
            finding.limit.insert(
                format!("{}_threshold", evaluation.relation),
                json!(assertion.threshold),
            );
            if assertion.suggested_fixes.is_empty() {
                finding.suggested_fixes.push(
                    "Adjust the swept source, load, bias network, or model so the normalized DC sweep curve meets the declared limit."
                        .to_string(),
                );
            } else {
                finding
                    .suggested_fixes
                    .extend(assertion.suggested_fixes.clone());
            }
            findings.push(finding);
        }
    }
    measurements
}

#[derive(Debug, Clone, Copy)]
struct SweepPoint {
    sweep_value: f64,
    value: f64,
}

#[derive(Debug, Clone, Copy)]
struct SweepEvaluation {
    relation: &'static str,
    margin: f64,
    passed: bool,
}

fn read_dc_sweep_csv(path: &Path) -> Result<BTreeMap<String, Vec<SweepPoint>>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read DC sweep CSV {}: {error}", path.display()))?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| format!("DC sweep CSV {} is empty.", path.display()))?;
    let columns: Vec<_> = header.split(',').map(str::trim).collect();
    let sweep_index = column_index(&columns, "sweep_value", path)?;
    let probe_index = column_index(&columns, "probe", path)?;
    let value_index = column_index(&columns, "value", path)?;
    let mut rows: BTreeMap<String, Vec<SweepPoint>> = BTreeMap::new();
    for (line_index, line) in lines.enumerate() {
        let fields: Vec<_> = line.split(',').map(str::trim).collect();
        if fields.len() != columns.len() {
            return Err(format!(
                "DC sweep CSV {} row {} has {} columns, expected {}.",
                path.display(),
                line_index + 2,
                fields.len(),
                columns.len()
            ));
        }
        let sweep_value = parse_float(fields[sweep_index]).ok_or_else(|| {
            format!(
                "DC sweep CSV {} row {} has non-numeric sweep value {}.",
                path.display(),
                line_index + 2,
                fields[sweep_index]
            )
        })?;
        let value = parse_float(fields[value_index]).ok_or_else(|| {
            format!(
                "DC sweep CSV {} row {} has non-numeric probe value {}.",
                path.display(),
                line_index + 2,
                fields[value_index]
            )
        })?;
        rows.entry(fields[probe_index].to_string())
            .or_default()
            .push(SweepPoint { sweep_value, value });
    }
    if rows.is_empty() {
        return Err(format!(
            "DC sweep CSV {} has no numeric rows.",
            path.display()
        ));
    }
    Ok(rows)
}

fn column_index(columns: &[&str], name: &str, path: &Path) -> Result<usize, String> {
    columns
        .iter()
        .position(|column| *column == name)
        .ok_or_else(|| format!("DC sweep CSV {} is missing {name} column.", path.display()))
}

fn aggregate_dc_sweep(
    assertion: &AnalogDcSweepAssertion,
    rows: &[SweepPoint],
) -> Option<SweepPoint> {
    match assertion.aggregation {
        AnalogDcSweepAggregation::Min => rows
            .iter()
            .copied()
            .min_by(|left, right| left.value.total_cmp(&right.value)),
        AnalogDcSweepAggregation::Max => rows
            .iter()
            .copied()
            .max_by(|left, right| left.value.total_cmp(&right.value)),
        AnalogDcSweepAggregation::Mean => {
            if rows.is_empty() {
                return None;
            }
            let value = rows.iter().map(|row| row.value).sum::<f64>() / rows.len() as f64;
            let sweep_value =
                rows.iter().map(|row| row.sweep_value).sum::<f64>() / rows.len() as f64;
            Some(SweepPoint { sweep_value, value })
        }
        AnalogDcSweepAggregation::Sample => {
            let target = assertion.at_sweep_value?;
            rows.iter().copied().min_by(|left, right| {
                (left.sweep_value - target)
                    .abs()
                    .total_cmp(&(right.sweep_value - target).abs())
            })
        }
    }
}

fn evaluate_sweep_value(assertion: &AnalogDcSweepAssertion, measured: f64) -> SweepEvaluation {
    match assertion.relation {
        crate::board_ir::AnalogRelation::Above => SweepEvaluation {
            relation: "above",
            margin: measured - assertion.threshold,
            passed: measured > assertion.threshold,
        },
        crate::board_ir::AnalogRelation::Below => SweepEvaluation {
            relation: "below",
            margin: assertion.threshold - measured,
            passed: measured < assertion.threshold,
        },
    }
}

fn push_missing_probe_finding(
    scenario: &Scenario,
    assertion: &AnalogDcSweepAssertion,
    sweep: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_DC_SWEEP_ANALYSIS,
        &scenario.name,
        format!(
            "DC sweep assertion {} could not be evaluated because probe {} is missing from dc_sweep.csv.",
            assertion.name, assertion.probe
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding.measured.insert(
        "dc_sweep".to_string(),
        json!(normalize_artifact_path(sweep)),
    );
    findings.push(finding);
}

fn assertion_unit<'a>(assertion: &'a AnalogDcSweepAssertion, quantity: &AnalogQuantity) -> &'a str {
    assertion.unit.as_deref().unwrap_or(match quantity {
        AnalogQuantity::Voltage => "V",
        AnalogQuantity::Current => "A",
        AnalogQuantity::Power => "W",
    })
}

fn aggregation_name(aggregation: AnalogDcSweepAggregation) -> &'static str {
    match aggregation {
        AnalogDcSweepAggregation::Min => "min",
        AnalogDcSweepAggregation::Max => "max",
        AnalogDcSweepAggregation::Mean => "mean",
        AnalogDcSweepAggregation::Sample => "sample",
    }
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn parse_float(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

fn sanitize_key(name: &str) -> String {
    let mut output = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "probe".to_string()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::validate_dc_sweep_assertion_contract;
    use crate::board_ir::{AnalogDcSweepAggregation, AnalogDcSweepAssertion, AnalogRelation};

    #[test]
    fn sample_assertion_requires_sweep_value() {
        let assertion = AnalogDcSweepAssertion {
            name: "sample".to_string(),
            probe: "out".to_string(),
            aggregation: AnalogDcSweepAggregation::Sample,
            relation: AnalogRelation::Above,
            threshold: 0.2,
            at_sweep_value: None,
            unit: None,
            suggested_fixes: Vec::new(),
        };

        assert!(validate_dc_sweep_assertion_contract(&assertion).is_err());
    }
}
