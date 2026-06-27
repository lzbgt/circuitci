use anyhow::{Context, Result};

use super::analog::{AnalogAssertionDraft, append_analog_assertion};

#[derive(Debug, Clone)]
pub(super) struct AnalogNoiseAssertionPreset {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) summary: &'static str,
}

const ANALOG_NOISE_ASSERTION_PRESETS: &[AnalogNoiseAssertionPreset] = &[
    AnalogNoiseAssertionPreset {
        id: "divider_output_noise",
        label: "Divider output",
        summary: "Output density < 10 nV/sqrt(Hz) at 1 kHz and output RMS < 3.5 uV",
    },
    AnalogNoiseAssertionPreset {
        id: "input_referred_noise",
        label: "Input-referred",
        summary: "Input noise density < 20 nV/sqrt(Hz) at 1 kHz and input RMS < 7 uV",
    },
];

pub(super) fn analog_noise_assertion_presets() -> &'static [AnalogNoiseAssertionPreset] {
    ANALOG_NOISE_ASSERTION_PRESETS
}

pub(super) fn append_analog_noise_assertion_preset(
    text: &str,
    scenario_name: &str,
    probe_name: &str,
    preset_id: &str,
) -> Result<String> {
    validate_target(text, scenario_name, probe_name)?;
    let preset = analog_noise_assertion_presets()
        .iter()
        .find(|preset| preset.id == preset_id)
        .with_context(|| format!("Noise observation preset {preset_id} was not found."))?;
    let mut updated = text.to_string();
    for draft in preset_assertions(preset.id, scenario_name, probe_name) {
        updated = append_analog_assertion(&updated, &draft)?;
    }
    Ok(updated)
}

fn validate_target(text: &str, scenario_name: &str, probe_name: &str) -> Result<()> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Scenario {scenario_name} was not found."))?;
    if scenario.scenario_type != "analog_noise" {
        anyhow::bail!(
            "Noise observation presets require an analog_noise run setup; scenario {} is {}.",
            scenario.name,
            scenario.scenario_type
        );
    }
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if !analog.probes.iter().any(|probe| probe.name == probe_name) {
        anyhow::bail!(
            "Probe {probe_name} was not found in scenario {}.",
            scenario.name
        );
    }
    Ok(())
}

fn preset_assertions(
    preset_id: &str,
    scenario_name: &str,
    probe_name: &str,
) -> Vec<AnalogAssertionDraft> {
    match preset_id {
        "divider_output_noise" => vec![
            noise_assertion(
                scenario_name,
                probe_name,
                NoiseAssertionSpec {
                    name_suffix: "output_density_1khz_below_10nv",
                    aggregation: "output_noise_density_at_frequency",
                    relation: "below",
                    threshold: 1.0e-8,
                    at_hz: 1000.0,
                },
            ),
            noise_assertion(
                scenario_name,
                probe_name,
                NoiseAssertionSpec {
                    name_suffix: "output_rms_below_3_5uv",
                    aggregation: "integrated_output_noise",
                    relation: "below",
                    threshold: 3.5e-6,
                    at_hz: 0.0,
                },
            ),
        ],
        "input_referred_noise" => vec![
            noise_assertion(
                scenario_name,
                probe_name,
                NoiseAssertionSpec {
                    name_suffix: "input_density_1khz_below_20nv",
                    aggregation: "input_noise_density_at_frequency",
                    relation: "below",
                    threshold: 2.0e-8,
                    at_hz: 1000.0,
                },
            ),
            noise_assertion(
                scenario_name,
                probe_name,
                NoiseAssertionSpec {
                    name_suffix: "input_rms_below_7uv",
                    aggregation: "integrated_input_noise",
                    relation: "below",
                    threshold: 7.0e-6,
                    at_hz: 0.0,
                },
            ),
        ],
        _ => Vec::new(),
    }
}

#[derive(Debug, Clone, Copy)]
struct NoiseAssertionSpec {
    name_suffix: &'static str,
    aggregation: &'static str,
    relation: &'static str,
    threshold: f64,
    at_hz: f64,
}

fn noise_assertion(
    scenario_name: &str,
    probe_name: &str,
    spec: NoiseAssertionSpec,
) -> AnalogAssertionDraft {
    AnalogAssertionDraft {
        scenario_name: scenario_name.to_string(),
        assertion_name: format!("{}_{}", sanitize_id(probe_name), spec.name_suffix),
        probe_name: probe_name.to_string(),
        reference_probe: String::new(),
        aggregation: spec.aggregation.to_string(),
        relation: spec.relation.to_string(),
        threshold: spec.threshold,
        reference_threshold: 0.0,
        target: 0.0,
        tolerance: 0.0,
        at_us: 0.0,
        at_hz: spec.at_hz,
        start_us: 0.0,
        end_us: 0.0,
        time_limit_us: 0.0,
        frequency_limit_hz: 0.0,
        duty_limit_percent: 0.0,
        count_limit: 0.0,
        overshoot_limit_percent: 0.0,
    }
}

fn sanitize_id(value: &str) -> String {
    let mut id = String::new();
    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
            id.push(character);
        } else if !id.ends_with('_') {
            id.push('_');
        }
    }
    let id = id.trim_matches('_');
    if id.is_empty() {
        "probe".to_string()
    } else {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::append_analog_noise_assertion_preset;

    fn project_yaml() -> &'static str {
        "project:
  name: noise_preset_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins: {P: vin, N: gnd}
      spice: {primitive: dc_voltage_source, dc_v: 5.0}
  nets:
    vin: {kind: power, nominal_voltage: 5.0, powered: true}
    midpoint: {kind: digital_or_analog}
    gnd: {kind: ground}
scenarios:
  - name: noise
    type: analog_noise
    checks: [SPICE_NOISE_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1]
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: vin, net: vin}
        - {node: midpoint, net: midpoint}
      pin_bindings:
        - {node: vin, endpoint: {component: V1, pin: P}}
        - {node: '0', endpoint: {component: V1, pin: N}}
      analysis:
        type: noise
        start_frequency_hz: 10.0
        stop_frequency_hz: 100000.0
        points_per_decade: 20
        noise_output_node: midpoint
        noise_input_source: V1
      stimuli: []
      probes:
        - {name: onoise, expression: V(midpoint)}
        - {name: inoise, expression: V(vin)}
      assertions: []
"
    }

    #[test]
    fn noise_output_preset_appends_density_and_integrated_checks() {
        let edited = append_analog_noise_assertion_preset(
            project_yaml(),
            "noise",
            "onoise",
            "divider_output_noise",
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = &project.scenarios[0].analog.as_ref().unwrap().assertions;
        assert_eq!(assertions.len(), 2);
        assert_eq!(assertions[0].name, "onoise_output_density_1khz_below_10nv");
        assert_eq!(
            assertions[0].aggregation,
            crate::board_ir::AnalogAggregation::OutputNoiseDensityAtFrequency
        );
        assert_eq!(assertions[0].threshold_v_per_sqrt_hz, Some(1.0e-8));
        assert_eq!(assertions[0].at_hz, Some(1000.0));
        assert_eq!(
            assertions[1].aggregation,
            crate::board_ir::AnalogAggregation::IntegratedOutputNoise
        );
        assert_eq!(assertions[1].threshold_v, Some(3.5e-6));
    }

    #[test]
    fn noise_input_referred_preset_appends_input_noise_checks() {
        let edited = append_analog_noise_assertion_preset(
            project_yaml(),
            "noise",
            "inoise",
            "input_referred_noise",
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = &project.scenarios[0].analog.as_ref().unwrap().assertions;
        assert_eq!(assertions.len(), 2);
        assert_eq!(
            assertions[0].aggregation,
            crate::board_ir::AnalogAggregation::InputNoiseDensityAtFrequency
        );
        assert_eq!(assertions[0].threshold_v_per_sqrt_hz, Some(2.0e-8));
        assert_eq!(
            assertions[1].aggregation,
            crate::board_ir::AnalogAggregation::IntegratedInputNoise
        );
        assert_eq!(assertions[1].threshold_v, Some(7.0e-6));
    }

    #[test]
    fn noise_preset_rejects_duplicate_assertion_names() {
        let edited = append_analog_noise_assertion_preset(
            project_yaml(),
            "noise",
            "onoise",
            "divider_output_noise",
        )
        .unwrap();
        let error = append_analog_noise_assertion_preset(
            &edited,
            "noise",
            "onoise",
            "divider_output_noise",
        )
        .unwrap_err();
        assert!(error.to_string().contains("already exists"));
    }
}
