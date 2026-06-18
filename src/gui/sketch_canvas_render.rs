use eframe::egui;

use super::sketch::{
    self, ProjectSnapshot, SketchNodeStyle, SketchPinSide, SketchSelection, edge_label_position,
    sketch_wire_points, with_opacity,
};
use super::sketch_canvas_interaction::WireDragTarget;
use super::sketch_probes::{SketchProbeBadge, SketchProbeStatus};
use super::sketch_routes;

pub(super) fn sketch_hover_tooltip(
    ui: &mut egui::Ui,
    node: &sketch::SketchNode,
    runtime_lines: &[String],
) {
    ui.strong(&node.label);
    ui.label(&node.detail);
    ui.separator();
    ui.label("Runtime probes");
    if runtime_lines.is_empty() {
        ui.label("No matching waveform probe is loaded for this node.");
    } else {
        for line in runtime_lines {
            ui.monospace(line);
        }
    }
}

pub(super) fn sketch_pin_hover_tooltip(ui: &mut egui::Ui, anchor: &sketch::SketchPinAnchor) {
    ui.strong(format!("{}.{}", anchor.component_id, anchor.pin));
    ui.label(format!("net: {}", anchor.net));
    ui.label(format!("net kind: {}", anchor.kind));
    ui.separator();
    ui.label("Click or drag this pin, then release on another pin, net, wire, or net label.");
}

pub(super) fn sketch_wire_hover_tooltip(ui: &mut egui::Ui, edge: &sketch::SketchEdge) {
    ui.strong(format!("net {}", edge.net_id));
    ui.label(format!("source: {}", edge.source));
    ui.separator();
    ui.label("Click this wire to select the net; drag it to shape the schematic route.");
    ui.label("Start wire mode first to connect another pin to it.");
    if !edge.route.is_empty() {
        ui.label("Right-click to insert route handles or clear the custom schematic route.");
    } else {
        ui.label("Right-click to insert a route handle at the pointer.");
    }
}

pub(super) fn sketch_wire_route_handle_tooltip(
    ui: &mut egui::Ui,
    edge: &sketch::SketchEdge,
    index: usize,
) {
    ui.strong(format!("wire route handle {}", index + 1));
    ui.label(format!("net: {}", edge.net_id));
    ui.label(format!("source: {}", edge.source));
    ui.separator();
    ui.label("Drag this handle to refine the schematic route.");
    ui.label("Right-click to delete this handle or clear the custom route.");
}

pub(super) fn sketch_probe_badge_tooltip(
    ui: &mut egui::Ui,
    badge: &SketchProbeBadge,
    status: SketchProbeStatus,
    sampled_value: Option<f64>,
) {
    ui.strong(format!(
        "{} probe {}",
        badge.probe.quantity.label(),
        badge.probe.probe_name
    ));
    ui.label(format!("scenario: {}", badge.probe.scenario_name));
    ui.label(format!("expression: {}", badge.probe.expression));
    ui.label(format!("assertion status: {}", status.label()));
    if let Some(value) = sampled_value {
        ui.label(format!("cursor sample: {:.6}", value));
    } else {
        ui.label("cursor sample: no matching loaded waveform");
    }
    if !badge.probe.assertion_names.is_empty() {
        ui.label(format!(
            "assertions: {}",
            badge.probe.assertion_names.join(", ")
        ));
    }
    ui.separator();
    ui.label("Click to open this probe in the Simulation stage.");
    ui.label("Right-click to open probe actions.");
    ui.label("Press A while hovering to add an assertion from current settings.");
    ui.label("Press Shift+A while hovering to require above the cursor sample.");
    ui.label("Press Shift+B while hovering to require below the cursor sample.");
    ui.label("Press X while hovering to clear assertions for this probe.");
    ui.label("Press Delete or Backspace while hovering to remove it.");
}

pub(super) fn draw_wire_edge(
    painter: &egui::Painter,
    edge: &sketch::SketchEdge,
    selected: bool,
    hovered: bool,
    zoom: f32,
    opacity: f32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let color = if selected {
        egui::Color32::from_rgb(93, 185, 255)
    } else if hovered {
        egui::Color32::from_rgb(255, 196, 87)
    } else {
        egui::Color32::from_gray(86)
    };
    let stroke_width = if selected || hovered { 2.0 } else { 1.0 };
    let color = with_opacity(color, opacity);
    let stroke = egui::Stroke::new(stroke_width, color);
    let points = sketch_wire_points(edge);
    draw_wire_points(painter, &points, stroke);
    draw_wire_junctions(painter, &points, color, selected || hovered);
    if zoom > 0.45 || selected || hovered {
        draw_wire_label(painter, edge, selected || hovered, opacity);
    }
}

pub(super) fn draw_wire_route_handles(
    painter: &egui::Painter,
    edge: &sketch::SketchEdge,
    hovered: Option<(&sketch::SketchEdge, usize)>,
    opacity: f32,
) {
    if edge.route.is_empty() {
        return;
    }
    let opacity = opacity.clamp(0.0, 1.0);
    for (index, point) in edge.route.iter().enumerate() {
        let hovered = hovered.is_some_and(|(hovered_edge, hovered_index)| {
            hovered_index == index
                && hovered_edge.net_id == edge.net_id
                && hovered_edge.source == edge.source
        });
        let fill = if hovered {
            egui::Color32::from_rgb(255, 196, 87)
        } else {
            egui::Color32::from_rgb(32, 126, 223)
        };
        painter.circle_filled(
            *point,
            if hovered { 5.5 } else { 4.5 },
            with_opacity(fill, opacity),
        );
        painter.circle_stroke(
            *point,
            if hovered { 7.5 } else { 6.5 },
            egui::Stroke::new(1.5, with_opacity(egui::Color32::WHITE, opacity)),
        );
    }
}

pub(super) fn draw_wire_route_preview(
    painter: &egui::Painter,
    edge: &sketch::SketchEdge,
    route_points: &[egui::Pos2],
) {
    let points = sketch_routes::wire_points(edge.start, route_points, edge.end);
    draw_wire_points(
        painter,
        &points,
        egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 196, 87)),
    );
    draw_wire_junctions(
        painter,
        &points,
        egui::Color32::from_rgb(255, 196, 87),
        true,
    );
}

pub(super) fn draw_pending_wire_route_handles(
    painter: &egui::Painter,
    route_points: &[egui::Pos2],
) {
    for point in route_points {
        painter.circle_filled(*point, 4.5, egui::Color32::from_rgb(255, 196, 87));
        painter.circle_stroke(
            *point,
            6.5,
            egui::Stroke::new(1.5, egui::Color32::from_rgb(36, 36, 36)),
        );
    }
}

pub(super) fn draw_wire_drag_target(painter: &egui::Painter, target: &WireDragTarget) {
    let color = egui::Color32::from_rgb(255, 196, 87);
    match target {
        WireDragTarget::Pin { pos, .. } => {
            painter.circle_filled(
                *pos,
                7.5,
                egui::Color32::from_rgba_unmultiplied(255, 196, 87, 44),
            );
            painter.circle_stroke(*pos, 9.0, egui::Stroke::new(2.0, color));
        }
        WireDragTarget::NetNode { rect, .. } => {
            let rect = rect.expand(5.0);
            painter.rect_filled(
                rect,
                4.0,
                egui::Color32::from_rgba_unmultiplied(255, 196, 87, 24),
            );
            painter.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(2.0, color),
                egui::StrokeKind::Inside,
            );
        }
        WireDragTarget::NetLabel { rect, .. } => {
            let rect = rect.expand(4.0);
            painter.rect_filled(
                rect,
                4.0,
                egui::Color32::from_rgba_unmultiplied(255, 196, 87, 28),
            );
            painter.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(2.0, color),
                egui::StrokeKind::Inside,
            );
        }
        WireDragTarget::Wire { snap, .. } => {
            painter.circle_filled(
                *snap,
                5.5,
                egui::Color32::from_rgba_unmultiplied(255, 196, 87, 80),
            );
            painter.circle_stroke(*snap, 7.0, egui::Stroke::new(2.0, color));
        }
    }
}

pub(super) fn placement_ghost_rect(
    canvas: egui::Rect,
    pointer: egui::Pos2,
    viewport: sketch::SketchViewport,
    snap_enabled: bool,
    grid_step: f32,
    size: egui::Vec2,
) -> egui::Rect {
    let center =
        sketch::snap_screen_point_to_grid(canvas, pointer, viewport, snap_enabled, grid_step);
    egui::Rect::from_center_size(center, size)
}

pub(super) fn placement_ghost_size(label: &str, style: SketchNodeStyle) -> egui::Vec2 {
    let width = (label.chars().count() as f32 * 7.0 + 56.0).clamp(120.0, 220.0);
    let size = egui::vec2(width, 72.0);
    if matches!(style.rotation_deg.rem_euclid(360), 90 | 270) {
        egui::vec2(size.y.max(88.0), size.x.min(180.0))
    } else {
        size
    }
}

pub(super) fn draw_placement_ghost(
    painter: &egui::Painter,
    rect: egui::Rect,
    label: &str,
    target_clear: bool,
    style: SketchNodeStyle,
) {
    let accent = if target_clear {
        egui::Color32::from_rgb(95, 190, 255)
    } else {
        egui::Color32::from_rgb(255, 116, 116)
    };
    let fill = if target_clear {
        egui::Color32::from_rgba_unmultiplied(38, 88, 112, 72)
    } else {
        egui::Color32::from_rgba_unmultiplied(112, 38, 38, 64)
    };
    painter.rect_filled(rect, 5.0, fill);
    painter.rect_stroke(
        rect,
        5.0,
        egui::Stroke::new(2.0, accent),
        egui::StrokeKind::Inside,
    );
    let side = resolved_placement_pin_side(style);
    match style.rotation_deg.rem_euclid(360) {
        90 | 270 => {
            let y = match side {
                SketchPinSide::Left => rect.top(),
                SketchPinSide::Auto | SketchPinSide::Right => rect.bottom(),
            };
            let rail = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, y),
                egui::vec2((rect.width() * 0.42).max(34.0), 3.0),
            );
            painter.rect_filled(rail, 2.0, accent);
            painter.circle_filled(
                egui::pos2(rect.center().x - rail.width() * 0.35, y),
                4.0,
                accent,
            );
            painter.circle_filled(
                egui::pos2(rect.center().x + rail.width() * 0.35, y),
                4.0,
                accent,
            );
        }
        _ => {
            let x = match side {
                SketchPinSide::Left => rect.left(),
                SketchPinSide::Auto | SketchPinSide::Right => rect.right(),
            };
            let rail = egui::Rect::from_center_size(
                egui::pos2(x, rect.center().y),
                egui::vec2(3.0, (rect.height() * 0.55).max(34.0)),
            );
            painter.rect_filled(rail, 2.0, accent);
            painter.circle_filled(
                egui::pos2(x, rect.center().y - rail.height() * 0.32),
                4.0,
                accent,
            );
            painter.circle_filled(
                egui::pos2(x, rect.center().y + rail.height() * 0.32),
                4.0,
                accent,
            );
        }
    }
    let text = if target_clear {
        label.to_string()
    } else {
        format!("{label} blocked")
    };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        compact_placement_label(&text),
        egui::FontId::monospace(11.0),
        egui::Color32::from_gray(238),
    );
    if style.rotation_deg.rem_euclid(360) != 0
        || style.mirrored
        || style.pin_side != SketchPinSide::Auto
    {
        let mut cues = Vec::new();
        if style.rotation_deg.rem_euclid(360) != 0 {
            cues.push(format!("{} deg", style.rotation_deg.rem_euclid(360)));
        }
        if style.mirrored {
            cues.push("flip".to_string());
        }
        if style.pin_side != SketchPinSide::Auto {
            cues.push(format!("pins {}", style.pin_side.as_str()));
        }
        painter.text(
            rect.right_top() + egui::vec2(-6.0, 6.0),
            egui::Align2::RIGHT_TOP,
            cues.join(" / "),
            egui::FontId::monospace(9.5),
            egui::Color32::from_gray(216),
        );
    }
}

fn resolved_placement_pin_side(style: SketchNodeStyle) -> SketchPinSide {
    match style.pin_side {
        SketchPinSide::Left | SketchPinSide::Right => style.pin_side,
        SketchPinSide::Auto if style.mirrored => SketchPinSide::Left,
        SketchPinSide::Auto => SketchPinSide::Right,
    }
}

fn compact_placement_label(label: &str) -> String {
    const MAX_CHARS: usize = 22;
    if label.chars().count() <= MAX_CHARS {
        return label.to_string();
    }
    let mut compact = label.chars().take(MAX_CHARS - 3).collect::<String>();
    compact.push_str("...");
    compact
}

pub(super) fn draw_wire_points(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    stroke: egui::Stroke,
) {
    for segment in points.windows(2) {
        painter.line_segment([segment[0], segment[1]], stroke);
    }
}

pub(super) fn draw_wire_junctions(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    color: egui::Color32,
    emphasized: bool,
) {
    let radius = if emphasized { 3.0 } else { 2.2 };
    for point in points {
        painter.circle_filled(*point, radius, color);
    }
}

fn draw_wire_label(
    painter: &egui::Painter,
    edge: &sketch::SketchEdge,
    emphasized: bool,
    opacity: f32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let label = compact_wire_label(&edge.net_id);
    let pos = edge_label_position(edge);
    let width = (label.len() as f32 * 7.0 + 8.0).clamp(24.0, 128.0);
    let rect = egui::Rect::from_min_size(pos + egui::vec2(-4.0, -9.0), egui::vec2(width, 18.0));
    let fill = if emphasized {
        egui::Color32::from_rgba_unmultiplied(30, 48, 58, 232)
    } else {
        egui::Color32::from_rgba_unmultiplied(24, 24, 24, 210)
    };
    painter.rect_filled(rect, 2.0, with_opacity(fill, opacity));
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, with_opacity(egui::Color32::from_gray(76), opacity)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(10.0),
        with_opacity(egui::Color32::from_gray(225), opacity),
    );
}

fn compact_wire_label(label: &str) -> String {
    const MAX_CHARS: usize = 18;
    if label.chars().count() <= MAX_CHARS {
        return label.to_string();
    }
    let mut compact = label.chars().take(MAX_CHARS - 3).collect::<String>();
    compact.push_str("...");
    compact
}

pub(super) fn wire_preview_start(
    graph: &sketch::SketchGraph,
    component_id: &str,
    pin_id: &str,
) -> Option<egui::Pos2> {
    let pin_id = pin_id.trim();
    graph
        .pin_anchors
        .iter()
        .find(|anchor| anchor.component_id == component_id && anchor.pin == pin_id)
        .map(|anchor| anchor.pos)
        .or_else(|| {
            graph
                .nodes
                .iter()
                .find(|node| node.selection == SketchSelection::Component(component_id.to_string()))
                .map(|node| node.rect.center())
        })
}

pub(super) fn component_context_pin(
    snapshot: &ProjectSnapshot,
    component_id: &str,
    preferred_pin: &str,
) -> (String, String) {
    let Some(component) = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == component_id)
    else {
        return ("P1".to_string(), String::new());
    };
    if let Some(pin) = component
        .pins
        .iter()
        .find(|pin| pin.pin == preferred_pin.trim())
    {
        return (pin.pin.clone(), pin.net.clone());
    }
    component
        .pins
        .first()
        .map(|pin| (pin.pin.clone(), pin.net.clone()))
        .unwrap_or_else(|| ("P1".to_string(), String::new()))
}
