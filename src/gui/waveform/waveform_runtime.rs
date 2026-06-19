use crate::gui::ScopeProbeTarget;
use crate::gui::sketch::{ProjectSnapshot, SketchSelection};

use super::waveform_trigger::{
    ScopeTriggerEdge, ScopeTriggerJump, scope_trigger_events, select_scope_trigger_event,
};
use super::{
    WaveformView, format_frequency_hz, format_time_s, format_value, interpolated_value, min_max,
    probe_unit, waveform_spectrum_peaks,
};

pub(in crate::gui) fn runtime_probe_lines_for_selection(
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

pub(in crate::gui) fn runtime_probe_activity_for_selection(
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

pub(in crate::gui) fn runtime_scope_probe_sample_label(
    waveforms: &[WaveformView],
    waveform_index: usize,
    cursor_us: f64,
    target: &ScopeProbeTarget,
) -> Option<String> {
    let waveform = waveforms.get(waveform_index)?;
    if waveform.label != target.scenario_name {
        return None;
    }
    let cursor_s = cursor_us / 1e6;
    let probe = waveform.probes.iter().find(|probe| {
        probe
            .label
            .trim()
            .eq_ignore_ascii_case(target.probe_name.trim())
    })?;
    let value = interpolated_value(&waveform.time_s, &probe.values, cursor_s)?;
    Some(format!(
        "{} {} @ {}",
        format_value(value),
        probe_unit(&probe.label),
        format_time_s(cursor_s)
    ))
}

pub(in crate::gui) fn runtime_scope_probe_sparkline_points(
    waveforms: &[WaveformView],
    waveform_index: usize,
    target: &ScopeProbeTarget,
    point_count: usize,
) -> Option<Vec<(f32, f32)>> {
    let waveform = waveforms.get(waveform_index)?;
    if waveform.label != target.scenario_name {
        return None;
    }
    let probe = waveform.probes.iter().find(|probe| {
        probe
            .label
            .trim()
            .eq_ignore_ascii_case(target.probe_name.trim())
    })?;
    let start_s = *waveform.time_s.first()?;
    let end_s = *waveform.time_s.last()?;
    if !start_s.is_finite() || !end_s.is_finite() || end_s <= start_s {
        return None;
    }
    let (min, max) = finite_min_max(&probe.values)?;
    let count = point_count.clamp(2, 48);
    let value_span = max - min;
    let mut points = Vec::with_capacity(count);
    for index in 0..count {
        let x = index as f64 / (count - 1) as f64;
        let time_s = start_s + (end_s - start_s) * x;
        let value = interpolated_value(&waveform.time_s, &probe.values, time_s)?;
        let y = if value_span.abs() <= f64::EPSILON {
            0.5
        } else {
            ((value - min) / value_span).clamp(0.0, 1.0)
        };
        points.push((x as f32, y as f32));
    }
    Some(points)
}

pub(in crate::gui) fn runtime_scope_probe_frequency_label(
    waveforms: &[WaveformView],
    waveform_index: usize,
    target: &ScopeProbeTarget,
) -> Option<String> {
    let waveform = waveforms.get(waveform_index)?;
    if waveform.label != target.scenario_name {
        return None;
    }
    let probe_index = waveform.probes.iter().position(|probe| {
        probe
            .label
            .trim()
            .eq_ignore_ascii_case(target.probe_name.trim())
    })?;
    let peak = waveform_spectrum_peaks(waveform, probe_index, 1)?
        .into_iter()
        .next()?;
    if !peak.frequency_hz.is_finite() || peak.frequency_hz <= 0.0 {
        return None;
    }
    Some(format!(
        "f {} · T {}",
        format_frequency_hz(peak.frequency_hz),
        format_time_s(1.0 / peak.frequency_hz)
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gui) enum RuntimeScopeProbeEdgeStep {
    Previous,
    Next,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::gui) struct RuntimeScopeProbeEdgeJump {
    pub(in crate::gui) time_us: f64,
    pub(in crate::gui) label: String,
}

pub(in crate::gui) fn runtime_scope_probe_edge_jump(
    waveforms: &[WaveformView],
    waveform_index: usize,
    cursor_us: f64,
    target: &ScopeProbeTarget,
    step: RuntimeScopeProbeEdgeStep,
) -> Option<RuntimeScopeProbeEdgeJump> {
    let waveform = waveforms.get(waveform_index)?;
    if waveform.label != target.scenario_name {
        return None;
    }
    let probe_index = waveform.probes.iter().position(|probe| {
        probe
            .label
            .trim()
            .eq_ignore_ascii_case(target.probe_name.trim())
    })?;
    let probe = waveform.probes.get(probe_index)?;
    let (min, max) = min_max(&probe.values)?;
    if (max - min).abs() <= f64::EPSILON {
        return None;
    }
    let threshold = (min + max) * 0.5;
    let events = scope_trigger_events(waveform, probe_index, threshold, ScopeTriggerEdge::Either);
    let event = select_scope_trigger_event(
        &events,
        cursor_us,
        match step {
            RuntimeScopeProbeEdgeStep::Previous => ScopeTriggerJump::Previous,
            RuntimeScopeProbeEdgeStep::Next => ScopeTriggerJump::Next,
        },
    )?;
    Some(RuntimeScopeProbeEdgeJump {
        time_us: event.time_us,
        label: format!(
            "{} edge @ {} ({})",
            event.edge.label(),
            format_time_s(event.time_us / 1e6),
            format_value(event.value)
        ),
    })
}

pub(in crate::gui) fn runtime_scope_probe_target_for_selection(
    waveforms: &[WaveformView],
    waveform_index: usize,
    selection: &SketchSelection,
    snapshot: &ProjectSnapshot,
) -> Option<ScopeProbeTarget> {
    let waveform = waveforms.get(waveform_index)?;
    let target = runtime_probe_target(selection, snapshot)?;
    waveform
        .probes
        .iter()
        .filter_map(|probe| {
            probe_target_match_rank(&probe.label, &target).map(|rank| (rank, probe))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, probe)| ScopeProbeTarget {
            scenario_name: waveform.label.clone(),
            probe_name: probe.label.clone(),
        })
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
    probe_target_match_rank(label, target).is_some()
}

fn probe_target_match_rank(label: &str, target: &RuntimeProbeTarget) -> Option<u8> {
    let normalized_label = normalized_probe_token(label);
    if let Some(component_id) = &target.component_id {
        let component = normalized_probe_token(component_id);
        if !component.is_empty() && normalized_label.contains(&component) {
            return Some(0);
        }
    }
    target
        .net_ids
        .iter()
        .any(|net_id| {
            let net = normalized_probe_token(net_id);
            !net.is_empty() && normalized_label.contains(&net)
        })
        .then_some(1)
}

fn normalized_probe_token(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn finite_min_max(values: &[f64]) -> Option<(f64, f64)> {
    let mut iter = values.iter().copied().filter(|value| value.is_finite());
    let first = iter.next()?;
    Some(iter.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    }))
}
