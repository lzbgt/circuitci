use super::CircuitCiApp;
use super::analog_sweeps::{
    AnalogMonteCarloCriteriaDraft, AnalogSweepComponentValueDraft, AnalogSweepDraft,
    AnalogSweepModelSectionDraft, AnalogSweepParameterDraft, AnalogSweepScenario,
    AnalogSweepSummary, analog_load_sweep_candidates, analog_sweep_presets, analog_sweep_scenarios,
    append_analog_sweep_component_value, append_analog_sweep_model_section,
    append_analog_sweep_parameter, append_analog_sweep_preset,
    append_analog_sweep_with_component_value, append_analog_sweep_with_model_section,
    append_analog_sweep_with_parameter, remove_analog_sweep, remove_analog_sweep_component_value,
    remove_analog_sweep_model_section, remove_analog_sweep_parameter,
    set_analog_monte_carlo_criteria,
};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn analog_sweep_editor(&mut self, ui: &mut egui::Ui) {
        let scenarios = match analog_sweep_scenarios(&self.project_yaml) {
            Ok(scenarios) => scenarios,
            Err(error) => {
                ui.collapsing("Run Input Sweeps", |ui| {
                    ui.label(format!("Run input sweeps unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Run Input Sweeps", |ui| {
            if scenarios.is_empty() {
                ui.label("No analog run setup is available. Add one first.");
                return;
            }
            initialize_analog_sweep_defaults(
                &scenarios,
                &mut self.analog_sweep_scenario,
                &mut self.analog_sweep_name,
            );
            egui::Grid::new("analog_sweep_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Run setup");
                    analog_sweep_scenario_combo(ui, &scenarios, &mut self.analog_sweep_scenario);
                    ui.end_row();

                    ui.label("Sweep");
                    ui.text_edit_singleline(&mut self.analog_sweep_name);
                    ui.end_row();

                    ui.label("Parameter");
                    ui.text_edit_singleline(&mut self.analog_sweep_parameter_name);
                    ui.end_row();

                    ui.label("Values");
                    ui.text_edit_singleline(&mut self.analog_sweep_parameter_values);
                    ui.end_row();

                    ui.label("Component");
                    ui.text_edit_singleline(&mut self.analog_sweep_component);
                    ui.end_row();

                    ui.label("Component field");
                    ui.text_edit_singleline(&mut self.analog_sweep_component_field);
                    ui.end_row();

                    ui.label("Component values");
                    ui.text_edit_singleline(&mut self.analog_sweep_component_values);
                    ui.end_row();

                    ui.label("Model file");
                    ui.text_edit_singleline(&mut self.analog_sweep_model_path);
                    ui.end_row();

                    ui.label("Sections");
                    ui.text_edit_singleline(&mut self.analog_sweep_model_sections);
                    ui.end_row();

                    ui.label("MC min yield %");
                    ui.text_edit_singleline(&mut self.analog_sweep_min_yield_percent);
                    ui.end_row();

                    ui.label("MC P1 margin");
                    ui.text_edit_singleline(&mut self.analog_sweep_min_p1_margin);
                    ui.end_row();

                    ui.label("MC P5 margin");
                    ui.text_edit_singleline(&mut self.analog_sweep_min_p5_margin);
                    ui.end_row();

                    ui.label("MC P50 margin");
                    ui.text_edit_singleline(&mut self.analog_sweep_min_p50_margin);
                    ui.end_row();

                    ui.label("MC P95 margin");
                    ui.text_edit_singleline(&mut self.analog_sweep_min_p95_margin);
                    ui.end_row();
                });
            let selected_sweep = selected_analog_sweep(
                &scenarios,
                &self.analog_sweep_scenario,
                &self.analog_sweep_name,
            );
            ui.horizontal(|ui| {
                if ui.button("Add Sweep + Parameter").clicked() {
                    self.apply_add_analog_sweep();
                }
                if ui.button("Add Sweep + Model Sections").clicked() {
                    self.apply_add_analog_sweep_with_model_section();
                }
                if ui.button("Add Sweep + Component Values").clicked() {
                    self.apply_add_analog_sweep_with_component_value();
                }
                if ui
                    .add_enabled(selected_sweep.is_some(), egui::Button::new("Remove Sweep"))
                    .clicked()
                {
                    self.apply_remove_analog_sweep();
                }
            });
            ui.add_space(4.0);
            if let Ok(candidates) =
                analog_load_sweep_candidates(&self.project_yaml, &self.analog_sweep_scenario)
                && !candidates.is_empty()
            {
                ui.strong("Generated load/source candidates");
                egui::Grid::new("analog_sweep_load_candidates")
                    .num_columns(5)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Component");
                        ui.strong("Field");
                        ui.strong("Nominal");
                        ui.strong("Values");
                        ui.strong("Use");
                        ui.end_row();
                        for candidate in candidates {
                            ui.label(&candidate.component);
                            ui.label(&candidate.field);
                            ui.label(format!("{:.6}", candidate.nominal));
                            ui.label(&candidate.values_csv);
                            if ui.button("Use").clicked() {
                                self.analog_sweep_component = candidate.component;
                                self.analog_sweep_component_field = candidate.field;
                                self.analog_sweep_component_values = candidate.values_csv;
                            }
                            ui.end_row();
                        }
                    });
            }
            ui.add_space(4.0);
            ui.strong("Corner presets");
            egui::Grid::new("analog_sweep_presets")
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Preset");
                    ui.strong("Inputs");
                    ui.strong("Add");
                    ui.end_row();
                    for preset in analog_sweep_presets() {
                        ui.label(preset.label);
                        ui.label(preset.summary);
                        if ui.button("Add").clicked() {
                            self.apply_add_analog_sweep_preset(preset.id, preset.sweep_name);
                        }
                        ui.end_row();
                    }
                });

            if let Some(selected_scenario) =
                selected_analog_sweep_scenario(&scenarios, &self.analog_sweep_scenario)
            {
                analog_sweep_rows(ui, selected_scenario, &mut self.analog_sweep_name);
            }

            if let Some(selected_sweep) = selected_sweep {
                ui.add_space(4.0);
                let has_monte_carlo = selected_sweep.monte_carlo.is_some();
                let has_criteria = selected_sweep
                    .monte_carlo
                    .as_ref()
                    .is_some_and(|monte_carlo| monte_carlo.criteria.is_some());
                ui.horizontal(|ui| {
                    if ui.button("Add Parameter").clicked() {
                        self.apply_add_analog_sweep_parameter();
                    }
                    if ui.button("Remove Parameter").clicked() {
                        self.apply_remove_analog_sweep_parameter();
                    }
                    if ui.button("Add Model Sections").clicked() {
                        self.apply_add_analog_sweep_model_section();
                    }
                    if ui.button("Remove Model Sections").clicked() {
                        self.apply_remove_analog_sweep_model_section();
                    }
                    if ui.button("Add Component Values").clicked() {
                        self.apply_add_analog_sweep_component_value();
                    }
                    if ui.button("Remove Component Values").clicked() {
                        self.apply_remove_analog_sweep_component_value();
                    }
                    if ui
                        .add_enabled(
                            has_monte_carlo,
                            egui::Button::new("Set Monte Carlo Criteria"),
                        )
                        .clicked()
                    {
                        self.apply_set_analog_monte_carlo_criteria();
                    }
                    if ui
                        .add_enabled(
                            has_monte_carlo && has_criteria,
                            egui::Button::new("Clear Monte Carlo Criteria"),
                        )
                        .clicked()
                    {
                        self.apply_clear_analog_monte_carlo_criteria();
                    }
                });
                if !has_monte_carlo {
                    ui.label("Monte Carlo criteria controls apply only to Monte Carlo sweeps.");
                }
            }
        });
    }

    fn apply_add_analog_sweep(&mut self) {
        let draft = AnalogSweepParameterDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            parameter_name: self.analog_sweep_parameter_name.clone(),
            values_csv: self.analog_sweep_parameter_values.clone(),
        };
        match append_analog_sweep_with_parameter(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Run input sweep {} added with parameter {}.",
                    draft.sweep_name.trim(),
                    draft.parameter_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_sweep_with_model_section(&mut self) {
        let draft = AnalogSweepModelSectionDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            path: self.analog_sweep_model_path.clone(),
            sections_csv: self.analog_sweep_model_sections.clone(),
        };
        match append_analog_sweep_with_model_section(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Run input sweep {} added with model section file {}.",
                    draft.sweep_name.trim(),
                    draft.path.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_sweep_with_component_value(&mut self) {
        let draft = AnalogSweepComponentValueDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            component: self.analog_sweep_component.clone(),
            field: self.analog_sweep_component_field.clone(),
            values_csv: self.analog_sweep_component_values.clone(),
        };
        match append_analog_sweep_with_component_value(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Run input sweep {} added with component value {}.{}.",
                    draft.sweep_name.trim(),
                    draft.component.trim(),
                    draft.field.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_sweep_preset(&mut self, preset_id: &str, sweep_name: &str) {
        match append_analog_sweep_preset(&self.project_yaml, &self.analog_sweep_scenario, preset_id)
        {
            Ok(updated) => {
                self.analog_sweep_name = sweep_name.to_string();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Run input corner preset {sweep_name} added."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_analog_sweep(&mut self) {
        let draft = AnalogSweepDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
        };
        match remove_analog_sweep(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!("Run input sweep {} removed.", draft.sweep_name.trim()),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_sweep_parameter(&mut self) {
        let draft = AnalogSweepParameterDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            parameter_name: self.analog_sweep_parameter_name.clone(),
            values_csv: self.analog_sweep_parameter_values.clone(),
        };
        match append_analog_sweep_parameter(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Run input parameter {} added to sweep {}.",
                    draft.parameter_name.trim(),
                    draft.sweep_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_sweep_model_section(&mut self) {
        let draft = AnalogSweepModelSectionDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            path: self.analog_sweep_model_path.clone(),
            sections_csv: self.analog_sweep_model_sections.clone(),
        };
        match append_analog_sweep_model_section(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Model section corner file {} added to sweep {}.",
                    draft.path.trim(),
                    draft.sweep_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_sweep_component_value(&mut self) {
        let draft = AnalogSweepComponentValueDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            component: self.analog_sweep_component.clone(),
            field: self.analog_sweep_component_field.clone(),
            values_csv: self.analog_sweep_component_values.clone(),
        };
        match append_analog_sweep_component_value(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Component value {}.{} added to sweep {}.",
                    draft.component.trim(),
                    draft.field.trim(),
                    draft.sweep_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_analog_sweep_parameter(&mut self) {
        let draft = AnalogSweepParameterDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            parameter_name: self.analog_sweep_parameter_name.clone(),
            values_csv: String::new(),
        };
        match remove_analog_sweep_parameter(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Run input parameter {} removed from sweep {}.",
                    draft.parameter_name.trim(),
                    draft.sweep_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_analog_sweep_model_section(&mut self) {
        let draft = AnalogSweepModelSectionDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            path: self.analog_sweep_model_path.clone(),
            sections_csv: String::new(),
        };
        match remove_analog_sweep_model_section(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Model section corner file {} removed from sweep {}.",
                    draft.path.trim(),
                    draft.sweep_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_analog_sweep_component_value(&mut self) {
        let draft = AnalogSweepComponentValueDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            component: self.analog_sweep_component.clone(),
            field: self.analog_sweep_component_field.clone(),
            values_csv: String::new(),
        };
        match remove_analog_sweep_component_value(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Component value {}.{} removed from sweep {}.",
                    draft.component.trim(),
                    draft.field.trim(),
                    draft.sweep_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_set_analog_monte_carlo_criteria(&mut self) {
        let draft = AnalogMonteCarloCriteriaDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            min_yield_percent: self.analog_sweep_min_yield_percent.clone(),
            min_p1_margin: self.analog_sweep_min_p1_margin.clone(),
            min_p5_margin: self.analog_sweep_min_p5_margin.clone(),
            min_p50_margin: self.analog_sweep_min_p50_margin.clone(),
            min_p95_margin: self.analog_sweep_min_p95_margin.clone(),
        };
        match set_analog_monte_carlo_criteria(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Monte Carlo criteria updated for sweep {}.",
                    draft.sweep_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_clear_analog_monte_carlo_criteria(&mut self) {
        let draft = AnalogMonteCarloCriteriaDraft {
            scenario_name: self.analog_sweep_scenario.clone(),
            sweep_name: self.analog_sweep_name.clone(),
            min_yield_percent: String::new(),
            min_p1_margin: String::new(),
            min_p5_margin: String::new(),
            min_p50_margin: String::new(),
            min_p95_margin: String::new(),
        };
        match set_analog_monte_carlo_criteria(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Monte Carlo criteria cleared for sweep {}.",
                    draft.sweep_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }
}

fn initialize_analog_sweep_defaults(
    scenarios: &[AnalogSweepScenario],
    selected_scenario: &mut String,
    selected_sweep: &mut String,
) {
    let scenario_stale = selected_scenario.trim().is_empty()
        || !scenarios
            .iter()
            .any(|scenario| scenario.name == *selected_scenario);
    if let (true, Some(scenario)) = (scenario_stale, scenarios.first()) {
        *selected_scenario = scenario.name.clone();
    }
    let Some(scenario) = selected_analog_sweep_scenario(scenarios, selected_scenario) else {
        return;
    };
    let sweep_stale = selected_sweep.trim().is_empty()
        || (!scenario.sweeps.is_empty()
            && !scenario
                .sweeps
                .iter()
                .any(|sweep| sweep.name == *selected_sweep));
    if let (true, Some(sweep)) = (sweep_stale, scenario.sweeps.first()) {
        *selected_sweep = sweep.name.clone();
    }
}

fn analog_sweep_scenario_combo(
    ui: &mut egui::Ui,
    scenarios: &[AnalogSweepScenario],
    selected_scenario: &mut String,
) {
    egui::ComboBox::from_id_salt("analog_sweep_scenario")
        .selected_text(if selected_scenario.is_empty() {
            "select run setup"
        } else {
            selected_scenario.as_str()
        })
        .show_ui(ui, |ui| {
            for scenario in scenarios {
                ui.selectable_value(selected_scenario, scenario.name.clone(), &scenario.name);
            }
        });
}

fn analog_sweep_rows(
    ui: &mut egui::Ui,
    scenario: &AnalogSweepScenario,
    selected_sweep: &mut String,
) {
    if scenario.sweeps.is_empty() {
        ui.label("No parameter sweeps are declared for this run setup.");
        return;
    }
    ui.add_space(4.0);
    ui.strong("Declared sweeps");
    egui::Grid::new("analog_sweep_rows")
        .num_columns(4)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Sweep");
            ui.strong("Parameters");
            ui.strong("Corners");
            ui.strong("Use");
            ui.end_row();
            for sweep in &scenario.sweeps {
                ui.monospace(&sweep.name);
                ui.label(parameter_summary(sweep));
                ui.label(sweep.corner_count.to_string());
                if ui.button("Select").clicked() {
                    *selected_sweep = sweep.name.clone();
                }
                ui.end_row();
            }
        });
}

fn selected_analog_sweep_scenario<'a>(
    scenarios: &'a [AnalogSweepScenario],
    selected_scenario: &str,
) -> Option<&'a AnalogSweepScenario> {
    scenarios
        .iter()
        .find(|scenario| scenario.name == selected_scenario)
}

fn selected_analog_sweep<'a>(
    scenarios: &'a [AnalogSweepScenario],
    selected_scenario: &str,
    selected_sweep: &str,
) -> Option<&'a AnalogSweepSummary> {
    selected_analog_sweep_scenario(scenarios, selected_scenario)?
        .sweeps
        .iter()
        .find(|sweep| sweep.name == selected_sweep)
}

fn parameter_summary(sweep: &AnalogSweepSummary) -> String {
    if sweep.parameters.is_empty()
        && sweep.component_values.is_empty()
        && sweep.model_sections.is_empty()
        && sweep.monte_carlo.is_none()
    {
        return "none".to_string();
    }
    let mut parts: Vec<String> = sweep
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "{} [{}]",
                parameter.name,
                parameter
                    .values
                    .iter()
                    .map(|value| format!("{value:.6}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    parts.extend(sweep.component_values.iter().map(|component_value| {
        format!(
            "{}.{} [{}]",
            component_value.component,
            component_value.field,
            component_value
                .values
                .iter()
                .map(|value| format!("{value:.6}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }));
    parts.extend(sweep.model_sections.iter().map(|model_section| {
        format!(
            "{} sections [{}]",
            model_section.path,
            model_section.sections.join(", ")
        )
    }));
    if let Some(monte_carlo) = &sweep.monte_carlo {
        let targets = monte_carlo
            .component_values
            .iter()
            .map(|component_value| {
                format!(
                    "{}.{} {:.6} +/-{:.3}%",
                    component_value.component,
                    component_value.field,
                    component_value.nominal,
                    component_value.tolerance_percent
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!(
            "Monte Carlo {} sample(s) [{targets}]",
            monte_carlo.samples
        ));
        if let Some(criteria) = &monte_carlo.criteria {
            let criteria_parts = [
                criteria
                    .min_yield_percent
                    .map(|value| format!("yield >= {value:.3}%")),
                criteria
                    .min_p1_margin
                    .map(|value| format!("P1 >= {value:.6}")),
                criteria
                    .min_p5_margin
                    .map(|value| format!("P5 >= {value:.6}")),
                criteria
                    .min_p50_margin
                    .map(|value| format!("P50 >= {value:.6}")),
                criteria
                    .min_p95_margin
                    .map(|value| format!("P95 >= {value:.6}")),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
            if !criteria_parts.is_empty() {
                parts.push(format!(
                    "Monte Carlo criteria [{}]",
                    criteria_parts.join(", ")
                ));
            }
        }
    }
    parts.join("; ")
}

#[cfg(test)]
mod tests {
    use super::CircuitCiApp;

    fn monte_carlo_project_yaml() -> &'static str {
        "project:
  name: gui_monte_carlo_criteria_action_test
  version: 0.1.0
board:
  components: {}
  nets:
    gnd:
      kind: ground
scenarios:
  - name: rc_run
    type: analog_transient
    analog:
      backend: auto
      netlist_source: file
      netlist: deck.cir
      model_files: []
      node_bindings:
        - { node: '0', net: gnd }
      pin_bindings: []
      analysis:
        type: tran
        stop_time_us: 1000
        max_step_us: 1
      stimuli: []
      sweeps:
        - name: rc_monte_carlo
          monte_carlo:
            samples: 8
            seed: 7
            component_values:
              - component: RLOAD
                field: value_ohm
                nominal: 1000.0
                tolerance_percent: 5.0
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"
    }

    #[test]
    fn app_action_sets_and_clears_monte_carlo_criteria() {
        let mut app = CircuitCiApp {
            project_yaml: monte_carlo_project_yaml().to_string(),
            analog_sweep_scenario: "rc_run".to_string(),
            analog_sweep_name: "rc_monte_carlo".to_string(),
            analog_sweep_min_yield_percent: "98".to_string(),
            analog_sweep_min_p1_margin: String::new(),
            analog_sweep_min_p5_margin: "0.05".to_string(),
            analog_sweep_min_p50_margin: "0.10".to_string(),
            analog_sweep_min_p95_margin: String::new(),
            ..Default::default()
        };

        app.apply_set_analog_monte_carlo_criteria();

        assert!(app.project_yaml_dirty);
        assert!(app.status.contains("Monte Carlo criteria updated"));
        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&app.project_yaml).unwrap();
        let criteria = project.scenarios[0].analog.as_ref().unwrap().sweeps[0]
            .monte_carlo
            .as_ref()
            .unwrap()
            .criteria
            .as_ref()
            .unwrap();
        assert_eq!(criteria.min_yield_percent, Some(98.0));
        assert_eq!(criteria.min_p5_margin, Some(0.05));
        assert_eq!(criteria.min_p50_margin, Some(0.10));

        app.apply_clear_analog_monte_carlo_criteria();

        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&app.project_yaml).unwrap();
        assert!(
            project.scenarios[0].analog.as_ref().unwrap().sweeps[0]
                .monte_carlo
                .as_ref()
                .unwrap()
                .criteria
                .is_none()
        );
    }
}
