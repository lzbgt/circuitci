use super::CircuitCiApp;
use super::analog::{
    AnalogProbeDraft, AnalogScenarioDraft, append_analog_transient_scenario_with_project_path,
    append_analog_voltage_probe,
};
use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

impl CircuitCiApp {
    pub(super) fn simulation_stage(&mut self, ui: &mut egui::Ui) {
        self.scope_run_toolbar(ui);
        ui.separator();

        let available = ui.available_size();
        let side_width = (available.x * 0.30).clamp(320.0, 420.0);
        let gap = 8.0;
        if available.x >= 980.0 {
            let scope_size = egui::vec2(
                (available.x - side_width - gap).max(560.0),
                available.y.max(520.0),
            );
            ui.horizontal_top(|ui| {
                self.waveform_scope_view(ui, scope_size);
                ui.add_space(gap);
                ui.allocate_ui_with_layout(
                    egui::vec2(side_width, scope_size.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.scope_side_dock(ui),
                );
            });
        } else {
            self.waveform_scope_view(
                ui,
                egui::vec2(available.x.max(560.0), (available.y * 0.62).max(360.0)),
            );
            ui.separator();
            self.scope_side_dock(ui);
        }
    }

    fn scope_run_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Scopes");
            if ui.button("Model").clicked() {
                self.stage = super::Stage::Sketch;
            }
            if ui
                .add_enabled(
                    self.background_job_elapsed_secs().is_none() && self.project_snapshot.is_some(),
                    egui::Button::new("Run"),
                )
                .clicked()
            {
                self.run_scope_model();
            }
            self.scope_auto_probe_button(ui);
            self.scope_auto_probe_run_toggle(ui);
            if ui.button("Fit Time").clicked() {
                self.fit_waveform_time_window();
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
            if let Some(report) = &self.report {
                ui.label(format!(
                    "result {}: critical {} / warning {} / info {}",
                    report.result,
                    report.summary.critical,
                    report.summary.warning,
                    report.summary.info
                ));
            }
        });
    }

    fn run_scope_model(&mut self) {
        self.run_model_with_scope_preparation();
    }

    pub(super) fn run_model_with_scope_preparation(&mut self) -> bool {
        if self.project_yaml_dirty {
            self.save_project_yaml();
            if self.project_yaml_dirty {
                return false;
            }
        }
        match self.prepare_auto_scope_probes_for_run() {
            Ok(true) => {
                self.save_project_yaml();
                if self.project_yaml_dirty {
                    return false;
                }
            }
            Ok(false) => {}
            Err(error) => {
                self.record_error(error);
                return false;
            }
        }
        match self.prepare_scope_run_inputs() {
            Ok(true) => {
                self.save_project_yaml();
                if self.project_yaml_dirty {
                    return false;
                }
            }
            Ok(false) => {}
            Err(error) => {
                self.record_error(error);
                return false;
            }
        }
        let had_background_job = self.background_job_elapsed_secs().is_some();
        self.validate_project();
        !had_background_job && self.background_job_elapsed_secs().is_some()
    }

    fn prepare_scope_run_inputs(&mut self) -> Result<bool> {
        let preparation = prepare_scope_run_yaml(
            &self.project_yaml,
            Path::new(&self.project_path),
            &self.analog_scenario_name,
            &self.analog_probe_name,
            self.analog_stop_time_us,
            self.analog_max_step_us,
        )?;
        let Some((updated, preparation)) = preparation else {
            return Ok(false);
        };
        self.remember_scope_probe_target(preparation.scenario_name(), preparation.probe_name());
        let status = preparation.status_message();
        self.apply_edited_project_yaml(updated, &status);
        Ok(true)
    }

    fn scope_side_dock(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.scope_run_readiness_panel(ui);
            ui.separator();
            self.waveform_controls_panel(ui);
            ui.separator();
            if let Some(snapshot) = self.project_snapshot.clone() {
                self.analog_scenario_editor(ui, &snapshot);
                self.analog_sweep_editor(ui);
                self.analog_generated_overview_panel(ui);
                self.analog_generated_settings_editor(ui);
                self.analog_generated_components_editor(ui);
                self.analog_stimulus_editor(ui);
                self.analog_model_file_manager(ui);
                self.selected_probe_assertions_panel(ui);
                self.analog_assertion_editor(ui);
            }
            self.spice_deck_editor(ui);
            self.scope_artifacts_and_findings(ui);
        });
    }

    fn scope_artifacts_and_findings(&mut self, ui: &mut egui::Ui) {
        if self.report.is_some() {
            ui.separator();
            let report = self.report.clone().expect("checked above");
            egui::CollapsingHeader::new("Artifacts")
                .default_open(false)
                .show(ui, |ui| {
                    ui.label("Waveforms");
                    if report.waveforms.is_empty() {
                        ui.label("No waveform artifacts were emitted by the current run setup.");
                    } else {
                        for waveform in &report.waveforms {
                            ui.monospace(waveform);
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Artifacts");
                    if report.artifacts.is_empty() {
                        ui.label("No artifacts were emitted.");
                    } else {
                        for artifact in &report.artifacts {
                            ui.monospace(artifact);
                        }
                    }
                });
            egui::CollapsingHeader::new("Findings")
                .default_open(false)
                .show(ui, |ui| {
                    self.findings_view(ui, &report);
                });
        } else {
            ui.separator();
            ui.label("Run validation to observe SPICE waveforms, generated decks, and findings.");
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScopeRunPreparation {
    AddedScenario {
        scenario_name: String,
        probe_name: String,
        net_id: String,
    },
    AddedProbe {
        scenario_name: String,
        probe_name: String,
        net_id: String,
    },
}

impl ScopeRunPreparation {
    fn scenario_name(&self) -> &str {
        match self {
            Self::AddedScenario { scenario_name, .. } | Self::AddedProbe { scenario_name, .. } => {
                scenario_name
            }
        }
    }

    fn probe_name(&self) -> &str {
        match self {
            Self::AddedScenario { probe_name, .. } | Self::AddedProbe { probe_name, .. } => {
                probe_name
            }
        }
    }

    fn status_message(&self) -> String {
        match self {
            Self::AddedScenario {
                scenario_name,
                probe_name,
                net_id,
            } => format!(
                "Run created transient setup {scenario_name} with voltage probe {probe_name} on net {net_id}."
            ),
            Self::AddedProbe {
                scenario_name,
                probe_name,
                net_id,
            } => format!(
                "Run added voltage probe {probe_name} on net {net_id} to run setup {scenario_name}."
            ),
        }
    }
}

fn prepare_scope_run_yaml(
    text: &str,
    project_path: &Path,
    preferred_scenario_name: &str,
    preferred_probe_name: &str,
    stop_time_us: f64,
    max_step_us: f64,
) -> Result<Option<(String, ScopeRunPreparation)>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if project
        .scenarios
        .iter()
        .filter_map(|scenario| scenario.analog.as_ref())
        .any(|analog| !analog.probes.is_empty())
    {
        return Ok(None);
    }

    if let Some((scenario_name, net_id, probe_name)) =
        scope_probe_for_existing_analog_scenario(&project, preferred_probe_name)?
    {
        let draft = AnalogProbeDraft {
            scenario_name: scenario_name.clone(),
            net_id: net_id.clone(),
            probe_name: probe_name.clone(),
        };
        let updated = append_analog_voltage_probe(text, &draft)?;
        return Ok(Some((
            updated,
            ScopeRunPreparation::AddedProbe {
                scenario_name,
                probe_name,
                net_id,
            },
        )));
    }

    let ground_net = default_scope_ground_net(&project)?;
    let probe_net = default_scope_probe_net(&project)?;
    let scenario_name = unique_scope_scenario_name(&project, preferred_scenario_name);
    let probe_name = nonblank_id(preferred_probe_name, "probe_voltage");
    let draft = AnalogScenarioDraft {
        name: scenario_name.clone(),
        ground_net,
        probe_net: probe_net.clone(),
        probe_name: probe_name.clone(),
        stop_time_us,
        max_step_us,
    };
    let updated = append_analog_transient_scenario_with_project_path(text, project_path, &draft)?;
    Ok(Some((
        updated,
        ScopeRunPreparation::AddedScenario {
            scenario_name,
            probe_name,
            net_id: probe_net,
        },
    )))
}

fn scope_probe_for_existing_analog_scenario(
    project: &crate::board_ir::BoardProject,
    preferred_probe_name: &str,
) -> Result<Option<(String, String, String)>> {
    for scenario in &project.scenarios {
        let Some(analog) = scenario.analog.as_ref() else {
            continue;
        };
        let Some(net_id) = analog
            .node_bindings
            .iter()
            .map(|binding| binding.net.as_str())
            .find(|net_id| {
                project
                    .board
                    .nets
                    .get(*net_id)
                    .is_some_and(|net| net.kind != crate::board_ir::NetKind::Ground)
            })
            .or_else(|| {
                analog
                    .node_bindings
                    .first()
                    .map(|binding| binding.net.as_str())
            })
        else {
            anyhow::bail!(
                "Run setup {} has no node bindings; add a voltage probe manually after binding schematic nets.",
                scenario.name
            );
        };
        let probe_name = unique_scope_probe_name(analog, preferred_probe_name);
        return Ok(Some((
            scenario.name.clone(),
            net_id.to_string(),
            probe_name,
        )));
    }
    Ok(None)
}

fn default_scope_ground_net(project: &crate::board_ir::BoardProject) -> Result<String> {
    project
        .board
        .nets
        .iter()
        .find_map(|(id, net)| (net.kind == crate::board_ir::NetKind::Ground).then(|| id.clone()))
        .context("Run needs a ground net before it can create a default transient setup.")
}

fn default_scope_probe_net(project: &crate::board_ir::BoardProject) -> Result<String> {
    project
        .board
        .nets
        .iter()
        .find_map(|(id, net)| (net.kind != crate::board_ir::NetKind::Ground).then(|| id.clone()))
        .or_else(|| project.board.nets.keys().next().cloned())
        .context("Run needs at least one schematic net before it can create a default scope probe.")
}

fn unique_scope_scenario_name(
    project: &crate::board_ir::BoardProject,
    preferred_scenario_name: &str,
) -> String {
    let existing = project
        .scenarios
        .iter()
        .map(|scenario| scenario.name.as_str())
        .collect::<Vec<_>>();
    unique_id(preferred_scenario_name, "gui_transient", &existing)
}

fn unique_scope_probe_name(
    analog: &crate::board_ir::AnalogScenario,
    preferred_probe_name: &str,
) -> String {
    let existing = analog
        .probes
        .iter()
        .map(|probe| probe.name.as_str())
        .collect::<Vec<_>>();
    unique_id(preferred_probe_name, "probe_voltage", &existing)
}

fn unique_id(preferred: &str, fallback: &str, existing: &[&str]) -> String {
    let base = nonblank_id(preferred, fallback);
    if !existing.iter().any(|name| *name == base) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}_{suffix}");
        if !existing.iter().any(|name| *name == candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search should always find a unique id")
}

fn nonblank_id(preferred: &str, fallback: &str) -> String {
    let trimmed = preferred.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{ScopeRunPreparation, prepare_scope_run_yaml};
    use std::path::Path;

    const BASE_PROJECT: &str = "project:
  name: scope_run_auto_probe_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.voltage_source
      spice:
        primitive: dc_voltage_source
        dc_voltage_v: 5.0
      pins:
        P: rail_5v
        N: gnd
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: rail_5v
        B: out
    C1:
      model: generic.analog.capacitor
      spice:
        primitive: capacitor
        value_f: 0.000001
      pins:
        A: out
        B: gnd
  nets:
    gnd: {kind: ground}
    out: {kind: digital_or_analog}
    rail_5v: {kind: power, nominal_voltage: 5.0, powered: true}
scenarios: []
";

    #[test]
    fn scope_run_preparation_adds_generated_scenario_and_probe() {
        let (updated, preparation) = prepare_scope_run_yaml(
            BASE_PROJECT,
            Path::new("project.yaml"),
            "gui_transient",
            "probe_voltage",
            100.0,
            1.0,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            preparation,
            ScopeRunPreparation::AddedScenario {
                scenario_name: "gui_transient".to_string(),
                probe_name: "probe_voltage".to_string(),
                net_id: "out".to_string(),
            }
        );
        let choices = crate::gui::analog::analog_scenario_choices(&updated).unwrap();
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].name, "gui_transient");
        assert_eq!(choices[0].probes[0].name, "probe_voltage");
    }

    #[test]
    fn scope_run_preparation_adds_probe_to_existing_empty_analog_scenario() {
        let project = BASE_PROJECT.replace(
            "scenarios: []",
            "scenarios:
  - name: existing_transient
    type: analog
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated: {components: [V1, R1, C1], ground_net: gnd}
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: out, net: out}
        - {node: rail_5v, net: rail_5v}
      pin_bindings:
        - {node: rail_5v, endpoint: {component: V1, pin: P}}
        - {node: '0', endpoint: {component: V1, pin: N}}
        - {node: rail_5v, endpoint: {component: R1, pin: A}}
        - {node: out, endpoint: {component: R1, pin: B}}
        - {node: out, endpoint: {component: C1, pin: A}}
        - {node: '0', endpoint: {component: C1, pin: B}}
      analysis: {type: tran, stop_time_us: 100.0, max_step_us: 1.0}
      stimuli: []
      probes: []
      assertions: []
",
        );
        let (updated, preparation) = prepare_scope_run_yaml(
            &project,
            Path::new("project.yaml"),
            "gui_transient",
            "probe_voltage",
            100.0,
            1.0,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            preparation,
            ScopeRunPreparation::AddedProbe {
                scenario_name: "existing_transient".to_string(),
                probe_name: "probe_voltage".to_string(),
                net_id: "out".to_string(),
            }
        );
        let choices = crate::gui::analog::analog_scenario_choices(&updated).unwrap();
        assert_eq!(choices[0].probes[0].name, "probe_voltage");
    }

    #[test]
    fn scope_run_preparation_keeps_existing_scope_probe() {
        let project = BASE_PROJECT.replace(
            "scenarios: []",
            "scenarios:
  - name: existing_transient
    type: analog
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated: {components: [V1, R1, C1], ground_net: gnd}
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: out, net: out}
      pin_bindings: []
      analysis: {type: tran, stop_time_us: 100.0, max_step_us: 1.0}
      stimuli: []
      probes:
        - {name: out_voltage, expression: V(out), quantity: voltage}
      assertions: []
",
        );

        assert!(
            prepare_scope_run_yaml(
                &project,
                Path::new("project.yaml"),
                "gui_transient",
                "probe_voltage",
                100.0,
                1.0,
            )
            .unwrap()
            .is_none()
        );
    }
}
