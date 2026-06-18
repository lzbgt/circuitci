use super::sketch::{
    SketchSelection, board_child_mapping_mut, encode_edited_project_yaml, schematic_node_key,
    validated_graph_id,
};
use anyhow::{Context, Result};

pub(super) fn rename_component(text: &str, old_id: &str, new_id: &str) -> Result<String> {
    let old_id = validated_graph_id(old_id, "component")?;
    let new_id = validated_graph_id(new_id, "component")?;
    if old_id == new_id {
        anyhow::bail!("New component ID must differ from the current component ID.");
    }

    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.components.contains_key(old_id) {
        anyhow::bail!("Board IR component {old_id} was not found.");
    }
    if project.board.components.contains_key(new_id) {
        anyhow::bail!("Board IR component {new_id} already exists.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    rename_board_mapping_key(&mut yaml, "components", old_id, new_id, "component")?;
    rename_schematic_node_metadata(&mut yaml, "component", old_id, new_id);
    rename_analog_component_references(&mut yaml, old_id, new_id);

    encode_edited_project_yaml(yaml)
}

pub(super) fn rename_net(text: &str, old_id: &str, new_id: &str) -> Result<String> {
    let old_id = validated_graph_id(old_id, "net")?;
    let new_id = validated_graph_id(new_id, "net")?;
    if old_id == new_id {
        anyhow::bail!("New net ID must differ from the current net ID.");
    }

    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.nets.contains_key(old_id) {
        anyhow::bail!("Board IR net {old_id} was not found.");
    }
    if project.board.nets.contains_key(new_id) {
        anyhow::bail!("Board IR net {new_id} already exists.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    rename_board_mapping_key(&mut yaml, "nets", old_id, new_id, "net")?;
    rename_component_pin_nets(&mut yaml, old_id, new_id)?;
    rename_schematic_node_metadata(&mut yaml, "net", old_id, new_id);
    rename_analog_net_references(&mut yaml, old_id, new_id);

    encode_edited_project_yaml(yaml)
}

fn rename_board_mapping_key(
    yaml: &mut serde_yaml_ng::Value,
    child: &str,
    old_id: &str,
    new_id: &str,
    label: &str,
) -> Result<()> {
    let mapping = board_child_mapping_mut(yaml, child)?;
    let old_key = serde_yaml_ng::Value::String(old_id.to_string());
    let new_key = serde_yaml_ng::Value::String(new_id.to_string());
    let value = mapping
        .remove(&old_key)
        .with_context(|| format!("Board IR {label} {old_id} was not found."))?;
    mapping.insert(new_key, value);
    Ok(())
}

fn rename_component_pin_nets(
    yaml: &mut serde_yaml_ng::Value,
    old_net: &str,
    new_net: &str,
) -> Result<()> {
    let components = board_child_mapping_mut(yaml, "components")?;
    for component in components.values_mut() {
        let Some(pins) = component
            .as_mapping_mut()
            .and_then(|component| component.get_mut(key("pins")))
            .and_then(serde_yaml_ng::Value::as_mapping_mut)
        else {
            continue;
        };
        for net in pins.values_mut() {
            if net.as_str() == Some(old_net) {
                *net = serde_yaml_ng::Value::String(new_net.to_string());
            }
        }
    }
    Ok(())
}

fn rename_schematic_node_metadata(
    yaml: &mut serde_yaml_ng::Value,
    kind: &str,
    old_id: &str,
    new_id: &str,
) {
    let Some(schematic) = yaml
        .as_mapping_mut()
        .and_then(|project| project.get_mut(key("board")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|board| board.get_mut(key("schematic")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
    else {
        return;
    };
    let selection = match kind {
        "component" => SketchSelection::Component(old_id.to_string()),
        "net" => SketchSelection::Net(old_id.to_string()),
        _ => return,
    };
    let replacement = match kind {
        "component" => SketchSelection::Component(new_id.to_string()),
        "net" => SketchSelection::Net(new_id.to_string()),
        _ => return,
    };
    let Some(old_key) = schematic_node_key(&selection) else {
        return;
    };
    let Some(new_key) = schematic_node_key(&replacement) else {
        return;
    };
    for child in ["node_positions", "node_styles"] {
        let Some(mapping) = schematic
            .get_mut(key(child))
            .and_then(serde_yaml_ng::Value::as_mapping_mut)
        else {
            continue;
        };
        if let Some(value) = mapping.remove(serde_yaml_ng::Value::String(old_key.clone())) {
            mapping.insert(serde_yaml_ng::Value::String(new_key.clone()), value);
        }
    }
}

fn rename_analog_component_references(
    yaml: &mut serde_yaml_ng::Value,
    old_component: &str,
    new_component: &str,
) {
    for analog in analog_mappings_mut(yaml) {
        if let Some(generated) = analog
            .get_mut(key("generated"))
            .and_then(serde_yaml_ng::Value::as_mapping_mut)
            && let Some(components) = generated
                .get_mut(key("components"))
                .and_then(serde_yaml_ng::Value::as_sequence_mut)
        {
            replace_sequence_string(components, old_component, new_component);
        }
        if let Some(pin_bindings) = analog
            .get_mut(key("pin_bindings"))
            .and_then(serde_yaml_ng::Value::as_sequence_mut)
        {
            for binding in pin_bindings {
                if let Some(endpoint) = binding
                    .as_mapping_mut()
                    .and_then(|binding| binding.get_mut(key("endpoint")))
                    .and_then(serde_yaml_ng::Value::as_mapping_mut)
                    && endpoint
                        .get(key("component"))
                        .and_then(serde_yaml_ng::Value::as_str)
                        == Some(old_component)
                {
                    endpoint.insert(
                        key("component"),
                        serde_yaml_ng::Value::String(new_component.to_string()),
                    );
                }
            }
        }
        if let Some(probes) = analog
            .get_mut(key("probes"))
            .and_then(serde_yaml_ng::Value::as_sequence_mut)
        {
            for probe in probes {
                if let Some(probe) = probe.as_mapping_mut()
                    && let Some(expression) = probe
                        .get(key("expression"))
                        .and_then(serde_yaml_ng::Value::as_str)
                {
                    let renamed = rename_component_branch_expression(
                        expression,
                        old_component,
                        new_component,
                    );
                    if renamed != expression {
                        probe.insert(key("expression"), serde_yaml_ng::Value::String(renamed));
                    }
                }
            }
        }
    }
}

fn rename_analog_net_references(yaml: &mut serde_yaml_ng::Value, old_net: &str, new_net: &str) {
    for analog in analog_mappings_mut(yaml) {
        if let Some(generated) = analog
            .get_mut(key("generated"))
            .and_then(serde_yaml_ng::Value::as_mapping_mut)
            && generated
                .get(key("ground_net"))
                .and_then(serde_yaml_ng::Value::as_str)
                == Some(old_net)
        {
            generated.insert(
                key("ground_net"),
                serde_yaml_ng::Value::String(new_net.to_string()),
            );
        }
        if let Some(node_bindings) = analog
            .get_mut(key("node_bindings"))
            .and_then(serde_yaml_ng::Value::as_sequence_mut)
        {
            for binding in node_bindings {
                if let Some(binding) = binding.as_mapping_mut()
                    && binding
                        .get(key("net"))
                        .and_then(serde_yaml_ng::Value::as_str)
                        == Some(old_net)
                {
                    binding.insert(
                        key("net"),
                        serde_yaml_ng::Value::String(new_net.to_string()),
                    );
                }
            }
        }
    }
}

fn analog_mappings_mut(yaml: &mut serde_yaml_ng::Value) -> Vec<&mut serde_yaml_ng::Mapping> {
    let Some(scenarios) = yaml
        .as_mapping_mut()
        .and_then(|project| project.get_mut(key("scenarios")))
        .and_then(serde_yaml_ng::Value::as_sequence_mut)
    else {
        return Vec::new();
    };
    scenarios
        .iter_mut()
        .filter_map(|scenario| {
            scenario
                .as_mapping_mut()
                .and_then(|scenario| scenario.get_mut(key("analog")))
                .and_then(serde_yaml_ng::Value::as_mapping_mut)
        })
        .collect()
}

fn replace_sequence_string(sequence: &mut [serde_yaml_ng::Value], old: &str, new: &str) {
    for value in sequence {
        if value.as_str() == Some(old) {
            *value = serde_yaml_ng::Value::String(new.to_string());
        }
    }
}

fn rename_component_branch_expression(
    expression: &str,
    old_component: &str,
    new_component: &str,
) -> String {
    let mut renamed = expression.to_string();
    for prefix in ["V", "I"] {
        renamed = rename_current_branch_call(
            &renamed,
            &generated_element_name(prefix, old_component),
            &generated_element_name(prefix, new_component),
        );
    }
    for prefix in ["R", "C", "L", "D", "Q", "M"] {
        renamed = rename_current_branch_call(
            &renamed,
            &generated_current_sense_name(prefix, old_component),
            &generated_current_sense_name(prefix, new_component),
        );
    }
    renamed
}

fn rename_current_branch_call(expression: &str, old_branch: &str, new_branch: &str) -> String {
    expression.replace(&format!("I({old_branch})"), &format!("I({new_branch})"))
}

fn generated_current_sense_name(prefix: &str, component_id: &str) -> String {
    format!("VCCI_{}", generated_element_name(prefix, component_id))
}

fn generated_element_name(prefix: &str, component_id: &str) -> String {
    let suffix = generated_element_suffix(component_id);
    if suffix.starts_with(prefix) {
        suffix
    } else {
        format!("{prefix}{suffix}")
    }
}

fn generated_element_suffix(component_id: &str) -> String {
    let mut suffix = String::new();
    for character in component_id.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            suffix.push(character);
        } else {
            suffix.push('_');
        }
    }
    if suffix.is_empty() {
        suffix.push('X');
    }
    suffix
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}
