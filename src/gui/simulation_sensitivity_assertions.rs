use super::CircuitCiApp;
use super::analog::{AnalogScenarioChoice, analog_scenario_choices};
use super::analog_sensitivity_assertions::{
    AnalogSensitivityAssertionDraft, append_analog_sensitivity_assertion,
    unique_analog_sensitivity_assertion_name,
};
use super::simulation_forms::{analog_scenario_combo, string_combo};
use crate::reports::Finding;
use eframe::egui;
use serde_json::Value;

impl CircuitCiApp {
    pub(super) fn sensitivity_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices
                .into_iter()
                .filter(|scenario| scenario.scenario_type == "analog_sensitivity")
                .collect::<Vec<_>>(),
            Err(error) => {
                ui.collapsing("Sensitivity Check", |ui| {
                    ui.label(format!("Sensitivity run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Sensitivity Check", |ui| {
            if choices.is_empty() {
                ui.label("No sensitivity run setup is available.");
                return;
            }
            initialize_sensitivity_assertion_defaults(&choices, self);
            egui::Grid::new("sensitivity_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "sensitivity_assertion_scenario",
                        &mut self.analog_sensitivity_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_sensitivity_assertion_name);
                    ui.end_row();

                    ui.label("Parameter");
                    ui.text_edit_singleline(&mut self.analog_sensitivity_assertion_parameter);
                    ui.end_row();

                    ui.label("Metric");
                    let previous_metric = self.analog_sensitivity_assertion_metric.clone();
                    sensitivity_metric_combo(ui, &mut self.analog_sensitivity_assertion_metric);
                    if previous_metric != self.analog_sensitivity_assertion_metric {
                        if self.analog_sensitivity_assertion_name.trim().is_empty()
                            || self.analog_sensitivity_assertion_name
                                == default_sensitivity_assertion_name(&previous_metric)
                        {
                            self.analog_sensitivity_assertion_name =
                                default_sensitivity_assertion_name(
                                    &self.analog_sensitivity_assertion_metric,
                                )
                                .to_string();
                        }
                        self.analog_sensitivity_assertion_relation =
                            default_sensitivity_relation(&self.analog_sensitivity_assertion_metric)
                                .to_string();
                    }
                    ui.end_row();

                    ui.label("Frequency");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.analog_sensitivity_assertion_frequency_enabled, "");
                        ui.add_enabled(
                            self.analog_sensitivity_assertion_frequency_enabled,
                            egui::DragValue::new(
                                &mut self.analog_sensitivity_assertion_frequency_hz,
                            )
                            .speed(10.0)
                            .range(1.0e-9..=1.0e15)
                            .suffix(" Hz"),
                        );
                    });
                    ui.end_row();

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "sensitivity_assertion_relation",
                        &mut self.analog_sensitivity_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_sensitivity_assertion_threshold)
                            .speed(0.0001)
                            .range(-1.0e12..=1.0e12)
                            .suffix(" output/param"),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Magnitude").clicked() {
                    self.apply_sensitivity_preset("sensitivity_magnitude");
                }
                if ui.button("Real").clicked() {
                    self.apply_sensitivity_preset("sensitivity_real");
                }
                if ui.button("Imag").clicked() {
                    self.apply_sensitivity_preset("sensitivity_imaginary");
                }
                if ui.button("Add Sensitivity Check").clicked() {
                    self.apply_add_sensitivity_assertion();
                }
            });
        });
    }

    pub(super) fn sensitivity_assertion_failure_actions(
        &mut self,
        ui: &mut egui::Ui,
        failures: &[Finding],
    ) {
        let rows = sensitivity_failure_rows(failures);
        ui.label("Sensitivity assertion failures");
        if rows.is_empty() {
            ui.label("No sensitivity assertion failures were emitted.");
            return;
        }
        egui::Grid::new("sensitivity_assertion_failure_actions")
            .num_columns(6)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Scenario");
                ui.strong("Check");
                ui.strong("Parameter");
                ui.strong("Metric");
                ui.strong("Limit");
                ui.strong("Actions");
                ui.end_row();
                for row in rows {
                    ui.monospace(&row.scenario);
                    ui.monospace(&row.assertion);
                    ui.monospace(&row.parameter);
                    ui.label(row.metric.as_str());
                    ui.label(format!(
                        "{} {:.6e}",
                        row.relation.as_deref().unwrap_or("limit"),
                        row.threshold.unwrap_or_default()
                    ));
                    if ui.button("Open Sensitivity Check").clicked() {
                        self.load_sensitivity_failure_row(&row);
                    }
                    ui.end_row();
                }
            });
    }

    fn apply_sensitivity_preset(&mut self, metric: &str) {
        self.analog_sensitivity_assertion_metric = metric.to_string();
        self.analog_sensitivity_assertion_name =
            default_sensitivity_assertion_name(metric).to_string();
        self.analog_sensitivity_assertion_relation =
            default_sensitivity_relation(metric).to_string();
    }

    fn apply_add_sensitivity_assertion(&mut self) {
        let requested_name = if self.analog_sensitivity_assertion_name.trim().is_empty() {
            default_sensitivity_assertion_name(&self.analog_sensitivity_assertion_metric)
                .to_string()
        } else {
            self.analog_sensitivity_assertion_name.trim().to_string()
        };
        let assertion_name = match unique_analog_sensitivity_assertion_name(
            &self.project_yaml,
            &self.analog_sensitivity_assertion_scenario,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogSensitivityAssertionDraft {
            scenario_name: self.analog_sensitivity_assertion_scenario.clone(),
            assertion_name: assertion_name.clone(),
            parameter: self.analog_sensitivity_assertion_parameter.clone(),
            frequency_hz: self
                .analog_sensitivity_assertion_frequency_enabled
                .then_some(self.analog_sensitivity_assertion_frequency_hz),
            metric: self.analog_sensitivity_assertion_metric.clone(),
            relation: self.analog_sensitivity_assertion_relation.clone(),
            threshold: self.analog_sensitivity_assertion_threshold,
        };
        match append_analog_sensitivity_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_sensitivity_assertion_name = assertion_name.clone();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Sensitivity check {} added.", assertion_name.trim()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn load_sensitivity_failure_row(&mut self, row: &SensitivityFailureRow) {
        self.analog_sensitivity_assertion_scenario = row.scenario.clone();
        self.analog_sensitivity_assertion_name = row.assertion.clone();
        self.analog_sensitivity_assertion_parameter = row.parameter.clone();
        self.analog_sensitivity_assertion_metric = row.metric.clone();
        self.analog_sensitivity_assertion_frequency_enabled = row.frequency_hz.is_some();
        if let Some(frequency_hz) = row.frequency_hz {
            self.analog_sensitivity_assertion_frequency_hz = frequency_hz;
        }
        if let Some(relation) = &row.relation {
            self.analog_sensitivity_assertion_relation = relation.clone();
        }
        if let Some(threshold) = row.threshold {
            self.analog_sensitivity_assertion_threshold = threshold;
        }
        self.status = format!(
            "Loaded sensitivity check {} from latest report.",
            row.assertion
        );
    }
}

#[derive(Debug, Clone)]
struct SensitivityFailureRow {
    scenario: String,
    assertion: String,
    parameter: String,
    frequency_hz: Option<f64>,
    metric: String,
    relation: Option<String>,
    threshold: Option<f64>,
}

fn sensitivity_failure_rows(failures: &[Finding]) -> Vec<SensitivityFailureRow> {
    failures
        .iter()
        .filter(|finding| finding.id == "SPICE_SENSITIVITY_ANALYSIS")
        .filter_map(sensitivity_failure_row)
        .collect()
}

fn sensitivity_failure_row(finding: &Finding) -> Option<SensitivityFailureRow> {
    let assertion = text_field(&finding.measured, "assertion")?;
    let parameter = text_field(&finding.measured, "parameter")?;
    let metric = text_field(&finding.measured, "metric")?;
    let frequency_hz = optional_f64_field(&finding.measured, "frequency_hz");
    let (relation, threshold) = threshold_limit(&finding.limit);
    Some(SensitivityFailureRow {
        scenario: finding.scenario.clone(),
        assertion,
        parameter,
        frequency_hz,
        metric,
        relation,
        threshold,
    })
}

fn initialize_sensitivity_assertion_defaults(
    choices: &[AnalogScenarioChoice],
    app: &mut CircuitCiApp,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == app.analog_sensitivity_assertion_scenario);
    if (app.analog_sensitivity_assertion_scenario.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        app.analog_sensitivity_assertion_scenario = scenario.name.clone();
    }
    if app.analog_sensitivity_assertion_parameter.trim().is_empty() {
        app.analog_sensitivity_assertion_parameter = "R1".to_string();
    }
    if !sensitivity_metric_options()
        .iter()
        .any(|option| option.0 == app.analog_sensitivity_assertion_metric.as_str())
    {
        app.analog_sensitivity_assertion_metric = "sensitivity_magnitude".to_string();
    }
    if !matches!(
        app.analog_sensitivity_assertion_relation.as_str(),
        "above" | "below"
    ) {
        app.analog_sensitivity_assertion_relation =
            default_sensitivity_relation(&app.analog_sensitivity_assertion_metric).to_string();
    }
    if app.analog_sensitivity_assertion_name.trim().is_empty() {
        app.analog_sensitivity_assertion_name =
            default_sensitivity_assertion_name(&app.analog_sensitivity_assertion_metric)
                .to_string();
    }
    if !app.analog_sensitivity_assertion_frequency_hz.is_finite()
        || app.analog_sensitivity_assertion_frequency_hz <= 0.0
    {
        app.analog_sensitivity_assertion_frequency_hz = 100.0;
    }
    if !app.analog_sensitivity_assertion_threshold.is_finite() {
        app.analog_sensitivity_assertion_threshold = 0.001;
    }
}

fn sensitivity_metric_combo(ui: &mut egui::Ui, selected: &mut String) {
    let selected_label = sensitivity_metric_options()
        .iter()
        .find(|option| option.0 == selected.as_str())
        .map(|option| option.1)
        .unwrap_or(selected.as_str());
    egui::ComboBox::from_id_salt("sensitivity_assertion_metric")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            for (metric, label) in sensitivity_metric_options() {
                ui.selectable_value(selected, (*metric).to_string(), *label);
            }
        });
}

fn sensitivity_metric_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("sensitivity_magnitude", "Magnitude"),
        ("sensitivity_real", "Real"),
        ("sensitivity_imaginary", "Imaginary"),
    ]
}

fn default_sensitivity_assertion_name(metric: &str) -> &'static str {
    match metric {
        "sensitivity_real" => "sensitivity_real_ceiling",
        "sensitivity_imaginary" => "sensitivity_imag_ceiling",
        _ => "sensitivity_magnitude_ceiling",
    }
}

fn default_sensitivity_relation(_metric: &str) -> &'static str {
    "below"
}

fn text_field(map: &std::collections::BTreeMap<String, Value>, name: &str) -> Option<String> {
    map.get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn optional_f64_field(map: &std::collections::BTreeMap<String, Value>, name: &str) -> Option<f64> {
    map.get(name).and_then(Value::as_f64)
}

fn threshold_limit(
    map: &std::collections::BTreeMap<String, Value>,
) -> (Option<String>, Option<f64>) {
    for (key, relation) in [
        ("above_threshold", "above"),
        ("below_threshold", "below"),
        ("required_parameter", "required"),
    ] {
        if let Some(value) = map.get(key).and_then(Value::as_f64) {
            return (Some(relation.to_string()), Some(value));
        }
    }
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reports::Severity;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn sensitivity_failure_loads_editor_state() {
        let mut app = CircuitCiApp::default();
        let row = sensitivity_failure_row(&sensitivity_failure()).unwrap();

        app.load_sensitivity_failure_row(&row);

        assert_eq!(app.analog_sensitivity_assertion_scenario, "sens_out");
        assert_eq!(
            app.analog_sensitivity_assertion_name,
            "r1_magnitude_ceiling"
        );
        assert_eq!(app.analog_sensitivity_assertion_parameter, "R1");
        assert!(app.analog_sensitivity_assertion_frequency_enabled);
        assert_eq!(app.analog_sensitivity_assertion_frequency_hz, 100.0);
        assert_eq!(
            app.analog_sensitivity_assertion_metric,
            "sensitivity_magnitude"
        );
        assert_eq!(app.analog_sensitivity_assertion_relation, "below");
        assert_eq!(app.analog_sensitivity_assertion_threshold, 0.001);
        assert_eq!(
            app.status,
            "Loaded sensitivity check r1_magnitude_ceiling from latest report."
        );
    }

    fn sensitivity_failure() -> Finding {
        Finding {
            id: "SPICE_SENSITIVITY_ANALYSIS".to_string(),
            scenario: "sens_out".to_string(),
            severity: Severity::Critical,
            message: "Sensitivity assertion failed".to_string(),
            component: None,
            net: None,
            endpoints: None,
            measured: BTreeMap::from([
                ("assertion".to_string(), json!("r1_magnitude_ceiling")),
                ("parameter".to_string(), json!("R1")),
                ("frequency_hz".to_string(), json!(100.0)),
                ("metric".to_string(), json!("sensitivity_magnitude")),
                (
                    "sensitivity_summary".to_string(),
                    json!("out/sensitivity_summary.csv"),
                ),
            ]),
            limit: BTreeMap::from([("below_threshold".to_string(), json!(0.001))]),
            suggested_fixes: Vec::new(),
        }
    }
}
