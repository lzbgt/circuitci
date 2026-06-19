use crate::gui::CircuitCiApp;
use eframe::egui;

use super::WaveformLoadRequest;
use super::waveform_load::{
    WaveformLoadDiagnostic, WaveformLoadStatusFilter, format_waveform_load_bytes,
    waveform_load_selected_probes_for_path,
};

impl CircuitCiApp {
    pub(super) fn waveform_load_diagnostics_panel(&mut self, ui: &mut egui::Ui) {
        if self.waveform_load_diagnostics.is_empty() {
            return;
        }
        let visible_indexes = waveform_load_diagnostic_visible_indexes(
            &self.waveform_load_diagnostics,
            &self.waveform_load_filter,
            self.waveform_load_status_filter,
            self.waveform_load_min_ms,
            self.waveform_load_slowest_first,
        );
        let loaded = self
            .waveform_load_diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.loaded)
            .count();
        let deferred = self
            .waveform_load_diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.deferred)
            .count();
        let skipped = self
            .waveform_load_diagnostics
            .len()
            .saturating_sub(loaded)
            .saturating_sub(deferred);

        let mut load_deferred_path = None;
        let mut load_deferred_request = None;
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Waveform Load Diagnostics");
                ui.label(format!(
                    "{} loaded, {} deferred, {} skipped",
                    loaded, deferred, skipped
                ));
                ui.checkbox(&mut self.waveform_defer_large_loads, "Defer large");
                if deferred > 0 && ui.button("Load Deferred").clicked() {
                    self.load_deferred_waveforms();
                }
                if ui.button("Copy CSV").clicked() {
                    ui.ctx().copy_text(waveform_load_diagnostics_csv(
                        &self.waveform_load_diagnostics,
                        &visible_indexes,
                    ));
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Find");
                ui.add(
                    egui::TextEdit::singleline(&mut self.waveform_load_filter)
                        .desired_width(180.0)
                        .hint_text("path or detail"),
                );
                egui::ComboBox::from_label("Status")
                    .selected_text(self.waveform_load_status_filter.label())
                    .show_ui(ui, |ui| {
                        for filter in WaveformLoadStatusFilter::ALL {
                            ui.selectable_value(
                                &mut self.waveform_load_status_filter,
                                filter,
                                filter.label(),
                            );
                        }
                    });
                ui.label("Min ms");
                ui.add(
                    egui::DragValue::new(&mut self.waveform_load_min_ms)
                        .range(0.0..=3_600_000.0)
                        .speed(10.0),
                );
                ui.checkbox(&mut self.waveform_load_slowest_first, "Slowest first");
                ui.label(format!(
                    "{} / {}",
                    visible_indexes.len(),
                    self.waveform_load_diagnostics.len()
                ));
                if ui.small_button("Clear Filters").clicked() {
                    self.waveform_load_filter.clear();
                    self.waveform_load_status_filter = WaveformLoadStatusFilter::All;
                    self.waveform_load_min_ms = 0.0;
                    self.waveform_load_slowest_first = false;
                }
            });
            if visible_indexes.is_empty() {
                ui.label("No waveform load diagnostics match the current filters.");
                return;
            }
            egui::Grid::new("waveform_load_diagnostics")
                .num_columns(9)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Status");
                    ui.label("File");
                    ui.label("Size");
                    ui.label("Rows");
                    ui.label("Probes");
                    ui.label("Preview");
                    ui.label("Load");
                    ui.label("Detail");
                    ui.label("Action");
                    ui.end_row();

                    for &index in &visible_indexes {
                        let diagnostic = &self.waveform_load_diagnostics[index];
                        ui.label(diagnostic.status_label());
                        ui.monospace(&diagnostic.path);
                        ui.monospace(format_waveform_load_bytes(diagnostic.bytes));
                        ui.monospace(diagnostic.samples.to_string());
                        ui.monospace(diagnostic.probes.to_string());
                        ui.vertical(|ui| {
                            ui.monospace(waveform_load_probe_summary_counts(
                                &self.waveform_load_diagnostics,
                                diagnostic,
                            ));
                            let preview = waveform_load_probe_summary_preview(
                                &self.waveform_load_diagnostics,
                                diagnostic,
                            );
                            if !preview.is_empty() {
                                ui.small(preview);
                            }
                        });
                        ui.monospace(format!("{} ms", diagnostic.elapsed_ms));
                        ui.label(&diagnostic.detail);
                        if diagnostic.deferred {
                            let unloaded_preview =
                                waveform_load_diagnostic_unloaded_preview_columns(
                                    &self.waveform_load_diagnostics,
                                    diagnostic,
                                );
                            ui.horizontal(|ui| {
                                if ui.small_button("Load").clicked() {
                                    load_deferred_path = Some(diagnostic.path.clone());
                                }
                                if ui
                                    .add_enabled(
                                        !unloaded_preview.is_empty(),
                                        egui::Button::new(format!(
                                            "Load Preview ({})",
                                            unloaded_preview.len()
                                        )),
                                    )
                                    .clicked()
                                {
                                    load_deferred_request =
                                        Some(WaveformLoadRequest::selected_columns(
                                            diagnostic.path.clone(),
                                            unloaded_preview,
                                        ));
                                }
                            });
                        } else {
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
        });
        if let Some(path) = load_deferred_path {
            self.load_deferred_waveform_path(path);
        }
        if let Some(request) = load_deferred_request {
            self.load_deferred_waveform_requests(vec![request]);
        }
    }
}

pub(super) fn waveform_load_diagnostic_visible_indexes(
    diagnostics: &[WaveformLoadDiagnostic],
    query: &str,
    status_filter: WaveformLoadStatusFilter,
    min_elapsed_ms: f64,
    slowest_first: bool,
) -> Vec<usize> {
    let query = query.trim().to_ascii_lowercase();
    let min_elapsed_ms = min_elapsed_ms.max(0.0).round() as u128;
    let mut indexes: Vec<_> = diagnostics
        .iter()
        .enumerate()
        .filter_map(|(index, diagnostic)| {
            if !matches!(
                (status_filter, diagnostic.loaded),
                (WaveformLoadStatusFilter::All, _)
                    | (WaveformLoadStatusFilter::Loaded, true)
                    | (WaveformLoadStatusFilter::Deferred, false)
                    | (WaveformLoadStatusFilter::Skipped, false)
            ) {
                return None;
            }
            if status_filter == WaveformLoadStatusFilter::Deferred && !diagnostic.deferred {
                return None;
            }
            if status_filter == WaveformLoadStatusFilter::Skipped && diagnostic.deferred {
                return None;
            }
            if diagnostic.elapsed_ms < min_elapsed_ms {
                return None;
            }
            if !query.is_empty()
                && !waveform_load_diagnostic_search_text(diagnostic).contains(&query)
            {
                return None;
            }
            Some(index)
        })
        .collect();
    if slowest_first {
        indexes.sort_by(|left, right| {
            diagnostics[*right]
                .elapsed_ms
                .cmp(&diagnostics[*left].elapsed_ms)
                .then_with(|| left.cmp(right))
        });
    }
    indexes
}

fn waveform_load_diagnostic_search_text(diagnostic: &WaveformLoadDiagnostic) -> String {
    format!(
        "{} {} {} {}",
        diagnostic.csv_status(),
        diagnostic.path,
        diagnostic.detail,
        diagnostic.probe_preview.join(" ")
    )
    .to_ascii_lowercase()
}

pub(super) fn waveform_load_diagnostics_csv(
    diagnostics: &[WaveformLoadDiagnostic],
    visible_indexes: &[usize],
) -> String {
    let mut csv = String::from(
        "status,path,size_bytes,samples,probes,elapsed_ms,preview_columns,loaded_preview_columns,unloaded_preview_columns,detail\n",
    );
    for diagnostic in visible_indexes
        .iter()
        .filter_map(|&index| diagnostics.get(index))
    {
        let loaded_preview = waveform_load_loaded_preview_for_diagnostic(diagnostics, diagnostic);
        let unloaded_preview =
            waveform_load_unloaded_preview_for_diagnostic(diagnostic, &loaded_preview);
        let fields = [
            diagnostic.csv_status().to_string(),
            diagnostic.path.clone(),
            diagnostic
                .bytes
                .map(|bytes| bytes.to_string())
                .unwrap_or_default(),
            diagnostic.samples.to_string(),
            diagnostic.probes.to_string(),
            diagnostic.elapsed_ms.to_string(),
            diagnostic.probe_preview.join("; "),
            loaded_preview.join("; "),
            unloaded_preview.join("; "),
            diagnostic.detail.clone(),
        ];
        csv.push_str(
            &fields
                .into_iter()
                .map(waveform_load_csv_escape)
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    csv
}

fn waveform_load_probe_summary_counts(
    diagnostics: &[WaveformLoadDiagnostic],
    diagnostic: &WaveformLoadDiagnostic,
) -> String {
    if diagnostic.probe_preview.is_empty() {
        return if diagnostic.loaded {
            "all columns".to_string()
        } else {
            "no preview".to_string()
        };
    }
    let loaded_preview = waveform_load_loaded_preview_for_diagnostic(diagnostics, diagnostic);
    let unloaded_preview =
        waveform_load_unloaded_preview_for_diagnostic(diagnostic, &loaded_preview);
    if diagnostic.deferred {
        format!(
            "{} preview; {} loaded; {} unloaded",
            diagnostic.probe_preview.len(),
            loaded_preview.len(),
            unloaded_preview.len()
        )
    } else if diagnostic.loaded {
        format!("{} selected loaded", diagnostic.probe_preview.len())
    } else {
        format!("{} selected requested", diagnostic.probe_preview.len())
    }
}

pub(super) fn waveform_load_diagnostic_unloaded_preview_columns(
    diagnostics: &[WaveformLoadDiagnostic],
    diagnostic: &WaveformLoadDiagnostic,
) -> Vec<String> {
    let loaded_preview = waveform_load_loaded_preview_for_diagnostic(diagnostics, diagnostic);
    waveform_load_unloaded_preview_for_diagnostic(diagnostic, &loaded_preview)
}

fn waveform_load_probe_summary_preview(
    diagnostics: &[WaveformLoadDiagnostic],
    diagnostic: &WaveformLoadDiagnostic,
) -> String {
    if diagnostic.probe_preview.is_empty() {
        return String::new();
    }
    if diagnostic.deferred {
        let loaded_preview = waveform_load_loaded_preview_for_diagnostic(diagnostics, diagnostic);
        let unloaded_preview =
            waveform_load_unloaded_preview_for_diagnostic(diagnostic, &loaded_preview);
        let mut parts = Vec::new();
        parts.push(format!(
            "preview: {}",
            waveform_load_probe_preview_compact(&diagnostic.probe_preview, 4)
        ));
        if !loaded_preview.is_empty() {
            parts.push(format!(
                "loaded: {}",
                waveform_load_probe_preview_compact(&loaded_preview, 3)
            ));
        }
        if !unloaded_preview.is_empty() {
            parts.push(format!(
                "unloaded: {}",
                waveform_load_probe_preview_compact(&unloaded_preview, 3)
            ));
        }
        return parts.join(" | ");
    }
    format!(
        "columns: {}",
        waveform_load_probe_preview_compact(&diagnostic.probe_preview, 4)
    )
}

fn waveform_load_loaded_preview_for_diagnostic(
    diagnostics: &[WaveformLoadDiagnostic],
    diagnostic: &WaveformLoadDiagnostic,
) -> Vec<String> {
    if diagnostic.deferred {
        waveform_load_selected_probes_for_path(diagnostics, &diagnostic.path)
    } else if diagnostic.loaded && diagnostic.is_selected_column_update() {
        diagnostic.probe_preview.clone()
    } else {
        Vec::new()
    }
}

fn waveform_load_unloaded_preview_for_diagnostic(
    diagnostic: &WaveformLoadDiagnostic,
    loaded_preview: &[String],
) -> Vec<String> {
    diagnostic
        .probe_preview
        .iter()
        .filter(|probe| {
            !loaded_preview
                .iter()
                .any(|loaded| loaded.trim().eq_ignore_ascii_case(probe.trim()))
        })
        .cloned()
        .collect()
}

fn waveform_load_probe_preview_compact(probes: &[String], max_visible: usize) -> String {
    if probes.is_empty() {
        return "none".to_string();
    }
    let visible = probes
        .iter()
        .take(max_visible)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if probes.len() > max_visible {
        format!("{visible}, +{} more", probes.len() - max_visible)
    } else {
        visible
    }
}

fn waveform_load_csv_escape(value: String) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}
