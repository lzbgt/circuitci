use super::CircuitCiApp;
use super::analog::{
    AnalogAssertionDraft, AnalogAssertionRemoveDraft, AnalogAssertionReplaceDraft,
    AnalogProbeAssertionsRemoveDraft, AnalogScenarioDraft, analog_probe_assertion_summaries,
    analog_scenario_choices, append_analog_assertion, append_analog_transient_scenario,
    remove_analog_assertion, remove_analog_assertions_for_probe, replace_analog_assertion,
    unique_analog_assertion_name,
};
use super::analog_generated::{
    AnalogGeneratedComponentDraft, AnalogGeneratedNodeBindingDraft, AnalogGeneratedSettingsDraft,
    analog_generated_scenarios, exclude_generated_component, include_generated_component,
    replace_generated_node_binding, replace_generated_settings,
};
use super::analog_models::{
    AnalogModelFileDraft, AnalogModelFileRemoveDraft, analog_model_file_scenarios,
    append_analog_model_file, model_file_sha256, remove_analog_model_file,
};
use super::analog_stimulus::{
    AnalogStimulusDraft, AnalogStimulusKind, AnalogStimulusPulseDraft, analog_stimulus_choices,
    replace_analog_stimulus,
};
use super::analog_sweeps::{
    AnalogSweepDraft, AnalogSweepParameterDraft, AnalogSweepScenario, AnalogSweepSummary,
    analog_sweep_scenarios, append_analog_sweep_parameter, append_analog_sweep_with_parameter,
    remove_analog_sweep, remove_analog_sweep_parameter,
};
use super::simulation_forms::*;
use super::sketch::ProjectSnapshot;
use super::sketch_probes::SketchProbe;
use super::waveform::{format_value, quick_assertion_margin, waveform_probe_value_for_badge};
use eframe::egui;
use std::path::Path;

impl CircuitCiApp {
    pub(super) fn analog_scenario_editor(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        ui.collapsing("Run Setup", |ui| {
            initialize_analog_net_defaults(
                snapshot,
                &mut self.analog_ground_net,
                &mut self.analog_probe_net,
            );
            egui::Grid::new("analog_transient_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Input set");
                    ui.text_edit_singleline(&mut self.analog_scenario_name);
                    ui.end_row();

                    ui.label("Ground net");
                    net_combo(
                        ui,
                        "analog_ground_net",
                        &mut self.analog_ground_net,
                        snapshot,
                    );
                    ui.end_row();

                    ui.label("Probe net");
                    net_combo(ui, "analog_probe_net", &mut self.analog_probe_net, snapshot);
                    ui.end_row();

                    ui.label("Probe name");
                    ui.text_edit_singleline(&mut self.analog_probe_name);
                    ui.end_row();

                    ui.label("Stop time");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_stop_time_us)
                            .speed(1.0)
                            .range(0.001..=1_000_000.0)
                            .suffix(" us"),
                    );
                    ui.end_row();

                    ui.label("Max step");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_max_step_us)
                            .speed(0.1)
                            .range(0.001..=1_000_000.0)
                            .suffix(" us"),
                    );
                    ui.end_row();
                });
            if ui.button("Add Run Setup").clicked() {
                self.apply_add_analog_scenario();
            }
        });
    }

    pub(super) fn analog_generated_components_editor(&mut self, ui: &mut egui::Ui) {
        let scenarios = match analog_generated_scenarios(&self.project_yaml) {
            Ok(scenarios) => scenarios,
            Err(error) => {
                ui.collapsing("Generated Components", |ui| {
                    ui.label(format!("Generated components unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Generated Components", |ui| {
            if scenarios.is_empty() {
                ui.label("No generated-from-board run setup is available.");
                return;
            }
            initialize_generated_component_defaults(
                &scenarios,
                &mut self.analog_generated_scenario,
                &mut self.analog_generated_component,
            );
            let selected_scenario =
                selected_generated_scenario(&scenarios, &self.analog_generated_scenario)
                    .or_else(|| scenarios.first());
            let Some(selected_scenario) = selected_scenario else {
                return;
            };
            egui::Grid::new("analog_generated_components_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    let previous_scenario = self.analog_generated_scenario.clone();
                    generated_scenario_combo(ui, &mut self.analog_generated_scenario, &scenarios);
                    if self.analog_generated_scenario != previous_scenario {
                        self.analog_generated_component.clear();
                        initialize_generated_component_defaults(
                            &scenarios,
                            &mut self.analog_generated_scenario,
                            &mut self.analog_generated_component,
                        );
                    }
                    ui.end_row();

                    ui.label("Component");
                    generated_component_combo(
                        ui,
                        selected_scenario,
                        &mut self.analog_generated_component,
                    );
                    ui.end_row();

                    ui.label("Included count");
                    ui.label(selected_scenario.components.len().to_string());
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                let selected_component = selected_scenario
                    .board_components
                    .iter()
                    .find(|component| component.id == self.analog_generated_component);
                let is_included = selected_component.is_some_and(|component| component.included);
                if ui
                    .add_enabled(!is_included, egui::Button::new("Include Component"))
                    .clicked()
                {
                    self.apply_include_generated_component();
                }
                if ui
                    .add_enabled(is_included, egui::Button::new("Exclude Component"))
                    .clicked()
                {
                    self.apply_exclude_generated_component();
                }
            });
            if selected_scenario.components.is_empty() {
                ui.label("No components are currently included.");
            } else {
                ui.label("Included components");
                egui::Grid::new("analog_generated_component_list")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Component");
                        ui.strong("Action");
                        ui.end_row();
                        for component_id in &selected_scenario.components {
                            ui.monospace(component_id);
                            if ui.button("Exclude").clicked() {
                                self.apply_exclude_generated_component_by_id(
                                    &selected_scenario.name,
                                    component_id,
                                );
                            }
                            ui.end_row();
                        }
                    });
            }
        });
    }

    pub(super) fn analog_generated_settings_editor(&mut self, ui: &mut egui::Ui) {
        let scenarios = match analog_generated_scenarios(&self.project_yaml) {
            Ok(scenarios) => scenarios,
            Err(error) => {
                ui.collapsing("Generated Run Settings", |ui| {
                    ui.label(format!("Generated settings unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Generated Run Settings", |ui| {
            if scenarios.is_empty() {
                ui.label("No generated-from-board run setup is available.");
                return;
            }
            initialize_generated_settings_defaults(
                &scenarios,
                &mut self.analog_generated_scenario,
                &mut self.analog_generated_ground_net,
                &mut self.analog_generated_stop_time_us,
                &mut self.analog_generated_max_step_us,
                &mut self.analog_generated_node_net,
                &mut self.analog_generated_node_name,
            );
            let selected_scenario =
                selected_generated_scenario(&scenarios, &self.analog_generated_scenario)
                    .or_else(|| scenarios.first());
            let Some(selected_scenario) = selected_scenario else {
                return;
            };
            egui::Grid::new("analog_generated_settings_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    let previous_scenario = self.analog_generated_scenario.clone();
                    generated_scenario_combo(ui, &mut self.analog_generated_scenario, &scenarios);
                    if self.analog_generated_scenario != previous_scenario {
                        load_generated_settings_values(
                            selected_generated_scenario(
                                &scenarios,
                                &self.analog_generated_scenario,
                            )
                            .or_else(|| scenarios.first())
                            .expect("scenarios is not empty"),
                            &mut self.analog_generated_ground_net,
                            &mut self.analog_generated_stop_time_us,
                            &mut self.analog_generated_max_step_us,
                            &mut self.analog_generated_node_net,
                            &mut self.analog_generated_node_name,
                        );
                    }
                    ui.end_row();

                    ui.label("Ground net");
                    generated_net_combo(
                        ui,
                        "analog_generated_ground_net",
                        selected_scenario,
                        &mut self.analog_generated_ground_net,
                    );
                    ui.end_row();

                    ui.label("Stop time");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_generated_stop_time_us)
                            .speed(1.0)
                            .range(0.001..=1_000_000.0)
                            .suffix(" us"),
                    );
                    ui.end_row();

                    ui.label("Max step");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_generated_max_step_us)
                            .speed(0.1)
                            .range(0.001..=1_000_000.0)
                            .suffix(" us"),
                    );
                    ui.end_row();

                    ui.label("Node net");
                    generated_net_combo(
                        ui,
                        "analog_generated_node_net",
                        selected_scenario,
                        &mut self.analog_generated_node_net,
                    );
                    ui.end_row();

                    ui.label("SPICE node");
                    ui.text_edit_singleline(&mut self.analog_generated_node_name);
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Save Settings").clicked() {
                    self.apply_replace_generated_settings();
                }
                if ui.button("Save Node Binding").clicked() {
                    self.apply_replace_generated_node_binding();
                }
                if ui.button("Reload Selected").clicked() {
                    load_generated_settings_values(
                        selected_scenario,
                        &mut self.analog_generated_ground_net,
                        &mut self.analog_generated_stop_time_us,
                        &mut self.analog_generated_max_step_us,
                        &mut self.analog_generated_node_net,
                        &mut self.analog_generated_node_name,
                    );
                }
            });
            ui.label("Node bindings");
            egui::Grid::new("analog_generated_node_bindings")
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Net");
                    ui.strong("Kind");
                    ui.strong("SPICE node");
                    ui.end_row();
                    for net in &selected_scenario.board_nets {
                        let node = selected_scenario
                            .node_bindings
                            .iter()
                            .find(|binding| binding.net == net.id)
                            .map(|binding| binding.node.as_str())
                            .unwrap_or("unbound");
                        if ui
                            .selectable_label(self.analog_generated_node_net == net.id, &net.id)
                            .clicked()
                        {
                            self.analog_generated_node_net = net.id.clone();
                            self.analog_generated_node_name = node.to_string();
                        }
                        ui.label(&net.kind);
                        ui.monospace(node);
                        ui.end_row();
                    }
                });
        });
    }

    pub(super) fn analog_stimulus_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_stimulus_choices(&self.project_yaml) {
            Ok(choices) => choices,
            Err(error) => {
                ui.collapsing("Source Stimulus", |ui| {
                    ui.label(format!("Source stimuli unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Source Stimulus", |ui| {
            if choices.is_empty() {
                ui.label("No generated run setup source primitives are available.");
                return;
            }
            initialize_analog_stimulus_defaults(
                &choices,
                &mut self.analog_stimulus_scenario,
                &mut self.analog_stimulus_component,
                &mut self.analog_stimulus_dc_value,
                &mut self.analog_stimulus_initial_value,
                &mut self.analog_stimulus_pulsed_value,
                &mut self.analog_stimulus_delay_us,
                &mut self.analog_stimulus_rise_us,
                &mut self.analog_stimulus_fall_us,
                &mut self.analog_stimulus_width_us,
                &mut self.analog_stimulus_period_us,
            );
            let selected = selected_analog_stimulus_choice(
                &choices,
                &self.analog_stimulus_scenario,
                &self.analog_stimulus_component,
            )
            .or_else(|| choices.first());
            let Some(selected) = selected else {
                return;
            };
            let selected_kind = selected.kind;
            egui::Grid::new("analog_stimulus_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Source");
                    let previous_scenario = self.analog_stimulus_scenario.clone();
                    let previous_component = self.analog_stimulus_component.clone();
                    analog_stimulus_combo(
                        ui,
                        &mut self.analog_stimulus_scenario,
                        &mut self.analog_stimulus_component,
                        &choices,
                    );
                    if self.analog_stimulus_scenario != previous_scenario
                        || self.analog_stimulus_component != previous_component
                    {
                        load_selected_analog_stimulus_values(
                            &choices,
                            &self.analog_stimulus_scenario,
                            &self.analog_stimulus_component,
                            &mut self.analog_stimulus_dc_value,
                            &mut self.analog_stimulus_initial_value,
                            &mut self.analog_stimulus_pulsed_value,
                            &mut self.analog_stimulus_delay_us,
                            &mut self.analog_stimulus_rise_us,
                            &mut self.analog_stimulus_fall_us,
                            &mut self.analog_stimulus_width_us,
                            &mut self.analog_stimulus_period_us,
                        );
                    }
                    ui.end_row();

                    ui.label("Primitive");
                    ui.label(selected_kind.label());
                    ui.end_row();

                    if selected_kind.is_pulse() {
                        ui.label("Initial");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_stimulus_initial_value)
                                .speed(stimulus_value_speed(selected_kind))
                                .suffix(selected_kind.value_unit()),
                        );
                        ui.end_row();

                        ui.label("Pulsed");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_stimulus_pulsed_value)
                                .speed(stimulus_value_speed(selected_kind))
                                .suffix(selected_kind.value_unit()),
                        );
                        ui.end_row();

                        for (label, value, min) in [
                            ("Delay", &mut self.analog_stimulus_delay_us, 0.0),
                            ("Rise", &mut self.analog_stimulus_rise_us, 0.0),
                            ("Fall", &mut self.analog_stimulus_fall_us, 0.0),
                            ("Width", &mut self.analog_stimulus_width_us, 0.001),
                            ("Period", &mut self.analog_stimulus_period_us, 0.001),
                        ] {
                            ui.label(label);
                            ui.add(
                                egui::DragValue::new(value)
                                    .speed(0.1)
                                    .range(min..=1_000_000.0)
                                    .suffix(" us"),
                            );
                            ui.end_row();
                        }
                    } else {
                        ui.label("Value");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_stimulus_dc_value)
                                .speed(stimulus_value_speed(selected_kind))
                                .suffix(selected_kind.value_unit()),
                        );
                        ui.end_row();
                    }
                });
            ui.horizontal(|ui| {
                if ui.button("Reload Source Values").clicked() {
                    load_selected_analog_stimulus_values(
                        &choices,
                        &self.analog_stimulus_scenario,
                        &self.analog_stimulus_component,
                        &mut self.analog_stimulus_dc_value,
                        &mut self.analog_stimulus_initial_value,
                        &mut self.analog_stimulus_pulsed_value,
                        &mut self.analog_stimulus_delay_us,
                        &mut self.analog_stimulus_rise_us,
                        &mut self.analog_stimulus_fall_us,
                        &mut self.analog_stimulus_width_us,
                        &mut self.analog_stimulus_period_us,
                    );
                }
                if ui.button("Save Source Stimulus").clicked() {
                    self.apply_replace_analog_stimulus(selected_kind);
                }
            });
        });
    }

    pub(super) fn analog_model_file_manager(&mut self, ui: &mut egui::Ui) {
        let scenarios = match analog_model_file_scenarios(&self.project_yaml) {
            Ok(scenarios) => scenarios,
            Err(error) => {
                ui.collapsing("SPICE Model Files", |ui| {
                    ui.label(format!("Analog model files unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("SPICE Model Files", |ui| {
            if scenarios.is_empty() {
                ui.label("No analog run setup is available. Add one first.");
                return;
            }
            initialize_model_file_scenario_default(&scenarios, &mut self.analog_model_scenario);
            egui::Grid::new("analog_model_file_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_model_scenario_combo(
                        ui,
                        "analog_model_file_scenario",
                        &mut self.analog_model_scenario,
                        &scenarios,
                    );
                    ui.end_row();

                    ui.label("Model/include path");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.analog_model_path);
                        if ui.button("Browse").clicked() {
                            self.pick_analog_model_file_path();
                            self.refresh_analog_model_file_sha();
                        }
                    });
                    ui.end_row();

                    ui.label("SHA-256");
                    ui.horizontal(|ui| {
                        ui.monospace(if self.analog_model_sha256.is_empty() {
                            "not computed"
                        } else {
                            self.analog_model_sha256.as_str()
                        });
                        if ui.button("Compute").clicked() {
                            self.refresh_analog_model_file_sha();
                        }
                    });
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Add Model File").clicked() {
                    self.apply_add_analog_model_file();
                }
                if ui.button("Clear Fields").clicked() {
                    self.analog_model_path.clear();
                    self.analog_model_sha256.clear();
                }
            });

            let selected = scenarios
                .iter()
                .find(|scenario| scenario.name == self.analog_model_scenario)
                .or_else(|| scenarios.first());
            if let Some(scenario) = selected {
                ui.separator();
                ui.strong(format!("{} model files", scenario.name));
                if scenario.model_files.is_empty() {
                    ui.label("No model/include files declared for this run setup.");
                } else {
                    egui::Grid::new("analog_model_file_list")
                        .num_columns(3)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.strong("Path");
                            ui.strong("SHA-256");
                            ui.strong("Actions");
                            ui.end_row();
                            for model_file in &scenario.model_files {
                                ui.monospace(&model_file.path);
                                if let Some(sha256) = &model_file.sha256 {
                                    ui.monospace(sha256);
                                } else {
                                    ui.label("missing");
                                }
                                if ui.button("Remove").clicked() {
                                    self.apply_remove_analog_model_file(
                                        &scenario.name,
                                        &model_file.path,
                                    );
                                }
                                ui.end_row();
                            }
                        });
                }
            }
        });
    }

    pub(super) fn analog_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices,
            Err(error) => {
                ui.collapsing("Observation Check", |ui| {
                    ui.label(format!("Run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Observation Check", |ui| {
            if choices.is_empty() {
                ui.label("No analog run setup is available. Add one first.");
                return;
            }
            initialize_analog_assertion_defaults(
                &choices,
                &mut self.analog_assertion_scenario,
                &mut self.analog_assertion_probe,
                &mut self.analog_assertion_end_us,
            );
            let selected_scenario = choices
                .iter()
                .find(|scenario| scenario.name == self.analog_assertion_scenario);
            egui::Grid::new("analog_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "analog_assertion_scenario",
                        &mut self.analog_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_assertion_name);
                    ui.end_row();

                    ui.label("Probe");
                    analog_probe_combo(
                        ui,
                        "analog_assertion_probe",
                        &mut self.analog_assertion_probe,
                        selected_scenario,
                    );
                    ui.end_row();

                    ui.label("Aggregation");
                    string_combo(
                        ui,
                        "analog_assertion_aggregation",
                        &mut self.analog_assertion_aggregation,
                        &[
                            "sample",
                            "min",
                            "max",
                            "mean",
                            "rms",
                            "rising_crossing_time",
                            "falling_crossing_time",
                            "min_high_pulse_width",
                            "min_low_pulse_width",
                            "duty_cycle",
                            "crossing_count",
                            "rising_crossing_count",
                            "falling_crossing_count",
                        ],
                    );
                    ui.end_row();

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "analog_assertion_relation",
                        &mut self.analog_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    let unit = selected_scenario
                        .and_then(|scenario| {
                            scenario
                                .probes
                                .iter()
                                .find(|probe| probe.name == self.analog_assertion_probe)
                        })
                        .map(|probe| match probe.quantity.as_str() {
                            "current" => " A",
                            "power" => " W",
                            _ => " V",
                        })
                        .unwrap_or(" V");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_assertion_threshold)
                            .speed(0.1)
                            .suffix(unit),
                    );
                    ui.end_row();

                    if self.analog_assertion_aggregation == "sample" {
                        ui.label("At");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_assertion_at_us)
                                .speed(1.0)
                                .range(0.0..=1_000_000.0)
                                .suffix(" us"),
                        );
                        ui.end_row();
                    } else {
                        ui.label("Start");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_assertion_start_us)
                                .speed(1.0)
                                .range(0.0..=1_000_000.0)
                                .suffix(" us"),
                        );
                        ui.end_row();

                        ui.label("End");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_assertion_end_us)
                                .speed(1.0)
                                .range(0.0..=1_000_000.0)
                                .suffix(" us"),
                        );
                        ui.end_row();

                        if matches!(
                            self.analog_assertion_aggregation.as_str(),
                            "rising_crossing_time"
                                | "falling_crossing_time"
                                | "min_high_pulse_width"
                                | "min_low_pulse_width"
                        ) {
                            ui.label("Time limit");
                            ui.add(
                                egui::DragValue::new(&mut self.analog_assertion_time_limit_us)
                                    .speed(1.0)
                                    .range(0.0..=1_000_000.0)
                                    .suffix(" us"),
                            );
                            ui.end_row();
                        } else if self.analog_assertion_aggregation == "duty_cycle" {
                            ui.label("Duty limit");
                            ui.add(
                                egui::DragValue::new(&mut self.analog_assertion_duty_limit_percent)
                                    .speed(0.5)
                                    .range(0.0..=100.0)
                                    .suffix("%"),
                            );
                            ui.end_row();
                        } else if matches!(
                            self.analog_assertion_aggregation.as_str(),
                            "crossing_count" | "rising_crossing_count" | "falling_crossing_count"
                        ) {
                            ui.label("Count limit");
                            ui.add(
                                egui::DragValue::new(&mut self.analog_assertion_count_limit)
                                    .speed(1.0)
                                    .range(0.0..=1_000_000.0)
                                    .suffix(" crossings"),
                            );
                            ui.end_row();
                        }
                    }
                });
            if self.analog_assertion_edit_original.trim().is_empty() {
                if ui.button("Add Check").clicked() {
                    self.apply_add_analog_assertion();
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Editing {}",
                        self.analog_assertion_edit_original.trim()
                    ));
                    if ui.button("Save Check").clicked() {
                        self.apply_replace_analog_assertion();
                    }
                    if ui.button("Cancel Edit").clicked() {
                        self.analog_assertion_edit_original.clear();
                    }
                });
            }
        });
    }

    pub(super) fn analog_sweep_editor(&mut self, ui: &mut egui::Ui) {
        let scenarios = match analog_sweep_scenarios(&self.project_yaml) {
            Ok(scenarios) => scenarios,
            Err(error) => {
                ui.collapsing("Run Input Sweeps", |ui| {
                    ui.label(format!("Run input sweeps unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Run Input Sweeps", |ui| {
            if scenarios.is_empty() {
                ui.label("No analog run setup is available. Add one first.");
                return;
            }
            initialize_analog_sweep_defaults(
                &scenarios,
                &mut self.analog_sweep_scenario,
                &mut self.analog_sweep_name,
            );
            egui::Grid::new("analog_sweep_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_sweep_scenario_combo(ui, &scenarios, &mut self.analog_sweep_scenario);
                    ui.end_row();

                    ui.label("Sweep");
                    ui.text_edit_singleline(&mut self.analog_sweep_name);
                    ui.end_row();

                    ui.label("Parameter");
                    ui.text_edit_singleline(&mut self.analog_sweep_parameter_name);
                    ui.end_row();

                    ui.label("Values");
                    ui.text_edit_singleline(&mut self.analog_sweep_parameter_values);
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Add Sweep + Parameter").clicked() {
                    self.apply_add_analog_sweep();
                }
                let selected_sweep = selected_analog_sweep(
                    &scenarios,
                    &self.analog_sweep_scenario,
                    &self.analog_sweep_name,
                );
                if ui
                    .add_enabled(selected_sweep.is_some(), egui::Button::new("Remove Sweep"))
                    .clicked()
                {
                    self.apply_remove_analog_sweep();
                }
            });

            if let Some(selected_scenario) =
                selected_analog_sweep_scenario(&scenarios, &self.analog_sweep_scenario)
            {
                analog_sweep_rows(ui, selected_scenario, &mut self.analog_sweep_name);
            }

            if selected_analog_sweep(
                &scenarios,
                &self.analog_sweep_scenario,
                &self.analog_sweep_name,
            )
            .is_some()
            {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Add Parameter").clicked() {
                        self.apply_add_analog_sweep_parameter();
                    }
                    if ui.button("Remove Parameter").clicked() {
                        self.apply_remove_analog_sweep_parameter();
                    }
                });
            }
        });
    }

    pub(super) fn selected_probe_assertions_panel(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Selected Probe Checks", |ui| {
            if self.analog_assertion_scenario.trim().is_empty()
                || self.analog_assertion_probe.trim().is_empty()
            {
                ui.label("Select a schematic probe element to inspect its checks.");
                return;
            }
            ui.horizontal(|ui| {
                ui.strong(format!(
                    "{} / {}",
                    self.analog_assertion_scenario, self.analog_assertion_probe
                ));
                if ui.button("Add from current settings").clicked() {
                    let scenario_name = self.analog_assertion_scenario.clone();
                    let probe_name = self.analog_assertion_probe.clone();
                    self.apply_add_canvas_probe_assertion(&scenario_name, &probe_name);
                }
                if ui.button("Clear checks").clicked() {
                    let scenario_name = self.analog_assertion_scenario.clone();
                    let probe_name = self.analog_assertion_probe.clone();
                    self.apply_remove_canvas_probe_assertions(&scenario_name, &probe_name);
                }
            });
            let rows = match analog_probe_assertion_summaries(
                &self.project_yaml,
                self.report.as_ref(),
                &self.analog_assertion_scenario,
                &self.analog_assertion_probe,
            ) {
                Ok(rows) => rows,
                Err(error) => {
                    ui.label(format!("Selected probe unavailable: {error}"));
                    return;
                }
            };
            if rows.is_empty() {
                ui.label("No checks reference this probe.");
                return;
            }
            egui::Grid::new("selected_probe_assertions")
                .num_columns(6)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Status");
                    ui.strong("Check");
                    ui.strong("Check");
                    ui.strong("Timing");
                    ui.strong("Failure");
                    ui.strong("Actions");
                    ui.end_row();
                    for row in rows {
                        ui.colored_label(assertion_status_color(row.status), row.status.label());
                        ui.monospace(&row.name);
                        ui.label(format!(
                            "{} {} {}",
                            row.aggregation, row.relation, row.threshold
                        ));
                        ui.label(row.timing);
                        if let Some(message) = row.failure_message {
                            ui.label(message);
                        } else {
                            ui.label("");
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Edit").clicked() {
                                self.load_analog_assertion_editor(&row.draft, &row.name);
                            }
                            if ui.button("Delete").clicked() {
                                self.apply_remove_analog_assertion(
                                    &row.draft.scenario_name,
                                    &row.name,
                                );
                            }
                        });
                        ui.end_row();
                    }
                });
        });
    }

    fn apply_add_analog_scenario(&mut self) {
        let draft = AnalogScenarioDraft {
            name: self.analog_scenario_name.clone(),
            ground_net: self.analog_ground_net.clone(),
            probe_net: self.analog_probe_net.clone(),
            probe_name: self.analog_probe_name.clone(),
            stop_time_us: self.analog_stop_time_us,
            max_step_us: self.analog_max_step_us,
        };
        match append_analog_transient_scenario(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!("Run setup {} added.", self.analog_scenario_name.trim()),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_assertion(&mut self) {
        let draft = AnalogAssertionDraft {
            scenario_name: self.analog_assertion_scenario.clone(),
            assertion_name: self.analog_assertion_name.clone(),
            probe_name: self.analog_assertion_probe.clone(),
            aggregation: self.analog_assertion_aggregation.clone(),
            relation: self.analog_assertion_relation.clone(),
            threshold: self.analog_assertion_threshold,
            at_us: self.analog_assertion_at_us,
            start_us: self.analog_assertion_start_us,
            end_us: self.analog_assertion_end_us,
            time_limit_us: self.analog_assertion_time_limit_us,
            duty_limit_percent: self.analog_assertion_duty_limit_percent,
            count_limit: self.analog_assertion_count_limit,
        };
        match append_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Observation check {} added.",
                    self.analog_assertion_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_sweep(&mut self) {
        let draft = AnalogSweepParameterDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            parameter_name: self.analog_sweep_parameter_name.clone(),
            values_csv: self.analog_sweep_parameter_values.clone(),
        };
        match append_analog_sweep_with_parameter(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Run input sweep {} added with parameter {}.",
                    draft.sweep_name.trim(),
                    draft.parameter_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_analog_sweep(&mut self) {
        let draft = AnalogSweepDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
        };
        match remove_analog_sweep(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!("Run input sweep {} removed.", draft.sweep_name.trim()),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_sweep_parameter(&mut self) {
        let draft = AnalogSweepParameterDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            parameter_name: self.analog_sweep_parameter_name.clone(),
            values_csv: self.analog_sweep_parameter_values.clone(),
        };
        match append_analog_sweep_parameter(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Run input parameter {} added to sweep {}.",
                    draft.parameter_name.trim(),
                    draft.sweep_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_analog_sweep_parameter(&mut self) {
        let draft = AnalogSweepParameterDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            parameter_name: self.analog_sweep_parameter_name.clone(),
            values_csv: String::new(),
        };
        match remove_analog_sweep_parameter(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Run input parameter {} removed from sweep {}.",
                    draft.parameter_name.trim(),
                    draft.sweep_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_include_generated_component(&mut self) {
        let scenario_name = self.analog_generated_scenario.clone();
        let component_id = self.analog_generated_component.clone();
        self.apply_include_generated_component_by_id(&scenario_name, &component_id);
    }

    fn apply_include_generated_component_by_id(&mut self, scenario_name: &str, component_id: &str) {
        let draft = AnalogGeneratedComponentDraft {
            scenario_name: scenario_name.to_string(),
            component_id: component_id.to_string(),
        };
        match include_generated_component(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Component {} included in generated run setup {}.",
                    draft.component_id, draft.scenario_name
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_exclude_generated_component(&mut self) {
        let scenario_name = self.analog_generated_scenario.clone();
        let component_id = self.analog_generated_component.clone();
        self.apply_exclude_generated_component_by_id(&scenario_name, &component_id);
    }

    fn apply_exclude_generated_component_by_id(&mut self, scenario_name: &str, component_id: &str) {
        let draft = AnalogGeneratedComponentDraft {
            scenario_name: scenario_name.to_string(),
            component_id: component_id.to_string(),
        };
        match exclude_generated_component(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Component {} excluded from generated run setup {}.",
                    draft.component_id, draft.scenario_name
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_replace_generated_settings(&mut self) {
        let draft = AnalogGeneratedSettingsDraft {
            scenario_name: self.analog_generated_scenario.clone(),
            ground_net: self.analog_generated_ground_net.clone(),
            stop_time_us: self.analog_generated_stop_time_us,
            max_step_us: self.analog_generated_max_step_us,
        };
        match replace_generated_settings(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Generated run setup {} settings updated.",
                    draft.scenario_name
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_replace_generated_node_binding(&mut self) {
        let draft = AnalogGeneratedNodeBindingDraft {
            scenario_name: self.analog_generated_scenario.clone(),
            net: self.analog_generated_node_net.clone(),
            node: self.analog_generated_node_name.clone(),
        };
        match replace_generated_node_binding(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Generated run setup {} node binding for {} updated.",
                    draft.scenario_name, draft.net
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_replace_analog_stimulus(&mut self, kind: AnalogStimulusKind) {
        let draft = AnalogStimulusDraft {
            scenario_name: self.analog_stimulus_scenario.clone(),
            component_id: self.analog_stimulus_component.clone(),
            kind,
            dc_value: self.analog_stimulus_dc_value,
            pulse: AnalogStimulusPulseDraft {
                initial: self.analog_stimulus_initial_value,
                pulsed: self.analog_stimulus_pulsed_value,
                delay_us: self.analog_stimulus_delay_us,
                rise_us: self.analog_stimulus_rise_us,
                fall_us: self.analog_stimulus_fall_us,
                width_us: self.analog_stimulus_width_us,
                period_us: self.analog_stimulus_period_us,
            },
        };
        match replace_analog_stimulus(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Source stimulus {} in run setup {} updated.",
                    draft.component_id.trim(),
                    draft.scenario_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn refresh_analog_model_file_sha(&mut self) {
        match model_file_sha256(Path::new(&self.project_path), &self.analog_model_path) {
            Ok(sha256) => {
                self.analog_model_sha256 = sha256;
                self.status = "SPICE model file hash computed.".to_string();
                self.push_diagnostic("Computed SHA-256 for selected SPICE model file.");
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_model_file(&mut self) {
        if self.analog_model_sha256.trim().is_empty() {
            self.refresh_analog_model_file_sha();
            if self.analog_model_sha256.trim().is_empty() {
                return;
            }
        }
        let draft = AnalogModelFileDraft {
            scenario_name: self.analog_model_scenario.clone(),
            path: self.analog_model_path.clone(),
            sha256: self.analog_model_sha256.clone(),
        };
        match append_analog_model_file(&self.project_yaml, Path::new(&self.project_path), &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Analog model file {} added to run setup {}.",
                    draft.path.trim(),
                    draft.scenario_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_analog_model_file(&mut self, scenario_name: &str, path: &str) {
        let draft = AnalogModelFileRemoveDraft {
            scenario_name: scenario_name.to_string(),
            path: path.to_string(),
        };
        match remove_analog_model_file(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Analog model file {} removed from run setup {}.",
                    draft.path, draft.scenario_name
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_replace_analog_assertion(&mut self) {
        let original_assertion_name = self.analog_assertion_edit_original.trim().to_string();
        let draft = AnalogAssertionReplaceDraft {
            scenario_name: self.analog_assertion_scenario.clone(),
            original_assertion_name: original_assertion_name.clone(),
            replacement: AnalogAssertionDraft {
                scenario_name: self.analog_assertion_scenario.clone(),
                assertion_name: self.analog_assertion_name.clone(),
                probe_name: self.analog_assertion_probe.clone(),
                aggregation: self.analog_assertion_aggregation.clone(),
                relation: self.analog_assertion_relation.clone(),
                threshold: self.analog_assertion_threshold,
                at_us: self.analog_assertion_at_us,
                start_us: self.analog_assertion_start_us,
                end_us: self.analog_assertion_end_us,
                time_limit_us: self.analog_assertion_time_limit_us,
                duty_limit_percent: self.analog_assertion_duty_limit_percent,
                count_limit: self.analog_assertion_count_limit,
            },
        };
        match replace_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_edit_original.clear();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Observation check {original_assertion_name} updated."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn load_analog_assertion_editor(&mut self, draft: &AnalogAssertionDraft, original_name: &str) {
        self.analog_assertion_scenario = draft.scenario_name.clone();
        self.analog_assertion_name = draft.assertion_name.clone();
        self.analog_assertion_edit_original = original_name.to_string();
        self.analog_assertion_probe = draft.probe_name.clone();
        self.analog_assertion_aggregation = draft.aggregation.clone();
        self.analog_assertion_relation = draft.relation.clone();
        self.analog_assertion_threshold = draft.threshold;
        self.analog_assertion_at_us = draft.at_us;
        self.analog_assertion_start_us = draft.start_us;
        self.analog_assertion_end_us = draft.end_us;
        self.analog_assertion_time_limit_us = draft.time_limit_us;
        self.analog_assertion_duty_limit_percent = draft.duty_limit_percent;
        self.analog_assertion_count_limit = draft.count_limit;
        self.status = format!("Editing observation check {original_name}.");
    }

    fn apply_remove_analog_assertion(&mut self, scenario_name: &str, assertion_name: &str) {
        let draft = AnalogAssertionRemoveDraft {
            scenario_name: scenario_name.to_string(),
            assertion_name: assertion_name.to_string(),
        };
        match remove_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                if self.analog_assertion_scenario == draft.scenario_name
                    && self.analog_assertion_edit_original == draft.assertion_name
                {
                    self.analog_assertion_edit_original.clear();
                }
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Removed check {} in run setup {}.",
                        draft.assertion_name, draft.scenario_name
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_add_canvas_probe_assertion(
        &mut self,
        scenario_name: &str,
        probe_name: &str,
    ) {
        let requested_name = if self.analog_assertion_name.trim().is_empty()
            || self.analog_assertion_name.trim() == "probe_above_min"
        {
            format!("{probe_name}_check")
        } else {
            self.analog_assertion_name.trim().to_string()
        };
        let assertion_name = match unique_analog_assertion_name(
            &self.project_yaml,
            scenario_name,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogAssertionDraft {
            scenario_name: scenario_name.to_string(),
            assertion_name: assertion_name.clone(),
            probe_name: probe_name.to_string(),
            aggregation: self.analog_assertion_aggregation.clone(),
            relation: self.analog_assertion_relation.clone(),
            threshold: self.analog_assertion_threshold,
            at_us: self.waveform_cursor_a_us,
            start_us: self.analog_assertion_start_us,
            end_us: self.analog_assertion_end_us,
            time_limit_us: self.analog_assertion_time_limit_us,
            duty_limit_percent: self.analog_assertion_duty_limit_percent,
            count_limit: self.analog_assertion_count_limit,
        };
        match append_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_scenario = scenario_name.to_string();
                self.analog_assertion_probe = probe_name.to_string();
                self.analog_assertion_name = assertion_name.clone();
                self.analog_assertion_edit_original.clear();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Observation check {assertion_name} added from canvas probe element."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_quick_canvas_probe_assertion(
        &mut self,
        probe: &SketchProbe,
        relation: &str,
    ) {
        let Some(measured) = waveform_probe_value_for_badge(
            &self.waveforms,
            self.selected_waveform,
            self.waveform_cursor_a_us,
            probe,
        ) else {
            self.record_error(anyhow::anyhow!(
                "No loaded waveform sample matches probe {} at the current cursor.",
                probe.probe_name
            ));
            return;
        };
        let margin = quick_assertion_margin(measured);
        let threshold = match relation {
            "above" => measured - margin,
            "below" => measured + margin,
            _ => {
                self.record_error(anyhow::anyhow!(
                    "Quick check relation {relation} is not supported."
                ));
                return;
            }
        };
        let requested_name = format!("{}_{}_cursor", probe.probe_name, relation);
        let assertion_name = match unique_analog_assertion_name(
            &self.project_yaml,
            &probe.scenario_name,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogAssertionDraft {
            scenario_name: probe.scenario_name.clone(),
            assertion_name: assertion_name.clone(),
            probe_name: probe.probe_name.clone(),
            aggregation: "sample".to_string(),
            relation: relation.to_string(),
            threshold,
            at_us: self.waveform_cursor_a_us,
            start_us: self.analog_assertion_start_us,
            end_us: self.analog_assertion_end_us,
            time_limit_us: self.analog_assertion_time_limit_us,
            duty_limit_percent: self.analog_assertion_duty_limit_percent,
            count_limit: self.analog_assertion_count_limit,
        };
        match append_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_scenario = draft.scenario_name.clone();
                self.analog_assertion_name = assertion_name.clone();
                self.analog_assertion_edit_original.clear();
                self.analog_assertion_probe = draft.probe_name.clone();
                self.analog_assertion_aggregation = draft.aggregation.clone();
                self.analog_assertion_relation = draft.relation.clone();
                self.analog_assertion_threshold = draft.threshold;
                self.analog_assertion_at_us = draft.at_us;
                self.analog_assertion_time_limit_us = draft.time_limit_us;
                self.analog_assertion_duty_limit_percent = draft.duty_limit_percent;
                self.analog_assertion_count_limit = draft.count_limit;
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Quick check {assertion_name} added from {} = {}.",
                        probe.probe_name,
                        format_value(measured)
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_remove_canvas_probe_assertions(
        &mut self,
        scenario_name: &str,
        probe_name: &str,
    ) {
        let draft = AnalogProbeAssertionsRemoveDraft {
            scenario_name: scenario_name.to_string(),
            probe_name: probe_name.to_string(),
        };
        match remove_analog_assertions_for_probe(&self.project_yaml, &draft) {
            Ok(updated) => {
                if self.analog_assertion_scenario == draft.scenario_name
                    && self.analog_assertion_probe == draft.probe_name
                {
                    self.analog_assertion_name.clear();
                    self.analog_assertion_edit_original.clear();
                }
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Removed checks for probe {} in run setup {}.",
                        draft.probe_name, draft.scenario_name
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }
}

fn initialize_analog_sweep_defaults(
    scenarios: &[AnalogSweepScenario],
    selected_scenario: &mut String,
    selected_sweep: &mut String,
) {
    let scenario_stale = selected_scenario.trim().is_empty()
        || !scenarios
            .iter()
            .any(|scenario| scenario.name == *selected_scenario);
    if let (true, Some(scenario)) = (scenario_stale, scenarios.first()) {
        *selected_scenario = scenario.name.clone();
    }
    let Some(scenario) = selected_analog_sweep_scenario(scenarios, selected_scenario) else {
        return;
    };
    let sweep_stale = selected_sweep.trim().is_empty()
        || (!scenario.sweeps.is_empty()
            && !scenario
                .sweeps
                .iter()
                .any(|sweep| sweep.name == *selected_sweep));
    if let (true, Some(sweep)) = (sweep_stale, scenario.sweeps.first()) {
        *selected_sweep = sweep.name.clone();
    }
}

fn analog_sweep_scenario_combo(
    ui: &mut egui::Ui,
    scenarios: &[AnalogSweepScenario],
    selected_scenario: &mut String,
) {
    egui::ComboBox::from_id_salt("analog_sweep_scenario")
        .selected_text(if selected_scenario.is_empty() {
            "select run setup"
        } else {
            selected_scenario.as_str()
        })
        .show_ui(ui, |ui| {
            for scenario in scenarios {
                ui.selectable_value(selected_scenario, scenario.name.clone(), &scenario.name);
            }
        });
}

fn analog_sweep_rows(
    ui: &mut egui::Ui,
    scenario: &AnalogSweepScenario,
    selected_sweep: &mut String,
) {
    if scenario.sweeps.is_empty() {
        ui.label("No parameter sweeps are declared for this run setup.");
        return;
    }
    ui.add_space(4.0);
    ui.strong("Declared sweeps");
    egui::Grid::new("analog_sweep_rows")
        .num_columns(4)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Sweep");
            ui.strong("Parameters");
            ui.strong("Corners");
            ui.strong("Use");
            ui.end_row();
            for sweep in &scenario.sweeps {
                ui.monospace(&sweep.name);
                ui.label(parameter_summary(sweep));
                ui.label(sweep.corner_count.to_string());
                if ui.button("Select").clicked() {
                    *selected_sweep = sweep.name.clone();
                }
                ui.end_row();
            }
        });
}

fn selected_analog_sweep_scenario<'a>(
    scenarios: &'a [AnalogSweepScenario],
    selected_scenario: &str,
) -> Option<&'a AnalogSweepScenario> {
    scenarios
        .iter()
        .find(|scenario| scenario.name == selected_scenario)
}

fn selected_analog_sweep<'a>(
    scenarios: &'a [AnalogSweepScenario],
    selected_scenario: &str,
    selected_sweep: &str,
) -> Option<&'a AnalogSweepSummary> {
    selected_analog_sweep_scenario(scenarios, selected_scenario)?
        .sweeps
        .iter()
        .find(|sweep| sweep.name == selected_sweep)
}

fn parameter_summary(sweep: &AnalogSweepSummary) -> String {
    if sweep.parameters.is_empty() {
        return "none".to_string();
    }
    sweep
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "{} [{}]",
                parameter.name,
                parameter
                    .values
                    .iter()
                    .map(|value| format!("{value:.6}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}
