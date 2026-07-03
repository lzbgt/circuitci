use super::waveform_load::waveform_load_preflight;
use super::{
    WaveformLoadDiagnostic, WaveformProbe, WaveformProbeQuantity, WaveformView, WaveformXAxis,
};
use crate::reports::ValidationReport;
use anyhow::{Context, Result};
use std::path::Path;
use std::time::Instant;

pub(in crate::gui) fn load_report_waveforms_with_progress_and_cancel<F, C>(
    report: &ValidationReport,
    on_progress: F,
    should_cancel: C,
    defer_large_waveforms: bool,
) -> Result<(Vec<WaveformView>, Vec<WaveformLoadDiagnostic>)>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let waveform_paths = report_waveform_paths(report);
    load_waveform_paths_with_progress_and_cancel(
        &waveform_paths,
        on_progress,
        should_cancel,
        defer_large_waveforms,
    )
}

pub(in crate::gui) fn load_waveform_paths_with_progress_and_cancel<F, C>(
    waveform_paths: &[String],
    mut on_progress: F,
    should_cancel: C,
    defer_large_waveforms: bool,
) -> Result<(Vec<WaveformView>, Vec<WaveformLoadDiagnostic>)>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let requests: Vec<_> = waveform_paths
        .iter()
        .cloned()
        .map(WaveformLoadRequest::all_columns)
        .collect();
    load_waveform_requests_with_progress_and_cancel(
        &requests,
        &mut on_progress,
        should_cancel,
        defer_large_waveforms,
    )
}

pub(in crate::gui) fn load_waveform_requests_with_progress_and_cancel<F, C>(
    waveform_requests: &[WaveformLoadRequest],
    mut on_progress: F,
    should_cancel: C,
    defer_large_waveforms: bool,
) -> Result<(Vec<WaveformView>, Vec<WaveformLoadDiagnostic>)>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let waveform_count = waveform_requests.len();
    let mut waveforms = Vec::new();
    let mut diagnostics = Vec::new();
    for (index, request) in waveform_requests.iter().enumerate() {
        if should_cancel() {
            return Err(crate::cancellation::canceled(
                "Waveform loading canceled before completion.",
            ));
        }
        let started_at = Instant::now();
        let path = Path::new(&request.path);
        let preflight = waveform_load_preflight(path);
        let bytes = preflight.bytes;
        on_progress(
            "Loading waveforms",
            format!(
                "Loading waveform {} of {}: {}.",
                index + 1,
                waveform_count,
                request.path
            ),
        );
        let preflight_stage = if preflight.warning {
            "Large waveform artifact"
        } else {
            "Waveform preflight"
        };
        on_progress(
            preflight_stage,
            format!("{}: {}.", request.path, preflight.summary),
        );
        if defer_large_waveforms && preflight.warning {
            diagnostics.push(WaveformLoadDiagnostic::deferred(
                request.path.clone(),
                &preflight,
                started_at.elapsed().as_millis(),
            ));
            on_progress(
                "Deferred waveform artifact",
                format!("Deferred waveform {}: {}.", request.path, preflight.summary),
            );
            continue;
        }
        match load_waveform_csv_selected_with_progress_and_cancel(
            path,
            &request.path,
            &request.probe_labels,
            &mut on_progress,
            &should_cancel,
        ) {
            Ok(view) => {
                let samples = view.time_s.len();
                let probes = view.probes.len();
                let elapsed_ms = started_at.elapsed().as_millis();
                let diagnostic = if request.probe_labels.is_empty() {
                    WaveformLoadDiagnostic::loaded(
                        request.path.clone(),
                        bytes,
                        samples,
                        probes,
                        elapsed_ms,
                    )
                } else {
                    WaveformLoadDiagnostic::loaded_selected(
                        request.path.clone(),
                        bytes,
                        samples,
                        probes,
                        elapsed_ms,
                        request.probe_labels.clone(),
                    )
                };
                diagnostics.push(diagnostic);
                waveforms.push(view);
            }
            Err(error) if crate::cancellation::is_canceled(&error) => return Err(error),
            Err(error) => {
                let detail = format!("{error:#}");
                let diagnostic = if request.probe_labels.is_empty() {
                    WaveformLoadDiagnostic::skipped(
                        request.path.clone(),
                        bytes,
                        started_at.elapsed().as_millis(),
                        detail.clone(),
                    )
                } else {
                    WaveformLoadDiagnostic::skipped_selected(
                        request.path.clone(),
                        bytes,
                        started_at.elapsed().as_millis(),
                        request.probe_labels.clone(),
                        detail.clone(),
                    )
                };
                diagnostics.push(diagnostic);
                on_progress(
                    "Skipping waveform",
                    format!("Skipped waveform {}: {detail}.", request.path),
                );
            }
        }
    }
    Ok((waveforms, diagnostics))
}

fn report_waveform_paths(report: &ValidationReport) -> Vec<String> {
    let mut paths = report.waveforms.clone();
    for artifact in &report.artifacts {
        if is_hb_spectrum_path(artifact) && !paths.iter().any(|path| path == artifact) {
            paths.push(artifact.clone());
        }
    }
    paths
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::gui) struct WaveformLoadRequest {
    path: String,
    probe_labels: Vec<String>,
}

impl WaveformLoadRequest {
    pub(in crate::gui) fn all_columns(path: String) -> Self {
        Self {
            path,
            probe_labels: Vec::new(),
        }
    }

    pub(in crate::gui) fn selected_columns(path: String, probe_labels: Vec<String>) -> Self {
        Self { path, probe_labels }
    }

    pub(in crate::gui) fn probe_count(&self) -> usize {
        self.probe_labels.len()
    }
}

#[cfg(test)]
pub(super) fn load_waveform_csv_with_progress_and_cancel<F, C>(
    path: &Path,
    label: &str,
    mut on_progress: F,
    should_cancel: C,
) -> Result<WaveformView>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    load_waveform_csv_selected_with_progress_and_cancel(
        path,
        label,
        &[],
        &mut on_progress,
        should_cancel,
    )
}

fn load_waveform_csv_selected_with_progress_and_cancel<F, C>(
    path: &Path,
    label: &str,
    selected_probe_labels: &[String],
    mut on_progress: F,
    should_cancel: C,
) -> Result<WaveformView>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    use std::io::BufRead;

    if selected_probe_labels.is_empty() && is_hb_spectrum_path(label) {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read HB spectrum CSV {}.", path.display()))?;
        return parse_hb_spectrum_csv_rows(&text, label);
    }

    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to read waveform CSV {}.", path.display()))?;
    let total_bytes = file.metadata().ok().map(|metadata| metadata.len());
    let mut reader = std::io::BufReader::new(file);
    let mut line = String::new();
    let mut line_index = 0usize;
    let mut bytes_read = 0u64;
    let mut last_progress_bytes = 0u64;
    let mut builder = WaveformCsvBuilder::new(selected_probe_labels.to_vec());

    loop {
        if line_index.is_multiple_of(4096) && should_cancel() {
            return Err(crate::cancellation::canceled(format!(
                "Waveform loading canceled while reading {}.",
                path.display()
            )));
        }
        line.clear();
        let byte_count = reader
            .read_line(&mut line)
            .with_context(|| format!("Failed to read waveform CSV {}.", path.display()))?;
        if byte_count == 0 {
            break;
        }
        bytes_read += byte_count as u64;
        builder.ingest_line(line_index, &line)?;
        line_index += 1;

        if bytes_read.saturating_sub(last_progress_bytes) >= 1_048_576 {
            last_progress_bytes = bytes_read;
            let progress = total_bytes
                .filter(|total| *total > 0)
                .map(|total| format!("{:.0}%", (bytes_read as f64 / total as f64) * 100.0))
                .unwrap_or_else(|| format!("{} KiB", bytes_read / 1024));
            on_progress(
                "Loading waveforms",
                format!(
                    "{}: {progress}, {} sample row(s).",
                    label,
                    builder.sample_count()
                ),
            );
        }
    }

    if should_cancel() {
        return Err(crate::cancellation::canceled(format!(
            "Waveform loading canceled while reading {}.",
            path.display()
        )));
    }
    builder.finish(label)
}

#[cfg(test)]
pub(super) fn parse_waveform_csv_text(text: &str, label: &str) -> Result<WaveformView> {
    let mut builder = WaveformCsvBuilder::new(Vec::new());
    for (line_index, line) in text.lines().enumerate() {
        builder.ingest_line(line_index, line)?;
    }
    builder.finish(label)
}

#[cfg(test)]
pub(super) fn parse_hb_spectrum_csv_text(text: &str, label: &str) -> Result<WaveformView> {
    parse_hb_spectrum_csv_rows(text, label)
}

fn is_hb_spectrum_path(path: &str) -> bool {
    path.ends_with("/hb_spectrum.csv") || path == "hb_spectrum.csv"
}

fn parse_hb_spectrum_csv_rows(text: &str, label: &str) -> Result<WaveformView> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines.next().context("HB spectrum CSV has no header row.")?;
    let header = split_waveform_fields(header);
    if header
        != [
            "output_expression",
            "fundamental_frequency_hz",
            "harmonic",
            "frequency_hz",
            "real",
            "imaginary",
            "magnitude",
            "phase_deg",
        ]
    {
        anyhow::bail!(
            "HB spectrum CSV header must be output_expression,fundamental_frequency_hz,harmonic,frequency_hz,real,imaginary,magnitude,phase_deg."
        );
    }

    let mut rows = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let fields = split_waveform_fields(line);
        if fields.len() < 8 {
            anyhow::bail!(
                "HB spectrum row {} has {} fields, expected at least 8.",
                line_index + 2,
                fields.len()
            );
        }
        let numeric_offset = fields.len() - 7;
        let expression = fields[..numeric_offset].join(",");
        let frequency_hz = parse_waveform_float(fields[numeric_offset + 2]).with_context(|| {
            format!("HB spectrum row {} has invalid frequency.", line_index + 2)
        })?;
        if frequency_hz < 0.0 {
            continue;
        }
        let real = parse_waveform_float(fields[numeric_offset + 3]).with_context(|| {
            format!("HB spectrum row {} has invalid real value.", line_index + 2)
        })?;
        let imaginary = parse_waveform_float(fields[numeric_offset + 4]).with_context(|| {
            format!(
                "HB spectrum row {} has invalid imaginary value.",
                line_index + 2
            )
        })?;
        let magnitude = parse_waveform_float(fields[numeric_offset + 5]).with_context(|| {
            format!(
                "HB spectrum row {} has invalid magnitude value.",
                line_index + 2
            )
        })?;
        let phase_deg = parse_waveform_float(fields[numeric_offset + 6]).with_context(|| {
            format!(
                "HB spectrum row {} has invalid phase value.",
                line_index + 2
            )
        })?;
        rows.push((
            frequency_hz,
            expression,
            real,
            imaginary,
            magnitude,
            phase_deg,
        ));
    }

    if rows.is_empty() {
        anyhow::bail!("HB spectrum CSV has no non-negative frequency rows.");
    }
    rows.sort_by(|left, right| left.0.total_cmp(&right.0));
    let expression = rows[0].1.clone();
    if rows
        .iter()
        .any(|(_, row_expression, ..)| row_expression != &expression)
    {
        anyhow::bail!("HB spectrum CSV must contain one output expression per artifact.");
    }
    let mut time_s = Vec::with_capacity(rows.len());
    let mut real_values = Vec::with_capacity(rows.len());
    let mut imaginary_values = Vec::with_capacity(rows.len());
    let mut magnitude_values = Vec::with_capacity(rows.len());
    let mut phase_values = Vec::with_capacity(rows.len());
    let mut previous_frequency = None;
    for (frequency_hz, _, real, imaginary, magnitude, phase_deg) in rows {
        if previous_frequency.is_some_and(|previous| frequency_hz <= previous) {
            anyhow::bail!(
                "HB spectrum CSV has duplicate or non-increasing non-negative frequency {frequency_hz}."
            );
        }
        previous_frequency = Some(frequency_hz);
        time_s.push(WaveformXAxis::FrequencyHz.storage_from_csv_value(frequency_hz));
        real_values.push(real);
        imaginary_values.push(imaginary);
        magnitude_values.push(magnitude);
        phase_values.push(phase_deg);
    }

    Ok(WaveformView {
        label: label.to_string(),
        path: label.to_string(),
        x_axis: WaveformXAxis::FrequencyHz,
        time_s,
        probes: vec![
            WaveformProbe {
                label: format!("{expression} magnitude"),
                values: magnitude_values,
                derived: false,
                expression: Some(expression.clone()),
                promoted_quantity: waveform_probe_quantity_from_label(&expression),
            },
            WaveformProbe {
                label: format!("{expression} phase deg"),
                values: phase_values,
                derived: false,
                expression: Some(expression.clone()),
                promoted_quantity: None,
            },
            WaveformProbe {
                label: format!("{expression} real"),
                values: real_values,
                derived: false,
                expression: Some(expression.clone()),
                promoted_quantity: waveform_probe_quantity_from_label(&expression),
            },
            WaveformProbe {
                label: format!("{expression} imaginary"),
                values: imaginary_values,
                derived: false,
                expression: Some(expression),
                promoted_quantity: None,
            },
        ],
    })
}

#[derive(Default)]
struct WaveformCsvBuilder {
    x_axis: WaveformXAxis,
    time_s: Vec<f64>,
    probe_labels: Vec<String>,
    probe_values: Vec<Vec<f64>>,
    selected_probe_labels: Vec<String>,
    selected_probe_columns: Option<Vec<usize>>,
}

impl WaveformCsvBuilder {
    fn new(selected_probe_labels: Vec<String>) -> Self {
        Self {
            selected_probe_labels,
            ..Self::default()
        }
    }

    fn sample_count(&self) -> usize {
        self.time_s.len()
    }

    fn ingest_line(&mut self, line_index: usize, line: &str) -> Result<()> {
        let fields = split_waveform_fields(line);
        if fields.is_empty() {
            return Ok(());
        }
        let Some(time) = parse_waveform_float(fields[0]) else {
            if self.time_s.is_empty() {
                self.x_axis = waveform_x_axis_from_header(fields[0]);
                let labels: Vec<_> = fields
                    .iter()
                    .skip(1)
                    .map(|field| waveform_probe_label_from_header(field))
                    .collect();
                self.apply_header_labels(labels)?;
                return Ok(());
            }
            anyhow::bail!(
                "Waveform row {} has non-numeric time value {}.",
                line_index + 1,
                fields[0]
            );
        };
        let x_value = self.x_axis.storage_from_csv_value(time);
        if let Some(previous) = self.time_s.last()
            && x_value <= *previous
        {
            anyhow::bail!(
                "Waveform row {} has non-increasing time value {}.",
                line_index + 1,
                fields[0]
            );
        }
        let probe_count = fields.len().saturating_sub(1);
        if probe_count == 0 {
            anyhow::bail!("Waveform row {} has no probe columns.", line_index + 1);
        }
        if self.probe_values.is_empty() {
            if !self.selected_probe_labels.is_empty() && self.selected_probe_columns.is_none() {
                anyhow::bail!(
                    "Column-selective waveform loading requires a header row before numeric samples."
                );
            }
            let selected_columns = self
                .selected_probe_columns
                .clone()
                .unwrap_or_else(|| (0..probe_count).collect());
            if selected_columns.is_empty() {
                anyhow::bail!("Waveform selection did not match any probe columns.");
            }
            let required_probe_count = selected_columns
                .iter()
                .copied()
                .max()
                .map(|index| index + 1)
                .unwrap_or(0);
            if probe_count < required_probe_count {
                anyhow::bail!(
                    "Waveform row {} has {} probe columns, expected at least {}.",
                    line_index + 1,
                    probe_count,
                    required_probe_count
                );
            }
            self.selected_probe_columns = Some(selected_columns);
            let selected_count = self.selected_probe_columns.as_ref().map_or(0, Vec::len);
            self.probe_values = vec![Vec::new(); selected_count];
            if self.probe_labels.len() != selected_count {
                let labels = self
                    .selected_probe_columns
                    .as_ref()
                    .into_iter()
                    .flatten()
                    .map(|index| format!("probe_{}", index + 1))
                    .collect::<Vec<_>>();
                self.probe_labels = labels;
            }
        } else {
            let required_probe_count = self
                .selected_probe_columns
                .as_ref()
                .into_iter()
                .flatten()
                .copied()
                .max()
                .map(|index| index + 1)
                .unwrap_or(self.probe_values.len());
            if probe_count < required_probe_count {
                anyhow::bail!(
                    "Waveform row {} has {} probe columns, expected at least {}.",
                    line_index + 1,
                    probe_count,
                    required_probe_count
                );
            }
        }
        if probe_count < self.probe_values.len() {
            anyhow::bail!(
                "Waveform row {} has {} probe columns, expected at least {}.",
                line_index + 1,
                probe_count,
                self.probe_values.len()
            );
        }
        self.time_s.push(x_value);
        let selected_columns = self
            .selected_probe_columns
            .clone()
            .unwrap_or_else(|| (0..self.probe_values.len()).collect());
        for (values_index, column_index) in selected_columns.into_iter().enumerate() {
            let value = parse_waveform_float(fields[column_index + 1]).with_context(|| {
                format!(
                    "Waveform row {} has non-numeric probe value {}.",
                    line_index + 1,
                    fields[column_index + 1]
                )
            })?;
            self.probe_values[values_index].push(value);
        }
        Ok(())
    }

    fn apply_header_labels(&mut self, labels: Vec<String>) -> Result<()> {
        if self.selected_probe_labels.is_empty() {
            self.probe_labels = labels;
            self.selected_probe_columns = None;
            return Ok(());
        }
        let mut columns = Vec::new();
        let mut selected_labels = Vec::new();
        for requested in &self.selected_probe_labels {
            let requested = waveform_probe_label_from_header(requested);
            let Some((index, label)) = labels
                .iter()
                .enumerate()
                .find(|(_, label)| label.trim().eq_ignore_ascii_case(requested.trim()))
            else {
                continue;
            };
            if !columns.contains(&index) {
                columns.push(index);
                selected_labels.push(label.clone());
            }
        }
        if columns.is_empty() {
            anyhow::bail!(
                "Waveform header does not contain requested probe column(s): {}.",
                self.selected_probe_labels.join(", ")
            );
        }
        self.probe_labels = selected_labels;
        self.selected_probe_columns = Some(columns);
        Ok(())
    }

    fn finish(self, label: &str) -> Result<WaveformView> {
        let Self {
            time_s,
            probe_labels,
            probe_values,
            x_axis,
            ..
        } = self;
        if time_s.is_empty() {
            anyhow::bail!("Waveform CSV has no numeric samples.");
        }

        let probes = probe_labels
            .into_iter()
            .zip(probe_values)
            .map(|(label, values)| WaveformProbe {
                promoted_quantity: waveform_probe_quantity_from_label(&label),
                label,
                values,
                derived: false,
                expression: None,
            })
            .collect();
        Ok(WaveformView {
            label: label.to_string(),
            path: label.to_string(),
            x_axis,
            time_s,
            probes,
        })
    }
}

fn waveform_x_axis_from_header(header: &str) -> WaveformXAxis {
    let normalized = header.trim().to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "frequency" | "frequency_hz" | "freq" | "freq_hz" | "hz"
    ) {
        WaveformXAxis::FrequencyHz
    } else {
        WaveformXAxis::TimeSeconds
    }
}

fn waveform_probe_label_from_header(header: &str) -> String {
    let label = header.trim();
    if let Some(stem) = label.strip_suffix("_mag_db") {
        format!("{stem} magnitude dB")
    } else if let Some(stem) = label.strip_suffix("_phase_deg") {
        format!("{stem} phase deg")
    } else if let Some(stem) = label.strip_suffix("_mag") {
        format!("{stem} linear magnitude")
    } else if label == "onoise_v_per_sqrt_hz" {
        "output noise density".to_string()
    } else if label == "inoise_v_per_sqrt_hz" {
        "input noise density".to_string()
    } else {
        label.to_string()
    }
}

pub(super) fn waveform_probe_quantity_from_label(label: &str) -> Option<WaveformProbeQuantity> {
    let normalized = label.trim().to_ascii_lowercase().replace(' ', "");
    if normalized.starts_with("v(") {
        Some(WaveformProbeQuantity::Voltage)
    } else if normalized.starts_with("i(")
        || normalized.starts_with("-i(")
        || normalized.starts_with("abs(i(")
    {
        Some(WaveformProbeQuantity::Current)
    } else if normalized.contains("v(") && normalized.contains("i(") && normalized.contains('*') {
        Some(WaveformProbeQuantity::Power)
    } else {
        None
    }
}

fn split_waveform_fields(line: &str) -> Vec<&str> {
    line.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

fn parse_waveform_float(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}
