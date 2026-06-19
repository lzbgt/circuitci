use eframe::egui;

use super::sketch::{
    ProjectSnapshot, SketchEdge, SketchGraph, SketchNode, SketchPinAnchor, SketchSelection,
    hit_test_wire, runtime_scope_chip_rect,
};
use super::sketch_bundles::SketchNetBundleBadge;
use super::sketch_canvas_interaction::hit_test_wire_route_handle;
use super::sketch_component_labels::SketchComponentLabelBadge;
use super::sketch_hierarchy::{SketchHierarchyConnectorBadge, SketchHierarchyView};
use super::sketch_minimap::SketchMinimap;
use super::sketch_net_labels::SketchNetLabelBadge;
use super::sketch_probes::{SketchProbeBadge, hit_test_probe_badge};
use super::waveform::{
    WaveformView, runtime_probe_activity_for_selection, runtime_scope_probe_target_for_selection,
};

pub(super) struct SketchCanvasHitContext<'a, 'runtime> {
    pub(super) graph: &'a SketchGraph,
    pub(super) hierarchy_view: Option<&'a SketchHierarchyView>,
    pub(super) bundle_badges: &'a [SketchNetBundleBadge],
    pub(super) hierarchy_connector_badges: &'a [SketchHierarchyConnectorBadge],
    pub(super) net_label_badges: &'a [SketchNetLabelBadge],
    pub(super) component_label_badges: &'a [SketchComponentLabelBadge],
    pub(super) minimap: Option<&'a SketchMinimap>,
    pub(super) waveforms: &'runtime [WaveformView],
    pub(super) selected_waveform: usize,
    pub(super) waveform_cursor_a_us: f64,
    pub(super) snapshot: &'runtime ProjectSnapshot,
    pub(super) runtime_scope_overlay_visible: bool,
}

#[derive(Default)]
pub(super) struct SketchCanvasHoverTargets<'a> {
    pub(super) node: Option<&'a SketchNode>,
    pub(super) runtime_scope_node: Option<&'a SketchNode>,
    pub(super) anchor: Option<&'a SketchPinAnchor>,
    pub(super) route_handle: Option<(&'a SketchEdge, usize)>,
    pub(super) wire: Option<&'a SketchEdge>,
    pub(super) probe_badge: Option<&'a SketchProbeBadge>,
    pub(super) bundle_badge: Option<&'a SketchNetBundleBadge>,
    pub(super) hierarchy_connector_badge: Option<&'a SketchHierarchyConnectorBadge>,
    pub(super) net_label_badge: Option<&'a SketchNetLabelBadge>,
    pub(super) component_label_badge: Option<&'a SketchComponentLabelBadge>,
    pub(super) pointer_over_minimap: bool,
}

impl SketchCanvasHoverTargets<'_> {
    pub(super) fn blank_canvas_hovered(&self) -> bool {
        self.node.is_none()
            && self.anchor.is_none()
            && self.route_handle.is_none()
            && self.wire.is_none()
            && self.probe_badge.is_none()
            && self.bundle_badge.is_none()
            && self.hierarchy_connector_badge.is_none()
            && self.net_label_badge.is_none()
            && self.component_label_badge.is_none()
            && !self.pointer_over_minimap
    }

    pub(super) fn placement_target_clear(&self) -> bool {
        self.node.is_none()
            && self.anchor.is_none()
            && self.wire.is_none()
            && self.probe_badge.is_none()
            && self.bundle_badge.is_none()
            && self.hierarchy_connector_badge.is_none()
            && self.net_label_badge.is_none()
            && self.component_label_badge.is_none()
    }

    pub(super) fn runtime_scope_chip_hovered(&self, selection: &SketchSelection) -> bool {
        self.runtime_scope_node
            .is_some_and(|node| node.selection == *selection)
    }
}

pub(super) fn hover_targets<'a>(
    context: &SketchCanvasHitContext<'a, '_>,
    pointer_hover: Option<egui::Pos2>,
) -> SketchCanvasHoverTargets<'a> {
    let Some(position) = pointer_hover else {
        return SketchCanvasHoverTargets::default();
    };
    let pointer_over_minimap = context
        .minimap
        .is_some_and(|minimap| minimap.rect.contains(position));
    let node = context.graph.nodes.iter().find(|node| {
        selection_visible(context.hierarchy_view, &node.selection) && node.rect.contains(position)
    });
    let runtime_scope_node = context
        .runtime_scope_overlay_visible
        .then(|| {
            context.graph.nodes.iter().find(|node| {
                selection_visible(context.hierarchy_view, &node.selection)
                    && runtime_scope_chip_rect(node).contains(position)
                    && node_has_runtime_scope_activity(context, node)
            })
        })
        .flatten();
    let anchor = context.graph.pin_anchors.iter().find(|anchor| {
        anchor_visible(context.hierarchy_view, anchor) && anchor.pos.distance(position) <= 8.0
    });
    let route_handle = if node.is_none() && anchor.is_none() {
        hit_test_wire_route_handle(context.graph, position)
            .filter(|(edge, _)| edge_visible(context.hierarchy_view, edge))
    } else {
        None
    };
    let wire = if node.is_none() && anchor.is_none() && route_handle.is_none() {
        hit_test_wire(context.graph, position)
            .filter(|edge| edge_visible(context.hierarchy_view, edge))
    } else {
        None
    };
    let probe_badge = hit_test_probe_badge(&context.graph.probe_badges, position)
        .filter(|badge| probe_badge_visible(context.hierarchy_view, badge));
    let bundle_badge =
        super::sketch_bundles::hit_test_net_bundle_badge(context.bundle_badges, position)
            .filter(|badge| bundle_badge_visible(context.hierarchy_view, badge));
    let hierarchy_connector_badge = super::sketch_hierarchy::hit_test_hierarchy_connector_badge(
        context.hierarchy_connector_badges,
        position,
    );
    let net_label_badge =
        super::sketch_net_labels::hit_test_net_label_badge(context.net_label_badges, position)
            .filter(|badge| {
                selection_visible(
                    context.hierarchy_view,
                    &SketchSelection::Net(badge.net_id.clone()),
                )
            });
    let component_label_badge = super::sketch_component_labels::hit_test_component_label_badge(
        context.component_label_badges,
        position,
    )
    .filter(|badge| {
        selection_visible(
            context.hierarchy_view,
            &SketchSelection::Component(badge.component_id.clone()),
        )
    });
    SketchCanvasHoverTargets {
        node,
        runtime_scope_node,
        anchor,
        route_handle,
        wire,
        probe_badge,
        bundle_badge,
        hierarchy_connector_badge,
        net_label_badge,
        component_label_badge,
        pointer_over_minimap,
    }
}

pub(super) fn position_hits_interactive_item(
    context: &SketchCanvasHitContext<'_, '_>,
    position: egui::Pos2,
) -> bool {
    context.graph.nodes.iter().any(|node| {
        selection_visible(context.hierarchy_view, &node.selection) && node.rect.contains(position)
    }) || context.graph.pin_anchors.iter().any(|anchor| {
        anchor_visible(context.hierarchy_view, anchor) && anchor.pos.distance(position) <= 8.0
    }) || hit_test_wire_route_handle(context.graph, position)
        .is_some_and(|(edge, _)| edge_visible(context.hierarchy_view, edge))
        || hit_test_wire(context.graph, position)
            .is_some_and(|edge| edge_visible(context.hierarchy_view, edge))
        || hit_test_probe_badge(&context.graph.probe_badges, position)
            .is_some_and(|badge| probe_badge_visible(context.hierarchy_view, badge))
        || super::sketch_bundles::hit_test_net_bundle_badge(context.bundle_badges, position)
            .is_some_and(|badge| bundle_badge_visible(context.hierarchy_view, badge))
        || super::sketch_hierarchy::hit_test_hierarchy_connector_badge(
            context.hierarchy_connector_badges,
            position,
        )
        .is_some()
        || super::sketch_net_labels::hit_test_net_label_badge(context.net_label_badges, position)
            .is_some_and(|badge| {
                selection_visible(
                    context.hierarchy_view,
                    &SketchSelection::Net(badge.net_id.clone()),
                )
            })
        || super::sketch_component_labels::hit_test_component_label_badge(
            context.component_label_badges,
            position,
        )
        .is_some_and(|badge| {
            selection_visible(
                context.hierarchy_view,
                &SketchSelection::Component(badge.component_id.clone()),
            )
        })
        || context
            .minimap
            .is_some_and(|minimap| minimap.rect.contains(position))
}

pub(super) fn runtime_scope_activity_count(context: &SketchCanvasHitContext<'_, '_>) -> usize {
    context
        .graph
        .nodes
        .iter()
        .filter(|node| {
            selection_visible(context.hierarchy_view, &node.selection)
                && node_has_runtime_scope_activity(context, node)
        })
        .count()
}

fn selection_visible(view: Option<&SketchHierarchyView>, selection: &SketchSelection) -> bool {
    view.is_none_or(|view| view.interaction_visible(selection))
}

fn anchor_visible(view: Option<&SketchHierarchyView>, anchor: &SketchPinAnchor) -> bool {
    view.is_none_or(|view| view.anchor_visible(anchor))
}

fn edge_visible(view: Option<&SketchHierarchyView>, edge: &SketchEdge) -> bool {
    view.is_none_or(|view| view.edge_visible(edge))
}

fn probe_badge_visible(view: Option<&SketchHierarchyView>, badge: &SketchProbeBadge) -> bool {
    view.is_none_or(|view| view.probe_badge_visible(badge))
}

fn bundle_badge_visible(view: Option<&SketchHierarchyView>, badge: &SketchNetBundleBadge) -> bool {
    view.is_none_or(|view| view.bundle_badge_visible(badge))
}

fn node_has_runtime_scope_activity(
    context: &SketchCanvasHitContext<'_, '_>,
    node: &SketchNode,
) -> bool {
    runtime_probe_activity_for_selection(
        context.waveforms,
        context.selected_waveform,
        context.waveform_cursor_a_us,
        &node.selection,
        context.snapshot,
    )
    .is_some()
        && runtime_scope_probe_target_for_selection(
            context.waveforms,
            context.selected_waveform,
            &node.selection,
            context.snapshot,
        )
        .is_some()
}
