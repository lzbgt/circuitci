use crate::gui::CircuitCiApp;
use eframe::egui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WaveformLoadDiagnostic {
    pub(super) path: String,
    pub(super) loaded: bool,
    pub(super) bytes: Option<u64>,
    pub(super) samples: usize,
    pub(super) probes: usize,
    pub(super) elapsed_ms: u128,
    pub(super) detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaveformLoadStatusFilter {
    All,
    Loaded,
    Skipped,
}

impl WaveformLoadStatusFilter {
    pub(super) const ALL: [Self; 3] = [Self::All, Self::Loaded, Self::Skipped];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Loaded => "Loaded",
            Self::Skipped => "Skipped",
        }
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
        let skipped = self.waveform_load_diagnostics.len().saturating_sub(loaded);

        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Waveform Load Diagnostics");
                ui.label(format!("{} loaded, {} skipped", loaded, skipped));
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
                .num_columns(7)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Status");
                    ui.label("File");
                    ui.label("Size");
                    ui.label("Rows");
                    ui.label("Probes");
                    ui.label("Load");
                    ui.label("Detail");
                    ui.end_row();

                    for &index in &visible_indexes {
                        let diagnostic = &self.waveform_load_diagnostics[index];
                        ui.label(if diagnostic.loaded {
                            "Loaded"
                        } else {
                            "Skipped"
                        });
                        ui.monospace(&diagnostic.path);
                        ui.monospace(format_waveform_load_bytes(diagnostic.bytes));
                        ui.monospace(diagnostic.samples.to_string());
                        ui.monospace(diagnostic.probes.to_string());
                        ui.monospace(format!("{} ms", diagnostic.elapsed_ms));
                        ui.label(&diagnostic.detail);
                        ui.end_row();
                    }
                });
        });
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
                    | (WaveformLoadStatusFilter::Skipped, false)
            ) {
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
        if diagnostic.loaded {
            "loaded"
        } else {
            "skipped"
        },
        diagnostic.path,
        diagnostic.detail
    )
    .to_ascii_lowercase()
}

pub(super) fn waveform_load_diagnostics_csv(rows: &[&WaveformLoadDiagnostic]) -> String {
    let mut csv = String::from("status,path,size_bytes,samples,probes,elapsed_ms,detail\n");
    for diagnostic in rows {
        let fields = [
            if diagnostic.loaded {
                "loaded"
            } else {
                "skipped"
            }
            .to_string(),
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
