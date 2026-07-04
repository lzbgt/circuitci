use super::CircuitCiApp;
use super::analog::{
    AnalogAcScenarioDraft, AnalogAssertionDraft, AnalogAssertionReplaceDraft,
    AnalogDcScenarioDraft, AnalogHarmonicBalanceScenarioDraft, AnalogNoiseScenarioDraft,
    AnalogScenarioDraft, analog_scenario_choices, append_analog_ac_scenario_with_project_path,
    append_analog_assertion, append_analog_dc_scenario_with_project_path,
    append_analog_harmonic_balance_scenario_with_project_path,
    append_analog_noise_scenario_with_project_path,
    append_analog_transient_scenario_with_project_path, replace_analog_assertion,
};
use super::analog_ac_presets::{analog_ac_assertion_presets, append_analog_ac_assertion_preset};
use super::analog_dc_presets::{analog_dc_assertion_presets, append_analog_dc_assertion_preset};
use super::analog_generated::{
    AnalogGeneratedComponentDraft, AnalogGeneratedNodeBindingDraft, AnalogGeneratedSettingsDraft,
    analog_generated_scenarios, exclude_generated_component,
    include_generated_component_with_project_path, replace_generated_node_binding,
    replace_generated_settings,
};
use super::analog_models::{
    AnalogModelFileDraft, AnalogModelFileRemoveDraft, analog_model_file_scenarios,
    append_analog_model_file, model_file_sha256, remove_analog_model_file,
};
use super::analog_noise_presets::{
    analog_noise_assertion_presets, append_analog_noise_assertion_preset,
};
use super::analog_stimulus::{
    AnalogStimulusDraft, AnalogStimulusKind, AnalogStimulusPulseDraft, analog_stimulus_choices,
    replace_analog_stimulus,
};
use super::simulation_forms::*;
use super::sketch::ProjectSnapshot;
use super::sketch_spice::SketchSpiceKind;
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
                    ui.label("Observation");
                    egui::ComboBox::from_id_salt("analog_run_setup_kind")
                        .selected_text(match self.analog_run_setup_kind.as_str() {
                            "ac" => "AC/Bode",
                            "dc" => "DC operating point",
                            "noise" => "Noise",
                            "hb" => "Harmonic Balance",
                            _ => "Transient",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.analog_run_setup_kind,
                                "transient".to_string(),
                                "Transient",
                            );
                            ui.selectable_value(
                                &mut self.analog_run_setup_kind,
                                "ac".to_string(),
                                "AC/Bode",
                            );
                            ui.selectable_value(
                                &mut self.analog_run_setup_kind,
                                "dc".to_string(),
                                "DC operating point",
                            );
                            ui.selectable_value(
                                &mut self.analog_run_setup_kind,
                                "noise".to_string(),
                                "Noise",
                            );
                            ui.selectable_value(
                                &mut self.analog_run_setup_kind,
                                "hb".to_string(),
                                "Harmonic Balance",
                            );
                        });
                    ui.end_row();

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

                    if matches!(self.analog_run_setup_kind.as_str(), "ac" | "noise") {
                        ui.label("Start frequency");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_start_frequency_hz)
                                .speed(10.0)
                                .range(1.0e-9..=1.0e15)
                                .suffix(" Hz"),
                        );
                        ui.end_row();

                        ui.label("Stop frequency");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_stop_frequency_hz)
                                .speed(100.0)
                                .range(1.0e-9..=1.0e15)
                                .suffix(" Hz"),
                        );
                        ui.end_row();

                        ui.label("Points/decade");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_points_per_decade)
                                .speed(1.0)
                                .range(1..=1000),
                        );
                        ui.end_row();
                        if self.analog_run_setup_kind == "noise" {
                            initialize_noise_source_default(
                                snapshot,
                                &mut self.analog_noise_input_source,
                            );
                            ui.label("Input source");
                            noise_source_combo(
                                ui,
                                "analog_noise_input_source",
                                &mut self.analog_noise_input_source,
                                snapshot,
                            );
                            ui.end_row();
                        }
                    } else if self.analog_run_setup_kind == "hb" {
                        initialize_noise_source_default(snapshot, &mut self.analog_hb_drive_source);
                        ui.label("Fundamental");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_hb_fundamental_frequency_hz)
                                .speed(100.0)
                                .range(1.0e-9..=1.0e15)
                                .suffix(" Hz"),
                        );
                        ui.end_row();

                        ui.label("Harmonics");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_hb_harmonics)
                                .speed(1.0)
                                .range(1..=1024),
                        );
                        ui.end_row();

                        ui.label("Drive source");
                        noise_source_combo(
                            ui,
                            "analog_hb_drive_source",
                            &mut self.analog_hb_drive_source,
                            snapshot,
                        );
                        ui.end_row();
                    } else if self.analog_run_setup_kind == "dc" {
                        ui.label("Analysis");
                        ui.label("Operating point");
                        ui.end_row();
                    } else {
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
                    }
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
                &mut self.analog_generated_start_frequency_hz,
                &mut self.analog_generated_stop_frequency_hz,
                &mut self.analog_generated_points_per_decade,
                &mut self.analog_generated_noise_input_source,
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
                            GeneratedSettingsFormValues {
                                ground_net: &mut self.analog_generated_ground_net,
                                stop_time_us: &mut self.analog_generated_stop_time_us,
                                max_step_us: &mut self.analog_generated_max_step_us,
                                start_frequency_hz: &mut self.analog_generated_start_frequency_hz,
                                stop_frequency_hz: &mut self.analog_generated_stop_frequency_hz,
                                points_per_decade: &mut self.analog_generated_points_per_decade,
                                noise_input_source: &mut self.analog_generated_noise_input_source,
                                node_net: &mut self.analog_generated_node_net,
                                node_name: &mut self.analog_generated_node_name,
                            },
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

                    if matches!(
                        selected_scenario.scenario_type.as_str(),
                        "analog_ac" | "analog_noise"
                    ) {
                        ui.label("Start frequency");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_generated_start_frequency_hz)
                                .speed(10.0)
                                .range(1.0e-9..=1.0e15)
                                .suffix(" Hz"),
                        );
                        ui.end_row();

                        ui.label("Stop frequency");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_generated_stop_frequency_hz)
                                .speed(100.0)
                                .range(1.0e-9..=1.0e15)
                                .suffix(" Hz"),
                        );
                        ui.end_row();

                        ui.label("Points/decade");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_generated_points_per_decade)
                                .speed(1.0)
                                .range(1..=1000),
                        );
                        ui.end_row();
                        if selected_scenario.scenario_type == "analog_noise" {
                            ui.label("Output node");
                            ui.monospace(
                                selected_scenario
                                    .noise_output_node
                                    .as_deref()
                                    .unwrap_or("unbound"),
                            );
                            ui.end_row();

                            ui.label("Input source");
                            generated_noise_source_combo(
                                ui,
                                selected_scenario,
                                &mut self.analog_generated_noise_input_source,
                            );
                            ui.end_row();
                        }
                    } else if selected_scenario.scenario_type == "analog_dc" {
                        ui.label("Analysis");
                        ui.label("Operating point");
                        ui.end_row();
                    } else {
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
                    }

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
                        GeneratedSettingsFormValues {
                            ground_net: &mut self.analog_generated_ground_net,
                            stop_time_us: &mut self.analog_generated_stop_time_us,
                            max_step_us: &mut self.analog_generated_max_step_us,
                            start_frequency_hz: &mut self.analog_generated_start_frequency_hz,
                            stop_frequency_hz: &mut self.analog_generated_stop_frequency_hz,
                            points_per_decade: &mut self.analog_generated_points_per_decade,
                            noise_input_source: &mut self.analog_generated_noise_input_source,
                            node_net: &mut self.analog_generated_node_net,
                            node_name: &mut self.analog_generated_node_name,
                        },
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
                    let aggregation_options = if selected_scenario
                        .is_some_and(|scenario| scenario.scenario_type == "analog_ac")
                    {
                        &[
                            "gain_db_at_frequency",
                            "phase_deg_at_frequency",
                            "rising_gain_crossing_frequency",
                            "falling_gain_crossing_frequency",
                            "phase_margin_deg",
                            "gain_margin_db",
                        ][..]
                    } else if selected_scenario
                        .is_some_and(|scenario| scenario.scenario_type == "analog_dc")
                    {
                        &["operating_point"][..]
                    } else {
                        &[
                            "sample",
                            "min",
                            "max",
                            "mean",
                            "rms",
                            "integral",
                            "energy",
                            "settling_time",
                            "overshoot_percent",
                            "rising_phase_delay",
                            "falling_phase_delay",
                            "rising_setup_time",
                            "rising_hold_time",
                            "falling_setup_time",
                            "falling_hold_time",
                            "rising_crossing_time",
                            "falling_crossing_time",
                            "min_high_pulse_width",
                            "min_low_pulse_width",
                            "duty_cycle",
                            "crossing_count",
                            "rising_crossing_count",
                            "falling_crossing_count",
                        ][..]
                    };
                    if !aggregation_options.contains(&self.analog_assertion_aggregation.as_str()) {
                        self.analog_assertion_aggregation = aggregation_options[0].to_string();
                    }
                    string_combo(
                        ui,
                        "analog_assertion_aggregation",
                        &mut self.analog_assertion_aggregation,
                        aggregation_options,
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

                    let reference_timing = matches!(
                        self.analog_assertion_aggregation.as_str(),
                        "rising_phase_delay"
                            | "falling_phase_delay"
                            | "rising_setup_time"
                            | "rising_hold_time"
                            | "falling_setup_time"
                            | "falling_hold_time"
                    );
                    if reference_timing {
                        ui.label("Reference");
                        analog_probe_combo(
                            ui,
                            "analog_assertion_reference_probe",
                            &mut self.analog_assertion_reference_probe,
                            selected_scenario,
                        );
                        ui.end_row();

                        ui.label("Reference threshold");
                        let reference_unit = selected_scenario
                            .and_then(|scenario| {
                                scenario.probes.iter().find(|probe| {
                                    probe.name == self.analog_assertion_reference_probe
                                })
                            })
                            .map(|probe| match probe.quantity.as_str() {
                                "current" => " A",
                                "power" => " W",
                                _ => " V",
                            })
                            .unwrap_or(" V");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_assertion_reference_threshold)
                                .speed(0.1)
                                .suffix(reference_unit),
                        );
                        ui.end_row();
                    }

                    let target_based = matches!(
                        self.analog_assertion_aggregation.as_str(),
                        "settling_time" | "overshoot_percent"
                    );
                    ui.label(if target_based { "Target" } else { "Threshold" });
                    let probe_quantity = selected_scenario
                        .and_then(|scenario| {
                            scenario
                                .probes
                                .iter()
                                .find(|probe| probe.name == self.analog_assertion_probe)
                        })
                        .map(|probe| probe.quantity.as_str());
                    let unit = probe_quantity
                        .map(|quantity| match quantity {
                            "current" => " A",
                            "power" => " W",
                            _ => " V",
                        })
                        .unwrap_or(" V");
                    let unit = match self.analog_assertion_aggregation.as_str() {
                        "energy" => " J",
                        "integral" => match probe_quantity {
                            Some("current") => " C",
                            Some("power") => " J",
                            _ => " V*s",
                        },
                        "gain_db_at_frequency"
                        | "rising_gain_crossing_frequency"
                        | "falling_gain_crossing_frequency"
                        | "gain_margin_db" => " dB",
                        "phase_deg_at_frequency" | "phase_margin_deg" => " deg",
                        "group_delay_s_at_frequency" => " s",
                        _ => unit,
                    };
                    ui.add(
                        egui::DragValue::new(if target_based {
                            &mut self.analog_assertion_target
                        } else {
                            &mut self.analog_assertion_threshold
                        })
                        .speed(0.1)
                        .suffix(unit),
                    );
                    ui.end_row();

                    if self.analog_assertion_aggregation == "settling_time" {
                        ui.label("Tolerance");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_assertion_tolerance)
                                .speed(0.01)
                                .range(0.0..=1_000_000.0)
                                .suffix(unit),
                        );
                        ui.end_row();
                    }

                    if matches!(
                        self.analog_assertion_aggregation.as_str(),
                        "gain_db_at_frequency"
                            | "phase_deg_at_frequency"
                            | "group_delay_s_at_frequency"
                    ) {
                        ui.label("At");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_assertion_at_hz)
                                .speed(10.0)
                                .range(0.000001..=1.0e12)
                                .suffix(" Hz"),
                        );
                        ui.end_row();
                    } else if matches!(
                        self.analog_assertion_aggregation.as_str(),
                        "rising_gain_crossing_frequency" | "falling_gain_crossing_frequency"
                    ) {
                        ui.label("Frequency limit");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_assertion_frequency_limit_hz)
                                .speed(10.0)
                                .range(0.000001..=1.0e12)
                                .suffix(" Hz"),
                        );
                        ui.end_row();
                    } else if self.analog_assertion_aggregation == "operating_point" {
                        ui.label("Analysis");
                        ui.label("Operating point");
                        ui.end_row();
                    } else if self.analog_assertion_aggregation == "sample" {
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
                                | "settling_time"
                                | "rising_phase_delay"
                                | "falling_phase_delay"
                                | "rising_setup_time"
                                | "rising_hold_time"
                                | "falling_setup_time"
                                | "falling_hold_time"
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
                        } else if self.analog_assertion_aggregation == "overshoot_percent" {
                            ui.label("Overshoot limit");
                            ui.add(
                                egui::DragValue::new(
                                    &mut self.analog_assertion_overshoot_limit_percent,
                                )
                                .speed(0.5)
                                .range(0.0..=1_000_000.0)
                                .suffix("%"),
                            );
                            ui.end_row();
                        }
                    }
                });
            if selected_scenario.is_some_and(|scenario| scenario.scenario_type == "analog_ac") {
                ui.separator();
                ui.strong("Bode check presets");
                egui::Grid::new("analog_ac_assertion_presets")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Preset");
                        ui.strong("Checks");
                        ui.strong("Add");
                        ui.end_row();
                        for preset in analog_ac_assertion_presets() {
                            ui.label(preset.label);
                            ui.label(preset.summary);
                            if ui
                                .add_enabled(
                                    !self.analog_assertion_probe.trim().is_empty(),
                                    egui::Button::new("Add"),
                                )
                                .clicked()
                            {
                                self.apply_add_analog_ac_assertion_preset(preset.id, preset.label);
                            }
                            ui.end_row();
                        }
                    });
            } else if selected_scenario
                .is_some_and(|scenario| scenario.scenario_type == "analog_dc")
            {
                ui.separator();
                ui.strong("DC check presets");
                egui::Grid::new("analog_dc_assertion_presets")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Preset");
                        ui.strong("Checks");
                        ui.strong("Add");
                        ui.end_row();
                        for preset in analog_dc_assertion_presets() {
                            ui.label(preset.label);
                            ui.label(preset.summary);
                            if ui
                                .add_enabled(
                                    !self.analog_assertion_probe.trim().is_empty(),
                                    egui::Button::new("Add"),
                                )
                                .clicked()
                            {
                                self.apply_add_analog_dc_assertion_preset(preset.id, preset.label);
                            }
                            ui.end_row();
                        }
                    });
            } else if selected_scenario
                .is_some_and(|scenario| scenario.scenario_type == "analog_noise")
            {
                ui.separator();
                ui.strong("Noise check presets");
                egui::Grid::new("analog_noise_assertion_presets")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Preset");
                        ui.strong("Checks");
                        ui.strong("Add");
                        ui.end_row();
                        for preset in analog_noise_assertion_presets() {
                            ui.label(preset.label);
                            ui.label(preset.summary);
                            if ui
                                .add_enabled(
                                    !self.analog_assertion_probe.trim().is_empty(),
                                    egui::Button::new("Add"),
                                )
                                .clicked()
                            {
                                self.apply_add_analog_noise_assertion_preset(
                                    preset.id,
                                    preset.label,
                                );
                            }
                            ui.end_row();
                        }
                    });
            }
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

    fn apply_add_analog_scenario(&mut self) {
        if self.analog_run_setup_kind == "ac" {
            let draft = AnalogAcScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                start_frequency_hz: self.analog_start_frequency_hz,
                stop_frequency_hz: self.analog_stop_frequency_hz,
                points_per_decade: self.analog_points_per_decade,
            };
            match append_analog_ac_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "AC/Bode run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "dc" {
            let draft = AnalogDcScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
            };
            match append_analog_dc_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "DC operating-point run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "noise" {
            let input_probe = format!("{}_input", self.analog_probe_name.trim());
            let draft = AnalogNoiseScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                output_probe_name: self.analog_probe_name.clone(),
                input_probe_name: input_probe,
                input_source: self.analog_noise_input_source.clone(),
                start_frequency_hz: self.analog_start_frequency_hz,
                stop_frequency_hz: self.analog_stop_frequency_hz,
                points_per_decade: self.analog_points_per_decade,
            };
            match append_analog_noise_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Noise run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "hb" {
            let draft = AnalogHarmonicBalanceScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                fundamental_frequency_hz: self.analog_hb_fundamental_frequency_hz,
                harmonics: self.analog_hb_harmonics,
                drive_source: self.analog_hb_drive_source.clone(),
            };
            match append_analog_harmonic_balance_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Harmonic-balance run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else {
            let draft = AnalogScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                stop_time_us: self.analog_stop_time_us,
                max_step_us: self.analog_max_step_us,
            };
            match append_analog_transient_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!("Run setup {} added.", self.analog_scenario_name.trim()),
                ),
                Err(error) => self.record_error(error),
            }
        }
    }

    fn apply_add_analog_assertion(&mut self) {
        let draft = AnalogAssertionDraft {
            scenario_name: self.analog_assertion_scenario.clone(),
            assertion_name: self.analog_assertion_name.clone(),
            probe_name: self.analog_assertion_probe.clone(),
            reference_probe: self.analog_assertion_reference_probe.clone(),
            aggregation: self.analog_assertion_aggregation.clone(),
            relation: self.analog_assertion_relation.clone(),
            threshold: self.analog_assertion_threshold,
            reference_threshold: self.analog_assertion_reference_threshold,
            target: self.analog_assertion_target,
            tolerance: self.analog_assertion_tolerance,
            at_us: self.analog_assertion_at_us,
            at_hz: self.analog_assertion_at_hz,
            start_us: self.analog_assertion_start_us,
            end_us: self.analog_assertion_end_us,
            time_limit_us: self.analog_assertion_time_limit_us,
            frequency_limit_hz: self.analog_assertion_frequency_limit_hz,
            duty_limit_percent: self.analog_assertion_duty_limit_percent,
            count_limit: self.analog_assertion_count_limit,
            overshoot_limit_percent: self.analog_assertion_overshoot_limit_percent,
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

    fn apply_add_analog_ac_assertion_preset(&mut self, preset_id: &str, preset_label: &str) {
        match append_analog_ac_assertion_preset(
            &self.project_yaml,
            &self.analog_assertion_scenario,
            &self.analog_assertion_probe,
            preset_id,
        ) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "AC/Bode check preset {preset_label} added for {}.",
                    self.analog_assertion_probe.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_dc_assertion_preset(&mut self, preset_id: &str, preset_label: &str) {
        match append_analog_dc_assertion_preset(
            &self.project_yaml,
            &self.analog_assertion_scenario,
            &self.analog_assertion_probe,
            preset_id,
        ) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "DC check preset {preset_label} added for {}.",
                    self.analog_assertion_probe.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_noise_assertion_preset(&mut self, preset_id: &str, preset_label: &str) {
        match append_analog_noise_assertion_preset(
            &self.project_yaml,
            &self.analog_assertion_scenario,
            &self.analog_assertion_probe,
            preset_id,
        ) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Noise check preset {preset_label} added for {}.",
                    self.analog_assertion_probe.trim()
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
        match include_generated_component_with_project_path(
            &self.project_yaml,
            Path::new(&self.project_path),
            &draft,
        ) {
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
            start_frequency_hz: self.analog_generated_start_frequency_hz,
            stop_frequency_hz: self.analog_generated_stop_frequency_hz,
            points_per_decade: self.analog_generated_points_per_decade,
            noise_input_source: self.analog_generated_noise_input_source.clone(),
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
                reference_probe: self.analog_assertion_reference_probe.clone(),
                aggregation: self.analog_assertion_aggregation.clone(),
                relation: self.analog_assertion_relation.clone(),
                threshold: self.analog_assertion_threshold,
                reference_threshold: self.analog_assertion_reference_threshold,
                target: self.analog_assertion_target,
                tolerance: self.analog_assertion_tolerance,
                at_us: self.analog_assertion_at_us,
                at_hz: self.analog_assertion_at_hz,
                start_us: self.analog_assertion_start_us,
                end_us: self.analog_assertion_end_us,
                time_limit_us: self.analog_assertion_time_limit_us,
                frequency_limit_hz: self.analog_assertion_frequency_limit_hz,
                duty_limit_percent: self.analog_assertion_duty_limit_percent,
                count_limit: self.analog_assertion_count_limit,
                overshoot_limit_percent: self.analog_assertion_overshoot_limit_percent,
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
}

fn initialize_noise_source_default(snapshot: &ProjectSnapshot, selected: &mut String) {
    if !selected.is_empty()
        && snapshot
            .components_detail
            .iter()
            .any(|component| component.id == *selected)
    {
        return;
    }
    if let Some(component) = snapshot
        .components_detail
        .iter()
        .find(|component| component.spice.as_ref().is_some_and(is_noise_source_spice))
        .or_else(|| snapshot.components_detail.first())
    {
        *selected = component.id.clone();
    }
}

fn noise_source_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    snapshot: &ProjectSnapshot,
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select source"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for component in snapshot
                .components_detail
                .iter()
                .filter(|component| component.spice.as_ref().is_some_and(is_noise_source_spice))
                .chain(snapshot.components_detail.iter().filter(|component| {
                    component
                        .spice
                        .as_ref()
                        .is_none_or(|spice| !is_noise_source_spice(spice))
                }))
            {
                ui.selectable_value(selected, component.id.clone(), &component.id);
            }
        });
}

fn generated_noise_source_combo(
    ui: &mut egui::Ui,
    scenario: &super::analog_generated::AnalogGeneratedScenario,
    selected: &mut String,
) {
    if selected.is_empty()
        && let Some(input_source) = &scenario.noise_input_source
    {
        *selected = input_source.clone();
    }
    egui::ComboBox::from_id_salt("analog_generated_noise_input_source")
        .selected_text(if selected.is_empty() {
            "select source"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for component in &scenario.board_components {
                ui.selectable_value(selected, component.id.clone(), &component.id);
            }
        });
}

fn is_noise_source_spice(spice: &super::sketch_spice::SketchComponentSpice) -> bool {
    matches!(
        spice.kind,
        SketchSpiceKind::DcVoltageSource
            | SketchSpiceKind::PulseVoltageSource
            | SketchSpiceKind::DcCurrentSource
            | SketchSpiceKind::PulseCurrentSource
    )
}
