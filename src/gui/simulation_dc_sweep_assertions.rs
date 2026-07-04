use super::CircuitCiApp;
use super::analog::{AnalogScenarioChoice, analog_scenario_choices};
use super::analog_dc_sweep_assertions::{
    AnalogDcSweepAssertionDraft, append_analog_dc_sweep_assertion,
    unique_analog_dc_sweep_assertion_name,
};
use super::simulation_forms::{analog_probe_combo, analog_scenario_combo, string_combo};
use crate::reports::Finding;
use eframe::egui;
use serde_json::Value;

impl CircuitCiApp {
    pub(super) fn dc_sweep_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices
                .into_iter()
                .filter(|scenario| scenario.scenario_type == "analog_dc_sweep")
                .collect::<Vec<_>>(),
            Err(error) => {
                ui.collapsing("DC Sweep Check", |ui| {
                    ui.label(format!("DC sweep run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("DC Sweep Check", |ui| {
            if choices.is_empty() {
                ui.label("No DC sweep run setup is available.");
                return;
            }
            initialize_dc_sweep_assertion_defaults(&choices, self);
            let selected_scenario = choices
                .iter()
                .find(|scenario| scenario.name == self.analog_dc_sweep_assertion_scenario);
            if self.analog_dc_sweep_assertion_probe.trim().is_empty()
                && let Some(probe) = selected_scenario.and_then(|scenario| scenario.probes.first())
            {
                self.analog_dc_sweep_assertion_probe = probe.name.clone();
            }
            egui::Grid::new("dc_sweep_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "dc_sweep_assertion_scenario",
                        &mut self.analog_dc_sweep_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_dc_sweep_assertion_name);
                    ui.end_row();

                    ui.label("Probe");
                    analog_probe_combo(
                        ui,
                        "dc_sweep_assertion_probe",
                        &mut self.analog_dc_sweep_assertion_probe,
                        selected_scenario,
                    );
                    ui.end_row();

                    ui.label("Aggregation");
                    string_combo(
                        ui,
                        "dc_sweep_assertion_aggregation",
                        &mut self.analog_dc_sweep_assertion_aggregation,
                        &["min", "max", "mean", "sample"],
                    );
                    ui.end_row();

                    if self.analog_dc_sweep_assertion_aggregation == "sample" {
                        ui.label("Sweep value");
                        ui.add(
                            egui::DragValue::new(
                                &mut self.analog_dc_sweep_assertion_at_sweep_value,
                            )
                            .speed(0.001)
                            .range(-1.0e12..=1.0e12),
                        );
                        ui.end_row();
                    }

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "dc_sweep_assertion_relation",
                        &mut self.analog_dc_sweep_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_dc_sweep_assertion_threshold)
                            .speed(0.001)
                            .range(-1.0e12..=1.0e12),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Max Ceiling").clicked() {
                    self.apply_dc_sweep_preset("max", "below", 0.4);
                }
                if ui.button("Min Floor").clicked() {
                    self.apply_dc_sweep_preset("min", "above", 0.1);
                }
                if ui.button("Sample Ceiling").clicked() {
                    self.apply_dc_sweep_preset("sample", "below", 0.4);
                }
                if ui.button("Add DC Sweep Check").clicked() {
                    self.apply_add_dc_sweep_assertion();
                }
            });
        });
    }

    pub(super) fn dc_sweep_assertion_failure_actions(
        &mut self,
        ui: &mut egui::Ui,
        failures: &[Finding],
    ) {
        let rows = dc_sweep_failure_rows(failures);
        ui.label("DC sweep assertion failures");
        if rows.is_empty() {
            ui.label("No DC sweep assertion failures were emitted.");
            return;
        }
        egui::Grid::new("dc_sweep_assertion_failure_actions")
            .num_columns(6)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Scenario");
                ui.strong("Check");
                ui.strong("Probe");
                ui.strong("Aggregation");
                ui.strong("Limit");
                ui.strong("Actions");
                ui.end_row();
                for row in rows {
                    ui.monospace(&row.scenario);
                    ui.monospace(&row.assertion);
                    ui.monospace(&row.probe);
                    ui.monospace(&row.aggregation);
                    ui.label(format!(
                        "{} {:.6e}",
                        row.relation.as_deref().unwrap_or("limit"),
                        row.threshold.unwrap_or_default()
                    ));
                    if ui.button("Open DC Sweep Check").clicked() {
                        self.load_dc_sweep_failure_row(&row);
                    }
                    ui.end_row();
                }
            });
    }

    fn apply_dc_sweep_preset(&mut self, aggregation: &str, relation: &str, threshold: f64) {
        self.analog_dc_sweep_assertion_aggregation = aggregation.to_string();
        self.analog_dc_sweep_assertion_relation = relation.to_string();
        self.analog_dc_sweep_assertion_threshold = threshold;
        if self.analog_dc_sweep_assertion_name.trim().is_empty()
            || self
                .analog_dc_sweep_assertion_name
                .trim()
                .ends_with("_limit")
        {
            self.analog_dc_sweep_assertion_name = default_dc_sweep_assertion_name(
                &self.analog_dc_sweep_assertion_probe,
                aggregation,
                relation,
            );
        }
    }

    fn apply_add_dc_sweep_assertion(&mut self) {
        let requested_name = if self.analog_dc_sweep_assertion_name.trim().is_empty() {
            default_dc_sweep_assertion_name(
                &self.analog_dc_sweep_assertion_probe,
                &self.analog_dc_sweep_assertion_aggregation,
                &self.analog_dc_sweep_assertion_relation,
            )
        } else {
            self.analog_dc_sweep_assertion_name.trim().to_string()
        };
        let assertion_name = match unique_analog_dc_sweep_assertion_name(
            &self.project_yaml,
            &self.analog_dc_sweep_assertion_scenario,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogDcSweepAssertionDraft {
            scenario_name: self.analog_dc_sweep_assertion_scenario.clone(),
            assertion_name: assertion_name.clone(),
            probe: self.analog_dc_sweep_assertion_probe.clone(),
            aggregation: self.analog_dc_sweep_assertion_aggregation.clone(),
            relation: self.analog_dc_sweep_assertion_relation.clone(),
            threshold: self.analog_dc_sweep_assertion_threshold,
            at_sweep_value: (self.analog_dc_sweep_assertion_aggregation == "sample")
                .then_some(self.analog_dc_sweep_assertion_at_sweep_value),
        };
        match append_analog_dc_sweep_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_dc_sweep_assertion_name = assertion_name.clone();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("DC sweep check {} added.", assertion_name.trim()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn load_dc_sweep_failure_row(&mut self, row: &DcSweepFailureRow) {
        self.analog_dc_sweep_assertion_scenario = row.scenario.clone();
        self.analog_dc_sweep_assertion_name = row.assertion.clone();
        self.analog_dc_sweep_assertion_probe = row.probe.clone();
        self.analog_dc_sweep_assertion_aggregation = row.aggregation.clone();
        if let Some(relation) = &row.relation {
            self.analog_dc_sweep_assertion_relation = relation.clone();
        }
        if let Some(threshold) = row.threshold {
            self.analog_dc_sweep_assertion_threshold = threshold;
        }
        if let Some(at_sweep_value) = row.at_sweep_value.or(row.sweep_value) {
            self.analog_dc_sweep_assertion_at_sweep_value = at_sweep_value;
        }
        self.status = format!(
            "Loaded DC sweep check {} from latest report.",
            row.assertion
        );
    }
}

#[derive(Debug, Clone)]
struct DcSweepFailureRow {
    scenario: String,
    assertion: String,
    probe: String,
    aggregation: String,
    relation: Option<String>,
    threshold: Option<f64>,
    sweep_value: Option<f64>,
    at_sweep_value: Option<f64>,
}

fn dc_sweep_failure_rows(failures: &[Finding]) -> Vec<DcSweepFailureRow> {
    failures
        .iter()
        .filter(|finding| finding.id == "SPICE_DC_SWEEP_ANALYSIS")
        .filter_map(dc_sweep_failure_row)
        .collect()
}

fn dc_sweep_failure_row(finding: &Finding) -> Option<DcSweepFailureRow> {
    let assertion = text_field(&finding.measured, "assertion")?;
    let probe = text_field(&finding.measured, "probe")?;
    let aggregation =
        text_field(&finding.measured, "aggregation").unwrap_or_else(|| "sample".to_string());
    let (relation, threshold) = threshold_limit(&finding.limit);
    Some(DcSweepFailureRow {
        scenario: finding.scenario.clone(),
        assertion,
        probe,
        aggregation,
        relation,
        threshold,
        sweep_value: number_field(&finding.measured, "sweep_value"),
        at_sweep_value: number_field(&finding.measured, "at_sweep_value"),
    })
}

fn initialize_dc_sweep_assertion_defaults(
    choices: &[AnalogScenarioChoice],
    app: &mut CircuitCiApp,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == app.analog_dc_sweep_assertion_scenario);
    if (app.analog_dc_sweep_assertion_scenario.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        app.analog_dc_sweep_assertion_scenario = scenario.name.clone();
        if let Some(probe) = scenario.probes.first() {
            app.analog_dc_sweep_assertion_probe = probe.name.clone();
        }
    }
    if app.analog_dc_sweep_assertion_probe.trim().is_empty() {
        app.analog_dc_sweep_assertion_probe = "out_voltage".to_string();
    }
    if !matches!(
        app.analog_dc_sweep_assertion_aggregation.as_str(),
        "min" | "max" | "mean" | "sample"
    ) {
        app.analog_dc_sweep_assertion_aggregation = "max".to_string();
    }
    if !matches!(
        app.analog_dc_sweep_assertion_relation.as_str(),
        "above" | "below"
    ) {
        app.analog_dc_sweep_assertion_relation = "below".to_string();
    }
    if app.analog_dc_sweep_assertion_name.trim().is_empty() {
        app.analog_dc_sweep_assertion_name = default_dc_sweep_assertion_name(
            &app.analog_dc_sweep_assertion_probe,
            &app.analog_dc_sweep_assertion_aggregation,
            &app.analog_dc_sweep_assertion_relation,
        );
    }
    if !app.analog_dc_sweep_assertion_threshold.is_finite() {
        app.analog_dc_sweep_assertion_threshold = 0.4;
    }
    if !app.analog_dc_sweep_assertion_at_sweep_value.is_finite() {
        app.analog_dc_sweep_assertion_at_sweep_value = 1.0;
    }
}

fn default_dc_sweep_assertion_name(probe: &str, aggregation: &str, relation: &str) -> String {
    let probe = probe.trim();
    let probe = if probe.is_empty() { "probe" } else { probe };
    format!("{probe}_{aggregation}_{relation}_limit")
}

fn text_field(map: &std::collections::BTreeMap<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

fn number_field(map: &std::collections::BTreeMap<String, Value>, key: &str) -> Option<f64> {
    map.get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
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
    fn dc_sweep_failure_loads_editor_state() {
        let mut app = CircuitCiApp::default();
        let row = dc_sweep_failure_row(&dc_sweep_failure()).unwrap();

        app.load_dc_sweep_failure_row(&row);

        assert_eq!(app.analog_dc_sweep_assertion_scenario, "sweep_out");
        assert_eq!(app.analog_dc_sweep_assertion_name, "out_sample_below_limit");
        assert_eq!(app.analog_dc_sweep_assertion_probe, "out_voltage");
        assert_eq!(app.analog_dc_sweep_assertion_aggregation, "sample");
        assert_eq!(app.analog_dc_sweep_assertion_relation, "below");
        assert_eq!(app.analog_dc_sweep_assertion_threshold, 0.4);
        assert_eq!(app.analog_dc_sweep_assertion_at_sweep_value, 1.0);
        assert_eq!(
            app.status,
            "Loaded DC sweep check out_sample_below_limit from latest report."
        );
    }

    #[test]
    fn legacy_dc_sweep_failure_defaults_to_sample_at_measured_sweep_value() {
        let mut app = CircuitCiApp::default();
        let mut finding = dc_sweep_failure();
        finding.measured.remove("aggregation");
        finding.measured.remove("at_sweep_value");
        let row = dc_sweep_failure_row(&finding).unwrap();

        app.load_dc_sweep_failure_row(&row);

        assert_eq!(app.analog_dc_sweep_assertion_aggregation, "sample");
        assert_eq!(app.analog_dc_sweep_assertion_at_sweep_value, 0.5);
    }

    fn dc_sweep_failure() -> Finding {
        Finding {
            id: "SPICE_DC_SWEEP_ANALYSIS".to_string(),
            scenario: "sweep_out".to_string(),
            severity: Severity::Critical,
            message: "DC sweep assertion failed".to_string(),
            component: None,
            net: None,
            endpoints: None,
            measured: BTreeMap::from([
                ("assertion".to_string(), json!("out_sample_below_limit")),
                ("probe".to_string(), json!("out_voltage")),
                ("aggregation".to_string(), json!("sample")),
                ("value".to_string(), json!(0.51)),
                ("sweep_value".to_string(), json!(0.5)),
                ("at_sweep_value".to_string(), json!(1.0)),
                ("unit".to_string(), json!("V")),
                ("dc_sweep".to_string(), json!("out/dc_sweep.csv")),
            ]),
            limit: BTreeMap::from([("below_threshold".to_string(), json!(0.4))]),
            suggested_fixes: Vec::new(),
        }
    }
}
