use super::CircuitCiApp;
use super::analog::{
    AnalogSParameterNetworkAssertionDraft, AnalogSParameterReflectionDraft, AnalogScenarioChoice,
    analog_scenario_choices, append_analog_sparameter_network_assertion,
    unique_analog_sparameter_network_assertion_name,
};
use super::simulation_forms::{analog_scenario_combo, string_combo};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn sparameter_network_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices
                .into_iter()
                .filter(|scenario| scenario.scenario_type == "analog_sparameter")
                .collect::<Vec<_>>(),
            Err(error) => {
                ui.collapsing("RF Network Check", |ui| {
                    ui.label(format!("S-parameter run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("RF Network Check", |ui| {
            if choices.is_empty() {
                ui.label("No S-parameter run setup is available.");
                return;
            }
            initialize_sparameter_network_assertion_defaults(
                &choices,
                &mut self.analog_sparameter_network_assertion_scenario,
                &mut self.analog_sparameter_network_assertion_name,
                &mut self.analog_sparameter_network_assertion_metric,
                &mut self.analog_sparameter_network_assertion_relation,
                &mut self.analog_sparameter_network_assertion_threshold,
            );
            egui::Grid::new("sparameter_network_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "sparameter_network_assertion_scenario",
                        &mut self.analog_sparameter_network_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_sparameter_network_assertion_name);
                    ui.end_row();

                    ui.label("Metric");
                    let previous_metric = self.analog_sparameter_network_assertion_metric.clone();
                    sparameter_network_metric_combo(
                        ui,
                        &mut self.analog_sparameter_network_assertion_metric,
                    );
                    if previous_metric != self.analog_sparameter_network_assertion_metric {
                        self.apply_sparameter_network_metric_defaults(&previous_metric);
                    }
                    ui.end_row();

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "sparameter_network_assertion_relation",
                        &mut self.analog_sparameter_network_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.analog_sparameter_network_assertion_threshold,
                        )
                        .speed(0.01)
                        .range(-1.0e12..=1.0e12)
                        .suffix(" ratio"),
                    );
                    ui.end_row();

                    self.sparameter_reflection_editor_row(ui, ReflectionRole::Source);
                    self.sparameter_reflection_editor_row(ui, ReflectionRole::Load);
                });
            ui.horizontal(|ui| {
                if ui.button("Use Stability Preset").clicked() {
                    self.analog_sparameter_network_assertion_name = "stable_rollet_k".to_string();
                    self.analog_sparameter_network_assertion_metric = "rollet_k_min".to_string();
                    self.analog_sparameter_network_assertion_relation = "above".to_string();
                    self.analog_sparameter_network_assertion_threshold = 1.0;
                }
                if ui.button("Use Delta Preset").clicked() {
                    self.analog_sparameter_network_assertion_name = "stable_delta".to_string();
                    self.analog_sparameter_network_assertion_metric =
                        "stability_delta_magnitude_max".to_string();
                    self.analog_sparameter_network_assertion_relation = "below".to_string();
                    self.analog_sparameter_network_assertion_threshold = 1.0;
                }
                if ui.button("Add Network Check").clicked() {
                    self.apply_add_sparameter_network_assertion();
                }
            });
        });
    }

    fn apply_sparameter_network_metric_defaults(&mut self, previous_metric: &str) {
        if self
            .analog_sparameter_network_assertion_name
            .trim()
            .is_empty()
            || self.analog_sparameter_network_assertion_name
                == default_sparameter_network_assertion_name(previous_metric)
        {
            self.analog_sparameter_network_assertion_name =
                default_sparameter_network_assertion_name(
                    &self.analog_sparameter_network_assertion_metric,
                )
                .to_string();
        }
        self.analog_sparameter_network_assertion_relation =
            default_sparameter_network_relation(&self.analog_sparameter_network_assertion_metric)
                .to_string();
        self.analog_sparameter_network_assertion_threshold =
            default_sparameter_network_threshold(&self.analog_sparameter_network_assertion_metric);
        if sparameter_network_metric_requires_source_reflection(
            &self.analog_sparameter_network_assertion_metric,
        ) {
            self.analog_sparameter_network_source_reflection_enabled = true;
        }
        if sparameter_network_metric_requires_load_reflection(
            &self.analog_sparameter_network_assertion_metric,
        ) {
            self.analog_sparameter_network_load_reflection_enabled = true;
        }
    }

    fn sparameter_reflection_editor_row(&mut self, ui: &mut egui::Ui, role: ReflectionRole) {
        let required = role.required_for_metric(&self.analog_sparameter_network_assertion_metric);
        let (enabled, real, imaginary) = match role {
            ReflectionRole::Source => (
                &mut self.analog_sparameter_network_source_reflection_enabled,
                &mut self.analog_sparameter_network_source_reflection_real,
                &mut self.analog_sparameter_network_source_reflection_imaginary,
            ),
            ReflectionRole::Load => (
                &mut self.analog_sparameter_network_load_reflection_enabled,
                &mut self.analog_sparameter_network_load_reflection_real,
                &mut self.analog_sparameter_network_load_reflection_imaginary,
            ),
        };
        if required {
            *enabled = true;
        }
        ui.label(role.label());
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!required, |ui| {
                ui.checkbox(enabled, if required { "required" } else { "enabled" });
            });
            ui.add_enabled_ui(*enabled, |ui| {
                ui.label("Re");
                ui.add(egui::DragValue::new(real).speed(0.01).range(-0.999..=0.999));
                ui.label("Im");
                ui.add(
                    egui::DragValue::new(imaginary)
                        .speed(0.01)
                        .range(-0.999..=0.999),
                );
            });
        });
        ui.end_row();
    }

    fn apply_add_sparameter_network_assertion(&mut self) {
        let requested_name = if self
            .analog_sparameter_network_assertion_name
            .trim()
            .is_empty()
        {
            default_sparameter_network_assertion_name(
                &self.analog_sparameter_network_assertion_metric,
            )
            .to_string()
        } else {
            self.analog_sparameter_network_assertion_name
                .trim()
                .to_string()
        };
        let assertion_name = match unique_analog_sparameter_network_assertion_name(
            &self.project_yaml,
            &self.analog_sparameter_network_assertion_scenario,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogSParameterNetworkAssertionDraft {
            scenario_name: self.analog_sparameter_network_assertion_scenario.clone(),
            assertion_name: assertion_name.clone(),
            metric: self.analog_sparameter_network_assertion_metric.clone(),
            relation: self.analog_sparameter_network_assertion_relation.clone(),
            threshold: self.analog_sparameter_network_assertion_threshold,
            source_reflection: self
                .analog_sparameter_network_source_reflection_enabled
                .then_some(AnalogSParameterReflectionDraft {
                    real: self.analog_sparameter_network_source_reflection_real,
                    imaginary: self.analog_sparameter_network_source_reflection_imaginary,
                }),
            load_reflection: self
                .analog_sparameter_network_load_reflection_enabled
                .then_some(AnalogSParameterReflectionDraft {
                    real: self.analog_sparameter_network_load_reflection_real,
                    imaginary: self.analog_sparameter_network_load_reflection_imaginary,
                }),
        };
        match append_analog_sparameter_network_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_sparameter_network_assertion_name = assertion_name.clone();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("RF network check {} added.", assertion_name.trim()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ReflectionRole {
    Source,
    Load,
}

impl ReflectionRole {
    fn label(self) -> &'static str {
        match self {
            Self::Source => "Source gamma",
            Self::Load => "Load gamma",
        }
    }

    fn required_for_metric(self, metric: &str) -> bool {
        match self {
            Self::Source => sparameter_network_metric_requires_source_reflection(metric),
            Self::Load => sparameter_network_metric_requires_load_reflection(metric),
        }
    }
}

fn initialize_sparameter_network_assertion_defaults(
    choices: &[AnalogScenarioChoice],
    scenario_name: &mut String,
    assertion_name: &mut String,
    metric: &mut String,
    relation: &mut String,
    threshold: &mut f64,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == *scenario_name);
    if (scenario_name.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        *scenario_name = scenario.name.clone();
    }
    if !sparameter_network_metric_options()
        .iter()
        .any(|option| option.0 == metric.as_str())
    {
        *metric = "rollet_k_min".to_string();
    }
    if !matches!(relation.as_str(), "above" | "below") {
        *relation = default_sparameter_network_relation(metric).to_string();
    }
    if assertion_name.trim().is_empty() {
        *assertion_name = default_sparameter_network_assertion_name(metric).to_string();
    }
    if !threshold.is_finite() {
        *threshold = default_sparameter_network_threshold(metric);
    }
}

fn sparameter_network_metric_combo(ui: &mut egui::Ui, selected: &mut String) {
    let selected_label = sparameter_network_metric_options()
        .iter()
        .find(|option| option.0 == selected.as_str())
        .map(|option| option.1)
        .unwrap_or(selected.as_str());
    egui::ComboBox::from_id_salt("sparameter_network_assertion_metric")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            for (metric, label) in sparameter_network_metric_options() {
                ui.selectable_value(selected, (*metric).to_string(), *label);
            }
        });
}

fn sparameter_network_metric_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("rollet_k_min", "Rollet K minimum"),
        ("stability_delta_magnitude_max", "Stability |Delta| maximum"),
        (
            "maximum_available_gain_db_min",
            "Maximum available gain minimum",
        ),
        ("maximum_stable_gain_db_min", "Maximum stable gain minimum"),
        (
            "maximum_unilateral_gain_db_min",
            "Maximum unilateral gain minimum",
        ),
        ("transducer_gain_db_min", "Transducer gain minimum"),
        ("available_gain_db_min", "Available gain minimum"),
        ("operating_gain_db_min", "Operating gain minimum"),
        ("passivity_max_singular_value", "Passivity singular maximum"),
        ("reciprocity_error_linear", "Reciprocity error maximum"),
    ]
}

fn default_sparameter_network_assertion_name(metric: &str) -> &'static str {
    match metric {
        "stability_delta_magnitude_max" => "stable_delta",
        "maximum_available_gain_db_min" => "available_gain_floor",
        "maximum_stable_gain_db_min" => "stable_gain_floor",
        "maximum_unilateral_gain_db_min" => "unilateral_gain_floor",
        "transducer_gain_db_min" => "transducer_gain_floor",
        "available_gain_db_min" => "source_available_gain_floor",
        "operating_gain_db_min" => "load_operating_gain_floor",
        "passivity_max_singular_value" => "passive_two_port",
        "reciprocity_error_linear" => "reciprocal_two_port",
        _ => "stable_rollet_k",
    }
}

fn default_sparameter_network_relation(metric: &str) -> &'static str {
    if matches!(
        metric,
        "rollet_k_min"
            | "maximum_available_gain_db_min"
            | "maximum_stable_gain_db_min"
            | "maximum_unilateral_gain_db_min"
            | "transducer_gain_db_min"
            | "available_gain_db_min"
            | "operating_gain_db_min"
    ) {
        "above"
    } else {
        "below"
    }
}

fn default_sparameter_network_threshold(metric: &str) -> f64 {
    match metric {
        "reciprocity_error_linear" => 0.01,
        "maximum_available_gain_db_min"
        | "maximum_stable_gain_db_min"
        | "maximum_unilateral_gain_db_min"
        | "transducer_gain_db_min"
        | "available_gain_db_min"
        | "operating_gain_db_min" => 0.0,
        _ => 1.0,
    }
}

fn sparameter_network_metric_requires_source_reflection(metric: &str) -> bool {
    matches!(metric, "transducer_gain_db_min" | "available_gain_db_min")
}

fn sparameter_network_metric_requires_load_reflection(metric: &str) -> bool {
    matches!(metric, "transducer_gain_db_min" | "operating_gain_db_min")
}
