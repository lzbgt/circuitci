use crate::board_ir::{
    AnalogRelation, AnalogScenario, AnalogSensitivityAssertion, AnalogSensitivityMetric, Scenario,
};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_SENSITIVITY_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_util::normalize_artifact_path;

pub(super) fn validate_sensitivity_assertion_contract(
    analog: &AnalogScenario,
) -> Result<(), String> {
    let mode = analog.analysis.sensitivity_mode.as_deref().unwrap_or("");
    let mut names = BTreeSet::new();
    for assertion in &analog.analysis.sensitivity_assertions {
        let name = assertion.name.trim();
        if name.is_empty() {
            return Err(
                "analog_sensitivity sensitivity_assertions entries require non-empty name."
                    .to_string(),
            );
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "analog_sensitivity declares duplicate sensitivity_assertions name {name}."
            ));
        }
        if assertion.parameter.trim().is_empty() {
            return Err(format!(
                "analog_sensitivity sensitivity_assertion {name} requires non-empty parameter."
            ));
        }
        if assertion.frequency_hz.is_some() && mode == "dc" {
            return Err(format!(
                "analog_sensitivity dc sensitivity_assertion {name} must omit frequency_hz."
            ));
        }
        if assertion
            .frequency_hz
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(format!(
                "analog_sensitivity sensitivity_assertion {name} frequency_hz must be finite and > 0."
            ));
        }
        if !assertion.threshold.is_finite() {
            return Err(format!(
                "analog_sensitivity sensitivity_assertion {name} requires finite threshold."
            ));
        }
        if assertion
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(format!(
                "analog_sensitivity sensitivity_assertion {name} unit must be non-empty when provided."
            ));
        }
    }
    Ok(())
}

pub(super) fn evaluate_sensitivity_assertions(
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before sensitivity assertions");
    if analog.analysis.sensitivity_assertions.is_empty() {
        return Vec::new();
    }
    let rows = match read_sensitivity_summary(summary) {
        Ok(rows) => rows,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_SENSITIVITY_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.sensitivity_assertions {
        let Some(row) = selected_row(assertion, &rows, scenario, summary, findings) else {
            continue;
        };
        let measured = metric_value(assertion.metric, row);
        let evaluation = evaluate_sensitivity_assertion_value(assertion, measured);
        let unit = sensitivity_assertion_unit(assertion);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: row.parameter.clone(),
            measured,
            limit: assertion.threshold,
            margin: evaluation.margin,
            relation: evaluation.relation,
            unit: unit.to_string(),
            quantity: format!("sensitivity {}", metric_name(assertion.metric)),
            passed: evaluation.passed,
        });
        push_sensitivity_assertion_finding(scenario, assertion, row, summary, evaluation, findings);
    }
    measurements
}

#[derive(Debug, Clone)]
struct SensitivitySummaryRow {
    output_expression: String,
    mode: String,
    parameter: String,
    frequency_hz: Option<f64>,
    sensitivity_real: f64,
    sensitivity_imaginary: f64,
    sensitivity_magnitude: f64,
}

#[derive(Debug, Clone, Copy)]
struct SensitivityAssertionEvaluation {
    relation: &'static str,
    margin: f64,
    passed: bool,
}

fn selected_row<'a>(
    assertion: &AnalogSensitivityAssertion,
    rows: &'a [SensitivitySummaryRow],
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Option<&'a SensitivitySummaryRow> {
    let mut matching: Vec<&SensitivitySummaryRow> = rows
        .iter()
        .filter(|row| row.parameter.eq_ignore_ascii_case(&assertion.parameter))
        .collect();
    if let Some(target_frequency_hz) = assertion.frequency_hz {
        matching.retain(|row| {
            row.frequency_hz
                .is_some_and(|frequency| frequencies_match(frequency, target_frequency_hz))
        });
    }
    match matching.as_slice() {
        [row] => Some(*row),
        [] => {
            push_missing_sensitivity_row_finding(scenario, assertion, summary, findings);
            None
        }
        _ => {
            push_ambiguous_sensitivity_row_finding(scenario, assertion, summary, findings);
            None
        }
    }
}

fn frequencies_match(actual: f64, expected: f64) -> bool {
    let scale = actual.abs().max(expected.abs()).max(1.0);
    (actual - expected).abs() <= scale * 1.0e-9
}

fn evaluate_sensitivity_assertion_value(
    assertion: &AnalogSensitivityAssertion,
    measured: f64,
) -> SensitivityAssertionEvaluation {
    match assertion.relation {
        AnalogRelation::Above => SensitivityAssertionEvaluation {
            relation: "above",
            margin: measured - assertion.threshold,
            passed: measured > assertion.threshold,
        },
        AnalogRelation::Below => SensitivityAssertionEvaluation {
            relation: "below",
            margin: assertion.threshold - measured,
            passed: measured < assertion.threshold,
        },
    }
}

fn push_sensitivity_assertion_finding(
    scenario: &Scenario,
    assertion: &AnalogSensitivityAssertion,
    row: &SensitivitySummaryRow,
    summary: &Path,
    evaluation: SensitivityAssertionEvaluation,
    findings: &mut Vec<Finding>,
) {
    if evaluation.passed {
        return;
    }
    let unit = sensitivity_assertion_unit(assertion);
    let measured = metric_value(assertion.metric, row);
    let mut finding = Finding::critical(
        SPICE_SENSITIVITY_ANALYSIS,
        &scenario.name,
        format!(
            "Sensitivity assertion {} failed: parameter {} metric {} measured {:.6e} {}, expected {} {:.6e} {}.",
            assertion.name,
            row.parameter,
            metric_name(assertion.metric),
            measured,
            unit,
            evaluation.relation,
            assertion.threshold,
            unit
        ),
    );
    insert_row_metadata(assertion, row, summary, &mut finding);
    finding
        .measured
        .insert("value".to_string(), json!(measured));
    finding.measured.insert("unit".to_string(), json!(unit));
    finding
        .measured
        .insert("margin".to_string(), json!(evaluation.margin));
    finding.limit.insert(
        format!("{}_threshold", evaluation.relation),
        json!(assertion.threshold),
    );
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Adjust component values, bias, compensation, loading, or model parameters so the normalized sensitivity row meets the declared limit."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.clone());
    }
    findings.push(finding);
}

fn push_missing_sensitivity_row_finding(
    scenario: &Scenario,
    assertion: &AnalogSensitivityAssertion,
    summary: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_SENSITIVITY_ANALYSIS,
        &scenario.name,
        format!(
            "Sensitivity assertion {} references missing normalized parameter {}.",
            assertion.name, assertion.parameter
        ),
    );
    insert_missing_metadata(assertion, summary, &mut finding);
    finding.limit.insert(
        "required_parameter".to_string(),
        json!(&assertion.parameter),
    );
    findings.push(finding);
}

fn push_ambiguous_sensitivity_row_finding(
    scenario: &Scenario,
    assertion: &AnalogSensitivityAssertion,
    summary: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_SENSITIVITY_ANALYSIS,
        &scenario.name,
        format!(
            "Sensitivity assertion {} must set frequency_hz because multiple rows matched parameter {}.",
            assertion.name, assertion.parameter
        ),
    );
    insert_missing_metadata(assertion, summary, &mut finding);
    finding
        .limit
        .insert("required_field".to_string(), json!("frequency_hz"));
    findings.push(finding);
}

fn insert_row_metadata(
    assertion: &AnalogSensitivityAssertion,
    row: &SensitivitySummaryRow,
    summary: &Path,
    finding: &mut Finding,
) {
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding
        .measured
        .insert("parameter".to_string(), json!(&row.parameter));
    finding
        .measured
        .insert("metric".to_string(), json!(metric_name(assertion.metric)));
    finding.measured.insert(
        "output_expression".to_string(),
        json!(&row.output_expression),
    );
    finding
        .measured
        .insert("mode".to_string(), json!(&row.mode));
    finding
        .measured
        .insert("frequency_hz".to_string(), json!(row.frequency_hz));
    finding
        .measured
        .insert("sensitivity_real".to_string(), json!(row.sensitivity_real));
    finding.measured.insert(
        "sensitivity_imaginary".to_string(),
        json!(row.sensitivity_imaginary),
    );
    finding.measured.insert(
        "sensitivity_magnitude".to_string(),
        json!(row.sensitivity_magnitude),
    );
    finding.measured.insert(
        "sensitivity_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
}

fn insert_missing_metadata(
    assertion: &AnalogSensitivityAssertion,
    summary: &Path,
    finding: &mut Finding,
) {
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding
        .measured
        .insert("parameter".to_string(), json!(&assertion.parameter));
    finding
        .measured
        .insert("metric".to_string(), json!(metric_name(assertion.metric)));
    finding
        .measured
        .insert("frequency_hz".to_string(), json!(assertion.frequency_hz));
    finding.measured.insert(
        "sensitivity_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding.suggested_fixes.push(
        "Confirm .SENS emitted the requested parameter and frequency row, or update sensitivity_assertions[] to match sensitivity_summary.csv."
            .to_string(),
    );
}

fn metric_value(metric: AnalogSensitivityMetric, row: &SensitivitySummaryRow) -> f64 {
    match metric {
        AnalogSensitivityMetric::SensitivityReal => row.sensitivity_real,
        AnalogSensitivityMetric::SensitivityImaginary => row.sensitivity_imaginary,
        AnalogSensitivityMetric::SensitivityMagnitude => row.sensitivity_magnitude,
    }
}

fn sensitivity_assertion_unit(assertion: &AnalogSensitivityAssertion) -> &str {
    assertion
        .unit
        .as_deref()
        .unwrap_or("output_unit/parameter_unit")
}

fn metric_name(metric: AnalogSensitivityMetric) -> &'static str {
    match metric {
        AnalogSensitivityMetric::SensitivityReal => "sensitivity_real",
        AnalogSensitivityMetric::SensitivityImaginary => "sensitivity_imaginary",
        AnalogSensitivityMetric::SensitivityMagnitude => "sensitivity_magnitude",
    }
}

fn read_sensitivity_summary(path: &Path) -> Result<Vec<SensitivitySummaryRow>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read normalized sensitivity summary {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "Sensitivity summary CSV has no header row.".to_string())?;
    if header
        != "output_expression,mode,parameter,frequency_hz,sensitivity_real,sensitivity_imaginary,sensitivity_magnitude"
    {
        return Err("Sensitivity summary CSV has unexpected header.".to_string());
    }
    let mut rows = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let fields = split_csv_fields(line)
            .ok_or_else(|| format!("Sensitivity summary row {} is malformed.", line_index + 2))?;
        if fields.len() != 7 {
            return Err(format!(
                "Sensitivity summary row {} has {} fields, expected 7.",
                line_index + 2,
                fields.len()
            ));
        }
        let frequency_hz = if fields[3].is_empty() {
            None
        } else {
            Some(parse_finite_f64(
                &fields[3],
                line_index + 2,
                "frequency_hz",
            )?)
        };
        let sensitivity_real = parse_finite_f64(&fields[4], line_index + 2, "sensitivity_real")?;
        let sensitivity_imaginary =
            parse_finite_f64(&fields[5], line_index + 2, "sensitivity_imaginary")?;
        let sensitivity_magnitude =
            parse_finite_f64(&fields[6], line_index + 2, "sensitivity_magnitude")?;
        rows.push(SensitivitySummaryRow {
            output_expression: fields[0].clone(),
            mode: fields[1].clone(),
            parameter: fields[2].clone(),
            frequency_hz,
            sensitivity_real,
            sensitivity_imaginary,
            sensitivity_magnitude,
        });
    }
    if rows.is_empty() {
        return Err("Sensitivity summary CSV has no parameter rows.".to_string());
    }
    Ok(rows)
}

fn parse_finite_f64(field: &str, line: usize, name: &str) -> Result<f64, String> {
    let value = field
        .parse::<f64>()
        .map_err(|_| format!("Sensitivity summary row {line} has invalid {name}."))?;
    if !value.is_finite() {
        return Err(format!(
            "Sensitivity summary row {line} has non-finite {name}."
        ));
    }
    Ok(value)
}

fn split_csv_fields(line: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                fields.push(field.trim().to_string());
                field = String::new();
            }
            _ => field.push(ch),
        }
    }
    if quoted {
        return None;
    }
    fields.push(field.trim().to_string());
    Some(fields)
}
