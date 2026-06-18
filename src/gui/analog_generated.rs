use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub(super) struct AnalogGeneratedComponentDraft {
    pub(super) scenario_name: String,
    pub(super) component_id: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogGeneratedScenario {
    pub(super) name: String,
    pub(super) components: Vec<String>,
    pub(super) board_components: Vec<AnalogGeneratedComponentChoice>,
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
                components: generated.components.clone(),
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

pub(super) fn include_generated_component(
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

    serialize_validated(
        yaml,
        "Edited generated component YAML is not valid Board IR.",
    )
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

fn insert_string(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: &str) {
    mapping.insert(key(name), serde_yaml_ng::Value::String(value.to_string()));
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        AnalogGeneratedComponentDraft, analog_generated_scenarios, exclude_generated_component,
        include_generated_component,
    };

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

    #[test]
    fn analog_generated_scenarios_lists_included_and_available_components() {
        let scenarios = analog_generated_scenarios(project_yaml()).unwrap();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].components, vec!["V1", "R1"]);
        let c1 = scenarios[0]
            .board_components
            .iter()
            .find(|component| component.id == "C1")
            .unwrap();
        assert!(!c1.included);
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
}
