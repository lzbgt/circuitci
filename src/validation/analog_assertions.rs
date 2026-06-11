use crate::board_ir::{
    AnalogAggregation, AnalogAssertion, AnalogProbe, AnalogQuantity, AnalogRelation, Scenario,
};
use crate::reports::Finding;
use serde_json::json;

use super::SPICE_TRANSIENT_ANALYSIS;
use super::analog_runner::NgspiceRun;
use super::analog_util::normalize_artifact_path;
use super::common::validation_input_missing;

pub(super) struct AssertionThreshold {
    pub(super) value: f64,
    pub(super) unit: &'static str,
    pub(super) limit_key: &'static str,
}

pub(super) fn evaluate_waveform_assertions(
    scenario: &Scenario,
    run: &NgspiceRun,
    findings: &mut Vec<Finding>,
) {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before assertion evaluation");
    for assertion in &analog.assertions {
        let Some(probe_index) = analog
            .probes
            .iter()
            .position(|probe| probe.name == assertion.probe)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog assertion {} references unknown probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            continue;
        };
        let probe = &analog.probes[probe_index];
        let Some(threshold) = threshold_for(assertion, probe) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog assertion {} is missing a threshold for probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            continue;
        };
        let measured = match measured_assertion_value(
            assertion,
            &run.series.time_s,
            &run.series.values_by_probe[probe_index],
        ) {
            Some(value) => value,
            None => {
                let mut finding = Finding::critical(
                    SPICE_TRANSIENT_ANALYSIS,
                    &scenario.name,
                    format!(
                        "Waveform does not cover assertion {} over its requested time range.",
                        assertion.name
                    ),
                );
                finding.measured.insert(
                    "waveform".to_string(),
                    json!(normalize_artifact_path(&run.waveform)),
                );
                insert_time_limit(assertion, &mut finding);
                findings.push(finding);
                continue;
            }
        };
        let passed = match assertion.relation {
            AnalogRelation::Below => measured < threshold.value,
            AnalogRelation::Above => measured > threshold.value,
        };
        if !passed {
            let relation = match assertion.relation {
                AnalogRelation::Below => "below",
                AnalogRelation::Above => "above",
            };
            let aggregation = aggregation_label(&assertion.aggregation);
            let mut finding = Finding::critical(
                SPICE_TRANSIENT_ANALYSIS,
                &scenario.name,
                format!(
                    "Analog assertion {} failed: {aggregation} probe {} measured {:.6} {}, expected {relation} {:.6} {}{}.",
                    assertion.name,
                    assertion.probe,
                    measured,
                    threshold.unit,
                    threshold.value,
                    threshold.unit,
                    assertion_time_phrase(assertion)
                ),
            );
            finding
                .measured
                .insert(assertion.probe.clone(), json!(measured));
            finding
                .measured
                .insert(format!("{}_unit", assertion.probe), json!(threshold.unit));
            finding.measured.insert(
                format!("{}_quantity", assertion.probe),
                json!(quantity_name(&probe.quantity)),
            );
            insert_measured_time(assertion, &mut finding);
            finding.limit.insert(
                format!("{relation}{}", threshold.limit_key),
                json!(threshold.value),
            );
            finding
                .suggested_fixes
                .push("Adjust the circuit or device model so the simulated waveform meets the declared physical threshold.".to_string());
            findings.push(finding);
        }
    }
}

pub(super) fn validate_probe_contract(probe: &AnalogProbe) -> Result<(), String> {
    let expression = probe
        .expression
        .trim()
        .to_ascii_lowercase()
        .replace(' ', "");
    let valid = match probe.quantity {
        AnalogQuantity::Voltage => expression.starts_with("v("),
        AnalogQuantity::Current => {
            expression.starts_with("i(")
                || expression.starts_with("-i(")
                || expression.starts_with("abs(i(")
        }
        AnalogQuantity::Power => {
            expression.contains("v(") && expression.contains("i(") && expression.contains('*')
        }
    };
    if valid {
        Ok(())
    } else {
        Err(format!(
            "expression {} is not consistent with declared {} quantity",
            probe.expression,
            quantity_name(&probe.quantity)
        ))
    }
}

pub(super) fn validate_assertion_contract(
    assertion: &AnalogAssertion,
    stop_time_us: f64,
) -> Result<(), String> {
    match assertion.aggregation {
        AnalogAggregation::Sample => {
            if assertion.start_us.is_some() || assertion.end_us.is_some() {
                return Err("sample aggregation must not declare start_us or end_us".to_string());
            }
            let Some(at_us) = assertion.at_us else {
                return Err("requires at_us for sample aggregation".to_string());
            };
            if !at_us.is_finite() || at_us < 0.0 || at_us > stop_time_us {
                return Err(
                    "sample time must be finite and within the transient stop time".to_string(),
                );
            }
        }
        AnalogAggregation::Min | AnalogAggregation::Max => {
            if assertion.at_us.is_some() {
                return Err("window aggregation must not declare at_us".to_string());
            }
            let (Some(start_us), Some(end_us)) = (assertion.start_us, assertion.end_us) else {
                return Err("requires start_us and end_us for window aggregation".to_string());
            };
            if !start_us.is_finite()
                || !end_us.is_finite()
                || start_us < 0.0
                || end_us < start_us
                || end_us > stop_time_us
            {
                return Err(
                    "window bounds must be finite, ordered, and within the transient stop time"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

pub(super) fn threshold_count(assertion: &AnalogAssertion) -> usize {
    [
        assertion.threshold_v,
        assertion.threshold_a,
        assertion.threshold_w,
    ]
    .into_iter()
    .filter(|threshold| threshold.is_some_and(f64::is_finite))
    .count()
}

pub(super) fn threshold_for(
    assertion: &AnalogAssertion,
    probe: &AnalogProbe,
) -> Option<AssertionThreshold> {
    let (value, unit, limit_key) = match probe.quantity {
        AnalogQuantity::Voltage => (assertion.threshold_v?, "V", "_V"),
        AnalogQuantity::Current => (assertion.threshold_a?, "A", "_A"),
        AnalogQuantity::Power => (assertion.threshold_w?, "W", "_W"),
    };
    value.is_finite().then_some(AssertionThreshold {
        value,
        unit,
        limit_key,
    })
}

fn measured_assertion_value(
    assertion: &AnalogAssertion,
    times: &[f64],
    values: &[f64],
) -> Option<f64> {
    match assertion.aggregation {
        AnalogAggregation::Sample => interpolate_at(times, values, assertion.at_us? / 1_000_000.0),
        AnalogAggregation::Min | AnalogAggregation::Max => {
            let start = assertion.start_us? / 1_000_000.0;
            let end = assertion.end_us? / 1_000_000.0;
            aggregate_window(times, values, start, end, &assertion.aggregation)
        }
    }
}

fn aggregate_window(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
    aggregation: &AnalogAggregation,
) -> Option<f64> {
    if start > end {
        return None;
    }
    let mut selected = Vec::new();
    selected.push(interpolate_at(times, values, start)?);
    for (time, value) in times.iter().copied().zip(values.iter().copied()) {
        if time > start && time < end {
            selected.push(value);
        }
    }
    selected.push(interpolate_at(times, values, end)?);
    match aggregation {
        AnalogAggregation::Min => selected.into_iter().reduce(f64::min),
        AnalogAggregation::Max => selected.into_iter().reduce(f64::max),
        AnalogAggregation::Sample => None,
    }
}

fn aggregation_label(aggregation: &AnalogAggregation) -> &'static str {
    match aggregation {
        AnalogAggregation::Sample => "sampled",
        AnalogAggregation::Min => "minimum",
        AnalogAggregation::Max => "maximum",
    }
}

pub(super) fn quantity_name(quantity: &AnalogQuantity) -> &'static str {
    match quantity {
        AnalogQuantity::Voltage => "voltage",
        AnalogQuantity::Current => "current",
        AnalogQuantity::Power => "power",
    }
}

fn assertion_time_phrase(assertion: &AnalogAssertion) -> String {
    match assertion.aggregation {
        AnalogAggregation::Sample => format!(" at {} us", assertion.at_us.unwrap_or_default()),
        AnalogAggregation::Min | AnalogAggregation::Max => format!(
            " from {} us to {} us",
            assertion.start_us.unwrap_or_default(),
            assertion.end_us.unwrap_or_default()
        ),
    }
}

fn insert_time_limit(assertion: &AnalogAssertion, finding: &mut Finding) {
    match assertion.aggregation {
        AnalogAggregation::Sample => {
            if let Some(at_us) = assertion.at_us {
                finding
                    .limit
                    .insert("sample_time_us".to_string(), json!(at_us));
            }
        }
        AnalogAggregation::Min | AnalogAggregation::Max => {
            if let Some(start_us) = assertion.start_us {
                finding
                    .limit
                    .insert("start_us".to_string(), json!(start_us));
            }
            if let Some(end_us) = assertion.end_us {
                finding.limit.insert("end_us".to_string(), json!(end_us));
            }
        }
    }
}

fn insert_measured_time(assertion: &AnalogAssertion, finding: &mut Finding) {
    match assertion.aggregation {
        AnalogAggregation::Sample => {
            if let Some(at_us) = assertion.at_us {
                finding
                    .measured
                    .insert("sample_time_us".to_string(), json!(at_us));
            }
        }
        AnalogAggregation::Min | AnalogAggregation::Max => {
            if let Some(start_us) = assertion.start_us {
                finding
                    .measured
                    .insert("start_us".to_string(), json!(start_us));
            }
            if let Some(end_us) = assertion.end_us {
                finding.measured.insert("end_us".to_string(), json!(end_us));
            }
        }
    }
}

pub(super) fn interpolate_at(times: &[f64], values: &[f64], target: f64) -> Option<f64> {
    if times.len() != values.len() || times.is_empty() {
        return None;
    }
    if target < times[0] || target > *times.last()? {
        return None;
    }
    for index in 0..times.len() {
        if (times[index] - target).abs() <= f64::EPSILON {
            return Some(values[index]);
        }
        if index + 1 < times.len() && times[index] <= target && target <= times[index + 1] {
            let span = times[index + 1] - times[index];
            if span <= 0.0 {
                return None;
            }
            let fraction = (target - times[index]) / span;
            return Some(values[index] + fraction * (values[index + 1] - values[index]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        aggregate_window, threshold_count, validate_assertion_contract, validate_probe_contract,
    };
    use crate::board_ir::{
        AnalogAggregation, AnalogAssertion, AnalogProbe, AnalogQuantity, AnalogRelation,
    };

    #[test]
    fn window_aggregation_interpolates_boundaries() {
        let times = [0.0, 1.0, 2.0, 3.0];
        let values = [0.0, 10.0, 2.0, 8.0];
        let min = aggregate_window(&times, &values, 0.5, 2.5, &AnalogAggregation::Min).unwrap();
        let max = aggregate_window(&times, &values, 0.5, 2.5, &AnalogAggregation::Max).unwrap();
        assert_eq!(min, 2.0);
        assert_eq!(max, 10.0);
    }

    #[test]
    fn window_aggregation_rejects_out_of_range_window() {
        let times = [0.0, 1.0];
        let values = [0.0, 1.0];
        assert!(aggregate_window(&times, &values, -0.1, 0.5, &AnalogAggregation::Min).is_none());
        assert!(aggregate_window(&times, &values, 0.5, 1.1, &AnalogAggregation::Max).is_none());
    }

    #[test]
    fn probe_contract_rejects_mismatched_quantity_expression() {
        let probe = AnalogProbe {
            name: "bad_current".to_string(),
            expression: "V(nrst)".to_string(),
            quantity: AnalogQuantity::Current,
        };
        assert!(validate_probe_contract(&probe).is_err());

        let probe = AnalogProbe {
            name: "base_current".to_string(),
            expression: "abs(I(VRTS))".to_string(),
            quantity: AnalogQuantity::Current,
        };
        assert!(validate_probe_contract(&probe).is_ok());
    }

    #[test]
    fn assertion_contract_rejects_contradictory_timing_and_thresholds() {
        let assertion = AnalogAssertion {
            name: "bad_sample".to_string(),
            probe: "nrst".to_string(),
            at_us: Some(100.0),
            start_us: Some(0.0),
            end_us: None,
            aggregation: AnalogAggregation::Sample,
            relation: AnalogRelation::Above,
            threshold_v: Some(1.0),
            threshold_a: None,
            threshold_w: None,
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());

        let assertion = AnalogAssertion {
            name: "bad_units".to_string(),
            probe: "nrst".to_string(),
            at_us: Some(100.0),
            start_us: None,
            end_us: None,
            aggregation: AnalogAggregation::Sample,
            relation: AnalogRelation::Above,
            threshold_v: Some(1.0),
            threshold_a: Some(0.001),
            threshold_w: None,
        };
        assert_eq!(threshold_count(&assertion), 2);
    }
}
