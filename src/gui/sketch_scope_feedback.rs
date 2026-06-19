use eframe::egui;

use super::sketch::{
    SketchEdge, SketchNode, SketchPinAnchor, SketchSelection, sketch_wire_points, with_opacity,
};
use super::sketch_canvas_render::draw_wire_points;
use super::sketch_component_labels::SketchComponentLabelBadge;
use super::sketch_net_labels::SketchNetLabelBadge;
use super::sketch_probes::{SketchProbeBadge, SketchProbeTarget};
use super::sketch_scope_tools::SketchScopeProbeTool;

pub(super) struct ScopeProbeToolFeedback {
    tool: SketchScopeProbeTool,
    selection: Option<SketchSelection>,
    geometry: ScopeProbeToolFeedbackGeometry,
    valid: bool,
    label: String,
}

pub(super) struct ScopeProbeToolHoverInput<'a> {
    pub(super) tool: SketchScopeProbeTool,
    pub(super) hovered_probe_badge: Option<&'a SketchProbeBadge>,
    pub(super) hovered_anchor: Option<&'a SketchPinAnchor>,
    pub(super) hovered_net_label_badge: Option<&'a SketchNetLabelBadge>,
    pub(super) hovered_component_label_badge: Option<&'a SketchComponentLabelBadge>,
    pub(super) hovered_wire: Option<&'a SketchEdge>,
    pub(super) hovered_node: Option<&'a SketchNode>,
    pub(super) pointer_hover: Option<egui::Pos2>,
}

enum ScopeProbeToolFeedbackGeometry {
    Rect(egui::Rect),
    Circle(egui::Pos2, f32),
    Polyline(Vec<egui::Pos2>),
    Point(egui::Pos2),
}

pub(super) fn scope_probe_tool_hover_feedback(
    input: ScopeProbeToolHoverInput<'_>,
) -> ScopeProbeToolFeedback {
    let tool = input.tool;
    let (selection, geometry) = if let Some(badge) = input.hovered_probe_badge {
        let selection = match &badge.probe.target {
            SketchProbeTarget::Component(component_id) => {
                SketchSelection::Component(component_id.clone())
            }
            SketchProbeTarget::Net(net_id) => SketchSelection::Net(net_id.clone()),
        };
        (
            Some(selection),
            ScopeProbeToolFeedbackGeometry::Rect(badge.rect),
        )
    } else if let Some(anchor) = input.hovered_anchor {
        let selection = match tool {
            SketchScopeProbeTool::Voltage => SketchSelection::Net(anchor.net.clone()),
            SketchScopeProbeTool::Current | SketchScopeProbeTool::Power => {
                SketchSelection::Component(anchor.component_id.clone())
            }
        };
        (
            Some(selection),
            ScopeProbeToolFeedbackGeometry::Circle(anchor.pos, 12.0),
        )
    } else if let Some(badge) = input.hovered_net_label_badge {
        (
            Some(SketchSelection::Net(badge.net_id.clone())),
            ScopeProbeToolFeedbackGeometry::Rect(badge.rect),
        )
    } else if let Some(badge) = input.hovered_component_label_badge {
        (
            Some(SketchSelection::Component(badge.component_id.clone())),
            ScopeProbeToolFeedbackGeometry::Rect(badge.rect),
        )
    } else if let Some(edge) = input.hovered_wire {
        (
            Some(SketchSelection::Net(edge.net_id.clone())),
            ScopeProbeToolFeedbackGeometry::Polyline(sketch_wire_points(edge)),
        )
    } else if let Some(node) = input.hovered_node {
        (
            Some(node.selection.clone()),
            ScopeProbeToolFeedbackGeometry::Rect(node.rect),
        )
    } else {
        (
            None,
            ScopeProbeToolFeedbackGeometry::Point(input.pointer_hover.unwrap_or(egui::Pos2::ZERO)),
        )
    };
    let valid = selection
        .as_ref()
        .is_some_and(|selection| tool.accepts_selection(selection));
    let label = tool.target_label(selection.as_ref());
    ScopeProbeToolFeedback {
        tool,
        selection,
        geometry,
        valid,
        label,
    }
}

pub(super) fn draw_scope_probe_tool_feedback(
    painter: &egui::Painter,
    feedback: &ScopeProbeToolFeedback,
) {
    let color = if feedback.valid {
        egui::Color32::from_rgb(99, 224, 172)
    } else {
        egui::Color32::from_rgb(255, 112, 112)
    };
    let stroke = egui::Stroke::new(2.5, color);
    let label_pos = match &feedback.geometry {
        ScopeProbeToolFeedbackGeometry::Rect(rect) => {
            painter.rect_stroke(rect.expand(5.0), 6.0, stroke, egui::StrokeKind::Outside);
            rect.right_top() + egui::vec2(8.0, -4.0)
        }
        ScopeProbeToolFeedbackGeometry::Circle(center, radius) => {
            painter.circle_stroke(*center, *radius, stroke);
            *center + egui::vec2(*radius + 8.0, -*radius)
        }
        ScopeProbeToolFeedbackGeometry::Polyline(points) => {
            draw_wire_points(painter, points, egui::Stroke::new(4.0, color));
            points
                .get(points.len().saturating_div(2))
                .copied()
                .unwrap_or(egui::Pos2::ZERO)
                + egui::vec2(8.0, -10.0)
        }
        ScopeProbeToolFeedbackGeometry::Point(point) => *point + egui::vec2(12.0, -12.0),
    };
    let fill = with_opacity(egui::Color32::from_rgb(20, 24, 28), 0.92);
    let text_color = if feedback.valid {
        egui::Color32::WHITE
    } else {
        color
    };
    let text = if feedback.selection.is_some() {
        feedback.label.clone()
    } else {
        format!("{} tool: {}", feedback.tool.button_label(), feedback.label)
    };
    let galley = painter.layout_no_wrap(text, egui::FontId::monospace(11.0), text_color);
    let label_rect = egui::Rect::from_min_size(label_pos, galley.size() + egui::vec2(10.0, 6.0));
    painter.rect_filled(label_rect, 4.0, fill);
    painter.rect_stroke(
        label_rect,
        4.0,
        egui::Stroke::new(1.0, color),
        egui::StrokeKind::Outside,
    );
    painter.galley(label_pos + egui::vec2(5.0, 3.0), galley, text_color);
}
