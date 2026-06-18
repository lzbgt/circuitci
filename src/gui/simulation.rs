use super::CircuitCiApp;
use super::analog::{
    AnalogAssertionDraft, AnalogAssertionRemoveDraft, AnalogAssertionReplaceDraft,
    AnalogAssertionUiStatus, AnalogProbeAssertionsRemoveDraft, AnalogScenarioChoice,
    AnalogScenarioDraft, analog_probe_assertion_summaries, analog_scenario_choices,
    append_analog_assertion, append_analog_transient_scenario, remove_analog_assertion,
    remove_analog_assertions_for_probe, replace_analog_assertion, unique_analog_assertion_name,
};
use super::sketch::{ProjectSnapshot, SketchSelection};
use crate::reports::ValidationReport;
use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

impl CircuitCiApp {
    pub(super) fn simulation_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Simulation And Observation");
        ui.separator();
        if let Some(snapshot) = self.project_snapshot.clone() {
            self.analog_scenario_editor(ui, &snapshot);
            ui.separator();
            self.selected_probe_assertions_panel(ui);
            ui.separator();
            self.analog_assertion_editor(ui);
            ui.separator();
        }
        self.spice_deck_editor(ui);
        ui.separator();
        if self.report.is_some() {
            self.waveform_view(ui);
            ui.separator();
            let report = self.report.as_ref().expect("checked above");
            ui.label("Waveforms");
            if report.waveforms.is_empty() {
                ui.label("No waveform artifacts were emitted by the current scenario set.");
            } else {
                for waveform in &report.waveforms {
                    ui.monospace(waveform);
                }
            }
            ui.add_space(8.0);
            ui.label("Artifacts");
            if report.artifacts.is_empty() {
                ui.label("No artifacts were emitted.");
            } else {
                for artifact in &report.artifacts {
                    ui.monospace(artifact);
                }
            }
            ui.separator();
            self.findings_view(ui, report);
        } else {
            ui.label(
                "Run validation to observe SPICE waveforms, generated decks, and rule findings.",
            );
        }
    }

    fn analog_scenario_editor(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        ui.collapsing("Analog Transient Scenario", |ui| {
            initialize_analog_net_defaults(
                snapshot,
                &mut self.analog_ground_net,
                &mut self.analog_probe_net,
            );
            egui::Grid::new("analog_transient_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Scenario");
                    ui.text_edit_singleline(&mut self.analog_scenario_name);
                    ui.end_row();

                    ui.label("Ground net");
                    net_combo(
                        ui,
                        "analog_ground_net",
                        &mut self.analog_ground_net,
                        snapshot,
                    );
                    ui.end_row();

                    ui.label("Probe net");
                    net_combo(ui, "analog_probe_net", &mut self.analog_probe_net, snapshot);
                    ui.end_row();

                    ui.label("Probe name");
                    ui.text_edit_singleline(&mut self.analog_probe_name);
                    ui.end_row();

                    ui.label("Stop time");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_stop_time_us)
                            .speed(1.0)
                            .range(0.001..=1_000_000.0)
                            .suffix(" us"),
                    );
                    ui.end_row();

                    ui.label("Max step");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_max_step_us)
                            .speed(0.1)
                            .range(0.001..=1_000_000.0)
                            .suffix(" us"),
                    );
                    ui.end_row();
                });
            if ui.button("Add Analog Scenario").clicked() {
                self.apply_add_analog_scenario();
            }
        });
    }

    fn analog_assertion_editor(&mut self, ui: &mut egui::Ui) {
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices,
            Err(error) => {
                ui.collapsing("Analog Assertion", |ui| {
                    ui.label(format!("Analog scenarios unavailable: {error}"));
                });
                return;
            }
        };
        ui.collapsing("Analog Assertion", |ui| {
            if choices.is_empty() {
                ui.label("No analog scenario is available. Add one first.");
                return;
            }
            initialize_analog_assertion_defaults(
                &choices,
                &mut self.analog_assertion_scenario,
                &mut self.analog_assertion_probe,
                &mut self.analog_assertion_end_us,
            );
            let selected_scenario = choices
                .iter()
                .find(|scenario| scenario.name == self.analog_assertion_scenario);
            egui::Grid::new("analog_assertion_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Scenario");
                    analog_scenario_combo(
                        ui,
                        "analog_assertion_scenario",
                        &mut self.analog_assertion_scenario,
                        &choices,
                    );
                    ui.end_row();

                    ui.label("Assertion");
                    ui.text_edit_singleline(&mut self.analog_assertion_name);
                    ui.end_row();

                    ui.label("Probe");
                    analog_probe_combo(
                        ui,
                        "analog_assertion_probe",
                        &mut self.analog_assertion_probe,
                        selected_scenario,
                    );
                    ui.end_row();

                    ui.label("Aggregation");
                    string_combo(
                        ui,
                        "analog_assertion_aggregation",
                        &mut self.analog_assertion_aggregation,
                        &["sample", "min", "max"],
                    );
                    ui.end_row();

                    ui.label("Relation");
                    string_combo(
                        ui,
                        "analog_assertion_relation",
                        &mut self.analog_assertion_relation,
                        &["above", "below"],
                    );
                    ui.end_row();

                    ui.label("Threshold");
                    let unit = selected_scenario
                        .and_then(|scenario| {
                            scenario
                                .probes
                                .iter()
                                .find(|probe| probe.name == self.analog_assertion_probe)
                        })
                        .map(|probe| match probe.quantity.as_str() {
                            "current" => " A",
                            "power" => " W",
                            _ => " V",
                        })
                        .unwrap_or(" V");
                    ui.add(
                        egui::DragValue::new(&mut self.analog_assertion_threshold)
                            .speed(0.1)
                            .suffix(unit),
                    );
                    ui.end_row();

                    if self.analog_assertion_aggregation == "sample" {
                        ui.label("At");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_assertion_at_us)
                                .speed(1.0)
                                .range(0.0..=1_000_000.0)
                                .suffix(" us"),
                        );
                        ui.end_row();
                    } else {
                        ui.label("Start");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_assertion_start_us)
                                .speed(1.0)
                                .range(0.0..=1_000_000.0)
                                .suffix(" us"),
                        );
                        ui.end_row();

                        ui.label("End");
                        ui.add(
                            egui::DragValue::new(&mut self.analog_assertion_end_us)
                                .speed(1.0)
                                .range(0.0..=1_000_000.0)
                                .suffix(" us"),
                        );
                        ui.end_row();
                    }
                });
            if self.analog_assertion_edit_original.trim().is_empty() {
                if ui.button("Add Analog Assertion").clicked() {
                    self.apply_add_analog_assertion();
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Editing {}",
                        self.analog_assertion_edit_original.trim()
                    ));
                    if ui.button("Save Assertion").clicked() {
                        self.apply_replace_analog_assertion();
                    }
                    if ui.button("Cancel Edit").clicked() {
                        self.analog_assertion_edit_original.clear();
                    }
                });
            }
        });
    }

    fn selected_probe_assertions_panel(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Selected Probe Assertions", |ui| {
            if self.analog_assertion_scenario.trim().is_empty()
                || self.analog_assertion_probe.trim().is_empty()
            {
                ui.label("Select a schematic probe badge to inspect its assertions.");
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
                if ui.button("Clear assertions").clicked() {
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
                ui.label("No assertions reference this probe.");
                return;
            }
            egui::Grid::new("selected_probe_assertions")
                .num_columns(6)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Status");
                    ui.strong("Assertion");
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

    fn apply_add_analog_scenario(&mut self) {
        let draft = AnalogScenarioDraft {
            name: self.analog_scenario_name.clone(),
            ground_net: self.analog_ground_net.clone(),
            probe_net: self.analog_probe_net.clone(),
            probe_name: self.analog_probe_name.clone(),
            stop_time_us: self.analog_stop_time_us,
            max_step_us: self.analog_max_step_us,
        };
        match append_analog_transient_scenario(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Analog scenario {} added.",
                    self.analog_scenario_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_add_analog_assertion(&mut self) {
        let draft = AnalogAssertionDraft {
            scenario_name: self.analog_assertion_scenario.clone(),
            assertion_name: self.analog_assertion_name.clone(),
            probe_name: self.analog_assertion_probe.clone(),
            aggregation: self.analog_assertion_aggregation.clone(),
            relation: self.analog_assertion_relation.clone(),
            threshold: self.analog_assertion_threshold,
            at_us: self.analog_assertion_at_us,
            start_us: self.analog_assertion_start_us,
            end_us: self.analog_assertion_end_us,
        };
        match append_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Analog assertion {} added.",
                    self.analog_assertion_name.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_replace_analog_assertion(&mut self) {
        let original_assertion_name = self.analog_assertion_edit_original.trim().to_string();
        let draft = AnalogAssertionReplaceDraft {
            scenario_name: self.analog_assertion_scenario.clone(),
            original_assertion_name: original_assertion_name.clone(),
            replacement: AnalogAssertionDraft {
                scenario_name: self.analog_assertion_scenario.clone(),
                assertion_name: self.analog_assertion_name.clone(),
                probe_name: self.analog_assertion_probe.clone(),
                aggregation: self.analog_assertion_aggregation.clone(),
                relation: self.analog_assertion_relation.clone(),
                threshold: self.analog_assertion_threshold,
                at_us: self.analog_assertion_at_us,
                start_us: self.analog_assertion_start_us,
                end_us: self.analog_assertion_end_us,
            },
        };
        match replace_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_edit_original.clear();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Analog assertion {original_assertion_name} updated."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn load_analog_assertion_editor(&mut self, draft: &AnalogAssertionDraft, original_name: &str) {
        self.analog_assertion_scenario = draft.scenario_name.clone();
        self.analog_assertion_name = draft.assertion_name.clone();
        self.analog_assertion_edit_original = original_name.to_string();
        self.analog_assertion_probe = draft.probe_name.clone();
        self.analog_assertion_aggregation = draft.aggregation.clone();
        self.analog_assertion_relation = draft.relation.clone();
        self.analog_assertion_threshold = draft.threshold;
        self.analog_assertion_at_us = draft.at_us;
        self.analog_assertion_start_us = draft.start_us;
        self.analog_assertion_end_us = draft.end_us;
        self.status = format!("Editing analog assertion {original_name}.");
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
                        "Removed assertion {} in scenario {}.",
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
            aggregation: self.analog_assertion_aggregation.clone(),
            relation: self.analog_assertion_relation.clone(),
            threshold: self.analog_assertion_threshold,
            at_us: self.waveform_cursor_a_us,
            start_us: self.analog_assertion_start_us,
            end_us: self.analog_assertion_end_us,
        };
        match append_analog_assertion(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_scenario = scenario_name.to_string();
                self.analog_assertion_probe = probe_name.to_string();
                self.analog_assertion_name = assertion_name.clone();
                self.analog_assertion_edit_original.clear();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Analog assertion {assertion_name} added from canvas probe badge."),
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
                        "Removed assertions for probe {} in scenario {}.",
                        draft.probe_name, draft.scenario_name
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn waveform_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Waveform Viewer");
            if self.waveforms.is_empty() {
                ui.label("No parsed CSV waveform is available.");
            }
        });
        if self.waveforms.is_empty() {
            return;
        }

        self.selected_waveform = self.selected_waveform.min(self.waveforms.len() - 1);
        ui.horizontal_wrapped(|ui| {
            for (index, waveform) in self.waveforms.iter().enumerate() {
                if ui
                    .selectable_label(self.selected_waveform == index, &waveform.label)
                    .clicked()
                {
                    self.selected_waveform = index;
                    self.selected_probe = 0;
                    self.waveform_cursor_a_us = 0.0;
                    self.waveform_cursor_b_us = 0.0;
                    self.waveform_playing = false;
                }
            }
        });

        {
            let waveform = &self.waveforms[self.selected_waveform];
            if waveform.probes.is_empty() {
                ui.label("Waveform has no probe columns.");
                return;
            }

            self.selected_probe = self.selected_probe.min(waveform.probes.len() - 1);
            ui.horizontal_wrapped(|ui| {
                for (index, probe) in waveform.probes.iter().enumerate() {
                    if ui
                        .selectable_label(self.selected_probe == index, &probe.label)
                        .clicked()
                    {
                        self.selected_probe = index;
                        self.waveform_cursor_a_us = 0.0;
                        self.waveform_cursor_b_us = 0.0;
                        self.waveform_playing = false;
                    }
                }
            });
        }

        self.waveform_playback_panel(ui);
        let waveform = &self.waveforms[self.selected_waveform];
        waveform_measurement_panel(
            ui,
            waveform,
            self.selected_probe,
            &mut self.waveform_cursor_a_us,
            &mut self.waveform_cursor_b_us,
        );
        draw_waveform_plot(
            ui,
            waveform,
            self.selected_probe,
            self.waveform_cursor_a_us,
            self.waveform_cursor_b_us,
        );
    }

    fn waveform_playback_panel(&mut self, ui: &mut egui::Ui) {
        let Some((start_us, end_us)) =
            waveform_time_range_for_view(&self.waveforms, self.selected_waveform)
        else {
            return;
        };
        if self.waveform_cursor_a_us < start_us || self.waveform_cursor_a_us > end_us {
            self.waveform_cursor_a_us = start_us;
        }
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Simulation Time");
                if ui
                    .button(if self.waveform_playing {
                        "Pause"
                    } else {
                        "Play"
                    })
                    .clicked()
                {
                    self.waveform_playing = !self.waveform_playing;
                }
                if ui.button("Start").clicked() {
                    self.waveform_cursor_a_us = start_us;
                    self.waveform_cursor_b_us = start_us;
                    self.waveform_playing = false;
                }
                ui.add(
                    egui::Slider::new(&mut self.waveform_cursor_a_us, start_us..=end_us)
                        .text("time")
                        .suffix(" us")
                        .show_value(true),
                );
                self.waveform_cursor_b_us = self.waveform_cursor_a_us;
                ui.label("speed");
                ui.add(
                    egui::DragValue::new(&mut self.waveform_playback_speed)
                        .speed(0.1)
                        .range(0.1..=1000.0)
                        .suffix("x"),
                );
            });
            ui.small(
                "Cursor A drives graph hover probes and runtime node tinting. Cursor B follows during playback/scrub.",
            );
        });
    }
}

#[derive(Debug, Clone)]
pub(super) struct WaveformView {
    label: String,
    path: String,
    time_s: Vec<f64>,
    probes: Vec<WaveformProbe>,
}

#[derive(Debug, Clone)]
struct WaveformProbe {
    label: String,
    values: Vec<f64>,
}

pub(super) fn load_report_waveforms(report: &ValidationReport) -> Vec<WaveformView> {
    report
        .waveforms
        .iter()
        .filter_map(|waveform| load_waveform_csv(Path::new(waveform), waveform).ok())
        .collect()
}

pub(super) fn runtime_probe_lines_for_selection(
    waveforms: &[WaveformView],
    waveform_index: usize,
    cursor_us: f64,
    selection: &SketchSelection,
    snapshot: &ProjectSnapshot,
) -> Vec<String> {
    let Some(waveform) = waveforms.get(waveform_index) else {
        return Vec::new();
    };
    let target = match runtime_probe_target(selection, snapshot) {
        Some(target) => target,
        None => return Vec::new(),
    };
    let cursor_s = cursor_us / 1e6;
    let mut lines = Vec::new();
    for probe in &waveform.probes {
        if !probe_matches_target(&probe.label, &target) {
            continue;
        }
        if let Some(value) = interpolated_value(&waveform.time_s, &probe.values, cursor_s) {
            lines.push(format!(
                "{} @ {} = {} {}",
                probe.label,
                format_time_s(cursor_s),
                format_value(value),
                probe_unit(&probe.label)
            ));
        }
        if lines.len() >= 6 {
            break;
        }
    }
    lines
}

pub(super) fn runtime_probe_activity_for_selection(
    waveforms: &[WaveformView],
    waveform_index: usize,
    cursor_us: f64,
    selection: &SketchSelection,
    snapshot: &ProjectSnapshot,
) -> Option<f64> {
    let waveform = waveforms.get(waveform_index)?;
    let target = runtime_probe_target(selection, snapshot)?;
    let cursor_s = cursor_us / 1e6;
    let mut activity: f64 = 0.0;
    let mut matched = false;
    for probe in &waveform.probes {
        if !probe_matches_target(&probe.label, &target) {
            continue;
        }
        let value = interpolated_value(&waveform.time_s, &probe.values, cursor_s)?;
        let range = min_max(&probe.values)?;
        let scale = range.0.abs().max(range.1.abs()).max(1.0e-12);
        activity = activity.max((value.abs() / scale).clamp(0.0, 1.0));
        matched = true;
    }
    matched.then_some(activity)
}

pub(super) fn waveform_time_range_for_view(
    waveforms: &[WaveformView],
    waveform_index: usize,
) -> Option<(f64, f64)> {
    waveforms
        .get(waveform_index)
        .and_then(waveform_time_range_us)
}

struct RuntimeProbeTarget {
    component_id: Option<String>,
    net_ids: Vec<String>,
}

fn runtime_probe_target(
    selection: &SketchSelection,
    snapshot: &ProjectSnapshot,
) -> Option<RuntimeProbeTarget> {
    match selection {
        SketchSelection::Net(net_id) => Some(RuntimeProbeTarget {
            component_id: None,
            net_ids: vec![net_id.clone()],
        }),
        SketchSelection::Component(component_id) => {
            let component = snapshot
                .components_detail
                .iter()
                .find(|component| &component.id == component_id)?;
            let mut net_ids = Vec::new();
            for pin in &component.pins {
                if !net_ids.contains(&pin.net) {
                    net_ids.push(pin.net.clone());
                }
            }
            Some(RuntimeProbeTarget {
                component_id: Some(component_id.clone()),
                net_ids,
            })
        }
        SketchSelection::Overflow(_) => None,
    }
}

fn probe_matches_target(label: &str, target: &RuntimeProbeTarget) -> bool {
    let normalized_label = normalized_probe_token(label);
    if let Some(component_id) = &target.component_id {
        let component = normalized_probe_token(component_id);
        if !component.is_empty() && normalized_label.contains(&component) {
            return true;
        }
    }
    target.net_ids.iter().any(|net_id| {
        let net = normalized_probe_token(net_id);
        !net.is_empty() && normalized_label.contains(&net)
    })
}

fn normalized_probe_token(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn probe_unit(label: &str) -> &'static str {
    let normalized = label.trim().to_ascii_lowercase();
    if normalized.starts_with("i(")
        || normalized.starts_with("i_")
        || normalized.contains("current")
    {
        "A"
    } else if normalized.starts_with("p(")
        || normalized.starts_with("p_")
        || normalized.contains("power")
    {
        "W"
    } else {
        "V"
    }
}

fn initialize_analog_net_defaults(
    snapshot: &ProjectSnapshot,
    ground_net: &mut String,
    probe_net: &mut String,
) {
    if ground_net.is_empty()
        && let Some(net) = snapshot.nets_detail.iter().find(|net| net.kind == "ground")
    {
        *ground_net = net.id.clone();
    }
    if probe_net.is_empty()
        && let Some(net) = snapshot
            .nets_detail
            .iter()
            .find(|net| net.kind != "ground")
            .or_else(|| snapshot.nets_detail.first())
    {
        *probe_net = net.id.clone();
    }
}

fn net_combo(ui: &mut egui::Ui, id: &str, selected: &mut String, snapshot: &ProjectSnapshot) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select net"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for net in &snapshot.nets_detail {
                ui.selectable_value(selected, net.id.clone(), &net.id);
            }
        });
}

fn initialize_analog_assertion_defaults(
    choices: &[AnalogScenarioChoice],
    scenario_name: &mut String,
    probe_name: &mut String,
    end_us: &mut f64,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == *scenario_name);
    if (scenario_name.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        *scenario_name = scenario.name.clone();
        *end_us = scenario.stop_time_us;
    }
    let selected_scenario = choices
        .iter()
        .find(|scenario| scenario.name == *scenario_name)
        .or_else(|| choices.first());
    if let Some(scenario) = selected_scenario {
        let probe_missing = !scenario
            .probes
            .iter()
            .any(|probe| probe.name == *probe_name);
        if (probe_name.is_empty() || probe_missing)
            && let Some(probe) = scenario.probes.first()
        {
            *probe_name = probe.name.clone();
        }
        if *end_us <= 0.0 || *end_us > scenario.stop_time_us {
            *end_us = scenario.stop_time_us;
        }
    }
}

fn analog_scenario_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    choices: &[AnalogScenarioChoice],
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select scenario"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for scenario in choices {
                ui.selectable_value(selected, scenario.name.clone(), &scenario.name);
            }
        });
}

fn analog_probe_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    scenario: Option<&AnalogScenarioChoice>,
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select probe"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            if let Some(scenario) = scenario {
                for probe in &scenario.probes {
                    ui.selectable_value(
                        selected,
                        probe.name.clone(),
                        format!("{} ({})", probe.name, probe.quantity),
                    );
                }
            }
        });
}

fn assertion_status_color(status: AnalogAssertionUiStatus) -> egui::Color32 {
    match status {
        AnalogAssertionUiStatus::Unknown => egui::Color32::from_rgb(230, 190, 90),
        AnalogAssertionUiStatus::Pass => egui::Color32::from_rgb(86, 190, 112),
        AnalogAssertionUiStatus::Fail => egui::Color32::from_rgb(232, 83, 83),
    }
}

fn string_combo(ui: &mut egui::Ui, id: &str, selected: &mut String, values: &[&str]) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected.as_str())
        .show_ui(ui, |ui| {
            for value in values {
                ui.selectable_value(selected, (*value).to_string(), *value);
            }
        });
}

fn load_waveform_csv(path: &Path, label: &str) -> Result<WaveformView> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read waveform CSV {}.", path.display()))?;
    parse_waveform_csv_text(&text, label)
}

fn parse_waveform_csv_text(text: &str, label: &str) -> Result<WaveformView> {
    let mut time_s = Vec::new();
    let mut probe_labels = Vec::new();
    let mut probe_values: Vec<Vec<f64>> = Vec::new();

    for (line_index, line) in text.lines().enumerate() {
        let fields = split_waveform_fields(line);
        if fields.is_empty() {
            continue;
        }
        let Some(time) = parse_waveform_float(fields[0]) else {
            if time_s.is_empty() {
                probe_labels = fields
                    .iter()
                    .skip(1)
                    .map(|field| (*field).to_string())
                    .collect();
                continue;
            }
            anyhow::bail!(
                "Waveform row {} has non-numeric time value {}.",
                line_index + 1,
                fields[0]
            );
        };
        if let Some(previous) = time_s.last()
            && time <= *previous
        {
            anyhow::bail!(
                "Waveform row {} has non-increasing time value {}.",
                line_index + 1,
                fields[0]
            );
        }
        let probe_count = fields.len().saturating_sub(1);
        if probe_count == 0 {
            anyhow::bail!("Waveform row {} has no probe columns.", line_index + 1);
        }
        if probe_values.is_empty() {
            probe_values = vec![Vec::new(); probe_count];
            if probe_labels.len() != probe_count {
                probe_labels = (0..probe_count)
                    .map(|index| format!("probe_{}", index + 1))
                    .collect();
            }
        } else if probe_count < probe_values.len() {
            anyhow::bail!(
                "Waveform row {} has {} probe columns, expected at least {}.",
                line_index + 1,
                probe_count,
                probe_values.len()
            );
        }
        time_s.push(time);
        for (index, values) in probe_values.iter_mut().enumerate() {
            let value = parse_waveform_float(fields[index + 1]).with_context(|| {
                format!(
                    "Waveform row {} has non-numeric probe value {}.",
                    line_index + 1,
                    fields[index + 1]
                )
            })?;
            values.push(value);
        }
    }

    if time_s.is_empty() {
        anyhow::bail!("Waveform CSV has no numeric samples.");
    }

    let probes = probe_labels
        .into_iter()
        .zip(probe_values)
        .map(|(label, values)| WaveformProbe { label, values })
        .collect();
    Ok(WaveformView {
        label: label.to_string(),
        path: label.to_string(),
        time_s,
        probes,
    })
}

fn split_waveform_fields(line: &str) -> Vec<&str> {
    line.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

fn parse_waveform_float(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

fn waveform_measurement_panel(
    ui: &mut egui::Ui,
    waveform: &WaveformView,
    probe_index: usize,
    cursor_a_us: &mut f64,
    cursor_b_us: &mut f64,
) {
    let Some((start_us, end_us)) = waveform_time_range_us(waveform) else {
        return;
    };
    if *cursor_a_us < start_us || *cursor_a_us > end_us {
        *cursor_a_us = start_us;
    }
    if *cursor_b_us < start_us || *cursor_b_us > end_us {
        *cursor_b_us = end_us;
    }

    ui.group(|ui| {
        ui.horizontal_wrapped(|ui| {
            ui.strong("Measurements");
            ui.label(format!(
                "range {}",
                format_time_s((end_us - start_us) / 1e6)
            ));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Cursor A");
            ui.add(
                egui::DragValue::new(cursor_a_us)
                    .speed(((end_us - start_us) / 200.0).max(0.001))
                    .range(start_us..=end_us)
                    .suffix(" us"),
            );
            ui.label("Cursor B");
            ui.add(
                egui::DragValue::new(cursor_b_us)
                    .speed(((end_us - start_us) / 200.0).max(0.001))
                    .range(start_us..=end_us)
                    .suffix(" us"),
            );
        });

        if let Some(measurement) =
            waveform_measurement(waveform, probe_index, *cursor_a_us, *cursor_b_us)
        {
            egui::Grid::new("waveform_measurements")
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("A");
                    ui.monospace(format_time_s(measurement.cursor_a.time_s));
                    ui.label("value");
                    ui.monospace(format_value(measurement.cursor_a.value));
                    ui.end_row();

                    ui.label("B");
                    ui.monospace(format_time_s(measurement.cursor_b.time_s));
                    ui.label("value");
                    ui.monospace(format_value(measurement.cursor_b.value));
                    ui.end_row();

                    ui.label("Delta");
                    ui.monospace(format_time_s(measurement.delta_t_s));
                    ui.label("value");
                    ui.monospace(format_value(measurement.delta_value));
                    ui.end_row();

                    ui.label("Probe min");
                    ui.monospace(format_value(measurement.full_min));
                    ui.label("max");
                    ui.monospace(format_value(measurement.full_max));
                    ui.end_row();

                    ui.label("Cursor min");
                    ui.monospace(format_value(measurement.window_min));
                    ui.label("max");
                    ui.monospace(format_value(measurement.window_max));
                    ui.end_row();
                });
        }
    });
}

fn draw_waveform_plot(
    ui: &mut egui::Ui,
    waveform: &WaveformView,
    probe_index: usize,
    cursor_a_us: f64,
    cursor_b_us: f64,
) {
    let probe = &waveform.probes[probe_index];
    let Some((x_min, x_max)) = min_max(&waveform.time_s) else {
        ui.label("Waveform has no time samples.");
        return;
    };
    let Some((y_min, y_max)) = min_max(&probe.values) else {
        ui.label("Selected probe has no samples.");
        return;
    };

    ui.label(format!(
        "{} samples from {}",
        waveform.time_s.len(),
        waveform.path
    ));
    let desired_size = egui::vec2(ui.available_width().max(360.0), 300.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(16));

    let plot_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(56.0, 16.0),
        rect.max - egui::vec2(16.0, 38.0),
    );
    draw_plot_frame(&painter, plot_rect);

    let x_span = positive_span(x_min, x_max);
    let y_span = positive_span(y_min, y_max);
    let map_point = |x: f64, y: f64| -> egui::Pos2 {
        let x_ratio = ((x - x_min) / x_span).clamp(0.0, 1.0) as f32;
        let y_ratio = ((y - y_min) / y_span).clamp(0.0, 1.0) as f32;
        egui::pos2(
            plot_rect.left() + x_ratio * plot_rect.width(),
            plot_rect.bottom() - y_ratio * plot_rect.height(),
        )
    };

    draw_cursor_line(
        &painter,
        plot_rect,
        cursor_a_us / 1e6,
        x_min,
        x_span,
        egui::Color32::from_rgb(255, 196, 87),
        "A",
    );
    draw_cursor_line(
        &painter,
        plot_rect,
        cursor_b_us / 1e6,
        x_min,
        x_span,
        egui::Color32::from_rgb(135, 220, 140),
        "B",
    );

    for tick in 0..=4 {
        let ratio = tick as f32 / 4.0;
        let x = plot_rect.left() + ratio * plot_rect.width();
        painter.line_segment(
            [
                egui::pos2(x, plot_rect.top()),
                egui::pos2(x, plot_rect.bottom()),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_gray(44)),
        );
        let y = plot_rect.top() + ratio * plot_rect.height();
        painter.line_segment(
            [
                egui::pos2(plot_rect.left(), y),
                egui::pos2(plot_rect.right(), y),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_gray(44)),
        );
    }

    let points: Vec<_> = waveform
        .time_s
        .iter()
        .copied()
        .zip(probe.values.iter().copied())
        .map(|(x, y)| map_point(x, y))
        .collect();
    if points.len() >= 2 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(93, 185, 255)),
        ));
    }

    let font = egui::FontId::monospace(12.0);
    painter.text(
        egui::pos2(plot_rect.left(), rect.bottom() - 22.0),
        egui::Align2::LEFT_CENTER,
        format!("t {:.3e}..{:.3e} s", x_min, x_max),
        font.clone(),
        egui::Color32::LIGHT_GRAY,
    );
    painter.text(
        egui::pos2(plot_rect.left(), rect.top() + 8.0),
        egui::Align2::LEFT_CENTER,
        format!("{} {:.3e}..{:.3e}", probe.label, y_min, y_max),
        font,
        egui::Color32::LIGHT_GRAY,
    );
}

fn draw_cursor_line(
    painter: &egui::Painter,
    plot_rect: egui::Rect,
    time_s: f64,
    x_min: f64,
    x_span: f64,
    color: egui::Color32,
    label: &str,
) {
    let ratio = ((time_s - x_min) / x_span).clamp(0.0, 1.0) as f32;
    let x = plot_rect.left() + ratio * plot_rect.width();
    painter.line_segment(
        [
            egui::pos2(x, plot_rect.top()),
            egui::pos2(x, plot_rect.bottom()),
        ],
        egui::Stroke::new(1.5, color),
    );
    painter.text(
        egui::pos2(x + 4.0, plot_rect.top() + 8.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::monospace(12.0),
        color,
    );
}

fn draw_plot_frame(painter: &egui::Painter, rect: egui::Rect) {
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(96));
    painter.line_segment([rect.left_top(), rect.right_top()], stroke);
    painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_bottom(), rect.left_bottom()], stroke);
    painter.line_segment([rect.left_bottom(), rect.left_top()], stroke);
}

fn min_max(values: &[f64]) -> Option<(f64, f64)> {
    let mut iter = values.iter().copied();
    let first = iter.next()?;
    let (min, max) = iter.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    });
    Some((min, max))
}

fn positive_span(min: f64, max: f64) -> f64 {
    let span = max - min;
    if span.abs() < f64::EPSILON { 1.0 } else { span }
}

#[derive(Debug, Clone, Copy)]
struct WaveformCursor {
    time_s: f64,
    value: f64,
}

#[derive(Debug, Clone, Copy)]
struct WaveformMeasurement {
    cursor_a: WaveformCursor,
    cursor_b: WaveformCursor,
    delta_t_s: f64,
    delta_value: f64,
    full_min: f64,
    full_max: f64,
    window_min: f64,
    window_max: f64,
}

fn waveform_measurement(
    waveform: &WaveformView,
    probe_index: usize,
    cursor_a_us: f64,
    cursor_b_us: f64,
) -> Option<WaveformMeasurement> {
    let probe = waveform.probes.get(probe_index)?;
    let full_range = min_max(&probe.values)?;
    let cursor_a = cursor_measurement(waveform, probe, cursor_a_us)?;
    let cursor_b = cursor_measurement(waveform, probe, cursor_b_us)?;
    let (start_s, end_s) = ordered_pair(cursor_a.time_s, cursor_b.time_s);
    let window_range = window_min_max(&waveform.time_s, &probe.values, start_s, end_s).unwrap_or((
        cursor_a.value.min(cursor_b.value),
        cursor_a.value.max(cursor_b.value),
    ));
    Some(WaveformMeasurement {
        cursor_a,
        cursor_b,
        delta_t_s: cursor_b.time_s - cursor_a.time_s,
        delta_value: cursor_b.value - cursor_a.value,
        full_min: full_range.0,
        full_max: full_range.1,
        window_min: window_range.0,
        window_max: window_range.1,
    })
}

fn cursor_measurement(
    waveform: &WaveformView,
    probe: &WaveformProbe,
    cursor_us: f64,
) -> Option<WaveformCursor> {
    let cursor_s = cursor_us / 1e6;
    Some(WaveformCursor {
        time_s: cursor_s,
        value: interpolated_value(&waveform.time_s, &probe.values, cursor_s)?,
    })
}

fn waveform_time_range_us(waveform: &WaveformView) -> Option<(f64, f64)> {
    let first = *waveform.time_s.first()? * 1e6;
    let last = *waveform.time_s.last()? * 1e6;
    Some((first, last))
}

fn interpolated_value(times: &[f64], values: &[f64], time_s: f64) -> Option<f64> {
    if times.len() != values.len() || times.is_empty() || !time_s.is_finite() {
        return None;
    }
    if time_s <= times[0] {
        return Some(values[0]);
    }
    let last_index = times.len() - 1;
    if time_s >= times[last_index] {
        return Some(values[last_index]);
    }
    for index in 1..times.len() {
        let left_t = times[index - 1];
        let right_t = times[index];
        if time_s <= right_t {
            let span = right_t - left_t;
            if span.abs() < f64::EPSILON {
                return Some(values[index]);
            }
            let ratio = (time_s - left_t) / span;
            return Some(values[index - 1] + ratio * (values[index] - values[index - 1]));
        }
    }
    None
}

fn window_min_max(times: &[f64], values: &[f64], start_s: f64, end_s: f64) -> Option<(f64, f64)> {
    if times.len() != values.len() || times.is_empty() {
        return None;
    }
    let start_value = interpolated_value(times, values, start_s)?;
    let end_value = interpolated_value(times, values, end_s)?;
    let mut min = start_value.min(end_value);
    let mut max = start_value.max(end_value);
    for (time, value) in times.iter().copied().zip(values.iter().copied()) {
        if time >= start_s && time <= end_s {
            min = min.min(value);
            max = max.max(value);
        }
    }
    Some((min, max))
}

fn ordered_pair(left: f64, right: f64) -> (f64, f64) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn format_time_s(value: f64) -> String {
    format!("{value:.6e} s")
}

fn format_value(value: f64) -> String {
    format!("{value:.6e}")
}

#[cfg(test)]
mod tests {
    use super::{
        interpolated_value, parse_waveform_csv_text, runtime_probe_activity_for_selection,
        runtime_probe_lines_for_selection, waveform_measurement, waveform_time_range_for_view,
    };
    use crate::gui::sketch::{
        ProjectSnapshot, SketchComponent, SketchNet, SketchNodeStyle, SketchPin, SketchSelection,
    };

    #[test]
    fn waveform_parser_accepts_ngspice_header_and_samples() {
        let text = "time v(out) i(load)
0.0 0.0 0.001
1e-6 3.3 0.002
";
        let waveform = parse_waveform_csv_text(text, "waveform.csv").unwrap();
        assert_eq!(waveform.time_s, vec![0.0, 1e-6]);
        assert_eq!(waveform.probes[0].label, "v(out)");
        assert_eq!(waveform.probes[0].values, vec![0.0, 3.3]);
        assert_eq!(waveform.probes[1].label, "i(load)");
        assert_eq!(waveform.probes[1].values, vec![0.001, 0.002]);
    }

    #[test]
    fn waveform_parser_rejects_non_increasing_time() {
        let error = parse_waveform_csv_text(
            "time v(out)
1e-6 1.0
1e-6 2.0
",
            "waveform.csv",
        )
        .unwrap_err();
        assert!(error.to_string().contains("non-increasing time"));
    }

    #[test]
    fn interpolation_returns_linear_value_between_samples() {
        let value = interpolated_value(&[0.0, 1.0e-6, 2.0e-6], &[0.0, 2.0, 4.0], 1.5e-6).unwrap();
        assert!((value - 3.0).abs() < 1.0e-12);
    }

    #[test]
    fn waveform_measurement_reports_cursor_delta_and_ranges() {
        let waveform = parse_waveform_csv_text(
            "time v(out)
0.0 0.0
1e-6 2.0
2e-6 1.0
",
            "waveform.csv",
        )
        .unwrap();
        let measurement = waveform_measurement(&waveform, 0, 0.5, 1.5).unwrap();
        assert!((measurement.cursor_a.value - 1.0).abs() < 1.0e-12);
        assert!((measurement.cursor_b.value - 1.5).abs() < 1.0e-12);
        assert!((measurement.delta_t_s - 1.0e-6).abs() < 1.0e-18);
        assert!((measurement.delta_value - 0.5).abs() < 1.0e-12);
        assert_eq!(measurement.full_min, 0.0);
        assert_eq!(measurement.full_max, 2.0);
        assert_eq!(measurement.window_max, 2.0);
    }

    #[test]
    fn runtime_probe_lines_match_hovered_net() {
        let waveform = parse_waveform_csv_text(
            "time v(out) i(R1)
0.0 0.0 0.001
1e-6 3.3 0.003
",
            "waveform.csv",
        )
        .unwrap();
        let snapshot = probe_snapshot();
        let lines = runtime_probe_lines_for_selection(
            &[waveform],
            0,
            0.5,
            &SketchSelection::Net("out".to_string()),
            &snapshot,
        );
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("v(out)"));
        assert!(lines[0].contains("1.650000e0"));
    }

    #[test]
    fn runtime_probe_lines_match_hovered_component() {
        let waveform = parse_waveform_csv_text(
            "time v(out) i(R1)
0.0 0.0 0.001
1e-6 3.3 0.003
",
            "waveform.csv",
        )
        .unwrap();
        let snapshot = probe_snapshot();
        let lines = runtime_probe_lines_for_selection(
            &[waveform],
            0,
            1.0,
            &SketchSelection::Component("R1".to_string()),
            &snapshot,
        );
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().any(|line| line.contains("v(out)")));
        assert!(lines.iter().any(|line| line.contains("i(R1)")));
    }

    #[test]
    fn runtime_probe_activity_normalizes_matching_probe_value() {
        let waveform = parse_waveform_csv_text(
            "time v(out) i(R1)
0.0 0.0 0.000
1e-6 3.0 0.003
",
            "waveform.csv",
        )
        .unwrap();
        let snapshot = probe_snapshot();
        let activity = runtime_probe_activity_for_selection(
            &[waveform],
            0,
            0.5,
            &SketchSelection::Net("out".to_string()),
            &snapshot,
        )
        .unwrap();
        assert!((activity - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn runtime_probe_activity_ignores_unmatched_selection() {
        let waveform = parse_waveform_csv_text(
            "time v(other)
0.0 0.0
1e-6 3.0
",
            "waveform.csv",
        )
        .unwrap();
        let snapshot = probe_snapshot();
        let activity = runtime_probe_activity_for_selection(
            &[waveform],
            0,
            0.5,
            &SketchSelection::Net("out".to_string()),
            &snapshot,
        );
        assert_eq!(activity, None);
    }

    #[test]
    fn waveform_time_range_for_view_returns_microseconds() {
        let waveform = parse_waveform_csv_text(
            "time v(out)
0.0 0.0
2e-6 1.0
",
            "waveform.csv",
        )
        .unwrap();
        assert_eq!(
            waveform_time_range_for_view(&[waveform], 0),
            Some((0.0, 2.0))
        );
    }

    fn probe_snapshot() -> ProjectSnapshot {
        ProjectSnapshot {
            name: "probe_graph".to_string(),
            components: 1,
            nets: 2,
            scenarios: 1,
            libraries: Vec::new(),
            components_detail: vec![SketchComponent {
                id: "R1".to_string(),
                model: "generic.analog.resistor".to_string(),
                part_number: None,
                position: None,
                pins: vec![
                    SketchPin {
                        pin: "A".to_string(),
                        net: "out".to_string(),
                    },
                    SketchPin {
                        pin: "B".to_string(),
                        net: "gnd".to_string(),
                    },
                ],
                style: SketchNodeStyle::default(),
            }],
            nets_detail: vec![
                SketchNet {
                    id: "out".to_string(),
                    kind: "digital_or_analog".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: vec!["R1.A".to_string()],
                    position: None,
                },
                SketchNet {
                    id: "gnd".to_string(),
                    kind: "ground".to_string(),
                    nominal_voltage: Some(0.0),
                    powered: Some(true),
                    connections: vec!["R1.B".to_string()],
                    position: None,
                },
            ],
            probes: Vec::new(),
        }
    }
}
