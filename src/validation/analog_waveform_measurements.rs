use crate::board_ir::{AnalogAggregation, AnalogAssertion};

pub(super) fn measured_assertion_value(
    assertion: &AnalogAssertion,
    times: &[f64],
    values: &[f64],
    threshold: f64,
    tolerance: Option<f64>,
) -> Option<f64> {
    match assertion.aggregation {
        AnalogAggregation::Sample => interpolate_at(times, values, assertion.at_us? / 1_000_000.0),
        AnalogAggregation::Min
        | AnalogAggregation::Max
        | AnalogAggregation::Mean
        | AnalogAggregation::Rms
        | AnalogAggregation::Integral
        | AnalogAggregation::Energy => {
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
        AnalogAggregation::SettlingTime => {
            let start = assertion.start_us? / 1_000_000.0;
            let end = assertion.end_us? / 1_000_000.0;
            settling_time_us(times, values, start, end, threshold, tolerance?)
        }
        AnalogAggregation::OvershootPercent => {
            let start = assertion.start_us? / 1_000_000.0;
            let end = assertion.end_us? / 1_000_000.0;
            overshoot_percent(times, values, start, end, threshold)
        }
        AnalogAggregation::CrossingCount
        | AnalogAggregation::RisingCrossingCount
        | AnalogAggregation::FallingCrossingCount => {
            let start = assertion.start_us? / 1_000_000.0;
            let end = assertion.end_us? / 1_000_000.0;
            crossing_count(times, values, start, end, threshold, &assertion.aggregation)
                .map(|count| count as f64)
        }
        AnalogAggregation::RisingPhaseDelay
        | AnalogAggregation::FallingPhaseDelay
        | AnalogAggregation::RisingSetupTime
        | AnalogAggregation::RisingHoldTime
        | AnalogAggregation::FallingSetupTime
        | AnalogAggregation::FallingHoldTime
        | AnalogAggregation::OperatingPoint
        | AnalogAggregation::GainDbAtFrequency
        | AnalogAggregation::PhaseDegAtFrequency
        | AnalogAggregation::RisingGainCrossingFrequency
        | AnalogAggregation::FallingGainCrossingFrequency
        | AnalogAggregation::PhaseMarginDeg
        | AnalogAggregation::GainMarginDb
        | AnalogAggregation::GroupDelaySAtFrequency
        | AnalogAggregation::OutputNoiseDensityAtFrequency
        | AnalogAggregation::InputNoiseDensityAtFrequency
        | AnalogAggregation::IntegratedOutputNoise
        | AnalogAggregation::IntegratedInputNoise => None,
    }
}

pub(super) fn phase_delay_us(
    times: &[f64],
    reference_values: &[f64],
    values: &[f64],
    window: (f64, f64),
    thresholds: (f64, f64),
    aggregation: &AnalogAggregation,
) -> Option<f64> {
    let (start, end) = window;
    let (reference_threshold, threshold) = thresholds;
    let edge = match aggregation {
        AnalogAggregation::RisingPhaseDelay => AnalogAggregation::RisingCrossingTime,
        AnalogAggregation::FallingPhaseDelay => AnalogAggregation::FallingCrossingTime,
        _ => return None,
    };
    let reference_time_us = crossing_time_us(
        times,
        reference_values,
        start,
        end,
        reference_threshold,
        &edge,
    )?;
    let reference_time = reference_time_us / 1_000_000.0;
    let target_time_us = crossing_time_us(times, values, reference_time, end, threshold, &edge)?;
    Some(target_time_us - reference_time_us)
}

pub(super) fn setup_hold_time_us(
    times: &[f64],
    reference_values: &[f64],
    values: &[f64],
    window: (f64, f64),
    thresholds: (f64, f64),
    aggregation: &AnalogAggregation,
) -> Option<f64> {
    let (start, end) = window;
    let (reference_threshold, threshold) = thresholds;
    let reference_edge = match aggregation {
        AnalogAggregation::RisingSetupTime | AnalogAggregation::RisingHoldTime => {
            CrossingEdge::Rising
        }
        AnalogAggregation::FallingSetupTime | AnalogAggregation::FallingHoldTime => {
            CrossingEdge::Falling
        }
        _ => return None,
    };
    let reference_crossings = crossing_times(
        times,
        reference_values,
        start,
        end,
        reference_threshold,
        reference_edge,
    )?;
    let data_crossings = crossing_times(times, values, start, end, threshold, CrossingEdge::Any)?;
    let margins = reference_crossings.into_iter().map(|reference_time| {
        if matches!(
            aggregation,
            AnalogAggregation::RisingSetupTime | AnalogAggregation::FallingSetupTime
        ) {
            let previous = data_crossings
                .iter()
                .copied()
                .filter(|crossing| *crossing <= reference_time)
                .next_back()
                .unwrap_or(start);
            reference_time - previous
        } else {
            let next = data_crossings
                .iter()
                .copied()
                .find(|crossing| *crossing >= reference_time)
                .unwrap_or(end);
            next - reference_time
        }
    });
    margins
        .filter(|margin| margin.is_finite() && *margin >= 0.0)
        .map(|margin| margin * 1_000_000.0)
        .reduce(f64::min)
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
        AnalogAggregation::Mean
        | AnalogAggregation::Rms
        | AnalogAggregation::Integral
        | AnalogAggregation::Energy => {
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
                AnalogAggregation::Integral | AnalogAggregation::Energy => Some(integral),
                _ => unreachable!("window aggregate branch filters mean/rms/integral/energy"),
            }
        }
        AnalogAggregation::Sample
        | AnalogAggregation::OperatingPoint
        | AnalogAggregation::RisingCrossingTime
        | AnalogAggregation::FallingCrossingTime
        | AnalogAggregation::MinHighPulseWidth
        | AnalogAggregation::MinLowPulseWidth
        | AnalogAggregation::DutyCycle
        | AnalogAggregation::SettlingTime
        | AnalogAggregation::OvershootPercent
        | AnalogAggregation::RisingPhaseDelay
        | AnalogAggregation::FallingPhaseDelay
        | AnalogAggregation::RisingSetupTime
        | AnalogAggregation::RisingHoldTime
        | AnalogAggregation::FallingSetupTime
        | AnalogAggregation::FallingHoldTime
        | AnalogAggregation::CrossingCount
        | AnalogAggregation::RisingCrossingCount
        | AnalogAggregation::FallingCrossingCount
        | AnalogAggregation::GainDbAtFrequency
        | AnalogAggregation::PhaseDegAtFrequency
        | AnalogAggregation::RisingGainCrossingFrequency
        | AnalogAggregation::FallingGainCrossingFrequency
        | AnalogAggregation::PhaseMarginDeg
        | AnalogAggregation::GainMarginDb
        | AnalogAggregation::GroupDelaySAtFrequency
        | AnalogAggregation::OutputNoiseDensityAtFrequency
        | AnalogAggregation::InputNoiseDensityAtFrequency
        | AnalogAggregation::IntegratedOutputNoise
        | AnalogAggregation::IntegratedInputNoise => None,
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

fn settling_time_us(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
    target: f64,
    tolerance: f64,
) -> Option<f64> {
    if start > end || !target.is_finite() || !tolerance.is_finite() || tolerance < 0.0 {
        return None;
    }
    let lower = target - tolerance;
    let upper = target + tolerance;
    let selected = window_samples(times, values, start, end)?;
    let mut last_unsettled = None;
    for segment in selected.windows(2) {
        let (t0, y0) = segment[0];
        let (t1, y1) = segment[1];
        if y0 < lower || y0 > upper {
            last_unsettled = Some(t0);
        }
        if y1 < lower || y1 > upper {
            last_unsettled = Some(t1);
        }
        for boundary in [lower, upper] {
            if let Some(crossing) = threshold_crossing_between(t0, y0, t1, y1, boundary)
                && crossing > start
                && crossing < end
            {
                last_unsettled = Some(crossing);
            }
        }
    }
    Some(last_unsettled.map_or(0.0, |time| (time - start) * 1_000_000.0))
}

fn overshoot_percent(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
    target: f64,
) -> Option<f64> {
    if start > end || !target.is_finite() || target.abs() <= f64::EPSILON {
        return None;
    }
    let max_value = window_samples(times, values, start, end)?
        .into_iter()
        .map(|(_, value)| value)
        .reduce(f64::max)?;
    Some(((max_value - target).max(0.0) / target.abs()) * 100.0)
}

fn window_samples(times: &[f64], values: &[f64], start: f64, end: f64) -> Option<Vec<(f64, f64)>> {
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
enum CrossingEdge {
    Any,
    Rising,
    Falling,
}

fn crossing_times(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
    threshold: f64,
    edge: CrossingEdge,
) -> Option<Vec<f64>> {
    if start > end || !threshold.is_finite() {
        return None;
    }
    let selected = threshold_selected_points(times, values, start, end)?;
    let mut crossings = Vec::new();
    for segment in selected.windows(2) {
        let (t0, y0) = segment[0];
        let (t1, y1) = segment[1];
        let crosses = match edge {
            CrossingEdge::Any => {
                (y0 < threshold && y1 >= threshold) || (y0 > threshold && y1 <= threshold)
            }
            CrossingEdge::Rising => y0 < threshold && y1 >= threshold,
            CrossingEdge::Falling => y0 > threshold && y1 <= threshold,
        };
        if crosses {
            crossings.push(threshold_crossing_between(t0, y0, t1, y1, threshold)?);
        }
    }
    Some(crossings)
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
        overshoot_percent, phase_delay_us, settling_time_us, setup_hold_time_us,
    };
    use crate::board_ir::AnalogAggregation;

    #[test]
    fn window_aggregation_interpolates_boundaries() {
        let times = [0.0, 1.0, 2.0, 3.0];
        let values = [0.0, 10.0, 20.0, 30.0];

        let min = aggregate_window(&times, &values, 0.5, 2.5, &AnalogAggregation::Min).unwrap();
        let max = aggregate_window(&times, &values, 0.5, 2.5, &AnalogAggregation::Max).unwrap();

        assert_eq!(min, 5.0);
        assert_eq!(max, 25.0);
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
    fn window_aggregation_computes_interpolated_integral_and_energy() {
        let times = [0.0, 1.0, 2.0];
        let values = [0.0, 2.0, 4.0];

        let integral =
            aggregate_window(&times, &values, 0.5, 2.0, &AnalogAggregation::Integral).unwrap();
        let energy =
            aggregate_window(&times, &values, 0.5, 2.0, &AnalogAggregation::Energy).unwrap();

        assert!((integral - 3.75).abs() < 1.0e-12);
        assert_eq!(energy, integral);
    }

    #[test]
    fn window_aggregation_rejects_out_of_range_window() {
        let times = [0.0, 1.0];
        let values = [0.0, 10.0];

        assert!(aggregate_window(&times, &values, -0.1, 0.5, &AnalogAggregation::Min).is_none());
        assert!(aggregate_window(&times, &values, 0.5, 1.1, &AnalogAggregation::Max).is_none());
        assert!(aggregate_window(&times, &values, 0.5, 0.5, &AnalogAggregation::Mean).is_none());
    }

    #[test]
    fn crossing_time_interpolates_first_matching_edge() {
        let times = [0.0, 1.0e-6, 2.0e-6, 3.0e-6];
        let values = [0.0, 0.4, 1.4, 0.0];

        let rising = crossing_time_us(
            &times,
            &values,
            0.0,
            3.0e-6,
            0.9,
            &AnalogAggregation::RisingCrossingTime,
        )
        .unwrap();
        let falling = crossing_time_us(
            &times,
            &values,
            0.0,
            3.0e-6,
            0.7,
            &AnalogAggregation::FallingCrossingTime,
        )
        .unwrap();

        assert!((rising - 1.5).abs() < 1.0e-9);
        assert!((falling - 2.5).abs() < 1.0e-9);
    }

    #[test]
    fn crossing_time_returns_none_without_requested_edge() {
        let times = [0.0, 1.0e-6];
        let values = [1.0, 0.0];

        assert!(
            crossing_time_us(
                &times,
                &values,
                0.0,
                1.0e-6,
                0.5,
                &AnalogAggregation::RisingCrossingTime
            )
            .is_none()
        );
    }

    #[test]
    fn pulse_width_measures_min_complete_high_and_low_pulses() {
        let times = [0.0, 1.0e-6, 2.0e-6, 4.0e-6, 5.0e-6, 7.0e-6];
        let values = [0.0, 1.0, 1.0, 0.0, 0.0, 1.0];

        let high = min_pulse_width_us(
            &times,
            &values,
            0.0,
            7.0e-6,
            0.5,
            &AnalogAggregation::MinHighPulseWidth,
        )
        .unwrap();
        let low = min_pulse_width_us(
            &times,
            &values,
            0.0,
            7.0e-6,
            0.5,
            &AnalogAggregation::MinLowPulseWidth,
        )
        .unwrap();

        assert!((high - 2.5).abs() < 1.0e-9);
        assert!((low - 3.0).abs() < 1.0e-9);
    }

    #[test]
    fn duty_cycle_integrates_threshold_clipped_high_time() {
        let times = [0.0, 1.0e-6, 2.0e-6, 3.0e-6, 4.0e-6];
        let values = [0.0, 1.0, 1.0, 0.0, 0.0];

        let duty = duty_cycle_percent(&times, &values, 0.0, 4.0e-6, 0.5).unwrap();

        assert!((duty - 50.0).abs() < 1.0e-9);
    }

    #[test]
    fn settling_time_tracks_last_band_boundary_crossing() {
        let times = [0.0, 1.0e-6, 2.0e-6, 3.0e-6, 4.0e-6];
        let values = [0.0, 1.2, 0.95, 1.05, 1.0];
        let settling = settling_time_us(&times, &values, 0.0, 4.0e-6, 1.0, 0.1).unwrap();
        assert!((settling - 1.4).abs() < 1.0e-9);
    }

    #[test]
    fn overshoot_percent_measures_peak_above_target() {
        let times = [0.0, 1.0e-6, 2.0e-6, 3.0e-6];
        let values = [0.0, 1.1, 1.2, 1.0];
        let overshoot = overshoot_percent(&times, &values, 0.0, 3.0e-6, 1.0).unwrap();
        assert!((overshoot - 20.0).abs() < 1.0e-12);
    }

    #[test]
    fn phase_delay_measures_reference_to_probe_crossing_time() {
        let times = [0.0, 1.0e-6, 2.0e-6, 3.0e-6];
        let reference = [-1.0, 1.0, 1.0, 1.0];
        let delayed = [-1.0, -1.0, 1.0, 1.0];
        let delay = phase_delay_us(
            &times,
            &reference,
            &delayed,
            (0.0, 3.0e-6),
            (0.0, 0.0),
            &AnalogAggregation::RisingPhaseDelay,
        )
        .unwrap();

        assert!((delay - 1.0).abs() < 1.0e-9);
    }

    #[test]
    fn setup_hold_time_measures_data_stability_around_reference_edges() {
        let times = [0.0, 1.0e-6, 2.0e-6, 3.0e-6, 4.0e-6, 5.0e-6];
        let reference = [0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
        let data = [0.0, 1.0, 1.0, 1.0, 0.0, 0.0];
        let setup = setup_hold_time_us(
            &times,
            &reference,
            &data,
            (0.0, 5.0e-6),
            (0.5, 0.5),
            &AnalogAggregation::RisingSetupTime,
        )
        .unwrap();
        let hold = setup_hold_time_us(
            &times,
            &reference,
            &data,
            (0.0, 5.0e-6),
            (0.5, 0.5),
            &AnalogAggregation::RisingHoldTime,
        )
        .unwrap();

        assert!((setup - 1.0).abs() < 1.0e-9);
        assert!((hold - 2.0).abs() < 1.0e-9);
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
}
