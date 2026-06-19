use super::waveform_load::format_waveform_load_bytes;
use super::{CircuitCiApp, WaveformView};
use eframe::egui;
use std::fs;

const WAVEFORM_FOOTPRINT_WARNING_BYTES: usize = 256 * 1024 * 1024;

impl CircuitCiApp {
    pub(super) fn loaded_waveform_footprint_ui(&mut self, ui: &mut egui::Ui) {
        let rows = waveform_footprint_rows(
            &self.waveforms,
            &self.waveform_footprint_filter,
            self.waveform_footprint_sort_key,
            self.waveform_footprint_descending,
        );
        let total_bytes = waveform_footprint_total_bytes(&self.waveforms);
        let all_rows = waveform_footprint_rows(
            &self.waveforms,
            "",
            WaveformFootprintSortKey::EstimatedBytes,
            true,
        );
        let largest_unload_targets = waveform_footprint_largest_unload_targets(
            &all_rows,
            WAVEFORM_FOOTPRINT_WARNING_BYTES,
            total_bytes,
        );
        let mut next_waveform = None;
        let mut unload_waveform = None;
        let mut export_csv = false;
        ui.collapsing(
            format!(
                "Loaded Footprint ({}; {})",
                self.waveforms.len(),
                format_waveform_load_bytes(Some(total_bytes as u64))
            ),
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Find");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.waveform_footprint_filter)
                            .desired_width(180.0)
                            .hint_text("file, path, probe"),
                    );
                    if response.changed() {
                        self.waveform_footprint_filter =
                            self.waveform_footprint_filter.trim_start().to_string();
                    }
                    egui::ComboBox::from_label("Sort")
                        .selected_text(self.waveform_footprint_sort_key.label())
                        .show_ui(ui, |ui| {
                            for sort_key in WaveformFootprintSortKey::ALL {
                                ui.selectable_value(
                                    &mut self.waveform_footprint_sort_key,
                                    sort_key,
                                    sort_key.label(),
                                );
                            }
                        });
                    ui.checkbox(&mut self.waveform_footprint_descending, "Descending");
                    ui.label(format!("{} / {} visible", rows.len(), self.waveforms.len()));
                    if ui
                        .add_enabled(!rows.is_empty(), egui::Button::new("Copy CSV"))
                        .clicked()
                    {
                        ui.ctx().copy_text(waveform_footprint_csv(&rows));
                        self.status = format!(
                            "Copied {} loaded waveform footprint row(s) as CSV.",
                            rows.len()
                        );
                    }
                    if ui
                        .add_enabled(!rows.is_empty(), egui::Button::new("Export CSV"))
                        .clicked()
                    {
                        export_csv = true;
                    }
                    if ui
                        .add_enabled(
                            !self.waveform_footprint_filter.trim().is_empty(),
                            egui::Button::new("Clear"),
                        )
                        .clicked()
                    {
                        self.waveform_footprint_filter.clear();
                        self.waveform_footprint_unload_preview.clear();
                    }
                    if ui
                        .add_enabled(
                            !rows.is_empty(),
                            egui::Button::new(format!("Preview Unload Visible ({})", rows.len())),
                        )
                        .on_hover_text(
                            "Preview a bulk unload for the currently visible footprint rows.",
                        )
                        .clicked()
                    {
                        self.waveform_footprint_unload_preview =
                            waveform_footprint_unload_targets(&rows);
                    }
                });
                if total_bytes > WAVEFORM_FOOTPRINT_WARNING_BYTES {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "Loaded waveform estimate exceeds the {} budget by {}.",
                            format_waveform_load_bytes(Some(
                                WAVEFORM_FOOTPRINT_WARNING_BYTES as u64,
                            )),
                            format_waveform_load_bytes(Some(
                                (total_bytes - WAVEFORM_FOOTPRINT_WARNING_BYTES) as u64,
                            )),
                        ));
                        if ui
                            .add_enabled(
                                !largest_unload_targets.is_empty(),
                                egui::Button::new(format!(
                                    "Preview Unload Largest ({})",
                                    largest_unload_targets.len()
                                )),
                            )
                            .on_hover_text(
                                "Preview unloading the largest loaded waveform views until the estimate is back under budget.",
                            )
                            .clicked()
                        {
                            self.waveform_footprint_unload_preview =
                                largest_unload_targets.clone();
                        }
                    });
                }
                if !self.waveform_footprint_unload_preview.is_empty() {
                    let preview_count = self.waveform_footprint_unload_preview.len();
                    let preview_bytes = self
                        .waveform_footprint_unload_preview
                        .iter()
                        .map(|target| target.estimated_bytes)
                        .sum::<usize>();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "Unload preview: {preview_count} waveform view(s), {}.",
                            format_waveform_load_bytes(Some(preview_bytes as u64))
                        ));
                        if ui.button("Confirm Unload").clicked() {
                            let targets = self.waveform_footprint_unload_preview.clone();
                            self.waveform_footprint_unload_preview.clear();
                            let removed = self.unload_waveform_footprint_targets(&targets);
                            self.status =
                                format!("Unloaded {removed} waveform artifact(s) from memory.");
                        }
                        if ui.button("Cancel").clicked() {
                            self.waveform_footprint_unload_preview.clear();
                            self.status = "Canceled loaded waveform unload preview.".to_string();
                        }
                    });
                }
                if rows.is_empty() {
                    ui.small("No loaded waveform matches the current footprint filter.");
                    return;
                }
                egui::Grid::new("loaded_waveform_footprint")
                    .num_columns(7)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Waveform");
                        ui.label("Rows");
                        ui.label("Traces");
                        ui.label("Values");
                        ui.label("Estimate");
                        ui.label("Path");
                        ui.label("Action");
                        ui.end_row();

                        for row in &rows {
                            ui.monospace(&row.label);
                            ui.monospace(row.samples.to_string());
                            ui.monospace(row.probes.to_string());
                            ui.monospace(row.values.to_string());
                            ui.monospace(format_waveform_load_bytes(Some(
                                row.estimated_bytes as u64,
                            )));
                            ui.small(&row.path);
                            ui.horizontal(|ui| {
                                if ui.small_button("Select").clicked() {
                                    next_waveform = Some(row.waveform_index);
                                }
                                if ui
                                    .small_button("Unload")
                                    .on_hover_text(
                                        "Unload this parsed waveform artifact from memory.",
                                    )
                                    .clicked()
                                {
                                    unload_waveform = Some(row.waveform_index);
                                }
                            });
                            ui.end_row();
                        }
                    });
            },
        );
        if export_csv {
            self.export_loaded_waveform_footprint_csv(&rows);
        }
        if let Some(index) = unload_waveform {
            self.waveform_footprint_unload_preview.clear();
            self.unload_waveform_view(index);
        } else if let Some(index) = next_waveform {
            self.selected_waveform = index.min(self.waveforms.len().saturating_sub(1));
            self.selected_probe = 0;
            self.waveform_cursor_a_us = 0.0;
            self.waveform_cursor_b_us = 0.0;
            self.waveform_window_start_us = None;
            self.waveform_window_end_us = None;
            self.waveform_value_min = None;
            self.waveform_value_max = None;
            self.clear_waveform_view_history();
            self.waveform_trigger_threshold = 0.0;
            self.waveform_playing = false;
        }
    }

    fn export_loaded_waveform_footprint_csv(&mut self, rows: &[WaveformFootprintRow]) {
        if rows.is_empty() {
            self.status =
                "No loaded waveform footprint rows match the current export filters.".to_string();
            return;
        }
        let Some(path) = self.pick_scope_footprint_export_path() else {
            return;
        };
        match fs::write(&path, waveform_footprint_csv(rows)) {
            Ok(()) => {
                self.status = format!(
                    "Exported {} loaded waveform footprint row(s) to {}.",
                    rows.len(),
                    path.display()
                );
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to export loaded waveform footprint to {}: {error}",
                    path.display()
                ));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::gui) enum WaveformFootprintSortKey {
    #[default]
    EstimatedBytes,
    Samples,
    Probes,
    Label,
}

impl WaveformFootprintSortKey {
    pub(super) const ALL: [Self; 4] = [
        Self::EstimatedBytes,
        Self::Samples,
        Self::Probes,
        Self::Label,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::EstimatedBytes => "Memory",
            Self::Samples => "Rows",
            Self::Probes => "Traces",
            Self::Label => "Label",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WaveformFootprintRow {
    pub(super) waveform_index: usize,
    pub(super) label: String,
    pub(super) path: String,
    pub(super) samples: usize,
    pub(super) probes: usize,
    pub(super) values: usize,
    pub(super) estimated_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::gui) struct WaveformFootprintUnloadTarget {
    label: String,
    path: String,
    samples: usize,
    probes: usize,
    values: usize,
    estimated_bytes: usize,
}

impl WaveformFootprintUnloadTarget {
    fn from_row(row: &WaveformFootprintRow) -> Self {
        Self {
            label: row.label.clone(),
            path: row.path.clone(),
            samples: row.samples,
            probes: row.probes,
            values: row.values,
            estimated_bytes: row.estimated_bytes,
        }
    }
}

pub(super) fn waveform_footprint_rows(
    waveforms: &[WaveformView],
    query: &str,
    sort_key: WaveformFootprintSortKey,
    descending: bool,
) -> Vec<WaveformFootprintRow> {
    let query = query.trim().to_ascii_lowercase();
    let mut rows = waveforms
        .iter()
        .enumerate()
        .filter_map(|(index, waveform)| {
            if !query.is_empty() && !waveform_footprint_search_text(waveform).contains(&query) {
                return None;
            }
            let values = waveform_footprint_value_count(waveform);
            Some(WaveformFootprintRow {
                waveform_index: index,
                label: waveform.label.clone(),
                path: waveform.path.clone(),
                samples: waveform.time_s.len(),
                probes: waveform.probes.len(),
                values,
                estimated_bytes: values * std::mem::size_of::<f64>(),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let order = match sort_key {
            WaveformFootprintSortKey::EstimatedBytes => left
                .estimated_bytes
                .cmp(&right.estimated_bytes)
                .then_with(|| left.label.cmp(&right.label)),
            WaveformFootprintSortKey::Samples => left
                .samples
                .cmp(&right.samples)
                .then_with(|| left.label.cmp(&right.label)),
            WaveformFootprintSortKey::Probes => left
                .probes
                .cmp(&right.probes)
                .then_with(|| left.label.cmp(&right.label)),
            WaveformFootprintSortKey::Label => left.label.cmp(&right.label),
        };
        if descending { order.reverse() } else { order }
    });
    rows
}

pub(super) fn waveform_footprint_unload_targets(
    rows: &[WaveformFootprintRow],
) -> Vec<WaveformFootprintUnloadTarget> {
    rows.iter()
        .map(WaveformFootprintUnloadTarget::from_row)
        .collect()
}

pub(super) fn waveform_footprint_csv(rows: &[WaveformFootprintRow]) -> String {
    let mut csv =
        String::from("waveform,path,samples,probes,values,estimated_bytes,estimated_size\n");
    for row in rows {
        let fields = [
            row.label.clone(),
            row.path.clone(),
            row.samples.to_string(),
            row.probes.to_string(),
            row.values.to_string(),
            row.estimated_bytes.to_string(),
            format_waveform_load_bytes(Some(row.estimated_bytes as u64)),
        ];
        csv.push_str(
            &fields
                .into_iter()
                .map(waveform_footprint_csv_escape)
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    csv
}

pub(super) fn waveform_footprint_largest_unload_targets(
    rows: &[WaveformFootprintRow],
    budget_bytes: usize,
    total_bytes: usize,
) -> Vec<WaveformFootprintUnloadTarget> {
    if total_bytes <= budget_bytes {
        return Vec::new();
    }
    let mut rows = rows.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .estimated_bytes
            .cmp(&left.estimated_bytes)
            .then_with(|| left.label.cmp(&right.label))
    });
    let mut remaining_bytes = total_bytes;
    let mut targets = Vec::new();
    for row in rows {
        targets.push(WaveformFootprintUnloadTarget::from_row(row));
        remaining_bytes = remaining_bytes.saturating_sub(row.estimated_bytes);
        if remaining_bytes <= budget_bytes {
            break;
        }
    }
    targets
}

pub(super) fn waveform_footprint_target_index(
    waveforms: &[WaveformView],
    target: &WaveformFootprintUnloadTarget,
    skipped_indexes: &[usize],
) -> Option<usize> {
    waveforms.iter().enumerate().find_map(|(index, waveform)| {
        (!skipped_indexes.contains(&index)
            && waveform.label == target.label
            && waveform.path == target.path
            && waveform.time_s.len() == target.samples
            && waveform.probes.len() == target.probes
            && waveform_footprint_value_count(waveform) == target.values
            && waveform_footprint_estimated_bytes(waveform) == target.estimated_bytes)
            .then_some(index)
    })
}

fn waveform_footprint_search_text(waveform: &WaveformView) -> String {
    let probes = waveform
        .probes
        .iter()
        .map(|probe| probe.label.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    format!("{} {} {}", waveform.label, waveform.path, probes).to_ascii_lowercase()
}

fn waveform_footprint_estimated_bytes(waveform: &WaveformView) -> usize {
    waveform_footprint_value_count(waveform) * std::mem::size_of::<f64>()
}

fn waveform_footprint_total_bytes(waveforms: &[WaveformView]) -> usize {
    waveforms
        .iter()
        .map(waveform_footprint_estimated_bytes)
        .sum::<usize>()
}

fn waveform_footprint_csv_escape(value: String) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

fn waveform_footprint_value_count(waveform: &WaveformView) -> usize {
    waveform.time_s.len()
        + waveform
            .probes
            .iter()
            .map(|probe| probe.values.len())
            .sum::<usize>()
}
