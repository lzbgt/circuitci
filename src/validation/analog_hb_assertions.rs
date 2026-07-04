use crate::board_ir::{
    AnalogHarmonicBalanceAssertion, AnalogHarmonicBalanceMetric, AnalogRelation, AnalogScenario,
    Scenario,
};
use crate::reports::Finding;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::SPICE_HARMONIC_BALANCE_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_util::normalize_artifact_path;

pub(super) fn validate_hb_assertion_contract(analog: &AnalogScenario) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for assertion in &analog.analysis.hb_assertions {
        let name = assertion.name.trim();
        if name.is_empty() {
            return Err(
                "analog_harmonic_balance hb_assertions entries require non-empty name.".to_string(),
            );
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "analog_harmonic_balance declares duplicate hb_assertions name {name}."
            ));
        }
        if assertion.harmonic > 1024 {
            return Err(format!(
                "analog_harmonic_balance hb_assertion {name} harmonic must be in 0..=1024."
            ));
        }
        if !assertion.threshold.is_finite() {
            return Err(format!(
                "analog_harmonic_balance hb_assertion {name} requires finite threshold."
            ));
        }
        if assertion
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(format!(
                "analog_harmonic_balance hb_assertion {name} unit must be non-empty when provided."
            ));
        }
    }
    Ok(())
}

pub(super) fn evaluate_hb_assertions(
    scenario: &Scenario,
    spectrum: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before HB assertions");
    if analog.analysis.hb_assertions.is_empty() {
        return Vec::new();
    }
    let rows = match read_hb_spectrum(spectrum) {
        Ok(rows) => rows,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_HARMONIC_BALANCE_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let by_harmonic: BTreeMap<u32, &HbSpectrumRow> = rows
        .iter()
        .filter_map(|row| {
            u32::try_from(row.harmonic)
                .ok()
                .map(|harmonic| (harmonic, row))
        })
        .collect();
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.hb_assertions {
        let Some(row) = by_harmonic.get(&assertion.harmonic).copied() else {
            push_missing_harmonic_finding(scenario, assertion, spectrum, findings);
            continue;
        };
        let measured = hb_metric_value(assertion.metric, row);
        let evaluation = evaluate_hb_assertion_value(assertion, measured);
        let unit = hb_assertion_unit(assertion);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: format!("harmonic_{}", assertion.harmonic),
            measured,
            limit: assertion.threshold,
            margin: evaluation.margin,
            relation: evaluation.relation,
            unit: unit.to_string(),
            quantity: format!("hb {}", metric_name(assertion.metric)),
            passed: evaluation.passed,
        });
        push_hb_assertion_finding(scenario, assertion, row, spectrum, evaluation, findings);
    }
    measurements
}

#[derive(Debug, Clone)]
struct HbSpectrumRow {
    output_expression: String,
    fundamental_frequency_hz: f64,
    harmonic: i64,
    frequency_hz: f64,
    real: f64,
    imaginary: f64,
    magnitude: f64,
    phase_deg: f64,
}

#[derive(Debug, Clone, Copy)]
struct HbAssertionEvaluation {
    relation: &'static str,
    margin: f64,
    passed: bool,
}

fn hb_metric_value(metric: AnalogHarmonicBalanceMetric, row: &HbSpectrumRow) -> f64 {
    match metric {
        AnalogHarmonicBalanceMetric::Magnitude => row.magnitude,
        AnalogHarmonicBalanceMetric::PhaseDeg => row.phase_deg,
        AnalogHarmonicBalanceMetric::Real => row.real,
        AnalogHarmonicBalanceMetric::Imaginary => row.imaginary,
    }
}

fn evaluate_hb_assertion_value(
    assertion: &AnalogHarmonicBalanceAssertion,
    measured: f64,
) -> HbAssertionEvaluation {
    match assertion.relation {
        AnalogRelation::Above => HbAssertionEvaluation {
            relation: "above",
            margin: measured - assertion.threshold,
            passed: measured > assertion.threshold,
        },
        AnalogRelation::Below => HbAssertionEvaluation {
            relation: "below",
            margin: assertion.threshold - measured,
            passed: measured < assertion.threshold,
        },
    }
}

fn push_hb_assertion_finding(
    scenario: &Scenario,
    assertion: &AnalogHarmonicBalanceAssertion,
    row: &HbSpectrumRow,
    spectrum: &Path,
    evaluation: HbAssertionEvaluation,
    findings: &mut Vec<Finding>,
) {
    if evaluation.passed {
        return;
    }
    let unit = hb_assertion_unit(assertion);
    let measured = hb_metric_value(assertion.metric, row);
    let mut finding = Finding::critical(
        SPICE_HARMONIC_BALANCE_ANALYSIS,
        &scenario.name,
        format!(
            "Harmonic-balance assertion {} failed: {} measured {:.6e} {}, expected {} {:.6e} {}.",
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
        .insert("harmonic".to_string(), json!(assertion.harmonic));
    finding
        .measured
        .insert("frequency_hz".to_string(), json!(row.frequency_hz));
    finding.measured.insert(
        "fundamental_frequency_hz".to_string(),
        json!(row.fundamental_frequency_hz),
    );
    finding.measured.insert(
        "output_expression".to_string(),
        json!(&row.output_expression),
    );
    finding.measured.insert(
        "hb_spectrum".to_string(),
        json!(normalize_artifact_path(spectrum)),
    );
    finding.limit.insert(
        format!("{}_threshold", evaluation.relation),
        json!(assertion.threshold),
    );
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Adjust the periodic drive, nonlinear operating point, load, or model parameters so the normalized harmonic-balance spectrum meets the declared limit."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.clone());
    }
    findings.push(finding);
}

fn push_missing_harmonic_finding(
    scenario: &Scenario,
    assertion: &AnalogHarmonicBalanceAssertion,
    spectrum: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_HARMONIC_BALANCE_ANALYSIS,
        &scenario.name,
        format!(
            "Harmonic-balance assertion {} references missing normalized harmonic {}.",
            assertion.name, assertion.harmonic
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding
        .measured
        .insert("harmonic".to_string(), json!(assertion.harmonic));
    finding.measured.insert(
        "hb_spectrum".to_string(),
        json!(normalize_artifact_path(spectrum)),
    );
    finding
        .limit
        .insert("required_harmonic".to_string(), json!(assertion.harmonic));
    finding.suggested_fixes.push(
        "Increase hb_harmonics, confirm the backend produced that harmonic row, or update hb_assertions[] to a harmonic present in hb_spectrum.csv."
            .to_string(),
    );
    findings.push(finding);
}

fn hb_assertion_unit(assertion: &AnalogHarmonicBalanceAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or(match assertion.metric {
        AnalogHarmonicBalanceMetric::Magnitude
        | AnalogHarmonicBalanceMetric::Real
        | AnalogHarmonicBalanceMetric::Imaginary => "output_unit",
        AnalogHarmonicBalanceMetric::PhaseDeg => "deg",
    })
}

fn metric_name(metric: AnalogHarmonicBalanceMetric) -> &'static str {
    match metric {
        AnalogHarmonicBalanceMetric::Magnitude => "magnitude",
        AnalogHarmonicBalanceMetric::PhaseDeg => "phase_deg",
        AnalogHarmonicBalanceMetric::Real => "real",
        AnalogHarmonicBalanceMetric::Imaginary => "imaginary",
    }
}

fn read_hb_spectrum(path: &Path) -> Result<Vec<HbSpectrumRow>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read normalized harmonic-balance spectrum {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| format!("Harmonic-balance spectrum CSV {} is empty.", path.display()))?;
    let columns = parse_csv_line(header)?;
    let expression_index = column_index(&columns, "output_expression", path)?;
    let fundamental_index = column_index(&columns, "fundamental_frequency_hz", path)?;
    let harmonic_index = column_index(&columns, "harmonic", path)?;
    let frequency_index = column_index(&columns, "frequency_hz", path)?;
    let real_index = column_index(&columns, "real", path)?;
    let imaginary_index = column_index(&columns, "imaginary", path)?;
    let magnitude_index = column_index(&columns, "magnitude", path)?;
    let phase_index = column_index(&columns, "phase_deg", path)?;
    let mut rows = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let fields = parse_csv_line(line)?;
        if fields.len() != columns.len() {
            return Err(format!(
                "Harmonic-balance spectrum {} row {} has {} columns, expected {}.",
                path.display(),
                line_index + 2,
                fields.len(),
                columns.len()
            ));
        }
        rows.push(HbSpectrumRow {
            output_expression: fields[expression_index].trim().to_string(),
            fundamental_frequency_hz: parse_f64(
                &fields[fundamental_index],
                "fundamental_frequency_hz",
                path,
                line_index + 2,
            )?,
            harmonic: parse_i64(&fields[harmonic_index], "harmonic", path, line_index + 2)?,
            frequency_hz: parse_f64(
                &fields[frequency_index],
                "frequency_hz",
                path,
                line_index + 2,
            )?,
            real: parse_f64(&fields[real_index], "real", path, line_index + 2)?,
            imaginary: parse_f64(&fields[imaginary_index], "imaginary", path, line_index + 2)?,
            magnitude: parse_f64(&fields[magnitude_index], "magnitude", path, line_index + 2)?,
            phase_deg: parse_f64(&fields[phase_index], "phase_deg", path, line_index + 2)?,
        });
    }
    if rows.is_empty() {
        return Err(format!(
            "Harmonic-balance spectrum CSV {} contains no rows.",
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
                "Harmonic-balance spectrum CSV {} is missing required column {name}.",
                path.display()
            )
        })
}

fn parse_i64(value: &str, column: &str, path: &Path, row: usize) -> Result<i64, String> {
    value.trim().parse::<i64>().map_err(|error| {
        format!(
            "Harmonic-balance spectrum {} row {} has invalid {column} value {}: {error}",
            path.display(),
            row,
            value.trim()
        )
    })
}

fn parse_f64(value: &str, column: &str, path: &Path, row: usize) -> Result<f64, String> {
    let parsed = value.trim().parse::<f64>().map_err(|error| {
        format!(
            "Harmonic-balance spectrum {} row {} has invalid {column} value {}: {error}",
            path.display(),
            row,
            value.trim()
        )
    })?;
    if !parsed.is_finite() {
        return Err(format!(
            "Harmonic-balance spectrum {} row {} has non-finite {column} value {}.",
            path.display(),
            row,
            value.trim()
        ));
    }
    Ok(parsed)
}

fn parse_csv_line(line: &str) -> Result<Vec<String>, String> {
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
    if quoted {
        return Err("Harmonic-balance spectrum CSV has an unterminated quoted field.".to_string());
    }
    fields.push(field);
    Ok(fields)
}
