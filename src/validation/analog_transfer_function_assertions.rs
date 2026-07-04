use crate::board_ir::{
    AnalogRelation, AnalogScenario, AnalogTransferFunctionAssertion, AnalogTransferFunctionMetric,
    Scenario,
};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_TRANSFER_FUNCTION_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_util::normalize_artifact_path;

pub(super) fn validate_transfer_function_assertion_contract(
    analog: &AnalogScenario,
) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for assertion in &analog.analysis.transfer_function_assertions {
        let name = assertion.name.trim();
        if name.is_empty() {
            return Err(
                "analog_transfer_function transfer_function_assertions entries require non-empty name."
                    .to_string(),
            );
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "analog_transfer_function declares duplicate transfer_function_assertions name {name}."
            ));
        }
        if !assertion.threshold.is_finite() {
            return Err(format!(
                "analog_transfer_function transfer_function_assertion {name} requires finite threshold."
            ));
        }
        if assertion
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(format!(
                "analog_transfer_function transfer_function_assertion {name} unit must be non-empty when provided."
            ));
        }
    }
    Ok(())
}

pub(super) fn evaluate_transfer_function_assertions(
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before transfer-function assertions");
    if analog.analysis.transfer_function_assertions.is_empty() {
        return Vec::new();
    }
    let row = match read_transfer_function_summary(summary) {
        Ok(row) => row,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_TRANSFER_FUNCTION_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.transfer_function_assertions {
        let measured = metric_value(assertion.metric, &row);
        let evaluation = evaluate_transfer_function_assertion_value(assertion, measured);
        let unit = transfer_function_assertion_unit(assertion);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: metric_name(assertion.metric).to_string(),
            measured,
            limit: assertion.threshold,
            margin: evaluation.margin,
            relation: evaluation.relation,
            unit: unit.to_string(),
            quantity: format!("transfer_function {}", metric_name(assertion.metric)),
            passed: evaluation.passed,
        });
        push_transfer_function_assertion_finding(
            scenario, assertion, &row, summary, evaluation, findings,
        );
    }
    measurements
}

#[derive(Debug, Clone)]
struct TransferFunctionSummaryRow {
    output_expression: String,
    input_source: String,
    transfer_function_gain: f64,
    input_resistance_ohm: f64,
    output_resistance_ohm: f64,
}

#[derive(Debug, Clone, Copy)]
struct TransferFunctionAssertionEvaluation {
    relation: &'static str,
    margin: f64,
    passed: bool,
}

fn evaluate_transfer_function_assertion_value(
    assertion: &AnalogTransferFunctionAssertion,
    measured: f64,
) -> TransferFunctionAssertionEvaluation {
    match assertion.relation {
        AnalogRelation::Above => TransferFunctionAssertionEvaluation {
            relation: "above",
            margin: measured - assertion.threshold,
            passed: measured > assertion.threshold,
        },
        AnalogRelation::Below => TransferFunctionAssertionEvaluation {
            relation: "below",
            margin: assertion.threshold - measured,
            passed: measured < assertion.threshold,
        },
    }
}

fn push_transfer_function_assertion_finding(
    scenario: &Scenario,
    assertion: &AnalogTransferFunctionAssertion,
    row: &TransferFunctionSummaryRow,
    summary: &Path,
    evaluation: TransferFunctionAssertionEvaluation,
    findings: &mut Vec<Finding>,
) {
    if evaluation.passed {
        return;
    }
    let unit = transfer_function_assertion_unit(assertion);
    let measured = metric_value(assertion.metric, row);
    let mut finding = Finding::critical(
        SPICE_TRANSFER_FUNCTION_ANALYSIS,
        &scenario.name,
        format!(
            "Transfer-function assertion {} failed: metric {} measured {:.6e} {}, expected {} {:.6e} {}.",
            assertion.name,
            metric_name(assertion.metric),
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
    insert_row_metadata(row, summary, &mut finding);
    finding
        .measured
        .insert("metric".to_string(), json!(metric_name(assertion.metric)));
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
            "Adjust bias, loading, source resistance, feedback, compensation, or component values so the normalized .TF summary meets the declared limit."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.clone());
    }
    findings.push(finding);
}

fn insert_row_metadata(row: &TransferFunctionSummaryRow, summary: &Path, finding: &mut Finding) {
    finding.measured.insert(
        "output_expression".to_string(),
        json!(&row.output_expression),
    );
    finding
        .measured
        .insert("input_source".to_string(), json!(&row.input_source));
    finding.measured.insert(
        "transfer_function_gain".to_string(),
        json!(row.transfer_function_gain),
    );
    finding.measured.insert(
        "input_resistance_ohm".to_string(),
        json!(row.input_resistance_ohm),
    );
    finding.measured.insert(
        "output_resistance_ohm".to_string(),
        json!(row.output_resistance_ohm),
    );
    finding.measured.insert(
        "transfer_function_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
}

fn metric_value(metric: AnalogTransferFunctionMetric, row: &TransferFunctionSummaryRow) -> f64 {
    match metric {
        AnalogTransferFunctionMetric::TransferFunctionGain => row.transfer_function_gain,
        AnalogTransferFunctionMetric::InputResistanceOhm => row.input_resistance_ohm,
        AnalogTransferFunctionMetric::OutputResistanceOhm => row.output_resistance_ohm,
    }
}

fn transfer_function_assertion_unit(assertion: &AnalogTransferFunctionAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or(match assertion.metric {
        AnalogTransferFunctionMetric::TransferFunctionGain => "ratio",
        AnalogTransferFunctionMetric::InputResistanceOhm
        | AnalogTransferFunctionMetric::OutputResistanceOhm => "ohm",
    })
}

fn metric_name(metric: AnalogTransferFunctionMetric) -> &'static str {
    match metric {
        AnalogTransferFunctionMetric::TransferFunctionGain => "transfer_function_gain",
        AnalogTransferFunctionMetric::InputResistanceOhm => "input_resistance_ohm",
        AnalogTransferFunctionMetric::OutputResistanceOhm => "output_resistance_ohm",
    }
}

fn read_transfer_function_summary(path: &Path) -> Result<TransferFunctionSummaryRow, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read normalized transfer-function summary {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "Transfer-function summary CSV has no header row.".to_string())?;
    if header
        != "output_expression,input_source,transfer_function_gain,input_resistance_ohm,output_resistance_ohm"
    {
        return Err("Transfer-function summary CSV has unexpected header.".to_string());
    }
    let row = lines
        .next()
        .ok_or_else(|| "Transfer-function summary CSV has no scalar row.".to_string())?;
    if lines.next().is_some() {
        return Err("Transfer-function summary CSV has multiple scalar rows.".to_string());
    }
    let fields = split_csv_fields(row)
        .ok_or_else(|| "Transfer-function summary row is malformed.".to_string())?;
    if fields.len() != 5 {
        return Err(format!(
            "Transfer-function summary row has {} fields, expected 5.",
            fields.len()
        ));
    }
    Ok(TransferFunctionSummaryRow {
        output_expression: fields[0].clone(),
        input_source: fields[1].clone(),
        transfer_function_gain: parse_finite_f64(&fields[2], "transfer_function_gain")?,
        input_resistance_ohm: parse_finite_f64(&fields[3], "input_resistance_ohm")?,
        output_resistance_ohm: parse_finite_f64(&fields[4], "output_resistance_ohm")?,
    })
}

fn parse_finite_f64(field: &str, name: &str) -> Result<f64, String> {
    let value = field
        .parse::<f64>()
        .map_err(|_| format!("Transfer-function summary row has invalid {name}."))?;
    if !value.is_finite() {
        return Err(format!(
            "Transfer-function summary row has non-finite {name}."
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
