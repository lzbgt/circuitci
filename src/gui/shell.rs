use super::project::PendingProjectAction;
use super::{CircuitCiApp, Stage};
use crate::reports::{Finding, Limitation, ValidationReport};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Import KiCad Schematic").clicked() {
                        self.request_project_action(
                            PendingProjectAction::ImportKiCadSchematic,
                            Some(ui.ctx()),
                        );
                        ui.close();
                    }
                    if ui.button("Import KiCad PCB").clicked() {
                        self.request_project_action(
                            PendingProjectAction::ImportKiCadPcb,
                            Some(ui.ctx()),
                        );
                        ui.close();
                    }
                    if ui.button("Import SPICE Deck").clicked() {
                        self.request_project_action(
                            PendingProjectAction::ImportSpiceDeck,
                            Some(ui.ctx()),
                        );
                        ui.close();
                    }
                    if ui.button("Open Project...").clicked() {
                        self.pick_and_request_project_load(ui.ctx());
                        ui.close();
                    }
                    if ui.button("Load Project").clicked() {
                        self.request_project_action(
                            PendingProjectAction::LoadProjectSummary {
                                path: self.project_path.clone(),
                            },
                            Some(ui.ctx()),
                        );
                        ui.close();
                    }
                    if ui.button("Load Project YAML").clicked() {
                        self.request_project_action(
                            PendingProjectAction::LoadProjectYaml {
                                path: self.project_path.clone(),
                            },
                            Some(ui.ctx()),
                        );
                        ui.close();
                    }
                    if ui.button("Save Project YAML").clicked() {
                        self.save_project_yaml();
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            self.background_validation_elapsed_secs().is_none(),
                            egui::Button::new("Validate"),
                        )
                        .clicked()
                    {
                        self.validate_project();
                        ui.close();
                    }
                    if ui.button("Suggest Scenarios").clicked() {
                        self.suggest_scenarios();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        self.request_project_action(PendingProjectAction::Quit, Some(ui.ctx()));
                        ui.close();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(
                            !self.project_yaml_undo.is_empty(),
                            egui::Button::new("Undo"),
                        )
                        .clicked()
                    {
                        self.undo_project_yaml_edit();
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            !self.project_yaml_redo.is_empty(),
                            egui::Button::new("Redo"),
                        )
                        .clicked()
                    {
                        self.redo_project_yaml_edit();
                        ui.close();
                    }
                });
                ui.menu_button("Workflow", |ui| {
                    for stage in Stage::ALL {
                        if ui.button(stage.label()).clicked() {
                            self.stage = stage;
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Simulation", |ui| {
                    if ui
                        .add_enabled(
                            self.background_validation_elapsed_secs().is_none(),
                            egui::Button::new("Run Validation + Analog Scenarios"),
                        )
                        .clicked()
                    {
                        self.validate_project();
                        self.stage = Stage::Simulation;
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            self.background_validation_elapsed_secs().is_some()
                                && !self.background_validation_cancel_requested(),
                            egui::Button::new("Cancel Background Validation"),
                        )
                        .clicked()
                    {
                        self.cancel_background_validation();
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    ui.label("Native Rust desktop shell for CircuitCI.");
                    ui.label("The engine remains the CLI/library validation runtime.");
                });
            });
        });
    }

    pub(super) fn workflow_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("workflow_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (index, stage) in Stage::ALL.iter().enumerate() {
                    let label = format!("{}. {}", index + 1, stage.label());
                    if ui.selectable_label(self.stage == *stage, label).clicked() {
                        self.stage = *stage;
                    }
                }
            });
        });
    }

    pub(super) fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("project_panel")
            .resizable(true)
            .default_width(310.0)
            .show(ctx, |ui| {
                ui.heading("CircuitCI");
                ui.separator();
                ui.label("Project");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.project_path);
                    if ui.button("Browse").clicked() {
                        self.pick_project_path();
                    }
                    if ui.button("Open").clicked() {
                        self.pick_and_request_project_load(ui.ctx());
                    }
                });
                ui.label("Output");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.output_dir);
                    if ui.button("Folder").clicked() {
                        self.pick_output_dir();
                    }
                });
                ui.label("Profile");
                ui.text_edit_singleline(&mut self.profile);
                ui.horizontal(|ui| {
                    if ui.button("Load").clicked() {
                        self.request_project_action(
                            PendingProjectAction::LoadProjectSummary {
                                path: self.project_path.clone(),
                            },
                            Some(ui.ctx()),
                        );
                    }
                    if ui.button("Save").clicked() {
                        self.save_project_yaml();
                    }
                    if ui
                        .add_enabled(
                            self.background_validation_elapsed_secs().is_none(),
                            egui::Button::new("Validate"),
                        )
                        .clicked()
                    {
                        self.validate_project();
                    }
                    if ui.button("Suggest").clicked() {
                        self.suggest_scenarios();
                    }
                });
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.project_yaml_undo.is_empty(),
                            egui::Button::new("Undo"),
                        )
                        .clicked()
                    {
                        self.undo_project_yaml_edit();
                    }
                    if ui
                        .add_enabled(
                            !self.project_yaml_redo.is_empty(),
                            egui::Button::new("Redo"),
                        )
                        .clicked()
                    {
                        self.redo_project_yaml_edit();
                    }
                    if !self.project_yaml_undo.is_empty() || !self.project_yaml_redo.is_empty() {
                        ui.label(format!(
                            "{} undo / {} redo",
                            self.project_yaml_undo.len(),
                            self.project_yaml_redo.len()
                        ));
                    }
                });
                ui.separator();
                if let Some(elapsed_secs) = self.background_validation_elapsed_secs() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new());
                            let suffix = if self.background_validation_cancel_requested() {
                                "cancel requested"
                            } else {
                                "running"
                            };
                            ui.label(format!("Validation {suffix} for {elapsed_secs:.1}s"));
                            if ui
                                .add_enabled(
                                    !self.background_validation_cancel_requested(),
                                    egui::Button::new("Cancel"),
                                )
                                .clicked()
                            {
                                self.cancel_background_validation();
                            }
                        });
                    });
                }
                ui.separator();
                if let Some(snapshot) = &self.project_snapshot {
                    ui.label(format!("Name: {}", snapshot.name));
                    ui.label(format!("Components: {}", snapshot.components));
                    ui.label(format!("Nets: {}", snapshot.nets));
                    ui.label(format!("Scenarios: {}", snapshot.scenarios));
                    if self.project_yaml_dirty {
                        ui.label("YAML: unsaved edits");
                    }
                    if !snapshot.libraries.is_empty() {
                        ui.label("Libraries");
                        for library in &snapshot.libraries {
                            ui.monospace(library);
                        }
                    }
                } else {
                    ui.label("No project loaded.");
                }
            });
    }

    pub(super) fn bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("messages")
            .resizable(true)
            .default_height(120.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Status:");
                    ui.label(&self.status);
                });
                if let Some(elapsed_secs) = self.background_validation_elapsed_secs() {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        let suffix = if self.background_validation_cancel_requested() {
                            "cancel requested"
                        } else {
                            "running"
                        };
                        ui.label(format!(
                            "Background validation {suffix}, {elapsed_secs:.1}s"
                        ));
                        if ui
                            .add_enabled(
                                !self.background_validation_cancel_requested(),
                                egui::Button::new("Cancel"),
                            )
                            .clicked()
                        {
                            self.cancel_background_validation();
                        }
                    });
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for diagnostic in self.diagnostics.iter().rev().take(40) {
                        ui.label(diagnostic);
                    }
                });
            });
    }

    pub(super) fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| match self.stage {
            Stage::Project => self.project_stage(ui),
            Stage::Import => self.import_stage(ui),
            Stage::Sketch => self.sketch_stage(ui),
            Stage::Library => self.library_stage(ui),
            Stage::Simulation => self.simulation_stage(ui),
            Stage::Reports => self.reports_stage(ui),
        });
    }

    fn project_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Project Workflow");
        ui.separator();
        ui.label("Load a Board IR project or an imported KiCad/EDA artifact, then run validation directly through the CircuitCI engine.");
        ui.add_space(8.0);
        if let Some(report) = &self.report {
            ui.horizontal(|ui| {
                ui.strong(format!("Result: {}", report.result));
                ui.label(format!(
                    "critical {} / warning {} / info {}",
                    report.summary.critical, report.summary.warning, report.summary.info
                ));
            });
        }
        ui.add_space(8.0);
        ui.label("Recommended flow");
        ui.label("Import or sketch -> bind library -> simulate/validate -> inspect reports -> revise design evidence.");
    }

    fn reports_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Reports");
        ui.separator();
        if self.report.is_some() {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.report_markdown)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(32)
                        .lock_focus(true),
                );
            });
        } else {
            ui.label("No report loaded.");
        }
    }

    pub(super) fn findings_view(&self, ui: &mut egui::Ui, report: &ValidationReport) {
        finding_group(ui, "Critical", &report.failures);
        finding_group(ui, "Warnings", &report.warnings);
        finding_group(ui, "Info", &report.infos);
        limitation_group(ui, &report.limitations);
    }
}

fn finding_group(ui: &mut egui::Ui, title: &str, findings: &[Finding]) {
    egui::CollapsingHeader::new(format!("{title} ({})", findings.len()))
        .default_open(!findings.is_empty())
        .show(ui, |ui| {
            for finding in findings {
                ui.group(|ui| {
                    ui.strong(&finding.id);
                    ui.label(&finding.scenario);
                    ui.label(&finding.message);
                    if !finding.suggested_fixes.is_empty() {
                        ui.label("Suggested fixes");
                        for fix in &finding.suggested_fixes {
                            ui.label(fix);
                        }
                    }
                });
            }
        });
}

fn limitation_group(ui: &mut egui::Ui, limitations: &[Limitation]) {
    egui::CollapsingHeader::new(format!("Limitations ({})", limitations.len())).show(ui, |ui| {
        for limitation in limitations {
            ui.group(|ui| {
                ui.strong(&limitation.id);
                ui.label(format!(
                    "{} / confidence {} / blocking {}",
                    limitation.scope, limitation.confidence, limitation.blocking
                ));
                ui.label(&limitation.message);
            });
        }
    });
}
