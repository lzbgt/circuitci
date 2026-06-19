use super::{
    CircuitCiApp, WaveformProbe, WaveformView, format_time_s, format_value, interpolated_value,
    min_max, probe_unit,
};
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScopeTriggerEdge {
    Rising,
    Falling,
    Either,
}

impl ScopeTriggerEdge {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Rising => "rising",
            Self::Falling => "falling",
            Self::Either => "either",
        }
    }

    fn from_label(label: &str) -> Option<Self> {
        match label.trim().to_ascii_lowercase().as_str() {
            "rising" => Some(Self::Rising),
            "falling" => Some(Self::Falling),
            "either" => Some(Self::Either),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ScopeTriggerEvent {
    pub(super) time_us: f64,
    pub(super) value: f64,
    pub(super) edge: ScopeTriggerEdge,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ScopeTriggerEventRow {
    pub(super) index: usize,
    pub(super) edge: &'static str,
    pub(super) time_s: f64,
    pub(super) value: f64,
    pub(super) delta_t_s: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScopeTriggerJump {
    Previous,
    Next,
}

impl CircuitCiApp {
    pub(super) fn waveform_trigger_panel(&mut self, ui: &mut egui::Ui) {
        let Some(waveform) = self.waveforms.get(self.selected_waveform) else {
            return;
        };
        let Some(probe) = waveform.probes.get(self.selected_probe) else {
            return;
        };
        let unit = probe_unit(&probe.label);
        let threshold_step = trigger_threshold_step(probe);
        let cursor_a_value = interpolated_value(
            &waveform.time_s,
            &probe.values,
            self.waveform_cursor_a_us / 1e6,
        );
        let auto_threshold = min_max(&probe.values).map(|(min, max)| (min + max) * 0.5);
        let events = scope_trigger_events(
            waveform,
            self.selected_probe,
            self.waveform_trigger_threshold,
            self.selected_scope_trigger_edge(),
        );
        let event_count = events.len();
        let mut set_from_a = false;
        let mut set_auto = false;
        let mut jump_previous = false;
        let mut jump_next = false;
        ui.horizontal_wrapped(|ui| {
            ui.strong("Trigger");
            scope_trigger_edge_combo(ui, &mut self.waveform_trigger_edge);
            ui.label("level");
            ui.add(
                egui::DragValue::new(&mut self.waveform_trigger_threshold)
                    .speed(threshold_step)
                    .suffix(format!(" {unit}")),
            );
            if ui
                .add_enabled(cursor_a_value.is_some(), egui::Button::new("From A"))
                .clicked()
            {
                set_from_a = true;
            }
            if ui
                .add_enabled(auto_threshold.is_some(), egui::Button::new("Auto"))
                .clicked()
            {
                set_auto = true;
            }
            if ui
                .add_enabled(event_count > 0, egui::Button::new("Prev Edge"))
                .clicked()
            {
                jump_previous = true;
            }
            if ui
                .add_enabled(event_count > 0, egui::Button::new("Next Edge"))
                .clicked()
            {
                jump_next = true;
            }
            ui.label(format!("{event_count} event(s)"));
        });
        if set_from_a && let Some(value) = cursor_a_value {
            self.waveform_trigger_threshold = value;
            self.status = format!(
                "Scope trigger level set to {} {unit} from cursor A.",
                format_value(value)
            );
        }
        if set_auto && let Some(value) = auto_threshold {
            self.waveform_trigger_threshold = value;
            self.status = format!("Scope trigger level set to {} {unit}.", format_value(value));
        }
        if jump_previous {
            self.jump_scope_trigger_event(ScopeTriggerJump::Previous);
        }
        if jump_next {
            self.jump_scope_trigger_event(ScopeTriggerJump::Next);
        }
        self.waveform_trigger_event_table(ui, &events);
    }

    fn waveform_trigger_event_table(&mut self, ui: &mut egui::Ui, events: &[ScopeTriggerEvent]) {
        if events.is_empty() {
            return;
        }
        let rows = scope_trigger_event_rows(events, self.waveform_cursor_a_us);
        let mut jump_event = None;
        let mut snapshot_event = None;
        let mut focus_schematic = false;
        let can_focus_schematic = self.selected_scope_sketch_probe().is_some();
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Trigger Events");
                ui.label("jump sets cursor A");
                if rows.len() > 64 {
                    ui.label(format!("showing 64 of {}", rows.len()));
                }
            });
            egui::ScrollArea::vertical()
                .max_height(132.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new("scope_trigger_events")
                        .num_columns(8)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("#");
                            ui.label("Edge");
                            ui.label("Time");
                            ui.label("Value");
                            ui.label("Delta A");
                            ui.label("");
                            ui.label("");
                            ui.label("");
                            ui.end_row();

                            for row in rows.iter().take(64) {
                                ui.monospace(row.index.to_string());
                                ui.monospace(row.edge);
                                ui.monospace(format_time_s(row.time_s));
                                ui.monospace(format_value(row.value));
                                ui.monospace(format_time_s(row.delta_t_s));
                                if ui.button("Jump").clicked() {
                                    jump_event = events.get(row.index - 1).copied();
                                }
                                if ui.button("Snap").clicked() {
                                    snapshot_event = events.get(row.index - 1).copied();
                                }
                                if ui
                                    .add_enabled(can_focus_schematic, egui::Button::new("Focus"))
                                    .clicked()
                                {
                                    focus_schematic = true;
                                }
                                ui.end_row();
                            }
                        });
                });
        });
        if let Some(event) = jump_event {
            self.set_waveform_cursor_a(event.time_us);
            self.waveform_playing = false;
            self.status = format!(
                "Jumped to trigger event {} at {}.",
                event.edge.label(),
                format_time_s(event.time_us / 1e6)
            );
            self.focus_selected_scope_schematic_context_silent();
        }
        if let Some(event) = snapshot_event {
            self.capture_scope_trigger_snapshot(event);
        }
        if focus_schematic {
            self.focus_selected_scope_schematic_context();
        }
    }

    pub(super) fn selected_scope_trigger_edge(&self) -> ScopeTriggerEdge {
        ScopeTriggerEdge::from_label(&self.waveform_trigger_edge)
            .unwrap_or(ScopeTriggerEdge::Rising)
    }

    pub(super) fn selected_scope_trigger_events(&self) -> Vec<ScopeTriggerEvent> {
        let Some(waveform) = self.waveforms.get(self.selected_waveform) else {
            return Vec::new();
        };
        scope_trigger_events(
            waveform,
            self.selected_probe,
            self.waveform_trigger_threshold,
            self.selected_scope_trigger_edge(),
        )
    }

    fn jump_scope_trigger_event(&mut self, direction: ScopeTriggerJump) {
        let events = self.selected_scope_trigger_events();
        let Some(event) = select_scope_trigger_event(&events, self.waveform_cursor_a_us, direction)
        else {
            self.status = "No selected trace trigger events found.".to_string();
            return;
        };
        self.set_waveform_cursor_a(event.time_us);
        self.waveform_playing = false;
        self.status = format!(
            "Jumped to {} trigger at {} ({}).",
            self.selected_scope_trigger_edge().label(),
            format_time_s(event.time_us / 1e6),
            format_value(event.value)
        );
    }
}

pub(super) fn scope_trigger_events(
    waveform: &WaveformView,
    probe_index: usize,
    threshold: f64,
    edge: ScopeTriggerEdge,
) -> Vec<ScopeTriggerEvent> {
    let Some(probe) = waveform.probes.get(probe_index) else {
        return Vec::new();
    };
    if !threshold.is_finite() || waveform.time_s.len() != probe.values.len() {
        return Vec::new();
    }

    waveform
        .time_s
        .windows(2)
        .zip(probe.values.windows(2))
        .filter_map(|(times, values)| {
            let (t0, t1) = (times[0], times[1]);
            let (v0, v1) = (values[0], values[1]);
            if !t0.is_finite()
                || !t1.is_finite()
                || !v0.is_finite()
                || !v1.is_finite()
                || t1 <= t0
                || (v1 - v0).abs() < f64::EPSILON
            {
                return None;
            }
            let rising = v0 < threshold && v1 >= threshold;
            let falling = v0 > threshold && v1 <= threshold;
            let matched_edge = match edge {
                ScopeTriggerEdge::Rising if rising => Some(ScopeTriggerEdge::Rising),
                ScopeTriggerEdge::Falling if falling => Some(ScopeTriggerEdge::Falling),
                ScopeTriggerEdge::Either if rising => Some(ScopeTriggerEdge::Rising),
                ScopeTriggerEdge::Either if falling => Some(ScopeTriggerEdge::Falling),
                _ => None,
            };
            let matched_edge = matched_edge?;
            let ratio = ((threshold - v0) / (v1 - v0)).clamp(0.0, 1.0);
            Some(ScopeTriggerEvent {
                time_us: (t0 + (t1 - t0) * ratio) * 1e6,
                value: threshold,
                edge: matched_edge,
            })
        })
        .collect()
}

pub(super) fn scope_trigger_event_rows(
    events: &[ScopeTriggerEvent],
    cursor_a_us: f64,
) -> Vec<ScopeTriggerEventRow> {
    events
        .iter()
        .copied()
        .enumerate()
        .map(|(index, event)| ScopeTriggerEventRow {
            index: index + 1,
            edge: event.edge.label(),
            time_s: event.time_us / 1e6,
            value: event.value,
            delta_t_s: (event.time_us - cursor_a_us) / 1e6,
        })
        .collect()
}

pub(super) fn select_scope_trigger_event(
    events: &[ScopeTriggerEvent],
    cursor_us: f64,
    direction: ScopeTriggerJump,
) -> Option<ScopeTriggerEvent> {
    if events.is_empty() {
        return None;
    }
    let epsilon = 1.0e-9_f64.max(cursor_us.abs() * 1.0e-12);
    match direction {
        ScopeTriggerJump::Next => events
            .iter()
            .copied()
            .find(|event| event.time_us > cursor_us + epsilon)
            .or_else(|| events.first().copied()),
        ScopeTriggerJump::Previous => events
            .iter()
            .rev()
            .copied()
            .find(|event| event.time_us < cursor_us - epsilon)
            .or_else(|| events.last().copied()),
    }
}

fn scope_trigger_edge_combo(ui: &mut egui::Ui, selected: &mut String) {
    if ScopeTriggerEdge::from_label(selected).is_none() {
        *selected = ScopeTriggerEdge::Rising.label().to_string();
    }
    egui::ComboBox::from_id_salt("scope_trigger_edge")
        .selected_text(selected.as_str())
        .show_ui(ui, |ui| {
            for edge in [
                ScopeTriggerEdge::Rising,
                ScopeTriggerEdge::Falling,
                ScopeTriggerEdge::Either,
            ] {
                ui.selectable_value(selected, edge.label().to_string(), edge.label());
            }
        });
}

fn trigger_threshold_step(probe: &WaveformProbe) -> f64 {
    min_max(&probe.values)
        .map(|(min, max)| (max - min).abs() / 200.0)
        .unwrap_or(1.0)
        .max(1.0e-12)
}
