use super::sketch::{
    SketchSelection, board_child_mapping_mut, encode_edited_project_yaml, schematic_node_key,
    validated_graph_id,
};
use anyhow::{Context, Result};
use eframe::egui;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn duplicate_components_with_local_nets(
    text: &str,
    component_ids: &[String],
    position_offset: egui::Vec2,
) -> Result<(String, Vec<SketchSelection>)> {
    let selected_components = component_ids
        .iter()
        .map(|id| validated_graph_id(id, "component").map(str::to_string))
        .collect::<Result<BTreeSet<_>>>()?;
    if selected_components.is_empty() {
        anyhow::bail!("Select at least one component to duplicate.");
    }
    if !position_offset.x.is_finite() || !position_offset.y.is_finite() {
        anyhow::bail!("Duplicate position offset must be finite.");
    }

    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    for component_id in &selected_components {
        if !project.board.components.contains_key(component_id) {
            anyhow::bail!("Board IR component {component_id} was not found.");
        }
    }

    let mut existing_components = project
        .board
        .components
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut component_id_map = BTreeMap::new();
    for component_id in &selected_components {
        let duplicate_id = unique_duplicate_id(component_id, &mut existing_components);
        component_id_map.insert(component_id.clone(), duplicate_id);
    }

    let mut referenced_by_net = BTreeMap::<String, BTreeSet<String>>::new();
    for (component_id, component) in &project.board.components {
        for net_id in component.pins.values() {
            referenced_by_net
                .entry(net_id.clone())
                .or_default()
                .insert(component_id.clone());
        }
    }
    let used_selected_nets = selected_components
        .iter()
        .filter_map(|component_id| project.board.components.get(component_id))
        .flat_map(|component| component.pins.values().cloned())
        .collect::<BTreeSet<_>>();
    let local_nets = used_selected_nets
        .into_iter()
        .filter(|net_id| {
            referenced_by_net
                .get(net_id)
                .is_some_and(|references| references.is_subset(&selected_components))
        })
        .collect::<Vec<_>>();

    let mut existing_nets = project.board.nets.keys().cloned().collect::<BTreeSet<_>>();
    let mut net_id_map = BTreeMap::new();
    for net_id in local_nets {
        let duplicate_id = unique_duplicate_id(&net_id, &mut existing_nets);
        net_id_map.insert(net_id, duplicate_id);
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let component_values = {
        let components = board_child_mapping_mut(&mut yaml, "components")?;
        selected_components
            .iter()
            .map(|component_id| {
                let value = components
                    .get(serde_yaml_ng::Value::String(component_id.clone()))
                    .with_context(|| format!("Board IR component {component_id} was not found."))?
                    .clone();
                Ok((component_id.clone(), value))
            })
            .collect::<Result<Vec<_>>>()?
    };
    let net_values = {
        let nets = board_child_mapping_mut(&mut yaml, "nets")?;
        net_id_map
            .keys()
            .map(|net_id| {
                let value = nets
                    .get(serde_yaml_ng::Value::String(net_id.clone()))
                    .with_context(|| format!("Board IR net {net_id} was not found."))?
                    .clone();
                Ok((net_id.clone(), value))
            })
            .collect::<Result<Vec<_>>>()?
    };

    {
        let nets = board_child_mapping_mut(&mut yaml, "nets")?;
        for (net_id, value) in net_values {
            let duplicate_id = net_id_map
                .get(&net_id)
                .expect("net value was captured from net_id_map");
            nets.insert(serde_yaml_ng::Value::String(duplicate_id.clone()), value);
        }
    }
    {
        let components = board_child_mapping_mut(&mut yaml, "components")?;
        for (component_id, mut value) in component_values {
            let duplicate_id = component_id_map
                .get(&component_id)
                .expect("component value was captured from selected components");
            let component = value
                .as_mapping_mut()
                .with_context(|| format!("Board IR component {component_id} must be an object."))?;
            component.remove(serde_yaml_ng::Value::String("source".to_string()));
            if let Some(pins) = component
                .get_mut(serde_yaml_ng::Value::String("pins".to_string()))
                .and_then(serde_yaml_ng::Value::as_mapping_mut)
            {
                for net_value in pins.values_mut() {
                    if let Some(net_id) = net_value.as_str()
                        && let Some(duplicate_net_id) = net_id_map.get(net_id)
                    {
                        *net_value = serde_yaml_ng::Value::String(duplicate_net_id.clone());
                    }
                }
            }
            components.insert(serde_yaml_ng::Value::String(duplicate_id.clone()), value);
        }
    }
    duplicate_schematic_metadata(&mut yaml, &component_id_map, &net_id_map, position_offset)?;

    let mut selections = component_id_map
        .values()
        .cloned()
        .map(SketchSelection::Component)
        .collect::<Vec<_>>();
    selections.extend(net_id_map.values().cloned().map(SketchSelection::Net));
    Ok((encode_edited_project_yaml(yaml)?, selections))
}

fn unique_duplicate_id(original: &str, existing: &mut BTreeSet<String>) -> String {
    let (prefix, number) = split_trailing_number(original);
    if let Some(number) = number {
        for value in (number + 1).. {
            let candidate = format!("{prefix}{value}");
            if existing.insert(candidate.clone()) {
                return candidate;
            }
        }
    }
    let base = format!("{original}_copy");
    let mut candidate = base.clone();
    let mut suffix = 2usize;
    while !existing.insert(candidate.clone()) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    candidate
}

fn split_trailing_number(value: &str) -> (&str, Option<u64>) {
    let split = value
        .char_indices()
        .rev()
        .find(|(_, character)| !character.is_ascii_digit())
        .map(|(index, character)| index + character.len_utf8())
        .unwrap_or(0);
    if split == value.len() {
        return (value, None);
    }
    value[split..]
        .parse::<u64>()
        .ok()
        .map(|number| (&value[..split], Some(number)))
        .unwrap_or((value, None))
}

fn duplicate_schematic_metadata(
    yaml: &mut serde_yaml_ng::Value,
    component_id_map: &BTreeMap<String, String>,
    net_id_map: &BTreeMap<String, String>,
    position_offset: egui::Vec2,
) -> Result<()> {
    let Some(schematic) = yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?
        .get_mut(serde_yaml_ng::Value::String("board".to_string()))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|board| board.get_mut(serde_yaml_ng::Value::String("schematic".to_string())))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
    else {
        return Ok(());
    };
    let mapping_pairs = component_id_map
        .iter()
        .map(|(from, to)| {
            (
                SketchSelection::Component(from.clone()),
                SketchSelection::Component(to.clone()),
            )
        })
        .chain(net_id_map.iter().map(|(from, to)| {
            (
                SketchSelection::Net(from.clone()),
                SketchSelection::Net(to.clone()),
            )
        }))
        .collect::<Vec<_>>();
    if let Some(positions) = schematic
        .get_mut(serde_yaml_ng::Value::String("node_positions".to_string()))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
    {
        let copies = mapping_pairs
            .iter()
            .filter_map(|(from, to)| {
                let from_key = schematic_node_key(from)?;
                let to_key = schematic_node_key(to)?;
                let mut value = positions
                    .get(serde_yaml_ng::Value::String(from_key))?
                    .clone();
                if let Some(position) = value.as_mapping_mut() {
                    offset_position_field(position, "x", f64::from(position_offset.x));
                    offset_position_field(position, "y", f64::from(position_offset.y));
                }
                Some((to_key, value))
            })
            .collect::<Vec<_>>();
        for (key, value) in copies {
            positions.insert(serde_yaml_ng::Value::String(key), value);
        }
    }
    if let Some(styles) = schematic
        .get_mut(serde_yaml_ng::Value::String("node_styles".to_string()))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
    {
        let copies = component_id_map
            .iter()
            .filter_map(|(from, to)| {
                let from_key = schematic_node_key(&SketchSelection::Component(from.clone()))?;
                let to_key = schematic_node_key(&SketchSelection::Component(to.clone()))?;
                let value = styles.get(serde_yaml_ng::Value::String(from_key))?.clone();
                Some((to_key, value))
            })
            .collect::<Vec<_>>();
        for (key, value) in copies {
            styles.insert(serde_yaml_ng::Value::String(key), value);
        }
    }
    Ok(())
}

fn offset_position_field(position: &mut serde_yaml_ng::Mapping, field: &str, offset: f64) {
    let key = serde_yaml_ng::Value::String(field.to_string());
    if let Some(value) = position.get_mut(&key)
        && let Some(number) = value.as_f64()
    {
        *value = serde_yaml_ng::to_value(number + offset).unwrap_or_else(|_| value.clone());
    }
}
