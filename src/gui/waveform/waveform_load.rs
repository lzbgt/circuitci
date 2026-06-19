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
    pub(super) probe_preview: Vec<String>,
    pub(super) elapsed_ms: u128,
    pub(super) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WaveformLoadPreflight {
    pub(super) bytes: Option<u64>,
    pub(super) estimated_rows: Option<usize>,
    pub(super) probe_preview: Vec<String>,
    pub(super) warning: bool,
    pub(super) summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeferredWaveformArtifact {
    pub(crate) path: String,
    pub(crate) label: String,
    pub(crate) size_label: String,
    pub(crate) samples: usize,
    pub(crate) probes: usize,
    pub(crate) probe_preview: Vec<String>,
    pub(crate) loaded_probe_preview: Vec<String>,
    pub(crate) detail: String,
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
            probe_preview: Vec::new(),
            elapsed_ms,
            detail: format!("Loaded {samples} sample row(s) across {probes} probe(s)."),
        }
    }

    pub(super) fn loaded_selected(
        path: String,
        bytes: Option<u64>,
        samples: usize,
        probes: usize,
        elapsed_ms: u128,
        probe_preview: Vec<String>,
    ) -> Self {
        Self {
            path,
            loaded: true,
            deferred: false,
            bytes,
            samples,
            probes,
            probe_preview,
            elapsed_ms,
            detail: format!(
                "Loaded {samples} sample row(s) across {probes} selected probe column(s); full deferred artifact remains available."
            ),
        }
    }

    pub(super) fn skipped_selected(
        path: String,
        bytes: Option<u64>,
        elapsed_ms: u128,
        probe_preview: Vec<String>,
        detail: String,
    ) -> Self {
        Self {
            path,
            loaded: false,
            deferred: false,
            bytes,
            samples: 0,
            probes: probe_preview.len(),
            probe_preview,
            elapsed_ms,
            detail: format!("Selected probe column load failed: {detail}"),
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
            probe_preview: Vec::new(),
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
            probes: preflight.probe_preview.len(),
            probe_preview: preflight.probe_preview.clone(),
            elapsed_ms,
            detail: format!(
                "Deferred large waveform artifact; use Load Deferred to parse it when needed ({})",
                preflight.summary
            ),
        }
    }

    pub(super) fn status_label(&self) -> &'static str {
        if self.loaded {
            "Loaded"
        } else if self.deferred {
            "Deferred"
        } else {
            "Skipped"
        }
    }

    pub(super) fn csv_status(&self) -> &'static str {
        if self.loaded {
            "loaded"
        } else if self.deferred {
            "deferred"
        } else {
            "skipped"
        }
    }

    pub(super) fn is_selected_column_update(&self) -> bool {
        !self.deferred && !self.probe_preview.is_empty()
    }
}

pub(super) fn waveform_load_preflight(path: &Path) -> WaveformLoadPreflight {
    let bytes = path.metadata().ok().map(|metadata| metadata.len());
    let sample = inspect_waveform_sample(path, bytes).unwrap_or_default();
    let estimated_rows = sample.estimated_rows;
    let probe_preview = sample.probe_preview;
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
        probe_preview,
        warning,
        summary,
    }
}

#[derive(Default)]
struct WaveformLoadSample {
    estimated_rows: Option<usize>,
    probe_preview: Vec<String>,
}

fn inspect_waveform_sample(path: &Path, bytes: Option<u64>) -> std::io::Result<WaveformLoadSample> {
    let Some(total_bytes) = bytes else {
        return Ok(WaveformLoadSample::default());
    };
    if total_bytes == 0 {
        return Ok(WaveformLoadSample {
            estimated_rows: Some(0),
            probe_preview: Vec::new(),
        });
    }
    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0_u8; PREFLIGHT_SAMPLE_BYTES.min(total_bytes as usize)];
    let bytes_read = file.read(&mut buffer)?;
    if bytes_read == 0 {
        return Ok(WaveformLoadSample {
            estimated_rows: Some(0),
            probe_preview: Vec::new(),
        });
    }
    buffer.truncate(bytes_read);
    let line_breaks = buffer.iter().filter(|byte| **byte == b'\n').count();
    let sample_rows = line_breaks.saturating_sub(1);
    let probe_preview = waveform_probe_preview_from_sample(&buffer);
    if bytes_read as u64 >= total_bytes {
        return Ok(WaveformLoadSample {
            estimated_rows: Some(sample_rows),
            probe_preview,
        });
    }
    if line_breaks <= 1 {
        return Ok(WaveformLoadSample {
            estimated_rows: None,
            probe_preview,
        });
    }
    let estimated_total_lines = ((line_breaks as u128) * (total_bytes as u128)
        + (bytes_read as u128).saturating_sub(1))
        / bytes_read as u128;
    let estimated_rows = estimated_total_lines
        .saturating_sub(1)
        .min(usize::MAX as u128) as usize;
    Ok(WaveformLoadSample {
        estimated_rows: Some(estimated_rows),
        probe_preview,
    })
}

fn waveform_probe_preview_from_sample(buffer: &[u8]) -> Vec<String> {
    let sample = String::from_utf8_lossy(buffer);
    for line in sample.lines() {
        let fields = split_waveform_preview_fields(line);
        if fields.len() <= 1 {
            continue;
        }
        if fields[0].parse::<f64>().ok().is_some_and(f64::is_finite) {
            continue;
        }
        return fields.into_iter().skip(1).map(str::to_string).collect();
    }
    Vec::new()
}

fn split_waveform_preview_fields(line: &str) -> Vec<&str> {
    line.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

pub(crate) fn waveform_load_deferred_paths(diagnostics: &[WaveformLoadDiagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.deferred)
        .map(|diagnostic| diagnostic.path.clone())
        .collect()
}

pub(crate) fn waveform_load_deferred_artifacts(
    diagnostics: &[WaveformLoadDiagnostic],
) -> Vec<DeferredWaveformArtifact> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.deferred)
        .map(|diagnostic| DeferredWaveformArtifact {
            path: diagnostic.path.clone(),
            label: deferred_waveform_label(&diagnostic.path),
            size_label: format_waveform_load_bytes(diagnostic.bytes),
            samples: diagnostic.samples,
            probes: diagnostic.probes,
            probe_preview: diagnostic.probe_preview.clone(),
            loaded_probe_preview: waveform_load_selected_probes_for_path(
                diagnostics,
                &diagnostic.path,
            ),
            detail: diagnostic.detail.clone(),
        })
        .collect()
}

pub(super) fn waveform_load_selected_probes_for_path(
    diagnostics: &[WaveformLoadDiagnostic],
    path: &str,
) -> Vec<String> {
    let mut probes = Vec::new();
    for diagnostic in diagnostics {
        if diagnostic.path != path
            || !diagnostic.loaded
            || diagnostic.deferred
            || !diagnostic.is_selected_column_update()
        {
            continue;
        }
        for probe in &diagnostic.probe_preview {
            if !probes
                .iter()
                .any(|loaded: &String| loaded.eq_ignore_ascii_case(probe))
            {
                probes.push(probe.clone());
            }
        }
    }
    probes
}

pub(crate) fn merge_waveform_load_diagnostics(
    diagnostics: &mut Vec<WaveformLoadDiagnostic>,
    updates: Vec<WaveformLoadDiagnostic>,
) {
    for update in updates {
        if update.loaded && !update.deferred && !update.is_selected_column_update() {
            diagnostics.retain(|diagnostic| diagnostic.path != update.path);
            diagnostics.push(update);
            continue;
        }
        if let Some(existing) = diagnostics.iter_mut().find(|diagnostic| {
            diagnostic.path == update.path
                && diagnostic.deferred == update.deferred
                && diagnostic.is_selected_column_update() == update.is_selected_column_update()
                && (!update.is_selected_column_update()
                    || diagnostic.probe_preview == update.probe_preview)
        }) {
            *existing = update;
        } else {
            diagnostics.push(update);
        }
    }
}

fn deferred_waveform_label(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .filter(|file_name| !file_name.is_empty())
        .unwrap_or(path)
        .to_string()
}

pub(super) fn format_waveform_load_bytes(bytes: Option<u64>) -> String {
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
