use eframe::egui;

#[cfg(test)]
pub(super) use super::kicad_symbol_library::KiCadSymbolPinAnchor;
use super::sketch::{
    ProjectSnapshot, SketchComponent, SketchEdge, SketchGraph, SketchNode, SketchNodeStyle,
    SketchPinSide, SketchPosition, SketchSelection, SketchViewport, SketchWireRouteEdit,
    wire_route_key,
};
pub(super) use super::sketch_layout_pins::compact_label;
#[cfg(test)]
pub(super) use super::sketch_layout_pins::component_pin_anchors_from_kicad;
use super::sketch_layout_pins::{
    component_node_size, component_pin_anchors, net_node_size, node_rect_from_position,
    push_overflow_hint,
};
use super::sketch_probes::{SketchProbeBadge, layout_probe_badges, probe_badge_interaction_rect};
use super::sketch_routes;
use super::sketch_symbols::{SketchSymbolKind, component_symbol_kind};

const MIN_SKETCH_GRID_STEP: f32 = 4.0;
const MAX_SKETCH_GRID_STEP: f32 = 96.0;
const MAX_SKETCH_ITEMS_PER_SIDE: usize = 512;
const MAX_SKETCH_EDGES: usize = 512;
const SCHEMATIC_SIGNAL_Y_FRACTION: f32 = 0.38;
const SCHEMATIC_GROUND_Y_FRACTION: f32 = 0.78;
const SCHEMATIC_COLUMN_STEP: f32 = 170.0;
const SCHEMATIC_LAYER_STEP: f32 = 156.0;

#[derive(Debug, Default)]
struct SchematicDefaultLayout {
    component_positions: std::collections::BTreeMap<String, egui::Pos2>,
    net_positions: std::collections::BTreeMap<String, egui::Pos2>,
}

#[derive(Debug, Default)]
struct SchematicFlowLayout {
    component_ranks: std::collections::BTreeMap<String, usize>,
    net_ranks: std::collections::BTreeMap<String, usize>,
    component_orders: std::collections::BTreeMap<String, usize>,
    net_orders: std::collections::BTreeMap<String, usize>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct SketchAutoLayoutPlan {
    pub(super) positions: Vec<(SketchSelection, f64, f64)>,
    pub(super) styles: Vec<(String, SketchNodeStyle)>,
    pub(super) wire_routes: Vec<SketchWireRouteEdit>,
    pub(super) probe_positions: Vec<(String, f64, f64)>,
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
            component.pins.len(),
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
            detail: schematic_component_detail(component),
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

    let probe_badges = layout_probe_badges(snapshot, rect, &nodes, &pin_anchors, &edges);

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

fn schematic_component_detail(component: &SketchComponent) -> String {
    if let Some(part_number) = component.part_number.as_deref().map(str::trim)
        && !part_number.is_empty()
    {
        return compact_label(part_number, 34);
    }
    if let Some(spice) = &component.spice {
        return compact_label(&schematic_spice_detail(spice), 34);
    }
    let model_leaf = component
        .model
        .rsplit('.')
        .next()
        .filter(|leaf| !leaf.is_empty())
        .unwrap_or(component.model.as_str());
    if component.pins.len() > 4 {
        compact_label(&format!("{model_leaf} / {} pins", component.pins.len()), 34)
    } else {
        compact_label(model_leaf, 34)
    }
}

fn schematic_spice_detail(spice: &super::sketch_spice::SketchComponentSpice) -> String {
    use super::sketch_spice::SketchSpiceKind;

    match spice.kind {
        SketchSpiceKind::Resistor => engineering_value(spice.value, "Ohm"),
        SketchSpiceKind::Capacitor => engineering_value(spice.value, "F"),
        SketchSpiceKind::Inductor => engineering_value(spice.value, "H"),
        SketchSpiceKind::DcVoltageSource => engineering_value(spice.value, "V"),
        SketchSpiceKind::DcCurrentSource => engineering_value(spice.value, "A"),
        SketchSpiceKind::PulseVoltageSource => format!(
            "pulse {}..{} V",
            compact_decimal(spice.pulse.initial),
            compact_decimal(spice.pulse.pulsed)
        ),
        SketchSpiceKind::PulseCurrentSource => format!(
            "pulse {}..{} A",
            compact_decimal(spice.pulse.initial),
            compact_decimal(spice.pulse.pulsed)
        ),
    }
}

fn engineering_value(value: f64, unit: &str) -> String {
    if !value.is_finite() {
        return format!("{value} {unit}");
    }
    if value == 0.0 {
        return format!("0 {unit}");
    }
    let abs = value.abs();
    let (factor, prefix) = [
        (1.0e9, "G"),
        (1.0e6, "M"),
        (1.0e3, "k"),
        (1.0, ""),
        (1.0e-3, "m"),
        (1.0e-6, "u"),
        (1.0e-9, "n"),
        (1.0e-12, "p"),
    ]
    .into_iter()
    .find(|(factor, _)| abs >= *factor)
    .unwrap_or((1.0e-12, "p"));
    format!("{} {prefix}{unit}", compact_decimal(value / factor))
}

fn compact_decimal(value: f64) -> String {
    let abs = value.abs();
    let precision = if abs >= 100.0 {
        0
    } else if abs >= 10.0 {
        1
    } else {
        2
    };
    let mut text = format!("{value:.precision$}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
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
    let probe_positions = classical_auto_layout_probe_positions(
        snapshot,
        &positions,
        &styles,
        &wire_routes,
        canvas,
        snap_enabled,
        grid_step,
    );

    SketchAutoLayoutPlan {
        positions,
        styles,
        wire_routes,
        probe_positions,
    }
}

pub(super) fn default_probe_element_position(
    snapshot: &ProjectSnapshot,
    element_id: &str,
    canvas_size: egui::Vec2,
    snap_enabled: bool,
    grid_step: f32,
) -> Option<(String, f64, f64)> {
    let size = egui::vec2(canvas_size.x.max(720.0), canvas_size.y.max(420.0));
    let canvas = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
    let graph = layout_sketch_graph(canvas, snapshot);
    let mut occupied = probe_lane_occupied_rects(&graph);
    for badge in &graph.probe_badges {
        if badge.probe.element_id.as_deref() != Some(element_id) {
            occupied.push(probe_badge_interaction_rect(badge).expand(8.0));
        }
    }
    let badge = graph
        .probe_badges
        .iter()
        .find(|badge| badge.probe.element_id.as_deref() == Some(element_id))?;
    place_probe_badge_in_lane(badge, &mut occupied, canvas, snap_enabled, grid_step)
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
        include_rect(probe_badge_interaction_rect(badge));
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
        badge.anchor = transform_viewport_pos(badge.anchor, canvas, viewport);
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
    let flow = schematic_flow_layout(snapshot, &net_kinds);

    let mut source_rows = std::collections::BTreeMap::<usize, usize>::new();
    let mut rank_rows = std::collections::BTreeMap::<usize, usize>::new();
    let mut signal_fallback_col = 0usize;
    let mut block_row = 0usize;
    for component in snapshot.components_detail.iter().take(component_count) {
        let symbol = component_symbol_kind(component);
        let size = component_node_size(
            symbol,
            component.kicad_symbol_id.is_some(),
            component.pins.len(),
            fallback_component_size,
        );
        let position = if is_source_component(component, symbol) {
            let rank = flow
                .component_ranks
                .get(&component.id)
                .copied()
                .unwrap_or(0);
            let x = schematic_rank_x(left, right, size.x, rank);
            let row = component_rank_row(&flow, &mut source_rows, &component.id, rank);
            let y = signal_y - size.y * 0.5 + row as f32 * (size.y + 12.0);
            egui::pos2(x, y)
        } else if is_ground_shunt_component(component, &net_kinds) {
            let rank = component_signal_rank(component, &flow, &net_kinds)
                .or_else(|| flow.component_ranks.get(&component.id).copied())
                .unwrap_or_else(|| {
                    signal_fallback_col += 1;
                    signal_fallback_col
                });
            let row = component_rank_row(&flow, &mut rank_rows, &component.id, rank);
            let x = schematic_rank_x(left, right, size.x, rank);
            egui::pos2(
                x,
                (signal_y + ground_y) * 0.5 - size.y * 0.5 + row as f32 * (size.y + 20.0),
            )
        } else if is_power_shunt_component(component, &net_kinds) {
            let rank = component_signal_rank(component, &flow, &net_kinds)
                .or_else(|| flow.component_ranks.get(&component.id).copied())
                .unwrap_or_else(|| {
                    signal_fallback_col += 1;
                    signal_fallback_col
                });
            let row = component_rank_row(&flow, &mut rank_rows, &component.id, rank);
            let x = schematic_rank_x(left, right, size.x, rank);
            egui::pos2(
                x,
                (top_rail_y + signal_y) * 0.5 - size.y * 0.5 - row as f32 * (size.y + 20.0),
            )
        } else if symbol.is_kicad_device_symbol() {
            let rank = flow
                .component_ranks
                .get(&component.id)
                .copied()
                .or_else(|| component_signal_rank(component, &flow, &net_kinds))
                .unwrap_or_else(|| {
                    signal_fallback_col += 1;
                    signal_fallback_col
                });
            let row = component_rank_row(&flow, &mut rank_rows, &component.id, rank);
            let x = schematic_rank_x(left, right, size.x, rank);
            egui::pos2(x, signal_y - size.y * 0.5 + row as f32 * (size.y + 12.0))
        } else if let Some(rank) = flow.component_ranks.get(&component.id).copied() {
            let row = component_rank_row(&flow, &mut rank_rows, &component.id, rank);
            let y = signal_y - size.y * 0.5 + row as f32 * (size.y + 16.0);
            let x = schematic_rank_x(left, right, size.x, rank);
            egui::pos2(x, y)
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
                component.pins.len(),
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
            let ranked_x = flow
                .net_ranks
                .get(&net.id)
                .map(|rank| schematic_rank_x(left, right, size.x, *rank))
                .unwrap_or(connected_x - size.x * 0.5);
            let lane = flow.net_orders.get(&net.id).copied().unwrap_or_else(|| {
                let lane = signal_index;
                signal_index += 1;
                lane
            });
            let x = ranked_x + lane as f32 * 10.0;
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

fn schematic_rank_x(left: f32, right: f32, width: f32, rank: usize) -> f32 {
    clamp_schematic_x(
        left + SCHEMATIC_LAYER_STEP * rank as f32,
        left,
        right - width,
    )
}

fn next_rank_row(rows: &mut std::collections::BTreeMap<usize, usize>, rank: usize) -> usize {
    let row = rows.entry(rank).or_insert(0);
    let value = *row;
    *row += 1;
    value
}

fn component_rank_row(
    flow: &SchematicFlowLayout,
    rows: &mut std::collections::BTreeMap<usize, usize>,
    component_id: &str,
    rank: usize,
) -> usize {
    if let Some(order) = flow.component_orders.get(component_id).copied() {
        let row = rows.entry(rank).or_insert(0);
        *row = (*row).max(order.saturating_add(1));
        order
    } else {
        next_rank_row(rows, rank)
    }
}

fn schematic_flow_layout(
    snapshot: &ProjectSnapshot,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
) -> SchematicFlowLayout {
    let component_by_id = snapshot
        .components_detail
        .iter()
        .map(|component| (component.id.as_str(), component))
        .collect::<std::collections::BTreeMap<_, _>>();
    let net_by_id = snapshot
        .nets_detail
        .iter()
        .map(|net| (net.id.as_str(), net))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut flow = SchematicFlowLayout::default();
    let mut queue = std::collections::VecDeque::<(String, usize)>::new();

    for component in &snapshot.components_detail {
        let symbol = component_symbol_kind(component);
        if !is_source_component(component, symbol) {
            continue;
        }
        insert_min_rank(&mut flow.component_ranks, component.id.clone(), 0);
        for pin in &component.pins {
            if is_power_or_ground_net(pin.net.as_str(), net_kinds) {
                continue;
            }
            if insert_min_rank(&mut flow.net_ranks, pin.net.clone(), 1) {
                queue.push_back((pin.net.clone(), 1));
            }
        }
    }

    if queue.is_empty()
        && snapshot
            .components_detail
            .iter()
            .any(|component| component_symbol_kind(component).is_kicad_device_symbol())
        && let Some(net) = snapshot
            .nets_detail
            .iter()
            .find(|net| !is_power_net(net) && !is_ground_net(net))
    {
        insert_min_rank(&mut flow.net_ranks, net.id.clone(), 1);
        queue.push_back((net.id.clone(), 1));
    }

    while let Some((net_id, net_rank)) = queue.pop_front() {
        let Some(net) = net_by_id.get(net_id.as_str()) else {
            continue;
        };
        for connection in &net.connections {
            let Some((component_id, _)) = connection.split_once('.') else {
                continue;
            };
            let Some(component) = component_by_id.get(component_id) else {
                continue;
            };
            let symbol = component_symbol_kind(component);
            let component_rank = if is_source_component(component, symbol) {
                0
            } else {
                net_rank.saturating_add(1)
            };
            insert_min_rank(
                &mut flow.component_ranks,
                component.id.clone(),
                component_rank,
            );
            for pin in &component.pins {
                if pin.net == net_id || is_power_or_ground_net(pin.net.as_str(), net_kinds) {
                    continue;
                }
                let next_rank = component_rank.saturating_add(1);
                if next_rank <= 24
                    && insert_min_rank(&mut flow.net_ranks, pin.net.clone(), next_rank)
                {
                    queue.push_back((pin.net.clone(), next_rank));
                }
            }
        }
    }

    assign_barycentric_lane_orders(snapshot, net_kinds, &mut flow);
    flow
}

fn assign_barycentric_lane_orders(
    snapshot: &ProjectSnapshot,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
    flow: &mut SchematicFlowLayout,
) {
    let component_index = snapshot
        .components_detail
        .iter()
        .enumerate()
        .map(|(index, component)| (component.id.as_str(), index))
        .collect::<std::collections::BTreeMap<_, _>>();
    let net_index = snapshot
        .nets_detail
        .iter()
        .enumerate()
        .map(|(index, net)| (net.id.as_str(), index))
        .collect::<std::collections::BTreeMap<_, _>>();
    flow.component_orders = initial_rank_orders(&flow.component_ranks, &component_index);
    flow.net_orders = initial_rank_orders(&flow.net_ranks, &net_index);
    for _ in 0..4 {
        flow.component_orders = barycentric_component_orders(
            snapshot,
            net_kinds,
            &flow.component_ranks,
            &flow.net_orders,
            &component_index,
        );
        flow.net_orders = barycentric_net_orders(
            snapshot,
            &flow.net_ranks,
            &flow.component_orders,
            &net_index,
        );
    }
}

fn initial_rank_orders(
    ranks: &std::collections::BTreeMap<String, usize>,
    original_index: &std::collections::BTreeMap<&str, usize>,
) -> std::collections::BTreeMap<String, usize> {
    let mut by_rank = std::collections::BTreeMap::<usize, Vec<&str>>::new();
    for (id, rank) in ranks {
        by_rank.entry(*rank).or_default().push(id.as_str());
    }
    let mut orders = std::collections::BTreeMap::new();
    for ids in by_rank.values_mut() {
        ids.sort_by_key(|id| original_index.get(id).copied().unwrap_or(usize::MAX));
        for (order, id) in ids.iter().enumerate() {
            orders.insert((*id).to_string(), order);
        }
    }
    orders
}

fn barycentric_component_orders(
    snapshot: &ProjectSnapshot,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
    ranks: &std::collections::BTreeMap<String, usize>,
    net_orders: &std::collections::BTreeMap<String, usize>,
    original_index: &std::collections::BTreeMap<&str, usize>,
) -> std::collections::BTreeMap<String, usize> {
    let mut by_rank = std::collections::BTreeMap::<usize, Vec<(&str, f64, usize)>>::new();
    for component in &snapshot.components_detail {
        let Some(rank) = ranks.get(&component.id).copied() else {
            continue;
        };
        let mut sum = 0.0;
        let mut count = 0usize;
        for pin in &component.pins {
            if is_power_or_ground_net(pin.net.as_str(), net_kinds) {
                continue;
            }
            if let Some(order) = net_orders.get(&pin.net) {
                sum += *order as f64;
                count += 1;
            }
        }
        let fallback = original_index
            .get(component.id.as_str())
            .copied()
            .unwrap_or(usize::MAX);
        let barycenter = if count == 0 {
            fallback as f64
        } else {
            sum / count as f64
        };
        by_rank
            .entry(rank)
            .or_default()
            .push((component.id.as_str(), barycenter, fallback));
    }
    rank_orders_from_barycenters(by_rank)
}

fn barycentric_net_orders(
    snapshot: &ProjectSnapshot,
    ranks: &std::collections::BTreeMap<String, usize>,
    component_orders: &std::collections::BTreeMap<String, usize>,
    original_index: &std::collections::BTreeMap<&str, usize>,
) -> std::collections::BTreeMap<String, usize> {
    let mut by_rank = std::collections::BTreeMap::<usize, Vec<(&str, f64, usize)>>::new();
    for net in &snapshot.nets_detail {
        let Some(rank) = ranks.get(&net.id).copied() else {
            continue;
        };
        let mut sum = 0.0;
        let mut count = 0usize;
        for connection in &net.connections {
            let Some((component_id, _)) = connection.split_once('.') else {
                continue;
            };
            if let Some(order) = component_orders.get(component_id) {
                sum += *order as f64;
                count += 1;
            }
        }
        let fallback = original_index
            .get(net.id.as_str())
            .copied()
            .unwrap_or(usize::MAX);
        let barycenter = if count == 0 {
            fallback as f64
        } else {
            sum / count as f64
        };
        by_rank
            .entry(rank)
            .or_default()
            .push((net.id.as_str(), barycenter, fallback));
    }
    rank_orders_from_barycenters(by_rank)
}

fn rank_orders_from_barycenters(
    mut by_rank: std::collections::BTreeMap<usize, Vec<(&str, f64, usize)>>,
) -> std::collections::BTreeMap<String, usize> {
    let mut orders = std::collections::BTreeMap::new();
    for ids in by_rank.values_mut() {
        ids.sort_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.0.cmp(right.0))
        });
        for (order, (id, _, _)) in ids.iter().enumerate() {
            orders.insert((*id).to_string(), order);
        }
    }
    orders
}

fn insert_min_rank(
    ranks: &mut std::collections::BTreeMap<String, usize>,
    id: String,
    rank: usize,
) -> bool {
    match ranks.get_mut(&id) {
        Some(existing) if rank < *existing => {
            *existing = rank;
            true
        }
        Some(_) => false,
        None => {
            ranks.insert(id, rank);
            true
        }
    }
}

fn component_signal_rank(
    component: &SketchComponent,
    flow: &SchematicFlowLayout,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
) -> Option<usize> {
    component
        .pins
        .iter()
        .filter(|pin| !is_power_or_ground_net(pin.net.as_str(), net_kinds))
        .filter_map(|pin| flow.net_ranks.get(&pin.net).copied())
        .min()
}

fn is_power_or_ground_net(
    net_id: &str,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
) -> bool {
    let kind = net_kinds.get(net_id).copied().unwrap_or("");
    is_ground_kind(kind)
        || net_id.eq_ignore_ascii_case("gnd")
        || net_id == "0"
        || kind.eq_ignore_ascii_case("power")
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

fn is_power_shunt_component(
    component: &SketchComponent,
    net_kinds: &std::collections::BTreeMap<&str, &str>,
) -> bool {
    if component.pins.len() != 2 {
        return false;
    }
    let power_pins = component
        .pins
        .iter()
        .filter(|pin| {
            net_kinds
                .get(pin.net.as_str())
                .is_some_and(|kind| kind.eq_ignore_ascii_case("power"))
        })
        .count();
    power_pins == 1
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

fn classical_auto_layout_probe_positions(
    snapshot: &ProjectSnapshot,
    positions: &[(SketchSelection, f64, f64)],
    styles: &[(String, SketchNodeStyle)],
    wire_routes: &[SketchWireRouteEdit],
    canvas: egui::Rect,
    snap_enabled: bool,
    grid_step: f32,
) -> Vec<(String, f64, f64)> {
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
    for (source, net_id, points) in wire_routes {
        planned.wire_routes.insert(
            wire_route_key(source, net_id),
            points
                .iter()
                .map(|(x, y)| SketchPosition { x: *x, y: *y })
                .collect(),
        );
    }
    for probe in &mut planned.probes {
        probe.position = None;
    }
    let graph = layout_sketch_graph(canvas, &planned);
    let mut occupied = probe_lane_occupied_rects(&graph);
    let mut edits = Vec::new();
    for badge in graph
        .probe_badges
        .iter()
        .filter(|badge| badge.probe.element_id.is_some())
    {
        if let Some(edit) =
            place_probe_badge_in_lane(badge, &mut occupied, canvas, snap_enabled, grid_step)
        {
            edits.push(edit);
        }
    }
    edits
}

fn probe_lane_occupied_rects(graph: &SketchGraph) -> Vec<egui::Rect> {
    graph
        .nodes
        .iter()
        .map(|node| node.rect.expand(12.0))
        .collect::<Vec<_>>()
}

fn place_probe_badge_in_lane(
    badge: &SketchProbeBadge,
    occupied: &mut Vec<egui::Rect>,
    canvas: egui::Rect,
    snap_enabled: bool,
    grid_step: f32,
) -> Option<(String, f64, f64)> {
    let element_id = badge.probe.element_id.clone()?;
    let mut rect = badge.rect;
    let mut interaction = probe_badge_interaction_rect(badge);
    let mut guard = 0;
    while occupied
        .iter()
        .any(|existing| existing.intersects(interaction))
        && guard < 18
    {
        let delta = egui::vec2(0.0, 38.0);
        rect = rect.translate(delta);
        let mut shifted = badge.clone();
        shifted.rect = rect;
        interaction = probe_badge_interaction_rect(&shifted);
        guard += 1;
    }
    occupied.push(interaction.expand(8.0));
    let (x, y) = snap_schematic_position(
        (rect.left() - canvas.left()) as f64,
        (rect.top() - canvas.top()) as f64,
        snap_enabled,
        grid_step,
    );
    Some((element_id, x, y))
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
#[cfg(test)]
#[path = "sketch_layout_tests.rs"]
mod sketch_layout_tests;
