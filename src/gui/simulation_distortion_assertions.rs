use super::CircuitCiApp;
use super::analog::{AnalogScenarioChoice, analog_scenario_choices};
use super::analog_distortion_assertions::{
    AnalogDistortionAssertionDraft, append_analog_distortion_assertion,
    unique_analog_distortion_assertion_name,
};
use super::simulation_forms::{analog_scenario_combo, string_combo};
use crate::reports::Finding;
use eframe::egui;
use serde_json::Value;

impl CircuitCiApp {
    pub(super) fn distortion_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices
                .into_iter()
                .filter(|scenario| scenario.scenario_type == "analog_distortion")
                .collect::<Vec<_>>(),
            Err(error) => {
                ui.collapsing("Distortion Check", |ui| {
                    ui.label(format!("Distortion run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Distortion Check", |ui| {
            if choices.is_empty() {
                ui.label("No distortion run setup is available.");
                return;
            }
            initialize_distortion_assertion_defaults(&choices, self);
            egui::Grid::new("distortion_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "distortion_assertion_scenario",
                        &mut self.analog_distortion_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_distortion_assertion_name);
                    ui.end_row();

                    ui.label("Component");
                    ui.text_edit_singleline(&mut self.analog_distortion_assertion_component);
                    ui.end_row();

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "distortion_assertion_relation",
                        &mut self.analog_distortion_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_distortion_assertion_threshold)
                            .speed(0.0001)
                            .range(0.0..=1.0e12)
                            .suffix(" ratio"),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("IM Sum").clicked() {
                    self.apply_distortion_preset("im_f1_plus_f2", "im_sum_below_limit", 0.001);
                }
                if ui.button("H2").clicked() {
                    self.apply_distortion_preset("h2", "h2_below_limit", 0.001);
                }
                if ui.button("H3").clicked() {
                    self.apply_distortion_preset("h3", "h3_below_limit", 0.001);
                }
                if ui.button("Add Distortion Check").clicked() {
                    self.apply_add_distortion_assertion();
                }
            });
        });
    }

    pub(super) fn distortion_assertion_failure_actions(
        &mut self,
        ui: &mut egui::Ui,
        failures: &[Finding],
    ) {
        let rows = distortion_failure_rows(failures);
        ui.label("Distortion assertion failures");
        if rows.is_empty() {
            ui.label("No distortion assertion failures were emitted.");
            return;
        }
        egui::Grid::new("distortion_assertion_failure_actions")
            .num_columns(5)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Scenario");
                ui.strong("Check");
                ui.strong("Component");
                ui.strong("Limit");
                ui.strong("Actions");
                ui.end_row();
                for row in rows {
                    ui.monospace(&row.scenario);
                    ui.monospace(&row.assertion);
                    ui.monospace(&row.component);
                    ui.label(format!(
                        "{} {:.6e}",
                        row.relation.as_deref().unwrap_or("required"),
                        row.threshold.unwrap_or_default()
                    ));
                    if ui.button("Open Distortion Check").clicked() {
                        self.load_distortion_failure_row(&row);
                    }
                    ui.end_row();
                }
            });
    }

    fn apply_distortion_preset(&mut self, component: &str, name: &str, threshold: f64) {
        self.analog_distortion_assertion_component = component.to_string();
        self.analog_distortion_assertion_name = name.to_string();
        self.analog_distortion_assertion_relation = "below".to_string();
        self.analog_distortion_assertion_threshold = threshold;
    }

    fn apply_add_distortion_assertion(&mut self) {
        let requested_name = if self.analog_distortion_assertion_name.trim().is_empty() {
            default_distortion_assertion_name(&self.analog_distortion_assertion_component)
                .to_string()
        } else {
            self.analog_distortion_assertion_name.trim().to_string()
        };
        let assertion_name = match unique_analog_distortion_assertion_name(
            &self.project_yaml,
            &self.analog_distortion_assertion_scenario,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogDistortionAssertionDraft {
            scenario_name: self.analog_distortion_assertion_scenario.clone(),
            assertion_name: assertion_name.clone(),
            component: self.analog_distortion_assertion_component.clone(),
            relation: self.analog_distortion_assertion_relation.clone(),
            threshold: self.analog_distortion_assertion_threshold,
        };
        match append_analog_distortion_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_distortion_assertion_name = assertion_name.clone();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Distortion check {} added.", assertion_name.trim()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn load_distortion_failure_row(&mut self, row: &DistortionFailureRow) {
        self.analog_distortion_assertion_scenario = row.scenario.clone();
        self.analog_distortion_assertion_name = row.assertion.clone();
        self.analog_distortion_assertion_component = row.component.clone();
        if let Some(relation) = &row.relation {
            self.analog_distortion_assertion_relation = relation.clone();
        }
        if let Some(threshold) = row.threshold {
            self.analog_distortion_assertion_threshold = threshold;
        }
        self.status = format!(
            "Loaded distortion check {} from latest report.",
            row.assertion
        );
    }
}

#[derive(Debug, Clone)]
struct DistortionFailureRow {
    scenario: String,
    assertion: String,
    component: String,
    relation: Option<String>,
    threshold: Option<f64>,
}

fn distortion_failure_rows(failures: &[Finding]) -> Vec<DistortionFailureRow> {
    failures
        .iter()
        .filter(|finding| finding.id == "SPICE_DISTORTION_ANALYSIS")
        .filter_map(distortion_failure_row)
        .collect()
}

fn distortion_failure_row(finding: &Finding) -> Option<DistortionFailureRow> {
    let assertion = text_field(&finding.measured, "assertion")?;
    let component = text_field(&finding.measured, "component")
        .or_else(|| text_field(&finding.limit, "required_component"))?;
    let (relation, threshold) = threshold_limit(&finding.limit);
    Some(DistortionFailureRow {
        scenario: finding.scenario.clone(),
        assertion,
        component,
        relation,
        threshold,
    })
}

fn initialize_distortion_assertion_defaults(
    choices: &[AnalogScenarioChoice],
    app: &mut CircuitCiApp,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == app.analog_distortion_assertion_scenario);
    if (app.analog_distortion_assertion_scenario.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        app.analog_distortion_assertion_scenario = scenario.name.clone();
    }
    if app.analog_distortion_assertion_component.trim().is_empty() {
        app.analog_distortion_assertion_component = "im_f1_plus_f2".to_string();
    }
    if !matches!(
        app.analog_distortion_assertion_relation.as_str(),
        "above" | "below"
    ) {
        app.analog_distortion_assertion_relation = "below".to_string();
    }
    if app.analog_distortion_assertion_name.trim().is_empty() {
        app.analog_distortion_assertion_name =
            default_distortion_assertion_name(&app.analog_distortion_assertion_component)
                .to_string();
    }
    if !app.analog_distortion_assertion_threshold.is_finite()
        || app.analog_distortion_assertion_threshold < 0.0
    {
        app.analog_distortion_assertion_threshold = 0.001;
    }
}

fn default_distortion_assertion_name(component: &str) -> String {
    let component = component.trim();
    if component.is_empty() {
        return "distortion_below_limit".to_string();
    }
    format!("{component}_below_limit")
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
    fn distortion_failure_loads_editor_state() {
        let mut app = CircuitCiApp::default();
        let row = distortion_failure_row(&distortion_failure()).unwrap();

        app.load_distortion_failure_row(&row);

        assert_eq!(app.analog_distortion_assertion_scenario, "disto_out");
        assert_eq!(app.analog_distortion_assertion_name, "im_sum_below_limit");
        assert_eq!(app.analog_distortion_assertion_component, "im_f1_plus_f2");
        assert_eq!(app.analog_distortion_assertion_relation, "below");
        assert_eq!(app.analog_distortion_assertion_threshold, 0.001);
        assert_eq!(
            app.status,
            "Loaded distortion check im_sum_below_limit from latest report."
        );
    }

    #[test]
    fn missing_component_failure_loads_required_component() {
        let mut app = CircuitCiApp::default();
        let row = distortion_failure_row(&missing_component_failure()).unwrap();

        app.load_distortion_failure_row(&row);

        assert_eq!(app.analog_distortion_assertion_component, "h3");
        assert_eq!(
            app.analog_distortion_assertion_name,
            "no_such_component_limit"
        );
        assert_eq!(app.analog_distortion_assertion_relation, "below");
        assert_eq!(app.analog_distortion_assertion_threshold, 0.001);
    }

    fn distortion_failure() -> Finding {
        Finding {
            id: "SPICE_DISTORTION_ANALYSIS".to_string(),
            scenario: "disto_out".to_string(),
            severity: Severity::Critical,
            message: "Distortion assertion failed".to_string(),
            component: None,
            net: None,
            endpoints: None,
            measured: BTreeMap::from([
                ("assertion".to_string(), json!("im_sum_below_limit")),
                ("component".to_string(), json!("im_f1_plus_f2")),
                ("max_magnitude".to_string(), json!(0.004031128874149)),
                ("frequency_hz_at_max".to_string(), json!(1900.0)),
                ("unit".to_string(), json!("ratio")),
                (
                    "distortion_summary".to_string(),
                    json!("out/distortion_summary.csv"),
                ),
            ]),
            limit: BTreeMap::from([("below_threshold".to_string(), json!(0.001))]),
            suggested_fixes: Vec::new(),
        }
    }

    fn missing_component_failure() -> Finding {
        Finding {
            id: "SPICE_DISTORTION_ANALYSIS".to_string(),
            scenario: "disto_out".to_string(),
            severity: Severity::Critical,
            message: "Distortion assertion failed".to_string(),
            component: None,
            net: None,
            endpoints: None,
            measured: BTreeMap::from([
                ("assertion".to_string(), json!("no_such_component_limit")),
                ("component".to_string(), json!("h3")),
                (
                    "distortion_summary".to_string(),
                    json!("out/distortion_summary.csv"),
                ),
            ]),
            limit: BTreeMap::from([
                ("required_component".to_string(), json!("h3")),
                ("below_threshold".to_string(), json!(0.001)),
            ]),
            suggested_fixes: Vec::new(),
        }
    }
}
