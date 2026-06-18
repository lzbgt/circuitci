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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SketchSelection {
    Component(String),
    Net(String),
    Overflow(String),
}

impl SketchSelection {
    pub(super) fn matches(&self, node: &SketchNode) -> bool {
        self == &node.selection
    }
}

#[derive(Debug)]
pub(super) struct SketchGraph {
    pub(super) nodes: Vec<SketchNode>,
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
    let node_height = 44.0;
    let row_gap = 10.0;
    let left_x = rect.left() + margin;
    let right_x = rect.right() - margin - node_width;
    let top = rect.top() + margin;
    let max_rows = ((rect.height() - 2.0 * margin) / (node_height + row_gap))
        .floor()
        .max(1.0) as usize;

    let mut nodes = Vec::new();
    let component_count = snapshot.components_detail.len().min(max_rows);
    for (index, component) in snapshot.components_detail.iter().take(max_rows).enumerate() {
        let default = egui::pos2(left_x, top + index as f32 * (node_height + row_gap));
        nodes.push(SketchNode {
            selection: SketchSelection::Component(component.id.clone()),
            label: component.id.clone(),
            detail: compact_label(&component.model, 34),
            rect: node_rect_from_position(
                rect,
                component.position,
                default,
                node_width,
                node_height,
            ),
        });
    }

    let net_count = snapshot.nets_detail.len().min(max_rows);
    for (index, net) in snapshot.nets_detail.iter().take(max_rows).enumerate() {
        let default = egui::pos2(right_x, top + index as f32 * (node_height + row_gap));
        nodes.push(SketchNode {
            selection: SketchSelection::Net(net.id.clone()),
            label: net.id.clone(),
            detail: format!("{} / {} conn", net.kind, net.connections.len()),
            rect: node_rect_from_position(rect, net.position, default, node_width, node_height),
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

    let mut edges = Vec::new();
    for component in snapshot.components_detail.iter().take(component_count) {
        let Some(start) = component_centers.get(component.id.as_str()).copied() else {
            continue;
        };
        for pin in &component.pins {
            if let Some(end) = net_centers.get(pin.net.as_str()).copied() {
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
            rect.bottom() - margin - node_height,
            node_width,
            node_height,
            snapshot.components_detail.len() - component_count,
            "more components",
        );
    }
    if snapshot.nets_detail.len() > net_count {
        push_overflow_hint(
            &mut nodes,
            right_x,
            rect.bottom() - margin - node_height,
            node_width,
            node_height,
            snapshot.nets_detail.len() - net_count,
            "more nets",
        );
    }

    SketchGraph { nodes, edges }
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

pub(super) fn draw_sketch_node(painter: &egui::Painter, node: &SketchNode, selected: bool) {
    let fill = match node.selection {
        SketchSelection::Component(_) => egui::Color32::from_rgb(36, 52, 70),
        SketchSelection::Net(_) => egui::Color32::from_rgb(42, 62, 46),
        SketchSelection::Overflow(_) => egui::Color32::from_gray(36),
    };
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
        SketchPosition, SketchSelection, add_component, add_net, assign_component_pin,
        edit_schematic_node_position, layout_sketch_graph, remove_component, remove_component_pin,
        remove_net, validate_board_ir_yaml_text,
    };
    use crate::gui::sketch::{ProjectSnapshot, SketchComponent, SketchNet, SketchPin};

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
}
