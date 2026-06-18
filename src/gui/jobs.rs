use super::project::{optional_path, sanitized_project_name};
use super::simulation::load_report_waveforms;
use super::{CircuitCiApp, Stage, suggest_from_gui, validate_from_gui};
use crate::reports::ValidationReport;
use anyhow::{Context, Result};
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

pub(super) struct BackgroundGuiJob {
    receiver: Receiver<BackgroundJobResult>,
    started_at: Instant,
    label: &'static str,
    canceled: bool,
}

enum BackgroundJobResult {
    Validation(Box<ValidationJobResult>),
    Suggestions(Box<SuggestionJobResult>),
    ImportProject(Box<ImportProjectJobResult>),
}

struct ValidationJobResult {
    project_path: PathBuf,
    profile: String,
    output_dir: PathBuf,
    result: Result<ValidationJobOutput>,
}

struct ValidationJobOutput {
    report: ValidationReport,
    markdown: String,
}

struct SuggestionJobResult {
    project_path: PathBuf,
    profile: String,
    result: Result<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportProjectKind {
    KiCadSchematic,
    KiCadPcb,
    SpiceDeck,
}

struct ImportProjectJobResult {
    kind: ImportProjectKind,
    prior_project_path: String,
    state_key: String,
    output_project_path: PathBuf,
    import_pcb_project_path: Option<String>,
    next_stage: Option<Stage>,
    success_status: &'static str,
    success_diagnostic: String,
    result: Result<()>,
}

impl CircuitCiApp {
    pub(super) fn validate_project(&mut self) {
        let project_path = PathBuf::from(self.project_path.clone());
        let profile = self.profile.clone();
        let output_dir = PathBuf::from(self.output_dir.clone());
        self.start_background_job("validation", move |sender| {
            let thread_project_path = project_path.clone();
            let thread_profile = profile.clone();
            let thread_output_dir = output_dir.clone();
            thread::spawn(move || {
                let result =
                    validate_from_gui(&thread_project_path, &thread_profile, &thread_output_dir)
                        .map(|(report, markdown)| ValidationJobOutput { report, markdown });
                let _ = sender.send(BackgroundJobResult::Validation(Box::new(
                    ValidationJobResult {
                        project_path: thread_project_path,
                        profile: thread_profile,
                        output_dir: thread_output_dir,
                        result,
                    },
                )));
            });
        });
    }

    pub(super) fn suggest_scenarios(&mut self) {
        let project_path = PathBuf::from(self.project_path.clone());
        let profile = self.profile.clone();
        self.start_background_job("scenario suggestions", move |sender| {
            let thread_project_path = project_path.clone();
            let thread_profile = profile.clone();
            thread::spawn(move || {
                let result = suggest_from_gui(&thread_project_path, &thread_profile);
                let _ = sender.send(BackgroundJobResult::Suggestions(Box::new(
                    SuggestionJobResult {
                        project_path: thread_project_path,
                        profile: thread_profile,
                        result,
                    },
                )));
            });
        });
    }

    pub(super) fn import_kicad_schematic(&mut self) {
        let schematic = Path::new(&self.import_schematic_path).to_path_buf();
        let output = Path::new(&self.import_output_path).to_path_buf();
        let mapping = optional_path(&self.import_mapping_path);
        let name = if self.import_project_name.trim().is_empty() {
            sanitized_project_name(&schematic, "imported_kicad_project")
        } else {
            self.import_project_name.trim().to_string()
        };
        let default_model = self.import_default_model.trim().to_string();
        let state_key = self.kicad_schematic_import_key();
        let prior_project_path = self.project_path.clone();
        self.start_background_job("KiCad schematic import", move |sender| {
            thread::spawn(move || {
                let options = crate::importers::kicad::KicadImportOptions {
                    input: schematic,
                    output: output.clone(),
                    name,
                    default_model,
                    mapping,
                };
                let result = crate::importers::kicad_sch::import_kicad_schematic(&options);
                let output_project_path = output.clone();
                let _ = sender.send(BackgroundJobResult::ImportProject(Box::new(
                    ImportProjectJobResult {
                        kind: ImportProjectKind::KiCadSchematic,
                        prior_project_path,
                        state_key,
                        output_project_path,
                        import_pcb_project_path: Some(output.to_string_lossy().into_owned()),
                        next_stage: None,
                        success_status: "KiCad schematic imported.",
                        success_diagnostic: "KiCad schematic imported to Board IR.".to_string(),
                        result,
                    },
                )));
            });
        });
    }

    pub(super) fn import_kicad_pcb(&mut self) {
        let input = Path::new(&self.import_pcb_path).to_path_buf();
        let project = Path::new(&self.import_pcb_project_path).to_path_buf();
        let output = Path::new(&self.import_pcb_output_path).to_path_buf();
        let state_key = self.kicad_pcb_import_key();
        let prior_project_path = self.project_path.clone();
        self.start_background_job("KiCad PCB import", move |sender| {
            thread::spawn(move || {
                let options = crate::importers::kicad_pcb::KicadPcbPlacementImportOptions {
                    input,
                    project,
                    output: output.clone(),
                };
                let result =
                    crate::importers::kicad_pcb::import_kicad_pcb_placements(&options).map(
                        |summary| {
                            format!(
                                "KiCad PCB imported: {} placements, {} pads, {} route segments, {} vias.",
                                summary.placements,
                                summary.pads,
                                summary.route_segments,
                                summary.route_vias
                            )
                        },
                    );
                let (result, success_diagnostic) = match result {
                    Ok(diagnostic) => (Ok(()), diagnostic),
                    Err(error) => (Err(error), String::new()),
                };
                let _ = sender.send(BackgroundJobResult::ImportProject(Box::new(ImportProjectJobResult {
                    kind: ImportProjectKind::KiCadPcb,
                    prior_project_path,
                    state_key,
                    output_project_path: output,
                    import_pcb_project_path: None,
                    next_stage: None,
                    success_status: "KiCad PCB evidence imported.",
                    success_diagnostic,
                    result,
                })));
            });
        });
    }

    pub(super) fn import_spice_deck(&mut self) {
        let deck = Path::new(&self.import_spice_deck_path).to_path_buf();
        let output = Path::new(&self.import_spice_output_path).to_path_buf();
        let name = if self.import_spice_project_name.trim().is_empty() {
            sanitized_project_name(&deck, "imported_spice_project")
        } else {
            self.import_spice_project_name.trim().to_string()
        };
        let backend = self.import_spice_backend.trim().to_string();
        let stop_time_us = self.import_spice_stop_time_us;
        let max_step_us = self.import_spice_max_step_us;
        let state_key = self.spice_import_key();
        let prior_project_path = self.project_path.clone();
        self.start_background_job("SPICE deck import", move |sender| {
            thread::spawn(move || {
                let options = crate::importers::spice::SpiceImportOptions {
                    input: deck.clone(),
                    output: output.clone(),
                    name,
                    backend,
                    stop_time_us,
                    max_step_us,
                };
                let result = crate::importers::spice::import_spice(&options);
                let _ = sender.send(BackgroundJobResult::ImportProject(Box::new(
                    ImportProjectJobResult {
                        kind: ImportProjectKind::SpiceDeck,
                        prior_project_path,
                        state_key,
                        output_project_path: output,
                        import_pcb_project_path: None,
                        next_stage: Some(Stage::Simulation),
                        success_status: "SPICE deck imported.",
                        success_diagnostic: format!(
                            "SPICE deck imported to Board IR from {}.",
                            deck.display()
                        ),
                        result,
                    },
                )));
            });
        });
    }

    pub(super) fn cancel_background_job(&mut self) {
        let Some(job) = &mut self.background_job else {
            return;
        };
        job.canceled = true;
        self.status = "Cancel requested; background result will be ignored.".to_string();
        self.push_diagnostic(
            "Background jobs cannot interrupt an in-flight engine/importer call yet; the result will be ignored.",
        );
    }

    pub(super) fn background_job_elapsed_secs(&self) -> Option<f32> {
        self.background_job
            .as_ref()
            .map(|job| job.started_at.elapsed().as_secs_f32())
    }

    pub(super) fn background_job_label(&self) -> Option<&'static str> {
        self.background_job.as_ref().map(|job| job.label)
    }

    pub(super) fn background_job_cancel_requested(&self) -> bool {
        self.background_job.as_ref().is_some_and(|job| job.canceled)
    }

    pub(super) fn poll_background_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.background_job else {
            return;
        };
        match job.receiver.try_recv() {
            Ok(result) => {
                let canceled = self.background_job.as_ref().is_some_and(|job| job.canceled);
                self.background_job = None;
                if canceled {
                    self.status = "Background job canceled.".to_string();
                    self.push_diagnostic("Ignored canceled background job result.");
                    return;
                }
                self.apply_background_job_result(result);
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
            Err(TryRecvError::Disconnected) => {
                self.background_job = None;
                self.record_error(anyhow::anyhow!(
                    "Background worker exited before returning a result."
                ));
            }
        }
    }

    fn start_background_job<F>(&mut self, label: &'static str, spawn: F)
    where
        F: FnOnce(mpsc::Sender<BackgroundJobResult>),
    {
        if self.background_job.is_some() {
            self.status = "Background job already running.".to_string();
            self.push_diagnostic("A background GUI job is already running.");
            return;
        }

        let (sender, receiver) = mpsc::channel();
        spawn(sender);
        self.background_job = Some(BackgroundGuiJob {
            receiver,
            started_at: Instant::now(),
            label,
            canceled: false,
        });
        self.status = format!("Background {label} running.");
        self.push_diagnostic(&format!("Background {label} started."));
    }

    fn apply_background_job_result(&mut self, result: BackgroundJobResult) {
        match result {
            BackgroundJobResult::Validation(result) => self.apply_validation_result(*result),
            BackgroundJobResult::Suggestions(result) => self.apply_suggestion_result(*result),
            BackgroundJobResult::ImportProject(result) => self.apply_import_project_result(*result),
        }
    }

    fn apply_validation_result(&mut self, result: ValidationJobResult) {
        if PathBuf::from(self.project_path.clone()) != result.project_path
            || self.profile != result.profile
            || PathBuf::from(self.output_dir.clone()) != result.output_dir
        {
            self.status = "Ignored stale background validation result.".to_string();
            self.push_diagnostic(
                "Ignored a background validation result because project/profile/output changed.",
            );
            return;
        }
        match result.result.with_context(|| {
            format!(
                "Background validation failed for {}.",
                result.project_path.display()
            )
        }) {
            Ok(output) => {
                let waveforms = load_report_waveforms(&output.report);
                let waveform_count = waveforms.len();
                self.status = format!("Validation {}", output.report.result);
                self.report_markdown = output.markdown;
                self.report = Some(output.report);
                self.waveforms = waveforms;
                self.selected_waveform = 0;
                self.selected_probe = 0;
                self.waveform_cursor_a_us = 0.0;
                self.waveform_cursor_b_us = 0.0;
                self.waveform_playing = false;
                self.stage = if waveform_count == 0 {
                    Stage::Reports
                } else {
                    Stage::Simulation
                };
                self.push_diagnostic(&format!(
                    "Background validation report written; loaded {waveform_count} waveform view(s)."
                ));
                self.load_project_summary_unchecked();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_suggestion_result(&mut self, result: SuggestionJobResult) {
        if PathBuf::from(self.project_path.clone()) != result.project_path
            || self.profile != result.profile
        {
            self.status = "Ignored stale scenario suggestion result.".to_string();
            self.push_diagnostic("Ignored scenario suggestions because project/profile changed.");
            return;
        }
        match result.result {
            Ok(yaml) => {
                self.status = "Scenario suggestions generated.".to_string();
                self.suggestions_yaml = yaml;
                self.stage = Stage::Library;
                self.push_diagnostic("Scenario suggestion YAML updated.");
                self.load_project_summary_unchecked();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_import_project_result(&mut self, result: ImportProjectJobResult) {
        if self.project_path != result.prior_project_path
            || self.import_state_key(result.kind) != result.state_key
        {
            self.status = "Ignored stale import result.".to_string();
            self.push_diagnostic(
                "Ignored an import result because project or import settings changed.",
            );
            return;
        }
        match result.result {
            Ok(()) => {
                self.project_path = result.output_project_path.to_string_lossy().into_owned();
                if let Some(path) = result.import_pcb_project_path {
                    self.import_pcb_project_path = path;
                }
                self.status = result.success_status.to_string();
                self.push_diagnostic(&result.success_diagnostic);
                self.load_project_summary_unchecked();
                if let Some(stage) = result.next_stage {
                    self.stage = stage;
                }
            }
            Err(error) => self.record_error(error),
        }
    }

    fn import_state_key(&self, kind: ImportProjectKind) -> String {
        match kind {
            ImportProjectKind::KiCadSchematic => self.kicad_schematic_import_key(),
            ImportProjectKind::KiCadPcb => self.kicad_pcb_import_key(),
            ImportProjectKind::SpiceDeck => self.spice_import_key(),
        }
    }

    fn kicad_schematic_import_key(&self) -> String {
        [
            self.import_schematic_path.trim(),
            self.import_mapping_path.trim(),
            self.import_output_path.trim(),
            self.import_project_name.trim(),
            self.import_default_model.trim(),
        ]
        .join("\u{1f}")
    }

    fn kicad_pcb_import_key(&self) -> String {
        [
            self.import_pcb_path.trim(),
            self.import_pcb_project_path.trim(),
            self.import_pcb_output_path.trim(),
        ]
        .join("\u{1f}")
    }

    fn spice_import_key(&self) -> String {
        [
            self.import_spice_deck_path.trim().to_string(),
            self.import_spice_output_path.trim().to_string(),
            self.import_spice_project_name.trim().to_string(),
            self.import_spice_backend.trim().to_string(),
            self.import_spice_stop_time_us.to_string(),
            self.import_spice_max_step_us.to_string(),
        ]
        .join("\u{1f}")
    }
}

#[cfg(test)]
mod tests {
    use super::{CircuitCiApp, ImportProjectJobResult, ImportProjectKind, ValidationJobResult};
    use std::path::PathBuf;

    #[test]
    fn cancel_without_job_is_noop() {
        let mut app = CircuitCiApp::default();
        app.cancel_background_job();
        assert!(app.background_job_elapsed_secs().is_none());
    }

    #[test]
    fn stale_validation_result_is_ignored() {
        let mut app = CircuitCiApp::default();
        app.project_path = "new.project.yaml".to_string();
        app.profile = "default".to_string();
        app.output_dir = "out/new".to_string();

        app.apply_validation_result(ValidationJobResult {
            project_path: PathBuf::from("old.project.yaml"),
            profile: "default".to_string(),
            output_dir: PathBuf::from("out/new"),
            result: Err(anyhow::anyhow!("should not be observed")),
        });

        assert_eq!(app.status, "Ignored stale background validation result.");
        assert!(app.report.is_none());
    }

    #[test]
    fn stale_import_result_is_ignored() {
        let mut app = CircuitCiApp::default();
        app.project_path = "active.project.yaml".to_string();
        app.import_output_path = "out/new.project.yaml".to_string();

        app.apply_import_project_result(ImportProjectJobResult {
            kind: ImportProjectKind::KiCadSchematic,
            prior_project_path: "different.project.yaml".to_string(),
            state_key: app.kicad_schematic_import_key(),
            output_project_path: PathBuf::from("out/new.project.yaml"),
            import_pcb_project_path: Some("out/new.project.yaml".to_string()),
            next_stage: None,
            success_status: "should not apply",
            success_diagnostic: "should not apply".to_string(),
            result: Ok(()),
        });

        assert_eq!(app.status, "Ignored stale import result.");
        assert_eq!(app.project_path, "active.project.yaml");
    }
}
