use super::project::{optional_path, sanitized_project_name};
use super::waveform::{WaveformView, load_report_waveforms_with_progress_and_cancel};
use super::{CircuitCiApp, Stage, validate_from_gui};
use crate::cancellation;
use crate::reports::ValidationReport;
use anyhow::Result;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const BACKGROUND_JOB_HISTORY_LIMIT: usize = 16;
const BACKGROUND_JOB_EVENT_LIMIT: usize = 12;

pub(super) struct BackgroundGuiJob {
    receiver: Receiver<BackgroundJobMessage>,
    started_at: Instant,
    label: &'static str,
    target: String,
    canceled: bool,
    cancel_token: Arc<AtomicBool>,
    events: Vec<BackgroundJobEvent>,
}

pub(super) struct BackgroundJobRecord {
    pub(super) label: String,
    pub(super) outcome: String,
    pub(super) elapsed_secs: f32,
    pub(super) detail: String,
    pub(super) output_path: Option<String>,
}

pub(super) struct BackgroundJobEvent {
    pub(super) stage: String,
    pub(super) detail: String,
    pub(super) elapsed_secs: f32,
}

enum BackgroundJobMessage {
    Progress(BackgroundJobProgress),
    Finished(BackgroundJobResult),
}

struct BackgroundJobProgress {
    stage: &'static str,
    detail: String,
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
    waveforms: Vec<WaveformView>,
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
        let target = format!("{} -> {}", project_path.display(), output_dir.display());
        self.start_background_job("validation", target, move |sender, cancel_token| {
            let thread_project_path = project_path.clone();
            let thread_profile = profile.clone();
            let thread_output_dir = output_dir.clone();
            thread::spawn(move || {
                send_background_progress(
                    &sender,
                    "Preparing validation",
                    format!(
                        "{} -> {}",
                        thread_project_path.display(),
                        thread_output_dir.display()
                    ),
                );
                let result = validate_from_gui(
                    &thread_project_path,
                    &thread_profile,
                    &thread_output_dir,
                    |stage, detail| send_background_progress(&sender, stage, detail),
                    || cancel_token.load(Ordering::Relaxed),
                )
                .and_then(|(report, markdown)| {
                    let waveforms = load_report_waveforms_with_progress_and_cancel(
                        &report,
                        |stage, detail| send_background_progress(&sender, stage, detail),
                        || cancel_token.load(Ordering::Relaxed),
                    )?;
                    Ok(ValidationJobOutput {
                        report,
                        markdown,
                        waveforms,
                    })
                });
                send_background_progress(
                    &sender,
                    "Validation finished",
                    "Applying report and waveform artifacts.".to_string(),
                );
                let _ = sender.send(BackgroundJobMessage::Finished(
                    BackgroundJobResult::Validation(Box::new(ValidationJobResult {
                        project_path: thread_project_path,
                        profile: thread_profile,
                        output_dir: thread_output_dir,
                        result,
                    })),
                ));
            });
        });
    }

    pub(super) fn suggest_scenarios(&mut self) {
        let project_path = PathBuf::from(self.project_path.clone());
        let profile = self.profile.clone();
        let target = format!("{} ({profile})", project_path.display());
        self.start_background_job(
            "scenario suggestions",
            target,
            move |sender, cancel_token| {
                let thread_project_path = project_path.clone();
                let thread_profile = profile.clone();
                thread::spawn(move || {
                    send_background_progress(
                        &sender,
                        "Generating suggestions",
                        format!("{} ({thread_profile})", thread_project_path.display()),
                    );
                    let result = super::suggest_from_gui_with_cancel(
                        &thread_project_path,
                        &thread_profile,
                        || cancel_token.load(Ordering::Relaxed),
                    );
                    send_background_progress(
                        &sender,
                        "Suggestions finished",
                        "Applying generated scenario YAML.".to_string(),
                    );
                    let _ = sender.send(BackgroundJobMessage::Finished(
                        BackgroundJobResult::Suggestions(Box::new(SuggestionJobResult {
                            project_path: thread_project_path,
                            profile: thread_profile,
                            result,
                        })),
                    ));
                });
            },
        );
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
        let target = format!("{} -> {}", schematic.display(), output.display());
        self.start_background_job(
            "KiCad schematic import",
            target,
            move |sender, cancel_token| {
                thread::spawn(move || {
                    let options = crate::importers::kicad::KicadImportOptions {
                        input: schematic,
                        output: output.clone(),
                        name,
                        default_model,
                        mapping,
                    };
                    let result =
                        crate::importers::kicad_sch::import_kicad_schematic_with_progress_and_cancel(
                            &options,
                            |stage, detail| send_background_progress(&sender, stage, detail),
                            || cancel_token.load(Ordering::Relaxed),
                        );
                    let output_project_path = output.clone();
                    send_background_progress(
                        &sender,
                        "Schematic import finished",
                        format!("Writing {}.", output_project_path.display()),
                    );
                    let _ = sender.send(BackgroundJobMessage::Finished(
                        BackgroundJobResult::ImportProject(Box::new(ImportProjectJobResult {
                            kind: ImportProjectKind::KiCadSchematic,
                            prior_project_path,
                            state_key,
                            output_project_path,
                            import_pcb_project_path: Some(output.to_string_lossy().into_owned()),
                            next_stage: None,
                            success_status: "KiCad schematic imported.",
                            success_diagnostic: "KiCad schematic imported to Board IR.".to_string(),
                            result,
                        })),
                    ));
                });
            },
        );
    }

    pub(super) fn import_kicad_pcb(&mut self) {
        let input = Path::new(&self.import_pcb_path).to_path_buf();
        let project = Path::new(&self.import_pcb_project_path).to_path_buf();
        let output = Path::new(&self.import_pcb_output_path).to_path_buf();
        let state_key = self.kicad_pcb_import_key();
        let prior_project_path = self.project_path.clone();
        let target = format!(
            "{} + {} -> {}",
            input.display(),
            project.display(),
            output.display()
        );
        self.start_background_job("KiCad PCB import", target, move |sender, cancel_token| {
            thread::spawn(move || {
                let options = crate::importers::kicad_pcb::KicadPcbPlacementImportOptions {
                    input,
                    project,
                    output: output.clone(),
                };
                let result = crate::importers::kicad_pcb::import_kicad_pcb_placements_with_progress_and_cancel(
                    &options,
                    |stage, detail| send_background_progress(&sender, stage, detail),
                    || cancel_token.load(Ordering::Relaxed),
                )
                .map(|summary| {
                    format!(
                                "KiCad PCB imported: {} placements, {} pads, {} route segments, {} vias.",
                                summary.placements,
                                summary.pads,
                                summary.route_segments,
                                summary.route_vias
                            )
                });
                let (result, success_diagnostic) = match result {
                    Ok(diagnostic) => (Ok(()), diagnostic),
                    Err(error) => (Err(error), String::new()),
                };
                send_background_progress(
                    &sender,
                    "PCB import finished",
                    "Applying placement, pad, route, and board-outline evidence.".to_string(),
                );
                let _ = sender.send(BackgroundJobMessage::Finished(BackgroundJobResult::ImportProject(Box::new(ImportProjectJobResult {
                    kind: ImportProjectKind::KiCadPcb,
                    prior_project_path,
                    state_key,
                    output_project_path: output,
                    import_pcb_project_path: None,
                    next_stage: None,
                    success_status: "KiCad PCB evidence imported.",
                    success_diagnostic,
                    result,
                }))));
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
        let target = format!("{} -> {}", deck.display(), output.display());
        self.start_background_job("SPICE deck import", target, move |sender, cancel_token| {
            thread::spawn(move || {
                let options = crate::importers::spice::SpiceImportOptions {
                    input: deck.clone(),
                    output: output.clone(),
                    name,
                    backend,
                    stop_time_us,
                    max_step_us,
                };
                let result = crate::importers::spice::import_spice_with_progress_and_cancel(
                    &options,
                    |stage, detail| send_background_progress(&sender, stage, detail),
                    || cancel_token.load(Ordering::Relaxed),
                );
                send_background_progress(
                    &sender,
                    "SPICE import finished",
                    format!("Writing {}.", output.display()),
                );
                let _ = sender.send(BackgroundJobMessage::Finished(
                    BackgroundJobResult::ImportProject(Box::new(ImportProjectJobResult {
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
                    })),
                ));
            });
        });
    }

    pub(super) fn cancel_background_job(&mut self) {
        let Some(job) = &mut self.background_job else {
            return;
        };
        job.canceled = true;
        job.cancel_token.store(true, Ordering::Relaxed);
        self.status = "Cancel requested; worker will stop where supported.".to_string();
        self.push_diagnostic(
            "Cancel requested. External ngspice validation runs are terminated where possible; importer and embedded calls still finish before their result is ignored.",
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

    pub(super) fn background_job_target(&self) -> Option<&str> {
        self.background_job.as_ref().map(|job| job.target.as_str())
    }

    pub(super) fn background_job_cancel_requested(&self) -> bool {
        self.background_job.as_ref().is_some_and(|job| job.canceled)
    }

    pub(super) fn background_job_events(&self) -> Option<&[BackgroundJobEvent]> {
        self.background_job
            .as_ref()
            .map(|job| job.events.as_slice())
    }

    pub(super) fn poll_background_job(&mut self, ctx: &egui::Context) {
        loop {
            let Some(message) = self
                .background_job
                .as_ref()
                .map(|job| job.receiver.try_recv())
            else {
                return;
            };
            match message {
                Ok(BackgroundJobMessage::Progress(progress)) => {
                    self.push_background_job_event(progress);
                }
                Ok(BackgroundJobMessage::Finished(result)) => {
                    let Some(job) = &self.background_job else {
                        return;
                    };
                    let label = job.label;
                    let target = job.target.clone();
                    let elapsed_secs = job.started_at.elapsed().as_secs_f32();
                    let canceled = job.canceled;
                    self.background_job = None;
                    if canceled {
                        self.status = "Background job canceled.".to_string();
                        let detail = canceled_job_detail(&result).unwrap_or_else(|| {
                            format!("Canceled {label} for {target}; ignored worker result.")
                        });
                        let output_path = background_job_output_path(&result);
                        self.push_diagnostic(&detail);
                        self.push_canceled_background_job_record(
                            label,
                            elapsed_secs,
                            detail,
                            output_path,
                        );
                        return;
                    }
                    self.apply_background_job_result(result, elapsed_secs);
                    return;
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100));
                    return;
                }
                Err(TryRecvError::Disconnected) => {
                    let Some(job) = &self.background_job else {
                        return;
                    };
                    let label = job.label;
                    let target = job.target.clone();
                    let elapsed_secs = job.started_at.elapsed().as_secs_f32();
                    self.background_job = None;
                    self.push_background_job_record(
                        label,
                        "failed",
                        elapsed_secs,
                        format!("{target}: worker exited before returning a result."),
                        None,
                    );
                    self.record_error(anyhow::anyhow!(
                        "Background worker exited before returning a result."
                    ));
                    return;
                }
            }
        }
    }

    fn start_background_job<F>(&mut self, label: &'static str, target: String, spawn: F)
    where
        F: FnOnce(mpsc::Sender<BackgroundJobMessage>, Arc<AtomicBool>),
    {
        if self.background_job.is_some() {
            self.status = "Background job already running.".to_string();
            self.push_diagnostic("A background GUI job is already running.");
            return;
        }

        let (sender, receiver) = mpsc::channel();
        let cancel_token = Arc::new(AtomicBool::new(false));
        spawn(sender, Arc::clone(&cancel_token));
        self.background_job = Some(BackgroundGuiJob {
            receiver,
            started_at: Instant::now(),
            label,
            target,
            canceled: false,
            cancel_token,
            events: Vec::new(),
        });
        self.status = format!("Background {label} running.");
        self.push_diagnostic(&format!("Background {label} started."));
    }

    fn apply_background_job_result(&mut self, result: BackgroundJobResult, elapsed_secs: f32) {
        match result {
            BackgroundJobResult::Validation(result) => {
                self.apply_validation_result(*result, elapsed_secs)
            }
            BackgroundJobResult::Suggestions(result) => {
                self.apply_suggestion_result(*result, elapsed_secs)
            }
            BackgroundJobResult::ImportProject(result) => {
                self.apply_import_project_result(*result, elapsed_secs)
            }
        }
    }

    fn apply_validation_result(&mut self, result: ValidationJobResult, elapsed_secs: f32) {
        if PathBuf::from(self.project_path.clone()) != result.project_path
            || self.profile != result.profile
            || PathBuf::from(self.output_dir.clone()) != result.output_dir
        {
            self.status = "Ignored stale background validation result.".to_string();
            self.push_diagnostic(
                "Ignored a background validation result because project/profile/output changed.",
            );
            self.push_background_job_record(
                "validation",
                "stale",
                elapsed_secs,
                format!(
                    "Ignored result for {} because project/profile/output changed.",
                    result.project_path.display()
                ),
                Some(result.output_dir.to_string_lossy().into_owned()),
            );
            return;
        }
        match result.result {
            Ok(output) => {
                let waveforms = output.waveforms;
                let waveform_count = waveforms.len();
                self.status = format!("Validation {}", output.report.result);
                self.report_markdown = output.markdown;
                self.report = Some(output.report);
                self.waveforms = waveforms;
                self.waveform_plot_cache.clear();
                self.waveform_pinned_traces.clear();
                self.waveform_trace_presets.clear();
                self.waveform_trace_styles.clear();
                self.waveform_measurement_snapshots.clear();
                self.selected_waveform = 0;
                self.selected_probe = 0;
                self.apply_pending_scope_probe_focus();
                self.waveform_cursor_a_us = 0.0;
                self.waveform_cursor_b_us = 0.0;
                self.waveform_playing = false;
                self.waveform_window_start_us = None;
                self.waveform_window_end_us = None;
                self.waveform_value_min = None;
                self.waveform_value_max = None;
                self.clear_waveform_view_history();
                self.waveform_trigger_threshold = 0.0;
                self.stage = if waveform_count == 0 {
                    Stage::Reports
                } else {
                    Stage::Simulation
                };
                self.push_diagnostic(&format!(
                    "Background validation report written; loaded {waveform_count} waveform view(s)."
                ));
                self.push_background_job_record(
                    "validation",
                    "completed",
                    elapsed_secs,
                    format!(
                        "Validation {}; loaded {waveform_count} waveform view(s).",
                        self.status.trim_start_matches("Validation ")
                    ),
                    Some(result.output_dir.to_string_lossy().into_owned()),
                );
                self.load_project_summary_unchecked();
            }
            Err(error) => {
                if cancellation::is_canceled(&error) {
                    let detail = format!("{error:#}");
                    self.status = "Background job canceled.".to_string();
                    self.push_diagnostic(&detail);
                    self.push_canceled_background_job_record(
                        "validation",
                        elapsed_secs,
                        detail,
                        Some(result.output_dir.to_string_lossy().into_owned()),
                    );
                    return;
                }
                let detail = format!(
                    "Background validation failed for {}.\n{error:#}",
                    result.project_path.display()
                );
                self.push_background_job_record(
                    "validation",
                    "failed",
                    elapsed_secs,
                    detail,
                    Some(result.output_dir.to_string_lossy().into_owned()),
                );
                self.record_error(error);
            }
        }
    }

    fn apply_suggestion_result(&mut self, result: SuggestionJobResult, elapsed_secs: f32) {
        if PathBuf::from(self.project_path.clone()) != result.project_path
            || self.profile != result.profile
        {
            self.status = "Ignored stale scenario suggestion result.".to_string();
            self.push_diagnostic("Ignored scenario suggestions because project/profile changed.");
            self.push_background_job_record(
                "scenario suggestions",
                "stale",
                elapsed_secs,
                format!(
                    "Ignored suggestions for {} because project/profile changed.",
                    result.project_path.display()
                ),
                None,
            );
            return;
        }
        match result.result {
            Ok(yaml) => {
                self.status = "Scenario suggestions generated.".to_string();
                self.suggestions_yaml = yaml;
                self.stage = Stage::Library;
                self.push_diagnostic("Scenario suggestion YAML updated.");
                self.push_background_job_record(
                    "scenario suggestions",
                    "completed",
                    elapsed_secs,
                    format!(
                        "Scenario suggestions generated for {}.",
                        result.project_path.display()
                    ),
                    None,
                );
                self.load_project_summary_unchecked();
            }
            Err(error) => {
                if cancellation::is_canceled(&error) {
                    let detail = format!("{error:#}");
                    self.status = "Background job canceled.".to_string();
                    self.push_diagnostic(&detail);
                    self.push_canceled_background_job_record(
                        "scenario suggestions",
                        elapsed_secs,
                        detail,
                        None,
                    );
                    return;
                }
                let detail = format!("{error:#}");
                self.push_background_job_record(
                    "scenario suggestions",
                    "failed",
                    elapsed_secs,
                    detail,
                    None,
                );
                self.record_error(error);
            }
        }
    }

    fn apply_import_project_result(&mut self, result: ImportProjectJobResult, elapsed_secs: f32) {
        if self.project_path != result.prior_project_path
            || self.import_state_key(result.kind) != result.state_key
        {
            self.status = "Ignored stale import result.".to_string();
            self.push_diagnostic(
                "Ignored an import result because project or import settings changed.",
            );
            self.push_background_job_record(
                result.kind.label(),
                "stale",
                elapsed_secs,
                "Ignored import result because project or import settings changed.".to_string(),
                Some(result.output_project_path.to_string_lossy().into_owned()),
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
                self.push_background_job_record(
                    result.kind.label(),
                    "completed",
                    elapsed_secs,
                    result.success_diagnostic,
                    Some(result.output_project_path.to_string_lossy().into_owned()),
                );
                self.load_project_summary_unchecked();
                if let Some(stage) = result.next_stage {
                    self.stage = stage;
                }
            }
            Err(error) => {
                if cancellation::is_canceled(&error) {
                    let detail = format!("{error:#}");
                    self.status = "Background job canceled.".to_string();
                    self.push_diagnostic(&detail);
                    self.push_canceled_background_job_record(
                        result.kind.label(),
                        elapsed_secs,
                        detail,
                        Some(result.output_project_path.to_string_lossy().into_owned()),
                    );
                    return;
                }
                let detail = format!("{error:#}");
                self.push_background_job_record(
                    result.kind.label(),
                    "failed",
                    elapsed_secs,
                    detail,
                    Some(result.output_project_path.to_string_lossy().into_owned()),
                );
                self.record_error(error);
            }
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

impl ImportProjectKind {
    fn label(self) -> &'static str {
        match self {
            Self::KiCadSchematic => "KiCad schematic import",
            Self::KiCadPcb => "KiCad PCB import",
            Self::SpiceDeck => "SPICE deck import",
        }
    }
}

impl CircuitCiApp {
    fn push_background_job_event(&mut self, progress: BackgroundJobProgress) {
        let Some(job) = &mut self.background_job else {
            return;
        };
        let event = BackgroundJobEvent {
            stage: progress.stage.to_string(),
            detail: progress.detail,
            elapsed_secs: job.started_at.elapsed().as_secs_f32(),
        };
        self.status = format!("Background {}: {}", job.label, event.stage);
        job.events.push(event);
        if job.events.len() > BACKGROUND_JOB_EVENT_LIMIT {
            let overflow = job.events.len() - BACKGROUND_JOB_EVENT_LIMIT;
            job.events.drain(0..overflow);
        }
    }

    fn push_background_job_record(
        &mut self,
        label: &str,
        outcome: &str,
        elapsed_secs: f32,
        detail: String,
        output_path: Option<String>,
    ) {
        self.background_job_history.push(BackgroundJobRecord {
            label: label.to_string(),
            outcome: outcome.to_string(),
            elapsed_secs,
            detail,
            output_path,
        });
        if self.background_job_history.len() > BACKGROUND_JOB_HISTORY_LIMIT {
            let overflow = self.background_job_history.len() - BACKGROUND_JOB_HISTORY_LIMIT;
            self.background_job_history.drain(0..overflow);
        }
    }

    fn push_canceled_background_job_record(
        &mut self,
        label: &str,
        elapsed_secs: f32,
        detail: String,
        output_path: Option<String>,
    ) {
        self.push_background_job_record(label, "canceled", elapsed_secs, detail, output_path);
    }
}

fn send_background_progress(
    sender: &mpsc::Sender<BackgroundJobMessage>,
    stage: &'static str,
    detail: String,
) {
    let _ = sender.send(BackgroundJobMessage::Progress(BackgroundJobProgress {
        stage,
        detail,
    }));
}

fn canceled_job_detail(result: &BackgroundJobResult) -> Option<String> {
    match result {
        BackgroundJobResult::Validation(result) => result
            .result
            .as_ref()
            .err()
            .filter(|error| cancellation::is_canceled(error))
            .map(|error| format!("{error:#}")),
        BackgroundJobResult::Suggestions(result) => result
            .result
            .as_ref()
            .err()
            .filter(|error| cancellation::is_canceled(error))
            .map(|error| format!("{error:#}")),
        BackgroundJobResult::ImportProject(result) => result
            .result
            .as_ref()
            .err()
            .filter(|error| cancellation::is_canceled(error))
            .map(|error| format!("{error:#}")),
    }
}

fn background_job_output_path(result: &BackgroundJobResult) -> Option<String> {
    match result {
        BackgroundJobResult::Validation(result) => {
            Some(result.output_dir.to_string_lossy().into_owned())
        }
        BackgroundJobResult::ImportProject(result) => {
            Some(result.output_project_path.to_string_lossy().into_owned())
        }
        BackgroundJobResult::Suggestions(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BACKGROUND_JOB_EVENT_LIMIT, BACKGROUND_JOB_HISTORY_LIMIT, BackgroundJobProgress,
        CircuitCiApp, ImportProjectJobResult, ImportProjectKind, SuggestionJobResult,
        ValidationJobResult,
    };
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;

    #[test]
    fn cancel_without_job_is_noop() {
        let mut app = CircuitCiApp::default();
        app.cancel_background_job();
        assert!(app.background_job_elapsed_secs().is_none());
    }

    #[test]
    fn progress_messages_update_active_job_events() {
        let mut app = CircuitCiApp::default();
        app.start_background_job(
            "validation",
            "project -> out".to_string(),
            |_sender, _cancel_token| {},
        );
        app.push_background_job_event(BackgroundJobProgress {
            stage: "Loading project",
            detail: "project.yaml".to_string(),
        });

        let events = app.background_job_events().expect("active job");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].stage, "Loading project");
        assert_eq!(events[0].detail, "project.yaml");
        assert_eq!(app.status, "Background validation: Loading project");
    }

    #[test]
    fn progress_event_history_is_capped() {
        let mut app = CircuitCiApp::default();
        app.start_background_job(
            "validation",
            "project -> out".to_string(),
            |_sender, _cancel_token| {},
        );
        for index in 0..(BACKGROUND_JOB_EVENT_LIMIT + 2) {
            app.push_background_job_event(BackgroundJobProgress {
                stage: "Step",
                detail: format!("event {index}"),
            });
        }

        let events = app.background_job_events().expect("active job");
        assert_eq!(events.len(), BACKGROUND_JOB_EVENT_LIMIT);
        assert_eq!(events[0].detail, "event 2");
    }

    #[test]
    fn cancel_sets_worker_cancel_token() {
        let mut app = CircuitCiApp::default();
        app.start_background_job(
            "validation",
            "project -> out".to_string(),
            |_sender, _cancel_token| {},
        );

        app.cancel_background_job();

        let job = app.background_job.as_ref().expect("active job");
        assert!(job.canceled);
        assert!(job.cancel_token.load(Ordering::Relaxed));
        assert_eq!(
            app.status,
            "Cancel requested; worker will stop where supported."
        );
    }

    #[test]
    fn stale_validation_result_is_ignored() {
        let mut app = CircuitCiApp {
            project_path: "new.project.yaml".to_string(),
            profile: "default".to_string(),
            output_dir: "out/new".to_string(),
            ..Default::default()
        };

        app.apply_validation_result(
            ValidationJobResult {
                project_path: PathBuf::from("old.project.yaml"),
                profile: "default".to_string(),
                output_dir: PathBuf::from("out/new"),
                result: Err(anyhow::anyhow!("should not be observed")),
            },
            1.25,
        );

        assert_eq!(app.status, "Ignored stale background validation result.");
        assert!(app.report.is_none());
        assert_eq!(app.background_job_history.len(), 1);
        assert_eq!(app.background_job_history[0].outcome, "stale");
    }

    #[test]
    fn stale_import_result_is_ignored() {
        let mut app = CircuitCiApp {
            project_path: "active.project.yaml".to_string(),
            import_output_path: "out/new.project.yaml".to_string(),
            ..Default::default()
        };

        app.apply_import_project_result(
            ImportProjectJobResult {
                kind: ImportProjectKind::KiCadSchematic,
                prior_project_path: "different.project.yaml".to_string(),
                state_key: app.kicad_schematic_import_key(),
                output_project_path: PathBuf::from("out/new.project.yaml"),
                import_pcb_project_path: Some("out/new.project.yaml".to_string()),
                next_stage: None,
                success_status: "should not apply",
                success_diagnostic: "should not apply".to_string(),
                result: Ok(()),
            },
            2.0,
        );

        assert_eq!(app.status, "Ignored stale import result.");
        assert_eq!(app.project_path, "active.project.yaml");
        assert_eq!(app.background_job_history.len(), 1);
        assert_eq!(
            app.background_job_history[0].label,
            "KiCad schematic import"
        );
    }

    #[test]
    fn canceled_suggestion_result_is_recorded_as_canceled() {
        let mut app = CircuitCiApp {
            project_path: "active.project.yaml".to_string(),
            profile: "default".to_string(),
            ..Default::default()
        };

        app.apply_suggestion_result(
            SuggestionJobResult {
                project_path: PathBuf::from("active.project.yaml"),
                profile: "default".to_string(),
                result: Err(crate::cancellation::canceled(
                    "Scenario suggestions canceled before completion.",
                )),
            },
            0.75,
        );

        assert_eq!(app.status, "Background job canceled.");
        assert_eq!(app.background_job_history.len(), 1);
        assert_eq!(app.background_job_history[0].label, "scenario suggestions");
        assert_eq!(app.background_job_history[0].outcome, "canceled");
        assert!(
            app.background_job_history[0]
                .detail
                .contains("Scenario suggestions canceled")
        );
        assert!(app.background_job_history[0].output_path.is_none());
    }

    #[test]
    fn canceled_import_result_is_recorded_as_canceled() {
        let mut app = CircuitCiApp {
            project_path: "active.project.yaml".to_string(),
            import_spice_deck_path: "input.cir".to_string(),
            import_spice_output_path: "out/imported.project.yaml".to_string(),
            import_spice_project_name: "imported".to_string(),
            import_spice_backend: "auto".to_string(),
            ..Default::default()
        };
        let state_key = app.spice_import_key();

        app.apply_import_project_result(
            ImportProjectJobResult {
                kind: ImportProjectKind::SpiceDeck,
                prior_project_path: "active.project.yaml".to_string(),
                state_key,
                output_project_path: PathBuf::from("out/imported.project.yaml"),
                import_pcb_project_path: None,
                next_stage: None,
                success_status: "should not apply",
                success_diagnostic: "should not apply".to_string(),
                result: Err(crate::cancellation::canceled(
                    "SPICE import canceled before completion.",
                )),
            },
            1.25,
        );

        assert_eq!(app.status, "Background job canceled.");
        assert_eq!(app.project_path, "active.project.yaml");
        assert_eq!(app.background_job_history.len(), 1);
        assert_eq!(app.background_job_history[0].label, "SPICE deck import");
        assert_eq!(app.background_job_history[0].outcome, "canceled");
        assert_eq!(
            app.background_job_history[0].output_path.as_deref(),
            Some("out/imported.project.yaml")
        );
        assert!(
            app.background_job_history[0]
                .detail
                .contains("SPICE import canceled")
        );
    }

    #[test]
    fn job_history_is_capped() {
        let mut app = CircuitCiApp::default();
        for index in 0..(BACKGROUND_JOB_HISTORY_LIMIT + 3) {
            app.push_background_job_record(
                "job",
                "completed",
                index as f32,
                format!("detail {index}"),
                None,
            );
        }

        assert_eq!(
            app.background_job_history.len(),
            BACKGROUND_JOB_HISTORY_LIMIT
        );
        assert_eq!(app.background_job_history[0].detail, "detail 3");
    }
}
