use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

#[derive(Debug, Clone)]
pub(super) struct ProjectSnapshot {
    pub(super) name: String,
    pub(super) components: usize,
    pub(super) nets: usize,
    pub(super) scenarios: usize,
    pub(super) libraries: Vec<String>,
    pub(super) components_detail: Vec<SketchComponent>,
    pub(super) nets_detail: Vec<SketchNet>,
}

#[derive(Debug, Clone)]
pub(super) struct SketchComponent {
    pub(super) id: String,
    pub(super) model: String,
    pub(super) part_number: Option<String>,
    pub(super) pins: Vec<SketchPin>,
    pub(super) position: Option<SketchPosition>,
}

#[derive(Debug, Clone)]
pub(super) struct SketchPin {
    pub(super) pin: String,
    pub(super) net: String,
}

#[derive(Debug, Clone)]
pub(super) struct SketchNet {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) nominal_voltage: Option<f64>,
    pub(super) powered: Option<bool>,
    pub(super) connections: Vec<String>,
    pub(super) position: Option<SketchPosition>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SketchPosition {
    pub(super) x: f64,
    pub(super) y: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SketchViewport {
    pub(super) pan: egui::Vec2,
    pub(super) zoom: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SketchSelection {
    Component(String),
    Net(String),
    Overflow(String),
}

#[derive(Debug)]
pub(super) struct SketchGraph {
    pub(super) nodes: Vec<SketchNode>,
    pub(super) pin_anchors: Vec<SketchPinAnchor>,
    pub(super) edges: Vec<SketchEdge>,
}

#[derive(Debug)]
pub(super) struct SketchNode {
    pub(super) selection: SketchSelection,
    pub(super) label: String,
    pub(super) detail: String,
    pub(super) rect: egui::Rect,
}

#[derive(Debug)]
pub(super) struct SketchPinAnchor {
    pub(super) component_id: String,
    pub(super) pin: String,
    pub(super) net: String,
    pub(super) pos: egui::Pos2,
    pub(super) label_pos: egui::Pos2,
}

#[derive(Debug)]
pub(super) struct SketchEdge {
    pub(super) start: egui::Pos2,
    pub(super) end: egui::Pos2,
}
pub(super) fn load_project_snapshot(path: &Path) -> Result<ProjectSnapshot> {
    let project = crate::board_ir::load_project(path)?;
    Ok(project_snapshot_from_project(project))
}

pub(super) fn load_project_snapshot_from_yaml(text: &str) -> Result<ProjectSnapshot> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    Ok(project_snapshot_from_project(project))
}

fn project_snapshot_from_project(project: crate::board_ir::BoardProject) -> ProjectSnapshot {
    let positions = project.board.schematic.node_positions;
    let components_detail: Vec<_> = project
        .board
        .components
        .iter()
        .map(|(id, component)| SketchComponent {
            id: id.clone(),
            model: component.model.clone(),
            part_number: component.part_number.clone(),
            position: sketch_position_for(&positions, &SketchSelection::Component(id.clone())),
            pins: component
                .pins
                .iter()
                .map(|(pin, net)| SketchPin {
                    pin: pin.clone(),
                    net: net.clone(),
                })
                .collect(),
        })
        .collect();
    let mut connections_by_net = std::collections::BTreeMap::<String, Vec<String>>::new();
    for component in &components_detail {
        for pin in &component.pins {
            connections_by_net
                .entry(pin.net.clone())
                .or_default()
                .push(format!("{}.{}", component.id, pin.pin));
        }
    }
    let nets_detail: Vec<_> = project
        .board
        .nets
        .iter()
        .map(|(id, net)| SketchNet {
            id: id.clone(),
            kind: net_kind_label(&net.kind).to_string(),
            nominal_voltage: net.nominal_voltage,
            powered: net.powered,
            connections: connections_by_net.remove(id).unwrap_or_default(),
            position: sketch_position_for(&positions, &SketchSelection::Net(id.clone())),
        })
        .collect();
    ProjectSnapshot {
        name: project.project.name,
        components: project.board.components.len(),
        nets: project.board.nets.len(),
        scenarios: project.scenarios.len(),
        libraries: project
            .libraries
            .iter()
            .map(|library| library.to_string())
            .collect(),
        components_detail,
        nets_detail,
    }
}

fn sketch_position_for(
    positions: &std::collections::BTreeMap<String, crate::board_ir::SchematicNodePosition>,
    selection: &SketchSelection,
) -> Option<SketchPosition> {
    positions
        .get(&schematic_node_key(selection)?)
        .map(|position| SketchPosition {
            x: position.x,
            y: position.y,
        })
}

pub(super) fn validate_board_ir_yaml_text(text: &str) -> Result<()> {
    let _project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    Ok(())
}

fn net_kind_label(kind: &crate::board_ir::NetKind) -> &'static str {
    match kind {
        crate::board_ir::NetKind::Power => "power",
        crate::board_ir::NetKind::Ground => "ground",
        crate::board_ir::NetKind::DigitalOrAnalog => "digital_or_analog",
    }
}

pub(super) fn edit_component_model(text: &str, component_id: &str, model: &str) -> Result<String> {
    edit_component_field(
        text,
        component_id,
        "model",
        Some(serde_yaml_ng::Value::String(model.trim().to_string())),
    )
}

pub(super) fn edit_schematic_node_position(
    text: &str,
    selection: &SketchSelection,
    x: f64,
    y: f64,
) -> Result<String> {
    if !x.is_finite() || !y.is_finite() {
        anyhow::bail!("Schematic node position must be finite.");
    }
    let key = schematic_node_key(selection).with_context(
        || "Only component and net nodes can be positioned on the schematic canvas.",
    )?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let board = yaml
            .as_mapping_mut()
            .context("Board IR project must be a YAML object.")?
            .get_mut(serde_yaml_ng::Value::String("board".to_string()))
            .context("Board IR project is missing board.")?
            .as_mapping_mut()
            .context("Board IR field board must be an object.")?;
        let schematic = ensure_child_mapping_mut(board, "schematic", "board schematic")?;
        let positions =
            ensure_child_mapping_mut(schematic, "node_positions", "schematic node positions")?;
        let mut position = serde_yaml_ng::Mapping::new();
        position.insert(
            serde_yaml_ng::Value::String("x".to_string()),
            serde_yaml_ng::to_value(x)?,
        );
        position.insert(
            serde_yaml_ng::Value::String("y".to_string()),
            serde_yaml_ng::to_value(y)?,
        );
        positions.insert(
            serde_yaml_ng::Value::String(key),
            serde_yaml_ng::Value::Mapping(position),
        );
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn add_component(text: &str, component_id: &str, model: &str) -> Result<String> {
    let component_id = validated_graph_id(component_id, "component")?;
    let model = model.trim();
    if model.is_empty() {
        anyhow::bail!("Component model must not be blank.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let components = ensure_board_child_mapping_mut(&mut yaml, "components")?;
        let key = serde_yaml_ng::Value::String(component_id.to_string());
        if components.contains_key(&key) {
            anyhow::bail!("Board IR component {component_id} already exists.");
        }
        let mut component = serde_yaml_ng::Mapping::new();
        component.insert(
            serde_yaml_ng::Value::String("model".to_string()),
            serde_yaml_ng::Value::String(model.to_string()),
        );
        components.insert(key, serde_yaml_ng::Value::Mapping(component));
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn add_component_with_ports(
    text: &str,
    component_id: &str,
    model: &str,
    ports: &[(String, String)],
) -> Result<String> {
    let component_id = validated_graph_id(component_id, "component")?;
    let model = model.trim();
    if model.is_empty() {
        anyhow::bail!("Component model must not be blank.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let components = ensure_board_child_mapping_mut(&mut yaml, "components")?;
        let component_key = serde_yaml_ng::Value::String(component_id.to_string());
        if components.contains_key(&component_key) {
            anyhow::bail!("Board IR component {component_id} already exists.");
        }
    }

    let pin_nets = default_pin_nets(&mut yaml, component_id, ports)?;
    {
        let components = ensure_board_child_mapping_mut(&mut yaml, "components")?;
        let mut component = serde_yaml_ng::Mapping::new();
        component.insert(
            serde_yaml_ng::Value::String("model".to_string()),
            serde_yaml_ng::Value::String(model.to_string()),
        );
        if !pin_nets.is_empty() {
            let pins = pin_nets
                .into_iter()
                .map(|(pin, net)| {
                    (
                        serde_yaml_ng::Value::String(pin),
                        serde_yaml_ng::Value::String(net),
                    )
                })
                .collect();
            component.insert(
                serde_yaml_ng::Value::String("pins".to_string()),
                serde_yaml_ng::Value::Mapping(pins),
            );
        }
        components.insert(
            serde_yaml_ng::Value::String(component_id.to_string()),
            serde_yaml_ng::Value::Mapping(component),
        );
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn remove_component(text: &str, component_id: &str) -> Result<String> {
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let components = board_child_mapping_mut(&mut yaml, "components")?;
        let key = serde_yaml_ng::Value::String(component_id.to_string());
        if components.remove(&key).is_none() {
            anyhow::bail!("Board IR component {component_id} was not found.");
        }
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn assign_component_pin(
    text: &str,
    component_id: &str,
    pin_id: &str,
    net_id: &str,
) -> Result<String> {
    let pin_id = validated_graph_id(pin_id, "pin")?;
    let net_id = validated_graph_id(net_id, "net")?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let nets = board_child_mapping_mut(&mut yaml, "nets")?;
        let net_key = serde_yaml_ng::Value::String(net_id.to_string());
        if !nets.contains_key(&net_key) {
            anyhow::bail!("Board IR net {net_id} was not found.");
        }
    }
    {
        let components = board_child_mapping_mut(&mut yaml, "components")?;
        let component = named_child_mapping_mut(components, component_id, "component")?;
        let pins = ensure_child_mapping_mut(component, "pins", "component pins")?;
        pins.insert(
            serde_yaml_ng::Value::String(pin_id.to_string()),
            serde_yaml_ng::Value::String(net_id.to_string()),
        );
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn connect_component_pins(
    text: &str,
    source_component_id: &str,
    source_pin_id: &str,
    target_component_id: &str,
    target_pin_id: &str,
) -> Result<String> {
    let source_pin_id = validated_graph_id(source_pin_id, "source pin")?;
    let target_pin_id = validated_graph_id(target_pin_id, "target pin")?;
    if source_component_id == target_component_id && source_pin_id == target_pin_id {
        anyhow::bail!("Cannot wire a pin to itself.");
    }

    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let source_net = component_pin_net(&project, source_component_id, source_pin_id)?;
    let target_net = component_pin_net(&project, target_component_id, target_pin_id)?;

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let net_id = if let Some(net_id) = source_net.or(target_net) {
        net_id
    } else {
        insert_generated_wire_net(&mut yaml, source_component_id, source_pin_id)?
    };
    {
        let components = board_child_mapping_mut(&mut yaml, "components")?;
        let source = named_child_mapping_mut(components, source_component_id, "component")?;
        let pins = ensure_child_mapping_mut(source, "pins", "component pins")?;
        pins.insert(
            serde_yaml_ng::Value::String(source_pin_id.to_string()),
            serde_yaml_ng::Value::String(net_id.clone()),
        );
    }
    {
        let components = board_child_mapping_mut(&mut yaml, "components")?;
        let target = named_child_mapping_mut(components, target_component_id, "component")?;
        let pins = ensure_child_mapping_mut(target, "pins", "component pins")?;
        pins.insert(
            serde_yaml_ng::Value::String(target_pin_id.to_string()),
            serde_yaml_ng::Value::String(net_id),
        );
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn remove_component_pin(text: &str, component_id: &str, pin_id: &str) -> Result<String> {
    let pin_id = validated_graph_id(pin_id, "pin")?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let components = board_child_mapping_mut(&mut yaml, "components")?;
        let component = named_child_mapping_mut(components, component_id, "component")?;
        let pins = child_mapping_mut(component, "pins", "component pins")?;
        let pin_key = serde_yaml_ng::Value::String(pin_id.to_string());
        if pins.remove(&pin_key).is_none() {
            anyhow::bail!("Board IR component {component_id} pin {pin_id} was not found.");
        }
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn edit_component_part_number(
    text: &str,
    component_id: &str,
    part_number: &str,
) -> Result<String> {
    let part_number = part_number.trim();
    edit_component_field(
        text,
        component_id,
        "part_number",
        if part_number.is_empty() {
            None
        } else {
            Some(serde_yaml_ng::Value::String(part_number.to_string()))
        },
    )
}

pub(super) fn add_net(text: &str, net_id: &str, kind: &str) -> Result<String> {
    let net_id = validated_graph_id(net_id, "net")?;
    let kind = normalized_net_kind(kind)?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let nets = ensure_board_child_mapping_mut(&mut yaml, "nets")?;
        let key = serde_yaml_ng::Value::String(net_id.to_string());
        if nets.contains_key(&key) {
            anyhow::bail!("Board IR net {net_id} already exists.");
        }
        let mut net = serde_yaml_ng::Mapping::new();
        net.insert(
            serde_yaml_ng::Value::String("kind".to_string()),
            serde_yaml_ng::Value::String(kind.to_string()),
        );
        nets.insert(key, serde_yaml_ng::Value::Mapping(net));
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn remove_net(text: &str, net_id: &str) -> Result<String> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let references: Vec<_> = project
        .board
        .components
        .iter()
        .flat_map(|(component_id, component)| {
            component
                .pins
                .iter()
                .filter(move |(_, pin_net)| pin_net.as_str() == net_id)
                .map(move |(pin_id, _)| format!("{component_id}.{pin_id}"))
        })
        .collect();
    if !references.is_empty() {
        anyhow::bail!(
            "Board IR net {net_id} is still referenced by {}.",
            references.join(", ")
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let nets = board_child_mapping_mut(&mut yaml, "nets")?;
        let key = serde_yaml_ng::Value::String(net_id.to_string());
        if nets.remove(&key).is_none() {
            anyhow::bail!("Board IR net {net_id} was not found.");
        }
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn edit_net_kind(text: &str, net_id: &str, kind: &str) -> Result<String> {
    let kind = normalized_net_kind(kind)?;
    edit_net_field(
        text,
        net_id,
        "kind",
        Some(serde_yaml_ng::Value::String(kind.to_string())),
    )
}

pub(super) fn edit_net_nominal_voltage(
    text: &str,
    net_id: &str,
    voltage: Option<f64>,
) -> Result<String> {
    edit_net_field(
        text,
        net_id,
        "nominal_voltage",
        voltage
            .map(serde_yaml_ng::to_value)
            .transpose()
            .context("Failed to encode net nominal voltage.")?,
    )
}

pub(super) fn edit_net_powered(text: &str, net_id: &str, powered: Option<bool>) -> Result<String> {
    edit_net_field(
        text,
        net_id,
        "powered",
        powered
            .map(serde_yaml_ng::to_value)
            .transpose()
            .context("Failed to encode net powered flag.")?,
    )
}

fn edit_component_field(
    text: &str,
    component_id: &str,
    field: &str,
    value: Option<serde_yaml_ng::Value>,
) -> Result<String> {
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let components = board_child_mapping_mut(&mut yaml, "components")?;
        let component = named_child_mapping_mut(components, component_id, "component")?;
        set_or_remove_yaml_field(component, field, value);
    }
    encode_edited_project_yaml(yaml)
}

fn edit_net_field(
    text: &str,
    net_id: &str,
    field: &str,
    value: Option<serde_yaml_ng::Value>,
) -> Result<String> {
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let nets = board_child_mapping_mut(&mut yaml, "nets")?;
        let net = named_child_mapping_mut(nets, net_id, "net")?;
        set_or_remove_yaml_field(net, field, value);
    }
    encode_edited_project_yaml(yaml)
}

fn board_child_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    field: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    yaml.as_mapping_mut()
        .context("Board IR project must be a YAML object.")?
        .get_mut(serde_yaml_ng::Value::String("board".to_string()))
        .context("Board IR project is missing board.")?
        .as_mapping_mut()
        .context("Board IR field board must be an object.")?
        .get_mut(serde_yaml_ng::Value::String(field.to_string()))
        .with_context(|| format!("Board IR board is missing {field}."))?
        .as_mapping_mut()
        .with_context(|| format!("Board IR board.{field} must be an object."))
}

fn ensure_board_child_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    field: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let board = yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?
        .get_mut(serde_yaml_ng::Value::String("board".to_string()))
        .context("Board IR project is missing board.")?
        .as_mapping_mut()
        .context("Board IR field board must be an object.")?;
    let key = serde_yaml_ng::Value::String(field.to_string());
    if !board.contains_key(&key) {
        board.insert(
            key.clone(),
            serde_yaml_ng::Value::Mapping(Default::default()),
        );
    }
    board
        .get_mut(&key)
        .expect("field was inserted when absent")
        .as_mapping_mut()
        .with_context(|| format!("Board IR board.{field} must be an object."))
}

fn named_child_mapping_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    id: &str,
    label: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    mapping
        .get_mut(serde_yaml_ng::Value::String(id.to_string()))
        .with_context(|| format!("Board IR {label} {id} was not found."))?
        .as_mapping_mut()
        .with_context(|| format!("Board IR {label} {id} must be an object."))
}

fn ensure_child_mapping_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    field: &str,
    label: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let key = serde_yaml_ng::Value::String(field.to_string());
    if !mapping.contains_key(&key) {
        mapping.insert(
            key.clone(),
            serde_yaml_ng::Value::Mapping(Default::default()),
        );
    }
    mapping
        .get_mut(&key)
        .expect("field was inserted when absent")
        .as_mapping_mut()
        .with_context(|| format!("Board IR {label} must be an object."))
}

fn default_pin_nets(
    yaml: &mut serde_yaml_ng::Value,
    component_id: &str,
    ports: &[(String, String)],
) -> Result<Vec<(String, String)>> {
    let mut pin_nets = Vec::new();
    for (pin_id, port_kind) in ports {
        let pin_id = validated_graph_id(pin_id, "pin")?.to_string();
        let net_id = insert_default_pin_net(yaml, component_id, &pin_id, port_kind)?;
        pin_nets.push((pin_id, net_id));
    }
    Ok(pin_nets)
}

fn insert_default_pin_net(
    yaml: &mut serde_yaml_ng::Value,
    component_id: &str,
    pin_id: &str,
    port_kind: &str,
) -> Result<String> {
    let base = generated_net_base(component_id, pin_id);
    let net_kind = default_net_kind_for_port(port_kind);
    let nets = ensure_board_child_mapping_mut(yaml, "nets")?;
    let mut net_id = base.clone();
    let mut suffix = 2;
    while nets.contains_key(serde_yaml_ng::Value::String(net_id.clone())) {
        net_id = format!("{base}_{suffix}");
        suffix += 1;
    }
    let mut net = serde_yaml_ng::Mapping::new();
    net.insert(
        serde_yaml_ng::Value::String("kind".to_string()),
        serde_yaml_ng::Value::String(net_kind.to_string()),
    );
    nets.insert(
        serde_yaml_ng::Value::String(net_id.clone()),
        serde_yaml_ng::Value::Mapping(net),
    );
    Ok(net_id)
}

fn generated_net_base(component_id: &str, pin_id: &str) -> String {
    format!("{component_id}_{pin_id}").to_ascii_lowercase()
}

fn default_net_kind_for_port(port_kind: &str) -> &'static str {
    match port_kind {
        "electrical_power" => "power",
        "electrical_ground" => "ground",
        _ => "digital_or_analog",
    }
}

fn component_pin_net(
    project: &crate::board_ir::BoardProject,
    component_id: &str,
    pin_id: &str,
) -> Result<Option<String>> {
    let component = project
        .board
        .components
        .get(component_id)
        .with_context(|| format!("Board IR component {component_id} was not found."))?;
    Ok(component.pins.get(pin_id).cloned())
}

fn insert_generated_wire_net(
    yaml: &mut serde_yaml_ng::Value,
    component_id: &str,
    pin_id: &str,
) -> Result<String> {
    let base = generated_wire_net_base(component_id, pin_id);
    let nets = ensure_board_child_mapping_mut(yaml, "nets")?;
    let mut net_id = base.clone();
    let mut suffix = 2;
    while nets.contains_key(serde_yaml_ng::Value::String(net_id.clone())) {
        net_id = format!("{base}_{suffix}");
        suffix += 1;
    }
    let mut net = serde_yaml_ng::Mapping::new();
    net.insert(
        serde_yaml_ng::Value::String("kind".to_string()),
        serde_yaml_ng::Value::String("digital_or_analog".to_string()),
    );
    nets.insert(
        serde_yaml_ng::Value::String(net_id.clone()),
        serde_yaml_ng::Value::Mapping(net),
    );
    Ok(net_id)
}

fn generated_wire_net_base(component_id: &str, pin_id: &str) -> String {
    format!("net_{}_{}", component_id, pin_id).to_ascii_lowercase()
}

fn child_mapping_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    field: &str,
    label: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    mapping
        .get_mut(serde_yaml_ng::Value::String(field.to_string()))
        .with_context(|| format!("Board IR {label} field {field} was not found."))?
        .as_mapping_mut()
        .with_context(|| format!("Board IR {label} must be an object."))
}

fn set_or_remove_yaml_field(
    mapping: &mut serde_yaml_ng::Mapping,
    field: &str,
    value: Option<serde_yaml_ng::Value>,
) {
    let key = serde_yaml_ng::Value::String(field.to_string());
    if let Some(value) = value {
        mapping.insert(key, value);
    } else {
        mapping.remove(&key);
    }
}

fn encode_edited_project_yaml(yaml: serde_yaml_ng::Value) -> Result<String> {
    let text =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    validate_board_ir_yaml_text(&text)?;
    Ok(text)
}

fn validated_graph_id<'a>(id: &'a str, label: &str) -> Result<&'a str> {
    let id = id.trim();
    if id.is_empty() {
        anyhow::bail!("Board IR {label} id must not be blank.");
    }
    if !id
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        anyhow::bail!(
            "Board IR {label} id {id} contains unsupported characters for GUI graph editing."
        );
    }
    Ok(id)
}

fn normalized_net_kind(kind: &str) -> Result<&str> {
    match kind {
        "power" | "ground" | "digital_or_analog" => Ok(kind),
        _ => anyhow::bail!("Unsupported net kind {kind}."),
    }
}

fn schematic_node_key(selection: &SketchSelection) -> Option<String> {
    match selection {
        SketchSelection::Component(id) => Some(format!("component:{id}")),
        SketchSelection::Net(id) => Some(format!("net:{id}")),
        SketchSelection::Overflow(_) => None,
    }
}

pub(super) fn layout_sketch_graph(rect: egui::Rect, snapshot: &ProjectSnapshot) -> SketchGraph {
    let margin = 18.0;
    let node_width = ((rect.width() - 3.0 * margin) / 2.0).clamp(150.0, 260.0);
    let component_height = 92.0;
    let net_height = 44.0;
    let row_gap = 10.0;
    let left_x = rect.left() + margin;
    let right_x = rect.right() - margin - node_width;
    let top = rect.top() + margin;
    let max_component_rows = ((rect.height() - 2.0 * margin) / (component_height + row_gap))
        .floor()
        .max(1.0) as usize;
    let max_net_rows = ((rect.height() - 2.0 * margin) / (net_height + row_gap))
        .floor()
        .max(1.0) as usize;

    let mut nodes = Vec::new();
    let component_count = snapshot.components_detail.len().min(max_component_rows);
    for (index, component) in snapshot
        .components_detail
        .iter()
        .take(max_component_rows)
        .enumerate()
    {
        let default = egui::pos2(left_x, top + index as f32 * (component_height + row_gap));
        nodes.push(SketchNode {
            selection: SketchSelection::Component(component.id.clone()),
            label: component.id.clone(),
            detail: compact_label(
                &format!("{} / {} pins", component.model, component.pins.len()),
                34,
            ),
            rect: node_rect_from_position(
                rect,
                component.position,
                default,
                node_width,
                component_height,
            ),
        });
    }

    let net_count = snapshot.nets_detail.len().min(max_net_rows);
    for (index, net) in snapshot.nets_detail.iter().take(max_net_rows).enumerate() {
        let default = egui::pos2(right_x, top + index as f32 * (net_height + row_gap));
        nodes.push(SketchNode {
            selection: SketchSelection::Net(net.id.clone()),
            label: net.id.clone(),
            detail: format!("{} / {} conn", net.kind, net.connections.len()),
            rect: node_rect_from_position(rect, net.position, default, node_width, net_height),
        });
    }

    let mut component_centers = std::collections::BTreeMap::new();
    let mut net_centers = std::collections::BTreeMap::new();
    for node in &nodes {
        match &node.selection {
            SketchSelection::Component(id) => {
                component_centers.insert(id.as_str(), node.rect.center());
            }
            SketchSelection::Net(id) => {
                net_centers.insert(id.as_str(), node.rect.center());
            }
            SketchSelection::Overflow(_) => {}
        }
    }

    let mut pin_anchors = Vec::new();
    for component in snapshot.components_detail.iter().take(component_count) {
        let Some(node) = nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Component(component.id.clone()))
        else {
            continue;
        };
        for anchor in component_pin_anchors(component, node.rect) {
            pin_anchors.push(anchor);
        }
    }

    let mut pin_anchor_positions = std::collections::BTreeMap::new();
    for anchor in &pin_anchors {
        pin_anchor_positions.insert(
            (anchor.component_id.as_str(), anchor.pin.as_str()),
            anchor.pos,
        );
    }

    let mut edges = Vec::new();
    for component in snapshot.components_detail.iter().take(component_count) {
        let Some(start) = component_centers.get(component.id.as_str()).copied() else {
            continue;
        };
        for pin in &component.pins {
            if let Some(end) = net_centers.get(pin.net.as_str()).copied() {
                let start = pin_anchor_positions
                    .get(&(component.id.as_str(), pin.pin.as_str()))
                    .copied()
                    .unwrap_or(start);
                edges.push(SketchEdge { start, end });
                if edges.len() >= 80 {
                    break;
                }
            }
        }
        if edges.len() >= 80 {
            break;
        }
    }

    if snapshot.components_detail.len() > component_count {
        push_overflow_hint(
            &mut nodes,
            left_x,
            rect.bottom() - margin - component_height,
            node_width,
            component_height,
            snapshot.components_detail.len() - component_count,
            "more components",
        );
    }
    if snapshot.nets_detail.len() > net_count {
        push_overflow_hint(
            &mut nodes,
            right_x,
            rect.bottom() - margin - net_height,
            node_width,
            net_height,
            snapshot.nets_detail.len() - net_count,
            "more nets",
        );
    }

    SketchGraph {
        nodes,
        pin_anchors,
        edges,
    }
}

pub(super) fn layout_sketch_graph_viewport(
    rect: egui::Rect,
    snapshot: &ProjectSnapshot,
    viewport: SketchViewport,
) -> SketchGraph {
    let mut graph = layout_sketch_graph(rect, snapshot);
    transform_sketch_graph(&mut graph, rect, viewport);
    graph
}

pub(super) fn sketch_graph_bounds(graph: &SketchGraph) -> Option<egui::Rect> {
    let mut bounds: Option<egui::Rect> = None;
    let mut include_rect = |rect: egui::Rect| {
        bounds = Some(bounds.map_or(rect, |current| current.union(rect)));
    };
    for node in &graph.nodes {
        if matches!(node.selection, SketchSelection::Overflow(_)) {
            continue;
        }
        include_rect(node.rect);
    }
    for anchor in &graph.pin_anchors {
        include_rect(egui::Rect::from_center_size(
            anchor.pos,
            egui::vec2(10.0, 10.0),
        ));
        include_rect(egui::Rect::from_center_size(
            anchor.label_pos,
            egui::vec2(56.0, 14.0),
        ));
    }
    for edge in &graph.edges {
        include_rect(egui::Rect::from_two_pos(edge.start, edge.end));
    }
    bounds
}

pub(super) fn persisted_node_position_from_screen(
    canvas: egui::Rect,
    screen_position: egui::Pos2,
    screen_node_rect: egui::Rect,
    viewport: SketchViewport,
) -> (f64, f64) {
    let zoom = viewport.zoom.clamp(0.25, 4.0);
    let logical_position = inverse_viewport_pos(screen_position, canvas, viewport);
    let logical_width = screen_node_rect.width() / zoom;
    let logical_height = screen_node_rect.height() / zoom;
    let x = (logical_position.x - canvas.left() - logical_width / 2.0)
        .clamp(0.0, (canvas.width() - logical_width).max(0.0));
    let y = (logical_position.y - canvas.top() - logical_height / 2.0)
        .clamp(0.0, (canvas.height() - logical_height).max(0.0));
    (x as f64, y as f64)
}

fn transform_sketch_graph(graph: &mut SketchGraph, canvas: egui::Rect, viewport: SketchViewport) {
    for node in &mut graph.nodes {
        node.rect = transform_viewport_rect(node.rect, canvas, viewport);
    }
    for anchor in &mut graph.pin_anchors {
        anchor.pos = transform_viewport_pos(anchor.pos, canvas, viewport);
        anchor.label_pos = transform_viewport_pos(anchor.label_pos, canvas, viewport);
    }
    for edge in &mut graph.edges {
        edge.start = transform_viewport_pos(edge.start, canvas, viewport);
        edge.end = transform_viewport_pos(edge.end, canvas, viewport);
    }
}

fn transform_viewport_rect(
    rect: egui::Rect,
    canvas: egui::Rect,
    viewport: SketchViewport,
) -> egui::Rect {
    egui::Rect::from_min_max(
        transform_viewport_pos(rect.min, canvas, viewport),
        transform_viewport_pos(rect.max, canvas, viewport),
    )
}

fn transform_viewport_pos(
    pos: egui::Pos2,
    canvas: egui::Rect,
    viewport: SketchViewport,
) -> egui::Pos2 {
    let zoom = viewport.zoom.clamp(0.25, 4.0);
    canvas.min + viewport.pan + (pos - canvas.min) * zoom
}

fn inverse_viewport_pos(
    pos: egui::Pos2,
    canvas: egui::Rect,
    viewport: SketchViewport,
) -> egui::Pos2 {
    let zoom = viewport.zoom.clamp(0.25, 4.0);
    canvas.min + (pos - canvas.min - viewport.pan) / zoom
}

fn component_pin_anchors(component: &SketchComponent, rect: egui::Rect) -> Vec<SketchPinAnchor> {
    let visible_count = component.pins.len().min(8);
    if visible_count == 0 {
        return Vec::new();
    }
    component
        .pins
        .iter()
        .take(visible_count)
        .enumerate()
        .map(|(index, pin)| {
            let y = pin_anchor_y(rect, index, visible_count);
            SketchPinAnchor {
                component_id: component.id.clone(),
                pin: pin.pin.clone(),
                net: pin.net.clone(),
                pos: egui::pos2(rect.right(), y),
                label_pos: egui::pos2(rect.right() - 8.0, y),
            }
        })
        .collect()
}

fn pin_anchor_y(rect: egui::Rect, index: usize, count: usize) -> f32 {
    if count <= 1 {
        return rect.center().y;
    }
    let top = rect.top() + 30.0;
    let bottom = rect.bottom() - 12.0;
    top + (bottom - top) * index as f32 / (count - 1) as f32
}

fn node_rect_from_position(
    canvas: egui::Rect,
    position: Option<SketchPosition>,
    default: egui::Pos2,
    width: f32,
    height: f32,
) -> egui::Rect {
    let min = if let Some(position) = position {
        egui::pos2(
            canvas.left() + position.x as f32,
            canvas.top() + position.y as f32,
        )
    } else {
        default
    };
    let clamped = egui::pos2(
        min.x
            .clamp(canvas.left(), (canvas.right() - width).max(canvas.left())),
        min.y
            .clamp(canvas.top(), (canvas.bottom() - height).max(canvas.top())),
    );
    egui::Rect::from_min_size(clamped, egui::vec2(width, height))
}

fn push_overflow_hint(
    nodes: &mut Vec<SketchNode>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    count: usize,
    label: &str,
) {
    nodes.push(SketchNode {
        selection: SketchSelection::Overflow(label.to_string()),
        label: format!("+{count}"),
        detail: label.to_string(),
        rect: egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height)),
    });
}

pub(super) fn draw_sketch_node(
    painter: &egui::Painter,
    node: &SketchNode,
    selected: bool,
    runtime_activity: Option<f64>,
) {
    let base_fill = match node.selection {
        SketchSelection::Component(_) => egui::Color32::from_rgb(36, 52, 70),
        SketchSelection::Net(_) => egui::Color32::from_rgb(42, 62, 46),
        SketchSelection::Overflow(_) => egui::Color32::from_gray(36),
    };
    let fill = runtime_activity
        .map(|activity| runtime_activity_fill(base_fill, activity))
        .unwrap_or(base_fill);
    let stroke = if selected {
        egui::Stroke::new(2.0, egui::Color32::from_rgb(93, 185, 255))
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_gray(108))
    };
    painter.rect_filled(node.rect, 4.0, fill);
    painter.rect_stroke(node.rect, 4.0, stroke, egui::StrokeKind::Inside);
    painter.text(
        node.rect.left_top() + egui::vec2(8.0, 9.0),
        egui::Align2::LEFT_CENTER,
        compact_label(&node.label, 24),
        egui::FontId::monospace(13.0),
        egui::Color32::WHITE,
    );
    painter.text(
        node.rect.left_bottom() + egui::vec2(8.0, -12.0),
        egui::Align2::LEFT_CENTER,
        compact_label(&node.detail, 34),
        egui::FontId::monospace(11.0),
        egui::Color32::LIGHT_GRAY,
    );
}

pub(super) fn draw_sketch_pin_anchor(
    painter: &egui::Painter,
    anchor: &SketchPinAnchor,
    active: bool,
) {
    let fill = if active {
        egui::Color32::from_rgb(255, 196, 87)
    } else {
        egui::Color32::from_rgb(115, 166, 224)
    };
    painter.circle_filled(anchor.pos, 4.0, fill);
    painter.circle_stroke(
        anchor.pos,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(18)),
    );
    painter.text(
        anchor.label_pos,
        egui::Align2::RIGHT_CENTER,
        compact_label(&anchor.pin, 10),
        egui::FontId::monospace(10.5),
        egui::Color32::LIGHT_GRAY,
    );
}

fn runtime_activity_fill(base: egui::Color32, activity: f64) -> egui::Color32 {
    let activity = activity.clamp(0.0, 1.0) as f32;
    let highlight = egui::Color32::from_rgb(255, 196, 87);
    let mix = |base: u8, highlight: u8| -> u8 {
        (base as f32 + (highlight as f32 - base as f32) * activity * 0.7).round() as u8
    };
    egui::Color32::from_rgb(
        mix(base.r(), highlight.r()),
        mix(base.g(), highlight.g()),
        mix(base.b(), highlight.b()),
    )
}

fn compact_label(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut text = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    text.push_str("...");
    text
}

#[cfg(test)]
mod tests {
    use super::{
        SketchPosition, SketchSelection, SketchViewport, add_component, add_component_with_ports,
        add_net, assign_component_pin, connect_component_pins, edit_schematic_node_position,
        layout_sketch_graph, layout_sketch_graph_viewport, load_project_snapshot_from_yaml,
        persisted_node_position_from_screen, remove_component, remove_component_pin, remove_net,
        sketch_graph_bounds, validate_board_ir_yaml_text,
    };
    use crate::gui::CircuitCiApp;
    use crate::gui::sketch::{ProjectSnapshot, SketchComponent, SketchNet, SketchPin};
    use eframe::egui;

    fn editable_project_yaml() -> &'static str {
        "project:
  name: gui_graph_edit_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins:
        A: net_a
        B: gnd
  nets:
    net_a:
      kind: digital_or_analog
    gnd:
      kind: ground
"
    }

    #[test]
    fn add_and_remove_component_emit_valid_yaml() {
        let edited = add_component(
            editable_project_yaml(),
            "U2",
            "generic.schematic.imported_component",
        )
        .unwrap();
        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("U2:"));
        assert!(edited.contains("generic.schematic.imported_component"));

        let edited = remove_component(&edited, "U2").unwrap();
        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(!edited.contains("U2:"));
    }

    #[test]
    fn add_component_with_ports_creates_default_pin_nets() {
        let ports = vec![
            ("VIN".to_string(), "electrical_power".to_string()),
            ("GND".to_string(), "electrical_ground".to_string()),
            ("OUT".to_string(), "digital_electrical_output".to_string()),
        ];
        let edited = add_component_with_ports(
            editable_project_yaml(),
            "U2",
            "vendor.example.power_stage",
            &ports,
        )
        .unwrap();

        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("U2:"));
        assert!(edited.contains("VIN: u2_vin"));
        assert!(edited.contains("GND: u2_gnd"));
        assert!(edited.contains("OUT: u2_out"));
        assert!(edited.contains("u2_vin:\n      kind: power"));
        assert!(edited.contains("u2_gnd:\n      kind: ground"));
        assert!(edited.contains("u2_out:\n      kind: digital_or_analog"));
    }

    #[test]
    fn add_component_with_ports_suffixes_existing_generated_net() {
        let ports = vec![("VIN".to_string(), "electrical_power".to_string())];
        let project = add_net(editable_project_yaml(), "u2_vin", "power").unwrap();
        let edited =
            add_component_with_ports(&project, "U2", "vendor.example.power_stage", &ports).unwrap();

        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("VIN: u2_vin_2"));
    }

    #[test]
    fn add_and_remove_unreferenced_net_emit_valid_yaml() {
        let edited = add_net(editable_project_yaml(), "sense_new", "digital_or_analog").unwrap();
        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("sense_new:"));

        let edited = remove_net(&edited, "sense_new").unwrap();
        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(!edited.contains("sense_new:"));
    }

    #[test]
    fn remove_referenced_net_fails_closed() {
        let error = remove_net(editable_project_yaml(), "net_a").unwrap_err();
        assert!(error.to_string().contains("R1.A"));
    }

    #[test]
    fn assign_and_remove_component_pin_emit_valid_yaml() {
        let edited = add_net(editable_project_yaml(), "sense_new", "digital_or_analog").unwrap();
        let edited = assign_component_pin(&edited, "R1", "SENSE", "sense_new").unwrap();
        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("SENSE: sense_new"));

        let edited = remove_component_pin(&edited, "R1", "SENSE").unwrap();
        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(!edited.contains("SENSE: sense_new"));
    }

    #[test]
    fn assign_component_pin_rejects_missing_net() {
        let error = assign_component_pin(editable_project_yaml(), "R1", "SENSE", "missing_net")
            .unwrap_err();
        assert!(error.to_string().contains("missing_net"));
    }

    #[test]
    fn connect_component_pins_reuses_source_net() {
        let project = add_component(
            editable_project_yaml(),
            "U2",
            "generic.schematic.imported_component",
        )
        .unwrap();
        let project = assign_component_pin(&project, "U2", "P1", "gnd").unwrap();
        let edited = connect_component_pins(&project, "R1", "A", "U2", "P1").unwrap();

        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("A: net_a"));
        assert!(edited.contains("P1: net_a"));
    }

    #[test]
    fn connect_component_pins_creates_net_when_both_pins_are_unbound() {
        let project = add_component(
            editable_project_yaml(),
            "U2",
            "generic.schematic.imported_component",
        )
        .unwrap();
        let edited = connect_component_pins(&project, "R1", "SENSE", "U2", "P1").unwrap();

        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("SENSE: net_r1_sense"));
        assert!(edited.contains("P1: net_r1_sense"));
        assert!(edited.contains("net_r1_sense:\n      kind: digital_or_analog"));
    }

    #[test]
    fn add_component_rejects_unsafe_gui_id() {
        let error = add_component(
            editable_project_yaml(),
            "bad id",
            "generic.schematic.imported_component",
        )
        .unwrap_err();
        assert!(error.to_string().contains("unsupported characters"));
    }

    #[test]
    fn edit_schematic_node_position_emits_valid_yaml() {
        let edited = edit_schematic_node_position(
            editable_project_yaml(),
            &SketchSelection::Component("R1".to_string()),
            42.0,
            84.0,
        )
        .unwrap();
        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("component:R1"));
        assert!(edited.contains("x: 42"));
        assert!(edited.contains("y: 84"));
    }

    #[test]
    fn sketch_graph_layout_uses_saved_node_position() {
        let snapshot = ProjectSnapshot {
            name: "positioned_graph".to_string(),
            components: 1,
            nets: 1,
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![SketchComponent {
                id: "R1".to_string(),
                model: "generic.analog.resistor".to_string(),
                part_number: None,
                position: Some(SketchPosition { x: 50.0, y: 70.0 }),
                pins: vec![SketchPin {
                    pin: "A".to_string(),
                    net: "net_a".to_string(),
                }],
            }],
            nets_detail: vec![SketchNet {
                id: "net_a".to_string(),
                kind: "digital_or_analog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["R1.A".to_string()],
                position: Some(SketchPosition { x: 310.0, y: 70.0 }),
            }],
        };
        let graph = layout_sketch_graph(
            eframe::egui::Rect::from_min_size(
                eframe::egui::pos2(10.0, 20.0),
                eframe::egui::vec2(640.0, 320.0),
            ),
            &snapshot,
        );
        let component = graph
            .nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Component("R1".to_string()))
            .unwrap();
        assert_eq!(component.rect.left(), 60.0);
        assert_eq!(component.rect.top(), 90.0);
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn sketch_graph_layout_renders_pin_anchors() {
        let snapshot = ProjectSnapshot {
            name: "pin_graph".to_string(),
            components: 1,
            nets: 2,
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![SketchComponent {
                id: "U1".to_string(),
                model: "vendor.example.dual_pin".to_string(),
                part_number: None,
                position: None,
                pins: vec![
                    SketchPin {
                        pin: "VIN".to_string(),
                        net: "vin".to_string(),
                    },
                    SketchPin {
                        pin: "GND".to_string(),
                        net: "gnd".to_string(),
                    },
                ],
            }],
            nets_detail: vec![
                SketchNet {
                    id: "vin".to_string(),
                    kind: "power".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: vec!["U1.VIN".to_string()],
                    position: None,
                },
                SketchNet {
                    id: "gnd".to_string(),
                    kind: "ground".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: vec!["U1.GND".to_string()],
                    position: None,
                },
            ],
        };
        let graph = layout_sketch_graph(
            eframe::egui::Rect::from_min_size(
                eframe::egui::pos2(0.0, 0.0),
                eframe::egui::vec2(720.0, 360.0),
            ),
            &snapshot,
        );

        assert_eq!(graph.pin_anchors.len(), 2);
        let vin_anchor = graph
            .pin_anchors
            .iter()
            .find(|anchor| anchor.component_id == "U1" && anchor.pin == "VIN")
            .unwrap();
        assert_eq!(vin_anchor.net, "vin");
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.start.distance(vin_anchor.pos) < 0.01)
        );
    }

    #[test]
    fn sketch_graph_viewport_transforms_nodes_and_edges() {
        let snapshot = ProjectSnapshot {
            name: "viewport_graph".to_string(),
            components: 1,
            nets: 1,
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![SketchComponent {
                id: "R1".to_string(),
                model: "generic.analog.resistor".to_string(),
                part_number: None,
                position: Some(SketchPosition { x: 20.0, y: 30.0 }),
                pins: vec![SketchPin {
                    pin: "A".to_string(),
                    net: "net_a".to_string(),
                }],
            }],
            nets_detail: vec![SketchNet {
                id: "net_a".to_string(),
                kind: "digital_or_analog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["R1.A".to_string()],
                position: Some(SketchPosition { x: 300.0, y: 30.0 }),
            }],
        };
        let canvas = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(640.0, 320.0));
        let graph = layout_sketch_graph_viewport(
            canvas,
            &snapshot,
            SketchViewport {
                pan: egui::vec2(12.0, -8.0),
                zoom: 2.0,
            },
        );
        let component = graph
            .nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Component("R1".to_string()))
            .unwrap();

        assert_eq!(component.rect.left(), 62.0);
        assert_eq!(component.rect.top(), 62.0);
        assert!(component.rect.width() > 250.0);
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn sketch_graph_bounds_excludes_overflow_hints() {
        let snapshot = ProjectSnapshot {
            name: "bounds".to_string(),
            components: 1,
            nets: 1,
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![SketchComponent {
                id: "U1".to_string(),
                model: "generic.ic".to_string(),
                part_number: None,
                position: Some(SketchPosition { x: 20.0, y: 30.0 }),
                pins: vec![SketchPin {
                    pin: "OUT".to_string(),
                    net: "net_a".to_string(),
                }],
            }],
            nets_detail: vec![SketchNet {
                id: "net_a".to_string(),
                kind: "digital_or_analog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["U1.OUT".to_string()],
                position: Some(SketchPosition { x: 360.0, y: 90.0 }),
            }],
        };
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
        let graph = layout_sketch_graph(canvas, &snapshot);
        let bounds = super::sketch_graph_bounds(&graph).unwrap();

        assert!(bounds.left() <= 20.0);
        assert!(bounds.right() >= 360.0);
        assert!(bounds.bottom() >= 90.0);
    }

    #[test]
    fn persisted_node_position_inverts_viewport_transform() {
        let canvas = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(640.0, 320.0));
        let viewport = SketchViewport {
            pan: egui::vec2(12.0, -8.0),
            zoom: 2.0,
        };
        let screen_node =
            egui::Rect::from_min_size(egui::pos2(62.0, 62.0), egui::vec2(300.0, 184.0));
        let (x, y) = persisted_node_position_from_screen(
            canvas,
            egui::pos2(62.0 + 150.0, 62.0 + 92.0),
            screen_node,
            viewport,
        );

        assert_eq!(x, 20.0);
        assert_eq!(y, 30.0);
    }

    #[test]
    fn multi_selected_items_delete_as_one_validated_edit() {
        let yaml = "project:
  name: gui_delete_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins:
        A: net_a
        B: gnd
  nets:
    net_a:
      kind: digital_or_analog
    gnd:
      kind: ground
    loose:
      kind: digital_or_analog
";
        let mut app = CircuitCiApp {
            project_yaml: yaml.to_string(),
            project_snapshot: Some(load_project_snapshot_from_yaml(yaml).unwrap()),
            ..CircuitCiApp::default()
        };
        app.selected_sketch_items
            .insert(SketchSelection::Component("R1".to_string()));
        app.selected_sketch_items
            .insert(SketchSelection::Net("loose".to_string()));

        app.apply_delete_selected_sketch_item();

        validate_board_ir_yaml_text(&app.project_yaml).unwrap();
        assert!(!app.project_yaml.contains("R1:"));
        assert!(!app.project_yaml.contains("loose:"));
        assert!(app.project_yaml.contains("gnd:"));
        assert!(app.selected_sketch_items.is_empty());
        assert_eq!(app.project_yaml_undo.len(), 1);
    }

    #[test]
    fn fit_sketch_content_places_transformed_bounds_inside_canvas() {
        let snapshot = ProjectSnapshot {
            name: "fit".to_string(),
            components: 2,
            nets: 1,
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![
                SketchComponent {
                    id: "U1".to_string(),
                    model: "generic.ic".to_string(),
                    part_number: None,
                    position: Some(SketchPosition { x: 20.0, y: 40.0 }),
                    pins: vec![SketchPin {
                        pin: "OUT".to_string(),
                        net: "far_net".to_string(),
                    }],
                },
                SketchComponent {
                    id: "U2".to_string(),
                    model: "generic.ic".to_string(),
                    part_number: None,
                    position: Some(SketchPosition { x: 820.0, y: 420.0 }),
                    pins: vec![SketchPin {
                        pin: "IN".to_string(),
                        net: "far_net".to_string(),
                    }],
                },
            ],
            nets_detail: vec![SketchNet {
                id: "far_net".to_string(),
                kind: "digital_or_analog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["U1.OUT".to_string(), "U2.IN".to_string()],
                position: Some(SketchPosition { x: 460.0, y: 240.0 }),
            }],
        };
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
        let mut app = CircuitCiApp::default();

        app.fit_sketch_content(canvas, &snapshot);
        let graph = layout_sketch_graph_viewport(canvas, &snapshot, app.sketch_viewport());
        let bounds = sketch_graph_bounds(&graph).unwrap();
        let viewport = canvas.shrink(24.0);

        assert!(app.sketch_zoom < 1.0);
        assert!(viewport.contains(bounds.left_top()));
        assert!(viewport.contains(bounds.right_bottom()));
    }
}
