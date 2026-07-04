use super::CircuitCiApp;
use super::analog::{
    AnalogSParameterAssertionDraft, analog_scenario_choices, append_analog_sparameter_assertion,
    unique_analog_sparameter_assertion_name,
};
use super::simulation_forms::{analog_scenario_combo, string_combo};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn sparameter_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices
                .into_iter()
                .filter(|scenario| scenario.scenario_type == "analog_sparameter")
                .collect::<Vec<_>>(),
            Err(error) => {
                ui.collapsing("RF Port Check", |ui| {
                    ui.label(format!("S-parameter run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("RF Port Check", |ui| {
            if choices.is_empty() {
                ui.label("No S-parameter run setup is available.");
                return;
            }
            initialize_sparameter_assertion_defaults(&choices, self);
            egui::Grid::new("sparameter_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "sparameter_assertion_scenario",
                        &mut self.analog_sparameter_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_sparameter_assertion_name);
                    ui.end_row();

                    ui.label("Parameter");
                    ui.text_edit_singleline(&mut self.analog_sparameter_assertion_parameter);
                    ui.end_row();

                    ui.label("Metric");
                    let previous_metric = self.analog_sparameter_assertion_metric.clone();
                    sparameter_metric_combo(ui, &mut self.analog_sparameter_assertion_metric);
                    if previous_metric != self.analog_sparameter_assertion_metric {
                        if self.analog_sparameter_assertion_name.trim().is_empty()
                            || self.analog_sparameter_assertion_name
                                == default_sparameter_assertion_name(&previous_metric)
                        {
                            self.analog_sparameter_assertion_name =
                                default_sparameter_assertion_name(
                                    &self.analog_sparameter_assertion_metric,
                                )
                                .to_string();
                        }
                        self.analog_sparameter_assertion_parameter =
                            default_sparameter_parameter(&self.analog_sparameter_assertion_metric)
                                .to_string();
                        self.analog_sparameter_assertion_aggregation =
                            default_sparameter_aggregation(
                                &self.analog_sparameter_assertion_metric,
                            )
                            .to_string();
                        self.analog_sparameter_assertion_relation =
                            default_sparameter_relation(&self.analog_sparameter_assertion_metric)
                                .to_string();
                        self.analog_sparameter_assertion_threshold =
                            default_sparameter_threshold(&self.analog_sparameter_assertion_metric);
                    }
                    ui.end_row();

                    ui.label("Aggregation");
                    string_combo(
                        ui,
                        "sparameter_assertion_aggregation",
                        &mut self.analog_sparameter_assertion_aggregation,
                        &["min", "max"],
                    );
                    ui.end_row();

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "sparameter_assertion_relation",
                        &mut self.analog_sparameter_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_sparameter_assertion_threshold)
                            .speed(sparameter_threshold_speed(
                                &self.analog_sparameter_assertion_metric,
                            ))
                            .range(-1.0e12..=1.0e12)
                            .suffix(sparameter_threshold_suffix(
                                &self.analog_sparameter_assertion_metric,
                            )),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Return Loss").clicked() {
                    self.apply_sparameter_preset("return_loss_db");
                }
                if ui.button("Insertion Loss").clicked() {
                    self.apply_sparameter_preset("insertion_loss_db");
                }
                if ui.button("VSWR").clicked() {
                    self.apply_sparameter_preset("vswr");
                }
                if ui.button("Group Delay").clicked() {
                    self.apply_sparameter_preset("group_delay_s");
                }
                if ui.button("Add Port Check").clicked() {
                    self.apply_add_sparameter_assertion();
                }
            });
        });
    }

    fn apply_sparameter_preset(&mut self, metric: &str) {
        self.analog_sparameter_assertion_metric = metric.to_string();
        self.analog_sparameter_assertion_name =
            default_sparameter_assertion_name(metric).to_string();
        self.analog_sparameter_assertion_parameter =
            default_sparameter_parameter(metric).to_string();
        self.analog_sparameter_assertion_aggregation =
            default_sparameter_aggregation(metric).to_string();
        self.analog_sparameter_assertion_relation = default_sparameter_relation(metric).to_string();
        self.analog_sparameter_assertion_threshold = default_sparameter_threshold(metric);
    }

    fn apply_add_sparameter_assertion(&mut self) {
        let requested_name = if self.analog_sparameter_assertion_name.trim().is_empty() {
            default_sparameter_assertion_name(&self.analog_sparameter_assertion_metric).to_string()
        } else {
            self.analog_sparameter_assertion_name.trim().to_string()
        };
        let assertion_name = match unique_analog_sparameter_assertion_name(
            &self.project_yaml,
            &self.analog_sparameter_assertion_scenario,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogSParameterAssertionDraft {
            scenario_name: self.analog_sparameter_assertion_scenario.clone(),
            assertion_name: assertion_name.clone(),
            parameter: self.analog_sparameter_assertion_parameter.clone(),
            metric: self.analog_sparameter_assertion_metric.clone(),
            aggregation: self.analog_sparameter_assertion_aggregation.clone(),
            relation: self.analog_sparameter_assertion_relation.clone(),
            threshold: self.analog_sparameter_assertion_threshold,
        };
        match append_analog_sparameter_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_sparameter_assertion_name = assertion_name.clone();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("RF port check {} added.", assertion_name.trim()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }
}

fn initialize_sparameter_assertion_defaults(
    choices: &[super::analog::AnalogScenarioChoice],
    app: &mut CircuitCiApp,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == app.analog_sparameter_assertion_scenario);
    if (app.analog_sparameter_assertion_scenario.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        app.analog_sparameter_assertion_scenario = scenario.name.clone();
    }
    if !sparameter_metric_options()
        .iter()
        .any(|option| option.0 == app.analog_sparameter_assertion_metric.as_str())
    {
        app.analog_sparameter_assertion_metric = "return_loss_db".to_string();
    }
    if app.analog_sparameter_assertion_name.trim().is_empty() {
        app.analog_sparameter_assertion_name =
            default_sparameter_assertion_name(&app.analog_sparameter_assertion_metric).to_string();
    }
    if app.analog_sparameter_assertion_parameter.trim().is_empty() {
        app.analog_sparameter_assertion_parameter =
            default_sparameter_parameter(&app.analog_sparameter_assertion_metric).to_string();
    }
    if !matches!(
        app.analog_sparameter_assertion_aggregation.as_str(),
        "min" | "max"
    ) {
        app.analog_sparameter_assertion_aggregation =
            default_sparameter_aggregation(&app.analog_sparameter_assertion_metric).to_string();
    }
    if !matches!(
        app.analog_sparameter_assertion_relation.as_str(),
        "above" | "below"
    ) {
        app.analog_sparameter_assertion_relation =
            default_sparameter_relation(&app.analog_sparameter_assertion_metric).to_string();
    }
    if !app.analog_sparameter_assertion_threshold.is_finite() {
        app.analog_sparameter_assertion_threshold =
            default_sparameter_threshold(&app.analog_sparameter_assertion_metric);
    }
}

fn sparameter_metric_combo(ui: &mut egui::Ui, selected: &mut String) {
    let selected_label = sparameter_metric_options()
        .iter()
        .find(|option| option.0 == selected.as_str())
        .map(|option| option.1)
        .unwrap_or(selected.as_str());
    egui::ComboBox::from_id_salt("sparameter_assertion_metric")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            for (metric, label) in sparameter_metric_options() {
                ui.selectable_value(selected, (*metric).to_string(), *label);
            }
        });
}

fn sparameter_metric_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("return_loss_db", "Return loss"),
        ("insertion_loss_db", "Insertion loss"),
        ("vswr", "VSWR"),
        ("group_delay_s", "Group delay"),
        ("magnitude_db", "Magnitude dB"),
        ("magnitude_linear", "Magnitude linear"),
    ]
}

fn default_sparameter_assertion_name(metric: &str) -> &'static str {
    match metric {
        "insertion_loss_db" => "s21_insertion_loss_ceiling",
        "vswr" => "s11_vswr_ceiling",
        "group_delay_s" => "s21_group_delay_ceiling",
        "magnitude_db" => "s21_magnitude_db_ceiling",
        "magnitude_linear" => "s21_magnitude_ceiling",
        _ => "s11_return_loss_floor",
    }
}

fn default_sparameter_parameter(metric: &str) -> &'static str {
    match metric {
        "return_loss_db" | "vswr" => "s11",
        _ => "s21",
    }
}

fn default_sparameter_aggregation(metric: &str) -> &'static str {
    match metric {
        "return_loss_db" => "min",
        _ => "max",
    }
}

fn default_sparameter_relation(metric: &str) -> &'static str {
    match metric {
        "return_loss_db" => "above",
        _ => "below",
    }
}

fn default_sparameter_threshold(metric: &str) -> f64 {
    match metric {
        "return_loss_db" => 10.0,
        "insertion_loss_db" => 3.0,
        "vswr" => 2.0,
        "group_delay_s" => 1.0e-9,
        "magnitude_linear" => 1.0,
        _ => 0.0,
    }
}

fn sparameter_threshold_speed(metric: &str) -> f64 {
    match metric {
        "group_delay_s" => 1.0e-10,
        "magnitude_linear" | "vswr" => 0.01,
        _ => 0.1,
    }
}

fn sparameter_threshold_suffix(metric: &str) -> &'static str {
    match metric {
        "return_loss_db" | "insertion_loss_db" | "magnitude_db" => " dB",
        "group_delay_s" => " s",
        _ => " ratio",
    }
}
