use crate::gui::CircuitCiApp;
use eframe::egui;
use std::io::Read;
use std::path::Path;

const PREFLIGHT_SAMPLE_BYTES: usize = 64 * 1024;
const LARGE_WAVEFORM_BYTES: u64 = 50 * 1024 * 1024;
const LARGE_WAVEFORM_ROWS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WaveformLoadDiagnostic {
    pub(super) path: String,
    pub(super) loaded: bool,
    pub(super) deferred: bool,
    pub(super) bytes: Option<u64>,
    pub(super) samples: usize,
    pub(super) probes: usize,
    pub(super) elapsed_ms: u128,
    pub(super) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WaveformLoadPreflight {
    pub(super) bytes: Option<u64>,
    pub(super) estimated_rows: Option<usize>,
    pub(super) warning: bool,
    pub(super) summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaveformLoadStatusFilter {
    All,
    Loaded,
    Deferred,
    Skipped,
}

impl WaveformLoadStatusFilter {
    pub(super) const ALL: [Self; 4] = [Self::All, Self::Loaded, Self::Deferred, Self::Skipped];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Loaded => "Loaded",
            Self::Deferred => "Deferred",
            Self::Skipped => "Skipped",
        }
    }
}

impl WaveformLoadDiagnostic {
    pub(super) fn loaded(
        path: String,
        bytes: Option<u64>,
        samples: usize,
        probes: usize,
        elapsed_ms: u128,
    ) -> Self {
        Self {
            path,
            loaded: true,
            deferred: false,
            bytes,
            samples,
            probes,
            elapsed_ms,
            detail: format!("Loaded {samples} sample row(s) across {probes} probe(s)."),
        }
    }

    pub(super) fn skipped(
        path: String,
        bytes: Option<u64>,
        elapsed_ms: u128,
        detail: String,
    ) -> Self {
        Self {
            path,
            loaded: false,
            deferred: false,
            bytes,
            samples: 0,
            probes: 0,
            elapsed_ms,
            detail,
        }
    }

    pub(super) fn deferred(
        path: String,
        preflight: &WaveformLoadPreflight,
        elapsed_ms: u128,
    ) -> Self {
        Self {
            path,
            loaded: false,
            deferred: true,
            bytes: preflight.bytes,
            samples: preflight.estimated_rows.unwrap_or(0),
            probes: 0,
            elapsed_ms,
            detail: format!(
                "Deferred large waveform artifact; use Load Deferred to parse it when needed ({})",
                preflight.summary
            ),
        }
    }

    fn status_label(&self) -> &'static str {
        if self.loaded {
            "Loaded"
        } else if self.deferred {
            "Deferred"
        } else {
            "Skipped"
        }
    }

    fn csv_status(&self) -> &'static str {
        if self.loaded {
            "loaded"
        } else if self.deferred {
            "deferred"
        } else {
            "skipped"
        }
    }
}

pub(super) fn waveform_load_preflight(path: &Path) -> WaveformLoadPreflight {
    let bytes = path.metadata().ok().map(|metadata| metadata.len());
    let estimated_rows = estimate_waveform_data_rows(path, bytes).ok().flatten();
    let warning = bytes.is_some_and(|bytes| bytes >= LARGE_WAVEFORM_BYTES)
        || estimated_rows.is_some_and(|rows| rows >= LARGE_WAVEFORM_ROWS);
    let mut parts = Vec::new();
    match bytes {
        Some(bytes) => parts.push(format!(
            "{} on disk",
            format_waveform_load_bytes(Some(bytes))
        )),
        None => parts.push("size unknown".to_string()),
    }
    match estimated_rows {
        Some(rows) => parts.push(format!("~{rows} data row(s)")),
        None => parts.push("row estimate unavailable".to_string()),
    }
    let summary = parts.join(", ");
    WaveformLoadPreflight {
        bytes,
        estimated_rows,
        warning,
        summary,
    }
}

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
                    let rows: Vec<_> = visible_indexes
                        .iter()
                        .filter_map(|&index| self.waveform_load_diagnostics.get(index))
                        .collect();
                    ui.ctx().copy_text(waveform_load_diagnostics_csv(&rows));
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
                .num_columns(8)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Status");
                    ui.label("File");
                    ui.label("Size");
                    ui.label("Rows");
                    ui.label("Probes");
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
                        ui.monospace(format!("{} ms", diagnostic.elapsed_ms));
                        ui.label(&diagnostic.detail);
                        if diagnostic.deferred {
                            if ui.small_button("Load").clicked() {
                                load_deferred_path = Some(diagnostic.path.clone());
                            }
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
    }
}

fn estimate_waveform_data_rows(path: &Path, bytes: Option<u64>) -> std::io::Result<Option<usize>> {
    let Some(total_bytes) = bytes else {
        return Ok(None);
    };
    if total_bytes == 0 {
        return Ok(Some(0));
    }
    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0_u8; PREFLIGHT_SAMPLE_BYTES.min(total_bytes as usize)];
    let bytes_read = file.read(&mut buffer)?;
    if bytes_read == 0 {
        return Ok(Some(0));
    }
    buffer.truncate(bytes_read);
    let line_breaks = buffer.iter().filter(|byte| **byte == b'\n').count();
    let sample_rows = line_breaks.saturating_sub(1);
    if bytes_read as u64 >= total_bytes {
        return Ok(Some(sample_rows));
    }
    if line_breaks <= 1 {
        return Ok(None);
    }
    let estimated_total_lines = ((line_breaks as u128) * (total_bytes as u128)
        + (bytes_read as u128).saturating_sub(1))
        / bytes_read as u128;
    let estimated_rows = estimated_total_lines
        .saturating_sub(1)
        .min(usize::MAX as u128) as usize;
    Ok(Some(estimated_rows))
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
        "{} {} {}",
        diagnostic.csv_status(),
        diagnostic.path,
        diagnostic.detail
    )
    .to_ascii_lowercase()
}

pub(super) fn waveform_load_diagnostics_csv(rows: &[&WaveformLoadDiagnostic]) -> String {
    let mut csv = String::from("status,path,size_bytes,samples,probes,elapsed_ms,detail\n");
    for diagnostic in rows {
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

pub(crate) fn waveform_load_deferred_paths(diagnostics: &[WaveformLoadDiagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.deferred)
        .map(|diagnostic| diagnostic.path.clone())
        .collect()
}

pub(crate) fn merge_waveform_load_diagnostics(
    diagnostics: &mut Vec<WaveformLoadDiagnostic>,
    updates: Vec<WaveformLoadDiagnostic>,
) {
    for update in updates {
        if let Some(existing) = diagnostics
            .iter_mut()
            .find(|diagnostic| diagnostic.path == update.path)
        {
            *existing = update;
        } else {
            diagnostics.push(update);
        }
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

fn format_waveform_load_bytes(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "unknown".to_string();
    };
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes / KIB)
    } else {
        format!("{bytes:.0} B")
    }
}
