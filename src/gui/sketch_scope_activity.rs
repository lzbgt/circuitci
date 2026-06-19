use eframe::egui;

use super::sketch::compact_label;
use super::sketch_canvas_hits::RuntimeScopeActivityTarget;
use super::waveform::{
    RuntimeScopeProbeEdgeStep, ScopeSnapshotSourceFilter, WaveformView,
    runtime_scope_probe_edge_jump, runtime_scope_probe_frequency_label,
    runtime_scope_probe_sample_label, runtime_scope_probe_sparkline_points, scope_snapshots_csv,
    scope_snapshots_markdown, waveform_time_range_for_view,
};
use super::{CircuitCiApp, ScopeMeasurementSnapshot, Stage};

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
        let legend_size = egui::vec2(328.0, 292.0);
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
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(
                            &mut self.sketch_runtime_scope_overlay_visible,
                            "Show on schematic",
                        )
                        .on_hover_text(
                            "Show runtime scope tinting and clickable scope chips for loaded waveform traces.",
                        );
                        let activity_snapshot_count =
                            scope_activity_snapshot_count(&self.waveform_measurement_snapshots);
                        ui.small(scope_activity_snapshot_status(
                            &self.waveform_measurement_snapshots,
                        ))
                        .on_hover_text(
                            "Scope Activity Snap rows are reportable measurement snapshots.",
                        );
                        if ui
                            .add_enabled(
                                activity_snapshot_count > 0,
                                egui::Button::new("Open Snapshots"),
                            )
                            .on_hover_text(
                                "Open Scopes with measurement snapshots filtered to Scope Activity samples.",
                            )
                            .clicked()
                        {
                            self.open_scope_activity_snapshots_from_sketch();
                        }
                        if ui
                            .add_enabled(
                                activity_snapshot_count > 0,
                                egui::Button::new("Clear Activity Snapshots"),
                            )
                            .on_hover_text(
                                "Clear only Scope Activity measurement snapshots; keep cursor, trigger, and region snapshots.",
                            )
                            .clicked()
                        {
                            self.clear_scope_activity_snapshots_from_sketch();
                        }
                        if ui
                            .add_enabled(
                                !visible_indexes.is_empty(),
                                egui::Button::new("Snap Visible"),
                            )
                            .on_hover_text(
                                "Capture current Cursor A samples for every visible Scope Activity trace.",
                            )
                            .clicked()
                        {
                            self.capture_visible_scope_activity_snapshots_from_sketch(
                                targets,
                                &visible_indexes,
                            );
                        }
                        if ui
                            .add_enabled(
                                !visible_indexes.is_empty(),
                                egui::Button::new("Freq Visible"),
                            )
                            .on_hover_text(
                                "Capture dominant frequency and period snapshots for every visible Scope Activity trace.",
                            )
                            .clicked()
                        {
                            self.capture_visible_scope_activity_frequency_snapshots_from_sketch(
                                targets,
                                &visible_indexes,
                            );
                        }
                        if ui
                            .add_enabled(
                                !visible_indexes.is_empty(),
                                egui::Button::new("Copy CSV"),
                            )
                            .on_hover_text(
                                "Copy visible Scope Activity sample and frequency rows as CSV without adding snapshots.",
                            )
                            .clicked()
                        {
                            self.copy_visible_scope_activity_observations_csv(
                                ui.ctx(),
                                targets,
                                &visible_indexes,
                            );
                        }
                        if ui
                            .add_enabled(
                                !visible_indexes.is_empty(),
                                egui::Button::new("Copy MD"),
                            )
                            .on_hover_text(
                                "Copy visible Scope Activity sample and frequency rows as Markdown without adding snapshots.",
                            )
                            .clicked()
                        {
                            self.copy_visible_scope_activity_observations_markdown(
                                ui.ctx(),
                                targets,
                                &visible_indexes,
                            );
                        }
                        ui.add_enabled_ui(!visible_indexes.is_empty(), |ui| {
                            ui.menu_button("Bundle Visible", |ui| {
                                if ui.button("Export").clicked() {
                                    self.export_visible_scope_activity_report_bundle(
                                        targets,
                                        &visible_indexes,
                                    );
                                    ui.close();
                                }
                                if ui.button("Export + Open").clicked() {
                                    self.export_visible_scope_activity_report_bundle_and_open(
                                        targets,
                                        &visible_indexes,
                                    );
                                    ui.close();
                                }
                            })
                            .response
                            .on_hover_text(
                                "Export visible Scope Activity sample and frequency rows as a focused report bundle.",
                            );
                        });
                    });
                    let mut load_compare_preset = None;
                    let mut delete_compare_preset = None;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "{} pinned compare trace(s)",
                            self.waveform_pinned_traces.len()
                        ));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.waveform_trace_preset_name)
                                .desired_width(96.0)
                                .hint_text("set name"),
                        )
                        .on_hover_text(
                            "Optional name for the saved Scope Activity compare set.",
                        );
                        if ui
                            .add_enabled(
                                !self.waveform_pinned_traces.is_empty(),
                                egui::Button::new("Open Compare"),
                            )
                            .on_hover_text(
                                "Open Scopes with the currently pinned comparison traces visible.",
                            )
                            .clicked()
                        {
                            self.open_pinned_scope_compare();
                        }
                        if ui
                            .add_enabled(
                                !self.waveform_pinned_traces.is_empty(),
                                egui::Button::new("Save Compare"),
                            )
                            .on_hover_text(
                                "Save the currently pinned Scope Activity traces as a reusable Scopes compare set.",
                            )
                            .clicked()
                        {
                            self.save_pinned_scope_compare_from_sketch();
                        }
                        ui.menu_button(
                            format!("Saved ({})", self.waveform_trace_presets.len()),
                            |ui| {
                                if self.waveform_trace_presets.is_empty() {
                                    ui.label("No saved compare sets.");
                                }
                                for index in 0..self.waveform_trace_presets.len() {
                                    let name = self
                                        .scope_compare_preset_name_at(index)
                                        .unwrap_or("unnamed")
                                        .to_string();
                                    ui.horizontal(|ui| {
                                        if ui.button(&name).clicked() {
                                            load_compare_preset = Some(index);
                                            ui.close();
                                        }
                                        if ui.small_button("Delete").clicked() {
                                            delete_compare_preset = Some(index);
                                            ui.close();
                                        }
                                    });
                                }
                            },
                        )
                        .response
                        .on_hover_text(
                            "Load or delete reusable Scopes compare sets from the schematic.",
                        );
                        if ui
                            .add_enabled(
                                !self.waveform_pinned_traces.is_empty(),
                                egui::Button::new("Clear Pins"),
                            )
                            .on_hover_text(
                                "Remove every trace pinned from Scope Activity for comparison.",
                            )
                            .clicked()
                        {
                            self.clear_scope_compare_pins_from_sketch();
                        }
                    });
                    if let Some(index) = delete_compare_preset {
                        self.delete_scope_compare_preset_from_sketch(index);
                    }
                    if let Some(index) = load_compare_preset {
                        self.load_scope_compare_preset_from_sketch(index);
                    }
                    if let Some(range) =
                        runtime_scope_activity_cursor_range_us(&self.waveforms, self.selected_waveform)
                    {
                        let clamped =
                            clamp_runtime_scope_activity_cursor_us(self.waveform_cursor_a_us, range);
                        if (clamped - self.waveform_cursor_a_us).abs() > f64::EPSILON {
                            self.waveform_cursor_a_us = clamped;
                        }
                        ui.horizontal(|ui| {
                            ui.label("Cursor A");
                            let mut cursor_us = self.waveform_cursor_a_us;
                            let slider_changed = ui
                                .add_sized(
                                    egui::vec2(150.0, 18.0),
                                    egui::Slider::new(&mut cursor_us, range.0..=range.1)
                                        .show_value(false),
                                )
                                .changed();
                            let drag_changed = ui
                                .add(
                                    egui::DragValue::new(&mut cursor_us)
                                        .speed(((range.1 - range.0) / 200.0).max(0.001))
                                        .range(range.0..=range.1)
                                        .suffix(" us"),
                                )
                                .changed();
                            if slider_changed || drag_changed {
                                self.waveform_cursor_a_us =
                                    clamp_runtime_scope_activity_cursor_us(cursor_us, range);
                                self.waveform_playing = false;
                            }
                        });
                    }
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
                                ui.horizontal(|ui| {
                                    if ui
                                        .add_sized(
                                            egui::vec2(132.0, 20.0),
                                            egui::Button::new(compact_label(&button_label, 26)),
                                        )
                                        .on_hover_text(format!(
                                            "{} in {}",
                                            row.target.probe_name, row.target.scenario_name
                                        ))
                                        .clicked()
                                    {
                                        self.open_scope_probe_target(row.target.clone());
                                    }
                                    let pinned =
                                        self.scope_probe_target_pinned_for_compare(&row.target);
                                    if ui
                                        .button(if pinned { "Unpin" } else { "Pin" })
                                        .on_hover_text(
                                            if pinned {
                                                "Remove this trace from the Scopes compare overlay."
                                            } else {
                                                "Pin this loaded trace into the Scopes compare overlay."
                                            },
                                        )
                                        .clicked()
                                    {
                                        if pinned {
                                            self.unpin_scope_probe_target_for_compare(
                                                row.target.clone(),
                                            );
                                        } else {
                                            self.pin_scope_probe_target_for_compare(
                                                row.target.clone(),
                                            );
                                        }
                                    }
                                    let previous_edge = runtime_scope_probe_edge_jump(
                                        &self.waveforms,
                                        self.selected_waveform,
                                        self.waveform_cursor_a_us,
                                        &row.target,
                                        RuntimeScopeProbeEdgeStep::Previous,
                                    );
                                    if ui
                                        .add_enabled(previous_edge.is_some(), egui::Button::new("Prev"))
                                        .on_hover_text(
                                            previous_edge
                                                .as_ref()
                                                .map(|edge| edge.label.as_str())
                                                .unwrap_or("No edge event for this trace."),
                                        )
                                        .clicked()
                                        && let Some(edge) = previous_edge
                                    {
                                        self.waveform_cursor_a_us = edge.time_us;
                                        self.waveform_playing = false;
                                        self.status = format!(
                                            "Scope Activity jumped {} to {}.",
                                            row.target.probe_name, edge.label
                                        );
                                    }
                                    let next_edge = runtime_scope_probe_edge_jump(
                                        &self.waveforms,
                                        self.selected_waveform,
                                        self.waveform_cursor_a_us,
                                        &row.target,
                                        RuntimeScopeProbeEdgeStep::Next,
                                    );
                                    if ui
                                        .add_enabled(next_edge.is_some(), egui::Button::new("Next"))
                                        .on_hover_text(
                                            next_edge
                                                .as_ref()
                                                .map(|edge| edge.label.as_str())
                                                .unwrap_or("No edge event for this trace."),
                                        )
                                        .clicked()
                                        && let Some(edge) = next_edge
                                    {
                                        self.waveform_cursor_a_us = edge.time_us;
                                        self.waveform_playing = false;
                                        self.status = format!(
                                            "Scope Activity jumped {} to {}.",
                                            row.target.probe_name, edge.label
                                        );
                                    }
                                    if ui
                                        .button("Snap")
                                        .on_hover_text(
                                            "Capture the current Scope Activity sample into Scopes measurement snapshots.",
                                        )
                                        .clicked()
                                    {
                                        self.capture_scope_activity_sample_snapshot(
                                            row.target.clone(),
                                        );
                                    }
                                    if ui
                                        .button("Freq Snap")
                                        .on_hover_text(
                                            "Capture this trace's dominant frequency and period into Scopes measurement snapshots.",
                                        )
                                        .clicked()
                                    {
                                        self.capture_scope_activity_frequency_snapshot(
                                            row.target.clone(),
                                        );
                                    }
                                    ui.menu_button("Copy", |ui| {
                                        if ui.button("CSV").clicked() {
                                            self.copy_scope_activity_target_observations_csv(
                                                ui.ctx(),
                                                &row.target,
                                            );
                                            ui.close();
                                        }
                                        if ui.button("Markdown").clicked() {
                                            self.copy_scope_activity_target_observations_markdown(
                                                ui.ctx(),
                                                &row.target,
                                            );
                                            ui.close();
                                        }
                                    })
                                    .response
                                    .on_hover_text(
                                        "Copy this trace's current sample plus frequency row.",
                                    );
                                    ui.menu_button("Bundle", |ui| {
                                        if ui.button("Export").clicked() {
                                            self.export_scope_activity_target_report_bundle(
                                                row.target.clone(),
                                            );
                                            ui.close();
                                        }
                                        if ui.button("Export + Open").clicked() {
                                            self.export_scope_activity_target_report_bundle_and_open(
                                                row.target.clone(),
                                            );
                                            ui.close();
                                        }
                                    })
                                    .response
                                    .on_hover_text(
                                        "Export this trace's sample/frequency observations as a scope report bundle.",
                                    );
                                    let sample = runtime_scope_probe_sample_label(
                                        &self.waveforms,
                                        self.selected_waveform,
                                        self.waveform_cursor_a_us,
                                        &row.target,
                                    )
                                    .unwrap_or_else(|| "sample unavailable".to_string());
                                    ui.monospace(compact_label(&sample, 20));
                                    if let Some(frequency) = runtime_scope_probe_frequency_label(
                                        &self.waveforms,
                                        self.selected_waveform,
                                        &row.target,
                                    ) {
                                        ui.monospace(compact_label(&frequency, 22));
                                    }
                                    if let Some(points) = runtime_scope_probe_sparkline_points(
                                        &self.waveforms,
                                        self.selected_waveform,
                                        &row.target,
                                        36,
                                    ) {
                                        let cursor_fraction =
                                            runtime_scope_activity_cursor_fraction(
                                                &self.waveforms,
                                                self.selected_waveform,
                                                self.waveform_cursor_a_us,
                                            );
                                        draw_runtime_scope_sparkline(ui, &points, cursor_fraction);
                                    }
                                });
                            }
                        });
                });
            });
    }

    pub(super) fn open_scope_activity_snapshots_from_sketch(&mut self) {
        let activity_snapshot_count =
            scope_activity_snapshot_count(&self.waveform_measurement_snapshots);
        if activity_snapshot_count == 0 {
            self.status = "No Scope Activity snapshots captured yet.".to_string();
            return;
        }
        self.stage = Stage::Simulation;
        self.waveform_snapshot_filter.clear();
        self.waveform_snapshot_source_filter = ScopeSnapshotSourceFilter::ScopeActivity;
        self.status = format!(
            "Showing {} in Scopes.",
            scope_activity_snapshot_status_for_count(activity_snapshot_count)
        );
    }

    pub(super) fn clear_scope_activity_snapshots_from_sketch(&mut self) -> usize {
        let before = self.waveform_measurement_snapshots.len();
        self.waveform_measurement_snapshots
            .retain(|snapshot| !snapshot.source.starts_with(SCOPE_ACTIVITY_SNAPSHOT_SOURCE));
        let removed = before.saturating_sub(self.waveform_measurement_snapshots.len());
        self.status = if removed == 0 {
            "No Scope Activity snapshots to clear.".to_string()
        } else {
            format!("Cleared {removed} Scope Activity snapshot(s).")
        };
        removed
    }

    pub(super) fn capture_visible_scope_activity_snapshots_from_sketch(
        &mut self,
        targets: &[RuntimeScopeActivityTarget],
        visible_indexes: &[usize],
    ) -> usize {
        let visible_targets = visible_indexes
            .iter()
            .filter_map(|&index| targets.get(index).map(|row| row.target.clone()))
            .collect::<Vec<_>>();
        if visible_targets.is_empty() {
            self.status = "No visible Scope Activity traces to snapshot.".to_string();
            return 0;
        }
        let requested = visible_targets.len();
        let mut captured = 0;
        for target in visible_targets {
            if self.capture_scope_activity_sample_snapshot(target) {
                captured += 1;
            }
        }
        let skipped = requested.saturating_sub(captured);
        self.status = match (captured, skipped) {
            (0, _) => "No visible Scope Activity samples could be captured.".to_string(),
            (_, 0) => format!("Captured {captured} visible Scope Activity sample snapshot(s)."),
            _ => format!(
                "Captured {captured} visible Scope Activity sample snapshot(s); {skipped} unavailable."
            ),
        };
        captured
    }

    pub(super) fn capture_visible_scope_activity_frequency_snapshots_from_sketch(
        &mut self,
        targets: &[RuntimeScopeActivityTarget],
        visible_indexes: &[usize],
    ) -> usize {
        let visible_targets = visible_indexes
            .iter()
            .filter_map(|&index| targets.get(index).map(|row| row.target.clone()))
            .collect::<Vec<_>>();
        if visible_targets.is_empty() {
            self.status = "No visible Scope Activity traces to frequency-snapshot.".to_string();
            return 0;
        }
        let requested = visible_targets.len();
        let mut captured = 0;
        for target in visible_targets {
            if self.capture_scope_activity_frequency_snapshot(target) {
                captured += 1;
            }
        }
        let skipped = requested.saturating_sub(captured);
        self.status = match (captured, skipped) {
            (0, _) => "No visible Scope Activity frequencies could be captured.".to_string(),
            (_, 0) => {
                format!("Captured {captured} visible Scope Activity frequency snapshot(s).")
            }
            _ => format!(
                "Captured {captured} visible Scope Activity frequency snapshot(s); {skipped} unavailable."
            ),
        };
        captured
    }

    pub(super) fn visible_scope_activity_observation_snapshots(
        &self,
        targets: &[RuntimeScopeActivityTarget],
        visible_indexes: &[usize],
    ) -> Vec<ScopeMeasurementSnapshot> {
        let mut rows = Vec::new();
        for &index in visible_indexes {
            let Some(target) = targets.get(index).map(|row| &row.target) else {
                continue;
            };
            self.append_scope_activity_observation_snapshots(&mut rows, target);
        }
        rows
    }

    pub(super) fn scope_activity_target_observation_snapshots(
        &self,
        target: &super::ScopeProbeTarget,
    ) -> Vec<ScopeMeasurementSnapshot> {
        let mut rows = Vec::new();
        self.append_scope_activity_observation_snapshots(&mut rows, target);
        rows
    }

    fn append_scope_activity_observation_snapshots(
        &self,
        rows: &mut Vec<ScopeMeasurementSnapshot>,
        target: &super::ScopeProbeTarget,
    ) {
        if let Ok(snapshot) = self.scope_activity_sample_snapshot_row(
            target,
            format!("Scope Activity {}", rows.len() + 1),
        ) {
            rows.push(snapshot);
        }
        if let Ok(snapshot) = self.scope_activity_frequency_snapshot_row(
            target,
            format!("Scope Activity Freq {}", rows.len() + 1),
        ) {
            rows.push(snapshot);
        }
    }

    fn copy_scope_activity_target_observations_csv(
        &mut self,
        ctx: &egui::Context,
        target: &super::ScopeProbeTarget,
    ) -> usize {
        let rows = self.scope_activity_target_observation_snapshots(target);
        if rows.is_empty() {
            self.status = format!(
                "No Scope Activity observations are available to copy for {}.",
                target.probe_name
            );
            return 0;
        }
        let count = rows.len();
        ctx.copy_text(scope_snapshots_csv(&rows));
        self.status = format!(
            "Copied {count} Scope Activity observation row(s) for {} as CSV.",
            target.probe_name
        );
        count
    }

    fn copy_scope_activity_target_observations_markdown(
        &mut self,
        ctx: &egui::Context,
        target: &super::ScopeProbeTarget,
    ) -> usize {
        let rows = self.scope_activity_target_observation_snapshots(target);
        if rows.is_empty() {
            self.status = format!(
                "No Scope Activity observations are available to copy for {}.",
                target.probe_name
            );
            return 0;
        }
        let count = rows.len();
        ctx.copy_text(scope_snapshots_markdown(&rows));
        self.status = format!(
            "Copied {count} Scope Activity observation row(s) for {} as Markdown.",
            target.probe_name
        );
        count
    }

    pub(super) fn export_scope_activity_target_report_bundle(
        &mut self,
        target: super::ScopeProbeTarget,
    ) -> usize {
        self.export_scope_activity_target_report_bundle_impl(target, false)
    }

    pub(super) fn export_scope_activity_target_report_bundle_and_open(
        &mut self,
        target: super::ScopeProbeTarget,
    ) -> usize {
        self.export_scope_activity_target_report_bundle_impl(target, true)
    }

    fn export_scope_activity_target_report_bundle_impl(
        &mut self,
        target: super::ScopeProbeTarget,
        open_index: bool,
    ) -> usize {
        let rows = self.scope_activity_target_observation_snapshots(&target);
        if rows.is_empty() {
            self.status = format!(
                "No Scope Activity observations are available to bundle for {}.",
                target.probe_name
            );
            return 0;
        }
        let count = rows.len();
        self.export_scope_report_bundle(&rows);
        if self.status.contains("Exported scope report bundle") {
            self.status = format!(
                "Exported Scope Activity report bundle with {count} observation row(s) for {}. {}",
                target.probe_name, self.status
            );
        }
        if open_index {
            let export_status = self.status.clone();
            if let Some(bundle) = self.waveform_recent_report_bundles.first().cloned() {
                if self.open_scope_report_bundle_index(&bundle) {
                    self.status = format!("{export_status} {}", self.status);
                }
            } else {
                self.status = format!(
                    "{export_status} No recent report bundle was available to open for {}.",
                    target.probe_name
                );
            }
        }
        count
    }

    fn copy_visible_scope_activity_observations_csv(
        &mut self,
        ctx: &egui::Context,
        targets: &[RuntimeScopeActivityTarget],
        visible_indexes: &[usize],
    ) -> usize {
        let rows = self.visible_scope_activity_observation_snapshots(targets, visible_indexes);
        if rows.is_empty() {
            self.status =
                "No visible Scope Activity observations are available to copy.".to_string();
            return 0;
        }
        let count = rows.len();
        ctx.copy_text(scope_snapshots_csv(&rows));
        self.status = format!("Copied {count} visible Scope Activity observation row(s) as CSV.");
        count
    }

    fn copy_visible_scope_activity_observations_markdown(
        &mut self,
        ctx: &egui::Context,
        targets: &[RuntimeScopeActivityTarget],
        visible_indexes: &[usize],
    ) -> usize {
        let rows = self.visible_scope_activity_observation_snapshots(targets, visible_indexes);
        if rows.is_empty() {
            self.status =
                "No visible Scope Activity observations are available to copy.".to_string();
            return 0;
        }
        let count = rows.len();
        ctx.copy_text(scope_snapshots_markdown(&rows));
        self.status =
            format!("Copied {count} visible Scope Activity observation row(s) as Markdown.");
        count
    }

    pub(super) fn export_visible_scope_activity_report_bundle(
        &mut self,
        targets: &[RuntimeScopeActivityTarget],
        visible_indexes: &[usize],
    ) -> usize {
        self.export_visible_scope_activity_report_bundle_impl(targets, visible_indexes, false)
    }

    pub(super) fn export_visible_scope_activity_report_bundle_and_open(
        &mut self,
        targets: &[RuntimeScopeActivityTarget],
        visible_indexes: &[usize],
    ) -> usize {
        self.export_visible_scope_activity_report_bundle_impl(targets, visible_indexes, true)
    }

    fn export_visible_scope_activity_report_bundle_impl(
        &mut self,
        targets: &[RuntimeScopeActivityTarget],
        visible_indexes: &[usize],
        open_index: bool,
    ) -> usize {
        let rows = self.visible_scope_activity_observation_snapshots(targets, visible_indexes);
        if rows.is_empty() {
            self.status =
                "No visible Scope Activity observations are available to bundle.".to_string();
            return 0;
        }
        let count = rows.len();
        self.export_scope_report_bundle(&rows);
        if self.status.contains("Exported scope report bundle") {
            self.status = format!(
                "Exported visible Scope Activity report bundle with {count} observation row(s). {}",
                self.status
            );
        }
        if open_index {
            let export_status = self.status.clone();
            if let Some(bundle) = self.waveform_recent_report_bundles.first().cloned() {
                if self.open_scope_report_bundle_index(&bundle) {
                    self.status = format!("{export_status} {}", self.status);
                }
            } else {
                self.status = format!(
                    "{export_status} No recent visible report bundle was available to open."
                );
            }
        }
        count
    }
}

const SCOPE_ACTIVITY_SNAPSHOT_SOURCE: &str = "scope activity";

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

pub(super) fn runtime_scope_activity_cursor_range_us(
    waveforms: &[WaveformView],
    waveform_index: usize,
) -> Option<(f64, f64)> {
    waveform_time_range_for_view(waveforms, waveform_index)
        .filter(|(start_us, end_us)| end_us > start_us)
}

pub(super) fn clamp_runtime_scope_activity_cursor_us(cursor_us: f64, range: (f64, f64)) -> f64 {
    cursor_us.clamp(range.0, range.1)
}

pub(super) fn scope_activity_snapshot_count(snapshots: &[ScopeMeasurementSnapshot]) -> usize {
    snapshots
        .iter()
        .filter(|snapshot| snapshot.source.starts_with(SCOPE_ACTIVITY_SNAPSHOT_SOURCE))
        .count()
}

pub(super) fn scope_activity_snapshot_status(snapshots: &[ScopeMeasurementSnapshot]) -> String {
    scope_activity_snapshot_status_for_count(scope_activity_snapshot_count(snapshots))
}

fn scope_activity_snapshot_status_for_count(count: usize) -> String {
    match count {
        1 => "1 activity snapshot".to_string(),
        count => format!("{count} activity snapshots"),
    }
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

fn runtime_scope_activity_cursor_fraction(
    waveforms: &[WaveformView],
    waveform_index: usize,
    cursor_us: f64,
) -> Option<f32> {
    let (start_us, end_us) = runtime_scope_activity_cursor_range_us(waveforms, waveform_index)?;
    if end_us <= start_us {
        return None;
    }
    Some(((cursor_us - start_us) / (end_us - start_us)).clamp(0.0, 1.0) as f32)
}

fn draw_runtime_scope_sparkline(
    ui: &mut egui::Ui,
    points: &[(f32, f32)],
    cursor_fraction: Option<f32>,
) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(64.0, 18.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(16, 22, 27));
    painter.rect_stroke(
        rect,
        3.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(62, 78, 86)),
        egui::StrokeKind::Inside,
    );
    let plot_rect = rect.shrink2(egui::vec2(3.0, 3.0));
    painter.line_segment(
        [
            egui::pos2(plot_rect.left(), plot_rect.center().y),
            egui::pos2(plot_rect.right(), plot_rect.center().y),
        ],
        egui::Stroke::new(0.7, egui::Color32::from_rgb(41, 52, 58)),
    );
    let mapped = points
        .iter()
        .map(|(x, y)| {
            egui::pos2(
                plot_rect.left() + plot_rect.width() * x.clamp(0.0, 1.0),
                plot_rect.bottom() - plot_rect.height() * y.clamp(0.0, 1.0),
            )
        })
        .collect::<Vec<_>>();
    if mapped.len() >= 2 {
        painter.add(egui::Shape::line(
            mapped,
            egui::Stroke::new(1.25, egui::Color32::from_rgb(110, 235, 180)),
        ));
    }
    if let Some(cursor_fraction) = cursor_fraction {
        let x = plot_rect.left() + plot_rect.width() * cursor_fraction.clamp(0.0, 1.0);
        painter.line_segment(
            [
                egui::pos2(x, plot_rect.top()),
                egui::pos2(x, plot_rect.bottom()),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 204, 92)),
        );
    }
    response.on_hover_text("Loaded waveform sparkline for this schematic trace.");
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

    #[test]
    fn runtime_scope_activity_cursor_clamp_stays_inside_loaded_range() {
        let range = (1.0, 3.0);

        assert_eq!(clamp_runtime_scope_activity_cursor_us(0.0, range), 1.0);
        assert_eq!(clamp_runtime_scope_activity_cursor_us(2.0, range), 2.0);
        assert_eq!(clamp_runtime_scope_activity_cursor_us(5.0, range), 3.0);
    }

    fn snapshot(source: &str) -> ScopeMeasurementSnapshot {
        ScopeMeasurementSnapshot {
            label: String::new(),
            note: String::new(),
            source: source.to_string(),
            trace: None,
            trace_label: String::new(),
            time_a_us: None,
            time_b_us: None,
            value_a: None,
            value_b: None,
            delta_value: None,
            rms_value: None,
            event_edge: None,
            unit: String::new(),
        }
    }

    #[test]
    fn scope_activity_snapshot_status_counts_only_scope_activity_rows() {
        let snapshots = vec![
            snapshot("cursor"),
            snapshot("scope activity"),
            snapshot("trigger"),
            snapshot("scope activity frequency"),
        ];

        assert_eq!(scope_activity_snapshot_count(&snapshots), 2);
        assert_eq!(
            scope_activity_snapshot_status(&snapshots),
            "2 activity snapshots"
        );
        assert_eq!(
            scope_activity_snapshot_status(&[snapshot("scope activity")]),
            "1 activity snapshot"
        );
        assert_eq!(scope_activity_snapshot_status(&[]), "0 activity snapshots");
    }

    #[test]
    fn open_scope_activity_snapshots_sets_scopes_filter() {
        let mut app = CircuitCiApp {
            stage: Stage::Sketch,
            waveform_snapshot_filter: "stale search".to_string(),
            waveform_snapshot_source_filter: ScopeSnapshotSourceFilter::Region,
            waveform_measurement_snapshots: vec![snapshot("cursor"), snapshot("scope activity")],
            ..Default::default()
        };

        app.open_scope_activity_snapshots_from_sketch();

        assert_eq!(app.stage, Stage::Simulation);
        assert!(app.waveform_snapshot_filter.is_empty());
        assert_eq!(
            app.waveform_snapshot_source_filter,
            ScopeSnapshotSourceFilter::ScopeActivity
        );
        assert_eq!(app.status, "Showing 1 activity snapshot in Scopes.");
    }

    #[test]
    fn open_scope_activity_snapshots_without_rows_stays_put() {
        let mut app = CircuitCiApp {
            stage: Stage::Sketch,
            waveform_snapshot_source_filter: ScopeSnapshotSourceFilter::Region,
            waveform_measurement_snapshots: vec![snapshot("cursor")],
            ..Default::default()
        };

        app.open_scope_activity_snapshots_from_sketch();

        assert_eq!(app.stage, Stage::Sketch);
        assert_eq!(
            app.waveform_snapshot_source_filter,
            ScopeSnapshotSourceFilter::Region
        );
        assert_eq!(app.status, "No Scope Activity snapshots captured yet.");
    }

    #[test]
    fn clear_scope_activity_snapshots_keeps_other_sources() {
        let mut app = CircuitCiApp {
            waveform_measurement_snapshots: vec![
                snapshot("cursor selected"),
                snapshot("scope activity"),
                snapshot("region selected"),
                snapshot("scope activity frequency"),
                snapshot("trigger"),
            ],
            ..Default::default()
        };

        assert_eq!(app.clear_scope_activity_snapshots_from_sketch(), 2);

        let sources = app
            .waveform_measurement_snapshots
            .iter()
            .map(|snapshot| snapshot.source.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec!["cursor selected", "region selected", "trigger"]
        );
        assert_eq!(app.status, "Cleared 2 Scope Activity snapshot(s).");
    }

    #[test]
    fn clear_scope_activity_snapshots_reports_noop() {
        let mut app = CircuitCiApp {
            waveform_measurement_snapshots: vec![snapshot("cursor selected")],
            ..Default::default()
        };

        assert_eq!(app.clear_scope_activity_snapshots_from_sketch(), 0);
        assert_eq!(app.waveform_measurement_snapshots.len(), 1);
        assert_eq!(app.status, "No Scope Activity snapshots to clear.");
    }
}
