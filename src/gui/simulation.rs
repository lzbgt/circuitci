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
                }
            }
        });

        draw_waveform_plot(ui, waveform, self.selected_probe);
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

fn draw_waveform_plot(ui: &mut egui::Ui, waveform: &WaveformView, probe_index: usize) {
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

#[cfg(test)]
mod tests {
    use super::parse_waveform_csv_text;

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
}
