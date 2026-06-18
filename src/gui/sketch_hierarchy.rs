use eframe::egui;
use std::collections::{BTreeMap, BTreeSet};

use super::CircuitCiApp;
use super::sketch::{self, ProjectSnapshot, SketchSelection, with_opacity};
use super::sketch_bundles::SketchNetBundleBadge;
use super::sketch_probes::{SketchProbeBadge, SketchProbeTarget};

const MAX_HIERARCHY_ROWS: usize = 48;
const DIMMED_HIERARCHY_OPACITY: f32 = 0.24;
const MAX_CONNECTOR_TARGETS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SketchHierarchyTarget {
    label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SketchHierarchyFocusMode {
    Dim,
    Isolate,
}

impl SketchHierarchyFocusMode {
    fn label(self) -> &'static str {
        match self {
            Self::Dim => "dim unrelated",
            Self::Isolate => "hide unrelated",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SketchHierarchyFocus {
    label: String,
    mode: SketchHierarchyFocusMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SketchHierarchyGroup {
    label: String,
    detail: String,
    components: Vec<String>,
    nets: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct SketchHierarchyView {
    label: String,
    mode: SketchHierarchyFocusMode,
    selections: BTreeSet<SketchSelection>,
}

#[derive(Debug, Clone)]
pub(super) struct SketchHierarchyConnectorBadge {
    pub(super) net_id: String,
    pub(super) external_targets: Vec<String>,
    pub(super) rect: egui::Rect,
    anchor: egui::Pos2,
}

impl CircuitCiApp {
    pub(super) fn sketch_hierarchy_panel(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        ui.collapsing("Schematic Hierarchy", |ui| {
            if let Some(focus) = &self.sketch_hierarchy_focus {
                let focus_label = focus.label.clone();
                let focus_mode = focus.mode.label();
                ui.horizontal(|ui| {
                    ui.label(format!("Focused: {focus_label} ({focus_mode})"));
                    if ui.button("Clear Focus").clicked() {
                        self.clear_sketch_hierarchy_focus();
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label("Search");
                ui.text_edit_singleline(&mut self.sketch_hierarchy_query);
                if ui.button("Clear").clicked() {
                    self.sketch_hierarchy_query.clear();
                }
            });
            let groups = filter_hierarchy_groups(
                derive_hierarchy_groups(snapshot),
                &self.sketch_hierarchy_query,
            );
            if groups.is_empty() {
                ui.label("No derived sheet groups in this flattened Board IR graph.");
                return;
            }
            ui.label(format!(
                "{} of {} derived sheet group(s)",
                groups.len().min(MAX_HIERARCHY_ROWS),
                groups.len()
            ));
            egui::ScrollArea::vertical()
                .max_height(130.0)
                .show(ui, |ui| {
                    egui::Grid::new("sketch_hierarchy_rows")
                        .num_columns(7)
                        .striped(true)
                        .show(ui, |ui| {
                            for group in groups.iter().take(MAX_HIERARCHY_ROWS) {
                                ui.monospace("sheet");
                                ui.label(&group.label);
                                ui.label(&group.detail);
                                if ui.button("Select").clicked() {
                                    self.select_hierarchy_group(group);
                                }
                                if ui.button("Fit").clicked() {
                                    self.select_hierarchy_group(group);
                                    self.sketch_hierarchy_fit_target =
                                        Some(SketchHierarchyTarget::from_group(group));
                                }
                                if ui.button("Focus").clicked() {
                                    self.apply_sketch_hierarchy_focus(
                                        group,
                                        SketchHierarchyFocusMode::Dim,
                                    );
                                }
                                if ui.button("Isolate").clicked() {
                                    self.apply_sketch_hierarchy_focus(
                                        group,
                                        SketchHierarchyFocusMode::Isolate,
                                    );
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    }

    pub(super) fn fit_sketch_hierarchy_target(
        &mut self,
        canvas: egui::Rect,
        snapshot: &ProjectSnapshot,
        target: &SketchHierarchyTarget,
    ) {
        let groups = derive_hierarchy_groups(snapshot);
        let Some(group) = groups.iter().find(|group| group.label == target.label) else {
            self.status = "Hierarchy target is no longer present in the sketch.".to_string();
            return;
        };
        let graph = sketch::layout_sketch_graph(canvas, snapshot);
        let Some(bounds) = hierarchy_group_bounds(&graph, group) else {
            self.status =
                "Hierarchy target is not visible in the current sketch layout.".to_string();
            return;
        };
        fit_viewport_to_bounds(self, canvas, bounds);
    }

    fn select_hierarchy_group(&mut self, group: &SketchHierarchyGroup) {
        self.selected_sketch_items = group
            .components
            .iter()
            .cloned()
            .map(SketchSelection::Component)
            .chain(group.nets.iter().cloned().map(SketchSelection::Net))
            .collect();
        self.selected_sketch_item = self.selected_sketch_items.iter().next().cloned();
        self.status = format!(
            "Selected hierarchy group {} ({} components, {} nets).",
            group.label,
            group.components.len(),
            group.nets.len()
        );
    }

    fn apply_sketch_hierarchy_focus(
        &mut self,
        group: &SketchHierarchyGroup,
        mode: SketchHierarchyFocusMode,
    ) {
        self.select_hierarchy_group(group);
        self.sketch_hierarchy_focus = Some(SketchHierarchyFocus::from_group(group, mode));
        self.status = format!(
            "Focused hierarchy group {} with {}.",
            group.label,
            mode.label()
        );
    }

    fn clear_sketch_hierarchy_focus(&mut self) {
        self.sketch_hierarchy_focus = None;
        self.status = "Cleared schematic hierarchy focus.".to_string();
    }

    pub(super) fn sketch_hierarchy_view(
        &mut self,
        snapshot: &ProjectSnapshot,
    ) -> Option<SketchHierarchyView> {
        let focus = self.sketch_hierarchy_focus.clone()?;
        let groups = derive_hierarchy_groups(snapshot);
        let Some(group) = groups.iter().find(|group| group.label == focus.label) else {
            self.sketch_hierarchy_focus = None;
            self.status = "Cleared stale schematic hierarchy focus.".to_string();
            return None;
        };
        Some(SketchHierarchyView::from_group(group, focus.mode))
    }
}

impl SketchHierarchyTarget {
    fn from_group(group: &SketchHierarchyGroup) -> Self {
        Self {
            label: group.label.clone(),
        }
    }
}

impl SketchHierarchyFocus {
    fn from_group(group: &SketchHierarchyGroup, mode: SketchHierarchyFocusMode) -> Self {
        Self {
            label: group.label.clone(),
            mode,
        }
    }
}

impl SketchHierarchyView {
    fn from_group(group: &SketchHierarchyGroup, mode: SketchHierarchyFocusMode) -> Self {
        let selections = group
            .components
            .iter()
            .cloned()
            .map(SketchSelection::Component)
            .chain(group.nets.iter().cloned().map(SketchSelection::Net))
            .collect();
        Self {
            label: group.label.clone(),
            mode,
            selections,
        }
    }

    pub(super) fn label(&self) -> &str {
        &self.label
    }

    pub(super) fn interaction_visible(&self, selection: &SketchSelection) -> bool {
        self.mode != SketchHierarchyFocusMode::Isolate || self.matches_selection(selection)
    }

    pub(super) fn selection_opacity(&self, selection: &SketchSelection) -> f32 {
        self.opacity(self.matches_selection(selection))
    }

    pub(super) fn edge_visible(&self, edge: &sketch::SketchEdge) -> bool {
        self.mode != SketchHierarchyFocusMode::Isolate || self.matches_edge(edge)
    }

    pub(super) fn edge_opacity(&self, edge: &sketch::SketchEdge) -> f32 {
        self.opacity(self.matches_edge(edge))
    }

    pub(super) fn anchor_visible(&self, anchor: &sketch::SketchPinAnchor) -> bool {
        self.mode != SketchHierarchyFocusMode::Isolate || self.matches_anchor(anchor)
    }

    pub(super) fn anchor_opacity(&self, anchor: &sketch::SketchPinAnchor) -> f32 {
        self.opacity(self.matches_anchor(anchor))
    }

    pub(super) fn probe_badge_visible(&self, badge: &SketchProbeBadge) -> bool {
        self.mode != SketchHierarchyFocusMode::Isolate || self.matches_probe_badge(badge)
    }

    pub(super) fn probe_badge_opacity(&self, badge: &SketchProbeBadge) -> f32 {
        self.opacity(self.matches_probe_badge(badge))
    }

    pub(super) fn bundle_badge_visible(&self, badge: &SketchNetBundleBadge) -> bool {
        self.mode != SketchHierarchyFocusMode::Isolate || self.matches_bundle_badge(badge)
    }

    pub(super) fn bundle_badge_opacity(&self, badge: &SketchNetBundleBadge) -> f32 {
        self.opacity(self.matches_bundle_badge(badge))
    }

    fn focused_nets(&self) -> impl Iterator<Item = &str> {
        self.selections
            .iter()
            .filter_map(|selection| match selection {
                SketchSelection::Net(net) => Some(net.as_str()),
                SketchSelection::Component(_) | SketchSelection::Overflow(_) => None,
            })
    }

    fn matches_selection(&self, selection: &SketchSelection) -> bool {
        self.selections.contains(selection)
    }

    fn matches_edge(&self, edge: &sketch::SketchEdge) -> bool {
        self.selections
            .contains(&SketchSelection::Net(edge.net_id.clone()))
            || edge_source_component(&edge.source).is_some_and(|component_id| {
                self.selections
                    .contains(&SketchSelection::Component(component_id.to_string()))
            })
    }

    fn matches_anchor(&self, anchor: &sketch::SketchPinAnchor) -> bool {
        self.selections
            .contains(&SketchSelection::Component(anchor.component_id.clone()))
            || self
                .selections
                .contains(&SketchSelection::Net(anchor.net.clone()))
    }

    fn matches_probe_badge(&self, badge: &SketchProbeBadge) -> bool {
        match &badge.probe.target {
            SketchProbeTarget::Component(component_id) => self
                .selections
                .contains(&SketchSelection::Component(component_id.clone())),
            SketchProbeTarget::Net(net_id) => self
                .selections
                .contains(&SketchSelection::Net(net_id.clone())),
        }
    }

    fn matches_bundle_badge(&self, badge: &SketchNetBundleBadge) -> bool {
        badge
            .bundle
            .members
            .iter()
            .any(|net| self.selections.contains(&SketchSelection::Net(net.clone())))
    }

    fn opacity(&self, matched: bool) -> f32 {
        if matched {
            1.0
        } else {
            DIMMED_HIERARCHY_OPACITY
        }
    }
}

pub(super) fn layout_hierarchy_connector_badges(
    snapshot: &ProjectSnapshot,
    graph: &sketch::SketchGraph,
    view: &SketchHierarchyView,
) -> Vec<SketchHierarchyConnectorBadge> {
    let focused_nets = view.focused_nets().collect::<BTreeSet<_>>();
    if focused_nets.is_empty() {
        return Vec::new();
    }
    let mut external_targets_by_net = BTreeMap::<String, BTreeSet<String>>::new();
    for component in &snapshot.components_detail {
        let component_selection = SketchSelection::Component(component.id.clone());
        if view.matches_selection(&component_selection) {
            continue;
        }
        for pin in &component.pins {
            if focused_nets.contains(pin.net.as_str()) {
                external_targets_by_net
                    .entry(pin.net.clone())
                    .or_default()
                    .insert(format!("{}.{}", component.id, pin.pin));
            }
        }
    }
    external_targets_by_net
        .into_iter()
        .filter_map(|(net_id, external_targets)| {
            let node = graph
                .nodes
                .iter()
                .find(|node| node.selection == SketchSelection::Net(net_id.clone()))?;
            let targets = external_targets.into_iter().collect::<Vec<_>>();
            if targets.is_empty() {
                return None;
            }
            let label_width =
                ((targets.len().to_string().len() as f32) * 7.0 + 70.0).clamp(78.0, 132.0);
            let rect = egui::Rect::from_min_size(
                node.rect.right_top() + egui::vec2(8.0, 6.0),
                egui::vec2(label_width, 20.0),
            );
            Some(SketchHierarchyConnectorBadge {
                net_id,
                external_targets: targets,
                rect,
                anchor: node.rect.right_center(),
            })
        })
        .collect()
}

pub(super) fn hit_test_hierarchy_connector_badge(
    badges: &[SketchHierarchyConnectorBadge],
    position: egui::Pos2,
) -> Option<&SketchHierarchyConnectorBadge> {
    badges
        .iter()
        .find(|badge| badge.rect.expand(4.0).contains(position))
}

pub(super) fn draw_hierarchy_connector_badge(
    painter: &egui::Painter,
    badge: &SketchHierarchyConnectorBadge,
    hovered: bool,
) {
    let color = if hovered {
        egui::Color32::from_rgb(255, 196, 87)
    } else {
        egui::Color32::from_rgb(155, 189, 255)
    };
    painter.line_segment(
        [badge.anchor, badge.rect.left_center()],
        egui::Stroke::new(1.0, with_opacity(color, 0.9)),
    );
    painter.rect_filled(
        badge.rect,
        3.0,
        egui::Color32::from_rgba_unmultiplied(30, 44, 70, if hovered { 242 } else { 220 }),
    );
    painter.rect_stroke(
        badge.rect,
        3.0,
        egui::Stroke::new(if hovered { 2.0 } else { 1.0 }, color),
        egui::StrokeKind::Inside,
    );
    painter.text(
        badge.rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("offsheet {}", badge.external_targets.len()),
        egui::FontId::monospace(10.0),
        egui::Color32::WHITE,
    );
}

pub(super) fn hierarchy_connector_tooltip(
    ui: &mut egui::Ui,
    badge: &SketchHierarchyConnectorBadge,
) {
    ui.strong(format!("Net {} leaves focused sheet", badge.net_id));
    ui.label(format!(
        "{} external endpoint(s)",
        badge.external_targets.len()
    ));
    ui.separator();
    for target in badge.external_targets.iter().take(MAX_CONNECTOR_TARGETS) {
        ui.monospace(target);
    }
    if badge.external_targets.len() > MAX_CONNECTOR_TARGETS {
        ui.label(format!(
            "{} more",
            badge.external_targets.len() - MAX_CONNECTOR_TARGETS
        ));
    }
    ui.separator();
    ui.label("Derived from flattened Board IR pin/net bindings.");
    ui.label("Click to select the underlying net.");
}

pub(super) fn derive_hierarchy_groups(snapshot: &ProjectSnapshot) -> Vec<SketchHierarchyGroup> {
    let mut builders = BTreeMap::<String, HierarchyGroupBuilder>::new();
    for component in &snapshot.components_detail {
        for path in &component.source_paths {
            if is_root_instance_path(path) {
                continue;
            }
            let label = format!("KiCad path {path}");
            builders
                .entry(format!("source:{path}"))
                .or_insert_with(|| HierarchyGroupBuilder::new(label, "source instance path"))
                .components
                .insert(component.id.clone());
        }
        if let Some(prefix) = component_sheet_prefix(&component.id) {
            builders
                .entry(format!("prefix:{prefix}"))
                .or_insert_with(|| {
                    HierarchyGroupBuilder::new(humanize_sheet_prefix(&prefix), "namespaced IDs")
                })
                .components
                .insert(component.id.clone());
        }
    }
    for builder in builders.values_mut() {
        for component_id in &builder.components {
            if let Some(component) = snapshot
                .components_detail
                .iter()
                .find(|component| component.id == *component_id)
            {
                builder
                    .nets
                    .extend(component.pins.iter().map(|pin| pin.net.clone()));
            }
        }
    }
    for net in &snapshot.nets_detail {
        for (key, builder) in &mut builders {
            let Some(prefix) = key.strip_prefix("prefix:") else {
                continue;
            };
            if net.id.starts_with(&format!("{prefix}_")) {
                builder.nets.insert(net.id.clone());
            }
        }
    }
    builders
        .into_values()
        .map(HierarchyGroupBuilder::finish)
        .filter(|group| !group.components.is_empty() || !group.nets.is_empty())
        .collect()
}

fn edge_source_component(source: &str) -> Option<&str> {
    source.split_once('.').map(|(component, _)| component)
}

fn filter_hierarchy_groups(
    groups: Vec<SketchHierarchyGroup>,
    query: &str,
) -> Vec<SketchHierarchyGroup> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return groups;
    }
    groups
        .into_iter()
        .filter(|group| {
            group.label.to_lowercase().contains(&query)
                || group.detail.to_lowercase().contains(&query)
                || group
                    .components
                    .iter()
                    .any(|component| component.to_lowercase().contains(&query))
                || group
                    .nets
                    .iter()
                    .any(|net| net.to_lowercase().contains(&query))
        })
        .collect()
}

fn hierarchy_group_bounds(
    graph: &sketch::SketchGraph,
    group: &SketchHierarchyGroup,
) -> Option<egui::Rect> {
    let mut bounds: Option<egui::Rect> = None;
    for node in &graph.nodes {
        let included = match &node.selection {
            SketchSelection::Component(id) => group.components.contains(id),
            SketchSelection::Net(id) => group.nets.contains(id),
            SketchSelection::Overflow(_) => false,
        };
        if included {
            bounds = Some(bounds.map_or(node.rect, |current| current.union(node.rect)));
        }
    }
    bounds.map(|bounds| bounds.expand(72.0))
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

#[derive(Debug)]
struct HierarchyGroupBuilder {
    label: String,
    source: &'static str,
    components: BTreeSet<String>,
    nets: BTreeSet<String>,
}

impl HierarchyGroupBuilder {
    fn new(label: String, source: &'static str) -> Self {
        Self {
            label,
            source,
            components: BTreeSet::new(),
            nets: BTreeSet::new(),
        }
    }

    fn finish(self) -> SketchHierarchyGroup {
        let components = self.components.into_iter().collect::<Vec<_>>();
        let nets = self.nets.into_iter().collect::<Vec<_>>();
        SketchHierarchyGroup {
            label: self.label,
            detail: format!(
                "{} components / {} nets / {}",
                components.len(),
                nets.len(),
                self.source
            ),
            components,
            nets,
        }
    }
}

fn is_root_instance_path(path: &str) -> bool {
    let trimmed = path.trim();
    trimmed.is_empty() || trimmed == "/"
}

fn component_sheet_prefix(component_id: &str) -> Option<String> {
    let (prefix, local) = component_id.split_once("__")?;
    (!prefix.is_empty() && !local.is_empty()).then(|| prefix.to_string())
}

fn humanize_sheet_prefix(prefix: &str) -> String {
    prefix
        .split(['_', '-', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => format!(
                    "{}{}",
                    first.to_uppercase(),
                    characters.as_str().to_lowercase()
                ),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{
        SketchHierarchyFocusMode, SketchHierarchyView, derive_hierarchy_groups,
        layout_hierarchy_connector_badges,
    };
    use crate::gui::sketch::{
        ProjectSnapshot, SketchComponent, SketchEdge, SketchNet, SketchNodeStyle, SketchPin,
        SketchPinAnchor, SketchSelection,
    };
    use crate::gui::sketch_bundles::{SketchNetBundle, SketchNetBundleBadge};
    use crate::gui::sketch_probes::{
        SketchProbe, SketchProbeBadge, SketchProbeQuantity, SketchProbeTarget,
    };
    use eframe::egui;

    fn snapshot() -> ProjectSnapshot {
        ProjectSnapshot {
            name: "hierarchy".to_string(),
            components: 3,
            nets: 3,
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![
                SketchComponent {
                    id: "analog_frontend__R1".to_string(),
                    model: "generic.analog.resistor".to_string(),
                    part_number: None,
                    spice: None,
                    pins: vec![SketchPin {
                        pin: "A".to_string(),
                        net: "analog_frontend_filter_out".to_string(),
                    }],
                    position: None,
                    style: SketchNodeStyle::default(),
                    source_paths: Vec::new(),
                },
                SketchComponent {
                    id: "analog_frontend__C1".to_string(),
                    model: "generic.analog.capacitor".to_string(),
                    part_number: None,
                    spice: None,
                    pins: vec![SketchPin {
                        pin: "A".to_string(),
                        net: "analog_frontend_filter_out".to_string(),
                    }],
                    position: None,
                    style: SketchNodeStyle::default(),
                    source_paths: Vec::new(),
                },
                SketchComponent {
                    id: "U1".to_string(),
                    model: "generic.ic".to_string(),
                    part_number: None,
                    spice: None,
                    pins: vec![SketchPin {
                        pin: "OUT".to_string(),
                        net: "root_out".to_string(),
                    }],
                    position: None,
                    style: SketchNodeStyle::default(),
                    source_paths: vec!["/frontend".to_string()],
                },
            ],
            nets_detail: vec![
                SketchNet {
                    id: "analog_frontend_filter_out".to_string(),
                    kind: "digital_or_analog".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: Vec::new(),
                    position: None,
                },
                SketchNet {
                    id: "analog_frontend_local".to_string(),
                    kind: "digital_or_analog".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: Vec::new(),
                    position: None,
                },
                SketchNet {
                    id: "root_out".to_string(),
                    kind: "digital_or_analog".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: Vec::new(),
                    position: None,
                },
            ],
            probes: Vec::new(),
        }
    }

    #[test]
    fn derives_groups_from_namespaced_ids_and_source_paths() {
        let groups = derive_hierarchy_groups(&snapshot());
        assert!(groups.iter().any(|group| {
            group.label == "Analog Frontend"
                && group.components
                    == vec![
                        "analog_frontend__C1".to_string(),
                        "analog_frontend__R1".to_string(),
                    ]
                && group
                    .nets
                    .contains(&"analog_frontend_filter_out".to_string())
        }));
        assert!(groups.iter().any(|group| {
            group.label == "KiCad path /frontend" && group.components == vec!["U1".to_string()]
        }));
    }

    #[test]
    fn focus_view_dims_or_hides_unrelated_graph_objects() {
        let groups = derive_hierarchy_groups(&snapshot());
        let group = groups
            .iter()
            .find(|group| group.label == "Analog Frontend")
            .unwrap();
        let dim = SketchHierarchyView::from_group(group, SketchHierarchyFocusMode::Dim);
        let isolate = SketchHierarchyView::from_group(group, SketchHierarchyFocusMode::Isolate);
        let focused_component = SketchSelection::Component("analog_frontend__R1".to_string());
        let unrelated_component = SketchSelection::Component("U1".to_string());
        assert!(dim.interaction_visible(&unrelated_component));
        assert_eq!(dim.selection_opacity(&focused_component), 1.0);
        assert!(dim.selection_opacity(&unrelated_component) < 0.5);
        assert!(isolate.interaction_visible(&focused_component));
        assert!(!isolate.interaction_visible(&unrelated_component));

        let focused_edge = SketchEdge {
            net_id: "analog_frontend_filter_out".to_string(),
            source: "analog_frontend__R1.A".to_string(),
            start: egui::pos2(0.0, 0.0),
            end: egui::pos2(1.0, 1.0),
        };
        let unrelated_edge = SketchEdge {
            net_id: "root_out".to_string(),
            source: "U1.OUT".to_string(),
            start: egui::pos2(0.0, 0.0),
            end: egui::pos2(1.0, 1.0),
        };
        assert!(isolate.edge_visible(&focused_edge));
        assert!(!isolate.edge_visible(&unrelated_edge));

        let focused_anchor = SketchPinAnchor {
            component_id: "analog_frontend__R1".to_string(),
            pin: "A".to_string(),
            net: "analog_frontend_filter_out".to_string(),
            pos: egui::pos2(0.0, 0.0),
            label_pos: egui::pos2(0.0, 0.0),
            label_align: egui::Align2::CENTER_CENTER,
        };
        let unrelated_anchor = SketchPinAnchor {
            component_id: "U1".to_string(),
            pin: "OUT".to_string(),
            net: "root_out".to_string(),
            pos: egui::pos2(0.0, 0.0),
            label_pos: egui::pos2(0.0, 0.0),
            label_align: egui::Align2::CENTER_CENTER,
        };
        assert!(isolate.anchor_visible(&focused_anchor));
        assert!(!isolate.anchor_visible(&unrelated_anchor));

        let focused_probe = SketchProbeBadge {
            probe: SketchProbe {
                scenario_name: "analog".to_string(),
                probe_name: "v_filter".to_string(),
                expression: "V(filter)".to_string(),
                quantity: SketchProbeQuantity::Voltage,
                target: SketchProbeTarget::Net("analog_frontend_filter_out".to_string()),
                assertion_names: Vec::new(),
            },
            rect: egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0)),
        };
        let unrelated_probe = SketchProbeBadge {
            probe: SketchProbe {
                scenario_name: "analog".to_string(),
                probe_name: "v_root".to_string(),
                expression: "V(root)".to_string(),
                quantity: SketchProbeQuantity::Voltage,
                target: SketchProbeTarget::Net("root_out".to_string()),
                assertion_names: Vec::new(),
            },
            rect: egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0)),
        };
        assert!(isolate.probe_badge_visible(&focused_probe));
        assert!(!isolate.probe_badge_visible(&unrelated_probe));

        let focused_bundle = SketchNetBundleBadge {
            bundle: SketchNetBundle {
                label: "analog_frontend".to_string(),
                members: vec!["analog_frontend_filter_out".to_string()],
            },
            rect: egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0)),
            spine_x: 0.0,
            y_min: 0.0,
            y_max: 10.0,
            member_points: Vec::new(),
        };
        let unrelated_bundle = SketchNetBundleBadge {
            bundle: SketchNetBundle {
                label: "root".to_string(),
                members: vec!["root_out".to_string()],
            },
            rect: egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0)),
            spine_x: 0.0,
            y_min: 0.0,
            y_max: 10.0,
            member_points: Vec::new(),
        };
        assert!(isolate.bundle_badge_visible(&focused_bundle));
        assert!(!isolate.bundle_badge_visible(&unrelated_bundle));
    }

    #[test]
    fn connector_badges_expose_external_focused_net_endpoints() {
        let groups = derive_hierarchy_groups(&snapshot());
        let group = groups
            .iter()
            .find(|group| group.label == "Analog Frontend")
            .unwrap();
        let view = SketchHierarchyView::from_group(group, SketchHierarchyFocusMode::Isolate);
        let graph = crate::gui::sketch::layout_sketch_graph(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0)),
            &snapshot(),
        );
        assert!(layout_hierarchy_connector_badges(&snapshot(), &graph, &view).is_empty());

        let mut snapshot = snapshot();
        snapshot.components_detail.push(SketchComponent {
            id: "U2".to_string(),
            model: "generic.ic".to_string(),
            part_number: None,
            spice: None,
            pins: vec![SketchPin {
                pin: "IN".to_string(),
                net: "analog_frontend_filter_out".to_string(),
            }],
            position: None,
            style: SketchNodeStyle::default(),
            source_paths: Vec::new(),
        });
        snapshot.components += 1;
        let graph = crate::gui::sketch::layout_sketch_graph(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0)),
            &snapshot,
        );
        let badges = layout_hierarchy_connector_badges(&snapshot, &graph, &view);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].net_id, "analog_frontend_filter_out");
        assert_eq!(badges[0].external_targets, vec!["U2.IN".to_string()]);
    }
}
