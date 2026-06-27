use crate::board_ir::{
    AnalogAggregation, AnalogAssertion, AnalogProbe, AnalogQuantity, AnalogRelation, Scenario,
};
use crate::reports::Finding;
use serde_json::json;

use super::SPICE_TRANSIENT_ANALYSIS;
use super::analog_runner::NgspiceRun;
use super::analog_util::normalize_artifact_path;
use super::analog_waveform_measurements::{
    measured_assertion_value, phase_delay_us, setup_hold_time_us,
};
use super::common::validation_input_missing;

pub(super) struct AssertionThreshold {
    pub(super) value: f64,
    pub(super) unit: &'static str,
    pub(super) limit_key: &'static str,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogAssertionMeasurement {
    pub(super) assertion_name: String,
    pub(super) probe_name: String,
    pub(super) measured: f64,
    pub(super) limit: f64,
    pub(super) margin: f64,
    pub(super) relation: &'static str,
    pub(super) unit: &'static str,
    pub(super) quantity: String,
    pub(super) passed: bool,
}

pub(super) fn evaluate_waveform_assertions(
    scenario: &Scenario,
    run: &NgspiceRun,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let mut measurements = Vec::new();
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
        let reference_timing = if is_reference_timing_aggregation(&assertion.aggregation) {
            let Some(reference_probe_name) = assertion.reference_probe.as_deref() else {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "Analog assertion {} is missing a reference probe.",
                        assertion.name
                    ),
                );
                continue;
            };
            let Some(reference_probe_index) = analog
                .probes
                .iter()
                .position(|probe| probe.name == reference_probe_name)
            else {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "Analog assertion {} references unknown reference probe {}.",
                        assertion.name, reference_probe_name
                    ),
                );
                continue;
            };
            let reference_probe = &analog.probes[reference_probe_index];
            let Some(reference_threshold) = reference_threshold_for(assertion, reference_probe)
            else {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "Analog assertion {} is missing a reference threshold for probe {}.",
                        assertion.name, reference_probe_name
                    ),
                );
                continue;
            };
            Some((reference_probe_index, reference_threshold))
        } else {
            None
        };
        let Some(signal_threshold) = signal_threshold_for(assertion, probe) else {
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
        let tolerance = if matches!(assertion.aggregation, AnalogAggregation::SettlingTime) {
            match tolerance_for(assertion, probe) {
                Some(value) => Some(value),
                None => {
                    validation_input_missing(
                        findings,
                        scenario,
                        format!(
                            "Analog assertion {} is missing a finite tolerance for probe {}.",
                            assertion.name, assertion.probe
                        ),
                    );
                    continue;
                }
            }
        } else {
            None
        };
        let measured = if let Some((reference_probe_index, reference_threshold)) = reference_timing
        {
            let window = (
                assertion.start_us.unwrap_or_default() / 1_000_000.0,
                assertion.end_us.unwrap_or_default() / 1_000_000.0,
            );
            let thresholds = (reference_threshold.value, signal_threshold.value);
            if is_phase_delay_aggregation(&assertion.aggregation) {
                phase_delay_us(
                    &run.series.time_s,
                    &run.series.values_by_probe[reference_probe_index],
                    &run.series.values_by_probe[probe_index],
                    window,
                    thresholds,
                    &assertion.aggregation,
                )
            } else {
                setup_hold_time_us(
                    &run.series.time_s,
                    &run.series.values_by_probe[reference_probe_index],
                    &run.series.values_by_probe[probe_index],
                    window,
                    thresholds,
                    &assertion.aggregation,
                )
            }
        } else {
            measured_assertion_value(
                assertion,
                &run.series.time_s,
                &run.series.values_by_probe[probe_index],
                signal_threshold.value,
                tolerance.as_ref().map(|threshold| threshold.value),
            )
        };
        let measured = match measured {
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
        let (relation, margin, passed) = match assertion.relation {
            AnalogRelation::Below => (
                "below",
                comparison_threshold.value - measured,
                measured < comparison_threshold.value,
            ),
            AnalogRelation::Above => (
                "above",
                measured - comparison_threshold.value,
                measured > comparison_threshold.value,
            ),
        };
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: assertion.probe.clone(),
            measured,
            limit: comparison_threshold.value,
            margin,
            relation,
            unit: comparison_threshold.unit,
            quantity: measured_quantity_name(assertion, &probe.quantity).to_string(),
            passed,
        });
        if !passed {
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
                .insert("assertion".to_string(), json!(&assertion.name));
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
                if let Some(reference_probe_name) = assertion.reference_probe.as_deref() {
                    finding
                        .measured
                        .insert("reference_probe".to_string(), json!(reference_probe_name));
                }
            }
            insert_measured_time(assertion, &mut finding);
            finding.limit.insert(
                format!("{relation}{}", comparison_threshold.limit_key),
                json!(comparison_threshold.value),
            );
            if assertion.suggested_fixes.is_empty() {
                finding
                    .suggested_fixes
                    .push("Adjust the circuit or device model so the simulated waveform meets the declared physical timing or threshold limit.".to_string());
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
        AnalogAggregation::GainDbAtFrequency
        | AnalogAggregation::PhaseDegAtFrequency
        | AnalogAggregation::RisingGainCrossingFrequency
        | AnalogAggregation::FallingGainCrossingFrequency
        | AnalogAggregation::PhaseMarginDeg
        | AnalogAggregation::GainMarginDb => {
            return Err("AC aggregations are only valid for analog_ac scenarios".to_string());
        }
        AnalogAggregation::OutputNoiseDensityAtFrequency
        | AnalogAggregation::InputNoiseDensityAtFrequency
        | AnalogAggregation::IntegratedOutputNoise
        | AnalogAggregation::IntegratedInputNoise => {
            return Err("noise aggregations are only valid for analog_noise scenarios".to_string());
        }
        AnalogAggregation::OperatingPoint => {
            return Err(
                "operating_point aggregation is only valid for analog_dc scenarios".to_string(),
            );
        }
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
            if assertion.reference_probe.is_some() {
                return Err("sample aggregation must not declare reference_probe".to_string());
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
                if assertion.overshoot_limit_percent.is_some() {
                    return Err(
                        "timing aggregation must not declare overshoot_limit_percent".to_string(),
                    );
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
                if assertion.overshoot_limit_percent.is_some() {
                    return Err(
                        "duty-cycle aggregation must not declare overshoot_limit_percent"
                            .to_string(),
                    );
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
                if assertion.overshoot_limit_percent.is_some() {
                    return Err(
                        "crossing-count aggregation must not declare overshoot_limit_percent"
                            .to_string(),
                    );
                }
            } else if matches!(assertion.aggregation, AnalogAggregation::OvershootPercent) {
                let Some(overshoot_limit_percent) = assertion.overshoot_limit_percent else {
                    return Err(
                        "requires overshoot_limit_percent for overshoot aggregation".to_string()
                    );
                };
                if !overshoot_limit_percent.is_finite() || overshoot_limit_percent < 0.0 {
                    return Err("overshoot limit must be finite and nonnegative".to_string());
                }
                if assertion.time_limit_us.is_some() {
                    return Err("overshoot aggregation must not declare time_limit_us".to_string());
                }
                if assertion.duty_limit_percent.is_some() {
                    return Err(
                        "overshoot aggregation must not declare duty_limit_percent".to_string()
                    );
                }
                if assertion.count_limit.is_some() {
                    return Err("overshoot aggregation must not declare count_limit".to_string());
                }
            } else if assertion.time_limit_us.is_some() {
                return Err("non-timing aggregation must not declare time_limit_us".to_string());
            } else if assertion.duty_limit_percent.is_some() {
                return Err(
                    "non-duty-cycle aggregation must not declare duty_limit_percent".to_string(),
                );
            } else if assertion.count_limit.is_some() {
                return Err("non-count aggregation must not declare count_limit".to_string());
            } else if assertion.overshoot_limit_percent.is_some() {
                return Err(
                    "non-overshoot aggregation must not declare overshoot_limit_percent"
                        .to_string(),
                );
            }
        }
    }
    if is_reference_timing_aggregation(&assertion.aggregation) {
        let Some(reference_probe) = assertion.reference_probe.as_deref() else {
            return Err("reference-timing aggregation must declare reference_probe".to_string());
        };
        if reference_probe.trim().is_empty() {
            return Err("reference-timing reference_probe must not be blank".to_string());
        }
        if reference_threshold_count(assertion) != 1 {
            return Err(
                "reference-timing aggregation must declare exactly one finite reference threshold unit"
                    .to_string(),
            );
        }
    } else {
        if assertion.reference_probe.is_some() {
            return Err(
                "non-reference-timing aggregation must not declare reference_probe".to_string(),
            );
        }
        if reference_threshold_count(assertion) != 0 {
            return Err(
                "non-reference-timing aggregation must not declare reference threshold fields"
                    .to_string(),
            );
        }
    }
    if uses_target_as_reference(&assertion.aggregation) {
        if threshold_count(assertion) != 0 {
            return Err("target-based aggregation must not declare threshold_* fields".to_string());
        }
        if target_count(assertion) != 1 {
            return Err(
                "target-based aggregation must declare exactly one finite target unit".to_string(),
            );
        }
        if matches!(assertion.aggregation, AnalogAggregation::SettlingTime) {
            if tolerance_count(assertion) != 1 {
                return Err(
                    "settling-time aggregation must declare exactly one finite tolerance unit"
                        .to_string(),
                );
            }
        } else if tolerance_count(assertion) != 0 {
            return Err("overshoot aggregation must not declare tolerance_* fields".to_string());
        }
    } else if target_count(assertion) != 0 || tolerance_count(assertion) != 0 {
        return Err(
            "non-target aggregation must not declare target_* or tolerance_* fields".to_string(),
        );
    }
    Ok(())
}

pub(super) fn threshold_count(assertion: &AnalogAssertion) -> usize {
    [
        assertion.threshold_v,
        assertion.threshold_a,
        assertion.threshold_w,
        assertion.threshold_vs,
        assertion.threshold_c,
        assertion.threshold_j,
        assertion.threshold_v_per_sqrt_hz,
    ]
    .into_iter()
    .filter(|threshold| threshold.is_some_and(f64::is_finite))
    .count()
}

pub(super) fn reference_threshold_count(assertion: &AnalogAssertion) -> usize {
    [
        assertion.reference_threshold_v,
        assertion.reference_threshold_a,
        assertion.reference_threshold_w,
    ]
    .into_iter()
    .filter(|threshold| threshold.is_some_and(f64::is_finite))
    .count()
}

pub(super) fn target_count(assertion: &AnalogAssertion) -> usize {
    [assertion.target_v, assertion.target_a, assertion.target_w]
        .into_iter()
        .filter(|target| target.is_some_and(f64::is_finite))
        .count()
}

pub(super) fn tolerance_count(assertion: &AnalogAssertion) -> usize {
    [
        assertion.tolerance_v,
        assertion.tolerance_a,
        assertion.tolerance_w,
    ]
    .into_iter()
    .filter(|tolerance| tolerance.is_some_and(|value| value.is_finite() && value >= 0.0))
    .count()
}

pub(super) fn threshold_for(
    assertion: &AnalogAssertion,
    probe: &AnalogProbe,
) -> Option<AssertionThreshold> {
    let (value, unit, limit_key) = if matches!(assertion.aggregation, AnalogAggregation::Energy) {
        if !matches!(probe.quantity, AnalogQuantity::Power) {
            return None;
        }
        (assertion.threshold_j?, "J", "_J")
    } else if matches!(assertion.aggregation, AnalogAggregation::Integral) {
        match probe.quantity {
            AnalogQuantity::Voltage => (assertion.threshold_vs?, "V*s", "_V_s"),
            AnalogQuantity::Current => (assertion.threshold_c?, "C", "_C"),
            AnalogQuantity::Power => (assertion.threshold_j?, "J", "_J"),
        }
    } else {
        match probe.quantity {
            AnalogQuantity::Voltage => (assertion.threshold_v?, "V", "_V"),
            AnalogQuantity::Current => (assertion.threshold_a?, "A", "_A"),
            AnalogQuantity::Power => (assertion.threshold_w?, "W", "_W"),
        }
    };
    value.is_finite().then_some(AssertionThreshold {
        value,
        unit,
        limit_key,
    })
}

pub(super) fn target_for(
    assertion: &AnalogAssertion,
    probe: &AnalogProbe,
) -> Option<AssertionThreshold> {
    let (value, unit, limit_key) = match probe.quantity {
        AnalogQuantity::Voltage => (assertion.target_v?, "V", "_target_V"),
        AnalogQuantity::Current => (assertion.target_a?, "A", "_target_A"),
        AnalogQuantity::Power => (assertion.target_w?, "W", "_target_W"),
    };
    value.is_finite().then_some(AssertionThreshold {
        value,
        unit,
        limit_key,
    })
}

pub(super) fn tolerance_for(
    assertion: &AnalogAssertion,
    probe: &AnalogProbe,
) -> Option<AssertionThreshold> {
    let (value, unit, limit_key) = match probe.quantity {
        AnalogQuantity::Voltage => (assertion.tolerance_v?, "V", "_tolerance_V"),
        AnalogQuantity::Current => (assertion.tolerance_a?, "A", "_tolerance_A"),
        AnalogQuantity::Power => (assertion.tolerance_w?, "W", "_tolerance_W"),
    };
    (value.is_finite() && value >= 0.0).then_some(AssertionThreshold {
        value,
        unit,
        limit_key,
    })
}

pub(super) fn reference_threshold_for(
    assertion: &AnalogAssertion,
    probe: &AnalogProbe,
) -> Option<AssertionThreshold> {
    let (value, unit, limit_key) = match probe.quantity {
        AnalogQuantity::Voltage => (assertion.reference_threshold_v?, "V", "_reference_V"),
        AnalogQuantity::Current => (assertion.reference_threshold_a?, "A", "_reference_A"),
        AnalogQuantity::Power => (assertion.reference_threshold_w?, "W", "_reference_W"),
    };
    value.is_finite().then_some(AssertionThreshold {
        value,
        unit,
        limit_key,
    })
}

pub(super) fn assertion_reference_contract_is_complete(
    assertion: &AnalogAssertion,
    probe: &AnalogProbe,
) -> bool {
    if matches!(assertion.aggregation, AnalogAggregation::SettlingTime) {
        target_for(assertion, probe).is_some()
            && tolerance_for(assertion, probe).is_some()
            && threshold_count(assertion) == 0
    } else if matches!(assertion.aggregation, AnalogAggregation::OvershootPercent) {
        target_for(assertion, probe).is_some()
            && tolerance_count(assertion) == 0
            && threshold_count(assertion) == 0
    } else if is_reference_timing_aggregation(&assertion.aggregation) {
        threshold_for(assertion, probe).is_some()
            && assertion
                .reference_probe
                .as_deref()
                .is_some_and(|name| !name.trim().is_empty())
            && reference_threshold_count(assertion) == 1
    } else {
        threshold_count(assertion) == 1 && threshold_for(assertion, probe).is_some()
    }
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
    } else if matches!(assertion.aggregation, AnalogAggregation::OvershootPercent) {
        let value = assertion.overshoot_limit_percent?;
        value.is_finite().then_some(AssertionThreshold {
            value,
            unit: "%",
            limit_key: "_overshoot_percent",
        })
    } else {
        Some(AssertionThreshold {
            value: signal_threshold.value,
            unit: signal_threshold.unit,
            limit_key: signal_threshold.limit_key,
        })
    }
}

fn signal_threshold_for(
    assertion: &AnalogAssertion,
    probe: &AnalogProbe,
) -> Option<AssertionThreshold> {
    if uses_target_as_reference(&assertion.aggregation) {
        target_for(assertion, probe)
    } else {
        threshold_for(assertion, probe)
    }
}

fn aggregation_label(aggregation: &AnalogAggregation) -> &'static str {
    match aggregation {
        AnalogAggregation::Sample => "sampled",
        AnalogAggregation::OperatingPoint => "operating point",
        AnalogAggregation::Min => "minimum",
        AnalogAggregation::Max => "maximum",
        AnalogAggregation::Mean => "mean",
        AnalogAggregation::Rms => "RMS",
        AnalogAggregation::Integral => "integral",
        AnalogAggregation::Energy => "energy",
        AnalogAggregation::SettlingTime => "settling time",
        AnalogAggregation::OvershootPercent => "overshoot",
        AnalogAggregation::RisingPhaseDelay => "rising phase-delay",
        AnalogAggregation::FallingPhaseDelay => "falling phase-delay",
        AnalogAggregation::RisingSetupTime => "rising setup time",
        AnalogAggregation::RisingHoldTime => "rising hold time",
        AnalogAggregation::FallingSetupTime => "falling setup time",
        AnalogAggregation::FallingHoldTime => "falling hold time",
        AnalogAggregation::RisingCrossingTime => "rising crossing-time",
        AnalogAggregation::FallingCrossingTime => "falling crossing-time",
        AnalogAggregation::MinHighPulseWidth => "minimum high pulse width",
        AnalogAggregation::MinLowPulseWidth => "minimum low pulse width",
        AnalogAggregation::DutyCycle => "duty cycle",
        AnalogAggregation::CrossingCount => "crossing count",
        AnalogAggregation::RisingCrossingCount => "rising crossing count",
        AnalogAggregation::FallingCrossingCount => "falling crossing count",
        AnalogAggregation::GainDbAtFrequency => "gain at frequency",
        AnalogAggregation::PhaseDegAtFrequency => "phase at frequency",
        AnalogAggregation::RisingGainCrossingFrequency => "rising gain crossing frequency",
        AnalogAggregation::FallingGainCrossingFrequency => "falling gain crossing frequency",
        AnalogAggregation::PhaseMarginDeg => "phase margin",
        AnalogAggregation::GainMarginDb => "gain margin",
        AnalogAggregation::OutputNoiseDensityAtFrequency => "output noise density",
        AnalogAggregation::InputNoiseDensityAtFrequency => "input noise density",
        AnalogAggregation::IntegratedOutputNoise => "integrated output noise",
        AnalogAggregation::IntegratedInputNoise => "integrated input noise",
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
    if matches!(assertion.aggregation, AnalogAggregation::Energy) {
        "energy"
    } else if matches!(assertion.aggregation, AnalogAggregation::SettlingTime) {
        "time"
    } else if matches!(assertion.aggregation, AnalogAggregation::OvershootPercent) {
        "overshoot"
    } else if is_reference_timing_aggregation(&assertion.aggregation) {
        "time"
    } else if matches!(assertion.aggregation, AnalogAggregation::Integral) {
        match probe_quantity {
            AnalogQuantity::Voltage => "voltage integral",
            AnalogQuantity::Current => "charge",
            AnalogQuantity::Power => "energy",
        }
    } else if matches!(assertion.aggregation, AnalogAggregation::DutyCycle) {
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
        AnalogAggregation::OperatingPoint => String::new(),
        AnalogAggregation::Min
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
        | AnalogAggregation::FallingCrossingCount => format!(
            " from {} us to {} us",
            assertion.start_us.unwrap_or_default(),
            assertion.end_us.unwrap_or_default()
        ),
        AnalogAggregation::GainDbAtFrequency | AnalogAggregation::PhaseDegAtFrequency => {
            format!(" at {} Hz", assertion.at_hz.unwrap_or_default())
        }
        AnalogAggregation::RisingGainCrossingFrequency
        | AnalogAggregation::FallingGainCrossingFrequency => {
            format!(
                " at {} dB crossing",
                assertion.threshold_db.unwrap_or_default()
            )
        }
        AnalogAggregation::PhaseMarginDeg => " at unity-gain crossing".to_string(),
        AnalogAggregation::GainMarginDb => " at -180 deg phase crossing".to_string(),
        AnalogAggregation::OutputNoiseDensityAtFrequency
        | AnalogAggregation::InputNoiseDensityAtFrequency => {
            format!(" at {} Hz", assertion.at_hz.unwrap_or_default())
        }
        AnalogAggregation::IntegratedOutputNoise | AnalogAggregation::IntegratedInputNoise => {
            String::new()
        }
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
        AnalogAggregation::OperatingPoint => {}
        AnalogAggregation::Min
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
            if let Some(overshoot_limit_percent) = assertion.overshoot_limit_percent {
                finding.limit.insert(
                    "overshoot_limit_percent".to_string(),
                    json!(overshoot_limit_percent),
                );
            }
            if let Some(reference_probe) = assertion.reference_probe.as_deref() {
                finding
                    .limit
                    .insert("reference_probe".to_string(), json!(reference_probe));
            }
            if let Some(reference_threshold_v) = assertion.reference_threshold_v {
                finding.limit.insert(
                    "reference_threshold_v".to_string(),
                    json!(reference_threshold_v),
                );
            }
            if let Some(reference_threshold_a) = assertion.reference_threshold_a {
                finding.limit.insert(
                    "reference_threshold_a".to_string(),
                    json!(reference_threshold_a),
                );
            }
            if let Some(reference_threshold_w) = assertion.reference_threshold_w {
                finding.limit.insert(
                    "reference_threshold_w".to_string(),
                    json!(reference_threshold_w),
                );
            }
            if let Some(target_v) = assertion.target_v {
                finding
                    .limit
                    .insert("target_v".to_string(), json!(target_v));
            }
            if let Some(target_a) = assertion.target_a {
                finding
                    .limit
                    .insert("target_a".to_string(), json!(target_a));
            }
            if let Some(target_w) = assertion.target_w {
                finding
                    .limit
                    .insert("target_w".to_string(), json!(target_w));
            }
            if let Some(tolerance_v) = assertion.tolerance_v {
                finding
                    .limit
                    .insert("tolerance_v".to_string(), json!(tolerance_v));
            }
            if let Some(tolerance_a) = assertion.tolerance_a {
                finding
                    .limit
                    .insert("tolerance_a".to_string(), json!(tolerance_a));
            }
            if let Some(tolerance_w) = assertion.tolerance_w {
                finding
                    .limit
                    .insert("tolerance_w".to_string(), json!(tolerance_w));
            }
        }
        AnalogAggregation::GainDbAtFrequency | AnalogAggregation::PhaseDegAtFrequency => {
            if let Some(at_hz) = assertion.at_hz {
                finding.limit.insert("at_hz".to_string(), json!(at_hz));
            }
            if let Some(threshold_db) = assertion.threshold_db {
                finding
                    .limit
                    .insert("threshold_db".to_string(), json!(threshold_db));
            }
            if let Some(threshold_deg) = assertion.threshold_deg {
                finding
                    .limit
                    .insert("threshold_deg".to_string(), json!(threshold_deg));
            }
        }
        AnalogAggregation::RisingGainCrossingFrequency
        | AnalogAggregation::FallingGainCrossingFrequency => {
            if let Some(threshold_db) = assertion.threshold_db {
                finding
                    .limit
                    .insert("threshold_db".to_string(), json!(threshold_db));
            }
            if let Some(frequency_limit_hz) = assertion.frequency_limit_hz {
                finding
                    .limit
                    .insert("frequency_limit_hz".to_string(), json!(frequency_limit_hz));
            }
        }
        AnalogAggregation::PhaseMarginDeg => {
            if let Some(threshold_deg) = assertion.threshold_deg {
                finding
                    .limit
                    .insert("threshold_deg".to_string(), json!(threshold_deg));
            }
        }
        AnalogAggregation::GainMarginDb => {
            if let Some(threshold_db) = assertion.threshold_db {
                finding
                    .limit
                    .insert("threshold_db".to_string(), json!(threshold_db));
            }
        }
        AnalogAggregation::OutputNoiseDensityAtFrequency
        | AnalogAggregation::InputNoiseDensityAtFrequency => {
            if let Some(at_hz) = assertion.at_hz {
                finding.limit.insert("at_hz".to_string(), json!(at_hz));
            }
            if let Some(threshold) = assertion.threshold_v_per_sqrt_hz {
                finding
                    .limit
                    .insert("threshold_v_per_sqrt_hz".to_string(), json!(threshold));
            }
        }
        AnalogAggregation::IntegratedOutputNoise | AnalogAggregation::IntegratedInputNoise => {
            if let Some(threshold_v) = assertion.threshold_v {
                finding
                    .limit
                    .insert("threshold_v".to_string(), json!(threshold_v));
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
        AnalogAggregation::OperatingPoint => {}
        AnalogAggregation::Min
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
        AnalogAggregation::GainDbAtFrequency | AnalogAggregation::PhaseDegAtFrequency => {
            if let Some(at_hz) = assertion.at_hz {
                finding.measured.insert("at_hz".to_string(), json!(at_hz));
            }
        }
        AnalogAggregation::RisingGainCrossingFrequency
        | AnalogAggregation::FallingGainCrossingFrequency => {
            if let Some(threshold_db) = assertion.threshold_db {
                finding
                    .measured
                    .insert("threshold_db".to_string(), json!(threshold_db));
            }
        }
        AnalogAggregation::PhaseMarginDeg | AnalogAggregation::GainMarginDb => {}
        AnalogAggregation::OutputNoiseDensityAtFrequency
        | AnalogAggregation::InputNoiseDensityAtFrequency => {
            if let Some(at_hz) = assertion.at_hz {
                finding.measured.insert("at_hz".to_string(), json!(at_hz));
            }
        }
        AnalogAggregation::IntegratedOutputNoise | AnalogAggregation::IntegratedInputNoise => {}
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
            | AnalogAggregation::SettlingTime
            | AnalogAggregation::RisingPhaseDelay
            | AnalogAggregation::FallingPhaseDelay
            | AnalogAggregation::RisingSetupTime
            | AnalogAggregation::RisingHoldTime
            | AnalogAggregation::FallingSetupTime
            | AnalogAggregation::FallingHoldTime
    )
}

fn uses_signal_threshold_as_level(aggregation: &AnalogAggregation) -> bool {
    matches!(
        aggregation,
        AnalogAggregation::RisingCrossingTime
            | AnalogAggregation::FallingCrossingTime
            | AnalogAggregation::MinHighPulseWidth
            | AnalogAggregation::MinLowPulseWidth
            | AnalogAggregation::RisingPhaseDelay
            | AnalogAggregation::FallingPhaseDelay
            | AnalogAggregation::RisingSetupTime
            | AnalogAggregation::RisingHoldTime
            | AnalogAggregation::FallingSetupTime
            | AnalogAggregation::FallingHoldTime
            | AnalogAggregation::DutyCycle
            | AnalogAggregation::CrossingCount
            | AnalogAggregation::RisingCrossingCount
            | AnalogAggregation::FallingCrossingCount
    )
}

fn uses_target_as_reference(aggregation: &AnalogAggregation) -> bool {
    matches!(
        aggregation,
        AnalogAggregation::SettlingTime | AnalogAggregation::OvershootPercent
    )
}

fn is_phase_delay_aggregation(aggregation: &AnalogAggregation) -> bool {
    matches!(
        aggregation,
        AnalogAggregation::RisingPhaseDelay | AnalogAggregation::FallingPhaseDelay
    )
}

fn is_reference_timing_aggregation(aggregation: &AnalogAggregation) -> bool {
    matches!(
        aggregation,
        AnalogAggregation::RisingPhaseDelay
            | AnalogAggregation::FallingPhaseDelay
            | AnalogAggregation::RisingSetupTime
            | AnalogAggregation::RisingHoldTime
            | AnalogAggregation::FallingSetupTime
            | AnalogAggregation::FallingHoldTime
    )
}

#[cfg(test)]
mod tests {
    use super::{
        target_count, threshold_count, threshold_for, tolerance_count, validate_assertion_contract,
        validate_probe_contract,
    };
    use crate::board_ir::{
        AnalogAggregation, AnalogAssertion, AnalogProbe, AnalogQuantity, AnalogRelation,
    };

    #[test]
    fn integral_and_energy_thresholds_use_integrated_units() {
        let current_probe = AnalogProbe {
            name: "load_current".to_string(),
            expression: "I(VLOAD)".to_string(),
            quantity: AnalogQuantity::Current,
        };
        let current_integral = AnalogAssertion {
            name: "charge_limit".to_string(),
            probe: "load_current".to_string(),
            reference_probe: None,
            at_us: None,
            start_us: Some(0.0),
            end_us: Some(100.0),
            time_limit_us: None,
            at_hz: None,
            frequency_limit_hz: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::Integral,
            relation: AnalogRelation::Below,
            threshold_v: None,
            threshold_a: None,
            threshold_w: None,
            threshold_vs: None,
            threshold_c: Some(1.0e-6),
            threshold_j: None,
            threshold_db: None,
            threshold_deg: None,
            threshold_v_per_sqrt_hz: None,
            reference_threshold_v: None,
            reference_threshold_a: None,
            reference_threshold_w: None,
            target_v: None,
            target_a: None,
            target_w: None,
            tolerance_v: None,
            tolerance_a: None,
            tolerance_w: None,
            overshoot_limit_percent: None,
            suggested_fixes: Vec::new(),
        };
        let threshold = threshold_for(&current_integral, &current_probe).unwrap();
        assert_eq!(threshold.unit, "C");
        assert_eq!(threshold.limit_key, "_C");

        let power_probe = AnalogProbe {
            name: "load_power".to_string(),
            expression: "V(out)*I(VLOAD)".to_string(),
            quantity: AnalogQuantity::Power,
        };
        let energy = AnalogAssertion {
            name: "energy_limit".to_string(),
            probe: "load_power".to_string(),
            reference_probe: None,
            at_us: None,
            start_us: Some(0.0),
            end_us: Some(100.0),
            time_limit_us: None,
            at_hz: None,
            frequency_limit_hz: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::Energy,
            relation: AnalogRelation::Below,
            threshold_v: None,
            threshold_a: None,
            threshold_w: Some(1.0),
            threshold_vs: None,
            threshold_c: None,
            threshold_j: None,
            threshold_db: None,
            threshold_deg: None,
            threshold_v_per_sqrt_hz: None,
            reference_threshold_v: None,
            reference_threshold_a: None,
            reference_threshold_w: None,
            target_v: None,
            target_a: None,
            target_w: None,
            tolerance_v: None,
            tolerance_a: None,
            tolerance_w: None,
            overshoot_limit_percent: None,
            suggested_fixes: Vec::new(),
        };
        assert!(threshold_for(&energy, &power_probe).is_none());
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
            reference_probe: None,
            at_us: Some(100.0),
            start_us: Some(0.0),
            end_us: None,
            time_limit_us: None,
            at_hz: None,
            frequency_limit_hz: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::Sample,
            relation: AnalogRelation::Above,
            threshold_v: Some(1.0),
            threshold_a: None,
            threshold_w: None,
            threshold_vs: None,
            threshold_c: None,
            threshold_j: None,
            threshold_db: None,
            threshold_deg: None,
            threshold_v_per_sqrt_hz: None,
            reference_threshold_v: None,
            reference_threshold_a: None,
            reference_threshold_w: None,
            target_v: None,
            target_a: None,
            target_w: None,
            tolerance_v: None,
            tolerance_a: None,
            tolerance_w: None,
            overshoot_limit_percent: None,
            suggested_fixes: Vec::new(),
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());

        let assertion = AnalogAssertion {
            name: "bad_units".to_string(),
            probe: "nrst".to_string(),
            reference_probe: None,
            at_us: Some(100.0),
            start_us: None,
            end_us: None,
            time_limit_us: None,
            at_hz: None,
            frequency_limit_hz: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::Sample,
            relation: AnalogRelation::Above,
            threshold_v: Some(1.0),
            threshold_a: Some(0.001),
            threshold_w: None,
            threshold_vs: None,
            threshold_c: None,
            threshold_j: None,
            threshold_db: None,
            threshold_deg: None,
            threshold_v_per_sqrt_hz: None,
            reference_threshold_v: None,
            reference_threshold_a: None,
            reference_threshold_w: None,
            target_v: None,
            target_a: None,
            target_w: None,
            tolerance_v: None,
            tolerance_a: None,
            tolerance_w: None,
            overshoot_limit_percent: None,
            suggested_fixes: Vec::new(),
        };
        assert_eq!(threshold_count(&assertion), 2);

        let assertion = AnalogAssertion {
            name: "bad_crossing".to_string(),
            probe: "nrst".to_string(),
            reference_probe: None,
            at_us: None,
            start_us: Some(0.0),
            end_us: Some(1000.0),
            time_limit_us: None,
            at_hz: None,
            frequency_limit_hz: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::RisingCrossingTime,
            relation: AnalogRelation::Below,
            threshold_v: Some(1.0),
            threshold_a: None,
            threshold_w: None,
            threshold_vs: None,
            threshold_c: None,
            threshold_j: None,
            threshold_db: None,
            threshold_deg: None,
            threshold_v_per_sqrt_hz: None,
            reference_threshold_v: None,
            reference_threshold_a: None,
            reference_threshold_w: None,
            target_v: None,
            target_a: None,
            target_w: None,
            tolerance_v: None,
            tolerance_a: None,
            tolerance_w: None,
            overshoot_limit_percent: None,
            suggested_fixes: Vec::new(),
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());

        let assertion = AnalogAssertion {
            name: "bad_duty".to_string(),
            probe: "nrst".to_string(),
            reference_probe: None,
            at_us: None,
            start_us: Some(0.0),
            end_us: Some(1000.0),
            time_limit_us: None,
            at_hz: None,
            frequency_limit_hz: None,
            duty_limit_percent: Some(101.0),
            count_limit: None,
            aggregation: AnalogAggregation::DutyCycle,
            relation: AnalogRelation::Below,
            threshold_v: Some(1.0),
            threshold_a: None,
            threshold_w: None,
            threshold_vs: None,
            threshold_c: None,
            threshold_j: None,
            threshold_db: None,
            threshold_deg: None,
            threshold_v_per_sqrt_hz: None,
            reference_threshold_v: None,
            reference_threshold_a: None,
            reference_threshold_w: None,
            target_v: None,
            target_a: None,
            target_w: None,
            tolerance_v: None,
            tolerance_a: None,
            tolerance_w: None,
            overshoot_limit_percent: None,
            suggested_fixes: Vec::new(),
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());

        let assertion = AnalogAssertion {
            name: "bad_count".to_string(),
            probe: "nrst".to_string(),
            reference_probe: None,
            at_us: None,
            start_us: Some(0.0),
            end_us: Some(1000.0),
            time_limit_us: None,
            at_hz: None,
            frequency_limit_hz: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::CrossingCount,
            relation: AnalogRelation::Below,
            threshold_v: Some(1.0),
            threshold_a: None,
            threshold_w: None,
            threshold_vs: None,
            threshold_c: None,
            threshold_j: None,
            threshold_db: None,
            threshold_deg: None,
            threshold_v_per_sqrt_hz: None,
            reference_threshold_v: None,
            reference_threshold_a: None,
            reference_threshold_w: None,
            target_v: None,
            target_a: None,
            target_w: None,
            tolerance_v: None,
            tolerance_a: None,
            tolerance_w: None,
            overshoot_limit_percent: None,
            suggested_fixes: Vec::new(),
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());

        let assertion = AnalogAssertion {
            name: "bad_settling_reference".to_string(),
            probe: "nrst".to_string(),
            reference_probe: None,
            at_us: None,
            start_us: Some(0.0),
            end_us: Some(1000.0),
            time_limit_us: Some(20.0),
            at_hz: None,
            frequency_limit_hz: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::SettlingTime,
            relation: AnalogRelation::Below,
            threshold_v: None,
            threshold_a: None,
            threshold_w: None,
            threshold_vs: None,
            threshold_c: None,
            threshold_j: None,
            threshold_db: None,
            threshold_deg: None,
            threshold_v_per_sqrt_hz: None,
            reference_threshold_v: None,
            reference_threshold_a: None,
            reference_threshold_w: None,
            target_v: Some(3.3),
            target_a: None,
            target_w: None,
            tolerance_v: None,
            tolerance_a: None,
            tolerance_w: None,
            overshoot_limit_percent: None,
            suggested_fixes: Vec::new(),
        };
        assert_eq!(target_count(&assertion), 1);
        assert_eq!(tolerance_count(&assertion), 0);
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());

        let assertion = AnalogAssertion {
            name: "good_overshoot_reference".to_string(),
            probe: "nrst".to_string(),
            reference_probe: None,
            at_us: None,
            start_us: Some(0.0),
            end_us: Some(1000.0),
            time_limit_us: None,
            at_hz: None,
            frequency_limit_hz: None,
            duty_limit_percent: None,
            count_limit: None,
            aggregation: AnalogAggregation::OvershootPercent,
            relation: AnalogRelation::Below,
            threshold_v: None,
            threshold_a: None,
            threshold_w: None,
            threshold_vs: None,
            threshold_c: None,
            threshold_j: None,
            threshold_db: None,
            threshold_deg: None,
            threshold_v_per_sqrt_hz: None,
            reference_threshold_v: None,
            reference_threshold_a: None,
            reference_threshold_w: None,
            target_v: Some(3.3),
            target_a: None,
            target_w: None,
            tolerance_v: None,
            tolerance_a: None,
            tolerance_w: None,
            overshoot_limit_percent: Some(10.0),
            suggested_fixes: Vec::new(),
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_ok());
    }
}
