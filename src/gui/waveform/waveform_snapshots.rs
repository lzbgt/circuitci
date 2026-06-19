use super::waveform_context::find_scope_probe;
use super::waveform_plot::{WaveformSnapshotMarker, valid_waveform_trace};
use super::waveform_trace_selector::shift_trace_after_waveform_removal;
use super::waveform_trigger::ScopeTriggerEvent;
use super::{
    ScopeCursorLegendRow, ScopeRegionStatsRow, WaveformTraceRef, format_time_s, format_value,
    interpolated_value, probe_unit, scope_cursor_legend_rows, waveform_time_range_for_view,
};
use crate::gui::{CircuitCiApp, ScopeMeasurementSnapshot, ScopeProbeTarget};
use eframe::egui;
use std::fs;

const MAX_SCOPE_SNAPSHOTS: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::gui) enum ScopeSnapshotSourceFilter {
    #[default]
    All,
    Cursor,
    Trigger,
    Region,
}

impl ScopeSnapshotSourceFilter {
    const ALL: [Self; 4] = [Self::All, Self::Cursor, Self::Trigger, Self::Region];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Cursor => "Cursor",
            Self::Trigger => "Trigger",
            Self::Region => "Region",
        }
    }

    fn matches(self, snapshot: &ScopeMeasurementSnapshot) -> bool {
        match self {
            Self::All => true,
            Self::Cursor => snapshot.source.starts_with("cursor "),
            Self::Trigger => snapshot.source == "trigger",
            Self::Region => is_region_snapshot(snapshot),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::gui) enum ScopeSnapshotSortKey {
    #[default]
    Captured,
    Newest,
    Time,
    Source,
    Trace,
    Label,
}

impl ScopeSnapshotSortKey {
    const ALL: [Self; 6] = [
        Self::Captured,
        Self::Newest,
        Self::Time,
        Self::Source,
        Self::Trace,
        Self::Label,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Captured => "Captured",
            Self::Newest => "Newest",
            Self::Time => "Time",
            Self::Source => "Source",
            Self::Trace => "Trace",
            Self::Label => "Label",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::gui) enum ScopeSnapshotGroupMode {
    #[default]
    None,
    Source,
    Trace,
    Unit,
}

impl ScopeSnapshotGroupMode {
    const ALL: [Self; 4] = [Self::None, Self::Source, Self::Trace, Self::Unit];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Source => "Source",
            Self::Trace => "Trace",
            Self::Unit => "Unit",
        }
    }
}

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
            note: String::new(),
            source: "trigger".to_string(),
            trace: Some(self.selected_scope_trace()),
            trace_label,
            time_a_us: Some(event.time_us),
            time_b_us: None,
            value_a: Some(event.value),
            value_b: None,
            delta_value: None,
            rms_value: None,
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

    pub(in crate::gui) fn capture_scope_activity_sample_snapshot(
        &mut self,
        target: ScopeProbeTarget,
    ) -> bool {
        let Some((waveform_index, probe_index)) =
            find_scope_probe(&self.waveforms, &target.scenario_name, &target.probe_name)
        else {
            self.status = format!(
                "Scope Activity trace {} from scenario {} is not loaded yet.",
                target.probe_name, target.scenario_name
            );
            return false;
        };
        let trace = WaveformTraceRef {
            waveform_index,
            probe_index,
        };
        let Some(waveform) = self.waveforms.get(waveform_index) else {
            self.status = "Scope Activity waveform is no longer loaded.".to_string();
            return false;
        };
        let Some(probe) = waveform.probes.get(probe_index) else {
            self.status = "Scope Activity trace is no longer loaded.".to_string();
            return false;
        };
        let cursor_s = self.waveform_cursor_a_us / 1e6;
        let Some(value) = interpolated_value(&waveform.time_s, &probe.values, cursor_s) else {
            self.status = format!(
                "No Scope Activity sample is available for {} at {}.",
                target.probe_name,
                format_time_s(cursor_s)
            );
            return false;
        };
        let trace_label = trace_label(&self.waveforms, trace).unwrap_or_else(|| {
            probe
                .expression
                .as_deref()
                .unwrap_or(&probe.label)
                .to_string()
        });
        let snapshot = ScopeMeasurementSnapshot {
            label: format!(
                "Scope Activity {}",
                self.waveform_measurement_snapshots.len() + 1
            ),
            note: String::new(),
            source: "scope activity".to_string(),
            trace: Some(trace),
            trace_label: trace_label.clone(),
            time_a_us: Some(self.waveform_cursor_a_us),
            time_b_us: None,
            value_a: Some(value),
            value_b: None,
            delta_value: None,
            rms_value: None,
            event_edge: None,
            unit: probe_unit(&probe.label).to_string(),
        };
        self.push_scope_measurement_snapshot(snapshot);
        self.status = format!(
            "Captured Scope Activity sample for {trace_label} at {}.",
            format_time_s(cursor_s)
        );
        true
    }

    pub(super) fn capture_scope_region_stat_snapshots(
        &mut self,
        rows: &[ScopeRegionStatsRow],
        start_us: f64,
        end_us: f64,
    ) {
        if rows.is_empty() {
            self.status = "No scope region statistics are available to snapshot.".to_string();
            return;
        }
        let count = rows.len();
        let label = format!("Region {}", self.waveform_measurement_snapshots.len() + 1);
        for row in rows {
            self.push_scope_measurement_snapshot(region_snapshot(&label, start_us, end_us, row));
        }
        self.status = format!("Captured {count} scope region statistic snapshot(s).");
    }

    pub(super) fn waveform_measurement_snapshots_panel(&mut self, ui: &mut egui::Ui) {
        self.prune_scope_measurement_snapshots();
        if self.waveform_measurement_snapshots.is_empty()
            && self.waveform_recent_report_bundles.is_empty()
        {
            return;
        }
        let mut remove_index = None;
        let mut jump_index = None;
        let mut focus_index = None;
        let mut export_csv = false;
        let mut export_markdown = false;
        let mut export_bundle = false;
        let visible_indexes = self.visible_scope_measurement_snapshot_indexes();
        let visible_snapshots = self.filtered_scope_measurement_snapshots(&visible_indexes);
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Measurement Snapshots");
                ui.label("runtime only");
                if ui.button("Copy CSV").clicked() {
                    let csv = scope_snapshots_csv(&visible_snapshots);
                    ui.ctx().copy_text(csv);
                    self.status = format!(
                        "Copied {} scope measurement snapshot row(s) as CSV.",
                        visible_snapshots.len()
                    );
                }
                if ui.button("Copy Markdown").clicked() {
                    let markdown = scope_snapshots_markdown(&visible_snapshots);
                    ui.ctx().copy_text(markdown);
                    self.status = format!(
                        "Copied {} scope measurement snapshot row(s) as Markdown.",
                        visible_snapshots.len()
                    );
                }
                if ui.button("Export CSV").clicked() {
                    export_csv = true;
                }
                if ui.button("Export Markdown").clicked() {
                    export_markdown = true;
                }
                if ui.button("Export Bundle").clicked() {
                    export_bundle = true;
                }
                if ui.button("Clear").clicked() {
                    self.waveform_measurement_snapshots.clear();
                    self.status = "Cleared scope measurement snapshots.".to_string();
                }
            });
            self.scope_recent_report_bundles_ui(ui, &visible_snapshots);
            if self.waveform_measurement_snapshots.is_empty() {
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.label("Find");
                ui.add(
                    egui::TextEdit::singleline(&mut self.waveform_snapshot_filter)
                        .desired_width(180.0)
                        .hint_text("label, note, trace, unit, source"),
                );
                egui::ComboBox::from_label("Source")
                    .selected_text(self.waveform_snapshot_source_filter.label())
                    .show_ui(ui, |ui| {
                        for filter in ScopeSnapshotSourceFilter::ALL {
                            ui.selectable_value(
                                &mut self.waveform_snapshot_source_filter,
                                filter,
                                filter.label(),
                            );
                        }
                    });
                egui::ComboBox::from_label("Sort")
                    .selected_text(self.waveform_snapshot_sort_key.label())
                    .show_ui(ui, |ui| {
                        for sort_key in ScopeSnapshotSortKey::ALL {
                            ui.selectable_value(
                                &mut self.waveform_snapshot_sort_key,
                                sort_key,
                                sort_key.label(),
                            );
                        }
                    });
                egui::ComboBox::from_label("Group")
                    .selected_text(self.waveform_snapshot_group_mode.label())
                    .show_ui(ui, |ui| {
                        for group_mode in ScopeSnapshotGroupMode::ALL {
                            ui.selectable_value(
                                &mut self.waveform_snapshot_group_mode,
                                group_mode,
                                group_mode.label(),
                            );
                        }
                    });
                ui.label(format!(
                    "{} / {}",
                    visible_indexes.len(),
                    self.waveform_measurement_snapshots.len()
                ));
                if ui.small_button("Clear Filters").clicked() {
                    self.waveform_snapshot_filter.clear();
                    self.waveform_snapshot_source_filter = ScopeSnapshotSourceFilter::All;
                    self.waveform_snapshot_sort_key = ScopeSnapshotSortKey::Captured;
                    self.waveform_snapshot_group_mode = ScopeSnapshotGroupMode::None;
                }
            });
            if visible_indexes.is_empty() {
                ui.label("No measurement snapshots match the current filters.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(132.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        egui::Grid::new("scope_measurement_snapshots")
                            .num_columns(12)
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Label");
                                ui.label("Note");
                                ui.label("Source");
                                ui.label("Trace");
                                ui.label("A/Event/Min");
                                ui.label("B/Max");
                                ui.label("Delta/Mean");
                                ui.label("RMS");
                                ui.label("Unit");
                                ui.label("");
                                ui.label("");
                                ui.label("");
                                ui.end_row();

                                let mut previous_group = None;
                                for &index in &visible_indexes {
                                    let group = scope_snapshot_group_label(
                                        &self.waveform_measurement_snapshots[index],
                                        self.waveform_snapshot_group_mode,
                                    );
                                    if group != previous_group {
                                        if let Some(group) = &group {
                                            ui.strong(group);
                                            for _ in 1..12 {
                                                ui.label("");
                                            }
                                            ui.end_row();
                                        }
                                        previous_group = group;
                                    }
                                    let can_jump = self.waveform_measurement_snapshots[index]
                                        .trace
                                        .is_some_and(|trace| {
                                            valid_waveform_trace(&self.waveforms, trace)
                                        });
                                    let snapshot = &mut self.waveform_measurement_snapshots[index];
                                    ui.add(
                                        egui::TextEdit::singleline(&mut snapshot.label)
                                            .desired_width(88.0),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut snapshot.note)
                                            .desired_width(140.0)
                                            .hint_text("note"),
                                    );
                                    ui.monospace(snapshot_source(snapshot));
                                    ui.monospace(&snapshot.trace_label);
                                    ui.monospace(snapshot_value_a(snapshot));
                                    ui.monospace(snapshot_value_b(snapshot));
                                    ui.monospace(snapshot_delta(snapshot));
                                    ui.monospace(snapshot_rms(snapshot));
                                    ui.monospace(&snapshot.unit);
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
            }
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
        if export_csv {
            self.export_scope_measurement_snapshots_csv(&visible_snapshots);
        }
        if export_markdown {
            self.export_scope_measurement_snapshots_markdown(&visible_snapshots);
        }
        if export_bundle {
            self.export_scope_report_bundle(&visible_snapshots);
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

    pub(super) fn scope_snapshot_markers(
        &self,
        visible_traces: &[WaveformTraceRef],
    ) -> Vec<WaveformSnapshotMarker> {
        self.waveform_measurement_snapshots
            .iter()
            .enumerate()
            .filter_map(|(snapshot_index, snapshot)| {
                let trace = snapshot.trace?;
                (visible_traces.contains(&trace) && valid_waveform_trace(&self.waveforms, trace))
                    .then(|| WaveformSnapshotMarker {
                        snapshot_index,
                        trace,
                        label: snapshot.label.clone(),
                        note: snapshot.note.clone(),
                        source: snapshot.source.clone(),
                        trace_label: snapshot.trace_label.clone(),
                        time_a_us: snapshot.time_a_us,
                        time_b_us: snapshot.time_b_us,
                        value_a: if is_region_snapshot(snapshot) {
                            None
                        } else {
                            snapshot.value_a
                        },
                        value_b: if is_region_snapshot(snapshot) {
                            None
                        } else {
                            snapshot.value_b
                        },
                        event_edge: snapshot.event_edge.clone(),
                    })
            })
            .collect()
    }

    pub(super) fn selected_scope_trace_label(&self) -> Option<String> {
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
        self.apply_waveform_view_change(|app| {
            app.restore_scope_snapshot_time_window(&snapshot);
        });
        if let Some(time_us) = snapshot.time_a_us {
            self.set_waveform_cursor_a(time_us);
        }
        if let Some(time_us) = snapshot.time_b_us {
            self.set_waveform_cursor_b(time_us);
        }
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

    pub(super) fn shift_scope_measurement_snapshots_after_waveform_removal(
        &mut self,
        removed_waveform_index: usize,
    ) {
        self.waveform_measurement_snapshots.retain_mut(|snapshot| {
            let Some(trace) = &mut snapshot.trace else {
                return true;
            };
            shift_trace_after_waveform_removal(trace, removed_waveform_index)
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

    pub(super) fn visible_scope_measurement_snapshot_indexes(&self) -> Vec<usize> {
        scope_snapshot_visible_indexes_sorted(
            &self.waveform_measurement_snapshots,
            &self.waveform_snapshot_filter,
            self.waveform_snapshot_source_filter,
            self.waveform_snapshot_sort_key,
            self.waveform_snapshot_group_mode,
        )
    }

    fn filtered_scope_measurement_snapshots(
        &self,
        indexes: &[usize],
    ) -> Vec<ScopeMeasurementSnapshot> {
        indexes
            .iter()
            .filter_map(|&index| self.waveform_measurement_snapshots.get(index).cloned())
            .collect()
    }

    fn export_scope_measurement_snapshots_csv(&mut self, snapshots: &[ScopeMeasurementSnapshot]) {
        if snapshots.is_empty() {
            self.status =
                "No scope measurement snapshots match the current export filters.".to_string();
            return;
        }
        let Some(path) = self.pick_scope_snapshot_export_path() else {
            return;
        };
        let csv = scope_snapshots_csv(snapshots);
        match fs::write(&path, csv) {
            Ok(()) => {
                self.status = format!(
                    "Exported {} scope measurement snapshot row(s) to {}.",
                    snapshots.len(),
                    path.display()
                );
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to export scope measurement snapshots to {}: {error}",
                    path.display()
                ));
            }
        }
    }

    fn export_scope_measurement_snapshots_markdown(
        &mut self,
        snapshots: &[ScopeMeasurementSnapshot],
    ) {
        if snapshots.is_empty() {
            self.status =
                "No scope measurement snapshots match the current export filters.".to_string();
            return;
        }
        let Some(path) = self.pick_scope_snapshot_markdown_export_path() else {
            return;
        };
        let markdown = scope_snapshots_markdown(snapshots);
        match fs::write(&path, markdown) {
            Ok(()) => {
                self.status = format!(
                    "Exported {} scope measurement snapshot row(s) as Markdown to {}.",
                    snapshots.len(),
                    path.display()
                );
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to export scope measurement snapshots as Markdown to {}: {error}",
                    path.display()
                ));
            }
        }
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
        note: String::new(),
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
        rms_value: None,
        event_edge: None,
        unit: row.unit.to_string(),
    }
}

fn region_snapshot(
    label: &str,
    start_us: f64,
    end_us: f64,
    row: &ScopeRegionStatsRow,
) -> ScopeMeasurementSnapshot {
    ScopeMeasurementSnapshot {
        label: label.to_string(),
        note: String::new(),
        source: if row.selected {
            "region selected".to_string()
        } else {
            "region pinned".to_string()
        },
        trace: Some(row.trace),
        trace_label: row.label.clone(),
        time_a_us: Some(start_us),
        time_b_us: Some(end_us),
        value_a: Some(row.min),
        value_b: Some(row.max),
        delta_value: Some(row.mean),
        rms_value: Some(row.rms),
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
    if is_region_snapshot(snapshot) {
        return snapshot
            .value_a
            .map(format_value)
            .unwrap_or_else(|| "-".to_string());
    }
    match (snapshot.time_a_us, snapshot.value_a) {
        (Some(time_us), Some(value)) => {
            format!("{} @ {}", format_value(value), format_time_s(time_us / 1e6))
        }
        _ => "-".to_string(),
    }
}

fn snapshot_value_b(snapshot: &ScopeMeasurementSnapshot) -> String {
    if is_region_snapshot(snapshot) {
        return snapshot
            .value_b
            .map(format_value)
            .unwrap_or_else(|| "-".to_string());
    }
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

fn snapshot_rms(snapshot: &ScopeMeasurementSnapshot) -> String {
    snapshot
        .rms_value
        .map(format_value)
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn scope_snapshots_csv(snapshots: &[ScopeMeasurementSnapshot]) -> String {
    let mut csv = String::from(
        "label,note,source,trace,time_a_s,time_b_s,value_a_or_min,value_b_or_max,delta_or_mean,rms,event_edge,unit\n",
    );
    for snapshot in snapshots {
        let fields = [
            snapshot.label.clone(),
            snapshot.note.clone(),
            snapshot_source(snapshot),
            snapshot.trace_label.clone(),
            snapshot
                .time_a_us
                .map(|value| format_value(value / 1e6))
                .unwrap_or_default(),
            snapshot
                .time_b_us
                .map(|value| format_value(value / 1e6))
                .unwrap_or_default(),
            snapshot.value_a.map(format_value).unwrap_or_default(),
            snapshot.value_b.map(format_value).unwrap_or_default(),
            snapshot.delta_value.map(format_value).unwrap_or_default(),
            snapshot.rms_value.map(format_value).unwrap_or_default(),
            snapshot.event_edge.clone().unwrap_or_default(),
            snapshot.unit.clone(),
        ];
        csv.push_str(
            &fields
                .into_iter()
                .map(csv_escape)
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    csv
}

pub(super) fn scope_snapshots_markdown(snapshots: &[ScopeMeasurementSnapshot]) -> String {
    let mut markdown = String::from("## Scope Measurement Snapshots\n\n");
    if snapshots.is_empty() {
        markdown.push_str("_No measurement snapshots matched the current filters._\n");
        return markdown;
    }
    markdown.push_str(
        "| Label | Note | Source | Trace | A/Event/Min | B/Max | Delta/Mean | RMS | Unit |\n",
    );
    markdown.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for snapshot in snapshots {
        let fields = [
            snapshot.label.clone(),
            snapshot.note.clone(),
            snapshot_source(snapshot),
            snapshot.trace_label.clone(),
            snapshot_value_a(snapshot),
            snapshot_value_b(snapshot),
            snapshot_delta(snapshot),
            snapshot_rms(snapshot),
            snapshot.unit.clone(),
        ];
        markdown.push('|');
        for field in fields {
            markdown.push(' ');
            markdown.push_str(&markdown_escape(&field));
            markdown.push_str(" |");
        }
        markdown.push('\n');
    }
    markdown
}

pub(super) fn scope_snapshot_visible_indexes(
    snapshots: &[ScopeMeasurementSnapshot],
    query: &str,
    source_filter: ScopeSnapshotSourceFilter,
) -> Vec<usize> {
    let query = query.trim().to_ascii_lowercase();
    snapshots
        .iter()
        .enumerate()
        .filter_map(|(index, snapshot)| {
            (source_filter.matches(snapshot)
                && (query.is_empty() || snapshot_search_text(snapshot).contains(&query)))
            .then_some(index)
        })
        .collect()
}

pub(super) fn scope_snapshot_visible_indexes_sorted(
    snapshots: &[ScopeMeasurementSnapshot],
    query: &str,
    source_filter: ScopeSnapshotSourceFilter,
    sort_key: ScopeSnapshotSortKey,
    group_mode: ScopeSnapshotGroupMode,
) -> Vec<usize> {
    let mut indexes = scope_snapshot_visible_indexes(snapshots, query, source_filter);
    indexes.sort_by(|left, right| {
        let left_snapshot = &snapshots[*left];
        let right_snapshot = &snapshots[*right];
        let group_order = scope_snapshot_group_label(left_snapshot, group_mode)
            .cmp(&scope_snapshot_group_label(right_snapshot, group_mode));
        if group_order != std::cmp::Ordering::Equal {
            return group_order;
        }
        scope_snapshot_sort_order(left_snapshot, right_snapshot, *left, *right, sort_key)
    });
    indexes
}

fn scope_snapshot_sort_order(
    left: &ScopeMeasurementSnapshot,
    right: &ScopeMeasurementSnapshot,
    left_index: usize,
    right_index: usize,
    sort_key: ScopeSnapshotSortKey,
) -> std::cmp::Ordering {
    match sort_key {
        ScopeSnapshotSortKey::Captured => left_index.cmp(&right_index),
        ScopeSnapshotSortKey::Newest => right_index.cmp(&left_index),
        ScopeSnapshotSortKey::Time => snapshot_sort_time(left)
            .total_cmp(&snapshot_sort_time(right))
            .then_with(|| left_index.cmp(&right_index)),
        ScopeSnapshotSortKey::Source => snapshot_source(left)
            .cmp(&snapshot_source(right))
            .then_with(|| left_index.cmp(&right_index)),
        ScopeSnapshotSortKey::Trace => left
            .trace_label
            .cmp(&right.trace_label)
            .then_with(|| left_index.cmp(&right_index)),
        ScopeSnapshotSortKey::Label => left
            .label
            .cmp(&right.label)
            .then_with(|| left_index.cmp(&right_index)),
    }
}

fn snapshot_sort_time(snapshot: &ScopeMeasurementSnapshot) -> f64 {
    snapshot_times(snapshot)
        .into_iter()
        .fold(f64::INFINITY, f64::min)
}

fn scope_snapshot_group_label(
    snapshot: &ScopeMeasurementSnapshot,
    group_mode: ScopeSnapshotGroupMode,
) -> Option<String> {
    match group_mode {
        ScopeSnapshotGroupMode::None => None,
        ScopeSnapshotGroupMode::Source => Some(snapshot_source(snapshot)),
        ScopeSnapshotGroupMode::Trace => Some(blank_fallback(&snapshot.trace_label, "untraced")),
        ScopeSnapshotGroupMode::Unit => Some(blank_fallback(&snapshot.unit, "unitless")),
    }
}

fn blank_fallback(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn snapshot_search_text(snapshot: &ScopeMeasurementSnapshot) -> String {
    [
        snapshot.label.as_str(),
        snapshot.note.as_str(),
        snapshot.source.as_str(),
        snapshot.event_edge.as_deref().unwrap_or(""),
        snapshot.trace_label.as_str(),
        snapshot.unit.as_str(),
    ]
    .join(" ")
    .to_ascii_lowercase()
}

fn csv_escape(value: String) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

pub(super) fn markdown_escape(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return "-".to_string();
    }
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "<br>")
        .replace('\r', "")
}

fn is_region_snapshot(snapshot: &ScopeMeasurementSnapshot) -> bool {
    snapshot.source.starts_with("region ")
}

fn snapshot_times(snapshot: &ScopeMeasurementSnapshot) -> Vec<f64> {
    [snapshot.time_a_us, snapshot.time_b_us]
        .into_iter()
        .flatten()
        .filter(|time_us| time_us.is_finite())
        .collect()
}
