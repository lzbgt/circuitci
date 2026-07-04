use crate::board_ir::{
    AnalogRelation, AnalogSParameterNoiseAssertion, AnalogSParameterNoiseMetric, AnalogScenario,
    Scenario,
};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::SPICE_S_PARAMETER_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_util::normalize_artifact_path;

pub(super) fn validate_s_parameter_noise_assertion_contract(
    analog: &AnalogScenario,
) -> Result<(), String> {
    if analog.analysis.s_parameter_noise_assertions.is_empty() {
        return Ok(());
    }
    if analog.analysis.s_parameter_ports.len() != 2 {
        return Err(
            "analog_sparameter s_parameter_noise_assertions require exactly two declared S-parameter ports."
                .to_string(),
        );
    }
    let mut names = BTreeSet::new();
    for assertion in &analog.analysis.s_parameter_noise_assertions {
        let name = assertion.name.trim();
        if name.is_empty() {
            return Err(
                "analog_sparameter s_parameter_noise_assertions entries require non-empty name."
                    .to_string(),
            );
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "analog_sparameter declares duplicate s_parameter_noise_assertions name {name}."
            ));
        }
        if !assertion.threshold.is_finite() {
            return Err(format!(
                "analog_sparameter s_parameter_noise_assertion {name} requires finite threshold."
            ));
        }
        if assertion
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(format!(
                "analog_sparameter s_parameter_noise_assertion {name} unit must be non-empty when provided."
            ));
        }
    }
    Ok(())
}

pub(super) fn evaluate_s_parameter_noise_assertion_boundary(
    scenario: &Scenario,
    s_parameters: &Path,
    findings: &mut Vec<Finding>,
) {
    let Some(analog) = scenario.analog.as_ref() else {
        return;
    };
    if analog.analysis.s_parameter_noise_assertions.is_empty() {
        return;
    }
    for assertion in &analog.analysis.s_parameter_noise_assertions {
        push_s_parameter_noise_unavailable_finding(scenario, assertion, s_parameters, findings);
    }
}

pub(super) fn evaluate_s_parameter_noise_assertions(
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let Some(analog) = scenario.analog.as_ref() else {
        return Vec::new();
    };
    if analog.analysis.s_parameter_noise_assertions.is_empty() {
        return Vec::new();
    }
    let row = match read_s_parameter_noise_summary(summary) {
        Ok(row) => row,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_S_PARAMETER_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.s_parameter_noise_assertions {
        let measured = noise_metric_value(assertion.metric, &row);
        let (relation, margin, passed) = match assertion.relation {
            AnalogRelation::Below => (
                "below",
                assertion.threshold - measured,
                measured < assertion.threshold,
            ),
            AnalogRelation::Above => (
                "above",
                measured - assertion.threshold,
                measured > assertion.threshold,
            ),
        };
        let unit = noise_metric_unit(assertion);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: "two_port_sp_noise".to_string(),
            measured,
            limit: assertion.threshold,
            margin,
            relation,
            unit: unit.to_string(),
            quantity: format!("s_parameter_noise {}", noise_metric_name(assertion.metric)),
            passed,
        });
        if !passed {
            push_s_parameter_noise_failed_finding(
                scenario, assertion, &row, summary, measured, relation, findings,
            );
        }
    }
    measurements
}

fn push_s_parameter_noise_unavailable_finding(
    scenario: &Scenario,
    assertion: &AnalogSParameterNoiseAssertion,
    s_parameters: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_S_PARAMETER_ANALYSIS,
        &scenario.name,
        format!(
            "S-parameter noise assertion {} metric {} requires normalized two-port SP-noise evidence that this backend does not yet emit.",
            assertion.name,
            noise_metric_name(assertion.metric)
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding.measured.insert(
        "metric".to_string(),
        json!(noise_metric_name(assertion.metric)),
    );
    finding.measured.insert(
        "relation".to_string(),
        json!(relation_name(&assertion.relation)),
    );
    finding.measured.insert(
        "s_parameters".to_string(),
        json!(normalize_artifact_path(s_parameters)),
    );
    finding.measured.insert(
        "backend_status".to_string(),
        json!("planned_not_implemented"),
    );
    finding.measured.insert(
        "source_evidence".to_string(),
        json!("ngspice .SP donoise=1 provides NF, NFmin, Rn, and SOpt for two-port SP noise; the selected backend did not emit normalized s_parameter_noise_summary evidence for this run."),
    );
    finding.limit.insert(
        format!(
            "{}_{}",
            relation_name(&assertion.relation),
            noise_metric_unit(assertion)
        ),
        json!(assertion.threshold),
    );
    finding.limit.insert(
        "required_normalized_output".to_string(),
        json!("s_parameter_noise_summary"),
    );
    finding.limit.insert(
        "required_backend_feature".to_string(),
        json!("ngspice_sp_donoise_two_port_noise_outputs"),
    );
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Run an RF SP-noise backend that emits NF/NFmin/Rn/SOpt and retain it as s_parameter_noise_summary.csv before enabling this sign-off gate."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.iter().cloned());
    }
    findings.push(finding);
}

#[derive(Clone, Copy)]
struct SParameterNoiseSummaryRow {
    row_count: usize,
    min_frequency_hz: f64,
    max_frequency_hz: f64,
    max_noise_figure_db: f64,
    frequency_hz_at_max_noise_figure: f64,
    max_minimum_noise_figure_db: f64,
    frequency_hz_at_max_minimum_noise_figure: f64,
    max_equivalent_noise_resistance_ohm: f64,
    frequency_hz_at_max_equivalent_noise_resistance: f64,
    max_optimum_source_reflection_magnitude: f64,
    frequency_hz_at_max_optimum_source_reflection_magnitude: f64,
}

fn read_s_parameter_noise_summary(path: &Path) -> Result<SParameterNoiseSummaryRow, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read S-parameter noise summary {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "S-parameter noise summary is empty".to_string())?;
    let names: Vec<_> = header.split(',').map(str::trim).collect();
    let expected = [
        "row_count",
        "min_frequency_hz",
        "max_frequency_hz",
        "max_noise_figure_db",
        "frequency_hz_at_max_noise_figure",
        "max_minimum_noise_figure_db",
        "frequency_hz_at_max_minimum_noise_figure",
        "max_equivalent_noise_resistance_ohm",
        "frequency_hz_at_max_equivalent_noise_resistance",
        "max_optimum_source_reflection_magnitude",
        "frequency_hz_at_max_optimum_source_reflection_magnitude",
    ];
    if names != expected {
        return Err("S-parameter noise summary has unexpected columns".to_string());
    }
    let row = lines
        .next()
        .ok_or_else(|| "S-parameter noise summary has no data row".to_string())?;
    let fields: Vec<_> = row.split(',').map(str::trim).collect();
    if fields.len() != expected.len() {
        return Err(format!(
            "S-parameter noise summary row has {} columns, expected {}",
            fields.len(),
            expected.len()
        ));
    }
    Ok(SParameterNoiseSummaryRow {
        row_count: parse_usize(fields[0], "row_count")?,
        min_frequency_hz: parse_finite(fields[1], "min_frequency_hz")?,
        max_frequency_hz: parse_finite(fields[2], "max_frequency_hz")?,
        max_noise_figure_db: parse_finite(fields[3], "max_noise_figure_db")?,
        frequency_hz_at_max_noise_figure: parse_finite(
            fields[4],
            "frequency_hz_at_max_noise_figure",
        )?,
        max_minimum_noise_figure_db: parse_finite(fields[5], "max_minimum_noise_figure_db")?,
        frequency_hz_at_max_minimum_noise_figure: parse_finite(
            fields[6],
            "frequency_hz_at_max_minimum_noise_figure",
        )?,
        max_equivalent_noise_resistance_ohm: parse_finite(
            fields[7],
            "max_equivalent_noise_resistance_ohm",
        )?,
        frequency_hz_at_max_equivalent_noise_resistance: parse_finite(
            fields[8],
            "frequency_hz_at_max_equivalent_noise_resistance",
        )?,
        max_optimum_source_reflection_magnitude: parse_finite(
            fields[9],
            "max_optimum_source_reflection_magnitude",
        )?,
        frequency_hz_at_max_optimum_source_reflection_magnitude: parse_finite(
            fields[10],
            "frequency_hz_at_max_optimum_source_reflection_magnitude",
        )?,
    })
}

fn push_s_parameter_noise_failed_finding(
    scenario: &Scenario,
    assertion: &AnalogSParameterNoiseAssertion,
    row: &SParameterNoiseSummaryRow,
    summary: &Path,
    measured: f64,
    relation: &str,
    findings: &mut Vec<Finding>,
) {
    let unit = noise_metric_unit(assertion);
    let mut finding = Finding::critical(
        SPICE_S_PARAMETER_ANALYSIS,
        &scenario.name,
        format!(
            "S-parameter noise assertion {} failed: {} measured {:.6} {}, expected {relation} {:.6} {}.",
            assertion.name,
            noise_metric_name(assertion.metric),
            measured,
            unit,
            assertion.threshold,
            unit
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding.measured.insert(
        "metric".to_string(),
        json!(noise_metric_name(assertion.metric)),
    );
    finding
        .measured
        .insert("measured".to_string(), json!(measured));
    finding
        .measured
        .insert("row_count".to_string(), json!(row.row_count));
    finding.measured.insert(
        "frequency_start_hz".to_string(),
        json!(row.min_frequency_hz),
    );
    finding
        .measured
        .insert("frequency_stop_hz".to_string(), json!(row.max_frequency_hz));
    finding.measured.insert(
        "frequency_hz".to_string(),
        json!(noise_metric_frequency(assertion.metric, row)),
    );
    finding.measured.insert(
        "s_parameter_noise_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding
        .limit
        .insert(format!("{relation}_{unit}"), json!(assertion.threshold));
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Adjust the RF device model, bias, source/load match, or noise-optimized source reflection so the simulated two-port SP-noise metric meets the declared limit."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.iter().cloned());
    }
    findings.push(finding);
}

fn noise_metric_value(metric: AnalogSParameterNoiseMetric, row: &SParameterNoiseSummaryRow) -> f64 {
    match metric {
        AnalogSParameterNoiseMetric::NoiseFigureDbMax => row.max_noise_figure_db,
        AnalogSParameterNoiseMetric::MinimumNoiseFigureDbMax => row.max_minimum_noise_figure_db,
        AnalogSParameterNoiseMetric::EquivalentNoiseResistanceOhmMax => {
            row.max_equivalent_noise_resistance_ohm
        }
        AnalogSParameterNoiseMetric::OptimumSourceReflectionMagnitudeMax => {
            row.max_optimum_source_reflection_magnitude
        }
    }
}

fn noise_metric_frequency(
    metric: AnalogSParameterNoiseMetric,
    row: &SParameterNoiseSummaryRow,
) -> f64 {
    match metric {
        AnalogSParameterNoiseMetric::NoiseFigureDbMax => row.frequency_hz_at_max_noise_figure,
        AnalogSParameterNoiseMetric::MinimumNoiseFigureDbMax => {
            row.frequency_hz_at_max_minimum_noise_figure
        }
        AnalogSParameterNoiseMetric::EquivalentNoiseResistanceOhmMax => {
            row.frequency_hz_at_max_equivalent_noise_resistance
        }
        AnalogSParameterNoiseMetric::OptimumSourceReflectionMagnitudeMax => {
            row.frequency_hz_at_max_optimum_source_reflection_magnitude
        }
    }
}

fn parse_usize(field: &str, name: &str) -> Result<usize, String> {
    field
        .parse::<usize>()
        .map_err(|_| format!("S-parameter noise summary has invalid {name}"))
}

fn parse_finite(field: &str, name: &str) -> Result<f64, String> {
    let value = field
        .parse::<f64>()
        .map_err(|_| format!("S-parameter noise summary has invalid {name}"))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("S-parameter noise summary has non-finite {name}"))
    }
}

fn noise_metric_name(metric: AnalogSParameterNoiseMetric) -> &'static str {
    match metric {
        AnalogSParameterNoiseMetric::NoiseFigureDbMax => "noise_figure_db_max",
        AnalogSParameterNoiseMetric::MinimumNoiseFigureDbMax => "minimum_noise_figure_db_max",
        AnalogSParameterNoiseMetric::EquivalentNoiseResistanceOhmMax => {
            "equivalent_noise_resistance_ohm_max"
        }
        AnalogSParameterNoiseMetric::OptimumSourceReflectionMagnitudeMax => {
            "optimum_source_reflection_magnitude_max"
        }
    }
}

fn noise_metric_unit(assertion: &AnalogSParameterNoiseAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or(match assertion.metric {
        AnalogSParameterNoiseMetric::NoiseFigureDbMax
        | AnalogSParameterNoiseMetric::MinimumNoiseFigureDbMax => "dB",
        AnalogSParameterNoiseMetric::EquivalentNoiseResistanceOhmMax => "ohm",
        AnalogSParameterNoiseMetric::OptimumSourceReflectionMagnitudeMax => "ratio",
    })
}

fn relation_name(relation: &AnalogRelation) -> &'static str {
    match relation {
        AnalogRelation::Above => "above",
        AnalogRelation::Below => "below",
    }
}
