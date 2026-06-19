use anyhow::{Context, Result};
use eframe::egui;
use std::collections::BTreeSet;
use std::path::Path;

use super::analog::{
    AnalogCurrentProbeDraft, AnalogExpressionProbeDraft, AnalogProbeDraft,
    append_analog_current_probe, append_analog_expression_probe, append_analog_voltage_probe,
};
use super::analog_branches::current_probe_expression;
use super::{CircuitCiApp, ScopeProbeTarget};

const MAX_AUTO_VOLTAGE_PROBES: usize = 8;
const MAX_AUTO_CURRENT_PROBES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
struct AutoScopeProbeCandidate {
    scenario_name: String,
    probe_name: String,
    expression: String,
    kind: AutoScopeProbeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AutoScopeProbeKind {
    Voltage {
        net_id: String,
    },
    Current {
        component_id: String,
        generated: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AutoScopeProbePreviewRow {
    scenario_name: String,
    probe_name: String,
    expression: String,
    quantity: &'static str,
    target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AutoScopeProbeEdit {
    updated_yaml: String,
    added_voltage: usize,
    added_current: usize,
    first_target: Option<ScopeProbeTarget>,
}

impl AutoScopeProbeEdit {
    fn added_total(&self) -> usize {
        self.added_voltage + self.added_current
    }

    fn status_message(&self) -> String {
        format!(
            "Auto Scope Probes added {} probe(s): {} voltage, {} current.",
            self.added_total(),
            self.added_voltage,
            self.added_current
        )
    }
}

impl CircuitCiApp {
    pub(super) fn scope_auto_probe_button(&mut self, ui: &mut egui::Ui) {
        if ui
            .add_enabled(
                self.project_snapshot.is_some() && !self.project_yaml.trim().is_empty(),
                egui::Button::new("Auto Probes"),
            )
            .on_hover_text(
                "Add missing voltage probes for analog nodes and current probes for source branches.",
            )
            .clicked()
        {
            self.apply_auto_scope_probes();
        }
    }

    pub(super) fn scope_auto_probe_run_toggle(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.scope_auto_probes_before_run, "Auto before Run")
            .on_hover_text(
                "When enabled, Run first adds missing useful scope probes for analog nets and source branches.",
            );
    }

    pub(super) fn prepare_auto_scope_probes_for_run(&mut self) -> Result<bool> {
        if !self.scope_auto_probes_before_run {
            return Ok(false);
        }
        let edit =
            auto_scope_probe_project_yaml(&self.project_yaml, Path::new(&self.project_path))?;
        if edit.added_total() == 0 {
            self.push_diagnostic("Auto Scope Probes before Run found no missing useful probes.");
            return Ok(false);
        }
        let status = edit.status_message();
        let target = edit.first_target.clone();
        self.apply_edited_project_yaml(edit.updated_yaml, &status);
        if let Some(target) = target {
            self.remember_scope_probe_target(&target.scenario_name, &target.probe_name);
        }
        Ok(true)
    }

    pub(super) fn scope_run_readiness_panel(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Run Readiness")
            .default_open(true)
            .show(ui, |ui| {
                if self.project_yaml.trim().is_empty() {
                    ui.label("Load a project to preview Run scope probes.");
                    return;
                }

                ui.horizontal(|ui| {
                    ui.label("Auto probes");
                    self.scope_auto_probe_run_toggle(ui);
                });

                match auto_scope_probe_preview_rows(
                    &self.project_yaml,
                    Path::new(&self.project_path),
                ) {
                    Ok(rows) if rows.is_empty() => {
                        if self.scope_auto_probes_before_run {
                            ui.label(
                                "No missing useful Auto Probes. Run will use existing probes or the fallback transient probe path.",
                            );
                        } else {
                            ui.label("Auto-before-Run is off. Run will not add Auto Probes.");
                        }
                    }
                    Ok(rows) => {
                        let voltage = rows
                            .iter()
                            .filter(|row| row.quantity == "voltage")
                            .count();
                        let current = rows
                            .iter()
                            .filter(|row| row.quantity == "current")
                            .count();
                        if self.scope_auto_probes_before_run {
                            ui.label(format!(
                                "Run will add {} probe(s): {voltage} voltage, {current} current.",
                                rows.len()
                            ));
                        } else {
                            ui.label(format!(
                                "Auto-before-Run is off. {} probe(s) are available to add manually.",
                                rows.len()
                            ));
                        }
                        egui::Grid::new("scope_run_readiness_probe_preview")
                            .num_columns(4)
                            .striped(true)
                            .show(ui, |ui| {
                                ui.strong("Kind");
                                ui.strong("Probe");
                                ui.strong("Expression");
                                ui.strong("Target");
                                ui.end_row();
                                for row in &rows {
                                    ui.label(row.quantity);
                                    ui.monospace(format!(
                                        "{} / {}",
                                        row.scenario_name, row.probe_name
                                    ));
                                    ui.monospace(&row.expression);
                                    ui.label(&row.target);
                                    ui.end_row();
                                }
                            });
                    }
                    Err(error) => {
                        ui.colored_label(
                            egui::Color32::from_rgb(180, 50, 50),
                            format!("Cannot preview Run probes: {error:#}"),
                        );
                    }
                }
            });
    }

    fn apply_auto_scope_probes(&mut self) {
        match auto_scope_probe_project_yaml(&self.project_yaml, Path::new(&self.project_path)) {
            Ok(edit) if edit.added_total() == 0 => {
                self.status = "Auto Scope Probes found no missing useful probes.".to_string();
                self.push_diagnostic("Auto Scope Probes found no missing useful probes.");
            }
            Ok(edit) => {
                let status = edit.status_message();
                let target = edit.first_target.clone();
                self.apply_edited_project_yaml(edit.updated_yaml, &status);
                if let Some(target) = target {
                    self.remember_scope_probe_target(&target.scenario_name, &target.probe_name);
                }
            }
            Err(error) => self.record_error(error),
        }
    }
}

fn auto_scope_probe_project_yaml(
    project_yaml: &str,
    project_path: &Path,
) -> Result<AutoScopeProbeEdit> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(project_yaml).context("Project YAML is not valid Board IR.")?;
    let candidates = auto_scope_probe_candidates(&project, project_path)?;
    let mut updated_yaml = project_yaml.to_string();
    let mut added_voltage = 0;
    let mut added_current = 0;
    let mut first_target = None;

    for candidate in candidates {
        updated_yaml = match &candidate.kind {
            AutoScopeProbeKind::Voltage { net_id } => {
                added_voltage += 1;
                append_analog_voltage_probe(
                    &updated_yaml,
                    &AnalogProbeDraft {
                        scenario_name: candidate.scenario_name.clone(),
                        net_id: net_id.clone(),
                        probe_name: candidate.probe_name.clone(),
                    },
                )?
            }
            AutoScopeProbeKind::Current {
                component_id,
                generated,
            } => {
                added_current += 1;
                if *generated {
                    append_analog_current_probe(
                        &updated_yaml,
                        project_path,
                        &AnalogCurrentProbeDraft {
                            scenario_name: candidate.scenario_name.clone(),
                            component_id: component_id.clone(),
                            probe_name: candidate.probe_name.clone(),
                        },
                    )?
                } else {
                    append_analog_expression_probe(
                        &updated_yaml,
                        &AnalogExpressionProbeDraft {
                            scenario_name: candidate.scenario_name.clone(),
                            probe_name: candidate.probe_name.clone(),
                            expression: candidate.expression.clone(),
                            quantity: "current".to_string(),
                        },
                    )?
                }
            }
        };
        first_target.get_or_insert(ScopeProbeTarget {
            scenario_name: candidate.scenario_name,
            probe_name: candidate.probe_name,
        });
    }

    Ok(AutoScopeProbeEdit {
        updated_yaml,
        added_voltage,
        added_current,
        first_target,
    })
}

fn auto_scope_probe_candidates(
    project: &crate::board_ir::BoardProject,
    project_path: &Path,
) -> Result<Vec<AutoScopeProbeCandidate>> {
    let mut candidates = Vec::new();
    for scenario in &project.scenarios {
        let Some(analog) = scenario.analog.as_ref() else {
            continue;
        };
        let mut planned_names = analog
            .probes
            .iter()
            .map(|probe| probe.name.clone())
            .collect::<BTreeSet<_>>();
        let mut planned_expressions = analog
            .probes
            .iter()
            .map(|probe| normalized_probe_expression(&probe.expression, &probe.quantity))
            .collect::<BTreeSet<_>>();

        let mut voltage_count = 0;
        for binding in &analog.node_bindings {
            if voltage_count >= MAX_AUTO_VOLTAGE_PROBES {
                break;
            }
            let Some(net) = project.board.nets.get(&binding.net) else {
                continue;
            };
            if net.kind == crate::board_ir::NetKind::Ground {
                continue;
            }
            let expression = format!("V({})", binding.node);
            let key = normalized_expression(&expression, "voltage");
            if planned_expressions.contains(&key) {
                continue;
            }
            let probe_name = unique_auto_probe_name("v", &binding.net, &mut planned_names);
            planned_expressions.insert(key);
            voltage_count += 1;
            candidates.push(AutoScopeProbeCandidate {
                scenario_name: scenario.name.clone(),
                probe_name,
                expression,
                kind: AutoScopeProbeKind::Voltage {
                    net_id: binding.net.clone(),
                },
            });
        }

        let mut current_count = 0;
        for component_id in auto_current_probe_components(project, analog) {
            if current_count >= MAX_AUTO_CURRENT_PROBES {
                break;
            }
            let expression = current_probe_expression(project, project_path, &component_id)?;
            let key = normalized_expression(&expression, "current");
            if planned_expressions.contains(&key) {
                continue;
            }
            let probe_name = unique_auto_probe_name("i", &component_id, &mut planned_names);
            planned_expressions.insert(key);
            current_count += 1;
            candidates.push(AutoScopeProbeCandidate {
                scenario_name: scenario.name.clone(),
                probe_name,
                expression,
                kind: AutoScopeProbeKind::Current {
                    component_id,
                    generated: analog.netlist_source
                        == crate::board_ir::AnalogNetlistSource::GeneratedFromBoard,
                },
            });
        }
    }
    Ok(candidates)
}

fn auto_scope_probe_preview_rows(
    project_yaml: &str,
    project_path: &Path,
) -> Result<Vec<AutoScopeProbePreviewRow>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(project_yaml).context("Project YAML is not valid Board IR.")?;
    Ok(auto_scope_probe_candidates(&project, project_path)?
        .into_iter()
        .map(|candidate| {
            let (quantity, target) = match &candidate.kind {
                AutoScopeProbeKind::Voltage { net_id } => ("voltage", format!("net {net_id}")),
                AutoScopeProbeKind::Current { component_id, .. } => {
                    ("current", format!("source {component_id}"))
                }
            };
            AutoScopeProbePreviewRow {
                scenario_name: candidate.scenario_name,
                probe_name: candidate.probe_name,
                expression: candidate.expression,
                quantity,
                target,
            }
        })
        .collect())
}

fn auto_current_probe_components(
    project: &crate::board_ir::BoardProject,
    analog: &crate::board_ir::AnalogScenario,
) -> Vec<String> {
    let candidates = if let Some(generated) = analog.generated.as_ref() {
        generated.components.to_vec()
    } else {
        analog
            .pin_bindings
            .iter()
            .map(|binding| binding.endpoint.component.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    };
    candidates
        .into_iter()
        .filter(|component_id| {
            project
                .board
                .components
                .get(component_id)
                .and_then(|component| component.spice.as_ref())
                .is_some_and(|spice| {
                    matches!(
                        spice.primitive,
                        crate::board_ir::SpicePrimitive::DcVoltageSource
                            | crate::board_ir::SpicePrimitive::PulseVoltageSource
                            | crate::board_ir::SpicePrimitive::DcCurrentSource
                            | crate::board_ir::SpicePrimitive::PulseCurrentSource
                    )
                })
        })
        .collect()
}

fn normalized_probe_expression(
    expression: &str,
    quantity: &crate::board_ir::AnalogQuantity,
) -> String {
    let quantity = match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "voltage",
        crate::board_ir::AnalogQuantity::Current => "current",
        crate::board_ir::AnalogQuantity::Power => "power",
    };
    normalized_expression(expression, quantity)
}

fn normalized_expression(expression: &str, quantity: &str) -> String {
    format!(
        "{}:{}",
        quantity,
        expression
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase()
    )
}

fn unique_auto_probe_name(prefix: &str, target: &str, existing: &mut BTreeSet<String>) -> String {
    let suffix = sanitize_probe_id(target);
    let base = if suffix.is_empty() {
        format!("{prefix}_probe")
    } else {
        format!("{prefix}_{suffix}")
    };
    if existing.insert(base.clone()) {
        return base;
    }
    for index in 2.. {
        let candidate = format!("{base}_{index}");
        if existing.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search should always find a unique probe name")
}

fn sanitize_probe_id(value: &str) -> String {
    let mut sanitized = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character);
        } else if !sanitized.ends_with('_') {
            sanitized.push('_');
        }
    }
    sanitized.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        auto_scope_probe_candidates, auto_scope_probe_preview_rows, auto_scope_probe_project_yaml,
    };
    use crate::gui::analog::analog_scenario_choices;
    use crate::gui::{CircuitCiApp, ScopeProbeTarget};
    use std::path::Path;

    const AUTO_SCOPE_PROJECT: &str = "project:
  name: auto_scope_probe_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.voltage_source
      spice: {primitive: dc_voltage_source, dc_v: 5.0}
      pins: {P: rail_5v, N: gnd}
    VOUT:
      model: generic.analog.voltage_source
      spice:
        primitive: pulse_voltage_source
        pulse: {initial_v: 0.0, pulsed_v: 5.0, delay_us: 0.0, rise_us: 1.0, fall_us: 1.0, width_us: 326.0, period_us: 686.0}
      pins: {P: out, N: gnd}
    RTIM:
      model: generic.analog.resistor
      spice: {primitive: resistor, value_ohm: 10000.0}
      pins: {A: out, B: timing}
    CTIM:
      model: generic.analog.capacitor
      spice: {primitive: capacitor, value_f: 0.00000001}
      pins: {A: timing, B: gnd}
  nets:
    gnd: {kind: ground}
    out: {kind: digital_or_analog}
    rail_5v: {kind: power, nominal_voltage: 5.0, powered: true}
    timing: {kind: digital_or_analog}
scenarios:
  - name: astable
    type: analog
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated: {components: [V1, VOUT, RTIM, CTIM], ground_net: gnd}
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: out, net: out}
        - {node: rail_5v, net: rail_5v}
        - {node: timing, net: timing}
      pin_bindings:
        - {node: rail_5v, endpoint: {component: V1, pin: P}}
        - {node: '0', endpoint: {component: V1, pin: N}}
        - {node: out, endpoint: {component: VOUT, pin: P}}
        - {node: '0', endpoint: {component: VOUT, pin: N}}
      analysis: {type: tran, stop_time_us: 5000.0, max_step_us: 2.0}
      stimuli: []
      probes: []
      assertions: []
";

    #[test]
    fn auto_scope_candidates_cover_analog_nodes_and_sources() {
        let project = serde_yaml_ng::from_str(AUTO_SCOPE_PROJECT).unwrap();
        let candidates = auto_scope_probe_candidates(&project, Path::new("project.yaml")).unwrap();
        let names = candidates
            .iter()
            .map(|candidate| candidate.probe_name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, ["v_out", "v_rail_5v", "v_timing", "i_V1", "i_VOUT"]);
    }

    #[test]
    fn auto_scope_project_yaml_adds_missing_probes_once() {
        let edit =
            auto_scope_probe_project_yaml(AUTO_SCOPE_PROJECT, Path::new("project.yaml")).unwrap();
        assert_eq!(edit.added_voltage, 3);
        assert_eq!(edit.added_current, 2);
        assert_eq!(edit.first_target.as_ref().unwrap().probe_name, "v_out");
        let choices = analog_scenario_choices(&edit.updated_yaml).unwrap();
        assert_eq!(choices[0].probes.len(), 5);

        let second =
            auto_scope_probe_project_yaml(&edit.updated_yaml, Path::new("project.yaml")).unwrap();
        assert_eq!(second.added_total(), 0);
    }

    #[test]
    fn auto_scope_preview_rows_show_run_probe_setup() {
        let rows =
            auto_scope_probe_preview_rows(AUTO_SCOPE_PROJECT, Path::new("project.yaml")).unwrap();

        let summary = rows
            .iter()
            .map(|row| {
                format!(
                    "{}:{}:{}:{}",
                    row.quantity, row.probe_name, row.expression, row.target
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            summary,
            [
                "voltage:v_out:V(out):net out",
                "voltage:v_rail_5v:V(rail_5v):net rail_5v",
                "voltage:v_timing:V(timing):net timing",
                "current:i_V1:I(V1):source V1",
                "current:i_VOUT:I(VOUT):source VOUT",
            ]
        );
    }

    #[test]
    fn auto_scope_before_run_edits_project_and_remembers_first_probe() {
        let mut app = CircuitCiApp {
            project_yaml: AUTO_SCOPE_PROJECT.to_string(),
            project_path: "project.yaml".to_string(),
            ..CircuitCiApp::default()
        };

        assert!(app.prepare_auto_scope_probes_for_run().unwrap());

        assert!(app.project_yaml_dirty);
        assert_eq!(
            app.pending_scope_probe,
            Some(ScopeProbeTarget {
                scenario_name: "astable".to_string(),
                probe_name: "v_out".to_string(),
            })
        );
        let choices = analog_scenario_choices(&app.project_yaml).unwrap();
        assert_eq!(choices[0].probes.len(), 5);
    }

    #[test]
    fn auto_scope_before_run_can_be_disabled() {
        let mut app = CircuitCiApp {
            project_yaml: AUTO_SCOPE_PROJECT.to_string(),
            project_path: "project.yaml".to_string(),
            scope_auto_probes_before_run: false,
            ..CircuitCiApp::default()
        };

        assert!(!app.prepare_auto_scope_probes_for_run().unwrap());

        assert!(!app.project_yaml_dirty);
        assert_eq!(app.pending_scope_probe, None);
        assert_eq!(app.project_yaml, AUTO_SCOPE_PROJECT);
    }
}
