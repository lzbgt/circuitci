use super::waveform_plot::{
    scope_trace_color_for_style, scope_trace_style, scope_visible_styled_trace_refs,
    scope_visible_trace_refs, valid_waveform_trace,
};
use super::{
    CircuitCiApp, DeferredWaveformArtifact, WaveformLoadRequest, WaveformProbe,
    WaveformProbeQuantity, WaveformTraceColor, WaveformTracePreset, WaveformTraceRef,
    WaveformTraceStyle, WaveformView, waveform_load_deferred_artifacts,
};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn waveform_selector(&mut self, ui: &mut egui::Ui) {
        let mut load_deferred_path = None;
        let deferred_artifacts = waveform_load_deferred_artifacts(&self.waveform_load_diagnostics);
        if self.waveforms.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Waveforms");
                if deferred_artifacts.is_empty() {
                    ui.label("No parsed or deferred waveform artifacts.");
                } else {
                    ui.label(format!(
                        "{} deferred waveform artifact(s)",
                        deferred_artifacts.len()
                    ));
                }
            });
            self.deferred_waveform_artifacts_ui(ui, &deferred_artifacts, &mut load_deferred_path);
            if let Some(path) = load_deferred_path {
                self.load_deferred_waveform_path(path);
            }
            return;
        }

        self.selected_waveform = self.selected_waveform.min(self.waveforms.len() - 1);
        let mut next_waveform = None;
        ui.horizontal_wrapped(|ui| {
            ui.strong("Waveforms");
            for (index, waveform) in self.waveforms.iter().enumerate() {
                if ui
                    .selectable_label(self.selected_waveform == index, &waveform.label)
                    .clicked()
                {
                    next_waveform = Some(index);
                }
            }
            if !deferred_artifacts.is_empty() {
                ui.label(format!("{} deferred", deferred_artifacts.len()));
            }
        });
        self.deferred_waveform_artifacts_ui(ui, &deferred_artifacts, &mut load_deferred_path);
        if let Some(index) = next_waveform.filter(|index| *index != self.selected_waveform) {
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
            self.clear_waveform_view_history();
            self.waveform_trigger_threshold = 0.0;
            self.waveform_playing = false;
        }
        if let Some(path) = load_deferred_path {
            self.load_deferred_waveform_path(path);
        }
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
            let can_split_units = self.current_scope_compare_traces().len() > 1;
            let split_response = ui.add_enabled(
                can_split_units,
                egui::Checkbox::new(&mut self.waveform_split_trace_units, "Split Units"),
            );
            if split_response.changed() {
                self.waveform_value_min = None;
                self.waveform_value_max = None;
                self.clear_waveform_view_history();
                self.status = if self.waveform_split_trace_units {
                    "Scopes split visible compare traces into per-unit lanes.".to_string()
                } else {
                    "Scopes returned to shared-axis overlay mode.".to_string()
                };
            }
            if !can_split_units {
                self.waveform_split_trace_units = false;
            }
        });
        if focus_schematic {
            self.focus_selected_scope_schematic_context();
        }
        self.waveform_trace_styles_ui(ui);
        self.waveform_compare_presets_ui(ui);
    }

    fn select_waveform_probe_index(&mut self, index: usize, probe_count: usize) {
        self.selected_probe = index.min(probe_count.saturating_sub(1));
        self.waveform_math_left = self.waveform_math_left.min(probe_count.saturating_sub(1));
        self.waveform_math_right = self.waveform_math_right.min(probe_count.saturating_sub(1));
        self.waveform_cursor_a_us = 0.0;
        self.waveform_cursor_b_us = 0.0;
        self.waveform_value_min = None;
        self.waveform_value_max = None;
        self.clear_waveform_view_history();
        self.waveform_trigger_threshold = 0.0;
        self.waveform_playing = false;
    }

    fn deferred_waveform_artifacts_ui(
        &mut self,
        ui: &mut egui::Ui,
        artifacts: &[DeferredWaveformArtifact],
        load_deferred_path: &mut Option<String>,
    ) {
        if artifacts.is_empty() {
            return;
        }
        let visible_indexes =
            deferred_waveform_artifact_visible_indexes(artifacts, &self.waveform_deferred_filter);
        let visible_paths = visible_indexes
            .iter()
            .filter_map(|&index| artifacts.get(index))
            .map(|artifact| artifact.path.clone())
            .collect::<Vec<_>>();
        let matching_probe_requests = deferred_waveform_matching_probe_requests(
            artifacts,
            &visible_indexes,
            &self.waveform_deferred_filter,
        );
        let matching_probe_count = matching_probe_requests
            .iter()
            .map(WaveformLoadRequest::probe_count)
            .sum::<usize>();
        let mut load_probe_requests = None;
        ui.horizontal_wrapped(|ui| {
            ui.menu_button(format!("Deferred Waveforms ({})", artifacts.len()), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Find");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.waveform_deferred_filter)
                            .desired_width(180.0)
                            .hint_text("file, probe, detail"),
                    );
                    if response.changed() {
                        self.waveform_deferred_filter =
                            self.waveform_deferred_filter.trim_start().to_string();
                    }
                    if ui
                        .add_enabled(
                            !self.waveform_deferred_filter.trim().is_empty(),
                            egui::Button::new("Clear"),
                        )
                        .clicked()
                    {
                        self.waveform_deferred_filter.clear();
                    }
                    ui.label(format!(
                        "{} / {} visible",
                        visible_indexes.len(),
                        artifacts.len()
                    ));
                });
                if visible_indexes.is_empty() {
                    ui.label("No deferred waveform artifact matches the current filter.");
                    return;
                }
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for &index in &visible_indexes {
                            let artifact = &artifacts[index];
                            let matching_probes = deferred_waveform_artifact_matching_probe_labels(
                                artifact,
                                &self.waveform_deferred_filter,
                            );
                            ui.horizontal(|ui| {
                                if ui.button("Load").clicked() {
                                    *load_deferred_path = Some(artifact.path.clone());
                                    ui.close();
                                }
                                if ui
                                    .add_enabled(
                                        !matching_probes.is_empty(),
                                        egui::Button::new("Load Matching"),
                                    )
                                    .clicked()
                                {
                                    load_probe_requests =
                                        Some(vec![WaveformLoadRequest::selected_columns(
                                            artifact.path.clone(),
                                            matching_probes.clone(),
                                        )]);
                                    ui.close();
                                }
                                ui.monospace(&artifact.label);
                                let loaded_count = artifact.loaded_probe_preview.len();
                                let loaded = if loaded_count == 0 {
                                    String::new()
                                } else {
                                    format!("; {loaded_count} loaded")
                                };
                                ui.label(format!(
                                    "{}; ~{} row(s); {} trace(s){}",
                                    artifact.size_label, artifact.samples, artifact.probes, loaded
                                ));
                            });
                            let preview = deferred_probe_preview_text(
                                &artifact.probe_preview,
                                &artifact.loaded_probe_preview,
                            );
                            if !preview.is_empty() {
                                ui.small(preview);
                            }
                            if !artifact.detail.is_empty() {
                                ui.small(&artifact.detail);
                            }
                        }
                    });
            });
            if ui.button("Load All Deferred").clicked() {
                self.load_deferred_waveforms();
            }
            if ui
                .add_enabled(
                    !visible_paths.is_empty() && visible_paths.len() < artifacts.len(),
                    egui::Button::new("Load Visible"),
                )
                .clicked()
            {
                self.load_deferred_waveform_paths(visible_paths);
            }
            if ui
                .add_enabled(
                    !matching_probe_requests.is_empty(),
                    egui::Button::new(format!("Load Matching Traces ({matching_probe_count})")),
                )
                .clicked()
            {
                load_probe_requests = Some(matching_probe_requests);
            }
        });
        if let Some(requests) = load_probe_requests {
            self.load_deferred_waveform_requests(requests);
        }
    }

    fn waveform_compare_presets_ui(&mut self, ui: &mut egui::Ui) {
        let current_traces = self.current_scope_compare_traces();
        let mut load_preset = None;
        let mut delete_preset = None;

        ui.horizontal_wrapped(|ui| {
            ui.strong("Compare Sets");
            ui.add(
                egui::TextEdit::singleline(&mut self.waveform_trace_preset_name)
                    .desired_width(140.0)
                    .hint_text("set name"),
            );
            if ui
                .add_enabled(!current_traces.is_empty(), egui::Button::new("Save Set"))
                .clicked()
            {
                self.save_current_scope_compare_preset(current_traces.clone());
            }
            ui.menu_button(
                format!("Saved ({})", self.waveform_trace_presets.len()),
                |ui| {
                    if self.waveform_trace_presets.is_empty() {
                        ui.label("No saved compare sets.");
                    }
                    for (index, preset) in self.waveform_trace_presets.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.button(&preset.name).clicked() {
                                load_preset = Some(index);
                                ui.close();
                            }
                            if ui.small_button("Delete").clicked() {
                                delete_preset = Some(index);
                                ui.close();
                            }
                        });
                    }
                },
            );
            if ui
                .add_enabled(
                    !self.waveform_trace_presets.is_empty(),
                    egui::Button::new("Clear Sets"),
                )
                .clicked()
            {
                let count = self.waveform_trace_presets.len();
                self.waveform_trace_presets.clear();
                self.status = format!("Cleared {count} saved scope compare set(s).");
            }
        });

        if let Some(index) = delete_preset
            && index < self.waveform_trace_presets.len()
        {
            let preset = self.waveform_trace_presets.remove(index);
            self.status = format!("Deleted scope compare set {}.", preset.name);
        }
        if let Some(index) = load_preset {
            self.apply_scope_compare_preset(index);
        }
    }

    fn waveform_trace_styles_ui(&mut self, ui: &mut egui::Ui) {
        let current_traces = self.current_scope_compare_traces();
        if current_traces.len() < 2 {
            return;
        }
        let visible_traces =
            scope_visible_styled_trace_refs(&current_traces, &self.waveform_trace_styles);
        let mut reset_current = false;

        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Trace Styles");
                ui.label(format!(
                    "{} / {} visible",
                    visible_traces.len(),
                    current_traces.len()
                ));
                if ui
                    .add_enabled(
                        current_traces
                            .iter()
                            .any(|trace| self.trace_style_has_override(*trace)),
                        egui::Button::new("Reset Current"),
                    )
                    .clicked()
                {
                    reset_current = true;
                }
            });

            egui::Grid::new("scope_trace_styles")
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Show");
                    ui.label("Trace");
                    ui.label("Color");
                    ui.label("Role");
                    ui.end_row();

                    for (index, trace) in current_traces.iter().copied().enumerate() {
                        let active = index == 0;
                        let style = scope_trace_style(&self.waveform_trace_styles, trace);
                        let mut visible = style.visible;
                        if active {
                            let mut checked = true;
                            ui.add_enabled(false, egui::Checkbox::new(&mut checked, ""));
                        } else if ui.checkbox(&mut visible, "").changed() {
                            self.set_scope_trace_visible(trace, visible);
                        }

                        ui.monospace(self.scope_trace_display_label(trace));

                        let mut color = style.color;
                        let swatch =
                            scope_trace_color_for_style(index, trace, &self.waveform_trace_styles);
                        egui::ComboBox::from_id_salt((
                            "scope_trace_color",
                            trace.waveform_index,
                            trace.probe_index,
                        ))
                        .selected_text(color.map_or("Auto", WaveformTraceColor::label))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut color, None, "Auto");
                            for choice in WaveformTraceColor::all() {
                                ui.selectable_value(&mut color, Some(choice), choice.label());
                            }
                        });
                        if color != style.color {
                            self.set_scope_trace_color(trace, color);
                        }
                        ui.colored_label(swatch, if active { "active" } else { "pinned" });
                        ui.end_row();
                    }
                });
        });

        if reset_current {
            self.reset_current_scope_trace_styles(&current_traces);
        }
    }

    pub(super) fn current_scope_compare_traces(&self) -> Vec<WaveformTraceRef> {
        scope_visible_trace_refs(
            &self.waveforms,
            self.selected_waveform,
            self.selected_probe,
            &self.waveform_pinned_traces,
        )
    }

    fn scope_trace_display_label(&self, trace: WaveformTraceRef) -> String {
        let Some((waveform, probe)) =
            self.waveforms
                .get(trace.waveform_index)
                .and_then(|waveform| {
                    waveform
                        .probes
                        .get(trace.probe_index)
                        .map(|probe| (waveform, probe))
                })
        else {
            return format!("{}:{}", trace.waveform_index, trace.probe_index);
        };
        format!(
            "{} / {}",
            waveform.label,
            probe.expression.as_deref().unwrap_or(&probe.label)
        )
    }

    fn trace_style_has_override(&self, trace: WaveformTraceRef) -> bool {
        self.waveform_trace_styles
            .iter()
            .any(|style| style.trace == trace && !style.is_default())
    }

    fn set_scope_trace_visible(&mut self, trace: WaveformTraceRef, visible: bool) {
        self.update_scope_trace_style(trace, |style| {
            style.visible = visible;
        });
        self.status = if visible {
            "Scope trace shown.".to_string()
        } else {
            "Scope trace hidden from compare overlay.".to_string()
        };
    }

    fn set_scope_trace_color(
        &mut self,
        trace: WaveformTraceRef,
        color: Option<WaveformTraceColor>,
    ) {
        self.update_scope_trace_style(trace, |style| {
            style.color = color;
        });
        self.status = match color {
            Some(color) => format!("Scope trace color set to {}.", color.label()),
            None => "Scope trace color reset to Auto.".to_string(),
        };
    }

    fn update_scope_trace_style(
        &mut self,
        trace: WaveformTraceRef,
        update: impl FnOnce(&mut WaveformTraceStyle),
    ) {
        if !valid_waveform_trace(&self.waveforms, trace) {
            return;
        }
        let mut style = scope_trace_style(&self.waveform_trace_styles, trace);
        update(&mut style);
        if style.is_default() {
            self.waveform_trace_styles
                .retain(|existing| existing.trace != trace);
        } else if let Some(existing) = self
            .waveform_trace_styles
            .iter_mut()
            .find(|existing| existing.trace == trace)
        {
            *existing = style;
        } else {
            self.waveform_trace_styles.push(style);
        }
    }

    fn reset_current_scope_trace_styles(&mut self, traces: &[WaveformTraceRef]) {
        let count = self.waveform_trace_styles.len();
        self.waveform_trace_styles
            .retain(|style| !traces.contains(&style.trace));
        let removed = count - self.waveform_trace_styles.len();
        self.status = format!("Reset {removed} current scope trace style override(s).");
    }

    pub(super) fn save_current_scope_compare_preset(&mut self, traces: Vec<WaveformTraceRef>) {
        let traces = traces
            .into_iter()
            .filter(|trace| valid_waveform_trace(&self.waveforms, *trace))
            .fold(Vec::new(), |mut unique, trace| {
                if !unique.contains(&trace) {
                    unique.push(trace);
                }
                unique
            });
        if traces.is_empty() {
            return;
        }
        let name = self.scope_compare_preset_name();
        let preset = WaveformTracePreset {
            name: name.clone(),
            traces,
        };
        if let Some(existing) = self
            .waveform_trace_presets
            .iter_mut()
            .find(|existing| existing.name == name)
        {
            *existing = preset;
            self.status = format!("Updated scope compare set {name}.");
        } else {
            self.waveform_trace_presets.push(preset);
            self.status = format!("Saved scope compare set {name}.");
        }
        self.waveform_trace_preset_name.clear();
    }

    fn scope_compare_preset_name(&self) -> String {
        let requested = self.waveform_trace_preset_name.trim();
        if requested.is_empty() {
            format!("Compare {}", self.waveform_trace_presets.len() + 1)
        } else {
            requested.to_string()
        }
    }

    pub(super) fn apply_scope_compare_preset(&mut self, index: usize) {
        let Some(preset) = self.waveform_trace_presets.get(index).cloned() else {
            return;
        };
        let mut traces: Vec<_> = preset
            .traces
            .into_iter()
            .filter(|trace| valid_waveform_trace(&self.waveforms, *trace))
            .fold(Vec::new(), |mut unique, trace| {
                if !unique.contains(&trace) {
                    unique.push(trace);
                }
                unique
            });
        let Some(selected) = traces.first().copied() else {
            self.status = format!("Scope compare set {} has no loaded traces.", preset.name);
            return;
        };
        self.selected_waveform = selected.waveform_index;
        self.select_waveform_probe_index(
            selected.probe_index,
            self.waveforms[selected.waveform_index].probes.len(),
        );
        traces.remove(0);
        self.waveform_pinned_traces = traces;
        self.prune_scope_trace_pins();
        self.focus_selected_scope_schematic_context_silent();
        self.status = format!("Loaded scope compare set {}.", preset.name);
    }

    pub(super) fn shift_scope_trace_presets_after_probe_removal(
        &mut self,
        waveform_index: usize,
        removed_probe_index: usize,
    ) {
        for preset in &mut self.waveform_trace_presets {
            preset.traces.retain_mut(|trace| {
                if trace.waveform_index != waveform_index {
                    return true;
                }
                if trace.probe_index == removed_probe_index {
                    return false;
                }
                if trace.probe_index > removed_probe_index {
                    trace.probe_index -= 1;
                }
                true
            });
        }
        self.waveform_trace_presets
            .retain(|preset| !preset.traces.is_empty());
    }

    pub(super) fn shift_scope_trace_styles_after_probe_removal(
        &mut self,
        waveform_index: usize,
        removed_probe_index: usize,
    ) {
        self.waveform_trace_styles.retain_mut(|style| {
            if style.trace.waveform_index != waveform_index {
                return true;
            }
            if style.trace.probe_index == removed_probe_index {
                return false;
            }
            if style.trace.probe_index > removed_probe_index {
                style.trace.probe_index -= 1;
            }
            true
        });
    }
}

fn deferred_probe_preview_text(probes: &[String], loaded_probes: &[String]) -> String {
    if probes.is_empty() {
        return String::new();
    }
    let visible = probes
        .iter()
        .take(6)
        .map(|probe| {
            if deferred_waveform_probe_is_loaded(probe, loaded_probes) {
                format!("{probe} (loaded)")
            } else {
                probe.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    if probes.len() > 6 {
        format!("{visible}, +{} more", probes.len() - 6)
    } else {
        visible
    }
}

pub(super) fn deferred_waveform_matching_probe_requests(
    artifacts: &[DeferredWaveformArtifact],
    visible_indexes: &[usize],
    query: &str,
) -> Vec<WaveformLoadRequest> {
    visible_indexes
        .iter()
        .filter_map(|&index| artifacts.get(index))
        .filter_map(|artifact| {
            let probes = deferred_waveform_artifact_matching_probe_labels(artifact, query);
            (!probes.is_empty())
                .then(|| WaveformLoadRequest::selected_columns(artifact.path.clone(), probes))
        })
        .collect()
}

fn deferred_waveform_artifact_matching_probe_labels(
    artifact: &DeferredWaveformArtifact,
    query: &str,
) -> Vec<String> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    artifact
        .probe_preview
        .iter()
        .filter(|probe| {
            probe.to_ascii_lowercase().contains(&query)
                && !deferred_waveform_probe_is_loaded(probe, &artifact.loaded_probe_preview)
        })
        .cloned()
        .collect()
}

fn deferred_waveform_probe_is_loaded(probe: &str, loaded_probes: &[String]) -> bool {
    loaded_probes
        .iter()
        .any(|loaded| loaded.trim().eq_ignore_ascii_case(probe.trim()))
}

pub(super) fn deferred_waveform_artifact_visible_indexes(
    artifacts: &[DeferredWaveformArtifact],
    query: &str,
) -> Vec<usize> {
    let query = query.trim().to_ascii_lowercase();
    artifacts
        .iter()
        .enumerate()
        .filter_map(|(index, artifact)| {
            if query.is_empty() || deferred_waveform_artifact_search_text(artifact).contains(&query)
            {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

fn deferred_waveform_artifact_search_text(artifact: &DeferredWaveformArtifact) -> String {
    format!(
        "{} {} {} {} {} {}",
        artifact.label,
        artifact.path,
        artifact.size_label,
        artifact.samples,
        artifact.detail,
        artifact.probe_preview.join(" ")
    )
    .to_ascii_lowercase()
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
