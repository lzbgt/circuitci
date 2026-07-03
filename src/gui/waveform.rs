use super::CircuitCiApp;
use super::analog::{
    AnalogAssertionDraft, AnalogExpressionProbeDraft, analog_scenario_choices,
    append_analog_assertion, append_analog_expression_probe, unique_analog_assertion_name,
};
use super::sketch_probes::SketchProbe;
use anyhow::{Context, Result};
use eframe::egui;

mod waveform_bundle_integrity;
mod waveform_bundle_recent;
mod waveform_bundles;
mod waveform_context;
mod waveform_deferred;
mod waveform_export;
mod waveform_footprint;
mod waveform_io;
mod waveform_load;
mod waveform_load_diagnostics;
mod waveform_monte_carlo;
mod waveform_noise;
mod waveform_operating_point;
mod waveform_plot;
mod waveform_runtime;
mod waveform_snapshots;
mod waveform_trace_selector;
mod waveform_trigger;
mod waveform_view;
#[cfg(test)]
use waveform_bundle_integrity::{
    scope_report_bundle_artifact_detail_rows, scope_report_bundle_changed_artifacts,
    scope_report_bundle_integrity_details, scope_report_bundle_integrity_details_csv,
    scope_report_bundle_integrity_details_markdown,
    scope_report_bundle_integrity_projected_details, scope_report_bundle_missing_artifacts,
};
#[cfg(test)]
use waveform_bundle_recent::{cleanup_old_scope_report_bundle_dirs, old_scope_report_bundle_dirs};
#[cfg(test)]
use waveform_bundles::{scope_report_bundle_index_path, unique_scope_report_bundle_dir};
#[cfg(test)]
use waveform_deferred::{
    clear_deferred_waveform_column_picks,
    deferred_waveform_artifact_filtered_unloaded_probe_labels,
    deferred_waveform_artifact_picked_probe_labels,
    deferred_waveform_artifact_unloaded_probe_labels, deferred_waveform_artifact_visible_indexes,
    deferred_waveform_matching_probe_requests, deferred_waveform_remaining_probe_requests,
    select_deferred_waveform_column_picks,
};
pub(super) use waveform_export::ScopePlotSvgSizePreset;
#[cfg(test)]
use waveform_export::scope_plot_svg;
use waveform_footprint::waveform_footprint_target_index;
pub(super) use waveform_footprint::{
    WaveformFootprintSortKey, WaveformFootprintSourceFilter, WaveformFootprintUnloadTarget,
};
#[cfg(test)]
use waveform_footprint::{
    waveform_footprint_csv, waveform_footprint_largest_unload_targets, waveform_footprint_rows,
    waveform_footprint_rows_with_diagnostics, waveform_footprint_source_summaries,
    waveform_footprint_summary_csv, waveform_footprint_summary_markdown,
    waveform_footprint_unload_targets,
};
#[cfg(test)]
use waveform_io::load_waveform_paths_with_progress_and_cancel;
use waveform_io::waveform_probe_quantity_from_label;
pub(super) use waveform_io::{
    WaveformLoadRequest, load_report_waveforms_with_progress_and_cancel,
    load_waveform_requests_with_progress_and_cancel,
};
#[cfg(test)]
use waveform_io::{
    load_waveform_csv_with_progress_and_cancel, parse_hb_spectrum_csv_text, parse_waveform_csv_text,
};
use waveform_load::waveform_load_forget_loaded_view;
#[cfg(test)]
use waveform_load::waveform_load_preflight;
pub(super) use waveform_load::{
    DeferredWaveformArtifact, WaveformLoadDiagnostic, WaveformLoadPreviewFilter,
    WaveformLoadStatusFilter, merge_waveform_load_diagnostics, waveform_load_deferred_artifacts,
    waveform_load_deferred_paths,
};
#[cfg(test)]
use waveform_load_diagnostics::{
    waveform_load_diagnostic_unloaded_preview_columns, waveform_load_diagnostic_visible_indexes,
    waveform_load_diagnostics_csv,
};
#[cfg(test)]
use waveform_noise::parse_noise_total_csv_text;
pub(super) use waveform_noise::{
    NoiseTotalView, load_report_noise_totals_with_progress_and_cancel,
};
#[cfg(test)]
use waveform_operating_point::parse_operating_point_csv_text;
pub(super) use waveform_operating_point::{
    OperatingPointView, load_report_operating_points_with_progress_and_cancel,
};
pub(super) use waveform_plot::{WaveformCursorTarget, WaveformPlotCache};
#[cfg(test)]
use waveform_plot::{
    WaveformPlotLaneMode, WaveformPlotTrigger, WaveformPlotView, WaveformSnapshotChip,
    WaveformSnapshotMarker, clamp_value_window, expanded_value_bounds, nearest_scope_cursor_target,
    plot_x_to_time_us, plot_y_to_value, scope_plot_size, scope_snapshot_chip_hit,
    scope_trace_color_for_style, scope_trace_lanes, scope_visible_styled_trace_refs,
    scope_visible_trace_refs, scope_zoom_box_interaction, waveform_time_window_for_view,
    waveform_trace_bounds_in_window, zoom_time_window,
};
pub(in crate::gui) use waveform_runtime::{
    RuntimeScopeProbeEdgeStep, runtime_probe_activity_for_selection,
    runtime_probe_lines_for_selection, runtime_scope_probe_edge_jump,
    runtime_scope_probe_frequency, runtime_scope_probe_frequency_label,
    runtime_scope_probe_sample_label, runtime_scope_probe_sparkline_points,
    runtime_scope_probe_target_for_selection,
};
pub(super) use waveform_snapshots::{
    ScopeSnapshotGroupMode, ScopeSnapshotSortKey, ScopeSnapshotSourceFilter, scope_snapshots_csv,
    scope_snapshots_markdown,
};
#[cfg(test)]
use waveform_snapshots::{scope_snapshot_visible_indexes, scope_snapshot_visible_indexes_sorted};
#[cfg(test)]
use waveform_trace_selector::{
    WaveformProbeGroup, waveform_probe_choices, waveform_probe_group_choices,
};
#[cfg(test)]
use waveform_trigger::scope_trigger_event_rows;
#[cfg(test)]
use waveform_trigger::{
    ScopeTriggerEdge, ScopeTriggerJump, scope_trigger_events, select_scope_trigger_event,
};

impl CircuitCiApp {
    pub(super) fn waveform_scope_view(&mut self, ui: &mut egui::Ui, desired_size: egui::Vec2) {
        self.waveform_scope_header(ui);
        self.waveform_load_diagnostics_panel(ui);
        self.monte_carlo_yield_results_panel(ui);
        self.operating_point_results_panel(ui);
        self.noise_total_results_panel(ui);
        if self.waveforms.is_empty() {
            self.waveform_selector(ui);
            return;
        }
        self.prune_scope_trace_pins();

        self.waveform_selector(ui);
        self.waveform_probe_selector(ui);
        self.waveform_schematic_context_strip(ui);
        self.waveform_playback_panel(ui);
        self.waveform_scope_plot(ui, desired_size);
        self.waveform_scope_cursor_legend(ui);
    }

    pub(super) fn waveform_controls_panel(&mut self, ui: &mut egui::Ui) {
        if self.waveforms.is_empty() {
            if self.operating_points.is_empty()
                && self.noise_totals.is_empty()
                && !self.has_monte_carlo_yield_summaries()
            {
                ui.label("Run creates a default transient voltage probe when the schematic has no analog probes.");
            } else if self.has_monte_carlo_yield_summaries()
                && self.operating_points.is_empty()
                && self.noise_totals.is_empty()
            {
                ui.label("Monte Carlo yield summaries are shown in the main results table.");
            } else if !self.noise_totals.is_empty() {
                ui.label("Noise integrated RMS values are shown in the main results table.");
            } else {
                ui.label("DC operating-point values are shown in the main results table.");
            }
            ui.label("For explicit traces, select a net/component in Schematic and add Probe V, Probe I, or Probe P.");
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
        waveform_frequency_panel(ui, waveform, self.selected_probe);
    }

    fn waveform_scope_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Scopes");
            if self.waveforms.is_empty() {
                ui.label("No scope traces loaded. Run will auto-create a default voltage probe if none exist.");
            }
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
                let label = waveform.probes[index].label.clone();
                self.selected_probe = index;
                self.waveform_plot_cache.clear();
                self.waveform_value_min = None;
                self.waveform_value_max = None;
                self.clear_waveform_view_history();
                self.status = format!("Derived waveform channel {label} added.");
                self.waveform_math_name.clear();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_selected_waveform_math_channel(&mut self) {
        let waveform_index = self.selected_waveform;
        let removed_probe_index = self.selected_probe;
        let Some((label, probe_count)) =
            self.waveforms.get_mut(waveform_index).and_then(|waveform| {
                let probe = waveform.probes.get(removed_probe_index)?;
                if !probe.derived {
                    return None;
                }
                let label = probe.label.clone();
                waveform.probes.remove(removed_probe_index);
                Some((label, waveform.probes.len()))
            })
        else {
            return;
        };
        self.selected_probe = self.selected_probe.min(probe_count.saturating_sub(1));
        self.waveform_plot_cache.clear();
        self.shift_scope_trace_pins_after_probe_removal(waveform_index, removed_probe_index);
        self.shift_scope_trace_presets_after_probe_removal(waveform_index, removed_probe_index);
        self.shift_scope_trace_styles_after_probe_removal(waveform_index, removed_probe_index);
        self.shift_scope_measurement_snapshots_after_probe_removal(
            waveform_index,
            removed_probe_index,
        );
        self.prune_scope_trace_pins();
        self.waveform_math_left = self.waveform_math_left.min(probe_count.saturating_sub(1));
        self.waveform_math_right = self.waveform_math_right.min(probe_count.saturating_sub(1));
        self.waveform_value_min = None;
        self.waveform_value_max = None;
        self.clear_waveform_view_history();
        self.status = format!("Derived waveform channel {label} removed.");
    }

    fn shift_scope_trace_pins_after_probe_removal(
        &mut self,
        waveform_index: usize,
        removed_probe_index: usize,
    ) {
        self.waveform_pinned_traces.retain_mut(|trace| {
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

    pub(super) fn unload_waveform_view(&mut self, waveform_index: usize) {
        let Some(waveform) = self.waveforms.get(waveform_index).cloned() else {
            self.status = "No loaded waveform artifact is selected for unload.".to_string();
            return;
        };
        self.waveform_footprint_unload_preview.clear();
        let probe_labels = waveform
            .probes
            .iter()
            .filter(|probe| !probe.derived)
            .map(|probe| probe.label.clone())
            .collect::<Vec<_>>();
        self.waveforms.remove(waveform_index);
        waveform_load_forget_loaded_view(
            &mut self.waveform_load_diagnostics,
            &waveform.path,
            &probe_labels,
        );
        self.shift_scope_trace_pins_after_waveform_removal(waveform_index);
        self.shift_scope_trace_presets_after_waveform_removal(waveform_index);
        self.shift_scope_trace_styles_after_waveform_removal(waveform_index);
        self.shift_scope_measurement_snapshots_after_waveform_removal(waveform_index);
        self.waveform_plot_cache.clear();
        self.waveform_value_min = None;
        self.waveform_value_max = None;
        self.waveform_window_start_us = None;
        self.waveform_window_end_us = None;
        self.clear_waveform_view_history();
        self.waveform_playing = false;
        if self.waveforms.is_empty() {
            self.selected_waveform = 0;
            self.selected_probe = 0;
            self.waveform_cursor_a_us = 0.0;
            self.waveform_cursor_b_us = 0.0;
        } else {
            if self.selected_waveform == waveform_index {
                self.selected_waveform = waveform_index.min(self.waveforms.len() - 1);
                self.selected_probe = 0;
                self.waveform_cursor_a_us = 0.0;
                self.waveform_cursor_b_us = 0.0;
            } else if self.selected_waveform > waveform_index {
                self.selected_waveform -= 1;
            }
            let probe_count = self.waveforms[self.selected_waveform].probes.len();
            self.selected_probe = self.selected_probe.min(probe_count.saturating_sub(1));
        }
        self.status = format!("Unloaded waveform artifact {} from memory.", waveform.label);
    }

    pub(super) fn unload_waveform_for_diagnostic(&mut self, diagnostic: &WaveformLoadDiagnostic) {
        let Some(index) = self
            .waveforms
            .iter()
            .position(|waveform| waveform_matches_load_diagnostic(waveform, diagnostic))
        else {
            self.status =
                "Loaded waveform view for diagnostic row is no longer available.".to_string();
            return;
        };
        self.unload_waveform_view(index);
    }

    pub(super) fn unload_waveform_footprint_targets(
        &mut self,
        targets: &[WaveformFootprintUnloadTarget],
    ) -> usize {
        let mut indexes = Vec::new();
        for target in targets {
            if let Some(index) = waveform_footprint_target_index(&self.waveforms, target, &indexes)
            {
                indexes.push(index);
            }
        }
        indexes.sort_unstable_by(|left, right| right.cmp(left));
        indexes.dedup();
        let removed = indexes.len();
        for index in indexes {
            self.unload_waveform_view(index);
        }
        removed
    }

    fn shift_scope_trace_pins_after_waveform_removal(&mut self, removed_waveform_index: usize) {
        self.waveform_pinned_traces.retain_mut(|trace| {
            waveform_trace_selector::shift_trace_after_waveform_removal(
                trace,
                removed_waveform_index,
            )
        });
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
                reference_probe: self.analog_assertion_reference_probe.clone(),
                aggregation: self.analog_assertion_aggregation.clone(),
                relation: self.analog_assertion_relation.clone(),
                threshold: self.analog_assertion_threshold,
                reference_threshold: self.analog_assertion_reference_threshold,
                target: self.analog_assertion_target,
                tolerance: self.analog_assertion_tolerance,
                at_us: self.analog_assertion_at_us,
                at_hz: self.analog_assertion_at_hz,
                start_us: self.analog_assertion_start_us,
                end_us: self.analog_assertion_end_us,
                time_limit_us: self.analog_assertion_time_limit_us,
                frequency_limit_hz: self.analog_assertion_frequency_limit_hz,
                duty_limit_percent: self.analog_assertion_duty_limit_percent,
                count_limit: self.analog_assertion_count_limit,
                overshoot_limit_percent: self.analog_assertion_overshoot_limit_percent,
            },
        )
    }
}

#[derive(Debug, Clone)]
pub(super) struct WaveformView {
    label: String,
    path: String,
    x_axis: WaveformXAxis,
    time_s: Vec<f64>,
    probes: Vec<WaveformProbe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WaveformXAxis {
    TimeSeconds,
    FrequencyHz,
}

impl Default for WaveformXAxis {
    fn default() -> Self {
        Self::TimeSeconds
    }
}

impl WaveformXAxis {
    fn display_name(self) -> &'static str {
        match self {
            Self::TimeSeconds => "time",
            Self::FrequencyHz => "frequency",
        }
    }

    fn short_label(self) -> &'static str {
        match self {
            Self::TimeSeconds => "t",
            Self::FrequencyHz => "f",
        }
    }

    fn input_suffix(self) -> &'static str {
        match self {
            Self::TimeSeconds => " us",
            Self::FrequencyHz => " Hz",
        }
    }

    fn storage_from_csv_value(self, value: f64) -> f64 {
        match self {
            Self::TimeSeconds => value,
            Self::FrequencyHz => value / 1.0e6,
        }
    }

    fn display_from_cursor_us(self, cursor_us: f64) -> f64 {
        match self {
            Self::TimeSeconds => cursor_us / 1.0e6,
            Self::FrequencyHz => cursor_us,
        }
    }

    fn display_from_storage(self, value: f64) -> f64 {
        match self {
            Self::TimeSeconds => value,
            Self::FrequencyHz => value * 1.0e6,
        }
    }

    fn format_display_value(self, value: f64) -> String {
        match self {
            Self::TimeSeconds => format_time_s(value),
            Self::FrequencyHz => format_frequency_hz(value),
        }
    }

    fn format_cursor_us(self, cursor_us: f64) -> String {
        self.format_display_value(self.display_from_cursor_us(cursor_us))
    }

    fn format_storage_value(self, value: f64) -> String {
        self.format_display_value(self.display_from_storage(value))
    }

    fn format_delta_storage(self, value: f64) -> String {
        self.format_storage_value(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WaveformTraceRef {
    waveform_index: usize,
    probe_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WaveformTracePreset {
    name: String,
    traces: Vec<WaveformTraceRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WaveformTraceColor {
    Blue,
    Amber,
    Green,
    Red,
    Purple,
    Cyan,
}

impl WaveformTraceColor {
    fn all() -> [Self; 6] {
        [
            Self::Blue,
            Self::Amber,
            Self::Green,
            Self::Red,
            Self::Purple,
            Self::Cyan,
        ]
    }

    fn label(self) -> &'static str {
        match self {
            Self::Blue => "Blue",
            Self::Amber => "Amber",
            Self::Green => "Green",
            Self::Red => "Red",
            Self::Purple => "Purple",
            Self::Cyan => "Cyan",
        }
    }

    fn color(self) -> egui::Color32 {
        match self {
            Self::Blue => egui::Color32::from_rgb(93, 185, 255),
            Self::Amber => egui::Color32::from_rgb(255, 196, 87),
            Self::Green => egui::Color32::from_rgb(135, 220, 140),
            Self::Red => egui::Color32::from_rgb(247, 118, 142),
            Self::Purple => egui::Color32::from_rgb(187, 154, 247),
            Self::Cyan => egui::Color32::from_rgb(125, 207, 255),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WaveformTraceStyle {
    trace: WaveformTraceRef,
    color: Option<WaveformTraceColor>,
    visible: bool,
}

impl WaveformTraceStyle {
    fn default_for(trace: WaveformTraceRef) -> Self {
        Self {
            trace,
            color: None,
            visible: true,
        }
    }

    fn is_default(self) -> bool {
        self.color.is_none() && self.visible
    }
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

#[derive(Debug, Clone, PartialEq)]
struct ScopeCursorLegendRow {
    selected: bool,
    trace: WaveformTraceRef,
    label: String,
    unit: &'static str,
    cursor_a_value: f64,
    cursor_b_value: f64,
    delta_value: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct ScopeRegionStatsRow {
    selected: bool,
    trace: WaveformTraceRef,
    label: String,
    unit: &'static str,
    min: f64,
    max: f64,
    mean: f64,
    rms: f64,
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

fn scope_cursor_legend_rows(
    waveforms: &[WaveformView],
    traces: &[WaveformTraceRef],
    cursor_a_us: f64,
    cursor_b_us: f64,
) -> Vec<ScopeCursorLegendRow> {
    traces
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(trace_order, trace)| {
            let waveform = waveforms.get(trace.waveform_index)?;
            let probe = waveform.probes.get(trace.probe_index)?;
            let cursor_a = cursor_measurement(waveform, probe, cursor_a_us)?;
            let cursor_b = cursor_measurement(waveform, probe, cursor_b_us)?;
            let trace_label = probe.expression.as_deref().unwrap_or(&probe.label);
            let label = if waveforms.len() > 1 {
                format!("{} / {trace_label}", waveform.label)
            } else {
                trace_label.to_string()
            };
            Some(ScopeCursorLegendRow {
                selected: trace_order == 0,
                trace,
                label,
                unit: probe_unit(&probe.label),
                cursor_a_value: cursor_a.value,
                cursor_b_value: cursor_b.value,
                delta_value: cursor_b.value - cursor_a.value,
            })
        })
        .collect()
}

fn scope_region_stats_rows(
    waveforms: &[WaveformView],
    traces: &[WaveformTraceRef],
    start_us: f64,
    end_us: f64,
) -> Vec<ScopeRegionStatsRow> {
    let (start_s, end_s) = ordered_pair(start_us / 1e6, end_us / 1e6);
    traces
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(trace_order, trace)| {
            let waveform = waveforms.get(trace.waveform_index)?;
            let probe = waveform.probes.get(trace.probe_index)?;
            let stats = waveform_region_stats(&waveform.time_s, &probe.values, start_s, end_s)?;
            let trace_label = probe.expression.as_deref().unwrap_or(&probe.label);
            let label = if waveforms.len() > 1 {
                format!("{} / {trace_label}", waveform.label)
            } else {
                trace_label.to_string()
            };
            Some(ScopeRegionStatsRow {
                selected: trace_order == 0,
                trace,
                label,
                unit: probe_unit(&probe.label),
                min: stats.min,
                max: stats.max,
                mean: stats.mean,
                rms: stats.rms,
            })
        })
        .collect()
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

fn probe_unit(label: &str) -> &'static str {
    let normalized = label.trim().to_ascii_lowercase();
    if normalized.contains("mag_db") || normalized.contains("magnitude db") {
        "dB"
    } else if normalized.contains("phase_deg") || normalized.contains("phase") {
        "deg"
    } else if normalized.ends_with("_mag") || normalized.contains("linear magnitude") {
        "ratio"
    } else if normalized.contains("noise density")
        || normalized.contains("v_per_sqrt_hz")
        || normalized.contains("v/sqrt")
    {
        "V/sqrt(Hz)"
    } else if normalized.starts_with("i(")
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
                "{} range {}",
                waveform.x_axis.display_name(),
                waveform.x_axis.format_cursor_us((end_us - start_us).abs())
            ));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Cursor A");
            ui.add(
                egui::DragValue::new(cursor_a_us)
                    .speed(((end_us - start_us) / 200.0).max(0.001))
                    .range(start_us..=end_us)
                    .suffix(waveform.x_axis.input_suffix()),
            );
            ui.label("Cursor B");
            ui.add(
                egui::DragValue::new(cursor_b_us)
                    .speed(((end_us - start_us) / 200.0).max(0.001))
                    .range(start_us..=end_us)
                    .suffix(waveform.x_axis.input_suffix()),
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
                    ui.monospace(
                        waveform
                            .x_axis
                            .format_storage_value(measurement.cursor_a.time_s),
                    );
                    ui.label("value");
                    ui.monospace(format_value(measurement.cursor_a.value));
                    ui.end_row();

                    ui.label("B");
                    ui.monospace(
                        waveform
                            .x_axis
                            .format_storage_value(measurement.cursor_b.time_s),
                    );
                    ui.label("value");
                    ui.monospace(format_value(measurement.cursor_b.value));
                    ui.end_row();

                    ui.label("Delta");
                    ui.monospace(waveform.x_axis.format_delta_storage(measurement.delta_t_s));
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

fn waveform_frequency_panel(ui: &mut egui::Ui, waveform: &WaveformView, probe_index: usize) {
    if waveform.x_axis == WaveformXAxis::FrequencyHz {
        return;
    }
    let Some(peaks) = waveform_spectrum_peaks(waveform, probe_index, 5) else {
        return;
    };
    ui.group(|ui| {
        ui.horizontal_wrapped(|ui| {
            ui.strong("Frequency Domain");
            if let Some(peak) = peaks.first() {
                ui.label(format!(
                    "dominant {}",
                    format_frequency_hz(peak.frequency_hz)
                ));
            }
        });
        egui::Grid::new("waveform_frequency_peaks")
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Peak");
                ui.label("Frequency");
                ui.label("Magnitude");
                ui.end_row();
                for (index, peak) in peaks.iter().enumerate() {
                    ui.monospace((index + 1).to_string());
                    ui.monospace(format_frequency_hz(peak.frequency_hz));
                    ui.monospace(format_value(peak.magnitude));
                    ui.end_row();
                }
            });
    });
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

struct WaveformRegionStats {
    min: f64,
    max: f64,
    mean: f64,
    rms: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct WaveformSpectrumPeak {
    frequency_hz: f64,
    magnitude: f64,
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

pub(super) fn waveform_spectrum_peaks(
    waveform: &WaveformView,
    probe_index: usize,
    max_peaks: usize,
) -> Option<Vec<WaveformSpectrumPeak>> {
    let probe = waveform.probes.get(probe_index)?;
    waveform_spectrum_peaks_from_samples(&waveform.time_s, &probe.values, max_peaks)
}

pub(super) fn waveform_spectrum_peaks_from_samples(
    times: &[f64],
    values: &[f64],
    max_peaks: usize,
) -> Option<Vec<WaveformSpectrumPeak>> {
    const MAX_SPECTRUM_SAMPLES: usize = 512;
    if times.len() != values.len() || times.len() < 8 || max_peaks == 0 {
        return None;
    }
    let start_s = *times.first()?;
    let end_s = *times.last()?;
    let duration_s = end_s - start_s;
    if !duration_s.is_finite() || duration_s <= 0.0 {
        return None;
    }
    let sample_cap = times.len().min(MAX_SPECTRUM_SAMPLES);
    let sample_count = sample_cap.next_power_of_two() >> usize::from(!sample_cap.is_power_of_two());
    if sample_count < 8 {
        return None;
    }
    let step_s = duration_s / (sample_count - 1) as f64;
    let mut samples = (0..sample_count)
        .filter_map(|index| {
            let time_s = start_s + index as f64 * step_s;
            interpolated_value(times, values, time_s)
        })
        .collect::<Vec<_>>();
    if samples.len() != sample_count {
        return None;
    }
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    for (index, sample) in samples.iter_mut().enumerate() {
        let window = 0.5
            - 0.5 * (2.0 * std::f64::consts::PI * index as f64 / (sample_count - 1) as f64).cos();
        *sample = (*sample - mean) * window;
    }

    let sample_rate_hz = (sample_count - 1) as f64 / duration_s;
    let mut peaks = Vec::with_capacity(sample_count / 2);
    for bin in 1..=(sample_count / 2) {
        let mut real = 0.0;
        let mut imag = 0.0;
        for (index, sample) in samples.iter().copied().enumerate() {
            let angle =
                -2.0 * std::f64::consts::PI * bin as f64 * index as f64 / sample_count as f64;
            real += sample * angle.cos();
            imag += sample * angle.sin();
        }
        let magnitude = 2.0 * real.hypot(imag) / sample_count as f64;
        if magnitude.is_finite() {
            peaks.push(WaveformSpectrumPeak {
                frequency_hz: bin as f64 * sample_rate_hz / sample_count as f64,
                magnitude,
            });
        }
    }
    peaks.sort_by(|left, right| right.magnitude.total_cmp(&left.magnitude));
    peaks.truncate(max_peaks);
    Some(peaks)
}

fn waveform_region_stats(
    times: &[f64],
    values: &[f64],
    start_s: f64,
    end_s: f64,
) -> Option<WaveformRegionStats> {
    if times.len() != values.len() || times.is_empty() || !start_s.is_finite() || !end_s.is_finite()
    {
        return None;
    }
    let (start_s, end_s) = ordered_pair(start_s, end_s);
    if (end_s - start_s).abs() < f64::EPSILON {
        let value = interpolated_value(times, values, start_s)?;
        return Some(WaveformRegionStats {
            min: value,
            max: value,
            mean: value,
            rms: value.abs(),
        });
    }

    let start_value = interpolated_value(times, values, start_s)?;
    let end_value = interpolated_value(times, values, end_s)?;
    let mut points = Vec::with_capacity(times.len().saturating_add(2));
    points.push((start_s, start_value));
    for (time, value) in times.iter().copied().zip(values.iter().copied()) {
        if time > start_s && time < end_s {
            points.push((time, value));
        }
    }
    points.push((end_s, end_value));
    points.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    points.dedup_by(|left, right| (left.0 - right.0).abs() < f64::EPSILON);
    if points.is_empty() {
        return None;
    }

    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for (_, value) in &points {
        min = min.min(*value);
        max = max.max(*value);
    }

    let mut integral = 0.0;
    let mut square_integral = 0.0;
    for pair in points.windows(2) {
        let (left_t, left_v) = pair[0];
        let (right_t, right_v) = pair[1];
        let dt = right_t - left_t;
        if dt <= 0.0 {
            continue;
        }
        integral += dt * (left_v + right_v) * 0.5;
        square_integral += dt * (left_v * left_v + left_v * right_v + right_v * right_v) / 3.0;
    }
    let duration = end_s - start_s;
    if duration <= 0.0 {
        return None;
    }
    let mean = integral / duration;
    let rms = (square_integral / duration).max(0.0).sqrt();
    Some(WaveformRegionStats {
        min,
        max,
        mean,
        rms,
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

fn waveform_matches_load_diagnostic(
    waveform: &WaveformView,
    diagnostic: &WaveformLoadDiagnostic,
) -> bool {
    if waveform.path != diagnostic.path || !diagnostic.loaded || diagnostic.deferred {
        return false;
    }
    if !diagnostic.is_selected_column_update() {
        return waveform_probe_labels(waveform).len() == diagnostic.probes;
    }
    waveform_probe_labels_equal(&waveform_probe_labels(waveform), &diagnostic.probe_preview)
}

fn waveform_probe_labels(waveform: &WaveformView) -> Vec<String> {
    waveform
        .probes
        .iter()
        .filter(|probe| !probe.derived)
        .map(|probe| probe.label.clone())
        .collect()
}

fn waveform_probe_labels_equal(left: &[String], right: &[String]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
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

fn format_frequency_hz(value: f64) -> String {
    if value.abs() >= 1.0e6 {
        format!("{:.6e} MHz", value / 1.0e6)
    } else if value.abs() >= 1.0e3 {
        format!("{:.6e} kHz", value / 1.0e3)
    } else {
        format!("{value:.6e} Hz")
    }
}

pub(super) fn format_value(value: f64) -> String {
    format!("{value:.6e}")
}

#[cfg(test)]
mod waveform_bundle_tests;
#[cfg(test)]
mod waveform_loading_tests;
#[cfg(test)]
mod waveform_measurement_tests;
#[cfg(test)]
mod waveform_scope_activity_tests;
#[cfg(test)]
mod waveform_scope_compare_tests;
#[cfg(test)]
mod waveform_scope_tests;
#[cfg(test)]
mod waveform_test_support;
#[cfg(test)]
mod waveform_tests;
