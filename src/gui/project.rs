use super::sketch::{
    SketchSelection, load_project_snapshot, load_project_snapshot_from_yaml,
    validate_board_ir_yaml_text,
};
use super::{CircuitCiApp, SketchViewportCommand, Stage};
use anyhow::Context;
use eframe::egui;
use std::path::{Path, PathBuf};

const PROJECT_YAML_HISTORY_LIMIT: usize = 64;
const NE555_SCOPE_EXAMPLE_PROJECT: &str = "examples/ne555_astable_scope_smoke/project.yaml";
const NE555_SCOPE_EXAMPLE_NAME: &str = "ne555_astable_scope";
const NE555_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v(out)", "v(timing)", "v(vcc)", "i(VCC)", "i(VOUT)"];
const NE555_SCOPE_EXPECTED_FREQUENCY: &str = "about 1.46 kHz";
const RC_LOWPASS_SCOPE_EXAMPLE_PROJECT: &str = "examples/rc_lowpass_scope/project.yaml";
const RC_LOWPASS_SCOPE_EXAMPLE_NAME: &str = "rc_lowpass_scope";
const RC_LOWPASS_SCOPE_EXPECTED_TRACES: &[&str] = &["v(input)", "v(filtered)", "i(VSIN)"];
const RC_LOWPASS_SCOPE_EXPECTED_FREQUENCY: &str = "1.00 kHz sine, fc about 1.59 kHz";
const COMPARATOR_THRESHOLD_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/comparator_threshold_scope/project.yaml";
const COMPARATOR_THRESHOLD_SCOPE_EXAMPLE_NAME: &str = "comparator_threshold_scope";
const COMPARATOR_THRESHOLD_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v(input)", "v(reference)", "v(output)", "v(vcc)"];
const COMPARATOR_THRESHOLD_SCOPE_EXPECTED_FREQUENCY: &str =
    "80 us input pulse crossing a 1.2 V reference";
const OPAMP_BUFFER_SCOPE_EXAMPLE_PROJECT: &str = "examples/good_ideal_opamp_buffer/project.yaml";
const OPAMP_BUFFER_SCOPE_EXAMPLE_NAME: &str = "good_ideal_opamp_buffer";
const OPAMP_BUFFER_SCOPE_EXPECTED_TRACES: &[&str] = &["v(input)", "v(output)", "v(vcc)"];
const OPAMP_BUFFER_SCOPE_EXPECTED_FREQUENCY: &str = "80 us input pulse through unity feedback";
const AP2112K_LDO_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_ap2112k_3v3_ldo_observation/project.yaml";
const AP2112K_LDO_SCOPE_EXAMPLE_NAME: &str = "good_ap2112k_3v3_ldo_observation";
const AP2112K_LDO_SCOPE_EXPECTED_TRACES: &[&str] = &["v_usb", "v_en", "v_rail3v3", "i_load"];
const AP2112K_LDO_SCOPE_EXPECTED_FREQUENCY: &str = "5 V enabled input, 3.3 V regulated load rail";
const TLV803_RESET_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tlv803ea29_reset_observation/project.yaml";
const TLV803_RESET_SCOPE_EXAMPLE_NAME: &str = "good_tlv803ea29_reset_observation";
const TLV803_RESET_SCOPE_EXPECTED_TRACES: &[&str] = &["v_rail", "reset_n"];
const TLV803_RESET_SCOPE_EXPECTED_FREQUENCY: &str = "3.3 V rail ramp with reset release";
const LOOP_STABILITY_BODE_EXAMPLE_PROJECT: &str = "examples/loop_stability_bode_scope/project.yaml";
const LOOP_STABILITY_BODE_EXAMPLE_NAME: &str = "loop_stability_bode_scope";
const LOOP_STABILITY_BODE_EXPECTED_TRACES: &[&str] = &["loop_mag_db", "loop_phase_deg", "loop_mag"];
const LOOP_STABILITY_BODE_EXPECTED_FREQUENCY: &str =
    "Bode loop gain with phase margin >45 deg and gain margin >6 dB";
const GUI_PROJECT_EXAMPLES: &[GuiProjectExample] = &[
    GuiProjectExample {
        id: "ne555_astable_scope",
        category: "Timer",
        open_label: "Open NE555 Scope Example",
        run_label: "Open NE555 + Run Scopes",
        workflow_title: "NE555 Scope Workflow",
        summary: "Astable-style timer output with timing-node and source-current traces.",
        project_path: NE555_SCOPE_EXAMPLE_PROJECT,
        project_name: NE555_SCOPE_EXAMPLE_NAME,
        expected_traces: NE555_SCOPE_EXPECTED_TRACES,
        expected_frequency: NE555_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
    GuiProjectExample {
        id: "rc_lowpass_scope",
        category: "Filter",
        open_label: "Open RC Low-Pass Scope Example",
        run_label: "Open RC Low-Pass + Run Scopes",
        workflow_title: "RC Low-Pass Scope Workflow",
        summary: "1 kHz sine into a first-order low-pass for input/output comparison.",
        project_path: RC_LOWPASS_SCOPE_EXAMPLE_PROJECT,
        project_name: RC_LOWPASS_SCOPE_EXAMPLE_NAME,
        expected_traces: RC_LOWPASS_SCOPE_EXPECTED_TRACES,
        expected_frequency: RC_LOWPASS_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
    GuiProjectExample {
        id: "comparator_threshold_scope",
        category: "Comparator",
        open_label: "Open Comparator Threshold Example",
        run_label: "Open Comparator + Run Scopes",
        workflow_title: "Comparator Threshold Workflow",
        summary: "Pulse input against a DC reference for output-state threshold checks.",
        project_path: COMPARATOR_THRESHOLD_SCOPE_EXAMPLE_PROJECT,
        project_name: COMPARATOR_THRESHOLD_SCOPE_EXAMPLE_NAME,
        expected_traces: COMPARATOR_THRESHOLD_SCOPE_EXPECTED_TRACES,
        expected_frequency: COMPARATOR_THRESHOLD_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("XU1"),
    },
    GuiProjectExample {
        id: "opamp_buffer_scope",
        category: "Op-Amp",
        open_label: "Open Op-Amp Buffer Example",
        run_label: "Open Op-Amp Buffer + Run Scopes",
        workflow_title: "Op-Amp Buffer Workflow",
        summary: "Unity-gain buffer tracking a pulse input with output settling checks.",
        project_path: OPAMP_BUFFER_SCOPE_EXAMPLE_PROJECT,
        project_name: OPAMP_BUFFER_SCOPE_EXAMPLE_NAME,
        expected_traces: OPAMP_BUFFER_SCOPE_EXPECTED_TRACES,
        expected_frequency: OPAMP_BUFFER_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("XU1"),
    },
    GuiProjectExample {
        id: "ap2112k_ldo_scope",
        category: "Regulator",
        open_label: "Open AP2112K LDO Example",
        run_label: "Open AP2112K + Run Scopes",
        workflow_title: "AP2112K LDO Workflow",
        summary: "Enabled 3.3 V LDO rail with load-current and output-window checks.",
        project_path: AP2112K_LDO_SCOPE_EXAMPLE_PROJECT,
        project_name: AP2112K_LDO_SCOPE_EXAMPLE_NAME,
        expected_traces: AP2112K_LDO_SCOPE_EXPECTED_TRACES,
        expected_frequency: AP2112K_LDO_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UREG"),
    },
    GuiProjectExample {
        id: "tlv803_reset_scope",
        category: "Reset",
        open_label: "Open TLV803 Reset Example",
        run_label: "Open TLV803 + Run Scopes",
        workflow_title: "TLV803 Reset Workflow",
        summary: "Reset-supervisor threshold release from a pulsed 3.3 V rail.",
        project_path: TLV803_RESET_SCOPE_EXAMPLE_PROJECT,
        project_name: TLV803_RESET_SCOPE_EXAMPLE_NAME,
        expected_traces: TLV803_RESET_SCOPE_EXPECTED_TRACES,
        expected_frequency: TLV803_RESET_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("URESET"),
    },
    GuiProjectExample {
        id: "loop_stability_bode_scope",
        category: "Stability",
        open_label: "Open Loop Stability Bode Example",
        run_label: "Open Loop Stability + Run Scopes",
        workflow_title: "Loop Stability Bode Workflow",
        summary: "Open-loop Bode response with executable phase and gain margin checks.",
        project_path: LOOP_STABILITY_BODE_EXAMPLE_PROJECT,
        project_name: LOOP_STABILITY_BODE_EXAMPLE_NAME,
        expected_traces: LOOP_STABILITY_BODE_EXPECTED_TRACES,
        expected_frequency: LOOP_STABILITY_BODE_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GuiProjectExample {
    pub(super) id: &'static str,
    pub(super) category: &'static str,
    pub(super) open_label: &'static str,
    pub(super) run_label: &'static str,
    pub(super) workflow_title: &'static str,
    pub(super) summary: &'static str,
    pub(super) project_path: &'static str,
    pub(super) project_name: &'static str,
    pub(super) expected_traces: &'static [&'static str],
    pub(super) expected_frequency: &'static str,
    pub(super) observation_preset_component: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScopeExampleWorkflowStatus {
    pub(super) title: &'static str,
    pub(super) state: &'static str,
    pub(super) action: &'static str,
    pub(super) expected_traces: &'static [&'static str],
    pub(super) expected_frequency: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PendingProjectAction {
    LoadProjectSummary { path: String },
    LoadProjectSummaryAndRunScopes { path: String },
    LoadProjectYaml { path: String },
    ImportKiCadSchematic,
    ImportKiCadPcb,
    ImportSpiceDeck,
    Quit,
}

impl PendingProjectAction {
    fn label(&self) -> &'static str {
        match self {
            Self::LoadProjectSummary { .. } => "load another project",
            Self::LoadProjectSummaryAndRunScopes { .. } => "load another project and run scopes",
            Self::LoadProjectYaml { .. } => "reload project YAML",
            Self::ImportKiCadSchematic => "import a KiCad schematic",
            Self::ImportKiCadPcb => "import KiCad PCB evidence",
            Self::ImportSpiceDeck => "import a SPICE deck",
            Self::Quit => "quit CircuitCI",
        }
    }
}

pub(super) fn gui_project_examples() -> &'static [GuiProjectExample] {
    GUI_PROJECT_EXAMPLES
}

impl CircuitCiApp {
    pub(super) fn selected_project_example(&self) -> GuiProjectExample {
        GUI_PROJECT_EXAMPLES
            .iter()
            .copied()
            .find(|example| example.id == self.selected_project_example_id)
            .unwrap_or(GUI_PROJECT_EXAMPLES[0])
    }

    pub(super) fn scope_example_workflow_status(&self) -> Option<ScopeExampleWorkflowStatus> {
        let example = self.active_scope_project_example()?;
        let (state, action) = if self.background_job_elapsed_secs().is_some() {
            (
                "Validation running",
                "Wait for waveform loading, then use Scope Activity or Scopes traces.",
            )
        } else if !self.waveforms.is_empty() {
            (
                "Waveforms loaded",
                "Inspect Scope Activity rows, Cursor A, edge stepping, snapshots, or report bundles.",
            )
        } else {
            (
                "Ready",
                "Use Run + Scopes to simulate and open oscilloscope traces.",
            )
        };
        Some(ScopeExampleWorkflowStatus {
            title: example.workflow_title,
            state,
            action,
            expected_traces: example.expected_traces,
            expected_frequency: example.expected_frequency,
        })
    }

    pub(super) fn request_project_example_load(
        &mut self,
        example: GuiProjectExample,
        ctx: Option<&egui::Context>,
    ) {
        self.request_project_action(
            PendingProjectAction::LoadProjectSummary {
                path: example.project_path.to_string(),
            },
            ctx,
        );
    }

    pub(super) fn request_project_example_load_and_run_scopes(
        &mut self,
        example: GuiProjectExample,
        ctx: Option<&egui::Context>,
    ) {
        self.request_project_action(
            PendingProjectAction::LoadProjectSummaryAndRunScopes {
                path: example.project_path.to_string(),
            },
            ctx,
        );
    }

    pub(super) fn run_scope_example_workflow_scopes(&mut self) -> bool {
        if self.active_scope_project_example().is_none() {
            self.status =
                "Open a scope-ready example before using this workflow action.".to_string();
            return false;
        }
        let was_running = self.background_job_elapsed_secs().is_some();
        self.run_schematic_model_open_scopes();
        self.background_job_elapsed_secs().is_some()
            || (was_running && self.stage == Stage::Simulation)
    }

    pub(super) fn open_scope_example_workflow_activity(&mut self) -> bool {
        if self.active_scope_project_example().is_none() {
            self.status =
                "Open a scope-ready example before using this workflow action.".to_string();
            return false;
        }
        self.stage = Stage::Sketch;
        self.sketch_runtime_scope_overlay_visible = true;
        self.sketch_scope_activity_window_open = true;
        if self.waveforms.is_empty() {
            self.status =
                "Scope Activity window opened; run validation to load waveform traces.".to_string();
        } else {
            self.status = "Opened the floating Scope Activity window.".to_string();
        }
        self.push_diagnostic("Example workflow opened the floating Scope Activity window.");
        true
    }

    pub(super) fn create_scope_example_observation_preset(&mut self) -> bool {
        let Some(example) = self.active_scope_project_example() else {
            self.status =
                "Open a scope-ready example before using this workflow action.".to_string();
            return false;
        };
        let Some(component_id) = example.observation_preset_component else {
            self.status = format!(
                "{} does not declare a model-aware observation preset shortcut.",
                example.project_name
            );
            return false;
        };
        let changed = self.apply_create_library_observation_preset(component_id);
        if changed {
            self.stage = Stage::Sketch;
            self.set_single_sketch_selection(Some(SketchSelection::Component(
                component_id.to_string(),
            )));
            self.push_diagnostic(&format!(
                "Example workflow created model-aware observation checks for {component_id}."
            ));
        }
        changed
    }

    pub(super) fn scope_example_workflow_action_buttons(&mut self, ui: &mut egui::Ui) {
        let can_start_run = self.background_job_elapsed_secs().is_none();
        let can_create_checks = self
            .active_scope_project_example()
            .and_then(|example| example.observation_preset_component)
            .is_some();
        if ui
            .add_enabled(can_start_run, egui::Button::new("Run + Scopes"))
            .clicked()
        {
            self.run_scope_example_workflow_scopes();
        }
        if ui
            .add_enabled(can_create_checks, egui::Button::new("Create Checks"))
            .clicked()
        {
            self.create_scope_example_observation_preset();
        }
        if ui.button("Open Scope Activity").clicked() {
            self.open_scope_example_workflow_activity();
        }
    }

    pub(super) fn sketch_scope_example_workflow_strip(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(status) = self.scope_example_workflow_status() else {
            return false;
        };
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(status.title);
                ui.label(format!("State: {}", status.state));
                self.scope_example_workflow_action_buttons(ui);
            });
            ui.label(format!("Next: {}", status.action));
            ui.label(format!(
                "Expected: {} | {}",
                status.expected_traces.join(", "),
                status.expected_frequency
            ));
        });
        true
    }

    fn active_scope_project_example(&self) -> Option<GuiProjectExample> {
        GUI_PROJECT_EXAMPLES.iter().copied().find(|example| {
            self.project_path == example.project_path
                || self
                    .project_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.name == example.project_name)
        })
    }

    fn prepare_scope_example_sketch_open(&mut self) -> bool {
        let Some(example) = self.active_scope_project_example() else {
            return false;
        };
        self.stage = Stage::Sketch;
        self.sketch_viewport_command = Some(SketchViewportCommand::FitAll);
        self.status = format!(
            "Opened {} in Sketch; fitting routed schematic.",
            example.project_name
        );
        self.push_diagnostic(&format!(
            "Opened scope-ready example {} in Sketch with routed schematic fit requested.",
            example.project_name
        ));
        true
    }

    pub(super) fn handle_close_request(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.viewport().close_requested()) && self.has_unsaved_project_work()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.pending_project_action.is_none() {
                self.request_project_action(PendingProjectAction::Quit, Some(ctx));
            }
        }
    }

    pub(super) fn request_project_action(
        &mut self,
        action: PendingProjectAction,
        ctx: Option<&egui::Context>,
    ) {
        if self.has_unsaved_project_work() {
            let label = action.label();
            self.pending_project_action = Some(action);
            self.status = format!("Confirm unsaved changes before {label}.");
            self.push_diagnostic(
                "Unsaved edits require confirmation before replacing the project workspace.",
            );
        } else {
            self.execute_project_action(action, ctx);
        }
    }

    pub(super) fn unsaved_project_action_dialog(&mut self, ctx: &egui::Context) {
        let Some(action) = self.pending_project_action.clone() else {
            return;
        };
        let action_label = action.label();
        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "You have unsaved edits. Save or discard them before you {action_label}."
                ));
                if self.project_yaml_dirty {
                    ui.label("Board IR YAML has unsaved edits.");
                }
                if self.spice_deck_dirty {
                    ui.label("The loaded SPICE deck has unsaved edits.");
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.project_yaml_dirty,
                            egui::Button::new("Save Project YAML"),
                        )
                        .clicked()
                    {
                        self.save_project_yaml();
                        if !self.has_unsaved_project_work()
                            && let Some(action) = self.pending_project_action.take()
                        {
                            self.execute_project_action(action, Some(ctx));
                        }
                    }
                    if ui.button("Continue Without Saving").clicked() {
                        self.discard_unsaved_project_work();
                        if let Some(action) = self.pending_project_action.take() {
                            self.execute_project_action(action, Some(ctx));
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending_project_action = None;
                        self.status = "Canceled project replacement.".to_string();
                    }
                });
                if self.spice_deck_dirty {
                    ui.label(
                        "Save SPICE deck edits from Observations > File-backed SPICE Deck before continuing.",
                    );
                }
            });
    }

    fn execute_project_action(
        &mut self,
        action: PendingProjectAction,
        ctx: Option<&egui::Context>,
    ) {
        match action {
            PendingProjectAction::LoadProjectSummary { path } => {
                self.project_path = path;
                if self.load_project_summary_unchecked() {
                    self.prepare_scope_example_sketch_open();
                }
            }
            PendingProjectAction::LoadProjectSummaryAndRunScopes { path } => {
                self.project_path = path;
                if self.load_project_summary_unchecked() {
                    self.run_schematic_model_open_scopes();
                }
            }
            PendingProjectAction::LoadProjectYaml { path } => {
                self.project_path = path;
                self.load_project_yaml_unchecked();
            }
            PendingProjectAction::ImportKiCadSchematic => self.import_kicad_schematic(),
            PendingProjectAction::ImportKiCadPcb => self.import_kicad_pcb(),
            PendingProjectAction::ImportSpiceDeck => self.import_spice_deck(),
            PendingProjectAction::Quit => {
                if let Some(ctx) = ctx {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    fn has_unsaved_project_work(&self) -> bool {
        self.project_yaml_dirty || self.spice_deck_dirty
    }

    fn discard_unsaved_project_work(&mut self) {
        self.project_yaml_dirty = false;
        self.spice_deck_dirty = false;
        self.spice_deck_text.clear();
        self.clear_project_yaml_history();
        self.status = "Discarded unsaved edits.".to_string();
    }

    pub(super) fn load_project_summary_unchecked(&mut self) -> bool {
        match load_project_snapshot(Path::new(&self.project_path)) {
            Ok(snapshot) => {
                let loaded_name = snapshot.name.clone();
                self.status = format!("Loaded {}", snapshot.name);
                self.project_snapshot = Some(snapshot);
                self.set_single_sketch_selection(None);
                let yaml_loaded = self.project_yaml_dirty || self.load_project_yaml_unchecked();
                self.push_diagnostic(&format!("Project summary loaded for {loaded_name}."));
                yaml_loaded
            }
            Err(error) => {
                self.record_error(error);
                false
            }
        }
    }

    pub(super) fn load_project_yaml_unchecked(&mut self) -> bool {
        match std::fs::read_to_string(Path::new(&self.project_path))
            .with_context(|| format!("Failed to read {}.", self.project_path))
            .and_then(|text| {
                validate_board_ir_yaml_text(&text)?;
                Ok(text)
            }) {
            Ok(text) => {
                self.project_yaml = text;
                self.project_yaml_dirty = false;
                self.spice_deck_dirty = false;
                self.spice_deck_text.clear();
                self.sketch_clipboard_components.clear();
                self.sketch_paste_requested = false;
                self.sketch_net_label_place_armed = false;
                self.sketch_net_label_edit = None;
                self.sketch_component_inline_edit = None;
                self.sketch_component_label_drag = None;
                self.sketch_probe_element_drag = None;
                self.sketch_scope_activity_window_open = false;
                self.sketch_selection_box_drag = None;
                self.sketch_selection_lasso_drag = None;
                self.clear_project_yaml_history();
                self.stage = Stage::Sketch;
                self.status = "Project YAML loaded.".to_string();
                self.push_diagnostic("Project YAML loaded into Sketch workspace.");
                true
            }
            Err(error) => {
                self.record_error(error);
                false
            }
        }
    }

    pub(super) fn save_project_yaml(&mut self) {
        match validate_board_ir_yaml_text(&self.project_yaml).and_then(|()| {
            std::fs::write(Path::new(&self.project_path), &self.project_yaml)
                .with_context(|| format!("Failed to write {}.", self.project_path))
        }) {
            Ok(()) => {
                self.project_yaml_dirty = false;
                match load_project_snapshot_from_yaml(&self.project_yaml) {
                    Ok(snapshot) => {
                        self.project_snapshot = Some(snapshot);
                    }
                    Err(error) => {
                        self.record_error(error);
                        return;
                    }
                }
                self.status = "Project YAML saved.".to_string();
                self.push_diagnostic("Project YAML saved after schema parse validation.");
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn validate_project_yaml_text(&mut self) {
        match validate_board_ir_yaml_text(&self.project_yaml) {
            Ok(()) => {
                self.status = "Project YAML parses.".to_string();
                self.push_diagnostic("Project YAML parse validation passed.");
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_edited_project_yaml(&mut self, updated: String, message: &str) {
        match load_project_snapshot_from_yaml(&updated) {
            Ok(snapshot) => {
                self.push_project_yaml_undo(self.project_yaml.clone());
                self.project_yaml = updated;
                self.project_yaml_dirty = true;
                self.project_snapshot = Some(snapshot);
                self.status = message.to_string();
                self.push_diagnostic(message);
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn record_project_yaml_text_edit(&mut self, previous_yaml: String) {
        self.push_project_yaml_undo(previous_yaml);
        self.project_yaml_dirty = true;
        if let Ok(snapshot) = load_project_snapshot_from_yaml(&self.project_yaml) {
            self.project_snapshot = Some(snapshot);
        }
    }

    pub(super) fn undo_project_yaml_edit(&mut self) {
        let Some(previous) = self.project_yaml_undo.pop() else {
            return;
        };
        push_limited_history(
            &mut self.project_yaml_redo,
            self.project_yaml.clone(),
            PROJECT_YAML_HISTORY_LIMIT,
        );
        self.restore_project_yaml_history_entry(previous, "Undo applied.");
    }

    pub(super) fn redo_project_yaml_edit(&mut self) {
        let Some(next) = self.project_yaml_redo.pop() else {
            return;
        };
        push_limited_history(
            &mut self.project_yaml_undo,
            self.project_yaml.clone(),
            PROJECT_YAML_HISTORY_LIMIT,
        );
        self.restore_project_yaml_history_entry(next, "Redo applied.");
    }

    fn restore_project_yaml_history_entry(&mut self, yaml: String, message: &str) {
        self.project_yaml = yaml;
        self.project_yaml_dirty = true;
        if let Ok(snapshot) = load_project_snapshot_from_yaml(&self.project_yaml) {
            self.project_snapshot = Some(snapshot);
        }
        self.status = message.to_string();
        self.push_diagnostic(message);
    }

    fn push_project_yaml_undo(&mut self, previous_yaml: String) {
        if previous_yaml.is_empty() {
            return;
        }
        push_limited_history(
            &mut self.project_yaml_undo,
            previous_yaml,
            PROJECT_YAML_HISTORY_LIMIT,
        );
        self.project_yaml_redo.clear();
    }

    fn clear_project_yaml_history(&mut self) {
        self.project_yaml_undo.clear();
        self.project_yaml_redo.clear();
    }
}

pub(super) fn optional_path(text: &str) -> Option<PathBuf> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(Path::new(text).to_path_buf())
    }
}

pub(super) fn sanitized_project_name(path: &Path, fallback: &str) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(sanitize_identifier)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn sanitize_identifier(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
            result.push(character);
        } else if !result.ends_with('_') {
            result.push('_');
        }
    }
    result.trim_matches('_').to_string()
}

fn push_limited_history(stack: &mut Vec<String>, value: String, limit: usize) {
    if stack.last().is_some_and(|last| last == &value) {
        return;
    }
    stack.push(value);
    if stack.len() > limit {
        stack.remove(0);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PROJECT_YAML_HISTORY_LIMIT, PendingProjectAction, optional_path, push_limited_history,
        sanitized_project_name,
    };
    use crate::gui::CircuitCiApp;
    use crate::gui::sketch::{
        edit_component_model, edit_component_part_number, load_project_snapshot_from_yaml,
    };
    use std::path::Path;
    use tempfile::tempdir;

    fn editable_project_yaml() -> &'static str {
        "project:
  name: gui_project_history_test
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
    fn project_yaml_undo_redo_round_trips_validated_edit() {
        let mut app = CircuitCiApp {
            project_yaml: editable_project_yaml().to_string(),
            project_snapshot: Some(
                load_project_snapshot_from_yaml(editable_project_yaml()).unwrap(),
            ),
            ..CircuitCiApp::default()
        };
        let edited = edit_component_model(&app.project_yaml, "R1", "vendor.test.resistor").unwrap();
        app.apply_edited_project_yaml(edited, "edited");

        assert_eq!(app.project_yaml_undo.len(), 1);
        assert!(app.project_yaml.contains("vendor.test.resistor"));

        app.undo_project_yaml_edit();
        assert!(app.project_yaml.contains("generic.analog.resistor"));
        assert_eq!(app.project_yaml_redo.len(), 1);

        app.redo_project_yaml_edit();
        assert!(app.project_yaml.contains("vendor.test.resistor"));
        assert_eq!(app.project_yaml_undo.len(), 1);
    }

    #[test]
    fn project_yaml_branch_edit_clears_redo_stack() {
        let mut app = CircuitCiApp {
            project_yaml: editable_project_yaml().to_string(),
            project_snapshot: Some(
                load_project_snapshot_from_yaml(editable_project_yaml()).unwrap(),
            ),
            ..CircuitCiApp::default()
        };
        let edited = edit_component_model(&app.project_yaml, "R1", "vendor.test.resistor").unwrap();
        app.apply_edited_project_yaml(edited, "edited");
        app.undo_project_yaml_edit();
        assert_eq!(app.project_yaml_redo.len(), 1);

        let branched = edit_component_part_number(&app.project_yaml, "R1", "RC0603").unwrap();
        app.apply_edited_project_yaml(branched, "branched");
        assert!(app.project_yaml_redo.is_empty());
        assert!(app.project_yaml.contains("RC0603"));
    }

    #[test]
    fn project_yaml_history_is_capped() {
        let mut history = Vec::new();
        for index in 0..(PROJECT_YAML_HISTORY_LIMIT + 3) {
            push_limited_history(
                &mut history,
                format!("snapshot-{index}"),
                PROJECT_YAML_HISTORY_LIMIT,
            );
        }
        assert_eq!(history.len(), PROJECT_YAML_HISTORY_LIMIT);
        assert_eq!(history.first().unwrap(), "snapshot-3");
    }

    #[test]
    fn dirty_project_action_is_queued_without_replacing_workspace() {
        let mut app = CircuitCiApp {
            project_path: "original.yaml".to_string(),
            project_yaml: editable_project_yaml().to_string(),
            project_yaml_dirty: true,
            ..CircuitCiApp::default()
        };

        app.request_project_action(
            PendingProjectAction::LoadProjectYaml {
                path: "replacement.yaml".to_string(),
            },
            None,
        );

        assert_eq!(app.project_path, "original.yaml");
        assert_eq!(
            app.pending_project_action,
            Some(PendingProjectAction::LoadProjectYaml {
                path: "replacement.yaml".to_string(),
            })
        );
        assert!(app.status.contains("Confirm unsaved changes"));
    }

    #[test]
    fn clean_project_action_executes_immediately() {
        let temp = tempdir().unwrap();
        let project_path = temp.path().join("project.yaml");
        std::fs::write(&project_path, editable_project_yaml()).unwrap();
        let mut app = CircuitCiApp::default();

        app.request_project_action(
            PendingProjectAction::LoadProjectYaml {
                path: project_path.to_string_lossy().into_owned(),
            },
            None,
        );

        assert!(app.pending_project_action.is_none());
        assert_eq!(
            app.project_path,
            project_path.to_string_lossy().into_owned()
        );
        assert!(app.project_yaml.contains("gui_project_history_test"));
        assert!(!app.project_yaml_dirty);
    }

    #[test]
    fn discard_then_continue_loads_replacement_project() {
        let temp = tempdir().unwrap();
        let project_path = temp.path().join("project.yaml");
        std::fs::write(&project_path, editable_project_yaml()).unwrap();
        let mut app = CircuitCiApp {
            project_path: "original.yaml".to_string(),
            project_yaml: "dirty yaml".to_string(),
            project_yaml_dirty: true,
            project_yaml_undo: vec!["old yaml".to_string()],
            spice_deck_text: "dirty deck".to_string(),
            spice_deck_dirty: true,
            ..CircuitCiApp::default()
        };

        app.request_project_action(
            PendingProjectAction::LoadProjectYaml {
                path: project_path.to_string_lossy().into_owned(),
            },
            None,
        );
        app.discard_unsaved_project_work();
        let action = app.pending_project_action.take().unwrap();
        app.execute_project_action(action, None);

        assert_eq!(
            app.project_path,
            project_path.to_string_lossy().into_owned()
        );
        assert!(app.project_yaml.contains("gui_project_history_test"));
        assert!(!app.project_yaml_dirty);
        assert!(!app.spice_deck_dirty);
        assert!(app.spice_deck_text.is_empty());
        assert!(app.project_yaml_undo.is_empty());
    }

    #[test]
    fn optional_path_ignores_blank_mapping_path() {
        assert!(optional_path("  ").is_none());
        assert_eq!(
            optional_path("mapping.yaml").unwrap(),
            Path::new("mapping.yaml").to_path_buf()
        );
    }

    #[test]
    fn sanitized_project_name_uses_file_stem() {
        assert_eq!(
            sanitized_project_name(Path::new("some dir/root.kicad_sch"), "fallback"),
            "root"
        );
        assert_eq!(
            sanitized_project_name(Path::new("bad name!.kicad_sch"), "fallback"),
            "bad_name"
        );
    }
}
