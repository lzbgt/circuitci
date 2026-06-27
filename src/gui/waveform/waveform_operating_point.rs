use super::{CircuitCiApp, format_value};
use crate::reports::{Finding, ValidationReport};
use anyhow::{Context, Result};
use eframe::egui;
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::gui) struct OperatingPointView {
    pub(super) label: String,
    pub(super) path: String,
    pub(super) scenario: String,
    pub(super) sweep: Option<String>,
    pub(super) corner: Option<String>,
    pub(super) values: Vec<OperatingPointValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::gui) struct OperatingPointValue {
    pub(super) probe: String,
    pub(super) value: f64,
}

impl CircuitCiApp {
    pub(super) fn operating_point_results_panel(&mut self, ui: &mut egui::Ui) {
        if self.operating_points.is_empty() {
            return;
        }
        let mut export_bundle = false;
        egui::CollapsingHeader::new(format!(
            "DC Operating Points ({})",
            self.operating_points.len()
        ))
        .default_open(true)
        .show(ui, |ui| {
            let worst_keys = self
                .report
                .as_ref()
                .map(operating_point_worst_corner_keys)
                .unwrap_or_default();
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "{} operating-point artifact(s), {} probe value(s)",
                    self.operating_points.len(),
                    self.operating_points
                        .iter()
                        .map(|view| view.values.len())
                        .sum::<usize>()
                ));
                if ui.button("Copy CSV").clicked() {
                    ui.ctx().copy_text(operating_point_views_csv(
                        &self.operating_points,
                        self.report.as_ref(),
                    ));
                    self.status = "Copied DC operating-point table CSV.".to_string();
                }
                if ui.button("Copy Markdown").clicked() {
                    ui.ctx().copy_text(operating_point_views_markdown(
                        &self.operating_points,
                        self.report.as_ref(),
                    ));
                    self.status = "Copied DC operating-point table Markdown.".to_string();
                }
                if ui.button("Export Bundle").clicked() {
                    export_bundle = true;
                }
            });
            egui::Grid::new("dc_operating_point_results")
                .num_columns(7)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Scenario");
                    ui.strong("Sweep");
                    ui.strong("Corner");
                    ui.strong("Probe");
                    ui.strong("Value");
                    ui.strong("Worst");
                    ui.strong("Artifact");
                    ui.end_row();
                    for view in &self.operating_points {
                        let sweep = view.sweep.as_deref().unwrap_or("nominal");
                        let corner = view.corner.as_deref().unwrap_or("nominal");
                        for value in &view.values {
                            let worst = operating_point_row_key(view, &value.probe);
                            ui.monospace(&view.scenario);
                            ui.monospace(sweep);
                            ui.monospace(corner);
                            ui.monospace(&value.probe);
                            ui.monospace(format_value(value.value));
                            if worst_keys.contains(&worst) {
                                ui.label("limiting");
                            } else {
                                ui.label("");
                            }
                            ui.monospace(short_artifact_label(&view.path));
                            ui.end_row();
                        }
                    }
                });
        });
        if export_bundle {
            self.export_scope_report_bundle(&[]);
        }
    }
}

pub(in crate::gui) fn load_report_operating_points_with_progress_and_cancel<F, C>(
    report: &ValidationReport,
    mut on_progress: F,
    should_cancel: C,
) -> Result<Vec<OperatingPointView>>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let paths = operating_point_artifact_paths(report);
    let mut views = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        if should_cancel() {
            return Err(crate::cancellation::canceled(
                "DC operating-point loading canceled before completion.",
            ));
        }
        let started_at = Instant::now();
        on_progress(
            "Loading DC operating points",
            format!(
                "Loading operating point {} of {}: {}.",
                index + 1,
                paths.len(),
                path
            ),
        );
        let mut view = load_operating_point_csv(Path::new(path), path)?;
        let metadata = operating_point_path_metadata(path);
        view.scenario = metadata.scenario;
        view.sweep = metadata.sweep;
        view.corner = metadata.corner;
        on_progress(
            "Loaded DC operating point",
            format!(
                "{}: {} value(s) in {} ms.",
                path,
                view.values.len(),
                started_at.elapsed().as_millis()
            ),
        );
        views.push(view);
    }
    Ok(views)
}

fn operating_point_artifact_paths(report: &ValidationReport) -> Vec<String> {
    let mut paths = report
        .artifacts
        .iter()
        .filter(|artifact| artifact.ends_with("/operating_point.csv"))
        .cloned()
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn load_operating_point_csv(path: &Path, label: &str) -> Result<OperatingPointView> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read operating-point CSV {}.", path.display()))?;
    parse_operating_point_csv_text(&text, label)
}

pub(super) fn parse_operating_point_csv_text(
    text: &str,
    label: &str,
) -> Result<OperatingPointView> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .context("Operating-point CSV has no header row.")?;
    let values = lines
        .next()
        .context("Operating-point CSV has no value row.")?;
    if lines.next().is_some() {
        anyhow::bail!("Operating-point CSV must contain exactly one value row.");
    }
    let probes = split_operating_point_fields(header);
    let samples = split_operating_point_fields(values);
    if probes.is_empty() {
        anyhow::bail!("Operating-point CSV has no probe columns.");
    }
    if probes.len() != samples.len() {
        anyhow::bail!(
            "Operating-point CSV has {} probe columns but {} value columns.",
            probes.len(),
            samples.len()
        );
    }
    let values = probes
        .into_iter()
        .zip(samples)
        .map(|(probe, sample)| {
            let value = sample
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .with_context(|| format!("Operating-point value {sample} is not finite."))?;
            Ok(OperatingPointValue {
                probe: probe.to_string(),
                value,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let metadata = operating_point_path_metadata(label);
    Ok(OperatingPointView {
        label: short_artifact_label(label),
        path: label.to_string(),
        scenario: metadata.scenario,
        sweep: metadata.sweep,
        corner: metadata.corner,
        values,
    })
}

fn split_operating_point_fields(line: &str) -> Vec<&str> {
    line.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OperatingPointPathMetadata {
    scenario: String,
    sweep: Option<String>,
    corner: Option<String>,
}

fn operating_point_path_metadata(path: &str) -> OperatingPointPathMetadata {
    let parts = path.split('/').collect::<Vec<_>>();
    let Some(file_index) = parts
        .iter()
        .rposition(|part| *part == "operating_point.csv")
    else {
        return OperatingPointPathMetadata {
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
    let scenario = file_index
        .checked_sub(2)
        .and_then(|index| parts.get(index))
        .copied()
        .unwrap_or("unknown")
        .to_string();
    let (sweep, corner) = run
        .rsplit_once("_corner_")
        .map(|(sweep, corner)| (Some(sweep.to_string()), Some(format!("corner_{corner}"))))
        .unwrap_or((None, Some(run.to_string())));
    OperatingPointPathMetadata {
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

fn operating_point_worst_corner_keys(report: &ValidationReport) -> BTreeSet<String> {
    report
        .infos
        .iter()
        .filter(|finding| finding.id == "ANALOG_SWEEP_MARGIN_SUMMARY")
        .filter(|finding| {
            finding
                .measured
                .get("quantity")
                .and_then(serde_json::Value::as_str)
                == Some("operating point")
        })
        .filter_map(operating_point_finding_key)
        .collect()
}

fn operating_point_finding_key(finding: &Finding) -> Option<String> {
    let scenario = &finding.scenario;
    let sweep = finding
        .measured
        .get("analog_sweep")
        .and_then(serde_json::Value::as_str)?;
    let corner = finding
        .measured
        .get("analog_corner")
        .and_then(serde_json::Value::as_str)?;
    let probe = finding
        .measured
        .get("probe")
        .and_then(serde_json::Value::as_str)?;
    Some(format!("{scenario}|{sweep}|{corner}|{probe}"))
}

fn operating_point_row_key(view: &OperatingPointView, probe: &str) -> String {
    format!(
        "{}|{}|{}|{}",
        view.scenario,
        view.sweep.as_deref().unwrap_or("nominal"),
        view.corner.as_deref().unwrap_or("nominal"),
        probe
    )
}

pub(super) fn operating_point_views_csv(
    views: &[OperatingPointView],
    report: Option<&ValidationReport>,
) -> String {
    let mut csv = String::from("scenario,sweep,corner,probe,value,worst,artifact\n");
    let worst_keys = report
        .map(operating_point_worst_corner_keys)
        .unwrap_or_default();
    for view in views {
        let sweep = view.sweep.as_deref().unwrap_or("nominal");
        let corner = view.corner.as_deref().unwrap_or("nominal");
        for value in &view.values {
            let worst = worst_keys.contains(&operating_point_row_key(view, &value.probe));
            csv.push_str(&csv_row([
                view.scenario.clone(),
                sweep.to_string(),
                corner.to_string(),
                value.probe.clone(),
                format!("{:.12e}", value.value),
                worst_label(worst).to_string(),
                view.path.clone(),
            ]));
        }
    }
    csv
}

pub(super) fn operating_point_views_markdown(
    views: &[OperatingPointView],
    report: Option<&ValidationReport>,
) -> String {
    let mut markdown = String::from(
        "| Scenario | Sweep | Corner | Probe | Value | Worst | Artifact |\n| --- | --- | --- | --- | ---: | --- | --- |\n",
    );
    let worst_keys = report
        .map(operating_point_worst_corner_keys)
        .unwrap_or_default();
    for view in views {
        let sweep = view.sweep.as_deref().unwrap_or("nominal");
        let corner = view.corner.as_deref().unwrap_or("nominal");
        for value in &view.values {
            let worst = worst_keys.contains(&operating_point_row_key(view, &value.probe));
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                markdown_escape(&view.scenario),
                markdown_escape(sweep),
                markdown_escape(corner),
                markdown_escape(&value.probe),
                format_value(value.value),
                worst_label(worst),
                markdown_escape(&view.label),
            ));
        }
    }
    markdown
}

pub(super) fn operating_point_views_html(
    views: &[OperatingPointView],
    report: Option<&ValidationReport>,
) -> String {
    let worst_keys = report
        .map(operating_point_worst_corner_keys)
        .unwrap_or_default();
    let mut html = String::from(
        "\
<table>
  <thead>
    <tr><th>Scenario</th><th>Sweep</th><th>Corner</th><th>Probe</th><th>Value</th><th>Worst</th><th>Artifact</th></tr>
  </thead>
  <tbody>
",
    );
    for view in views {
        let sweep = view.sweep.as_deref().unwrap_or("nominal");
        let corner = view.corner.as_deref().unwrap_or("nominal");
        for value in &view.values {
            let worst = worst_keys.contains(&operating_point_row_key(view, &value.probe));
            html.push_str(&format!(
                "    <tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"number\">{}</td><td>{}</td><td>{}</td></tr>\n",
                html_escape(&view.scenario),
                html_escape(sweep),
                html_escape(corner),
                html_escape(&value.probe),
                html_escape(&format_value(value.value)),
                worst_label(worst),
                html_escape(&view.label),
            ));
        }
    }
    html.push_str("  </tbody>\n</table>");
    html
}

fn worst_label(worst: bool) -> &'static str {
    if worst { "limiting" } else { "" }
}

fn markdown_escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn html_escape(value: &str) -> String {
    if value.trim().is_empty() {
        return "-".to_string();
    }
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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
    use super::{
        load_report_operating_points_with_progress_and_cancel, operating_point_views_csv,
        parse_operating_point_csv_text,
    };
    use crate::reports::ValidationReport;

    #[test]
    fn operating_point_parser_reads_single_row_values_and_corner_metadata() {
        let view = parse_operating_point_csv_text(
            "vin,midpoint\n5.0,2.5\n",
            "out/analog/divider_dc_bias/divider_tolerance_corner_007/operating_point.csv",
        )
        .unwrap();
        assert_eq!(view.scenario, "divider_dc_bias");
        assert_eq!(view.sweep.as_deref(), Some("divider_tolerance"));
        assert_eq!(view.corner.as_deref(), Some("corner_007"));
        assert_eq!(view.values[0].probe, "vin");
        assert_eq!(view.values[0].value, 5.0);
        assert_eq!(view.values[1].probe, "midpoint");
        assert_eq!(view.values[1].value, 2.5);
    }

    #[test]
    fn operating_point_csv_export_flattens_corner_values() {
        let view = parse_operating_point_csv_text(
            "vin midpoint\n5.0 2.5\n",
            "out/analog/divider_dc_bias/divider_tolerance_corner_001/operating_point.csv",
        )
        .unwrap();
        let csv = operating_point_views_csv(&[view], None);
        assert!(csv.starts_with("scenario,sweep,corner,probe,value,worst,artifact\n"));
        assert!(
            csv.contains("divider_dc_bias,divider_tolerance,corner_001,vin,5.000000000000e0,,")
        );
        assert!(
            csv.contains(
                "divider_dc_bias,divider_tolerance,corner_001,midpoint,2.500000000000e0,,"
            )
        );
    }

    #[test]
    fn report_operating_point_loader_reads_artifacts_in_stable_order() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("analog").join("divider_dc_bias");
        let first = base
            .join("divider_tolerance_corner_001")
            .join("operating_point.csv");
        let second = base
            .join("divider_tolerance_corner_002")
            .join("operating_point.csv");
        std::fs::create_dir_all(first.parent().unwrap()).unwrap();
        std::fs::create_dir_all(second.parent().unwrap()).unwrap();
        std::fs::write(&first, "vin,midpoint\n5.0,2.5\n").unwrap();
        std::fs::write(&second, "vin,midpoint\n4.75,2.375\n").unwrap();
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
            load_report_operating_points_with_progress_and_cancel(&report, |_, _| {}, || false)
                .unwrap();

        assert_eq!(views.len(), 2);
        assert_eq!(views[0].corner.as_deref(), Some("corner_001"));
        assert_eq!(views[1].corner.as_deref(), Some("corner_002"));
        assert_eq!(views[1].values[1].probe, "midpoint");
        assert_eq!(views[1].values[1].value, 2.375);
    }
}
