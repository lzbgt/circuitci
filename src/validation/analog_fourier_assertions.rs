use crate::board_ir::{
    AnalogFourierAssertion, AnalogFourierMetric, AnalogRelation, AnalogScenario, Scenario,
};
use crate::reports::Finding;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::SPICE_FOURIER_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_util::normalize_artifact_path;

pub(super) fn validate_fourier_assertion_contract(analog: &AnalogScenario) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for assertion in &analog.analysis.fourier_assertions {
        let name = assertion.name.trim();
        if name.is_empty() {
            return Err(
                "analog_fourier fourier_assertions entries require non-empty name.".to_string(),
            );
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "analog_fourier declares duplicate fourier_assertions name {name}."
            ));
        }
        if !assertion.threshold.is_finite() {
            return Err(format!(
                "analog_fourier fourier_assertion {name} requires finite threshold."
            ));
        }
        match assertion.metric {
            AnalogFourierMetric::ThdPercent => {
                if assertion.harmonic.is_some() {
                    return Err(format!(
                        "analog_fourier fourier_assertion {name} must omit harmonic for thd_percent."
                    ));
                }
            }
            _ => {
                let Some(harmonic) = assertion.harmonic else {
                    return Err(format!(
                        "analog_fourier fourier_assertion {name} requires harmonic for metric {:?}.",
                        assertion.metric
                    ));
                };
                if harmonic > 1024 {
                    return Err(format!(
                        "analog_fourier fourier_assertion {name} harmonic must be in 0..=1024."
                    ));
                }
            }
        }
        if assertion
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(format!(
                "analog_fourier fourier_assertion {name} unit must be non-empty when provided."
            ));
        }
    }
    Ok(())
}

pub(super) fn evaluate_fourier_assertions(
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before Fourier assertions");
    if analog.analysis.fourier_assertions.is_empty() {
        return Vec::new();
    }
    let rows = match read_fourier_summary(summary) {
        Ok(rows) => rows,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_FOURIER_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let by_harmonic: BTreeMap<u32, &FourierSummaryRow> =
        rows.iter().map(|row| (row.harmonic, row)).collect();
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.fourier_assertions {
        let Some(measured) =
            measured_value(assertion, &rows, &by_harmonic, scenario, summary, findings)
        else {
            continue;
        };
        let evaluation = evaluate_fourier_assertion_value(assertion, measured.value);
        let unit = fourier_assertion_unit(assertion);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: measured.probe_name.clone(),
            measured: measured.value,
            limit: assertion.threshold,
            margin: evaluation.margin,
            relation: evaluation.relation,
            unit: unit.to_string(),
            quantity: format!("fourier {}", metric_name(assertion.metric)),
            passed: evaluation.passed,
        });
        push_fourier_assertion_finding(
            scenario, assertion, measured, summary, evaluation, findings,
        );
    }
    measurements
}

#[derive(Debug, Clone)]
struct FourierSummaryRow {
    output_expression: String,
    harmonic: u32,
    frequency_hz: f64,
    magnitude: f64,
    phase_deg: f64,
    normalized_magnitude: f64,
    normalized_phase_deg: f64,
    thd_percent: Option<f64>,
}

#[derive(Debug, Clone)]
struct FourierMeasuredValue {
    value: f64,
    probe_name: String,
    harmonic: Option<u32>,
    frequency_hz: Option<f64>,
    output_expression: String,
}

#[derive(Debug, Clone, Copy)]
struct FourierAssertionEvaluation {
    relation: &'static str,
    margin: f64,
    passed: bool,
}

fn measured_value(
    assertion: &AnalogFourierAssertion,
    rows: &[FourierSummaryRow],
    by_harmonic: &BTreeMap<u32, &FourierSummaryRow>,
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Option<FourierMeasuredValue> {
    if assertion.metric == AnalogFourierMetric::ThdPercent {
        let Some(row) = rows.iter().find(|row| row.thd_percent.is_some()) else {
            push_missing_fourier_metric_finding(
                scenario,
                assertion,
                "thd_percent",
                summary,
                findings,
            );
            return None;
        };
        return Some(FourierMeasuredValue {
            value: row.thd_percent.unwrap_or_default(),
            probe_name: "thd_percent".to_string(),
            harmonic: None,
            frequency_hz: None,
            output_expression: row.output_expression.clone(),
        });
    }
    let harmonic = assertion
        .harmonic
        .expect("harmonic was validated before Fourier assertion evaluation");
    let Some(row) = by_harmonic.get(&harmonic).copied() else {
        push_missing_fourier_harmonic_finding(scenario, assertion, harmonic, summary, findings);
        return None;
    };
    let value = match assertion.metric {
        AnalogFourierMetric::Magnitude => row.magnitude,
        AnalogFourierMetric::NormalizedMagnitude => row.normalized_magnitude,
        AnalogFourierMetric::PhaseDeg => row.phase_deg,
        AnalogFourierMetric::NormalizedPhaseDeg => row.normalized_phase_deg,
        AnalogFourierMetric::ThdPercent => unreachable!("handled above"),
    };
    Some(FourierMeasuredValue {
        value,
        probe_name: format!("harmonic_{harmonic}"),
        harmonic: Some(harmonic),
        frequency_hz: Some(row.frequency_hz),
        output_expression: row.output_expression.clone(),
    })
}

fn evaluate_fourier_assertion_value(
    assertion: &AnalogFourierAssertion,
    measured: f64,
) -> FourierAssertionEvaluation {
    match assertion.relation {
        AnalogRelation::Above => FourierAssertionEvaluation {
            relation: "above",
            margin: measured - assertion.threshold,
            passed: measured > assertion.threshold,
        },
        AnalogRelation::Below => FourierAssertionEvaluation {
            relation: "below",
            margin: assertion.threshold - measured,
            passed: measured < assertion.threshold,
        },
    }
}

fn push_fourier_assertion_finding(
    scenario: &Scenario,
    assertion: &AnalogFourierAssertion,
    measured: FourierMeasuredValue,
    summary: &Path,
    evaluation: FourierAssertionEvaluation,
    findings: &mut Vec<Finding>,
) {
    if evaluation.passed {
        return;
    }
    let unit = fourier_assertion_unit(assertion);
    let mut finding = Finding::critical(
        SPICE_FOURIER_ANALYSIS,
        &scenario.name,
        format!(
            "Fourier assertion {} failed: {} measured {:.6e} {}, expected {} {:.6e} {}.",
            assertion.name,
            metric_name(assertion.metric),
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
        .insert("metric".to_string(), json!(metric_name(assertion.metric)));
    finding
        .measured
        .insert("value".to_string(), json!(measured.value));
    finding.measured.insert("unit".to_string(), json!(unit));
    finding
        .measured
        .insert("margin".to_string(), json!(evaluation.margin));
    finding.measured.insert(
        "output_expression".to_string(),
        json!(measured.output_expression),
    );
    if let Some(harmonic) = measured.harmonic {
        finding
            .measured
            .insert("harmonic".to_string(), json!(harmonic));
    }
    if let Some(frequency_hz) = measured.frequency_hz {
        finding
            .measured
            .insert("frequency_hz".to_string(), json!(frequency_hz));
    }
    finding.measured.insert(
        "fourier_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding.limit.insert(
        format!("{}_threshold", evaluation.relation),
        json!(assertion.threshold),
    );
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Adjust stimulus shape, filtering, load, timing, or model parameters so the normalized Fourier summary meets the declared limit."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.clone());
    }
    findings.push(finding);
}

fn push_missing_fourier_harmonic_finding(
    scenario: &Scenario,
    assertion: &AnalogFourierAssertion,
    harmonic: u32,
    summary: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_FOURIER_ANALYSIS,
        &scenario.name,
        format!(
            "Fourier assertion {} references missing normalized harmonic {}.",
            assertion.name, harmonic
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding
        .measured
        .insert("harmonic".to_string(), json!(harmonic));
    finding.measured.insert(
        "fourier_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding
        .limit
        .insert("required_harmonic".to_string(), json!(harmonic));
    finding.suggested_fixes.push(
        "Increase fourier_harmonics, confirm the backend produced that harmonic row, or update fourier_assertions[] to a harmonic present in fourier_summary.csv."
            .to_string(),
    );
    findings.push(finding);
}

fn push_missing_fourier_metric_finding(
    scenario: &Scenario,
    assertion: &AnalogFourierAssertion,
    metric: &str,
    summary: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_FOURIER_ANALYSIS,
        &scenario.name,
        format!(
            "Fourier assertion {} references missing normalized metric {}.",
            assertion.name, metric
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding.measured.insert("metric".to_string(), json!(metric));
    finding.measured.insert(
        "fourier_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding
        .limit
        .insert("required_metric".to_string(), json!(metric));
    findings.push(finding);
}

fn fourier_assertion_unit(assertion: &AnalogFourierAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or(match assertion.metric {
        AnalogFourierMetric::Magnitude => "output_unit",
        AnalogFourierMetric::NormalizedMagnitude => "ratio",
        AnalogFourierMetric::PhaseDeg | AnalogFourierMetric::NormalizedPhaseDeg => "deg",
        AnalogFourierMetric::ThdPercent => "percent",
    })
}

fn metric_name(metric: AnalogFourierMetric) -> &'static str {
    match metric {
        AnalogFourierMetric::Magnitude => "magnitude",
        AnalogFourierMetric::NormalizedMagnitude => "normalized_magnitude",
        AnalogFourierMetric::PhaseDeg => "phase_deg",
        AnalogFourierMetric::NormalizedPhaseDeg => "normalized_phase_deg",
        AnalogFourierMetric::ThdPercent => "thd_percent",
    }
}

fn read_fourier_summary(path: &Path) -> Result<Vec<FourierSummaryRow>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read normalized Fourier summary {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| format!("Fourier summary CSV {} is empty.", path.display()))?;
    let columns = parse_csv_line(header);
    let expression_index = column_index(&columns, "output_expression", path)?;
    let harmonic_index = column_index(&columns, "harmonic", path)?;
    let frequency_index = column_index(&columns, "frequency_hz", path)?;
    let magnitude_index = column_index(&columns, "magnitude", path)?;
    let phase_index = column_index(&columns, "phase_deg", path)?;
    let normalized_magnitude_index = column_index(&columns, "normalized_magnitude", path)?;
    let normalized_phase_index = column_index(&columns, "normalized_phase_deg", path)?;
    let thd_index = column_index(&columns, "thd_percent", path)?;
    let mut rows = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let fields = parse_csv_line(line);
        if fields.len() != columns.len() {
            return Err(format!(
                "Fourier summary {} row {} has {} columns, expected {}.",
                path.display(),
                line_index + 2,
                fields.len(),
                columns.len()
            ));
        }
        rows.push(FourierSummaryRow {
            output_expression: fields[expression_index].trim().to_string(),
            harmonic: parse_u32(&fields[harmonic_index], "harmonic", path, line_index + 2)?,
            frequency_hz: parse_f64(
                &fields[frequency_index],
                "frequency_hz",
                path,
                line_index + 2,
            )?,
            magnitude: parse_f64(&fields[magnitude_index], "magnitude", path, line_index + 2)?,
            phase_deg: parse_f64(&fields[phase_index], "phase_deg", path, line_index + 2)?,
            normalized_magnitude: parse_f64(
                &fields[normalized_magnitude_index],
                "normalized_magnitude",
                path,
                line_index + 2,
            )?,
            normalized_phase_deg: parse_f64(
                &fields[normalized_phase_index],
                "normalized_phase_deg",
                path,
                line_index + 2,
            )?,
            thd_percent: parse_optional_f64(
                &fields[thd_index],
                "thd_percent",
                path,
                line_index + 2,
            )?,
        });
    }
    if rows.is_empty() {
        return Err(format!(
            "Fourier summary CSV {} contains no harmonic rows.",
            path.display()
        ));
    }
    Ok(rows)
}

fn column_index(columns: &[String], name: &str, path: &Path) -> Result<usize, String> {
    columns
        .iter()
        .position(|column| column == name)
        .ok_or_else(|| {
            format!(
                "Fourier summary CSV {} is missing required column {name}.",
                path.display()
            )
        })
}

fn parse_u32(value: &str, column: &str, path: &Path, row: usize) -> Result<u32, String> {
    value.trim().parse::<u32>().map_err(|error| {
        format!(
            "Fourier summary {} row {} has invalid {column} value {}: {error}",
            path.display(),
            row,
            value.trim()
        )
    })
}

fn parse_optional_f64(
    value: &str,
    column: &str,
    path: &Path,
    row: usize,
) -> Result<Option<f64>, String> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    parse_f64(value, column, path, row).map(Some)
}

fn parse_f64(value: &str, column: &str, path: &Path, row: usize) -> Result<f64, String> {
    let parsed = value.trim().parse::<f64>().map_err(|error| {
        format!(
            "Fourier summary {} row {} has invalid {column} value {}: {error}",
            path.display(),
            row,
            value.trim()
        )
    })?;
    if !parsed.is_finite() {
        return Err(format!(
            "Fourier summary {} row {} has non-finite {column} value {}.",
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
