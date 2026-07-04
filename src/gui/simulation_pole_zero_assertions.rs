use super::CircuitCiApp;
use super::analog::{AnalogScenarioChoice, analog_scenario_choices};
use super::analog_pole_zero_assertions::{
    AnalogPoleZeroAssertionDraft, append_analog_pole_zero_assertion,
    unique_analog_pole_zero_assertion_name,
};
use super::simulation_forms::{analog_scenario_combo, string_combo};
use crate::reports::Finding;
use eframe::egui;
use serde_json::Value;

impl CircuitCiApp {
    pub(super) fn pole_zero_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices
                .into_iter()
                .filter(|scenario| scenario.scenario_type == "analog_pole_zero")
                .collect::<Vec<_>>(),
            Err(error) => {
                ui.collapsing("Pole-Zero Check", |ui| {
                    ui.label(format!("Pole-zero run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Pole-Zero Check", |ui| {
            if choices.is_empty() {
                ui.label("No pole-zero run setup is available.");
                return;
            }
            initialize_pole_zero_assertion_defaults(&choices, self);
            egui::Grid::new("pole_zero_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "pole_zero_assertion_scenario",
                        &mut self.analog_pole_zero_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_pole_zero_assertion_name);
                    ui.end_row();

                    ui.label("Root");
                    ui.horizontal(|ui| {
                        string_combo(
                            ui,
                            "pole_zero_assertion_root_kind",
                            &mut self.analog_pole_zero_assertion_root_kind,
                            &["pole", "zero"],
                        );
                        ui.checkbox(&mut self.analog_pole_zero_assertion_root_index_enabled, "");
                        ui.add_enabled(
                            self.analog_pole_zero_assertion_root_index_enabled,
                            egui::DragValue::new(&mut self.analog_pole_zero_assertion_root_index)
                                .range(1..=1024),
                        );
                    });
                    ui.end_row();

                    ui.label("Metric");
                    let previous_metric = self.analog_pole_zero_assertion_metric.clone();
                    pole_zero_metric_combo(ui, &mut self.analog_pole_zero_assertion_metric);
                    if previous_metric != self.analog_pole_zero_assertion_metric {
                        if self.analog_pole_zero_assertion_name.trim().is_empty()
                            || self.analog_pole_zero_assertion_name
                                == default_pole_zero_assertion_name(&previous_metric)
                        {
                            self.analog_pole_zero_assertion_name =
                                default_pole_zero_assertion_name(
                                    &self.analog_pole_zero_assertion_metric,
                                )
                                .to_string();
                        }
                        self.analog_pole_zero_assertion_relation =
                            default_pole_zero_relation(&self.analog_pole_zero_assertion_metric)
                                .to_string();
                        self.analog_pole_zero_assertion_threshold =
                            default_pole_zero_threshold(&self.analog_pole_zero_assertion_metric);
                    }
                    ui.end_row();

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "pole_zero_assertion_relation",
                        &mut self.analog_pole_zero_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_pole_zero_assertion_threshold)
                            .speed(pole_zero_threshold_speed(
                                &self.analog_pole_zero_assertion_metric,
                            ))
                            .range(-1.0e15..=1.0e15)
                            .suffix(pole_zero_threshold_suffix(
                                &self.analog_pole_zero_assertion_metric,
                            )),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Stable Pole").clicked() {
                    self.apply_pole_zero_preset("pole", "real_rad_per_s");
                }
                if ui.button("Zero Freq").clicked() {
                    self.apply_pole_zero_preset("zero", "frequency_hz");
                }
                if ui.button("Imag").clicked() {
                    self.apply_pole_zero_preset("pole", "imaginary_rad_per_s");
                }
                if ui.button("Add PZ Check").clicked() {
                    self.apply_add_pole_zero_assertion();
                }
            });
        });
    }

    pub(super) fn pole_zero_assertion_failure_actions(
        &mut self,
        ui: &mut egui::Ui,
        failures: &[Finding],
    ) {
        let rows = pole_zero_failure_rows(failures);
        ui.label("Pole-zero assertion failures");
        if rows.is_empty() {
            ui.label("No pole-zero assertion failures were emitted.");
            return;
        }
        egui::Grid::new("pole_zero_assertion_failure_actions")
            .num_columns(6)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Scenario");
                ui.strong("Check");
                ui.strong("Root");
                ui.strong("Metric");
                ui.strong("Limit");
                ui.strong("Actions");
                ui.end_row();
                for row in rows {
                    ui.monospace(&row.scenario);
                    ui.monospace(&row.assertion);
                    ui.monospace(match row.root_index {
                        Some(index) => format!("{} {}", row.root_kind, index),
                        None => row.root_kind.clone(),
                    });
                    ui.label(row.metric.as_str());
                    ui.label(format!(
                        "{} {:.6e}",
                        row.relation.as_deref().unwrap_or("limit"),
                        row.threshold.unwrap_or_default()
                    ));
                    if ui.button("Open PZ Check").clicked() {
                        self.load_pole_zero_failure_row(&row);
                    }
                    ui.end_row();
                }
            });
    }

    fn apply_pole_zero_preset(&mut self, root_kind: &str, metric: &str) {
        self.analog_pole_zero_assertion_root_kind = root_kind.to_string();
        self.analog_pole_zero_assertion_metric = metric.to_string();
        self.analog_pole_zero_assertion_root_index_enabled = true;
        self.analog_pole_zero_assertion_root_index = 1;
        self.analog_pole_zero_assertion_name = default_pole_zero_assertion_name(metric).to_string();
        self.analog_pole_zero_assertion_relation = default_pole_zero_relation(metric).to_string();
        self.analog_pole_zero_assertion_threshold = default_pole_zero_threshold(metric);
    }

    fn apply_add_pole_zero_assertion(&mut self) {
        let requested_name = if self.analog_pole_zero_assertion_name.trim().is_empty() {
            default_pole_zero_assertion_name(&self.analog_pole_zero_assertion_metric).to_string()
        } else {
            self.analog_pole_zero_assertion_name.trim().to_string()
        };
        let assertion_name = match unique_analog_pole_zero_assertion_name(
            &self.project_yaml,
            &self.analog_pole_zero_assertion_scenario,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogPoleZeroAssertionDraft {
            scenario_name: self.analog_pole_zero_assertion_scenario.clone(),
            assertion_name: assertion_name.clone(),
            root_kind: self.analog_pole_zero_assertion_root_kind.clone(),
            root_index: self
                .analog_pole_zero_assertion_root_index_enabled
                .then_some(self.analog_pole_zero_assertion_root_index),
            metric: self.analog_pole_zero_assertion_metric.clone(),
            relation: self.analog_pole_zero_assertion_relation.clone(),
            threshold: self.analog_pole_zero_assertion_threshold,
        };
        match append_analog_pole_zero_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_pole_zero_assertion_name = assertion_name.clone();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Pole-zero check {} added.", assertion_name.trim()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn load_pole_zero_failure_row(&mut self, row: &PoleZeroFailureRow) {
        self.analog_pole_zero_assertion_scenario = row.scenario.clone();
        self.analog_pole_zero_assertion_name = row.assertion.clone();
        self.analog_pole_zero_assertion_root_kind = row.root_kind.clone();
        self.analog_pole_zero_assertion_root_index_enabled = row.root_index.is_some();
        if let Some(root_index) = row.root_index {
            self.analog_pole_zero_assertion_root_index = root_index;
        }
        self.analog_pole_zero_assertion_metric = row.metric.clone();
        if let Some(relation) = &row.relation {
            self.analog_pole_zero_assertion_relation = relation.clone();
        }
        if let Some(threshold) = row.threshold {
            self.analog_pole_zero_assertion_threshold = threshold;
        }
        self.status = format!(
            "Loaded pole-zero check {} from latest report.",
            row.assertion
        );
    }
}

#[derive(Debug, Clone)]
struct PoleZeroFailureRow {
    scenario: String,
    assertion: String,
    root_kind: String,
    root_index: Option<u32>,
    metric: String,
    relation: Option<String>,
    threshold: Option<f64>,
}

fn pole_zero_failure_rows(failures: &[Finding]) -> Vec<PoleZeroFailureRow> {
    failures
        .iter()
        .filter(|finding| finding.id == "SPICE_POLE_ZERO_ANALYSIS")
        .filter_map(pole_zero_failure_row)
        .collect()
}

fn pole_zero_failure_row(finding: &Finding) -> Option<PoleZeroFailureRow> {
    let assertion = text_field(&finding.measured, "assertion")?;
    let root_kind = text_field(&finding.measured, "root_kind")?;
    let metric = text_field(&finding.measured, "metric")?;
    let root_index = optional_u32_field(&finding.measured, "root_index")
        .or_else(|| optional_u32_field(&finding.limit, "required_root_index"));
    let (relation, threshold) = threshold_limit(&finding.limit);
    Some(PoleZeroFailureRow {
        scenario: finding.scenario.clone(),
        assertion,
        root_kind,
        root_index,
        metric,
        relation,
        threshold,
    })
}

fn initialize_pole_zero_assertion_defaults(
    choices: &[AnalogScenarioChoice],
    app: &mut CircuitCiApp,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == app.analog_pole_zero_assertion_scenario);
    if (app.analog_pole_zero_assertion_scenario.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        app.analog_pole_zero_assertion_scenario = scenario.name.clone();
    }
    if !matches!(
        app.analog_pole_zero_assertion_root_kind.as_str(),
        "pole" | "zero"
    ) {
        app.analog_pole_zero_assertion_root_kind = "pole".to_string();
    }
    if app.analog_pole_zero_assertion_root_index == 0 {
        app.analog_pole_zero_assertion_root_index = 1;
    }
    if !pole_zero_metric_options()
        .iter()
        .any(|option| option.0 == app.analog_pole_zero_assertion_metric.as_str())
    {
        app.analog_pole_zero_assertion_metric = "real_rad_per_s".to_string();
    }
    if !matches!(
        app.analog_pole_zero_assertion_relation.as_str(),
        "above" | "below"
    ) {
        app.analog_pole_zero_assertion_relation =
            default_pole_zero_relation(&app.analog_pole_zero_assertion_metric).to_string();
    }
    if app.analog_pole_zero_assertion_name.trim().is_empty() {
        app.analog_pole_zero_assertion_name =
            default_pole_zero_assertion_name(&app.analog_pole_zero_assertion_metric).to_string();
    }
    if !app.analog_pole_zero_assertion_threshold.is_finite() {
        app.analog_pole_zero_assertion_threshold =
            default_pole_zero_threshold(&app.analog_pole_zero_assertion_metric);
    }
}

fn pole_zero_metric_combo(ui: &mut egui::Ui, selected: &mut String) {
    let selected_label = pole_zero_metric_options()
        .iter()
        .find(|option| option.0 == selected.as_str())
        .map(|option| option.1)
        .unwrap_or(selected.as_str());
    egui::ComboBox::from_id_salt("pole_zero_assertion_metric")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            for (value, label) in pole_zero_metric_options() {
                ui.selectable_value(selected, value.to_string(), *label);
            }
        });
}

fn pole_zero_metric_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("real_rad_per_s", "Real part"),
        ("imaginary_rad_per_s", "Imaginary part"),
        ("frequency_hz", "Frequency"),
    ]
}

fn default_pole_zero_assertion_name(metric: &str) -> &'static str {
    match metric {
        "frequency_hz" => "root_frequency_ceiling",
        "imaginary_rad_per_s" => "root_imaginary_ceiling",
        _ => "stable_pole_real_ceiling",
    }
}

fn default_pole_zero_relation(metric: &str) -> &'static str {
    match metric {
        "real_rad_per_s" => "below",
        _ => "below",
    }
}

fn default_pole_zero_threshold(metric: &str) -> f64 {
    match metric {
        "frequency_hz" => 1.0e6,
        "imaginary_rad_per_s" => 1.0e6,
        _ => -500.0,
    }
}

fn pole_zero_threshold_speed(metric: &str) -> f64 {
    match metric {
        "frequency_hz" => 100.0,
        _ => 1000.0,
    }
}

fn pole_zero_threshold_suffix(metric: &str) -> &'static str {
    match metric {
        "frequency_hz" => " Hz",
        _ => " rad/s",
    }
}

fn text_field(map: &std::collections::BTreeMap<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

fn optional_u32_field(map: &std::collections::BTreeMap<String, Value>, key: &str) -> Option<u32> {
    let value = map.get(key)?;
    if let Some(value) = value.as_u64() {
        return u32::try_from(value).ok();
    }
    let value = value.as_f64()?;
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 || value > f64::from(u32::MAX) {
        return None;
    }
    Some(value as u32)
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
    fn pole_zero_failure_loads_editor_state() {
        let mut app = CircuitCiApp::default();
        let row = pole_zero_failure_row(&pole_zero_failure()).unwrap();

        app.load_pole_zero_failure_row(&row);

        assert_eq!(app.analog_pole_zero_assertion_scenario, "pz_out");
        assert_eq!(app.analog_pole_zero_assertion_name, "pole_too_slow");
        assert_eq!(app.analog_pole_zero_assertion_root_kind, "pole");
        assert!(app.analog_pole_zero_assertion_root_index_enabled);
        assert_eq!(app.analog_pole_zero_assertion_root_index, 1);
        assert_eq!(app.analog_pole_zero_assertion_metric, "real_rad_per_s");
        assert_eq!(app.analog_pole_zero_assertion_relation, "below");
        assert_eq!(app.analog_pole_zero_assertion_threshold, -1500.0);
        assert_eq!(
            app.status,
            "Loaded pole-zero check pole_too_slow from latest report."
        );
    }

    #[test]
    fn pole_zero_missing_root_failure_loads_required_index() {
        let row = pole_zero_failure_row(&missing_root_failure()).unwrap();
        let mut app = CircuitCiApp::default();

        app.load_pole_zero_failure_row(&row);

        assert_eq!(app.analog_pole_zero_assertion_root_index, 2);
        assert_eq!(app.analog_pole_zero_assertion_metric, "frequency_hz");
    }

    fn pole_zero_failure() -> Finding {
        Finding {
            id: "SPICE_POLE_ZERO_ANALYSIS".to_string(),
            scenario: "pz_out".to_string(),
            severity: Severity::Critical,
            message: "Pole-zero assertion failed".to_string(),
            component: None,
            net: None,
            endpoints: None,
            measured: BTreeMap::from([
                ("assertion".to_string(), json!("pole_too_slow")),
                ("root_kind".to_string(), json!("pole")),
                ("root_index".to_string(), json!(1)),
                ("metric".to_string(), json!("real_rad_per_s")),
                ("value".to_string(), json!(-1000.0)),
                ("unit".to_string(), json!("rad/s")),
                (
                    "pole_zero_summary".to_string(),
                    json!("out/pole_zero_summary.csv"),
                ),
            ]),
            limit: BTreeMap::from([("below_threshold".to_string(), json!(-1500.0))]),
            suggested_fixes: Vec::new(),
        }
    }

    fn missing_root_failure() -> Finding {
        Finding {
            id: "SPICE_POLE_ZERO_ANALYSIS".to_string(),
            scenario: "pz_out".to_string(),
            severity: Severity::Critical,
            message: "Pole-zero assertion failed".to_string(),
            component: None,
            net: None,
            endpoints: None,
            measured: BTreeMap::from([
                ("assertion".to_string(), json!("second_pole_required")),
                ("root_kind".to_string(), json!("pole")),
                ("metric".to_string(), json!("frequency_hz")),
                (
                    "pole_zero_summary".to_string(),
                    json!("out/pole_zero_summary.csv"),
                ),
            ]),
            limit: BTreeMap::from([("required_root_index".to_string(), json!(2))]),
            suggested_fixes: Vec::new(),
        }
    }
}
