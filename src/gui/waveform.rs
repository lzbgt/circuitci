use super::analog::{
    AnalogAssertionDraft, AnalogExpressionProbeDraft, analog_scenario_choices,
    append_analog_assertion, append_analog_expression_probe, unique_analog_assertion_name,
};
use super::sketch::{ProjectSnapshot, SketchSelection};
use super::sketch_probes::SketchProbe;
use super::{CircuitCiApp, ScopeProbeTarget, Stage};
use crate::reports::ValidationReport;
use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

impl CircuitCiApp {
    pub(super) fn open_scope_probe_target(&mut self, target: ScopeProbeTarget) {
        self.pending_scope_probe = Some(target.clone());
        let focused = self.focus_scope_probe(&target);
        self.stage = Stage::Simulation;
        if focused {
            self.status = format!(
                "Opened scope probe {} from scenario {}.",
                target.probe_name, target.scenario_name
            );
        } else {
            self.status = format!(
                "Scope target {} from scenario {} selected; run the model to load matching traces.",
                target.probe_name, target.scenario_name
            );
        }
    }

    pub(super) fn remember_scope_probe_target(&mut self, scenario_name: &str, probe_name: &str) {
        self.pending_scope_probe = Some(ScopeProbeTarget {
            scenario_name: scenario_name.trim().to_string(),
            probe_name: probe_name.trim().to_string(),
        });
    }

    pub(super) fn apply_pending_scope_probe_focus(&mut self) -> bool {
        let Some(target) = self.pending_scope_probe.clone() else {
            return false;
        };
        self.focus_scope_probe(&target)
    }

    fn focus_scope_probe(&mut self, target: &ScopeProbeTarget) -> bool {
        let Some((waveform_index, probe_index)) =
            find_scope_probe(&self.waveforms, &target.scenario_name, &target.probe_name)
        else {
            return false;
        };
        self.selected_waveform = waveform_index;
        self.selected_probe = probe_index;
        self.waveform_math_left = probe_index;
        self.waveform_math_right = probe_index;
        self.waveform_cursor_a_us = 0.0;
        self.waveform_cursor_b_us = 0.0;
        self.waveform_playing = false;
        true
    }

    pub(super) fn waveform_scope_view(&mut self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
        self.waveform_scope_header(ui);
        if self.waveforms.is_empty() {
            return;
        }

        self.waveform_selector(ui);
        self.waveform_probe_selector(ui);
        self.waveform_playback_panel(ui);
        self.waveform_scope_plot(ui, desired_size);
    }

    pub(super) fn waveform_controls_panel(&mut self, ui: &mut egui::Ui) {
        if self.waveforms.is_empty() {
            ui.label("Run a simulation to load scope traces.");
            return;
        }
        self.selected_waveform = self.selected_waveform.min(self.waveforms.len() - 1);
        let waveform = &self.waveforms[self.selected_waveform];
        if waveform.probes.is_empty() {
            ui.label("Waveform has no probe columns.");
            return;
        }
        self.selected_probe = self.selected_probe.min(waveform.probes.len() - 1);
        self.waveform_math_panel(ui);
        let waveform = &self.waveforms[self.selected_waveform];
        waveform_measurement_panel(
            ui,
            waveform,
            self.selected_probe,
            &mut self.waveform_cursor_a_us,
            &mut self.waveform_cursor_b_us,
        );
    }

    fn waveform_scope_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Scopes");
            if self.waveforms.is_empty() {
                ui.label("No parsed CSV waveform is available. Run the schematic model first.");
            }
        });
    }

    fn waveform_selector(&mut self, ui: &mut egui::Ui) {
        self.selected_waveform = self.selected_waveform.min(self.waveforms.len() - 1);
        ui.horizontal_wrapped(|ui| {
            for (index, waveform) in self.waveforms.iter().enumerate() {
                if ui
                    .selectable_label(self.selected_waveform == index, &waveform.label)
                    .clicked()
                {
                    self.selected_waveform = index;
                    self.selected_probe = 0;
                    self.waveform_math_left = 0;
                    self.waveform_math_right = 0;
                    self.waveform_math_name.clear();
                    self.waveform_cursor_a_us = 0.0;
                    self.waveform_cursor_b_us = 0.0;
                    self.waveform_playing = false;
                }
            }
        });
    }

    fn waveform_probe_selector(&mut self, ui: &mut egui::Ui) {
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
                    self.waveform_math_left =
                        self.waveform_math_left.min(waveform.probes.len() - 1);
                    self.waveform_math_right =
                        self.waveform_math_right.min(waveform.probes.len() - 1);
                    self.waveform_cursor_a_us = 0.0;
                    self.waveform_cursor_b_us = 0.0;
                    self.waveform_playing = false;
                }
            }
        });
    }

    fn waveform_scope_plot(&mut self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
        let waveform = &self.waveforms[self.selected_waveform];
        if waveform.probes.is_empty() {
            return;
        }
        draw_waveform_plot_sized(
            ui,
            waveform,
            self.selected_probe,
            self.waveform_cursor_a_us,
            self.waveform_cursor_b_us,
            scope_plot_size(desired_size),
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

    fn waveform_math_panel(&mut self, ui: &mut egui::Ui) {
        let Some(waveform) = self.waveforms.get(self.selected_waveform) else {
            return;
        };
        if waveform.probes.len() < 2 {
            return;
        }
        let probe_labels: Vec<String> = waveform
            .probes
            .iter()
            .map(|probe| probe.label.clone())
            .collect();
        let selected_probe_is_derived = waveform
            .probes
            .get(self.selected_probe)
            .is_some_and(|probe| probe.derived);
        let selected_promotion = waveform
            .probes
            .get(self.selected_probe)
            .filter(|probe| probe.derived)
            .map(|probe| WaveformPromotionChoice {
                label: probe.label.clone(),
                expression: probe
                    .expression
                    .clone()
                    .unwrap_or_else(|| probe.label.clone()),
                quantity: probe.promoted_quantity,
            });
        self.waveform_math_left = self.waveform_math_left.min(probe_labels.len() - 1);
        self.waveform_math_right = self.waveform_math_right.min(probe_labels.len() - 1);
        if self.waveform_math_right == self.waveform_math_left && probe_labels.len() > 1 {
            self.waveform_math_right = (self.waveform_math_left + 1).min(probe_labels.len() - 1);
        }
        ui.collapsing("Derived Waveform Channel", |ui| {
            egui::Grid::new("waveform_math_editor")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Left probe");
                    waveform_probe_combo(
                        ui,
                        "waveform_math_left",
                        &mut self.waveform_math_left,
                        &probe_labels,
                    );
                    ui.end_row();

                    ui.label("Operation");
                    waveform_math_operation_combo(ui, &mut self.waveform_math_operation);
                    ui.end_row();

                    ui.label("Right probe");
                    waveform_probe_combo(
                        ui,
                        "waveform_math_right",
                        &mut self.waveform_math_right,
                        &probe_labels,
                    );
                    ui.end_row();

                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.waveform_math_name);
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Add Derived Channel").clicked() {
                    self.apply_add_waveform_math_channel();
                }
                if ui
                    .add_enabled(
                        selected_probe_is_derived,
                        egui::Button::new("Remove Selected Derived"),
                    )
                    .clicked()
                {
                    self.apply_remove_selected_waveform_math_channel();
                }
            });
            if let Some(promotion) = selected_promotion {
                ui.separator();
                self.waveform_promotion_panel(ui, &promotion);
            }
        });
    }

    fn waveform_promotion_panel(&mut self, ui: &mut egui::Ui, promotion: &WaveformPromotionChoice) {
        ui.strong("Promote Selected Derived Channel");
        let Some(quantity) = promotion.quantity else {
            ui.label("This derived channel is dimensionless or mixed-unit and cannot be promoted as a Board IR analog probe.");
            return;
        };
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices,
            Err(error) => {
                ui.label(format!("Analog scenarios unavailable: {error}"));
                return;
            }
        };
        if choices.is_empty() {
            ui.label("No analog scenario is available.");
            return;
        }
        if self.waveform_promote_scenario.is_empty()
            || !choices
                .iter()
                .any(|choice| choice.name == self.waveform_promote_scenario)
        {
            self.waveform_promote_scenario = choices[0].name.clone();
        }
        if self.waveform_promote_probe_name.is_empty() {
            self.waveform_promote_probe_name = sanitized_probe_name(&promotion.label);
        }
        egui::Grid::new("waveform_promote_probe")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Scenario");
                waveform_analog_scenario_combo(ui, &choices, &mut self.waveform_promote_scenario);
                ui.end_row();

                ui.label("Probe name");
                ui.text_edit_singleline(&mut self.waveform_promote_probe_name);
                ui.end_row();

                ui.label("Quantity");
                ui.monospace(quantity.label());
                ui.end_row();

                ui.label("Expression");
                ui.monospace(&promotion.expression);
                ui.end_row();
            });
        ui.horizontal(|ui| {
            if ui.button("Promote As Board IR Probe").clicked() {
                self.apply_promote_waveform_channel(promotion, quantity, false);
            }
            if ui.button("Promote + Add Assertion").clicked() {
                self.apply_promote_waveform_channel(promotion, quantity, true);
            }
        });
    }

    fn apply_add_waveform_math_channel(&mut self) {
        let Some(waveform) = self.waveforms.get_mut(self.selected_waveform) else {
            return;
        };
        let draft = WaveformMathDraft {
            left_probe: self.waveform_math_left,
            right_probe: self.waveform_math_right,
            operation: self.waveform_math_operation.clone(),
            label: self.waveform_math_name.clone(),
        };
        match append_derived_waveform_probe(waveform, &draft) {
            Ok(index) => {
                self.selected_probe = index;
                self.status = format!(
                    "Derived waveform channel {} added.",
                    waveform.probes[index].label
                );
                self.waveform_math_name.clear();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_selected_waveform_math_channel(&mut self) {
        let Some(waveform) = self.waveforms.get_mut(self.selected_waveform) else {
            return;
        };
        let Some(probe) = waveform.probes.get(self.selected_probe) else {
            return;
        };
        if !probe.derived {
            return;
        }
        let label = probe.label.clone();
        waveform.probes.remove(self.selected_probe);
        self.selected_probe = self.selected_probe.saturating_sub(1);
        self.waveform_math_left = self
            .waveform_math_left
            .min(waveform.probes.len().saturating_sub(1));
        self.waveform_math_right = self
            .waveform_math_right
            .min(waveform.probes.len().saturating_sub(1));
        self.status = format!("Derived waveform channel {label} removed.");
    }

    fn apply_promote_waveform_channel(
        &mut self,
        promotion: &WaveformPromotionChoice,
        quantity: WaveformProbeQuantity,
        add_assertion: bool,
    ) {
        let draft = AnalogExpressionProbeDraft {
            scenario_name: self.waveform_promote_scenario.clone(),
            probe_name: self.waveform_promote_probe_name.clone(),
            expression: promotion.expression.clone(),
            quantity: quantity.label().to_string(),
        };
        match append_analog_expression_probe(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_scenario = draft.scenario_name.clone();
                self.analog_assertion_probe = draft.probe_name.clone();
                let updated = if add_assertion {
                    match self.append_assertion_for_promoted_waveform(&updated, &draft) {
                        Ok(updated) => updated,
                        Err(error) => {
                            self.record_error(error);
                            return;
                        }
                    }
                } else {
                    updated
                };
                let status = if add_assertion {
                    format!(
                        "Derived waveform channel {} promoted as analog probe {} with assertion.",
                        promotion.label, draft.probe_name
                    )
                } else {
                    format!(
                        "Derived waveform channel {} promoted as analog probe {}.",
                        promotion.label, draft.probe_name
                    )
                };
                self.apply_edited_project_yaml(updated, &status);
            }
            Err(error) => self.record_error(error),
        }
    }

    fn append_assertion_for_promoted_waveform(
        &self,
        yaml: &str,
        probe: &AnalogExpressionProbeDraft,
    ) -> anyhow::Result<String> {
        let requested_name = if self.analog_assertion_name.trim().is_empty() {
            format!("{}_check", probe.probe_name.trim())
        } else {
            sanitized_probe_name(&self.analog_assertion_name)
        };
        let assertion_name =
            unique_analog_assertion_name(yaml, &probe.scenario_name, &requested_name)?;
        append_analog_assertion(
            yaml,
            &AnalogAssertionDraft {
                scenario_name: probe.scenario_name.clone(),
                assertion_name,
                probe_name: probe.probe_name.clone(),
                aggregation: self.analog_assertion_aggregation.clone(),
                relation: self.analog_assertion_relation.clone(),
                threshold: self.analog_assertion_threshold,
                at_us: self.analog_assertion_at_us,
                start_us: self.analog_assertion_start_us,
                end_us: self.analog_assertion_end_us,
            },
        )
    }
}

fn find_scope_probe(
    waveforms: &[WaveformView],
    scenario_name: &str,
    probe_name: &str,
) -> Option<(usize, usize)> {
    let scenario_name = scenario_name.trim();
    let probe_name = probe_name.trim();
    if probe_name.is_empty() {
        return None;
    }
    waveforms
        .iter()
        .enumerate()
        .find_map(|(waveform_index, waveform)| {
            let scenario_matches = scenario_name.is_empty()
                || waveform.label.contains(scenario_name)
                || waveform.path.contains(scenario_name);
            if !scenario_matches {
                return None;
            }
            waveform
                .probes
                .iter()
                .position(|probe| probe.label.trim().eq_ignore_ascii_case(probe_name))
                .map(|probe_index| (waveform_index, probe_index))
        })
        .or_else(|| {
            waveforms
                .iter()
                .enumerate()
                .find_map(|(waveform_index, waveform)| {
                    waveform
                        .probes
                        .iter()
                        .position(|probe| probe.label.trim().eq_ignore_ascii_case(probe_name))
                        .map(|probe_index| (waveform_index, probe_index))
                })
        })
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
    derived: bool,
    expression: Option<String>,
    promoted_quantity: Option<WaveformProbeQuantity>,
}

#[derive(Debug, Clone)]
struct WaveformPromotionChoice {
    label: String,
    expression: String,
    quantity: Option<WaveformProbeQuantity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaveformProbeQuantity {
    Voltage,
    Current,
    Power,
}

impl WaveformProbeQuantity {
    fn label(self) -> &'static str {
        match self {
            Self::Voltage => "voltage",
            Self::Current => "current",
            Self::Power => "power",
        }
    }
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

pub(super) fn waveform_probe_value_for_badge(
    waveforms: &[WaveformView],
    waveform_index: usize,
    cursor_us: f64,
    probe: &SketchProbe,
) -> Option<f64> {
    let waveform = waveforms.get(waveform_index)?;
    let cursor_s = cursor_us / 1e6;
    let waveform_probe = waveform.probes.iter().find(|waveform_probe| {
        waveform_probe
            .label
            .trim()
            .eq_ignore_ascii_case(probe.probe_name.trim())
            || waveform_probe
                .label
                .trim()
                .eq_ignore_ascii_case(probe.expression.trim())
    })?;
    interpolated_value(&waveform.time_s, &waveform_probe.values, cursor_s)
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
pub(super) fn quick_assertion_margin(value: f64) -> f64 {
    (value.abs() * 0.01).max(1.0e-9)
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
        .map(|(label, values)| WaveformProbe {
            promoted_quantity: waveform_probe_quantity_from_label(&label),
            label,
            values,
            derived: false,
            expression: None,
        })
        .collect();
    Ok(WaveformView {
        label: label.to_string(),
        path: label.to_string(),
        time_s,
        probes,
    })
}

#[derive(Debug, Clone)]
struct WaveformMathDraft {
    left_probe: usize,
    right_probe: usize,
    operation: String,
    label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaveformMathOperation {
    Difference,
    Sum,
    Product,
    Ratio,
}

impl WaveformMathOperation {
    fn from_label(label: &str) -> Result<Self> {
        match label.trim() {
            "difference" => Ok(Self::Difference),
            "sum" => Ok(Self::Sum),
            "product" => Ok(Self::Product),
            "ratio" => Ok(Self::Ratio),
            other => anyhow::bail!("Unsupported waveform math operation {other}."),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Difference => "difference",
            Self::Sum => "sum",
            Self::Product => "product",
            Self::Ratio => "ratio",
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::Difference => "-",
            Self::Sum => "+",
            Self::Product => "*",
            Self::Ratio => "/",
        }
    }

    fn apply(self, left: f64, right: f64) -> Result<f64> {
        match self {
            Self::Difference => Ok(left - right),
            Self::Sum => Ok(left + right),
            Self::Product => Ok(left * right),
            Self::Ratio => {
                if right.abs() <= f64::EPSILON {
                    anyhow::bail!("Ratio denominator contains a zero sample.");
                }
                Ok(left / right)
            }
        }
    }
}

fn append_derived_waveform_probe(
    waveform: &mut WaveformView,
    draft: &WaveformMathDraft,
) -> Result<usize> {
    let operation = WaveformMathOperation::from_label(&draft.operation)?;
    let left = waveform
        .probes
        .get(draft.left_probe)
        .with_context(|| format!("Left probe index {} is out of range.", draft.left_probe))?;
    let right = waveform
        .probes
        .get(draft.right_probe)
        .with_context(|| format!("Right probe index {} is out of range.", draft.right_probe))?;
    if left.values.len() != right.values.len() || left.values.len() != waveform.time_s.len() {
        anyhow::bail!("Waveform probes must share the selected waveform time base.");
    }
    let values: Vec<f64> = left
        .values
        .iter()
        .copied()
        .zip(right.values.iter().copied())
        .map(|(left, right)| operation.apply(left, right))
        .collect::<Result<Vec<_>>>()?;
    if values.iter().any(|value| !value.is_finite()) {
        anyhow::bail!("Derived waveform channel produced a non-finite sample.");
    }
    let expression = format!("{} {} {}", left.label, operation.symbol(), right.label);
    let promoted_quantity =
        derived_waveform_quantity(operation, left.promoted_quantity, right.promoted_quantity);
    let label = unique_waveform_probe_label(
        waveform,
        &derived_waveform_label(&draft.label, operation, &left.label, &right.label),
    );
    waveform.probes.push(WaveformProbe {
        label,
        values,
        derived: true,
        expression: Some(expression),
        promoted_quantity,
    });
    Ok(waveform.probes.len() - 1)
}

fn waveform_probe_quantity_from_label(label: &str) -> Option<WaveformProbeQuantity> {
    let normalized = label.trim().to_ascii_lowercase().replace(' ', "");
    if normalized.starts_with("v(") {
        Some(WaveformProbeQuantity::Voltage)
    } else if normalized.starts_with("i(")
        || normalized.starts_with("-i(")
        || normalized.starts_with("abs(i(")
    {
        Some(WaveformProbeQuantity::Current)
    } else if normalized.contains("v(") && normalized.contains("i(") && normalized.contains('*') {
        Some(WaveformProbeQuantity::Power)
    } else {
        None
    }
}

fn derived_waveform_quantity(
    operation: WaveformMathOperation,
    left: Option<WaveformProbeQuantity>,
    right: Option<WaveformProbeQuantity>,
) -> Option<WaveformProbeQuantity> {
    match operation {
        WaveformMathOperation::Difference | WaveformMathOperation::Sum => {
            (left == right).then_some(left).flatten()
        }
        WaveformMathOperation::Product => match (left, right) {
            (Some(WaveformProbeQuantity::Voltage), Some(WaveformProbeQuantity::Current))
            | (Some(WaveformProbeQuantity::Current), Some(WaveformProbeQuantity::Voltage)) => {
                Some(WaveformProbeQuantity::Power)
            }
            _ => None,
        },
        WaveformMathOperation::Ratio => None,
    }
}

fn derived_waveform_label(
    requested: &str,
    operation: WaveformMathOperation,
    left: &str,
    right: &str,
) -> String {
    let requested = requested.trim();
    if !requested.is_empty() {
        return requested.to_string();
    }
    format!("{left} {} {right}", operation.symbol())
}

fn unique_waveform_probe_label(waveform: &WaveformView, requested: &str) -> String {
    let base = if requested.trim().is_empty() {
        "derived".to_string()
    } else {
        requested.trim().to_string()
    };
    if waveform.probes.iter().all(|probe| probe.label != base) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}_{suffix}");
        if waveform.probes.iter().all(|probe| probe.label != candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search must find a unique waveform label")
}

fn waveform_probe_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut usize,
    probe_labels: &[String],
) {
    *selected = (*selected).min(probe_labels.len().saturating_sub(1));
    let selected_text = probe_labels
        .get(*selected)
        .map(String::as_str)
        .unwrap_or("select probe");
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for (index, label) in probe_labels.iter().enumerate() {
                ui.selectable_value(selected, index, label);
            }
        });
}

fn waveform_math_operation_combo(ui: &mut egui::Ui, selected: &mut String) {
    if WaveformMathOperation::from_label(selected).is_err() {
        *selected = WaveformMathOperation::Difference.label().to_string();
    }
    egui::ComboBox::from_id_salt("waveform_math_operation")
        .selected_text(selected.as_str())
        .show_ui(ui, |ui| {
            for operation in [
                WaveformMathOperation::Difference,
                WaveformMathOperation::Sum,
                WaveformMathOperation::Product,
                WaveformMathOperation::Ratio,
            ] {
                ui.selectable_value(selected, operation.label().to_string(), operation.label());
            }
        });
}

fn waveform_analog_scenario_combo(
    ui: &mut egui::Ui,
    choices: &[super::analog::AnalogScenarioChoice],
    selected: &mut String,
) {
    egui::ComboBox::from_id_salt("waveform_promote_scenario")
        .selected_text(if selected.is_empty() {
            "select scenario"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for choice in choices {
                ui.selectable_value(selected, choice.name.clone(), &choice.name);
            }
        });
}

fn sanitized_probe_name(label: &str) -> String {
    let mut name = String::new();
    for character in label.trim().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
            name.push(character);
        } else if !name.ends_with('_') {
            name.push('_');
        }
    }
    let name = name.trim_matches('_');
    if name.is_empty() {
        "derived_probe".to_string()
    } else {
        name.to_string()
    }
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

fn draw_waveform_plot_sized(
    ui: &mut egui::Ui,
    waveform: &WaveformView,
    probe_index: usize,
    cursor_a_us: f64,
    cursor_b_us: f64,
    desired_size: egui::Vec2,
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
        format!(
            "{} {:.3e}..{:.3e}",
            probe.expression.as_deref().unwrap_or(&probe.label),
            y_min,
            y_max
        ),
        font,
        egui::Color32::LIGHT_GRAY,
    );
}

pub(super) fn scope_plot_size(available: egui::Vec2) -> egui::Vec2 {
    egui::vec2(available.x.max(560.0), available.y.max(360.0))
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

pub(super) fn format_value(value: f64) -> String {
    format!("{value:.6e}")
}

#[cfg(test)]
mod waveform_tests;
