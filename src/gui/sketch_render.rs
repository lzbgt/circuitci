use eframe::egui;

use super::sketch::{SketchNode, SketchPinAnchor, SketchSelection, compact_label};
use super::sketch_symbols::{SketchSymbolKind, draw_symbol_glyph};

pub(super) fn draw_sketch_node(
    painter: &egui::Painter,
    node: &SketchNode,
    selected: bool,
    runtime_activity: Option<f64>,
    runtime_scope_chip_hovered: bool,
    opacity: f32,
) {
    let opacity = normalized_opacity(opacity);
    let kicad_device_symbol = matches!(node.selection, SketchSelection::Component(_))
        && node.symbol.is_kicad_device_symbol();
    let lightweight_node = kicad_device_symbol || matches!(node.selection, SketchSelection::Net(_));
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
    if lightweight_node {
        if selected || runtime_activity.is_some() {
            painter.rect_filled(
                node.rect.expand(4.0),
                3.0,
                with_opacity(fill.linear_multiply(0.45), opacity),
            );
            painter.rect_stroke(node.rect.expand(4.0), 3.0, stroke, egui::StrokeKind::Inside);
        }
    } else {
        painter.rect_filled(node.rect, 4.0, with_opacity(fill, opacity));
        painter.rect_stroke(node.rect, 4.0, stroke, egui::StrokeKind::Inside);
    }
    draw_symbol_glyph(painter, node);
    if opacity < 0.999 && !lightweight_node {
        let alpha = ((1.0 - opacity) * 170.0).round() as u8;
        painter.rect_filled(
            node.rect.shrink(1.0),
            4.0,
            egui::Color32::from_rgba_unmultiplied(18, 18, 18, alpha),
        );
    }
    if kicad_device_symbol {
        draw_kicad_device_labels(painter, node, opacity);
    } else if matches!(node.selection, SketchSelection::Net(_)) {
        draw_net_label(painter, node, opacity);
    } else {
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
    if runtime_activity.is_some() {
        draw_runtime_scope_chip(painter, node, runtime_scope_chip_hovered, opacity);
    }
}

fn draw_net_label(painter: &egui::Painter, node: &SketchNode, opacity: f32) {
    painter.text(
        node.rect.center_top() + egui::vec2(0.0, -3.0),
        egui::Align2::CENTER_BOTTOM,
        compact_label(&node.label, 18),
        egui::FontId::monospace(11.0),
        with_opacity(egui::Color32::from_rgb(182, 235, 191), opacity),
    );
}

fn draw_kicad_device_labels(painter: &egui::Painter, node: &SketchNode, opacity: f32) {
    let label_pos = node.rect.center_top() + egui::vec2(0.0, -8.0);
    painter.text(
        label_pos,
        egui::Align2::CENTER_BOTTOM,
        compact_label(&node.label, 18),
        egui::FontId::monospace(12.0),
        with_opacity(egui::Color32::WHITE, opacity),
    );
    let show_detail = !matches!(node.symbol, SketchSymbolKind::Source);
    if show_detail {
        painter.text(
            node.rect.center_bottom() + egui::vec2(0.0, 8.0),
            egui::Align2::CENTER_TOP,
            compact_label(&node.detail, 22),
            egui::FontId::monospace(10.0),
            with_opacity(egui::Color32::LIGHT_GRAY, opacity),
        );
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

pub(super) fn runtime_scope_chip_rect(node: &SketchNode) -> egui::Rect {
    egui::Rect::from_min_size(
        node.rect.right_top() + egui::vec2(-52.0, 6.0),
        egui::vec2(44.0, 18.0),
    )
}

fn draw_runtime_scope_chip(
    painter: &egui::Painter,
    node: &SketchNode,
    hovered: bool,
    opacity: f32,
) {
    let text = "scope";
    let rect = runtime_scope_chip_rect(node);
    let fill = if hovered {
        egui::Color32::from_rgb(28, 96, 70)
    } else {
        egui::Color32::from_rgb(20, 70, 55)
    };
    let stroke = if hovered {
        egui::Stroke::new(
            2.0,
            with_opacity(egui::Color32::from_rgb(140, 255, 205), opacity),
        )
    } else {
        egui::Stroke::new(
            1.0,
            with_opacity(egui::Color32::from_rgb(100, 235, 170), opacity),
        )
    };
    painter.rect_filled(rect, 4.0, with_opacity(fill, opacity));
    painter.rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
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
