use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

use super::sketch_probes::{
    SketchProbe, SketchProbeBadge, derive_project_probes, layout_probe_badges,
};
use super::sketch_symbols::{SketchSymbolKind, component_symbol_kind, draw_symbol_glyph};

pub(super) const DEFAULT_SKETCH_GRID_STEP: f32 = 16.0;
const MIN_SKETCH_GRID_STEP: f32 = 4.0;
const MAX_SKETCH_GRID_STEP: f32 = 96.0;
const MAX_SKETCH_ITEMS_PER_SIDE: usize = 512;
const MAX_SKETCH_EDGES: usize = 512;

#[derive(Debug, Clone)]
pub(super) struct ProjectSnapshot {
    pub(super) name: String,
    pub(super) components: usize,
    pub(super) nets: usize,
    pub(super) scenarios: usize,
    pub(super) libraries: Vec<String>,
    pub(super) components_detail: Vec<SketchComponent>,
    pub(super) nets_detail: Vec<SketchNet>,
    pub(super) probes: Vec<SketchProbe>,
}

#[derive(Debug, Clone)]
pub(super) struct SketchComponent {
    pub(super) id: String,
    pub(super) model: String,
    pub(super) part_number: Option<String>,
    pub(super) pins: Vec<SketchPin>,
    pub(super) position: Option<SketchPosition>,
    pub(super) style: SketchNodeStyle,
    pub(super) source_paths: Vec<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SketchNodeStyle {
    pub(super) rotation_deg: i32,
    pub(super) mirrored: bool,
    pub(super) pin_side: SketchPinSide,
}

impl Default for SketchNodeStyle {
    fn default() -> Self {
        Self {
            rotation_deg: 0,
            mirrored: false,
            pin_side: SketchPinSide::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SketchPinSide {
    Auto,
    Left,
    Right,
}

impl SketchPinSide {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
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
    pub(super) probe_badges: Vec<SketchProbeBadge>,
}

#[derive(Debug)]
pub(super) struct SketchNode {
    pub(super) selection: SketchSelection,
    pub(super) label: String,
    pub(super) detail: String,
    pub(super) symbol: SketchSymbolKind,
    pub(super) style: SketchNodeStyle,
    pub(super) rect: egui::Rect,
}

#[derive(Debug)]
pub(super) struct SketchPinAnchor {
    pub(super) component_id: String,
    pub(super) pin: String,
    pub(super) net: String,
    pub(super) pos: egui::Pos2,
    pub(super) label_pos: egui::Pos2,
    pub(super) label_align: egui::Align2,
}

#[derive(Debug)]
pub(super) struct SketchEdge {
    pub(super) net_id: String,
    pub(super) source: String,
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
    let positions = &project.board.schematic.node_positions;
    let styles = &project.board.schematic.node_styles;
    let probes = derive_project_probes(&project);
    let components_detail: Vec<_> = project
        .board
        .components
        .iter()
        .map(|(id, component)| SketchComponent {
            id: id.clone(),
            model: component.model.clone(),
            part_number: component.part_number.clone(),
            position: sketch_position_for(positions, &SketchSelection::Component(id.clone())),
            style: sketch_style_for(styles, &SketchSelection::Component(id.clone())),
            source_paths: component
                .source
                .as_ref()
                .map(|source| {
                    source
                        .instances
                        .iter()
                        .map(|instance| instance.path.clone())
                        .collect()
                })
                .unwrap_or_default(),
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
            position: sketch_position_for(positions, &SketchSelection::Net(id.clone())),
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
        probes,
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

fn sketch_style_for(
    styles: &std::collections::BTreeMap<String, crate::board_ir::SchematicNodeStyle>,
    selection: &SketchSelection,
) -> SketchNodeStyle {
    let Some(style) = schematic_node_key(selection).and_then(|key| styles.get(&key)) else {
        return SketchNodeStyle::default();
    };
    SketchNodeStyle {
        rotation_deg: normalize_rotation_deg(style.rotation_deg.unwrap_or(0)),
        mirrored: style.mirrored.unwrap_or(false),
        pin_side: match style.pin_side {
            Some(crate::board_ir::SchematicPinSide::Left) => SketchPinSide::Left,
            Some(crate::board_ir::SchematicPinSide::Right) => SketchPinSide::Right,
            Some(crate::board_ir::SchematicPinSide::Auto) | None => SketchPinSide::Auto,
        },
    }
}

fn normalize_rotation_deg(rotation_deg: i32) -> i32 {
    rotation_deg.rem_euclid(360) / 90 * 90
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
    edit_schematic_node_positions(text, &[(selection.clone(), x, y)])
}

pub(super) fn edit_schematic_node_positions(
    text: &str,
    positions_to_set: &[(SketchSelection, f64, f64)],
) -> Result<String> {
    if positions_to_set.is_empty() {
        anyhow::bail!("At least one schematic node position is required.");
    }
    for (_, x, y) in positions_to_set {
        if !x.is_finite() || !y.is_finite() {
            anyhow::bail!("Schematic node position must be finite.");
        }
    }
    let keyed_positions = positions_to_set
        .iter()
        .map(|(selection, x, y)| {
            let key = schematic_node_key(selection).with_context(
                || "Only component and net nodes can be positioned on the schematic canvas.",
            )?;
            Ok((key, *x, *y))
        })
        .collect::<Result<Vec<_>>>()?;

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
        for (key, x, y) in keyed_positions {
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
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn edit_schematic_component_style(
    text: &str,
    component_id: &str,
    style: SketchNodeStyle,
) -> Result<String> {
    let component_id = validated_graph_id(component_id, "component")?;
    if !matches!(style.rotation_deg, 0 | 90 | 180 | 270) {
        anyhow::bail!("Schematic node rotation must be 0, 90, 180, or 270 degrees.");
    }
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.components.contains_key(component_id) {
        anyhow::bail!("Board IR component {component_id} was not found.");
    }

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
        let styles = ensure_child_mapping_mut(schematic, "node_styles", "schematic node styles")?;
        let mut node_style = serde_yaml_ng::Mapping::new();
        node_style.insert(
            serde_yaml_ng::Value::String("rotation_deg".to_string()),
            serde_yaml_ng::to_value(style.rotation_deg)?,
        );
        node_style.insert(
            serde_yaml_ng::Value::String("mirrored".to_string()),
            serde_yaml_ng::to_value(style.mirrored)?,
        );
        node_style.insert(
            serde_yaml_ng::Value::String("pin_side".to_string()),
            serde_yaml_ng::Value::String(style.pin_side.as_str().to_string()),
        );
        styles.insert(
            serde_yaml_ng::Value::String(format!("component:{component_id}")),
            serde_yaml_ng::Value::Mapping(node_style),
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
    let mut nodes = Vec::new();
    let component_count = snapshot
        .components_detail
        .len()
        .min(MAX_SKETCH_ITEMS_PER_SIDE);
    for (index, component) in snapshot
        .components_detail
        .iter()
        .take(component_count)
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
            symbol: component_symbol_kind(component),
            style: component.style,
            rect: node_rect_from_position(
                rect,
                component.position,
                default,
                node_width,
                component_height,
            ),
        });
    }

    let net_count = snapshot.nets_detail.len().min(MAX_SKETCH_ITEMS_PER_SIDE);
    for (index, net) in snapshot.nets_detail.iter().take(net_count).enumerate() {
        let default = egui::pos2(right_x, top + index as f32 * (net_height + row_gap));
        nodes.push(SketchNode {
            selection: SketchSelection::Net(net.id.clone()),
            label: net.id.clone(),
            detail: format!("{} / {} conn", net.kind, net.connections.len()),
            symbol: SketchSymbolKind::Net,
            style: SketchNodeStyle::default(),
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
                edges.push(SketchEdge {
                    net_id: pin.net.clone(),
                    source: format!("{}.{}", component.id, pin.pin),
                    start,
                    end,
                });
                if edges.len() >= MAX_SKETCH_EDGES {
                    break;
                }
            }
        }
        if edges.len() >= MAX_SKETCH_EDGES {
            break;
        }
    }

    let probe_badges = layout_probe_badges(snapshot, &nodes);

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
        probe_badges,
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
    for badge in &graph.probe_badges {
        include_rect(badge.rect);
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

pub(super) fn persisted_node_position_from_screen_with_snap(
    canvas: egui::Rect,
    screen_position: egui::Pos2,
    screen_node_rect: egui::Rect,
    viewport: SketchViewport,
    snap_enabled: bool,
    grid_step: f32,
) -> (f64, f64) {
    let (x, y) =
        persisted_node_position_from_screen(canvas, screen_position, screen_node_rect, viewport);
    snap_schematic_position(x, y, snap_enabled, grid_step)
}

pub(super) fn snap_schematic_position(
    x: f64,
    y: f64,
    snap_enabled: bool,
    grid_step: f32,
) -> (f64, f64) {
    if !snap_enabled {
        return (x, y);
    }
    let step = normalized_grid_step(grid_step) as f64;
    ((x / step).round() * step, (y / step).round() * step)
}

pub(super) fn snap_screen_point_to_grid(
    canvas: egui::Rect,
    screen_position: egui::Pos2,
    viewport: SketchViewport,
    snap_enabled: bool,
    grid_step: f32,
) -> egui::Pos2 {
    if !snap_enabled {
        return screen_position;
    }
    let logical = inverse_viewport_pos(screen_position, canvas, viewport);
    let step = normalized_grid_step(grid_step);
    let snapped = egui::pos2(
        canvas.left() + ((logical.x - canvas.left()) / step).round() * step,
        canvas.top() + ((logical.y - canvas.top()) / step).round() * step,
    );
    transform_viewport_pos(snapped, canvas, viewport)
}

pub(super) fn orthogonal_wire_points(start: egui::Pos2, end: egui::Pos2) -> Vec<egui::Pos2> {
    if (start.x - end.x).abs() <= 0.5 || (start.y - end.y).abs() <= 0.5 {
        return vec![start, end];
    }
    let mid_x = (start.x + end.x) / 2.0;
    vec![
        start,
        egui::pos2(mid_x, start.y),
        egui::pos2(mid_x, end.y),
        end,
    ]
}

pub(super) fn edge_label_position(edge: &SketchEdge) -> egui::Pos2 {
    let points = orthogonal_wire_points(edge.start, edge.end);
    let total = polyline_length(&points);
    if total <= f32::EPSILON {
        return edge.start;
    }
    let mut remaining = total / 2.0;
    for segment in points.windows(2) {
        let start = segment[0];
        let end = segment[1];
        let length = start.distance(end);
        if remaining <= length {
            let t = if length <= f32::EPSILON {
                0.0
            } else {
                remaining / length
            };
            return start + (end - start) * t + egui::vec2(6.0, -6.0);
        }
        remaining -= length;
    }
    edge.end
}

pub(super) fn hit_test_wire(graph: &SketchGraph, position: egui::Pos2) -> Option<&SketchEdge> {
    graph
        .edges
        .iter()
        .filter_map(|edge| {
            let distance =
                distance_to_polyline(position, &orthogonal_wire_points(edge.start, edge.end));
            (distance <= 6.0).then_some((distance, edge))
        })
        .min_by(|(left, _), (right, _)| left.total_cmp(right))
        .map(|(_, edge)| edge)
}

pub(super) fn draw_sketch_grid(
    painter: &egui::Painter,
    canvas: egui::Rect,
    viewport: SketchViewport,
    grid_enabled: bool,
    grid_step: f32,
) {
    if !grid_enabled {
        return;
    }
    let step = normalized_grid_step(grid_step);
    let screen_step = step * viewport.zoom.clamp(0.25, 4.0);
    if screen_step < 6.0 {
        return;
    }
    let visible_min = inverse_viewport_pos(canvas.min, canvas, viewport);
    let visible_max = inverse_viewport_pos(canvas.max, canvas, viewport);
    let color = egui::Color32::from_gray(34);
    let stroke = egui::Stroke::new(1.0, color);

    let mut x = (visible_min.x / step).floor() * step;
    while x <= visible_max.x {
        let top = transform_viewport_pos(egui::pos2(x, visible_min.y), canvas, viewport);
        let bottom = transform_viewport_pos(egui::pos2(x, visible_max.y), canvas, viewport);
        painter.line_segment([top, bottom], stroke);
        x += step;
    }

    let mut y = (visible_min.y / step).floor() * step;
    while y <= visible_max.y {
        let left = transform_viewport_pos(egui::pos2(visible_min.x, y), canvas, viewport);
        let right = transform_viewport_pos(egui::pos2(visible_max.x, y), canvas, viewport);
        painter.line_segment([left, right], stroke);
        y += step;
    }
}

fn normalized_grid_step(grid_step: f32) -> f32 {
    grid_step.clamp(MIN_SKETCH_GRID_STEP, MAX_SKETCH_GRID_STEP)
}

fn polyline_length(points: &[egui::Pos2]) -> f32 {
    points
        .windows(2)
        .map(|segment| segment[0].distance(segment[1]))
        .sum()
}

fn distance_to_polyline(position: egui::Pos2, points: &[egui::Pos2]) -> f32 {
    points
        .windows(2)
        .map(|segment| distance_to_segment(position, segment[0], segment[1]))
        .fold(f32::INFINITY, f32::min)
}

fn distance_to_segment(position: egui::Pos2, start: egui::Pos2, end: egui::Pos2) -> f32 {
    let segment = end - start;
    let length_sq = segment.length_sq();
    if length_sq <= f32::EPSILON {
        return position.distance(start);
    }
    let t = ((position - start).dot(segment) / length_sq).clamp(0.0, 1.0);
    let closest = start + segment * t;
    position.distance(closest)
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
    for badge in &mut graph.probe_badges {
        badge.rect = transform_viewport_rect(badge.rect, canvas, viewport);
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
    let pin_side = component_pin_side(component.style);
    component
        .pins
        .iter()
        .take(visible_count)
        .enumerate()
        .map(|(index, pin)| {
            let y = pin_anchor_y(rect, index, visible_count);
            let x = match pin_side {
                SketchPinSide::Left => rect.left(),
                SketchPinSide::Auto | SketchPinSide::Right => rect.right(),
            };
            let label_offset = match pin_side {
                SketchPinSide::Left => 8.0,
                SketchPinSide::Auto | SketchPinSide::Right => -8.0,
            };
            let label_align = match pin_side {
                SketchPinSide::Left => egui::Align2::LEFT_CENTER,
                SketchPinSide::Auto | SketchPinSide::Right => egui::Align2::RIGHT_CENTER,
            };
            SketchPinAnchor {
                component_id: component.id.clone(),
                pin: pin.pin.clone(),
                net: pin.net.clone(),
                pos: egui::pos2(x, y),
                label_pos: egui::pos2(x + label_offset, y),
                label_align,
            }
        })
        .collect()
}

fn component_pin_side(style: SketchNodeStyle) -> SketchPinSide {
    match style.pin_side {
        SketchPinSide::Left | SketchPinSide::Right => style.pin_side,
        SketchPinSide::Auto if style.mirrored => SketchPinSide::Left,
        SketchPinSide::Auto => SketchPinSide::Right,
    }
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
    egui::Rect::from_min_size(min, egui::vec2(width, height))
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
        symbol: SketchSymbolKind::Overflow,
        style: SketchNodeStyle::default(),
        rect: egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height)),
    });
}

pub(super) fn draw_sketch_node(
    painter: &egui::Painter,
    node: &SketchNode,
    selected: bool,
    runtime_activity: Option<f64>,
    opacity: f32,
) {
    let opacity = normalized_opacity(opacity);
    let base_fill = match node.selection {
        SketchSelection::Component(_) => egui::Color32::from_rgb(36, 52, 70),
        SketchSelection::Net(_) => egui::Color32::from_rgb(42, 62, 46),
        SketchSelection::Overflow(_) => egui::Color32::from_gray(36),
    };
    let fill = runtime_activity
        .map(|activity| runtime_activity_fill(base_fill, activity))
        .unwrap_or(base_fill);
    let stroke = if selected {
        egui::Stroke::new(
            2.0,
            with_opacity(egui::Color32::from_rgb(93, 185, 255), opacity),
        )
    } else {
        egui::Stroke::new(1.0, with_opacity(egui::Color32::from_gray(108), opacity))
    };
    painter.rect_filled(node.rect, 4.0, with_opacity(fill, opacity));
    painter.rect_stroke(node.rect, 4.0, stroke, egui::StrokeKind::Inside);
    draw_symbol_glyph(painter, node);
    if opacity < 0.999 {
        let alpha = ((1.0 - opacity) * 170.0).round() as u8;
        painter.rect_filled(
            node.rect.shrink(1.0),
            4.0,
            egui::Color32::from_rgba_unmultiplied(18, 18, 18, alpha),
        );
    }
    painter.text(
        node.rect.left_top() + egui::vec2(8.0, 9.0),
        egui::Align2::LEFT_CENTER,
        compact_label(&node.label, 24),
        egui::FontId::monospace(13.0),
        with_opacity(egui::Color32::WHITE, opacity),
    );
    painter.text(
        node.rect.left_bottom() + egui::vec2(8.0, -12.0),
        egui::Align2::LEFT_CENTER,
        compact_label(&node.detail, 34),
        egui::FontId::monospace(11.0),
        with_opacity(egui::Color32::LIGHT_GRAY, opacity),
    );
}

pub(super) fn draw_sketch_pin_anchor(
    painter: &egui::Painter,
    anchor: &SketchPinAnchor,
    active: bool,
    opacity: f32,
) {
    let opacity = normalized_opacity(opacity);
    let fill = if active {
        egui::Color32::from_rgb(255, 196, 87)
    } else {
        egui::Color32::from_rgb(115, 166, 224)
    };
    painter.circle_filled(anchor.pos, 4.0, with_opacity(fill, opacity));
    painter.circle_stroke(
        anchor.pos,
        4.0,
        egui::Stroke::new(1.0, with_opacity(egui::Color32::from_gray(18), opacity)),
    );
    painter.text(
        anchor.label_pos,
        anchor.label_align,
        compact_label(&anchor.pin, 10),
        egui::FontId::monospace(10.5),
        with_opacity(egui::Color32::LIGHT_GRAY, opacity),
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

pub(super) fn with_opacity(color: egui::Color32, opacity: f32) -> egui::Color32 {
    let opacity = normalized_opacity(opacity);
    egui::Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        ((color.a() as f32) * opacity).round() as u8,
    )
}

fn normalized_opacity(opacity: f32) -> f32 {
    opacity.clamp(0.0, 1.0)
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
