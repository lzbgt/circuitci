use eframe::egui;
use std::collections::{BTreeMap, BTreeSet};

use super::CircuitCiApp;
use super::sketch::{self, ProjectSnapshot, SketchSelection};

const MAX_HIERARCHY_ROWS: usize = 48;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SketchHierarchyTarget {
    label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SketchHierarchyGroup {
    label: String,
    detail: String,
    components: Vec<String>,
    nets: Vec<String>,
}

impl CircuitCiApp {
    pub(super) fn sketch_hierarchy_panel(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        ui.collapsing("Schematic Hierarchy", |ui| {
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
                        .num_columns(5)
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
}

impl SketchHierarchyTarget {
    fn from_group(group: &SketchHierarchyGroup) -> Self {
        Self {
            label: group.label.clone(),
        }
    }
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
    use super::derive_hierarchy_groups;
    use crate::gui::sketch::{
        ProjectSnapshot, SketchComponent, SketchNet, SketchNodeStyle, SketchPin,
    };

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
}
