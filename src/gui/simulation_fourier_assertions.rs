use super::CircuitCiApp;
use super::analog::{AnalogScenarioChoice, analog_scenario_choices};
use super::analog_fourier_assertions::{
    AnalogFourierAssertionDraft, append_analog_fourier_assertion,
    unique_analog_fourier_assertion_name,
};
use super::simulation_forms::{analog_scenario_combo, string_combo};
use crate::reports::Finding;
use eframe::egui;
use serde_json::Value;

impl CircuitCiApp {
    pub(super) fn fourier_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices
                .into_iter()
                .filter(|scenario| scenario.scenario_type == "analog_fourier")
                .collect::<Vec<_>>(),
            Err(error) => {
                ui.collapsing("Fourier Check", |ui| {
                    ui.label(format!("Fourier run setups unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Fourier Check", |ui| {
            if choices.is_empty() {
                ui.label("No Fourier run setup is available.");
                return;
            }
            initialize_fourier_assertion_defaults(&choices, self);
            egui::Grid::new("fourier_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_scenario_combo(
                        ui,
                        "fourier_assertion_scenario",
                        &mut self.analog_fourier_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Check");
                    ui.text_edit_singleline(&mut self.analog_fourier_assertion_name);
                    ui.end_row();

                    ui.label("Metric");
                    let previous_metric = self.analog_fourier_assertion_metric.clone();
                    fourier_metric_combo(ui, &mut self.analog_fourier_assertion_metric);
                    if previous_metric != self.analog_fourier_assertion_metric {
                        if self.analog_fourier_assertion_name.trim().is_empty()
                            || self.analog_fourier_assertion_name
                                == default_fourier_assertion_name(&previous_metric)
                        {
                            self.analog_fourier_assertion_name = default_fourier_assertion_name(
                                &self.analog_fourier_assertion_metric,
                            )
                            .to_string();
                        }
                        self.analog_fourier_assertion_relation =
                            default_fourier_relation(&self.analog_fourier_assertion_metric)
                                .to_string();
                        self.analog_fourier_assertion_threshold =
                            default_fourier_threshold(&self.analog_fourier_assertion_metric);
                    }
                    ui.end_row();

                    if self.analog_fourier_assertion_metric != "thd_percent" {
                        ui.label("Harmonic");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_fourier_assertion_harmonic)
                                .range(0..=1024),
                        );
                        ui.end_row();
                    } else {
                        ui.label("Harmonic");
                        ui.label("THD");
                        ui.end_row();
                    }

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "fourier_assertion_relation",
                        &mut self.analog_fourier_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_fourier_assertion_threshold)
                            .speed(fourier_threshold_speed(
                                &self.analog_fourier_assertion_metric,
                            ))
                            .range(-1.0e12..=1.0e12)
                            .suffix(fourier_threshold_suffix(
                                &self.analog_fourier_assertion_metric,
                            )),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("H2 Ratio").clicked() {
                    self.apply_fourier_preset("normalized_magnitude");
                }
                if ui.button("THD").clicked() {
                    self.apply_fourier_preset("thd_percent");
                }
                if ui.button("H1 Mag").clicked() {
                    self.apply_fourier_preset("magnitude");
                }
                if ui.button("H1 Phase").clicked() {
                    self.apply_fourier_preset("phase_deg");
                }
                if ui.button("Add Fourier Check").clicked() {
                    self.apply_add_fourier_assertion();
                }
            });
        });
    }

    pub(super) fn fourier_assertion_failure_actions(
        &mut self,
        ui: &mut egui::Ui,
        failures: &[Finding],
    ) {
        let rows = fourier_failure_rows(failures);
        ui.label("Fourier assertion failures");
        if rows.is_empty() {
            ui.label("No Fourier assertion failures were emitted.");
            return;
        }
        egui::Grid::new("fourier_assertion_failure_actions")
            .num_columns(6)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Scenario");
                ui.strong("Check");
                ui.strong("Harmonic");
                ui.strong("Metric");
                ui.strong("Limit");
                ui.strong("Actions");
                ui.end_row();
                for row in rows {
                    ui.monospace(&row.scenario);
                    ui.monospace(&row.assertion);
                    ui.monospace(
                        row.harmonic
                            .map(|harmonic| harmonic.to_string())
                            .unwrap_or_else(|| "THD".to_string()),
                    );
                    ui.label(row.metric.as_str());
                    ui.label(format!(
                        "{} {:.6e}",
                        row.relation.as_deref().unwrap_or("limit"),
                        row.threshold.unwrap_or_default()
                    ));
                    if ui.button("Open Fourier Check").clicked() {
                        self.load_fourier_failure_row(&row);
                    }
                    ui.end_row();
                }
            });
    }

    fn apply_fourier_preset(&mut self, metric: &str) {
        self.analog_fourier_assertion_metric = metric.to_string();
        self.analog_fourier_assertion_harmonic = default_fourier_harmonic(metric);
        self.analog_fourier_assertion_name = default_fourier_assertion_name(metric).to_string();
        self.analog_fourier_assertion_relation = default_fourier_relation(metric).to_string();
        self.analog_fourier_assertion_threshold = default_fourier_threshold(metric);
    }

    fn apply_add_fourier_assertion(&mut self) {
        let requested_name = if self.analog_fourier_assertion_name.trim().is_empty() {
            default_fourier_assertion_name(&self.analog_fourier_assertion_metric).to_string()
        } else {
            self.analog_fourier_assertion_name.trim().to_string()
        };
        let assertion_name = match unique_analog_fourier_assertion_name(
            &self.project_yaml,
            &self.analog_fourier_assertion_scenario,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogFourierAssertionDraft {
            scenario_name: self.analog_fourier_assertion_scenario.clone(),
            assertion_name: assertion_name.clone(),
            harmonic: if self.analog_fourier_assertion_metric == "thd_percent" {
                None
            } else {
                Some(self.analog_fourier_assertion_harmonic)
            },
            metric: self.analog_fourier_assertion_metric.clone(),
            relation: self.analog_fourier_assertion_relation.clone(),
            threshold: self.analog_fourier_assertion_threshold,
        };
        match append_analog_fourier_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_fourier_assertion_name = assertion_name.clone();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Fourier check {} added.", assertion_name.trim()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn load_fourier_failure_row(&mut self, row: &FourierFailureRow) {
        self.analog_fourier_assertion_scenario = row.scenario.clone();
        self.analog_fourier_assertion_name = row.assertion.clone();
        self.analog_fourier_assertion_metric = row.metric.clone();
        self.analog_fourier_assertion_harmonic = row
            .harmonic
            .unwrap_or_else(|| default_fourier_harmonic(&row.metric));
        if let Some(relation) = &row.relation {
            self.analog_fourier_assertion_relation = relation.clone();
        }
        if let Some(threshold) = row.threshold {
            self.analog_fourier_assertion_threshold = threshold;
        }
        self.status = format!("Loaded Fourier check {} from latest report.", row.assertion);
    }
}

#[derive(Debug, Clone)]
struct FourierFailureRow {
    scenario: String,
    assertion: String,
    harmonic: Option<u32>,
    metric: String,
    relation: Option<String>,
    threshold: Option<f64>,
}

fn fourier_failure_rows(failures: &[Finding]) -> Vec<FourierFailureRow> {
    failures
        .iter()
        .filter(|finding| finding.id == "SPICE_FOURIER_ANALYSIS")
        .filter_map(fourier_failure_row)
        .collect()
}

fn fourier_failure_row(finding: &Finding) -> Option<FourierFailureRow> {
    let assertion = text_field(&finding.measured, "assertion")?;
    let metric = text_field(&finding.measured, "metric")?;
    let harmonic = u32_field(&finding.measured, "harmonic");
    let (relation, threshold) = threshold_limit(&finding.limit);
    Some(FourierFailureRow {
        scenario: finding.scenario.clone(),
        assertion,
        harmonic,
        metric,
        relation,
        threshold,
    })
}

fn initialize_fourier_assertion_defaults(choices: &[AnalogScenarioChoice], app: &mut CircuitCiApp) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == app.analog_fourier_assertion_scenario);
    if (app.analog_fourier_assertion_scenario.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        app.analog_fourier_assertion_scenario = scenario.name.clone();
    }
    if !fourier_metric_options()
        .iter()
        .any(|option| option.0 == app.analog_fourier_assertion_metric.as_str())
    {
        app.analog_fourier_assertion_metric = "normalized_magnitude".to_string();
    }
    if !matches!(
        app.analog_fourier_assertion_relation.as_str(),
        "above" | "below"
    ) {
        app.analog_fourier_assertion_relation =
            default_fourier_relation(&app.analog_fourier_assertion_metric).to_string();
    }
    if app.analog_fourier_assertion_name.trim().is_empty() {
        app.analog_fourier_assertion_name =
            default_fourier_assertion_name(&app.analog_fourier_assertion_metric).to_string();
    }
    if app.analog_fourier_assertion_harmonic > 1024 {
        app.analog_fourier_assertion_harmonic =
            default_fourier_harmonic(&app.analog_fourier_assertion_metric);
    }
    if !app.analog_fourier_assertion_threshold.is_finite() {
        app.analog_fourier_assertion_threshold =
            default_fourier_threshold(&app.analog_fourier_assertion_metric);
    }
}

fn fourier_metric_combo(ui: &mut egui::Ui, selected: &mut String) {
    let selected_label = fourier_metric_options()
        .iter()
        .find(|option| option.0 == selected.as_str())
        .map(|option| option.1)
        .unwrap_or(selected.as_str());
    egui::ComboBox::from_id_salt("fourier_assertion_metric")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            for (metric, label) in fourier_metric_options() {
                ui.selectable_value(selected, (*metric).to_string(), *label);
            }
        });
}

fn fourier_metric_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("normalized_magnitude", "Normalized magnitude"),
        ("magnitude", "Magnitude"),
        ("phase_deg", "Phase"),
        ("normalized_phase_deg", "Normalized phase"),
        ("thd_percent", "THD"),
    ]
}

fn default_fourier_assertion_name(metric: &str) -> &'static str {
    match metric {
        "magnitude" => "h1_magnitude_floor",
        "phase_deg" => "h1_phase_ceiling",
        "normalized_phase_deg" => "h2_normalized_phase_ceiling",
        "thd_percent" => "thd_ceiling",
        _ => "h2_ratio_ceiling",
    }
}

fn default_fourier_harmonic(metric: &str) -> u32 {
    match metric {
        "magnitude" | "phase_deg" => 1,
        _ => 2,
    }
}

fn default_fourier_relation(metric: &str) -> &'static str {
    match metric {
        "magnitude" => "above",
        _ => "below",
    }
}

fn default_fourier_threshold(metric: &str) -> f64 {
    match metric {
        "magnitude" => 0.0,
        "phase_deg" | "normalized_phase_deg" => 180.0,
        "thd_percent" => 10.0,
        _ => 0.03,
    }
}

fn fourier_threshold_speed(metric: &str) -> f64 {
    match metric {
        "thd_percent" | "phase_deg" | "normalized_phase_deg" => 1.0,
        _ => 0.01,
    }
}

fn fourier_threshold_suffix(metric: &str) -> &'static str {
    match metric {
        "thd_percent" => " %",
        "phase_deg" | "normalized_phase_deg" => " deg",
        "normalized_magnitude" => " ratio",
        _ => " output",
    }
}

fn text_field(map: &std::collections::BTreeMap<String, Value>, name: &str) -> Option<String> {
    map.get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn u32_field(map: &std::collections::BTreeMap<String, Value>, name: &str) -> Option<u32> {
    u32::try_from(map.get(name)?.as_u64()?).ok()
}

fn threshold_limit(
    map: &std::collections::BTreeMap<String, Value>,
) -> (Option<String>, Option<f64>) {
    for (key, relation) in [
        ("above_threshold", "above"),
        ("below_threshold", "below"),
        ("required_harmonic", "required"),
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
    fn fourier_failure_loads_editor_state() {
        let mut app = CircuitCiApp::default();
        let row = fourier_failure_row(&fourier_failure()).unwrap();

        app.load_fourier_failure_row(&row);

        assert_eq!(app.analog_fourier_assertion_scenario, "fourier_out");
        assert_eq!(app.analog_fourier_assertion_name, "h2_ratio_ceiling");
        assert_eq!(app.analog_fourier_assertion_harmonic, 2);
        assert_eq!(app.analog_fourier_assertion_metric, "normalized_magnitude");
        assert_eq!(app.analog_fourier_assertion_relation, "below");
        assert_eq!(app.analog_fourier_assertion_threshold, 0.03);
        assert_eq!(
            app.status,
            "Loaded Fourier check h2_ratio_ceiling from latest report."
        );
    }

    #[test]
    fn fourier_thd_failure_loads_without_harmonic() {
        let mut finding = fourier_failure();
        finding
            .measured
            .insert("assertion".to_string(), json!("thd_ceiling"));
        finding
            .measured
            .insert("metric".to_string(), json!("thd_percent"));
        finding.measured.remove("harmonic");
        finding
            .limit
            .insert("below_threshold".to_string(), json!(10.0));
        let mut app = CircuitCiApp::default();
        let row = fourier_failure_row(&finding).unwrap();

        app.load_fourier_failure_row(&row);

        assert_eq!(app.analog_fourier_assertion_name, "thd_ceiling");
        assert_eq!(app.analog_fourier_assertion_metric, "thd_percent");
        assert_eq!(app.analog_fourier_assertion_threshold, 10.0);
    }

    fn fourier_failure() -> Finding {
        Finding {
            id: "SPICE_FOURIER_ANALYSIS".to_string(),
            scenario: "fourier_out".to_string(),
            severity: Severity::Critical,
            message: "Fourier assertion failed".to_string(),
            component: None,
            net: None,
            endpoints: None,
            measured: BTreeMap::from([
                ("assertion".to_string(), json!("h2_ratio_ceiling")),
                ("metric".to_string(), json!("normalized_magnitude")),
                ("harmonic".to_string(), json!(2)),
                (
                    "fourier_summary".to_string(),
                    json!("out/fourier_summary.csv"),
                ),
            ]),
            limit: BTreeMap::from([("below_threshold".to_string(), json!(0.03))]),
            suggested_fixes: Vec::new(),
        }
    }
}
