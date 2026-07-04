use super::CircuitCiApp;
use super::analog::{
    AnalogSParameterAssertionDraft, analog_scenario_choices, append_analog_sparameter_assertion,
    unique_analog_sparameter_assertion_name,
};
use super::simulation_forms::{analog_scenario_combo, string_combo};
use crate::reports::Finding;
use eframe::egui;
use serde_json::Value;

impl CircuitCiApp {
    pub(super) fn sparameter_assertion_failure_actions(
        &mut self,
        ui: &mut egui::Ui,
        failures: &[Finding],
    ) {
        let rows = sparameter_failure_rows(failures);
        ui.label("RF assertion failures");
        if rows.is_empty() {
            ui.label("No S-parameter assertion failures were emitted.");
            return;
        }
        egui::Grid::new("sparameter_assertion_failure_actions")
            .num_columns(6)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Scenario");
                ui.strong("Check");
                ui.strong("Kind");
                ui.strong("Metric");
                ui.strong("Limit");
                ui.strong("Actions");
                ui.end_row();
                for row in rows {
                    ui.monospace(&row.scenario);
                    ui.monospace(&row.assertion);
                    ui.label(row.kind.label());
                    ui.label(row.metric.as_str());
                    ui.label(format!(
                        "{} {:.6e}",
                        row.relation.as_deref().unwrap_or("limit"),
                        row.threshold.unwrap_or_default()
                    ));
                    if ui.button(row.kind.action_label()).clicked() {
                        self.load_sparameter_failure_row(&row);
                    }
                    ui.end_row();
                }
            });
    }

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
                if ui.button("Input Z").clicked() {
                    self.apply_sparameter_preset("impedance_magnitude_ohm");
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

    fn load_sparameter_failure_row(&mut self, row: &SParameterFailureRow) {
        match row.kind {
            SParameterFailureKind::Port => {
                self.analog_sparameter_assertion_scenario = row.scenario.clone();
                self.analog_sparameter_assertion_name = row.assertion.clone();
                if let Some(parameter) = &row.parameter {
                    self.analog_sparameter_assertion_parameter = parameter.clone();
                }
                self.analog_sparameter_assertion_metric = row.metric.clone();
                if let Some(aggregation) = &row.aggregation {
                    self.analog_sparameter_assertion_aggregation = aggregation.clone();
                }
                if let Some(relation) = &row.relation {
                    self.analog_sparameter_assertion_relation = relation.clone();
                }
                if let Some(threshold) = row.threshold {
                    self.analog_sparameter_assertion_threshold = threshold;
                }
                self.status = format!("Loaded RF port check {} from latest report.", row.assertion);
            }
            SParameterFailureKind::Network => {
                self.analog_sparameter_network_assertion_scenario = row.scenario.clone();
                self.analog_sparameter_network_assertion_name = row.assertion.clone();
                self.analog_sparameter_network_assertion_metric = row.metric.clone();
                if let Some(relation) = &row.relation {
                    self.analog_sparameter_network_assertion_relation = relation.clone();
                }
                if let Some(threshold) = row.threshold {
                    self.analog_sparameter_network_assertion_threshold = threshold;
                }
                self.status = format!(
                    "Loaded RF network check {} from latest report.",
                    row.assertion
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SParameterFailureKind {
    Port,
    Network,
}

impl SParameterFailureKind {
    fn label(self) -> &'static str {
        match self {
            Self::Port => "port",
            Self::Network => "network",
        }
    }

    fn action_label(self) -> &'static str {
        match self {
            Self::Port => "Open Port Check",
            Self::Network => "Open Network Check",
        }
    }
}

#[derive(Debug, Clone)]
struct SParameterFailureRow {
    kind: SParameterFailureKind,
    scenario: String,
    assertion: String,
    parameter: Option<String>,
    metric: String,
    aggregation: Option<String>,
    relation: Option<String>,
    threshold: Option<f64>,
}

fn sparameter_failure_rows(failures: &[Finding]) -> Vec<SParameterFailureRow> {
    failures
        .iter()
        .filter(|finding| finding.id == "SPICE_S_PARAMETER_ANALYSIS")
        .filter_map(sparameter_failure_row)
        .collect()
}

fn sparameter_failure_row(finding: &Finding) -> Option<SParameterFailureRow> {
    let assertion = text_field(&finding.measured, "assertion")?;
    let metric = text_field(&finding.measured, "metric")?;
    let (relation, threshold) = threshold_limit(&finding.limit);
    let parameter = text_field(&finding.measured, "parameter");
    if let Some(parameter) = parameter {
        return Some(SParameterFailureRow {
            kind: SParameterFailureKind::Port,
            scenario: finding.scenario.clone(),
            assertion,
            parameter: Some(parameter),
            metric,
            aggregation: text_field(&finding.measured, "aggregation"),
            relation,
            threshold,
        });
    }
    if finding.measured.contains_key("s_parameter_network_summary") {
        return Some(SParameterFailureRow {
            kind: SParameterFailureKind::Network,
            scenario: finding.scenario.clone(),
            assertion,
            parameter: None,
            metric,
            aggregation: None,
            relation,
            threshold,
        });
    }
    None
}

fn text_field(map: &std::collections::BTreeMap<String, Value>, name: &str) -> Option<String> {
    map.get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn threshold_limit(
    map: &std::collections::BTreeMap<String, Value>,
) -> (Option<String>, Option<f64>) {
    for relation in ["above", "below"] {
        let key = format!("{relation}_threshold");
        if let Some(threshold) = map.get(&key).and_then(Value::as_f64) {
            return (Some(relation.to_string()), Some(threshold));
        }
    }
    (None, None)
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
        ("mismatch_loss_db", "Mismatch loss"),
        ("group_delay_s", "Group delay"),
        ("impedance_real_ohm", "Z real"),
        ("impedance_imag_ohm", "Z imaginary"),
        ("impedance_magnitude_ohm", "Z magnitude"),
        ("magnitude_db", "Magnitude dB"),
        ("magnitude_linear", "Magnitude linear"),
    ]
}

fn default_sparameter_assertion_name(metric: &str) -> &'static str {
    match metric {
        "insertion_loss_db" => "s21_insertion_loss_ceiling",
        "vswr" => "s11_vswr_ceiling",
        "mismatch_loss_db" => "s11_mismatch_loss_ceiling",
        "group_delay_s" => "s21_group_delay_ceiling",
        "impedance_real_ohm" => "s11_impedance_real_window",
        "impedance_imag_ohm" => "s11_impedance_imag_ceiling",
        "impedance_magnitude_ohm" => "s11_impedance_magnitude_ceiling",
        "magnitude_db" => "s21_magnitude_db_ceiling",
        "magnitude_linear" => "s21_magnitude_ceiling",
        _ => "s11_return_loss_floor",
    }
}

fn default_sparameter_parameter(metric: &str) -> &'static str {
    match metric {
        "return_loss_db"
        | "vswr"
        | "mismatch_loss_db"
        | "impedance_real_ohm"
        | "impedance_imag_ohm"
        | "impedance_magnitude_ohm" => "s11",
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
        "mismatch_loss_db" => 0.5,
        "group_delay_s" => 1.0e-9,
        "impedance_real_ohm" | "impedance_magnitude_ohm" => 75.0,
        "impedance_imag_ohm" => 25.0,
        "magnitude_linear" => 1.0,
        _ => 0.0,
    }
}

fn sparameter_threshold_speed(metric: &str) -> f64 {
    match metric {
        "group_delay_s" => 1.0e-10,
        "impedance_real_ohm" | "impedance_imag_ohm" | "impedance_magnitude_ohm" => 1.0,
        "magnitude_linear" | "vswr" => 0.01,
        _ => 0.1,
    }
}

fn sparameter_threshold_suffix(metric: &str) -> &'static str {
    match metric {
        "return_loss_db" | "insertion_loss_db" | "magnitude_db" | "mismatch_loss_db" => " dB",
        "group_delay_s" => " s",
        "impedance_real_ohm" | "impedance_imag_ohm" | "impedance_magnitude_ohm" => " ohm",
        _ => " ratio",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reports::Finding;
    use serde_json::json;

    #[test]
    fn sparameter_failure_rows_parse_port_assertion_limit() {
        let mut finding = Finding::critical(
            "SPICE_S_PARAMETER_ANALYSIS",
            "two_port_sparameter",
            "S-parameter assertion s11_return_loss_floor failed",
        );
        finding
            .measured
            .insert("assertion".to_string(), json!("s11_return_loss_floor"));
        finding
            .measured
            .insert("parameter".to_string(), json!("s11"));
        finding
            .measured
            .insert("metric".to_string(), json!("return_loss_db"));
        finding
            .measured
            .insert("aggregation".to_string(), json!("min"));
        finding
            .limit
            .insert("above_threshold".to_string(), json!(10.0));

        let rows = sparameter_failure_rows(&[finding]);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SParameterFailureKind::Port);
        assert_eq!(rows[0].scenario, "two_port_sparameter");
        assert_eq!(rows[0].assertion, "s11_return_loss_floor");
        assert_eq!(rows[0].parameter.as_deref(), Some("s11"));
        assert_eq!(rows[0].metric, "return_loss_db");
        assert_eq!(rows[0].aggregation.as_deref(), Some("min"));
        assert_eq!(rows[0].relation.as_deref(), Some("above"));
        assert_eq!(rows[0].threshold, Some(10.0));
    }

    #[test]
    fn sparameter_impedance_failure_loads_rf_port_editor_state() {
        let mut finding = Finding::critical(
            "SPICE_S_PARAMETER_ANALYSIS",
            "two_port_sparameter",
            "S-parameter assertion s11_impedance_magnitude_ceiling failed",
        );
        finding.measured.insert(
            "assertion".to_string(),
            json!("s11_impedance_magnitude_ceiling"),
        );
        finding
            .measured
            .insert("parameter".to_string(), json!("s11"));
        finding
            .measured
            .insert("metric".to_string(), json!("impedance_magnitude_ohm"));
        finding
            .measured
            .insert("aggregation".to_string(), json!("max"));
        finding
            .limit
            .insert("below_threshold".to_string(), json!(75.0));

        let rows = sparameter_failure_rows(&[finding]);
        let mut app = CircuitCiApp::default();
        app.load_sparameter_failure_row(&rows[0]);

        assert_eq!(
            app.analog_sparameter_assertion_scenario,
            "two_port_sparameter"
        );
        assert_eq!(
            app.analog_sparameter_assertion_name,
            "s11_impedance_magnitude_ceiling"
        );
        assert_eq!(app.analog_sparameter_assertion_parameter, "s11");
        assert_eq!(
            app.analog_sparameter_assertion_metric,
            "impedance_magnitude_ohm"
        );
        assert_eq!(app.analog_sparameter_assertion_aggregation, "max");
        assert_eq!(app.analog_sparameter_assertion_relation, "below");
        assert_eq!(app.analog_sparameter_assertion_threshold, 75.0);
        assert!(
            app.status
                .contains("Loaded RF port check s11_impedance_magnitude_ceiling")
        );
    }

    #[test]
    fn sparameter_failure_rows_parse_network_assertion_limit() {
        let mut finding = Finding::critical(
            "SPICE_S_PARAMETER_ANALYSIS",
            "two_port_sparameter",
            "S-parameter network assertion stable_delta failed",
        );
        finding
            .measured
            .insert("assertion".to_string(), json!("stable_delta"));
        finding
            .measured
            .insert("metric".to_string(), json!("stability_delta_magnitude_max"));
        finding.measured.insert(
            "s_parameter_network_summary".to_string(),
            json!("out/s_parameter_network_summary.csv"),
        );
        finding
            .limit
            .insert("below_threshold".to_string(), json!(1.0));

        let rows = sparameter_failure_rows(&[finding]);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, SParameterFailureKind::Network);
        assert_eq!(rows[0].assertion, "stable_delta");
        assert_eq!(rows[0].metric, "stability_delta_magnitude_max");
        assert_eq!(rows[0].relation.as_deref(), Some("below"));
        assert_eq!(rows[0].threshold, Some(1.0));
    }
}
