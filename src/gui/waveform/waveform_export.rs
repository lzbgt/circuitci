use super::waveform_plot::{
    WaveformPlotTrigger, WaveformPlotView, WaveformSnapshotMarker, WaveformTraceLane,
    clamp_value_window, expanded_value_bounds, scope_trace_color_for_style, scope_trace_lanes,
    waveform_trace_bounds_in_window,
};
use super::{
    WaveformProbe, WaveformTraceRef, WaveformTraceStyle, WaveformView, format_value,
    interpolated_value, positive_span,
};
use eframe::egui;

const SVG_LEFT: f64 = 72.0;
const SVG_TOP: f64 = 48.0;
const SVG_RIGHT: f64 = 24.0;
const SVG_BOTTOM: f64 = 54.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui) enum ScopePlotSvgSizePreset {
    Compact,
    Report,
    Wide,
}

impl ScopePlotSvgSizePreset {
    pub(super) const ALL: [Self; 3] = [Self::Report, Self::Compact, Self::Wide];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact 720x405",
            Self::Report => "Report 960x540",
            Self::Wide => "Wide 1280x720",
        }
    }

    fn dimensions(self) -> (f64, f64) {
        match self {
            Self::Compact => (720.0, 405.0),
            Self::Report => (960.0, 540.0),
            Self::Wide => (1280.0, 720.0),
        }
    }
}

impl Default for ScopePlotSvgSizePreset {
    fn default() -> Self {
        Self::Report
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui) struct ScopePlotSvgOptions {
    pub(super) size_preset: ScopePlotSvgSizePreset,
    pub(super) include_cursors: bool,
    pub(super) include_trigger: bool,
    pub(super) include_snapshots: bool,
}

impl Default for ScopePlotSvgOptions {
    fn default() -> Self {
        Self {
            size_preset: ScopePlotSvgSizePreset::Report,
            include_cursors: true,
            include_trigger: true,
            include_snapshots: true,
        }
    }
}

pub(super) fn scope_plot_svg(
    waveforms: &[WaveformView],
    traces: &[WaveformTraceRef],
    cursor_a_us: f64,
    cursor_b_us: f64,
    view: WaveformPlotView<'_>,
    trace_styles: &[WaveformTraceStyle],
    options: ScopePlotSvgOptions,
) -> Option<String> {
    let primary = traces
        .first()
        .and_then(|trace| waveform_trace(waveforms, *trace))?;
    let (full_start_s, full_end_s) = super::waveform_time_range_us(primary.0)?;
    let (window_start_us, window_end_us) =
        view.visible_window_us.unwrap_or((full_start_s, full_end_s));
    let x_min = window_start_us / 1e6;
    let x_max = window_end_us / 1e6;
    let lanes = scope_trace_lanes(waveforms, traces, view.lane_mode);
    let (svg_width, svg_height) = options.size_preset.dimensions();
    let plot = SvgRect {
        x: SVG_LEFT,
        y: SVG_TOP,
        width: svg_width - SVG_LEFT - SVG_RIGHT,
        height: svg_height - SVG_TOP - SVG_BOTTOM,
    };
    let rendered = rendered_svg_lanes(
        waveforms,
        &lanes,
        plot,
        x_min,
        x_max,
        view.visible_value_window,
    )?;
    let mut svg = String::new();
    svg.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{svg_width:.0}" height="{svg_height:.0}" viewBox="0 0 {svg_width:.0} {svg_height:.0}" role="img" aria-label="CircuitCI scope plot">"##
    ));
    svg.push('\n');
    svg.push_str(r##"<rect width="100%" height="100%" fill="#101114"/>"##);
    svg.push('\n');
    svg.push_str(&format!(
        r##"<text x="24" y="26" fill="#d7dde8" font-family="monospace" font-size="15">CircuitCI Scope Plot - {}</text>"##,
        xml_escape(primary.0.path.as_str())
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r##"<text x="{:.1}" y="{:.1}" fill="#aeb6c2" font-family="monospace" font-size="12">t {}..{} s</text>"##,
        plot.x,
        svg_height - 22.0,
        format_value(x_min),
        format_value(x_max)
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="#121821" stroke="#687385" stroke-width="1"/>"##,
        plot.x, plot.y, plot.width, plot.height
    ));
    svg.push('\n');

    for lane in &rendered {
        draw_svg_lane_grid(&mut svg, lane.rect);
        svg.push_str(&format!(
            r##"<text x="{:.1}" y="{:.1}" fill="#c0c7d2" font-family="monospace" font-size="12">{} {}..{}</text>"##,
            lane.rect.x,
            lane.rect.y + 15.0,
            xml_escape(lane.unit),
            format_value(lane.y_min),
            format_value(lane.y_max)
        ));
        svg.push('\n');
        if options.include_trigger
            && let Some(trigger) = view.trigger
            && lane.traces.contains(&traces[0])
        {
            draw_svg_trigger(&mut svg, lane, trigger, (window_start_us, window_end_us));
        }
        for (lane_order, trace) in lane.traces.iter().copied().enumerate() {
            let Some((waveform, probe)) = waveform_trace(waveforms, trace) else {
                continue;
            };
            let trace_order = traces
                .iter()
                .position(|candidate| *candidate == trace)
                .unwrap_or(lane_order);
            let color = color_hex(scope_trace_color_for_style(
                trace_order,
                trace,
                trace_styles,
            ));
            let points = svg_trace_points(waveform, probe, x_min, x_max, lane);
            if points.len() >= 2 {
                svg.push_str(&format!(
                    r##"<polyline points="{}" fill="none" stroke="{}" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round"/>"##,
                    svg_points(&points),
                    color,
                    if trace_order == 0 { 2.8 } else { 2.0 }
                ));
                svg.push('\n');
            }
            if trace_order < 8 {
                let label = probe.expression.as_deref().unwrap_or(&probe.label);
                svg.push_str(&format!(
                    r##"<text x="{:.1}" y="{:.1}" fill="{}" font-family="monospace" font-size="12">{}{}</text>"##,
                    lane.rect.x + 10.0,
                    lane.rect.y + 34.0 + lane_order as f64 * 16.0,
                    color,
                    if trace_order == 0 { "* " } else { "  " },
                    xml_escape(label)
                ));
                svg.push('\n');
            }
        }
        if options.include_snapshots {
            draw_svg_snapshot_markers(
                &mut svg,
                lane,
                view.snapshot_markers,
                (window_start_us, window_end_us),
            );
        }
    }
    if options.include_cursors {
        draw_svg_cursor(
            &mut svg,
            plot,
            cursor_a_us,
            (window_start_us, window_end_us),
            "A",
            "#ffc457",
        );
        draw_svg_cursor(
            &mut svg,
            plot,
            cursor_b_us,
            (window_start_us, window_end_us),
            "B",
            "#87dc8c",
        );
    }
    svg.push_str("</svg>\n");
    Some(svg)
}

#[derive(Clone, Copy)]
struct SvgRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl SvgRect {
    fn right(self) -> f64 {
        self.x + self.width
    }

    fn bottom(self) -> f64 {
        self.y + self.height
    }
}

struct SvgLane {
    unit: &'static str,
    traces: Vec<WaveformTraceRef>,
    rect: SvgRect,
    y_min: f64,
    y_max: f64,
}

fn rendered_svg_lanes(
    waveforms: &[WaveformView],
    lanes: &[WaveformTraceLane],
    plot: SvgRect,
    x_min: f64,
    x_max: f64,
    visible_value_window: Option<(f64, f64)>,
) -> Option<Vec<SvgLane>> {
    let lane_count = lanes.len().max(1);
    let lane_gap = if lane_count > 1 { 10.0 } else { 0.0 };
    let lane_height = ((plot.height - lane_gap * (lane_count.saturating_sub(1) as f64))
        / lane_count as f64)
        .max(1.0);
    let mut rendered = Vec::new();
    for (index, lane) in lanes.iter().enumerate() {
        let Some((data_y_min, data_y_max)) =
            waveform_trace_bounds_in_window(waveforms, &lane.traces, x_min, x_max)
        else {
            continue;
        };
        let Some((data_y_min, data_y_max)) = expanded_value_bounds(data_y_min, data_y_max) else {
            continue;
        };
        let (y_min, y_max) = if lanes.len() == 1 {
            visible_value_window
                .and_then(|(value_min, value_max)| {
                    clamp_value_window(data_y_min, data_y_max, value_min, value_max)
                })
                .unwrap_or((data_y_min, data_y_max))
        } else {
            (data_y_min, data_y_max)
        };
        let top = plot.y + index as f64 * (lane_height + lane_gap);
        let bottom = if index + 1 == lane_count {
            plot.bottom()
        } else {
            (top + lane_height).min(plot.bottom())
        };
        rendered.push(SvgLane {
            unit: lane.unit,
            traces: lane.traces.clone(),
            rect: SvgRect {
                x: plot.x,
                y: top,
                width: plot.width,
                height: bottom - top,
            },
            y_min,
            y_max,
        });
    }
    (!rendered.is_empty()).then_some(rendered)
}

fn waveform_trace(
    waveforms: &[WaveformView],
    trace: WaveformTraceRef,
) -> Option<(&WaveformView, &WaveformProbe)> {
    let waveform = waveforms.get(trace.waveform_index)?;
    let probe = waveform.probes.get(trace.probe_index)?;
    Some((waveform, probe))
}

fn svg_trace_points(
    waveform: &WaveformView,
    probe: &WaveformProbe,
    start_s: f64,
    end_s: f64,
    lane: &SvgLane,
) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    if let Some(value) = interpolated_value(&waveform.time_s, &probe.values, start_s) {
        points.push(svg_point(lane, start_s, value, start_s, end_s));
    }
    for (time, value) in waveform
        .time_s
        .iter()
        .copied()
        .zip(probe.values.iter().copied())
    {
        if time > start_s && time < end_s {
            points.push(svg_point(lane, time, value, start_s, end_s));
        }
    }
    if let Some(value) = interpolated_value(&waveform.time_s, &probe.values, end_s) {
        points.push(svg_point(lane, end_s, value, start_s, end_s));
    }
    points
}

fn svg_point(lane: &SvgLane, time_s: f64, value: f64, start_s: f64, end_s: f64) -> (f64, f64) {
    let x_ratio = ((time_s - start_s) / positive_span(start_s, end_s)).clamp(0.0, 1.0);
    let y_ratio = ((value - lane.y_min) / positive_span(lane.y_min, lane.y_max)).clamp(0.0, 1.0);
    (
        lane.rect.x + x_ratio * lane.rect.width,
        lane.rect.bottom() - y_ratio * lane.rect.height,
    )
}

fn draw_svg_lane_grid(svg: &mut String, rect: SvgRect) {
    for tick in 0..=4 {
        let ratio = tick as f64 / 4.0;
        let x = rect.x + ratio * rect.width;
        let y = rect.y + ratio * rect.height;
        svg.push_str(&format!(
            r##"<line x1="{x:.1}" y1="{:.1}" x2="{x:.1}" y2="{:.1}" stroke="#28313d" stroke-width="1"/>"##,
            rect.y,
            rect.bottom()
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r##"<line x1="{:.1}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="#28313d" stroke-width="1"/>"##,
            rect.x,
            rect.right()
        ));
        svg.push('\n');
    }
}

fn draw_svg_cursor(
    svg: &mut String,
    plot: SvgRect,
    time_us: f64,
    window_us: (f64, f64),
    label: &str,
    color: &str,
) {
    let x = cursor_x(time_us, plot, window_us);
    svg.push_str(&format!(
        r##"<line x1="{x:.1}" y1="{:.1}" x2="{x:.1}" y2="{:.1}" stroke="{color}" stroke-width="1.6"/>"##,
        plot.y,
        plot.bottom()
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r##"<rect x="{:.1}" y="{:.1}" width="22" height="16" rx="3" fill="{color}" stroke="#20242c"/><text x="{:.1}" y="{:.1}" text-anchor="middle" fill="#101114" font-family="monospace" font-size="11">{}</text>"##,
        x - 11.0,
        plot.y + 3.0,
        x,
        plot.y + 15.0,
        xml_escape(label)
    ));
    svg.push('\n');
}

fn draw_svg_trigger(
    svg: &mut String,
    lane: &SvgLane,
    trigger: WaveformPlotTrigger<'_>,
    window_us: (f64, f64),
) {
    let color = "#68d6ff";
    if trigger.threshold.is_finite()
        && trigger.threshold >= lane.y_min
        && trigger.threshold <= lane.y_max
    {
        let y_ratio = ((trigger.threshold - lane.y_min) / positive_span(lane.y_min, lane.y_max))
            .clamp(0.0, 1.0);
        let y = lane.rect.bottom() - y_ratio * lane.rect.height;
        svg.push_str(&format!(
            r##"<line x1="{:.1}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="{color}" stroke-width="1.2" opacity="0.65"/><text x="{:.1}" y="{:.1}" text-anchor="end" fill="{color}" font-family="monospace" font-size="11">T</text>"##,
            lane.rect.x,
            lane.rect.right(),
            lane.rect.right() - 5.0,
            y - 5.0
        ));
        svg.push('\n');
    }
    for event_us in trigger.events_us.iter().copied().take(128) {
        if event_us < window_us.0 || event_us > window_us.1 {
            continue;
        }
        let x = cursor_x(event_us, lane.rect, window_us);
        svg.push_str(&format!(
            r##"<line x1="{x:.1}" y1="{:.1}" x2="{x:.1}" y2="{:.1}" stroke="{color}" stroke-width="1" opacity="0.65"/><circle cx="{x:.1}" cy="{:.1}" r="3" fill="{color}"/>"##,
            lane.rect.y,
            lane.rect.bottom(),
            lane.rect.y + 5.0
        ));
        svg.push('\n');
    }
}

fn draw_svg_snapshot_markers(
    svg: &mut String,
    lane: &SvgLane,
    markers: &[WaveformSnapshotMarker],
    window_us: (f64, f64),
) {
    let mut drawn = 0usize;
    for marker in markers {
        if !lane.traces.contains(&marker.trace) {
            continue;
        }
        for point in snapshot_marker_points(marker) {
            if drawn >= 64 || point.0 < window_us.0 || point.0 > window_us.1 {
                continue;
            }
            let color = snapshot_marker_color(marker);
            let x = cursor_x(point.0, lane.rect, window_us);
            let dot_y = point
                .1
                .map(|value| {
                    let ratio = ((value - lane.y_min) / positive_span(lane.y_min, lane.y_max))
                        .clamp(0.0, 1.0);
                    lane.rect.bottom() - ratio * lane.rect.height
                })
                .unwrap_or(lane.rect.y + 12.0);
            let label = snapshot_marker_label(marker, point.2);
            let chip_width = (label.chars().count() as f64 * 7.0 + 12.0).clamp(42.0, 170.0);
            let chip_x = (x + 8.0)
                .min(lane.rect.right() - chip_width - 4.0)
                .max(lane.rect.x + 4.0);
            let chip_y = (dot_y - 22.0).clamp(lane.rect.y + 18.0, lane.rect.bottom() - 18.0);
            svg.push_str(&format!(
                r##"<line x1="{x:.1}" y1="{:.1}" x2="{x:.1}" y2="{:.1}" stroke="{color}" stroke-width="1" opacity="0.55"/><circle cx="{x:.1}" cy="{dot_y:.1}" r="3.5" fill="{color}"/><rect x="{chip_x:.1}" y="{:.1}" width="{chip_width:.1}" height="17" rx="4" fill="#121821" stroke="{color}"/><text x="{:.1}" y="{:.1}" fill="{color}" font-family="monospace" font-size="10.5">{}</text>"##,
                lane.rect.y,
                lane.rect.bottom(),
                chip_y - 8.5,
                chip_x + 6.0,
                chip_y + 3.5,
                xml_escape(&label)
            ));
            svg.push('\n');
            drawn += 1;
        }
    }
}

fn snapshot_marker_points(
    marker: &WaveformSnapshotMarker,
) -> Vec<(f64, Option<f64>, &'static str)> {
    let mut points = Vec::new();
    if let Some(time_us) = marker.time_a_us {
        points.push((
            time_us,
            marker.value_a,
            if marker.event_edge.is_some() { "" } else { "A" },
        ));
    }
    if let Some(time_us) = marker.time_b_us {
        points.push((time_us, marker.value_b, "B"));
    }
    points
}

fn snapshot_marker_label(marker: &WaveformSnapshotMarker, suffix: &str) -> String {
    match marker.event_edge.as_deref() {
        Some(edge) => format!("{} {edge}", marker.label),
        None if suffix.is_empty() => marker.label.clone(),
        None => format!("{} {suffix}", marker.label),
    }
}

fn snapshot_marker_color(marker: &WaveformSnapshotMarker) -> &'static str {
    if marker.event_edge.is_some() || marker.source.contains("trigger") {
        "#68d6ff"
    } else if marker.source.contains("pinned") {
        "#be91ff"
    } else {
        "#ffc457"
    }
}

fn cursor_x(time_us: f64, rect: SvgRect, window_us: (f64, f64)) -> f64 {
    let ratio = ((time_us - window_us.0) / positive_span(window_us.0, window_us.1)).clamp(0.0, 1.0);
    rect.x + ratio * rect.width
}

fn svg_points(points: &[(f64, f64)]) -> String {
    points
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn color_hex(color: egui::Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
