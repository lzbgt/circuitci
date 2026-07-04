use crate::board_ir::{AnalogDistortionAssertion, AnalogRelation, AnalogScenario, Scenario};
use crate::reports::Finding;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::SPICE_DISTORTION_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_util::normalize_artifact_path;

pub(super) fn validate_distortion_assertion_contract(
    analog: &AnalogScenario,
) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for assertion in &analog.analysis.distortion_assertions {
        let name = assertion.name.trim();
        if name.is_empty() {
            return Err(
                "analog_distortion distortion_assertions entries require non-empty name."
                    .to_string(),
            );
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "analog_distortion declares duplicate distortion_assertions name {name}."
            ));
        }
        if assertion.component.trim().is_empty() {
            return Err(format!(
                "analog_distortion distortion_assertion {name} requires non-empty component."
            ));
        }
        if !assertion.threshold.is_finite() || assertion.threshold < 0.0 {
            return Err(format!(
                "analog_distortion distortion_assertion {name} requires finite non-negative threshold."
            ));
        }
        if assertion
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(format!(
                "analog_distortion distortion_assertion {name} unit must be non-empty when provided."
            ));
        }
    }
    Ok(())
}

pub(super) fn evaluate_distortion_assertions(
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before distortion assertions");
    if analog.analysis.distortion_assertions.is_empty() {
        return Vec::new();
    }
    let rows = match read_distortion_summary(summary) {
        Ok(rows) => rows,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_DISTORTION_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.distortion_assertions {
        let Some(row) = rows.get(&assertion.component.to_ascii_lowercase()) else {
            let mut finding = Finding::critical(
                SPICE_DISTORTION_ANALYSIS,
                &scenario.name,
                format!(
                    "Distortion assertion {} references missing normalized distortion component {}.",
                    assertion.name, assertion.component
                ),
            );
            finding
                .measured
                .insert("assertion".to_string(), json!(&assertion.name));
            finding
                .measured
                .insert("component".to_string(), json!(&assertion.component));
            finding.measured.insert(
                "distortion_summary".to_string(),
                json!(normalize_artifact_path(summary)),
            );
            finding.limit.insert(
                "required_component".to_string(),
                json!(&assertion.component),
            );
            finding.suggested_fixes.push(
                "Confirm the requested distortion mode produces this component, or update distortion_assertions[] to a component present in distortion_summary.csv."
                    .to_string(),
            );
            findings.push(finding);
            continue;
        };
        let evaluation = evaluate_distortion_assertion_value(assertion, row.max_magnitude);
        let unit = distortion_assertion_unit(assertion);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: assertion.component.clone(),
            measured: row.max_magnitude,
            limit: assertion.threshold,
            margin: evaluation.margin,
            relation: evaluation.relation,
            unit: unit.to_string(),
            quantity: "distortion max magnitude".to_string(),
            passed: evaluation.passed,
        });
        push_distortion_assertion_finding(scenario, assertion, row, summary, evaluation, findings);
    }
    measurements
}

#[derive(Debug, Clone)]
struct DistortionSummaryRow {
    output_expression: String,
    row_count: u64,
    max_magnitude: f64,
    frequency_hz_at_max: f64,
}

#[derive(Debug, Clone, Copy)]
struct DistortionAssertionEvaluation {
    relation: &'static str,
    margin: f64,
    passed: bool,
}

fn evaluate_distortion_assertion_value(
    assertion: &AnalogDistortionAssertion,
    measured: f64,
) -> DistortionAssertionEvaluation {
    match assertion.relation {
        AnalogRelation::Above => DistortionAssertionEvaluation {
            relation: "above",
            margin: measured - assertion.threshold,
            passed: measured > assertion.threshold,
        },
        AnalogRelation::Below => DistortionAssertionEvaluation {
            relation: "below",
            margin: assertion.threshold - measured,
            passed: measured < assertion.threshold,
        },
    }
}

fn push_distortion_assertion_finding(
    scenario: &Scenario,
    assertion: &AnalogDistortionAssertion,
    row: &DistortionSummaryRow,
    summary: &Path,
    evaluation: DistortionAssertionEvaluation,
    findings: &mut Vec<Finding>,
) {
    if evaluation.passed {
        return;
    }
    let unit = distortion_assertion_unit(assertion);
    let mut finding = Finding::critical(
        SPICE_DISTORTION_ANALYSIS,
        &scenario.name,
        format!(
            "Distortion assertion {} failed: component {} max magnitude {:.6e} {}, expected {} {:.6e} {}.",
            assertion.name,
            assertion.component,
            row.max_magnitude,
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
        .insert("component".to_string(), json!(&assertion.component));
    finding.measured.insert(
        "output_expression".to_string(),
        json!(&row.output_expression),
    );
    finding
        .measured
        .insert("max_magnitude".to_string(), json!(row.max_magnitude));
    finding.measured.insert(
        "frequency_hz_at_max".to_string(),
        json!(row.frequency_hz_at_max),
    );
    finding
        .measured
        .insert("row_count".to_string(), json!(row.row_count));
    finding.measured.insert("unit".to_string(), json!(unit));
    finding
        .measured
        .insert("margin".to_string(), json!(evaluation.margin));
    finding.measured.insert(
        "distortion_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding.limit.insert(
        format!("{}_threshold", evaluation.relation),
        json!(assertion.threshold),
    );
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Adjust bias, stimulus amplitudes, filtering, device sizing, or model parameters so the normalized distortion component meets the declared limit."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.clone());
    }
    findings.push(finding);
}

fn distortion_assertion_unit(assertion: &AnalogDistortionAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or("ratio")
}

fn read_distortion_summary(path: &Path) -> Result<BTreeMap<String, DistortionSummaryRow>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read normalized distortion summary {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| format!("Distortion summary CSV {} is empty.", path.display()))?;
    let columns = parse_csv_line(header);
    let component_index = column_index(&columns, "component", path)?;
    let expression_index = column_index(&columns, "output_expression", path)?;
    let row_count_index = column_index(&columns, "row_count", path)?;
    let magnitude_index = column_index(&columns, "max_magnitude", path)?;
    let frequency_index = column_index(&columns, "frequency_hz_at_max", path)?;
    let mut rows = BTreeMap::new();
    for (line_index, line) in lines.enumerate() {
        let fields = parse_csv_line(line);
        if fields.len() != columns.len() {
            return Err(format!(
                "Distortion summary {} row {} has {} columns, expected {}.",
                path.display(),
                line_index + 2,
                fields.len(),
                columns.len()
            ));
        }
        let component = fields[component_index].trim();
        if component.is_empty() {
            return Err(format!(
                "Distortion summary {} row {} has empty component.",
                path.display(),
                line_index + 2
            ));
        }
        let row_count = parse_u64(&fields[row_count_index], "row_count", path, line_index + 2)?;
        let max_magnitude = parse_f64(
            &fields[magnitude_index],
            "max_magnitude",
            path,
            line_index + 2,
        )?;
        let frequency_hz_at_max = parse_f64(
            &fields[frequency_index],
            "frequency_hz_at_max",
            path,
            line_index + 2,
        )?;
        rows.insert(
            component.to_ascii_lowercase(),
            DistortionSummaryRow {
                output_expression: fields[expression_index].trim().to_string(),
                row_count,
                max_magnitude,
                frequency_hz_at_max,
            },
        );
    }
    Ok(rows)
}

fn column_index(columns: &[String], name: &str, path: &Path) -> Result<usize, String> {
    columns
        .iter()
        .position(|column| column == name)
        .ok_or_else(|| {
            format!(
                "Distortion summary CSV {} is missing required column {name}.",
                path.display()
            )
        })
}

fn parse_u64(value: &str, column: &str, path: &Path, row: usize) -> Result<u64, String> {
    value.trim().parse::<u64>().map_err(|error| {
        format!(
            "Distortion summary {} row {} has invalid {column} value {}: {error}",
            path.display(),
            row,
            value.trim()
        )
    })
}

fn parse_f64(value: &str, column: &str, path: &Path, row: usize) -> Result<f64, String> {
    let parsed = value.trim().parse::<f64>().map_err(|error| {
        format!(
            "Distortion summary {} row {} has invalid {column} value {}: {error}",
            path.display(),
            row,
            value.trim()
        )
    })?;
    if !parsed.is_finite() {
        return Err(format!(
            "Distortion summary {} row {} has non-finite {column} value {}.",
            path.display(),
            row,
            value.trim()
        ));
    }
    Ok(parsed)
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                fields.push(field);
                field = String::new();
            }
            _ => field.push(ch),
        }
    }
    fields.push(field);
    fields
}
