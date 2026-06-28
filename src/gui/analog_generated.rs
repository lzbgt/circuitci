use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::analog_model_files::add_missing_generated_model_files;

#[derive(Debug, Clone)]
pub(super) struct AnalogGeneratedComponentDraft {
    pub(super) scenario_name: String,
    pub(super) component_id: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogGeneratedSettingsDraft {
    pub(super) scenario_name: String,
    pub(super) ground_net: String,
    pub(super) stop_time_us: f64,
    pub(super) max_step_us: f64,
    pub(super) start_frequency_hz: f64,
    pub(super) stop_frequency_hz: f64,
    pub(super) points_per_decade: u32,
    pub(super) noise_input_source: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogGeneratedNodeBindingDraft {
    pub(super) scenario_name: String,
    pub(super) net: String,
    pub(super) node: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogGeneratedScenario {
    pub(super) name: String,
    pub(super) scenario_type: String,
    pub(super) ground_net: String,
    pub(super) stop_time_us: f64,
    pub(super) max_step_us: f64,
    pub(super) start_frequency_hz: f64,
    pub(super) stop_frequency_hz: f64,
    pub(super) points_per_decade: u32,
    pub(super) noise_output_node: Option<String>,
    pub(super) noise_input_source: Option<String>,
    pub(super) components: Vec<String>,
    pub(super) board_nets: Vec<AnalogGeneratedNetChoice>,
    pub(super) node_bindings: Vec<AnalogGeneratedNodeBindingChoice>,
    pub(super) board_components: Vec<AnalogGeneratedComponentChoice>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogGeneratedNetChoice {
    pub(super) id: String,
    pub(super) kind: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogGeneratedNodeBindingChoice {
    pub(super) net: String,
    pub(super) node: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogGeneratedComponentChoice {
    pub(super) id: String,
    pub(super) model: String,
    pub(super) included: bool,
}

impl AnalogGeneratedComponentChoice {
    pub(super) fn label(&self) -> String {
        let marker = if self.included {
            "included"
        } else {
            "available"
        };
        format!("{} ({}, {})", self.id, self.model, marker)
    }
}

pub(super) fn analog_generated_scenarios(text: &str) -> Result<Vec<AnalogGeneratedScenario>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    Ok(project
        .scenarios
        .iter()
        .filter_map(|scenario| {
            let analog = scenario.analog.as_ref()?;
            if analog.netlist_source != crate::board_ir::AnalogNetlistSource::GeneratedFromBoard {
                return None;
            }
            let generated = analog.generated.as_ref()?;
            let included: BTreeSet<&str> =
                generated.components.iter().map(String::as_str).collect();
            Some(AnalogGeneratedScenario {
                name: scenario.name.clone(),
                scenario_type: scenario.scenario_type.clone(),
                ground_net: generated.ground_net.clone(),
                stop_time_us: analog.analysis.stop_time_us,
                max_step_us: analog.analysis.max_step_us,
                start_frequency_hz: analog.analysis.start_frequency_hz.unwrap_or(10.0),
                stop_frequency_hz: analog.analysis.stop_frequency_hz.unwrap_or(100_000.0),
                points_per_decade: analog.analysis.points_per_decade.unwrap_or(20),
                noise_output_node: analog.analysis.noise_output_node.clone(),
                noise_input_source: analog.analysis.noise_input_source.clone(),
                components: generated.components.clone(),
                board_nets: project
                    .board
                    .nets
                    .iter()
                    .map(|(net_id, net)| AnalogGeneratedNetChoice {
                        id: net_id.clone(),
                        kind: net_kind_label(&net.kind).to_string(),
                    })
                    .collect(),
                node_bindings: analog
                    .node_bindings
                    .iter()
                    .map(|binding| AnalogGeneratedNodeBindingChoice {
                        net: binding.net.clone(),
                        node: binding.node.clone(),
                    })
                    .collect(),
                board_components: project
                    .board
                    .components
                    .iter()
                    .map(|(component_id, component)| AnalogGeneratedComponentChoice {
                        id: component_id.clone(),
                        model: component.model.clone(),
                        included: included.contains(component_id.as_str()),
                    })
                    .collect(),
            })
        })
        .collect())
}

pub(super) fn replace_generated_settings(
    text: &str,
    draft: &AnalogGeneratedSettingsDraft,
) -> Result<String> {
    validate_generated_settings_draft(draft)?;
    let scenario_name = draft.scenario_name.trim();
    let ground_net = draft.ground_net.trim();
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = generated_scenario(&project, scenario_name)?;
    let analog = scenario
        .analog
        .as_ref()
        .expect("generated_scenario checked");
    let ground_spec = project
        .board
        .nets
        .get(ground_net)
        .with_context(|| format!("Ground net {ground_net} was not found."))?;
    if ground_spec.kind != crate::board_ir::NetKind::Ground {
        anyhow::bail!(
            "Ground net {} must have kind ground.",
            draft.ground_net.trim()
        );
    }
    let ground_net = draft.ground_net.trim();

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let generated_mapping = child_mapping_mut(analog_mapping, "generated", "analog.generated")?;
    insert_string(generated_mapping, "ground_net", ground_net);
    let analysis_mapping = child_mapping_mut(analog_mapping, "analysis", "analog analysis")?;
    if scenario.scenario_type == "analog_ac" {
        validate_ac_settings(draft)?;
        insert_string(analysis_mapping, "type", "ac");
        insert_number(
            analysis_mapping,
            "start_frequency_hz",
            draft.start_frequency_hz,
        )?;
        insert_number(
            analysis_mapping,
            "stop_frequency_hz",
            draft.stop_frequency_hz,
        )?;
        analysis_mapping.insert(
            key("points_per_decade"),
            serde_yaml_ng::to_value(draft.points_per_decade)
                .context("Failed to encode AC points_per_decade.")?,
        );
    } else if scenario.scenario_type == "analog_dc" {
        insert_string(analysis_mapping, "type", "op");
        analysis_mapping.remove(key("stop_time_us"));
        analysis_mapping.remove(key("max_step_us"));
        analysis_mapping.remove(key("start_frequency_hz"));
        analysis_mapping.remove(key("stop_frequency_hz"));
        analysis_mapping.remove(key("points_per_decade"));
        analysis_mapping.remove(key("noise_output_node"));
        analysis_mapping.remove(key("noise_reference_node"));
        analysis_mapping.remove(key("noise_input_source"));
    } else if scenario.scenario_type == "analog_noise" {
        validate_ac_settings(draft)?;
        let input_source = draft.noise_input_source.trim();
        if !noise_input_source_exists(&project, input_source) {
            anyhow::bail!(
                "Noise input source {input_source} must be an included voltage or current source component."
            );
        }
        let output_node = analog
            .analysis
            .noise_output_node
            .as_deref()
            .context("Generated noise settings require an existing noise_output_node.")?;
        insert_string(analysis_mapping, "type", "noise");
        insert_number(
            analysis_mapping,
            "start_frequency_hz",
            draft.start_frequency_hz,
        )?;
        insert_number(
            analysis_mapping,
            "stop_frequency_hz",
            draft.stop_frequency_hz,
        )?;
        analysis_mapping.insert(
            key("points_per_decade"),
            serde_yaml_ng::to_value(draft.points_per_decade)
                .context("Failed to encode noise points_per_decade.")?,
        );
        insert_string(analysis_mapping, "noise_output_node", output_node);
        insert_string(analysis_mapping, "noise_input_source", input_source);
        analysis_mapping.remove(key("stop_time_us"));
        analysis_mapping.remove(key("max_step_us"));
    } else {
        validate_transient_settings(draft)?;
        insert_string(analysis_mapping, "type", "tran");
        insert_number(analysis_mapping, "stop_time_us", draft.stop_time_us)?;
        insert_number(analysis_mapping, "max_step_us", draft.max_step_us)?;
    }

    let mut used_nodes: BTreeSet<String> = analog
        .node_bindings
        .iter()
        .map(|binding| binding.node.clone())
        .collect();
    used_nodes.remove("0");
    {
        let node_bindings =
            ensure_child_sequence_mut(analog_mapping, "node_bindings", "analog node bindings")?;
        retarget_ground_node_binding(node_bindings, ground_net, &mut used_nodes);
    }
    sync_pin_bindings_for_all_generated_components(&project, analog_mapping, scenario_name)?;

    serialize_validated(
        yaml,
        "Edited generated settings YAML is not valid Board IR.",
    )
}

fn noise_input_source_exists(project: &crate::board_ir::BoardProject, input_source: &str) -> bool {
    project
        .board
        .components
        .get(input_source)
        .and_then(|component| component.spice.as_ref())
        .is_some_and(|spice| {
            matches!(
                spice.primitive,
                crate::board_ir::SpicePrimitive::DcVoltageSource
                    | crate::board_ir::SpicePrimitive::PulseVoltageSource
                    | crate::board_ir::SpicePrimitive::DcCurrentSource
                    | crate::board_ir::SpicePrimitive::PulseCurrentSource
            )
        })
}

fn validate_ac_settings(draft: &AnalogGeneratedSettingsDraft) -> Result<()> {
    if !draft.start_frequency_hz.is_finite()
        || !draft.stop_frequency_hz.is_finite()
        || draft.start_frequency_hz <= 0.0
        || draft.stop_frequency_hz <= draft.start_frequency_hz
        || draft.points_per_decade == 0
        || draft.points_per_decade > 1000
    {
        anyhow::bail!(
            "Generated AC/Bode settings require positive start/stop frequencies, stop greater than start, and points per decade in 1..=1000."
        );
    }
    Ok(())
}

fn validate_transient_settings(draft: &AnalogGeneratedSettingsDraft) -> Result<()> {
    if !draft.stop_time_us.is_finite()
        || !draft.max_step_us.is_finite()
        || draft.stop_time_us <= 0.0
        || draft.max_step_us <= 0.0
        || draft.max_step_us > draft.stop_time_us
    {
        anyhow::bail!(
            "Stop time and max step must be finite positive values, with max step no larger than stop time."
        );
    }
    Ok(())
}

pub(super) fn replace_generated_node_binding(
    text: &str,
    draft: &AnalogGeneratedNodeBindingDraft,
) -> Result<String> {
    validate_generated_node_binding_draft(draft)?;
    let scenario_name = draft.scenario_name.trim();
    let net = draft.net.trim();
    let node = draft.node.trim();
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = generated_scenario(&project, scenario_name)?;
    let analog = scenario
        .analog
        .as_ref()
        .expect("generated_scenario checked");
    let generated = analog
        .generated
        .as_ref()
        .expect("generated_scenario checked");
    project
        .board
        .nets
        .get(net)
        .with_context(|| format!("Net {net} was not found."))?;
    if net == generated.ground_net && node != "0" {
        anyhow::bail!(
            "Ground net {} must remain bound to SPICE node 0.",
            generated.ground_net
        );
    }
    if net != generated.ground_net && node == "0" {
        anyhow::bail!(
            "Only ground net {} may use SPICE node 0.",
            generated.ground_net
        );
    }
    if analog
        .node_bindings
        .iter()
        .any(|binding| binding.net != net && binding.node == node)
    {
        anyhow::bail!("SPICE node {node} is already bound to another net.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    {
        let node_bindings =
            ensure_child_sequence_mut(analog_mapping, "node_bindings", "analog node bindings")?;
        upsert_node_binding(node_bindings, net, node);
    }
    sync_pin_bindings_for_net(&project, analog_mapping, scenario_name, net)?;

    serialize_validated(
        yaml,
        "Edited generated node binding YAML is not valid Board IR.",
    )
}

#[cfg(test)]
pub(super) fn include_generated_component(
    text: &str,
    draft: &AnalogGeneratedComponentDraft,
) -> Result<String> {
    include_generated_component_with_project_path(text, Path::new("project.yaml"), draft)
}

pub(super) fn include_generated_component_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogGeneratedComponentDraft,
) -> Result<String> {
    validate_generated_component_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = generated_scenario(&project, &draft.scenario_name)?;
    let analog = scenario
        .analog
        .as_ref()
        .expect("generated_scenario checked");
    let generated = analog
        .generated
        .as_ref()
        .expect("generated_scenario checked");
    if generated
        .components
        .iter()
        .any(|component_id| component_id == &draft.component_id)
    {
        anyhow::bail!(
            "Component {} is already included in generated scenario {}.",
            draft.component_id,
            draft.scenario_name
        );
    }
    let component = project
        .board
        .components
        .get(&draft.component_id)
        .with_context(|| format!("Component {} was not found.", draft.component_id))?;

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let generated_mapping = child_mapping_mut(analog_mapping, "generated", "analog.generated")?;
    let components =
        ensure_child_sequence_mut(generated_mapping, "components", "generated components")?;
    components.push(serde_yaml_ng::Value::String(draft.component_id.clone()));

    let mut node_by_net: BTreeMap<String, String> = analog
        .node_bindings
        .iter()
        .map(|binding| (binding.net.clone(), binding.node.clone()))
        .collect();
    let mut used_nodes: BTreeSet<String> = node_by_net.values().cloned().collect();
    {
        let node_bindings =
            ensure_child_sequence_mut(analog_mapping, "node_bindings", "analog node bindings")?;
        for net in component.pins.values() {
            if !node_by_net.contains_key(net) {
                let node = unique_node_name(net, &mut used_nodes);
                node_by_net.insert(net.clone(), node.clone());
                let mut binding = serde_yaml_ng::Mapping::new();
                insert_string(&mut binding, "node", &node);
                insert_string(&mut binding, "net", net);
                node_bindings.push(serde_yaml_ng::Value::Mapping(binding));
            }
        }
    }
    {
        let pin_bindings =
            ensure_child_sequence_mut(analog_mapping, "pin_bindings", "analog pin bindings")?;
        for (pin_id, net) in &component.pins {
            if pin_binding_exists(pin_bindings, &draft.component_id, pin_id) {
                continue;
            }
            let node = node_by_net.get(net).with_context(|| {
                format!(
                    "Component {}.{pin_id} references unknown net {net}.",
                    draft.component_id
                )
            })?;
            pin_bindings.push(pin_binding_value(&draft.component_id, pin_id, node));
        }
    }

    let updated = serialize_validated(
        yaml,
        "Edited generated component YAML is not valid Board IR.",
    )?;
    add_missing_generated_model_files(&updated, project_path, &draft.scenario_name)
}

pub(super) fn exclude_generated_component(
    text: &str,
    draft: &AnalogGeneratedComponentDraft,
) -> Result<String> {
    validate_generated_component_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = generated_scenario(&project, &draft.scenario_name)?;
    let analog = scenario
        .analog
        .as_ref()
        .expect("generated_scenario checked");
    let generated = analog
        .generated
        .as_ref()
        .expect("generated_scenario checked");
    if !generated
        .components
        .iter()
        .any(|component_id| component_id == &draft.component_id)
    {
        anyhow::bail!(
            "Component {} is not included in generated scenario {}.",
            draft.component_id,
            draft.scenario_name
        );
    }
    if generated.components.len() <= 1 {
        anyhow::bail!(
            "Generated scenario {} must keep at least one component.",
            draft.scenario_name
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let generated_mapping = child_mapping_mut(analog_mapping, "generated", "analog.generated")?;
    let components =
        ensure_child_sequence_mut(generated_mapping, "components", "generated components")?;
    components.retain(|value| value.as_str() != Some(draft.component_id.as_str()));
    let pin_bindings =
        ensure_child_sequence_mut(analog_mapping, "pin_bindings", "analog pin bindings")?;
    pin_bindings.retain(|value| !pin_binding_component_matches(value, &draft.component_id));

    serialize_validated(
        yaml,
        "Edited generated component YAML is not valid Board IR.",
    )
}

fn generated_scenario<'a>(
    project: &'a crate::board_ir::BoardProject,
    scenario_name: &str,
) -> Result<&'a crate::board_ir::Scenario> {
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Scenario {scenario_name} was not found."))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {scenario_name} is not an analog scenario."))?;
    if analog.netlist_source != crate::board_ir::AnalogNetlistSource::GeneratedFromBoard {
        anyhow::bail!(
            "Generated component editing requires a generated_from_board analog scenario; scenario {} uses a file-backed deck.",
            scenario.name
        );
    }
    if analog.generated.is_none() {
        anyhow::bail!("Scenario {} must declare analog.generated.", scenario.name);
    }
    Ok(scenario)
}

fn validate_generated_component_draft(draft: &AnalogGeneratedComponentDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.component_id, "component id")?;
    Ok(())
}

fn validate_generated_settings_draft(draft: &AnalogGeneratedSettingsDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.ground_net, "ground net")?;
    Ok(())
}

fn validate_generated_node_binding_draft(draft: &AnalogGeneratedNodeBindingDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.net, "net")?;
    validated_spice_node(&draft.node)?;
    Ok(())
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

fn validated_spice_node(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("SPICE node must not be blank.");
    }
    if value == "0" {
        return Ok(value);
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        anyhow::bail!("SPICE node {value} contains unsupported characters.");
    }
    if value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        anyhow::bail!("SPICE node {value} must not start with a digit unless it is ground node 0.");
    }
    Ok(value)
}

fn scenario_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    scenario_name: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let scenarios = yaml
        .as_mapping_mut()
        .and_then(|mapping| mapping.get_mut(key("scenarios")))
        .and_then(serde_yaml_ng::Value::as_sequence_mut)
        .context("Project YAML must declare scenarios as a list.")?;
    scenarios
        .iter_mut()
        .filter_map(serde_yaml_ng::Value::as_mapping_mut)
        .find(|mapping| {
            mapping
                .get(key("name"))
                .and_then(serde_yaml_ng::Value::as_str)
                == Some(scenario_name)
        })
        .with_context(|| format!("Scenario {scenario_name} was not found."))
}

fn child_mapping_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    field: &str,
    label: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    mapping
        .get_mut(key(field))
        .with_context(|| format!("{label} must declare {field}."))?
        .as_mapping_mut()
        .with_context(|| format!("{label} {field} must be a YAML object."))
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

fn pin_binding_exists(
    pin_bindings: &[serde_yaml_ng::Value],
    component_id: &str,
    pin_id: &str,
) -> bool {
    pin_bindings.iter().any(|value| {
        let endpoint = value
            .as_mapping()
            .and_then(|mapping| mapping.get(key("endpoint")))
            .and_then(serde_yaml_ng::Value::as_mapping);
        endpoint.is_some_and(|endpoint| {
            endpoint
                .get(key("component"))
                .and_then(serde_yaml_ng::Value::as_str)
                == Some(component_id)
                && endpoint
                    .get(key("pin"))
                    .and_then(serde_yaml_ng::Value::as_str)
                    == Some(pin_id)
        })
    })
}

fn pin_binding_component_matches(value: &serde_yaml_ng::Value, component_id: &str) -> bool {
    value
        .as_mapping()
        .and_then(|mapping| mapping.get(key("endpoint")))
        .and_then(serde_yaml_ng::Value::as_mapping)
        .and_then(|endpoint| endpoint.get(key("component")))
        .and_then(serde_yaml_ng::Value::as_str)
        == Some(component_id)
}

fn pin_binding_value(component_id: &str, pin_id: &str, node: &str) -> serde_yaml_ng::Value {
    let mut endpoint = serde_yaml_ng::Mapping::new();
    insert_string(&mut endpoint, "component", component_id);
    insert_string(&mut endpoint, "pin", pin_id);

    let mut binding = serde_yaml_ng::Mapping::new();
    insert_string(&mut binding, "node", node);
    binding.insert(key("endpoint"), serde_yaml_ng::Value::Mapping(endpoint));
    serde_yaml_ng::Value::Mapping(binding)
}

fn retarget_ground_node_binding(
    node_bindings: &mut Vec<serde_yaml_ng::Value>,
    ground_net: &str,
    used_nodes: &mut BTreeSet<String>,
) {
    let mut ground_binding_found = false;
    for binding in node_bindings.iter_mut() {
        let Some(mapping) = binding.as_mapping_mut() else {
            continue;
        };
        let net = mapping
            .get(key("net"))
            .and_then(serde_yaml_ng::Value::as_str)
            .map(str::to_string);
        let node = mapping
            .get(key("node"))
            .and_then(serde_yaml_ng::Value::as_str)
            .map(str::to_string);
        match (net.as_deref(), node.as_deref()) {
            (Some(net), _) if net == ground_net => {
                mapping.insert(key("node"), serde_yaml_ng::Value::String("0".to_string()));
                ground_binding_found = true;
            }
            (Some(net), Some("0")) => {
                let node = unique_node_name(net, used_nodes);
                mapping.insert(key("node"), serde_yaml_ng::Value::String(node));
            }
            _ => {}
        }
    }
    if !ground_binding_found {
        let mut binding = serde_yaml_ng::Mapping::new();
        insert_string(&mut binding, "node", "0");
        insert_string(&mut binding, "net", ground_net);
        node_bindings.push(serde_yaml_ng::Value::Mapping(binding));
    }
}

fn upsert_node_binding(node_bindings: &mut Vec<serde_yaml_ng::Value>, net: &str, node: &str) {
    for binding in node_bindings.iter_mut() {
        let Some(mapping) = binding.as_mapping_mut() else {
            continue;
        };
        if mapping
            .get(key("net"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(net)
        {
            mapping.insert(key("node"), serde_yaml_ng::Value::String(node.to_string()));
            return;
        }
    }
    let mut binding = serde_yaml_ng::Mapping::new();
    insert_string(&mut binding, "node", node);
    insert_string(&mut binding, "net", net);
    node_bindings.push(serde_yaml_ng::Value::Mapping(binding));
}

fn sync_pin_bindings_for_all_generated_components(
    project: &crate::board_ir::BoardProject,
    analog_mapping: &mut serde_yaml_ng::Mapping,
    scenario_name: &str,
) -> Result<()> {
    let scenario = generated_scenario(project, scenario_name)?;
    let analog = scenario
        .analog
        .as_ref()
        .expect("generated_scenario checked");
    let generated = analog
        .generated
        .as_ref()
        .expect("generated_scenario checked");
    let nets: BTreeSet<String> = generated
        .components
        .iter()
        .filter_map(|component_id| project.board.components.get(component_id))
        .flat_map(|component| component.pins.values().cloned())
        .collect();
    for net in nets {
        sync_pin_bindings_for_net(project, analog_mapping, scenario_name, &net)?;
    }
    Ok(())
}

fn sync_pin_bindings_for_net(
    project: &crate::board_ir::BoardProject,
    analog_mapping: &mut serde_yaml_ng::Mapping,
    scenario_name: &str,
    net: &str,
) -> Result<()> {
    let scenario = generated_scenario(project, scenario_name)?;
    let analog = scenario
        .analog
        .as_ref()
        .expect("generated_scenario checked");
    let generated = analog
        .generated
        .as_ref()
        .expect("generated_scenario checked");
    let node = node_binding_for_net(analog_mapping, net)
        .with_context(|| format!("Net {net} does not have an analog node binding."))?;
    let endpoints: BTreeSet<(String, String)> = generated
        .components
        .iter()
        .filter_map(|component_id| {
            project
                .board
                .components
                .get(component_id)
                .map(|component| (component_id, component))
        })
        .flat_map(|(component_id, component)| {
            component
                .pins
                .iter()
                .filter(move |(_, pin_net)| *pin_net == net)
                .map(move |(pin_id, _)| (component_id.clone(), pin_id.clone()))
        })
        .collect();
    let pin_bindings =
        ensure_child_sequence_mut(analog_mapping, "pin_bindings", "analog pin bindings")?;
    for (component_id, pin_id) in endpoints {
        upsert_pin_binding(pin_bindings, &component_id, &pin_id, &node);
    }
    Ok(())
}

fn node_binding_for_net(analog_mapping: &serde_yaml_ng::Mapping, net: &str) -> Option<String> {
    analog_mapping
        .get(key("node_bindings"))?
        .as_sequence()?
        .iter()
        .find_map(|binding| {
            let mapping = binding.as_mapping()?;
            (mapping
                .get(key("net"))
                .and_then(serde_yaml_ng::Value::as_str)
                == Some(net))
            .then(|| {
                mapping
                    .get(key("node"))
                    .and_then(serde_yaml_ng::Value::as_str)
                    .map(str::to_string)
            })?
        })
}

fn upsert_pin_binding(
    pin_bindings: &mut Vec<serde_yaml_ng::Value>,
    component_id: &str,
    pin_id: &str,
    node: &str,
) {
    for binding in pin_bindings.iter_mut() {
        let Some(mapping) = binding.as_mapping_mut() else {
            continue;
        };
        let endpoint = mapping
            .get(key("endpoint"))
            .and_then(serde_yaml_ng::Value::as_mapping);
        if endpoint.is_some_and(|endpoint| {
            endpoint
                .get(key("component"))
                .and_then(serde_yaml_ng::Value::as_str)
                == Some(component_id)
                && endpoint
                    .get(key("pin"))
                    .and_then(serde_yaml_ng::Value::as_str)
                    == Some(pin_id)
        }) {
            mapping.insert(key("node"), serde_yaml_ng::Value::String(node.to_string()));
            return;
        }
    }
    pin_bindings.push(pin_binding_value(component_id, pin_id, node));
}

fn unique_node_name(net: &str, used_nodes: &mut BTreeSet<String>) -> String {
    let base = sanitize_node_name(net);
    if used_nodes.insert(base.clone()) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}_{suffix}");
        if used_nodes.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search must find a node name")
}

fn sanitize_node_name(net: &str) -> String {
    let mut node = String::new();
    for character in net.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            node.push(character);
        } else if !node.ends_with('_') {
            node.push('_');
        }
    }
    let node = node.trim_matches('_');
    if node.is_empty() {
        "n".to_string()
    } else if node
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        format!("n_{node}")
    } else {
        node.to_string()
    }
}

fn serialize_validated(yaml: serde_yaml_ng::Value, context: &str) -> Result<String> {
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&updated).with_context(|| context.to_string())?;
    Ok(updated)
}

fn insert_number(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: f64) -> Result<()> {
    mapping.insert(
        key(name),
        serde_yaml_ng::to_value(value).context("Failed to encode generated scenario number.")?,
    );
    Ok(())
}

fn insert_string(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: &str) {
    mapping.insert(key(name), serde_yaml_ng::Value::String(value.to_string()));
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

fn net_kind_label(kind: &crate::board_ir::NetKind) -> &'static str {
    match kind {
        crate::board_ir::NetKind::Power => "power",
        crate::board_ir::NetKind::Ground => "ground",
        crate::board_ir::NetKind::DigitalOrAnalog => "digital_or_analog",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AnalogGeneratedComponentDraft, AnalogGeneratedNodeBindingDraft,
        AnalogGeneratedSettingsDraft, analog_generated_scenarios, exclude_generated_component,
        include_generated_component, include_generated_component_with_project_path,
        replace_generated_node_binding, replace_generated_settings,
    };
    use std::path::Path;

    fn project_yaml() -> &'static str {
        "project:
  name: gui_generated_membership_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      spice: {primitive: dc_voltage_source, dc_v: 5.0}
      pins: {P: rail_5v, N: gnd}
    R1:
      model: generic.analog.resistor
      spice: {primitive: resistor, value_ohm: 1000.0}
      pins: {A: rail_5v, B: out}
    C1:
      model: generic.analog.capacitor
      spice: {primitive: capacitor, value_f: 0.000001}
      pins: {A: out, B: gnd}
  nets:
    rail_5v: {kind: power}
    out: {kind: digital_or_analog}
    gnd: {kind: ground}
scenarios:
  - name: generated_transient
    type: analog_transient
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        components: [V1, R1]
        ground_net: gnd
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: rail_5v, net: rail_5v}
        - {node: out, net: out}
      pin_bindings:
        - {node: rail_5v, endpoint: {component: V1, pin: P}}
        - {node: '0', endpoint: {component: V1, pin: N}}
        - {node: rail_5v, endpoint: {component: R1, pin: A}}
        - {node: out, endpoint: {component: R1, pin: B}}
      analysis: {type: tran, stop_time_us: 100.0, max_step_us: 1.0}
      stimuli: []
      probes: []
      assertions: []
"
    }

    fn model_pack_project_yaml() -> &'static str {
        r#"
project:
  name: gui_generated_model_file_test
  version: 0.1.0
libraries:
  - ../../libs/generic/analog
board:
  components:
    VCC:
      model: generic.analog.dc_voltage_source
      pins: {P: vcc_5v, N: gnd}
      spice: {primitive: dc_voltage_source, dc_v: 5.0}
    VIN:
      model: generic.analog.dc_voltage_source
      pins: {P: input, N: gnd}
      spice: {primitive: dc_voltage_source, dc_v: 1.0}
    XU1:
      model: generic.analog.ideal_opamp
      pins: {INP: input, INN: output, VCC: vcc_5v, VEE: gnd, OUT: output}
    RLOAD:
      model: generic.analog.resistor
      pins: {A: output, B: gnd}
      spice: {primitive: resistor, value_ohm: 10000.0}
  nets:
    vcc_5v: {kind: power, nominal_voltage: 5.0, powered: true}
    input: {kind: digital_or_analog}
    output: {kind: digital_or_analog}
    gnd: {kind: ground}
scenarios:
  - name: generated_transient
    type: analog_transient
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        components: [VCC, VIN, RLOAD]
        ground_net: gnd
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: vcc, net: vcc_5v}
        - {node: in, net: input}
        - {node: out, net: output}
      pin_bindings:
        - {node: vcc, endpoint: {component: VCC, pin: P}}
        - {node: '0', endpoint: {component: VCC, pin: N}}
        - {node: in, endpoint: {component: VIN, pin: P}}
        - {node: '0', endpoint: {component: VIN, pin: N}}
        - {node: out, endpoint: {component: RLOAD, pin: A}}
        - {node: '0', endpoint: {component: RLOAD, pin: B}}
      analysis: {type: tran, stop_time_us: 10.0, max_step_us: 0.1}
      stimuli: []
      probes: []
      assertions: []
"#
    }

    #[test]
    fn analog_generated_scenarios_lists_included_and_available_components() {
        let scenarios = analog_generated_scenarios(project_yaml()).unwrap();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].components, vec!["V1", "R1"]);
        assert_eq!(scenarios[0].ground_net, "gnd");
        assert_eq!(scenarios[0].stop_time_us, 100.0);
        assert_eq!(scenarios[0].max_step_us, 1.0);
        assert!(
            scenarios[0]
                .node_bindings
                .iter()
                .any(|binding| binding.net == "gnd" && binding.node == "0")
        );
        let c1 = scenarios[0]
            .board_components
            .iter()
            .find(|component| component.id == "C1")
            .unwrap();
        assert!(!c1.included);
    }

    #[test]
    fn generated_settings_supports_ac_bode_analysis() {
        let yaml = project_yaml()
            .replace("generated_transient", "generated_bode")
            .replace("type: analog_transient", "type: analog_ac")
            .replace(
                "analysis: {type: tran, stop_time_us: 100.0, max_step_us: 1.0}",
                "analysis: {type: ac, start_frequency_hz: 10.0, stop_frequency_hz: 100000.0, points_per_decade: 20}",
            );
        let scenarios = analog_generated_scenarios(&yaml).unwrap();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].scenario_type, "analog_ac");
        assert_eq!(scenarios[0].start_frequency_hz, 10.0);
        assert_eq!(scenarios[0].stop_frequency_hz, 100_000.0);
        assert_eq!(scenarios[0].points_per_decade, 20);

        let edited = replace_generated_settings(
            &yaml,
            &AnalogGeneratedSettingsDraft {
                scenario_name: "generated_bode".to_string(),
                ground_net: "gnd".to_string(),
                stop_time_us: -1.0,
                max_step_us: -1.0,
                start_frequency_hz: 100.0,
                stop_frequency_hz: 1_000_000.0,
                points_per_decade: 40,
                noise_input_source: "V1".to_string(),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        assert_eq!(analog.analysis.analysis_type, "ac");
        assert_eq!(analog.analysis.start_frequency_hz, Some(100.0));
        assert_eq!(analog.analysis.stop_frequency_hz, Some(1_000_000.0));
        assert_eq!(analog.analysis.points_per_decade, Some(40));
    }

    #[test]
    fn include_generated_component_adds_component_and_pin_bindings() {
        let edited = include_generated_component(
            project_yaml(),
            &AnalogGeneratedComponentDraft {
                scenario_name: "generated_transient".to_string(),
                component_id: "C1".to_string(),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        assert!(
            analog
                .generated
                .as_ref()
                .unwrap()
                .components
                .contains(&"C1".to_string())
        );
        assert!(
            analog
                .pin_bindings
                .iter()
                .any(|binding| binding.endpoint.component == "C1" && binding.endpoint.pin == "A")
        );
    }

    #[test]
    fn include_generated_component_adds_required_model_file() {
        let edited = include_generated_component_with_project_path(
            model_pack_project_yaml(),
            Path::new("examples/good_ideal_opamp_buffer/project.yaml"),
            &AnalogGeneratedComponentDraft {
                scenario_name: "generated_transient".to_string(),
                component_id: "XU1".to_string(),
            },
        )
        .unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        assert!(
            analog
                .generated
                .as_ref()
                .unwrap()
                .components
                .contains(&"XU1".to_string())
        );
        assert_eq!(analog.model_files.len(), 1);
        assert_eq!(
            analog.model_files[0].path,
            "../../models/spice/generic/analog_behavioral.lib"
        );
        assert_eq!(
            analog.model_files[0].sha256.as_deref(),
            Some("4fdbeee10a9ca9eec41c4694f90e01fbde191978c109b45c52a6180992b0aee2")
        );
    }

    #[test]
    fn exclude_generated_component_removes_component_and_pin_bindings() {
        let edited = exclude_generated_component(
            project_yaml(),
            &AnalogGeneratedComponentDraft {
                scenario_name: "generated_transient".to_string(),
                component_id: "R1".to_string(),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        assert!(
            !analog
                .generated
                .as_ref()
                .unwrap()
                .components
                .contains(&"R1".to_string())
        );
        assert!(
            !analog
                .pin_bindings
                .iter()
                .any(|binding| binding.endpoint.component == "R1")
        );
    }

    #[test]
    fn exclude_generated_component_rejects_last_component() {
        let edited = exclude_generated_component(
            project_yaml(),
            &AnalogGeneratedComponentDraft {
                scenario_name: "generated_transient".to_string(),
                component_id: "R1".to_string(),
            },
        )
        .unwrap();
        let error = exclude_generated_component(
            &edited,
            &AnalogGeneratedComponentDraft {
                scenario_name: "generated_transient".to_string(),
                component_id: "V1".to_string(),
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("at least one component"));
    }

    #[test]
    fn replace_generated_settings_updates_timing_and_retargets_ground_node() {
        let edited = replace_generated_settings(
            project_yaml(),
            &AnalogGeneratedSettingsDraft {
                scenario_name: "generated_transient".to_string(),
                ground_net: "out".to_string(),
                stop_time_us: 250.0,
                max_step_us: 2.5,
                start_frequency_hz: 10.0,
                stop_frequency_hz: 100_000.0,
                points_per_decade: 20,
                noise_input_source: "V1".to_string(),
            },
        )
        .unwrap_err();
        assert!(edited.to_string().contains("must have kind ground"));

        let yaml = project_yaml().replace("out: {kind: digital_or_analog}", "out: {kind: ground}");
        let edited = replace_generated_settings(
            &yaml,
            &AnalogGeneratedSettingsDraft {
                scenario_name: "generated_transient".to_string(),
                ground_net: "out".to_string(),
                stop_time_us: 250.0,
                max_step_us: 2.5,
                start_frequency_hz: 10.0,
                stop_frequency_hz: 100_000.0,
                points_per_decade: 20,
                noise_input_source: "V1".to_string(),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        assert_eq!(analog.generated.as_ref().unwrap().ground_net, "out");
        assert_eq!(analog.analysis.stop_time_us, 250.0);
        assert_eq!(analog.analysis.max_step_us, 2.5);
        assert!(
            analog
                .node_bindings
                .iter()
                .any(|binding| binding.net == "out" && binding.node == "0")
        );
        assert!(
            analog
                .node_bindings
                .iter()
                .any(|binding| binding.net == "gnd" && binding.node != "0")
        );
    }

    #[test]
    fn replace_generated_settings_preserves_dc_operating_point_analysis() {
        let yaml = project_yaml()
            .replace("type: analog_transient", "type: analog_dc")
            .replace(
                "analysis: {type: tran, stop_time_us: 100.0, max_step_us: 1.0}",
                "analysis: {type: op}",
            );
        let edited = replace_generated_settings(
            &yaml,
            &AnalogGeneratedSettingsDraft {
                scenario_name: "generated_transient".to_string(),
                ground_net: "gnd".to_string(),
                stop_time_us: 250.0,
                max_step_us: 2.5,
                start_frequency_hz: 100.0,
                stop_frequency_hz: 100_000.0,
                points_per_decade: 40,
                noise_input_source: "V1".to_string(),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let scenario = &project.scenarios[0];
        assert_eq!(scenario.scenario_type, "analog_dc");
        let analog = scenario.analog.as_ref().unwrap();
        assert_eq!(analog.analysis.analysis_type, "op");
        assert_eq!(analog.analysis.stop_time_us, 0.0);
        assert_eq!(analog.analysis.max_step_us, 0.0);
        assert_eq!(analog.analysis.start_frequency_hz, None);
        assert_eq!(analog.analysis.stop_frequency_hz, None);
        assert_eq!(analog.analysis.points_per_decade, None);
    }

    #[test]
    fn replace_generated_settings_preserves_noise_analysis() {
        let yaml = project_yaml()
            .replace("type: analog_transient", "type: analog_noise")
            .replace(
                "analysis: {type: tran, stop_time_us: 100.0, max_step_us: 1.0}",
                "analysis: {type: noise, start_frequency_hz: 10.0, stop_frequency_hz: 100000.0, points_per_decade: 20, noise_output_node: out, noise_input_source: V1}",
            );
        let edited = replace_generated_settings(
            &yaml,
            &AnalogGeneratedSettingsDraft {
                scenario_name: "generated_transient".to_string(),
                ground_net: "gnd".to_string(),
                stop_time_us: 250.0,
                max_step_us: 2.5,
                start_frequency_hz: 100.0,
                stop_frequency_hz: 1_000_000.0,
                points_per_decade: 40,
                noise_input_source: "V1".to_string(),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let scenario = &project.scenarios[0];
        assert_eq!(scenario.scenario_type, "analog_noise");
        let analog = scenario.analog.as_ref().unwrap();
        assert_eq!(analog.analysis.analysis_type, "noise");
        assert_eq!(analog.analysis.start_frequency_hz, Some(100.0));
        assert_eq!(analog.analysis.stop_frequency_hz, Some(1_000_000.0));
        assert_eq!(analog.analysis.points_per_decade, Some(40));
        assert_eq!(analog.analysis.noise_output_node.as_deref(), Some("out"));
        assert_eq!(analog.analysis.noise_input_source.as_deref(), Some("V1"));
        assert_eq!(analog.analysis.stop_time_us, 0.0);
        assert_eq!(analog.analysis.max_step_us, 0.0);
    }

    #[test]
    fn replace_generated_settings_rejects_invalid_timing() {
        let error = replace_generated_settings(
            project_yaml(),
            &AnalogGeneratedSettingsDraft {
                scenario_name: "generated_transient".to_string(),
                ground_net: "gnd".to_string(),
                stop_time_us: 1.0,
                max_step_us: 2.0,
                start_frequency_hz: 10.0,
                stop_frequency_hz: 100_000.0,
                points_per_decade: 20,
                noise_input_source: "V1".to_string(),
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("max step"));
    }

    #[test]
    fn replace_generated_node_binding_updates_generated_pin_bindings() {
        let edited = replace_generated_node_binding(
            project_yaml(),
            &AnalogGeneratedNodeBindingDraft {
                scenario_name: "generated_transient".to_string(),
                net: "rail_5v".to_string(),
                node: "vcc".to_string(),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        assert!(
            analog
                .node_bindings
                .iter()
                .any(|binding| binding.net == "rail_5v" && binding.node == "vcc")
        );
        assert!(
            analog
                .pin_bindings
                .iter()
                .filter(|binding| binding.endpoint.component == "V1"
                    || binding.endpoint.component == "R1")
                .filter(|binding| binding.endpoint.pin == "P" || binding.endpoint.pin == "A")
                .all(|binding| binding.node == "vcc")
        );
    }

    #[test]
    fn replace_generated_node_binding_rejects_duplicate_and_bad_ground_nodes() {
        let duplicate = replace_generated_node_binding(
            project_yaml(),
            &AnalogGeneratedNodeBindingDraft {
                scenario_name: "generated_transient".to_string(),
                net: "out".to_string(),
                node: "rail_5v".to_string(),
            },
        )
        .unwrap_err();
        assert!(duplicate.to_string().contains("already bound"));

        let ground = replace_generated_node_binding(
            project_yaml(),
            &AnalogGeneratedNodeBindingDraft {
                scenario_name: "generated_transient".to_string(),
                net: "gnd".to_string(),
                node: "gnd_node".to_string(),
            },
        )
        .unwrap_err();
        assert!(ground.to_string().contains("node 0"));
    }
}
