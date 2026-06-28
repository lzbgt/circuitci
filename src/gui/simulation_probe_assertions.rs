use super::CircuitCiApp;
use super::analog::{
    AnalogAssertionDraft, AnalogAssertionRemoveDraft, AnalogProbeAssertionsRemoveDraft,
    analog_probe_assertion_summaries, append_analog_assertion, remove_analog_assertion,
    remove_analog_assertions_for_probe, unique_analog_assertion_name,
};
use super::simulation_forms::assertion_status_color;
use super::sketch_probes::SketchProbe;
use super::waveform::{format_value, quick_assertion_margin, waveform_probe_value_for_badge};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn selected_probe_assertions_panel(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Selected Probe Checks", |ui| {
            if self.analog_assertion_scenario.trim().is_empty()
                || self.analog_assertion_probe.trim().is_empty()
            {
                ui.label("Select a schematic probe element to inspect its checks.");
                return;
            }
            ui.horizontal(|ui| {
                ui.strong(format!(
                    "{} / {}",
                    self.analog_assertion_scenario, self.analog_assertion_probe
                ));
                if ui.button("Add from current settings").clicked() {
                    let scenario_name = self.analog_assertion_scenario.clone();
                    let probe_name = self.analog_assertion_probe.clone();
                    self.apply_add_canvas_probe_assertion(&scenario_name, &probe_name);
                }
                if ui.button("Clear checks").clicked() {
                    let scenario_name = self.analog_assertion_scenario.clone();
                    let probe_name = self.analog_assertion_probe.clone();
                    self.apply_remove_canvas_probe_assertions(&scenario_name, &probe_name);
                }
            });
            let rows = match analog_probe_assertion_summaries(
                &self.project_yaml,
                self.report.as_ref(),
                &self.analog_assertion_scenario,
                &self.analog_assertion_probe,
            ) {
                Ok(rows) => rows,
                Err(error) => {
                    ui.label(format!("Selected probe unavailable: {error}"));
                    return;
                }
            };
            if rows.is_empty() {
                ui.label("No checks reference this probe.");
                return;
            }
            egui::Grid::new("selected_probe_assertions")
                .num_columns(6)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Status");
                    ui.strong("Check");
                    ui.strong("Check");
                    ui.strong("Timing");
                    ui.strong("Failure");
                    ui.strong("Actions");
                    ui.end_row();
                    for row in rows {
                        ui.colored_label(assertion_status_color(row.status), row.status.label());
                        ui.monospace(&row.name);
                        ui.label(format!(
                            "{} {} {}",
                            row.aggregation, row.relation, row.threshold
                        ));
                        ui.label(row.timing);
                        if let Some(message) = row.failure_message {
                            ui.label(message);
                        } else {
                            ui.label("");
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Edit").clicked() {
                                self.load_analog_assertion_editor(&row.draft, &row.name);
                            }
                            if ui.button("Delete").clicked() {
                                self.apply_remove_analog_assertion(
                                    &row.draft.scenario_name,
                                    &row.name,
                                );
                            }
                        });
                        ui.end_row();
                    }
                });
        });
    }

    fn load_analog_assertion_editor(&mut self, draft: &AnalogAssertionDraft, original_name: &str) {
        self.analog_assertion_scenario = draft.scenario_name.clone();
        self.analog_assertion_name = draft.assertion_name.clone();
        self.analog_assertion_edit_original = original_name.to_string();
        self.analog_assertion_probe = draft.probe_name.clone();
        self.analog_assertion_reference_probe = draft.reference_probe.clone();
        self.analog_assertion_aggregation = draft.aggregation.clone();
        self.analog_assertion_relation = draft.relation.clone();
        self.analog_assertion_threshold = draft.threshold;
        self.analog_assertion_reference_threshold = draft.reference_threshold;
        self.analog_assertion_target = draft.target;
        self.analog_assertion_tolerance = draft.tolerance;
        self.analog_assertion_at_us = draft.at_us;
        self.analog_assertion_at_hz = draft.at_hz;
        self.analog_assertion_start_us = draft.start_us;
        self.analog_assertion_end_us = draft.end_us;
        self.analog_assertion_time_limit_us = draft.time_limit_us;
        self.analog_assertion_frequency_limit_hz = draft.frequency_limit_hz;
        self.analog_assertion_duty_limit_percent = draft.duty_limit_percent;
        self.analog_assertion_count_limit = draft.count_limit;
        self.analog_assertion_overshoot_limit_percent = draft.overshoot_limit_percent;
        self.status = format!("Editing observation check {original_name}.");
    }

    fn apply_remove_analog_assertion(&mut self, scenario_name: &str, assertion_name: &str) {
        let draft = AnalogAssertionRemoveDraft {
            scenario_name: scenario_name.to_string(),
            assertion_name: assertion_name.to_string(),
        };
        match remove_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                if self.analog_assertion_scenario == draft.scenario_name
                    && self.analog_assertion_edit_original == draft.assertion_name
                {
                    self.analog_assertion_edit_original.clear();
                }
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Removed check {} in run setup {}.",
                        draft.assertion_name, draft.scenario_name
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_add_canvas_probe_assertion(
        &mut self,
        scenario_name: &str,
        probe_name: &str,
    ) {
        let requested_name = if self.analog_assertion_name.trim().is_empty()
            || self.analog_assertion_name.trim() == "probe_above_min"
        {
            format!("{probe_name}_check")
        } else {
            self.analog_assertion_name.trim().to_string()
        };
        let assertion_name = match unique_analog_assertion_name(
            &self.project_yaml,
            scenario_name,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogAssertionDraft {
            scenario_name: scenario_name.to_string(),
            assertion_name: assertion_name.clone(),
            probe_name: probe_name.to_string(),
            reference_probe: self.analog_assertion_reference_probe.clone(),
            aggregation: self.analog_assertion_aggregation.clone(),
            relation: self.analog_assertion_relation.clone(),
            threshold: self.analog_assertion_threshold,
            reference_threshold: self.analog_assertion_reference_threshold,
            target: self.analog_assertion_target,
            tolerance: self.analog_assertion_tolerance,
            at_us: self.waveform_cursor_a_us,
            at_hz: self.analog_assertion_at_hz,
            start_us: self.analog_assertion_start_us,
            end_us: self.analog_assertion_end_us,
            time_limit_us: self.analog_assertion_time_limit_us,
            frequency_limit_hz: self.analog_assertion_frequency_limit_hz,
            duty_limit_percent: self.analog_assertion_duty_limit_percent,
            count_limit: self.analog_assertion_count_limit,
            overshoot_limit_percent: self.analog_assertion_overshoot_limit_percent,
        };
        match append_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_scenario = scenario_name.to_string();
                self.analog_assertion_probe = probe_name.to_string();
                self.analog_assertion_name = assertion_name.clone();
                self.analog_assertion_edit_original.clear();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Observation check {assertion_name} added from canvas probe element."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_quick_canvas_probe_assertion(
        &mut self,
        probe: &SketchProbe,
        relation: &str,
    ) {
        let Some(measured) = waveform_probe_value_for_badge(
            &self.waveforms,
            self.selected_waveform,
            self.waveform_cursor_a_us,
            probe,
        ) else {
            self.record_error(anyhow::anyhow!(
                "No loaded waveform sample matches probe {} at the current cursor.",
                probe.probe_name
            ));
            return;
        };
        let margin = quick_assertion_margin(measured);
        let threshold = match relation {
            "above" => measured - margin,
            "below" => measured + margin,
            _ => {
                self.record_error(anyhow::anyhow!(
                    "Quick check relation {relation} is not supported."
                ));
                return;
            }
        };
        let requested_name = format!("{}_{}_cursor", probe.probe_name, relation);
        let assertion_name = match unique_analog_assertion_name(
            &self.project_yaml,
            &probe.scenario_name,
            &requested_name,
        ) {
            Ok(name) => name,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        let draft = AnalogAssertionDraft {
            scenario_name: probe.scenario_name.clone(),
            assertion_name: assertion_name.clone(),
            probe_name: probe.probe_name.clone(),
            reference_probe: self.analog_assertion_reference_probe.clone(),
            aggregation: "sample".to_string(),
            relation: relation.to_string(),
            threshold,
            reference_threshold: self.analog_assertion_reference_threshold,
            target: self.analog_assertion_target,
            tolerance: self.analog_assertion_tolerance,
            at_us: self.waveform_cursor_a_us,
            at_hz: self.analog_assertion_at_hz,
            start_us: self.analog_assertion_start_us,
            end_us: self.analog_assertion_end_us,
            time_limit_us: self.analog_assertion_time_limit_us,
            frequency_limit_hz: self.analog_assertion_frequency_limit_hz,
            duty_limit_percent: self.analog_assertion_duty_limit_percent,
            count_limit: self.analog_assertion_count_limit,
            overshoot_limit_percent: self.analog_assertion_overshoot_limit_percent,
        };
        match append_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_scenario = draft.scenario_name.clone();
                self.analog_assertion_name = assertion_name.clone();
                self.analog_assertion_edit_original.clear();
                self.analog_assertion_probe = draft.probe_name.clone();
                self.analog_assertion_reference_probe = draft.reference_probe.clone();
                self.analog_assertion_aggregation = draft.aggregation.clone();
                self.analog_assertion_relation = draft.relation.clone();
                self.analog_assertion_threshold = draft.threshold;
                self.analog_assertion_reference_threshold = draft.reference_threshold;
                self.analog_assertion_target = draft.target;
                self.analog_assertion_tolerance = draft.tolerance;
                self.analog_assertion_at_us = draft.at_us;
                self.analog_assertion_time_limit_us = draft.time_limit_us;
                self.analog_assertion_duty_limit_percent = draft.duty_limit_percent;
                self.analog_assertion_count_limit = draft.count_limit;
                self.analog_assertion_overshoot_limit_percent = draft.overshoot_limit_percent;
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Quick check {assertion_name} added from {} = {}.",
                        probe.probe_name,
                        format_value(measured)
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_remove_canvas_probe_assertions(
        &mut self,
        scenario_name: &str,
        probe_name: &str,
    ) {
        let draft = AnalogProbeAssertionsRemoveDraft {
            scenario_name: scenario_name.to_string(),
            probe_name: probe_name.to_string(),
        };
        match remove_analog_assertions_for_probe(&self.project_yaml, &draft) {
            Ok(updated) => {
                if self.analog_assertion_scenario == draft.scenario_name
                    && self.analog_assertion_probe == draft.probe_name
                {
                    self.analog_assertion_name.clear();
                    self.analog_assertion_edit_original.clear();
                }
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Removed checks for probe {} in run setup {}.",
                        draft.probe_name, draft.scenario_name
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }
}
