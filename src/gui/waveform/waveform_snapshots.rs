use super::waveform_plot::valid_waveform_trace;
use super::waveform_trigger::ScopeTriggerEvent;
use super::{
    ScopeCursorLegendRow, WaveformTraceRef, format_time_s, format_value, scope_cursor_legend_rows,
    waveform_time_range_for_view,
};
use crate::gui::{CircuitCiApp, ScopeMeasurementSnapshot};
use eframe::egui;

const MAX_SCOPE_SNAPSHOTS: usize = 64;

impl CircuitCiApp {
    pub(super) fn capture_scope_cursor_snapshots(&mut self) {
        let rows = self.current_scope_cursor_snapshot_rows();
        if rows.is_empty() {
            self.status = "No scope cursor measurements are available to snapshot.".to_string();
            return;
        }
        let count = rows.len();
        let label = format!("Cursor {}", self.waveform_measurement_snapshots.len() + 1);
        for row in rows {
            self.push_scope_measurement_snapshot(cursor_snapshot(
                &label,
                Some(row.trace),
                self.waveform_cursor_a_us,
                self.waveform_cursor_b_us,
                &row,
            ));
        }
        self.status = format!("Captured {count} scope cursor measurement snapshot(s).");
    }

    pub(super) fn capture_scope_trigger_snapshot(&mut self, event: ScopeTriggerEvent) {
        let Some(trace_label) = self.selected_scope_trace_label() else {
            self.status = "No selected scope trace is available to snapshot.".to_string();
            return;
        };
        let unit = self
            .waveforms
            .get(self.selected_waveform)
            .and_then(|waveform| waveform.probes.get(self.selected_probe))
            .map(|probe| super::probe_unit(&probe.label))
            .unwrap_or("");
        let snapshot = ScopeMeasurementSnapshot {
            label: format!("Trigger {}", self.waveform_measurement_snapshots.len() + 1),
            source: "trigger".to_string(),
            trace: Some(self.selected_scope_trace()),
            trace_label,
            time_a_us: Some(event.time_us),
            time_b_us: None,
            value_a: Some(event.value),
            value_b: None,
            delta_value: None,
            event_edge: Some(event.edge.label().to_string()),
            unit: unit.to_string(),
        };
        self.push_scope_measurement_snapshot(snapshot);
        self.status = format!(
            "Captured {} trigger snapshot at {}.",
            event.edge.label(),
            format_time_s(event.time_us / 1e6)
        );
    }

    pub(super) fn waveform_measurement_snapshots_panel(&mut self, ui: &mut egui::Ui) {
        self.prune_scope_measurement_snapshots();
        if self.waveform_measurement_snapshots.is_empty() {
            return;
        }
        let mut remove_index = None;
        let mut jump_index = None;
        let mut focus_index = None;
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Measurement Snapshots");
                ui.label("runtime only");
                if ui.button("Clear").clicked() {
                    self.waveform_measurement_snapshots.clear();
                    self.status = "Cleared scope measurement snapshots.".to_string();
                }
            });
            egui::ScrollArea::vertical()
                .max_height(132.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new("scope_measurement_snapshots")
                        .num_columns(10)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Label");
                            ui.label("Source");
                            ui.label("Trace");
                            ui.label("A/Event");
                            ui.label("B");
                            ui.label("Delta");
                            ui.label("Unit");
                            ui.label("");
                            ui.label("");
                            ui.label("");
                            ui.end_row();

                            for (index, snapshot) in
                                self.waveform_measurement_snapshots.iter().enumerate()
                            {
                                ui.monospace(&snapshot.label);
                                ui.monospace(snapshot_source(snapshot));
                                ui.monospace(&snapshot.trace_label);
                                ui.monospace(snapshot_value_a(snapshot));
                                ui.monospace(snapshot_value_b(snapshot));
                                ui.monospace(snapshot_delta(snapshot));
                                ui.monospace(&snapshot.unit);
                                let can_jump = snapshot.trace.is_some_and(|trace| {
                                    valid_waveform_trace(&self.waveforms, trace)
                                });
                                if ui
                                    .add_enabled(can_jump, egui::Button::new("Jump"))
                                    .clicked()
                                {
                                    jump_index = Some(index);
                                }
                                if ui
                                    .add_enabled(can_jump, egui::Button::new("Focus"))
                                    .clicked()
                                {
                                    focus_index = Some(index);
                                }
                                if ui.small_button("Delete").clicked() {
                                    remove_index = Some(index);
                                }
                                ui.end_row();
                            }
                        });
                });
        });
        if let Some(index) = jump_index {
            self.activate_scope_measurement_snapshot(index, false);
        }
        if let Some(index) = focus_index {
            self.activate_scope_measurement_snapshot(index, true);
        }
        if let Some(index) = remove_index {
            self.waveform_measurement_snapshots.remove(index);
            self.status = "Deleted scope measurement snapshot.".to_string();
        }
    }

    fn current_scope_cursor_snapshot_rows(&self) -> Vec<ScopeCursorLegendRow> {
        let traces = self.current_scope_compare_traces();
        let traces = super::waveform_plot::scope_visible_styled_trace_refs(
            &traces,
            &self.waveform_trace_styles,
        );
        scope_cursor_legend_rows(
            &self.waveforms,
            &traces,
            self.waveform_cursor_a_us,
            self.waveform_cursor_b_us,
        )
    }

    fn selected_scope_trace_label(&self) -> Option<String> {
        trace_label(
            &self.waveforms,
            WaveformTraceRef {
                waveform_index: self.selected_waveform,
                probe_index: self.selected_probe,
            },
        )
    }

    fn push_scope_measurement_snapshot(&mut self, snapshot: ScopeMeasurementSnapshot) {
        self.waveform_measurement_snapshots.push(snapshot);
        let overflow = self
            .waveform_measurement_snapshots
            .len()
            .saturating_sub(MAX_SCOPE_SNAPSHOTS);
        if overflow > 0 {
            self.waveform_measurement_snapshots.drain(0..overflow);
        }
    }

    pub(super) fn activate_scope_measurement_snapshot(
        &mut self,
        index: usize,
        focus_schematic: bool,
    ) -> bool {
        let Some(snapshot) = self.waveform_measurement_snapshots.get(index).cloned() else {
            return false;
        };
        let Some(trace) = snapshot.trace else {
            self.status = "Scope measurement snapshot is not linked to a loaded trace.".to_string();
            return false;
        };
        if !valid_waveform_trace(&self.waveforms, trace) {
            self.status = "Scope measurement snapshot trace is no longer loaded.".to_string();
            return false;
        }
        self.selected_waveform = trace.waveform_index;
        self.selected_probe = trace.probe_index;
        self.waveform_math_left = trace.probe_index;
        self.waveform_math_right = trace.probe_index;
        self.waveform_playing = false;
        if let Some(time_us) = snapshot.time_a_us {
            self.set_waveform_cursor_a(time_us);
        }
        if let Some(time_us) = snapshot.time_b_us {
            self.set_waveform_cursor_b(time_us);
        }
        self.apply_waveform_view_change(|app| {
            app.restore_scope_snapshot_time_window(&snapshot);
        });
        if focus_schematic {
            if self.focus_selected_scope_schematic_context_silent() {
                self.status = format!(
                    "Focused schematic context for scope snapshot {}.",
                    snapshot.label
                );
            } else {
                self.status = format!(
                    "Restored scope snapshot {}, but no schematic probe context matched.",
                    snapshot.label
                );
            }
        } else {
            self.status = format!("Restored scope snapshot {}.", snapshot.label);
        }
        true
    }

    pub(super) fn shift_scope_measurement_snapshots_after_probe_removal(
        &mut self,
        waveform_index: usize,
        removed_probe_index: usize,
    ) {
        self.waveform_measurement_snapshots.retain_mut(|snapshot| {
            let Some(trace) = &mut snapshot.trace else {
                return true;
            };
            if trace.waveform_index != waveform_index {
                return true;
            }
            if trace.probe_index == removed_probe_index {
                return false;
            }
            if trace.probe_index > removed_probe_index {
                trace.probe_index -= 1;
            }
            true
        });
    }

    fn prune_scope_measurement_snapshots(&mut self) {
        let waveforms = &self.waveforms;
        self.waveform_measurement_snapshots.retain(|snapshot| {
            snapshot
                .trace
                .is_some_and(|trace| valid_waveform_trace(waveforms, trace))
        });
    }

    fn restore_scope_snapshot_time_window(&mut self, snapshot: &ScopeMeasurementSnapshot) {
        let Some((full_start_us, full_end_us)) =
            waveform_time_range_for_view(&self.waveforms, self.selected_waveform)
        else {
            return;
        };
        let times = snapshot_times(snapshot);
        if times.is_empty() {
            return;
        }
        let min_time = times.iter().copied().fold(f64::INFINITY, f64::min);
        let max_time = times.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if !min_time.is_finite() || !max_time.is_finite() {
            return;
        }
        let full_span = (full_end_us - full_start_us).max(1.0e-9);
        let selected_span = (max_time - min_time).abs();
        let span = if selected_span > 1.0e-9 {
            selected_span * 1.30
        } else {
            full_span * 0.10
        }
        .clamp(full_span * 0.002, full_span);
        let center = (min_time + max_time) * 0.5;
        self.set_waveform_time_window(center - span * 0.5, center + span * 0.5);
    }
}

fn cursor_snapshot(
    label: &str,
    trace: Option<WaveformTraceRef>,
    cursor_a_us: f64,
    cursor_b_us: f64,
    row: &ScopeCursorLegendRow,
) -> ScopeMeasurementSnapshot {
    ScopeMeasurementSnapshot {
        label: label.to_string(),
        source: if row.selected {
            "cursor selected".to_string()
        } else {
            "cursor pinned".to_string()
        },
        trace,
        trace_label: row.label.clone(),
        time_a_us: Some(cursor_a_us),
        time_b_us: Some(cursor_b_us),
        value_a: Some(row.cursor_a_value),
        value_b: Some(row.cursor_b_value),
        delta_value: Some(row.delta_value),
        event_edge: None,
        unit: row.unit.to_string(),
    }
}

fn trace_label(waveforms: &[super::WaveformView], trace: WaveformTraceRef) -> Option<String> {
    let waveform = waveforms.get(trace.waveform_index)?;
    let probe = waveform.probes.get(trace.probe_index)?;
    let probe_label = probe.expression.as_deref().unwrap_or(&probe.label);
    Some(if waveforms.len() > 1 {
        format!("{} / {probe_label}", waveform.label)
    } else {
        probe_label.to_string()
    })
}

fn snapshot_source(snapshot: &ScopeMeasurementSnapshot) -> String {
    match snapshot.event_edge.as_deref() {
        Some(edge) => format!("{} {edge}", snapshot.source),
        None => snapshot.source.clone(),
    }
}

fn snapshot_value_a(snapshot: &ScopeMeasurementSnapshot) -> String {
    match (snapshot.time_a_us, snapshot.value_a) {
        (Some(time_us), Some(value)) => {
            format!("{} @ {}", format_value(value), format_time_s(time_us / 1e6))
        }
        _ => "-".to_string(),
    }
}

fn snapshot_value_b(snapshot: &ScopeMeasurementSnapshot) -> String {
    match (snapshot.time_b_us, snapshot.value_b) {
        (Some(time_us), Some(value)) => {
            format!("{} @ {}", format_value(value), format_time_s(time_us / 1e6))
        }
        _ => "-".to_string(),
    }
}

fn snapshot_delta(snapshot: &ScopeMeasurementSnapshot) -> String {
    snapshot
        .delta_value
        .map(format_value)
        .unwrap_or_else(|| "-".to_string())
}

fn snapshot_times(snapshot: &ScopeMeasurementSnapshot) -> Vec<f64> {
    [snapshot.time_a_us, snapshot.time_b_us]
        .into_iter()
        .flatten()
        .filter(|time_us| time_us.is_finite())
        .collect()
}
