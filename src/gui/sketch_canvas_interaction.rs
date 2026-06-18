use eframe::egui;

use super::sketch::{self, SketchPinSide, SketchSelection, hit_test_wire, sketch_wire_points};
use super::sketch_net_labels;
use super::sketch_routes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SketchSelectionBoxMode {
    Replace,
    Add,
    Subtract,
}

impl SketchSelectionBoxMode {
    pub(super) fn from_modifiers(modifiers: egui::Modifiers) -> Option<Self> {
        if modifiers.alt {
            Some(Self::Subtract)
        } else if modifiers.command || modifiers.ctrl {
            Some(Self::Add)
        } else if modifiers.shift {
            Some(Self::Replace)
        } else {
            None
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Replace => "Select",
            Self::Add => "Add",
            Self::Subtract => "Subtract",
        }
    }

    pub(super) fn fill(self) -> egui::Color32 {
        match self {
            Self::Replace => egui::Color32::from_rgba_unmultiplied(93, 185, 255, 24),
            Self::Add => egui::Color32::from_rgba_unmultiplied(88, 214, 141, 24),
            Self::Subtract => egui::Color32::from_rgba_unmultiplied(255, 120, 90, 24),
        }
    }

    pub(super) fn stroke(self) -> egui::Color32 {
        match self {
            Self::Replace => egui::Color32::from_rgb(93, 185, 255),
            Self::Add => egui::Color32::from_rgb(88, 214, 141),
            Self::Subtract => egui::Color32::from_rgb(255, 120, 90),
        }
    }
}

pub(super) fn normalize_canvas_rotation(rotation_deg: i32) -> i32 {
    rotation_deg.rem_euclid(360) / 90 * 90
}

pub(super) fn next_pin_side(pin_side: SketchPinSide) -> SketchPinSide {
    match pin_side {
        SketchPinSide::Auto => SketchPinSide::Right,
        SketchPinSide::Right => SketchPinSide::Left,
        SketchPinSide::Left => SketchPinSide::Auto,
    }
}

pub(super) fn zoom_viewport_around(
    current_zoom: f32,
    current_pan: egui::Vec2,
    zoom_delta: f32,
    canvas: egui::Rect,
    focus: egui::Pos2,
) -> (f32, egui::Vec2) {
    let old_zoom = current_zoom.clamp(0.25, 4.0);
    let new_zoom = (old_zoom * zoom_delta).clamp(0.25, 4.0);
    if (new_zoom - old_zoom).abs() <= f32::EPSILON {
        return (old_zoom, current_pan);
    }
    let focus = egui::pos2(
        focus.x.clamp(canvas.left(), canvas.right()),
        focus.y.clamp(canvas.top(), canvas.bottom()),
    );
    let focus_offset = focus - canvas.min;
    let logical_focus = (focus_offset - current_pan) / old_zoom;
    (new_zoom, focus_offset - logical_focus * new_zoom)
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum WireDragTarget {
    Pin {
        component_id: String,
        pin: String,
        net: String,
        pos: egui::Pos2,
    },
    NetNode {
        net_id: String,
        rect: egui::Rect,
    },
    NetLabel {
        net_id: String,
        label_id: String,
        rect: egui::Rect,
    },
    Wire {
        net_id: String,
        source: String,
        start: egui::Pos2,
        end: egui::Pos2,
        snap: egui::Pos2,
    },
}

impl WireDragTarget {
    pub(super) fn snap_position(&self) -> egui::Pos2 {
        match self {
            WireDragTarget::Pin { pos, .. } => *pos,
            WireDragTarget::NetNode { rect, .. } => rect.center(),
            WireDragTarget::NetLabel { rect, .. } => rect.center(),
            WireDragTarget::Wire { snap, .. } => *snap,
        }
    }

    pub(super) fn is_source_pin(&self, component_id: &str, pin: &str) -> bool {
        matches!(
            self,
            WireDragTarget::Pin {
                component_id: target_component_id,
                pin: target_pin,
                ..
            } if target_component_id == component_id && target_pin == pin
        )
    }

    pub(super) fn matches_edge(&self, edge: &sketch::SketchEdge) -> bool {
        matches!(
            self,
            WireDragTarget::Wire {
                net_id,
                source,
                ..
            } if net_id == &edge.net_id && source == &edge.source
        )
    }
}

pub(super) fn wire_drag_target_at(
    graph: &sketch::SketchGraph,
    net_label_badges: &[sketch_net_labels::SketchNetLabelBadge],
    position: egui::Pos2,
    anchor_visible: impl Fn(&sketch::SketchPinAnchor) -> bool,
    edge_visible: impl Fn(&sketch::SketchEdge) -> bool,
    node_visible: impl Fn(&sketch::SketchNode) -> bool,
    label_visible: impl Fn(&sketch_net_labels::SketchNetLabelBadge) -> bool,
) -> Option<WireDragTarget> {
    if let Some(anchor) = graph
        .pin_anchors
        .iter()
        .find(|anchor| anchor_visible(anchor) && anchor.pos.distance(position) <= 10.0)
    {
        return Some(WireDragTarget::Pin {
            component_id: anchor.component_id.clone(),
            pin: anchor.pin.clone(),
            net: anchor.net.clone(),
            pos: anchor.pos,
        });
    }
    if let Some(badge) = sketch_net_labels::hit_test_net_label_badge(net_label_badges, position)
        .filter(|badge| label_visible(badge))
    {
        return Some(WireDragTarget::NetLabel {
            net_id: badge.net_id.clone(),
            label_id: badge.id.clone(),
            rect: badge.rect,
        });
    }
    if let Some(node) = graph
        .nodes
        .iter()
        .find(|node| node_visible(node) && node.rect.contains(position))
        && let SketchSelection::Net(net_id) = &node.selection
    {
        return Some(WireDragTarget::NetNode {
            net_id: net_id.clone(),
            rect: node.rect,
        });
    }
    hit_test_wire(graph, position)
        .filter(|edge| edge_visible(edge))
        .map(|edge| WireDragTarget::Wire {
            net_id: edge.net_id.clone(),
            source: edge.source.clone(),
            start: edge.start,
            end: edge.end,
            snap: closest_point_on_edge(position, edge),
        })
}

pub(super) fn closest_point_on_edge(position: egui::Pos2, edge: &sketch::SketchEdge) -> egui::Pos2 {
    sketch_routes::closest_point_on_polyline(position, &sketch_wire_points(edge))
}

pub(super) fn schematic_canvas_size(available: egui::Vec2) -> egui::Vec2 {
    egui::vec2(available.x.max(560.0), available.y.max(520.0))
}

pub(super) fn hit_test_wire_route_handle(
    graph: &sketch::SketchGraph,
    position: egui::Pos2,
) -> Option<(&sketch::SketchEdge, usize)> {
    graph
        .edges
        .iter()
        .flat_map(|edge| {
            edge.route
                .iter()
                .enumerate()
                .map(move |(index, point)| (edge, index, point.distance(position)))
        })
        .filter(|(_, _, distance)| *distance <= 8.0)
        .min_by(|(_, _, left), (_, _, right)| left.total_cmp(right))
        .map(|(edge, index, _)| (edge, index))
}

pub(super) fn wire_route_insert_index(edge: &sketch::SketchEdge, position: egui::Pos2) -> usize {
    sketch_routes::route_insert_index(edge.start, &edge.route, edge.end, position)
}
