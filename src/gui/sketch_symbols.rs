use eframe::egui;

use super::sketch::{SketchComponent, SketchNode, SketchNodeStyle, SketchSelection};

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

impl SketchSymbolKind {
    pub(super) fn is_kicad_device_symbol(self) -> bool {
        matches!(
            self,
            Self::Resistor | Self::Capacitor | Self::Inductor | Self::Diode | Self::Source
        )
    }
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
    let stroke = egui::Stroke::new(1.7, color);
    let mut rect = if node.symbol.is_kicad_device_symbol() {
        node.rect.shrink2(egui::vec2(8.0, 18.0))
    } else {
        node.rect.shrink2(egui::vec2(28.0, 26.0))
    };
    if !node.symbol.is_kicad_device_symbol() {
        rect.min.y = rect.min.y.max(node.rect.top() + 26.0);
        rect.max.y = rect.max.y.min(node.rect.bottom() - 20.0);
    }
    if rect.width() < 48.0 || rect.height() < 14.0 {
        return;
    }
    match node.symbol {
        SketchSymbolKind::Resistor => draw_resistor_symbol(painter, rect, stroke, node.style),
        SketchSymbolKind::Capacitor => draw_capacitor_symbol(painter, rect, stroke, node.style),
        SketchSymbolKind::Inductor => draw_inductor_symbol(painter, rect, stroke, node.style),
        SketchSymbolKind::Diode => draw_diode_symbol(painter, rect, stroke, color, node.style),
        SketchSymbolKind::Source => draw_source_symbol(painter, rect, stroke, color, node.style),
        SketchSymbolKind::Connector => {
            draw_connector_symbol(painter, rect, stroke, color, node.style)
        }
        SketchSymbolKind::Ic | SketchSymbolKind::Block => {
            draw_ic_symbol(
                painter,
                rect,
                stroke,
                color,
                node.symbol == SketchSymbolKind::Ic,
                node.style,
            );
        }
        SketchSymbolKind::Net => draw_net_symbol(painter, rect, stroke),
        SketchSymbolKind::Overflow => {}
    }
}

fn draw_resistor_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    style: SketchNodeStyle,
) {
    painter.line_segment(
        [
            symbol_point(rect, -1.0, 0.0, style),
            symbol_point(rect, -0.36, 0.0, style),
        ],
        stroke,
    );
    painter.line_segment(
        [
            symbol_point(rect, 0.36, 0.0, style),
            symbol_point(rect, 1.0, 0.0, style),
        ],
        stroke,
    );
    let body = [
        symbol_point(rect, -0.36, -0.38, style),
        symbol_point(rect, 0.36, -0.38, style),
        symbol_point(rect, 0.36, 0.38, style),
        symbol_point(rect, -0.36, 0.38, style),
        symbol_point(rect, -0.36, -0.38, style),
    ];
    painter.add(egui::Shape::line(body.to_vec(), stroke));
}

fn draw_capacitor_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    style: SketchNodeStyle,
) {
    painter.line_segment(
        [
            symbol_point(rect, -1.0, 0.0, style),
            symbol_point(rect, -0.12, 0.0, style),
        ],
        stroke,
    );
    painter.line_segment(
        [
            symbol_point(rect, 0.12, 0.0, style),
            symbol_point(rect, 1.0, 0.0, style),
        ],
        stroke,
    );
    painter.line_segment(
        [
            symbol_point(rect, -0.12, -0.78, style),
            symbol_point(rect, -0.12, 0.78, style),
        ],
        stroke,
    );
    painter.line_segment(
        [
            symbol_point(rect, 0.12, -0.78, style),
            symbol_point(rect, 0.12, 0.78, style),
        ],
        stroke,
    );
}

fn draw_inductor_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    style: SketchNodeStyle,
) {
    painter.line_segment(
        [
            symbol_point(rect, -1.0, 0.0, style),
            symbol_point(rect, -0.58, 0.0, style),
        ],
        stroke,
    );
    painter.line_segment(
        [
            symbol_point(rect, 0.58, 0.0, style),
            symbol_point(rect, 1.0, 0.0, style),
        ],
        stroke,
    );
    for index in 0..4 {
        let cx = -0.42 + 0.28 * index as f32;
        painter.circle_stroke(
            symbol_point(rect, cx, 0.0, style),
            (rect.height().min(rect.width()) * 0.11).clamp(3.5, 7.0),
            stroke,
        );
    }
}

fn draw_diode_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    color: egui::Color32,
    style: SketchNodeStyle,
) {
    painter.line_segment(
        [
            symbol_point(rect, -1.0, 0.0, style),
            symbol_point(rect, -0.26, 0.0, style),
        ],
        stroke,
    );
    painter.line_segment(
        [
            symbol_point(rect, 0.32, 0.0, style),
            symbol_point(rect, 1.0, 0.0, style),
        ],
        stroke,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            symbol_point(rect, -0.26, -0.72, style),
            symbol_point(rect, -0.26, 0.72, style),
            symbol_point(rect, 0.32, 0.0, style),
        ],
        color.linear_multiply(0.35),
        stroke,
    ));
    painter.line_segment(
        [
            symbol_point(rect, 0.42, -0.72, style),
            symbol_point(rect, 0.42, 0.72, style),
        ],
        stroke,
    );
}

fn draw_source_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    color: egui::Color32,
    style: SketchNodeStyle,
) {
    let center = symbol_point(rect, 0.0, 0.0, style);
    let radius = (rect.height() * 0.38).clamp(8.0, 15.0);
    painter.line_segment(
        [
            symbol_point(rect, -1.0, 0.0, style),
            symbol_point(rect, -0.34, 0.0, style),
        ],
        stroke,
    );
    painter.line_segment(
        [
            symbol_point(rect, 0.34, 0.0, style),
            symbol_point(rect, 1.0, 0.0, style),
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
    style: SketchNodeStyle,
) {
    let pin_count = 4;
    let body = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width() * 0.42, rect.height() * 0.62),
    );
    painter.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Inside);
    for index in 0..pin_count {
        let v = -0.75 + 1.5 * (index as f32 + 0.5) / pin_count as f32;
        let start = symbol_point(rect, -1.0, v, style);
        let end = symbol_point(rect, -0.22, v, style);
        painter.line_segment([start, end], stroke);
        painter.circle_filled(end, 2.0, color);
    }
}

fn draw_ic_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    color: egui::Color32,
    with_pins: bool,
    style: SketchNodeStyle,
) {
    let body = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width() * 0.46, rect.height() * 0.68),
    );
    painter.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Inside);
    if with_pins {
        for index in 0..3 {
            painter.line_segment(
                [
                    symbol_point(rect, -0.58, -0.5 + 0.5 * index as f32, style),
                    symbol_point(rect, -0.34, -0.5 + 0.5 * index as f32, style),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    symbol_point(rect, 0.34, -0.5 + 0.5 * index as f32, style),
                    symbol_point(rect, 0.58, -0.5 + 0.5 * index as f32, style),
                ],
                stroke,
            );
        }
    }
    painter.circle_filled(body.left_top() + egui::vec2(6.0, 6.0), 2.0, color);
}

fn symbol_point(rect: egui::Rect, x: f32, y: f32, style: SketchNodeStyle) -> egui::Pos2 {
    let x = if style.mirrored { -x } else { x };
    let (x, y) = match style.rotation_deg {
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

fn draw_net_symbol(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    let y = rect.center().y;
    let left = rect.left() + rect.width() * 0.12;
    let right = rect.right() - rect.width() * 0.12;
    painter.line_segment([egui::pos2(left, y), egui::pos2(right, y)], stroke);
    painter.circle_stroke(egui::pos2(left, y), 3.0, stroke);
    painter.circle_stroke(egui::pos2(right, y), 3.0, stroke);
}
