use super::waveform_plot::{
    WaveformPlotCursors, WaveformPlotLaneMode, WaveformPlotTrigger, WaveformPlotView,
    clamp_value_window, clamp_waveform_time_window, draw_waveform_plot_sized,
    expanded_value_bounds, scope_plot_size, scope_visible_styled_trace_refs,
    scope_visible_trace_refs, valid_waveform_trace, waveform_time_window_for_view,
    waveform_trace_bounds_in_window, zoom_time_window,
};
use super::{format_time_s, format_value, scope_cursor_legend_rows, waveform_time_range_for_view};
use crate::gui::{CircuitCiApp, WaveformViewWindow};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn waveform_scope_plot(&mut self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
        let waveform = &self.waveforms[self.selected_waveform];
        if waveform.probes.is_empty() {
            return;
        }
        let traces = scope_visible_trace_refs(
            &self.waveforms,
            self.selected_waveform,
            self.selected_probe,
            &self.waveform_pinned_traces,
        );
        let traces = scope_visible_styled_trace_refs(&traces, &self.waveform_trace_styles);
        let visible_window = self.visible_waveform_time_window();
        let visible_value_window = self.visible_waveform_value_window();
        let trigger_events = self.selected_scope_trigger_events();
        let trigger_times_us: Vec<f64> = trigger_events.iter().map(|event| event.time_us).collect();
        let interaction = draw_waveform_plot_sized(
            ui,
            &self.waveforms,
            &traces,
            WaveformPlotCursors {
                cursor_a_us: self.waveform_cursor_a_us,
                cursor_b_us: self.waveform_cursor_b_us,
                active_drag: &mut self.waveform_cursor_drag,
                box_zoom_start: &mut self.waveform_box_zoom_start,
            },
            WaveformPlotView {
                visible_window_us: visible_window,
                visible_value_window,
                lane_mode: if self.waveform_split_trace_units {
                    WaveformPlotLaneMode::ByUnit
                } else {
                    WaveformPlotLaneMode::Shared
                },
                trigger: Some(WaveformPlotTrigger {
                    threshold: self.waveform_trigger_threshold,
                    events_us: &trigger_times_us,
                }),
            },
            &self.waveform_trace_styles,
            scope_plot_size(desired_size),
        );
        if interaction.time_window_us.is_some() || interaction.value_window.is_some() {
            if interaction.view_dragging && self.waveform_view_drag_start.is_none() {
                self.waveform_view_drag_start = Some(self.waveform_view_window());
            }
            if interaction.view_dragging {
                if let Some((start_us, end_us)) = interaction.time_window_us {
                    self.set_waveform_time_window(start_us, end_us);
                }
                if let Some((value_min, value_max)) = interaction.value_window {
                    self.set_waveform_value_window(value_min, value_max);
                }
            } else {
                self.apply_waveform_view_change(|app| {
                    if let Some((start_us, end_us)) = interaction.time_window_us {
                        app.set_waveform_time_window(start_us, end_us);
                    }
                    if let Some((value_min, value_max)) = interaction.value_window {
                        app.set_waveform_value_window(value_min, value_max);
                    }
                });
            }
        }
        if !ui.input(|input| input.pointer.primary_down()) {
            self.commit_waveform_view_drag();
        }
        if let Some(cursor_a_us) = interaction.cursor_a_us {
            self.set_waveform_cursor_a(cursor_a_us);
        }
        if let Some(cursor_b_us) = interaction.cursor_b_us {
            self.set_waveform_cursor_b(cursor_b_us);
        }
    }

    pub(super) fn waveform_scope_cursor_legend(&self, ui: &mut egui::Ui) {
        let traces = scope_visible_trace_refs(
            &self.waveforms,
            self.selected_waveform,
            self.selected_probe,
            &self.waveform_pinned_traces,
        );
        let traces = scope_visible_styled_trace_refs(&traces, &self.waveform_trace_styles);
        let rows = scope_cursor_legend_rows(
            &self.waveforms,
            &traces,
            self.waveform_cursor_a_us,
            self.waveform_cursor_b_us,
        );
        if rows.is_empty() {
            return;
        }

        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Cursor Readout");
                ui.label(format!(
                    "A {}  B {}",
                    format_time_s(self.waveform_cursor_a_us / 1e6),
                    format_time_s(self.waveform_cursor_b_us / 1e6)
                ));
            });
            egui::Grid::new("scope_cursor_readout")
                .num_columns(6)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("");
                    ui.label("Trace");
                    ui.label("A");
                    ui.label("B");
                    ui.label("Delta");
                    ui.label("Unit");
                    ui.end_row();

                    for row in &rows {
                        ui.label(if row.selected { "*" } else { " " });
                        ui.monospace(&row.label);
                        ui.monospace(format_value(row.cursor_a_value));
                        ui.monospace(format_value(row.cursor_b_value));
                        ui.monospace(format_value(row.delta_value));
                        ui.monospace(row.unit);
                        ui.end_row();
                    }
                });
        });
    }

    pub(super) fn waveform_playback_panel(&mut self, ui: &mut egui::Ui) {
        let Some((full_start_us, full_end_us)) =
            waveform_time_range_for_view(&self.waveforms, self.selected_waveform)
        else {
            return;
        };
        let (window_start_us, window_end_us) = self
            .visible_waveform_time_window()
            .unwrap_or((full_start_us, full_end_us));
        if self.waveform_cursor_a_us < window_start_us || self.waveform_cursor_a_us > window_end_us
        {
            self.waveform_cursor_a_us = window_start_us;
        }
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Simulation Time");
                if ui
                    .button(if self.waveform_playing {
                        "Pause"
                    } else {
                        "Play"
                    })
                    .clicked()
                {
                    self.waveform_playing = !self.waveform_playing;
                }
                if ui.button("Start").clicked() {
                    self.waveform_cursor_a_us = window_start_us;
                    self.waveform_cursor_b_us = window_start_us;
                    self.waveform_playing = false;
                }
                ui.add(
                    egui::Slider::new(
                        &mut self.waveform_cursor_a_us,
                        window_start_us..=window_end_us,
                    )
                    .text("time")
                    .suffix(" us")
                    .show_value(true),
                );
                self.waveform_cursor_b_us = self.waveform_cursor_a_us;
                ui.label("speed");
                ui.add(
                    egui::DragValue::new(&mut self.waveform_playback_speed)
                        .speed(0.1)
                        .range(0.1..=1000.0)
                        .suffix("x"),
                );
            });
            ui.horizontal_wrapped(|ui| {
                ui.strong("Time Window");
                if ui
                    .add_enabled(
                        !self.waveform_view_back_stack.is_empty(),
                        egui::Button::new("Back"),
                    )
                    .clicked()
                {
                    self.restore_previous_waveform_view_window();
                }
                if ui
                    .add_enabled(
                        !self.waveform_view_forward_stack.is_empty(),
                        egui::Button::new("Forward"),
                    )
                    .clicked()
                {
                    self.restore_next_waveform_view_window();
                }
                if ui.button("Fit Time").clicked() {
                    self.apply_waveform_view_change(|app| app.fit_waveform_time_window());
                }
                if ui.button("Zoom In").clicked() {
                    self.apply_waveform_view_change(|app| app.zoom_waveform_time_window(0.5));
                }
                if ui.button("Zoom Out").clicked() {
                    self.apply_waveform_view_change(|app| app.zoom_waveform_time_window(2.0));
                }
                if ui.button("Pan Left").clicked() {
                    self.apply_waveform_view_change(|app| app.pan_waveform_time_window(-0.25));
                }
                if ui.button("Pan Right").clicked() {
                    self.apply_waveform_view_change(|app| app.pan_waveform_time_window(0.25));
                }
            });
            let mut edited_start = window_start_us;
            let mut edited_end = window_end_us;
            ui.horizontal_wrapped(|ui| {
                ui.label("start");
                let start_changed = ui
                    .add(
                        egui::DragValue::new(&mut edited_start)
                            .speed(1.0)
                            .suffix(" us"),
                    )
                    .changed();
                ui.label("end");
                let end_changed = ui
                    .add(
                        egui::DragValue::new(&mut edited_end)
                            .speed(1.0)
                            .suffix(" us"),
                    )
                    .changed();
                if start_changed || end_changed {
                    self.apply_waveform_view_change(|app| {
                        app.set_waveform_time_window(edited_start, edited_end)
                    });
                }
                ui.label(format!("full {:.3}..{:.3} us", full_start_us, full_end_us));
            });
            if self.waveform_split_trace_units {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Value Scale");
                    ui.label("Split Units auto-fits each lane independently.");
                });
            } else if let Some((value_min, value_max)) = self.visible_waveform_value_window() {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Value Scale");
                    if ui.button("Fit Y").clicked() {
                        self.apply_waveform_view_change(|app| app.fit_waveform_value_window());
                    }
                    if ui.button("Y Zoom In").clicked() {
                        self.apply_waveform_view_change(|app| app.zoom_waveform_value_window(0.5));
                    }
                    if ui.button("Y Zoom Out").clicked() {
                        self.apply_waveform_view_change(|app| app.zoom_waveform_value_window(2.0));
                    }
                    if ui.button("Pan Down").clicked() {
                        self.apply_waveform_view_change(|app| app.pan_waveform_value_window(-0.25));
                    }
                    if ui.button("Pan Up").clicked() {
                        self.apply_waveform_view_change(|app| app.pan_waveform_value_window(0.25));
                    }
                });
                let mut edited_min = value_min;
                let mut edited_max = value_max;
                ui.horizontal_wrapped(|ui| {
                    ui.label("min");
                    let min_changed = ui
                        .add(
                            egui::DragValue::new(&mut edited_min)
                                .speed(((value_max - value_min).abs() / 200.0).max(1.0e-12)),
                        )
                        .changed();
                    ui.label("max");
                    let max_changed = ui
                        .add(
                            egui::DragValue::new(&mut edited_max)
                                .speed(((value_max - value_min).abs() / 200.0).max(1.0e-12)),
                        )
                        .changed();
                    if min_changed || max_changed {
                        self.apply_waveform_view_change(|app| {
                            app.set_waveform_value_window(edited_min, edited_max)
                        });
                    }
                    if let Some((data_min, data_max)) = self.waveform_data_value_window() {
                        ui.label(format!(
                            "auto {}..{}",
                            format_value(data_min),
                            format_value(data_max)
                        ));
                    }
                });
            }
            self.waveform_trigger_panel(ui);
            let value_hint = if self.waveform_split_trace_units {
                "Split Units auto-fits value lanes; drag empty plot space pans time, wheel zooms time."
            } else {
                "Drag empty plot space to pan time/value; wheel zooms time, Shift-wheel zooms value."
            };
            ui.small(format!(
                "Click or drag cursor handles to set cursor A/B; Shift-click sets B. Alt/Option-drag a box to zoom. Back/Forward restores prior scope windows. {value_hint} Trigger markers are derived from the selected trace only."
            ));
        });
    }

    pub(super) fn waveform_view_window(&self) -> WaveformViewWindow {
        WaveformViewWindow {
            time_start_us: self.waveform_window_start_us,
            time_end_us: self.waveform_window_end_us,
            value_min: self.waveform_value_min,
            value_max: self.waveform_value_max,
        }
    }

    fn restore_waveform_view_window(&mut self, window: WaveformViewWindow) {
        self.waveform_window_start_us = window.time_start_us;
        self.waveform_window_end_us = window.time_end_us;
        self.waveform_value_min = window.value_min;
        self.waveform_value_max = window.value_max;
        if let Some((start_us, end_us)) = self.visible_waveform_time_window() {
            self.waveform_cursor_a_us = self.waveform_cursor_a_us.clamp(start_us, end_us);
            self.waveform_cursor_b_us = self.waveform_cursor_b_us.clamp(start_us, end_us);
        }
        self.waveform_playing = false;
    }

    pub(super) fn apply_waveform_view_change(&mut self, change: impl FnOnce(&mut Self)) {
        let before = self.waveform_view_window();
        change(self);
        let after = self.waveform_view_window();
        self.push_waveform_view_history(before, after);
    }

    fn push_waveform_view_history(
        &mut self,
        before: WaveformViewWindow,
        after: WaveformViewWindow,
    ) {
        if before == after {
            return;
        }
        self.waveform_view_back_stack.push(before);
        const MAX_SCOPE_VIEW_HISTORY: usize = 64;
        if self.waveform_view_back_stack.len() > MAX_SCOPE_VIEW_HISTORY {
            self.waveform_view_back_stack.remove(0);
        }
        self.waveform_view_forward_stack.clear();
    }

    pub(super) fn commit_waveform_view_drag(&mut self) {
        let Some(before) = self.waveform_view_drag_start.take() else {
            return;
        };
        let after = self.waveform_view_window();
        self.push_waveform_view_history(before, after);
    }

    pub(super) fn restore_previous_waveform_view_window(&mut self) {
        let Some(previous) = self.waveform_view_back_stack.pop() else {
            return;
        };
        let current = self.waveform_view_window();
        self.restore_waveform_view_window(previous);
        self.waveform_view_forward_stack.push(current);
    }

    pub(super) fn restore_next_waveform_view_window(&mut self) {
        let Some(next) = self.waveform_view_forward_stack.pop() else {
            return;
        };
        let current = self.waveform_view_window();
        self.restore_waveform_view_window(next);
        self.waveform_view_back_stack.push(current);
    }

    pub(in crate::gui) fn clear_waveform_view_history(&mut self) {
        self.waveform_view_back_stack.clear();
        self.waveform_view_forward_stack.clear();
        self.waveform_view_drag_start = None;
    }

    pub(in crate::gui) fn fit_waveform_time_window(&mut self) {
        if let Some((start_us, end_us)) =
            waveform_time_range_for_view(&self.waveforms, self.selected_waveform)
        {
            self.waveform_window_start_us = None;
            self.waveform_window_end_us = None;
            self.waveform_cursor_a_us = self.waveform_cursor_a_us.clamp(start_us, end_us);
            self.waveform_cursor_b_us = self.waveform_cursor_b_us.clamp(start_us, end_us);
        }
    }

    fn fit_waveform_value_window(&mut self) {
        self.waveform_value_min = None;
        self.waveform_value_max = None;
    }

    pub(super) fn visible_waveform_time_window(&self) -> Option<(f64, f64)> {
        waveform_time_window_for_view(
            &self.waveforms,
            self.selected_waveform,
            self.waveform_window_start_us,
            self.waveform_window_end_us,
        )
    }

    pub(super) fn set_waveform_time_window(&mut self, start_us: f64, end_us: f64) {
        let Some((start_us, end_us)) =
            clamp_waveform_time_window(&self.waveforms, self.selected_waveform, start_us, end_us)
        else {
            return;
        };
        self.waveform_window_start_us = Some(start_us);
        self.waveform_window_end_us = Some(end_us);
        self.waveform_cursor_a_us = self.waveform_cursor_a_us.clamp(start_us, end_us);
        self.waveform_cursor_b_us = self.waveform_cursor_b_us.clamp(start_us, end_us);
    }

    fn waveform_data_value_window(&self) -> Option<(f64, f64)> {
        let traces = scope_visible_trace_refs(
            &self.waveforms,
            self.selected_waveform,
            self.selected_probe,
            &self.waveform_pinned_traces,
        );
        let traces = scope_visible_styled_trace_refs(&traces, &self.waveform_trace_styles);
        let (start_us, end_us) = self.visible_waveform_time_window()?;
        let (value_min, value_max) = waveform_trace_bounds_in_window(
            &self.waveforms,
            &traces,
            start_us / 1e6,
            end_us / 1e6,
        )?;
        expanded_value_bounds(value_min, value_max)
    }

    pub(super) fn visible_waveform_value_window(&self) -> Option<(f64, f64)> {
        let (data_min, data_max) = self.waveform_data_value_window()?;
        match (self.waveform_value_min, self.waveform_value_max) {
            (Some(value_min), Some(value_max)) => {
                clamp_value_window(data_min, data_max, value_min, value_max)
            }
            _ => Some((data_min, data_max)),
        }
    }

    pub(super) fn set_waveform_value_window(&mut self, value_min: f64, value_max: f64) {
        let Some((data_min, data_max)) = self.waveform_data_value_window() else {
            return;
        };
        let Some((value_min, value_max)) =
            clamp_value_window(data_min, data_max, value_min, value_max)
        else {
            return;
        };
        self.waveform_value_min = Some(value_min);
        self.waveform_value_max = Some(value_max);
    }

    pub(super) fn set_waveform_cursor_a(&mut self, cursor_us: f64) {
        if let Some((start_us, end_us)) = self.visible_waveform_time_window() {
            self.waveform_cursor_a_us = cursor_us.clamp(start_us, end_us);
            self.waveform_playing = false;
        }
    }

    fn set_waveform_cursor_b(&mut self, cursor_us: f64) {
        if let Some((start_us, end_us)) = self.visible_waveform_time_window() {
            self.waveform_cursor_b_us = cursor_us.clamp(start_us, end_us);
            self.waveform_playing = false;
        }
    }

    fn zoom_waveform_time_window(&mut self, scale: f64) {
        let Some((start_us, end_us)) = self.visible_waveform_time_window() else {
            return;
        };
        let center_us = self.waveform_cursor_a_us.clamp(start_us, end_us);
        let (new_start, new_end) = zoom_time_window(start_us, end_us, center_us, scale);
        self.set_waveform_time_window(new_start, new_end);
    }

    fn zoom_waveform_value_window(&mut self, scale: f64) {
        let Some((value_min, value_max)) = self.visible_waveform_value_window() else {
            return;
        };
        let center = (value_min + value_max) * 0.5;
        let (new_min, new_max) = zoom_time_window(value_min, value_max, center, scale);
        self.set_waveform_value_window(new_min, new_max);
    }

    fn pan_waveform_time_window(&mut self, span_fraction: f64) {
        let Some((start_us, end_us)) = self.visible_waveform_time_window() else {
            return;
        };
        let delta_us = (end_us - start_us) * span_fraction;
        self.set_waveform_time_window(start_us + delta_us, end_us + delta_us);
    }

    fn pan_waveform_value_window(&mut self, span_fraction: f64) {
        let Some((value_min, value_max)) = self.visible_waveform_value_window() else {
            return;
        };
        let delta = (value_max - value_min) * span_fraction;
        self.set_waveform_value_window(value_min + delta, value_max + delta);
    }

    pub(super) fn selected_scope_trace(&self) -> super::WaveformTraceRef {
        super::WaveformTraceRef {
            waveform_index: self.selected_waveform,
            probe_index: self.selected_probe,
        }
    }

    pub(super) fn selected_scope_trace_pinned(&self) -> bool {
        let selected = self.selected_scope_trace();
        self.waveform_pinned_traces.contains(&selected)
    }

    pub(super) fn toggle_selected_scope_trace_pin(&mut self) {
        let selected = self.selected_scope_trace();
        if !valid_waveform_trace(&self.waveforms, selected) {
            return;
        }
        if let Some(index) = self
            .waveform_pinned_traces
            .iter()
            .position(|trace| *trace == selected)
        {
            self.waveform_pinned_traces.remove(index);
            self.status = "Selected scope trace unpinned.".to_string();
        } else {
            self.waveform_pinned_traces.push(selected);
            let label = self.waveforms[selected.waveform_index].probes[selected.probe_index]
                .label
                .clone();
            self.status = format!("Pinned scope trace {label} for comparison.");
        }
    }

    pub(super) fn prune_scope_trace_pins(&mut self) {
        let waveforms = &self.waveforms;
        self.waveform_pinned_traces
            .retain(|trace| valid_waveform_trace(waveforms, *trace));
        self.waveform_trace_styles
            .retain(|style| valid_waveform_trace(waveforms, style.trace));
    }
}
