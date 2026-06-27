use super::waveform_bundle_integrity::{
    SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_CSV, SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_MARKDOWN,
    ScopeReportBundleArtifactMetadata, html_escape, scope_report_bundle_artifact_manifest_csv,
    scope_report_bundle_artifact_metadata_html, scope_report_bundle_artifact_metadata_markdown,
    scope_report_bundle_content_metadata, scope_report_bundle_integrity_details,
    scope_report_bundle_integrity_details_csv, scope_report_bundle_integrity_details_markdown,
};
use super::waveform_footprint::{
    WaveformFootprintSortKey, WaveformFootprintSourceFilter, WaveformFootprintSourceSummary,
    waveform_footprint_rows_with_diagnostics, waveform_footprint_source_summaries,
    waveform_footprint_summary_markdown,
};
use super::waveform_load::format_waveform_load_bytes;
use super::waveform_operating_point::{
    operating_point_views_csv, operating_point_views_html, operating_point_views_markdown,
};
use super::waveform_snapshots::{markdown_escape, scope_snapshots_csv, scope_snapshots_markdown};
use crate::gui::{CircuitCiApp, ScopeMeasurementSnapshot};
use crate::reports::{Finding, ValidationReport};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const ANALOG_SWEEP_MARGIN_SUMMARY: &str = "ANALOG_SWEEP_MARGIN_SUMMARY";

pub(super) struct ScopeReportBundleFiles {
    scope_plot_svg: String,
    index_html: String,
    measurement_snapshots_csv: String,
    measurement_snapshots_markdown: String,
    operating_points_csv: String,
    operating_points_markdown: String,
    sweep_margin_summaries_csv: String,
    sweep_margin_summaries_markdown: String,
    readme: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ScopeSweepMarginSummaryRow {
    scenario: String,
    assertion: String,
    probe: String,
    sweep: String,
    corner: String,
    inputs: String,
    passed: bool,
    measured: String,
    relation: String,
    limit: String,
    margin: String,
    evaluated_corners: u64,
}

impl CircuitCiApp {
    pub(in crate::gui) fn export_scope_report_bundle(
        &mut self,
        snapshots: &[ScopeMeasurementSnapshot],
    ) {
        let Some(files) = self.prepare_scope_report_bundle_files(snapshots) else {
            return;
        };
        let base_dir = output_bundle_base_dir(&self.output_dir);
        let bundle_dir = unique_scope_report_bundle_dir(&base_dir, current_unix_millis());
        match write_scope_report_bundle_files(&bundle_dir, &files) {
            Ok(()) => {
                self.push_recent_scope_report_bundle(bundle_dir.to_string_lossy().into_owned());
                let operating_point_values = self
                    .operating_points
                    .iter()
                    .map(|view| view.values.len())
                    .sum::<usize>();
                self.status = format!(
                    "Exported scope report bundle with {} snapshot row(s) and {operating_point_values} DC value row(s) to {}.",
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

    pub(super) fn export_scope_compare_report_bundle(&mut self, open_index: bool) -> usize {
        if self.current_scope_compare_traces().len() < 2 {
            self.status =
                "Select and pin at least two visible scope traces before bundling a comparison."
                    .to_string();
            return 0;
        }
        let snapshots = self.current_scope_compare_cursor_snapshots(
            "Compare Set",
            "Runtime compare bundle row generated from selected and pinned Scopes traces.",
        );
        if snapshots.len() < 2 {
            self.status =
                "No visible scope comparison measurements are available to bundle.".to_string();
            return 0;
        }
        let count = snapshots.len();
        self.export_scope_report_bundle(&snapshots);
        if self.status.contains("Exported scope report bundle") {
            self.status = format!(
                "Exported scope compare report bundle with {count} trace row(s). {}",
                self.status
            );
        }
        if open_index {
            let export_status = self.status.clone();
            if let Some(bundle) = self.waveform_recent_report_bundles.first().cloned() {
                if self.open_scope_report_bundle_index(&bundle) {
                    self.status = format!("{export_status} {}", self.status);
                }
            } else {
                self.status = format!(
                    "{export_status} No recent compare report bundle was available to open."
                );
            }
        }
        count
    }

    pub(super) fn prepare_scope_report_bundle_files(
        &mut self,
        snapshots: &[ScopeMeasurementSnapshot],
    ) -> Option<ScopeReportBundleFiles> {
        if snapshots.is_empty() && self.operating_points.is_empty() {
            self.status = "No scope measurement snapshots or DC operating-point rows are available to bundle.".to_string();
            return None;
        }
        let scope_plot_svg = if let Some(scope_plot_svg) = self.current_scope_plot_svg() {
            scope_plot_svg
        } else if !self.operating_points.is_empty() {
            dc_operating_point_placeholder_svg()
        } else {
            self.status = "No scope plot is available to include in the report bundle.".to_string();
            return None;
        };
        let measurement_snapshots_csv = scope_snapshots_csv(snapshots);
        let measurement_snapshots_markdown = scope_snapshots_markdown(snapshots);
        let operating_points_csv =
            operating_point_views_csv(&self.operating_points, self.report.as_ref());
        let operating_points_markdown =
            operating_point_views_markdown(&self.operating_points, self.report.as_ref());
        let sweep_margin_rows = scope_sweep_margin_summary_rows(self.report.as_ref());
        let sweep_margin_summaries_csv = scope_sweep_margin_summaries_csv(&sweep_margin_rows);
        let sweep_margin_summaries_markdown =
            scope_sweep_margin_summaries_markdown(&sweep_margin_rows);
        let metadata = scope_report_bundle_content_metadata(
            &scope_plot_svg,
            &measurement_snapshots_csv,
            &measurement_snapshots_markdown,
            &operating_points_csv,
            &operating_points_markdown,
            &sweep_margin_summaries_csv,
            &sweep_margin_summaries_markdown,
        );
        Some(ScopeReportBundleFiles {
            scope_plot_svg,
            index_html: self.scope_report_bundle_index_html(
                snapshots,
                &sweep_margin_rows,
                &metadata,
            ),
            measurement_snapshots_csv,
            measurement_snapshots_markdown,
            operating_points_csv,
            operating_points_markdown,
            sweep_margin_summaries_csv,
            sweep_margin_summaries_markdown,
            readme: self.scope_report_bundle_readme(snapshots, &sweep_margin_rows, &metadata),
        })
    }

    fn scope_report_bundle_readme(
        &self,
        snapshots: &[ScopeMeasurementSnapshot],
        sweep_margin_rows: &[ScopeSweepMarginSummaryRow],
        metadata: &[ScopeReportBundleArtifactMetadata],
    ) -> String {
        let selected_context = self
            .selected_scope_trace_label()
            .unwrap_or_else(|| "unavailable".to_string());
        let query = self.waveform_snapshot_filter.trim();
        let query = if query.is_empty() { "(empty)" } else { query };
        let footprint_summary = self.scope_report_bundle_footprint_summary_markdown();
        format!(
            "\
# CircuitCI Scope Report Bundle

This folder is a runtime export from the Scopes workspace. It is derived from loaded waveform artifacts and transient GUI state; it is not persisted Board IR project truth.

## Files

- `scope_plot.svg` - configured Scopes plot SVG.
- `index.html` - local bundle index page with links and summary context.
- `measurement_snapshots.csv` - filtered measurement snapshot rows.
- `measurement_snapshots.md` - filtered measurement snapshot rows as Markdown.
- `operating_points.csv` - loaded DC operating-point rows.
- `operating_points.md` - loaded DC operating-point rows as Markdown.
- `sweep_margin_summaries.csv` - worst-corner sweep margin summary rows from the loaded validation report.
- `sweep_margin_summaries.md` - worst-corner sweep margin summary rows as Markdown.
- `README.md` - this manifest.
- `artifact_manifest.csv` - expected size and SHA-256 metadata for required bundle artifacts.
- `artifact_integrity_details.csv` - artifact integrity detail rows as CSV.
- `artifact_integrity_details.md` - artifact integrity detail rows as Markdown.

## Artifact Metadata

The table below covers generated content artifacts. `artifact_manifest.csv` records expected size and SHA-256 metadata for the required bundle files after export.
The artifact integrity detail files are generated from that manifest as optional
report conveniences and are not part of the manifest they describe.

{}

## Snapshot Projection

- Rows: {}
- Search: {}
- Source: {}
- Sort: {}
- Group: {}

## DC Operating Points

- Artifacts: {}
- Values: {}

{}

## Sweep Margin Summaries

- Rows: {}

{}

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

{}
",
            scope_report_bundle_artifact_metadata_markdown(metadata),
            snapshots.len(),
            markdown_escape(query),
            self.waveform_snapshot_source_filter.label(),
            self.waveform_snapshot_sort_key.label(),
            self.waveform_snapshot_group_mode.label(),
            self.operating_points.len(),
            self.operating_points
                .iter()
                .map(|view| view.values.len())
                .sum::<usize>(),
            operating_point_views_markdown(&self.operating_points, self.report.as_ref()),
            sweep_margin_rows.len(),
            scope_sweep_margin_summaries_markdown(sweep_margin_rows),
            self.waveform_plot_export_size.label(),
            yes_no(self.waveform_plot_export_cursors),
            yes_no(self.waveform_plot_export_trigger),
            yes_no(self.waveform_plot_export_snapshots),
            yes_no(self.waveform_split_trace_units),
            self.selected_waveform,
            self.selected_probe,
            markdown_escape(&selected_context),
            footprint_summary,
        )
    }

    fn scope_report_bundle_index_html(
        &self,
        snapshots: &[ScopeMeasurementSnapshot],
        sweep_margin_rows: &[ScopeSweepMarginSummaryRow],
        metadata: &[ScopeReportBundleArtifactMetadata],
    ) -> String {
        let selected_context = self
            .selected_scope_trace_label()
            .unwrap_or_else(|| "unavailable".to_string());
        let query = self.waveform_snapshot_filter.trim();
        let query = if query.is_empty() { "(empty)" } else { query };
        let (footprint_count, footprint_bytes, footprint_summaries) =
            self.scope_report_bundle_footprint_summary();
        format!(
            "\
<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\">
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
  <title>CircuitCI Scope Report Bundle</title>
  <style>
    body {{ font-family: system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif; margin: 2rem; line-height: 1.45; color: #202124; }}
    main {{ max-width: 980px; margin: 0 auto; }}
    table {{ border-collapse: collapse; width: 100%; margin: 0.75rem 0 1.5rem; }}
    th, td {{ border: 1px solid #d0d7de; padding: 0.35rem 0.5rem; text-align: left; }}
    th {{ background: #f6f8fa; }}
    td.number {{ text-align: right; font-variant-numeric: tabular-nums; }}
    img {{ max-width: 100%; border: 1px solid #d0d7de; }}
  </style>
</head>
<body>
<main>
  <h1>CircuitCI Scope Report Bundle</h1>
  <p>This is a runtime export from the Scopes workspace. It is derived from loaded waveform artifacts and transient GUI state; it is not persisted Board IR project truth.</p>
  <h2>Artifacts</h2>
  <ul>
    <li><a href=\"scope_plot.svg\">scope_plot.svg</a> - configured Scopes plot SVG.</li>
    <li><a href=\"measurement_snapshots.csv\">measurement_snapshots.csv</a> - filtered measurement snapshot rows.</li>
    <li><a href=\"measurement_snapshots.md\">measurement_snapshots.md</a> - filtered measurement snapshot rows as Markdown.</li>
    <li><a href=\"operating_points.csv\">operating_points.csv</a> - loaded DC operating-point rows.</li>
    <li><a href=\"operating_points.md\">operating_points.md</a> - loaded DC operating-point rows as Markdown.</li>
    <li><a href=\"sweep_margin_summaries.csv\">sweep_margin_summaries.csv</a> - worst-corner sweep margin summary rows from the loaded validation report.</li>
    <li><a href=\"sweep_margin_summaries.md\">sweep_margin_summaries.md</a> - worst-corner sweep margin summary rows as Markdown.</li>
    <li><a href=\"README.md\">README.md</a> - text manifest.</li>
    <li><a href=\"artifact_manifest.csv\">artifact_manifest.csv</a> - expected size and SHA-256 metadata.</li>
    <li><a href=\"artifact_integrity_details.csv\">artifact_integrity_details.csv</a> - artifact integrity detail rows as CSV.</li>
    <li><a href=\"artifact_integrity_details.md\">artifact_integrity_details.md</a> - artifact integrity detail rows as Markdown.</li>
  </ul>
  {}
  <h2>Plot Preview</h2>
  <p><a href=\"scope_plot.svg\"><img src=\"scope_plot.svg\" alt=\"CircuitCI scope plot\"></a></p>
  <h2>Snapshot Projection</h2>
  <table>
    <tbody>
      <tr><th>Rows</th><td class=\"number\">{}</td></tr>
      <tr><th>Search</th><td>{}</td></tr>
      <tr><th>Source</th><td>{}</td></tr>
      <tr><th>Sort</th><td>{}</td></tr>
      <tr><th>Group</th><td>{}</td></tr>
    </tbody>
  </table>
  <h2>DC Operating Points</h2>
  <p><a href=\"operating_points.csv\">CSV</a> | <a href=\"operating_points.md\">Markdown</a></p>
  <table>
    <tbody>
      <tr><th>Artifacts</th><td class=\"number\">{}</td></tr>
      <tr><th>Values</th><td class=\"number\">{}</td></tr>
    </tbody>
  </table>
  {}
  <h2>Sweep Margin Summaries</h2>
  <p><a href=\"sweep_margin_summaries.csv\">CSV</a> | <a href=\"sweep_margin_summaries.md\">Markdown</a></p>
  {}
  <h2>Plot Export Options</h2>
  <table>
    <tbody>
      <tr><th>Size</th><td>{}</td></tr>
      <tr><th>Include cursors</th><td>{}</td></tr>
      <tr><th>Include trigger markers</th><td>{}</td></tr>
      <tr><th>Include snapshot annotations</th><td>{}</td></tr>
      <tr><th>Split units</th><td>{}</td></tr>
    </tbody>
  </table>
  <h2>Selected Trace Context</h2>
  <table>
    <tbody>
      <tr><th>Selected waveform index</th><td class=\"number\">{}</td></tr>
      <tr><th>Selected probe index</th><td class=\"number\">{}</td></tr>
      <tr><th>Selected trace</th><td>{}</td></tr>
    </tbody>
  </table>
  {}
</main>
</body>
</html>
",
            scope_report_bundle_artifact_metadata_html(metadata),
            snapshots.len(),
            html_escape(query),
            html_escape(self.waveform_snapshot_source_filter.label()),
            html_escape(self.waveform_snapshot_sort_key.label()),
            html_escape(self.waveform_snapshot_group_mode.label()),
            self.operating_points.len(),
            self.operating_points
                .iter()
                .map(|view| view.values.len())
                .sum::<usize>(),
            operating_point_views_html(&self.operating_points, self.report.as_ref()),
            scope_sweep_margin_summaries_html(sweep_margin_rows),
            html_escape(self.waveform_plot_export_size.label()),
            yes_no(self.waveform_plot_export_cursors),
            yes_no(self.waveform_plot_export_trigger),
            yes_no(self.waveform_plot_export_snapshots),
            yes_no(self.waveform_split_trace_units),
            self.selected_waveform,
            self.selected_probe,
            html_escape(&selected_context),
            scope_report_bundle_footprint_summary_html(
                &footprint_summaries,
                footprint_count,
                footprint_bytes,
            ),
        )
    }

    fn scope_report_bundle_footprint_summary_markdown(&self) -> String {
        let (count, bytes, summaries) = self.scope_report_bundle_footprint_summary();
        waveform_footprint_summary_markdown(&summaries, count, bytes)
    }

    fn scope_report_bundle_footprint_summary(
        &self,
    ) -> (usize, usize, Vec<WaveformFootprintSourceSummary>) {
        let rows = waveform_footprint_rows_with_diagnostics(
            &self.waveforms,
            &self.waveform_load_diagnostics,
            "",
            WaveformFootprintSourceFilter::All,
            WaveformFootprintSortKey::EstimatedBytes,
            true,
        );
        let total_bytes = rows.iter().map(|row| row.estimated_bytes).sum();
        let summaries = waveform_footprint_source_summaries(&rows);
        (rows.len(), total_bytes, summaries)
    }
}

fn scope_report_bundle_footprint_summary_html(
    summaries: &[WaveformFootprintSourceSummary],
    total_count: usize,
    total_bytes: usize,
) -> String {
    let mut html = String::from(
        "\
<h2>Loaded Waveform Footprint Summary</h2>
<table>
  <thead>
    <tr><th>Source</th><th>Views</th><th>Estimated bytes</th><th>Estimated size</th></tr>
  </thead>
  <tbody>
",
    );
    html.push_str(&format!(
        "    <tr><td>Total</td><td class=\"number\">{total_count}</td><td class=\"number\">{total_bytes}</td><td class=\"number\">{}</td></tr>\n",
        html_escape(&format_waveform_load_bytes(Some(total_bytes as u64)))
    ));
    for summary in summaries {
        html.push_str(&format!(
            "    <tr><td>{}</td><td class=\"number\">{}</td><td class=\"number\">{}</td><td class=\"number\">{}</td></tr>\n",
            html_escape(summary.source.label()),
            summary.count,
            summary.estimated_bytes,
            html_escape(&format_waveform_load_bytes(Some(summary.estimated_bytes as u64)))
        ));
    }
    html.push_str("  </tbody>\n</table>");
    html
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

pub(super) fn write_scope_report_bundle_files(
    bundle_dir: &Path,
    files: &ScopeReportBundleFiles,
) -> std::io::Result<()> {
    fs::create_dir_all(bundle_dir)
        .and_then(|()| fs::write(bundle_dir.join("scope_plot.svg"), &files.scope_plot_svg))
        .and_then(|()| {
            fs::write(
                scope_report_bundle_index_path(bundle_dir),
                &files.index_html,
            )
        })
        .and_then(|()| {
            fs::write(
                bundle_dir.join("measurement_snapshots.csv"),
                &files.measurement_snapshots_csv,
            )
        })
        .and_then(|()| {
            fs::write(
                bundle_dir.join("measurement_snapshots.md"),
                &files.measurement_snapshots_markdown,
            )
        })
        .and_then(|()| {
            fs::write(
                bundle_dir.join("operating_points.csv"),
                &files.operating_points_csv,
            )
        })
        .and_then(|()| {
            fs::write(
                bundle_dir.join("operating_points.md"),
                &files.operating_points_markdown,
            )
        })
        .and_then(|()| {
            fs::write(
                bundle_dir.join("sweep_margin_summaries.csv"),
                &files.sweep_margin_summaries_csv,
            )
        })
        .and_then(|()| {
            fs::write(
                bundle_dir.join("sweep_margin_summaries.md"),
                &files.sweep_margin_summaries_markdown,
            )
        })
        .and_then(|()| fs::write(bundle_dir.join("README.md"), &files.readme))
        .and_then(|()| {
            let manifest = scope_report_bundle_artifact_manifest_csv(bundle_dir)?;
            fs::write(bundle_dir.join("artifact_manifest.csv"), manifest)
        })
        .and_then(|()| {
            let details = scope_report_bundle_integrity_details(bundle_dir);
            fs::write(
                bundle_dir.join(SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_CSV),
                scope_report_bundle_integrity_details_csv(&details),
            )
            .and_then(|()| {
                fs::write(
                    bundle_dir.join(SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_MARKDOWN),
                    scope_report_bundle_integrity_details_markdown(&details),
                )
            })
        })
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

pub(super) fn scope_report_bundle_index_path(bundle_dir: &Path) -> PathBuf {
    bundle_dir.join("index.html")
}

pub(super) fn scope_sweep_margin_summary_rows(
    report: Option<&ValidationReport>,
) -> Vec<ScopeSweepMarginSummaryRow> {
    report
        .into_iter()
        .flat_map(|report| report.infos.iter())
        .filter(|finding| finding.id == ANALOG_SWEEP_MARGIN_SUMMARY)
        .filter_map(scope_sweep_margin_summary_row)
        .collect()
}

fn scope_sweep_margin_summary_row(finding: &Finding) -> Option<ScopeSweepMarginSummaryRow> {
    let assertion = json_string(&finding.measured, "assertion")?;
    let sweep =
        json_string(&finding.measured, "analog_sweep").unwrap_or_else(|| "sweep".to_string());
    let corner =
        json_string(&finding.measured, "analog_corner").unwrap_or_else(|| "corner".to_string());
    let probe = json_string(&finding.measured, "probe").unwrap_or_default();
    let unit = json_string(&finding.measured, "measured_unit")
        .or_else(|| json_string(&finding.limit, "limit_unit"))
        .unwrap_or_default();
    let measured = format_numeric_with_unit(
        json_f64(&finding.measured, "measured_value"),
        &unit,
        "measured n/a",
    );
    let limit =
        format_numeric_with_unit(json_f64(&finding.limit, "limit_value"), &unit, "limit n/a");
    let margin = format_numeric_with_unit(json_f64(&finding.measured, "margin"), &unit, "n/a");
    Some(ScopeSweepMarginSummaryRow {
        scenario: finding.scenario.clone(),
        assertion,
        probe,
        sweep,
        corner,
        inputs: scope_sweep_margin_input_summary(finding),
        passed: json_bool(&finding.measured, "passed").unwrap_or(false),
        measured,
        relation: json_string(&finding.limit, "relation").unwrap_or_else(|| "limit".to_string()),
        limit,
        margin,
        evaluated_corners: json_u64(&finding.measured, "evaluated_corners").unwrap_or(0),
    })
}

pub(super) fn scope_sweep_margin_summaries_csv(rows: &[ScopeSweepMarginSummaryRow]) -> String {
    let mut csv = String::from(
        "scenario,assertion,probe,sweep,corner,inputs,passed,measured,relation,limit,margin,evaluated_corners\n",
    );
    for row in rows {
        let fields = [
            row.scenario.clone(),
            row.assertion.clone(),
            row.probe.clone(),
            row.sweep.clone(),
            row.corner.clone(),
            row.inputs.clone(),
            row.passed.to_string(),
            row.measured.clone(),
            row.relation.clone(),
            row.limit.clone(),
            row.margin.clone(),
            row.evaluated_corners.to_string(),
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

pub(super) fn scope_sweep_margin_summaries_markdown(rows: &[ScopeSweepMarginSummaryRow]) -> String {
    let mut markdown = String::from(
        "| Scenario | Assertion | Probe | Sweep | Corner | Inputs | Pass | Measured | Relation | Limit | Margin | Corners |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | ---: |\n",
    );
    for row in rows {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            markdown_escape(&row.scenario),
            markdown_escape(&row.assertion),
            markdown_escape(&row.probe),
            markdown_escape(&row.sweep),
            markdown_escape(&row.corner),
            markdown_escape(&row.inputs),
            if row.passed { "pass" } else { "fail" },
            markdown_escape(&row.measured),
            markdown_escape(&row.relation),
            markdown_escape(&row.limit),
            markdown_escape(&row.margin),
            row.evaluated_corners
        ));
    }
    markdown
}

fn scope_sweep_margin_summaries_html(rows: &[ScopeSweepMarginSummaryRow]) -> String {
    let mut html = String::from(
        "\
<table>
  <thead>
    <tr><th>Scenario</th><th>Assertion</th><th>Probe</th><th>Sweep</th><th>Corner</th><th>Inputs</th><th>Pass</th><th>Measured</th><th>Relation</th><th>Limit</th><th>Margin</th><th>Corners</th></tr>
  </thead>
  <tbody>
",
    );
    for row in rows {
        html.push_str(&format!(
            "    <tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"number\">{}</td></tr>\n",
            html_escape(&row.scenario),
            html_escape(&row.assertion),
            html_escape(&row.probe),
            html_escape(&row.sweep),
            html_escape(&row.corner),
            html_escape(&row.inputs),
            if row.passed { "pass" } else { "fail" },
            html_escape(&row.measured),
            html_escape(&row.relation),
            html_escape(&row.limit),
            html_escape(&row.margin),
            row.evaluated_corners
        ));
    }
    html.push_str("  </tbody>\n</table>");
    html
}

fn scope_sweep_margin_input_summary(finding: &Finding) -> String {
    let mut parts = Vec::new();
    if let Some(parameters) = json_number_map_summary(&finding.measured, "analog_parameters") {
        parts.push(parameters);
    }
    if let Some(component_values) =
        json_number_map_summary(&finding.measured, "analog_component_values")
    {
        parts.push(component_values);
    }
    if let Some(model_sections) =
        json_string_map_summary(&finding.measured, "analog_model_sections")
    {
        parts.push(model_sections);
    }
    parts.join("; ")
}

fn json_number_map_summary(map: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    let values = map.get(key)?.as_object()?;
    let parts: Vec<String> = values
        .iter()
        .filter_map(|(name, value)| value.as_f64().map(|number| (name, number)))
        .map(|(name, number)| format!("{name}={}", compact_number(number)))
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn json_string_map_summary(map: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    let values = map.get(key)?.as_object()?;
    let parts: Vec<String> = values
        .iter()
        .filter_map(|(name, value)| value.as_str().map(|string| (name, string)))
        .map(|(name, string)| format!("{name}:{string}"))
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn json_string(map: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    map.get(key)?.as_str().map(str::to_string)
}

fn json_f64(map: &BTreeMap<String, Value>, key: &str) -> Option<f64> {
    map.get(key)?.as_f64()
}

fn json_u64(map: &BTreeMap<String, Value>, key: &str) -> Option<u64> {
    map.get(key)?.as_u64()
}

fn json_bool(map: &BTreeMap<String, Value>, key: &str) -> Option<bool> {
    map.get(key)?.as_bool()
}

fn format_numeric_with_unit(value: Option<f64>, unit: &str, fallback: &str) -> String {
    value
        .map(|value| {
            format!("{} {}", compact_number(value), unit)
                .trim()
                .to_string()
        })
        .unwrap_or_else(|| fallback.to_string())
}

fn compact_number(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else if value.abs() >= 1.0e4 || value.abs() < 1.0e-3 {
        format!("{value:.6e}")
    } else {
        let text = format!("{value:.6}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn dc_operating_point_placeholder_svg() -> String {
    "\
<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"960\" height=\"240\" viewBox=\"0 0 960 240\" role=\"img\" aria-label=\"CircuitCI DC operating-point bundle\">
  <rect width=\"960\" height=\"240\" fill=\"#ffffff\"/>
  <rect x=\"24\" y=\"24\" width=\"912\" height=\"192\" fill=\"#f6f8fa\" stroke=\"#d0d7de\"/>
  <text x=\"48\" y=\"92\" font-family=\"system-ui, -apple-system, Segoe UI, sans-serif\" font-size=\"28\" fill=\"#202124\">CircuitCI DC Operating-Point Bundle</text>
  <text x=\"48\" y=\"136\" font-family=\"system-ui, -apple-system, Segoe UI, sans-serif\" font-size=\"18\" fill=\"#57606a\">No time or frequency trace plot was loaded for this export.</text>
  <text x=\"48\" y=\"168\" font-family=\"system-ui, -apple-system, Segoe UI, sans-serif\" font-size=\"18\" fill=\"#57606a\">See operating_points.csv and operating_points.md for bias evidence.</text>
</svg>
"
    .to_string()
}

fn csv_escape(value: String) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

fn current_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
