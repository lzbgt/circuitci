use crate::board_ir::{
    AnalogMonteCarloDistribution, AnalogMonteCarloSweep, AnalogSweepComponentField,
};
use std::collections::BTreeSet;

const MAX_MONTE_CARLO_SAMPLES: usize = 64;
const NORMAL_SIGMA_SPAN: f64 = 3.0;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct MonteCarloComponentValue {
    pub(super) component: String,
    pub(super) field: AnalogSweepComponentField,
    pub(super) value: f64,
}

pub(super) fn monte_carlo_component_value_samples(
    sweep_name: &str,
    monte_carlo: &AnalogMonteCarloSweep,
    occupied_component_fields: &BTreeSet<(String, AnalogSweepComponentField)>,
) -> Result<Vec<Vec<MonteCarloComponentValue>>, String> {
    if monte_carlo.samples == 0 || monte_carlo.samples > MAX_MONTE_CARLO_SAMPLES {
        return Err(format!(
            "Analog sweep {sweep_name} Monte Carlo samples must be in 1..={MAX_MONTE_CARLO_SAMPLES}."
        ));
    }
    if monte_carlo.component_values.is_empty() {
        return Err(format!(
            "Analog sweep {sweep_name} Monte Carlo must declare at least one component value."
        ));
    }
    let mut seen = occupied_component_fields.clone();
    for entry in &monte_carlo.component_values {
        let component = entry.component.trim();
        if component.is_empty() {
            return Err(format!(
                "Analog sweep {sweep_name} Monte Carlo has a component value entry with an empty component."
            ));
        }
        let key = (component.to_string(), entry.field);
        if !seen.insert(key.clone()) {
            return Err(format!(
                "Analog sweep {sweep_name} declares component value {}.{} more than once.",
                key.0,
                key.1.as_str()
            ));
        }
        if !entry.nominal.is_finite()
            || entry.tolerance_percent < 0.0
            || !entry.tolerance_percent.is_finite()
            || entry.field.requires_positive_value() && entry.nominal <= 0.0
        {
            return Err(format!(
                "Analog sweep {sweep_name} Monte Carlo component value {}.{} has a non-finite or out-of-range nominal/tolerance.",
                component,
                entry.field.as_str()
            ));
        }
    }

    let mut samples = Vec::with_capacity(monte_carlo.samples);
    for sample_index in 0..monte_carlo.samples {
        let mut sample = Vec::with_capacity(monte_carlo.component_values.len());
        for (entry_index, entry) in monte_carlo.component_values.iter().enumerate() {
            let value = sampled_component_value(monte_carlo.seed, sample_index, entry_index, entry);
            if entry.field.requires_positive_value() && value <= 0.0 {
                return Err(format!(
                    "Analog sweep {sweep_name} Monte Carlo component value {}.{} generated a non-positive sample.",
                    entry.component,
                    entry.field.as_str()
                ));
            }
            sample.push(MonteCarloComponentValue {
                component: entry.component.trim().to_string(),
                field: entry.field,
                value,
            });
        }
        samples.push(sample);
    }
    Ok(samples)
}

fn sampled_component_value(
    seed: u64,
    sample_index: usize,
    entry_index: usize,
    entry: &crate::board_ir::AnalogMonteCarloComponentValue,
) -> f64 {
    let tolerance = entry.tolerance_percent / 100.0;
    match entry.distribution {
        AnalogMonteCarloDistribution::Uniform => {
            let unit = deterministic_unit_sample(seed, sample_index, entry_index);
            entry.nominal * (1.0 - tolerance + unit * 2.0 * tolerance)
        }
        AnalogMonteCarloDistribution::Normal => {
            let z = deterministic_normal_sample(seed, sample_index, entry_index)
                .clamp(-NORMAL_SIGMA_SPAN, NORMAL_SIGMA_SPAN);
            entry.nominal * (1.0 + z * tolerance / NORMAL_SIGMA_SPAN)
        }
    }
}

fn deterministic_normal_sample(seed: u64, sample_index: usize, entry_index: usize) -> f64 {
    let u1 = deterministic_unit_sample(seed ^ 0x517c_c1b7_2722_0a95, sample_index, entry_index)
        .max(f64::MIN_POSITIVE);
    let u2 = deterministic_unit_sample(seed ^ 0xa24b_aed4_963e_e407, sample_index, entry_index);
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

fn deterministic_unit_sample(seed: u64, sample_index: usize, entry_index: usize) -> f64 {
    let mut state = seed
        ^ 0x9e37_79b9_7f4a_7c15_u64.wrapping_mul((sample_index as u64) + 1)
        ^ 0xbf58_476d_1ce4_e5b9_u64.wrapping_mul((entry_index as u64) + 1);
    state ^= state >> 30;
    state = state.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    state ^= state >> 27;
    state = state.wrapping_mul(0x94d0_49bb_1331_11eb);
    state ^= state >> 31;
    ((state >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64))
}

#[cfg(test)]
mod tests {
    use super::monte_carlo_component_value_samples;
    use crate::board_ir::{
        AnalogMonteCarloComponentValue, AnalogMonteCarloDistribution, AnalogMonteCarloSweep,
        AnalogSweepComponentField,
    };
    use std::collections::BTreeSet;

    fn rc_monte_carlo(seed: u64) -> AnalogMonteCarloSweep {
        AnalogMonteCarloSweep {
            samples: 4,
            seed,
            component_values: vec![
                AnalogMonteCarloComponentValue {
                    component: "R1".to_string(),
                    field: AnalogSweepComponentField::ValueOhm,
                    nominal: 1000.0,
                    tolerance_percent: 5.0,
                    distribution: AnalogMonteCarloDistribution::Uniform,
                },
                AnalogMonteCarloComponentValue {
                    component: "C1".to_string(),
                    field: AnalogSweepComponentField::ValueF,
                    nominal: 0.0000001,
                    tolerance_percent: 10.0,
                    distribution: AnalogMonteCarloDistribution::Uniform,
                },
            ],
            criteria: None,
        }
    }

    #[test]
    fn monte_carlo_component_samples_are_deterministic_and_bounded() {
        let samples =
            monte_carlo_component_value_samples("rc_mc", &rc_monte_carlo(42), &BTreeSet::new())
                .unwrap();
        let repeated =
            monte_carlo_component_value_samples("rc_mc", &rc_monte_carlo(42), &BTreeSet::new())
                .unwrap();

        assert_eq!(samples, repeated);
        assert_eq!(samples.len(), 4);
        for sample in samples {
            assert_eq!(sample.len(), 2);
            assert!((950.0..=1050.0).contains(&sample[0].value));
            assert!((0.00000009..=0.00000011).contains(&sample[1].value));
        }
    }

    #[test]
    fn monte_carlo_rejects_duplicate_explicit_component_value_target() {
        let occupied = BTreeSet::from([("R1".to_string(), AnalogSweepComponentField::ValueOhm)]);

        let error = monte_carlo_component_value_samples("rc_mc", &rc_monte_carlo(42), &occupied)
            .unwrap_err();

        assert!(error.contains("declares component value R1.value_ohm more than once"));
    }

    #[test]
    fn monte_carlo_normal_component_samples_are_deterministic_and_bounded_at_three_sigma() {
        let mut monte_carlo = rc_monte_carlo(77);
        monte_carlo.component_values[0].distribution = AnalogMonteCarloDistribution::Normal;
        monte_carlo.component_values[1].distribution = AnalogMonteCarloDistribution::Normal;

        let samples =
            monte_carlo_component_value_samples("rc_mc", &monte_carlo, &BTreeSet::new()).unwrap();
        let repeated =
            monte_carlo_component_value_samples("rc_mc", &monte_carlo, &BTreeSet::new()).unwrap();

        assert_eq!(samples, repeated);
        assert_eq!(samples.len(), 4);
        assert!(
            samples
                .iter()
                .any(|sample| (sample[0].value - 1000.0).abs() > 0.1)
        );
        for sample in samples {
            assert!((950.0..=1050.0).contains(&sample[0].value));
            assert!((0.00000009..=0.00000011).contains(&sample[1].value));
        }
    }
}
