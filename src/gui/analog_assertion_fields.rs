pub(super) fn aggregation_label(aggregation: &crate::board_ir::AnalogAggregation) -> &'static str {
    match aggregation {
        crate::board_ir::AnalogAggregation::Sample => "sample",
        crate::board_ir::AnalogAggregation::OperatingPoint => "operating_point",
        crate::board_ir::AnalogAggregation::Min => "min",
        crate::board_ir::AnalogAggregation::Max => "max",
        crate::board_ir::AnalogAggregation::Mean => "mean",
        crate::board_ir::AnalogAggregation::Rms => "rms",
        crate::board_ir::AnalogAggregation::Integral => "integral",
        crate::board_ir::AnalogAggregation::Energy => "energy",
        crate::board_ir::AnalogAggregation::SettlingTime => "settling_time",
        crate::board_ir::AnalogAggregation::OvershootPercent => "overshoot_percent",
        crate::board_ir::AnalogAggregation::RisingPhaseDelay => "rising_phase_delay",
        crate::board_ir::AnalogAggregation::FallingPhaseDelay => "falling_phase_delay",
        crate::board_ir::AnalogAggregation::RisingSetupTime => "rising_setup_time",
        crate::board_ir::AnalogAggregation::RisingHoldTime => "rising_hold_time",
        crate::board_ir::AnalogAggregation::FallingSetupTime => "falling_setup_time",
        crate::board_ir::AnalogAggregation::FallingHoldTime => "falling_hold_time",
        crate::board_ir::AnalogAggregation::RisingCrossingTime => "rising_crossing_time",
        crate::board_ir::AnalogAggregation::FallingCrossingTime => "falling_crossing_time",
        crate::board_ir::AnalogAggregation::MinHighPulseWidth => "min_high_pulse_width",
        crate::board_ir::AnalogAggregation::MinLowPulseWidth => "min_low_pulse_width",
        crate::board_ir::AnalogAggregation::DutyCycle => "duty_cycle",
        crate::board_ir::AnalogAggregation::CrossingCount => "crossing_count",
        crate::board_ir::AnalogAggregation::RisingCrossingCount => "rising_crossing_count",
        crate::board_ir::AnalogAggregation::FallingCrossingCount => "falling_crossing_count",
        crate::board_ir::AnalogAggregation::GainDbAtFrequency => "gain_db_at_frequency",
        crate::board_ir::AnalogAggregation::PhaseDegAtFrequency => "phase_deg_at_frequency",
        crate::board_ir::AnalogAggregation::RisingGainCrossingFrequency => {
            "rising_gain_crossing_frequency"
        }
        crate::board_ir::AnalogAggregation::FallingGainCrossingFrequency => {
            "falling_gain_crossing_frequency"
        }
        crate::board_ir::AnalogAggregation::PhaseMarginDeg => "phase_margin_deg",
        crate::board_ir::AnalogAggregation::GainMarginDb => "gain_margin_db",
        crate::board_ir::AnalogAggregation::GroupDelaySAtFrequency => "group_delay_s_at_frequency",
        crate::board_ir::AnalogAggregation::OutputNoiseDensityAtFrequency => {
            "output_noise_density_at_frequency"
        }
        crate::board_ir::AnalogAggregation::InputNoiseDensityAtFrequency => {
            "input_noise_density_at_frequency"
        }
        crate::board_ir::AnalogAggregation::IntegratedOutputNoise => "integrated_output_noise",
        crate::board_ir::AnalogAggregation::IntegratedInputNoise => "integrated_input_noise",
    }
}

pub(super) fn relation_label(relation: &crate::board_ir::AnalogRelation) -> &'static str {
    match relation {
        crate::board_ir::AnalogRelation::Above => "above",
        crate::board_ir::AnalogRelation::Below => "below",
    }
}

pub(super) fn assertion_threshold_label(
    assertion: &crate::board_ir::AnalogAssertion,
    quantity: &crate::board_ir::AnalogQuantity,
) -> String {
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::GainDbAtFrequency
            | crate::board_ir::AnalogAggregation::RisingGainCrossingFrequency
            | crate::board_ir::AnalogAggregation::FallingGainCrossingFrequency
            | crate::board_ir::AnalogAggregation::GainMarginDb
    ) {
        return assertion
            .threshold_db
            .map(|value| format!("{value:.6} dB"))
            .unwrap_or_else(|| "missing gain threshold".to_string());
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::PhaseDegAtFrequency
            | crate::board_ir::AnalogAggregation::PhaseMarginDeg
    ) {
        return assertion
            .threshold_deg
            .map(|value| format!("{value:.6} deg"))
            .unwrap_or_else(|| "missing phase threshold".to_string());
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::GroupDelaySAtFrequency
    ) {
        return assertion
            .threshold_s
            .map(|value| format!("{value:.6} s"))
            .unwrap_or_else(|| "missing group-delay threshold".to_string());
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::OutputNoiseDensityAtFrequency
            | crate::board_ir::AnalogAggregation::InputNoiseDensityAtFrequency
    ) {
        return assertion
            .threshold_v_per_sqrt_hz
            .map(|value| format!("{value:.6} V/sqrt(Hz)"))
            .unwrap_or_else(|| "missing noise density threshold".to_string());
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::IntegratedOutputNoise
            | crate::board_ir::AnalogAggregation::IntegratedInputNoise
    ) {
        return assertion
            .threshold_v
            .map(|value| format!("{value:.6} V"))
            .unwrap_or_else(|| "missing noise threshold".to_string());
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::Energy
    ) {
        return assertion
            .threshold_j
            .map(|value| format!("{value:.6} J"))
            .unwrap_or_else(|| "missing energy threshold".to_string());
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::Integral
    ) {
        return match quantity {
            crate::board_ir::AnalogQuantity::Voltage => assertion
                .threshold_vs
                .map(|value| format!("{value:.6} V*s"))
                .unwrap_or_else(|| "missing voltage integral threshold".to_string()),
            crate::board_ir::AnalogQuantity::Current => assertion
                .threshold_c
                .map(|value| format!("{value:.6} C"))
                .unwrap_or_else(|| "missing charge threshold".to_string()),
            crate::board_ir::AnalogQuantity::Power => assertion
                .threshold_j
                .map(|value| format!("{value:.6} J"))
                .unwrap_or_else(|| "missing energy threshold".to_string()),
        };
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::SettlingTime
            | crate::board_ir::AnalogAggregation::OvershootPercent
    ) {
        return match quantity {
            crate::board_ir::AnalogQuantity::Voltage => assertion
                .target_v
                .map(|value| format!("target {value:.6} V"))
                .unwrap_or_else(|| "missing voltage target".to_string()),
            crate::board_ir::AnalogQuantity::Current => assertion
                .target_a
                .map(|value| format!("target {value:.6} A"))
                .unwrap_or_else(|| "missing current target".to_string()),
            crate::board_ir::AnalogQuantity::Power => assertion
                .target_w
                .map(|value| format!("target {value:.6} W"))
                .unwrap_or_else(|| "missing power target".to_string()),
        };
    }
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => assertion
            .threshold_v
            .map(|value| format!("{value:.6} V"))
            .unwrap_or_else(|| "missing voltage threshold".to_string()),
        crate::board_ir::AnalogQuantity::Current => assertion
            .threshold_a
            .map(|value| format!("{value:.6} A"))
            .unwrap_or_else(|| "missing current threshold".to_string()),
        crate::board_ir::AnalogQuantity::Power => assertion
            .threshold_w
            .map(|value| format!("{value:.6} W"))
            .unwrap_or_else(|| "missing power threshold".to_string()),
    }
}

pub(super) fn assertion_threshold_value(
    assertion: &crate::board_ir::AnalogAssertion,
    quantity: &crate::board_ir::AnalogQuantity,
) -> Option<f64> {
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::GainDbAtFrequency
            | crate::board_ir::AnalogAggregation::RisingGainCrossingFrequency
            | crate::board_ir::AnalogAggregation::FallingGainCrossingFrequency
            | crate::board_ir::AnalogAggregation::GainMarginDb
    ) {
        return assertion.threshold_db;
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::PhaseDegAtFrequency
            | crate::board_ir::AnalogAggregation::PhaseMarginDeg
    ) {
        return assertion.threshold_deg;
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::GroupDelaySAtFrequency
    ) {
        return assertion.threshold_s;
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::OutputNoiseDensityAtFrequency
            | crate::board_ir::AnalogAggregation::InputNoiseDensityAtFrequency
    ) {
        return assertion.threshold_v_per_sqrt_hz;
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::IntegratedOutputNoise
            | crate::board_ir::AnalogAggregation::IntegratedInputNoise
    ) {
        return assertion.threshold_v;
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::Energy
    ) {
        return assertion.threshold_j;
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::Integral
    ) {
        return match quantity {
            crate::board_ir::AnalogQuantity::Voltage => assertion.threshold_vs,
            crate::board_ir::AnalogQuantity::Current => assertion.threshold_c,
            crate::board_ir::AnalogQuantity::Power => assertion.threshold_j,
        };
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::SettlingTime
            | crate::board_ir::AnalogAggregation::OvershootPercent
    ) {
        return match quantity {
            crate::board_ir::AnalogQuantity::Voltage => assertion.target_v,
            crate::board_ir::AnalogQuantity::Current => assertion.target_a,
            crate::board_ir::AnalogQuantity::Power => assertion.target_w,
        };
    }
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => assertion.threshold_v,
        crate::board_ir::AnalogQuantity::Current => assertion.threshold_a,
        crate::board_ir::AnalogQuantity::Power => assertion.threshold_w,
    }
}

pub(super) fn assertion_target_value(
    assertion: &crate::board_ir::AnalogAssertion,
    quantity: &crate::board_ir::AnalogQuantity,
) -> Option<f64> {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => assertion.target_v,
        crate::board_ir::AnalogQuantity::Current => assertion.target_a,
        crate::board_ir::AnalogQuantity::Power => assertion.target_w,
    }
}

pub(super) fn assertion_reference_threshold_value(
    assertion: &crate::board_ir::AnalogAssertion,
) -> Option<f64> {
    assertion
        .reference_threshold_v
        .or(assertion.reference_threshold_a)
        .or(assertion.reference_threshold_w)
}

pub(super) fn assertion_tolerance_value(
    assertion: &crate::board_ir::AnalogAssertion,
    quantity: &crate::board_ir::AnalogQuantity,
) -> Option<f64> {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => assertion.tolerance_v,
        crate::board_ir::AnalogQuantity::Current => assertion.tolerance_a,
        crate::board_ir::AnalogQuantity::Power => assertion.tolerance_w,
    }
}

pub(super) fn assertion_timing_label(assertion: &crate::board_ir::AnalogAssertion) -> String {
    match assertion.aggregation {
        crate::board_ir::AnalogAggregation::Sample => {
            format!("at {:.6} us", assertion.at_us.unwrap_or_default())
        }
        crate::board_ir::AnalogAggregation::OperatingPoint => "DC operating point".to_string(),
        crate::board_ir::AnalogAggregation::Min
        | crate::board_ir::AnalogAggregation::Max
        | crate::board_ir::AnalogAggregation::Mean
        | crate::board_ir::AnalogAggregation::Rms
        | crate::board_ir::AnalogAggregation::Integral
        | crate::board_ir::AnalogAggregation::Energy => {
            format!(
                "{:.6}..{:.6} us",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::RisingCrossingTime
        | crate::board_ir::AnalogAggregation::FallingCrossingTime
        | crate::board_ir::AnalogAggregation::MinHighPulseWidth
        | crate::board_ir::AnalogAggregation::MinLowPulseWidth
        | crate::board_ir::AnalogAggregation::SettlingTime
        | crate::board_ir::AnalogAggregation::RisingPhaseDelay
        | crate::board_ir::AnalogAggregation::FallingPhaseDelay
        | crate::board_ir::AnalogAggregation::RisingSetupTime
        | crate::board_ir::AnalogAggregation::RisingHoldTime
        | crate::board_ir::AnalogAggregation::FallingSetupTime
        | crate::board_ir::AnalogAggregation::FallingHoldTime => {
            format!(
                "{:.6}..{:.6} us, limit {:.6} us",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default(),
                assertion.time_limit_us.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::OvershootPercent => {
            format!(
                "{:.6}..{:.6} us, limit {:.6}%",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default(),
                assertion.overshoot_limit_percent.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::DutyCycle => {
            format!(
                "{:.6}..{:.6} us, limit {:.6}%",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default(),
                assertion.duty_limit_percent.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::CrossingCount
        | crate::board_ir::AnalogAggregation::RisingCrossingCount
        | crate::board_ir::AnalogAggregation::FallingCrossingCount => {
            format!(
                "{:.6}..{:.6} us, limit {:.6} crossings",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default(),
                assertion.count_limit.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::GainDbAtFrequency
        | crate::board_ir::AnalogAggregation::PhaseDegAtFrequency
        | crate::board_ir::AnalogAggregation::GroupDelaySAtFrequency => {
            format!("{:.6} Hz", assertion.at_hz.unwrap_or_default())
        }
        crate::board_ir::AnalogAggregation::RisingGainCrossingFrequency
        | crate::board_ir::AnalogAggregation::FallingGainCrossingFrequency => {
            format!(
                "threshold {:.6} dB, limit {:.6} Hz",
                assertion.threshold_db.unwrap_or_default(),
                assertion.frequency_limit_hz.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::PhaseMarginDeg => "unity-gain crossing".to_string(),
        crate::board_ir::AnalogAggregation::GainMarginDb => "phase -180 deg crossing".to_string(),
        crate::board_ir::AnalogAggregation::OutputNoiseDensityAtFrequency
        | crate::board_ir::AnalogAggregation::InputNoiseDensityAtFrequency => {
            format!("{:.6} Hz", assertion.at_hz.unwrap_or_default())
        }
        crate::board_ir::AnalogAggregation::IntegratedOutputNoise
        | crate::board_ir::AnalogAggregation::IntegratedInputNoise => {
            "integrated noise band".to_string()
        }
    }
}

pub(super) fn threshold_field(
    aggregation: &str,
    quantity: &crate::board_ir::AnalogQuantity,
) -> &'static str {
    if matches!(
        aggregation,
        "gain_db_at_frequency"
            | "rising_gain_crossing_frequency"
            | "falling_gain_crossing_frequency"
            | "gain_margin_db"
    ) {
        return "threshold_db";
    }
    if matches!(aggregation, "phase_deg_at_frequency" | "phase_margin_deg") {
        return "threshold_deg";
    }
    if aggregation == "group_delay_s_at_frequency" {
        return "threshold_s";
    }
    if matches!(
        aggregation,
        "output_noise_density_at_frequency" | "input_noise_density_at_frequency"
    ) {
        return "threshold_v_per_sqrt_hz";
    }
    if matches!(
        aggregation,
        "integrated_output_noise" | "integrated_input_noise"
    ) {
        return "threshold_v";
    }
    if aggregation == "energy" {
        return "threshold_j";
    }
    if aggregation == "integral" {
        return match quantity {
            crate::board_ir::AnalogQuantity::Voltage => "threshold_vs",
            crate::board_ir::AnalogQuantity::Current => "threshold_c",
            crate::board_ir::AnalogQuantity::Power => "threshold_j",
        };
    }
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "threshold_v",
        crate::board_ir::AnalogQuantity::Current => "threshold_a",
        crate::board_ir::AnalogQuantity::Power => "threshold_w",
    }
}

pub(super) fn target_field(quantity: &crate::board_ir::AnalogQuantity) -> &'static str {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "target_v",
        crate::board_ir::AnalogQuantity::Current => "target_a",
        crate::board_ir::AnalogQuantity::Power => "target_w",
    }
}

pub(super) fn tolerance_field(quantity: &crate::board_ir::AnalogQuantity) -> &'static str {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "tolerance_v",
        crate::board_ir::AnalogQuantity::Current => "tolerance_a",
        crate::board_ir::AnalogQuantity::Power => "tolerance_w",
    }
}

pub(super) fn reference_threshold_field(
    quantity: &crate::board_ir::AnalogQuantity,
) -> &'static str {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "reference_threshold_v",
        crate::board_ir::AnalogQuantity::Current => "reference_threshold_a",
        crate::board_ir::AnalogQuantity::Power => "reference_threshold_w",
    }
}

pub(super) fn quantity_label(quantity: &crate::board_ir::AnalogQuantity) -> &'static str {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "voltage",
        crate::board_ir::AnalogQuantity::Current => "current",
        crate::board_ir::AnalogQuantity::Power => "power",
    }
}
