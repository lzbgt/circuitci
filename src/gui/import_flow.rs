use super::CircuitCiApp;
use super::project::PendingProjectAction;
use eframe::egui;

const NE555_SCOPE_EXAMPLE_DECK: &str = "examples/ne555_astable_scope_smoke/deck.cir";
const NE555_SCOPE_EXAMPLE_OUTPUT: &str = "out/gui_import/ne555_astable_scope.project.yaml";
const NE555_SCOPE_EXAMPLE_NAME: &str = "ne555_astable_scope";
const NE555_SCOPE_EXAMPLE_STOP_TIME_US: f64 = 5_000.0;
const NE555_SCOPE_EXAMPLE_MAX_STEP_US: f64 = 2.0;

impl CircuitCiApp {
    pub(super) fn import_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Import");
        ui.separator();
        ui.label("Import external CAD evidence into Board IR, then edit and validate the generated project.");
        ui.add_space(8.0);

        ui.group(|ui| {
            ui.strong("KiCad Schematic To Board IR");
            egui::Grid::new("kicad_schematic_import_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Schematic");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.import_schematic_path);
                        if ui.button("Browse").clicked() {
                            self.pick_import_schematic_path();
                        }
                    });
                    ui.end_row();
                    ui.label("Mapping");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.import_mapping_path);
                        if ui.button("Browse").clicked() {
                            self.pick_import_mapping_path();
                        }
                    });
                    ui.end_row();
                    ui.label("Output project");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.import_output_path);
                        if ui.button("Choose").clicked() {
                            self.pick_import_output_path();
                        }
                    });
                    ui.end_row();
                    ui.label("Project name");
                    ui.text_edit_singleline(&mut self.import_project_name);
                    ui.end_row();
                    ui.label("Default model");
                    ui.text_edit_singleline(&mut self.import_default_model);
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.background_job_elapsed_secs().is_none(),
                        egui::Button::new("Import Schematic"),
                    )
                    .clicked()
                {
                    self.request_project_action(
                        PendingProjectAction::ImportKiCadSchematic,
                        Some(ui.ctx()),
                    );
                }
                if ui.button("Use As Project").clicked() {
                    self.request_project_action(
                        PendingProjectAction::LoadProjectSummary {
                            path: self.import_output_path.clone(),
                        },
                        Some(ui.ctx()),
                    );
                }
            });
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong("SPICE Deck To Board IR");
            egui::Grid::new("spice_import_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Deck");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.import_spice_deck_path);
                        if ui.button("Browse").clicked() {
                            self.pick_import_spice_deck_path();
                        }
                    });
                    ui.end_row();
                    ui.label("Output project");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.import_spice_output_path);
                        if ui.button("Choose").clicked() {
                            self.pick_import_spice_output_path();
                        }
                    });
                    ui.end_row();
                    ui.label("Project name");
                    ui.text_edit_singleline(&mut self.import_spice_project_name);
                    ui.end_row();
                    ui.label("Backend");
                    egui::ComboBox::from_id_salt("spice_import_backend")
                        .selected_text(&self.import_spice_backend)
                        .show_ui(ui, |ui| {
                            for backend in ["auto", "ngspice", "xyce", "embedded_ngspice"] {
                                ui.selectable_value(
                                    &mut self.import_spice_backend,
                                    backend.to_string(),
                                    backend,
                                );
                            }
                        });
                    ui.end_row();
                    ui.label("Stop time");
                    ui.add(
                        egui::DragValue::new(&mut self.import_spice_stop_time_us)
                            .speed(10.0)
                            .range(0.001..=1_000_000_000.0)
                            .suffix(" us"),
                    );
                    ui.end_row();
                    ui.label("Max step");
                    ui.add(
                        egui::DragValue::new(&mut self.import_spice_max_step_us)
                            .speed(0.1)
                            .range(0.001..=1_000_000_000.0)
                            .suffix(" us"),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                ui.label("Scope-ready example");
                if ui.button("Use NE555 Astable").clicked() {
                    self.use_ne555_scope_example_import_preset();
                }
                if ui
                    .add_enabled(
                        self.background_job_elapsed_secs().is_none(),
                        egui::Button::new("Import NE555"),
                    )
                    .clicked()
                {
                    self.use_ne555_scope_example_import_preset();
                    self.request_project_action(
                        PendingProjectAction::ImportSpiceDeck,
                        Some(ui.ctx()),
                    );
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.background_job_elapsed_secs().is_none(),
                        egui::Button::new("Import SPICE Deck"),
                    )
                    .clicked()
                {
                    self.request_project_action(
                        PendingProjectAction::ImportSpiceDeck,
                        Some(ui.ctx()),
                    );
                }
                if ui.button("Use As Project").clicked() {
                    self.request_project_action(
                        PendingProjectAction::LoadProjectSummary {
                            path: self.import_spice_output_path.clone(),
                        },
                        Some(ui.ctx()),
                    );
                }
            });
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong("KiCad PCB Layout Evidence");
            egui::Grid::new("kicad_pcb_import_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("PCB");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.import_pcb_path);
                        if ui.button("Browse").clicked() {
                            self.pick_import_pcb_path();
                        }
                    });
                    ui.end_row();
                    ui.label("Input project");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.import_pcb_project_path);
                        if ui.button("Browse").clicked() {
                            self.pick_import_pcb_project_path();
                        }
                    });
                    ui.end_row();
                    ui.label("Output project");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.import_pcb_output_path);
                        if ui.button("Choose").clicked() {
                            self.pick_import_pcb_output_path();
                        }
                    });
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.background_job_elapsed_secs().is_none(),
                        egui::Button::new("Import PCB Evidence"),
                    )
                    .clicked()
                {
                    self.request_project_action(
                        PendingProjectAction::ImportKiCadPcb,
                        Some(ui.ctx()),
                    );
                }
                if ui.button("Use As Project").clicked() {
                    self.request_project_action(
                        PendingProjectAction::LoadProjectSummary {
                            path: self.import_pcb_output_path.clone(),
                        },
                        Some(ui.ctx()),
                    );
                }
            });
        });
    }

    pub(super) fn use_ne555_scope_example_import_preset(&mut self) {
        self.import_spice_deck_path = NE555_SCOPE_EXAMPLE_DECK.to_string();
        self.import_spice_output_path = NE555_SCOPE_EXAMPLE_OUTPUT.to_string();
        self.import_spice_project_name = NE555_SCOPE_EXAMPLE_NAME.to_string();
        self.import_spice_backend = "auto".to_string();
        self.import_spice_stop_time_us = NE555_SCOPE_EXAMPLE_STOP_TIME_US;
        self.import_spice_max_step_us = NE555_SCOPE_EXAMPLE_MAX_STEP_US;
        self.status = "NE555 astable scope example selected.".to_string();
        self.push_diagnostic(
            "NE555 astable scope preset selected for SPICE import and Scopes workflow.",
        );
    }
}
