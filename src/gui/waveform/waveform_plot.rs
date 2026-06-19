use super::{
    WaveformProbe, WaveformTraceColor, WaveformTraceRef, WaveformTraceStyle, WaveformView,
    interpolated_value, min_max, ordered_pair, positive_span, waveform_time_range_for_view,
    waveform_time_range_us, window_min_max,
};
use eframe::egui;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui) enum WaveformCursorTarget {
    A,
    B,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct WaveformPlotInteraction {
    pub(super) time_window_us: Option<(f64, f64)>,
    pub(super) value_window: Option<(f64, f64)>,
    pub(super) cursor_a_us: Option<f64>,
    pub(super) cursor_b_us: Option<f64>,
}

pub(super) struct WaveformPlotCursors<'a> {
    pub(super) cursor_a_us: f64,
    pub(super) cursor_b_us: f64,
    pub(super) active_drag: &'a mut Option<WaveformCursorTarget>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformPlotTrigger<'a> {
    pub(super) threshold: f64,
    pub(super) events_us: &'a [f64],
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct WaveformPlotView<'a> {
    pub(super) visible_window_us: Option<(f64, f64)>,
    pub(super) visible_value_window: Option<(f64, f64)>,
    pub(super) trigger: Option<WaveformPlotTrigger<'a>>,
}

impl WaveformPlotInteraction {
    fn set_cursor(&mut self, target: WaveformCursorTarget, time_us: f64) {
        match target {
            WaveformCursorTarget::A => self.cursor_a_us = Some(time_us),
            WaveformCursorTarget::B => self.cursor_b_us = Some(time_us),
        }
    }
}

pub(super) fn waveform_time_window_for_view(
    waveforms: &[WaveformView],
    waveform_index: usize,
    start_us: Option<f64>,
    end_us: Option<f64>,
) -> Option<(f64, f64)> {
    match (start_us, end_us) {
        (Some(start_us), Some(end_us)) => {
            clamp_waveform_time_window(waveforms, waveform_index, start_us, end_us)
        }
        _ => waveform_time_range_for_view(waveforms, waveform_index),
    }
}

pub(super) fn clamp_waveform_time_window(
    waveforms: &[WaveformView],
    waveform_index: usize,
    start_us: f64,
    end_us: f64,
) -> Option<(f64, f64)> {
    let (full_start_us, full_end_us) = waveform_time_range_for_view(waveforms, waveform_index)?;
    if !start_us.is_finite() || !end_us.is_finite() {
        return Some((full_start_us, full_end_us));
    }
    let (mut start_us, mut end_us) = ordered_pair(start_us, end_us);
    let full_span = positive_span(full_start_us, full_end_us);
    let min_span = (full_span * 0.0001).max(1e-9);
    if end_us - start_us < min_span {
        let center = (start_us + end_us) * 0.5;
        start_us = center - min_span * 0.5;
        end_us = center + min_span * 0.5;
    }
    if start_us < full_start_us {
        let shift = full_start_us - start_us;
        start_us += shift;
        end_us += shift;
    }
    if end_us > full_end_us {
        let shift = end_us - full_end_us;
        start_us -= shift;
        end_us -= shift;
    }
    start_us = start_us.clamp(full_start_us, full_end_us);
    end_us = end_us.clamp(full_start_us, full_end_us);
    if end_us <= start_us {
        Some((full_start_us, full_end_us))
    } else {
        Some((start_us, end_us))
    }
}

pub(super) fn zoom_time_window(
    start_us: f64,
    end_us: f64,
    focus_us: f64,
    scale: f64,
) -> (f64, f64) {
    let scale = scale.clamp(0.05, 20.0);
    let span = positive_span(start_us, end_us);
    let new_span = span * scale;
    let focus_ratio = ((focus_us - start_us) / span).clamp(0.0, 1.0);
    let new_start = focus_us - new_span * focus_ratio;
    (new_start, new_start + new_span)
}

pub(super) fn expanded_value_bounds(value_min: f64, value_max: f64) -> Option<(f64, f64)> {
    if !value_min.is_finite() || !value_max.is_finite() {
        return None;
    }
    let (value_min, value_max) = ordered_pair(value_min, value_max);
    if value_max > value_min {
        Some((value_min, value_max))
    } else {
        let pad = value_min.abs().max(1.0) * 0.05;
        Some((value_min - pad, value_max + pad))
    }
}

pub(super) fn clamp_value_window(
    data_min: f64,
    data_max: f64,
    value_min: f64,
    value_max: f64,
) -> Option<(f64, f64)> {
    let (full_min, full_max) = expanded_value_bounds(data_min, data_max)?;
    if !value_min.is_finite() || !value_max.is_finite() {
        return Some((full_min, full_max));
    }
    let (mut value_min, mut value_max) = ordered_pair(value_min, value_max);
    let full_span = positive_span(full_min, full_max);
    let min_span = (full_span * 0.0001).max(1e-12);
    if value_max - value_min < min_span {
        let center = (value_min + value_max) * 0.5;
        value_min = center - min_span * 0.5;
        value_max = center + min_span * 0.5;
    }
    if value_min < full_min {
        let shift = full_min - value_min;
        value_min += shift;
        value_max += shift;
    }
    if value_max > full_max {
        let shift = value_max - full_max;
        value_min -= shift;
        value_max -= shift;
    }
    value_min = value_min.clamp(full_min, full_max);
    value_max = value_max.clamp(full_min, full_max);
    if value_max <= value_min {
        Some((full_min, full_max))
    } else {
        Some((value_min, value_max))
    }
}

pub(super) fn draw_waveform_plot_sized(
    ui: &mut egui::Ui,
    waveforms: &[WaveformView],
    traces: &[WaveformTraceRef],
    cursors: WaveformPlotCursors<'_>,
    view: WaveformPlotView<'_>,
    trace_styles: &[WaveformTraceStyle],
    desired_size: egui::Vec2,
) -> WaveformPlotInteraction {
    let Some(primary) = traces
        .first()
        .and_then(|trace| waveform_trace(waveforms, *trace))
    else {
        ui.label("No valid scope trace is selected.");
        return WaveformPlotInteraction::default();
    };
    let Some((full_start_us, full_end_us)) = waveform_time_range_us(primary.0) else {
        ui.label("Waveform has no time samples.");
        return WaveformPlotInteraction::default();
    };
    let (window_start_us, window_end_us) = view
        .visible_window_us
        .unwrap_or((full_start_us, full_end_us));
    let x_min = window_start_us / 1e6;
    let x_max = window_end_us / 1e6;
    let Some((data_y_min, data_y_max)) =
        waveform_trace_bounds_in_window(waveforms, traces, x_min, x_max)
    else {
        ui.label("Waveform has no time samples.");
        return WaveformPlotInteraction::default();
    };
    let Some((data_y_min, data_y_max)) = expanded_value_bounds(data_y_min, data_y_max) else {
        ui.label("Waveform has no finite value samples.");
        return WaveformPlotInteraction::default();
    };
    let (y_min, y_max) = view
        .visible_value_window
        .and_then(|(value_min, value_max)| {
            clamp_value_window(data_y_min, data_y_max, value_min, value_max)
        })
        .unwrap_or((data_y_min, data_y_max));

    ui.label(format!(
        "{} samples from {}; showing {} trace(s)",
        primary.0.time_s.len(),
        primary.0.path,
        traces.len()
    ));
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(16));

    let plot_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(56.0, 16.0),
        rect.max - egui::vec2(16.0, 38.0),
    );
    let mut interaction = WaveformPlotInteraction::default();
    let x_span_us = positive_span(window_start_us, window_end_us);
    let pointer_pos = response.interact_pointer_pos();
    let pointer_in_plot = pointer_pos.is_some_and(|pos| plot_rect.contains(pos));
    let cursor_target_at_pointer = pointer_pos
        .filter(|pos| plot_rect.expand(16.0).contains(*pos))
        .and_then(|pos| {
            nearest_scope_cursor_target(
                pos,
                plot_rect,
                cursors.cursor_a_us,
                cursors.cursor_b_us,
                window_start_us,
                window_end_us,
            )
        });
    if response.drag_started_by(egui::PointerButton::Primary) {
        *cursors.active_drag = cursor_target_at_pointer.or_else(|| {
            ui.input(|input| input.modifiers.shift)
                .then_some(WaveformCursorTarget::B)
        });
    }
    if response.drag_stopped_by(egui::PointerButton::Primary)
        || !ui.input(|input| input.pointer.primary_down())
    {
        *cursors.active_drag = None;
    }
    if response.clicked_by(egui::PointerButton::Primary)
        && pointer_in_plot
        && let Some(position) = pointer_pos
    {
        let target = cursor_target_at_pointer
            .or_else(|| {
                ui.input(|input| input.modifiers.shift)
                    .then_some(WaveformCursorTarget::B)
            })
            .unwrap_or(WaveformCursorTarget::A);
        interaction.set_cursor(
            target,
            plot_x_to_time_us(position.x, plot_rect, window_start_us, window_end_us),
        );
    }
    if response.dragged() {
        if let (Some(target), Some(position)) = (*cursors.active_drag, pointer_pos) {
            interaction.set_cursor(
                target,
                plot_x_to_time_us(position.x, plot_rect, window_start_us, window_end_us),
            );
        } else {
            let delta = ui.input(|input| input.pointer.delta());
            if delta.x.abs() > f32::EPSILON && plot_rect.width() > 1.0 {
                let delta_us = -(delta.x as f64 / plot_rect.width() as f64) * x_span_us;
                interaction.time_window_us =
                    Some((window_start_us + delta_us, window_end_us + delta_us));
            }
            if delta.y.abs() > f32::EPSILON && plot_rect.height() > 1.0 {
                let y_span = positive_span(y_min, y_max);
                let delta_value = (delta.y as f64 / plot_rect.height() as f64) * y_span;
                interaction.value_window = Some((y_min + delta_value, y_max + delta_value));
            }
        }
    }
    if response.hovered() {
        let (zoom_delta, scroll_delta, pointer, shift) = ui.input(|input| {
            (
                input.zoom_delta(),
                input.smooth_scroll_delta,
                input.pointer.hover_pos(),
                input.modifiers.shift,
            )
        });
        let scale = if (zoom_delta - 1.0).abs() > f32::EPSILON {
            Some(1.0 / zoom_delta as f64)
        } else if scroll_delta.y.abs() > f32::EPSILON {
            Some(if scroll_delta.y > 0.0 { 0.88 } else { 1.14 })
        } else {
            None
        };
        if let Some(scale) = scale {
            if shift {
                let focus_value = pointer
                    .map(|pos| plot_y_to_value(pos.y, plot_rect, y_min, y_max))
                    .unwrap_or((y_min + y_max) * 0.5);
                interaction.value_window = Some(zoom_time_window(y_min, y_max, focus_value, scale));
            } else {
                let focus_ratio = pointer
                    .map(|pos| ((pos.x - plot_rect.left()) / plot_rect.width()).clamp(0.0, 1.0))
                    .unwrap_or(0.5) as f64;
                let focus_us = window_start_us + focus_ratio * x_span_us;
                interaction.time_window_us = Some(zoom_time_window(
                    window_start_us,
                    window_end_us,
                    focus_us,
                    scale,
                ));
            }
        }
    }
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
        cursors.cursor_a_us / 1e6,
        (x_min, x_span),
        egui::Color32::from_rgb(255, 196, 87),
        "A",
        *cursors.active_drag == Some(WaveformCursorTarget::A),
    );
    draw_cursor_line(
        &painter,
        plot_rect,
        cursors.cursor_b_us / 1e6,
        (x_min, x_span),
        egui::Color32::from_rgb(135, 220, 140),
        "B",
        *cursors.active_drag == Some(WaveformCursorTarget::B),
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
    if let Some(trigger) = view.trigger {
        draw_trigger_markers(
            &painter,
            plot_rect,
            trigger,
            (window_start_us, window_end_us),
            (y_min, y_max),
        );
    }

    let font = egui::FontId::monospace(12.0);
    for (trace_order, trace) in traces.iter().copied().enumerate() {
        let Some((trace_waveform, trace_probe)) = waveform_trace(waveforms, trace) else {
            continue;
        };
        let color = scope_trace_color_for_style(trace_order, trace, trace_styles);
        let points = visible_trace_points(trace_waveform, trace_probe, x_min, x_max, &map_point);
        if points.len() >= 2 {
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(if trace_order == 0 { 2.4 } else { 1.8 }, color),
            ));
        }
        if trace_order < 6 {
            painter.text(
                egui::pos2(
                    plot_rect.left() + 8.0,
                    plot_rect.top() + 14.0 + trace_order as f32 * 16.0,
                ),
                egui::Align2::LEFT_CENTER,
                format!(
                    "{}{}",
                    if trace_order == 0 { "* " } else { "  " },
                    trace_probe
                        .expression
                        .as_deref()
                        .unwrap_or(&trace_probe.label)
                ),
                font.clone(),
                color,
            );
        }
    }

    painter.text(
        egui::pos2(plot_rect.left(), rect.bottom() - 22.0),
        egui::Align2::LEFT_CENTER,
        format!(
            "t {:.3e}..{:.3e} s  (full {:.3e}..{:.3e} s)",
            x_min,
            x_max,
            full_start_us / 1e6,
            full_end_us / 1e6
        ),
        font.clone(),
        egui::Color32::LIGHT_GRAY,
    );
    painter.text(
        egui::pos2(plot_rect.left(), rect.top() + 8.0),
        egui::Align2::LEFT_CENTER,
        format!(
            "{} {:.3e}..{:.3e}",
            primary.1.expression.as_deref().unwrap_or(&primary.1.label),
            y_min,
            y_max
        ),
        font,
        egui::Color32::LIGHT_GRAY,
    );
    if response.hovered() {
        let cursor = if cursor_target_at_pointer.is_some() || cursors.active_drag.is_some() {
            egui::CursorIcon::ResizeHorizontal
        } else {
            egui::CursorIcon::Grab
        };
        ui.ctx().set_cursor_icon(cursor);
    }
    interaction
}

pub(super) fn scope_plot_size(available: egui::Vec2) -> egui::Vec2 {
    egui::vec2(available.x.max(560.0), available.y.max(360.0))
}

pub(super) fn valid_waveform_trace(waveforms: &[WaveformView], trace: WaveformTraceRef) -> bool {
    waveform_trace(waveforms, trace).is_some()
}

fn waveform_trace(
    waveforms: &[WaveformView],
    trace: WaveformTraceRef,
) -> Option<(&WaveformView, &WaveformProbe)> {
    let waveform = waveforms.get(trace.waveform_index)?;
    let probe = waveform.probes.get(trace.probe_index)?;
    Some((waveform, probe))
}

pub(super) fn scope_visible_trace_refs(
    waveforms: &[WaveformView],
    selected_waveform: usize,
    selected_probe: usize,
    pinned: &[WaveformTraceRef],
) -> Vec<WaveformTraceRef> {
    let selected = WaveformTraceRef {
        waveform_index: selected_waveform,
        probe_index: selected_probe,
    };
    let mut traces = Vec::new();
    if valid_waveform_trace(waveforms, selected) {
        traces.push(selected);
    }
    for trace in pinned.iter().copied() {
        if valid_waveform_trace(waveforms, trace) && !traces.contains(&trace) {
            traces.push(trace);
        }
    }
    traces
}

pub(super) fn scope_visible_styled_trace_refs(
    traces: &[WaveformTraceRef],
    styles: &[WaveformTraceStyle],
) -> Vec<WaveformTraceRef> {
    traces
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, trace)| {
            if index == 0 || scope_trace_style(styles, trace).visible {
                Some(trace)
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn waveform_trace_bounds_in_window(
    waveforms: &[WaveformView],
    traces: &[WaveformTraceRef],
    start_s: f64,
    end_s: f64,
) -> Option<(f64, f64)> {
    let mut y_range: Option<(f64, f64)> = None;
    for trace in traces.iter().copied() {
        let (waveform, probe) = waveform_trace(waveforms, trace)?;
        let (y_min, y_max) = if start_s.is_finite() && end_s.is_finite() {
            window_min_max(&waveform.time_s, &probe.values, start_s, end_s)?
        } else {
            min_max(&probe.values)?
        };
        y_range = Some(match y_range {
            Some((bottom, top)) => (bottom.min(y_min), top.max(y_max)),
            None => (y_min, y_max),
        });
    }
    y_range
}

fn visible_trace_points(
    waveform: &WaveformView,
    probe: &WaveformProbe,
    start_s: f64,
    end_s: f64,
    map_point: &impl Fn(f64, f64) -> egui::Pos2,
) -> Vec<egui::Pos2> {
    let mut points = Vec::new();
    if let Some(value) = interpolated_value(&waveform.time_s, &probe.values, start_s) {
        points.push(map_point(start_s, value));
    }
    for (time, value) in waveform
        .time_s
        .iter()
        .copied()
        .zip(probe.values.iter().copied())
    {
        if time > start_s && time < end_s {
            points.push(map_point(time, value));
        }
    }
    if let Some(value) = interpolated_value(&waveform.time_s, &probe.values, end_s) {
        points.push(map_point(end_s, value));
    }
    points
}

fn scope_trace_color(index: usize) -> egui::Color32 {
    WaveformTraceColor::all()[index % WaveformTraceColor::all().len()].color()
}

pub(super) fn scope_trace_color_for_style(
    index: usize,
    trace: WaveformTraceRef,
    styles: &[WaveformTraceStyle],
) -> egui::Color32 {
    scope_trace_style(styles, trace)
        .color
        .map_or_else(|| scope_trace_color(index), WaveformTraceColor::color)
}

pub(super) fn scope_trace_style(
    styles: &[WaveformTraceStyle],
    trace: WaveformTraceRef,
) -> WaveformTraceStyle {
    styles
        .iter()
        .copied()
        .find(|style| style.trace == trace)
        .unwrap_or_else(|| WaveformTraceStyle::default_for(trace))
}

pub(super) fn plot_x_to_time_us(
    x: f32,
    plot_rect: egui::Rect,
    window_start_us: f64,
    window_end_us: f64,
) -> f64 {
    let ratio = if plot_rect.width() <= 1.0 {
        0.0
    } else {
        ((x - plot_rect.left()) / plot_rect.width()).clamp(0.0, 1.0) as f64
    };
    window_start_us + ratio * positive_span(window_start_us, window_end_us)
}

pub(super) fn plot_y_to_value(
    y: f32,
    plot_rect: egui::Rect,
    value_min: f64,
    value_max: f64,
) -> f64 {
    let ratio = if plot_rect.height() <= 1.0 {
        0.0
    } else {
        ((plot_rect.bottom() - y) / plot_rect.height()).clamp(0.0, 1.0) as f64
    };
    value_min + ratio * positive_span(value_min, value_max)
}

fn cursor_x(time_us: f64, plot_rect: egui::Rect, window_start_us: f64, window_end_us: f64) -> f32 {
    let ratio = ((time_us - window_start_us) / positive_span(window_start_us, window_end_us))
        .clamp(0.0, 1.0) as f32;
    plot_rect.left() + ratio * plot_rect.width()
}

pub(super) fn nearest_scope_cursor_target(
    pointer: egui::Pos2,
    plot_rect: egui::Rect,
    cursor_a_us: f64,
    cursor_b_us: f64,
    window_start_us: f64,
    window_end_us: f64,
) -> Option<WaveformCursorTarget> {
    const HANDLE_RADIUS: f32 = 10.0;
    const LINE_RADIUS: f32 = 7.0;
    let a_x = cursor_x(cursor_a_us, plot_rect, window_start_us, window_end_us);
    let b_x = cursor_x(cursor_b_us, plot_rect, window_start_us, window_end_us);
    let a_distance = (pointer.x - a_x).abs();
    let b_distance = (pointer.x - b_x).abs();
    let near_top_handle = pointer.y <= plot_rect.top() + HANDLE_RADIUS * 1.8;
    let threshold = if near_top_handle {
        HANDLE_RADIUS * 1.6
    } else {
        LINE_RADIUS
    };
    if a_distance.min(b_distance) > threshold {
        return None;
    }
    if a_distance <= b_distance {
        Some(WaveformCursorTarget::A)
    } else {
        Some(WaveformCursorTarget::B)
    }
}

fn draw_cursor_line(
    painter: &egui::Painter,
    plot_rect: egui::Rect,
    time_s: f64,
    x_range: (f64, f64),
    color: egui::Color32,
    label: &str,
    active: bool,
) {
    let (x_min, x_span) = x_range;
    let ratio = ((time_s - x_min) / x_span).clamp(0.0, 1.0) as f32;
    let x = plot_rect.left() + ratio * plot_rect.width();
    painter.line_segment(
        [
            egui::pos2(x, plot_rect.top()),
            egui::pos2(x, plot_rect.bottom()),
        ],
        egui::Stroke::new(if active { 2.4 } else { 1.5 }, color),
    );
    let handle_rect =
        egui::Rect::from_center_size(egui::pos2(x, plot_rect.top() + 7.0), egui::vec2(18.0, 14.0));
    painter.rect_filled(handle_rect, 3.0, color);
    painter.rect_stroke(
        handle_rect,
        3.0,
        egui::Stroke::new(if active { 2.0 } else { 1.0 }, egui::Color32::from_gray(24)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        handle_rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(11.0),
        egui::Color32::from_gray(20),
    );
}

fn draw_trigger_markers(
    painter: &egui::Painter,
    plot_rect: egui::Rect,
    trigger: WaveformPlotTrigger<'_>,
    time_window_us: (f64, f64),
    value_window: (f64, f64),
) {
    if !trigger.threshold.is_finite() {
        return;
    }
    let (window_start_us, window_end_us) = time_window_us;
    let (value_min, value_max) = value_window;
    let x_span = positive_span(window_start_us, window_end_us);
    let y_span = positive_span(value_min, value_max);
    let marker_color = egui::Color32::from_rgb(104, 214, 255);

    if trigger.threshold >= value_min && trigger.threshold <= value_max {
        let y_ratio = ((trigger.threshold - value_min) / y_span).clamp(0.0, 1.0) as f32;
        let y = plot_rect.bottom() - y_ratio * plot_rect.height();
        painter.line_segment(
            [
                egui::pos2(plot_rect.left(), y),
                egui::pos2(plot_rect.right(), y),
            ],
            egui::Stroke::new(1.2, marker_color.linear_multiply(0.55)),
        );
        painter.text(
            egui::pos2(plot_rect.right() - 4.0, y - 4.0),
            egui::Align2::RIGHT_BOTTOM,
            "T",
            egui::FontId::monospace(11.0),
            marker_color,
        );
    }

    for event_us in trigger.events_us.iter().copied().take(128) {
        if event_us < window_start_us || event_us > window_end_us {
            continue;
        }
        let x_ratio = ((event_us - window_start_us) / x_span).clamp(0.0, 1.0) as f32;
        let x = plot_rect.left() + x_ratio * plot_rect.width();
        painter.line_segment(
            [
                egui::pos2(x, plot_rect.top()),
                egui::pos2(x, plot_rect.bottom()),
            ],
            egui::Stroke::new(1.0, marker_color.linear_multiply(0.65)),
        );
        painter.circle_filled(egui::pos2(x, plot_rect.top() + 5.0), 3.0, marker_color);
    }
}

fn draw_plot_frame(painter: &egui::Painter, rect: egui::Rect) {
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(96));
    painter.line_segment([rect.left_top(), rect.right_top()], stroke);
    painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_bottom(), rect.left_bottom()], stroke);
    painter.line_segment([rect.left_bottom(), rect.left_top()], stroke);
}
