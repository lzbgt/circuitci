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
        let Some(signal_threshold) = threshold_for(assertion, probe) else {
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
        let Some(comparison_threshold) = comparison_threshold_for(assertion, &signal_threshold)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog assertion {} is missing a finite comparison limit.",
                    assertion.name
                ),
            );
            continue;
        };
        let measured = match measured_assertion_value(
            assertion,
            &run.series.time_s,
            &run.series.values_by_probe[probe_index],
            signal_threshold.value,
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
            AnalogRelation::Below => measured < comparison_threshold.value,
            AnalogRelation::Above => measured > comparison_threshold.value,
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
                    comparison_threshold.unit,
                    comparison_threshold.value,
                    comparison_threshold.unit,
                    assertion_time_phrase(assertion)
                ),
            );
            finding
                .measured
                .insert(assertion.probe.clone(), json!(measured));
            finding.measured.insert(
                format!("{}_unit", assertion.probe),
                json!(comparison_threshold.unit),
            );
            finding.measured.insert(
                format!("{}_quantity", assertion.probe),
                json!(measured_quantity_name(assertion, &probe.quantity)),
            );
            if uses_signal_threshold_as_level(&assertion.aggregation) {
                finding.measured.insert(
                    format!(
                        "{}_decision_threshold{}",
                        assertion.probe, signal_threshold.limit_key
                    ),
                    json!(signal_threshold.value),
                );
                finding.measured.insert(
                    format!("{}_decision_threshold_unit", assertion.probe),
                    json!(signal_threshold.unit),
                );
            }
            insert_measured_time(assertion, &mut finding);
            finding.limit.insert(
                format!("{relation}{}", comparison_threshold.limit_key),
                json!(comparison_threshold.value),
            );
            if assertion.suggested_fixes.is_empty() {
                finding
                    .suggested_fixes
                    .push("Adjust the circuit or device model so the simulated waveform meets the declared physical threshold.".to_string());
            } else {
                finding
                    .suggested_fixes
                    .extend(assertion.suggested_fixes.iter().cloned());
            }
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
            if assertion.time_limit_us.is_some() {
                return Err("sample aggregation must not declare time_limit_us".to_string());
            }
            if assertion.duty_limit_percent.is_some() {
                return Err("sample aggregation must not declare duty_limit_percent".to_string());
            }
            if assertion.count_limit.is_some() {
                return Err("sample aggregation must not declare count_limit".to_string());
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
        AnalogAggregation::Min
        | AnalogAggregation::Max
        | AnalogAggregation::Mean
        | AnalogAggregation::Rms
        | AnalogAggregation::RisingCrossingTime
        | AnalogAggregation::FallingCrossingTime
        | AnalogAggregation::MinHighPulseWidth
        | AnalogAggregation::MinLowPulseWidth
        | AnalogAggregation::DutyCycle
        | AnalogAggregation::CrossingCount
        | AnalogAggregation::RisingCrossingCount
        | AnalogAggregation::FallingCrossingCount => {
            if assertion.at_us.is_some() {
                return Err("window/crossing aggregation must not declare at_us".to_string());
            }
            let (Some(start_us), Some(end_us)) = (assertion.start_us, assertion.end_us) else {
                return Err(
                    "requires start_us and end_us for window/crossing aggregation".to_string(),
                );
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
            if requires_time_limit(&assertion.aggregation) {
                let Some(time_limit_us) = assertion.time_limit_us else {
                    return Err("requires time_limit_us for timing aggregation".to_string());
                };
                if !time_limit_us.is_finite() || time_limit_us < 0.0 || time_limit_us > stop_time_us
                {
                    return Err(
                        "timing limit must be finite and within the transient stop time"
                            .to_string(),
                    );
                }
                if assertion.duty_limit_percent.is_some() {
                    return Err(
                        "timing aggregation must not declare duty_limit_percent".to_string()
                    );
                }
                if assertion.count_limit.is_some() {
                    return Err("timing aggregation must not declare count_limit".to_string());
                }
            } else if matches!(assertion.aggregation, AnalogAggregation::DutyCycle) {
                let Some(duty_limit_percent) = assertion.duty_limit_percent else {
                    return Err(
                        "requires duty_limit_percent for duty-cycle aggregation".to_string()
                    );
                };
                if !duty_limit_percent.is_finite() || !(0.0..=100.0).contains(&duty_limit_percent) {
                    return Err(
                        "duty limit must be finite and between 0 and 100 percent".to_string()
                    );
                }
                if assertion.time_limit_us.is_some() {
                    return Err("duty-cycle aggregation must not declare time_limit_us".to_string());
                }
                if assertion.count_limit.is_some() {
                    return Err("duty-cycle aggregation must not declare count_limit".to_string());
                }
            } else if is_crossing_count_aggregation(&assertion.aggregation) {
                let Some(count_limit) = assertion.count_limit else {
                    return Err("requires count_limit for crossing-count aggregation".to_string());
                };
                if !count_limit.is_finite() || count_limit < 0.0 {
                    return Err("count limit must be finite and nonnegative".to_string());
                }
                if assertion.time_limit_us.is_some() {
                    return Err(
                        "crossing-count aggregation must not declare time_limit_us".to_string()
                    );
                }
                if assertion.duty_limit_percent.is_some() {
                    return Err(
                        "crossing-count aggregation must not declare duty_limit_percent"
                            .to_string(),
                    );
                }
            } else if assertion.time_limit_us.is_some() {
                return Err("non-timing aggregation must not declare time_limit_us".to_string());
            } else if assertion.duty_limit_percent.is_some() {
                return Err(
                    "non-duty-cycle aggregation must not declare duty_limit_percent".to_string(),
                );
            } else if assertion.count_limit.is_some() {
                return Err("non-count aggregation must not declare count_limit".to_string());
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

fn comparison_threshold_for(
    assertion: &AnalogAssertion,
    signal_threshold: &AssertionThreshold,
) -> Option<AssertionThreshold> {
    if requires_time_limit(&assertion.aggregation) {
        let value = assertion.time_limit_us?;
        value.is_finite().then_some(AssertionThreshold {
            value,
            unit: "us",
            limit_key: "_time_us",
        })
    } else if matches!(assertion.aggregation, AnalogAggregation::DutyCycle) {
        let value = assertion.duty_limit_percent?;
        value.is_finite().then_some(AssertionThreshold {
            value,
            unit: "%",
            limit_key: "_duty_percent",
        })
    } else if is_crossing_count_aggregation(&assertion.aggregation) {
        let value = assertion.count_limit?;
        value.is_finite().then_some(AssertionThreshold {
            value,
            unit: "crossings",
            limit_key: "_crossings",
        })
    } else {
        Some(AssertionThreshold {
            value: signal_threshold.value,
            unit: signal_threshold.unit,
            limit_key: signal_threshold.limit_key,
        })
    }
}

fn measured_assertion_value(
    assertion: &AnalogAssertion,
    times: &[f64],
    values: &[f64],
    threshold: f64,
) -> Option<f64> {
    match assertion.aggregation {
        AnalogAggregation::Sample => interpolate_at(times, values, assertion.at_us? / 1_000_000.0),
        AnalogAggregation::Min
        | AnalogAggregation::Max
        | AnalogAggregation::Mean
        | AnalogAggregation::Rms => {
            let start = assertion.start_us? / 1_000_000.0;
            let end = assertion.end_us? / 1_000_000.0;
            aggregate_window(times, values, start, end, &assertion.aggregation)
        }
        AnalogAggregation::RisingCrossingTime | AnalogAggregation::FallingCrossingTime => {
            let start = assertion.start_us? / 1_000_000.0;
            let end = assertion.end_us? / 1_000_000.0;
            crossing_time_us(times, values, start, end, threshold, &assertion.aggregation)
        }
        AnalogAggregation::MinHighPulseWidth | AnalogAggregation::MinLowPulseWidth => {
            let start = assertion.start_us? / 1_000_000.0;
            let end = assertion.end_us? / 1_000_000.0;
            min_pulse_width_us(times, values, start, end, threshold, &assertion.aggregation)
        }
        AnalogAggregation::DutyCycle => {
            let start = assertion.start_us? / 1_000_000.0;
            let end = assertion.end_us? / 1_000_000.0;
            duty_cycle_percent(times, values, start, end, threshold)
        }
        AnalogAggregation::CrossingCount
        | AnalogAggregation::RisingCrossingCount
        | AnalogAggregation::FallingCrossingCount => {
            let start = assertion.start_us? / 1_000_000.0;
            let end = assertion.end_us? / 1_000_000.0;
            crossing_count(times, values, start, end, threshold, &assertion.aggregation)
                .map(|count| count as f64)
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
    selected.push((start, interpolate_at(times, values, start)?));
    for (time, value) in times.iter().copied().zip(values.iter().copied()) {
        if time > start && time < end {
            selected.push((time, value));
        }
    }
    selected.push((end, interpolate_at(times, values, end)?));
    match aggregation {
        AnalogAggregation::Min => selected
            .into_iter()
            .map(|(_, value)| value)
            .reduce(f64::min),
        AnalogAggregation::Max => selected
            .into_iter()
            .map(|(_, value)| value)
            .reduce(f64::max),
        AnalogAggregation::Mean | AnalogAggregation::Rms => {
            let duration = end - start;
            if duration <= 0.0 {
                return None;
            }
            let mut integral = 0.0;
            let mut square_integral = 0.0;
            for segment in selected.windows(2) {
                let (t0, y0) = segment[0];
                let (t1, y1) = segment[1];
                let dt = t1 - t0;
                if dt <= 0.0 {
                    return None;
                }
                integral += dt * (y0 + y1) * 0.5;
                square_integral += dt * (y0 * y0 + y0 * y1 + y1 * y1) / 3.0;
            }
            match aggregation {
                AnalogAggregation::Mean => Some(integral / duration),
                AnalogAggregation::Rms => Some((square_integral / duration).sqrt()),
                _ => unreachable!("window aggregate branch filters mean/rms"),
            }
        }
        AnalogAggregation::Sample
        | AnalogAggregation::RisingCrossingTime
        | AnalogAggregation::FallingCrossingTime
        | AnalogAggregation::MinHighPulseWidth
        | AnalogAggregation::MinLowPulseWidth
        | AnalogAggregation::DutyCycle
        | AnalogAggregation::CrossingCount
        | AnalogAggregation::RisingCrossingCount
        | AnalogAggregation::FallingCrossingCount => None,
    }
}

fn crossing_time_us(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
    threshold: f64,
    aggregation: &AnalogAggregation,
) -> Option<f64> {
    if start > end || !threshold.is_finite() {
        return None;
    }
    let mut selected = Vec::new();
    selected.push((start, interpolate_at(times, values, start)?));
    for (time, value) in times.iter().copied().zip(values.iter().copied()) {
        if time > start && time < end {
            selected.push((time, value));
        }
    }
    selected.push((end, interpolate_at(times, values, end)?));

    for segment in selected.windows(2) {
        let (t0, y0) = segment[0];
        let (t1, y1) = segment[1];
        let dy = y1 - y0;
        let crosses = match aggregation {
            AnalogAggregation::RisingCrossingTime => y0 < threshold && y1 >= threshold,
            AnalogAggregation::FallingCrossingTime => y0 > threshold && y1 <= threshold,
            _ => return None,
        };
        if crosses {
            if dy.abs() <= f64::EPSILON {
                return Some(t1 * 1_000_000.0);
            }
            let fraction = (threshold - y0) / dy;
            if !(0.0..=1.0).contains(&fraction) {
                return None;
            }
            return Some((t0 + fraction * (t1 - t0)) * 1_000_000.0);
        }
    }
    None
}

fn min_pulse_width_us(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
    threshold: f64,
    aggregation: &AnalogAggregation,
) -> Option<f64> {
    let intervals = threshold_intervals(times, values, start, end, threshold)?;
    let want_high = matches!(aggregation, AnalogAggregation::MinHighPulseWidth);
    intervals
        .into_iter()
        .filter(|interval| {
            interval.high == want_high && interval.started_at_edge && interval.ended_at_edge
        })
        .map(|interval| (interval.end - interval.start) * 1_000_000.0)
        .filter(|width| width.is_finite() && *width >= 0.0)
        .reduce(f64::min)
}

fn duty_cycle_percent(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
    threshold: f64,
) -> Option<f64> {
    if end <= start {
        return None;
    }
    let high_time: f64 = threshold_intervals(times, values, start, end, threshold)?
        .into_iter()
        .filter(|interval| interval.high)
        .map(|interval| interval.end - interval.start)
        .sum();
    Some((high_time / (end - start)) * 100.0)
}

fn crossing_count(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
    threshold: f64,
    aggregation: &AnalogAggregation,
) -> Option<usize> {
    if start > end || !threshold.is_finite() {
        return None;
    }
    let selected = threshold_selected_points(times, values, start, end)?;
    let mut count = 0;
    for segment in selected.windows(2) {
        let (_, y0) = segment[0];
        let (_, y1) = segment[1];
        let crosses = match aggregation {
            AnalogAggregation::CrossingCount => {
                (y0 < threshold && y1 >= threshold) || (y0 > threshold && y1 <= threshold)
            }
            AnalogAggregation::RisingCrossingCount => y0 < threshold && y1 >= threshold,
            AnalogAggregation::FallingCrossingCount => y0 > threshold && y1 <= threshold,
            _ => return None,
        };
        if crosses {
            count += 1;
        }
    }
    Some(count)
}

#[derive(Debug, Clone, Copy)]
struct ThresholdInterval {
    high: bool,
    start: f64,
    end: f64,
    started_at_edge: bool,
    ended_at_edge: bool,
}

fn threshold_intervals(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
    threshold: f64,
) -> Option<Vec<ThresholdInterval>> {
    if start > end || !threshold.is_finite() {
        return None;
    }
    let selected = threshold_selected_points(times, values, start, end)?;

    let mut intervals = Vec::new();
    let mut state_high = selected.first()?.1 >= threshold;
    let mut state_start = start;
    let mut started_at_edge = false;
    for segment in selected.windows(2) {
        let (t0, y0) = segment[0];
        let (t1, y1) = segment[1];
        let crossing = threshold_crossing_between(t0, y0, t1, y1, threshold);
        if let Some(crossing_time) = crossing {
            if crossing_time >= state_start {
                intervals.push(ThresholdInterval {
                    high: state_high,
                    start: state_start,
                    end: crossing_time,
                    started_at_edge,
                    ended_at_edge: true,
                });
            }
            state_high = !state_high;
            state_start = crossing_time;
            started_at_edge = true;
        }
    }
    intervals.push(ThresholdInterval {
        high: state_high,
        start: state_start,
        end,
        started_at_edge,
        ended_at_edge: false,
    });
    Some(intervals)
}

fn threshold_selected_points(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
) -> Option<Vec<(f64, f64)>> {
    if start > end {
        return None;
    }
    let mut selected = Vec::new();
    selected.push((start, interpolate_at(times, values, start)?));
    for (time, value) in times.iter().copied().zip(values.iter().copied()) {
        if time > start && time < end {
            selected.push((time, value));
        }
    }
    selected.push((end, interpolate_at(times, values, end)?));
    Some(selected)
}

fn threshold_crossing_between(t0: f64, y0: f64, t1: f64, y1: f64, threshold: f64) -> Option<f64> {
    let crosses = (y0 < threshold && y1 >= threshold) || (y0 > threshold && y1 <= threshold);
    if !crosses {
        return None;
    }
    let dy = y1 - y0;
    if dy.abs() <= f64::EPSILON {
        return Some(t1);
    }
    let fraction = (threshold - y0) / dy;
    (0.0..=1.0)
        .contains(&fraction)
        .then_some(t0 + fraction * (t1 - t0))
}

fn aggregation_label(aggregation: &AnalogAggregation) -> &'static str {
    match aggregation {
        AnalogAggregation::Sample => "sampled",
        AnalogAggregation::Min => "minimum",
        AnalogAggregation::Max => "maximum",
        AnalogAggregation::Mean => "mean",
        AnalogAggregation::Rms => "RMS",
        AnalogAggregation::RisingCrossingTime => "rising crossing-time",
        AnalogAggregation::FallingCrossingTime => "falling crossing-time",
        AnalogAggregation::MinHighPulseWidth => "minimum high pulse width",
        AnalogAggregation::MinLowPulseWidth => "minimum low pulse width",
        AnalogAggregation::DutyCycle => "duty cycle",
        AnalogAggregation::CrossingCount => "crossing count",
        AnalogAggregation::RisingCrossingCount => "rising crossing count",
        AnalogAggregation::FallingCrossingCount => "falling crossing count",
    }
}

pub(super) fn quantity_name(quantity: &AnalogQuantity) -> &'static str {
    match quantity {
        AnalogQuantity::Voltage => "voltage",
        AnalogQuantity::Current => "current",
        AnalogQuantity::Power => "power",
    }
}

fn measured_quantity_name(
    assertion: &AnalogAssertion,
    probe_quantity: &AnalogQuantity,
) -> &'static str {
    if matches!(assertion.aggregation, AnalogAggregation::DutyCycle) {
        "duty cycle"
    } else if is_crossing_count_aggregation(&assertion.aggregation) {
        "count"
    } else if uses_signal_threshold_as_level(&assertion.aggregation) {
        "time"
    } else {
        quantity_name(probe_quantity)
    }
}

fn assertion_time_phrase(assertion: &AnalogAssertion) -> String {
    match assertion.aggregation {
        AnalogAggregation::Sample => format!(" at {} us", assertion.at_us.unwrap_or_default()),
        AnalogAggregation::Min
        | AnalogAggregation::Max
        | AnalogAggregation::Mean
        | AnalogAggregation::Rms
        | AnalogAggregation::RisingCrossingTime
        | AnalogAggregation::FallingCrossingTime
        | AnalogAggregation::MinHighPulseWidth
        | AnalogAggregation::MinLowPulseWidth
        | AnalogAggregation::DutyCycle
        | AnalogAggregation::CrossingCount
        | AnalogAggregation::RisingCrossingCount
        | AnalogAggregation::FallingCrossingCount => format!(
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
        AnalogAggregation::Min
        | AnalogAggregation::Max
        | AnalogAggregation::Mean
        | AnalogAggregation::Rms
        | AnalogAggregation::RisingCrossingTime
        | AnalogAggregation::FallingCrossingTime
        | AnalogAggregation::MinHighPulseWidth
        | AnalogAggregation::MinLowPulseWidth
        | AnalogAggregation::DutyCycle
        | AnalogAggregation::CrossingCount
        | AnalogAggregation::RisingCrossingCount
        | AnalogAggregation::FallingCrossingCount => {
            if let Some(start_us) = assertion.start_us {
                finding
                    .limit
                    .insert("start_us".to_string(), json!(start_us));
            }
            if let Some(end_us) = assertion.end_us {
                finding.limit.insert("end_us".to_string(), json!(end_us));
            }
            if let Some(time_limit_us) = assertion.time_limit_us {
                finding
                    .limit
                    .insert("time_limit_us".to_string(), json!(time_limit_us));
            }
            if let Some(duty_limit_percent) = assertion.duty_limit_percent {
                finding
                    .limit
                    .insert("duty_limit_percent".to_string(), json!(duty_limit_percent));
            }
            if let Some(count_limit) = assertion.count_limit {
                finding
                    .limit
                    .insert("count_limit".to_string(), json!(count_limit));
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
        AnalogAggregation::Min
        | AnalogAggregation::Max
        | AnalogAggregation::Mean
        | AnalogAggregation::Rms
        | AnalogAggregation::RisingCrossingTime
        | AnalogAggregation::FallingCrossingTime
        | AnalogAggregation::MinHighPulseWidth
        | AnalogAggregation::MinLowPulseWidth
        | AnalogAggregation::DutyCycle
        | AnalogAggregation::CrossingCount
        | AnalogAggregation::RisingCrossingCount
        | AnalogAggregation::FallingCrossingCount => {
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

fn is_crossing_count_aggregation(aggregation: &AnalogAggregation) -> bool {
    matches!(
        aggregation,
        AnalogAggregation::CrossingCount
            | AnalogAggregation::RisingCrossingCount
            | AnalogAggregation::FallingCrossingCount
    )
}

fn requires_time_limit(aggregation: &AnalogAggregation) -> bool {
    matches!(
        aggregation,
        AnalogAggregation::RisingCrossingTime
            | AnalogAggregation::FallingCrossingTime
            | AnalogAggregation::MinHighPulseWidth
            | AnalogAggregation::MinLowPulseWidth
    )
}

fn uses_signal_threshold_as_level(aggregation: &AnalogAggregation) -> bool {
    matches!(
        aggregation,
        AnalogAggregation::RisingCrossingTime
            | AnalogAggregation::FallingCrossingTime
            | AnalogAggregation::MinHighPulseWidth
            | AnalogAggregation::MinLowPulseWidth
            | AnalogAggregation::DutyCycle
            | AnalogAggregation::CrossingCount
            | AnalogAggregation::RisingCrossingCount
            | AnalogAggregation::FallingCrossingCount
    )
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
        aggregate_window, crossing_count, crossing_time_us, duty_cycle_percent, min_pulse_width_us,
        threshold_count, validate_assertion_contract, validate_probe_contract,
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
    fn window_aggregation_computes_time_weighted_mean_and_rms() {
        let times = [0.0, 1.0, 3.0];
        let values = [0.0, 2.0, 2.0];
        let mean = aggregate_window(&times, &values, 0.0, 3.0, &AnalogAggregation::Mean).unwrap();
        let rms = aggregate_window(&times, &values, 0.0, 3.0, &AnalogAggregation::Rms).unwrap();

        assert!((mean - (5.0 / 3.0)).abs() < 1.0e-12);
        assert!((rms - (28.0_f64 / 9.0).sqrt()).abs() < 1.0e-12);
    }

    #[test]
    fn window_aggregation_rejects_out_of_range_window() {
        let times = [0.0, 1.0];
        let values = [0.0, 1.0];
        assert!(aggregate_window(&times, &values, -0.1, 0.5, &AnalogAggregation::Min).is_none());
        assert!(aggregate_window(&times, &values, 0.5, 1.1, &AnalogAggregation::Max).is_none());
        assert!(aggregate_window(&times, &values, 0.5, 0.5, &AnalogAggregation::Mean).is_none());
    }

    #[test]
    fn crossing_time_interpolates_first_matching_edge() {
        let times = [0.0, 1.0e-6, 2.0e-6, 3.0e-6];
        let values = [0.0, 1.0, 3.0, 1.0];

        let rising = crossing_time_us(
            &times,
            &values,
            0.0,
            3.0e-6,
            2.0,
            &AnalogAggregation::RisingCrossingTime,
        )
        .unwrap();
        let falling = crossing_time_us(
            &times,
            &values,
            0.0,
            3.0e-6,
            2.0,
            &AnalogAggregation::FallingCrossingTime,
        )
        .unwrap();

        assert!((rising - 1.5).abs() < 1.0e-12);
        assert!((falling - 2.5).abs() < 1.0e-12);
    }

    #[test]
    fn crossing_time_returns_none_without_requested_edge() {
        let times = [0.0, 1.0e-6, 2.0e-6];
        let values = [0.0, 0.5, 1.0];
        assert!(
            crossing_time_us(
                &times,
                &values,
                0.0,
                2.0e-6,
                2.0,
                &AnalogAggregation::RisingCrossingTime,
            )
            .is_none()
        );
    }

    #[test]
    fn pulse_width_measures_min_complete_high_and_low_pulses() {
        let times = [0.0, 0.5e-6, 1.5e-6, 2.5e-6, 3.0e-6, 4.0e-6, 5.0e-6, 6.0e-6];
        let values = [0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0];

        let high = min_pulse_width_us(
            &times,
            &values,
            0.0,
            6.0e-6,
            0.5,
            &AnalogAggregation::MinHighPulseWidth,
        )
        .unwrap();
        let low = min_pulse_width_us(
            &times,
            &values,
            0.0,
            6.0e-6,
            0.5,
            &AnalogAggregation::MinLowPulseWidth,
        )
        .unwrap();

        assert!((high - 1.75).abs() < 1.0e-12);
        assert!((low - 1.5).abs() < 1.0e-12);
    }

    #[test]
    fn duty_cycle_integrates_threshold_clipped_high_time() {
        let times = [0.0, 1.0e-6, 2.0e-6, 4.0e-6];
        let values = [0.0, 1.0, 1.0, 0.0];
        let duty = duty_cycle_percent(&times, &values, 0.0, 4.0e-6, 0.5).unwrap();

        assert!((duty - 62.5).abs() < 1.0e-12);
    }

    #[test]
    fn crossing_count_counts_any_or_directed_threshold_edges() {
        let times = [0.0, 1.0e-6, 2.0e-6, 3.0e-6, 4.0e-6];
        let values = [0.0, 1.0, 0.0, 1.0, 0.0];

        let any = crossing_count(
            &times,
            &values,
            0.0,
            4.0e-6,
            0.5,
            &AnalogAggregation::CrossingCount,
        )
        .unwrap();
        let rising = crossing_count(
            &times,
            &values,
            0.0,
            4.0e-6,
            0.5,
            &AnalogAggregation::RisingCrossingCount,
        )
        .unwrap();
        let falling = crossing_count(
            &times,
            &values,
            0.0,
            4.0e-6,
            0.5,
            &AnalogAggregation::FallingCrossingCount,
        )
        .unwrap();

        assert_eq!(any, 4);
        assert_eq!(rising, 2);
        assert_eq!(falling, 2);
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
            time_limit_us: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::Sample,
            relation: AnalogRelation::Above,
            threshold_v: Some(1.0),
            threshold_a: None,
            threshold_w: None,
            suggested_fixes: Vec::new(),
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());

        let assertion = AnalogAssertion {
            name: "bad_units".to_string(),
            probe: "nrst".to_string(),
            at_us: Some(100.0),
            start_us: None,
            end_us: None,
            time_limit_us: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::Sample,
            relation: AnalogRelation::Above,
            threshold_v: Some(1.0),
            threshold_a: Some(0.001),
            threshold_w: None,
            suggested_fixes: Vec::new(),
        };
        assert_eq!(threshold_count(&assertion), 2);

        let assertion = AnalogAssertion {
            name: "bad_crossing".to_string(),
            probe: "nrst".to_string(),
            at_us: None,
            start_us: Some(0.0),
            end_us: Some(1000.0),
            time_limit_us: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::RisingCrossingTime,
            relation: AnalogRelation::Below,
            threshold_v: Some(1.0),
            threshold_a: None,
            threshold_w: None,
            suggested_fixes: Vec::new(),
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());

        let assertion = AnalogAssertion {
            name: "bad_duty".to_string(),
            probe: "nrst".to_string(),
            at_us: None,
            start_us: Some(0.0),
            end_us: Some(1000.0),
            time_limit_us: None,
            duty_limit_percent: Some(101.0),
            count_limit: None,
            aggregation: AnalogAggregation::DutyCycle,
            relation: AnalogRelation::Below,
            threshold_v: Some(1.0),
            threshold_a: None,
            threshold_w: None,
            suggested_fixes: Vec::new(),
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());

        let assertion = AnalogAssertion {
            name: "bad_count".to_string(),
            probe: "nrst".to_string(),
            at_us: None,
            start_us: Some(0.0),
            end_us: Some(1000.0),
            time_limit_us: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::CrossingCount,
            relation: AnalogRelation::Below,
            threshold_v: Some(1.0),
            threshold_a: None,
            threshold_w: None,
            suggested_fixes: Vec::new(),
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());
    }
}
