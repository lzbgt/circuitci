use anyhow::{Context, Result};

use super::analog::{AnalogAssertionDraft, append_analog_assertion};

#[derive(Debug, Clone)]
pub(super) struct AnalogAcAssertionPreset {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) summary: &'static str,
}

const ANALOG_AC_ASSERTION_PRESETS: &[AnalogAcAssertionPreset] = &[
    AnalogAcAssertionPreset {
        id: "lowpass_1khz",
        label: "Low-pass 1 kHz",
        summary: "Gain < -1 dB, phase < -20 deg, -3 dB cutoff > 1.4 kHz",
    },
    AnalogAcAssertionPreset {
        id: "unity_1khz",
        label: "Unity gain 1 kHz",
        summary: "-0.5..0.5 dB gain and -10..10 deg phase at 1 kHz",
    },
    AnalogAcAssertionPreset {
        id: "loop_stability",
        label: "Loop stability",
        summary: "Phase margin > 45 deg and gain margin > 6 dB",
    },
];

pub(super) fn analog_ac_assertion_presets() -> &'static [AnalogAcAssertionPreset] {
    ANALOG_AC_ASSERTION_PRESETS
}

pub(super) fn append_analog_ac_assertion_preset(
    text: &str,
    scenario_name: &str,
    probe_name: &str,
    preset_id: &str,
) -> Result<String> {
    validate_target(text, scenario_name, probe_name)?;
    let preset = analog_ac_assertion_presets()
        .iter()
        .find(|preset| preset.id == preset_id)
        .with_context(|| format!("AC/Bode observation preset {preset_id} was not found."))?;
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
    if scenario.scenario_type != "analog_ac" {
        anyhow::bail!(
            "AC/Bode observation presets require an analog_ac run setup; scenario {} is {}.",
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
        "lowpass_1khz" => vec![
            ac_assertion(
                scenario_name,
                probe_name,
                AcAssertionSpec {
                    name_suffix: "gain_at_1khz_below_minus_1db",
                    aggregation: "gain_db_at_frequency",
                    relation: "below",
                    threshold: -1.0,
                    at_hz: 1000.0,
                    frequency_limit_hz: 0.0,
                },
            ),
            ac_assertion(
                scenario_name,
                probe_name,
                AcAssertionSpec {
                    name_suffix: "phase_at_1khz_below_minus_20deg",
                    aggregation: "phase_deg_at_frequency",
                    relation: "below",
                    threshold: -20.0,
                    at_hz: 1000.0,
                    frequency_limit_hz: 0.0,
                },
            ),
            ac_assertion(
                scenario_name,
                probe_name,
                AcAssertionSpec {
                    name_suffix: "cutoff_above_1_4khz",
                    aggregation: "falling_gain_crossing_frequency",
                    relation: "above",
                    threshold: -3.0,
                    at_hz: 1000.0,
                    frequency_limit_hz: 1400.0,
                },
            ),
        ],
        "unity_1khz" => vec![
            ac_assertion(
                scenario_name,
                probe_name,
                AcAssertionSpec {
                    name_suffix: "unity_gain_at_1khz_above_minus_0_5db",
                    aggregation: "gain_db_at_frequency",
                    relation: "above",
                    threshold: -0.5,
                    at_hz: 1000.0,
                    frequency_limit_hz: 0.0,
                },
            ),
            ac_assertion(
                scenario_name,
                probe_name,
                AcAssertionSpec {
                    name_suffix: "unity_gain_at_1khz_below_0_5db",
                    aggregation: "gain_db_at_frequency",
                    relation: "below",
                    threshold: 0.5,
                    at_hz: 1000.0,
                    frequency_limit_hz: 0.0,
                },
            ),
            ac_assertion(
                scenario_name,
                probe_name,
                AcAssertionSpec {
                    name_suffix: "unity_phase_at_1khz_above_minus_10deg",
                    aggregation: "phase_deg_at_frequency",
                    relation: "above",
                    threshold: -10.0,
                    at_hz: 1000.0,
                    frequency_limit_hz: 0.0,
                },
            ),
            ac_assertion(
                scenario_name,
                probe_name,
                AcAssertionSpec {
                    name_suffix: "unity_phase_at_1khz_below_10deg",
                    aggregation: "phase_deg_at_frequency",
                    relation: "below",
                    threshold: 10.0,
                    at_hz: 1000.0,
                    frequency_limit_hz: 0.0,
                },
            ),
        ],
        "loop_stability" => vec![
            ac_assertion(
                scenario_name,
                probe_name,
                AcAssertionSpec {
                    name_suffix: "phase_margin_above_45deg",
                    aggregation: "phase_margin_deg",
                    relation: "above",
                    threshold: 45.0,
                    at_hz: 0.0,
                    frequency_limit_hz: 0.0,
                },
            ),
            ac_assertion(
                scenario_name,
                probe_name,
                AcAssertionSpec {
                    name_suffix: "gain_margin_above_6db",
                    aggregation: "gain_margin_db",
                    relation: "above",
                    threshold: 6.0,
                    at_hz: 0.0,
                    frequency_limit_hz: 0.0,
                },
            ),
        ],
        _ => Vec::new(),
    }
}

#[derive(Debug, Clone, Copy)]
struct AcAssertionSpec {
    name_suffix: &'static str,
    aggregation: &'static str,
    relation: &'static str,
    threshold: f64,
    at_hz: f64,
    frequency_limit_hz: f64,
}

fn ac_assertion(
    scenario_name: &str,
    probe_name: &str,
    spec: AcAssertionSpec,
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
        frequency_limit_hz: spec.frequency_limit_hz,
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
    use super::append_analog_ac_assertion_preset;

    fn project_yaml() -> &'static str {
        "project:
  name: ac_preset_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins: {P: input, N: gnd}
      spice: {primitive: dc_voltage_source, dc_v: 1.0}
  nets:
    input: {kind: power, nominal_voltage: 1.0, powered: true}
    filtered: {kind: digital_or_analog}
    gnd: {kind: ground}
scenarios:
  - name: bode
    type: analog_ac
    checks: [SPICE_AC_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1]
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: input, net: input}
        - {node: filtered, net: filtered}
      pin_bindings:
        - {node: input, endpoint: {component: V1, pin: P}}
        - {node: '0', endpoint: {component: V1, pin: N}}
      analysis: {type: ac, start_frequency_hz: 10.0, stop_frequency_hz: 100000.0, points_per_decade: 20}
      stimuli: []
      probes:
        - {name: filtered, expression: V(filtered)}
      assertions: []
"
    }

    #[test]
    fn lowpass_preset_appends_standard_bode_checks() {
        let edited =
            append_analog_ac_assertion_preset(project_yaml(), "bode", "filtered", "lowpass_1khz")
                .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = &project.scenarios[0].analog.as_ref().unwrap().assertions;
        assert_eq!(assertions.len(), 3);
        assert_eq!(assertions[0].name, "filtered_gain_at_1khz_below_minus_1db");
        assert_eq!(
            assertions[0].aggregation,
            crate::board_ir::AnalogAggregation::GainDbAtFrequency
        );
        assert_eq!(assertions[0].threshold_db, Some(-1.0));
        assert_eq!(assertions[1].threshold_deg, Some(-20.0));
        assert_eq!(
            assertions[2].aggregation,
            crate::board_ir::AnalogAggregation::FallingGainCrossingFrequency
        );
        assert_eq!(assertions[2].frequency_limit_hz, Some(1400.0));
    }

    #[test]
    fn ac_preset_rejects_duplicate_assertion_names() {
        let edited =
            append_analog_ac_assertion_preset(project_yaml(), "bode", "filtered", "unity_1khz")
                .unwrap();
        let error = append_analog_ac_assertion_preset(&edited, "bode", "filtered", "unity_1khz")
            .unwrap_err();
        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    fn stability_preset_appends_margin_checks() {
        let edited =
            append_analog_ac_assertion_preset(project_yaml(), "bode", "filtered", "loop_stability")
                .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = &project.scenarios[0].analog.as_ref().unwrap().assertions;
        assert_eq!(assertions.len(), 2);
        assert_eq!(assertions[0].name, "filtered_phase_margin_above_45deg");
        assert_eq!(
            assertions[0].aggregation,
            crate::board_ir::AnalogAggregation::PhaseMarginDeg
        );
        assert_eq!(assertions[0].threshold_deg, Some(45.0));
        assert_eq!(assertions[1].name, "filtered_gain_margin_above_6db");
        assert_eq!(
            assertions[1].aggregation,
            crate::board_ir::AnalogAggregation::GainMarginDb
        );
        assert_eq!(assertions[1].threshold_db, Some(6.0));
    }
}
