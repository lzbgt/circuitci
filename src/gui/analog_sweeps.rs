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
    pub(super) component_values: Vec<AnalogSweepComponentValueSummary>,
    pub(super) model_sections: Vec<AnalogSweepModelSectionSummary>,
    pub(super) monte_carlo: Option<AnalogMonteCarloSummary>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepParameterSummary {
    pub(super) name: String,
    pub(super) values: Vec<f64>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepComponentValueSummary {
    pub(super) component: String,
    pub(super) field: String,
    pub(super) values: Vec<f64>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepModelSectionSummary {
    pub(super) path: String,
    pub(super) sections: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogMonteCarloSummary {
    pub(super) samples: usize,
    pub(super) component_values: Vec<AnalogMonteCarloComponentValueSummary>,
    pub(super) criteria: Option<AnalogMonteCarloCriteriaSummary>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogMonteCarloCriteriaSummary {
    pub(super) min_yield_percent: Option<f64>,
    pub(super) min_p1_margin: Option<f64>,
    pub(super) min_p5_margin: Option<f64>,
    pub(super) min_p50_margin: Option<f64>,
    pub(super) min_p95_margin: Option<f64>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogMonteCarloComponentValueSummary {
    pub(super) component: String,
    pub(super) field: String,
    pub(super) nominal: f64,
    pub(super) tolerance_percent: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogLoadSweepCandidate {
    pub(super) component: String,
    pub(super) field: String,
    pub(super) nominal: f64,
    pub(super) values_csv: String,
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
pub(super) struct AnalogSweepModelSectionDraft {
    pub(super) scenario_name: String,
    pub(super) sweep_name: String,
    pub(super) path: String,
    pub(super) sections_csv: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSweepComponentValueDraft {
    pub(super) scenario_name: String,
    pub(super) sweep_name: String,
    pub(super) component: String,
    pub(super) field: String,
    pub(super) values_csv: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogMonteCarloCriteriaDraft {
    pub(super) scenario_name: String,
    pub(super) sweep_name: String,
    pub(super) min_yield_percent: String,
    pub(super) min_p1_margin: String,
    pub(super) min_p5_margin: String,
    pub(super) min_p50_margin: String,
    pub(super) min_p95_margin: String,
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
                        let component_values: Vec<_> = sweep
                            .component_values
                            .iter()
                            .map(|component_value| AnalogSweepComponentValueSummary {
                                component: component_value.component.clone(),
                                field: component_value.field.as_str().to_string(),
                                values: component_value.values.clone(),
                            })
                            .collect();
                        let model_sections: Vec<_> = sweep
                            .model_sections
                            .iter()
                            .map(|model_section| AnalogSweepModelSectionSummary {
                                path: model_section.path.clone(),
                                sections: model_section.sections.clone(),
                            })
                            .collect();
                        let parameter_corners: usize = parameters
                            .iter()
                            .map(|parameter| parameter.values.len().max(1))
                            .product();
                        let component_value_corners: usize = component_values
                            .iter()
                            .map(|component_value| component_value.values.len().max(1))
                            .product();
                        let model_section_corners: usize = model_sections
                            .iter()
                            .map(|model_section| model_section.sections.len().max(1))
                            .product();
                        let monte_carlo =
                            sweep
                                .monte_carlo
                                .as_ref()
                                .map(|monte_carlo| AnalogMonteCarloSummary {
                                    samples: monte_carlo.samples,
                                    component_values: monte_carlo
                                        .component_values
                                        .iter()
                                        .map(|component_value| {
                                            AnalogMonteCarloComponentValueSummary {
                                                component: component_value.component.clone(),
                                                field: component_value.field.as_str().to_string(),
                                                nominal: component_value.nominal,
                                                tolerance_percent: component_value
                                                    .tolerance_percent,
                                            }
                                        })
                                        .collect(),
                                    criteria: monte_carlo.criteria.as_ref().map(|criteria| {
                                        AnalogMonteCarloCriteriaSummary {
                                            min_yield_percent: criteria.min_yield_percent,
                                            min_p1_margin: criteria.min_p1_margin,
                                            min_p5_margin: criteria.min_p5_margin,
                                            min_p50_margin: criteria.min_p50_margin,
                                            min_p95_margin: criteria.min_p95_margin,
                                        }
                                    }),
                                });
                        let monte_carlo_corners = monte_carlo
                            .as_ref()
                            .map(|monte_carlo| monte_carlo.samples.max(1))
                            .unwrap_or(1);
                        AnalogSweepSummary {
                            name: sweep.name.clone(),
                            corner_count: parameter_corners
                                * component_value_corners
                                * model_section_corners
                                * monte_carlo_corners,
                            parameters,
                            component_values,
                            model_sections,
                            monte_carlo,
                        }
                    })
                    .collect(),
            })
        })
        .collect())
}

pub(super) fn analog_load_sweep_candidates(
    text: &str,
    scenario_name: &str,
) -> Result<Vec<AnalogLoadSweepCandidate>> {
    let scenario_name = scenario_name.trim();
    if scenario_name.is_empty() {
        return Ok(Vec::new());
    }
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Run setup {scenario_name} was not found."))?;
    let Some(analog) = &scenario.analog else {
        return Ok(Vec::new());
    };
    let Some(generated) = &analog.generated else {
        return Ok(Vec::new());
    };
    let mut candidates = Vec::new();
    for component_id in &generated.components {
        let Some(component) = project.board.components.get(component_id) else {
            continue;
        };
        let Some(spice) = &component.spice else {
            continue;
        };
        match spice.primitive {
            crate::board_ir::SpicePrimitive::Resistor => {
                push_candidate(
                    &mut candidates,
                    component_id,
                    "value_ohm",
                    spice.value_ohm,
                    true,
                );
            }
            crate::board_ir::SpicePrimitive::Capacitor => {
                push_candidate(
                    &mut candidates,
                    component_id,
                    "value_f",
                    spice.value_f,
                    true,
                );
            }
            crate::board_ir::SpicePrimitive::Inductor => {
                push_candidate(
                    &mut candidates,
                    component_id,
                    "value_h",
                    spice.value_h,
                    true,
                );
            }
            crate::board_ir::SpicePrimitive::DcVoltageSource => {
                push_candidate(&mut candidates, component_id, "dc_v", spice.dc_v, false);
            }
            crate::board_ir::SpicePrimitive::DcCurrentSource => {
                push_candidate(&mut candidates, component_id, "dc_a", spice.dc_a, false);
            }
            crate::board_ir::SpicePrimitive::PulseVoltageSource
            | crate::board_ir::SpicePrimitive::PulseCurrentSource => {}
        }
    }
    Ok(candidates)
}

fn push_candidate(
    candidates: &mut Vec<AnalogLoadSweepCandidate>,
    component: &str,
    field: &str,
    nominal: Option<f64>,
    positive: bool,
) {
    let Some(nominal) = nominal else {
        return;
    };
    if !nominal.is_finite() || positive && nominal <= 0.0 {
        return;
    }
    let values = if nominal == 0.0 {
        vec![-0.1, 0.0, 0.1]
    } else {
        vec![nominal * 0.9, nominal, nominal * 1.1]
    };
    candidates.push(AnalogLoadSweepCandidate {
        component: component.to_string(),
        field: field.to_string(),
        nominal,
        values_csv: values
            .iter()
            .map(|value| format_sweep_number(*value))
            .collect::<Vec<_>>()
            .join(", "),
    });
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

pub(super) fn append_analog_sweep_with_model_section(
    text: &str,
    draft: &AnalogSweepModelSectionDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let path = validated_model_section_path(&draft.path)?;
    let sections = parse_model_sections(&draft.sections_csv)?;
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
    sweep.insert(
        key("model_sections"),
        serde_yaml_ng::Value::Sequence(vec![model_section_value(path, &sections)]),
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

pub(super) fn append_analog_sweep_with_component_value(
    text: &str,
    draft: &AnalogSweepComponentValueDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let component = validated_component_id(&draft.component)?;
    let field = validated_component_value_field(&draft.field)?;
    let values = parse_sweep_values(&draft.values_csv)?;
    validate_component_value_range(field, &values)?;
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
    sweep.insert(
        key("component_values"),
        serde_yaml_ng::Value::Sequence(vec![component_value_entry(component, field, &values)]),
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

pub(super) fn append_analog_sweep_model_section(
    text: &str,
    draft: &AnalogSweepModelSectionDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let path = validated_model_section_path(&draft.path)?;
    let sections = parse_model_sections(&draft.sections_csv)?;

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
        .model_sections
        .iter()
        .any(|model_section| model_section.path == path)
    {
        anyhow::bail!("Model file {path} already exists in sweep {sweep_name}.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let sweep_mapping = sweep_mapping_mut(&mut yaml, scenario_name, sweep_name)?;
    let model_sections =
        ensure_child_sequence_mut(sweep_mapping, "model_sections", "sweep model sections")?;
    model_sections.push(model_section_value(path, &sections));
    validate_updated_yaml(yaml)
}

pub(super) fn append_analog_sweep_component_value(
    text: &str,
    draft: &AnalogSweepComponentValueDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let component = validated_component_id(&draft.component)?;
    let field = validated_component_value_field(&draft.field)?;
    let values = parse_sweep_values(&draft.values_csv)?;
    validate_component_value_range(field, &values)?;

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
        .component_values
        .iter()
        .any(|entry| entry.component == component && entry.field.as_str() == field)
    {
        anyhow::bail!("Component value {component}.{field} already exists in sweep {sweep_name}.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let sweep_mapping = sweep_mapping_mut(&mut yaml, scenario_name, sweep_name)?;
    let component_values =
        ensure_child_sequence_mut(sweep_mapping, "component_values", "sweep component values")?;
    component_values.push(component_value_entry(component, field, &values));
    validate_updated_yaml(yaml)
}

pub(super) fn remove_analog_sweep_model_section(
    text: &str,
    draft: &AnalogSweepModelSectionDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let path = validated_model_section_path(&draft.path)?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let sweep_mapping = sweep_mapping_mut(&mut yaml, scenario_name, sweep_name)?;
    let before_has_parameters = sweep_mapping
        .get(key("parameters"))
        .and_then(serde_yaml_ng::Value::as_sequence)
        .is_some_and(|parameters| !parameters.is_empty());
    let before_has_component_values = sweep_mapping
        .get(key("component_values"))
        .and_then(serde_yaml_ng::Value::as_sequence)
        .is_some_and(|component_values| !component_values.is_empty());
    let before_has_monte_carlo = sweep_mapping.contains_key(key("monte_carlo"));
    let model_sections =
        ensure_child_sequence_mut(sweep_mapping, "model_sections", "sweep model sections")?;
    let before = model_sections.len();
    model_sections.retain(|model_section| {
        model_section
            .as_mapping()
            .and_then(|mapping| mapping.get(key("path")))
            .and_then(serde_yaml_ng::Value::as_str)
            != Some(path)
    });
    if model_sections.len() == before {
        anyhow::bail!("Model file {path} was not found in sweep {sweep_name}.");
    }
    if model_sections.is_empty()
        && !before_has_parameters
        && !before_has_component_values
        && !before_has_monte_carlo
    {
        anyhow::bail!(
            "Sweep {sweep_name} must keep at least one parameter, component value, model section, or Monte Carlo block. Remove the sweep instead."
        );
    }
    validate_updated_yaml(yaml)
}

pub(super) fn remove_analog_sweep_component_value(
    text: &str,
    draft: &AnalogSweepComponentValueDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let component = validated_component_id(&draft.component)?;
    let field = validated_component_value_field(&draft.field)?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let sweep_mapping = sweep_mapping_mut(&mut yaml, scenario_name, sweep_name)?;
    let before_has_parameters = sweep_mapping
        .get(key("parameters"))
        .and_then(serde_yaml_ng::Value::as_sequence)
        .is_some_and(|parameters| !parameters.is_empty());
    let before_has_model_sections = sweep_mapping
        .get(key("model_sections"))
        .and_then(serde_yaml_ng::Value::as_sequence)
        .is_some_and(|model_sections| !model_sections.is_empty());
    let before_has_monte_carlo = sweep_mapping.contains_key(key("monte_carlo"));
    let component_values =
        ensure_child_sequence_mut(sweep_mapping, "component_values", "sweep component values")?;
    let before = component_values.len();
    component_values.retain(|component_value| {
        let Some(mapping) = component_value.as_mapping() else {
            return true;
        };
        let same_component = mapping
            .get(key("component"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(component);
        let same_field = mapping
            .get(key("field"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(field);
        !(same_component && same_field)
    });
    if component_values.len() == before {
        anyhow::bail!("Component value {component}.{field} was not found in sweep {sweep_name}.");
    }
    if component_values.is_empty()
        && !before_has_parameters
        && !before_has_model_sections
        && !before_has_monte_carlo
    {
        anyhow::bail!(
            "Sweep {sweep_name} must keep at least one parameter, component value, model section, or Monte Carlo block. Remove the sweep instead."
        );
    }
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
    let sweep_mapping = sweep_mapping_mut(&mut yaml, scenario_name, sweep_name)?;
    let before_has_model_sections = sweep_mapping
        .get(key("model_sections"))
        .and_then(serde_yaml_ng::Value::as_sequence)
        .is_some_and(|model_sections| !model_sections.is_empty());
    let before_has_component_values = sweep_mapping
        .get(key("component_values"))
        .and_then(serde_yaml_ng::Value::as_sequence)
        .is_some_and(|component_values| !component_values.is_empty());
    let before_has_monte_carlo = sweep_mapping.contains_key(key("monte_carlo"));
    let parameters = ensure_child_sequence_mut(sweep_mapping, "parameters", "sweep parameters")?;
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
    if parameters.is_empty()
        && !before_has_model_sections
        && !before_has_component_values
        && !before_has_monte_carlo
    {
        anyhow::bail!(
            "Sweep {sweep_name} must keep at least one parameter, component value, model section, or Monte Carlo block. Remove the sweep instead."
        );
    }
    validate_updated_yaml(yaml)
}

pub(super) fn set_analog_monte_carlo_criteria(
    text: &str,
    draft: &AnalogMonteCarloCriteriaDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let min_yield_percent =
        parse_optional_number(&draft.min_yield_percent, "minimum yield percent")?;
    if min_yield_percent.is_some_and(|value| !(0.0..=100.0).contains(&value)) {
        anyhow::bail!("Minimum yield percent must be between 0 and 100.");
    }
    let min_p1_margin = parse_optional_number(&draft.min_p1_margin, "minimum P1 margin")?;
    let min_p5_margin = parse_optional_number(&draft.min_p5_margin, "minimum P5 margin")?;
    let min_p50_margin = parse_optional_number(&draft.min_p50_margin, "minimum P50 margin")?;
    let min_p95_margin = parse_optional_number(&draft.min_p95_margin, "minimum P95 margin")?;

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
    if sweep.monte_carlo.is_none() {
        anyhow::bail!("Sweep {sweep_name} has no Monte Carlo block.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let sweep_mapping = sweep_mapping_mut(&mut yaml, scenario_name, sweep_name)?;
    let monte_carlo = sweep_mapping
        .get_mut(key("monte_carlo"))
        .context("Sweep Monte Carlo block was not found.")?
        .as_mapping_mut()
        .context("Sweep Monte Carlo block must be a YAML object.")?;
    let criteria_values = [
        ("min_yield_percent", min_yield_percent),
        ("min_p1_margin", min_p1_margin),
        ("min_p5_margin", min_p5_margin),
        ("min_p50_margin", min_p50_margin),
        ("min_p95_margin", min_p95_margin),
    ];
    if criteria_values.iter().all(|(_, value)| value.is_none()) {
        monte_carlo.remove(key("criteria"));
    } else {
        let mut criteria = serde_yaml_ng::Mapping::new();
        for (name, value) in criteria_values {
            if let Some(value) = value {
                criteria.insert(key(name), serde_yaml_ng::Value::Number(value.into()));
            }
        }
        monte_carlo.insert(key("criteria"), serde_yaml_ng::Value::Mapping(criteria));
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

fn model_section_value(path: &str, sections: &[String]) -> serde_yaml_ng::Value {
    let mut mapping = serde_yaml_ng::Mapping::new();
    insert_string(&mut mapping, "path", path);
    mapping.insert(
        key("sections"),
        serde_yaml_ng::Value::Sequence(
            sections
                .iter()
                .map(|section| serde_yaml_ng::Value::String(section.clone()))
                .collect(),
        ),
    );
    serde_yaml_ng::Value::Mapping(mapping)
}

fn component_value_entry(component: &str, field: &str, values: &[f64]) -> serde_yaml_ng::Value {
    let mut mapping = serde_yaml_ng::Mapping::new();
    insert_string(&mut mapping, "component", component);
    insert_string(&mut mapping, "field", field);
    mapping.insert(
        key("values"),
        serde_yaml_ng::Value::Sequence(
            values
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

fn parse_optional_number(text: &str, label: &str) -> Result<Option<f64>> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let value: f64 = text
        .parse()
        .with_context(|| format!("{label} {text} is not a number."))?;
    if !value.is_finite() {
        anyhow::bail!("{label} must be finite.");
    }
    Ok(Some(value))
}

fn format_sweep_number(value: f64) -> String {
    let text = format!("{value:.12}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn parse_model_sections(text: &str) -> Result<Vec<String>> {
    let mut sections = Vec::new();
    for raw in text.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        if !valid_model_section_name(raw) {
            anyhow::bail!("Model section {raw} contains unsupported characters.");
        }
        sections.push(raw.to_string());
    }
    if sections.is_empty() {
        anyhow::bail!("Model section list must include at least one section.");
    }
    Ok(sections)
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

fn validated_model_section_path(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("Model file path must not be blank.");
    }
    Ok(value)
}

fn validated_component_id(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("Component value sweep target must not be blank.");
    }
    Ok(value)
}

fn validated_component_value_field(value: &str) -> Result<&str> {
    let value = value.trim();
    match value {
        "value_ohm" | "value_f" | "value_h" | "dc_v" | "dc_a" => Ok(value),
        _ => anyhow::bail!(
            "Component value field {value} must be one of value_ohm, value_f, value_h, dc_v, or dc_a."
        ),
    }
}

fn validate_component_value_range(field: &str, values: &[f64]) -> Result<()> {
    if matches!(field, "value_ohm" | "value_f" | "value_h")
        && values.iter().any(|value| *value <= 0.0)
    {
        anyhow::bail!("Passive component value sweeps must use positive values.");
    }
    Ok(())
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

fn valid_model_section_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
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
    let sweep = sweep_mapping_mut(yaml, scenario_name, sweep_name)?;
    ensure_child_sequence_mut(sweep, "parameters", "sweep parameters")
}

fn sweep_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    scenario_name: &str,
    sweep_name: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
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
            return Ok(mapping);
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
        AnalogMonteCarloCriteriaDraft, AnalogSweepComponentValueDraft,
        AnalogSweepModelSectionDraft, AnalogSweepParameterDraft, analog_load_sweep_candidates,
        analog_sweep_scenarios, append_analog_sweep_component_value,
        append_analog_sweep_model_section, append_analog_sweep_parameter,
        append_analog_sweep_preset, append_analog_sweep_with_component_value,
        append_analog_sweep_with_model_section, append_analog_sweep_with_parameter,
        remove_analog_sweep_component_value, remove_analog_sweep_model_section,
        remove_analog_sweep_parameter, set_analog_monte_carlo_criteria,
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

    fn generated_load_project_yaml() -> &'static str {
        "project:
  name: gui_load_sweep_test
  version: 0.1.0
board:
  components:
    RLOAD:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: out
        B: gnd
    ILOAD:
      model: generic.analog.dc_current_source
      spice:
        primitive: dc_current_source
        dc_a: 0.01
      pins:
        P: out
        N: gnd
  nets:
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
scenarios:
  - name: load_run
    type: analog_transient
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [RLOAD, ILOAD]
      model_files: []
      node_bindings:
        - { node: out, net: out }
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

    fn monte_carlo_project_yaml() -> &'static str {
        "project:
  name: gui_monte_carlo_sweep_test
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
      sweeps:
        - name: rc_monte_carlo
          monte_carlo:
            samples: 8
            seed: 7
            criteria:
              min_yield_percent: 95.0
              min_p5_margin: 0.1
            component_values:
              - component: RLOAD
                field: value_ohm
                nominal: 1000.0
                tolerance_percent: 5.0
              - component: CLOAD
                field: value_f
                nominal: 0.0000001
                tolerance_percent: 10.0
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"
    }

    #[test]
    fn analog_sweep_scenarios_reports_monte_carlo_inputs() {
        let scenarios = analog_sweep_scenarios(monte_carlo_project_yaml()).unwrap();

        let sweep = &scenarios[0].sweeps[0];
        assert_eq!(sweep.name, "rc_monte_carlo");
        assert_eq!(sweep.corner_count, 8);
        let monte_carlo = sweep.monte_carlo.as_ref().unwrap();
        assert_eq!(monte_carlo.samples, 8);
        assert_eq!(monte_carlo.component_values[0].component, "RLOAD");
        assert_eq!(monte_carlo.component_values[0].field, "value_ohm");
        assert_eq!(monte_carlo.component_values[0].nominal, 1000.0);
        assert_eq!(monte_carlo.component_values[0].tolerance_percent, 5.0);
        let criteria = monte_carlo.criteria.as_ref().unwrap();
        assert_eq!(criteria.min_yield_percent, Some(95.0));
        assert_eq!(criteria.min_p5_margin, Some(0.1));
        assert_eq!(criteria.min_p1_margin, None);
    }

    #[test]
    fn set_monte_carlo_criteria_emits_valid_yaml() {
        let edited = set_analog_monte_carlo_criteria(
            monte_carlo_project_yaml(),
            &AnalogMonteCarloCriteriaDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_monte_carlo".to_string(),
                min_yield_percent: "99.5".to_string(),
                min_p1_margin: "0.01".to_string(),
                min_p5_margin: "0.02".to_string(),
                min_p50_margin: "0.10".to_string(),
                min_p95_margin: "0.20".to_string(),
            },
        )
        .unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let criteria = project.scenarios[0].analog.as_ref().unwrap().sweeps[0]
            .monte_carlo
            .as_ref()
            .unwrap()
            .criteria
            .as_ref()
            .unwrap();
        assert_eq!(criteria.min_yield_percent, Some(99.5));
        assert_eq!(criteria.min_p1_margin, Some(0.01));
        assert_eq!(criteria.min_p5_margin, Some(0.02));
        assert_eq!(criteria.min_p50_margin, Some(0.10));
        assert_eq!(criteria.min_p95_margin, Some(0.20));
    }

    #[test]
    fn clear_monte_carlo_criteria_removes_criteria() {
        let edited = set_analog_monte_carlo_criteria(
            monte_carlo_project_yaml(),
            &AnalogMonteCarloCriteriaDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_monte_carlo".to_string(),
                min_yield_percent: String::new(),
                min_p1_margin: String::new(),
                min_p5_margin: String::new(),
                min_p50_margin: String::new(),
                min_p95_margin: String::new(),
            },
        )
        .unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        assert!(
            project.scenarios[0].analog.as_ref().unwrap().sweeps[0]
                .monte_carlo
                .as_ref()
                .unwrap()
                .criteria
                .is_none()
        );
    }

    #[test]
    fn set_monte_carlo_criteria_rejects_invalid_yield_percent() {
        let error = set_analog_monte_carlo_criteria(
            monte_carlo_project_yaml(),
            &AnalogMonteCarloCriteriaDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_monte_carlo".to_string(),
                min_yield_percent: "101".to_string(),
                min_p1_margin: String::new(),
                min_p5_margin: String::new(),
                min_p50_margin: String::new(),
                min_p95_margin: String::new(),
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("between 0 and 100"));
    }

    #[test]
    fn set_monte_carlo_criteria_rejects_non_monte_carlo_sweep() {
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
        let error = set_analog_monte_carlo_criteria(
            &edited,
            &AnalogMonteCarloCriteriaDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "rc_tolerance".to_string(),
                min_yield_percent: "95".to_string(),
                min_p1_margin: String::new(),
                min_p5_margin: "0".to_string(),
                min_p50_margin: String::new(),
                min_p95_margin: String::new(),
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("has no Monte Carlo block"));
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
    fn append_component_value_sweep_emits_valid_yaml() {
        let edited = append_analog_sweep_with_component_value(
            project_yaml(),
            &AnalogSweepComponentValueDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "load_corner".to_string(),
                component: "RLOAD".to_string(),
                field: "value_ohm".to_string(),
                values_csv: "900, 1000, 1100".to_string(),
            },
        )
        .unwrap();
        let edited = append_analog_sweep_component_value(
            &edited,
            &AnalogSweepComponentValueDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "load_corner".to_string(),
                component: "ILOAD".to_string(),
                field: "dc_a".to_string(),
                values_csv: "0.005, 0.01".to_string(),
            },
        )
        .unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
        assert_eq!(sweep.component_values.len(), 2);
        assert_eq!(sweep.component_values[0].component, "RLOAD");

        let scenarios = analog_sweep_scenarios(&edited).unwrap();
        assert_eq!(scenarios[0].sweeps[0].corner_count, 6);
        assert_eq!(
            scenarios[0].sweeps[0].component_values[0].field,
            "value_ohm"
        );
    }

    #[test]
    fn generated_load_sweep_candidates_project_component_fields() {
        let candidates =
            analog_load_sweep_candidates(generated_load_project_yaml(), "load_run").unwrap();

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].component, "RLOAD");
        assert_eq!(candidates[0].field, "value_ohm");
        assert_eq!(candidates[0].values_csv, "900, 1000, 1100");
        assert_eq!(candidates[1].component, "ILOAD");
        assert_eq!(candidates[1].field, "dc_a");
        assert_eq!(candidates[1].values_csv, "0.009, 0.01, 0.011");
    }

    #[test]
    fn remove_component_value_preserves_sweep_when_parameter_remains() {
        let edited = append_analog_sweep_with_parameter(
            project_yaml(),
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "mixed_corner".to_string(),
                parameter_name: "RIN_VALUE".to_string(),
                values_csv: "950, 1050".to_string(),
            },
        )
        .unwrap();
        let edited = append_analog_sweep_component_value(
            &edited,
            &AnalogSweepComponentValueDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "mixed_corner".to_string(),
                component: "RLOAD".to_string(),
                field: "value_ohm".to_string(),
                values_csv: "900, 1100".to_string(),
            },
        )
        .unwrap();

        let edited = remove_analog_sweep_component_value(
            &edited,
            &AnalogSweepComponentValueDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "mixed_corner".to_string(),
                component: "RLOAD".to_string(),
                field: "value_ohm".to_string(),
                values_csv: String::new(),
            },
        )
        .unwrap();

        let scenarios = analog_sweep_scenarios(&edited).unwrap();
        assert_eq!(scenarios[0].sweeps[0].parameters.len(), 1);
        assert!(scenarios[0].sweeps[0].component_values.is_empty());
    }

    #[test]
    fn append_model_section_emits_valid_corner_sweep() {
        let edited = append_analog_sweep_with_parameter(
            project_yaml(),
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "model_corner".to_string(),
                parameter_name: "RIN_VALUE".to_string(),
                values_csv: "950, 1050".to_string(),
            },
        )
        .unwrap();
        let edited = append_analog_sweep_model_section(
            &edited,
            &AnalogSweepModelSectionDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "model_corner".to_string(),
                path: "models/vendor.lib".to_string(),
                sections_csv: "typ, slow, fast".to_string(),
            },
        )
        .unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
        assert_eq!(sweep.model_sections[0].path, "models/vendor.lib");
        assert_eq!(
            sweep.model_sections[0].sections,
            vec!["typ", "slow", "fast"]
        );

        let scenarios = analog_sweep_scenarios(&edited).unwrap();
        assert_eq!(scenarios[0].sweeps[0].corner_count, 6);
        assert_eq!(
            scenarios[0].sweeps[0].model_sections[0].path,
            "models/vendor.lib"
        );
    }

    #[test]
    fn append_sweep_with_model_section_emits_model_only_sweep() {
        let edited = append_analog_sweep_with_model_section(
            project_yaml(),
            &AnalogSweepModelSectionDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "model_corner".to_string(),
                path: "models/vendor.lib".to_string(),
                sections_csv: "typ, slow, fast".to_string(),
            },
        )
        .unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
        assert!(sweep.parameters.is_empty());
        assert_eq!(sweep.model_sections[0].path, "models/vendor.lib");

        let scenarios = analog_sweep_scenarios(&edited).unwrap();
        assert_eq!(scenarios[0].sweeps[0].corner_count, 3);
    }

    #[test]
    fn remove_model_section_preserves_sweep_when_parameter_remains() {
        let edited = append_analog_sweep_with_parameter(
            project_yaml(),
            &AnalogSweepParameterDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "model_corner".to_string(),
                parameter_name: "RIN_VALUE".to_string(),
                values_csv: "950, 1050".to_string(),
            },
        )
        .unwrap();
        let edited = append_analog_sweep_model_section(
            &edited,
            &AnalogSweepModelSectionDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "model_corner".to_string(),
                path: "models/vendor.lib".to_string(),
                sections_csv: "typ, slow".to_string(),
            },
        )
        .unwrap();

        let edited = remove_analog_sweep_model_section(
            &edited,
            &AnalogSweepModelSectionDraft {
                scenario_name: "rc_run".to_string(),
                sweep_name: "model_corner".to_string(),
                path: "models/vendor.lib".to_string(),
                sections_csv: String::new(),
            },
        )
        .unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
        assert!(sweep.model_sections.is_empty());
        assert_eq!(sweep.parameters.len(), 1);
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
