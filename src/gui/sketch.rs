use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

use super::sketch_net_labels;
use super::sketch_probes::{SketchProbe, SketchProbeBadge, derive_project_probes};
pub(super) use super::sketch_render::{
    draw_sketch_node, draw_sketch_pin_anchor, runtime_scope_chip_rect, with_opacity,
};
use super::sketch_spice::SketchComponentSpice;
use super::sketch_symbols::SketchSymbolKind;

#[cfg(test)]
pub(super) use super::sketch_layout::orthogonal_wire_points;
pub(super) use super::sketch_layout::{
    classical_sketch_auto_layout, compact_label, draw_sketch_grid, edge_label_position,
    hit_test_wire, layout_sketch_graph, layout_sketch_graph_viewport,
    persisted_node_position_from_screen, persisted_node_position_from_screen_with_snap,
    persisted_wire_route_point_from_screen_with_snap, screen_wire_route_point_from_persisted,
    sketch_graph_bounds, sketch_wire_points, snap_screen_point_to_grid,
};

pub(super) const DEFAULT_SKETCH_GRID_STEP: f32 = 16.0;

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
    pub(super) wire_routes: std::collections::BTreeMap<String, Vec<SketchPosition>>,
    pub(super) net_labels: Vec<SketchNetLabelPlacement>,
    pub(super) component_labels: std::collections::BTreeMap<String, SketchComponentLabelPositions>,
}

#[derive(Debug, Clone)]
pub(super) struct SketchComponent {
    pub(super) id: String,
    pub(super) model: String,
    pub(super) part_number: Option<String>,
    pub(super) spice: Option<SketchComponentSpice>,
    pub(super) pins: Vec<SketchPin>,
    pub(super) position: Option<SketchPosition>,
    pub(super) style: SketchNodeStyle,
    pub(super) kicad_symbol_id: Option<String>,
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

#[derive(Debug, Clone)]
pub(super) struct SketchNetLabelPlacement {
    pub(super) id: String,
    pub(super) net_id: String,
    pub(super) kind: SketchNetLabelKind,
    pub(super) position: SketchPosition,
}

#[derive(Debug, Clone, Default)]
pub(super) struct SketchComponentLabelPositions {
    pub(super) reference: Option<SketchPosition>,
    pub(super) value: Option<SketchPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SketchNetLabelKind {
    Local,
    OffPage,
}

impl SketchNetLabelKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::OffPage => "off_page",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Local => "Net Label",
            Self::OffPage => "Off-Page Connector",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub(super) kicad_symbol_id: Option<String>,
    pub(super) style: SketchNodeStyle,
    pub(super) rect: egui::Rect,
}

#[derive(Debug)]
pub(super) struct SketchPinAnchor {
    pub(super) component_id: String,
    pub(super) pin: String,
    pub(super) net: String,
    pub(super) kind: String,
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
    pub(super) route: Vec<egui::Pos2>,
}

pub(super) type SketchWireRouteEdit = (String, String, Vec<(f64, f64)>);

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
    let component_symbols = &project.board.schematic.component_symbols;
    let component_labels = &project.board.schematic.component_labels;
    let wire_routes = &project.board.schematic.wire_routes;
    let net_labels = &project.board.schematic.net_labels;
    let probes = derive_project_probes(&project);
    let components_detail: Vec<_> = project
        .board
        .components
        .iter()
        .map(|(id, component)| SketchComponent {
            id: id.clone(),
            model: component.model.clone(),
            part_number: component.part_number.clone(),
            spice: component
                .spice
                .as_ref()
                .map(SketchComponentSpice::from_board),
            position: sketch_position_for(positions, &SketchSelection::Component(id.clone())),
            style: sketch_style_for(styles, &SketchSelection::Component(id.clone())),
            kicad_symbol_id: component_symbols.get(id).cloned(),
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
        wire_routes: wire_routes
            .iter()
            .map(|(key, route)| {
                (
                    key.clone(),
                    route
                        .points
                        .iter()
                        .map(|point| SketchPosition {
                            x: point.x,
                            y: point.y,
                        })
                        .collect(),
                )
            })
            .collect(),
        net_labels: net_labels
            .iter()
            .filter(|(_, label)| project.board.nets.contains_key(&label.net))
            .map(|(id, label)| SketchNetLabelPlacement {
                id: id.clone(),
                net_id: label.net.clone(),
                kind: match label.kind {
                    crate::board_ir::SchematicNetLabelKind::OffPage => SketchNetLabelKind::OffPage,
                    crate::board_ir::SchematicNetLabelKind::Local => SketchNetLabelKind::Local,
                },
                position: SketchPosition {
                    x: label.x,
                    y: label.y,
                },
            })
            .collect(),
        component_labels: component_labels
            .iter()
            .filter(|(component_id, _)| project.board.components.contains_key(*component_id))
            .map(|(component_id, labels)| {
                (
                    component_id.clone(),
                    SketchComponentLabelPositions {
                        reference: labels.reference.as_ref().map(|position| SketchPosition {
                            x: position.x,
                            y: position.y,
                        }),
                        value: labels.value.as_ref().map(|position| SketchPosition {
                            x: position.x,
                            y: position.y,
                        }),
                    },
                )
            })
            .collect(),
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

pub(super) fn edit_schematic_wire_route(
    text: &str,
    source: &str,
    net_id: &str,
    points: &[(f64, f64)],
) -> Result<String> {
    edit_schematic_wire_routes(
        text,
        &[(source.to_string(), net_id.to_string(), points.to_vec())],
    )
}

pub(super) fn edit_schematic_wire_routes(
    text: &str,
    route_edits: &[SketchWireRouteEdit],
) -> Result<String> {
    if route_edits.is_empty() {
        return Ok(text.to_string());
    }
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let mut validated_routes = Vec::with_capacity(route_edits.len());
    for (source, net_id, points) in route_edits {
        let (component_id, pin_id) = parse_wire_source(source)?;
        let net_id = validated_graph_id(net_id, "net")?.to_string();
        if points.is_empty() {
            anyhow::bail!("At least one schematic wire route point is required.");
        }
        for (x, y) in points {
            if !x.is_finite() || !y.is_finite() {
                anyhow::bail!("Schematic wire route points must be finite.");
            }
        }
        if !project.board.nets.contains_key(&net_id) {
            anyhow::bail!("Board IR net {net_id} was not found.");
        }
        let pin_net = component_pin_net(&project, component_id, pin_id)?
            .with_context(|| format!("Board IR pin {source} is not connected to a net."))?;
        if pin_net != net_id {
            anyhow::bail!("Board IR pin {source} is connected to {pin_net}, not {net_id}.");
        }
        validated_routes.push((source.clone(), net_id, points.clone()));
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
        let routes = ensure_child_mapping_mut(schematic, "wire_routes", "schematic wire routes")?;
        for (source, net_id, points) in validated_routes {
            let mut route = serde_yaml_ng::Mapping::new();
            route.insert(
                serde_yaml_ng::Value::String("points".to_string()),
                serde_yaml_ng::Value::Sequence(
                    points
                        .iter()
                        .map(|(x, y)| -> Result<serde_yaml_ng::Value> {
                            let mut point = serde_yaml_ng::Mapping::new();
                            point.insert(
                                serde_yaml_ng::Value::String("x".to_string()),
                                serde_yaml_ng::to_value(*x)?,
                            );
                            point.insert(
                                serde_yaml_ng::Value::String("y".to_string()),
                                serde_yaml_ng::to_value(*y)?,
                            );
                            Ok(serde_yaml_ng::Value::Mapping(point))
                        })
                        .collect::<Result<Vec<_>>>()?,
                ),
            );
            routes.insert(
                serde_yaml_ng::Value::String(wire_route_key(&source, &net_id)),
                serde_yaml_ng::Value::Mapping(route),
            );
        }
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn remove_schematic_wire_route(
    text: &str,
    source: &str,
    net_id: &str,
) -> Result<String> {
    let (component_id, pin_id) = parse_wire_source(source)?;
    let net_id = validated_graph_id(net_id, "net")?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.nets.contains_key(net_id) {
        anyhow::bail!("Board IR net {net_id} was not found.");
    }
    let pin_net = component_pin_net(&project, component_id, pin_id)?
        .with_context(|| format!("Board IR pin {source} is not connected to a net."))?;
    if pin_net != net_id {
        anyhow::bail!("Board IR pin {source} is connected to {pin_net}, not {net_id}.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    if let Some(routes) = yaml
        .as_mapping_mut()
        .and_then(|project| project.get_mut(serde_yaml_ng::Value::String("board".to_string())))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|board| board.get_mut(serde_yaml_ng::Value::String("schematic".to_string())))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|schematic| {
            schematic.get_mut(serde_yaml_ng::Value::String("wire_routes".to_string()))
        })
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
    {
        routes.remove(serde_yaml_ng::Value::String(wire_route_key(source, net_id)));
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn edit_schematic_component_style(
    text: &str,
    component_id: &str,
    style: SketchNodeStyle,
) -> Result<String> {
    edit_schematic_component_styles(text, &[(component_id.to_string(), style)])
}

pub(super) fn edit_schematic_component_styles(
    text: &str,
    styles: &[(String, SketchNodeStyle)],
) -> Result<String> {
    if styles.is_empty() {
        return Ok(text.to_string());
    }
    let component_styles: Vec<(String, SketchNodeStyle)> = styles
        .iter()
        .map(|(component_id, style)| {
            Ok((
                validated_graph_id(component_id, "component")?.to_string(),
                *style,
            ))
        })
        .collect::<Result<_>>()?;
    for (_, style) in &component_styles {
        validate_schematic_node_style(*style)?;
    }
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    for (component_id, _) in &component_styles {
        if !project.board.components.contains_key(component_id) {
            anyhow::bail!("Board IR component {component_id} was not found.");
        }
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
        for (component_id, style) in component_styles {
            styles.insert(
                serde_yaml_ng::Value::String(format!("component:{component_id}")),
                serde_yaml_ng::Value::Mapping(schematic_node_style_mapping(style)?),
            );
        }
    }
    encode_edited_project_yaml(yaml)
}

fn validate_schematic_node_style(style: SketchNodeStyle) -> Result<()> {
    if !matches!(style.rotation_deg, 0 | 90 | 180 | 270) {
        anyhow::bail!("Schematic node rotation must be 0, 90, 180, or 270 degrees.");
    }
    Ok(())
}

pub(super) fn schematic_node_style_mapping(
    style: SketchNodeStyle,
) -> Result<serde_yaml_ng::Mapping> {
    validate_schematic_node_style(style)?;
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
    Ok(node_style)
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
    sketch_net_labels::remove_net_labels_for_net(&mut yaml, net_id);
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

pub(super) fn board_child_mapping_mut<'a>(
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

pub(super) fn ensure_board_child_mapping_mut<'a>(
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

pub(super) fn ensure_child_mapping_mut<'a>(
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

fn parse_wire_source(source: &str) -> Result<(&str, &str)> {
    let (component_id, pin_id) = source
        .trim()
        .split_once('.')
        .with_context(|| format!("Schematic wire source {source} must be component.pin."))?;
    Ok((
        validated_graph_id(component_id, "source component")?,
        validated_graph_id(pin_id, "source pin")?,
    ))
}

pub(super) fn wire_route_key(source: &str, net_id: &str) -> String {
    format!("{}->{}", source.trim(), net_id.trim())
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

pub(super) fn encode_edited_project_yaml(yaml: serde_yaml_ng::Value) -> Result<String> {
    let text =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    validate_board_ir_yaml_text(&text)?;
    Ok(text)
}

pub(super) fn validated_graph_id<'a>(id: &'a str, label: &str) -> Result<&'a str> {
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

pub(super) fn normalized_net_kind(kind: &str) -> Result<&str> {
    match kind {
        "power" | "ground" | "digital_or_analog" => Ok(kind),
        _ => anyhow::bail!("Unsupported net kind {kind}."),
    }
}

pub(super) fn schematic_node_key(selection: &SketchSelection) -> Option<String> {
    match selection {
        SketchSelection::Component(id) => Some(format!("component:{id}")),
        SketchSelection::Net(id) => Some(format!("net:{id}")),
        SketchSelection::Overflow(_) => None,
    }
}
