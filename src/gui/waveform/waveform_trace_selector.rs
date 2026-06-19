use super::{CircuitCiApp, WaveformProbe, WaveformProbeQuantity, WaveformView};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn waveform_selector(&mut self, ui: &mut egui::Ui) {
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
                    self.waveform_window_start_us = None;
                    self.waveform_window_end_us = None;
                    self.waveform_value_min = None;
                    self.waveform_value_max = None;
                    self.waveform_trigger_threshold = 0.0;
                    self.waveform_playing = false;
                }
            }
        });
    }

    pub(super) fn waveform_probe_selector(&mut self, ui: &mut egui::Ui) {
        let waveform = &self.waveforms[self.selected_waveform];
        if waveform.probes.is_empty() {
            ui.label("Waveform has no probe columns.");
            return;
        }

        let probe_count = waveform.probes.len();
        self.selected_probe = self.selected_probe.min(probe_count - 1);
        let probe_choices = waveform_probe_choices(waveform, &self.waveform_probe_filter);
        let selected_hidden = !probe_choices
            .iter()
            .any(|choice| choice.index == self.selected_probe);
        let no_matches = probe_choices.is_empty();

        ui.horizontal_wrapped(|ui| {
            ui.strong("Traces");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.waveform_probe_filter)
                    .desired_width(180.0)
                    .hint_text("name, expression, kind"),
            );
            if response.changed() {
                self.waveform_probe_filter = self.waveform_probe_filter.trim_start().to_string();
            }
            if ui
                .add_enabled(
                    !self.waveform_probe_filter.trim().is_empty(),
                    egui::Button::new("Clear"),
                )
                .clicked()
            {
                self.waveform_probe_filter.clear();
            }
            ui.checkbox(&mut self.waveform_probe_group_by_kind, "Group by kind");
            ui.label(format!("{} / {} visible", probe_choices.len(), probe_count));
            if selected_hidden {
                ui.label("active trace hidden by filter");
            }
        });

        let mut selected_new_trace = false;
        let mut selected_probe_index = None;
        if self.waveform_probe_group_by_kind {
            ui.horizontal_wrapped(|ui| {
                for (group, group_choices) in waveform_probe_group_choices(&probe_choices) {
                    ui.menu_button(
                        format!("{} ({})", group.label(), group_choices.len()),
                        |ui| {
                            for choice in group_choices {
                                if ui
                                    .selectable_label(
                                        self.selected_probe == choice.index,
                                        &choice.label,
                                    )
                                    .clicked()
                                {
                                    selected_probe_index = Some(choice.index);
                                    ui.close();
                                }
                            }
                        },
                    );
                }
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                for choice in &probe_choices {
                    if ui
                        .selectable_label(self.selected_probe == choice.index, &choice.label)
                        .clicked()
                    {
                        selected_probe_index = Some(choice.index);
                    }
                }
            });
            if probe_count > 24 {
                ui.small("Use filter or Group by kind to narrow larger waveform trace sets.");
            }
        }
        if no_matches {
            ui.small("No traces match the current filter.");
        }

        if let Some(index) = selected_probe_index {
            self.select_waveform_probe_index(index, probe_count);
            selected_new_trace = true;
        }
        if selected_new_trace {
            self.focus_selected_scope_schematic_context_silent();
        }
        let can_focus_schematic = self.selected_scope_sketch_probe().is_some();
        let mut focus_schematic = false;
        ui.horizontal_wrapped(|ui| {
            let pinned = self.selected_scope_trace_pinned();
            if ui
                .button(if pinned { "Unpin Trace" } else { "Pin Trace" })
                .clicked()
            {
                self.toggle_selected_scope_trace_pin();
            }
            if ui
                .add_enabled(
                    !self.waveform_pinned_traces.is_empty(),
                    egui::Button::new("Clear Pins"),
                )
                .clicked()
            {
                let count = self.waveform_pinned_traces.len();
                self.waveform_pinned_traces.clear();
                self.status = format!("Cleared {count} pinned scope trace(s).");
            }
            if !self.waveform_pinned_traces.is_empty() {
                ui.label(format!(
                    "{} pinned comparison trace(s)",
                    self.waveform_pinned_traces.len()
                ));
            }
            if ui
                .add_enabled(can_focus_schematic, egui::Button::new("Focus Schematic"))
                .clicked()
            {
                focus_schematic = true;
            }
        });
        if focus_schematic {
            self.focus_selected_scope_schematic_context();
        }
    }

    fn select_waveform_probe_index(&mut self, index: usize, probe_count: usize) {
        self.selected_probe = index.min(probe_count.saturating_sub(1));
        self.waveform_math_left = self.waveform_math_left.min(probe_count.saturating_sub(1));
        self.waveform_math_right = self.waveform_math_right.min(probe_count.saturating_sub(1));
        self.waveform_cursor_a_us = 0.0;
        self.waveform_cursor_b_us = 0.0;
        self.waveform_value_min = None;
        self.waveform_value_max = None;
        self.waveform_trigger_threshold = 0.0;
        self.waveform_playing = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WaveformProbeGroup {
    Voltage,
    Current,
    Power,
    Derived,
    Other,
}

impl WaveformProbeGroup {
    fn all() -> [Self; 5] {
        [
            Self::Voltage,
            Self::Current,
            Self::Power,
            Self::Derived,
            Self::Other,
        ]
    }

    fn label(self) -> &'static str {
        match self {
            Self::Voltage => "Voltage",
            Self::Current => "Current",
            Self::Power => "Power",
            Self::Derived => "Derived",
            Self::Other => "Other",
        }
    }

    fn searchable_label(self) -> &'static str {
        match self {
            Self::Voltage => "voltage",
            Self::Current => "current",
            Self::Power => "power",
            Self::Derived => "derived",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WaveformProbeChoice {
    pub(super) index: usize,
    pub(super) label: String,
    group: WaveformProbeGroup,
}

fn waveform_probe_group(probe: &WaveformProbe) -> WaveformProbeGroup {
    if probe.derived {
        return WaveformProbeGroup::Derived;
    }
    match probe
        .promoted_quantity
        .or_else(|| super::waveform_probe_quantity_from_label(&probe.label))
    {
        Some(WaveformProbeQuantity::Voltage) => WaveformProbeGroup::Voltage,
        Some(WaveformProbeQuantity::Current) => WaveformProbeGroup::Current,
        Some(WaveformProbeQuantity::Power) => WaveformProbeGroup::Power,
        None => WaveformProbeGroup::Other,
    }
}

fn waveform_probe_matches_filter(probe: &WaveformProbe, query: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }
    let group = waveform_probe_group(probe);
    probe.label.to_ascii_lowercase().contains(&query)
        || probe
            .expression
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(&query)
        || group.searchable_label().contains(&query)
}

pub(super) fn waveform_probe_choices(
    waveform: &WaveformView,
    query: &str,
) -> Vec<WaveformProbeChoice> {
    waveform
        .probes
        .iter()
        .enumerate()
        .filter(|(_, probe)| waveform_probe_matches_filter(probe, query))
        .map(|(index, probe)| WaveformProbeChoice {
            index,
            label: probe.label.clone(),
            group: waveform_probe_group(probe),
        })
        .collect()
}

pub(super) fn waveform_probe_group_choices(
    choices: &[WaveformProbeChoice],
) -> Vec<(WaveformProbeGroup, Vec<WaveformProbeChoice>)> {
    WaveformProbeGroup::all()
        .into_iter()
        .filter_map(|group| {
            let group_choices: Vec<_> = choices
                .iter()
                .filter(|choice| choice.group == group)
                .cloned()
                .collect();
            (!group_choices.is_empty()).then_some((group, group_choices))
        })
        .collect()
}
