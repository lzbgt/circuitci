use super::CircuitCiApp;
use super::analog::{
    AnalogAssertionDraft, AnalogScenarioChoice, AnalogScenarioDraft, analog_scenario_choices,
    append_analog_assertion, append_analog_transient_scenario,
};
use super::sketch::ProjectSnapshot;
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
            self.analog_assertion_editor(ui);
            ui.separator();
        }
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
            if ui.button("Add Analog Assertion").clicked() {
                self.apply_add_analog_assertion();
            }
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
                }
            }
        });

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
                }
            }
        });

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
    use super::{interpolated_value, parse_waveform_csv_text, waveform_measurement};

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
}
