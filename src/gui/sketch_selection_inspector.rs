use std::collections::BTreeSet;

use eframe::egui;

use super::sketch::{ProjectSnapshot, SketchSelection};
use super::{CircuitCiApp, SketchGroupAction, SketchViewportCommand};

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
}
