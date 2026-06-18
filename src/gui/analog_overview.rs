use anyhow::{Context, Result};
use eframe::egui;
use std::collections::BTreeSet;

use super::CircuitCiApp;

#[derive(Debug, Clone)]
pub(super) struct AnalogGeneratedOverview {
    pub(super) name: String,
    pub(super) backend: String,
    pub(super) ground_net: String,
    pub(super) stop_time_us: f64,
    pub(super) max_step_us: f64,
    pub(super) component_count: usize,
    pub(super) diagnostic_rows: Vec<AnalogOverviewRow>,
    pub(super) source_rows: Vec<AnalogOverviewRow>,
    pub(super) probe_rows: Vec<AnalogOverviewRow>,
    pub(super) assertion_rows: Vec<AnalogOverviewRow>,
    pub(super) model_file_rows: Vec<AnalogOverviewRow>,
    pub(super) binding_rows: Vec<AnalogOverviewRow>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogOverviewRow {
    pub(super) name: String,
    pub(super) detail: String,
    action: Option<AnalogOverviewAction>,
}

#[derive(Debug, Clone)]
enum AnalogOverviewAction {
    GeneratedComponent { component_id: String },
    GeneratedBinding { net: String, node: String },
    ModelFile { path: String },
    AssertionProbe { probe_name: Option<String> },
    ProbeAuthoring,
}

impl AnalogOverviewAction {
    fn label(&self) -> &'static str {
        match self {
            Self::GeneratedComponent { .. } => "Open component",
            Self::GeneratedBinding { .. } => "Open binding",
            Self::ModelFile { .. } => "Open model",
            Self::AssertionProbe { .. } => "Open assertion",
            Self::ProbeAuthoring => "Open probe",
        }
    }
}

pub(super) fn generated_analog_overviews(text: &str) -> Result<Vec<AnalogGeneratedOverview>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    Ok(project
        .scenarios
        .iter()
        .filter_map(|scenario| {
            let analog = scenario.analog.as_ref()?;
            if analog.netlist_source != crate::board_ir::AnalogNetlistSource::GeneratedFromBoard {
                return None;
            }
            let generated = analog.generated.as_ref()?;
            let source_rows: Vec<AnalogOverviewRow> = generated
                .components
                .iter()
                .filter_map(|component_id| {
                    let component = project.board.components.get(component_id)?;
                    let spice = component.spice.as_ref()?;
                    Some(AnalogOverviewRow {
                        name: component_id.clone(),
                        detail: spice_summary(spice),
                        action: None,
                    })
                })
                .collect();
            let diagnostic_rows = readiness_diagnostics(&project, analog, generated, &source_rows);
            Some(AnalogGeneratedOverview {
                name: scenario.name.clone(),
                backend: analog_backend_label(&analog.backend).to_string(),
                ground_net: generated.ground_net.clone(),
                stop_time_us: analog.analysis.stop_time_us,
                max_step_us: analog.analysis.max_step_us,
                component_count: generated.components.len(),
                diagnostic_rows,
                source_rows,
                probe_rows: analog
                    .probes
                    .iter()
                    .map(|probe| AnalogOverviewRow {
                        name: probe.name.clone(),
                        detail: format!(
                            "{} {}",
                            analog_quantity_label(&probe.quantity),
                            probe.expression
                        ),
                        action: None,
                    })
                    .collect(),
                assertion_rows: analog
                    .assertions
                    .iter()
                    .map(|assertion| AnalogOverviewRow {
                        name: assertion.name.clone(),
                        detail: format!("{} {}", assertion.probe, assertion_summary(assertion)),
                        action: None,
                    })
                    .collect(),
                model_file_rows: analog
                    .model_files
                    .iter()
                    .map(|model_file| AnalogOverviewRow {
                        name: model_file.path.clone(),
                        detail: model_file
                            .sha256
                            .as_ref()
                            .map(|sha| format!("sha256 {}", short_sha(sha)))
                            .unwrap_or_else(|| "missing sha256".to_string()),
                        action: None,
                    })
                    .collect(),
                binding_rows: analog
                    .node_bindings
                    .iter()
                    .map(|binding| AnalogOverviewRow {
                        name: binding.net.clone(),
                        detail: format!("SPICE node {}", binding.node),
                        action: None,
                    })
                    .collect(),
            })
        })
        .collect())
}

impl CircuitCiApp {
    pub(super) fn analog_generated_overview_panel(&mut self, ui: &mut egui::Ui) {
        let overviews = match generated_analog_overviews(&self.project_yaml) {
            Ok(overviews) => overviews,
            Err(error) => {
                ui.collapsing("Generated Analog Overview", |ui| {
                    ui.label(format!("Generated analog overview unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Generated Analog Overview", |ui| {
            if overviews.is_empty() {
                ui.label("No generated_from_board analog scenario is available.");
                return;
            }
            initialize_generated_overview_default(&overviews, &mut self.analog_generated_scenario);
            let previous_scenario = self.analog_generated_scenario.clone();
            generated_overview_combo(ui, &mut self.analog_generated_scenario, &overviews);
            if self.analog_generated_scenario != previous_scenario {
                self.analog_generated_component.clear();
                self.analog_generated_node_net.clear();
                self.analog_generated_node_name.clear();
            }
            let selected = selected_generated_overview(&overviews, &self.analog_generated_scenario)
                .or_else(|| overviews.first());
            let Some(selected) = selected else {
                return;
            };
            egui::Grid::new("analog_generated_overview_summary")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Backend");
                    ui.monospace(&selected.backend);
                    ui.end_row();

                    ui.label("Ground");
                    ui.monospace(&selected.ground_net);
                    ui.end_row();

                    ui.label("Transient");
                    ui.label(format!(
                        "stop {} us, max step {} us",
                        compact_number(selected.stop_time_us),
                        compact_number(selected.max_step_us)
                    ));
                    ui.end_row();

                    ui.label("Components");
                    ui.label(selected.component_count.to_string());
                    ui.end_row();
                });
            ui.add_space(6.0);
            self.analog_readiness_rows(ui, selected);
            analog_overview_rows(ui, "Sources", &selected.source_rows);
            analog_overview_rows(ui, "Probes", &selected.probe_rows);
            analog_overview_rows(ui, "Assertions", &selected.assertion_rows);
            analog_overview_rows(ui, "Model files", &selected.model_file_rows);
            analog_overview_rows(ui, "Node bindings", &selected.binding_rows);
        });
    }

    fn analog_readiness_rows(&mut self, ui: &mut egui::Ui, overview: &AnalogGeneratedOverview) {
        ui.strong("Readiness");
        if overview.diagnostic_rows.is_empty() {
            ui.label("Ready");
            return;
        }
        egui::Grid::new("analog_generated_overview_Readiness")
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                for row in &overview.diagnostic_rows {
                    ui.monospace(&row.name);
                    ui.label(&row.detail);
                    if let Some(action) = &row.action {
                        if ui.button(action.label()).clicked() {
                            self.apply_overview_action(&overview.name, action);
                        }
                    } else {
                        ui.label("");
                    }
                    ui.end_row();
                }
            });
    }

    fn apply_overview_action(&mut self, scenario_name: &str, action: &AnalogOverviewAction) {
        self.analog_generated_scenario = scenario_name.to_string();
        match action {
            AnalogOverviewAction::GeneratedComponent { component_id } => {
                self.analog_generated_component = component_id.clone();
                self.analog_stimulus_scenario = scenario_name.to_string();
                self.analog_stimulus_component = component_id.clone();
                self.status =
                    format!("Selected component {component_id} for generated scenario editing.");
            }
            AnalogOverviewAction::GeneratedBinding { net, node } => {
                self.analog_generated_node_net = net.clone();
                self.analog_generated_node_name = node.clone();
                self.status =
                    format!("Selected node binding {net} for generated scenario editing.");
            }
            AnalogOverviewAction::ModelFile { path } => {
                self.analog_model_scenario = scenario_name.to_string();
                self.analog_model_path = path.clone();
                self.analog_model_sha256.clear();
                self.status =
                    format!("Selected SPICE model file {path} for hashing or replacement.");
            }
            AnalogOverviewAction::AssertionProbe { probe_name } => {
                self.analog_assertion_scenario = scenario_name.to_string();
                self.analog_assertion_probe = probe_name.clone().unwrap_or_default();
                self.analog_assertion_edit_original.clear();
                if self.analog_assertion_name.trim().is_empty() {
                    self.analog_assertion_name = "probe_check".to_string();
                }
                self.status = "Selected assertion editor context for this scenario.".to_string();
            }
            AnalogOverviewAction::ProbeAuthoring => {
                self.analog_probe_scenario = scenario_name.to_string();
                self.status =
                    "Selected probe authoring context; add voltage/current/power probes from the schematic canvas."
                        .to_string();
            }
        }
        let status = self.status.clone();
        self.push_diagnostic(&status);
    }
}

fn initialize_generated_overview_default(
    overviews: &[AnalogGeneratedOverview],
    scenario_name: &mut String,
) {
    let missing = selected_generated_overview(overviews, scenario_name).is_none();
    if missing && let Some(first) = overviews.first() {
        *scenario_name = first.name.clone();
    }
}

fn selected_generated_overview<'a>(
    overviews: &'a [AnalogGeneratedOverview],
    scenario_name: &str,
) -> Option<&'a AnalogGeneratedOverview> {
    overviews
        .iter()
        .find(|overview| overview.name == scenario_name)
}

fn generated_overview_combo(
    ui: &mut egui::Ui,
    selected: &mut String,
    overviews: &[AnalogGeneratedOverview],
) {
    egui::ComboBox::from_id_salt("analog_generated_overview_scenario")
        .selected_text(if selected.is_empty() {
            "select scenario".to_string()
        } else {
            selected.clone()
        })
        .show_ui(ui, |ui| {
            for overview in overviews {
                ui.selectable_value(selected, overview.name.clone(), &overview.name);
            }
        });
}

fn analog_overview_rows(ui: &mut egui::Ui, label: &str, rows: &[AnalogOverviewRow]) {
    ui.strong(label);
    if rows.is_empty() {
        if label == "Readiness" {
            ui.label("Ready");
        } else {
            ui.label("None");
        }
        return;
    }
    egui::Grid::new(format!("analog_generated_overview_{label}"))
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for row in rows {
                ui.monospace(&row.name);
                ui.label(&row.detail);
                ui.end_row();
            }
        });
}

fn readiness_diagnostics(
    project: &crate::board_ir::BoardProject,
    analog: &crate::board_ir::AnalogScenario,
    generated: &crate::board_ir::AnalogGeneratedNetlist,
    source_rows: &[AnalogOverviewRow],
) -> Vec<AnalogOverviewRow> {
    let mut diagnostics = Vec::new();
    if generated.components.is_empty() {
        diagnostics.push(diagnostic(
            "Missing components",
            "No generated components are included.",
            None,
        ));
    }

    let existing_components: BTreeSet<&str> = project
        .board
        .components
        .keys()
        .map(String::as_str)
        .collect();
    let missing_components = joined_ids(
        generated
            .components
            .iter()
            .filter(|component_id| !existing_components.contains(component_id.as_str())),
    );
    if let Some(ids) = missing_components {
        diagnostics.push(diagnostic(
            "Missing component evidence",
            format!("Included components are not in board.components: {ids}."),
            None,
        ));
    }

    let source_components: BTreeSet<&str> =
        source_rows.iter().map(|row| row.name.as_str()).collect();
    if generated.components.is_empty() {
        // The missing-components diagnostic is more specific.
    } else if source_rows.is_empty() {
        diagnostics.push(diagnostic(
            "Missing source primitives",
            "No included component exposes a Board IR spice primitive.",
            generated.components.first().map(|component_id| {
                AnalogOverviewAction::GeneratedComponent {
                    component_id: component_id.clone(),
                }
            }),
        ));
    } else if let Some(ids) = joined_ids(
        generated
            .components
            .iter()
            .filter(|component_id| !source_components.contains(component_id.as_str())),
    ) {
        diagnostics.push(diagnostic(
            "Partial source coverage",
            format!("Included components without Board IR spice primitives: {ids}."),
            first_component_without_source(generated, &source_components).map(|component_id| {
                AnalogOverviewAction::GeneratedComponent {
                    component_id: component_id.clone(),
                }
            }),
        ));
    }

    if analog.probes.is_empty() {
        diagnostics.push(diagnostic(
            "Missing probes",
            "No waveform probes are declared.",
            Some(AnalogOverviewAction::ProbeAuthoring),
        ));
    }
    if analog.assertions.is_empty() {
        diagnostics.push(diagnostic(
            "Missing assertions",
            "No pass/fail waveform assertions are declared.",
            Some(AnalogOverviewAction::AssertionProbe {
                probe_name: analog.probes.first().map(|probe| probe.name.clone()),
            }),
        ));
    }
    for model_file in &analog.model_files {
        if model_file
            .sha256
            .as_deref()
            .is_none_or(|sha| sha.trim().is_empty())
        {
            diagnostics.push(diagnostic(
                "Missing model SHA",
                format!("{} has no SHA-256 evidence.", model_file.path),
                Some(AnalogOverviewAction::ModelFile {
                    path: model_file.path.clone(),
                }),
            ));
        }
    }

    let ground_binding = analog
        .node_bindings
        .iter()
        .find(|binding| binding.net == generated.ground_net);
    match ground_binding {
        Some(binding) if binding.node == "0" => {}
        Some(binding) => diagnostics.push(diagnostic(
            "Ground is not node 0",
            format!(
                "{} maps to SPICE node {}; generated ground should map to 0.",
                generated.ground_net, binding.node
            ),
            Some(AnalogOverviewAction::GeneratedBinding {
                net: generated.ground_net.clone(),
                node: "0".to_string(),
            }),
        )),
        None => diagnostics.push(diagnostic(
            "Missing ground binding",
            format!("{} has no SPICE node binding.", generated.ground_net),
            Some(AnalogOverviewAction::GeneratedBinding {
                net: generated.ground_net.clone(),
                node: "0".to_string(),
            }),
        )),
    }

    let bound_nets: BTreeSet<&str> = analog
        .node_bindings
        .iter()
        .map(|binding| binding.net.as_str())
        .collect();
    let referenced_nets = included_component_nets(project, &generated.components);
    if let Some(ids) = joined_ids(
        referenced_nets
            .iter()
            .filter(|net| !bound_nets.contains(net.as_str())),
    ) {
        diagnostics.push(diagnostic(
            "Missing node bindings",
            format!("Included component nets without SPICE nodes: {ids}."),
            first_missing_node_binding(&referenced_nets, &bound_nets).map(|net| {
                AnalogOverviewAction::GeneratedBinding {
                    node: if net == generated.ground_net {
                        "0".to_string()
                    } else {
                        net.clone()
                    },
                    net,
                }
            }),
        ));
    }

    let bound_pins: BTreeSet<(String, String)> = analog
        .pin_bindings
        .iter()
        .map(|binding| {
            (
                binding.endpoint.component.clone(),
                binding.endpoint.pin.clone(),
            )
        })
        .collect();
    let mut missing_pin_binding_labels = Vec::new();
    for component_id in &generated.components {
        let Some(component) = project.board.components.get(component_id) else {
            continue;
        };
        for pin in component.pins.keys() {
            if !bound_pins.contains(&(component_id.clone(), pin.clone())) {
                missing_pin_binding_labels.push(format!("{}.{}", component_id, pin));
            }
        }
    }
    let missing_pin_bindings = joined_ids(&missing_pin_binding_labels);
    if let Some(ids) = missing_pin_bindings {
        diagnostics.push(diagnostic(
            "Missing pin bindings",
            format!("Included component pins without SPICE node endpoints: {ids}."),
            missing_pin_binding_labels
                .first()
                .and_then(|label| label.split_once('.'))
                .map(
                    |(component_id, _pin)| AnalogOverviewAction::GeneratedComponent {
                        component_id: component_id.to_string(),
                    },
                ),
        ));
    }
    diagnostics
}

fn included_component_nets(
    project: &crate::board_ir::BoardProject,
    components: &[String],
) -> BTreeSet<String> {
    components
        .iter()
        .filter_map(|component_id| project.board.components.get(component_id))
        .flat_map(|component| component.pins.values().cloned())
        .collect()
}

fn first_component_without_source<'a>(
    generated: &'a crate::board_ir::AnalogGeneratedNetlist,
    source_components: &BTreeSet<&str>,
) -> Option<&'a String> {
    generated
        .components
        .iter()
        .find(|component_id| !source_components.contains(component_id.as_str()))
}

fn first_missing_node_binding(
    referenced_nets: &BTreeSet<String>,
    bound_nets: &BTreeSet<&str>,
) -> Option<String> {
    referenced_nets
        .iter()
        .find(|net| !bound_nets.contains(net.as_str()))
        .cloned()
}

fn diagnostic(
    name: impl Into<String>,
    detail: impl Into<String>,
    action: Option<AnalogOverviewAction>,
) -> AnalogOverviewRow {
    AnalogOverviewRow {
        name: name.into(),
        detail: detail.into(),
        action,
    }
}

fn joined_ids<I, S>(ids: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let values: Vec<String> = ids
        .into_iter()
        .map(|value| value.as_ref().to_string())
        .collect();
    if values.is_empty() {
        None
    } else {
        Some(values.join(", "))
    }
}

fn spice_summary(spice: &crate::board_ir::ComponentSpiceSpec) -> String {
    match spice.primitive {
        crate::board_ir::SpicePrimitive::DcVoltageSource => {
            format!("DC voltage source {}", value_label(spice.dc_v, "V"))
        }
        crate::board_ir::SpicePrimitive::PulseVoltageSource => spice
            .pulse
            .as_ref()
            .map(|pulse| {
                format!(
                    "pulse voltage source {} V to {} V, period {} us",
                    compact_number(pulse.initial_v),
                    compact_number(pulse.pulsed_v),
                    compact_number(pulse.period_us)
                )
            })
            .unwrap_or_else(|| "pulse voltage source".to_string()),
        crate::board_ir::SpicePrimitive::DcCurrentSource => {
            format!("DC current source {}", value_label(spice.dc_a, "A"))
        }
        crate::board_ir::SpicePrimitive::PulseCurrentSource => spice
            .current_pulse
            .as_ref()
            .map(|pulse| {
                format!(
                    "pulse current source {} A to {} A, period {} us",
                    compact_number(pulse.initial_a),
                    compact_number(pulse.pulsed_a),
                    compact_number(pulse.period_us)
                )
            })
            .unwrap_or_else(|| "pulse current source".to_string()),
        crate::board_ir::SpicePrimitive::Resistor => {
            format!("resistor {}", value_label(spice.value_ohm, "ohm"))
        }
        crate::board_ir::SpicePrimitive::Capacitor => {
            format!("capacitor {}", value_label(spice.value_f, "F"))
        }
        crate::board_ir::SpicePrimitive::Inductor => {
            format!("inductor {}", value_label(spice.value_h, "H"))
        }
    }
}

fn value_label(value: Option<f64>, unit: &str) -> String {
    value
        .map(|value| format!("{} {unit}", compact_number(value)))
        .unwrap_or_else(|| format!("missing {unit} value"))
}

fn assertion_summary(assertion: &crate::board_ir::AnalogAssertion) -> String {
    format!(
        "{} {} {} {}",
        aggregation_label(&assertion.aggregation),
        relation_label(&assertion.relation),
        assertion_threshold_label(assertion),
        assertion_timing_label(assertion)
    )
}

fn assertion_threshold_label(assertion: &crate::board_ir::AnalogAssertion) -> String {
    if let Some(value) = assertion.threshold_v {
        format!("{} V", compact_number(value))
    } else if let Some(value) = assertion.threshold_a {
        format!("{} A", compact_number(value))
    } else if let Some(value) = assertion.threshold_w {
        format!("{} W", compact_number(value))
    } else {
        "missing threshold".to_string()
    }
}

fn assertion_timing_label(assertion: &crate::board_ir::AnalogAssertion) -> String {
    if let Some(at_us) = assertion.at_us {
        format!("at {} us", compact_number(at_us))
    } else if let (Some(start_us), Some(end_us)) = (assertion.start_us, assertion.end_us) {
        format!(
            "from {} us to {} us",
            compact_number(start_us),
            compact_number(end_us)
        )
    } else {
        "over full waveform".to_string()
    }
}

fn aggregation_label(aggregation: &crate::board_ir::AnalogAggregation) -> &'static str {
    match aggregation {
        crate::board_ir::AnalogAggregation::Sample => "sample",
        crate::board_ir::AnalogAggregation::Min => "min",
        crate::board_ir::AnalogAggregation::Max => "max",
    }
}

fn relation_label(relation: &crate::board_ir::AnalogRelation) -> &'static str {
    match relation {
        crate::board_ir::AnalogRelation::Above => "above",
        crate::board_ir::AnalogRelation::Below => "below",
    }
}

fn analog_quantity_label(quantity: &crate::board_ir::AnalogQuantity) -> &'static str {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "voltage",
        crate::board_ir::AnalogQuantity::Current => "current",
        crate::board_ir::AnalogQuantity::Power => "power",
    }
}

fn analog_backend_label(backend: &crate::board_ir::AnalogBackend) -> &'static str {
    match backend {
        crate::board_ir::AnalogBackend::Auto => "auto",
        crate::board_ir::AnalogBackend::Ngspice => "ngspice",
        crate::board_ir::AnalogBackend::Xyce => "xyce",
        crate::board_ir::AnalogBackend::EmbeddedNgspice => "embedded-ngspice",
    }
}

fn short_sha(sha: &str) -> String {
    sha.chars().take(12).collect()
}

fn compact_number(value: f64) -> String {
    let formatted = format!("{value:.6}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{AnalogOverviewAction, generated_analog_overviews};

    fn project_yaml() -> &'static str {
        "project:
  name: analog_overview_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      spice:
        primitive: dc_voltage_source
        dc_v: 5.0
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
  nets:
    rail_5v:
      kind: power
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
scenarios:
  - name: gui_transient
    type: analog_transient
    analog:
      backend: ngspice
      netlist_source: generated_from_board
      generated:
        components: [V1, R1]
        ground_net: gnd
      model_files:
        - path: models/opamp.lib
          sha256: 0123456789abcdef
      node_bindings:
        - {net: gnd, node: '0'}
        - {net: rail_5v, node: rail_5v}
        - {net: out, node: out}
      pin_bindings:
        - node: rail_5v
          endpoint: {component: V1, pin: P}
        - node: '0'
          endpoint: {component: V1, pin: N}
        - node: rail_5v
          endpoint: {component: R1, pin: A}
        - node: out
          endpoint: {component: R1, pin: B}
      analysis:
        type: transient
        stop_time_us: 100
        max_step_us: 1
      stimuli: []
      probes:
        - name: out_voltage
          expression: V(out)
          quantity: voltage
      assertions:
        - name: out_above_min
          probe: out_voltage
          aggregation: sample
          relation: above
          threshold_v: 1.0
          at_us: 50
"
    }

    #[test]
    fn generated_analog_overviews_summarize_signoff_inputs() {
        let overviews = generated_analog_overviews(project_yaml()).unwrap();
        assert_eq!(overviews.len(), 1);
        let overview = &overviews[0];
        assert_eq!(overview.name, "gui_transient");
        assert_eq!(overview.backend, "ngspice");
        assert_eq!(overview.ground_net, "gnd");
        assert_eq!(overview.component_count, 2);
        assert!(overview.diagnostic_rows.is_empty());
        assert_eq!(overview.source_rows.len(), 2);
        assert!(
            overview
                .source_rows
                .iter()
                .any(|row| row.name == "V1" && row.detail.contains("5"))
        );
        assert_eq!(overview.probe_rows[0].detail, "voltage V(out)");
        assert!(
            overview.assertion_rows[0]
                .detail
                .contains("sample above 1 V")
        );
        assert_eq!(overview.model_file_rows[0].detail, "sha256 0123456789ab");
        assert_eq!(overview.binding_rows.len(), 3);
    }

    #[test]
    fn generated_analog_overviews_report_readiness_gaps() {
        let degraded = project_yaml()
            .replace("      probes:\n        - name: out_voltage\n          expression: V(out)\n          quantity: voltage\n", "      probes: []\n")
            .replace("      assertions:\n        - name: out_above_min\n          probe: out_voltage\n          aggregation: sample\n          relation: above\n          threshold_v: 1.0\n          at_us: 50\n", "      assertions: []\n")
            .replace("          sha256: 0123456789abcdef\n", "")
            .replace("        - node: out\n          endpoint: {component: R1, pin: B}\n", "");
        let overview = generated_analog_overviews(&degraded).unwrap().remove(0);
        let diagnostics: Vec<_> = overview
            .diagnostic_rows
            .iter()
            .map(|row| row.name.as_str())
            .collect();
        assert!(diagnostics.contains(&"Missing probes"));
        assert!(diagnostics.contains(&"Missing assertions"));
        assert!(diagnostics.contains(&"Missing model SHA"));
        assert!(diagnostics.contains(&"Missing pin bindings"));
        let missing_probes = overview
            .diagnostic_rows
            .iter()
            .find(|row| row.name == "Missing probes")
            .unwrap();
        assert!(matches!(
            &missing_probes.action,
            Some(AnalogOverviewAction::ProbeAuthoring)
        ));
        let missing_assertions = overview
            .diagnostic_rows
            .iter()
            .find(|row| row.name == "Missing assertions")
            .unwrap();
        assert!(matches!(
            &missing_assertions.action,
            Some(AnalogOverviewAction::AssertionProbe { probe_name }) if probe_name.is_none()
        ));
        let missing_model_sha = overview
            .diagnostic_rows
            .iter()
            .find(|row| row.name == "Missing model SHA")
            .unwrap();
        assert!(matches!(
            &missing_model_sha.action,
            Some(AnalogOverviewAction::ModelFile { path }) if path == "models/opamp.lib"
        ));
        let missing_pin_bindings = overview
            .diagnostic_rows
            .iter()
            .find(|row| row.name == "Missing pin bindings")
            .unwrap();
        assert!(matches!(
            &missing_pin_bindings.action,
            Some(AnalogOverviewAction::GeneratedComponent { component_id }) if component_id == "R1"
        ));
    }

    #[test]
    fn generated_analog_overview_actions_have_compact_labels() {
        assert_eq!(AnalogOverviewAction::ProbeAuthoring.label(), "Open probe");
        assert_eq!(
            AnalogOverviewAction::GeneratedBinding {
                net: "gnd".to_string(),
                node: "0".to_string(),
            }
            .label(),
            "Open binding"
        );
    }
}
