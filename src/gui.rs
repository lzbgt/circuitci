use crate::reports::ValidationReport;
use anyhow::{Context, Result};
use eframe::egui;
use std::collections::BTreeSet;
use std::path::Path;

mod analog;
#[cfg(test)]
mod analog_assertion_edit_tests;
mod analog_assertion_kinds;
mod analog_branches;
mod analog_generated;
mod analog_model_files;
mod analog_models;
mod analog_overview;
mod analog_stimulus;
mod analog_sweeps;
#[cfg(test)]
mod analog_tests;
mod file_dialogs;
#[cfg(test)]
mod gui_core_tests;
mod import_flow;
mod jobs;
mod kicad_symbol_library;
mod library;
mod library_observation_presets;
mod project;
mod scope_auto_probes;
mod shell;
mod simulation;
mod simulation_editors;
mod simulation_forms;
mod simulation_sweeps;
mod sketch;
mod sketch_actions;
mod sketch_alignment;
mod sketch_bundles;
mod sketch_canvas;
mod sketch_canvas_hits;
mod sketch_canvas_interaction;
mod sketch_canvas_menus;
mod sketch_canvas_render;
#[cfg(test)]
mod sketch_canvas_tests;
mod sketch_canvas_tools;
mod sketch_component_labels;
mod sketch_connectivity;
mod sketch_duplicate;
mod sketch_hierarchy;
mod sketch_inline_edit;
mod sketch_inspector;
mod sketch_layout;
mod sketch_minimap;
mod sketch_navigator;
mod sketch_net_labels;
mod sketch_palette;
#[cfg(test)]
mod sketch_probe_tests;
mod sketch_probes;
mod sketch_rename;
#[cfg(test)]
mod sketch_rename_tests;
mod sketch_render;
mod sketch_routes;
mod sketch_scope_activity;
mod sketch_scope_feedback;
mod sketch_scope_tools;
mod sketch_selection_inspector;
#[cfg(test)]
mod sketch_selection_tests;
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
use sketch_canvas_interaction::SketchSelectionBoxMode;
use sketch_component_labels::SketchComponentLabelKind;
use sketch_hierarchy::{SketchHierarchyFocus, SketchHierarchyTarget};
use sketch_inline_edit::SketchComponentInlineEdit;
use sketch_navigator::SketchNavigatorTarget;
use sketch_scope_tools::SketchScopeProbeTool;
use sketch_spice::SketchSpiceKind;
use waveform::{
    ScopePlotSvgSizePreset, ScopeSnapshotGroupMode, ScopeSnapshotSortKey,
    ScopeSnapshotSourceFilter, WaveformCursorTarget, WaveformFootprintSortKey,
    WaveformFootprintSourceFilter, WaveformFootprintUnloadTarget, WaveformLoadDiagnostic,
    WaveformLoadPreviewFilter, WaveformLoadStatusFilter, WaveformPlotCache, WaveformTracePreset,
    WaveformTraceRef, WaveformTraceStyle, WaveformView, waveform_time_range_for_view,
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
            Self::Simulation => "Observations",
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
struct SketchProbeElementDrag {
    element_id: String,
    current_center: egui::Pos2,
}

#[derive(Debug, Clone)]
struct SketchSelectionBoxDrag {
    start: egui::Pos2,
    mode: SketchSelectionBoxMode,
}

#[derive(Debug, Clone)]
struct SketchSelectionLassoDrag {
    points: Vec<egui::Pos2>,
    mode: SketchSelectionBoxMode,
}

#[derive(Debug, Clone)]
struct SketchGroupFrameDrag {
    pointer_start: egui::Pos2,
    last_applied_delta: egui::Vec2,
    node_starts: Vec<(SketchSelection, egui::Rect)>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SketchSnapMode {
    Free,
    Grid,
    Guides,
    GridAndGuides,
}

impl SketchSnapMode {
    fn label(self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Grid => "Grid",
            Self::Guides => "Guides",
            Self::GridAndGuides => "Grid + Guides",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScopeProbeTarget {
    pub(super) scenario_name: String,
    pub(super) probe_name: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct WaveformViewWindow {
    time_start_us: Option<f64>,
    time_end_us: Option<f64>,
    value_min: Option<f64>,
    value_max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ScopeMeasurementSnapshot {
    label: String,
    note: String,
    source: String,
    trace: Option<WaveformTraceRef>,
    trace_label: String,
    time_a_us: Option<f64>,
    time_b_us: Option<f64>,
    value_a: Option<f64>,
    value_b: Option<f64>,
    delta_value: Option<f64>,
    rms_value: Option<f64>,
    event_edge: Option<String>,
    unit: String,
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
    kicad_symbol_search: String,
    selected_kicad_symbol_id: String,
    kicad_symbol_import_path: String,
    imported_kicad_symbol_files: Vec<String>,
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
    analog_assertion_reference_probe: String,
    analog_assertion_aggregation: String,
    analog_assertion_relation: String,
    analog_assertion_threshold: f64,
    analog_assertion_reference_threshold: f64,
    analog_assertion_target: f64,
    analog_assertion_tolerance: f64,
    analog_assertion_at_us: f64,
    analog_assertion_at_hz: f64,
    analog_assertion_start_us: f64,
    analog_assertion_end_us: f64,
    analog_assertion_time_limit_us: f64,
    analog_assertion_frequency_limit_hz: f64,
    analog_assertion_duty_limit_percent: f64,
    analog_assertion_count_limit: f64,
    analog_assertion_overshoot_limit_percent: f64,
    analog_probe_scenario: String,
    scope_auto_probes_before_run: bool,
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
    analog_sweep_scenario: String,
    analog_sweep_name: String,
    analog_sweep_parameter_name: String,
    analog_sweep_parameter_values: String,
    analog_sweep_component: String,
    analog_sweep_component_field: String,
    analog_sweep_component_values: String,
    analog_sweep_model_path: String,
    analog_sweep_model_sections: String,
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
    selected_project_example_id: String,
    selected_sketch_item: Option<SketchSelection>,
    selected_sketch_items: BTreeSet<SketchSelection>,
    sketch_clipboard_components: Vec<String>,
    sketch_paste_requested: bool,
    sketch_selection_box_drag: Option<SketchSelectionBoxDrag>,
    sketch_selection_lasso_drag: Option<SketchSelectionLassoDrag>,
    sketch_group_frame_drag: Option<SketchGroupFrameDrag>,
    sketch_viewport_command: Option<SketchViewportCommand>,
    sketch_group_action: Option<SketchGroupAction>,
    sketch_zoom: f32,
    sketch_pan: egui::Vec2,
    sketch_grid_enabled: bool,
    sketch_snap_enabled: bool,
    sketch_guide_snap_enabled: bool,
    sketch_grid_step: f32,
    sketch_hierarchy_query: String,
    sketch_hierarchy_focus: Option<SketchHierarchyFocus>,
    sketch_hierarchy_fit_target: Option<SketchHierarchyTarget>,
    sketch_navigator_query: String,
    sketch_navigator_fit_target: Option<SketchNavigatorTarget>,
    sketch_net_bundles_visible: bool,
    sketch_runtime_scope_overlay_visible: bool,
    sketch_scope_activity_window_open: bool,
    sketch_runtime_scope_filter: String,
    sketch_palette_kind: SketchSpiceKind,
    sketch_palette_component_id: String,
    sketch_palette_value: f64,
    sketch_palette_place_armed: bool,
    sketch_library_place_armed: bool,
    sketch_scope_probe_tool: Option<SketchScopeProbeTool>,
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
    sketch_probe_element_drag: Option<SketchProbeElementDrag>,
    waveforms: Vec<WaveformView>,
    waveform_load_diagnostics: Vec<WaveformLoadDiagnostic>,
    waveform_load_filter: String,
    waveform_load_status_filter: WaveformLoadStatusFilter,
    waveform_load_preview_filter: WaveformLoadPreviewFilter,
    waveform_load_min_ms: f64,
    waveform_load_slowest_first: bool,
    waveform_defer_large_loads: bool,
    waveform_deferred_filter: String,
    waveform_deferred_column_filter: String,
    waveform_deferred_column_picks: BTreeSet<(String, String)>,
    waveform_footprint_filter: String,
    waveform_footprint_source_filter: WaveformFootprintSourceFilter,
    waveform_footprint_sort_key: WaveformFootprintSortKey,
    waveform_footprint_descending: bool,
    waveform_footprint_group_by_source: bool,
    waveform_footprint_unload_preview: Vec<WaveformFootprintUnloadTarget>,
    selected_waveform: usize,
    selected_probe: usize,
    waveform_probe_filter: String,
    waveform_probe_group_by_kind: bool,
    waveform_pinned_traces: Vec<WaveformTraceRef>,
    waveform_trace_presets: Vec<WaveformTracePreset>,
    waveform_trace_styles: Vec<WaveformTraceStyle>,
    waveform_measurement_snapshots: Vec<ScopeMeasurementSnapshot>,
    waveform_snapshot_filter: String,
    waveform_snapshot_source_filter: ScopeSnapshotSourceFilter,
    waveform_snapshot_sort_key: ScopeSnapshotSortKey,
    waveform_snapshot_group_mode: ScopeSnapshotGroupMode,
    waveform_trace_preset_name: String,
    waveform_split_trace_units: bool,
    waveform_plot_export_size: ScopePlotSvgSizePreset,
    waveform_plot_export_cursors: bool,
    waveform_plot_export_trigger: bool,
    waveform_plot_export_snapshots: bool,
    waveform_recent_report_bundles: Vec<String>,
    waveform_bundle_cleanup_preview: Vec<String>,
    waveform_bundle_refresh_preview: Option<String>,
    waveform_bundle_integrity_details: Option<String>,
    waveform_bundle_integrity_problems_only: bool,
    pending_scope_probe: Option<ScopeProbeTarget>,
    waveform_math_left: usize,
    waveform_math_right: usize,
    waveform_math_operation: String,
    waveform_math_name: String,
    waveform_promote_scenario: String,
    waveform_promote_probe_name: String,
    waveform_cursor_a_us: f64,
    waveform_cursor_b_us: f64,
    waveform_cursor_drag: Option<WaveformCursorTarget>,
    waveform_box_zoom_start: Option<egui::Pos2>,
    waveform_plot_cache: WaveformPlotCache,
    waveform_playing: bool,
    waveform_playback_speed: f64,
    waveform_window_start_us: Option<f64>,
    waveform_window_end_us: Option<f64>,
    waveform_value_min: Option<f64>,
    waveform_value_max: Option<f64>,
    waveform_view_back_stack: Vec<WaveformViewWindow>,
    waveform_view_forward_stack: Vec<WaveformViewWindow>,
    waveform_view_drag_start: Option<WaveformViewWindow>,
    waveform_trigger_threshold: f64,
    waveform_trigger_edge: String,
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
            kicad_symbol_search: String::new(),
            selected_kicad_symbol_id: String::new(),
            kicad_symbol_import_path: String::new(),
            imported_kicad_symbol_files: Vec::new(),
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
            analog_assertion_reference_probe: String::new(),
            analog_assertion_aggregation: "sample".to_string(),
            analog_assertion_relation: "above".to_string(),
            analog_assertion_threshold: 0.0,
            analog_assertion_reference_threshold: 0.0,
            analog_assertion_target: 0.0,
            analog_assertion_tolerance: 0.1,
            analog_assertion_at_us: 50.0,
            analog_assertion_at_hz: 1000.0,
            analog_assertion_start_us: 0.0,
            analog_assertion_end_us: 100.0,
            analog_assertion_time_limit_us: 50.0,
            analog_assertion_frequency_limit_hz: 1000.0,
            analog_assertion_duty_limit_percent: 50.0,
            analog_assertion_count_limit: 1.0,
            analog_assertion_overshoot_limit_percent: 10.0,
            analog_probe_scenario: String::new(),
            scope_auto_probes_before_run: true,
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
            analog_sweep_scenario: String::new(),
            analog_sweep_name: "input_sweep".to_string(),
            analog_sweep_parameter_name: "PARAM_NAME".to_string(),
            analog_sweep_parameter_values: "0.9, 1.0, 1.1".to_string(),
            analog_sweep_component: "RLOAD".to_string(),
            analog_sweep_component_field: "value_ohm".to_string(),
            analog_sweep_component_values: "900, 1000, 1100".to_string(),
            analog_sweep_model_path: "models/vendor.lib".to_string(),
            analog_sweep_model_sections: "typ, slow, fast".to_string(),
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
            selected_project_example_id: "ne555_astable_scope".to_string(),
            selected_sketch_item: None,
            selected_sketch_items: BTreeSet::new(),
            sketch_clipboard_components: Vec::new(),
            sketch_paste_requested: false,
            sketch_selection_box_drag: None,
            sketch_selection_lasso_drag: None,
            sketch_group_frame_drag: None,
            sketch_viewport_command: None,
            sketch_group_action: None,
            sketch_zoom: 1.0,
            sketch_pan: egui::Vec2::ZERO,
            sketch_grid_enabled: true,
            sketch_snap_enabled: true,
            sketch_guide_snap_enabled: true,
            sketch_grid_step: DEFAULT_SKETCH_GRID_STEP,
            sketch_hierarchy_query: String::new(),
            sketch_hierarchy_focus: None,
            sketch_hierarchy_fit_target: None,
            sketch_navigator_query: String::new(),
            sketch_navigator_fit_target: None,
            sketch_net_bundles_visible: false,
            sketch_runtime_scope_overlay_visible: true,
            sketch_scope_activity_window_open: false,
            sketch_runtime_scope_filter: String::new(),
            sketch_palette_kind: SketchSpiceKind::Resistor,
            sketch_palette_component_id: "R1".to_string(),
            sketch_palette_value: sketch_palette::default_primitive_value(
                SketchSpiceKind::Resistor,
            ),
            sketch_palette_place_armed: false,
            sketch_library_place_armed: false,
            sketch_scope_probe_tool: None,
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
            sketch_probe_element_drag: None,
            waveforms: Vec::new(),
            waveform_load_diagnostics: Vec::new(),
            waveform_load_filter: String::new(),
            waveform_load_status_filter: WaveformLoadStatusFilter::All,
            waveform_load_preview_filter: WaveformLoadPreviewFilter::All,
            waveform_load_min_ms: 0.0,
            waveform_load_slowest_first: false,
            waveform_defer_large_loads: true,
            waveform_deferred_filter: String::new(),
            waveform_deferred_column_filter: String::new(),
            waveform_deferred_column_picks: BTreeSet::new(),
            waveform_footprint_filter: String::new(),
            waveform_footprint_source_filter: WaveformFootprintSourceFilter::All,
            waveform_footprint_sort_key: WaveformFootprintSortKey::EstimatedBytes,
            waveform_footprint_descending: true,
            waveform_footprint_group_by_source: false,
            waveform_footprint_unload_preview: Vec::new(),
            selected_waveform: 0,
            selected_probe: 0,
            waveform_probe_filter: String::new(),
            waveform_probe_group_by_kind: false,
            waveform_pinned_traces: Vec::new(),
            waveform_trace_presets: Vec::new(),
            waveform_trace_styles: Vec::new(),
            waveform_measurement_snapshots: Vec::new(),
            waveform_snapshot_filter: String::new(),
            waveform_snapshot_source_filter: ScopeSnapshotSourceFilter::All,
            waveform_snapshot_sort_key: ScopeSnapshotSortKey::Captured,
            waveform_snapshot_group_mode: ScopeSnapshotGroupMode::None,
            waveform_trace_preset_name: String::new(),
            waveform_split_trace_units: false,
            waveform_plot_export_size: ScopePlotSvgSizePreset::Report,
            waveform_plot_export_cursors: true,
            waveform_plot_export_trigger: true,
            waveform_plot_export_snapshots: true,
            waveform_recent_report_bundles: Vec::new(),
            waveform_bundle_cleanup_preview: Vec::new(),
            waveform_bundle_refresh_preview: None,
            waveform_bundle_integrity_details: None,
            waveform_bundle_integrity_problems_only: false,
            pending_scope_probe: None,
            waveform_math_left: 0,
            waveform_math_right: 0,
            waveform_math_operation: "difference".to_string(),
            waveform_math_name: String::new(),
            waveform_promote_scenario: String::new(),
            waveform_promote_probe_name: String::new(),
            waveform_cursor_a_us: 0.0,
            waveform_cursor_b_us: 0.0,
            waveform_cursor_drag: None,
            waveform_box_zoom_start: None,
            waveform_plot_cache: WaveformPlotCache::default(),
            waveform_playing: false,
            waveform_playback_speed: 1.0,
            waveform_window_start_us: None,
            waveform_window_end_us: None,
            waveform_value_min: None,
            waveform_value_max: None,
            waveform_view_back_stack: Vec::new(),
            waveform_view_forward_stack: Vec::new(),
            waveform_view_drag_start: None,
            waveform_trigger_threshold: 0.0,
            waveform_trigger_edge: "rising".to_string(),
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
        if self.stage == Stage::Project {
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
                    "{} components / {} nets / {} run setups",
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
            if ui
                .add_enabled(
                    self.background_job_elapsed_secs().is_none() && self.project_snapshot.is_some(),
                    egui::Button::new("Run + Scopes"),
                )
                .on_hover_text("Run validation and open Scopes immediately with the pending scope probe focused when traces arrive.")
                .clicked()
            {
                self.run_schematic_model_open_scopes();
            }
            if ui.button("Scopes").clicked() {
                self.apply_pending_scope_probe_focus();
                self.stage = Stage::Simulation;
            }
            self.scope_auto_probe_button(ui);
            self.scope_auto_probe_run_toggle(ui);
            self.schematic_probe_toolbar_controls(ui);
            if ui
                .add_enabled(
                    self.project_snapshot.is_some(),
                    egui::Button::new("Auto Layout"),
                )
                .on_hover_text("Persist a classical textbook-style schematic layout with rails, signal flow, and vertical ground shunts.")
                .clicked()
            {
                self.apply_classical_sketch_auto_layout();
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
            self.sketch_snap_toolbar_controls(ui);
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

    fn schematic_probe_toolbar_controls(&mut self, ui: &mut egui::Ui) {
        self.schematic_scope_probe_tool_controls(ui);
        let Some(selection) = self.selected_sketch_item.clone() else {
            ui.label("Select a net/component to probe");
            return;
        };
        match selection {
            SketchSelection::Net(net_id) => {
                if ui.button("Probe V").clicked() {
                    self.ensure_net_probe_defaults(&net_id);
                    self.apply_add_voltage_probe_for_net(&net_id);
                }
                if ui.button("Scope V").clicked() {
                    self.open_or_create_scope_voltage_probe_for_net(&net_id);
                }
            }
            SketchSelection::Component(component_id) => {
                if ui.button("Probe I").clicked() {
                    self.ensure_component_probe_defaults(&component_id);
                    self.apply_add_current_probe_for_component(&component_id);
                }
                if ui.button("Scope I").clicked() {
                    self.open_or_create_scope_component_probe(
                        &component_id,
                        sketch_probes::SketchProbeQuantity::Current,
                    );
                }
                if ui.button("Probe P").clicked() {
                    self.ensure_component_probe_defaults(&component_id);
                    self.apply_add_power_probe_for_component(&component_id);
                }
                if ui.button("Scope P").clicked() {
                    self.open_or_create_scope_component_probe(
                        &component_id,
                        sketch_probes::SketchProbeQuantity::Power,
                    );
                }
            }
            SketchSelection::Overflow(_) => {
                ui.label("Select a visible net/component to probe");
            }
        }
    }

    fn open_or_create_scope_voltage_probe_for_net(&mut self, net_id: &str) {
        self.open_or_create_scope_voltage_probe_for_net_with_attachment(
            net_id,
            sketch_probes::SketchProbeAttachmentKind::Node,
            None,
        );
    }

    fn open_or_create_scope_voltage_probe_for_net_with_attachment(
        &mut self,
        net_id: &str,
        attachment: sketch_probes::SketchProbeAttachmentKind,
        source: Option<String>,
    ) {
        if let Some(target) = self.scope_probe_for_selected_net(net_id) {
            self.open_scope_probe_target(target);
            return;
        }
        self.ensure_net_probe_defaults(net_id);
        let target = ScopeProbeTarget {
            scenario_name: self.analog_probe_scenario.trim().to_string(),
            probe_name: self.analog_canvas_probe_name.trim().to_string(),
        };
        if self.apply_add_voltage_probe_for_net_with_attachment(net_id, attachment, source) {
            self.open_scope_probe_target(target);
        }
    }

    fn open_or_create_scope_component_probe(
        &mut self,
        component_id: &str,
        quantity: sketch_probes::SketchProbeQuantity,
    ) {
        self.open_or_create_scope_component_probe_with_attachment(
            component_id,
            quantity,
            sketch_probes::SketchProbeAttachmentKind::Node,
            None,
        );
    }

    fn open_or_create_scope_component_probe_with_attachment(
        &mut self,
        component_id: &str,
        quantity: sketch_probes::SketchProbeQuantity,
        attachment: sketch_probes::SketchProbeAttachmentKind,
        source: Option<String>,
    ) {
        if let Some(target) =
            self.scope_probe_for_selected_component_quantity(component_id, quantity)
        {
            self.open_scope_probe_target(target);
            return;
        }
        self.ensure_component_probe_defaults(component_id);
        let probe_name = match quantity {
            sketch_probes::SketchProbeQuantity::Voltage => return,
            sketch_probes::SketchProbeQuantity::Current => {
                self.analog_canvas_component_probe_name.trim().to_string()
            }
            sketch_probes::SketchProbeQuantity::Power => self
                .analog_canvas_component_power_probe_name
                .trim()
                .to_string(),
        };
        let target = ScopeProbeTarget {
            scenario_name: self.analog_probe_scenario.trim().to_string(),
            probe_name,
        };
        let added = match quantity {
            sketch_probes::SketchProbeQuantity::Voltage => false,
            sketch_probes::SketchProbeQuantity::Current => self
                .apply_add_current_probe_for_component_with_attachment(
                    component_id,
                    attachment,
                    source,
                ),
            sketch_probes::SketchProbeQuantity::Power => self
                .apply_add_power_probe_for_component_with_attachment(
                    component_id,
                    attachment,
                    source,
                ),
        };
        if added {
            self.open_scope_probe_target(target);
        }
    }

    fn scope_probe_for_selected_net(&self, net_id: &str) -> Option<ScopeProbeTarget> {
        self.project_snapshot
            .as_ref()?
            .probes
            .iter()
            .find(|probe| {
                matches!(
                    &probe.target,
                    sketch_probes::SketchProbeTarget::Net(target) if target == net_id
                )
            })
            .map(|probe| ScopeProbeTarget {
                scenario_name: probe.scenario_name.clone(),
                probe_name: probe.probe_name.clone(),
            })
    }

    fn scope_probe_for_selected_component_quantity(
        &self,
        component_id: &str,
        quantity: sketch_probes::SketchProbeQuantity,
    ) -> Option<ScopeProbeTarget> {
        self.project_snapshot
            .as_ref()?
            .probes
            .iter()
            .find(|probe| {
                probe.quantity == quantity
                    &&
                matches!(
                    &probe.target,
                    sketch_probes::SketchProbeTarget::Component(target) if target == component_id
                )
            })
            .map(|probe| ScopeProbeTarget {
                scenario_name: probe.scenario_name.clone(),
                probe_name: probe.probe_name.clone(),
            })
    }

    fn sketch_snap_toolbar_controls(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.checkbox(&mut self.sketch_grid_enabled, "Grid");
        ui.label("Step");
        let step_changed = ui
            .add(
                egui::DragValue::new(&mut self.sketch_grid_step)
                    .range(4.0..=96.0)
                    .speed(1.0),
            )
            .changed();
        if step_changed {
            self.normalize_sketch_grid_step();
        }
        for step in [8.0, 16.0, 32.0] {
            if ui.small_button(format!("{step:.0}")).clicked() {
                self.sketch_grid_step = step;
            }
        }

        let mut mode = self.sketch_snap_mode();
        egui::ComboBox::from_id_salt("sketch_snap_mode")
            .selected_text(mode.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut mode,
                    SketchSnapMode::Free,
                    SketchSnapMode::Free.label(),
                );
                ui.selectable_value(
                    &mut mode,
                    SketchSnapMode::Grid,
                    SketchSnapMode::Grid.label(),
                );
                ui.selectable_value(
                    &mut mode,
                    SketchSnapMode::Guides,
                    SketchSnapMode::Guides.label(),
                );
                ui.selectable_value(
                    &mut mode,
                    SketchSnapMode::GridAndGuides,
                    SketchSnapMode::GridAndGuides.label(),
                );
            });
        self.set_sketch_snap_mode(mode);
    }

    fn sketch_snap_mode(&self) -> SketchSnapMode {
        match (self.sketch_snap_enabled, self.sketch_guide_snap_enabled) {
            (false, false) => SketchSnapMode::Free,
            (true, false) => SketchSnapMode::Grid,
            (false, true) => SketchSnapMode::Guides,
            (true, true) => SketchSnapMode::GridAndGuides,
        }
    }

    fn set_sketch_snap_mode(&mut self, mode: SketchSnapMode) {
        (self.sketch_snap_enabled, self.sketch_guide_snap_enabled) = match mode {
            SketchSnapMode::Free => (false, false),
            SketchSnapMode::Grid => (true, false),
            SketchSnapMode::Guides => (false, true),
            SketchSnapMode::GridAndGuides => (true, true),
        };
    }

    fn normalize_sketch_grid_step(&mut self) {
        if !self.sketch_grid_step.is_finite() {
            self.sketch_grid_step = DEFAULT_SKETCH_GRID_STEP;
        }
        self.sketch_grid_step = self.sketch_grid_step.clamp(4.0, 96.0);
    }

    fn run_schematic_model(&mut self) {
        self.run_model_with_scope_preparation();
    }

    fn run_schematic_model_open_scopes(&mut self) {
        if self.run_model_with_scope_preparation() {
            self.open_scopes_for_running_validation();
        }
    }

    fn open_scopes_for_running_validation(&mut self) {
        self.apply_pending_scope_probe_focus();
        self.stage = Stage::Simulation;
        self.status = "Running validation in Scopes.".to_string();
        self.push_diagnostic("Run + Scopes opened the Scopes workspace while validation runs.");
    }

    fn schematic_side_dock(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.sketch_scope_example_workflow_strip(ui) {
                ui.separator();
            }
            self.scope_run_readiness_panel(ui);
            ui.separator();
            self.sketch_primitive_palette(ui);
            ui.separator();
            self.sketch_probe_element_palette(ui);
            ui.separator();
            self.sketch_overlay_panel(ui, snapshot);
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
                        self.project_snapshot.is_some(),
                        egui::Button::new("Auto Layout"),
                    )
                    .on_hover_text("Write the classical schematic layout into Board IR node positions and standard two-terminal orientations.")
                    .clicked()
                {
                    self.apply_classical_sketch_auto_layout();
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
                    "Drag blank canvas or use touchpad scroll to pan; pinch/Cmd-scroll zooms around the pointer; Shift+drag selects, Cmd/Ctrl+drag adds, Alt/Option+drag subtracts; Cmd/Ctrl+C copies, Cmd/Ctrl+V pastes, Cmd/Ctrl+D duplicates selected components.",
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
