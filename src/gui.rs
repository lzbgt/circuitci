use crate::reports::ValidationReport;
use anyhow::{Context, Result};
use eframe::egui;
use std::collections::BTreeSet;
use std::path::Path;

mod analog;
#[cfg(test)]
mod analog_assertion_edit_tests;
mod analog_branches;
mod analog_generated;
mod analog_models;
mod analog_overview;
mod analog_stimulus;
#[cfg(test)]
mod analog_tests;
mod file_dialogs;
mod import_flow;
mod jobs;
mod library;
mod project;
mod shell;
mod simulation;
mod sketch;
mod sketch_actions;
mod sketch_bundles;
mod sketch_canvas;
mod sketch_canvas_interaction;
mod sketch_canvas_menus;
mod sketch_canvas_render;
#[cfg(test)]
mod sketch_canvas_tests;
mod sketch_component_labels;
mod sketch_connectivity;
mod sketch_duplicate;
mod sketch_hierarchy;
mod sketch_inline_edit;
mod sketch_inspector;
mod sketch_minimap;
mod sketch_navigator;
mod sketch_net_labels;
mod sketch_palette;
mod sketch_probes;
mod sketch_rename;
mod sketch_render;
mod sketch_routes;
mod sketch_spice;
mod sketch_symbols;
#[cfg(test)]
mod sketch_tests;
mod sketch_wire_draft;
mod spice;
mod waveform;

use project::PendingProjectAction;
use sketch::{
    DEFAULT_SKETCH_GRID_STEP, ProjectSnapshot, SketchNetLabelKind, SketchPinSide, SketchSelection,
};
use sketch_component_labels::SketchComponentLabelKind;
use sketch_hierarchy::{SketchHierarchyFocus, SketchHierarchyTarget};
use sketch_inline_edit::SketchComponentInlineEdit;
use sketch_navigator::SketchNavigatorTarget;
use sketch_spice::SketchSpiceKind;
use waveform::{WaveformView, waveform_time_range_for_view};

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

#[derive(Debug, Clone)]
struct SketchWireRouteDrag {
    net_id: String,
    source: String,
    points: Vec<egui::Pos2>,
    point_index: usize,
}

#[derive(Debug, Clone)]
struct SketchNetLabelDrag {
    label_id: String,
    net_id: String,
    current_center: egui::Pos2,
}

#[derive(Debug, Clone)]
struct SketchComponentLabelDrag {
    component_id: String,
    kind: SketchComponentLabelKind,
    current_center: egui::Pos2,
}

#[derive(Debug, Clone)]
struct SketchNetLabelEdit {
    label_id: String,
    original_net_id: String,
    draft_net_id: String,
    draft_kind: SketchNetLabelKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SketchGroupAction {
    Nudge(egui::Vec2),
    AlignLeft,
    AlignRight,
    AlignTop,
    AlignBottom,
    AlignCenterX,
    AlignCenterY,
    DistributeHorizontal,
    DistributeVertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SketchViewportCommand {
    FitAll,
    FitSelection,
    Home,
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
    pending_project_action: Option<PendingProjectAction>,
    model_search: String,
    selected_library_model: String,
    new_component_id: String,
    new_component_model: String,
    new_net_id: String,
    new_net_kind: String,
    component_rename_id: String,
    net_rename_id: String,
    pin_edit_id: String,
    pin_edit_net: String,
    wire_pin_id: String,
    wire_from_component: Option<String>,
    sketch_wire_draft: sketch_wire_draft::SketchWireDraft,
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
    analog_generated_scenario: String,
    analog_generated_component: String,
    analog_generated_ground_net: String,
    analog_generated_stop_time_us: f64,
    analog_generated_max_step_us: f64,
    analog_generated_node_net: String,
    analog_generated_node_name: String,
    analog_model_scenario: String,
    analog_model_path: String,
    analog_model_sha256: String,
    analog_stimulus_scenario: String,
    analog_stimulus_component: String,
    analog_stimulus_dc_value: f64,
    analog_stimulus_initial_value: f64,
    analog_stimulus_pulsed_value: f64,
    analog_stimulus_delay_us: f64,
    analog_stimulus_rise_us: f64,
    analog_stimulus_fall_us: f64,
    analog_stimulus_width_us: f64,
    analog_stimulus_period_us: f64,
    project_snapshot: Option<ProjectSnapshot>,
    selected_sketch_item: Option<SketchSelection>,
    selected_sketch_items: BTreeSet<SketchSelection>,
    sketch_clipboard_components: Vec<String>,
    sketch_paste_requested: bool,
    marquee_start: Option<egui::Pos2>,
    sketch_viewport_command: Option<SketchViewportCommand>,
    sketch_group_action: Option<SketchGroupAction>,
    sketch_zoom: f32,
    sketch_pan: egui::Vec2,
    sketch_grid_enabled: bool,
    sketch_snap_enabled: bool,
    sketch_grid_step: f32,
    sketch_hierarchy_query: String,
    sketch_hierarchy_focus: Option<SketchHierarchyFocus>,
    sketch_hierarchy_fit_target: Option<SketchHierarchyTarget>,
    sketch_navigator_query: String,
    sketch_navigator_fit_target: Option<SketchNavigatorTarget>,
    sketch_palette_kind: SketchSpiceKind,
    sketch_palette_component_id: String,
    sketch_palette_value: f64,
    sketch_palette_place_armed: bool,
    sketch_library_place_armed: bool,
    sketch_placement_rotation_deg: i32,
    sketch_placement_mirrored: bool,
    sketch_placement_pin_side: SketchPinSide,
    sketch_net_label_net_id: String,
    sketch_net_label_kind: SketchNetLabelKind,
    sketch_net_label_net_kind: String,
    sketch_net_label_place_armed: bool,
    sketch_net_label_edit: Option<SketchNetLabelEdit>,
    sketch_reference_labels_visible: bool,
    sketch_value_labels_visible: bool,
    sketch_component_inline_edit: Option<SketchComponentInlineEdit>,
    sketch_last_canvas_rect: Option<egui::Rect>,
    sketch_pan_drag_active: bool,
    sketch_wire_route_drag: Option<SketchWireRouteDrag>,
    sketch_net_label_drag: Option<SketchNetLabelDrag>,
    sketch_component_label_drag: Option<SketchComponentLabelDrag>,
    waveforms: Vec<WaveformView>,
    selected_waveform: usize,
    selected_probe: usize,
    waveform_math_left: usize,
    waveform_math_right: usize,
    waveform_math_operation: String,
    waveform_math_name: String,
    waveform_promote_scenario: String,
    waveform_promote_probe_name: String,
    waveform_cursor_a_us: f64,
    waveform_cursor_b_us: f64,
    waveform_playing: bool,
    waveform_playback_speed: f64,
    background_job: Option<jobs::BackgroundGuiJob>,
    background_job_history: Vec<jobs::BackgroundJobRecord>,
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
            pending_project_action: None,
            model_search: String::new(),
            selected_library_model: String::new(),
            new_component_id: "U_NEW".to_string(),
            new_component_model: "generic.schematic.imported_component".to_string(),
            new_net_id: "net_new".to_string(),
            new_net_kind: "digital_or_analog".to_string(),
            component_rename_id: String::new(),
            net_rename_id: String::new(),
            pin_edit_id: "P1".to_string(),
            pin_edit_net: String::new(),
            wire_pin_id: "P1".to_string(),
            wire_from_component: None,
            sketch_wire_draft: Default::default(),
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
            analog_generated_scenario: String::new(),
            analog_generated_component: String::new(),
            analog_generated_ground_net: String::new(),
            analog_generated_stop_time_us: 100.0,
            analog_generated_max_step_us: 1.0,
            analog_generated_node_net: String::new(),
            analog_generated_node_name: String::new(),
            analog_model_scenario: String::new(),
            analog_model_path: String::new(),
            analog_model_sha256: String::new(),
            analog_stimulus_scenario: String::new(),
            analog_stimulus_component: String::new(),
            analog_stimulus_dc_value: 0.0,
            analog_stimulus_initial_value: 0.0,
            analog_stimulus_pulsed_value: 3.3,
            analog_stimulus_delay_us: 0.0,
            analog_stimulus_rise_us: 1.0,
            analog_stimulus_fall_us: 1.0,
            analog_stimulus_width_us: 10.0,
            analog_stimulus_period_us: 20.0,
            project_snapshot: None,
            selected_sketch_item: None,
            selected_sketch_items: BTreeSet::new(),
            sketch_clipboard_components: Vec::new(),
            sketch_paste_requested: false,
            marquee_start: None,
            sketch_viewport_command: None,
            sketch_group_action: None,
            sketch_zoom: 1.0,
            sketch_pan: egui::Vec2::ZERO,
            sketch_grid_enabled: true,
            sketch_snap_enabled: true,
            sketch_grid_step: DEFAULT_SKETCH_GRID_STEP,
            sketch_hierarchy_query: String::new(),
            sketch_hierarchy_focus: None,
            sketch_hierarchy_fit_target: None,
            sketch_navigator_query: String::new(),
            sketch_navigator_fit_target: None,
            sketch_palette_kind: SketchSpiceKind::Resistor,
            sketch_palette_component_id: "R1".to_string(),
            sketch_palette_value: sketch_palette::default_primitive_value(
                SketchSpiceKind::Resistor,
            ),
            sketch_palette_place_armed: false,
            sketch_library_place_armed: false,
            sketch_placement_rotation_deg: 0,
            sketch_placement_mirrored: false,
            sketch_placement_pin_side: SketchPinSide::Auto,
            sketch_net_label_net_id: "sig".to_string(),
            sketch_net_label_kind: SketchNetLabelKind::Local,
            sketch_net_label_net_kind: "digital_or_analog".to_string(),
            sketch_net_label_place_armed: false,
            sketch_net_label_edit: None,
            sketch_reference_labels_visible: true,
            sketch_value_labels_visible: true,
            sketch_component_inline_edit: None,
            sketch_last_canvas_rect: None,
            sketch_pan_drag_active: false,
            sketch_wire_route_drag: None,
            sketch_net_label_drag: None,
            sketch_component_label_drag: None,
            waveforms: Vec::new(),
            selected_waveform: 0,
            selected_probe: 0,
            waveform_math_left: 0,
            waveform_math_right: 0,
            waveform_math_operation: "difference".to_string(),
            waveform_math_name: String::new(),
            waveform_promote_scenario: String::new(),
            waveform_promote_probe_name: String::new(),
            waveform_cursor_a_us: 0.0,
            waveform_cursor_b_us: 0.0,
            waveform_playing: false,
            waveform_playback_speed: 1.0,
            background_job: None,
            background_job_history: Vec::new(),
        }
    }
}

impl eframe::App for CircuitCiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_close_request(ctx);
        self.poll_background_job(ctx);
        self.advance_waveform_playback(ctx);
        self.menu_bar(ctx);
        self.workflow_bar(ctx);
        if self.stage != Stage::Sketch {
            self.left_panel(ctx);
        }
        self.bottom_panel(ctx);
        self.central_panel(ctx);
        self.unsaved_project_action_dialog(ctx);
    }
}

impl CircuitCiApp {
    fn sketch_stage(&mut self, ui: &mut egui::Ui) {
        self.schematic_run_toolbar(ui);
        if let Some(snapshot) = self.project_snapshot.clone() {
            self.sketch_edit_toolbar(ui);
            ui.separator();

            let available = ui.available_size();
            let side_width = (available.x * 0.28).clamp(300.0, 380.0);
            let gap = 8.0;
            if available.x >= 900.0 {
                let canvas_size = egui::vec2(
                    (available.x - side_width - gap).max(560.0),
                    available.y.max(520.0),
                );
                ui.horizontal_top(|ui| {
                    self.draw_board_graph_sized(ui, &snapshot, canvas_size);
                    ui.add_space(gap);
                    ui.allocate_ui_with_layout(
                        egui::vec2(side_width, canvas_size.y),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| self.schematic_side_dock(ui, &snapshot),
                    );
                });
            } else {
                self.draw_board_graph_sized(
                    ui,
                    &snapshot,
                    egui::vec2(available.x.max(560.0), available.y.max(520.0)),
                );
                ui.separator();
                self.schematic_side_dock(ui, &snapshot);
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Load or import a project to open the schematic workspace.");
            });
        }
    }

    fn schematic_run_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Schematic");
            if let Some(snapshot) = &self.project_snapshot {
                ui.label(format!(
                    "{} components / {} nets / {} scenarios",
                    snapshot.components, snapshot.nets, snapshot.scenarios
                ));
            } else {
                ui.label("No project loaded");
            }
            if ui
                .add_enabled(
                    self.background_job_elapsed_secs().is_none() && self.project_snapshot.is_some(),
                    egui::Button::new("Run"),
                )
                .clicked()
            {
                self.run_schematic_model();
            }
            if ui.button("Scopes").clicked() {
                self.stage = Stage::Simulation;
            }
            if ui.button("Fit All").clicked() {
                self.sketch_viewport_command = Some(SketchViewportCommand::FitAll);
            }
            if ui
                .add_enabled(
                    self.has_fit_selection_target(),
                    egui::Button::new("Fit Selection"),
                )
                .clicked()
            {
                self.sketch_viewport_command = Some(SketchViewportCommand::FitSelection);
            }
            if ui.button("Home").clicked() {
                self.sketch_viewport_command = Some(SketchViewportCommand::Home);
            }
            if ui.button("Save").clicked() {
                self.save_project_yaml();
            }
            if ui.button("Validate YAML").clicked() {
                self.validate_project_yaml_text();
            }
            if self.project_yaml_dirty {
                ui.label("Unsaved edits");
            }
            if let Some(elapsed_secs) = self.background_job_elapsed_secs() {
                let label = self.background_job_label().unwrap_or("job");
                ui.add(egui::Spinner::new());
                ui.label(format!("{label} running for {elapsed_secs:.1}s"));
                if ui
                    .add_enabled(
                        !self.background_job_cancel_requested(),
                        egui::Button::new("Cancel"),
                    )
                    .clicked()
                {
                    self.cancel_background_job();
                }
            }
        });
        ui.separator();
    }

    fn run_schematic_model(&mut self) {
        if self.project_yaml_dirty {
            self.save_project_yaml();
            if self.project_yaml_dirty {
                return;
            }
        }
        self.validate_project();
    }

    fn schematic_side_dock(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.sketch_primitive_palette(ui);
            ui.separator();
            self.sketch_net_label_panel(ui, snapshot);
            ui.separator();
            self.sketch_component_label_panel(ui, snapshot);
            ui.separator();
            self.sketch_library_placement_panel(ui);
            ui.separator();
            self.sketch_inspector(ui, snapshot);
            ui.separator();
            self.sketch_hierarchy_panel(ui, snapshot);
            self.sketch_navigator_panel(ui, snapshot);
            ui.separator();
            egui::CollapsingHeader::new("Board IR YAML")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Reload YAML").clicked() {
                            self.request_project_action(
                                PendingProjectAction::LoadProjectYaml {
                                    path: self.project_path.clone(),
                                },
                                Some(ui.ctx()),
                            );
                        }
                        if ui.button("Save YAML").clicked() {
                            self.save_project_yaml();
                        }
                    });
                    if self.project_yaml.is_empty() {
                        ui.label("Load a project to edit Board IR YAML.");
                    } else {
                        let previous_yaml = self.project_yaml.clone();
                        let response = ui.add(
                            egui::TextEdit::multiline(&mut self.project_yaml)
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(18)
                                .lock_focus(true),
                        );
                        if response.changed() {
                            self.record_project_yaml_text_edit(previous_yaml);
                        }
                    }
                });
        });
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
                    self.sketch_viewport_command = Some(SketchViewportCommand::Home);
                }
                if ui.button("Fit All").clicked() {
                    self.sketch_viewport_command = Some(SketchViewportCommand::FitAll);
                }
                if ui
                    .add_enabled(
                        self.has_fit_selection_target(),
                        egui::Button::new("Fit Selection"),
                    )
                    .clicked()
                {
                    self.sketch_viewport_command = Some(SketchViewportCommand::FitSelection);
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
                if ui
                    .add_enabled(
                        self.has_duplicable_sketch_selection(),
                        egui::Button::new("Duplicate"),
                    )
                    .clicked()
                {
                    self.apply_duplicate_selected_sketch_items();
                }
                if ui
                    .add_enabled(
                        self.has_duplicable_sketch_selection(),
                        egui::Button::new("Copy"),
                    )
                    .clicked()
                {
                    self.apply_copy_selected_sketch_items();
                }
                if ui
                    .add_enabled(
                        self.has_pasteable_sketch_clipboard(),
                        egui::Button::new("Paste"),
                    )
                    .clicked()
                {
                    self.sketch_paste_requested = true;
                }
                ui.label(
                    "Drag blank canvas or use touchpad scroll to pan; pinch/Cmd-scroll zooms around the pointer; Shift+drag marquee selects; Cmd/Ctrl+C copies, Cmd/Ctrl+V pastes, Cmd/Ctrl+D duplicates selected components.",
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
                    if ui.button("Align Right").clicked() {
                        self.sketch_group_action = Some(SketchGroupAction::AlignRight);
                    }
                    if ui.button("Align Top").clicked() {
                        self.sketch_group_action = Some(SketchGroupAction::AlignTop);
                    }
                    if ui.button("Align Bottom").clicked() {
                        self.sketch_group_action = Some(SketchGroupAction::AlignBottom);
                    }
                    if ui.button("Center X").clicked() {
                        self.sketch_group_action = Some(SketchGroupAction::AlignCenterX);
                    }
                    if ui.button("Center Y").clicked() {
                        self.sketch_group_action = Some(SketchGroupAction::AlignCenterY);
                    }
                    if ui.button("Distribute X").clicked() {
                        self.sketch_group_action = Some(SketchGroupAction::DistributeHorizontal);
                    }
                    if ui.button("Distribute Y").clicked() {
                        self.sketch_group_action = Some(SketchGroupAction::DistributeVertical);
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

fn validate_from_gui<F, C>(
    project_path: &Path,
    profile: &str,
    output: &Path,
    mut on_progress: F,
    should_cancel: C,
) -> Result<(ValidationReport, String)>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let command = format!(
        "circuitci-gui validate {} --profile {} --output {}",
        display_path(project_path),
        profile,
        display_path(output)
    );
    let report = crate::suite::validate_and_write_project_report_with_progress_and_cancel(
        project_path,
        profile,
        output,
        command,
        &mut on_progress,
        should_cancel,
    )?;
    on_progress(
        "Loading markdown report",
        format!("Reading {}.", display_path(&output.join("report.md"))),
    );
    let markdown = std::fs::read_to_string(output.join("report.md"))
        .with_context(|| format!("Failed to read {}.", output.join("report.md").display()))?;
    Ok((report, markdown))
}

fn suggest_from_gui_with_cancel<C>(
    project_path: &Path,
    profile: &str,
    should_cancel: C,
) -> Result<String>
where
    C: Fn() -> bool,
{
    let project = crate::board_ir::load_project(project_path)?;
    if should_cancel() {
        return Err(crate::cancellation::canceled(
            "Scenario suggestions canceled before completion.",
        ));
    }
    let (library, library_findings) = crate::library::load_library(project_path, &project);
    if should_cancel() {
        return Err(crate::cancellation::canceled(
            "Scenario suggestions canceled before completion.",
        ));
    }
    let bound = crate::library::bind_project(&project, library, library_findings);
    if should_cancel() {
        return Err(crate::cancellation::canceled(
            "Scenario suggestions canceled before completion.",
        ));
    }
    let profile = if profile.trim().is_empty() || profile == "default" {
        None
    } else {
        Some(profile)
    };
    let report = crate::scenario_suggestions::suggest_scenarios_for_profile(&bound, profile);
    if should_cancel() {
        return Err(crate::cancellation::canceled(
            "Scenario suggestions canceled before completion.",
        ));
    }
    serde_yaml_ng::to_string(&report).context("Failed to serialize scenario suggestions.")
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::egui;
    use super::sketch::{
        ProjectSnapshot, SketchComponent, SketchNet, SketchNodeStyle, SketchPin,
        edit_component_model, edit_component_part_number, edit_net_kind, edit_net_nominal_voltage,
        edit_net_powered, layout_sketch_graph, validate_board_ir_yaml_text,
    };
    use super::sketch_canvas_render::component_context_pin;
    use std::path::Path;

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
    fn validate_from_gui_emits_phase_progress() {
        let output = tempfile::tempdir().unwrap();
        let mut stages = Vec::new();

        let (report, markdown) = super::validate_from_gui(
            Path::new("examples/good_current_source_load/project.yaml"),
            "default",
            output.path(),
            |stage, _detail| stages.push(stage.to_string()),
            || false,
        )
        .unwrap();

        assert_eq!(report.result, "pass");
        assert!(markdown.contains("# CircuitCI Report"));
        for expected in [
            "Loading project",
            "Loading models",
            "Binding models",
            "Running validation",
            "Preparing analog transient",
            "Checking analog model evidence",
            "Preparing analog deck",
            "Selecting analog backend",
            "Writing analog wrapper deck",
            "Running analog backend",
            "Loading analog waveform",
            "Evaluating analog assertions",
            "Applying profile coverage",
            "Assembling report",
            "Writing report",
            "Loading markdown report",
        ] {
            assert!(stages.iter().any(|stage| stage == expected), "{expected}");
        }
    }

    #[test]
    fn suggest_from_gui_cancellation_stops_before_yaml_output() {
        let error = super::suggest_from_gui_with_cancel(
            Path::new("examples/scenario_suggestions_power_reset/project.yaml"),
            "default",
            || true,
        )
        .unwrap_err();

        assert!(error.to_string().contains("canceled"));
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
                spice: None,
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
                source_paths: Vec::new(),
            }],
            nets_detail: Vec::new(),
            probes: Vec::new(),
            wire_routes: Default::default(),
            net_labels: Default::default(),
            component_labels: Default::default(),
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
                spice: None,
                position: None,
                pins: vec![SketchPin {
                    pin: "A".to_string(),
                    net: "net_a".to_string(),
                }],
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
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
            wire_routes: Default::default(),
            net_labels: Default::default(),
            component_labels: Default::default(),
        };
        let graph = layout_sketch_graph(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0)),
            &snapshot,
        );
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
    }
}
