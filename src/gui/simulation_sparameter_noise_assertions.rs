use super::CircuitCiApp;
use super::analog::{
    AnalogSParameterNoiseAssertionDraft, AnalogScenarioChoice, analog_scenario_choices,
    append_analog_sparameter_noise_assertion, unique_analog_sparameter_noise_assertion_name,
};
use super::simulation_forms::{analog_scenario_combo, string_combo};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn sparameter_noise_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices
                .into_iter()
                .filter(|scenario| scenario.scenario_type == "analog_sparameter")
                .collect::<Vec<_>>(),
            Err(error) => {
                ui.collapsing("RF Noise Check", |ui| {
                    ui.label(format!("S-parameter run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("RF Noise Check", |ui| {
            if choices.is_empty() {
                ui.label("No S-parameter run setup is available.");
                return;
            }
            initialize_sparameter_noise_assertion_defaults(&choices, self);
            egui::Grid::new("sparameter_noise_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "sparameter_noise_assertion_scenario",
                        &mut self.analog_sparameter_noise_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_sparameter_noise_assertion_name);
                    ui.end_row();

                    ui.label("Metric");
                    let previous_metric = self.analog_sparameter_noise_assertion_metric.clone();
                    sparameter_noise_metric_combo(
                        ui,
                        &mut self.analog_sparameter_noise_assertion_metric,
                    );
                    if previous_metric != self.analog_sparameter_noise_assertion_metric {
                        if self
                            .analog_sparameter_noise_assertion_name
                            .trim()
                            .is_empty()
                            || self.analog_sparameter_noise_assertion_name
                                == default_sparameter_noise_assertion_name(&previous_metric)
                        {
                            self.analog_sparameter_noise_assertion_name =
                                default_sparameter_noise_assertion_name(
                                    &self.analog_sparameter_noise_assertion_metric,
                                )
                                .to_string();
                        }
                        self.analog_sparameter_noise_assertion_relation =
                            default_sparameter_noise_relation(
                                &self.analog_sparameter_noise_assertion_metric,
                            )
                            .to_string();
                        self.analog_sparameter_noise_assertion_threshold =
                            default_sparameter_noise_threshold(
                                &self.analog_sparameter_noise_assertion_metric,
                            );
                    }
                    ui.end_row();

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "sparameter_noise_assertion_relation",
                        &mut self.analog_sparameter_noise_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_sparameter_noise_assertion_threshold)
                            .speed(sparameter_noise_threshold_speed(
                                &self.analog_sparameter_noise_assertion_metric,
                            ))
                            .range(0.0..=1.0e12)
                            .suffix(sparameter_noise_threshold_suffix(
                                &self.analog_sparameter_noise_assertion_metric,
                            )),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("NF").clicked() {
                    self.apply_sparameter_noise_preset("noise_figure_db_max");
                }
                if ui.button("NFmin").clicked() {
                    self.apply_sparameter_noise_preset("minimum_noise_figure_db_max");
                }
                if ui.button("Rn").clicked() {
                    self.apply_sparameter_noise_preset("equivalent_noise_resistance_ohm_max");
                }
                if ui.button("SOpt").clicked() {
                    self.apply_sparameter_noise_preset("optimum_source_reflection_magnitude_max");
                }
                if ui.button("Add Noise Check").clicked() {
                    self.apply_add_sparameter_noise_assertion();
                }
            });
        });
    }

    fn apply_sparameter_noise_preset(&mut self, metric: &str) {
        self.analog_sparameter_noise_assertion_metric = metric.to_string();
        self.analog_sparameter_noise_assertion_name =
            default_sparameter_noise_assertion_name(metric).to_string();
        self.analog_sparameter_noise_assertion_relation =
            default_sparameter_noise_relation(metric).to_string();
        self.analog_sparameter_noise_assertion_threshold =
            default_sparameter_noise_threshold(metric);
    }

    fn apply_add_sparameter_noise_assertion(&mut self) {
        let requested_name = if self
            .analog_sparameter_noise_assertion_name
            .trim()
            .is_empty()
        {
            default_sparameter_noise_assertion_name(&self.analog_sparameter_noise_assertion_metric)
                .to_string()
        } else {
            self.analog_sparameter_noise_assertion_name
                .trim()
                .to_string()
        };
        let assertion_name = match unique_analog_sparameter_noise_assertion_name(
            &self.project_yaml,
            &self.analog_sparameter_noise_assertion_scenario,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogSParameterNoiseAssertionDraft {
            scenario_name: self.analog_sparameter_noise_assertion_scenario.clone(),
            assertion_name: assertion_name.clone(),
            metric: self.analog_sparameter_noise_assertion_metric.clone(),
            relation: self.analog_sparameter_noise_assertion_relation.clone(),
            threshold: self.analog_sparameter_noise_assertion_threshold,
        };
        match append_analog_sparameter_noise_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_sparameter_noise_assertion_name = assertion_name.clone();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("RF noise check {} added.", assertion_name.trim()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }
}

fn initialize_sparameter_noise_assertion_defaults(
    choices: &[AnalogScenarioChoice],
    app: &mut CircuitCiApp,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == app.analog_sparameter_noise_assertion_scenario);
    if (app.analog_sparameter_noise_assertion_scenario.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        app.analog_sparameter_noise_assertion_scenario = scenario.name.clone();
    }
    if !sparameter_noise_metric_options()
        .iter()
        .any(|option| option.0 == app.analog_sparameter_noise_assertion_metric.as_str())
    {
        app.analog_sparameter_noise_assertion_metric = "noise_figure_db_max".to_string();
    }
    if !matches!(
        app.analog_sparameter_noise_assertion_relation.as_str(),
        "above" | "below"
    ) {
        app.analog_sparameter_noise_assertion_relation =
            default_sparameter_noise_relation(&app.analog_sparameter_noise_assertion_metric)
                .to_string();
    }
    if app.analog_sparameter_noise_assertion_name.trim().is_empty() {
        app.analog_sparameter_noise_assertion_name =
            default_sparameter_noise_assertion_name(&app.analog_sparameter_noise_assertion_metric)
                .to_string();
    }
    if !app.analog_sparameter_noise_assertion_threshold.is_finite() {
        app.analog_sparameter_noise_assertion_threshold =
            default_sparameter_noise_threshold(&app.analog_sparameter_noise_assertion_metric);
    }
}

fn sparameter_noise_metric_combo(ui: &mut egui::Ui, selected: &mut String) {
    let selected_label = sparameter_noise_metric_options()
        .iter()
        .find(|option| option.0 == selected.as_str())
        .map(|option| option.1)
        .unwrap_or(selected.as_str());
    egui::ComboBox::from_id_salt("sparameter_noise_assertion_metric")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            for (metric, label) in sparameter_noise_metric_options() {
                ui.selectable_value(selected, (*metric).to_string(), *label);
            }
        });
}

fn sparameter_noise_metric_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("noise_figure_db_max", "Noise figure maximum"),
        (
            "minimum_noise_figure_db_max",
            "Minimum noise figure maximum",
        ),
        (
            "equivalent_noise_resistance_ohm_max",
            "Equivalent noise resistance maximum",
        ),
        (
            "optimum_source_reflection_magnitude_max",
            "Optimum source gamma maximum",
        ),
    ]
}

fn default_sparameter_noise_assertion_name(metric: &str) -> &'static str {
    match metric {
        "minimum_noise_figure_db_max" => "rf_min_noise_figure_ceiling",
        "equivalent_noise_resistance_ohm_max" => "rf_noise_resistance_ceiling",
        "optimum_source_reflection_magnitude_max" => "rf_optimum_gamma_ceiling",
        _ => "rf_noise_figure_ceiling",
    }
}

fn default_sparameter_noise_relation(_metric: &str) -> &'static str {
    "below"
}

fn default_sparameter_noise_threshold(metric: &str) -> f64 {
    match metric {
        "minimum_noise_figure_db_max" => 2.0,
        "equivalent_noise_resistance_ohm_max" => 10.0,
        "optimum_source_reflection_magnitude_max" => 0.8,
        _ => 3.0,
    }
}

fn sparameter_noise_threshold_speed(metric: &str) -> f64 {
    match metric {
        "equivalent_noise_resistance_ohm_max" => 1.0,
        "optimum_source_reflection_magnitude_max" => 0.01,
        _ => 0.1,
    }
}

fn sparameter_noise_threshold_suffix(metric: &str) -> &'static str {
    match metric {
        "equivalent_noise_resistance_ohm_max" => " ohm",
        "optimum_source_reflection_magnitude_max" => " ratio",
        _ => " dB",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        default_sparameter_noise_assertion_name, default_sparameter_noise_threshold,
        sparameter_noise_threshold_suffix,
    };

    #[test]
    fn sparameter_noise_defaults_match_metric_units() {
        assert_eq!(
            default_sparameter_noise_assertion_name("minimum_noise_figure_db_max"),
            "rf_min_noise_figure_ceiling"
        );
        assert_eq!(
            sparameter_noise_threshold_suffix("equivalent_noise_resistance_ohm_max"),
            " ohm"
        );
        assert_eq!(
            default_sparameter_noise_threshold("optimum_source_reflection_magnitude_max"),
            0.8
        );
    }
}
