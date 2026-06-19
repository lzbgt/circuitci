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
        let visible_rows = targets.len().min(4);
        let legend_size = egui::vec2(292.0, 62.0 + visible_rows as f32 * 23.0);
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
                        ui.label(format!("{} targets", targets.len()));
                    });
                    ui.checkbox(
                        &mut self.sketch_runtime_scope_overlay_visible,
                        "Show on schematic",
                    )
                    .on_hover_text(
                        "Show runtime scope tinting and clickable scope chips for loaded waveform traces.",
                    );
                    ui.separator();
                    for row in targets.iter().take(4) {
                        let button_label = format!("{} · {}", row.target.probe_name, row.label);
                        if ui
                            .add_sized(
                                egui::vec2(260.0, 20.0),
                                egui::Button::new(compact_label(&button_label, 40)),
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
                    if targets.len() > 4 {
                        ui.label(format!("+{} more", targets.len() - 4));
                    }
                });
            });
    }
}
