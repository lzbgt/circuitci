use super::{CircuitCiApp, format_value};
use crate::reports::ValidationReport;
use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::gui) struct NoiseTotalView {
    pub(super) label: String,
    pub(super) path: String,
    pub(super) scenario: String,
    pub(super) sweep: Option<String>,
    pub(super) corner: Option<String>,
    pub(super) output_rms_v: f64,
    pub(super) input_rms_v: f64,
}

impl CircuitCiApp {
    pub(super) fn noise_total_results_panel(&mut self, ui: &mut egui::Ui) {
        if self.noise_totals.is_empty() {
            return;
        }
        egui::CollapsingHeader::new(format!("Noise Totals ({})", self.noise_totals.len()))
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "{} integrated-noise artifact(s)",
                        self.noise_totals.len()
                    ));
                    if ui.button("Copy CSV").clicked() {
                        ui.ctx()
                            .copy_text(noise_total_views_csv(&self.noise_totals));
                        self.status = "Copied noise-total table CSV.".to_string();
                    }
                    if ui.button("Copy Markdown").clicked() {
                        ui.ctx()
                            .copy_text(noise_total_views_markdown(&self.noise_totals));
                        self.status = "Copied noise-total table Markdown.".to_string();
                    }
                    if ui.button("Export Bundle").clicked() {
                        self.export_scope_report_bundle(&[]);
                    }
                });
                egui::Grid::new("noise_total_results")
                    .num_columns(6)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Scenario");
                        ui.strong("Sweep");
                        ui.strong("Corner");
                        ui.strong("Output RMS");
                        ui.strong("Input RMS");
                        ui.strong("Artifact");
                        ui.end_row();
                        for view in &self.noise_totals {
                            ui.monospace(&view.scenario);
                            ui.monospace(view.sweep.as_deref().unwrap_or("nominal"));
                            ui.monospace(view.corner.as_deref().unwrap_or("nominal"));
                            ui.monospace(format!("{} V", format_value(view.output_rms_v)));
                            ui.monospace(format!("{} V", format_value(view.input_rms_v)));
                            ui.monospace(&view.label);
                            ui.end_row();
                        }
                    });
            });
    }
}

pub(in crate::gui) fn load_report_noise_totals_with_progress_and_cancel<F, C>(
    report: &ValidationReport,
    mut on_progress: F,
    should_cancel: C,
) -> Result<Vec<NoiseTotalView>>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let paths = noise_total_artifact_paths(report);
    let mut views = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        if should_cancel() {
            return Err(crate::cancellation::canceled(
                "Noise total loading canceled before completion.",
            ));
        }
        let started_at = Instant::now();
        on_progress(
            "Loading noise totals",
            format!(
                "Loading noise total {} of {}: {}.",
                index + 1,
                paths.len(),
                path
            ),
        );
        let view = load_noise_total_csv(Path::new(path), path)?;
        on_progress(
            "Loaded noise total",
            format!(
                "{}: loaded in {} ms.",
                path,
                started_at.elapsed().as_millis()
            ),
        );
        views.push(view);
    }
    Ok(views)
}

fn noise_total_artifact_paths(report: &ValidationReport) -> Vec<String> {
    let mut paths = report
        .artifacts
        .iter()
        .filter(|artifact| artifact.ends_with("/noise_total.csv"))
        .cloned()
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn load_noise_total_csv(path: &Path, label: &str) -> Result<NoiseTotalView> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read noise-total CSV {}.", path.display()))?;
    parse_noise_total_csv_text(&text, label)
}

pub(super) fn parse_noise_total_csv_text(text: &str, label: &str) -> Result<NoiseTotalView> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines.next().context("Noise-total CSV has no header row.")?;
    let values = lines.next().context("Noise-total CSV has no value row.")?;
    if lines.next().is_some() {
        anyhow::bail!("Noise-total CSV must contain exactly one value row.");
    }
    let header = split_noise_total_fields(header);
    let values = split_noise_total_fields(values);
    if header != ["onoise_total_v", "inoise_total_v"] {
        anyhow::bail!("Noise-total CSV header must be onoise_total_v,inoise_total_v.");
    }
    if values.len() != 2 {
        anyhow::bail!("Noise-total CSV must contain two value columns.");
    }
    let output_rms_v = parse_noise_total_value(values[0])?;
    let input_rms_v = parse_noise_total_value(values[1])?;
    let metadata = noise_total_path_metadata(label);
    Ok(NoiseTotalView {
        label: short_artifact_label(label),
        path: label.to_string(),
        scenario: metadata.scenario,
        sweep: metadata.sweep,
        corner: metadata.corner,
        output_rms_v,
        input_rms_v,
    })
}

fn parse_noise_total_value(value: &str) -> Result<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .with_context(|| format!("Noise-total value {value} is not finite."))
}

fn split_noise_total_fields(line: &str) -> Vec<&str> {
    line.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NoiseTotalPathMetadata {
    scenario: String,
    sweep: Option<String>,
    corner: Option<String>,
}

fn noise_total_path_metadata(path: &str) -> NoiseTotalPathMetadata {
    let parts = path.split('/').collect::<Vec<_>>();
    let Some(file_index) = parts.iter().rposition(|part| *part == "noise_total.csv") else {
        return NoiseTotalPathMetadata {
            scenario: "unknown".to_string(),
            sweep: None,
            corner: None,
        };
    };
    let run = file_index
        .checked_sub(1)
        .and_then(|index| parts.get(index))
        .copied()
        .unwrap_or("nominal");
    let parent = file_index
        .checked_sub(2)
        .and_then(|index| parts.get(index))
        .copied()
        .unwrap_or("unknown");
    let (scenario, sweep, corner) = if parent == "analog" {
        (run.to_string(), None, None)
    } else {
        let (sweep, corner) = run
            .rsplit_once("_corner_")
            .map(|(sweep, corner)| (Some(sweep.to_string()), Some(format!("corner_{corner}"))))
            .unwrap_or((None, Some(run.to_string())));
        (parent.to_string(), sweep, corner)
    };
    NoiseTotalPathMetadata {
        scenario,
        sweep,
        corner,
    }
}

fn short_artifact_label(path: &str) -> String {
    let mut parts = path.rsplit('/');
    let Some(file) = parts.next() else {
        return path.to_string();
    };
    let Some(parent) = parts.next() else {
        return file.to_string();
    };
    format!("{parent}/{file}")
}

pub(super) fn noise_total_views_csv(views: &[NoiseTotalView]) -> String {
    let mut csv = String::from("scenario,sweep,corner,output_rms_v,input_rms_v,artifact\n");
    for view in views {
        csv.push_str(&csv_row([
            view.scenario.clone(),
            view.sweep.as_deref().unwrap_or("nominal").to_string(),
            view.corner.as_deref().unwrap_or("nominal").to_string(),
            format!("{:.12e}", view.output_rms_v),
            format!("{:.12e}", view.input_rms_v),
            view.path.clone(),
        ]));
    }
    csv
}

pub(super) fn noise_total_views_markdown(views: &[NoiseTotalView]) -> String {
    let mut markdown = String::from(
        "| Scenario | Sweep | Corner | Output RMS | Input RMS | Artifact |\n| --- | --- | --- | ---: | ---: | --- |\n",
    );
    for view in views {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} V | {} V | {} |\n",
            markdown_escape(&view.scenario),
            markdown_escape(view.sweep.as_deref().unwrap_or("nominal")),
            markdown_escape(view.corner.as_deref().unwrap_or("nominal")),
            format_value(view.output_rms_v),
            format_value(view.input_rms_v),
            markdown_escape(&view.label),
        ));
    }
    markdown
}

pub(super) fn noise_total_views_html(views: &[NoiseTotalView]) -> String {
    let mut html = String::from(
        "\
<table>
  <thead>
    <tr><th>Scenario</th><th>Sweep</th><th>Corner</th><th>Output RMS</th><th>Input RMS</th><th>Artifact</th></tr>
  </thead>
  <tbody>
",
    );
    for view in views {
        html.push_str(&format!(
            "    <tr><td>{}</td><td>{}</td><td>{}</td><td class=\"number\">{} V</td><td class=\"number\">{} V</td><td>{}</td></tr>\n",
            html_escape(&view.scenario),
            html_escape(view.sweep.as_deref().unwrap_or("nominal")),
            html_escape(view.corner.as_deref().unwrap_or("nominal")),
            html_escape(&format_value(view.output_rms_v)),
            html_escape(&format_value(view.input_rms_v)),
            html_escape(&view.label),
        ));
    }
    html.push_str("  </tbody>\n</table>");
    html
}

fn html_escape(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return "-".to_string();
    }
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn markdown_escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn csv_row<const N: usize>(fields: [String; N]) -> String {
    let mut row = fields.map(csv_escape).join(",");
    row.push('\n');
    row
}

fn csv_escape(value: String) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::{noise_total_views_csv, parse_noise_total_csv_text};
    use crate::reports::ValidationReport;

    #[test]
    fn noise_total_parser_reads_single_row_values_and_corner_metadata() {
        let view = parse_noise_total_csv_text(
            "onoise_total_v,inoise_total_v\n2.0e-7,4.0e-7\n",
            "out/analog/divider_output_noise/noise_corner_003/noise_total.csv",
        )
        .unwrap();
        assert_eq!(view.scenario, "divider_output_noise");
        assert_eq!(view.sweep.as_deref(), Some("noise"));
        assert_eq!(view.corner.as_deref(), Some("corner_003"));
        assert_eq!(view.output_rms_v, 2.0e-7);
        assert_eq!(view.input_rms_v, 4.0e-7);
    }

    #[test]
    fn noise_total_csv_export_flattens_rows() {
        let view = parse_noise_total_csv_text(
            "onoise_total_v inoise_total_v\n2.0e-7 4.0e-7\n",
            "out/analog/divider_output_noise/noise_total.csv",
        )
        .unwrap();
        let csv = noise_total_views_csv(&[view]);
        assert!(csv.starts_with("scenario,sweep,corner,output_rms_v,input_rms_v,artifact\n"));
        assert!(csv.contains("divider_output_noise,nominal,nominal"));
        assert!(csv.contains("2.000000000000e-7,4.000000000000e-7"));
    }

    #[test]
    fn report_noise_total_loader_reads_artifacts_in_stable_order() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("analog").join("divider_output_noise");
        let first = base.join("noise_corner_001").join("noise_total.csv");
        let second = base.join("noise_corner_002").join("noise_total.csv");
        std::fs::create_dir_all(first.parent().unwrap()).unwrap();
        std::fs::create_dir_all(second.parent().unwrap()).unwrap();
        std::fs::write(&first, "onoise_total_v,inoise_total_v\n1.0e-7,2.0e-7\n").unwrap();
        std::fs::write(&second, "onoise_total_v,inoise_total_v\n3.0e-7,4.0e-7\n").unwrap();
        let report = ValidationReport::from_parts(
            "project".to_string(),
            "default".to_string(),
            Vec::new(),
            Vec::new(),
            vec![
                second.to_string_lossy().into_owned(),
                first.to_string_lossy().into_owned(),
            ],
            Vec::new(),
            "validate".to_string(),
        );

        let views =
            super::load_report_noise_totals_with_progress_and_cancel(&report, |_, _| {}, || false)
                .unwrap();

        assert_eq!(views.len(), 2);
        assert_eq!(views[0].corner.as_deref(), Some("corner_001"));
        assert_eq!(views[1].input_rms_v, 4.0e-7);
    }
}
