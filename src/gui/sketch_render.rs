use eframe::egui;

use super::sketch::{SketchNode, SketchPinAnchor, SketchSelection, compact_label};
use super::sketch_symbols::draw_symbol_glyph;

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
    if runtime_activity.is_some() {
        draw_runtime_scope_chip(painter, node, opacity);
    }
}

pub(super) fn draw_sketch_pin_anchor(
    painter: &egui::Painter,
    anchor: &SketchPinAnchor,
    active: bool,
    labeled: bool,
    opacity: f32,
) {
    let opacity = normalized_opacity(opacity);
    let fill = if active {
        egui::Color32::from_rgb(255, 196, 87)
    } else {
        pin_kind_color(&anchor.kind)
    };
    let radius = if labeled || active { 5.0 } else { 4.0 };
    painter.circle_filled(anchor.pos, radius, with_opacity(fill, opacity));
    painter.circle_stroke(
        anchor.pos,
        radius,
        egui::Stroke::new(1.0, with_opacity(egui::Color32::from_gray(18), opacity)),
    );
    if labeled {
        let text = compact_label(
            &format!("{}:{}", anchor.pin, compact_pin_kind(&anchor.kind)),
            18,
        );
        let text_width = (text.chars().count() as f32 * 6.5 + 12.0).clamp(34.0, 118.0);
        let rect = match anchor.label_align {
            egui::Align2::LEFT_CENTER => egui::Rect::from_min_size(
                anchor.label_pos - egui::vec2(2.0, 10.0),
                egui::vec2(text_width, 20.0),
            ),
            _ => egui::Rect::from_min_size(
                anchor.label_pos - egui::vec2(text_width - 2.0, 10.0),
                egui::vec2(text_width, 20.0),
            ),
        };
        painter.rect_filled(
            rect,
            4.0,
            with_opacity(
                egui::Color32::from_rgba_unmultiplied(18, 25, 34, 235),
                opacity,
            ),
        );
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, with_opacity(fill, opacity)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::monospace(10.5),
            with_opacity(egui::Color32::WHITE, opacity),
        );
        return;
    }
    painter.text(
        anchor.label_pos,
        anchor.label_align,
        compact_label(&anchor.pin, 10),
        egui::FontId::monospace(10.5),
        with_opacity(egui::Color32::LIGHT_GRAY, opacity),
    );
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

fn pin_kind_color(kind: &str) -> egui::Color32 {
    match kind {
        "power" => egui::Color32::from_rgb(234, 105, 105),
        "ground" => egui::Color32::from_rgb(120, 195, 132),
        "digital" | "digital_or_analog" => egui::Color32::from_rgb(115, 166, 224),
        "analog" => egui::Color32::from_rgb(169, 139, 238),
        _ => egui::Color32::from_rgb(170, 178, 189),
    }
}

fn compact_pin_kind(kind: &str) -> &str {
    match kind {
        "digital_or_analog" => "dig/an",
        "unresolved" => "missing",
        other => other,
    }
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

fn draw_runtime_scope_chip(painter: &egui::Painter, node: &SketchNode, opacity: f32) {
    let text = "scope";
    let rect = egui::Rect::from_min_size(
        node.rect.right_top() + egui::vec2(-52.0, 6.0),
        egui::vec2(44.0, 18.0),
    );
    painter.rect_filled(
        rect,
        4.0,
        with_opacity(egui::Color32::from_rgb(20, 70, 55), opacity),
    );
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(
            1.0,
            with_opacity(egui::Color32::from_rgb(100, 235, 170), opacity),
        ),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::monospace(10.5),
        with_opacity(egui::Color32::WHITE, opacity),
    );
}

fn normalized_opacity(opacity: f32) -> f32 {
    opacity.clamp(0.0, 1.0)
}
