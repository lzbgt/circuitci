use anyhow::{Context, Result};

use super::analog::{AnalogAssertionDraft, append_analog_assertion};

#[derive(Debug, Clone)]
pub(super) struct AnalogDcAssertionPreset {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) summary: &'static str,
}

const ANALOG_DC_ASSERTION_PRESETS: &[AnalogDcAssertionPreset] = &[
    AnalogDcAssertionPreset {
        id: "rail_3v3",
        label: "3.3 V rail",
        summary: "Operating point between 3.135 V and 3.465 V",
    },
    AnalogDcAssertionPreset {
        id: "rail_5v",
        label: "5 V rail",
        summary: "Operating point between 4.75 V and 5.25 V",
    },
    AnalogDcAssertionPreset {
        id: "midpoint_2v5",
        label: "2.5 V midpoint",
        summary: "Operating point between 2.35 V and 2.65 V",
    },
];

pub(super) fn analog_dc_assertion_presets() -> &'static [AnalogDcAssertionPreset] {
    ANALOG_DC_ASSERTION_PRESETS
}

pub(super) fn append_analog_dc_assertion_preset(
    text: &str,
    scenario_name: &str,
    probe_name: &str,
    preset_id: &str,
) -> Result<String> {
    validate_target(text, scenario_name, probe_name)?;
    let preset = analog_dc_assertion_presets()
        .iter()
        .find(|preset| preset.id == preset_id)
        .with_context(|| format!("DC operating-point preset {preset_id} was not found."))?;
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
    if scenario.scenario_type != "analog_dc" {
        anyhow::bail!(
            "DC operating-point presets require an analog_dc run setup; scenario {} is {}.",
            scenario.name,
            scenario.scenario_type
        );
    }
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    let probe = analog
        .probes
        .iter()
        .find(|probe| probe.name == probe_name)
        .with_context(|| {
            format!(
                "Probe {probe_name} was not found in scenario {}.",
                scenario.name
            )
        })?;
    if probe.quantity != crate::board_ir::AnalogQuantity::Voltage {
        anyhow::bail!("DC operating-point presets currently require a voltage probe.");
    }
    Ok(())
}

fn preset_assertions(
    preset_id: &str,
    scenario_name: &str,
    probe_name: &str,
) -> Vec<AnalogAssertionDraft> {
    match preset_id {
        "rail_3v3" => dc_window_assertions(scenario_name, probe_name, "3v3", 3.135, 3.465),
        "rail_5v" => dc_window_assertions(scenario_name, probe_name, "5v", 4.75, 5.25),
        "midpoint_2v5" => dc_window_assertions(scenario_name, probe_name, "2v5", 2.35, 2.65),
        _ => Vec::new(),
    }
}

fn dc_window_assertions(
    scenario_name: &str,
    probe_name: &str,
    suffix: &str,
    min_v: f64,
    max_v: f64,
) -> Vec<AnalogAssertionDraft> {
    vec![
        dc_assertion(
            scenario_name,
            probe_name,
            &format!("{suffix}_above_min"),
            "above",
            min_v,
        ),
        dc_assertion(
            scenario_name,
            probe_name,
            &format!("{suffix}_below_max"),
            "below",
            max_v,
        ),
    ]
}

fn dc_assertion(
    scenario_name: &str,
    probe_name: &str,
    name_suffix: &str,
    relation: &str,
    threshold: f64,
) -> AnalogAssertionDraft {
    AnalogAssertionDraft {
        scenario_name: scenario_name.to_string(),
        assertion_name: format!("{}_{}", sanitize_id(probe_name), name_suffix),
        probe_name: probe_name.to_string(),
        reference_probe: String::new(),
        aggregation: "operating_point".to_string(),
        relation: relation.to_string(),
        threshold,
        reference_threshold: 0.0,
        target: 0.0,
        tolerance: 0.0,
        at_us: 0.0,
        at_hz: 0.0,
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
    use super::append_analog_dc_assertion_preset;

    fn project_yaml() -> &'static str {
        "project:
  name: dc_preset_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins: {P: rail_5v, N: gnd}
      spice: {primitive: dc_voltage_source, dc_v: 5.0}
  nets:
    rail_5v: {kind: power, nominal_voltage: 5.0, powered: true}
    gnd: {kind: ground}
scenarios:
  - name: bias
    type: analog_dc
    checks: [SPICE_DC_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1]
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: rail_5v, net: rail_5v}
      pin_bindings:
        - {node: rail_5v, endpoint: {component: V1, pin: P}}
        - {node: '0', endpoint: {component: V1, pin: N}}
      analysis: {type: op}
      stimuli: []
      probes:
        - {name: rail, expression: V(rail_5v)}
      assertions: []
"
    }

    #[test]
    fn dc_preset_appends_operating_point_window_checks() {
        let edited =
            append_analog_dc_assertion_preset(project_yaml(), "bias", "rail", "rail_5v").unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = &project.scenarios[0].analog.as_ref().unwrap().assertions;
        assert_eq!(assertions.len(), 2);
        assert_eq!(assertions[0].name, "rail_5v_above_min");
        assert_eq!(
            assertions[0].aggregation,
            crate::board_ir::AnalogAggregation::OperatingPoint
        );
        assert_eq!(assertions[0].threshold_v, Some(4.75));
        assert_eq!(assertions[1].name, "rail_5v_below_max");
        assert_eq!(assertions[1].threshold_v, Some(5.25));
    }

    #[test]
    fn dc_preset_rejects_duplicate_assertion_names() {
        let edited =
            append_analog_dc_assertion_preset(project_yaml(), "bias", "rail", "rail_3v3").unwrap();
        let error =
            append_analog_dc_assertion_preset(&edited, "bias", "rail", "rail_3v3").unwrap_err();
        assert!(error.to_string().contains("already exists"));
    }
}
