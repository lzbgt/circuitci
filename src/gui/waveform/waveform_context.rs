use super::{WaveformProbe, WaveformTraceRef, WaveformView};
use crate::gui::sketch::SketchSelection;
use crate::gui::sketch_probes::{SketchProbe, SketchProbeTarget};
use crate::gui::{CircuitCiApp, ScopeProbeTarget, SketchViewportCommand, Stage};
use eframe::egui;

impl CircuitCiApp {
    pub(in crate::gui) fn open_scope_probe_target(&mut self, target: ScopeProbeTarget) {
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

    pub(in crate::gui) fn scope_probe_target_pinned_for_compare(
        &self,
        target: &ScopeProbeTarget,
    ) -> bool {
        self.scope_probe_target_trace_ref(target)
            .is_some_and(|trace| self.waveform_pinned_traces.contains(&trace))
    }

    pub(in crate::gui) fn pin_scope_probe_target_for_compare(
        &mut self,
        target: ScopeProbeTarget,
    ) -> bool {
        let Some(trace) = self.scope_probe_target_trace_ref(&target) else {
            self.status = format!(
                "Scope trace {} from scenario {} is not loaded yet.",
                target.probe_name, target.scenario_name
            );
            return false;
        };
        if self.waveform_pinned_traces.contains(&trace) {
            self.status = format!(
                "Scope trace {} is already pinned for comparison.",
                target.probe_name
            );
            return true;
        }
        self.waveform_pinned_traces.push(trace);
        self.status = format!(
            "Pinned scope trace {} for comparison from Sketch.",
            target.probe_name
        );
        true
    }

    pub(in crate::gui) fn unpin_scope_probe_target_for_compare(
        &mut self,
        target: ScopeProbeTarget,
    ) -> bool {
        let Some(trace) = self.scope_probe_target_trace_ref(&target) else {
            self.status = format!(
                "Scope trace {} from scenario {} is not loaded yet.",
                target.probe_name, target.scenario_name
            );
            return false;
        };
        let before = self.waveform_pinned_traces.len();
        self.waveform_pinned_traces
            .retain(|pinned| *pinned != trace);
        if before == self.waveform_pinned_traces.len() {
            self.status = format!(
                "Scope trace {} is not pinned for comparison.",
                target.probe_name
            );
            return false;
        }
        self.status = format!(
            "Unpinned scope trace {} from Sketch compare.",
            target.probe_name
        );
        true
    }

    pub(in crate::gui) fn clear_scope_compare_pins_from_sketch(&mut self) -> usize {
        let count = self.waveform_pinned_traces.len();
        self.waveform_pinned_traces.clear();
        self.status = format!("Cleared {count} pinned scope trace(s) from Sketch.");
        count
    }

    pub(in crate::gui) fn open_pinned_scope_compare(&mut self) -> bool {
        self.prune_scope_trace_pins();
        let Some(trace) = self.waveform_pinned_traces.first().copied() else {
            self.status =
                "Pin at least one loaded Scope Activity trace before opening compare.".to_string();
            return false;
        };
        self.selected_waveform = trace.waveform_index;
        self.selected_probe = trace.probe_index;
        self.waveform_math_left = trace.probe_index;
        self.waveform_math_right = trace.probe_index;
        self.waveform_playing = false;
        self.stage = Stage::Simulation;
        self.status = format!(
            "Opened Scopes compare with {} pinned trace(s).",
            self.waveform_pinned_traces.len()
        );
        true
    }

    pub(in crate::gui) fn remember_scope_probe_target(
        &mut self,
        scenario_name: &str,
        probe_name: &str,
    ) {
        self.pending_scope_probe = Some(ScopeProbeTarget {
            scenario_name: scenario_name.trim().to_string(),
            probe_name: probe_name.trim().to_string(),
        });
    }

    pub(in crate::gui) fn apply_pending_scope_probe_focus(&mut self) -> bool {
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
        self.waveform_window_start_us = None;
        self.waveform_window_end_us = None;
        self.waveform_value_min = None;
        self.waveform_value_max = None;
        self.clear_waveform_view_history();
        self.waveform_playing = false;
        true
    }

    fn scope_probe_target_trace_ref(&self, target: &ScopeProbeTarget) -> Option<WaveformTraceRef> {
        let (waveform_index, probe_index) =
            find_scope_probe(&self.waveforms, &target.scenario_name, &target.probe_name)?;
        Some(WaveformTraceRef {
            waveform_index,
            probe_index,
        })
    }

    pub(super) fn focus_selected_scope_schematic_context(&mut self) -> bool {
        self.focus_selected_scope_schematic_context_with_status(true)
    }

    pub(super) fn focus_selected_scope_schematic_context_silent(&mut self) -> bool {
        self.focus_selected_scope_schematic_context_with_status(false)
    }

    fn focus_selected_scope_schematic_context_with_status(&mut self, update_status: bool) -> bool {
        let Some(sketch_probe) = self.selected_scope_sketch_probe() else {
            if update_status {
                self.status =
                    "Selected scope trace is not linked to a schematic probe.".to_string();
            }
            return false;
        };
        let selection = match &sketch_probe.target {
            SketchProbeTarget::Component(component_id) => {
                SketchSelection::Component(component_id.clone())
            }
            SketchProbeTarget::Net(net_id) => SketchSelection::Net(net_id.clone()),
        };
        self.selected_sketch_item = Some(selection.clone());
        self.selected_sketch_items.clear();
        self.selected_sketch_items.insert(selection);
        self.remember_scope_probe_target(&sketch_probe.scenario_name, &sketch_probe.probe_name);
        if update_status {
            self.status = format!(
                "Focused schematic {} for scope probe {}.",
                sketch_probe_target_label(&sketch_probe.target),
                sketch_probe.probe_name
            );
        }
        true
    }

    pub(super) fn open_selected_scope_schematic_context(&mut self, fit_context: bool) -> bool {
        let Some(sketch_probe) = self.selected_scope_sketch_probe() else {
            self.status = "Selected scope trace is not linked to a schematic probe.".to_string();
            return false;
        };
        let target_label = sketch_probe_target_label(&sketch_probe.target);
        if !self.focus_selected_scope_schematic_context_with_status(false) {
            return false;
        }
        self.stage = Stage::Sketch;
        if fit_context {
            self.sketch_viewport_command = Some(SketchViewportCommand::FitSelection);
            self.status = format!("Opened Sketch and queued fit for {target_label}.");
        } else {
            self.status = format!("Opened Sketch with {target_label} selected.");
        }
        true
    }

    pub(super) fn selected_scope_sketch_probe(&self) -> Option<SketchProbe> {
        let waveform = self.waveforms.get(self.selected_waveform)?;
        let probe = waveform.probes.get(self.selected_probe)?;
        let snapshot = self.project_snapshot.as_ref()?;
        snapshot
            .probes
            .iter()
            .find(|sketch_probe| {
                scope_waveform_matches_scenario(waveform, &sketch_probe.scenario_name)
                    && scope_probe_matches_sketch_probe(probe, sketch_probe)
            })
            .cloned()
            .or_else(|| {
                snapshot
                    .probes
                    .iter()
                    .find(|sketch_probe| scope_probe_matches_sketch_probe(probe, sketch_probe))
                    .cloned()
            })
    }

    pub(super) fn waveform_schematic_context_strip(&mut self, ui: &mut egui::Ui) {
        let Some(sketch_probe) = self.selected_scope_sketch_probe() else {
            return;
        };
        let target_label = sketch_probe_target_label(&sketch_probe.target);
        let mut open_sketch = false;
        let mut fit_context = false;
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Schematic Context");
                ui.monospace(&target_label);
                ui.label(format!(
                    "{} probe {}",
                    sketch_probe.quantity.label(),
                    sketch_probe.probe_name
                ));
                if ui.button("Open Sketch").clicked() {
                    open_sketch = true;
                }
                if ui.button("Fit Context").clicked() {
                    fit_context = true;
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("scenario");
                ui.monospace(&sketch_probe.scenario_name);
                ui.label("expression");
                ui.monospace(&sketch_probe.expression);
            });
        });
        if open_sketch {
            self.open_selected_scope_schematic_context(false);
        }
        if fit_context {
            self.open_selected_scope_schematic_context(true);
        }
    }
}

pub(super) fn find_scope_probe(
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
            let scenario_matches = scope_waveform_matches_scenario(waveform, scenario_name);
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

fn scope_waveform_matches_scenario(waveform: &WaveformView, scenario_name: &str) -> bool {
    let scenario_name = scenario_name.trim();
    scenario_name.is_empty()
        || waveform.label.contains(scenario_name)
        || waveform.path.contains(scenario_name)
}

fn scope_probe_matches_sketch_probe(probe: &WaveformProbe, sketch_probe: &SketchProbe) -> bool {
    let label = probe.label.trim();
    let expression = probe.expression.as_deref().unwrap_or(label).trim();
    label.eq_ignore_ascii_case(sketch_probe.probe_name.trim())
        || label.eq_ignore_ascii_case(sketch_probe.expression.trim())
        || expression.eq_ignore_ascii_case(sketch_probe.probe_name.trim())
        || expression.eq_ignore_ascii_case(sketch_probe.expression.trim())
}

fn sketch_probe_target_label(target: &SketchProbeTarget) -> String {
    match target {
        SketchProbeTarget::Component(component_id) => format!("component {component_id}"),
        SketchProbeTarget::Net(net_id) => format!("net {net_id}"),
    }
}
