use eframe::egui;

use super::kicad_symbol_library::{
    KiCadSymbolPinAnchor, kicad_default_symbol_pin_anchors, kicad_symbol_pin_anchors,
};
use super::sketch::{
    ProjectSnapshot, SketchComponent, SketchEdge, SketchGraph, SketchNode, SketchNodeStyle,
    SketchPinAnchor, SketchPinSide, SketchPosition, SketchSelection, SketchViewport,
    SketchWireRouteEdit, wire_route_key,
};
use super::sketch_probes::layout_probe_badges;
use super::sketch_routes;
use super::sketch_symbols::{SketchSymbolKind, component_symbol_kind, symbol_glyph_rect};

const MIN_SKETCH_GRID_STEP: f32 = 4.0;
const MAX_SKETCH_GRID_STEP: f32 = 96.0;
const MAX_SKETCH_ITEMS_PER_SIDE: usize = 512;
const MAX_SKETCH_EDGES: usize = 512;
const MAX_SKETCH_PIN_ANCHORS_PER_COMPONENT: usize = 64;
const SCHEMATIC_SIGNAL_Y_FRACTION: f32 = 0.38;
const SCHEMATIC_GROUND_Y_FRACTION: f32 = 0.78;
const SCHEMATIC_COLUMN_STEP: f32 = 150.0;

#[derive(Debug, Default)]
struct SchematicDefaultLayout {
    component_positions: std::collections::BTreeMap<String, egui::Pos2>,
    net_positions: std::collections::BTreeMap<String, egui::Pos2>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct SketchAutoLayoutPlan {
    pub(super) positions: Vec<(SketchSelection, f64, f64)>,
    pub(super) styles: Vec<(String, SketchNodeStyle)>,
    pub(super) wire_routes: Vec<SketchWireRouteEdit>,
}

pub(super) fn layout_sketch_graph(rect: egui::Rect, snapshot: &ProjectSnapshot) -> SketchGraph {
    let margin = 18.0;
    let node_width = ((rect.width() - 3.0 * margin) / 2.0).clamp(150.0, 260.0);
    let fallback_component_size = egui::vec2(node_width, 92.0);
    let fallback_net_size = egui::vec2(node_width, 44.0);
    let row_gap = 10.0;
    let left_x = rect.left() + margin;
    let right_x = rect.right() - margin - node_width;
    let top = rect.top() + margin;
    let mut nodes = Vec::new();
    let component_count = snapshot
        .components_detail
        .len()
        .min(MAX_SKETCH_ITEMS_PER_SIDE);
    let net_count = snapshot.nets_detail.len().min(MAX_SKETCH_ITEMS_PER_SIDE);
    let default_layout = schematic_default_layout(
        rect,
        snapshot,
        component_count,
        net_count,
        fallback_component_size,
        fallback_net_size,
    );
    let mut component_y = top;
    for component in snapshot.components_detail.iter().take(component_count) {
        let symbol = component_symbol_kind(component);
        let size = component_node_size(
            symbol,
            component.kicad_symbol_id.is_some(),
            fallback_component_size,
        );
        let default = default_layout
            .component_positions
            .get(&component.id)
            .copied()
            .unwrap_or_else(|| egui::pos2(left_x, component_y));
        component_y += size.y + row_gap;
        nodes.push(SketchNode {
            selection: SketchSelection::Component(component.id.clone()),
            label: component.id.clone(),
            detail: compact_label(
                &format!("{} / {} pins", component.model, component.pins.len()),
                34,
            ),
            symbol,
            kicad_symbol_id: component.kicad_symbol_id.clone(),
            style: component.style,
            rect: node_rect_from_position(rect, component.position, default, size.x, size.y),
        });
    }

    let mut net_y = top;
    for net in snapshot.nets_detail.iter().take(net_count) {
        let size = net_node_size(fallback_net_size);
        let default = default_layout
            .net_positions
            .get(&net.id)
            .copied()
            .unwrap_or_else(|| egui::pos2(right_x, net_y));
        net_y += size.y + row_gap;
        nodes.push(SketchNode {
            selection: SketchSelection::Net(net.id.clone()),
            label: net.id.clone(),
            detail: format!("{} / {} conn", net.kind, net.connections.len()),
            symbol: SketchSymbolKind::Net,
            kicad_symbol_id: None,
            style: SketchNodeStyle::default(),
            rect: node_rect_from_position(rect, net.position, default, size.x, size.y),
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
    let net_kinds = snapshot
        .nets_detail
        .iter()
        .map(|net| (net.id.as_str(), net.kind.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    for component in snapshot.components_detail.iter().take(component_count) {
        let Some(node) = nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Component(component.id.clone()))
        else {
            continue;
        };
        for anchor in component_pin_anchors(component, node.rect, &net_kinds) {
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
                    route: snapshot
                        .wire_routes
                        .get(&wire_route_key(
                            &format!("{}.{}", component.id, pin.pin),
                            &pin.net,
                        ))
                        .map(|points| {
                            points
                                .iter()
                                .map(|point| {
                                    egui::pos2(
                                        rect.left() + point.x as f32,
                                        rect.top() + point.y as f32,
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
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
            rect.bottom() - margin - fallback_component_size.y,
            fallback_component_size.x,
            fallback_component_size.y,
            snapshot.components_detail.len() - component_count,
            "more components",
        );
    }
    if snapshot.nets_detail.len() > net_count {
        push_overflow_hint(
            &mut nodes,
            right_x,
            rect.bottom() - margin - fallback_net_size.y,
            fallback_net_size.x,
            fallback_net_size.y,
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

pub(super) fn classical_sketch_auto_layout(
    snapshot: &ProjectSnapshot,
    canvas_size: egui::Vec2,
    snap_enabled: bool,
    grid_step: f32,
) -> SketchAutoLayoutPlan {
    let size = egui::vec2(canvas_size.x.max(720.0), canvas_size.y.max(420.0));
    let canvas = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
    let margin = 18.0;
    let node_width = ((canvas.width() - 3.0 * margin) / 2.0).clamp(150.0, 260.0);
    let fallback_component_size = egui::vec2(node_width, 92.0);
    let fallback_net_size = egui::vec2(node_width, 44.0);
    let component_count = snapshot
        .components_detail
        .len()
        .min(MAX_SKETCH_ITEMS_PER_SIDE);
    let net_count = snapshot.nets_detail.len().min(MAX_SKETCH_ITEMS_PER_SIDE);
    let default_layout = schematic_default_layout(
        canvas,
        snapshot,
        component_count,
        net_count,
        fallback_component_size,
        fallback_net_size,
    );

    let mut positions = Vec::with_capacity(component_count + net_count);
    for component in snapshot.components_detail.iter().take(component_count) {
        if let Some(pos) = default_layout.component_positions.get(&component.id) {
            let (x, y) = snap_schematic_position(
                (pos.x - canvas.left()) as f64,
                (pos.y - canvas.top()) as f64,
                snap_enabled,
                grid_step,
            );
            positions.push((SketchSelection::Component(component.id.clone()), x, y));
        }
    }
    for net in snapshot.nets_detail.iter().take(net_count) {
        if let Some(pos) = default_layout.net_positions.get(&net.id) {
            let (x, y) = snap_schematic_position(
                (pos.x - canvas.left()) as f64,
                (pos.y - canvas.top()) as f64,
                snap_enabled,
                grid_step,
            );
            positions.push((SketchSelection::Net(net.id.clone()), x, y));
        }
    }

    let net_kinds = snapshot
        .nets_detail
        .iter()
        .map(|net| (net.id.as_str(), net.kind.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let styles: Vec<(String, SketchNodeStyle)> = snapshot
        .components_detail
        .iter()
        .take(component_count)
        .filter_map(|component| {
            let style = classical_component_style(component, &net_kinds)?;
            (component.style != style).then(|| (component.id.clone(), style))
        })
        .collect();

    let wire_routes = classical_auto_layout_wire_routes(
        snapshot,
        &positions,
        &styles,
        canvas,
        snap_enabled,
        grid_step,
    );

    SketchAutoLayoutPlan {
        positions,
        styles,
        wire_routes,
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
        let points = sketch_wire_points(edge);
        for segment in points.windows(2) {
            include_rect(egui::Rect::from_two_pos(segment[0], segment[1]));
        }
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

pub(super) fn persisted_wire_route_point_from_screen_with_snap(
    canvas: egui::Rect,
    screen_position: egui::Pos2,
    viewport: SketchViewport,
    snap_enabled: bool,
    grid_step: f32,
) -> (f64, f64) {
    let snapped =
        snap_screen_point_to_grid(canvas, screen_position, viewport, snap_enabled, grid_step);
    let logical = inverse_viewport_pos(snapped, canvas, viewport);
    (
        (logical.x - canvas.left()) as f64,
        (logical.y - canvas.top()) as f64,
    )
}

pub(super) fn screen_wire_route_point_from_persisted(
    canvas: egui::Rect,
    point: (f64, f64),
    viewport: SketchViewport,
) -> egui::Pos2 {
    transform_viewport_pos(
        egui::pos2(
            canvas.left() + point.0 as f32,
            canvas.top() + point.1 as f32,
        ),
        canvas,
        viewport,
    )
}

#[cfg(test)]
pub(super) fn orthogonal_wire_points(start: egui::Pos2, end: egui::Pos2) -> Vec<egui::Pos2> {
    sketch_routes::orthogonal_points(start, end)
}

pub(super) fn sketch_wire_points(edge: &SketchEdge) -> Vec<egui::Pos2> {
    sketch_routes::wire_points(edge.start, &edge.route, edge.end)
}

pub(super) fn edge_label_position(edge: &SketchEdge) -> egui::Pos2 {
    let points = sketch_wire_points(edge);
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
            let distance = distance_to_polyline(position, &sketch_wire_points(edge));
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
        for point in &mut edge.route {
            *point = transform_viewport_pos(*point, canvas, viewport);
        }
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

fn schematic_default_layout(
    rect: egui::Rect,
    snapshot: &ProjectSnapshot,
    component_count: usize,
    net_count: usize,
    fallback_component_size: egui::Vec2,
    fallback_net_size: egui::Vec2,
) -> SchematicDefaultLayout {
    let mut layout = SchematicDefaultLayout::default();
    let margin = 24.0;
    let left = rect.left() + margin;
    let right = rect.right() - margin;
    let top_rail_y = rect.top() + margin;
    let signal_y = rect.top() + rect.height() * SCHEMATIC_SIGNAL_Y_FRACTION;
    let ground_y = rect.top() + rect.height() * SCHEMATIC_GROUND_Y_FRACTION;
    let net_kinds = snapshot
        .nets_detail
        .iter()
        .map(|net| (net.id.as_str(), net.kind.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut source_row = 0usize;
    let mut series_col = 0usize;
    let mut shunt_col = 0usize;
    let mut block_row = 0usize;
    for component in snapshot.components_detail.iter().take(component_count) {
        let symbol = component_symbol_kind(component);
        let size = component_node_size(
            symbol,
            component.kicad_symbol_id.is_some(),
            fallback_component_size,
        );
        let position = if is_source_component(component, symbol) {
            let y = signal_y - size.y * 0.5 + source_row as f32 * (size.y + 12.0);
            source_row += 1;
            egui::pos2(left, y)
        } else if is_ground_shunt_component(component, &net_kinds) {
            let x = left + SCHEMATIC_COLUMN_STEP * (shunt_col as f32 + 1.8);
            shunt_col += 1;
            egui::pos2(
                x.min(right - size.x),
                (signal_y + ground_y) * 0.5 - size.y * 0.5,
            )
        } else if symbol.is_kicad_device_symbol() {
            let x = left + SCHEMATIC_COLUMN_STEP * (series_col as f32 + 1.5);
            series_col += 1;
            egui::pos2(x.min(right - size.x), signal_y - size.y * 0.5)
        } else {
            let y = signal_y - size.y * 0.5 + block_row as f32 * (size.y + 16.0);
            let x = left + SCHEMATIC_COLUMN_STEP * 1.5;
            block_row += 1;
            egui::pos2(x.min(right - size.x), y)
        };
        layout
            .component_positions
            .insert(component.id.clone(), position);
    }

    let component_centers = layout
        .component_positions
        .iter()
        .filter_map(|(id, min)| {
            let component = snapshot
                .components_detail
                .iter()
                .find(|component| component.id == *id)?;
            let size = component_node_size(
                component_symbol_kind(component),
                component.kicad_symbol_id.is_some(),
                fallback_component_size,
            );
            Some((id.as_str(), *min + size * 0.5))
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut power_index = 0usize;
    let mut signal_index = 0usize;
    let mut ground_index = 0usize;
    for net in snapshot.nets_detail.iter().take(net_count) {
        let size = net_node_size(fallback_net_size);
        let connected_x = connected_component_average_x(net, &component_centers)
            .unwrap_or(left + SCHEMATIC_COLUMN_STEP * (signal_index as f32 + 1.5));
        let position = if is_power_net(net) {
            let x = connected_x + power_index as f32 * 18.0 - size.x * 0.5;
            power_index += 1;
            egui::pos2(clamp_schematic_x(x, left, right - size.x), top_rail_y)
        } else if is_ground_net(net) {
            let x = connected_x + ground_index as f32 * 18.0 - size.x * 0.5;
            ground_index += 1;
            egui::pos2(clamp_schematic_x(x, left, right - size.x), ground_y)
        } else {
            let x = connected_x + signal_index as f32 * 10.0 - size.x * 0.5;
            signal_index += 1;
            egui::pos2(
                clamp_schematic_x(x, left, right - size.x),
                (signal_y - size.y - 26.0).max(top_rail_y + 42.0),
            )
        };
        layout.net_positions.insert(net.id.clone(), position);
    }

    layout
}

fn clamp_schematic_x(x: f32, left: f32, right: f32) -> f32 {
    if right < left {
        left
    } else {
        x.clamp(left, right)
    }
}

fn is_source_component(component: &SketchComponent, symbol: SketchSymbolKind) -> bool {
    let model = component.model.to_ascii_lowercase();
    let id_prefix = component
        .id
        .chars()
        .next()
        .map(|value| value.to_ascii_uppercase());
    symbol == SketchSymbolKind::Source
        || model.contains("source")
        || matches!(id_prefix, Some('V') | Some('I'))
}

fn is_ground_shunt_component(
    component: &SketchComponent,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
) -> bool {
    if component.pins.len() != 2 {
        return false;
    }
    let ground_pins = component
        .pins
        .iter()
        .filter(|pin| {
            net_kinds
                .get(pin.net.as_str())
                .is_some_and(|kind| is_ground_kind(kind))
        })
        .count();
    ground_pins == 1
}

fn classical_component_style(
    component: &SketchComponent,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
) -> Option<SketchNodeStyle> {
    let symbol = component_symbol_kind(component);
    if !symbol.is_kicad_device_symbol()
        || component.pins.len() != 2
        || is_source_component(component, symbol)
    {
        return None;
    }
    let rotation_deg = if is_ground_shunt_component(component, net_kinds) {
        90
    } else {
        0
    };
    Some(SketchNodeStyle {
        rotation_deg,
        mirrored: false,
        pin_side: SketchPinSide::Auto,
    })
}

fn classical_auto_layout_wire_routes(
    snapshot: &ProjectSnapshot,
    positions: &[(SketchSelection, f64, f64)],
    styles: &[(String, SketchNodeStyle)],
    canvas: egui::Rect,
    snap_enabled: bool,
    grid_step: f32,
) -> Vec<SketchWireRouteEdit> {
    let mut planned = snapshot.clone();
    for (selection, x, y) in positions {
        match selection {
            SketchSelection::Component(id) => {
                if let Some(component) = planned
                    .components_detail
                    .iter_mut()
                    .find(|component| component.id == *id)
                {
                    component.position = Some(SketchPosition { x: *x, y: *y });
                }
            }
            SketchSelection::Net(id) => {
                if let Some(net) = planned.nets_detail.iter_mut().find(|net| net.id == *id) {
                    net.position = Some(SketchPosition { x: *x, y: *y });
                }
            }
            SketchSelection::Overflow(_) => {}
        }
    }
    for (component_id, style) in styles {
        if let Some(component) = planned
            .components_detail
            .iter_mut()
            .find(|component| component.id == *component_id)
        {
            component.style = *style;
        }
    }
    planned.wire_routes.clear();
    let graph = layout_sketch_graph(canvas, &planned);
    let net_kinds = planned
        .nets_detail
        .iter()
        .map(|net| (net.id.as_str(), net.kind.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    graph
        .edges
        .iter()
        .filter_map(|edge| {
            let kind = net_kinds.get(edge.net_id.as_str()).copied().unwrap_or("");
            let waypoint = classical_route_waypoint(edge.start, edge.end, kind)?;
            let (x, y) = snap_schematic_position(
                (waypoint.x - canvas.left()) as f64,
                (waypoint.y - canvas.top()) as f64,
                snap_enabled,
                grid_step,
            );
            Some((edge.source.clone(), edge.net_id.clone(), vec![(x, y)]))
        })
        .collect()
}

fn classical_route_waypoint(
    start: egui::Pos2,
    end: egui::Pos2,
    net_kind: &str,
) -> Option<egui::Pos2> {
    if (start.x - end.x).abs() <= 0.5 || (start.y - end.y).abs() <= 0.5 {
        return None;
    }
    let waypoint = if is_ground_kind(net_kind) || net_kind.eq_ignore_ascii_case("power") {
        egui::pos2(start.x, end.y)
    } else {
        egui::pos2(end.x, start.y)
    };
    if waypoint.distance_sq(start) <= 0.25 || waypoint.distance_sq(end) <= 0.25 {
        None
    } else {
        Some(waypoint)
    }
}

fn connected_component_average_x(
    net: &super::sketch::SketchNet,
    component_centers: &std::collections::BTreeMap<&str, egui::Pos2>,
) -> Option<f32> {
    let mut sum = 0.0;
    let mut count = 0usize;
    for connection in &net.connections {
        let Some((component_id, _)) = connection.split_once('.') else {
            continue;
        };
        if let Some(center) = component_centers.get(component_id) {
            sum += center.x;
            count += 1;
        }
    }
    (count > 0).then_some(sum / count as f32)
}

fn is_power_net(net: &super::sketch::SketchNet) -> bool {
    net.powered.unwrap_or(false) || net.kind.eq_ignore_ascii_case("power")
}

fn is_ground_net(net: &super::sketch::SketchNet) -> bool {
    is_ground_kind(&net.kind) || net.id.eq_ignore_ascii_case("gnd") || net.id == "0"
}

fn is_ground_kind(kind: &str) -> bool {
    kind.eq_ignore_ascii_case("ground")
}

fn component_pin_anchors(
    component: &SketchComponent,
    rect: egui::Rect,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
) -> Vec<SketchPinAnchor> {
    let visible_count = component
        .pins
        .len()
        .min(MAX_SKETCH_PIN_ANCHORS_PER_COMPONENT);
    if visible_count == 0 {
        return Vec::new();
    }
    let symbol = component_symbol_kind(component);
    if let Some(glyph_rect) = symbol_glyph_rect(rect, symbol, component.kicad_symbol_id.is_some()) {
        let kicad_anchors = component
            .kicad_symbol_id
            .as_deref()
            .map(|symbol_id| kicad_symbol_pin_anchors(symbol_id, glyph_rect, component.style))
            .unwrap_or_else(|| {
                kicad_default_symbol_pin_anchors(symbol, glyph_rect, component.style)
            });
        let anchors =
            component_pin_anchors_from_kicad(component, &kicad_anchors, net_kinds, visible_count);
        if !anchors.is_empty() {
            return anchors;
        }
    }
    if component.pins.len() == 2 && symbol.is_kicad_device_symbol() {
        return two_terminal_component_pin_anchors(component, rect, net_kinds);
    }
    generic_component_pin_anchors(component, rect, net_kinds, visible_count)
}

fn component_pin_anchors_from_kicad(
    component: &SketchComponent,
    kicad_anchors: &[KiCadSymbolPinAnchor],
    net_kinds: &std::collections::BTreeMap<&str, &str>,
    visible_count: usize,
) -> Vec<SketchPinAnchor> {
    component
        .pins
        .iter()
        .take(visible_count)
        .filter_map(|pin| {
            let anchor = kicad_anchors.iter().find(|anchor| anchor.pin == pin.pin)?;
            Some(SketchPinAnchor {
                component_id: component.id.clone(),
                pin: pin.pin.clone(),
                net: pin.net.clone(),
                kind: net_kinds
                    .get(pin.net.as_str())
                    .copied()
                    .unwrap_or("unresolved")
                    .to_string(),
                pos: anchor.pos,
                label_pos: anchor.label_pos,
                label_align: anchor.label_align,
            })
        })
        .collect()
}

fn two_terminal_component_pin_anchors(
    component: &SketchComponent,
    rect: egui::Rect,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
) -> Vec<SketchPinAnchor> {
    component
        .pins
        .iter()
        .enumerate()
        .map(|(index, pin)| {
            let (pos, label_pos, label_align) =
                two_terminal_pin_anchor(rect, index, component.style);
            SketchPinAnchor {
                component_id: component.id.clone(),
                pin: pin.pin.clone(),
                net: pin.net.clone(),
                kind: net_kinds
                    .get(pin.net.as_str())
                    .copied()
                    .unwrap_or("unresolved")
                    .to_string(),
                pos,
                label_pos,
                label_align,
            }
        })
        .collect()
}

fn generic_component_pin_anchors(
    component: &SketchComponent,
    rect: egui::Rect,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
    visible_count: usize,
) -> Vec<SketchPinAnchor> {
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
                kind: net_kinds
                    .get(pin.net.as_str())
                    .copied()
                    .unwrap_or("unresolved")
                    .to_string(),
                pos: egui::pos2(x, y),
                label_pos: egui::pos2(x + label_offset, y),
                label_align,
            }
        })
        .collect()
}

fn component_node_size(
    symbol: SketchSymbolKind,
    has_explicit_kicad_symbol: bool,
    fallback: egui::Vec2,
) -> egui::Vec2 {
    if symbol.is_kicad_device_symbol() {
        egui::vec2(104.0, 72.0)
    } else if has_explicit_kicad_symbol {
        egui::vec2(fallback.x.min(180.0), fallback.y.max(96.0))
    } else {
        fallback
    }
}

fn net_node_size(fallback: egui::Vec2) -> egui::Vec2 {
    egui::vec2(fallback.x.min(150.0), 32.0)
}

fn two_terminal_pin_anchor(
    rect: egui::Rect,
    index: usize,
    style: SketchNodeStyle,
) -> (egui::Pos2, egui::Pos2, egui::Align2) {
    let x = if index == 0 { -1.0 } else { 1.0 };
    let terminal = styled_normalized_point(rect, x, 0.0, style);
    let outward = terminal - rect.center();
    let outward = if outward.length_sq() > 0.0 {
        outward.normalized()
    } else {
        egui::vec2(if index == 0 { -1.0 } else { 1.0 }, 0.0)
    };
    let label_pos = terminal + outward * 10.0;
    let label_align = if outward.x.abs() >= outward.y.abs() {
        if outward.x < 0.0 {
            egui::Align2::RIGHT_CENTER
        } else {
            egui::Align2::LEFT_CENTER
        }
    } else if outward.y < 0.0 {
        egui::Align2::CENTER_BOTTOM
    } else {
        egui::Align2::CENTER_TOP
    };
    (terminal, label_pos, label_align)
}

fn styled_normalized_point(rect: egui::Rect, x: f32, y: f32, style: SketchNodeStyle) -> egui::Pos2 {
    let x = if style.mirrored { -x } else { x };
    let (x, y) = match style.rotation_deg.rem_euclid(360) {
        90 => (-y, x),
        180 => (-x, -y),
        270 => (y, -x),
        _ => (x, y),
    };
    egui::pos2(
        rect.center().x + x * rect.width() * 0.5,
        rect.center().y + y * rect.height() * 0.5,
    )
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
        kicad_symbol_id: None,
        style: SketchNodeStyle::default(),
        rect: egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height)),
    });
}

pub(super) fn compact_label(value: &str, max_chars: usize) -> String {
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
    use super::*;
    use crate::gui::sketch::{ProjectSnapshot, SketchNet, SketchPin};

    fn test_component() -> SketchComponent {
        SketchComponent {
            id: "U1".to_string(),
            model: "generic.schematic.imported_component".to_string(),
            part_number: None,
            spice: None,
            pins: vec![
                SketchPin {
                    pin: "1".to_string(),
                    net: "in".to_string(),
                },
                SketchPin {
                    pin: "2".to_string(),
                    net: "out".to_string(),
                },
            ],
            position: None,
            style: SketchNodeStyle::default(),
            kicad_symbol_id: Some("Device:R".to_string()),
            source_paths: Vec::new(),
        }
    }

    fn layout_test_snapshot() -> ProjectSnapshot {
        ProjectSnapshot {
            name: "classical_layout".to_string(),
            components: 3,
            nets: 3,
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![
                SketchComponent {
                    id: "V1".to_string(),
                    model: "generic.analog.voltage_source".to_string(),
                    part_number: None,
                    spice: None,
                    pins: vec![
                        SketchPin {
                            pin: "P".to_string(),
                            net: "vcc".to_string(),
                        },
                        SketchPin {
                            pin: "N".to_string(),
                            net: "gnd".to_string(),
                        },
                    ],
                    position: None,
                    style: SketchNodeStyle::default(),
                    kicad_symbol_id: None,
                    source_paths: Vec::new(),
                },
                SketchComponent {
                    id: "R1".to_string(),
                    model: "generic.analog.resistor".to_string(),
                    part_number: None,
                    spice: None,
                    pins: vec![
                        SketchPin {
                            pin: "A".to_string(),
                            net: "vcc".to_string(),
                        },
                        SketchPin {
                            pin: "B".to_string(),
                            net: "sig".to_string(),
                        },
                    ],
                    position: None,
                    style: SketchNodeStyle::default(),
                    kicad_symbol_id: None,
                    source_paths: Vec::new(),
                },
                SketchComponent {
                    id: "C1".to_string(),
                    model: "generic.analog.capacitor".to_string(),
                    part_number: None,
                    spice: None,
                    pins: vec![
                        SketchPin {
                            pin: "A".to_string(),
                            net: "sig".to_string(),
                        },
                        SketchPin {
                            pin: "B".to_string(),
                            net: "gnd".to_string(),
                        },
                    ],
                    position: None,
                    style: SketchNodeStyle::default(),
                    kicad_symbol_id: None,
                    source_paths: Vec::new(),
                },
            ],
            nets_detail: vec![
                SketchNet {
                    id: "vcc".to_string(),
                    kind: "power".to_string(),
                    nominal_voltage: Some(5.0),
                    powered: Some(true),
                    connections: vec!["V1.P".to_string(), "R1.A".to_string()],
                    position: None,
                },
                SketchNet {
                    id: "sig".to_string(),
                    kind: "digital_or_analog".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: vec!["R1.B".to_string(), "C1.A".to_string()],
                    position: None,
                },
                SketchNet {
                    id: "gnd".to_string(),
                    kind: "ground".to_string(),
                    nominal_voltage: Some(0.0),
                    powered: None,
                    connections: vec!["V1.N".to_string(), "C1.B".to_string()],
                    position: None,
                },
            ],
            probes: Vec::new(),
            wire_routes: Default::default(),
            net_labels: Default::default(),
            component_labels: Default::default(),
        }
    }

    #[test]
    fn default_layout_uses_classical_power_signal_ground_roles() {
        let graph = layout_sketch_graph(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(720.0, 420.0)),
            &layout_test_snapshot(),
        );
        let source = graph
            .nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Component("V1".to_string()))
            .unwrap();
        let resistor = graph
            .nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Component("R1".to_string()))
            .unwrap();
        let power = graph
            .nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Net("vcc".to_string()))
            .unwrap();
        let signal = graph
            .nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Net("sig".to_string()))
            .unwrap();
        let ground = graph
            .nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Net("gnd".to_string()))
            .unwrap();

        assert!(source.rect.center().x < resistor.rect.center().x);
        assert!(power.rect.center().y < signal.rect.center().y);
        assert!(signal.rect.center().y < ground.rect.center().y);
    }

    #[test]
    fn classical_auto_layout_persists_positions_and_vertical_shunts() {
        let plan = classical_sketch_auto_layout(
            &layout_test_snapshot(),
            egui::vec2(720.0, 420.0),
            true,
            16.0,
        );

        assert_eq!(plan.positions.len(), 6);
        assert!(
            plan.positions
                .iter()
                .any(|(selection, _, _)| *selection == SketchSelection::Component("V1".to_string()))
        );
        assert!(
            plan.positions
                .iter()
                .any(|(selection, _, _)| *selection == SketchSelection::Net("gnd".to_string()))
        );
        assert!(plan.positions.iter().all(|(_, x, y)| {
            *x >= 0.0
                && *y >= 0.0
                && (x % 16.0).abs() <= f64::EPSILON
                && (y % 16.0).abs() <= f64::EPSILON
        }));
        assert_eq!(
            plan.styles,
            vec![(
                "C1".to_string(),
                SketchNodeStyle {
                    rotation_deg: 90,
                    mirrored: false,
                    pin_side: SketchPinSide::Auto,
                }
            )]
        );
        assert_eq!(plan.wire_routes.len(), 6);
        assert!(plan.wire_routes.iter().any(|(source, net_id, points)| {
            source == "C1.B" && net_id == "gnd" && points.len() == 1
        }));
        assert!(
            plan.wire_routes
                .iter()
                .all(|(_, _, points)| points.iter().all(|(x, y)| *x >= 0.0 && *y >= 0.0))
        );
    }

    #[test]
    fn component_pin_anchors_use_matching_kicad_pin_geometry() {
        let kicad = vec![
            KiCadSymbolPinAnchor {
                pin: "1".to_string(),
                pos: egui::pos2(20.0, 40.0),
                label_pos: egui::pos2(10.0, 40.0),
                label_align: egui::Align2::RIGHT_CENTER,
            },
            KiCadSymbolPinAnchor {
                pin: "2".to_string(),
                pos: egui::pos2(80.0, 40.0),
                label_pos: egui::pos2(90.0, 40.0),
                label_align: egui::Align2::LEFT_CENTER,
            },
        ];
        let net_kinds = [("in", "analog"), ("out", "analog")]
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();

        let anchors = component_pin_anchors_from_kicad(&test_component(), &kicad, &net_kinds, 2);

        assert_eq!(anchors.len(), 2);
        assert_eq!(anchors[0].pin, "1");
        assert_eq!(anchors[0].pos, egui::pos2(20.0, 40.0));
        assert_eq!(anchors[0].label_align, egui::Align2::RIGHT_CENTER);
        assert_eq!(anchors[1].pin, "2");
        assert_eq!(anchors[1].pos, egui::pos2(80.0, 40.0));
    }

    #[test]
    fn component_pin_anchors_ignore_unmatched_kicad_pins() {
        let kicad = vec![KiCadSymbolPinAnchor {
            pin: "3".to_string(),
            pos: egui::pos2(20.0, 40.0),
            label_pos: egui::pos2(10.0, 40.0),
            label_align: egui::Align2::RIGHT_CENTER,
        }];
        let net_kinds = std::collections::BTreeMap::new();

        let anchors = component_pin_anchors_from_kicad(&test_component(), &kicad, &net_kinds, 2);

        assert!(anchors.is_empty());
    }
}
