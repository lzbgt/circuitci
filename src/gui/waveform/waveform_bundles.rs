use super::waveform_bundle_integrity::{
    SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_CSV, SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_MARKDOWN,
    ScopeReportBundleArtifactMetadata, html_escape, optional_size_label,
    scope_report_bundle_artifact_manifest_csv, scope_report_bundle_artifact_metadata_html,
    scope_report_bundle_artifact_metadata_markdown, scope_report_bundle_artifact_status,
    scope_report_bundle_content_metadata, scope_report_bundle_integrity_details,
    scope_report_bundle_integrity_details_csv, scope_report_bundle_integrity_details_markdown,
    scope_report_bundle_missing_artifacts, short_optional_sha,
};
use super::waveform_footprint::{
    WaveformFootprintSortKey, WaveformFootprintSourceFilter, WaveformFootprintSourceSummary,
    waveform_footprint_rows_with_diagnostics, waveform_footprint_source_summaries,
    waveform_footprint_summary_markdown,
};
use super::waveform_load::format_waveform_load_bytes;
use super::waveform_snapshots::{markdown_escape, scope_snapshots_csv, scope_snapshots_markdown};
use crate::gui::{CircuitCiApp, ScopeMeasurementSnapshot};
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RECENT_SCOPE_BUNDLES: usize = 5;
struct ScopeReportBundleFiles {
    scope_plot_svg: String,
    index_html: String,
    measurement_snapshots_csv: String,
    measurement_snapshots_markdown: String,
    readme: String,
}

impl CircuitCiApp {
    pub(super) fn export_scope_report_bundle(&mut self, snapshots: &[ScopeMeasurementSnapshot]) {
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

    fn prepare_scope_report_bundle_files(
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

    pub(super) fn scope_recent_report_bundles_ui(
        &mut self,
        ui: &mut egui::Ui,
        snapshots: &[ScopeMeasurementSnapshot],
    ) {
        let Some(latest) = self.waveform_recent_report_bundles.first().cloned() else {
            return;
        };
        let latest_status = scope_report_bundle_artifact_status(Path::new(&latest));
        let latest_needs_refresh = latest_status.needs_refresh();
        ui.horizontal_wrapped(|ui| {
            ui.label("Recent bundle");
            ui.monospace(display_path_tail(&latest));
            ui.label(latest_status.label());
            if ui.button("Open Bundle Folder").clicked() {
                self.open_scope_report_bundle(&latest);
            }
            if ui.button("Open Bundle Index").clicked() {
                self.open_scope_report_bundle_index(&latest);
            }
            if ui.button("Open Integrity CSV").clicked() {
                self.open_scope_report_bundle_artifact(
                    &latest,
                    SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_CSV,
                    "integrity CSV",
                );
            }
            if ui.button("Open Integrity Markdown").clicked() {
                self.open_scope_report_bundle_artifact(
                    &latest,
                    SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_MARKDOWN,
                    "integrity Markdown",
                );
            }
            egui::ComboBox::from_id_salt("scope_recent_report_bundle_copy_paths")
                .selected_text("Copy Path")
                .show_ui(ui, |ui| {
                    if ui.button("Folder").clicked() {
                        self.copy_scope_report_bundle_path(ui, &latest, None, "folder");
                        ui.close();
                    }
                    if ui.button("Index").clicked() {
                        self.copy_scope_report_bundle_path(
                            ui,
                            &latest,
                            Some("index.html"),
                            "index",
                        );
                        ui.close();
                    }
                    if ui.button("Integrity CSV").clicked() {
                        self.copy_scope_report_bundle_path(
                            ui,
                            &latest,
                            Some(SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_CSV),
                            "integrity CSV",
                        );
                        ui.close();
                    }
                    if ui.button("Integrity Markdown").clicked() {
                        self.copy_scope_report_bundle_path(
                            ui,
                            &latest,
                            Some(SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_MARKDOWN),
                            "integrity Markdown",
                        );
                        ui.close();
                    }
                });
            if ui.button("Details").clicked() {
                self.waveform_bundle_integrity_details = Some(latest.clone());
            }
            if latest_needs_refresh && ui.button("Preview Refresh").clicked() {
                self.preview_scope_report_bundle_refresh(&latest);
            }
            if ui.button("Clean Old Bundles").clicked() {
                self.preview_old_scope_report_bundles();
            }
            if self.waveform_recent_report_bundles.len() > 1 {
                let older_bundles = self
                    .waveform_recent_report_bundles
                    .iter()
                    .skip(1)
                    .cloned()
                    .collect::<Vec<_>>();
                egui::ComboBox::from_id_salt("scope_recent_report_bundles")
                    .selected_text("Older")
                    .show_ui(ui, |ui| {
                        for bundle in older_bundles {
                            let status = scope_report_bundle_artifact_status(Path::new(&bundle));
                            let needs_refresh = status.needs_refresh();
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "{} - {}",
                                    display_path_tail(&bundle),
                                    status.label()
                                ));
                                if ui.small_button("Open").clicked() {
                                    self.open_scope_report_bundle(&bundle);
                                    ui.close();
                                }
                                if ui.small_button("CSV").clicked() {
                                    self.open_scope_report_bundle_artifact(
                                        &bundle,
                                        SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_CSV,
                                        "integrity CSV",
                                    );
                                    ui.close();
                                }
                                if ui.small_button("MD").clicked() {
                                    self.open_scope_report_bundle_artifact(
                                        &bundle,
                                        SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_MARKDOWN,
                                        "integrity Markdown",
                                    );
                                    ui.close();
                                }
                                if ui.small_button("Path").clicked() {
                                    self.copy_scope_report_bundle_path(ui, &bundle, None, "folder");
                                    ui.close();
                                }
                                if ui.small_button("Details").clicked() {
                                    self.waveform_bundle_integrity_details = Some(bundle.clone());
                                    ui.close();
                                }
                                if needs_refresh && ui.small_button("Refresh").clicked() {
                                    self.preview_scope_report_bundle_refresh(&bundle);
                                    ui.close();
                                }
                            });
                        }
                    });
            }
        });
        if let Some(bundle) = self.waveform_bundle_refresh_preview.clone() {
            let status = scope_report_bundle_artifact_status(Path::new(&bundle));
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "Refresh preview: regenerate artifacts for {} ({}).",
                    display_path_tail(&bundle),
                    status.label()
                ));
                if ui.button("Confirm Refresh").clicked() {
                    self.confirm_scope_report_bundle_refresh(snapshots);
                }
                if ui.button("Cancel").clicked() {
                    self.waveform_bundle_refresh_preview = None;
                    self.status = "Canceled scope report bundle refresh.".to_string();
                }
            });
        }
        if let Some(bundle) = self.waveform_bundle_integrity_details.clone() {
            self.scope_report_bundle_integrity_details_ui(ui, &bundle);
        }
        if !self.waveform_bundle_cleanup_preview.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "Cleanup preview: remove {} old bundle folder(s).",
                    self.waveform_bundle_cleanup_preview.len()
                ));
                if ui.button("Confirm Cleanup").clicked() {
                    self.confirm_scope_report_bundle_cleanup();
                }
                if ui.button("Cancel").clicked() {
                    self.waveform_bundle_cleanup_preview.clear();
                    self.status = "Canceled scope report bundle cleanup.".to_string();
                }
            });
        }
    }

    fn push_recent_scope_report_bundle(&mut self, bundle: String) {
        self.waveform_recent_report_bundles
            .retain(|existing| existing != &bundle);
        self.waveform_recent_report_bundles.insert(0, bundle);
        self.waveform_recent_report_bundles
            .truncate(MAX_RECENT_SCOPE_BUNDLES);
    }

    fn scope_report_bundle_integrity_details_ui(&mut self, ui: &mut egui::Ui, bundle: &str) {
        let bundle_path = Path::new(bundle);
        let details = scope_report_bundle_integrity_details(bundle_path);
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Bundle Integrity Details");
                ui.monospace(display_path_tail(bundle));
                if let Some(error) = &details.manifest_error {
                    ui.label(format!("Manifest: {error}"));
                }
                if ui.button("Copy Details CSV").clicked() {
                    ui.ctx()
                        .copy_text(scope_report_bundle_integrity_details_csv(&details));
                    self.status = format!(
                        "Copied integrity details for {} as CSV.",
                        display_path_tail(bundle)
                    );
                }
                if ui.button("Copy Details Markdown").clicked() {
                    ui.ctx()
                        .copy_text(scope_report_bundle_integrity_details_markdown(&details));
                    self.status = format!(
                        "Copied integrity details for {} as Markdown.",
                        display_path_tail(bundle)
                    );
                }
                if ui.button("Close").clicked() {
                    self.waveform_bundle_integrity_details = None;
                }
            });
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new("scope_bundle_integrity_details")
                        .num_columns(7)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Artifact");
                            ui.label("State");
                            ui.label("Expected Size");
                            ui.label("Current Size");
                            ui.label("Expected SHA-256");
                            ui.label("Current SHA-256");
                            ui.label("Path");
                            ui.end_row();
                            for row in &details.rows {
                                ui.monospace(&row.label);
                                ui.label(row.state.label());
                                ui.monospace(optional_size_label(row.expected_size));
                                ui.monospace(optional_size_label(row.current_size));
                                ui.monospace(short_optional_sha(row.expected_sha256.as_deref()));
                                ui.monospace(short_optional_sha(row.current_sha256.as_deref()));
                                ui.monospace(&row.path);
                                ui.end_row();
                            }
                        });
                });
        });
    }

    pub(super) fn preview_scope_report_bundle_refresh(&mut self, bundle: &str) {
        let path = Path::new(bundle);
        if !is_scope_report_bundle_path(path) {
            self.status = format!("Refusing to refresh non-bundle path {}.", path.display());
            return;
        }
        let status = scope_report_bundle_artifact_status(path);
        if !status.needs_refresh() {
            self.waveform_bundle_refresh_preview = None;
            self.status = format!(
                "Scope report bundle artifacts are already complete for {}.",
                path.display()
            );
            return;
        }
        self.waveform_bundle_refresh_preview = Some(bundle.to_string());
        self.status = format!(
            "Previewing scope report bundle refresh for {}.",
            status.label()
        );
    }

    pub(super) fn confirm_scope_report_bundle_refresh(
        &mut self,
        snapshots: &[ScopeMeasurementSnapshot],
    ) {
        let Some(bundle) = self.waveform_bundle_refresh_preview.clone() else {
            return;
        };
        let bundle_dir = PathBuf::from(&bundle);
        if !is_scope_report_bundle_path(&bundle_dir) {
            self.waveform_bundle_refresh_preview = None;
            self.status = format!(
                "Refusing to refresh non-bundle path {}.",
                bundle_dir.display()
            );
            return;
        }
        let Some(files) = self.prepare_scope_report_bundle_files(snapshots) else {
            return;
        };
        match write_scope_report_bundle_files(&bundle_dir, &files) {
            Ok(()) => {
                self.waveform_bundle_refresh_preview = None;
                self.push_recent_scope_report_bundle(bundle);
                self.status = format!(
                    "Refreshed scope report bundle artifacts in {}.",
                    bundle_dir.display()
                );
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to refresh scope report bundle {}: {error}",
                    bundle_dir.display()
                ));
            }
        }
    }

    fn open_scope_report_bundle(&mut self, bundle: &str) {
        let path = Path::new(bundle);
        if !path.exists() {
            self.status = format!("Scope report bundle no longer exists: {}.", path.display());
            return;
        }
        match open_path_in_file_manager(path) {
            Ok(()) => {
                self.status = format!("Opened scope report bundle folder {}.", path.display());
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to open scope report bundle folder {}: {error}",
                    path.display()
                ));
            }
        }
    }

    fn open_scope_report_bundle_index(&mut self, bundle: &str) {
        let path = Path::new(bundle);
        if !path.exists() {
            self.status = format!("Scope report bundle no longer exists: {}.", path.display());
            return;
        }
        let index_path = scope_report_bundle_index_path(path);
        if !index_path.exists() {
            self.status = format!(
                "Scope report bundle index no longer exists: {}.",
                index_path.display()
            );
            return;
        }
        match open_path_in_file_manager(&index_path) {
            Ok(()) => {
                self.status = format!("Opened scope report bundle index {}.", index_path.display());
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to open scope report bundle index {}: {error}",
                    index_path.display()
                ));
            }
        }
    }

    fn open_scope_report_bundle_artifact(&mut self, bundle: &str, artifact: &str, label: &str) {
        let path = Path::new(bundle);
        if !path.exists() {
            self.status = format!("Scope report bundle no longer exists: {}.", path.display());
            return;
        }
        let artifact_path = path.join(artifact);
        if !artifact_path.exists() {
            self.status = format!(
                "Scope report bundle {label} file no longer exists: {}.",
                artifact_path.display()
            );
            return;
        }
        match open_path_in_file_manager(&artifact_path) {
            Ok(()) => {
                self.status = format!(
                    "Opened scope report bundle {label} file {}.",
                    artifact_path.display()
                );
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to open scope report bundle {label} file {}: {error}",
                    artifact_path.display()
                ));
            }
        }
    }

    fn copy_scope_report_bundle_path(
        &mut self,
        ui: &egui::Ui,
        bundle: &str,
        artifact: Option<&str>,
        label: &str,
    ) {
        let path = Path::new(bundle);
        if !path.exists() {
            self.status = format!("Scope report bundle no longer exists: {}.", path.display());
            return;
        }
        let copy_path = artifact.map_or_else(|| path.to_path_buf(), |artifact| path.join(artifact));
        if !copy_path.exists() {
            self.status = format!(
                "Scope report bundle {label} path no longer exists: {}.",
                copy_path.display()
            );
            return;
        }
        let copied = copy_path.to_string_lossy().into_owned();
        ui.ctx().copy_text(copied.clone());
        self.status = format!("Copied scope report bundle {label} path {copied}.");
    }

    fn preview_old_scope_report_bundles(&mut self) {
        match old_scope_report_bundle_dirs(
            &output_bundle_base_dir(&self.output_dir),
            MAX_RECENT_SCOPE_BUNDLES,
        ) {
            Ok(paths) if paths.is_empty() => {
                self.waveform_bundle_cleanup_preview.clear();
                self.status = "No old scope report bundle folders to clean.".to_string();
            }
            Ok(paths) => {
                let count = paths.len();
                self.waveform_bundle_cleanup_preview = paths
                    .into_iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect();
                self.status =
                    format!("Previewing {count} old scope report bundle folder(s) for cleanup.");
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to preview old scope report bundle folders: {error}"
                ));
            }
        }
    }

    fn confirm_scope_report_bundle_cleanup(&mut self) {
        match remove_scope_report_bundle_dirs(&self.waveform_bundle_cleanup_preview) {
            Ok(removed) => {
                self.waveform_bundle_cleanup_preview.clear();
                self.waveform_recent_report_bundles
                    .retain(|bundle| Path::new(bundle).exists());
                if self
                    .waveform_bundle_refresh_preview
                    .as_deref()
                    .is_some_and(|bundle| !Path::new(bundle).exists())
                {
                    self.waveform_bundle_refresh_preview = None;
                }
                if self
                    .waveform_bundle_integrity_details
                    .as_deref()
                    .is_some_and(|bundle| !Path::new(bundle).exists())
                {
                    self.waveform_bundle_integrity_details = None;
                }
                self.status = format!("Removed {removed} old scope report bundle folder(s).");
            }
            Err(error) => {
                self.record_error(anyhow::anyhow!(
                    "failed to clean old scope report bundle folders: {error}"
                ));
            }
        }
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

fn display_path_tail(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string())
}

fn open_path_in_file_manager(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(path);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };
    command.spawn().map(|_| ())
}

fn write_scope_report_bundle_files(
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

#[cfg(test)]
pub(super) fn cleanup_old_scope_report_bundle_dirs(
    base_dir: &Path,
    keep_count: usize,
) -> std::io::Result<usize> {
    let bundles = old_scope_report_bundle_dirs(base_dir, keep_count)?;
    let mut removed = 0usize;
    for bundle in bundles {
        fs::remove_dir_all(&bundle)?;
        removed += 1;
    }
    Ok(removed)
}

pub(super) fn old_scope_report_bundle_dirs(
    base_dir: &Path,
    keep_count: usize,
) -> std::io::Result<Vec<PathBuf>> {
    let mut bundles = scope_report_bundle_dirs(base_dir)?;
    bundles.sort_by(|left, right| {
        report_bundle_sort_key(right)
            .cmp(&report_bundle_sort_key(left))
            .then_with(|| right.cmp(left))
    });
    Ok(bundles.into_iter().skip(keep_count).collect())
}

fn remove_scope_report_bundle_dirs(bundles: &[String]) -> std::io::Result<usize> {
    let mut removed = 0usize;
    for bundle in bundles {
        let path = Path::new(bundle);
        if is_scope_report_bundle_dir(path) {
            fs::remove_dir_all(path)?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn scope_report_bundle_dirs(base_dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut bundles = Vec::new();
    if !base_dir.exists() {
        return Ok(bundles);
    }
    for entry in fs::read_dir(base_dir)? {
        let path = entry?.path();
        if is_scope_report_bundle_dir(&path) {
            bundles.push(path);
        }
    }
    Ok(bundles)
}

fn is_scope_report_bundle_dir(path: &Path) -> bool {
    path.is_dir() && is_scope_report_bundle_path(path)
}

fn is_scope_report_bundle_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("scope_report_bundle_"))
}

fn report_bundle_sort_key(path: &Path) -> u128 {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("scope_report_bundle_"))
        .map(|suffix| suffix.chars().take_while(|ch| ch.is_ascii_digit()))
        .and_then(|digits| digits.collect::<String>().parse::<u128>().ok())
        .unwrap_or_default()
}

fn current_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
