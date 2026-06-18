use super::project::PendingProjectAction;
use super::project::{optional_path, sanitized_project_name};
use super::{CircuitCiApp, Stage};
use eframe::egui;
use std::path::Path;

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
                    ui.text_edit_singleline(&mut self.import_schematic_path);
                    ui.end_row();
                    ui.label("Mapping");
                    ui.text_edit_singleline(&mut self.import_mapping_path);
                    ui.end_row();
                    ui.label("Output project");
                    ui.text_edit_singleline(&mut self.import_output_path);
                    ui.end_row();
                    ui.label("Project name");
                    ui.text_edit_singleline(&mut self.import_project_name);
                    ui.end_row();
                    ui.label("Default model");
                    ui.text_edit_singleline(&mut self.import_default_model);
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Import Schematic").clicked() {
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
                    ui.text_edit_singleline(&mut self.import_spice_deck_path);
                    ui.end_row();
                    ui.label("Output project");
                    ui.text_edit_singleline(&mut self.import_spice_output_path);
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
                if ui.button("Import SPICE Deck").clicked() {
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
                    ui.text_edit_singleline(&mut self.import_pcb_path);
                    ui.end_row();
                    ui.label("Input project");
                    ui.text_edit_singleline(&mut self.import_pcb_project_path);
                    ui.end_row();
                    ui.label("Output project");
                    ui.text_edit_singleline(&mut self.import_pcb_output_path);
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Import PCB Evidence").clicked() {
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

    pub(super) fn import_kicad_schematic(&mut self) {
        let schematic = Path::new(&self.import_schematic_path).to_path_buf();
        let output = Path::new(&self.import_output_path).to_path_buf();
        let mapping = optional_path(&self.import_mapping_path);
        let name = if self.import_project_name.trim().is_empty() {
            sanitized_project_name(&schematic, "imported_kicad_project")
        } else {
            self.import_project_name.trim().to_string()
        };
        let options = crate::importers::kicad::KicadImportOptions {
            input: schematic,
            output: output.clone(),
            name,
            default_model: self.import_default_model.trim().to_string(),
            mapping,
        };
        match crate::importers::kicad_sch::import_kicad_schematic(&options) {
            Ok(()) => {
                self.project_path = output.to_string_lossy().into_owned();
                self.import_pcb_project_path = self.project_path.clone();
                self.status = "KiCad schematic imported.".to_string();
                self.push_diagnostic("KiCad schematic imported to Board IR.");
                self.load_project_summary_unchecked();
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn import_kicad_pcb(&mut self) {
        let options = crate::importers::kicad_pcb::KicadPcbPlacementImportOptions {
            input: Path::new(&self.import_pcb_path).to_path_buf(),
            project: Path::new(&self.import_pcb_project_path).to_path_buf(),
            output: Path::new(&self.import_pcb_output_path).to_path_buf(),
        };
        match crate::importers::kicad_pcb::import_kicad_pcb_placements(&options) {
            Ok(summary) => {
                self.project_path = self.import_pcb_output_path.clone();
                self.status = "KiCad PCB evidence imported.".to_string();
                self.push_diagnostic(&format!(
                    "KiCad PCB imported: {} placements, {} pads, {} route segments, {} vias.",
                    summary.placements, summary.pads, summary.route_segments, summary.route_vias
                ));
                self.load_project_summary_unchecked();
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn import_spice_deck(&mut self) {
        let deck = Path::new(&self.import_spice_deck_path).to_path_buf();
        let output = Path::new(&self.import_spice_output_path).to_path_buf();
        let name = if self.import_spice_project_name.trim().is_empty() {
            sanitized_project_name(&deck, "imported_spice_project")
        } else {
            self.import_spice_project_name.trim().to_string()
        };
        let options = crate::importers::spice::SpiceImportOptions {
            input: deck.clone(),
            output: output.clone(),
            name,
            backend: self.import_spice_backend.trim().to_string(),
            stop_time_us: self.import_spice_stop_time_us,
            max_step_us: self.import_spice_max_step_us,
        };
        match crate::importers::spice::import_spice(&options) {
            Ok(()) => {
                self.project_path = output.to_string_lossy().into_owned();
                self.status = "SPICE deck imported.".to_string();
                self.push_diagnostic(&format!(
                    "SPICE deck imported to Board IR from {}.",
                    deck.display()
                ));
                self.load_project_summary_unchecked();
                self.stage = Stage::Simulation;
            }
            Err(error) => self.record_error(error),
        }
    }
}
