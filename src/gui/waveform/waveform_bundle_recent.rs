use super::waveform_bundle_integrity::{
    SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_CSV, SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_MARKDOWN,
    optional_size_label, scope_report_bundle_artifact_status,
    scope_report_bundle_integrity_details, scope_report_bundle_integrity_details_csv,
    scope_report_bundle_integrity_details_markdown, short_optional_sha,
};
use super::waveform_bundles::{
    output_bundle_base_dir, scope_report_bundle_index_path, write_scope_report_bundle_files,
};
use crate::gui::{CircuitCiApp, ScopeMeasurementSnapshot};
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_RECENT_SCOPE_BUNDLES: usize = 5;

impl CircuitCiApp {
    pub(super) fn scope_recent_report_bundles_ui(
        &mut self,
        ui: &mut egui::Ui,
        snapshots: &[ScopeMeasurementSnapshot],
    ) {
        let pruned = self.prune_missing_scope_report_bundles();
        if pruned > 0 {
            self.status = format!("Pruned {pruned} stale scope report bundle entrie(s).");
        }
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
            self.scope_older_report_bundles_ui(ui);
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

    fn scope_older_report_bundles_ui(&mut self, ui: &mut egui::Ui) {
        if self.waveform_recent_report_bundles.len() <= 1 {
            return;
        }
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

    pub(super) fn push_recent_scope_report_bundle(&mut self, bundle: String) {
        self.waveform_recent_report_bundles
            .retain(|existing| existing != &bundle);
        self.waveform_recent_report_bundles.insert(0, bundle);
        self.waveform_recent_report_bundles
            .truncate(MAX_RECENT_SCOPE_BUNDLES);
    }

    pub(super) fn prune_missing_scope_report_bundles(&mut self) -> usize {
        let before = self.waveform_recent_report_bundles.len();
        self.waveform_recent_report_bundles
            .retain(|bundle| Path::new(bundle).exists());
        self.waveform_bundle_cleanup_preview
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
        before.saturating_sub(self.waveform_recent_report_bundles.len())
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
