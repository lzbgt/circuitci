use super::simulation::load_report_waveforms;
use super::{CircuitCiApp, Stage, validate_from_gui};
use crate::reports::ValidationReport;
use anyhow::{Context, Result};
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

pub(super) struct BackgroundValidationJob {
    receiver: Receiver<ValidationJobResult>,
    started_at: Instant,
    canceled: bool,
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

impl CircuitCiApp {
    pub(super) fn validate_project(&mut self) {
        if self.background_validation.is_some() {
            self.status = "Validation already running.".to_string();
            self.push_diagnostic("Validation is already running in the background.");
            return;
        }

        let project_path = PathBuf::from(self.project_path.clone());
        let profile = self.profile.clone();
        let output_dir = PathBuf::from(self.output_dir.clone());
        let (sender, receiver) = mpsc::channel();
        let thread_project_path = project_path.clone();
        let thread_profile = profile.clone();
        let thread_output_dir = output_dir.clone();
        thread::spawn(move || {
            let result =
                validate_from_gui(&thread_project_path, &thread_profile, &thread_output_dir)
                    .map(|(report, markdown)| ValidationJobOutput { report, markdown });
            let _ = sender.send(ValidationJobResult {
                project_path: thread_project_path,
                profile: thread_profile,
                output_dir: thread_output_dir,
                result,
            });
        });

        self.background_validation = Some(BackgroundValidationJob {
            receiver,
            started_at: Instant::now(),
            canceled: false,
        });
        self.status = format!("Validation running for {}.", project_path.to_string_lossy());
        self.push_diagnostic("Validation started in the background.");
    }

    pub(super) fn cancel_background_validation(&mut self) {
        let Some(job) = &mut self.background_validation else {
            return;
        };
        job.canceled = true;
        self.status = "Cancel requested; validation result will be ignored.".to_string();
        self.push_diagnostic(
            "Background validation cannot interrupt an in-flight engine call yet; its result will be ignored.",
        );
    }

    pub(super) fn background_validation_elapsed_secs(&self) -> Option<f32> {
        self.background_validation
            .as_ref()
            .map(|job| job.started_at.elapsed().as_secs_f32())
    }

    pub(super) fn background_validation_cancel_requested(&self) -> bool {
        self.background_validation
            .as_ref()
            .is_some_and(|job| job.canceled)
    }

    pub(super) fn poll_background_validation(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.background_validation else {
            return;
        };
        match job.receiver.try_recv() {
            Ok(result) => {
                let canceled = self
                    .background_validation
                    .as_ref()
                    .is_some_and(|job| job.canceled);
                self.background_validation = None;
                if canceled {
                    self.status = "Background validation canceled.".to_string();
                    self.push_diagnostic("Ignored canceled background validation result.");
                    return;
                }
                self.apply_background_validation_result(result);
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Err(TryRecvError::Disconnected) => {
                self.background_validation = None;
                self.record_error(anyhow::anyhow!(
                    "Background validation worker exited before returning a result."
                ));
            }
        }
    }

    fn apply_background_validation_result(&mut self, result: ValidationJobResult) {
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
}

#[cfg(test)]
mod tests {
    use super::{CircuitCiApp, ValidationJobResult};
    use std::path::PathBuf;

    #[test]
    fn cancel_without_job_is_noop() {
        let mut app = CircuitCiApp::default();
        app.cancel_background_validation();
        assert!(app.background_validation_elapsed_secs().is_none());
    }

    #[test]
    fn stale_validation_result_is_ignored() {
        let mut app = CircuitCiApp::default();
        app.project_path = "new.project.yaml".to_string();
        app.profile = "default".to_string();
        app.output_dir = "out/new".to_string();

        app.apply_background_validation_result(ValidationJobResult {
            project_path: PathBuf::from("old.project.yaml"),
            profile: "default".to_string(),
            output_dir: PathBuf::from("out/new"),
            result: Err(anyhow::anyhow!("should not be observed")),
        });

        assert_eq!(app.status, "Ignored stale background validation result.");
        assert!(app.report.is_none());
    }
}
