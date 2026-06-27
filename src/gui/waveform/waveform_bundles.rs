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
use super::waveform_snapshots::{markdown_escape, scope_snapshots_csv, scope_snapshots_markdown};
use crate::gui::{CircuitCiApp, ScopeMeasurementSnapshot};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct ScopeReportBundleFiles {
    scope_plot_svg: String,
    index_html: String,
    measurement_snapshots_csv: String,
    measurement_snapshots_markdown: String,
    readme: String,
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
                self.status = format!(
                    "Exported scope report bundle with {} snapshot row(s) to {}.",
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
        if snapshots.is_empty() {
            self.status =
                "No scope measurement snapshots match the current bundle filters.".to_string();
            return None;
        }
        let Some(scope_plot_svg) = self.current_scope_plot_svg() else {
            self.status = "No scope plot is available to include in the report bundle.".to_string();
            return None;
        };
        let measurement_snapshots_csv = scope_snapshots_csv(snapshots);
        let measurement_snapshots_markdown = scope_snapshots_markdown(snapshots);
        let metadata = scope_report_bundle_content_metadata(
            &scope_plot_svg,
            &measurement_snapshots_csv,
            &measurement_snapshots_markdown,
        );
        Some(ScopeReportBundleFiles {
            scope_plot_svg,
            index_html: self.scope_report_bundle_index_html(snapshots, &metadata),
            measurement_snapshots_csv,
            measurement_snapshots_markdown,
            readme: self.scope_report_bundle_readme(snapshots, &metadata),
        })
    }

    fn scope_report_bundle_readme(
        &self,
        snapshots: &[ScopeMeasurementSnapshot],
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

fn current_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
