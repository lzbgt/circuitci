use eframe::egui;

use super::sketch::{
    ProjectSnapshot, SketchComponent, SketchEdge, SketchGraph, SketchNode, SketchNodeStyle,
    SketchPinAnchor, SketchPinSide, SketchPosition, SketchSelection, SketchViewport,
    wire_route_key,
};
use super::sketch_probes::layout_probe_badges;
use super::sketch_routes;
use super::sketch_symbols::{SketchSymbolKind, component_symbol_kind};

const MIN_SKETCH_GRID_STEP: f32 = 4.0;
const MAX_SKETCH_GRID_STEP: f32 = 96.0;
const MAX_SKETCH_ITEMS_PER_SIDE: usize = 512;
const MAX_SKETCH_EDGES: usize = 512;

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
    let mut component_y = top;
    for component in snapshot.components_detail.iter().take(component_count) {
        let symbol = component_symbol_kind(component);
        let size = component_node_size(symbol, fallback_component_size);
        let default = egui::pos2(left_x, component_y);
        component_y += size.y + row_gap;
        nodes.push(SketchNode {
            selection: SketchSelection::Component(component.id.clone()),
            label: component.id.clone(),
            detail: compact_label(
                &format!("{} / {} pins", component.model, component.pins.len()),
                34,
            ),
            symbol,
            style: component.style,
            rect: node_rect_from_position(rect, component.position, default, size.x, size.y),
        });
    }

    let net_count = snapshot.nets_detail.len().min(MAX_SKETCH_ITEMS_PER_SIDE);
    let mut net_y = top;
    for net in snapshot.nets_detail.iter().take(net_count) {
        let size = net_node_size(fallback_net_size);
        let default = egui::pos2(right_x, net_y);
        net_y += size.y + row_gap;
        nodes.push(SketchNode {
            selection: SketchSelection::Net(net.id.clone()),
            label: net.id.clone(),
            detail: format!("{} / {} conn", net.kind, net.connections.len()),
            symbol: SketchSymbolKind::Net,
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

fn component_pin_anchors(
    component: &SketchComponent,
    rect: egui::Rect,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
) -> Vec<SketchPinAnchor> {
    let visible_count = component.pins.len().min(8);
    if visible_count == 0 {
        return Vec::new();
    }
    if component.pins.len() == 2 && component_symbol_kind(component).is_kicad_device_symbol() {
        return component
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
            .collect();
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

fn component_node_size(symbol: SketchSymbolKind, fallback: egui::Vec2) -> egui::Vec2 {
    if symbol.is_kicad_device_symbol() {
        egui::vec2(104.0, 72.0)
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
