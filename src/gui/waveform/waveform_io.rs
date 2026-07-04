use super::waveform_load::waveform_load_preflight;
use super::{
    WaveformLoadDiagnostic, WaveformProbe, WaveformProbeQuantity, WaveformView, WaveformXAxis,
};
use crate::reports::ValidationReport;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
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
        if (is_hb_spectrum_path(artifact)
            || is_distortion_spectrum_path(artifact)
            || is_fourier_summary_path(artifact)
            || is_sensitivity_summary_path(artifact))
            && !paths.iter().any(|path| path == artifact)
        {
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
    if selected_probe_labels.is_empty() && is_distortion_spectrum_path(label) {
        let text = std::fs::read_to_string(path).with_context(|| {
            format!("Failed to read distortion spectrum CSV {}.", path.display())
        })?;
        return parse_distortion_spectrum_csv_rows(&text, label);
    }
    if selected_probe_labels.is_empty() && is_fourier_summary_path(label) {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read Fourier summary CSV {}.", path.display()))?;
        return parse_fourier_summary_csv_rows(&text, label);
    }
    if selected_probe_labels.is_empty() && is_sensitivity_summary_path(label) {
        let text = std::fs::read_to_string(path).with_context(|| {
            format!("Failed to read sensitivity summary CSV {}.", path.display())
        })?;
        return parse_sensitivity_summary_csv_rows(&text, label);
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

#[cfg(test)]
pub(super) fn parse_distortion_spectrum_csv_text(text: &str, label: &str) -> Result<WaveformView> {
    parse_distortion_spectrum_csv_rows(text, label)
}

#[cfg(test)]
pub(super) fn parse_fourier_summary_csv_text(text: &str, label: &str) -> Result<WaveformView> {
    parse_fourier_summary_csv_rows(text, label)
}

#[cfg(test)]
pub(super) fn parse_sensitivity_summary_csv_text(text: &str, label: &str) -> Result<WaveformView> {
    parse_sensitivity_summary_csv_rows(text, label)
}

fn is_hb_spectrum_path(path: &str) -> bool {
    path.ends_with("/hb_spectrum.csv") || path == "hb_spectrum.csv"
}

fn is_distortion_spectrum_path(path: &str) -> bool {
    path.ends_with("/distortion_spectrum.csv") || path == "distortion_spectrum.csv"
}

fn is_fourier_summary_path(path: &str) -> bool {
    path.ends_with("/fourier_summary.csv") || path == "fourier_summary.csv"
}

fn is_sensitivity_summary_path(path: &str) -> bool {
    path.ends_with("/sensitivity_summary.csv") || path == "sensitivity_summary.csv"
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

fn parse_distortion_spectrum_csv_rows(text: &str, label: &str) -> Result<WaveformView> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .context("Distortion spectrum CSV has no header row.")?;
    let header = split_distortion_csv_fields(header)?;
    if header
        != [
            "component",
            "frequency_hz",
            "output_expression",
            "real",
            "imaginary",
            "magnitude",
            "phase_degrees",
        ]
    {
        anyhow::bail!(
            "Distortion spectrum CSV header must be component,frequency_hz,output_expression,real,imaginary,magnitude,phase_degrees."
        );
    }

    let mut by_component: BTreeMap<String, Vec<DistortionSpectrumPoint>> = BTreeMap::new();
    for (line_index, line) in lines.enumerate() {
        let fields = split_distortion_csv_fields(line)?;
        if fields.len() != 7 {
            anyhow::bail!(
                "Distortion spectrum row {} has {} fields, expected 7.",
                line_index + 2,
                fields.len()
            );
        }
        let frequency_hz = parse_waveform_float(&fields[1]).with_context(|| {
            format!(
                "Distortion spectrum row {} has invalid frequency.",
                line_index + 2
            )
        })?;
        let real = parse_waveform_float(&fields[3]).with_context(|| {
            format!(
                "Distortion spectrum row {} has invalid real value.",
                line_index + 2
            )
        })?;
        let imaginary = parse_waveform_float(&fields[4]).with_context(|| {
            format!(
                "Distortion spectrum row {} has invalid imaginary value.",
                line_index + 2
            )
        })?;
        let magnitude = parse_waveform_float(&fields[5]).with_context(|| {
            format!(
                "Distortion spectrum row {} has invalid magnitude value.",
                line_index + 2
            )
        })?;
        let phase_deg = parse_waveform_float(&fields[6]).with_context(|| {
            format!(
                "Distortion spectrum row {} has invalid phase value.",
                line_index + 2
            )
        })?;
        by_component
            .entry(fields[0].clone())
            .or_default()
            .push(DistortionSpectrumPoint {
                frequency_hz,
                output_expression: fields[2].clone(),
                real,
                imaginary,
                magnitude,
                phase_deg,
            });
    }

    if by_component.is_empty() {
        anyhow::bail!("Distortion spectrum CSV has no numeric rows.");
    }
    let mut time_s = Vec::new();
    let mut probes = Vec::new();
    let mut reference_expression: Option<String> = None;
    for (component, mut rows) in by_component {
        rows.sort_by(|left, right| left.frequency_hz.total_cmp(&right.frequency_hz));
        let expression = rows[0].output_expression.clone();
        if rows.iter().any(|row| row.output_expression != expression) {
            anyhow::bail!(
                "Distortion spectrum component {component} must contain one output expression."
            );
        }
        match &reference_expression {
            Some(reference) if reference != &expression => {
                anyhow::bail!("Distortion spectrum CSV must contain one output expression.")
            }
            Some(_) => {}
            None => reference_expression = Some(expression.clone()),
        }
        let component_frequencies: Vec<_> = rows.iter().map(|row| row.frequency_hz).collect();
        if time_s.is_empty() {
            let mut previous = None;
            for frequency_hz in &component_frequencies {
                if previous.is_some_and(|value| frequency_hz <= &value) {
                    anyhow::bail!(
                        "Distortion spectrum component {component} has duplicate or non-increasing frequency {frequency_hz}."
                    );
                }
                previous = Some(*frequency_hz);
            }
            time_s = component_frequencies
                .iter()
                .map(|frequency_hz| {
                    WaveformXAxis::FrequencyHz.storage_from_csv_value(*frequency_hz)
                })
                .collect();
        } else {
            let comparable: Vec<_> = component_frequencies
                .iter()
                .map(|frequency_hz| {
                    WaveformXAxis::FrequencyHz.storage_from_csv_value(*frequency_hz)
                })
                .collect();
            if comparable != time_s {
                anyhow::bail!("Distortion spectrum components must share the same frequency grid.");
            }
        }
        let mut real_values = Vec::with_capacity(rows.len());
        let mut imaginary_values = Vec::with_capacity(rows.len());
        let mut magnitude_values = Vec::with_capacity(rows.len());
        let mut phase_values = Vec::with_capacity(rows.len());
        for row in rows {
            real_values.push(row.real);
            imaginary_values.push(row.imaginary);
            magnitude_values.push(row.magnitude);
            phase_values.push(row.phase_deg);
        }
        probes.push(WaveformProbe {
            label: format!("{expression} {component} magnitude"),
            values: magnitude_values,
            derived: false,
            expression: Some(expression.clone()),
            promoted_quantity: waveform_probe_quantity_from_label(&expression),
        });
        probes.push(WaveformProbe {
            label: format!("{expression} {component} phase deg"),
            values: phase_values,
            derived: false,
            expression: Some(expression.clone()),
            promoted_quantity: None,
        });
        probes.push(WaveformProbe {
            label: format!("{expression} {component} real"),
            values: real_values,
            derived: false,
            expression: Some(expression.clone()),
            promoted_quantity: waveform_probe_quantity_from_label(&expression),
        });
        probes.push(WaveformProbe {
            label: format!("{expression} {component} imaginary"),
            values: imaginary_values,
            derived: false,
            expression: Some(expression),
            promoted_quantity: None,
        });
    }

    Ok(WaveformView {
        label: label.to_string(),
        path: label.to_string(),
        x_axis: WaveformXAxis::FrequencyHz,
        time_s,
        probes,
    })
}

struct DistortionSpectrumPoint {
    frequency_hz: f64,
    output_expression: String,
    real: f64,
    imaginary: f64,
    magnitude: f64,
    phase_deg: f64,
}

fn parse_fourier_summary_csv_rows(text: &str, label: &str) -> Result<WaveformView> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .context("Fourier summary CSV has no header row.")?;
    let header = split_distortion_csv_fields(header)?;
    if header
        != [
            "output_expression",
            "fundamental_frequency_hz",
            "reported_harmonics",
            "harmonic",
            "frequency_hz",
            "magnitude",
            "phase_deg",
            "normalized_magnitude",
            "normalized_phase_deg",
            "thd_percent",
            "grid_size",
            "interpolation_degree",
            "periods",
        ]
    {
        anyhow::bail!(
            "Fourier summary CSV header must be output_expression,fundamental_frequency_hz,reported_harmonics,harmonic,frequency_hz,magnitude,phase_deg,normalized_magnitude,normalized_phase_deg,thd_percent,grid_size,interpolation_degree,periods."
        );
    }

    let mut rows = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let fields = split_distortion_csv_fields(line)?;
        if fields.len() != 13 {
            anyhow::bail!(
                "Fourier summary row {} has {} fields, expected 13.",
                line_index + 2,
                fields.len()
            );
        }
        let frequency_hz = parse_waveform_float(&fields[4]).with_context(|| {
            format!(
                "Fourier summary row {} has invalid frequency.",
                line_index + 2
            )
        })?;
        if frequency_hz < 0.0 {
            anyhow::bail!(
                "Fourier summary row {} has negative frequency {}.",
                line_index + 2,
                frequency_hz
            );
        }
        let magnitude = parse_waveform_float(&fields[5]).with_context(|| {
            format!(
                "Fourier summary row {} has invalid magnitude value.",
                line_index + 2
            )
        })?;
        let phase_deg = parse_waveform_float(&fields[6]).with_context(|| {
            format!(
                "Fourier summary row {} has invalid phase value.",
                line_index + 2
            )
        })?;
        let normalized_magnitude = parse_waveform_float(&fields[7]).with_context(|| {
            format!(
                "Fourier summary row {} has invalid normalized magnitude value.",
                line_index + 2
            )
        })?;
        let normalized_phase_deg = parse_waveform_float(&fields[8]).with_context(|| {
            format!(
                "Fourier summary row {} has invalid normalized phase value.",
                line_index + 2
            )
        })?;
        rows.push(FourierSummaryPoint {
            output_expression: fields[0].clone(),
            frequency_hz,
            magnitude,
            phase_deg,
            normalized_magnitude,
            normalized_phase_deg,
        });
    }

    if rows.is_empty() {
        anyhow::bail!("Fourier summary CSV has no harmonic rows.");
    }
    rows.sort_by(|left, right| left.frequency_hz.total_cmp(&right.frequency_hz));
    let expression = rows[0].output_expression.clone();
    if rows.iter().any(|row| row.output_expression != expression) {
        anyhow::bail!("Fourier summary CSV must contain one output expression per artifact.");
    }

    let mut time_s = Vec::with_capacity(rows.len());
    let mut magnitude_values = Vec::with_capacity(rows.len());
    let mut phase_values = Vec::with_capacity(rows.len());
    let mut normalized_magnitude_values = Vec::with_capacity(rows.len());
    let mut normalized_phase_values = Vec::with_capacity(rows.len());
    let mut previous_frequency = None;
    for row in rows {
        if previous_frequency.is_some_and(|previous| row.frequency_hz <= previous) {
            anyhow::bail!(
                "Fourier summary CSV has duplicate or non-increasing frequency {}.",
                row.frequency_hz
            );
        }
        previous_frequency = Some(row.frequency_hz);
        time_s.push(WaveformXAxis::FrequencyHz.storage_from_csv_value(row.frequency_hz));
        magnitude_values.push(row.magnitude);
        phase_values.push(row.phase_deg);
        normalized_magnitude_values.push(row.normalized_magnitude);
        normalized_phase_values.push(row.normalized_phase_deg);
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
                label: format!("{expression} normalized magnitude"),
                values: normalized_magnitude_values,
                derived: false,
                expression: Some(expression.clone()),
                promoted_quantity: None,
            },
            WaveformProbe {
                label: format!("{expression} normalized phase deg"),
                values: normalized_phase_values,
                derived: false,
                expression: Some(expression),
                promoted_quantity: None,
            },
        ],
    })
}

struct FourierSummaryPoint {
    output_expression: String,
    frequency_hz: f64,
    magnitude: f64,
    phase_deg: f64,
    normalized_magnitude: f64,
    normalized_phase_deg: f64,
}

fn parse_sensitivity_summary_csv_rows(text: &str, label: &str) -> Result<WaveformView> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .context("Sensitivity summary CSV has no header row.")?;
    let header = split_distortion_csv_fields(header)?;
    if header
        != [
            "output_expression",
            "mode",
            "parameter",
            "frequency_hz",
            "sensitivity_real",
            "sensitivity_imaginary",
            "sensitivity_magnitude",
        ]
    {
        anyhow::bail!(
            "Sensitivity summary CSV header must be output_expression,mode,parameter,frequency_hz,sensitivity_real,sensitivity_imaginary,sensitivity_magnitude."
        );
    }

    let mut by_parameter: BTreeMap<String, Vec<SensitivitySummaryPoint>> = BTreeMap::new();
    for (line_index, line) in lines.enumerate() {
        let fields = split_distortion_csv_fields(line)?;
        if fields.len() != 7 {
            anyhow::bail!(
                "Sensitivity summary row {} has {} fields, expected 7.",
                line_index + 2,
                fields.len()
            );
        }
        if fields[1] != "ac" {
            continue;
        }
        if fields[3].is_empty() {
            continue;
        }
        let frequency_hz = parse_waveform_float(&fields[3]).with_context(|| {
            format!(
                "Sensitivity summary row {} has invalid frequency.",
                line_index + 2
            )
        })?;
        if frequency_hz <= 0.0 {
            anyhow::bail!(
                "Sensitivity summary row {} has non-positive AC frequency {}.",
                line_index + 2,
                frequency_hz
            );
        }
        let sensitivity_real = parse_waveform_float(&fields[4]).with_context(|| {
            format!(
                "Sensitivity summary row {} has invalid real sensitivity.",
                line_index + 2
            )
        })?;
        let sensitivity_imaginary = parse_waveform_float(&fields[5]).with_context(|| {
            format!(
                "Sensitivity summary row {} has invalid imaginary sensitivity.",
                line_index + 2
            )
        })?;
        let sensitivity_magnitude = parse_waveform_float(&fields[6]).with_context(|| {
            format!(
                "Sensitivity summary row {} has invalid magnitude sensitivity.",
                line_index + 2
            )
        })?;
        by_parameter
            .entry(fields[2].clone())
            .or_default()
            .push(SensitivitySummaryPoint {
                output_expression: fields[0].clone(),
                frequency_hz,
                sensitivity_real,
                sensitivity_imaginary,
                sensitivity_magnitude,
            });
    }

    if by_parameter.is_empty() {
        anyhow::bail!("Sensitivity summary CSV has no AC frequency rows.");
    }
    let mut time_s = Vec::new();
    let mut probes = Vec::new();
    let mut reference_expression: Option<String> = None;
    for (parameter, mut rows) in by_parameter {
        rows.sort_by(|left, right| left.frequency_hz.total_cmp(&right.frequency_hz));
        let expression = rows[0].output_expression.clone();
        if rows.iter().any(|row| row.output_expression != expression) {
            anyhow::bail!(
                "Sensitivity summary parameter {parameter} must contain one output expression."
            );
        }
        match &reference_expression {
            Some(reference) if reference != &expression => {
                anyhow::bail!("Sensitivity summary CSV must contain one output expression.")
            }
            Some(_) => {}
            None => reference_expression = Some(expression.clone()),
        }
        let parameter_frequencies: Vec<_> = rows.iter().map(|row| row.frequency_hz).collect();
        if time_s.is_empty() {
            let mut previous = None;
            for frequency_hz in &parameter_frequencies {
                if previous.is_some_and(|value| frequency_hz <= &value) {
                    anyhow::bail!(
                        "Sensitivity summary parameter {parameter} has duplicate or non-increasing frequency {frequency_hz}."
                    );
                }
                previous = Some(*frequency_hz);
            }
            time_s = parameter_frequencies
                .iter()
                .map(|frequency_hz| {
                    WaveformXAxis::FrequencyHz.storage_from_csv_value(*frequency_hz)
                })
                .collect();
        } else {
            let comparable: Vec<_> = parameter_frequencies
                .iter()
                .map(|frequency_hz| {
                    WaveformXAxis::FrequencyHz.storage_from_csv_value(*frequency_hz)
                })
                .collect();
            if comparable != time_s {
                anyhow::bail!("Sensitivity summary parameters must share the same frequency grid.");
            }
        }
        let mut real_values = Vec::with_capacity(rows.len());
        let mut imaginary_values = Vec::with_capacity(rows.len());
        let mut magnitude_values = Vec::with_capacity(rows.len());
        for row in rows {
            real_values.push(row.sensitivity_real);
            imaginary_values.push(row.sensitivity_imaginary);
            magnitude_values.push(row.sensitivity_magnitude);
        }
        probes.push(WaveformProbe {
            label: format!("{expression} {parameter} sensitivity magnitude"),
            values: magnitude_values,
            derived: false,
            expression: Some(expression.clone()),
            promoted_quantity: waveform_probe_quantity_from_label(&expression),
        });
        probes.push(WaveformProbe {
            label: format!("{expression} {parameter} sensitivity real"),
            values: real_values,
            derived: false,
            expression: Some(expression.clone()),
            promoted_quantity: waveform_probe_quantity_from_label(&expression),
        });
        probes.push(WaveformProbe {
            label: format!("{expression} {parameter} sensitivity imaginary"),
            values: imaginary_values,
            derived: false,
            expression: Some(expression),
            promoted_quantity: None,
        });
    }

    Ok(WaveformView {
        label: label.to_string(),
        path: label.to_string(),
        x_axis: WaveformXAxis::FrequencyHz,
        time_s,
        probes,
    })
}

struct SensitivitySummaryPoint {
    output_expression: String,
    frequency_hz: f64,
    sensitivity_real: f64,
    sensitivity_imaginary: f64,
    sensitivity_magnitude: f64,
}

fn split_distortion_csv_fields(line: &str) -> Result<Vec<String>> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(field.trim().to_string());
                field.clear();
            }
            _ => field.push(ch),
        }
    }
    if in_quotes {
        anyhow::bail!("Distortion spectrum CSV row has an unterminated quoted field.");
    }
    fields.push(field.trim().to_string());
    Ok(fields)
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
        if line.trim_matches('\0').trim().is_empty() {
            return Ok(());
        }
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
            selected_probe_labels,
            selected_probe_columns: _,
        } = self;
        if time_s.is_empty() {
            anyhow::bail!("Waveform CSV has no numeric samples.");
        }

        let mut probes: Vec<_> = probe_labels
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
        if selected_probe_labels.is_empty() && x_axis == WaveformXAxis::FrequencyHz {
            append_derived_group_delay_probes(&mut probes, &time_s);
            append_derived_s_parameter_probes(&mut probes);
            probes.retain(|probe| !is_waveform_metadata_label(&probe.label));
        }
        Ok(WaveformView {
            label: label.to_string(),
            path: label.to_string(),
            x_axis,
            time_s,
            probes,
        })
    }
}

fn is_waveform_metadata_label(label: &str) -> bool {
    matches!(label.trim(), "reference_impedance_ohm")
}

fn append_derived_group_delay_probes(probes: &mut Vec<WaveformProbe>, frequency_storage: &[f64]) {
    let phase_probes: Vec<_> = probes
        .iter()
        .enumerate()
        .filter_map(|(index, probe)| {
            probe
                .label
                .strip_suffix(" phase deg")
                .map(|stem| (index, stem.to_string()))
        })
        .collect();
    for (phase_index, stem) in phase_probes {
        let Some(values) = group_delay_values_s(frequency_storage, &probes[phase_index].values)
        else {
            continue;
        };
        probes.push(WaveformProbe {
            label: format!("{stem} group delay s"),
            values,
            derived: true,
            expression: Some(stem),
            promoted_quantity: None,
        });
    }
}

fn group_delay_values_s(frequency_storage: &[f64], phase_deg: &[f64]) -> Option<Vec<f64>> {
    if frequency_storage.len() != phase_deg.len() || frequency_storage.len() < 2 {
        return None;
    }
    let frequency_hz: Vec<_> = frequency_storage
        .iter()
        .map(|value| value * 1.0e6)
        .collect();
    let mut phase_rad = Vec::with_capacity(phase_deg.len());
    let mut previous = None;
    let mut offset = 0.0;
    for (index, value) in phase_deg.iter().enumerate() {
        if !frequency_hz[index].is_finite()
            || frequency_hz[index] <= 0.0
            || index > 0 && frequency_hz[index] <= frequency_hz[index - 1]
            || !value.is_finite()
        {
            return None;
        }
        let raw = value.to_radians();
        if let Some(previous_unwrapped) = previous {
            let mut delta = raw + offset - previous_unwrapped;
            while delta > std::f64::consts::PI {
                offset -= std::f64::consts::TAU;
                delta -= std::f64::consts::TAU;
            }
            while delta < -std::f64::consts::PI {
                offset += std::f64::consts::TAU;
                delta += std::f64::consts::TAU;
            }
        }
        let unwrapped = raw + offset;
        previous = Some(unwrapped);
        phase_rad.push(unwrapped);
    }
    let mut delay = Vec::with_capacity(frequency_hz.len());
    for index in 0..frequency_hz.len() {
        let (left, right) = if index == 0 {
            (0, 1)
        } else if index + 1 == frequency_hz.len() {
            (index - 1, index)
        } else {
            (index - 1, index + 1)
        };
        let omega_span = std::f64::consts::TAU * (frequency_hz[right] - frequency_hz[left]);
        if !omega_span.is_finite() || omega_span <= 0.0 {
            return None;
        }
        delay.push(-(phase_rad[right] - phase_rad[left]) / omega_span);
    }
    Some(delay)
}

fn append_derived_s_parameter_probes(probes: &mut Vec<WaveformProbe>) {
    let mut magnitude_db_by_parameter = BTreeMap::new();
    let mut magnitude_linear_by_parameter = BTreeMap::new();
    let mut phase_deg_by_parameter = BTreeMap::new();
    let reference_impedance_index = probes
        .iter()
        .position(|probe| probe.label == "reference_impedance_ohm");
    for (index, probe) in probes.iter().enumerate() {
        if let Some(parameter) = probe.label.strip_suffix(" magnitude dB")
            && parse_s_parameter_term(parameter).is_some()
        {
            magnitude_db_by_parameter.insert(parameter.to_ascii_lowercase(), index);
        }
        if let Some(parameter) = probe.label.strip_suffix(" linear magnitude")
            && parse_s_parameter_term(parameter).is_some()
        {
            magnitude_linear_by_parameter.insert(parameter.to_ascii_lowercase(), index);
        }
        if let Some(parameter) = probe.label.strip_suffix(" phase deg")
            && parse_s_parameter_term(parameter).is_some()
        {
            phase_deg_by_parameter.insert(parameter.to_ascii_lowercase(), index);
        }
    }
    for (parameter, db_index) in magnitude_db_by_parameter {
        let Some((output_port, input_port)) = parse_s_parameter_term(&parameter) else {
            continue;
        };
        let db_values = probes[db_index].values.clone();
        if output_port == input_port {
            let return_loss_values = db_values.iter().map(|value| -value).collect();
            probes.push(WaveformProbe {
                label: format!("{parameter} return loss dB"),
                values: return_loss_values,
                derived: true,
                expression: Some(parameter.clone()),
                promoted_quantity: None,
            });
            if let Some(linear_index) = magnitude_linear_by_parameter.get(&parameter) {
                let linear_values = &probes[*linear_index].values;
                if linear_values
                    .iter()
                    .all(|value| value.is_finite() && *value >= 0.0 && *value < 1.0)
                {
                    let mut vswr_values = Vec::with_capacity(linear_values.len());
                    let mut mismatch_loss_values = Vec::with_capacity(linear_values.len());
                    for value in linear_values {
                        vswr_values.push((1.0 + value) / (1.0 - value));
                        mismatch_loss_values.push(-10.0 * (1.0 - value * value).log10());
                    }
                    probes.push(WaveformProbe {
                        label: format!("{parameter} VSWR"),
                        values: vswr_values,
                        derived: true,
                        expression: Some(parameter.clone()),
                        promoted_quantity: None,
                    });
                    probes.push(WaveformProbe {
                        label: format!("{parameter} mismatch loss dB"),
                        values: mismatch_loss_values,
                        derived: true,
                        expression: Some(parameter.clone()),
                        promoted_quantity: None,
                    });
                }
                if let Some(phase_index) = phase_deg_by_parameter.get(&parameter)
                    && let Some(impedance_values) = reflection_impedance_values_ohm(
                        probes,
                        *linear_index,
                        *phase_index,
                        reference_impedance_index,
                    )
                {
                    let real_values: Vec<_> =
                        impedance_values.iter().map(|value| value.real).collect();
                    let imaginary_values: Vec<_> = impedance_values
                        .iter()
                        .map(|value| value.imaginary)
                        .collect();
                    let magnitude_values: Vec<_> = impedance_values
                        .iter()
                        .map(|value| value.magnitude())
                        .collect();
                    probes.push(WaveformProbe {
                        label: format!("{parameter} impedance real ohm"),
                        values: real_values,
                        derived: true,
                        expression: Some(parameter.clone()),
                        promoted_quantity: None,
                    });
                    probes.push(WaveformProbe {
                        label: format!("{parameter} impedance imaginary ohm"),
                        values: imaginary_values,
                        derived: true,
                        expression: Some(parameter.clone()),
                        promoted_quantity: None,
                    });
                    probes.push(WaveformProbe {
                        label: format!("{parameter} impedance magnitude ohm"),
                        values: magnitude_values,
                        derived: true,
                        expression: Some(parameter.clone()),
                        promoted_quantity: None,
                    });
                }
            }
        } else {
            let insertion_loss_values = db_values.iter().map(|value| -value).collect();
            probes.push(WaveformProbe {
                label: format!("{parameter} insertion loss dB"),
                values: insertion_loss_values,
                derived: true,
                expression: Some(parameter),
                promoted_quantity: None,
            });
        }
    }
    append_derived_s_parameter_network_probes(
        probes,
        &magnitude_linear_by_parameter,
        &phase_deg_by_parameter,
    );
}

fn reflection_impedance_values_ohm(
    probes: &[WaveformProbe],
    magnitude_linear_index: usize,
    phase_deg_index: usize,
    reference_impedance_index: Option<usize>,
) -> Option<Vec<SParameterComplexValue>> {
    let magnitude_values = &probes.get(magnitude_linear_index)?.values;
    let phase_values = &probes.get(phase_deg_index)?.values;
    if magnitude_values.len() != phase_values.len() {
        return None;
    }
    let reference_values = reference_impedance_index
        .and_then(|index| probes.get(index))
        .map(|probe| probe.values.as_slice());
    if let Some(values) = reference_values
        && values.len() != magnitude_values.len()
    {
        return None;
    }
    let mut values = Vec::with_capacity(magnitude_values.len());
    for (index, (magnitude, phase_deg)) in magnitude_values.iter().zip(phase_values).enumerate() {
        if !magnitude.is_finite() || *magnitude < 0.0 || !phase_deg.is_finite() {
            return None;
        }
        let reference_impedance_ohm = reference_values.map_or(50.0, |values| values[index]);
        if !reference_impedance_ohm.is_finite() || reference_impedance_ohm <= 0.0 {
            return None;
        }
        let gamma = SParameterComplexValue::from_polar_degrees(*magnitude, *phase_deg);
        let numerator = SParameterComplexValue {
            real: 1.0 + gamma.real,
            imaginary: gamma.imaginary,
        };
        let denominator = SParameterComplexValue {
            real: 1.0 - gamma.real,
            imaginary: -gamma.imaginary,
        };
        let impedance = numerator
            .divide(denominator)?
            .scale(reference_impedance_ohm);
        if !impedance.real.is_finite() || !impedance.imaginary.is_finite() {
            return None;
        }
        values.push(impedance);
    }
    Some(values)
}

fn parse_s_parameter_term(parameter: &str) -> Option<(usize, usize)> {
    let digits = parameter
        .trim()
        .to_ascii_lowercase()
        .strip_prefix('s')?
        .to_string();
    if digits.len() < 2 || !digits.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let midpoint = digits.len() / 2;
    let output = digits[..midpoint].parse::<usize>().ok()?;
    let input = digits[midpoint..].parse::<usize>().ok()?;
    Some((output, input))
}

fn append_derived_s_parameter_network_probes(
    probes: &mut Vec<WaveformProbe>,
    magnitude_linear_by_parameter: &BTreeMap<String, usize>,
    phase_deg_by_parameter: &BTreeMap<String, usize>,
) {
    let Some(s11) = s_parameter_complex_values(
        probes,
        magnitude_linear_by_parameter,
        phase_deg_by_parameter,
        "s11",
    ) else {
        return;
    };
    let Some(s12) = s_parameter_complex_values(
        probes,
        magnitude_linear_by_parameter,
        phase_deg_by_parameter,
        "s12",
    ) else {
        return;
    };
    let Some(s21) = s_parameter_complex_values(
        probes,
        magnitude_linear_by_parameter,
        phase_deg_by_parameter,
        "s21",
    ) else {
        return;
    };
    let Some(s22) = s_parameter_complex_values(
        probes,
        magnitude_linear_by_parameter,
        phase_deg_by_parameter,
        "s22",
    ) else {
        return;
    };
    let sample_count = s11.len();
    if sample_count == 0
        || s12.len() != sample_count
        || s21.len() != sample_count
        || s22.len() != sample_count
    {
        return;
    }
    let mut reciprocity_values = Vec::with_capacity(sample_count);
    let mut passivity_values = Vec::with_capacity(sample_count);
    let mut stability_delta_values = Vec::with_capacity(sample_count);
    let mut rollet_k_values = Vec::with_capacity(sample_count);
    let mut rollet_k_available = true;
    for index in 0..sample_count {
        reciprocity_values.push(s21[index].subtract(s12[index]).magnitude());
        passivity_values.push(two_port_max_singular_value(
            s11[index], s12[index], s21[index], s22[index],
        ));
        let delta = stability_delta(s11[index], s12[index], s21[index], s22[index]);
        let delta_magnitude = delta.magnitude();
        stability_delta_values.push(delta_magnitude);
        if rollet_k_available {
            if let Some(rollet_k) = rollet_stability_factor(
                s11[index],
                s12[index],
                s21[index],
                s22[index],
                delta_magnitude,
            ) {
                rollet_k_values.push(rollet_k);
            } else {
                rollet_k_available = false;
                rollet_k_values.clear();
            }
        }
    }
    probes.push(WaveformProbe {
        label: "two-port reciprocity error".to_string(),
        values: reciprocity_values,
        derived: true,
        expression: Some("max |S21-S12|".to_string()),
        promoted_quantity: None,
    });
    probes.push(WaveformProbe {
        label: "two-port passivity singular value".to_string(),
        values: passivity_values,
        derived: true,
        expression: Some("max singular value(S)".to_string()),
        promoted_quantity: None,
    });
    probes.push(WaveformProbe {
        label: "two-port stability delta magnitude".to_string(),
        values: stability_delta_values,
        derived: true,
        expression: Some("|S11*S22 - S12*S21|".to_string()),
        promoted_quantity: None,
    });
    if rollet_k_available && rollet_k_values.len() == sample_count {
        probes.push(WaveformProbe {
            label: "two-port Rollet K".to_string(),
            values: rollet_k_values,
            derived: true,
            expression: Some("(1 - |S11|^2 - |S22|^2 + |Delta|^2) / (2*|S12*S21|)".to_string()),
            promoted_quantity: None,
        });
    }
}

fn s_parameter_complex_values(
    probes: &[WaveformProbe],
    magnitude_linear_by_parameter: &BTreeMap<String, usize>,
    phase_deg_by_parameter: &BTreeMap<String, usize>,
    parameter: &str,
) -> Option<Vec<SParameterComplexValue>> {
    let magnitude_index = *magnitude_linear_by_parameter.get(parameter)?;
    let phase_index = *phase_deg_by_parameter.get(parameter)?;
    let magnitude_values = &probes.get(magnitude_index)?.values;
    let phase_values = &probes.get(phase_index)?.values;
    if magnitude_values.len() != phase_values.len() {
        return None;
    }
    let mut values = Vec::with_capacity(magnitude_values.len());
    for (magnitude, phase_deg) in magnitude_values.iter().zip(phase_values) {
        if !magnitude.is_finite() || *magnitude < 0.0 || !phase_deg.is_finite() {
            return None;
        }
        values.push(SParameterComplexValue::from_polar_degrees(
            *magnitude, *phase_deg,
        ));
    }
    Some(values)
}

#[derive(Debug, Clone, Copy)]
struct SParameterComplexValue {
    real: f64,
    imaginary: f64,
}

impl SParameterComplexValue {
    fn from_polar_degrees(magnitude: f64, phase_deg: f64) -> Self {
        let phase_rad = phase_deg.to_radians();
        Self {
            real: magnitude * phase_rad.cos(),
            imaginary: magnitude * phase_rad.sin(),
        }
    }

    fn subtract(self, other: Self) -> Self {
        Self {
            real: self.real - other.real,
            imaginary: self.imaginary - other.imaginary,
        }
    }

    fn add(self, other: Self) -> Self {
        Self {
            real: self.real + other.real,
            imaginary: self.imaginary + other.imaginary,
        }
    }

    fn conjugate(self) -> Self {
        Self {
            real: self.real,
            imaginary: -self.imaginary,
        }
    }

    fn multiply(self, other: Self) -> Self {
        Self {
            real: self.real * other.real - self.imaginary * other.imaginary,
            imaginary: self.real * other.imaginary + self.imaginary * other.real,
        }
    }

    fn divide(self, other: Self) -> Option<Self> {
        let denominator = other.magnitude_squared();
        if !denominator.is_finite() || denominator <= f64::EPSILON {
            return None;
        }
        Some(Self {
            real: (self.real * other.real + self.imaginary * other.imaginary) / denominator,
            imaginary: (self.imaginary * other.real - self.real * other.imaginary) / denominator,
        })
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            real: self.real * factor,
            imaginary: self.imaginary * factor,
        }
    }

    fn magnitude_squared(self) -> f64 {
        self.real
            .mul_add(self.real, self.imaginary * self.imaginary)
    }

    fn magnitude(self) -> f64 {
        self.magnitude_squared().sqrt()
    }
}

fn two_port_max_singular_value(
    s11: SParameterComplexValue,
    s12: SParameterComplexValue,
    s21: SParameterComplexValue,
    s22: SParameterComplexValue,
) -> f64 {
    let a = s11.magnitude_squared() + s21.magnitude_squared();
    let d = s12.magnitude_squared() + s22.magnitude_squared();
    let b = s11
        .conjugate()
        .multiply(s12)
        .add(s21.conjugate().multiply(s22));
    let trace = a + d;
    let discriminant = (a - d).mul_add(a - d, 4.0 * b.magnitude_squared());
    let largest_eigenvalue = 0.5 * (trace + discriminant.max(0.0).sqrt());
    largest_eigenvalue.max(0.0).sqrt()
}

fn stability_delta(
    s11: SParameterComplexValue,
    s12: SParameterComplexValue,
    s21: SParameterComplexValue,
    s22: SParameterComplexValue,
) -> SParameterComplexValue {
    s11.multiply(s22).subtract(s12.multiply(s21))
}

fn rollet_stability_factor(
    s11: SParameterComplexValue,
    s12: SParameterComplexValue,
    s21: SParameterComplexValue,
    s22: SParameterComplexValue,
    delta_magnitude: f64,
) -> Option<f64> {
    let denominator = 2.0 * s12.multiply(s21).magnitude();
    if !denominator.is_finite() || denominator <= f64::EPSILON {
        return None;
    }
    let numerator =
        1.0 - s11.magnitude_squared() - s22.magnitude_squared() + delta_magnitude.powi(2);
    let rollet_k = numerator / denominator;
    rollet_k.is_finite().then_some(rollet_k)
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
    } else if let Some(stem) = label.strip_suffix("_mag_linear") {
        format!("{stem} linear magnitude")
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
