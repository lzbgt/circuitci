use std::collections::BTreeSet;

use eframe::egui;

use super::sketch::{self, ProjectSnapshot, SketchSelection};
use super::sketch_actions::sketch_selection_bounds;
use super::{CircuitCiApp, SketchGroupAction, SketchViewportCommand};

const QUICK_TOOLBAR_SIZE: egui::Vec2 = egui::vec2(396.0, 34.0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SketchMultiSelectionSummary {
    total: usize,
    components: usize,
    nets: usize,
    pins: usize,
    connections: usize,
    common_model: Option<String>,
    common_net_kind: Option<String>,
}

impl CircuitCiApp {
    pub(super) fn sketch_multi_selection_inspector(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &ProjectSnapshot,
    ) {
        let summary = summarize_multi_selection(snapshot, &self.selected_sketch_items);
        ui.strong(format!("{} selected", summary.total));
        ui.label(format!(
            "{} component(s), {} net(s)",
            summary.components, summary.nets
        ));
        ui.label(format!(
            "{} selected pin(s), {} selected net connection(s)",
            summary.pins, summary.connections
        ));
        if let Some(model) = &summary.common_model {
            ui.label(format!("Model: {model}"));
        } else if summary.components > 1 {
            ui.label("Model: mixed");
        }
        if let Some(kind) = &summary.common_net_kind {
            ui.label(format!("Net kind: {kind}"));
        } else if summary.nets > 1 {
            ui.label("Net kind: mixed");
        }

        ui.separator();
        ui.label("Viewport");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Fit Selection").clicked() {
                self.sketch_viewport_command = Some(SketchViewportCommand::FitSelection);
            }
            if ui.button("Clear Selection").clicked() {
                self.set_single_sketch_selection(None);
            }
        });

        ui.separator();
        ui.label("Move");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Left").clicked() {
                self.sketch_group_action = Some(SketchGroupAction::Nudge(egui::vec2(-16.0, 0.0)));
            }
            if ui.button("Right").clicked() {
                self.sketch_group_action = Some(SketchGroupAction::Nudge(egui::vec2(16.0, 0.0)));
            }
            if ui.button("Up").clicked() {
                self.sketch_group_action = Some(SketchGroupAction::Nudge(egui::vec2(0.0, -16.0)));
            }
            if ui.button("Down").clicked() {
                self.sketch_group_action = Some(SketchGroupAction::Nudge(egui::vec2(0.0, 16.0)));
            }
        });

        ui.separator();
        ui.label("Align");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Left").clicked() {
                self.sketch_group_action = Some(SketchGroupAction::AlignLeft);
            }
            if ui.button("Right").clicked() {
                self.sketch_group_action = Some(SketchGroupAction::AlignRight);
            }
            if ui.button("Top").clicked() {
                self.sketch_group_action = Some(SketchGroupAction::AlignTop);
            }
            if ui.button("Bottom").clicked() {
                self.sketch_group_action = Some(SketchGroupAction::AlignBottom);
            }
            if ui.button("Center X").clicked() {
                self.sketch_group_action = Some(SketchGroupAction::AlignCenterX);
            }
            if ui.button("Center Y").clicked() {
                self.sketch_group_action = Some(SketchGroupAction::AlignCenterY);
            }
        });

        ui.separator();
        ui.label("Distribute");
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(summary.total >= 3, egui::Button::new("Horizontal"))
                .clicked()
            {
                self.sketch_group_action = Some(SketchGroupAction::DistributeHorizontal);
            }
            if ui
                .add_enabled(summary.total >= 3, egui::Button::new("Vertical"))
                .clicked()
            {
                self.sketch_group_action = Some(SketchGroupAction::DistributeVertical);
            }
        });

        ui.separator();
        ui.label("Edit");
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.has_duplicable_sketch_selection(),
                    egui::Button::new("Duplicate"),
                )
                .clicked()
            {
                self.apply_duplicate_selected_sketch_items();
            }
            if ui
                .add_enabled(
                    self.has_duplicable_sketch_selection(),
                    egui::Button::new("Copy"),
                )
                .clicked()
            {
                self.apply_copy_selected_sketch_items();
            }
            if ui
                .add_enabled(
                    self.has_deletable_sketch_selection(),
                    egui::Button::new("Delete"),
                )
                .clicked()
            {
                self.apply_delete_selected_sketch_item();
            }
        });
    }

    pub(super) fn sketch_selection_quick_toolbar(
        &mut self,
        ui: &mut egui::Ui,
        canvas: egui::Rect,
        graph: &sketch::SketchGraph,
    ) {
        if self.selected_sketch_items.len() <= 1
            || self.sketch_selection_box_drag.is_some()
            || self.sketch_selection_lasso_drag.is_some()
            || self.sketch_wire_route_drag.is_some()
            || self.sketch_net_label_drag.is_some()
            || self.sketch_component_label_drag.is_some()
            || self.wire_from_component.is_some()
            || self.sketch_palette_place_armed
            || self.sketch_library_place_armed
            || self.sketch_net_label_place_armed
        {
            return;
        }
        let Some(bounds) = sketch_selection_bounds(graph, &self.selected_sketch_items) else {
            return;
        };
        let pos = quick_toolbar_position(canvas, bounds, QUICK_TOOLBAR_SIZE);
        egui::Area::new(egui::Id::new("sketch_selection_quick_toolbar"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                    ui.horizontal(|ui| {
                        ui.label(format!("{} selected", self.selected_sketch_items.len()));
                        if ui.small_button("Fit").clicked() {
                            self.sketch_viewport_command =
                                Some(SketchViewportCommand::FitSelection);
                        }
                        if ui.small_button("Align").clicked() {
                            self.sketch_group_action = Some(SketchGroupAction::AlignCenterY);
                        }
                        if ui.small_button("Distribute").clicked() {
                            self.sketch_group_action =
                                Some(SketchGroupAction::DistributeHorizontal);
                        }
                        if ui
                            .add_enabled(
                                self.has_duplicable_sketch_selection(),
                                egui::Button::new("Duplicate").small(),
                            )
                            .clicked()
                        {
                            self.apply_duplicate_selected_sketch_items();
                        }
                        if ui
                            .add_enabled(
                                self.has_deletable_sketch_selection(),
                                egui::Button::new("Delete").small(),
                            )
                            .clicked()
                        {
                            self.apply_delete_selected_sketch_item();
                        }
                    });
                });
            });
    }
}

pub(super) fn summarize_multi_selection(
    snapshot: &ProjectSnapshot,
    selected: &BTreeSet<SketchSelection>,
) -> SketchMultiSelectionSummary {
    let mut summary = SketchMultiSelectionSummary {
        total: selected
            .iter()
            .filter(|selection| !matches!(selection, SketchSelection::Overflow(_)))
            .count(),
        components: 0,
        nets: 0,
        pins: 0,
        connections: 0,
        common_model: None,
        common_net_kind: None,
    };
    let mut component_models = BTreeSet::new();
    let mut net_kinds = BTreeSet::new();
    for selection in selected {
        match selection {
            SketchSelection::Component(component_id) => {
                if let Some(component) = snapshot
                    .components_detail
                    .iter()
                    .find(|candidate| candidate.id == *component_id)
                {
                    summary.components += 1;
                    summary.pins += component.pins.len();
                    component_models.insert(component.model.clone());
                }
            }
            SketchSelection::Net(net_id) => {
                if let Some(net) = snapshot
                    .nets_detail
                    .iter()
                    .find(|candidate| candidate.id == *net_id)
                {
                    summary.nets += 1;
                    summary.connections += net.connections.len();
                    net_kinds.insert(net.kind.clone());
                }
            }
            SketchSelection::Overflow(_) => {}
        }
    }
    if component_models.len() == 1 {
        summary.common_model = component_models.into_iter().next();
    }
    if net_kinds.len() == 1 {
        summary.common_net_kind = net_kinds.into_iter().next();
    }
    summary
}

fn quick_toolbar_position(
    canvas: egui::Rect,
    selection_bounds: egui::Rect,
    toolbar_size: egui::Vec2,
) -> egui::Pos2 {
    let gap = 8.0;
    let min_x = canvas.left() + gap;
    let max_x = (canvas.right() - toolbar_size.x - gap).max(min_x);
    let x = (selection_bounds.center().x - toolbar_size.x / 2.0).clamp(min_x, max_x);
    let above_y = selection_bounds.top() - toolbar_size.y - gap;
    let below_y = selection_bounds.bottom() + gap;
    let min_y = canvas.top() + gap;
    let max_y = (canvas.bottom() - toolbar_size.y - gap).max(min_y);
    let y = if above_y >= canvas.top() + gap {
        above_y
    } else {
        below_y.min(max_y)
    };
    egui::pos2(x, y.max(min_y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::sketch::load_project_snapshot_from_yaml;

    #[test]
    fn multi_selection_summary_reports_counts_and_common_properties() {
        let yaml = "project:
  name: gui_multi_selection_summary_test
  version: 0.1.0
board:
  components:
    U1:
      model: generic.ic
      pins:
        A: sig
        B: gnd
    U2:
      model: generic.ic
      pins:
        A: sig
  nets:
    sig:
      kind: digital_or_analog
    gnd:
      kind: ground
";
        let snapshot = load_project_snapshot_from_yaml(yaml).unwrap();
        let selected = [
            SketchSelection::Component("U1".to_string()),
            SketchSelection::Component("U2".to_string()),
            SketchSelection::Net("sig".to_string()),
        ]
        .into_iter()
        .collect();

        let summary = summarize_multi_selection(&snapshot, &selected);

        assert_eq!(summary.total, 3);
        assert_eq!(summary.components, 2);
        assert_eq!(summary.nets, 1);
        assert_eq!(summary.pins, 3);
        assert_eq!(summary.connections, 2);
        assert_eq!(summary.common_model.as_deref(), Some("generic.ic"));
        assert_eq!(
            summary.common_net_kind.as_deref(),
            Some("digital_or_analog")
        );
    }

    #[test]
    fn quick_toolbar_position_stays_inside_canvas() {
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 260.0));
        let selection = egui::Rect::from_min_size(egui::pos2(16.0, 20.0), egui::vec2(80.0, 60.0));
        let size = egui::vec2(180.0, 34.0);

        let pos = quick_toolbar_position(canvas, selection, size);

        assert!(pos.x >= canvas.left());
        assert!(pos.x + size.x <= canvas.right());
        assert!(pos.y >= canvas.top());
        assert!(pos.y + size.y <= canvas.bottom());
        assert!(pos.y > selection.bottom());
    }

    #[test]
    fn quick_toolbar_position_handles_tiny_canvas() {
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(120.0, 42.0));
        let selection = egui::Rect::from_min_size(egui::pos2(20.0, 8.0), egui::vec2(24.0, 20.0));
        let size = egui::vec2(180.0, 34.0);

        let pos = quick_toolbar_position(canvas, selection, size);

        assert!(pos.x.is_finite());
        assert!(pos.y.is_finite());
        assert!(pos.x >= canvas.left());
        assert!(pos.y >= canvas.top());
    }
}
