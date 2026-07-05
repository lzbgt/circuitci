use super::CircuitCiApp;
use super::analog_generated::{
    AnalogGeneratedComponentDraft, AnalogGeneratedNodeBindingDraft, AnalogGeneratedSettingsDraft,
    analog_generated_scenarios, exclude_generated_component,
    include_generated_component_with_project_path, replace_generated_node_binding,
    replace_generated_settings,
};
use super::simulation_forms::*;
use super::simulation_run_setup_controls::generated_noise_source_combo;
use eframe::egui;
use std::path::Path;

impl CircuitCiApp {
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
}
