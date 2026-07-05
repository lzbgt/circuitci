use eframe::egui;

use super::kicad_symbol_library::{
    KiCadSymbolPinAnchor, kicad_default_symbol_pin_anchors, kicad_symbol_pin_anchors,
};
use super::sketch::{
    SketchComponent, SketchNode, SketchNodeStyle, SketchPinAnchor, SketchPinSide, SketchPosition,
    SketchSelection,
};
use super::sketch_symbols::{SketchSymbolKind, component_symbol_kind, symbol_glyph_rect};

const MAX_SKETCH_PIN_ANCHORS_PER_COMPONENT: usize = 64;
const HIGH_PIN_LABEL_LANE_THRESHOLD: usize = 6;
const HIGH_PIN_LABEL_SIDE_OFFSET: f32 = 18.0;
const HIGH_PIN_LABEL_MIN_Y_STEP: f32 = 18.0;
const HIGH_PIN_LABEL_TOP_PAD: f32 = 20.0;
const HIGH_PIN_LABEL_BOTTOM_PAD: f32 = 14.0;

pub(super) fn component_pin_anchors(
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
        let anchors = component_pin_anchors_from_kicad(
            component,
            &kicad_anchors,
            net_kinds,
            visible_count,
            rect,
        );
        if !anchors.is_empty() {
            return anchors;
        }
    }
    if component.pins.len() == 2 && symbol.is_kicad_device_symbol() {
        return two_terminal_component_pin_anchors(component, rect, net_kinds);
    }
    generic_component_pin_anchors(component, rect, net_kinds, visible_count)
}

pub(super) fn component_pin_anchors_from_kicad(
    component: &SketchComponent,
    kicad_anchors: &[KiCadSymbolPinAnchor],
    net_kinds: &std::collections::BTreeMap<&str, &str>,
    visible_count: usize,
    rect: egui::Rect,
) -> Vec<SketchPinAnchor> {
    let mut anchors = component
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
        .collect::<Vec<_>>();
    spread_high_pin_label_lanes(&mut anchors, rect);
    anchors
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PinLabelLane {
    Left,
    Right,
}

fn spread_high_pin_label_lanes(anchors: &mut [SketchPinAnchor], rect: egui::Rect) {
    if anchors.len() < HIGH_PIN_LABEL_LANE_THRESHOLD {
        return;
    }
    spread_high_pin_label_lane(anchors, rect, PinLabelLane::Left);
    spread_high_pin_label_lane(anchors, rect, PinLabelLane::Right);
}

fn spread_high_pin_label_lane(
    anchors: &mut [SketchPinAnchor],
    rect: egui::Rect,
    lane: PinLabelLane,
) {
    let mut lane_indices = anchors
        .iter()
        .enumerate()
        .filter_map(|(index, anchor)| (pin_label_lane(anchor) == Some(lane)).then_some(index))
        .collect::<Vec<_>>();
    if lane_indices.is_empty() {
        return;
    }
    lane_indices.sort_by(|left, right| {
        anchors[*left]
            .pos
            .y
            .total_cmp(&anchors[*right].pos.y)
            .then_with(|| anchors[*left].pin.cmp(&anchors[*right].pin))
    });

    let top = rect.top() + HIGH_PIN_LABEL_TOP_PAD;
    let bottom = rect.bottom() - HIGH_PIN_LABEL_BOTTOM_PAD;
    let available = (bottom - top).max(0.0);
    let step = if lane_indices.len() <= 1 {
        0.0
    } else {
        HIGH_PIN_LABEL_MIN_Y_STEP.min(available / (lane_indices.len() - 1) as f32)
    };
    let span = step * lane_indices.len().saturating_sub(1) as f32;
    let average_y = lane_indices
        .iter()
        .map(|index| anchors[*index].pos.y)
        .sum::<f32>()
        / lane_indices.len() as f32;
    let start = (average_y - span * 0.5)
        .max(top)
        .min((bottom - span).max(top));

    for (offset, index) in lane_indices.into_iter().enumerate() {
        let anchor = &mut anchors[index];
        anchor.label_pos.y = start + offset as f32 * step;
        match lane {
            PinLabelLane::Left => {
                anchor.label_pos.x = anchor.pos.x - HIGH_PIN_LABEL_SIDE_OFFSET;
                anchor.label_align = egui::Align2::RIGHT_CENTER;
            }
            PinLabelLane::Right => {
                anchor.label_pos.x = anchor.pos.x + HIGH_PIN_LABEL_SIDE_OFFSET;
                anchor.label_align = egui::Align2::LEFT_CENTER;
            }
        }
    }
}

fn pin_label_lane(anchor: &SketchPinAnchor) -> Option<PinLabelLane> {
    if anchor.label_align == egui::Align2::RIGHT_CENTER || anchor.label_pos.x < anchor.pos.x {
        Some(PinLabelLane::Left)
    } else if anchor.label_align == egui::Align2::LEFT_CENTER || anchor.label_pos.x > anchor.pos.x {
        Some(PinLabelLane::Right)
    } else {
        None
    }
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

pub(super) fn component_node_size(
    symbol: SketchSymbolKind,
    has_explicit_kicad_symbol: bool,
    pin_count: usize,
    fallback: egui::Vec2,
) -> egui::Vec2 {
    if symbol.is_kicad_device_symbol() {
        egui::vec2(104.0, 72.0)
    } else if has_explicit_kicad_symbol {
        let readable_height = 48.0 + pin_count.max(2) as f32 * 18.0;
        egui::vec2(
            fallback.x.clamp(150.0, 220.0),
            fallback.y.max(readable_height.min(360.0)),
        )
    } else if pin_count > 4 {
        let readable_height =
            48.0 + pin_count.min(MAX_SKETCH_PIN_ANCHORS_PER_COMPONENT) as f32 * 18.0;
        egui::vec2(fallback.x, fallback.y.max(readable_height.min(360.0)))
    } else {
        fallback
    }
}

pub(super) fn net_node_size(fallback: egui::Vec2) -> egui::Vec2 {
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

pub(super) fn node_rect_from_position(
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

pub(super) fn push_overflow_hint(
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
