use eframe::egui;

use super::sketch::{SketchComponent, SketchNode, SketchSelection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SketchSymbolKind {
    Resistor,
    Capacitor,
    Inductor,
    Diode,
    Source,
    Connector,
    Ic,
    Block,
    Net,
    Overflow,
}

pub(super) fn component_symbol_kind(component: &SketchComponent) -> SketchSymbolKind {
    let model = component.model.to_ascii_lowercase();
    let id = component.id.to_ascii_uppercase();
    let id_prefix = id.chars().next();
    if model.contains("resistor") || matches!(id_prefix, Some('R')) {
        SketchSymbolKind::Resistor
    } else if model.contains("capacitor") || matches!(id_prefix, Some('C')) {
        SketchSymbolKind::Capacitor
    } else if model.contains("inductor") || matches!(id_prefix, Some('L')) {
        SketchSymbolKind::Inductor
    } else if model.contains("diode") || matches!(id_prefix, Some('D')) {
        SketchSymbolKind::Diode
    } else if model.contains("voltage_source")
        || model.contains("current_source")
        || matches!(id_prefix, Some('V') | Some('I'))
    {
        SketchSymbolKind::Source
    } else if model.contains("connector")
        || model.contains("header")
        || matches!(id_prefix, Some('J') | Some('P'))
    {
        SketchSymbolKind::Connector
    } else if model.contains("mcu")
        || model.contains("regulator")
        || model.contains("controller")
        || model.contains("driver")
        || model.contains("bridge")
        || model.contains("transceiver")
        || model.contains("sensor")
        || matches!(id_prefix, Some('U'))
    {
        SketchSymbolKind::Ic
    } else {
        SketchSymbolKind::Block
    }
}

pub(super) fn draw_symbol_glyph(painter: &egui::Painter, node: &SketchNode) {
    if matches!(node.symbol, SketchSymbolKind::Overflow) {
        return;
    }
    let color = match node.selection {
        SketchSelection::Component(_) => egui::Color32::from_rgb(217, 230, 241),
        SketchSelection::Net(_) => egui::Color32::from_rgb(182, 235, 191),
        SketchSelection::Overflow(_) => egui::Color32::LIGHT_GRAY,
    };
    let stroke = egui::Stroke::new(1.5, color);
    let mut rect = node.rect.shrink2(egui::vec2(28.0, 26.0));
    rect.min.y = rect.min.y.max(node.rect.top() + 26.0);
    rect.max.y = rect.max.y.min(node.rect.bottom() - 20.0);
    if rect.width() < 48.0 || rect.height() < 14.0 {
        return;
    }
    match node.symbol {
        SketchSymbolKind::Resistor => draw_resistor_symbol(painter, rect, stroke),
        SketchSymbolKind::Capacitor => draw_capacitor_symbol(painter, rect, stroke),
        SketchSymbolKind::Inductor => draw_inductor_symbol(painter, rect, stroke),
        SketchSymbolKind::Diode => draw_diode_symbol(painter, rect, stroke, color),
        SketchSymbolKind::Source => draw_source_symbol(painter, rect, stroke, color),
        SketchSymbolKind::Connector => draw_connector_symbol(painter, rect, stroke, color),
        SketchSymbolKind::Ic | SketchSymbolKind::Block => {
            draw_ic_symbol(
                painter,
                rect,
                stroke,
                color,
                node.symbol == SketchSymbolKind::Ic,
            );
        }
        SketchSymbolKind::Net => draw_net_symbol(painter, rect, stroke),
        SketchSymbolKind::Overflow => {}
    }
}

fn draw_resistor_symbol(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    let y = rect.center().y;
    let left = rect.left();
    let right = rect.right();
    let body_left = left + rect.width() * 0.22;
    let body_right = right - rect.width() * 0.22;
    painter.line_segment([egui::pos2(left, y), egui::pos2(body_left, y)], stroke);
    painter.line_segment([egui::pos2(body_right, y), egui::pos2(right, y)], stroke);
    let step = (body_right - body_left) / 6.0;
    let amp = (rect.height() * 0.33).clamp(5.0, 10.0);
    let mut points = Vec::with_capacity(7);
    for index in 0..=6 {
        let x = body_left + step * index as f32;
        let py = if index == 0 || index == 6 {
            y
        } else if index % 2 == 1 {
            y - amp
        } else {
            y + amp
        };
        points.push(egui::pos2(x, py));
    }
    painter.add(egui::Shape::line(points, stroke));
}

fn draw_capacitor_symbol(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    let y = rect.center().y;
    let plate_gap = rect.width() * 0.08;
    let plate_h = (rect.height() * 0.72).clamp(12.0, 28.0);
    let x1 = rect.center().x - plate_gap;
    let x2 = rect.center().x + plate_gap;
    painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(x1, y)], stroke);
    painter.line_segment([egui::pos2(x2, y), egui::pos2(rect.right(), y)], stroke);
    painter.line_segment(
        [
            egui::pos2(x1, y - plate_h / 2.0),
            egui::pos2(x1, y + plate_h / 2.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(x2, y - plate_h / 2.0),
            egui::pos2(x2, y + plate_h / 2.0),
        ],
        stroke,
    );
}

fn draw_inductor_symbol(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    let y = rect.center().y;
    let left = rect.left();
    let right = rect.right();
    let body_left = left + rect.width() * 0.2;
    let radius = (rect.height() * 0.28).clamp(4.0, 8.0);
    let loops = 4;
    let body_width = radius * 2.0 * loops as f32;
    let body_right = (body_left + body_width).min(right - rect.width() * 0.18);
    painter.line_segment([egui::pos2(left, y), egui::pos2(body_left, y)], stroke);
    painter.line_segment([egui::pos2(body_right, y), egui::pos2(right, y)], stroke);
    for index in 0..loops {
        let cx = body_left + radius + radius * 2.0 * index as f32;
        painter.circle_stroke(egui::pos2(cx, y), radius, stroke);
    }
}

fn draw_diode_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    color: egui::Color32,
) {
    let y = rect.center().y;
    let tri_left = rect.center().x - rect.width() * 0.13;
    let tri_right = rect.center().x + rect.width() * 0.13;
    let half_h = (rect.height() * 0.35).clamp(7.0, 13.0);
    painter.line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(tri_left, y)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(tri_right, y), egui::pos2(rect.right(), y)],
        stroke,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(tri_left, y - half_h),
            egui::pos2(tri_left, y + half_h),
            egui::pos2(tri_right, y),
        ],
        color.linear_multiply(0.35),
        stroke,
    ));
    painter.line_segment(
        [
            egui::pos2(tri_right + 3.0, y - half_h),
            egui::pos2(tri_right + 3.0, y + half_h),
        ],
        stroke,
    );
}

fn draw_source_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    color: egui::Color32,
) {
    let center = rect.center();
    let radius = (rect.height() * 0.38).clamp(8.0, 15.0);
    painter.line_segment(
        [
            egui::pos2(rect.left(), center.y),
            egui::pos2(center.x - radius, center.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x + radius, center.y),
            egui::pos2(rect.right(), center.y),
        ],
        stroke,
    );
    painter.circle_stroke(center, radius, stroke);
    let font = egui::FontId::monospace(11.0);
    painter.text(
        center + egui::vec2(-3.5, -4.0),
        egui::Align2::CENTER_CENTER,
        "+",
        font.clone(),
        color,
    );
    painter.text(
        center + egui::vec2(4.0, 5.0),
        egui::Align2::CENTER_CENTER,
        "-",
        font,
        color,
    );
}

fn draw_connector_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    color: egui::Color32,
) {
    let pin_count = 4;
    let body = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width() * 0.42, rect.height() * 0.62),
    );
    painter.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Inside);
    for index in 0..pin_count {
        let y = body.top() + body.height() * (index as f32 + 0.5) / pin_count as f32;
        let start = egui::pos2(rect.left(), y);
        let end = egui::pos2(body.left(), y);
        painter.line_segment([start, end], stroke);
        painter.circle_filled(egui::pos2(body.left() + 5.0, y), 2.0, color);
    }
}

fn draw_ic_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    color: egui::Color32,
    with_pins: bool,
) {
    let body = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width() * 0.46, rect.height() * 0.68),
    );
    painter.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Inside);
    if with_pins {
        for index in 0..3 {
            let y = body.top() + body.height() * (index as f32 + 1.0) / 4.0;
            painter.line_segment(
                [egui::pos2(body.left() - 8.0, y), egui::pos2(body.left(), y)],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(body.right(), y),
                    egui::pos2(body.right() + 8.0, y),
                ],
                stroke,
            );
        }
    }
    painter.circle_filled(body.left_top() + egui::vec2(6.0, 6.0), 2.0, color);
}

fn draw_net_symbol(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    let y = rect.center().y;
    let left = rect.left() + rect.width() * 0.12;
    let right = rect.right() - rect.width() * 0.12;
    painter.line_segment([egui::pos2(left, y), egui::pos2(right, y)], stroke);
    painter.circle_stroke(egui::pos2(left, y), 3.0, stroke);
    painter.circle_stroke(egui::pos2(right, y), 3.0, stroke);
}
