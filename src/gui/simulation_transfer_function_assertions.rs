use super::CircuitCiApp;
use super::analog::{AnalogScenarioChoice, analog_scenario_choices};
use super::analog_transfer_function_assertions::{
    AnalogTransferFunctionAssertionDraft, append_analog_transfer_function_assertion,
    unique_analog_transfer_function_assertion_name,
};
use super::simulation_forms::{analog_scenario_combo, string_combo};
use crate::reports::Finding;
use eframe::egui;
use serde_json::Value;

impl CircuitCiApp {
    pub(super) fn transfer_function_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices
                .into_iter()
                .filter(|scenario| scenario.scenario_type == "analog_transfer_function")
                .collect::<Vec<_>>(),
            Err(error) => {
                ui.collapsing("Transfer Function Check", |ui| {
                    ui.label(format!("Transfer-function run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Transfer Function Check", |ui| {
            if choices.is_empty() {
                ui.label("No transfer-function run setup is available.");
                return;
            }
            initialize_transfer_function_assertion_defaults(&choices, self);
            egui::Grid::new("transfer_function_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "transfer_function_assertion_scenario",
                        &mut self.analog_transfer_function_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_transfer_function_assertion_name);
                    ui.end_row();

                    ui.label("Metric");
                    let previous_metric = self.analog_transfer_function_assertion_metric.clone();
                    transfer_function_metric_combo(
                        ui,
                        &mut self.analog_transfer_function_assertion_metric,
                    );
                    if previous_metric != self.analog_transfer_function_assertion_metric {
                        if self
                            .analog_transfer_function_assertion_name
                            .trim()
                            .is_empty()
                            || self.analog_transfer_function_assertion_name
                                == default_transfer_function_assertion_name(&previous_metric)
                        {
                            self.analog_transfer_function_assertion_name =
                                default_transfer_function_assertion_name(
                                    &self.analog_transfer_function_assertion_metric,
                                )
                                .to_string();
                        }
                        self.analog_transfer_function_assertion_relation =
                            default_transfer_function_relation(
                                &self.analog_transfer_function_assertion_metric,
                            )
                            .to_string();
                        self.analog_transfer_function_assertion_threshold =
                            default_transfer_function_threshold(
                                &self.analog_transfer_function_assertion_metric,
                            );
                    }
                    ui.end_row();

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "transfer_function_assertion_relation",
                        &mut self.analog_transfer_function_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.analog_transfer_function_assertion_threshold,
                        )
                        .speed(transfer_function_threshold_speed(
                            &self.analog_transfer_function_assertion_metric,
                        ))
                        .range(-1.0e12..=1.0e12)
                        .suffix(transfer_function_threshold_suffix(
                            &self.analog_transfer_function_assertion_metric,
                        )),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Gain").clicked() {
                    self.apply_transfer_function_preset("transfer_function_gain");
                }
                if ui.button("Input R").clicked() {
                    self.apply_transfer_function_preset("input_resistance_ohm");
                }
                if ui.button("Output R").clicked() {
                    self.apply_transfer_function_preset("output_resistance_ohm");
                }
                if ui.button("Add TF Check").clicked() {
                    self.apply_add_transfer_function_assertion();
                }
            });
        });
    }

    pub(super) fn transfer_function_assertion_failure_actions(
        &mut self,
        ui: &mut egui::Ui,
        failures: &[Finding],
    ) {
        let rows = transfer_function_failure_rows(failures);
        ui.label("Transfer-function assertion failures");
        if rows.is_empty() {
            ui.label("No transfer-function assertion failures were emitted.");
            return;
        }
        egui::Grid::new("transfer_function_assertion_failure_actions")
            .num_columns(5)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Scenario");
                ui.strong("Check");
                ui.strong("Metric");
                ui.strong("Limit");
                ui.strong("Actions");
                ui.end_row();
                for row in rows {
                    ui.monospace(&row.scenario);
                    ui.monospace(&row.assertion);
                    ui.label(row.metric.as_str());
                    ui.label(format!(
                        "{} {:.6e}",
                        row.relation.as_deref().unwrap_or("limit"),
                        row.threshold.unwrap_or_default()
                    ));
                    if ui.button("Open TF Check").clicked() {
                        self.load_transfer_function_failure_row(&row);
                    }
                    ui.end_row();
                }
            });
    }

    fn apply_transfer_function_preset(&mut self, metric: &str) {
        self.analog_transfer_function_assertion_metric = metric.to_string();
        self.analog_transfer_function_assertion_name =
            default_transfer_function_assertion_name(metric).to_string();
        self.analog_transfer_function_assertion_relation =
            default_transfer_function_relation(metric).to_string();
        self.analog_transfer_function_assertion_threshold =
            default_transfer_function_threshold(metric);
    }

    fn apply_add_transfer_function_assertion(&mut self) {
        let requested_name = if self
            .analog_transfer_function_assertion_name
            .trim()
            .is_empty()
        {
            default_transfer_function_assertion_name(
                &self.analog_transfer_function_assertion_metric,
            )
            .to_string()
        } else {
            self.analog_transfer_function_assertion_name
                .trim()
                .to_string()
        };
        let assertion_name = match unique_analog_transfer_function_assertion_name(
            &self.project_yaml,
            &self.analog_transfer_function_assertion_scenario,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogTransferFunctionAssertionDraft {
            scenario_name: self.analog_transfer_function_assertion_scenario.clone(),
            assertion_name: assertion_name.clone(),
            metric: self.analog_transfer_function_assertion_metric.clone(),
            relation: self.analog_transfer_function_assertion_relation.clone(),
            threshold: self.analog_transfer_function_assertion_threshold,
        };
        match append_analog_transfer_function_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_transfer_function_assertion_name = assertion_name.clone();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Transfer-function check {} added.", assertion_name.trim()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn load_transfer_function_failure_row(&mut self, row: &TransferFunctionFailureRow) {
        self.analog_transfer_function_assertion_scenario = row.scenario.clone();
        self.analog_transfer_function_assertion_name = row.assertion.clone();
        self.analog_transfer_function_assertion_metric = row.metric.clone();
        if let Some(relation) = &row.relation {
            self.analog_transfer_function_assertion_relation = relation.clone();
        }
        if let Some(threshold) = row.threshold {
            self.analog_transfer_function_assertion_threshold = threshold;
        }
        self.status = format!(
            "Loaded transfer-function check {} from latest report.",
            row.assertion
        );
    }
}

#[derive(Debug, Clone)]
struct TransferFunctionFailureRow {
    scenario: String,
    assertion: String,
    metric: String,
    relation: Option<String>,
    threshold: Option<f64>,
}

fn transfer_function_failure_rows(failures: &[Finding]) -> Vec<TransferFunctionFailureRow> {
    failures
        .iter()
        .filter(|finding| finding.id == "SPICE_TRANSFER_FUNCTION_ANALYSIS")
        .filter_map(transfer_function_failure_row)
        .collect()
}

fn transfer_function_failure_row(finding: &Finding) -> Option<TransferFunctionFailureRow> {
    let assertion = text_field(&finding.measured, "assertion")?;
    let metric = text_field(&finding.measured, "metric")?;
    let (relation, threshold) = threshold_limit(&finding.limit);
    Some(TransferFunctionFailureRow {
        scenario: finding.scenario.clone(),
        assertion,
        metric,
        relation,
        threshold,
    })
}

fn initialize_transfer_function_assertion_defaults(
    choices: &[AnalogScenarioChoice],
    app: &mut CircuitCiApp,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == app.analog_transfer_function_assertion_scenario);
    if (app.analog_transfer_function_assertion_scenario.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        app.analog_transfer_function_assertion_scenario = scenario.name.clone();
    }
    if !transfer_function_metric_options()
        .iter()
        .any(|option| option.0 == app.analog_transfer_function_assertion_metric.as_str())
    {
        app.analog_transfer_function_assertion_metric = "transfer_function_gain".to_string();
    }
    if !matches!(
        app.analog_transfer_function_assertion_relation.as_str(),
        "above" | "below"
    ) {
        app.analog_transfer_function_assertion_relation =
            default_transfer_function_relation(&app.analog_transfer_function_assertion_metric)
                .to_string();
    }
    if app
        .analog_transfer_function_assertion_name
        .trim()
        .is_empty()
    {
        app.analog_transfer_function_assertion_name = default_transfer_function_assertion_name(
            &app.analog_transfer_function_assertion_metric,
        )
        .to_string();
    }
    if !app.analog_transfer_function_assertion_threshold.is_finite() {
        app.analog_transfer_function_assertion_threshold =
            default_transfer_function_threshold(&app.analog_transfer_function_assertion_metric);
    }
}

fn transfer_function_metric_combo(ui: &mut egui::Ui, selected: &mut String) {
    let selected_label = transfer_function_metric_options()
        .iter()
        .find(|option| option.0 == selected.as_str())
        .map(|option| option.1)
        .unwrap_or(selected.as_str());
    egui::ComboBox::from_id_salt("transfer_function_assertion_metric")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            for (value, label) in transfer_function_metric_options() {
                ui.selectable_value(selected, value.to_string(), *label);
            }
        });
}

fn transfer_function_metric_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("transfer_function_gain", "Transfer gain"),
        ("input_resistance_ohm", "Input resistance"),
        ("output_resistance_ohm", "Output resistance"),
    ]
}

fn default_transfer_function_assertion_name(metric: &str) -> &'static str {
    match metric {
        "input_resistance_ohm" => "input_resistance_floor",
        "output_resistance_ohm" => "output_resistance_ceiling",
        _ => "transfer_gain_floor",
    }
}

fn default_transfer_function_relation(metric: &str) -> &'static str {
    match metric {
        "input_resistance_ohm" | "transfer_function_gain" => "above",
        _ => "below",
    }
}

fn default_transfer_function_threshold(metric: &str) -> f64 {
    match metric {
        "input_resistance_ohm" => 1.0e3,
        "output_resistance_ohm" => 1.0e3,
        _ => 0.5,
    }
}

fn transfer_function_threshold_speed(metric: &str) -> f64 {
    match metric {
        "input_resistance_ohm" | "output_resistance_ohm" => 10.0,
        _ => 0.001,
    }
}

fn transfer_function_threshold_suffix(metric: &str) -> &'static str {
    match metric {
        "input_resistance_ohm" | "output_resistance_ohm" => " ohm",
        _ => " ratio",
    }
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
    fn transfer_function_failure_loads_editor_state() {
        let mut app = CircuitCiApp::default();
        let row = transfer_function_failure_row(&transfer_function_failure()).unwrap();

        app.load_transfer_function_failure_row(&row);

        assert_eq!(app.analog_transfer_function_assertion_scenario, "tf_out");
        assert_eq!(app.analog_transfer_function_assertion_name, "gain_floor");
        assert_eq!(
            app.analog_transfer_function_assertion_metric,
            "transfer_function_gain"
        );
        assert_eq!(app.analog_transfer_function_assertion_relation, "above");
        assert_eq!(app.analog_transfer_function_assertion_threshold, 0.75);
        assert_eq!(
            app.status,
            "Loaded transfer-function check gain_floor from latest report."
        );
    }

    fn transfer_function_failure() -> Finding {
        Finding {
            id: "SPICE_TRANSFER_FUNCTION_ANALYSIS".to_string(),
            scenario: "tf_out".to_string(),
            severity: Severity::Critical,
            message: "Transfer-function assertion failed".to_string(),
            component: None,
            net: None,
            endpoints: None,
            measured: BTreeMap::from([
                ("assertion".to_string(), json!("gain_floor")),
                ("metric".to_string(), json!("transfer_function_gain")),
                ("value".to_string(), json!(0.5)),
                ("unit".to_string(), json!("ratio")),
                (
                    "transfer_function_summary".to_string(),
                    json!("out/transfer_function_summary.csv"),
                ),
            ]),
            limit: BTreeMap::from([("above_threshold".to_string(), json!(0.75))]),
            suggested_fixes: Vec::new(),
        }
    }
}
