use crate::board_ir::{
    AnalogPoleZeroAssertion, AnalogPoleZeroMetric, AnalogPoleZeroRootKind, AnalogRelation,
    AnalogScenario, Scenario,
};
use crate::reports::Finding;
use serde_json::json;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use super::SPICE_POLE_ZERO_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_util::normalize_artifact_path;

pub(super) fn validate_pole_zero_assertion_contract(analog: &AnalogScenario) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for assertion in &analog.analysis.pole_zero_assertions {
        let name = assertion.name.trim();
        if name.is_empty() {
            return Err(
                "analog_pole_zero pole_zero_assertions entries require non-empty name.".to_string(),
            );
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "analog_pole_zero declares duplicate pole_zero_assertions name {name}."
            ));
        }
        if assertion.root_index == Some(0) {
            return Err(format!(
                "analog_pole_zero pole_zero_assertion {name} root_index must be >= 1."
            ));
        }
        if !assertion.threshold.is_finite() {
            return Err(format!(
                "analog_pole_zero pole_zero_assertion {name} requires finite threshold."
            ));
        }
        if assertion
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(format!(
                "analog_pole_zero pole_zero_assertion {name} unit must be non-empty when provided."
            ));
        }
    }
    Ok(())
}

pub(super) fn evaluate_pole_zero_assertions(
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before pole-zero assertions");
    if analog.analysis.pole_zero_assertions.is_empty() {
        return Vec::new();
    }
    let rows = match read_pole_zero_summary(summary) {
        Ok(rows) => rows,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_POLE_ZERO_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let by_key: HashMap<(AnalogPoleZeroRootKind, u32), &PoleZeroSummaryRow> = rows
        .iter()
        .map(|row| ((row.root_kind, row.root_index), row))
        .collect();
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.pole_zero_assertions {
        let Some(row) = selected_row(assertion, &rows, &by_key, scenario, summary, findings) else {
            continue;
        };
        let measured = metric_value(assertion.metric, row);
        let evaluation = evaluate_pole_zero_assertion_value(assertion, measured);
        let unit = pole_zero_assertion_unit(assertion);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: format!("{}_{}", root_kind_name(row.root_kind), row.root_index),
            measured,
            limit: assertion.threshold,
            margin: evaluation.margin,
            relation: evaluation.relation,
            unit: unit.to_string(),
            quantity: format!("pole_zero {}", metric_name(assertion.metric)),
            passed: evaluation.passed,
        });
        push_pole_zero_assertion_finding(scenario, assertion, row, summary, evaluation, findings);
    }
    measurements
}

#[derive(Debug, Clone)]
struct PoleZeroSummaryRow {
    output_node: String,
    reference_node: String,
    input_source: String,
    mode: String,
    root_kind: AnalogPoleZeroRootKind,
    root_index: u32,
    real_rad_per_s: f64,
    imaginary_rad_per_s: f64,
    frequency_hz: f64,
}

#[derive(Debug, Clone, Copy)]
struct PoleZeroAssertionEvaluation {
    relation: &'static str,
    margin: f64,
    passed: bool,
}

fn selected_row<'a>(
    assertion: &AnalogPoleZeroAssertion,
    rows: &'a [PoleZeroSummaryRow],
    by_key: &HashMap<(AnalogPoleZeroRootKind, u32), &'a PoleZeroSummaryRow>,
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Option<&'a PoleZeroSummaryRow> {
    if let Some(root_index) = assertion.root_index {
        let Some(row) = by_key.get(&(assertion.root_kind, root_index)).copied() else {
            push_missing_pole_zero_root_finding(scenario, assertion, root_index, summary, findings);
            return None;
        };
        return Some(row);
    }
    let mut matching = rows
        .iter()
        .filter(|row| row.root_kind == assertion.root_kind);
    let Some(row) = matching.next() else {
        push_missing_pole_zero_kind_finding(scenario, assertion, summary, findings);
        return None;
    };
    if matching.next().is_some() {
        push_ambiguous_pole_zero_root_finding(scenario, assertion, summary, findings);
        return None;
    }
    Some(row)
}

fn evaluate_pole_zero_assertion_value(
    assertion: &AnalogPoleZeroAssertion,
    measured: f64,
) -> PoleZeroAssertionEvaluation {
    match assertion.relation {
        AnalogRelation::Above => PoleZeroAssertionEvaluation {
            relation: "above",
            margin: measured - assertion.threshold,
            passed: measured > assertion.threshold,
        },
        AnalogRelation::Below => PoleZeroAssertionEvaluation {
            relation: "below",
            margin: assertion.threshold - measured,
            passed: measured < assertion.threshold,
        },
    }
}

fn push_pole_zero_assertion_finding(
    scenario: &Scenario,
    assertion: &AnalogPoleZeroAssertion,
    row: &PoleZeroSummaryRow,
    summary: &Path,
    evaluation: PoleZeroAssertionEvaluation,
    findings: &mut Vec<Finding>,
) {
    if evaluation.passed {
        return;
    }
    let unit = pole_zero_assertion_unit(assertion);
    let measured = metric_value(assertion.metric, row);
    let mut finding = Finding::critical(
        SPICE_POLE_ZERO_ANALYSIS,
        &scenario.name,
        format!(
            "Pole-zero assertion {} failed: {} {} metric {} measured {:.6e} {}, expected {} {:.6e} {}.",
            assertion.name,
            root_kind_name(row.root_kind),
            row.root_index,
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
    finding.measured.insert(
        "root_kind".to_string(),
        json!(root_kind_name(row.root_kind)),
    );
    finding
        .measured
        .insert("root_index".to_string(), json!(row.root_index));
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
    finding
        .measured
        .insert("output_node".to_string(), json!(&row.output_node));
    finding
        .measured
        .insert("reference_node".to_string(), json!(&row.reference_node));
    finding
        .measured
        .insert("input_source".to_string(), json!(&row.input_source));
    finding
        .measured
        .insert("mode".to_string(), json!(&row.mode));
    finding
        .measured
        .insert("real_rad_per_s".to_string(), json!(row.real_rad_per_s));
    finding.measured.insert(
        "imaginary_rad_per_s".to_string(),
        json!(row.imaginary_rad_per_s),
    );
    finding
        .measured
        .insert("frequency_hz".to_string(), json!(row.frequency_hz));
    finding.measured.insert(
        "pole_zero_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding.limit.insert(
        format!("{}_threshold", evaluation.relation),
        json!(assertion.threshold),
    );
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Adjust compensation, feedback elements, parasitics, loading, or model parameters so the normalized pole-zero roots meet the declared limit."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.clone());
    }
    findings.push(finding);
}

fn push_missing_pole_zero_root_finding(
    scenario: &Scenario,
    assertion: &AnalogPoleZeroAssertion,
    root_index: u32,
    summary: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_POLE_ZERO_ANALYSIS,
        &scenario.name,
        format!(
            "Pole-zero assertion {} references missing normalized {} root {}.",
            assertion.name,
            root_kind_name(assertion.root_kind),
            root_index
        ),
    );
    insert_missing_root_metadata(assertion, summary, &mut finding);
    finding
        .limit
        .insert("required_root_index".to_string(), json!(root_index));
    findings.push(finding);
}

fn push_missing_pole_zero_kind_finding(
    scenario: &Scenario,
    assertion: &AnalogPoleZeroAssertion,
    summary: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_POLE_ZERO_ANALYSIS,
        &scenario.name,
        format!(
            "Pole-zero assertion {} references missing normalized {} root.",
            assertion.name,
            root_kind_name(assertion.root_kind)
        ),
    );
    insert_missing_root_metadata(assertion, summary, &mut finding);
    finding.limit.insert(
        "required_root_kind".to_string(),
        json!(root_kind_name(assertion.root_kind)),
    );
    findings.push(finding);
}

fn push_ambiguous_pole_zero_root_finding(
    scenario: &Scenario,
    assertion: &AnalogPoleZeroAssertion,
    summary: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_POLE_ZERO_ANALYSIS,
        &scenario.name,
        format!(
            "Pole-zero assertion {} must set root_index because multiple {} roots were emitted.",
            assertion.name,
            root_kind_name(assertion.root_kind)
        ),
    );
    insert_missing_root_metadata(assertion, summary, &mut finding);
    finding
        .limit
        .insert("required_field".to_string(), json!("root_index"));
    findings.push(finding);
}

fn insert_missing_root_metadata(
    assertion: &AnalogPoleZeroAssertion,
    summary: &Path,
    finding: &mut Finding,
) {
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding.measured.insert(
        "root_kind".to_string(),
        json!(root_kind_name(assertion.root_kind)),
    );
    finding
        .measured
        .insert("metric".to_string(), json!(metric_name(assertion.metric)));
    finding.measured.insert(
        "pole_zero_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding.suggested_fixes.push(
        "Confirm the requested .PZ mode emits the expected root, or update pole_zero_assertions[] to match a root present in pole_zero_summary.csv."
            .to_string(),
    );
}

fn metric_value(metric: AnalogPoleZeroMetric, row: &PoleZeroSummaryRow) -> f64 {
    match metric {
        AnalogPoleZeroMetric::RealRadPerS => row.real_rad_per_s,
        AnalogPoleZeroMetric::ImaginaryRadPerS => row.imaginary_rad_per_s,
        AnalogPoleZeroMetric::FrequencyHz => row.frequency_hz,
    }
}

fn pole_zero_assertion_unit(assertion: &AnalogPoleZeroAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or(match assertion.metric {
        AnalogPoleZeroMetric::RealRadPerS | AnalogPoleZeroMetric::ImaginaryRadPerS => "rad/s",
        AnalogPoleZeroMetric::FrequencyHz => "Hz",
    })
}

fn metric_name(metric: AnalogPoleZeroMetric) -> &'static str {
    match metric {
        AnalogPoleZeroMetric::RealRadPerS => "real_rad_per_s",
        AnalogPoleZeroMetric::ImaginaryRadPerS => "imaginary_rad_per_s",
        AnalogPoleZeroMetric::FrequencyHz => "frequency_hz",
    }
}

fn root_kind_name(kind: AnalogPoleZeroRootKind) -> &'static str {
    match kind {
        AnalogPoleZeroRootKind::Pole => "pole",
        AnalogPoleZeroRootKind::Zero => "zero",
    }
}

fn read_pole_zero_summary(path: &Path) -> Result<Vec<PoleZeroSummaryRow>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read normalized pole-zero summary {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "Pole-zero summary CSV has no header row.".to_string())?;
    if header
        != "output_node,reference_node,input_source,mode,root_kind,root_index,real_rad_per_s,imaginary_rad_per_s,frequency_hz"
    {
        return Err("Pole-zero summary CSV has unexpected header.".to_string());
    }
    let mut rows = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let fields = split_csv_fields(line)
            .ok_or_else(|| format!("Pole-zero summary row {} is malformed.", line_index + 2))?;
        if fields.len() != 9 {
            return Err(format!(
                "Pole-zero summary row {} has {} fields, expected 9.",
                line_index + 2,
                fields.len()
            ));
        }
        let root_kind = match fields[4].as_str() {
            "pole" => AnalogPoleZeroRootKind::Pole,
            "zero" => AnalogPoleZeroRootKind::Zero,
            other => {
                return Err(format!(
                    "Pole-zero summary row {} has invalid root kind {other}.",
                    line_index + 2
                ));
            }
        };
        let root_index = fields[5].parse::<u32>().map_err(|_| {
            format!(
                "Pole-zero summary row {} has invalid root index.",
                line_index + 2
            )
        })?;
        let real_rad_per_s = parse_finite_f64(&fields[6], line_index + 2, "real_rad_per_s")?;
        let imaginary_rad_per_s =
            parse_finite_f64(&fields[7], line_index + 2, "imaginary_rad_per_s")?;
        let frequency_hz = parse_finite_f64(&fields[8], line_index + 2, "frequency_hz")?;
        rows.push(PoleZeroSummaryRow {
            output_node: fields[0].clone(),
            reference_node: fields[1].clone(),
            input_source: fields[2].clone(),
            mode: fields[3].clone(),
            root_kind,
            root_index,
            real_rad_per_s,
            imaginary_rad_per_s,
            frequency_hz,
        });
    }
    if rows.is_empty() {
        return Err("Pole-zero summary CSV has no root rows.".to_string());
    }
    Ok(rows)
}

fn parse_finite_f64(field: &str, line: usize, name: &str) -> Result<f64, String> {
    let value = field
        .parse::<f64>()
        .map_err(|_| format!("Pole-zero summary row {line} has invalid {name}."))?;
    if !value.is_finite() {
        return Err(format!(
            "Pole-zero summary row {line} has non-finite {name}."
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
