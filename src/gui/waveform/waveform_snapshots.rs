use super::waveform_plot::{WaveformSnapshotMarker, valid_waveform_trace};
use super::waveform_trace_selector::shift_trace_after_waveform_removal;
use super::waveform_trigger::ScopeTriggerEvent;
use super::{
    ScopeCursorLegendRow, ScopeRegionStatsRow, WaveformTraceRef, format_time_s, format_value,
    scope_cursor_legend_rows, waveform_time_range_for_view,
};
use crate::gui::{CircuitCiApp, ScopeMeasurementSnapshot};
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_SCOPE_SNAPSHOTS: usize = 64;
const MAX_RECENT_SCOPE_BUNDLES: usize = 5;

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

    fn label(self) -> &'static str {
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

    fn label(self) -> &'static str {
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

    fn label(self) -> &'static str {
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
            self.scope_recent_report_bundles_ui(ui);
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

    pub(super) fn export_scope_report_bundle(&mut self, snapshots: &[ScopeMeasurementSnapshot]) {
        if snapshots.is_empty() {
            self.status =
                "No scope measurement snapshots match the current bundle filters.".to_string();
            return;
        }
        let Some(svg) = self.current_scope_plot_svg() else {
            self.status = "No scope plot is available to include in the report bundle.".to_string();
            return;
        };
        let base_dir = output_bundle_base_dir(&self.output_dir);
        let bundle_dir = unique_scope_report_bundle_dir(&base_dir, current_unix_millis());
        match fs::create_dir_all(&bundle_dir)
            .and_then(|()| fs::write(bundle_dir.join("scope_plot.svg"), svg))
            .and_then(|()| {
                fs::write(
                    bundle_dir.join("measurement_snapshots.csv"),
                    scope_snapshots_csv(snapshots),
                )
            })
            .and_then(|()| {
                fs::write(
                    bundle_dir.join("measurement_snapshots.md"),
                    scope_snapshots_markdown(snapshots),
                )
            })
            .and_then(|()| {
                fs::write(
                    bundle_dir.join("README.md"),
                    self.scope_report_bundle_readme(snapshots),
                )
            }) {
            Ok(()) => {
                self.push_recent_scope_report_bundle(bundle_dir.to_string_lossy().into_owned());
                self.status = format!(
                    "Exported scope report bundle with {} snapshot row(s) to {}.",
                    snapshots.len(),
                    bundle_dir.display()
                );
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to export scope report bundle to {}: {error}",
                    bundle_dir.display()
                ));
            }
        }
    }

    fn scope_report_bundle_readme(&self, snapshots: &[ScopeMeasurementSnapshot]) -> String {
        let selected_context = self
            .selected_scope_trace_label()
            .unwrap_or_else(|| "unavailable".to_string());
        let query = self.waveform_snapshot_filter.trim();
        let query = if query.is_empty() { "(empty)" } else { query };
        format!(
            "\
# CircuitCI Scope Report Bundle

This folder is a runtime export from the Scopes workspace. It is derived from loaded waveform artifacts and transient GUI state; it is not persisted Board IR project truth.

## Files

- `scope_plot.svg` - configured Scopes plot SVG.
- `measurement_snapshots.csv` - filtered measurement snapshot rows.
- `measurement_snapshots.md` - filtered measurement snapshot rows as Markdown.
- `README.md` - this manifest.

## Snapshot Projection

- Rows: {}
- Search: {}
- Source: {}
- Sort: {}
- Group: {}

## Plot Export Options

- Size: {}
- Include cursors: {}
- Include trigger markers: {}
- Include snapshot annotations: {}
- Split units: {}

## Selected Trace Context

- Selected waveform index: {}
- Selected probe index: {}
- Selected trace: {}
",
            snapshots.len(),
            markdown_escape(query),
            self.waveform_snapshot_source_filter.label(),
            self.waveform_snapshot_sort_key.label(),
            self.waveform_snapshot_group_mode.label(),
            self.waveform_plot_export_size.label(),
            yes_no(self.waveform_plot_export_cursors),
            yes_no(self.waveform_plot_export_trigger),
            yes_no(self.waveform_plot_export_snapshots),
            yes_no(self.waveform_split_trace_units),
            self.selected_waveform,
            self.selected_probe,
            markdown_escape(&selected_context),
        )
    }

    fn scope_recent_report_bundles_ui(&mut self, ui: &mut egui::Ui) {
        let Some(latest) = self.waveform_recent_report_bundles.first().cloned() else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.label("Recent bundle");
            ui.monospace(display_path_tail(&latest));
            if ui.button("Open Bundle Folder").clicked() {
                self.open_scope_report_bundle(&latest);
            }
            if ui.button("Clean Old Bundles").clicked() {
                self.preview_old_scope_report_bundles();
            }
            if self.waveform_recent_report_bundles.len() > 1 {
                let older_bundles = self
                    .waveform_recent_report_bundles
                    .iter()
                    .skip(1)
                    .cloned()
                    .collect::<Vec<_>>();
                egui::ComboBox::from_id_salt("scope_recent_report_bundles")
                    .selected_text("Older")
                    .show_ui(ui, |ui| {
                        for bundle in older_bundles {
                            if ui.button(display_path_tail(&bundle)).clicked() {
                                self.open_scope_report_bundle(&bundle);
                                ui.close();
                            }
                        }
                    });
            }
        });
        if !self.waveform_bundle_cleanup_preview.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "Cleanup preview: remove {} old bundle folder(s).",
                    self.waveform_bundle_cleanup_preview.len()
                ));
                if ui.button("Confirm Cleanup").clicked() {
                    self.confirm_scope_report_bundle_cleanup();
                }
                if ui.button("Cancel").clicked() {
                    self.waveform_bundle_cleanup_preview.clear();
                    self.status = "Canceled scope report bundle cleanup.".to_string();
                }
            });
        }
    }

    fn push_recent_scope_report_bundle(&mut self, bundle: String) {
        self.waveform_recent_report_bundles
            .retain(|existing| existing != &bundle);
        self.waveform_recent_report_bundles.insert(0, bundle);
        self.waveform_recent_report_bundles
            .truncate(MAX_RECENT_SCOPE_BUNDLES);
    }

    fn open_scope_report_bundle(&mut self, bundle: &str) {
        let path = Path::new(bundle);
        if !path.exists() {
            self.status = format!("Scope report bundle no longer exists: {}.", path.display());
            return;
        }
        match open_path_in_file_manager(path) {
            Ok(()) => {
                self.status = format!("Opened scope report bundle folder {}.", path.display());
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to open scope report bundle folder {}: {error}",
                    path.display()
                ));
            }
        }
    }

    fn preview_old_scope_report_bundles(&mut self) {
        match old_scope_report_bundle_dirs(
            &output_bundle_base_dir(&self.output_dir),
            MAX_RECENT_SCOPE_BUNDLES,
        ) {
            Ok(paths) if paths.is_empty() => {
                self.waveform_bundle_cleanup_preview.clear();
                self.status = "No old scope report bundle folders to clean.".to_string();
            }
            Ok(paths) => {
                let count = paths.len();
                self.waveform_bundle_cleanup_preview = paths
                    .into_iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect();
                self.status =
                    format!("Previewing {count} old scope report bundle folder(s) for cleanup.");
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to preview old scope report bundle folders: {error}"
                ));
            }
        }
    }

    fn confirm_scope_report_bundle_cleanup(&mut self) {
        match remove_scope_report_bundle_dirs(&self.waveform_bundle_cleanup_preview) {
            Ok(removed) => {
                self.waveform_bundle_cleanup_preview.clear();
                self.waveform_recent_report_bundles
                    .retain(|bundle| Path::new(bundle).exists());
                self.status = format!("Removed {removed} old scope report bundle folder(s).");
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to clean old scope report bundle folders: {error}"
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

fn markdown_escape(value: &str) -> String {
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

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn display_path_tail(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string())
}

fn open_path_in_file_manager(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(path);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };
    command.spawn().map(|_| ())
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

pub(super) fn output_bundle_base_dir(output_dir: &str) -> PathBuf {
    let trimmed = output_dir.trim();
    if trimmed.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(trimmed)
    }
}

pub(super) fn unique_scope_report_bundle_dir(base_dir: &Path, unix_millis: u128) -> PathBuf {
    let stem = format!("scope_report_bundle_{unix_millis}");
    let first = base_dir.join(&stem);
    if !first.exists() {
        return first;
    }
    for suffix in 2..1000 {
        let candidate = base_dir.join(format!("{stem}_{suffix:02}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    base_dir.join(format!("{stem}_overflow"))
}

#[cfg(test)]
pub(super) fn cleanup_old_scope_report_bundle_dirs(
    base_dir: &Path,
    keep_count: usize,
) -> std::io::Result<usize> {
    let bundles = old_scope_report_bundle_dirs(base_dir, keep_count)?;
    let mut removed = 0usize;
    for bundle in bundles {
        fs::remove_dir_all(&bundle)?;
        removed += 1;
    }
    Ok(removed)
}

pub(super) fn old_scope_report_bundle_dirs(
    base_dir: &Path,
    keep_count: usize,
) -> std::io::Result<Vec<PathBuf>> {
    let mut bundles = scope_report_bundle_dirs(base_dir)?;
    bundles.sort_by(|left, right| {
        report_bundle_sort_key(right)
            .cmp(&report_bundle_sort_key(left))
            .then_with(|| right.cmp(left))
    });
    Ok(bundles.into_iter().skip(keep_count).collect())
}

fn remove_scope_report_bundle_dirs(bundles: &[String]) -> std::io::Result<usize> {
    let mut removed = 0usize;
    for bundle in bundles {
        let path = Path::new(bundle);
        if is_scope_report_bundle_dir(path) {
            fs::remove_dir_all(path)?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn scope_report_bundle_dirs(base_dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut bundles = Vec::new();
    if !base_dir.exists() {
        return Ok(bundles);
    }
    for entry in fs::read_dir(base_dir)? {
        let path = entry?.path();
        if is_scope_report_bundle_dir(&path) {
            bundles.push(path);
        }
    }
    Ok(bundles)
}

fn is_scope_report_bundle_dir(path: &Path) -> bool {
    path.is_dir()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("scope_report_bundle_"))
}

fn report_bundle_sort_key(path: &Path) -> u128 {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("scope_report_bundle_"))
        .map(|suffix| suffix.chars().take_while(|ch| ch.is_ascii_digit()))
        .and_then(|digits| digits.collect::<String>().parse::<u128>().ok())
        .unwrap_or_default()
}

fn current_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
