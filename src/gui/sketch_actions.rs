use super::sketch::{
    self, ProjectSnapshot, SketchSelection, SketchViewport, edit_schematic_node_positions,
    layout_sketch_graph, layout_sketch_graph_viewport, load_project_snapshot_from_yaml,
    persisted_node_position_from_screen_with_snap, remove_component, remove_net,
    sketch_graph_bounds,
};
use super::sketch_canvas_interaction::SketchSelectionBoxMode;
use super::sketch_duplicate::duplicate_components_with_local_nets;
use super::{CircuitCiApp, SketchGroupAction, SketchViewportCommand};
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
        match &selection {
            Some(SketchSelection::Component(id)) => self.component_rename_id = id.clone(),
            Some(SketchSelection::Net(id)) => self.net_rename_id = id.clone(),
            _ => {}
        }
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
        mode: SketchSelectionBoxMode,
    ) {
        let hits = graph
            .nodes
            .iter()
            .filter(|node| !matches!(node.selection, SketchSelection::Overflow(_)))
            .filter(|node| marquee.intersects(node.rect))
            .map(|node| node.selection.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let hit_count = hits.len();
        match mode {
            SketchSelectionBoxMode::Replace => {
                self.selected_sketch_items = hits;
            }
            SketchSelectionBoxMode::Add => {
                self.ensure_multi_selection_set();
                self.selected_sketch_items.extend(hits.iter().cloned());
            }
            SketchSelectionBoxMode::Subtract => {
                self.ensure_multi_selection_set();
                for selection in &hits {
                    self.selected_sketch_items.remove(selection);
                }
            }
        }
        self.selected_sketch_item = self.selected_sketch_items.iter().next().cloned();
        self.status = format!(
            "{} sketch item(s) {}; {} selected.",
            hit_count,
            match mode {
                SketchSelectionBoxMode::Replace => "boxed",
                SketchSelectionBoxMode::Add => "added",
                SketchSelectionBoxMode::Subtract => "removed",
            },
            self.selected_sketch_items.len()
        );
    }

    fn ensure_multi_selection_set(&mut self) {
        if self.selected_sketch_items.is_empty()
            && let Some(selection) = self.selected_sketch_item.clone()
            && !matches!(selection, SketchSelection::Overflow(_))
        {
            self.selected_sketch_items.insert(selection);
        }
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
                let Some(bounds) = self.selected_node_bounds(graph) else {
                    return;
                };
                self.apply_selected_schematic_targets(
                    canvas,
                    graph,
                    viewport,
                    |node| node.rect.center() + egui::vec2(bounds.left() - node.rect.left(), 0.0),
                    "Selected sketch items aligned left.",
                );
            }
            SketchGroupAction::AlignRight => {
                let Some(bounds) = self.selected_node_bounds(graph) else {
                    return;
                };
                self.apply_selected_schematic_targets(
                    canvas,
                    graph,
                    viewport,
                    |node| node.rect.center() + egui::vec2(bounds.right() - node.rect.right(), 0.0),
                    "Selected sketch items aligned right.",
                );
            }
            SketchGroupAction::AlignTop => {
                let Some(bounds) = self.selected_node_bounds(graph) else {
                    return;
                };
                self.apply_selected_schematic_targets(
                    canvas,
                    graph,
                    viewport,
                    |node| node.rect.center() + egui::vec2(0.0, bounds.top() - node.rect.top()),
                    "Selected sketch items aligned top.",
                );
            }
            SketchGroupAction::AlignBottom => {
                let Some(bounds) = self.selected_node_bounds(graph) else {
                    return;
                };
                self.apply_selected_schematic_targets(
                    canvas,
                    graph,
                    viewport,
                    |node| {
                        node.rect.center() + egui::vec2(0.0, bounds.bottom() - node.rect.bottom())
                    },
                    "Selected sketch items aligned bottom.",
                );
            }
            SketchGroupAction::AlignCenterX => {
                let Some(bounds) = self.selected_node_bounds(graph) else {
                    return;
                };
                self.apply_selected_schematic_targets(
                    canvas,
                    graph,
                    viewport,
                    |node| {
                        node.rect.center()
                            + egui::vec2(bounds.center().x - node.rect.center().x, 0.0)
                    },
                    "Selected sketch items centered horizontally.",
                );
            }
            SketchGroupAction::AlignCenterY => {
                let Some(bounds) = self.selected_node_bounds(graph) else {
                    return;
                };
                self.apply_selected_schematic_targets(
                    canvas,
                    graph,
                    viewport,
                    |node| {
                        node.rect.center()
                            + egui::vec2(0.0, bounds.center().y - node.rect.center().y)
                    },
                    "Selected sketch items centered vertically.",
                );
            }
            SketchGroupAction::DistributeHorizontal => {
                self.apply_distribute_selected_schematic_centers(
                    canvas,
                    graph,
                    viewport,
                    DistributionAxis::Horizontal,
                );
            }
            SketchGroupAction::DistributeVertical => {
                self.apply_distribute_selected_schematic_centers(
                    canvas,
                    graph,
                    viewport,
                    DistributionAxis::Vertical,
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

    fn selected_node_bounds(&self, graph: &sketch::SketchGraph) -> Option<egui::Rect> {
        self.selected_nodes(graph)
            .map(|node| node.rect)
            .reduce(|accumulator, rect| accumulator.union(rect))
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

    fn apply_distribute_selected_schematic_centers(
        &mut self,
        canvas: egui::Rect,
        graph: &sketch::SketchGraph,
        viewport: SketchViewport,
        axis: DistributionAxis,
    ) {
        let mut nodes = self.selected_nodes(graph).collect::<Vec<_>>();
        if nodes.len() < 3 {
            self.status = "Select at least three sketch items to distribute.".to_string();
            return;
        }
        nodes.sort_by(|left, right| axis.center(left).total_cmp(&axis.center(right)));
        let first = axis.center(nodes[0]);
        let last = axis.center(nodes[nodes.len() - 1]);
        let step = (last - first) / (nodes.len() - 1) as f32;
        let updates = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                let target = axis.with_center(node.rect.center(), first + step * index as f32);
                let (x, y) = persisted_node_position_from_screen_with_snap(
                    canvas,
                    target,
                    node.rect,
                    viewport,
                    self.sketch_snap_enabled,
                    self.sketch_grid_step,
                );
                (node.selection.clone(), x, y)
            })
            .collect::<Vec<_>>();
        let message = match axis {
            DistributionAxis::Horizontal => "Selected sketch items distributed horizontally.",
            DistributionAxis::Vertical => "Selected sketch items distributed vertically.",
        };
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

    pub(super) fn apply_sketch_viewport_command(
        &mut self,
        canvas: egui::Rect,
        snapshot: &ProjectSnapshot,
        command: SketchViewportCommand,
    ) {
        match command {
            SketchViewportCommand::FitAll => {
                self.fit_sketch_content(canvas, snapshot);
                self.status = "Fit all schematic content.".to_string();
            }
            SketchViewportCommand::FitSelection => {
                self.fit_selected_sketch_content(canvas, snapshot);
            }
            SketchViewportCommand::Home => {
                self.sketch_zoom = 1.0;
                self.sketch_pan = egui::Vec2::ZERO;
                self.status = "Schematic viewport reset.".to_string();
            }
        }
    }

    pub(super) fn fit_sketch_content(&mut self, canvas: egui::Rect, snapshot: &ProjectSnapshot) {
        let graph = layout_sketch_graph(canvas, snapshot);
        let Some(bounds) = sketch_graph_bounds(&graph) else {
            self.sketch_zoom = 1.0;
            self.sketch_pan = egui::Vec2::ZERO;
            return;
        };
        fit_viewport_to_bounds(self, canvas, bounds);
    }

    pub(super) fn fit_selected_sketch_content(
        &mut self,
        canvas: egui::Rect,
        snapshot: &ProjectSnapshot,
    ) {
        let selected = self.selected_sketch_selection_set();
        if selected.is_empty() {
            self.status = "Select a component or net before fitting selection.".to_string();
            return;
        }
        let graph = layout_sketch_graph(canvas, snapshot);
        let Some(bounds) = sketch_selection_bounds(&graph, &selected) else {
            self.status =
                "Selected schematic item is not visible in the current graph.".to_string();
            return;
        };
        fit_viewport_to_bounds(self, canvas, bounds.expand(48.0));
        self.status = format!("Fit {} selected schematic item(s).", selected.len());
    }

    pub(super) fn has_fit_selection_target(&self) -> bool {
        self.selected_sketch_items
            .iter()
            .any(|selection| !matches!(selection, SketchSelection::Overflow(_)))
            || self
                .selected_sketch_item
                .as_ref()
                .is_some_and(|selection| !matches!(selection, SketchSelection::Overflow(_)))
    }

    fn selected_sketch_selection_set(&self) -> std::collections::BTreeSet<SketchSelection> {
        if self.selected_sketch_items.is_empty() {
            return self
                .selected_sketch_item
                .clone()
                .filter(|selection| !matches!(selection, SketchSelection::Overflow(_)))
                .into_iter()
                .collect();
        }
        self.selected_sketch_items
            .iter()
            .filter(|selection| !matches!(selection, SketchSelection::Overflow(_)))
            .cloned()
            .collect()
    }
}

fn fit_viewport_to_bounds(app: &mut CircuitCiApp, canvas: egui::Rect, bounds: egui::Rect) {
    let padding = 80.0;
    let available =
        (canvas.size() - egui::vec2(padding * 2.0, padding * 2.0)).max(egui::Vec2::splat(1.0));
    let content = bounds.size().max(egui::Vec2::splat(1.0));
    let zoom = (available.x / content.x)
        .min(available.y / content.y)
        .clamp(0.25, 4.0);
    let fitted_size = content * zoom;
    let target_min = canvas.min + egui::vec2(padding, padding) + (available - fitted_size) / 2.0;
    app.sketch_zoom = zoom;
    app.sketch_pan = target_min - canvas.min - (bounds.min - canvas.min) * zoom;
}

fn sketch_selection_bounds(
    graph: &sketch::SketchGraph,
    selected: &std::collections::BTreeSet<SketchSelection>,
) -> Option<egui::Rect> {
    let selected_components = selected
        .iter()
        .filter_map(|selection| match selection {
            SketchSelection::Component(component_id) => Some(component_id.as_str()),
            SketchSelection::Net(_) | SketchSelection::Overflow(_) => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let selected_nets = selected
        .iter()
        .filter_map(|selection| match selection {
            SketchSelection::Net(net_id) => Some(net_id.as_str()),
            SketchSelection::Component(_) | SketchSelection::Overflow(_) => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut bounds: Option<egui::Rect> = None;
    let mut include_rect = |rect: egui::Rect| {
        bounds = Some(bounds.map_or(rect, |current| current.union(rect)));
    };
    for node in &graph.nodes {
        if selected.contains(&node.selection) {
            include_rect(node.rect);
        }
    }
    for anchor in &graph.pin_anchors {
        if selected_components.contains(anchor.component_id.as_str())
            || selected_nets.contains(anchor.net.as_str())
        {
            include_rect(egui::Rect::from_center_size(
                anchor.pos,
                egui::vec2(12.0, 12.0),
            ));
            include_rect(egui::Rect::from_center_size(
                anchor.label_pos,
                egui::vec2(64.0, 18.0),
            ));
        }
    }
    for edge in &graph.edges {
        let source_component = edge
            .source
            .split_once('.')
            .map_or(edge.source.as_str(), |(component, _)| component);
        if selected_nets.contains(edge.net_id.as_str())
            || selected_components.contains(source_component)
        {
            include_rect(egui::Rect::from_two_pos(edge.start, edge.end));
        }
    }
    bounds
}

impl CircuitCiApp {
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

    pub(super) fn has_duplicable_sketch_selection(&self) -> bool {
        self.selected_sketch_items
            .iter()
            .any(|selection| matches!(selection, SketchSelection::Component(_)))
            || self
                .selected_sketch_item
                .as_ref()
                .is_some_and(|selection| matches!(selection, SketchSelection::Component(_)))
    }

    pub(super) fn apply_duplicate_selected_sketch_items(&mut self) {
        let component_ids = self.selected_component_ids();
        if component_ids.is_empty() {
            self.status = "Select at least one component to duplicate.".to_string();
            return;
        }
        match duplicate_components_with_local_nets(
            &self.project_yaml,
            &component_ids,
            egui::vec2(32.0, 32.0),
        ) {
            Ok((updated, selections)) => {
                self.apply_edited_project_yaml(
                    updated,
                    &format!("{} component(s) duplicated.", component_ids.len()),
                );
                self.selected_sketch_items = selections.into_iter().collect();
                self.selected_sketch_item = self.selected_sketch_items.iter().next().cloned();
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_copy_selected_sketch_items(&mut self) {
        let component_ids = self.selected_component_ids();
        if component_ids.is_empty() {
            self.status = "Select at least one component to copy.".to_string();
            return;
        }
        self.sketch_clipboard_components = component_ids;
        self.status = format!(
            "{} component(s) copied to sketch clipboard.",
            self.sketch_clipboard_components.len()
        );
    }

    pub(super) fn has_pasteable_sketch_clipboard(&self) -> bool {
        !self.sketch_clipboard_components.is_empty()
    }

    pub(super) fn apply_paste_sketch_clipboard(
        &mut self,
        canvas: egui::Rect,
        target_screen_position: Option<egui::Pos2>,
    ) {
        let component_ids = self.sketch_clipboard_components.clone();
        if component_ids.is_empty() {
            self.status = "Copy at least one component before pasting.".to_string();
            return;
        }
        let target = target_screen_position
            .filter(|position| canvas.contains(*position))
            .unwrap_or_else(|| canvas.center());
        match self.paste_components_at(&component_ids, canvas, target) {
            Ok((updated, selections)) => {
                self.apply_edited_project_yaml(
                    updated,
                    &format!("{} component(s) pasted.", component_ids.len()),
                );
                self.selected_sketch_items = selections.into_iter().collect();
                self.selected_sketch_item = self.selected_sketch_items.iter().next().cloned();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn paste_components_at(
        &self,
        component_ids: &[String],
        canvas: egui::Rect,
        target_screen_position: egui::Pos2,
    ) -> anyhow::Result<(String, Vec<SketchSelection>)> {
        let (duplicated, selections) = duplicate_components_with_local_nets(
            &self.project_yaml,
            component_ids,
            egui::vec2(32.0, 32.0),
        )?;
        let snapshot = load_project_snapshot_from_yaml(&duplicated)?;
        let viewport = self.sketch_viewport();
        let graph = layout_sketch_graph_viewport(canvas, &snapshot, viewport);
        let selection_set = selections.iter().collect::<std::collections::BTreeSet<_>>();
        let selected_nodes = graph
            .nodes
            .iter()
            .filter(|node| selection_set.contains(&node.selection))
            .collect::<Vec<_>>();
        if selected_nodes.is_empty() {
            return Ok((duplicated, selections));
        }
        let bounds = selected_nodes
            .iter()
            .map(|node| node.rect)
            .reduce(|accumulator, rect| accumulator.union(rect))
            .expect("selected_nodes is not empty");
        let delta = target_screen_position - bounds.center();
        let updates = selected_nodes
            .iter()
            .map(|node| {
                let (x, y) = persisted_node_position_from_screen_with_snap(
                    canvas,
                    node.rect.center() + delta,
                    node.rect,
                    viewport,
                    self.sketch_snap_enabled,
                    self.sketch_grid_step,
                );
                (node.selection.clone(), x, y)
            })
            .collect::<Vec<_>>();
        Ok((
            edit_schematic_node_positions(&duplicated, &updates)?,
            selections,
        ))
    }

    fn selected_component_ids(&self) -> Vec<String> {
        if self.selected_sketch_items.is_empty() {
            return self
                .selected_sketch_item
                .as_ref()
                .and_then(|selection| match selection {
                    SketchSelection::Component(component_id) => Some(component_id.clone()),
                    SketchSelection::Net(_) | SketchSelection::Overflow(_) => None,
                })
                .into_iter()
                .collect();
        }
        self.selected_sketch_items
            .iter()
            .filter_map(|selection| match selection {
                SketchSelection::Component(component_id) => Some(component_id.clone()),
                SketchSelection::Net(_) | SketchSelection::Overflow(_) => None,
            })
            .collect()
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

#[derive(Debug, Clone, Copy)]
enum DistributionAxis {
    Horizontal,
    Vertical,
}

impl DistributionAxis {
    fn center(self, node: &sketch::SketchNode) -> f32 {
        match self {
            Self::Horizontal => node.rect.center().x,
            Self::Vertical => node.rect.center().y,
        }
    }

    fn with_center(self, center: egui::Pos2, value: f32) -> egui::Pos2 {
        match self {
            Self::Horizontal => egui::pos2(value, center.y),
            Self::Vertical => egui::pos2(center.x, value),
        }
    }
}
