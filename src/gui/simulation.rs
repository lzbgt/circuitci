use super::CircuitCiApp;
use super::analog::{
    AnalogAssertionDraft, AnalogAssertionRemoveDraft, AnalogAssertionReplaceDraft,
    AnalogAssertionUiStatus, AnalogProbeAssertionsRemoveDraft, AnalogScenarioChoice,
    AnalogScenarioDraft, analog_probe_assertion_summaries, analog_scenario_choices,
    append_analog_assertion, append_analog_transient_scenario, remove_analog_assertion,
    remove_analog_assertions_for_probe, replace_analog_assertion, unique_analog_assertion_name,
};
use super::analog_generated::{
    AnalogGeneratedComponentDraft, AnalogGeneratedNodeBindingDraft, AnalogGeneratedScenario,
    AnalogGeneratedSettingsDraft, analog_generated_scenarios, exclude_generated_component,
    include_generated_component, replace_generated_node_binding, replace_generated_settings,
};
use super::analog_models::{
    AnalogModelFileDraft, AnalogModelFileRemoveDraft, AnalogModelFileScenario,
    analog_model_file_scenarios, append_analog_model_file, model_file_sha256,
    remove_analog_model_file,
};
use super::analog_stimulus::{
    AnalogStimulusChoice, AnalogStimulusDraft, AnalogStimulusKind, AnalogStimulusPulseDraft,
    analog_stimulus_choices, replace_analog_stimulus,
};
use super::sketch::ProjectSnapshot;
use super::sketch_probes::SketchProbe;
use super::waveform::{format_value, quick_assertion_margin, waveform_probe_value_for_badge};
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
            if ui.button("Fit Time").clicked()
                && let Some((start_us, end_us)) = super::waveform::waveform_time_range_for_view(
                    &self.waveforms,
                    self.selected_waveform,
                )
            {
                self.waveform_cursor_a_us = start_us;
                self.waveform_cursor_b_us = end_us;
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
        if self.project_yaml_dirty {
            self.save_project_yaml();
            if self.project_yaml_dirty {
                return;
            }
        }
        self.validate_project();
    }

    fn scope_side_dock(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.waveform_controls_panel(ui);
            ui.separator();
            if let Some(snapshot) = self.project_snapshot.clone() {
                self.analog_scenario_editor(ui, &snapshot);
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
                        ui.label("No waveform artifacts were emitted by the current scenario set.");
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

    fn analog_scenario_editor(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        ui.collapsing("Analog Transient Scenario", |ui| {
            initialize_analog_net_defaults(
                snapshot,
                &mut self.analog_ground_net,
                &mut self.analog_probe_net,
            );
            egui::Grid::new("analog_transient_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Scenario");
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
            if ui.button("Add Analog Scenario").clicked() {
                self.apply_add_analog_scenario();
            }
        });
    }

    fn analog_generated_components_editor(&mut self, ui: &mut egui::Ui) {
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
                ui.label("No generated_from_board analog scenario is available.");
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
                    ui.label("Scenario");
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

    fn analog_generated_settings_editor(&mut self, ui: &mut egui::Ui) {
        let scenarios = match analog_generated_scenarios(&self.project_yaml) {
            Ok(scenarios) => scenarios,
            Err(error) => {
                ui.collapsing("Generated Scenario Settings", |ui| {
                    ui.label(format!("Generated settings unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Generated Scenario Settings", |ui| {
            if scenarios.is_empty() {
                ui.label("No generated_from_board analog scenario is available.");
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
                    ui.label("Scenario");
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

    fn analog_stimulus_editor(&mut self, ui: &mut egui::Ui) {
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
                ui.label("No generated analog scenario source primitives are available.");
                ui.label("Add a generated scenario containing DC or pulse source components.");
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

    fn analog_model_file_manager(&mut self, ui: &mut egui::Ui) {
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
                ui.label("No analog scenario is available. Add one first.");
                return;
            }
            initialize_model_file_scenario_default(&scenarios, &mut self.analog_model_scenario);
            egui::Grid::new("analog_model_file_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Scenario");
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
                    ui.label("No model/include files declared for this scenario.");
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

    fn analog_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices,
            Err(error) => {
                ui.collapsing("Analog Assertion", |ui| {
                    ui.label(format!("Analog scenarios unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Analog Assertion", |ui| {
            if choices.is_empty() {
                ui.label("No analog scenario is available. Add one first.");
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
                    ui.label("Scenario");
                    analog_scenario_combo(
                        ui,
                        "analog_assertion_scenario",
                        &mut self.analog_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Assertion");
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
                        &["sample", "min", "max"],
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
                    }
                });
            if self.analog_assertion_edit_original.trim().is_empty() {
                if ui.button("Add Analog Assertion").clicked() {
                    self.apply_add_analog_assertion();
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Editing {}",
                        self.analog_assertion_edit_original.trim()
                    ));
                    if ui.button("Save Assertion").clicked() {
                        self.apply_replace_analog_assertion();
                    }
                    if ui.button("Cancel Edit").clicked() {
                        self.analog_assertion_edit_original.clear();
                    }
                });
            }
        });
    }

    fn selected_probe_assertions_panel(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Selected Probe Assertions", |ui| {
            if self.analog_assertion_scenario.trim().is_empty()
                || self.analog_assertion_probe.trim().is_empty()
            {
                ui.label("Select a schematic probe badge to inspect its assertions.");
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
                if ui.button("Clear assertions").clicked() {
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
                ui.label("No assertions reference this probe.");
                return;
            }
            egui::Grid::new("selected_probe_assertions")
                .num_columns(6)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Status");
                    ui.strong("Assertion");
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
                &format!(
                    "Analog scenario {} added.",
                    self.analog_scenario_name.trim()
                ),
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
        };
        match append_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Analog assertion {} added.",
                    self.analog_assertion_name.trim()
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
                    "Component {} included in generated scenario {}.",
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
                    "Component {} excluded from generated scenario {}.",
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
                    "Generated scenario {} settings updated.",
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
                    "Generated scenario {} node binding for {} updated.",
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
                    "Source stimulus {} in scenario {} updated.",
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
                    "Analog model file {} added to scenario {}.",
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
                    "Analog model file {} removed from scenario {}.",
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
            },
        };
        match replace_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_edit_original.clear();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Analog assertion {original_assertion_name} updated."),
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
        self.status = format!("Editing analog assertion {original_name}.");
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
                        "Removed assertion {} in scenario {}.",
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
        };
        match append_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_scenario = scenario_name.to_string();
                self.analog_assertion_probe = probe_name.to_string();
                self.analog_assertion_name = assertion_name.clone();
                self.analog_assertion_edit_original.clear();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Analog assertion {assertion_name} added from canvas probe badge."),
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
                    "Quick assertion relation {relation} is not supported."
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
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Quick assertion {assertion_name} added from {} = {}.",
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
                        "Removed assertions for probe {} in scenario {}.",
                        draft.probe_name, draft.scenario_name
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }
}

fn initialize_analog_net_defaults(
    snapshot: &ProjectSnapshot,
    ground_net: &mut String,
    probe_net: &mut String,
) {
    if ground_net.is_empty()
        && let Some(net) = snapshot.nets_detail.iter().find(|net| net.kind == "ground")
    {
        *ground_net = net.id.clone();
    }
    if probe_net.is_empty()
        && let Some(net) = snapshot
            .nets_detail
            .iter()
            .find(|net| net.kind != "ground")
            .or_else(|| snapshot.nets_detail.first())
    {
        *probe_net = net.id.clone();
    }
}

fn net_combo(ui: &mut egui::Ui, id: &str, selected: &mut String, snapshot: &ProjectSnapshot) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select net"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for net in &snapshot.nets_detail {
                ui.selectable_value(selected, net.id.clone(), &net.id);
            }
        });
}

fn initialize_analog_assertion_defaults(
    choices: &[AnalogScenarioChoice],
    scenario_name: &mut String,
    probe_name: &mut String,
    end_us: &mut f64,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == *scenario_name);
    if (scenario_name.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        *scenario_name = scenario.name.clone();
        *end_us = scenario.stop_time_us;
    }
    let selected_scenario = choices
        .iter()
        .find(|scenario| scenario.name == *scenario_name)
        .or_else(|| choices.first());
    if let Some(scenario) = selected_scenario {
        let probe_missing = !scenario
            .probes
            .iter()
            .any(|probe| probe.name == *probe_name);
        if (probe_name.is_empty() || probe_missing)
            && let Some(probe) = scenario.probes.first()
        {
            *probe_name = probe.name.clone();
        }
        if *end_us <= 0.0 || *end_us > scenario.stop_time_us {
            *end_us = scenario.stop_time_us;
        }
    }
}

fn initialize_model_file_scenario_default(
    scenarios: &[AnalogModelFileScenario],
    scenario_name: &mut String,
) {
    let scenario_missing = !scenarios
        .iter()
        .any(|scenario| scenario.name == *scenario_name);
    if (scenario_name.is_empty() || scenario_missing)
        && let Some(scenario) = scenarios.first()
    {
        *scenario_name = scenario.name.clone();
    }
}

fn initialize_generated_component_defaults(
    scenarios: &[AnalogGeneratedScenario],
    scenario_name: &mut String,
    component_id: &mut String,
) {
    let scenario_missing = selected_generated_scenario(scenarios, scenario_name).is_none();
    if (scenario_name.is_empty() || scenario_missing)
        && let Some(scenario) = scenarios.first()
    {
        *scenario_name = scenario.name.clone();
    }
    let Some(scenario) = selected_generated_scenario(scenarios, scenario_name) else {
        return;
    };
    let component_missing = !scenario
        .board_components
        .iter()
        .any(|component| component.id == *component_id);
    if (component_id.is_empty() || component_missing)
        && let Some(component) = scenario
            .board_components
            .iter()
            .find(|component| !component.included)
            .or_else(|| scenario.board_components.first())
    {
        *component_id = component.id.clone();
    }
}

#[allow(clippy::too_many_arguments)]
fn initialize_generated_settings_defaults(
    scenarios: &[AnalogGeneratedScenario],
    scenario_name: &mut String,
    ground_net: &mut String,
    stop_time_us: &mut f64,
    max_step_us: &mut f64,
    node_net: &mut String,
    node_name: &mut String,
) {
    let scenario_missing = selected_generated_scenario(scenarios, scenario_name).is_none();
    if (scenario_name.is_empty() || scenario_missing)
        && let Some(scenario) = scenarios.first()
    {
        *scenario_name = scenario.name.clone();
        load_generated_settings_values(
            scenario,
            ground_net,
            stop_time_us,
            max_step_us,
            node_net,
            node_name,
        );
        return;
    }
    let Some(scenario) = selected_generated_scenario(scenarios, scenario_name) else {
        return;
    };
    if ground_net.is_empty()
        || !scenario
            .board_nets
            .iter()
            .any(|net| net.id == *ground_net && net.kind == "ground")
    {
        *ground_net = scenario.ground_net.clone();
    }
    if *stop_time_us <= 0.0 || *max_step_us <= 0.0 || *max_step_us > *stop_time_us {
        *stop_time_us = scenario.stop_time_us;
        *max_step_us = scenario.max_step_us;
    }
    let node_net_missing = !scenario.board_nets.iter().any(|net| net.id == *node_net);
    if node_net.is_empty() || node_net_missing {
        load_generated_node_binding_values(scenario, node_net, node_name);
    }
}

fn load_generated_settings_values(
    scenario: &AnalogGeneratedScenario,
    ground_net: &mut String,
    stop_time_us: &mut f64,
    max_step_us: &mut f64,
    node_net: &mut String,
    node_name: &mut String,
) {
    *ground_net = scenario.ground_net.clone();
    *stop_time_us = scenario.stop_time_us;
    *max_step_us = scenario.max_step_us;
    load_generated_node_binding_values(scenario, node_net, node_name);
}

fn load_generated_node_binding_values(
    scenario: &AnalogGeneratedScenario,
    node_net: &mut String,
    node_name: &mut String,
) {
    if let Some(binding) = scenario
        .node_bindings
        .iter()
        .find(|binding| binding.net == scenario.ground_net)
        .or_else(|| scenario.node_bindings.first())
    {
        *node_net = binding.net.clone();
        *node_name = binding.node.clone();
    } else if let Some(net) = scenario.board_nets.first() {
        *node_net = net.id.clone();
        *node_name = if net.id == scenario.ground_net {
            "0".to_string()
        } else {
            net.id.clone()
        };
    }
}

fn selected_generated_scenario<'a>(
    scenarios: &'a [AnalogGeneratedScenario],
    scenario_name: &str,
) -> Option<&'a AnalogGeneratedScenario> {
    scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
}

#[allow(clippy::too_many_arguments)]
fn initialize_analog_stimulus_defaults(
    choices: &[AnalogStimulusChoice],
    scenario_name: &mut String,
    component_id: &mut String,
    dc_value: &mut f64,
    initial_value: &mut f64,
    pulsed_value: &mut f64,
    delay_us: &mut f64,
    rise_us: &mut f64,
    fall_us: &mut f64,
    width_us: &mut f64,
    period_us: &mut f64,
) {
    let selected_missing =
        selected_analog_stimulus_choice(choices, scenario_name, component_id).is_none();
    if (scenario_name.is_empty() || component_id.is_empty() || selected_missing)
        && let Some(choice) = choices.first()
    {
        *scenario_name = choice.scenario_name.clone();
        *component_id = choice.component_id.clone();
        load_analog_stimulus_choice_values(
            choice,
            dc_value,
            initial_value,
            pulsed_value,
            delay_us,
            rise_us,
            fall_us,
            width_us,
            period_us,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn load_selected_analog_stimulus_values(
    choices: &[AnalogStimulusChoice],
    scenario_name: &str,
    component_id: &str,
    dc_value: &mut f64,
    initial_value: &mut f64,
    pulsed_value: &mut f64,
    delay_us: &mut f64,
    rise_us: &mut f64,
    fall_us: &mut f64,
    width_us: &mut f64,
    period_us: &mut f64,
) {
    if let Some(choice) = selected_analog_stimulus_choice(choices, scenario_name, component_id) {
        load_analog_stimulus_choice_values(
            choice,
            dc_value,
            initial_value,
            pulsed_value,
            delay_us,
            rise_us,
            fall_us,
            width_us,
            period_us,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn load_analog_stimulus_choice_values(
    choice: &AnalogStimulusChoice,
    dc_value: &mut f64,
    initial_value: &mut f64,
    pulsed_value: &mut f64,
    delay_us: &mut f64,
    rise_us: &mut f64,
    fall_us: &mut f64,
    width_us: &mut f64,
    period_us: &mut f64,
) {
    *dc_value = choice.dc_value;
    *initial_value = choice.pulse.initial;
    *pulsed_value = choice.pulse.pulsed;
    *delay_us = choice.pulse.delay_us;
    *rise_us = choice.pulse.rise_us;
    *fall_us = choice.pulse.fall_us;
    *width_us = choice.pulse.width_us;
    *period_us = choice.pulse.period_us;
}

fn selected_analog_stimulus_choice<'a>(
    choices: &'a [AnalogStimulusChoice],
    scenario_name: &str,
    component_id: &str,
) -> Option<&'a AnalogStimulusChoice> {
    choices
        .iter()
        .find(|choice| choice.scenario_name == scenario_name && choice.component_id == component_id)
}

fn analog_scenario_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    choices: &[AnalogScenarioChoice],
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select scenario"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for scenario in choices {
                ui.selectable_value(selected, scenario.name.clone(), &scenario.name);
            }
        });
}

fn analog_model_scenario_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    scenarios: &[AnalogModelFileScenario],
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select scenario"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for scenario in scenarios {
                ui.selectable_value(selected, scenario.name.clone(), &scenario.name);
            }
        });
}

fn generated_scenario_combo(
    ui: &mut egui::Ui,
    selected: &mut String,
    scenarios: &[AnalogGeneratedScenario],
) {
    egui::ComboBox::from_id_salt("analog_generated_scenario")
        .selected_text(if selected.is_empty() {
            "select scenario"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for scenario in scenarios {
                ui.selectable_value(selected, scenario.name.clone(), &scenario.name);
            }
        });
}

fn generated_component_combo(
    ui: &mut egui::Ui,
    scenario: &AnalogGeneratedScenario,
    selected: &mut String,
) {
    egui::ComboBox::from_id_salt("analog_generated_component")
        .selected_text(if selected.is_empty() {
            "select component".to_string()
        } else {
            selected.clone()
        })
        .show_ui(ui, |ui| {
            for component in &scenario.board_components {
                ui.selectable_value(selected, component.id.clone(), component.label());
            }
        });
}

fn generated_net_combo(
    ui: &mut egui::Ui,
    id: &str,
    scenario: &AnalogGeneratedScenario,
    selected: &mut String,
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select net".to_string()
        } else {
            selected.clone()
        })
        .show_ui(ui, |ui| {
            for net in &scenario.board_nets {
                ui.selectable_value(
                    selected,
                    net.id.clone(),
                    format!("{} ({})", net.id, net.kind),
                );
            }
        });
}

fn analog_stimulus_combo(
    ui: &mut egui::Ui,
    scenario_name: &mut String,
    component_id: &mut String,
    choices: &[AnalogStimulusChoice],
) {
    let label = selected_analog_stimulus_choice(choices, scenario_name, component_id)
        .map(AnalogStimulusChoice::label)
        .unwrap_or_else(|| "select source".to_string());
    egui::ComboBox::from_id_salt("analog_stimulus_source")
        .selected_text(label)
        .show_ui(ui, |ui| {
            for choice in choices {
                if ui
                    .selectable_label(
                        choice.scenario_name == *scenario_name
                            && choice.component_id == *component_id,
                        choice.label(),
                    )
                    .clicked()
                {
                    *scenario_name = choice.scenario_name.clone();
                    *component_id = choice.component_id.clone();
                }
            }
        });
}

fn stimulus_value_speed(kind: AnalogStimulusKind) -> f64 {
    match kind {
        AnalogStimulusKind::DcVoltage | AnalogStimulusKind::PulseVoltage => 0.1,
        AnalogStimulusKind::DcCurrent | AnalogStimulusKind::PulseCurrent => 0.001,
    }
}

fn analog_probe_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    scenario: Option<&AnalogScenarioChoice>,
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select probe"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            if let Some(scenario) = scenario {
                for probe in &scenario.probes {
                    ui.selectable_value(
                        selected,
                        probe.name.clone(),
                        format!("{} ({})", probe.name, probe.quantity),
                    );
                }
            }
        });
}

fn assertion_status_color(status: AnalogAssertionUiStatus) -> egui::Color32 {
    match status {
        AnalogAssertionUiStatus::Unknown => egui::Color32::from_rgb(230, 190, 90),
        AnalogAssertionUiStatus::Pass => egui::Color32::from_rgb(86, 190, 112),
        AnalogAssertionUiStatus::Fail => egui::Color32::from_rgb(232, 83, 83),
    }
}

fn string_combo(ui: &mut egui::Ui, id: &str, selected: &mut String, values: &[&str]) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected.as_str())
        .show_ui(ui, |ui| {
            for value in values {
                ui.selectable_value(selected, (*value).to_string(), *value);
            }
        });
}
