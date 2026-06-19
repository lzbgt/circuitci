use eframe::egui;

use super::CircuitCiApp;
use super::sketch::compact_label;
use super::sketch_canvas_hits::RuntimeScopeActivityTarget;

impl CircuitCiApp {
    pub(super) fn sketch_runtime_scope_activity_legend(
        &mut self,
        ui: &egui::Ui,
        canvas_rect: egui::Rect,
        targets: &[RuntimeScopeActivityTarget],
    ) {
        if targets.is_empty() {
            return;
        }
        let visible_indexes =
            runtime_scope_activity_visible_indexes(targets, &self.sketch_runtime_scope_filter);
        let legend_size = egui::vec2(328.0, 252.0);
        let pos = canvas_rect.right_top()
            + egui::vec2(
                -(legend_size.x + 12.0),
                12.0 + self.sketch_hierarchy_focus.as_ref().map_or(0.0, |_| 30.0),
            );
        egui::Area::new(egui::Id::new("sketch_runtime_scope_activity_legend"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(legend_size.x);
                    ui.horizontal(|ui| {
                        let (swatch_rect, _) = ui
                            .allocate_exact_size(egui::vec2(44.0, 18.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            swatch_rect,
                            4.0,
                            egui::Color32::from_rgb(20, 70, 55),
                        );
                        ui.painter().rect_stroke(
                            swatch_rect,
                            4.0,
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 235, 170)),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            swatch_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "scope",
                            egui::FontId::monospace(10.5),
                            egui::Color32::WHITE,
                        );
                        ui.strong("Scope Activity");
                        ui.label(format!(
                            "{} of {} targets",
                            visible_indexes.len(),
                            targets.len()
                        ));
                    });
                    ui.checkbox(
                        &mut self.sketch_runtime_scope_overlay_visible,
                        "Show on schematic",
                    )
                    .on_hover_text(
                        "Show runtime scope tinting and clickable scope chips for loaded waveform traces.",
                    );
                    ui.horizontal(|ui| {
                        ui.label("Find");
                        let response = ui.add_sized(
                            egui::vec2(188.0, 20.0),
                            egui::TextEdit::singleline(&mut self.sketch_runtime_scope_filter)
                                .hint_text("trace, target, scenario"),
                        );
                        if response.changed() {
                            self.sketch_runtime_scope_filter =
                                self.sketch_runtime_scope_filter.trim_start().to_string();
                        }
                        if ui
                            .add_enabled(
                                !self.sketch_runtime_scope_filter.trim().is_empty(),
                                egui::Button::new("Clear"),
                            )
                            .clicked()
                        {
                            self.sketch_runtime_scope_filter.clear();
                        }
                    });
                    ui.separator();
                    if visible_indexes.is_empty() {
                        ui.small("No loaded schematic trace matches the current filter.");
                        return;
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("sketch_runtime_scope_activity_rows")
                        .max_height(142.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for index in visible_indexes {
                                let row = &targets[index];
                                let button_label =
                                    format!("{} · {}", row.target.probe_name, row.label);
                                if ui
                                    .add_sized(
                                        egui::vec2(292.0, 20.0),
                                        egui::Button::new(compact_label(&button_label, 48)),
                                    )
                                    .on_hover_text(format!(
                                        "{} in {}",
                                        row.target.probe_name, row.target.scenario_name
                                    ))
                                    .clicked()
                                {
                                    self.open_scope_probe_target(row.target.clone());
                                }
                            }
                        });
                });
            });
    }
}

pub(super) fn runtime_scope_activity_visible_indexes(
    targets: &[RuntimeScopeActivityTarget],
    query: &str,
) -> Vec<usize> {
    let query = query.trim().to_ascii_lowercase();
    targets
        .iter()
        .enumerate()
        .filter_map(|(index, target)| {
            (query.is_empty() || runtime_scope_activity_matches(target, &query)).then_some(index)
        })
        .collect()
}

fn runtime_scope_activity_matches(target: &RuntimeScopeActivityTarget, query: &str) -> bool {
    target.label.to_ascii_lowercase().contains(query)
        || target
            .target
            .probe_name
            .to_ascii_lowercase()
            .contains(query)
        || target
            .target
            .scenario_name
            .to_ascii_lowercase()
            .contains(query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::ScopeProbeTarget;

    fn activity(label: &str, probe: &str, scenario: &str) -> RuntimeScopeActivityTarget {
        RuntimeScopeActivityTarget {
            label: label.to_string(),
            target: ScopeProbeTarget {
                probe_name: probe.to_string(),
                scenario_name: scenario.to_string(),
            },
        }
    }

    #[test]
    fn runtime_scope_activity_filter_matches_label_probe_and_scenario() {
        let targets = vec![
            activity("R1", "i(R1)", "transient"),
            activity("out", "v(out)", "ne555_astable"),
            activity("timing", "v(timing)", "startup"),
        ];

        assert_eq!(
            runtime_scope_activity_visible_indexes(&targets, ""),
            vec![0, 1, 2]
        );
        assert_eq!(
            runtime_scope_activity_visible_indexes(&targets, "OUT"),
            vec![1]
        );
        assert_eq!(
            runtime_scope_activity_visible_indexes(&targets, "i("),
            vec![0]
        );
        assert_eq!(
            runtime_scope_activity_visible_indexes(&targets, "start"),
            vec![2]
        );
        assert!(runtime_scope_activity_visible_indexes(&targets, "missing").is_empty());
    }
}
