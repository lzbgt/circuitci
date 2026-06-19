use super::waveform_trigger::ScopeTriggerEvent;
use super::{
    ScopeCursorLegendRow, WaveformTraceRef, format_time_s, format_value, scope_cursor_legend_rows,
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
        if self.waveform_measurement_snapshots.is_empty() {
            return;
        }
        let mut remove_index = None;
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
                        .num_columns(8)
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
                                if ui.small_button("Delete").clicked() {
                                    remove_index = Some(index);
                                }
                                ui.end_row();
                            }
                        });
                });
        });
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
}

fn cursor_snapshot(
    label: &str,
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
