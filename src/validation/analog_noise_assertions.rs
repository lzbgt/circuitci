use crate::board_ir::{AnalogAggregation, AnalogAssertion, AnalogRelation, Scenario};
use crate::reports::Finding;
use serde_json::json;
use std::fs;
use std::path::Path;

use super::SPICE_NOISE_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_util::normalize_artifact_path;
use super::common::validation_input_missing;

struct NoiseSpectrum {
    frequency_hz: Vec<f64>,
    output_density: Vec<f64>,
    input_density: Vec<f64>,
}

struct NoiseTotal {
    output_rms_v: f64,
    input_rms_v: f64,
}

pub(super) fn validate_noise_assertion_contract(
    assertion: &AnalogAssertion,
    start_hz: f64,
    stop_hz: f64,
) -> Result<(), String> {
    if !is_noise_aggregation(&assertion.aggregation) {
        return Err("analog_noise assertions require a noise aggregation".to_string());
    }
    if assertion.at_us.is_some()
        || assertion.start_us.is_some()
        || assertion.end_us.is_some()
        || assertion.time_limit_us.is_some()
        || assertion.frequency_limit_hz.is_some()
        || assertion.duty_limit_percent.is_some()
        || assertion.count_limit.is_some()
        || assertion.overshoot_limit_percent.is_some()
        || assertion.threshold_a.is_some()
        || assertion.threshold_w.is_some()
        || assertion.threshold_vs.is_some()
        || assertion.threshold_s.is_some()
        || assertion.threshold_c.is_some()
        || assertion.threshold_j.is_some()
        || assertion.threshold_db.is_some()
        || assertion.threshold_deg.is_some()
        || assertion.reference_probe.is_some()
        || assertion.reference_threshold_v.is_some()
        || assertion.reference_threshold_a.is_some()
        || assertion.reference_threshold_w.is_some()
        || assertion.target_v.is_some()
        || assertion.target_a.is_some()
        || assertion.target_w.is_some()
        || assertion.tolerance_v.is_some()
        || assertion.tolerance_a.is_some()
        || assertion.tolerance_w.is_some()
    {
        return Err(
            "noise aggregation must only declare frequency and noise voltage thresholds"
                .to_string(),
        );
    }
    match assertion.aggregation {
        AnalogAggregation::OutputNoiseDensityAtFrequency
        | AnalogAggregation::InputNoiseDensityAtFrequency => {
            let at_hz = finite_field(assertion.at_hz, "at_hz")?;
            if at_hz < start_hz || at_hz > stop_hz {
                return Err("at_hz must be inside the noise sweep frequency range".to_string());
            }
            finite_field(assertion.threshold_v_per_sqrt_hz, "threshold_v_per_sqrt_hz")?;
            reject_field(assertion.threshold_v, "threshold_v")?;
        }
        AnalogAggregation::IntegratedOutputNoise | AnalogAggregation::IntegratedInputNoise => {
            if assertion.at_hz.is_some() {
                return Err("integrated noise aggregation must not declare at_hz".to_string());
            }
            finite_field(assertion.threshold_v, "threshold_v")?;
            reject_field(assertion.threshold_v_per_sqrt_hz, "threshold_v_per_sqrt_hz")?;
        }
        _ => unreachable!("filtered by is_noise_aggregation"),
    }
    Ok(())
}

pub(super) fn evaluate_noise_assertions(
    scenario: &Scenario,
    noise_spectrum: &Path,
    noise_total: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let mut measurements = Vec::new();
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before noise assertion evaluation");
    let spectrum = match parse_noise_spectrum_csv(noise_spectrum) {
        Ok(spectrum) => spectrum,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_NOISE_ANALYSIS,
                &scenario.name,
                format!(
                    "Failed to parse noise spectrum {}: {message}",
                    noise_spectrum.display()
                ),
            ));
            return measurements;
        }
    };
    let total = match parse_noise_total_csv(noise_total) {
        Ok(total) => total,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_NOISE_ANALYSIS,
                &scenario.name,
                format!(
                    "Failed to parse total noise {}: {message}",
                    noise_total.display()
                ),
            ));
            return measurements;
        }
    };
    for assertion in &analog.assertions {
        let measured = match measure_noise_assertion(assertion, &spectrum, &total) {
            Ok(value) => value,
            Err(message) => {
                validation_input_missing(
                    findings,
                    scenario,
                    format!("Analog noise assertion {} {message}.", assertion.name),
                );
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
            unit: unit.to_string(),
            quantity: quantity.clone(),
            passed,
        });
        if !passed {
            let mut finding = Finding::critical(
                SPICE_NOISE_ANALYSIS,
                &scenario.name,
                format!(
                    "Noise assertion {} failed: {} measured {:.6} {}, expected {relation} {:.6} {}.",
                    assertion.name,
                    noise_aggregation_label(&assertion.aggregation),
                    measured,
                    unit,
                    limit,
                    unit
                ),
            );
            finding
                .measured
                .insert("assertion".to_string(), json!(&assertion.name));
            finding
                .measured
                .insert(assertion.probe.clone(), json!(measured));
            finding
                .measured
                .insert("quantity".to_string(), json!(quantity));
            finding.measured.insert(
                "noise_spectrum".to_string(),
                json!(normalize_artifact_path(noise_spectrum)),
            );
            finding.measured.insert(
                "noise_total".to_string(),
                json!(normalize_artifact_path(noise_total)),
            );
            if let Some(at_hz) = assertion.at_hz {
                finding.measured.insert("at_hz".to_string(), json!(at_hz));
            }
            finding
                .limit
                .insert(format!("{relation}_{unit}"), json!(limit));
            if assertion.suggested_fixes.is_empty() {
                finding.suggested_fixes.push(
                    "Adjust noise-sensitive resistor values, bandwidth, source impedance, filtering, or the selected device model so the simulated noise meets the declared limit."
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

fn measure_noise_assertion(
    assertion: &AnalogAssertion,
    spectrum: &NoiseSpectrum,
    total: &NoiseTotal,
) -> Result<f64, String> {
    match assertion.aggregation {
        AnalogAggregation::OutputNoiseDensityAtFrequency => interpolate_log_frequency(
            &spectrum.frequency_hz,
            &spectrum.output_density,
            finite_field(assertion.at_hz, "at_hz")?,
        ),
        AnalogAggregation::InputNoiseDensityAtFrequency => interpolate_log_frequency(
            &spectrum.frequency_hz,
            &spectrum.input_density,
            finite_field(assertion.at_hz, "at_hz")?,
        ),
        AnalogAggregation::IntegratedOutputNoise => Ok(total.output_rms_v),
        AnalogAggregation::IntegratedInputNoise => Ok(total.input_rms_v),
        _ => Err("unsupported noise aggregation".to_string()),
    }
}

fn comparison_limit(assertion: &AnalogAssertion) -> (f64, &'static str, String) {
    match assertion.aggregation {
        AnalogAggregation::OutputNoiseDensityAtFrequency => (
            assertion.threshold_v_per_sqrt_hz.unwrap_or_default(),
            "V/sqrt(Hz)",
            "output_noise_density".to_string(),
        ),
        AnalogAggregation::InputNoiseDensityAtFrequency => (
            assertion.threshold_v_per_sqrt_hz.unwrap_or_default(),
            "V/sqrt(Hz)",
            "input_noise_density".to_string(),
        ),
        AnalogAggregation::IntegratedOutputNoise => (
            assertion.threshold_v.unwrap_or_default(),
            "V",
            "integrated_output_noise".to_string(),
        ),
        AnalogAggregation::IntegratedInputNoise => (
            assertion.threshold_v.unwrap_or_default(),
            "V",
            "integrated_input_noise".to_string(),
        ),
        _ => unreachable!("only noise aggregations are evaluated here"),
    }
}

fn noise_aggregation_label(aggregation: &AnalogAggregation) -> &'static str {
    match aggregation {
        AnalogAggregation::OutputNoiseDensityAtFrequency => "output noise density",
        AnalogAggregation::InputNoiseDensityAtFrequency => "input-referred noise density",
        AnalogAggregation::IntegratedOutputNoise => "integrated output RMS noise",
        AnalogAggregation::IntegratedInputNoise => "integrated input-referred RMS noise",
        _ => "noise response",
    }
}

fn is_noise_aggregation(aggregation: &AnalogAggregation) -> bool {
    matches!(
        aggregation,
        AnalogAggregation::OutputNoiseDensityAtFrequency
            | AnalogAggregation::InputNoiseDensityAtFrequency
            | AnalogAggregation::IntegratedOutputNoise
            | AnalogAggregation::IntegratedInputNoise
    )
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

fn parse_noise_spectrum_csv(path: &Path) -> Result<NoiseSpectrum, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read noise spectrum {}: {error}", path.display()))?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "noise spectrum CSV is empty".to_string())?;
    let names = split_fields(header);
    if names
        != [
            "frequency_hz",
            "onoise_v_per_sqrt_hz",
            "inoise_v_per_sqrt_hz",
        ]
    {
        return Err("noise spectrum CSV has unexpected columns".to_string());
    }
    let mut frequency_hz = Vec::new();
    let mut output_density = Vec::new();
    let mut input_density = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let fields = split_fields(line);
        if fields.len() != 3 {
            return Err(format!(
                "noise spectrum row {} has {} columns, expected 3",
                line_index + 2,
                fields.len()
            ));
        }
        let frequency = parse_finite(fields[0]).ok_or_else(|| {
            format!(
                "noise spectrum row {} has invalid frequency",
                line_index + 2
            )
        })?;
        if frequency <= 0.0 || frequency_hz.last().is_some_and(|last| frequency <= *last) {
            return Err(format!(
                "noise spectrum row {} has non-positive or non-increasing frequency",
                line_index + 2
            ));
        }
        frequency_hz.push(frequency);
        output_density.push(parse_finite(fields[1]).ok_or_else(|| {
            format!(
                "noise spectrum row {} has invalid output density",
                line_index + 2
            )
        })?);
        input_density.push(parse_finite(fields[2]).ok_or_else(|| {
            format!(
                "noise spectrum row {} has invalid input density",
                line_index + 2
            )
        })?);
    }
    if frequency_hz.is_empty() {
        return Err("noise spectrum CSV has no data rows".to_string());
    }
    Ok(NoiseSpectrum {
        frequency_hz,
        output_density,
        input_density,
    })
}

fn parse_noise_total_csv(path: &Path) -> Result<NoiseTotal, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read total noise {}: {error}", path.display()))?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "total noise CSV is empty".to_string())?;
    let names = split_fields(header);
    if names != ["onoise_total_v", "inoise_total_v"] {
        return Err("total noise CSV has unexpected columns".to_string());
    }
    let value_line = lines
        .next()
        .ok_or_else(|| "total noise CSV has no value row".to_string())?;
    let fields = split_fields(value_line);
    if fields.len() != 2 {
        return Err("total noise value row must have 2 columns".to_string());
    }
    Ok(NoiseTotal {
        output_rms_v: parse_finite(fields[0])
            .ok_or_else(|| "total output noise is invalid".to_string())?,
        input_rms_v: parse_finite(fields[1])
            .ok_or_else(|| "total input noise is invalid".to_string())?,
    })
}

fn interpolate_log_frequency(
    frequency_hz: &[f64],
    values: &[f64],
    sample_hz: f64,
) -> Result<f64, String> {
    if !sample_hz.is_finite() || sample_hz <= 0.0 {
        return Err("sample frequency must be positive and finite".to_string());
    }
    let first = frequency_hz[0];
    let last = *frequency_hz
        .last()
        .expect("noise spectrum has at least one frequency");
    if sample_hz < first || sample_hz > last {
        return Err(format!(
            "sample frequency {sample_hz} Hz is outside noise sweep range {first}..{last} Hz"
        ));
    }
    for (index, frequency) in frequency_hz.iter().copied().enumerate() {
        if (frequency - sample_hz).abs() <= f64::EPSILON * sample_hz.max(1.0) {
            return Ok(values[index]);
        }
    }
    for (frequency_pair, value_pair) in frequency_hz.windows(2).zip(values.windows(2)) {
        let f0 = frequency_pair[0];
        let f1 = frequency_pair[1];
        if sample_hz >= f0 && sample_hz <= f1 {
            let span = f1.log10() - f0.log10();
            if span <= 0.0 {
                return Err("noise frequency span is not increasing".to_string());
            }
            let fraction = (sample_hz.log10() - f0.log10()) / span;
            return Ok(value_pair[0] + fraction * (value_pair[1] - value_pair[0]));
        }
    }
    Err("sample frequency was not bracketed by noise spectrum".to_string())
}

fn split_fields(line: &str) -> Vec<&str> {
    line.split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

fn parse_finite(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::{evaluate_noise_assertions, validate_noise_assertion_contract};
    use crate::board_ir::{AnalogAggregation, BoardProject};
    use crate::reports::Finding;

    #[test]
    fn noise_assertions_evaluate_density_and_integrated_noise() {
        let project: BoardProject = serde_yaml_ng::from_str(
            r#"
project:
  name: noise_assertion
  version: 0.1.0
board:
  components: {}
  nets: {}
scenarios:
  - name: output_noise
    type: analog_noise
    checks: [SPICE_NOISE_ANALYSIS]
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis:
        type: noise
        start_frequency_hz: 10
        stop_frequency_hz: 10000
        points_per_decade: 10
        noise_output_node: out
        noise_input_source: VIN
      stimuli: []
      probes:
        - { name: onoise, expression: onoise_spectrum }
      assertions:
        - name: density_100hz_below
          probe: onoise
          aggregation: output_noise_density_at_frequency
          relation: below
          at_hz: 100
          threshold_v_per_sqrt_hz: 5.0e-9
        - name: integrated_below
          probe: onoise
          aggregation: integrated_output_noise
          relation: below
          threshold_v: 1.0e-6
"#,
        )
        .unwrap();
        let scenario = &project.scenarios[0];
        let analog = scenario.analog.as_ref().unwrap();
        for assertion in &analog.assertions {
            validate_noise_assertion_contract(assertion, 10.0, 10_000.0).unwrap();
        }
        let dir = tempfile::tempdir().unwrap();
        let spectrum = dir.path().join("noise_spectrum.csv");
        std::fs::write(
            &spectrum,
            "frequency_hz,onoise_v_per_sqrt_hz,inoise_v_per_sqrt_hz\n1.0e1,2.0e-9,4.0e-9\n1.0e2,3.0e-9,6.0e-9\n1.0e3,4.0e-9,8.0e-9\n",
        )
        .unwrap();
        let total = dir.path().join("noise_total.csv");
        std::fs::write(&total, "onoise_total_v,inoise_total_v\n2.0e-7,4.0e-7\n").unwrap();
        let mut findings = Vec::<Finding>::new();
        let measurements = evaluate_noise_assertions(scenario, &spectrum, &total, &mut findings);
        assert!(findings.is_empty());
        assert_eq!(measurements.len(), 2);
        assert_eq!(measurements[0].assertion_name, "density_100hz_below");
        assert_eq!(measurements[0].quantity, "output_noise_density");
        assert_eq!(measurements[1].quantity, "integrated_output_noise");
        assert!(measurements.iter().all(|measurement| measurement.passed));
    }

    #[test]
    fn noise_contract_rejects_transient_aggregation() {
        let project: BoardProject = serde_yaml_ng::from_str(
            r#"
project:
  name: noise_contract
  version: 0.1.0
board:
  components: {}
  nets: {}
scenarios:
  - name: output_noise
    type: analog_noise
    checks: [SPICE_NOISE_ANALYSIS]
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis:
        type: noise
        start_frequency_hz: 10
        stop_frequency_hz: 10000
        points_per_decade: 10
        noise_output_node: out
        noise_input_source: VIN
      stimuli: []
      probes:
        - { name: onoise, expression: onoise_spectrum }
      assertions:
        - name: sample
          probe: onoise
          aggregation: sample
          relation: below
          at_us: 1.0
          threshold_v: 1.0
"#,
        )
        .unwrap();
        let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
        assert_eq!(assertion.aggregation, AnalogAggregation::Sample);
        let error = validate_noise_assertion_contract(assertion, 10.0, 10_000.0).unwrap_err();
        assert!(error.contains("noise aggregation"));
    }
}
