use super::CircuitCiApp;
use super::analog::{AnalogScenarioChoice, analog_scenario_choices};
use super::analog_measure_assertions::{
    AnalogMeasureAssertionDraft, append_analog_measure_assertion,
    measure_assertion_measurement_choices, unique_analog_measure_assertion_name,
};
use super::simulation_forms::{analog_scenario_combo, string_combo};
use crate::reports::Finding;
use eframe::egui;
use serde_json::Value;

impl CircuitCiApp {
    pub(super) fn measure_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices
                .into_iter()
                .filter(|scenario| scenario.scenario_type == "analog_measure")
                .collect::<Vec<_>>(),
            Err(error) => {
                ui.collapsing("Measure Check", |ui| {
                    ui.label(format!("Measure run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Measure Check", |ui| {
            if choices.is_empty() {
                ui.label("No measure run setup is available.");
                return;
            }
            initialize_measure_assertion_defaults(&choices, self);
            let measurement_choices = measure_assertion_measurement_choices(
                &self.project_yaml,
                &self.analog_measure_assertion_scenario,
            )
            .unwrap_or_default();
            if self.analog_measure_assertion_measurement.trim().is_empty()
                && let Some(measurement) = measurement_choices.first()
            {
                self.analog_measure_assertion_measurement = measurement.clone();
            }
            egui::Grid::new("measure_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "measure_assertion_scenario",
                        &mut self.analog_measure_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_measure_assertion_name);
                    ui.end_row();

                    ui.label("Measurement");
                    if measurement_choices.is_empty() {
                        ui.text_edit_singleline(&mut self.analog_measure_assertion_measurement);
                    } else {
                        measurement_combo(
                            ui,
                            &mut self.analog_measure_assertion_measurement,
                            &measurement_choices,
                        );
                    }
                    ui.end_row();

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "measure_assertion_relation",
                        &mut self.analog_measure_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_measure_assertion_threshold)
                            .speed(0.001)
                            .range(-1.0e12..=1.0e12),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Below").clicked() {
                    self.apply_measure_preset("below", 0.4);
                }
                if ui.button("Above").clicked() {
                    self.apply_measure_preset("above", 0.1);
                }
                if ui.button("Add Measure Check").clicked() {
                    self.apply_add_measure_assertion();
                }
            });
        });
    }

    pub(super) fn measure_assertion_failure_actions(
        &mut self,
        ui: &mut egui::Ui,
        failures: &[Finding],
    ) {
        let rows = measure_failure_rows(failures);
        ui.label("Measure assertion failures");
        if rows.is_empty() {
            ui.label("No measure assertion failures were emitted.");
            return;
        }
        egui::Grid::new("measure_assertion_failure_actions")
            .num_columns(5)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Scenario");
                ui.strong("Check");
                ui.strong("Measurement");
                ui.strong("Limit");
                ui.strong("Actions");
                ui.end_row();
                for row in rows {
                    ui.monospace(&row.scenario);
                    ui.monospace(&row.assertion);
                    ui.monospace(&row.measurement);
                    ui.label(format!(
                        "{} {:.6e}",
                        row.relation.as_deref().unwrap_or("limit"),
                        row.threshold.unwrap_or_default()
                    ));
                    if ui.button("Open Measure Check").clicked() {
                        self.load_measure_failure_row(&row);
                    }
                    ui.end_row();
                }
            });
    }

    fn apply_measure_preset(&mut self, relation: &str, threshold: f64) {
        self.analog_measure_assertion_relation = relation.to_string();
        self.analog_measure_assertion_threshold = threshold;
        if self.analog_measure_assertion_name.trim().is_empty() {
            self.analog_measure_assertion_name =
                default_measure_assertion_name(&self.analog_measure_assertion_measurement);
        }
    }

    fn apply_add_measure_assertion(&mut self) {
        let requested_name = if self.analog_measure_assertion_name.trim().is_empty() {
            default_measure_assertion_name(&self.analog_measure_assertion_measurement)
        } else {
            self.analog_measure_assertion_name.trim().to_string()
        };
        let assertion_name = match unique_analog_measure_assertion_name(
            &self.project_yaml,
            &self.analog_measure_assertion_scenario,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogMeasureAssertionDraft {
            scenario_name: self.analog_measure_assertion_scenario.clone(),
            assertion_name: assertion_name.clone(),
            measurement: self.analog_measure_assertion_measurement.clone(),
            relation: self.analog_measure_assertion_relation.clone(),
            threshold: self.analog_measure_assertion_threshold,
        };
        match append_analog_measure_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_measure_assertion_name = assertion_name.clone();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Measure check {} added.", assertion_name.trim()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn load_measure_failure_row(&mut self, row: &MeasureFailureRow) {
        self.analog_measure_assertion_scenario = row.scenario.clone();
        self.analog_measure_assertion_name = row.assertion.clone();
        self.analog_measure_assertion_measurement = row.measurement.clone();
        if let Some(relation) = &row.relation {
            self.analog_measure_assertion_relation = relation.clone();
        }
        if let Some(threshold) = row.threshold {
            self.analog_measure_assertion_threshold = threshold;
        }
        self.status = format!("Loaded measure check {} from latest report.", row.assertion);
    }
}

#[derive(Debug, Clone)]
struct MeasureFailureRow {
    scenario: String,
    assertion: String,
    measurement: String,
    relation: Option<String>,
    threshold: Option<f64>,
}

fn measure_failure_rows(failures: &[Finding]) -> Vec<MeasureFailureRow> {
    failures
        .iter()
        .filter(|finding| finding.id == "SPICE_MEASURE_ANALYSIS")
        .filter_map(measure_failure_row)
        .collect()
}

fn measure_failure_row(finding: &Finding) -> Option<MeasureFailureRow> {
    let assertion = text_field(&finding.measured, "assertion")?;
    let measurement = text_field(&finding.measured, "measurement")?;
    let (relation, threshold) = threshold_limit(&finding.limit);
    Some(MeasureFailureRow {
        scenario: finding.scenario.clone(),
        assertion,
        measurement,
        relation,
        threshold,
    })
}

fn initialize_measure_assertion_defaults(choices: &[AnalogScenarioChoice], app: &mut CircuitCiApp) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == app.analog_measure_assertion_scenario);
    if (app.analog_measure_assertion_scenario.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        app.analog_measure_assertion_scenario = scenario.name.clone();
    }
    if app.analog_measure_assertion_measurement.trim().is_empty() {
        app.analog_measure_assertion_measurement = "avg_out".to_string();
    }
    if !matches!(
        app.analog_measure_assertion_relation.as_str(),
        "above" | "below"
    ) {
        app.analog_measure_assertion_relation = "below".to_string();
    }
    if app.analog_measure_assertion_name.trim().is_empty() {
        app.analog_measure_assertion_name =
            default_measure_assertion_name(&app.analog_measure_assertion_measurement);
    }
    if !app.analog_measure_assertion_threshold.is_finite() {
        app.analog_measure_assertion_threshold = 0.4;
    }
}

fn measurement_combo(ui: &mut egui::Ui, selected: &mut String, choices: &[String]) {
    let selected_text = if selected.trim().is_empty() {
        choices.first().map(String::as_str).unwrap_or("measurement")
    } else {
        selected.as_str()
    };
    egui::ComboBox::from_id_salt("measure_assertion_measurement")
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for choice in choices {
                ui.selectable_value(selected, choice.clone(), choice);
            }
        });
}

fn default_measure_assertion_name(measurement: &str) -> String {
    let measurement = measurement.trim();
    if measurement.is_empty() {
        return "measure_below_limit".to_string();
    }
    format!("{measurement}_below_limit")
}

fn text_field(map: &std::collections::BTreeMap<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

fn threshold_limit(
    map: &std::collections::BTreeMap<String, Value>,
) -> (Option<String>, Option<f64>) {
    for (key, relation) in [("above_threshold", "above"), ("below_threshold", "below")] {
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
    fn measure_failure_loads_editor_state() {
        let mut app = CircuitCiApp::default();
        let row = measure_failure_row(&measure_failure()).unwrap();

        app.load_measure_failure_row(&row);

        assert_eq!(app.analog_measure_assertion_scenario, "measure_out");
        assert_eq!(app.analog_measure_assertion_name, "avg_out_below_limit");
        assert_eq!(app.analog_measure_assertion_measurement, "avg_out");
        assert_eq!(app.analog_measure_assertion_relation, "below");
        assert_eq!(app.analog_measure_assertion_threshold, 0.4);
        assert_eq!(
            app.status,
            "Loaded measure check avg_out_below_limit from latest report."
        );
    }

    fn measure_failure() -> Finding {
        Finding {
            id: "SPICE_MEASURE_ANALYSIS".to_string(),
            scenario: "measure_out".to_string(),
            severity: Severity::Critical,
            message: "Measure assertion failed".to_string(),
            component: None,
            net: None,
            endpoints: None,
            measured: BTreeMap::from([
                ("assertion".to_string(), json!("avg_out_below_limit")),
                ("measurement".to_string(), json!("avg_out")),
                ("value".to_string(), json!(0.51)),
                ("unit".to_string(), json!("V")),
                (
                    "measure_summary".to_string(),
                    json!("out/measure_summary.csv"),
                ),
            ]),
            limit: BTreeMap::from([("below_threshold".to_string(), json!(0.4))]),
            suggested_fixes: Vec::new(),
        }
    }
}
