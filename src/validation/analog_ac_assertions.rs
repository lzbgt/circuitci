use crate::board_ir::{AnalogAggregation, AnalogAssertion, AnalogRelation, Scenario};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::SPICE_AC_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_util::normalize_artifact_path;
use super::common::validation_input_missing;

struct BodeResponse {
    frequency_hz: Vec<f64>,
    columns: BTreeMap<String, Vec<f64>>,
}

pub(super) fn validate_ac_assertion_contract(
    assertion: &AnalogAssertion,
    start_hz: f64,
    stop_hz: f64,
) -> Result<(), String> {
    if !is_ac_aggregation(&assertion.aggregation) {
        return Err("analog_ac assertions require an AC aggregation".to_string());
    }
    if assertion.at_us.is_some()
        || assertion.start_us.is_some()
        || assertion.end_us.is_some()
        || assertion.time_limit_us.is_some()
        || assertion.duty_limit_percent.is_some()
        || assertion.count_limit.is_some()
        || assertion.overshoot_limit_percent.is_some()
    {
        return Err("AC aggregation must not declare time-domain fields".to_string());
    }
    if assertion.reference_probe.is_some()
        || assertion.reference_threshold_v.is_some()
        || assertion.reference_threshold_a.is_some()
        || assertion.reference_threshold_w.is_some()
    {
        return Err("AC aggregation must not declare reference-probe fields".to_string());
    }
    if assertion.threshold_v.is_some()
        || assertion.threshold_a.is_some()
        || assertion.threshold_w.is_some()
        || assertion.threshold_vs.is_some()
        || assertion.threshold_c.is_some()
        || assertion.threshold_j.is_some()
        || assertion.target_v.is_some()
        || assertion.target_a.is_some()
        || assertion.target_w.is_some()
        || assertion.tolerance_v.is_some()
        || assertion.tolerance_a.is_some()
        || assertion.tolerance_w.is_some()
    {
        return Err("AC aggregation must use threshold_db, threshold_deg, or frequency_limit_hz instead of transient units".to_string());
    }
    match assertion.aggregation {
        AnalogAggregation::GainDbAtFrequency => {
            validate_frequency_sample(assertion, start_hz, stop_hz)?;
            finite_field(assertion.threshold_db, "threshold_db")?;
            reject_field(assertion.threshold_deg, "threshold_deg")?;
            reject_field(assertion.frequency_limit_hz, "frequency_limit_hz")?;
        }
        AnalogAggregation::PhaseDegAtFrequency => {
            validate_frequency_sample(assertion, start_hz, stop_hz)?;
            finite_field(assertion.threshold_deg, "threshold_deg")?;
            reject_field(assertion.threshold_db, "threshold_db")?;
            reject_field(assertion.frequency_limit_hz, "frequency_limit_hz")?;
        }
        AnalogAggregation::RisingGainCrossingFrequency
        | AnalogAggregation::FallingGainCrossingFrequency => {
            if assertion.at_hz.is_some() {
                return Err(
                    "gain-crossing frequency aggregation must not declare at_hz".to_string()
                );
            }
            finite_field(assertion.threshold_db, "threshold_db")?;
            let frequency_limit_hz =
                finite_field(assertion.frequency_limit_hz, "frequency_limit_hz")?;
            if frequency_limit_hz <= 0.0 {
                return Err("frequency_limit_hz must be positive".to_string());
            }
            reject_field(assertion.threshold_deg, "threshold_deg")?;
        }
        AnalogAggregation::Sample
        | AnalogAggregation::Min
        | AnalogAggregation::Max
        | AnalogAggregation::Mean
        | AnalogAggregation::Rms
        | AnalogAggregation::Integral
        | AnalogAggregation::Energy
        | AnalogAggregation::SettlingTime
        | AnalogAggregation::OvershootPercent
        | AnalogAggregation::RisingPhaseDelay
        | AnalogAggregation::FallingPhaseDelay
        | AnalogAggregation::RisingSetupTime
        | AnalogAggregation::RisingHoldTime
        | AnalogAggregation::FallingSetupTime
        | AnalogAggregation::FallingHoldTime
        | AnalogAggregation::RisingCrossingTime
        | AnalogAggregation::FallingCrossingTime
        | AnalogAggregation::MinHighPulseWidth
        | AnalogAggregation::MinLowPulseWidth
        | AnalogAggregation::DutyCycle
        | AnalogAggregation::CrossingCount
        | AnalogAggregation::RisingCrossingCount
        | AnalogAggregation::FallingCrossingCount => unreachable!("filtered by is_ac_aggregation"),
    }
    Ok(())
}

pub(super) fn evaluate_ac_assertions(
    scenario: &Scenario,
    bode: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let mut measurements = Vec::new();
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before AC assertion evaluation");
    let response = match parse_bode_csv(bode) {
        Ok(response) => response,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_AC_ANALYSIS,
                &scenario.name,
                format!(
                    "Failed to parse AC Bode response {}: {message}",
                    bode.display()
                ),
            ));
            return measurements;
        }
    };
    for assertion in &analog.assertions {
        let Some(probe) = analog
            .probes
            .iter()
            .find(|probe| probe.name == assertion.probe)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog AC assertion {} references unknown probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            continue;
        };
        let measured = match measure_ac_assertion(assertion, &response, &probe.name) {
            Ok(value) => value,
            Err(message) => {
                let mut finding = Finding::critical(
                    SPICE_AC_ANALYSIS,
                    &scenario.name,
                    format!(
                        "AC assertion {} could not be evaluated: {message}.",
                        assertion.name
                    ),
                );
                finding
                    .measured
                    .insert("bode".to_string(), json!(normalize_artifact_path(bode)));
                findings.push(finding);
                continue;
            }
        };
        let (limit, unit, quantity) = comparison_limit(assertion);
        let (relation, margin, passed) = match assertion.relation {
            AnalogRelation::Below => ("below", limit - measured, measured < limit),
            AnalogRelation::Above => ("above", measured - limit, measured > limit),
        };
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: assertion.probe.clone(),
            measured,
            limit,
            margin,
            relation,
            unit,
            quantity: quantity.clone(),
            passed,
        });
        if !passed {
            let mut finding = Finding::critical(
                SPICE_AC_ANALYSIS,
                &scenario.name,
                format!(
                    "AC assertion {} failed: {} probe {} measured {:.6} {}, expected {relation} {:.6} {}.",
                    assertion.name,
                    ac_aggregation_label(&assertion.aggregation),
                    assertion.probe,
                    measured,
                    unit,
                    limit,
                    unit
                ),
            );
            finding
                .measured
                .insert(assertion.probe.clone(), json!(measured));
            finding
                .measured
                .insert("quantity".to_string(), json!(quantity.clone()));
            finding
                .measured
                .insert("bode".to_string(), json!(normalize_artifact_path(bode)));
            if let Some(at_hz) = assertion.at_hz {
                finding.measured.insert("at_hz".to_string(), json!(at_hz));
            }
            if let Some(threshold_db) = assertion.threshold_db {
                finding
                    .measured
                    .insert("decision_threshold_db".to_string(), json!(threshold_db));
            }
            finding
                .limit
                .insert(format!("{relation}_{unit}"), json!(limit));
            if assertion.suggested_fixes.is_empty() {
                finding.suggested_fixes.push(
                    "Adjust filter values, gain structure, compensation, or model corners so the simulated AC response meets the declared frequency-domain limit."
                        .to_string(),
                );
            } else {
                finding
                    .suggested_fixes
                    .extend(assertion.suggested_fixes.iter().cloned());
            }
            findings.push(finding);
        }
    }
    measurements
}

fn validate_frequency_sample(
    assertion: &AnalogAssertion,
    start_hz: f64,
    stop_hz: f64,
) -> Result<(), String> {
    let at_hz = finite_field(assertion.at_hz, "at_hz")?;
    if at_hz < start_hz || at_hz > stop_hz {
        return Err("at_hz must be inside the AC sweep frequency range".to_string());
    }
    Ok(())
}

fn finite_field(value: Option<f64>, name: &str) -> Result<f64, String> {
    let Some(value) = value else {
        return Err(format!("requires {name}"));
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{name} must be finite"))
    }
}

fn reject_field(value: Option<f64>, name: &str) -> Result<(), String> {
    if value.is_some() {
        Err(format!("must not declare {name}"))
    } else {
        Ok(())
    }
}

fn is_ac_aggregation(aggregation: &AnalogAggregation) -> bool {
    matches!(
        aggregation,
        AnalogAggregation::GainDbAtFrequency
            | AnalogAggregation::PhaseDegAtFrequency
            | AnalogAggregation::RisingGainCrossingFrequency
            | AnalogAggregation::FallingGainCrossingFrequency
    )
}

fn measure_ac_assertion(
    assertion: &AnalogAssertion,
    response: &BodeResponse,
    probe_name: &str,
) -> Result<f64, String> {
    let probe_key = sanitize_csv_column(probe_name);
    match assertion.aggregation {
        AnalogAggregation::GainDbAtFrequency => interpolate_log_frequency(
            response,
            &format!("{probe_key}_mag_db"),
            finite_field(assertion.at_hz, "at_hz")?,
        ),
        AnalogAggregation::PhaseDegAtFrequency => interpolate_log_frequency(
            response,
            &format!("{probe_key}_phase_deg"),
            finite_field(assertion.at_hz, "at_hz")?,
        ),
        AnalogAggregation::RisingGainCrossingFrequency => gain_crossing_frequency(
            response,
            &format!("{probe_key}_mag_db"),
            finite_field(assertion.threshold_db, "threshold_db")?,
            CrossingDirection::Rising,
        ),
        AnalogAggregation::FallingGainCrossingFrequency => gain_crossing_frequency(
            response,
            &format!("{probe_key}_mag_db"),
            finite_field(assertion.threshold_db, "threshold_db")?,
            CrossingDirection::Falling,
        ),
        AnalogAggregation::Sample
        | AnalogAggregation::Min
        | AnalogAggregation::Max
        | AnalogAggregation::Mean
        | AnalogAggregation::Rms
        | AnalogAggregation::Integral
        | AnalogAggregation::Energy
        | AnalogAggregation::SettlingTime
        | AnalogAggregation::OvershootPercent
        | AnalogAggregation::RisingPhaseDelay
        | AnalogAggregation::FallingPhaseDelay
        | AnalogAggregation::RisingSetupTime
        | AnalogAggregation::RisingHoldTime
        | AnalogAggregation::FallingSetupTime
        | AnalogAggregation::FallingHoldTime
        | AnalogAggregation::RisingCrossingTime
        | AnalogAggregation::FallingCrossingTime
        | AnalogAggregation::MinHighPulseWidth
        | AnalogAggregation::MinLowPulseWidth
        | AnalogAggregation::DutyCycle
        | AnalogAggregation::CrossingCount
        | AnalogAggregation::RisingCrossingCount
        | AnalogAggregation::FallingCrossingCount => Err("unsupported AC aggregation".to_string()),
    }
}

fn comparison_limit(assertion: &AnalogAssertion) -> (f64, &'static str, String) {
    match assertion.aggregation {
        AnalogAggregation::GainDbAtFrequency => (
            assertion.threshold_db.unwrap_or_default(),
            "dB",
            "gain".to_string(),
        ),
        AnalogAggregation::PhaseDegAtFrequency => (
            assertion.threshold_deg.unwrap_or_default(),
            "deg",
            "phase".to_string(),
        ),
        AnalogAggregation::RisingGainCrossingFrequency
        | AnalogAggregation::FallingGainCrossingFrequency => (
            assertion.frequency_limit_hz.unwrap_or_default(),
            "Hz",
            "frequency".to_string(),
        ),
        _ => unreachable!("only AC aggregations are evaluated here"),
    }
}

fn ac_aggregation_label(aggregation: &AnalogAggregation) -> &'static str {
    match aggregation {
        AnalogAggregation::GainDbAtFrequency => "gain at frequency",
        AnalogAggregation::PhaseDegAtFrequency => "phase at frequency",
        AnalogAggregation::RisingGainCrossingFrequency => "rising gain crossing frequency",
        AnalogAggregation::FallingGainCrossingFrequency => "falling gain crossing frequency",
        _ => "AC response",
    }
}

fn parse_bode_csv(path: &Path) -> Result<BodeResponse, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read Bode CSV {}: {error}", path.display()))?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "Bode CSV is empty".to_string())?;
    let names: Vec<String> = split_fields(header)
        .into_iter()
        .map(ToString::to_string)
        .collect();
    if names.first().is_none_or(|name| name != "frequency_hz") {
        return Err("Bode CSV first column must be frequency_hz".to_string());
    }
    let mut frequency_hz = Vec::new();
    let mut columns: BTreeMap<String, Vec<f64>> = names
        .iter()
        .skip(1)
        .map(|name| (name.clone(), Vec::new()))
        .collect();
    for (line_index, line) in lines.enumerate() {
        let fields = split_fields(line);
        if fields.len() != names.len() {
            return Err(format!(
                "Bode row {} has {} columns, expected {}",
                line_index + 2,
                fields.len(),
                names.len()
            ));
        }
        let frequency = parse_finite(fields[0])
            .ok_or_else(|| format!("Bode row {} has invalid frequency", line_index + 2))?;
        if frequency <= 0.0 || frequency_hz.last().is_some_and(|last| frequency <= *last) {
            return Err(format!(
                "Bode row {} has non-positive or non-increasing frequency",
                line_index + 2
            ));
        }
        frequency_hz.push(frequency);
        for (name, field) in names.iter().skip(1).zip(fields.iter().skip(1)) {
            let value = parse_finite(field).ok_or_else(|| {
                format!("Bode row {} has invalid value for {name}", line_index + 2)
            })?;
            columns
                .get_mut(name)
                .expect("columns were initialized from the header")
                .push(value);
        }
    }
    if frequency_hz.is_empty() {
        return Err("Bode CSV has no data rows".to_string());
    }
    Ok(BodeResponse {
        frequency_hz,
        columns,
    })
}

fn split_fields(line: &str) -> Vec<&str> {
    line.split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

fn parse_finite(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|value| value.is_finite())
}

fn interpolate_log_frequency(
    response: &BodeResponse,
    column: &str,
    frequency_hz: f64,
) -> Result<f64, String> {
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return Err("sample frequency must be positive and finite".to_string());
    }
    let values = response
        .columns
        .get(column)
        .ok_or_else(|| format!("Bode CSV is missing column {column}"))?;
    let first = response.frequency_hz[0];
    let last = *response
        .frequency_hz
        .last()
        .expect("Bode response has at least one frequency");
    if frequency_hz < first || frequency_hz > last {
        return Err(format!(
            "sample frequency {frequency_hz} Hz is outside Bode range {first}..{last} Hz"
        ));
    }
    for (index, frequency) in response.frequency_hz.iter().copied().enumerate() {
        if (frequency - frequency_hz).abs() <= f64::EPSILON * frequency_hz.max(1.0) {
            return Ok(values[index]);
        }
    }
    for (frequency_pair, value_pair) in response.frequency_hz.windows(2).zip(values.windows(2)) {
        let f0 = frequency_pair[0];
        let f1 = frequency_pair[1];
        if frequency_hz >= f0 && frequency_hz <= f1 {
            let span = f1.log10() - f0.log10();
            if span <= 0.0 {
                return Err("Bode frequency span is not increasing".to_string());
            }
            let fraction = (frequency_hz.log10() - f0.log10()) / span;
            return Ok(value_pair[0] + fraction * (value_pair[1] - value_pair[0]));
        }
    }
    Err("sample frequency was not bracketed by Bode data".to_string())
}

enum CrossingDirection {
    Rising,
    Falling,
}

fn gain_crossing_frequency(
    response: &BodeResponse,
    column: &str,
    threshold_db: f64,
    direction: CrossingDirection,
) -> Result<f64, String> {
    if !threshold_db.is_finite() {
        return Err("threshold_db must be finite".to_string());
    }
    let values = response
        .columns
        .get(column)
        .ok_or_else(|| format!("Bode CSV is missing column {column}"))?;
    for (frequency_pair, value_pair) in response.frequency_hz.windows(2).zip(values.windows(2)) {
        let y0 = value_pair[0];
        let y1 = value_pair[1];
        let crosses = match direction {
            CrossingDirection::Rising => y0 < threshold_db && y1 >= threshold_db,
            CrossingDirection::Falling => y0 > threshold_db && y1 <= threshold_db,
        };
        if crosses {
            let dy = y1 - y0;
            if dy.abs() <= f64::EPSILON {
                return Ok(frequency_pair[1]);
            }
            let fraction = (threshold_db - y0) / dy;
            if !(0.0..=1.0).contains(&fraction) {
                return Err("gain crossing interpolation fell outside its segment".to_string());
            }
            let log_f0 = frequency_pair[0].log10();
            let log_f1 = frequency_pair[1].log10();
            return Ok(10f64.powf(log_f0 + fraction * (log_f1 - log_f0)));
        }
    }
    Err(format!(
        "Bode response never crossed {threshold_db} dB for {column}"
    ))
}

fn sanitize_csv_column(name: &str) -> String {
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
    use super::{evaluate_ac_assertions, validate_ac_assertion_contract};
    use crate::board_ir::BoardProject;
    use crate::reports::Finding;

    #[test]
    fn ac_assertions_measure_gain_phase_and_cutoff() {
        let project: BoardProject = serde_yaml_ng::from_str(
            r#"
project: { name: ac_assertion_test, version: 0.1.0 }
board: { components: {}, nets: {} }
scenarios:
  - name: rc_ac
    type: analog_ac
    checks: [SPICE_AC_ANALYSIS]
    analog:
      backend: auto
      netlist_source: file
      netlist: deck_ac.cir
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis:
        type: ac
        start_frequency_hz: 10.0
        stop_frequency_hz: 100000.0
      stimuli: []
      probes:
        - name: filtered
          expression: V(filtered)
      assertions:
        - name: gain_at_1k
          probe: filtered
          aggregation: gain_db_at_frequency
          relation: below
          at_hz: 1000.0
          threshold_db: -1.0
        - name: phase_at_1k
          probe: filtered
          aggregation: phase_deg_at_frequency
          relation: below
          at_hz: 1000.0
          threshold_deg: -20.0
        - name: cutoff_above_1k
          probe: filtered
          aggregation: falling_gain_crossing_frequency
          relation: above
          threshold_db: -3.0
          frequency_limit_hz: 1000.0
"#,
        )
        .unwrap();
        let scenario = &project.scenarios[0];
        for assertion in &scenario.analog.as_ref().unwrap().assertions {
            validate_ac_assertion_contract(assertion, 10.0, 100000.0).unwrap();
        }
        let dir = tempfile::tempdir().unwrap();
        let bode = dir.path().join("bode.csv");
        std::fs::write(
            &bode,
            "frequency_hz,filtered_mag_db,filtered_phase_deg,filtered_mag
1.000000000000e2,-1.714000000000e-2,-3.595000000000e0,9.980000000000e-1
1.000000000000e3,-1.445000000000e0,-3.214000000000e1,8.460000000000e-1
1.591500000000e3,-3.010300000000e0,-4.500000000000e1,7.071000000000e-1
1.000000000000e4,-1.608000000000e1,-8.096000000000e1,1.570000000000e-1
",
        )
        .unwrap();
        let mut findings = Vec::<Finding>::new();

        let measurements = evaluate_ac_assertions(scenario, &bode, &mut findings);

        assert!(findings.is_empty());
        assert_eq!(measurements.len(), 3);
        assert!(measurements.iter().any(|measurement| {
            measurement.assertion_name == "gain_at_1k" && measurement.measured < -1.4
        }));
        assert!(measurements.iter().any(|measurement| {
            measurement.assertion_name == "phase_at_1k" && measurement.measured < -30.0
        }));
        assert!(measurements.iter().any(|measurement| {
            measurement.assertion_name == "cutoff_above_1k"
                && (1500.0..1700.0).contains(&measurement.measured)
        }));
    }
}
