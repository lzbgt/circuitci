use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepScenario {
    pub(super) name: String,
    pub(super) sweeps: Vec<AnalogSweepSummary>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepSummary {
    pub(super) name: String,
    pub(super) corner_count: usize,
    pub(super) parameters: Vec<AnalogSweepParameterSummary>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepParameterSummary {
    pub(super) name: String,
    pub(super) values: Vec<f64>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepDraft {
    pub(super) scenario_name: String,
    pub(super) sweep_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepParameterDraft {
    pub(super) scenario_name: String,
    pub(super) sweep_name: String,
    pub(super) parameter_name: String,
    pub(super) values_csv: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepPreset {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) sweep_name: &'static str,
    pub(super) summary: &'static str,
    pub(super) parameters: &'static [AnalogSweepPresetParameter],
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepPresetParameter {
    pub(super) name: &'static str,
    pub(super) values: &'static [f64],
}

const SUPPLY_VALUES: &[f64] = &[4.5, 5.0, 5.5];
const LOAD_VALUES: &[f64] = &[900.0, 1000.0, 1100.0];
const TEMPERATURE_VALUES: &[f64] = &[-40.0, 25.0, 85.0];
const MODEL_CORNER_VALUES: &[f64] = &[0.0, 1.0, 2.0];
const R_TOLERANCE_VALUES: &[f64] = &[950.0, 1000.0, 1050.0];
const C_TOLERANCE_VALUES: &[f64] = &[0.000000095, 0.0000001, 0.000000105];

const ANALOG_SWEEP_PRESETS: &[AnalogSweepPreset] = &[
    AnalogSweepPreset {
        id: "supply_5v",
        label: "Supply 5 V +/-10%",
        sweep_name: "supply_5v_corner",
        summary: "SUPPLY_V = 4.5, 5.0, 5.5",
        parameters: &[AnalogSweepPresetParameter {
            name: "SUPPLY_V",
            values: SUPPLY_VALUES,
        }],
    },
    AnalogSweepPreset {
        id: "load_1k",
        label: "Load 1 kOhm +/-10%",
        sweep_name: "load_1k_corner",
        summary: "LOAD_OHM = 900, 1000, 1100",
        parameters: &[AnalogSweepPresetParameter {
            name: "LOAD_OHM",
            values: LOAD_VALUES,
        }],
    },
    AnalogSweepPreset {
        id: "temperature",
        label: "Temperature -40/25/85 C",
        sweep_name: "temperature_corner",
        summary: "TEMP_C = -40, 25, 85 and ngspice .temp",
        parameters: &[AnalogSweepPresetParameter {
            name: "TEMP_C",
            values: TEMPERATURE_VALUES,
        }],
    },
    AnalogSweepPreset {
        id: "model_corner",
        label: "Model selector 0/1/2",
        sweep_name: "model_corner",
        summary: "MODEL_CORNER = 0, 1, 2 for parametric model decks",
        parameters: &[AnalogSweepPresetParameter {
            name: "MODEL_CORNER",
            values: MODEL_CORNER_VALUES,
        }],
    },
    AnalogSweepPreset {
        id: "rc_tolerance",
        label: "R/C +/-5%",
        sweep_name: "rc_tolerance",
        summary: "RIN_VALUE x COUT_VALUE = 9 corners",
        parameters: &[
            AnalogSweepPresetParameter {
                name: "RIN_VALUE",
                values: R_TOLERANCE_VALUES,
            },
            AnalogSweepPresetParameter {
                name: "COUT_VALUE",
                values: C_TOLERANCE_VALUES,
            },
        ],
    },
];

pub(super) fn analog_sweep_presets() -> &'static [AnalogSweepPreset] {
    ANALOG_SWEEP_PRESETS
}

pub(super) fn analog_sweep_scenarios(text: &str) -> Result<Vec<AnalogSweepScenario>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    Ok(project
        .scenarios
        .iter()
        .filter_map(|scenario| {
            let analog = scenario.analog.as_ref()?;
            Some(AnalogSweepScenario {
                name: scenario.name.clone(),
                sweeps: analog
                    .sweeps
                    .iter()
                    .map(|sweep| {
                        let parameters: Vec<_> = sweep
                            .parameters
                            .iter()
                            .map(|parameter| AnalogSweepParameterSummary {
                                name: parameter.name.clone(),
                                values: parameter.values.clone(),
                            })
                            .collect();
                        AnalogSweepSummary {
                            name: sweep.name.clone(),
                            corner_count: if parameters.is_empty() {
                                0
                            } else {
                                parameters
                                    .iter()
                                    .map(|parameter| parameter.values.len().max(1))
                                    .product()
                            },
                            parameters,
                        }
                    })
                    .collect(),
            })
        })
        .collect())
}

pub(super) fn append_analog_sweep_preset(
    text: &str,
    scenario_name: &str,
    preset_id: &str,
) -> Result<String> {
    let scenario_name = validated_id(scenario_name, "run setup")?;
    let preset = analog_sweep_presets()
        .iter()
        .find(|preset| preset.id == preset_id)
        .with_context(|| format!("Run input sweep preset {preset_id} was not found."))?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Run setup {scenario_name} was not found."))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Run setup {scenario_name} is not analog."))?;
    if analog
        .sweeps
        .iter()
        .any(|sweep| sweep.name == preset.sweep_name)
    {
        anyhow::bail!(
            "Sweep {} already exists in run setup {scenario_name}.",
            preset.sweep_name
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let analog_mapping = analog_mapping_mut(&mut yaml, scenario_name)?;
    let sweeps = ensure_child_sequence_mut(analog_mapping, "sweeps", "analog sweeps")?;
    let mut sweep = serde_yaml_ng::Mapping::new();
    insert_string(&mut sweep, "name", preset.sweep_name);
    sweep.insert(
        key("parameters"),
        serde_yaml_ng::Value::Sequence(
            preset
                .parameters
                .iter()
                .map(preset_parameter_value)
                .collect(),
        ),
    );
    sweeps.push(serde_yaml_ng::Value::Mapping(sweep));
    validate_updated_yaml(yaml)
}

pub(super) fn append_analog_sweep_with_parameter(
    text: &str,
    draft: &AnalogSweepParameterDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let parameter_name = validated_spice_parameter_name(&draft.parameter_name)?;
    let values = parse_sweep_values(&draft.values_csv)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Run setup {scenario_name} was not found."))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Run setup {scenario_name} is not analog."))?;
    if analog.sweeps.iter().any(|sweep| sweep.name == sweep_name) {
        anyhow::bail!("Sweep {sweep_name} already exists in run setup {scenario_name}.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let analog_mapping = analog_mapping_mut(&mut yaml, scenario_name)?;
    let sweeps = ensure_child_sequence_mut(analog_mapping, "sweeps", "analog sweeps")?;
    let mut sweep = serde_yaml_ng::Mapping::new();
    insert_string(&mut sweep, "name", sweep_name);
    let mut parameter = serde_yaml_ng::Mapping::new();
    insert_string(&mut parameter, "name", parameter_name);
    parameter.insert(
        key("values"),
        serde_yaml_ng::Value::Sequence(
            values
                .iter()
                .map(|value| serde_yaml_ng::Value::Number((*value).into()))
                .collect(),
        ),
    );
    sweep.insert(
        key("parameters"),
        serde_yaml_ng::Value::Sequence(vec![serde_yaml_ng::Value::Mapping(parameter)]),
    );
    sweeps.push(serde_yaml_ng::Value::Mapping(sweep));
    validate_updated_yaml(yaml)
}

pub(super) fn remove_analog_sweep(text: &str, draft: &AnalogSweepDraft) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let analog_mapping = analog_mapping_mut(&mut yaml, scenario_name)?;
    let sweeps = ensure_child_sequence_mut(analog_mapping, "sweeps", "analog sweeps")?;
    let before = sweeps.len();
    sweeps.retain(|sweep| {
        sweep
            .as_mapping()
            .and_then(|mapping| mapping.get(key("name")))
            .and_then(serde_yaml_ng::Value::as_str)
            != Some(sweep_name)
    });
    if sweeps.len() == before {
        anyhow::bail!("Sweep {sweep_name} was not found in run setup {scenario_name}.");
    }
    validate_updated_yaml(yaml)
}

pub(super) fn append_analog_sweep_parameter(
    text: &str,
    draft: &AnalogSweepParameterDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let parameter_name = validated_spice_parameter_name(&draft.parameter_name)?;
    let values = parse_sweep_values(&draft.values_csv)?;

    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Run setup {scenario_name} was not found."))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Run setup {scenario_name} is not analog."))?;
    let sweep = analog
        .sweeps
        .iter()
        .find(|sweep| sweep.name == sweep_name)
        .with_context(|| format!("Sweep {sweep_name} was not found."))?;
    if sweep
        .parameters
        .iter()
        .any(|parameter| parameter.name == parameter_name)
    {
        anyhow::bail!("Parameter {parameter_name} already exists in sweep {sweep_name}.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let parameters = sweep_parameters_mut(&mut yaml, scenario_name, sweep_name)?;
    let mut parameter = serde_yaml_ng::Mapping::new();
    insert_string(&mut parameter, "name", parameter_name);
    parameter.insert(
        key("values"),
        serde_yaml_ng::Value::Sequence(
            values
                .iter()
                .map(|value| serde_yaml_ng::Value::Number((*value).into()))
                .collect(),
        ),
    );
    parameters.push(serde_yaml_ng::Value::Mapping(parameter));
    validate_updated_yaml(yaml)
}

pub(super) fn remove_analog_sweep_parameter(
    text: &str,
    draft: &AnalogSweepParameterDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let parameter_name = validated_spice_parameter_name(&draft.parameter_name)?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let parameters = sweep_parameters_mut(&mut yaml, scenario_name, sweep_name)?;
    let before = parameters.len();
    parameters.retain(|parameter| {
        parameter
            .as_mapping()
            .and_then(|mapping| mapping.get(key("name")))
            .and_then(serde_yaml_ng::Value::as_str)
            != Some(parameter_name)
    });
    if parameters.len() == before {
        anyhow::bail!("Parameter {parameter_name} was not found in sweep {sweep_name}.");
    }
    if parameters.is_empty() {
        anyhow::bail!(
            "Sweep {sweep_name} must keep at least one parameter. Remove the sweep instead."
        );
    }
    validate_updated_yaml(yaml)
}

fn preset_parameter_value(parameter: &AnalogSweepPresetParameter) -> serde_yaml_ng::Value {
    let mut mapping = serde_yaml_ng::Mapping::new();
    insert_string(&mut mapping, "name", parameter.name);
    mapping.insert(
        key("values"),
        serde_yaml_ng::Value::Sequence(
            parameter
                .values
                .iter()
                .map(|value| serde_yaml_ng::Value::Number((*value).into()))
                .collect(),
        ),
    );
    serde_yaml_ng::Value::Mapping(mapping)
}

fn parse_sweep_values(text: &str) -> Result<Vec<f64>> {
    let mut values = Vec::new();
    for raw in text.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let value: f64 = raw
            .parse()
            .with_context(|| format!("Sweep value {raw} is not a number."))?;
        if !value.is_finite() {
            anyhow::bail!("Sweep value {raw} must be finite.");
        }
        values.push(value);
    }
    if values.is_empty() {
        anyhow::bail!("Sweep parameter values must include at least one number.");
    }
    Ok(values)
}

fn validated_id<'a>(value: &'a str, label: &str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{label} must not be blank.");
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        anyhow::bail!("{label} {value} contains unsupported characters.");
    }
    Ok(value)
}

fn validated_spice_parameter_name(value: &str) -> Result<&str> {
    let value = value.trim();
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        anyhow::bail!("SPICE parameter name must not be blank.");
    };
    if (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        Ok(value)
    } else {
        anyhow::bail!("SPICE parameter name {value} must match [A-Za-z_][A-Za-z0-9_]*.")
    }
}

fn validate_updated_yaml(yaml: serde_yaml_ng::Value) -> Result<String> {
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog sweep YAML is not valid Board IR.")?;
    Ok(updated)
}

fn analog_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    scenario_name: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let scenarios = yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?
        .get_mut(key("scenarios"))
        .context("Board IR project must declare scenarios.")?
        .as_sequence_mut()
        .context("Board IR scenarios must be a YAML list.")?;
    for scenario in scenarios {
        let Some(mapping) = scenario.as_mapping_mut() else {
            continue;
        };
        let is_target = mapping
            .get(key("name"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(scenario_name);
        if is_target {
            return mapping
                .get_mut(key("analog"))
                .context("Run setup must declare analog.")?
                .as_mapping_mut()
                .context("Run setup analog must be a YAML object.");
        }
    }
    anyhow::bail!("Run setup {scenario_name} was not found.");
}

fn sweep_parameters_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    scenario_name: &str,
    sweep_name: &str,
) -> Result<&'a mut Vec<serde_yaml_ng::Value>> {
    let analog = analog_mapping_mut(yaml, scenario_name)?;
    let sweeps = ensure_child_sequence_mut(analog, "sweeps", "analog sweeps")?;
    for sweep in sweeps {
        let Some(mapping) = sweep.as_mapping_mut() else {
            continue;
        };
        let is_target = mapping
            .get(key("name"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(sweep_name);
        if is_target {
            return ensure_child_sequence_mut(mapping, "parameters", "sweep parameters");
        }
    }
    anyhow::bail!("Sweep {sweep_name} was not found in run setup {scenario_name}.");
}

fn ensure_child_sequence_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    field: &str,
    label: &str,
) -> Result<&'a mut Vec<serde_yaml_ng::Value>> {
    let key = key(field);
    if !mapping.contains_key(&key) {
        mapping.insert(key.clone(), serde_yaml_ng::Value::Sequence(Vec::new()));
    }
    mapping
        .get_mut(&key)
        .expect("field was inserted when absent")
        .as_sequence_mut()
        .with_context(|| format!("{label} must be a YAML list."))
}

fn insert_string(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: &str) {
    mapping.insert(
        key(name),
        serde_yaml_ng::Value::String(value.trim().to_string()),
    );
}

fn key(value: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        AnalogSweepParameterDraft, analog_sweep_scenarios, append_analog_sweep_parameter,
        append_analog_sweep_preset, append_analog_sweep_with_parameter,
        remove_analog_sweep_parameter,
    };

    fn project_yaml() -> &'static str {
        "project:
  name: gui_sweep_test
  version: 0.1.0
board:
  components: {}
  nets:
    gnd:
      kind: ground
scenarios:
  - name: rc_run
    type: analog_transient
    analog:
      backend: auto
      netlist_source: file
      netlist: deck.cir
      model_files: []
      node_bindings:
        - { node: '0', net: gnd }
      pin_bindings: []
      analysis:
        type: tran
        stop_time_us: 1000
        max_step_us: 1
      stimuli: []
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"
    }

    #[test]
    fn append_sweep_and_parameter_emit_valid_yaml() {
        let edited = append_analog_sweep_with_parameter(
            project_yaml(),
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_tolerance".to_string(),
                parameter_name: "RIN_VALUE".to_string(),
                values_csv: "950, 1000, 1050".to_string(),
            },
        )
        .unwrap();
        let edited = append_analog_sweep_parameter(
            &edited,
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_tolerance".to_string(),
                parameter_name: "COUT_VALUE".to_string(),
                values_csv: "9.5e-8, 1.0e-7".to_string(),
            },
        )
        .unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
        assert_eq!(sweep.name, "rc_tolerance");
        assert_eq!(sweep.parameters.len(), 2);

        let scenarios = analog_sweep_scenarios(&edited).unwrap();
        assert_eq!(scenarios[0].sweeps[0].corner_count, 6);
    }

    #[test]
    fn append_temperature_preset_emits_executable_sweep() {
        let edited = append_analog_sweep_preset(project_yaml(), "rc_run", "temperature").unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
        assert_eq!(sweep.name, "temperature_corner");
        assert_eq!(sweep.parameters[0].name, "TEMP_C");
        assert_eq!(sweep.parameters[0].values, vec![-40.0, 25.0, 85.0]);

        let scenarios = analog_sweep_scenarios(&edited).unwrap();
        assert_eq!(scenarios[0].sweeps[0].corner_count, 3);
    }

    #[test]
    fn append_rc_tolerance_preset_emits_nine_corners() {
        let edited = append_analog_sweep_preset(project_yaml(), "rc_run", "rc_tolerance").unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
        assert_eq!(sweep.name, "rc_tolerance");
        assert_eq!(sweep.parameters.len(), 2);
        assert_eq!(sweep.parameters[0].name, "RIN_VALUE");
        assert_eq!(sweep.parameters[1].name, "COUT_VALUE");

        let scenarios = analog_sweep_scenarios(&edited).unwrap();
        assert_eq!(scenarios[0].sweeps[0].corner_count, 9);
    }

    #[test]
    fn append_sweep_preset_rejects_duplicate_name() {
        let edited = append_analog_sweep_preset(project_yaml(), "rc_run", "load_1k").unwrap();

        let error = append_analog_sweep_preset(&edited, "rc_run", "load_1k")
            .unwrap_err()
            .to_string();

        assert!(error.contains("already exists"));
    }

    #[test]
    fn sweep_parameter_remove_preserves_sweep_when_other_parameters_remain() {
        let edited = append_analog_sweep_with_parameter(
            project_yaml(),
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_tolerance".to_string(),
                parameter_name: "RIN_VALUE".to_string(),
                values_csv: "950, 1000, 1050".to_string(),
            },
        )
        .unwrap();
        let edited = append_analog_sweep_parameter(
            &edited,
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_tolerance".to_string(),
                parameter_name: "COUT_VALUE".to_string(),
                values_csv: "9.5e-8, 1.0e-7".to_string(),
            },
        )
        .unwrap();
        let edited = remove_analog_sweep_parameter(
            &edited,
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_tolerance".to_string(),
                parameter_name: "RIN_VALUE".to_string(),
                values_csv: String::new(),
            },
        )
        .unwrap();
        let scenarios = analog_sweep_scenarios(&edited).unwrap();

        assert_eq!(scenarios[0].sweeps[0].name, "rc_tolerance");
        assert_eq!(scenarios[0].sweeps[0].parameters.len(), 1);
        assert_eq!(scenarios[0].sweeps[0].parameters[0].name, "COUT_VALUE");
    }

    #[test]
    fn sweep_parameter_remove_rejects_last_parameter() {
        let edited = append_analog_sweep_with_parameter(
            project_yaml(),
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_tolerance".to_string(),
                parameter_name: "RIN_VALUE".to_string(),
                values_csv: "950, 1000, 1050".to_string(),
            },
        )
        .unwrap();
        let error = remove_analog_sweep_parameter(
            &edited,
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_tolerance".to_string(),
                parameter_name: "RIN_VALUE".to_string(),
                values_csv: String::new(),
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("Remove the sweep instead"));
    }

    #[test]
    fn sweep_parameter_rejects_invalid_spice_name() {
        let edited = append_analog_sweep_with_parameter(
            project_yaml(),
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_tolerance".to_string(),
                parameter_name: "RIN_VALUE".to_string(),
                values_csv: "950, 1000, 1050".to_string(),
            },
        )
        .unwrap();
        let error = append_analog_sweep_parameter(
            &edited,
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_tolerance".to_string(),
                parameter_name: "1BAD".to_string(),
                values_csv: "1".to_string(),
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("SPICE parameter name"));
    }
}
