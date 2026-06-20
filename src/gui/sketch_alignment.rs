use eframe::egui;
use std::collections::BTreeSet;

use super::sketch::{SketchGraph, SketchNode, SketchSelection};

const GUIDE_TOLERANCE_PX: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SketchAlignmentGuides {
    pub(super) vertical: Option<SketchAlignmentGuide>,
    pub(super) horizontal: Option<SketchAlignmentGuide>,
}

impl SketchAlignmentGuides {
    pub(super) fn is_empty(self) -> bool {
        self.vertical.is_none() && self.horizontal.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SketchAlignmentGuide {
    pub(super) coordinate: f32,
    pub(super) kind: SketchAlignmentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SketchAlignmentKind {
    Leading,
    Center,
    Trailing,
}

pub(super) fn guides_for_rect(
    graph: &SketchGraph,
    moving_rect: egui::Rect,
    excluded: &BTreeSet<SketchSelection>,
) -> SketchAlignmentGuides {
    let mut vertical: Option<(SketchAlignmentGuide, f32)> = None;
    let mut horizontal: Option<(SketchAlignmentGuide, f32)> = None;

    for node in graph
        .nodes
        .iter()
        .filter(|node| !excluded.contains(&node.selection))
    {
        compare_axis(
            &mut vertical,
            moving_rect.left(),
            moving_rect.center().x,
            moving_rect.right(),
            node.rect.left(),
            node.rect.center().x,
            node.rect.right(),
        );
        compare_axis(
            &mut horizontal,
            moving_rect.top(),
            moving_rect.center().y,
            moving_rect.bottom(),
            node.rect.top(),
            node.rect.center().y,
            node.rect.bottom(),
        );
    }

    SketchAlignmentGuides {
        vertical: vertical.map(|(guide, _)| guide),
        horizontal: horizontal.map(|(guide, _)| guide),
    }
}

pub(super) fn selection_bounds<'a>(
    nodes: impl Iterator<Item = &'a SketchNode>,
) -> Option<egui::Rect> {
    nodes
        .map(|node| node.rect)
        .reduce(|bounds, rect| bounds.union(rect))
}

pub(super) fn moved_selection_bounds(
    starts: &[(SketchSelection, egui::Rect)],
    delta: egui::Vec2,
) -> Option<egui::Rect> {
    starts
        .iter()
        .map(|(_, rect)| rect.translate(delta))
        .reduce(|bounds, rect| bounds.union(rect))
}

pub(super) fn draw_alignment_guides(
    painter: &egui::Painter,
    canvas: egui::Rect,
    guides: SketchAlignmentGuides,
) {
    if guides.is_empty() {
        return;
    }
    let color = egui::Color32::from_rgb(99, 224, 172);
    let stroke = egui::Stroke::new(1.25, color);
    if let Some(guide) = guides.vertical {
        draw_dashed_line(
            painter,
            egui::pos2(guide.coordinate, canvas.top()),
            egui::pos2(guide.coordinate, canvas.bottom()),
            stroke,
        );
        draw_guide_label(
            painter,
            egui::pos2(guide.coordinate + 5.0, canvas.top() + 16.0),
            guide.kind,
        );
    }
    if let Some(guide) = guides.horizontal {
        draw_dashed_line(
            painter,
            egui::pos2(canvas.left(), guide.coordinate),
            egui::pos2(canvas.right(), guide.coordinate),
            stroke,
        );
        draw_guide_label(
            painter,
            egui::pos2(canvas.left() + 14.0, guide.coordinate - 5.0),
            guide.kind,
        );
    }
}

pub(super) fn snap_rect_to_guides(
    rect: egui::Rect,
    guides: SketchAlignmentGuides,
    enabled: bool,
) -> egui::Rect {
    if !enabled || guides.is_empty() {
        return rect;
    }
    rect.translate(guide_offset(rect, guides))
}

pub(super) fn snap_delta_to_guides(
    graph: &SketchGraph,
    starts: &[(SketchSelection, egui::Rect)],
    delta: egui::Vec2,
    enabled: bool,
) -> egui::Vec2 {
    if !enabled {
        return delta;
    }
    let Some(bounds) = moved_selection_bounds(starts, delta) else {
        return delta;
    };
    let excluded = starts
        .iter()
        .map(|(selection, _)| selection.clone())
        .collect();
    let guides = guides_for_rect(graph, bounds, &excluded);
    delta + guide_offset(bounds, guides)
}

fn compare_axis(
    best: &mut Option<(SketchAlignmentGuide, f32)>,
    moving_leading: f32,
    moving_center: f32,
    moving_trailing: f32,
    fixed_leading: f32,
    fixed_center: f32,
    fixed_trailing: f32,
) {
    for (moving, fixed, kind) in [
        (moving_leading, fixed_leading, SketchAlignmentKind::Leading),
        (moving_center, fixed_center, SketchAlignmentKind::Center),
        (
            moving_trailing,
            fixed_trailing,
            SketchAlignmentKind::Trailing,
        ),
    ] {
        let distance = (moving - fixed).abs();
        if distance <= GUIDE_TOLERANCE_PX
            && best
                .as_ref()
                .is_none_or(|(_, best_distance)| distance < *best_distance)
        {
            *best = Some((
                SketchAlignmentGuide {
                    coordinate: fixed,
                    kind,
                },
                distance,
            ));
        }
    }
}

fn guide_offset(rect: egui::Rect, guides: SketchAlignmentGuides) -> egui::Vec2 {
    egui::vec2(
        guides
            .vertical
            .map(|guide| axis_offset(rect.left(), rect.center().x, rect.right(), guide))
            .unwrap_or(0.0),
        guides
            .horizontal
            .map(|guide| axis_offset(rect.top(), rect.center().y, rect.bottom(), guide))
            .unwrap_or(0.0),
    )
}

fn axis_offset(leading: f32, center: f32, trailing: f32, guide: SketchAlignmentGuide) -> f32 {
    guide.coordinate
        - match guide.kind {
            SketchAlignmentKind::Leading => leading,
            SketchAlignmentKind::Center => center,
            SketchAlignmentKind::Trailing => trailing,
        }
}

fn draw_dashed_line(
    painter: &egui::Painter,
    start: egui::Pos2,
    end: egui::Pos2,
    stroke: egui::Stroke,
) {
    let delta = end - start;
    let length = delta.length();
    if length <= f32::EPSILON {
        return;
    }
    let direction = delta / length;
    let dash = 8.0;
    let gap = 6.0;
    let mut offset = 0.0;
    while offset < length {
        let segment_end = (offset + dash).min(length);
        painter.line_segment(
            [start + direction * offset, start + direction * segment_end],
            stroke,
        );
        offset += dash + gap;
    }
}

fn draw_guide_label(painter: &egui::Painter, position: egui::Pos2, kind: SketchAlignmentKind) {
    let label = match kind {
        SketchAlignmentKind::Leading => "Edge",
        SketchAlignmentKind::Center => "Center",
        SketchAlignmentKind::Trailing => "Edge",
    };
    painter.text(
        position,
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::monospace(9.5),
        egui::Color32::from_rgb(99, 224, 172),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::sketch::SketchNodeStyle;
    use crate::gui::sketch_symbols::SketchSymbolKind;

    fn node(id: &str, rect: egui::Rect) -> SketchNode {
        SketchNode {
            selection: SketchSelection::Component(id.to_string()),
            label: id.to_string(),
            detail: String::new(),
            symbol: SketchSymbolKind::Block,
            kicad_symbol_id: None,
            style: SketchNodeStyle::default(),
            rect,
        }
    }

    #[test]
    fn guide_matches_near_centers_and_edges() {
        let graph = SketchGraph {
            nodes: vec![node(
                "R1",
                egui::Rect::from_min_size(egui::pos2(100.0, 80.0), egui::vec2(80.0, 40.0)),
            )],
            pin_anchors: Vec::new(),
            edges: Vec::new(),
            probe_badges: Vec::new(),
        };
        let moving = egui::Rect::from_min_size(egui::pos2(110.0, 90.0), egui::vec2(60.0, 20.0));
        let guides = guides_for_rect(&graph, moving, &BTreeSet::new());

        assert_eq!(
            guides.vertical,
            Some(SketchAlignmentGuide {
                coordinate: 140.0,
                kind: SketchAlignmentKind::Center,
            })
        );
        assert_eq!(
            guides.horizontal,
            Some(SketchAlignmentGuide {
                coordinate: 100.0,
                kind: SketchAlignmentKind::Center,
            })
        );
    }

    #[test]
    fn guide_ignores_excluded_selection() {
        let graph = SketchGraph {
            nodes: vec![node(
                "R1",
                egui::Rect::from_min_size(egui::pos2(100.0, 80.0), egui::vec2(80.0, 40.0)),
            )],
            pin_anchors: Vec::new(),
            edges: Vec::new(),
            probe_badges: Vec::new(),
        };
        let moving = egui::Rect::from_min_size(egui::pos2(110.0, 90.0), egui::vec2(60.0, 20.0));
        let excluded = BTreeSet::from([SketchSelection::Component("R1".to_string())]);

        assert!(guides_for_rect(&graph, moving, &excluded).is_empty());
    }

    #[test]
    fn snap_rect_to_guides_moves_enabled_axes_only() {
        let rect = egui::Rect::from_min_size(egui::pos2(110.0, 90.0), egui::vec2(60.0, 20.0));
        let guides = SketchAlignmentGuides {
            vertical: Some(SketchAlignmentGuide {
                coordinate: 140.0,
                kind: SketchAlignmentKind::Center,
            }),
            horizontal: None,
        };

        let snapped = snap_rect_to_guides(rect, guides, true);

        assert_eq!(snapped.center().x, 140.0);
        assert_eq!(snapped.center().y, rect.center().y);
        assert_eq!(snap_rect_to_guides(rect, guides, false), rect);
    }

    #[test]
    fn moved_bounds_translate_all_start_rects() {
        let starts = vec![
            (
                SketchSelection::Component("R1".to_string()),
                egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(30.0, 20.0)),
            ),
            (
                SketchSelection::Component("R2".to_string()),
                egui::Rect::from_min_size(egui::pos2(80.0, 60.0), egui::vec2(20.0, 20.0)),
            ),
        ];

        let bounds = moved_selection_bounds(&starts, egui::vec2(5.0, -10.0)).unwrap();

        assert_eq!(bounds.min, egui::pos2(15.0, 10.0));
        assert_eq!(bounds.max, egui::pos2(105.0, 70.0));
    }

    #[test]
    fn snap_delta_to_guides_uses_unselected_neighbors() {
        let graph = SketchGraph {
            nodes: vec![
                node(
                    "R1",
                    egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(30.0, 20.0)),
                ),
                node(
                    "R2",
                    egui::Rect::from_min_size(egui::pos2(100.0, 80.0), egui::vec2(80.0, 40.0)),
                ),
            ],
            pin_anchors: Vec::new(),
            edges: Vec::new(),
            probe_badges: Vec::new(),
        };
        let starts = vec![(
            SketchSelection::Component("R1".to_string()),
            egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(30.0, 20.0)),
        )];

        let snapped = snap_delta_to_guides(&graph, &starts, egui::vec2(114.0, 70.0), true);

        assert_eq!(snapped, egui::vec2(115.0, 70.0));
        assert_eq!(
            snap_delta_to_guides(&graph, &starts, egui::vec2(114.0, 70.0), false),
            egui::vec2(114.0, 70.0)
        );
    }
}
