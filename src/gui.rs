use crate::reports::{Finding, Limitation, ValidationReport};
use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

mod analog;
mod library;
mod project;
mod simulation;
mod sketch;
mod spice;

use project::{optional_path, sanitized_project_name};
use simulation::{
    WaveformView, load_report_waveforms, runtime_probe_activity_for_selection,
    runtime_probe_lines_for_selection, waveform_time_range_for_view,
};
use sketch::{
    ProjectSnapshot, SketchSelection, add_component, add_net, assign_component_pin,
    draw_sketch_node, draw_sketch_pin_anchor, edit_component_model, edit_component_part_number,
    edit_net_kind, edit_net_nominal_voltage, edit_net_powered, edit_schematic_node_position,
    layout_sketch_graph, remove_component, remove_component_pin, remove_net,
};

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("CircuitCI")
            .with_inner_size([1280.0, 820.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CircuitCI",
        options,
        Box::new(|_cc| Ok(Box::new(CircuitCiApp::default()))),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Project,
    Import,
    Sketch,
    Library,
    Simulation,
    Reports,
}

impl Stage {
    const ALL: [Self; 6] = [
        Self::Project,
        Self::Import,
        Self::Sketch,
        Self::Library,
        Self::Simulation,
        Self::Reports,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Import => "Import",
            Self::Sketch => "Sketch",
            Self::Library => "Library",
            Self::Simulation => "Simulation",
            Self::Reports => "Reports",
        }
    }
}

pub struct CircuitCiApp {
    project_path: String,
    output_dir: String,
    profile: String,
    import_schematic_path: String,
    import_mapping_path: String,
    import_output_path: String,
    import_project_name: String,
    import_default_model: String,
    import_pcb_path: String,
    import_pcb_project_path: String,
    import_pcb_output_path: String,
    import_spice_deck_path: String,
    import_spice_output_path: String,
    import_spice_project_name: String,
    import_spice_backend: String,
    import_spice_stop_time_us: f64,
    import_spice_max_step_us: f64,
    spice_deck_scenario: String,
    spice_deck_path: String,
    spice_deck_text: String,
    spice_deck_dirty: bool,
    stage: Stage,
    status: String,
    diagnostics: Vec<String>,
    report: Option<ValidationReport>,
    report_markdown: String,
    suggestions_yaml: String,
    project_yaml: String,
    project_yaml_dirty: bool,
    project_yaml_undo: Vec<String>,
    project_yaml_redo: Vec<String>,
    model_search: String,
    selected_library_model: String,
    new_component_id: String,
    new_component_model: String,
    new_net_id: String,
    new_net_kind: String,
    pin_edit_id: String,
    pin_edit_net: String,
    wire_pin_id: String,
    wire_from_component: Option<String>,
    analog_scenario_name: String,
    analog_ground_net: String,
    analog_probe_net: String,
    analog_probe_name: String,
    analog_stop_time_us: f64,
    analog_max_step_us: f64,
    analog_assertion_scenario: String,
    analog_assertion_name: String,
    analog_assertion_probe: String,
    analog_assertion_aggregation: String,
    analog_assertion_relation: String,
    analog_assertion_threshold: f64,
    analog_assertion_at_us: f64,
    analog_assertion_start_us: f64,
    analog_assertion_end_us: f64,
    project_snapshot: Option<ProjectSnapshot>,
    selected_sketch_item: Option<SketchSelection>,
    waveforms: Vec<WaveformView>,
    selected_waveform: usize,
    selected_probe: usize,
    waveform_cursor_a_us: f64,
    waveform_cursor_b_us: f64,
    waveform_playing: bool,
    waveform_playback_speed: f64,
}

impl Default for CircuitCiApp {
    fn default() -> Self {
        Self {
            project_path: "demos/smart_robot/circuitci/wheel_actuator/project.yaml".to_string(),
            output_dir: "out/gui".to_string(),
            profile: "default".to_string(),
            import_schematic_path: "demos/smart_robot/kicad/wheel_actuator/root.kicad_sch"
                .to_string(),
            import_mapping_path: "demos/smart_robot/kicad/wheel_actuator/circuitci.kicad-map.yaml"
                .to_string(),
            import_output_path: "out/gui_import/wheel_actuator_imported.project.yaml".to_string(),
            import_project_name: String::new(),
            import_default_model: "generic.schematic.imported_component".to_string(),
            import_pcb_path: "demos/smart_robot/kicad/wheel_actuator/wheel_actuator.kicad_pcb"
                .to_string(),
            import_pcb_project_path: "out/gui_import/wheel_actuator_imported.project.yaml"
                .to_string(),
            import_pcb_output_path: "out/gui_import/wheel_actuator_with_pcb.project.yaml"
                .to_string(),
            import_spice_deck_path: "examples/import_spice_rc/deck.cir".to_string(),
            import_spice_output_path: "out/gui_import/imported_spice.project.yaml".to_string(),
            import_spice_project_name: String::new(),
            import_spice_backend: "auto".to_string(),
            import_spice_stop_time_us: 1000.0,
            import_spice_max_step_us: 1.0,
            spice_deck_scenario: String::new(),
            spice_deck_path: String::new(),
            spice_deck_text: String::new(),
            spice_deck_dirty: false,
            stage: Stage::Project,
            status: "Ready".to_string(),
            diagnostics: Vec::new(),
            report: None,
            report_markdown: String::new(),
            suggestions_yaml: String::new(),
            project_yaml: String::new(),
            project_yaml_dirty: false,
            project_yaml_undo: Vec::new(),
            project_yaml_redo: Vec::new(),
            model_search: String::new(),
            selected_library_model: String::new(),
            new_component_id: "U_NEW".to_string(),
            new_component_model: "generic.schematic.imported_component".to_string(),
            new_net_id: "net_new".to_string(),
            new_net_kind: "digital_or_analog".to_string(),
            pin_edit_id: "P1".to_string(),
            pin_edit_net: String::new(),
            wire_pin_id: "P1".to_string(),
            wire_from_component: None,
            analog_scenario_name: "gui_transient".to_string(),
            analog_ground_net: String::new(),
            analog_probe_net: String::new(),
            analog_probe_name: "probe_voltage".to_string(),
            analog_stop_time_us: 100.0,
            analog_max_step_us: 1.0,
            analog_assertion_scenario: String::new(),
            analog_assertion_name: "probe_above_min".to_string(),
            analog_assertion_probe: String::new(),
            analog_assertion_aggregation: "sample".to_string(),
            analog_assertion_relation: "above".to_string(),
            analog_assertion_threshold: 0.0,
            analog_assertion_at_us: 50.0,
            analog_assertion_start_us: 0.0,
            analog_assertion_end_us: 100.0,
            project_snapshot: None,
            selected_sketch_item: None,
            waveforms: Vec::new(),
            selected_waveform: 0,
            selected_probe: 0,
            waveform_cursor_a_us: 0.0,
            waveform_cursor_b_us: 0.0,
            waveform_playing: false,
            waveform_playback_speed: 1.0,
        }
    }
}

impl eframe::App for CircuitCiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.advance_waveform_playback(ctx);
        self.menu_bar(ctx);
        self.workflow_bar(ctx);
        self.left_panel(ctx);
        self.bottom_panel(ctx);
        self.central_panel(ctx);
    }
}

impl CircuitCiApp {
    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Import KiCad Schematic").clicked() {
                        self.import_kicad_schematic();
                        ui.close();
                    }
                    if ui.button("Import KiCad PCB").clicked() {
                        self.import_kicad_pcb();
                        ui.close();
                    }
                    if ui.button("Import SPICE Deck").clicked() {
                        self.import_spice_deck();
                        ui.close();
                    }
                    if ui.button("Load Project").clicked() {
                        self.load_project_summary();
                        ui.close();
                    }
                    if ui.button("Load Project YAML").clicked() {
                        self.load_project_yaml();
                        ui.close();
                    }
                    if ui.button("Save Project YAML").clicked() {
                        self.save_project_yaml();
                        ui.close();
                    }
                    if ui.button("Validate").clicked() {
                        self.validate_project();
                        ui.close();
                    }
                    if ui.button("Suggest Scenarios").clicked() {
                        self.suggest_scenarios();
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
                    if ui.button("Run Validation + Analog Scenarios").clicked() {
                        self.validate_project();
                        self.stage = Stage::Simulation;
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

    fn workflow_bar(&mut self, ctx: &egui::Context) {
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

    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("project_panel")
            .resizable(true)
            .default_width(310.0)
            .show(ctx, |ui| {
                ui.heading("CircuitCI");
                ui.separator();
                ui.label("Project");
                ui.text_edit_singleline(&mut self.project_path);
                ui.label("Output");
                ui.text_edit_singleline(&mut self.output_dir);
                ui.label("Profile");
                ui.text_edit_singleline(&mut self.profile);
                ui.horizontal(|ui| {
                    if ui.button("Load").clicked() {
                        self.load_project_summary();
                    }
                    if ui.button("Save").clicked() {
                        self.save_project_yaml();
                    }
                    if ui.button("Validate").clicked() {
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

    fn bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("messages")
            .resizable(true)
            .default_height(120.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Status:");
                    ui.label(&self.status);
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for diagnostic in self.diagnostics.iter().rev().take(40) {
                        ui.label(diagnostic);
                    }
                });
            });
    }

    fn central_panel(&mut self, ctx: &egui::Context) {
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

    fn import_stage(&mut self, ui: &mut egui::Ui) {
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
                    self.import_kicad_schematic();
                }
                if ui.button("Use As Project").clicked() {
                    self.project_path = self.import_output_path.clone();
                    self.load_project_summary();
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
                    self.import_spice_deck();
                }
                if ui.button("Use As Project").clicked() {
                    self.project_path = self.import_spice_output_path.clone();
                    self.load_project_summary();
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
                    self.import_kicad_pcb();
                }
                if ui.button("Use As Project").clicked() {
                    self.project_path = self.import_pcb_output_path.clone();
                    self.load_project_summary();
                }
            });
        });
    }

    fn sketch_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Sketch Workspace");
        ui.separator();
        ui.label("Edit the Board IR YAML evidence directly, save it, then rerun validation and waveform observation through the same engine path.");
        ui.add_space(8.0);
        if let Some(snapshot) = &self.project_snapshot {
            egui::Grid::new("sketch_grid").striped(true).show(ui, |ui| {
                ui.label("Board graph");
                ui.label(format!(
                    "{} components, {} nets",
                    snapshot.components, snapshot.nets
                ));
                ui.end_row();
                ui.label("Scenario set");
                ui.label(format!("{} scenarios", snapshot.scenarios));
                ui.end_row();
            });
        }
        if let Some(snapshot) = self.project_snapshot.clone() {
            ui.separator();
            self.sketch_edit_toolbar(ui);
            ui.separator();
            ui.horizontal(|ui| {
                self.draw_board_graph(ui, &snapshot);
                self.sketch_inspector(ui, &snapshot);
            });
        }
        ui.horizontal(|ui| {
            if ui.button("Load YAML").clicked() {
                self.load_project_yaml();
            }
            if ui.button("Save YAML").clicked() {
                self.save_project_yaml();
            }
            if ui.button("Validate YAML").clicked() {
                self.validate_project_yaml_text();
            }
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
            if self.project_yaml_dirty {
                ui.label("Unsaved edits");
            }
        });
        ui.separator();
        if self.project_yaml.is_empty() {
            ui.label("Load a project to edit its Board IR YAML.");
        } else {
            let previous_yaml = self.project_yaml.clone();
            let response = egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.project_yaml)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(36)
                        .lock_focus(true),
                )
            });
            if response.inner.changed() {
                self.record_project_yaml_text_edit(previous_yaml);
            }
        }
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

    fn findings_view(&self, ui: &mut egui::Ui, report: &ValidationReport) {
        finding_group(ui, "Critical", &report.failures);
        finding_group(ui, "Warnings", &report.warnings);
        finding_group(ui, "Info", &report.infos);
        limitation_group(ui, &report.limitations);
    }

    fn import_kicad_schematic(&mut self) {
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
                self.load_project_summary();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn import_kicad_pcb(&mut self) {
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
                self.load_project_summary();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn import_spice_deck(&mut self) {
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
                self.load_project_summary();
                self.stage = Stage::Simulation;
            }
            Err(error) => self.record_error(error),
        }
    }

    fn validate_project(&mut self) {
        match validate_from_gui(
            Path::new(&self.project_path),
            &self.profile,
            Path::new(&self.output_dir),
        ) {
            Ok((report, markdown)) => {
                let waveforms = load_report_waveforms(&report);
                let waveform_count = waveforms.len();
                self.status = format!("Validation {}", report.result);
                self.report_markdown = markdown;
                self.report = Some(report);
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
                    "Validation report written; loaded {waveform_count} waveform view(s)."
                ));
                self.load_project_summary();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn suggest_scenarios(&mut self) {
        match suggest_from_gui(Path::new(&self.project_path), &self.profile) {
            Ok(yaml) => {
                self.status = "Scenario suggestions generated.".to_string();
                self.suggestions_yaml = yaml;
                self.stage = Stage::Library;
                self.push_diagnostic("Scenario suggestion YAML updated.");
                self.load_project_summary();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn record_error(&mut self, error: anyhow::Error) {
        self.status = "Error".to_string();
        self.push_diagnostic(&format!("{error:#}"));
    }

    fn push_diagnostic(&mut self, message: &str) {
        self.diagnostics.push(message.to_string());
    }

    fn sketch_edit_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Graph Edits", |ui| {
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
                ui.label("Graph, property, wire, and YAML edits share one Board IR history.");
            });
            egui::Grid::new("sketch_graph_edit_grid")
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Component");
                    ui.text_edit_singleline(&mut self.new_component_id);
                    ui.text_edit_singleline(&mut self.new_component_model);
                    if ui.button("Add Component").clicked() {
                        self.apply_add_component();
                    }
                    ui.end_row();

                    ui.label("Net");
                    ui.text_edit_singleline(&mut self.new_net_id);
                    egui::ComboBox::from_id_salt("new_net_kind")
                        .selected_text(&self.new_net_kind)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.new_net_kind,
                                "digital_or_analog".to_string(),
                                "digital_or_analog",
                            );
                            ui.selectable_value(
                                &mut self.new_net_kind,
                                "power".to_string(),
                                "power",
                            );
                            ui.selectable_value(
                                &mut self.new_net_kind,
                                "ground".to_string(),
                                "ground",
                            );
                        });
                    if ui.button("Add Net").clicked() {
                        self.apply_add_net();
                    }
                    ui.end_row();
                });
        });
    }

    fn draw_board_graph(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        let desired_size = egui::vec2((ui.available_width() * 0.64).max(460.0), 340.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, egui::Color32::from_gray(18));
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
            egui::StrokeKind::Inside,
        );

        let graph = layout_sketch_graph(rect, snapshot);
        for edge in &graph.edges {
            painter.line_segment(
                [edge.start, edge.end],
                egui::Stroke::new(1.0, egui::Color32::from_gray(72)),
            );
        }
        if let Some(component_id) = &self.wire_from_component
            && let Some(pointer) = ui.ctx().pointer_hover_pos()
            && rect.contains(pointer)
            && let Some(source) = wire_preview_start(&graph, component_id, &self.wire_pin_id)
        {
            painter.line_segment(
                [source, pointer],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 196, 87)),
            );
        }
        for node in &graph.nodes {
            let selected = self
                .selected_sketch_item
                .as_ref()
                .is_some_and(|selection| selection.matches(node));
            let runtime_activity = runtime_probe_activity_for_selection(
                &self.waveforms,
                self.selected_waveform,
                self.waveform_cursor_a_us,
                &node.selection,
                snapshot,
            );
            draw_sketch_node(&painter, node, selected, runtime_activity);
        }
        for anchor in &graph.pin_anchors {
            let active = self.wire_from_component.as_ref() == Some(&anchor.component_id)
                && self.wire_pin_id.trim() == anchor.pin;
            draw_sketch_pin_anchor(&painter, anchor, active);
        }

        let hovered_node = if response.hovered() {
            ui.ctx()
                .pointer_hover_pos()
                .and_then(|position| graph.nodes.iter().find(|node| node.rect.contains(position)))
        } else {
            None
        };
        let hovered_anchor = if response.hovered() {
            ui.ctx().pointer_hover_pos().and_then(|position| {
                graph
                    .pin_anchors
                    .iter()
                    .find(|anchor| anchor.pos.distance(position) <= 8.0)
            })
        } else {
            None
        };

        if response.clicked()
            && let Some(position) = response.interact_pointer_pos()
        {
            let clicked_anchor = graph
                .pin_anchors
                .iter()
                .find(|anchor| anchor.pos.distance(position) <= 8.0);
            let clicked = graph
                .nodes
                .iter()
                .find(|node| node.rect.contains(position))
                .map(|node| node.selection.clone());
            if let Some(anchor) = clicked_anchor {
                self.selected_sketch_item =
                    Some(SketchSelection::Component(anchor.component_id.clone()));
                self.pin_edit_id = anchor.pin.clone();
                self.pin_edit_net = anchor.net.clone();
                self.wire_pin_id = anchor.pin.clone();
                self.wire_from_component = Some(anchor.component_id.clone());
                self.status = format!(
                    "Wire mode: click a net node to connect {}.{}.",
                    anchor.component_id, anchor.pin
                );
            } else if let Some(SketchSelection::Net(net_id)) = &clicked
                && let Some(component_id) = self.wire_from_component.clone()
            {
                self.apply_visual_wire(component_id, net_id.clone());
            } else {
                self.selected_sketch_item = clicked;
            }
        }

        if response.drag_started()
            && let Some(position) = response.interact_pointer_pos()
        {
            self.selected_sketch_item = graph
                .nodes
                .iter()
                .find(|node| node.rect.contains(position))
                .map(|node| node.selection.clone());
        }
        if response.dragged()
            && let (Some(selection), Some(position)) = (
                self.selected_sketch_item.clone(),
                response.interact_pointer_pos(),
            )
            && !matches!(selection, SketchSelection::Overflow(_))
            && let Some(node) = graph.nodes.iter().find(|node| node.selection == selection)
        {
            let x = (position.x - rect.left() - node.rect.width() / 2.0)
                .clamp(0.0, (rect.width() - node.rect.width()).max(0.0));
            let y = (position.y - rect.top() - node.rect.height() / 2.0)
                .clamp(0.0, (rect.height() - node.rect.height()).max(0.0));
            self.apply_schematic_node_position(selection, x as f64, y as f64);
        }

        if let Some(anchor) = hovered_anchor {
            response.on_hover_ui(|ui| {
                sketch_pin_hover_tooltip(ui, anchor);
            });
        } else if let Some(node) = hovered_node {
            let runtime_lines = runtime_probe_lines_for_selection(
                &self.waveforms,
                self.selected_waveform,
                self.waveform_cursor_a_us,
                &node.selection,
                snapshot,
            );
            response.on_hover_ui(|ui| {
                sketch_hover_tooltip(ui, node, &runtime_lines);
            });
        }
    }

    fn advance_waveform_playback(&mut self, ctx: &egui::Context) {
        if !self.waveform_playing {
            return;
        }
        let Some((start_us, end_us)) =
            waveform_time_range_for_view(&self.waveforms, self.selected_waveform)
        else {
            self.waveform_playing = false;
            return;
        };
        if end_us <= start_us {
            self.waveform_playing = false;
            return;
        }
        let dt_us = ctx.input(|input| input.unstable_dt as f64 * 1_000_000.0);
        let step_us = (dt_us * self.waveform_playback_speed.max(0.0)).max(0.0);
        self.waveform_cursor_a_us += step_us;
        if self.waveform_cursor_a_us > end_us {
            self.waveform_cursor_a_us = start_us;
        }
        self.waveform_cursor_b_us = self.waveform_cursor_a_us;
        ctx.request_repaint();
    }

    fn sketch_inspector(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        ui.vertical(|ui| {
            ui.set_min_width(260.0);
            ui.heading("Inspector");
            match &self.selected_sketch_item {
                Some(SketchSelection::Component(id)) => {
                    if let Some(component) = snapshot
                        .components_detail
                        .iter()
                        .find(|item| &item.id == id)
                    {
                        ui.strong(&component.id);
                        let mut model = component.model.clone();
                        ui.label("Model");
                        if ui.text_edit_singleline(&mut model).changed() {
                            self.apply_component_model_edit(&component.id, &model);
                        }

                        let mut part_number = component.part_number.clone().unwrap_or_default();
                        ui.label("Part number");
                        if ui.text_edit_singleline(&mut part_number).changed() {
                            self.apply_component_part_number_edit(&component.id, &part_number);
                        }

                        ui.label(format!("pins: {}", component.pins.len()));
                        egui::ScrollArea::vertical()
                            .max_height(230.0)
                            .show(ui, |ui| {
                                for pin in &component.pins {
                                    ui.monospace(format!("{} -> {}", pin.pin, pin.net));
                                }
                            });
                        ui.separator();
                        ui.label("Pin assignment");
                        if self.pin_edit_net.is_empty()
                            && let Some(net) = snapshot.nets_detail.first()
                        {
                            self.pin_edit_net = net.id.clone();
                        }
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.pin_edit_id);
                            egui::ComboBox::from_id_salt("pin_edit_net")
                                .selected_text(if self.pin_edit_net.is_empty() {
                                    "select net"
                                } else {
                                    &self.pin_edit_net
                                })
                                .show_ui(ui, |ui| {
                                    for net in &snapshot.nets_detail {
                                        ui.selectable_value(
                                            &mut self.pin_edit_net,
                                            net.id.clone(),
                                            &net.id,
                                        );
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Assign Pin").clicked() {
                                self.apply_assign_component_pin(&component.id);
                            }
                            if ui.button("Remove Pin").clicked() {
                                self.apply_remove_component_pin(&component.id);
                            }
                        });
                        ui.separator();
                        ui.label("Visual wire");
                        ui.horizontal(|ui| {
                            ui.label("pin");
                            ui.text_edit_singleline(&mut self.wire_pin_id);
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Start Wire").clicked() {
                                self.wire_from_component = Some(component.id.clone());
                                if self.wire_pin_id.trim().is_empty() {
                                    self.wire_pin_id = self.pin_edit_id.clone();
                                }
                                self.status = format!(
                                    "Wire mode: click a net node to connect {}.{}.",
                                    component.id,
                                    self.wire_pin_id.trim()
                                );
                            }
                            if ui.button("Cancel Wire").clicked() {
                                self.wire_from_component = None;
                            }
                        });
                        if let Some(source) = &self.wire_from_component {
                            ui.label(format!(
                                "Active: {source}.{} -> click a net",
                                self.wire_pin_id.trim()
                            ));
                        }
                        if ui.button("Remove Component").clicked() {
                            self.apply_remove_component(&component.id);
                        }
                    }
                }
                Some(SketchSelection::Net(id)) => {
                    if let Some(net) = snapshot.nets_detail.iter().find(|item| &item.id == id) {
                        ui.strong(&net.id);
                        let mut kind = net.kind.clone();
                        egui::ComboBox::from_label("Kind")
                            .selected_text(&kind)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut kind, "power".to_string(), "power");
                                ui.selectable_value(&mut kind, "ground".to_string(), "ground");
                                ui.selectable_value(
                                    &mut kind,
                                    "digital_or_analog".to_string(),
                                    "digital_or_analog",
                                );
                            });
                        if kind != net.kind {
                            self.apply_net_kind_edit(&net.id, &kind);
                        }

                        ui.horizontal(|ui| {
                            ui.label("Nominal voltage");
                            if let Some(voltage) = net.nominal_voltage {
                                let mut edited = voltage;
                                if ui
                                    .add(egui::DragValue::new(&mut edited).speed(0.1).suffix(" V"))
                                    .changed()
                                {
                                    self.apply_net_nominal_voltage_edit(&net.id, Some(edited));
                                }
                                if ui.button("Clear").clicked() {
                                    self.apply_net_nominal_voltage_edit(&net.id, None);
                                }
                            } else if ui.button("Set").clicked() {
                                self.apply_net_nominal_voltage_edit(&net.id, Some(0.0));
                            }
                        });

                        let mut powered = match net.powered {
                            Some(true) => "true",
                            Some(false) => "false",
                            None => "unset",
                        }
                        .to_string();
                        egui::ComboBox::from_label("Powered")
                            .selected_text(&powered)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut powered, "unset".to_string(), "unset");
                                ui.selectable_value(&mut powered, "true".to_string(), "true");
                                ui.selectable_value(&mut powered, "false".to_string(), "false");
                            });
                        if powered
                            != match net.powered {
                                Some(true) => "true",
                                Some(false) => "false",
                                None => "unset",
                            }
                        {
                            self.apply_net_powered_edit(
                                &net.id,
                                match powered.as_str() {
                                    "true" => Some(true),
                                    "false" => Some(false),
                                    _ => None,
                                },
                            );
                        }

                        ui.label(format!("connections: {}", net.connections.len()));
                        egui::ScrollArea::vertical()
                            .max_height(230.0)
                            .show(ui, |ui| {
                                for connection in &net.connections {
                                    ui.monospace(connection);
                                }
                            });
                        if ui.button("Remove Net").clicked() {
                            self.apply_remove_net(&net.id);
                        }
                    }
                }
                Some(SketchSelection::Overflow(label)) => {
                    ui.strong(label);
                    ui.label("Only the first visible rows are drawn to keep the graph readable.");
                    ui.label("Use the YAML editor for the complete project.");
                }
                None => {
                    ui.label("Select a component or net in the graph.");
                    ui.label(format!(
                        "{} components, {} nets",
                        snapshot.components_detail.len(),
                        snapshot.nets_detail.len()
                    ));
                }
            }
        });
    }

    fn apply_component_model_edit(&mut self, component_id: &str, model: &str) {
        match edit_component_model(&self.project_yaml, component_id, model) {
            Ok(updated) => self.apply_edited_project_yaml(updated, "Component model updated."),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_component(&mut self) {
        match add_component(
            &self.project_yaml,
            &self.new_component_id,
            &self.new_component_model,
        ) {
            Ok(updated) => {
                let component_id = self.new_component_id.trim().to_string();
                self.selected_sketch_item = Some(SketchSelection::Component(component_id.clone()));
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Component {component_id} added."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_component(&mut self, component_id: &str) {
        match remove_component(&self.project_yaml, component_id) {
            Ok(updated) => {
                self.selected_sketch_item = None;
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Component {component_id} removed."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_component_part_number_edit(&mut self, component_id: &str, part_number: &str) {
        match edit_component_part_number(&self.project_yaml, component_id, part_number) {
            Ok(updated) => {
                self.apply_edited_project_yaml(updated, "Component part number updated.")
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_assign_component_pin(&mut self, component_id: &str) {
        match assign_component_pin(
            &self.project_yaml,
            component_id,
            &self.pin_edit_id,
            &self.pin_edit_net,
        ) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Component {component_id} pin {} assigned to {}.",
                    self.pin_edit_id.trim(),
                    self.pin_edit_net.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_component_pin(&mut self, component_id: &str) {
        match remove_component_pin(&self.project_yaml, component_id, &self.pin_edit_id) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Component {component_id} pin {} removed.",
                    self.pin_edit_id.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_visual_wire(&mut self, component_id: String, net_id: String) {
        match assign_component_pin(
            &self.project_yaml,
            &component_id,
            &self.wire_pin_id,
            &net_id,
        ) {
            Ok(updated) => {
                self.pin_edit_id = self.wire_pin_id.clone();
                self.pin_edit_net = net_id.clone();
                self.wire_from_component = None;
                self.selected_sketch_item = Some(SketchSelection::Component(component_id.clone()));
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Visual wire assigned {component_id}.{} to {net_id}.",
                        self.wire_pin_id.trim()
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_schematic_node_position(&mut self, selection: SketchSelection, x: f64, y: f64) {
        match edit_schematic_node_position(&self.project_yaml, &selection, x, y) {
            Ok(updated) => {
                self.selected_sketch_item = Some(selection);
                self.apply_edited_project_yaml(updated, "Schematic node position updated.");
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_net(&mut self) {
        match add_net(&self.project_yaml, &self.new_net_id, &self.new_net_kind) {
            Ok(updated) => {
                let net_id = self.new_net_id.trim().to_string();
                self.selected_sketch_item = Some(SketchSelection::Net(net_id.clone()));
                self.apply_edited_project_yaml(updated, &format!("Net {net_id} added."));
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_net(&mut self, net_id: &str) {
        match remove_net(&self.project_yaml, net_id) {
            Ok(updated) => {
                self.selected_sketch_item = None;
                self.apply_edited_project_yaml(updated, &format!("Net {net_id} removed."));
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_net_kind_edit(&mut self, net_id: &str, kind: &str) {
        match edit_net_kind(&self.project_yaml, net_id, kind) {
            Ok(updated) => self.apply_edited_project_yaml(updated, "Net kind updated."),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_net_nominal_voltage_edit(&mut self, net_id: &str, voltage: Option<f64>) {
        match edit_net_nominal_voltage(&self.project_yaml, net_id, voltage) {
            Ok(updated) => self.apply_edited_project_yaml(updated, "Net nominal voltage updated."),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_net_powered_edit(&mut self, net_id: &str, powered: Option<bool>) {
        match edit_net_powered(&self.project_yaml, net_id, powered) {
            Ok(updated) => self.apply_edited_project_yaml(updated, "Net powered flag updated."),
            Err(error) => self.record_error(error),
        }
    }
}

fn validate_from_gui(
    project_path: &Path,
    profile: &str,
    output: &Path,
) -> Result<(ValidationReport, String)> {
    let command = format!(
        "circuitci-gui validate {} --profile {} --output {}",
        display_path(project_path),
        profile,
        display_path(output)
    );
    let report =
        crate::suite::validate_and_write_project_report(project_path, profile, output, command)?;
    let markdown = std::fs::read_to_string(output.join("report.md"))
        .with_context(|| format!("Failed to read {}.", output.join("report.md").display()))?;
    Ok((report, markdown))
}

fn suggest_from_gui(project_path: &Path, profile: &str) -> Result<String> {
    let project = crate::board_ir::load_project(project_path)?;
    let (library, library_findings) = crate::library::load_library(project_path, &project);
    let bound = crate::library::bind_project(&project, library, library_findings);
    let profile = if profile.trim().is_empty() || profile == "default" {
        None
    } else {
        Some(profile)
    };
    let report = crate::scenario_suggestions::suggest_scenarios_for_profile(&bound, profile);
    serde_yaml_ng::to_string(&report).context("Failed to serialize scenario suggestions.")
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

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn sketch_hover_tooltip(ui: &mut egui::Ui, node: &sketch::SketchNode, runtime_lines: &[String]) {
    ui.strong(&node.label);
    ui.label(&node.detail);
    ui.separator();
    ui.label("Runtime probes");
    if runtime_lines.is_empty() {
        ui.label("No matching waveform probe is loaded for this node.");
    } else {
        for line in runtime_lines {
            ui.monospace(line);
        }
    }
}

fn sketch_pin_hover_tooltip(ui: &mut egui::Ui, anchor: &sketch::SketchPinAnchor) {
    ui.strong(format!("{}.{}", anchor.component_id, anchor.pin));
    ui.label(format!("net: {}", anchor.net));
    ui.separator();
    ui.label("Click this pin, then click a net node to rewire it.");
}

fn wire_preview_start(
    graph: &sketch::SketchGraph,
    component_id: &str,
    pin_id: &str,
) -> Option<egui::Pos2> {
    let pin_id = pin_id.trim();
    graph
        .pin_anchors
        .iter()
        .find(|anchor| anchor.component_id == component_id && anchor.pin == pin_id)
        .map(|anchor| anchor.pos)
        .or_else(|| {
            graph
                .nodes
                .iter()
                .find(|node| node.selection == SketchSelection::Component(component_id.to_string()))
                .map(|node| node.rect.center())
        })
}

#[cfg(test)]
mod tests {
    use super::egui;
    use super::sketch::{
        ProjectSnapshot, SketchComponent, SketchNet, SketchPin, edit_component_model,
        edit_component_part_number, edit_net_kind, edit_net_nominal_voltage, edit_net_powered,
        layout_sketch_graph, validate_board_ir_yaml_text,
    };

    fn editable_project_yaml() -> &'static str {
        "project:
  name: gui_editor_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins:
        A: net_a
        B: gnd
  nets:
    net_a:
      kind: digital_or_analog
    gnd:
      kind: ground
"
    }

    #[test]
    fn board_ir_editor_accepts_minimal_project_yaml() {
        validate_board_ir_yaml_text(
            "project:
  name: gui_editor_test
  version: 0.1.0
board:
  components: {}
  nets: {}
",
        )
        .unwrap();
    }

    #[test]
    fn board_ir_editor_rejects_invalid_project_yaml() {
        let error = validate_board_ir_yaml_text(
            "project:
  name: gui_editor_test
",
        )
        .unwrap_err();
        assert!(error.to_string().contains("Board IR"));
    }

    #[test]
    fn board_ir_component_form_edits_emit_valid_yaml() {
        let edited =
            edit_component_model(editable_project_yaml(), "R1", "vendor.test.resistor").unwrap();
        let edited = edit_component_part_number(&edited, "R1", "RC0603FR-0710KL").unwrap();
        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("vendor.test.resistor"));
        assert!(edited.contains("RC0603FR-0710KL"));
    }

    #[test]
    fn board_ir_net_form_edits_emit_valid_yaml() {
        let edited = edit_net_kind(editable_project_yaml(), "net_a", "power").unwrap();
        let edited = edit_net_nominal_voltage(&edited, "net_a", Some(3.3)).unwrap();
        let edited = edit_net_powered(&edited, "net_a", Some(true)).unwrap();
        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("kind: power"));
        assert!(edited.contains("nominal_voltage: 3.3"));
        assert!(edited.contains("powered: true"));
    }

    #[test]
    fn sketch_graph_layout_connects_component_to_net() {
        let snapshot = ProjectSnapshot {
            name: "graph".to_string(),
            components: 1,
            nets: 1,
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![SketchComponent {
                id: "R1".to_string(),
                model: "generic.analog.resistor".to_string(),
                part_number: None,
                position: None,
                pins: vec![SketchPin {
                    pin: "A".to_string(),
                    net: "net_a".to_string(),
                }],
            }],
            nets_detail: vec![SketchNet {
                id: "net_a".to_string(),
                kind: "DigitalOrAnalog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["R1.A".to_string()],
                position: None,
            }],
        };
        let graph = layout_sketch_graph(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0)),
            &snapshot,
        );
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
    }
}
