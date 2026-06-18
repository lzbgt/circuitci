use crate::reports::{Finding, Limitation, ValidationReport};
use anyhow::{Context, Result};
use eframe::egui;
use std::collections::BTreeSet;
use std::path::Path;

mod analog;
#[cfg(test)]
mod analog_assertion_edit_tests;
mod import_flow;
mod library;
mod project;
mod simulation;
mod sketch;
mod sketch_actions;
mod sketch_inspector;
mod sketch_probes;
mod sketch_symbols;
#[cfg(test)]
mod sketch_tests;
mod spice;

use simulation::{
    WaveformView, load_report_waveforms, runtime_probe_activity_for_selection,
    runtime_probe_lines_for_selection, waveform_probe_value_for_badge,
    waveform_time_range_for_view,
};
use sketch::{
    DEFAULT_SKETCH_GRID_STEP, ProjectSnapshot, SketchSelection, draw_sketch_grid, draw_sketch_node,
    draw_sketch_pin_anchor, edge_label_position, hit_test_wire, layout_sketch_graph_viewport,
    orthogonal_wire_points, persisted_node_position_from_screen_with_snap,
    snap_screen_point_to_grid,
};
use sketch_inspector::{
    default_current_probe_name_for_component, default_power_probe_name_for_component,
    default_probe_name_for_net,
};
use sketch_probes::{
    SketchProbeBadge, SketchProbeStatus, draw_probe_badge, hit_test_probe_badge,
    probe_assertion_status,
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum SketchGroupAction {
    Nudge(egui::Vec2),
    AlignLeft,
    AlignTop,
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
    analog_assertion_edit_original: String,
    analog_assertion_probe: String,
    analog_assertion_aggregation: String,
    analog_assertion_relation: String,
    analog_assertion_threshold: f64,
    analog_assertion_at_us: f64,
    analog_assertion_start_us: f64,
    analog_assertion_end_us: f64,
    analog_probe_scenario: String,
    analog_canvas_probe_name: String,
    analog_canvas_component_probe_name: String,
    analog_canvas_component_power_probe_name: String,
    project_snapshot: Option<ProjectSnapshot>,
    selected_sketch_item: Option<SketchSelection>,
    selected_sketch_items: BTreeSet<SketchSelection>,
    marquee_start: Option<egui::Pos2>,
    sketch_fit_requested: bool,
    sketch_group_action: Option<SketchGroupAction>,
    sketch_zoom: f32,
    sketch_pan: egui::Vec2,
    sketch_grid_enabled: bool,
    sketch_snap_enabled: bool,
    sketch_grid_step: f32,
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
            analog_assertion_edit_original: String::new(),
            analog_assertion_probe: String::new(),
            analog_assertion_aggregation: "sample".to_string(),
            analog_assertion_relation: "above".to_string(),
            analog_assertion_threshold: 0.0,
            analog_assertion_at_us: 50.0,
            analog_assertion_start_us: 0.0,
            analog_assertion_end_us: 100.0,
            analog_probe_scenario: String::new(),
            analog_canvas_probe_name: String::new(),
            analog_canvas_component_probe_name: String::new(),
            analog_canvas_component_power_probe_name: String::new(),
            project_snapshot: None,
            selected_sketch_item: None,
            selected_sketch_items: BTreeSet::new(),
            marquee_start: None,
            sketch_fit_requested: false,
            sketch_group_action: None,
            sketch_zoom: 1.0,
            sketch_pan: egui::Vec2::ZERO,
            sketch_grid_enabled: true,
            sketch_snap_enabled: true,
            sketch_grid_step: DEFAULT_SKETCH_GRID_STEP,
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
            ui.horizontal(|ui| {
                if ui.button("-").clicked() {
                    self.sketch_zoom = (self.sketch_zoom / 1.2).clamp(0.25, 4.0);
                }
                ui.add(
                    egui::Slider::new(&mut self.sketch_zoom, 0.25..=4.0)
                        .logarithmic(true)
                        .text("zoom"),
                );
                if ui.button("+").clicked() {
                    self.sketch_zoom = (self.sketch_zoom * 1.2).clamp(0.25, 4.0);
                }
                if ui.button("Reset View").clicked() {
                    self.sketch_zoom = 1.0;
                    self.sketch_pan = egui::Vec2::ZERO;
                }
                if ui.button("Fit Content").clicked() {
                    self.sketch_fit_requested = true;
                }
                if ui.button("Reset Pan").clicked() {
                    self.sketch_pan = egui::Vec2::ZERO;
                }
                ui.checkbox(&mut self.sketch_grid_enabled, "Grid");
                ui.checkbox(&mut self.sketch_snap_enabled, "Snap");
                ui.add(
                    egui::DragValue::new(&mut self.sketch_grid_step)
                        .range(4.0..=96.0)
                        .speed(1.0)
                        .suffix(" grid"),
                );
                if ui
                    .add_enabled(
                        self.has_deletable_sketch_selection(),
                        egui::Button::new("Delete Selected"),
                    )
                    .clicked()
                {
                    self.apply_delete_selected_sketch_item();
                }
                ui.label(
                    "Middle/right drag pans; Shift+drag marquee selects; snap affects schematic node placement.",
                );
            });
            if self.selected_sketch_items.len() > 1 {
                ui.horizontal(|ui| {
                    ui.label(format!("{} selected", self.selected_sketch_items.len()));
                    if ui.button("Nudge Left").clicked() {
                        self.sketch_group_action =
                            Some(SketchGroupAction::Nudge(egui::vec2(-16.0, 0.0)));
                    }
                    if ui.button("Nudge Right").clicked() {
                        self.sketch_group_action =
                            Some(SketchGroupAction::Nudge(egui::vec2(16.0, 0.0)));
                    }
                    if ui.button("Nudge Up").clicked() {
                        self.sketch_group_action =
                            Some(SketchGroupAction::Nudge(egui::vec2(0.0, -16.0)));
                    }
                    if ui.button("Nudge Down").clicked() {
                        self.sketch_group_action =
                            Some(SketchGroupAction::Nudge(egui::vec2(0.0, 16.0)));
                    }
                    if ui.button("Align Left").clicked() {
                        self.sketch_group_action = Some(SketchGroupAction::AlignLeft);
                    }
                    if ui.button("Align Top").clicked() {
                        self.sketch_group_action = Some(SketchGroupAction::AlignTop);
                    }
                });
            }
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

        if self.sketch_fit_requested {
            self.fit_sketch_content(rect, snapshot);
            self.sketch_fit_requested = false;
        }
        self.handle_sketch_viewport_input(ui, rect, &response);
        let viewport = self.sketch_viewport();
        draw_sketch_grid(
            &painter,
            rect,
            viewport,
            self.sketch_grid_enabled,
            self.sketch_grid_step,
        );
        let graph = layout_sketch_graph_viewport(rect, snapshot, viewport);
        if let Some(action) = self.sketch_group_action.take() {
            self.apply_sketch_group_action(rect, &graph, viewport, action);
        }
        let pointer_hover = if response.hovered() {
            ui.ctx().pointer_hover_pos()
        } else {
            None
        };
        let hovered_node = pointer_hover
            .and_then(|position| graph.nodes.iter().find(|node| node.rect.contains(position)));
        let hovered_anchor = pointer_hover.and_then(|position| {
            graph
                .pin_anchors
                .iter()
                .find(|anchor| anchor.pos.distance(position) <= 8.0)
        });
        let hovered_wire = if hovered_node.is_none() && hovered_anchor.is_none() {
            pointer_hover.and_then(|position| hit_test_wire(&graph, position))
        } else {
            None
        };
        let hovered_probe_badge =
            pointer_hover.and_then(|position| hit_test_probe_badge(&graph.probe_badges, position));
        for edge in &graph.edges {
            let wire_selection = SketchSelection::Net(edge.net_id.clone());
            let selected = self.selection_is_selected(&wire_selection);
            let hovered = hovered_wire
                .is_some_and(|wire| wire.net_id == edge.net_id && wire.source == edge.source);
            draw_wire_edge(&painter, edge, selected, hovered, self.sketch_zoom);
        }
        if let Some(component_id) = &self.wire_from_component
            && let Some(pointer) = ui.ctx().pointer_hover_pos()
            && rect.contains(pointer)
            && let Some(source) = wire_preview_start(&graph, component_id, &self.wire_pin_id)
        {
            let pointer = snap_screen_point_to_grid(
                rect,
                pointer,
                viewport,
                self.sketch_snap_enabled,
                self.sketch_grid_step,
            );
            draw_wire_polyline(
                &painter,
                source,
                pointer,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 196, 87)),
            );
        }
        for node in &graph.nodes {
            let selected = self.selection_is_selected(&node.selection);
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
        for badge in &graph.probe_badges {
            let hovered = hovered_probe_badge.is_some_and(|hovered| {
                hovered.probe.scenario_name == badge.probe.scenario_name
                    && hovered.probe.probe_name == badge.probe.probe_name
            });
            let status = probe_assertion_status(self.report.as_ref(), &badge.probe);
            draw_probe_badge(&painter, badge, hovered, status);
        }

        if response.clicked_by(egui::PointerButton::Primary)
            && let Some(position) = response.interact_pointer_pos()
        {
            let multi_select = ui.input(|input| input.modifiers.shift || input.modifiers.command);
            let clicked_probe_badge = hit_test_probe_badge(&graph.probe_badges, position);
            let clicked_anchor = graph
                .pin_anchors
                .iter()
                .find(|anchor| anchor.pos.distance(position) <= 8.0);
            let clicked = graph
                .nodes
                .iter()
                .find(|node| node.rect.contains(position))
                .map(|node| node.selection.clone());
            let clicked_wire = if clicked_anchor.is_none() && clicked.is_none() {
                hit_test_wire(&graph, position)
            } else {
                None
            };
            if let Some(badge) = clicked_probe_badge {
                self.open_probe_badge_in_simulation(badge);
            } else if let Some(anchor) = clicked_anchor {
                if let Some(source_component_id) = self.wire_from_component.clone()
                    && !(source_component_id == anchor.component_id
                        && self.wire_pin_id.trim() == anchor.pin)
                {
                    self.apply_visual_pin_wire(
                        source_component_id,
                        anchor.component_id.clone(),
                        anchor.pin.clone(),
                    );
                } else {
                    self.set_single_sketch_selection(Some(SketchSelection::Component(
                        anchor.component_id.clone(),
                    )));
                    self.pin_edit_id = anchor.pin.clone();
                    self.pin_edit_net = anchor.net.clone();
                    self.wire_pin_id = anchor.pin.clone();
                    self.wire_from_component = Some(anchor.component_id.clone());
                    self.status = format!(
                        "Wire mode: click another pin or net node to connect {}.{}.",
                        anchor.component_id, anchor.pin
                    );
                }
            } else if let Some(SketchSelection::Net(net_id)) = &clicked
                && let Some(component_id) = self.wire_from_component.clone()
            {
                self.apply_visual_wire(component_id, net_id.clone());
            } else if let Some(edge) = clicked_wire
                && let Some(component_id) = self.wire_from_component.clone()
            {
                self.apply_visual_wire(component_id, edge.net_id.clone());
            } else if multi_select {
                if let Some(selection) = clicked {
                    self.toggle_sketch_selection(selection);
                } else if let Some(edge) = clicked_wire {
                    self.toggle_sketch_selection(SketchSelection::Net(edge.net_id.clone()));
                }
            } else if let Some(edge) = clicked_wire {
                self.set_single_sketch_selection(Some(SketchSelection::Net(edge.net_id.clone())));
                self.status = format!("Selected net {} from wire {}.", edge.net_id, edge.source);
            } else {
                self.set_single_sketch_selection(clicked);
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(position) = response.interact_pointer_pos()
        {
            let clicked_node = graph
                .nodes
                .iter()
                .find(|node| node.rect.contains(position))
                .map(|node| node.selection.clone());
            if clicked_node.is_none() && ui.input(|input| input.modifiers.shift) {
                self.marquee_start = Some(position);
            } else if clicked_node.is_some() {
                let multi_selected_hit = clicked_node
                    .as_ref()
                    .is_some_and(|selection| self.selected_sketch_items.contains(selection));
                if !multi_selected_hit {
                    self.set_single_sketch_selection(clicked_node);
                }
            }
        }
        if let Some(start) = self.marquee_start
            && let Some(current) = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().pointer_hover_pos())
        {
            let marquee = egui::Rect::from_two_pos(start, current);
            painter.rect_filled(
                marquee,
                0.0,
                egui::Color32::from_rgba_unmultiplied(93, 185, 255, 24),
            );
            painter.rect_stroke(
                marquee,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(93, 185, 255)),
                egui::StrokeKind::Inside,
            );
        } else if response.dragged_by(egui::PointerButton::Primary)
            && let (Some(selection), Some(position)) = (
                self.selected_sketch_item.clone(),
                response.interact_pointer_pos(),
            )
            && !matches!(selection, SketchSelection::Overflow(_))
            && let Some(node) = graph.nodes.iter().find(|node| node.selection == selection)
        {
            if self.selected_sketch_items.len() > 1
                && self.selected_sketch_items.contains(&selection)
            {
                let delta = ui.input(|input| input.pointer.delta());
                self.apply_selected_schematic_screen_delta(
                    rect,
                    &graph,
                    viewport,
                    delta,
                    "Selected sketch items moved.",
                );
            } else {
                let (x, y) = persisted_node_position_from_screen_with_snap(
                    rect,
                    position,
                    node.rect,
                    viewport,
                    self.sketch_snap_enabled,
                    self.sketch_grid_step,
                );
                self.apply_schematic_node_position(selection, x, y);
            }
        }

        if response.drag_stopped_by(egui::PointerButton::Primary)
            && let Some(start) = self.marquee_start.take()
            && let Some(end) = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().pointer_hover_pos())
        {
            self.apply_marquee_selection(egui::Rect::from_two_pos(start, end), &graph);
        }

        let delete_pressed = response.hovered()
            && ui.input(|input| {
                input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace)
            });
        let quick_above_pressed = response.hovered()
            && ui.input(|input| input.modifiers.shift && input.key_pressed(egui::Key::A));
        let quick_below_pressed = response.hovered()
            && ui.input(|input| input.modifiers.shift && input.key_pressed(egui::Key::B));
        let add_assertion_pressed = response.hovered()
            && ui.input(|input| !input.modifiers.shift && input.key_pressed(egui::Key::A));
        let clear_assertions_pressed =
            response.hovered() && ui.input(|input| input.key_pressed(egui::Key::X));
        if let Some(badge) = hovered_probe_badge {
            if quick_above_pressed {
                self.apply_quick_canvas_probe_assertion(&badge.probe, "above");
            } else if quick_below_pressed {
                self.apply_quick_canvas_probe_assertion(&badge.probe, "below");
            } else if add_assertion_pressed {
                self.apply_add_canvas_probe_assertion(
                    &badge.probe.scenario_name,
                    &badge.probe.probe_name,
                );
            } else if clear_assertions_pressed {
                self.apply_remove_canvas_probe_assertions(
                    &badge.probe.scenario_name,
                    &badge.probe.probe_name,
                );
            } else if delete_pressed {
                self.apply_remove_canvas_probe(&badge.probe.scenario_name, &badge.probe.probe_name);
            }
        } else if delete_pressed {
            self.apply_delete_selected_sketch_item();
        }

        if let Some(badge) = hovered_probe_badge {
            let sampled_value = waveform_probe_value_for_badge(
                &self.waveforms,
                self.selected_waveform,
                self.waveform_cursor_a_us,
                &badge.probe,
            );
            response.context_menu(|ui| {
                self.probe_badge_context_menu(ui, badge, sampled_value);
            });
            response.on_hover_ui(|ui| {
                let status = probe_assertion_status(self.report.as_ref(), &badge.probe);
                sketch_probe_badge_tooltip(ui, badge, status, sampled_value);
            });
        } else if let Some(node) = hovered_node {
            response.context_menu(|ui| {
                self.sketch_node_context_menu(ui, node, snapshot);
            });
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
        } else if let Some(anchor) = hovered_anchor {
            response.on_hover_ui(|ui| {
                sketch_pin_hover_tooltip(ui, anchor);
            });
        } else if let Some(edge) = hovered_wire {
            response.context_menu(|ui| {
                self.sketch_wire_context_menu(ui, edge);
            });
            response.on_hover_ui(|ui| {
                sketch_wire_hover_tooltip(ui, edge);
            });
        }
    }

    fn open_probe_badge_in_simulation(&mut self, badge: &SketchProbeBadge) {
        self.analog_probe_scenario = badge.probe.scenario_name.clone();
        self.analog_assertion_scenario = badge.probe.scenario_name.clone();
        self.analog_assertion_probe = badge.probe.probe_name.clone();
        self.stage = Stage::Simulation;
        self.status = format!(
            "Selected {} probe {} from scenario {}.",
            badge.probe.quantity.label(),
            badge.probe.probe_name,
            badge.probe.scenario_name
        );
    }

    fn probe_badge_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        badge: &SketchProbeBadge,
        sampled_value: Option<f64>,
    ) {
        ui.strong(format!(
            "{} probe {}",
            badge.probe.quantity.label(),
            badge.probe.probe_name
        ));
        if ui.button("Open in Simulation").clicked() {
            self.open_probe_badge_in_simulation(badge);
            ui.close();
        }
        ui.separator();
        if ui.button("Add Assertion From Settings").clicked() {
            self.apply_add_canvas_probe_assertion(
                &badge.probe.scenario_name,
                &badge.probe.probe_name,
            );
            ui.close();
        }
        if ui
            .add_enabled(
                sampled_value.is_some(),
                egui::Button::new("Quick Require Above Cursor Sample"),
            )
            .clicked()
        {
            self.apply_quick_canvas_probe_assertion(&badge.probe, "above");
            ui.close();
        }
        if ui
            .add_enabled(
                sampled_value.is_some(),
                egui::Button::new("Quick Require Below Cursor Sample"),
            )
            .clicked()
        {
            self.apply_quick_canvas_probe_assertion(&badge.probe, "below");
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(
                !badge.probe.assertion_names.is_empty(),
                egui::Button::new("Clear Probe Assertions"),
            )
            .clicked()
        {
            self.apply_remove_canvas_probe_assertions(
                &badge.probe.scenario_name,
                &badge.probe.probe_name,
            );
            ui.close();
        }
        if ui.button("Remove Probe").clicked() {
            self.apply_remove_canvas_probe(&badge.probe.scenario_name, &badge.probe.probe_name);
            ui.close();
        }
    }

    fn sketch_node_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        node: &sketch::SketchNode,
        snapshot: &ProjectSnapshot,
    ) {
        match &node.selection {
            SketchSelection::Component(component_id) => {
                ui.strong(format!("Component {component_id}"));
                if ui.button("Inspect Component").clicked() {
                    self.set_single_sketch_selection(Some(node.selection.clone()));
                    ui.close();
                }
                if ui.button("Start Wire From Pin").clicked() {
                    let (pin, net) =
                        component_context_pin(snapshot, component_id, &self.wire_pin_id);
                    self.set_single_sketch_selection(Some(node.selection.clone()));
                    self.pin_edit_id = pin.clone();
                    self.pin_edit_net = net;
                    self.wire_pin_id = pin.clone();
                    self.wire_from_component = Some(component_id.clone());
                    self.status = format!(
                        "Wire mode: click another pin, net, or wire to connect {component_id}.{pin}."
                    );
                    ui.close();
                }
                ui.separator();
                if ui.button("Add Current Probe").clicked() {
                    self.ensure_component_probe_defaults(component_id);
                    self.apply_add_current_probe_for_component(component_id);
                    ui.close();
                }
                if ui.button("Add Power Probe").clicked() {
                    self.ensure_component_probe_defaults(component_id);
                    self.apply_add_power_probe_for_component(component_id);
                    ui.close();
                }
                ui.separator();
                if ui.button("Delete Component").clicked() {
                    self.set_single_sketch_selection(Some(node.selection.clone()));
                    self.apply_delete_selected_sketch_item();
                    ui.close();
                }
            }
            SketchSelection::Net(net_id) => {
                ui.strong(format!("Net {net_id}"));
                self.net_context_menu(ui, net_id, "Inspect Net", "Delete Net");
            }
            SketchSelection::Overflow(label) => {
                ui.strong(label);
                ui.label("Open the YAML editor or use Fit Content for hidden graph items.");
            }
        }
    }

    fn sketch_wire_context_menu(&mut self, ui: &mut egui::Ui, edge: &sketch::SketchEdge) {
        ui.strong(format!("Wire {}", edge.net_id));
        ui.label(format!("source: {}", edge.source));
        self.net_context_menu(ui, &edge.net_id, "Inspect Wire Net", "Delete Wire Net");
    }

    fn net_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        net_id: &str,
        inspect_label: &str,
        delete_label: &str,
    ) {
        if ui.button(inspect_label).clicked() {
            self.set_single_sketch_selection(Some(SketchSelection::Net(net_id.to_string())));
            ui.close();
        }
        if let Some(component_id) = self.wire_from_component.clone()
            && ui.button("Connect Active Wire Here").clicked()
        {
            self.apply_visual_wire(component_id, net_id.to_string());
            ui.close();
        }
        ui.separator();
        if ui.button("Add Voltage Probe").clicked() {
            self.ensure_net_probe_defaults(net_id);
            self.apply_add_voltage_probe_for_net(net_id);
            ui.close();
        }
        ui.separator();
        if ui.button(delete_label).clicked() {
            self.set_single_sketch_selection(Some(SketchSelection::Net(net_id.to_string())));
            self.apply_delete_selected_sketch_item();
            ui.close();
        }
    }

    fn ensure_net_probe_defaults(&mut self, net_id: &str) {
        self.ensure_canvas_probe_scenario();
        if self.analog_canvas_probe_name.trim().is_empty() {
            self.analog_canvas_probe_name = default_probe_name_for_net(net_id);
        }
    }

    fn ensure_component_probe_defaults(&mut self, component_id: &str) {
        self.ensure_canvas_probe_scenario();
        if self.analog_canvas_component_probe_name.trim().is_empty() {
            self.analog_canvas_component_probe_name =
                default_current_probe_name_for_component(component_id);
        }
        if self
            .analog_canvas_component_power_probe_name
            .trim()
            .is_empty()
        {
            self.analog_canvas_component_power_probe_name =
                default_power_probe_name_for_component(component_id);
        }
    }

    fn ensure_canvas_probe_scenario(&mut self) {
        let Ok(choices) = analog::analog_scenario_choices(&self.project_yaml) else {
            return;
        };
        if (self.analog_probe_scenario.is_empty()
            || !choices
                .iter()
                .any(|choice| choice.name == self.analog_probe_scenario))
            && let Some(choice) = choices.first()
        {
            self.analog_probe_scenario = choice.name.clone();
        }
    }

    fn handle_sketch_viewport_input(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        response: &egui::Response,
    ) {
        if response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
        {
            let delta = ui.input(|input| input.pointer.delta());
            self.sketch_pan += delta;
        }

        if response.hovered() {
            let zoom_delta = ui.input(|input| input.zoom_delta());
            if (zoom_delta - 1.0).abs() > f32::EPSILON {
                self.zoom_sketch_canvas(zoom_delta, rect, rect.center());
            }
        }
    }

    fn zoom_sketch_canvas(&mut self, zoom_delta: f32, canvas: egui::Rect, focus: egui::Pos2) {
        let old_zoom = self.sketch_zoom.clamp(0.25, 4.0);
        let new_zoom = (old_zoom * zoom_delta).clamp(0.25, 4.0);
        if (new_zoom - old_zoom).abs() <= f32::EPSILON {
            return;
        }
        let focus_offset = focus - canvas.min;
        let logical_focus = (focus_offset - self.sketch_pan) / old_zoom;
        self.sketch_pan = focus_offset - logical_focus * new_zoom;
        self.sketch_zoom = new_zoom;
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
    ui.label("Click this pin, then click another pin or net node to wire it.");
}

fn sketch_wire_hover_tooltip(ui: &mut egui::Ui, edge: &sketch::SketchEdge) {
    ui.strong(format!("net {}", edge.net_id));
    ui.label(format!("source: {}", edge.source));
    ui.separator();
    ui.label("Click this wire to select the net; start wire mode first to connect to it.");
}

fn component_context_pin(
    snapshot: &ProjectSnapshot,
    component_id: &str,
    preferred_pin: &str,
) -> (String, String) {
    let Some(component) = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == component_id)
    else {
        return ("P1".to_string(), String::new());
    };
    if let Some(pin) = component
        .pins
        .iter()
        .find(|pin| pin.pin == preferred_pin.trim())
    {
        return (pin.pin.clone(), pin.net.clone());
    }
    component
        .pins
        .first()
        .map(|pin| (pin.pin.clone(), pin.net.clone()))
        .unwrap_or_else(|| ("P1".to_string(), String::new()))
}

fn sketch_probe_badge_tooltip(
    ui: &mut egui::Ui,
    badge: &SketchProbeBadge,
    status: SketchProbeStatus,
    sampled_value: Option<f64>,
) {
    ui.strong(format!(
        "{} probe {}",
        badge.probe.quantity.label(),
        badge.probe.probe_name
    ));
    ui.label(format!("scenario: {}", badge.probe.scenario_name));
    ui.label(format!("expression: {}", badge.probe.expression));
    ui.label(format!("assertion status: {}", status.label()));
    if let Some(value) = sampled_value {
        ui.label(format!("cursor sample: {:.6}", value));
    } else {
        ui.label("cursor sample: no matching loaded waveform");
    }
    if !badge.probe.assertion_names.is_empty() {
        ui.label(format!(
            "assertions: {}",
            badge.probe.assertion_names.join(", ")
        ));
    }
    ui.separator();
    ui.label("Click to open this probe in the Simulation stage.");
    ui.label("Right-click to open probe actions.");
    ui.label("Press A while hovering to add an assertion from current settings.");
    ui.label("Press Shift+A while hovering to require above the cursor sample.");
    ui.label("Press Shift+B while hovering to require below the cursor sample.");
    ui.label("Press X while hovering to clear assertions for this probe.");
    ui.label("Press Delete or Backspace while hovering to remove it.");
}

fn draw_wire_edge(
    painter: &egui::Painter,
    edge: &sketch::SketchEdge,
    selected: bool,
    hovered: bool,
    zoom: f32,
) {
    let color = if selected {
        egui::Color32::from_rgb(93, 185, 255)
    } else if hovered {
        egui::Color32::from_rgb(255, 196, 87)
    } else {
        egui::Color32::from_gray(86)
    };
    let stroke_width = if selected || hovered { 2.0 } else { 1.0 };
    let stroke = egui::Stroke::new(stroke_width, color);
    let points = orthogonal_wire_points(edge.start, edge.end);
    draw_wire_points(painter, &points, stroke);
    draw_wire_junctions(painter, &points, color, selected || hovered);
    if zoom > 0.45 || selected || hovered {
        draw_wire_label(painter, edge, selected || hovered);
    }
}

fn draw_wire_polyline(
    painter: &egui::Painter,
    start: egui::Pos2,
    end: egui::Pos2,
    stroke: egui::Stroke,
) {
    let points = orthogonal_wire_points(start, end);
    draw_wire_points(painter, &points, stroke);
}

fn draw_wire_points(painter: &egui::Painter, points: &[egui::Pos2], stroke: egui::Stroke) {
    for segment in points.windows(2) {
        painter.line_segment([segment[0], segment[1]], stroke);
    }
}

fn draw_wire_junctions(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    color: egui::Color32,
    emphasized: bool,
) {
    let radius = if emphasized { 3.0 } else { 2.2 };
    for point in points {
        painter.circle_filled(*point, radius, color);
    }
}

fn draw_wire_label(painter: &egui::Painter, edge: &sketch::SketchEdge, emphasized: bool) {
    let label = compact_wire_label(&edge.net_id);
    let pos = edge_label_position(edge);
    let width = (label.len() as f32 * 7.0 + 8.0).clamp(24.0, 128.0);
    let rect = egui::Rect::from_min_size(pos + egui::vec2(-4.0, -9.0), egui::vec2(width, 18.0));
    let fill = if emphasized {
        egui::Color32::from_rgba_unmultiplied(30, 48, 58, 232)
    } else {
        egui::Color32::from_rgba_unmultiplied(24, 24, 24, 210)
    };
    painter.rect_filled(rect, 2.0, fill);
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(76)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(10.0),
        egui::Color32::from_gray(225),
    );
}

fn compact_wire_label(label: &str) -> String {
    const MAX_CHARS: usize = 18;
    if label.chars().count() <= MAX_CHARS {
        return label.to_string();
    }
    let mut compact = label.chars().take(MAX_CHARS - 3).collect::<String>();
    compact.push_str("...");
    compact
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
    use super::component_context_pin;
    use super::egui;
    use super::sketch::{
        ProjectSnapshot, SketchComponent, SketchNet, SketchNodeStyle, SketchPin,
        edit_component_model, edit_component_part_number, edit_net_kind, edit_net_nominal_voltage,
        edit_net_powered, layout_sketch_graph, validate_board_ir_yaml_text,
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
    fn component_context_pin_prefers_existing_wire_pin_then_first_pin() {
        let snapshot = ProjectSnapshot {
            name: "graph".to_string(),
            components: 1,
            nets: 2,
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![SketchComponent {
                id: "U1".to_string(),
                model: "generic.ic".to_string(),
                part_number: None,
                position: None,
                pins: vec![
                    SketchPin {
                        pin: "VCC".to_string(),
                        net: "rail".to_string(),
                    },
                    SketchPin {
                        pin: "GND".to_string(),
                        net: "gnd".to_string(),
                    },
                ],
                style: SketchNodeStyle::default(),
            }],
            nets_detail: Vec::new(),
            probes: Vec::new(),
        };

        assert_eq!(
            component_context_pin(&snapshot, "U1", " GND "),
            ("GND".to_string(), "gnd".to_string())
        );
        assert_eq!(
            component_context_pin(&snapshot, "U1", "OUT"),
            ("VCC".to_string(), "rail".to_string())
        );
        assert_eq!(
            component_context_pin(&snapshot, "MISSING", "OUT"),
            ("P1".to_string(), String::new())
        );
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
                style: SketchNodeStyle::default(),
            }],
            nets_detail: vec![SketchNet {
                id: "net_a".to_string(),
                kind: "DigitalOrAnalog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["R1.A".to_string()],
                position: None,
            }],
            probes: Vec::new(),
        };
        let graph = layout_sketch_graph(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0)),
            &snapshot,
        );
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
    }
}
