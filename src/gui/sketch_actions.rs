use super::sketch::{
    self, ProjectSnapshot, SketchSelection, SketchViewport, edit_schematic_node_positions,
    layout_sketch_graph, persisted_node_position_from_screen_with_snap, remove_component,
    remove_net, sketch_graph_bounds,
};
use super::{CircuitCiApp, SketchGroupAction};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn selection_is_selected(&self, selection: &SketchSelection) -> bool {
        self.selected_sketch_items.contains(selection)
            || self
                .selected_sketch_item
                .as_ref()
                .is_some_and(|selected| selected == selection)
    }

    pub(super) fn set_single_sketch_selection(&mut self, selection: Option<SketchSelection>) {
        self.selected_sketch_item = selection;
        self.selected_sketch_items.clear();
    }

    pub(super) fn toggle_sketch_selection(&mut self, selection: SketchSelection) {
        if matches!(selection, SketchSelection::Overflow(_)) {
            return;
        }
        if !self.selected_sketch_items.remove(&selection) {
            self.selected_sketch_items.insert(selection.clone());
        }
        self.selected_sketch_item = self.selected_sketch_items.iter().next().cloned();
    }

    pub(super) fn apply_marquee_selection(
        &mut self,
        marquee: egui::Rect,
        graph: &sketch::SketchGraph,
    ) {
        self.selected_sketch_items = graph
            .nodes
            .iter()
            .filter(|node| !matches!(node.selection, SketchSelection::Overflow(_)))
            .filter(|node| marquee.intersects(node.rect))
            .map(|node| node.selection.clone())
            .collect();
        self.selected_sketch_item = self.selected_sketch_items.iter().next().cloned();
        self.status = format!(
            "{} sketch item(s) selected.",
            self.selected_sketch_items.len()
        );
    }

    pub(super) fn apply_sketch_group_action(
        &mut self,
        canvas: egui::Rect,
        graph: &sketch::SketchGraph,
        viewport: SketchViewport,
        action: SketchGroupAction,
    ) {
        match action {
            SketchGroupAction::Nudge(delta) => {
                self.apply_selected_schematic_screen_delta(
                    canvas,
                    graph,
                    viewport,
                    delta * viewport.zoom.clamp(0.25, 4.0),
                    "Selected sketch items nudged.",
                );
            }
            SketchGroupAction::AlignLeft => {
                let Some(left) = self
                    .selected_nodes(graph)
                    .map(|node| node.rect.left())
                    .reduce(f32::min)
                else {
                    return;
                };
                self.apply_selected_schematic_targets(
                    canvas,
                    graph,
                    viewport,
                    |node| node.rect.center() + egui::vec2(left - node.rect.left(), 0.0),
                    "Selected sketch items aligned left.",
                );
            }
            SketchGroupAction::AlignTop => {
                let Some(top) = self
                    .selected_nodes(graph)
                    .map(|node| node.rect.top())
                    .reduce(f32::min)
                else {
                    return;
                };
                self.apply_selected_schematic_targets(
                    canvas,
                    graph,
                    viewport,
                    |node| node.rect.center() + egui::vec2(0.0, top - node.rect.top()),
                    "Selected sketch items aligned top.",
                );
            }
        }
    }

    fn selected_nodes<'a>(
        &'a self,
        graph: &'a sketch::SketchGraph,
    ) -> impl Iterator<Item = &'a sketch::SketchNode> + 'a {
        graph
            .nodes
            .iter()
            .filter(|node| self.selected_sketch_items.contains(&node.selection))
            .filter(|node| !matches!(node.selection, SketchSelection::Overflow(_)))
    }

    pub(super) fn apply_selected_schematic_screen_delta(
        &mut self,
        canvas: egui::Rect,
        graph: &sketch::SketchGraph,
        viewport: SketchViewport,
        screen_delta: egui::Vec2,
        message: &str,
    ) {
        if screen_delta.length_sq() <= f32::EPSILON {
            return;
        }
        self.apply_selected_schematic_targets(
            canvas,
            graph,
            viewport,
            |node| node.rect.center() + screen_delta,
            message,
        );
    }

    fn apply_selected_schematic_targets(
        &mut self,
        canvas: egui::Rect,
        graph: &sketch::SketchGraph,
        viewport: SketchViewport,
        target_center: impl Fn(&sketch::SketchNode) -> egui::Pos2,
        message: &str,
    ) {
        let updates = self
            .selected_nodes(graph)
            .map(|node| {
                let (x, y) = persisted_node_position_from_screen_with_snap(
                    canvas,
                    target_center(node),
                    node.rect,
                    viewport,
                    self.sketch_snap_enabled,
                    self.sketch_grid_step,
                );
                (node.selection.clone(), x, y)
            })
            .collect::<Vec<_>>();
        if updates.is_empty() {
            return;
        }
        match edit_schematic_node_positions(&self.project_yaml, &updates) {
            Ok(updated) => self.apply_edited_project_yaml(updated, message),
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn sketch_viewport(&self) -> SketchViewport {
        SketchViewport {
            pan: self.sketch_pan,
            zoom: self.sketch_zoom.clamp(0.25, 4.0),
        }
    }

    pub(super) fn fit_sketch_content(&mut self, canvas: egui::Rect, snapshot: &ProjectSnapshot) {
        let graph = layout_sketch_graph(canvas, snapshot);
        let Some(bounds) = sketch_graph_bounds(&graph) else {
            self.sketch_zoom = 1.0;
            self.sketch_pan = egui::Vec2::ZERO;
            return;
        };
        let padding = 28.0;
        let available =
            (canvas.size() - egui::vec2(padding * 2.0, padding * 2.0)).max(egui::Vec2::splat(1.0));
        let content = bounds.size().max(egui::Vec2::splat(1.0));
        let zoom = (available.x / content.x)
            .min(available.y / content.y)
            .clamp(0.25, 4.0);
        let fitted_size = content * zoom;
        let target_min =
            canvas.min + egui::vec2(padding, padding) + (available - fitted_size) / 2.0;
        self.sketch_zoom = zoom;
        self.sketch_pan = target_min - canvas.min - (bounds.min - canvas.min) * zoom;
    }

    pub(super) fn apply_delete_selected_sketch_item(&mut self) {
        if !self.selected_sketch_items.is_empty() {
            self.apply_delete_selected_sketch_items();
            return;
        }
        match self.selected_sketch_item.clone() {
            Some(SketchSelection::Component(component_id)) => {
                self.apply_remove_component(&component_id);
            }
            Some(SketchSelection::Net(net_id)) => {
                self.apply_remove_net(&net_id);
            }
            Some(SketchSelection::Overflow(_)) | None => {
                self.status = "No deletable sketch item selected.".to_string();
            }
        }
    }

    pub(super) fn has_deletable_sketch_selection(&self) -> bool {
        self.selected_sketch_item
            .as_ref()
            .is_some_and(|selection| !matches!(selection, SketchSelection::Overflow(_)))
            || self
                .selected_sketch_items
                .iter()
                .any(|selection| !matches!(selection, SketchSelection::Overflow(_)))
    }

    fn apply_delete_selected_sketch_items(&mut self) {
        let mut updated = self.project_yaml.clone();
        let mut removed = 0usize;
        for selection in self.selected_sketch_items.iter() {
            if let SketchSelection::Component(component_id) = selection {
                match remove_component(&updated, component_id) {
                    Ok(next) => {
                        updated = next;
                        removed += 1;
                    }
                    Err(error) => {
                        self.record_error(error);
                        return;
                    }
                }
            }
        }
        for selection in self.selected_sketch_items.iter() {
            if let SketchSelection::Net(net_id) = selection {
                match remove_net(&updated, net_id) {
                    Ok(next) => {
                        updated = next;
                        removed += 1;
                    }
                    Err(error) => {
                        self.record_error(error);
                        return;
                    }
                }
            }
        }
        self.selected_sketch_item = None;
        self.selected_sketch_items.clear();
        self.apply_edited_project_yaml(updated, &format!("{removed} sketch item(s) removed."));
    }
}
