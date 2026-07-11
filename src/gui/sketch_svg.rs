use anyhow::{Context, Result, bail};
use eframe::egui;
use std::collections::BTreeSet;
use std::path::Path;

use super::sketch::{
    ProjectSnapshot, SketchGraph, SketchNode, SketchPinAnchor, SketchSelection, compact_label,
    layout_sketch_graph, load_project_snapshot, sketch_graph_bounds, sketch_wire_points,
};
use super::sketch_render::sketch_net_label_y_offsets;
use super::sketch_symbols::SketchSymbolKind;

const EXPORT_CANVAS_MIN_WIDTH: f32 = 720.0;
const EXPORT_CANVAS_MIN_HEIGHT: f32 = 420.0;
const EXPORT_PADDING: f32 = 80.0;
const GRID_STEP: f32 = 32.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SketchSvgExportSummary {
    pub components: usize,
    pub nets: usize,
    pub wires: usize,
    pub pin_anchors: usize,
    pub width: u32,
    pub height: u32,
}

pub fn export_sketch_svg(
    project: &Path,
    output: &Path,
    width: u32,
    height: u32,
) -> Result<SketchSvgExportSummary> {
    let snapshot = load_project_snapshot(project).with_context(|| {
        format!(
            "Failed to load Board IR project for Sketch SVG export {}",
            project.display()
        )
    })?;
    let (svg, summary) = sketch_svg(&snapshot, width, height)?;
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create Sketch SVG output directory {}",
                parent.display()
            )
        })?;
    }
    std::fs::write(output, svg)
        .with_context(|| format!("Failed to write Sketch SVG {}", output.display()))?;
    Ok(summary)
}

fn sketch_svg(
    snapshot: &ProjectSnapshot,
    width: u32,
    height: u32,
) -> Result<(String, SketchSvgExportSummary)> {
    if width == 0 || height == 0 {
        bail!("Sketch SVG width and height must be positive.");
    }
    let canvas = egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(
            (width as f32).max(EXPORT_CANVAS_MIN_WIDTH),
            (height as f32).max(EXPORT_CANVAS_MIN_HEIGHT),
        ),
    );
    let graph = layout_sketch_graph(canvas, snapshot);
    let content_bounds = sketch_graph_bounds(&graph).unwrap_or(canvas);
    let view = content_bounds.expand(EXPORT_PADDING);
    let mut svg = String::new();
    svg.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="{:.1} {:.1} {:.1} {:.1}" role="img" aria-label="CircuitCI Sketch schematic">"##,
        view.min.x,
        view.min.y,
        view.width(),
        view.height()
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="#101114"/>"##,
        view.min.x,
        view.min.y,
        view.width(),
        view.height()
    ));
    svg.push('\n');
    draw_grid(&mut svg, view);
    svg.push_str(&format!(
        r##"<text x="{:.1}" y="{:.1}" fill="#d7dde8" font-family="monospace" font-size="18">CircuitCI Sketch - {}</text>"##,
        view.min.x + 24.0,
        view.min.y + 34.0,
        xml_escape(&snapshot.name)
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r##"<text x="{:.1}" y="{:.1}" fill="#9aa5b4" font-family="monospace" font-size="12">{} components / {} nets / {} run setups</text>"##,
        view.min.x + 24.0,
        view.min.y + 54.0,
        snapshot.components,
        snapshot.nets,
        snapshot.scenarios
    ));
    svg.push('\n');
    draw_edges(&mut svg, &graph);
    let net_label_y_offsets = sketch_net_label_y_offsets(&graph);
    for node in &graph.nodes {
        let label_y_offset = match &node.selection {
            SketchSelection::Net(net_id) => net_label_y_offsets.get(net_id).copied().unwrap_or(0.0),
            _ => 0.0,
        };
        draw_node(&mut svg, node, label_y_offset);
    }
    let hidden_pin_label_components = device_symbol_component_ids(&graph);
    for anchor in &graph.pin_anchors {
        draw_pin_anchor(&mut svg, anchor, &hidden_pin_label_components);
    }
    svg.push_str("</svg>\n");

    let summary = SketchSvgExportSummary {
        components: snapshot.components,
        nets: snapshot.nets,
        wires: graph.edges.len(),
        pin_anchors: graph.pin_anchors.len(),
        width,
        height,
    };
    Ok((svg, summary))
}

fn device_symbol_component_ids(graph: &SketchGraph) -> BTreeSet<&str> {
    graph
        .nodes
        .iter()
        .filter_map(|node| match &node.selection {
            SketchSelection::Component(component_id) if node.symbol.is_kicad_device_symbol() => {
                Some(component_id.as_str())
            }
            _ => None,
        })
        .collect()
}

fn draw_grid(svg: &mut String, view: egui::Rect) {
    let mut x = (view.min.x / GRID_STEP).floor() * GRID_STEP;
    let mut lines = 0usize;
    while x <= view.max.x && lines < 240 {
        svg.push_str(&format!(
            r##"<line x1="{x:.1}" y1="{:.1}" x2="{x:.1}" y2="{:.1}" stroke="#1f252e" stroke-width="1"/>"##,
            view.min.y, view.max.y
        ));
        svg.push('\n');
        x += GRID_STEP;
        lines += 1;
    }
    let mut y = (view.min.y / GRID_STEP).floor() * GRID_STEP;
    lines = 0;
    while y <= view.max.y && lines < 240 {
        svg.push_str(&format!(
            r##"<line x1="{:.1}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="#1f252e" stroke-width="1"/>"##,
            view.min.x, view.max.x
        ));
        svg.push('\n');
        y += GRID_STEP;
        lines += 1;
    }
}

fn draw_edges(svg: &mut String, graph: &SketchGraph) {
    for edge in &graph.edges {
        let points = sketch_wire_points(edge)
            .iter()
            .map(|point| format!("{:.1},{:.1}", point.x, point.y))
            .collect::<Vec<_>>()
            .join(" ");
        svg.push_str(&format!(
            r##"<polyline data-net="{}" data-source="{}" points="{}" fill="none" stroke="#6f7a88" stroke-width="2.0" stroke-linecap="round" stroke-linejoin="round"/>"##,
            xml_escape(&edge.net_id),
            xml_escape(&edge.source),
            points
        ));
        svg.push('\n');
    }
}

fn draw_node(svg: &mut String, node: &SketchNode, label_y_offset: f32) {
    let is_device_symbol = matches!(node.selection, SketchSelection::Component(_))
        && node.symbol.is_kicad_device_symbol();
    let lightweight_node = is_device_symbol || matches!(node.selection, SketchSelection::Net(_));
    let (fill, stroke, text_fill) = match node.selection {
        SketchSelection::Component(_) => ("#243446", "#6c7886", "#ffffff"),
        SketchSelection::Net(_) => ("#2a3e2e", "#6ea56f", "#b6ebbf"),
        SketchSelection::Overflow(_) => ("#242424", "#707070", "#d5d5d5"),
    };
    let kind = match node.selection {
        SketchSelection::Component(_) => "component",
        SketchSelection::Net(_) => "net",
        SketchSelection::Overflow(_) => "overflow",
    };
    if !lightweight_node {
        svg.push_str(&format!(
            r##"<rect data-kind="{kind}" data-id="{}" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="4" fill="{fill}" stroke="{stroke}" stroke-width="1.2"/>"##,
            xml_escape(&selection_id(&node.selection)),
            node.rect.min.x,
            node.rect.min.y,
            node.rect.width(),
            node.rect.height()
        ));
        svg.push('\n');
    } else {
        svg.push_str(&format!(
            r##"<g data-kind="{kind}" data-id="{}">"##,
            xml_escape(&selection_id(&node.selection))
        ));
        svg.push('\n');
    }
    draw_symbol_svg(svg, node);
    draw_node_labels(svg, node, text_fill, is_device_symbol, label_y_offset);
    if lightweight_node {
        svg.push_str("</g>\n");
    }
}

fn draw_node_labels(
    svg: &mut String,
    node: &SketchNode,
    text_fill: &str,
    is_device_symbol: bool,
    label_y_offset: f32,
) {
    if is_device_symbol {
        let label = compact_label(&node.label, 18);
        if matches!(node.symbol, SketchSymbolKind::Source) {
            draw_text(
                svg,
                node.rect.left() - 6.0,
                node.rect.center().y + 4.0,
                "end",
                text_fill,
                12.0,
                &label,
            );
            return;
        }
        draw_text(
            svg,
            node.rect.center().x,
            node.rect.top() - 8.0,
            "middle",
            text_fill,
            12.0,
            &label,
        );
        return;
    }
    if matches!(node.selection, SketchSelection::Net(_)) {
        let label = compact_label(&node.label, 18);
        draw_text(
            svg,
            node.rect.center().x,
            node.rect.top() - 6.0 + label_y_offset,
            "middle",
            text_fill,
            11.0,
            &label,
        );
        return;
    }
    let label = compact_label_to_width(&node.label, node.rect.width() - 16.0, 13.0);
    draw_text(
        svg,
        node.rect.min.x + 8.0,
        node.rect.min.y + 18.0,
        "start",
        text_fill,
        13.0,
        &label,
    );
    if !node.detail.is_empty() {
        let detail = compact_label_to_width(&node.detail, node.rect.width() - 16.0, 11.0);
        draw_text(
            svg,
            node.rect.min.x + 8.0,
            node.rect.max.y - 10.0,
            "start",
            "#c7ced8",
            11.0,
            &detail,
        );
    }
}

fn draw_text(
    svg: &mut String,
    x: f32,
    y: f32,
    anchor: &str,
    fill: &str,
    font_size: f32,
    text: &str,
) {
    svg.push_str(&format!(
        r##"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" fill="{fill}" font-family="monospace" font-size="{font_size:.1}">{}</text>"##,
        xml_escape(text)
    ));
    svg.push('\n');
}

fn compact_label_to_width(label: &str, width: f32, font_size: f32) -> String {
    let max_chars = (width / (font_size * 0.62)).floor().clamp(6.0, 64.0) as usize;
    compact_label(label, max_chars)
}

fn draw_symbol_svg(svg: &mut String, node: &SketchNode) {
    if matches!(node.symbol, SketchSymbolKind::Overflow) {
        return;
    }
    let Some(rect) = symbol_svg_rect(node) else {
        return;
    };
    let stroke = match node.selection {
        SketchSelection::Component(_) => "#d9e6f1",
        SketchSelection::Net(_) => "#b6ebbf",
        SketchSelection::Overflow(_) => "#d5d5d5",
    };
    match node.symbol {
        SketchSymbolKind::Resistor => draw_resistor_symbol_svg(svg, rect, stroke, node),
        SketchSymbolKind::Capacitor => draw_capacitor_symbol_svg(svg, rect, stroke, node),
        SketchSymbolKind::Inductor => draw_inductor_symbol_svg(svg, rect, stroke, node),
        SketchSymbolKind::Diode => draw_diode_symbol_svg(svg, rect, stroke, node),
        SketchSymbolKind::Source => draw_source_symbol_svg(svg, rect, stroke, node),
        SketchSymbolKind::Connector => draw_connector_symbol_svg(svg, rect, stroke, node),
        SketchSymbolKind::Ic | SketchSymbolKind::Block => {
            draw_block_symbol_svg(svg, rect, stroke, node, node.symbol == SketchSymbolKind::Ic)
        }
        SketchSymbolKind::Net => draw_net_symbol_svg(svg, rect, stroke, node),
        SketchSymbolKind::Overflow => {}
    }
}

fn symbol_svg_rect(node: &SketchNode) -> Option<egui::Rect> {
    let has_explicit_symbol = node.kicad_symbol_id.is_some();
    let mut rect = if node.symbol.is_kicad_device_symbol() || has_explicit_symbol {
        node.rect.shrink2(egui::vec2(8.0, 18.0))
    } else {
        node.rect.shrink2(egui::vec2(28.0, 26.0))
    };
    if !node.symbol.is_kicad_device_symbol() && !has_explicit_symbol {
        rect.min.y = rect.min.y.max(node.rect.top() + 26.0);
        rect.max.y = rect.max.y.min(node.rect.bottom() - 20.0);
    }
    (rect.width() >= 48.0 && rect.height() >= 14.0).then_some(rect)
}

fn draw_resistor_symbol_svg(svg: &mut String, rect: egui::Rect, stroke: &str, node: &SketchNode) {
    draw_svg_line(
        svg,
        rect,
        (-1.0, 0.0),
        (-0.36, 0.0),
        stroke,
        node,
        Some("resistor-lead"),
    );
    draw_svg_line(
        svg,
        rect,
        (0.36, 0.0),
        (1.0, 0.0),
        stroke,
        node,
        Some("resistor-lead"),
    );
    draw_svg_polyline(
        svg,
        rect,
        &[
            (-0.36, -0.38),
            (0.36, -0.38),
            (0.36, 0.38),
            (-0.36, 0.38),
            (-0.36, -0.38),
        ],
        stroke,
        node,
        Some("resistor"),
        "none",
    );
}

fn draw_capacitor_symbol_svg(svg: &mut String, rect: egui::Rect, stroke: &str, node: &SketchNode) {
    draw_svg_line(
        svg,
        rect,
        (-1.0, 0.0),
        (-0.12, 0.0),
        stroke,
        node,
        Some("capacitor-lead"),
    );
    draw_svg_line(
        svg,
        rect,
        (0.12, 0.0),
        (1.0, 0.0),
        stroke,
        node,
        Some("capacitor-lead"),
    );
    draw_svg_line(
        svg,
        rect,
        (-0.12, -0.78),
        (-0.12, 0.78),
        stroke,
        node,
        Some("capacitor"),
    );
    draw_svg_line(
        svg,
        rect,
        (0.12, -0.78),
        (0.12, 0.78),
        stroke,
        node,
        Some("capacitor"),
    );
}

fn draw_inductor_symbol_svg(svg: &mut String, rect: egui::Rect, stroke: &str, node: &SketchNode) {
    draw_svg_line(
        svg,
        rect,
        (-1.0, 0.0),
        (-0.58, 0.0),
        stroke,
        node,
        Some("inductor-lead"),
    );
    draw_svg_line(
        svg,
        rect,
        (0.58, 0.0),
        (1.0, 0.0),
        stroke,
        node,
        Some("inductor-lead"),
    );
    for index in 0..4 {
        let center = symbol_svg_point(rect, -0.42 + 0.28 * index as f32, 0.0, node);
        let radius = (rect.height().min(rect.width()) * 0.11).clamp(3.5, 7.0);
        svg.push_str(&format!(
            r##"<circle data-symbol="inductor" cx="{:.1}" cy="{:.1}" r="{radius:.1}" fill="none" stroke="{stroke}" stroke-width="1.7"/>"##,
            center.x, center.y
        ));
        svg.push('\n');
    }
}

fn draw_diode_symbol_svg(svg: &mut String, rect: egui::Rect, stroke: &str, node: &SketchNode) {
    draw_svg_line(
        svg,
        rect,
        (-1.0, 0.0),
        (-0.26, 0.0),
        stroke,
        node,
        Some("diode-lead"),
    );
    draw_svg_line(
        svg,
        rect,
        (0.32, 0.0),
        (1.0, 0.0),
        stroke,
        node,
        Some("diode-lead"),
    );
    draw_svg_polyline(
        svg,
        rect,
        &[(-0.26, -0.72), (-0.26, 0.72), (0.32, 0.0), (-0.26, -0.72)],
        stroke,
        node,
        Some("diode"),
        "#415467",
    );
    draw_svg_line(
        svg,
        rect,
        (0.42, -0.72),
        (0.42, 0.72),
        stroke,
        node,
        Some("diode-cathode"),
    );
}

fn draw_source_symbol_svg(svg: &mut String, rect: egui::Rect, stroke: &str, node: &SketchNode) {
    draw_svg_line(
        svg,
        rect,
        (-1.0, 0.0),
        (-0.34, 0.0),
        stroke,
        node,
        Some("source-lead"),
    );
    draw_svg_line(
        svg,
        rect,
        (0.34, 0.0),
        (1.0, 0.0),
        stroke,
        node,
        Some("source-lead"),
    );
    let center = symbol_svg_point(rect, 0.0, 0.0, node);
    let radius = (rect.height() * 0.38).clamp(8.0, 15.0);
    svg.push_str(&format!(
        r##"<circle data-symbol="source" cx="{:.1}" cy="{:.1}" r="{radius:.1}" fill="none" stroke="{stroke}" stroke-width="1.7"/>"##,
        center.x, center.y
    ));
    svg.push('\n');
    draw_text(
        svg,
        center.x - 3.5,
        center.y - 4.0,
        "middle",
        stroke,
        11.0,
        "+",
    );
    draw_text(
        svg,
        center.x + 4.0,
        center.y + 5.0,
        "middle",
        stroke,
        11.0,
        "-",
    );
}

fn draw_connector_symbol_svg(svg: &mut String, rect: egui::Rect, stroke: &str, node: &SketchNode) {
    let body = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width() * 0.42, rect.height() * 0.62),
    );
    svg.push_str(&format!(
        r##"<rect data-symbol="connector" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" fill="none" stroke="{stroke}" stroke-width="1.7"/>"##,
        body.min.x,
        body.min.y,
        body.width(),
        body.height()
    ));
    svg.push('\n');
    for index in 0..4 {
        let v = -0.75 + 1.5 * (index as f32 + 0.5) / 4.0;
        draw_svg_line(
            svg,
            rect,
            (-1.0, v),
            (-0.22, v),
            stroke,
            node,
            Some("connector-pin"),
        );
        let end = symbol_svg_point(rect, -0.22, v, node);
        svg.push_str(&format!(
            r##"<circle data-symbol="connector-pin" cx="{:.1}" cy="{:.1}" r="2.0" fill="{stroke}"/>"##,
            end.x, end.y
        ));
        svg.push('\n');
    }
}

fn draw_block_symbol_svg(
    svg: &mut String,
    rect: egui::Rect,
    stroke: &str,
    node: &SketchNode,
    with_pins: bool,
) {
    let body = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width() * 0.46, rect.height() * 0.68),
    );
    svg.push_str(&format!(
        r##"<rect data-symbol="block" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" fill="none" stroke="{stroke}" stroke-width="1.7"/>"##,
        body.min.x,
        body.min.y,
        body.width(),
        body.height()
    ));
    svg.push('\n');
    if with_pins {
        for index in 0..3 {
            let v = -0.5 + 0.5 * index as f32;
            draw_svg_line(
                svg,
                rect,
                (-0.58, v),
                (-0.34, v),
                stroke,
                node,
                Some("ic-pin"),
            );
            draw_svg_line(
                svg,
                rect,
                (0.34, v),
                (0.58, v),
                stroke,
                node,
                Some("ic-pin"),
            );
        }
    }
    svg.push_str(&format!(
        r##"<circle data-symbol="pin-1" cx="{:.1}" cy="{:.1}" r="2.0" fill="{stroke}"/>"##,
        body.left() + 6.0,
        body.top() + 6.0
    ));
    svg.push('\n');
}

fn draw_net_symbol_svg(svg: &mut String, rect: egui::Rect, stroke: &str, node: &SketchNode) {
    draw_svg_line(
        svg,
        rect,
        (-0.76, 0.0),
        (0.76, 0.0),
        stroke,
        node,
        Some("net"),
    );
    for x in [-0.76, 0.76] {
        let point = symbol_svg_point(rect, x, 0.0, node);
        svg.push_str(&format!(
            r##"<circle data-symbol="net-terminal" cx="{:.1}" cy="{:.1}" r="3.0" fill="none" stroke="{stroke}" stroke-width="1.7"/>"##,
            point.x, point.y
        ));
        svg.push('\n');
    }
}

fn draw_svg_line(
    svg: &mut String,
    rect: egui::Rect,
    start: (f32, f32),
    end: (f32, f32),
    stroke: &str,
    node: &SketchNode,
    symbol: Option<&str>,
) {
    let start = symbol_svg_point(rect, start.0, start.1, node);
    let end = symbol_svg_point(rect, end.0, end.1, node);
    let symbol_attr = symbol
        .map(|symbol| format!(r#" data-symbol="{symbol}""#))
        .unwrap_or_default();
    svg.push_str(&format!(
        r##"<line{symbol_attr} x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{stroke}" stroke-width="1.7" stroke-linecap="round"/>"##,
        start.x, start.y, end.x, end.y
    ));
    svg.push('\n');
}

fn draw_svg_polyline(
    svg: &mut String,
    rect: egui::Rect,
    points: &[(f32, f32)],
    stroke: &str,
    node: &SketchNode,
    symbol: Option<&str>,
    fill: &str,
) {
    let rendered = points
        .iter()
        .map(|(x, y)| symbol_svg_point(rect, *x, *y, node))
        .map(|point| format!("{:.1},{:.1}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ");
    let symbol_attr = symbol
        .map(|symbol| format!(r#" data-symbol="{symbol}""#))
        .unwrap_or_default();
    svg.push_str(&format!(
        r##"<polyline{symbol_attr} points="{rendered}" fill="{fill}" stroke="{stroke}" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>"##
    ));
    svg.push('\n');
}

fn symbol_svg_point(rect: egui::Rect, x: f32, y: f32, node: &SketchNode) -> egui::Pos2 {
    let x = if node.style.mirrored { -x } else { x };
    let (x, y) = match node.style.rotation_deg {
        90 => (-y, x),
        180 => (-x, -y),
        270 => (y, -x),
        _ => (x, y),
    };
    egui::pos2(
        rect.center().x + x * rect.width() * 0.5,
        rect.center().y + y * rect.height() * 0.5,
    )
}

fn draw_pin_anchor(
    svg: &mut String,
    anchor: &SketchPinAnchor,
    hidden_pin_label_components: &BTreeSet<&str>,
) {
    let color = pin_color(&anchor.kind);
    svg.push_str(&format!(
        r##"<circle data-component="{}" data-pin="{}" data-net="{}" cx="{:.1}" cy="{:.1}" r="4.5" fill="{color}" stroke="#121212" stroke-width="1"/>"##,
        xml_escape(&anchor.component_id),
        xml_escape(&anchor.pin),
        xml_escape(&anchor.net),
        anchor.pos.x,
        anchor.pos.y
    ));
    svg.push('\n');
    if hidden_pin_label_components.contains(anchor.component_id.as_str()) {
        return;
    }
    let (text_anchor, dx) = match anchor.label_align {
        egui::Align2::LEFT_CENTER => ("start", 0.0),
        egui::Align2::RIGHT_CENTER => ("end", 0.0),
        _ => ("middle", 0.0),
    };
    svg.push_str(&format!(
        r##"<text data-pin-label="{}:{}" x="{:.1}" y="{:.1}" text-anchor="{text_anchor}" dominant-baseline="middle" fill="#cfd6df" font-family="monospace" font-size="10.5">{}</text>"##,
        xml_escape(&anchor.component_id),
        xml_escape(&anchor.pin),
        anchor.label_pos.x + dx,
        anchor.label_pos.y,
        xml_escape(&compact_label(&anchor.pin, 10))
    ));
    svg.push('\n');
}

fn selection_id(selection: &SketchSelection) -> String {
    match selection {
        SketchSelection::Component(id)
        | SketchSelection::Net(id)
        | SketchSelection::Overflow(id) => id.clone(),
    }
}

fn pin_color(kind: &str) -> &'static str {
    match kind {
        "power" => "#ea6969",
        "ground" => "#78c384",
        "digital" | "digital_or_analog" => "#73a6e0",
        "analog" => "#a98bee",
        _ => "#aab2bd",
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::sketch::{SketchComponent, SketchNet, SketchPin};

    fn snapshot_with_high_pin_block() -> ProjectSnapshot {
        let pins = (1..=8)
            .map(|index| SketchPin {
                pin: index.to_string(),
                net: format!("net_{index}"),
            })
            .collect::<Vec<_>>();
        let nets = pins
            .iter()
            .map(|pin| SketchNet {
                id: pin.net.clone(),
                kind: "digital".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec![format!("U1.{}", pin.pin)],
                position: None,
            })
            .collect::<Vec<_>>();
        ProjectSnapshot {
            name: "svg_high_pin".to_string(),
            components: 1,
            nets: nets.len(),
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![SketchComponent {
                id: "U1".to_string(),
                model: "generic.schematic.imported_component".to_string(),
                part_number: None,
                spice: None,
                pins,
                position: None,
                style: Default::default(),
                kicad_symbol_id: Some("Driver:DRV8245P".to_string()),
                source_paths: Vec::new(),
            }],
            nets_detail: nets,
            probes: Vec::new(),
            wire_routes: Default::default(),
            net_labels: Vec::new(),
            component_labels: Default::default(),
        }
    }

    fn snapshot_with_passive_device_symbol() -> ProjectSnapshot {
        ProjectSnapshot {
            name: "svg_passive_symbol".to_string(),
            components: 1,
            nets: 2,
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: vec![SketchComponent {
                id: "R1".to_string(),
                model: "generic.analog.resistor".to_string(),
                part_number: None,
                spice: None,
                pins: vec![
                    SketchPin {
                        pin: "A".to_string(),
                        net: "in".to_string(),
                    },
                    SketchPin {
                        pin: "B".to_string(),
                        net: "out".to_string(),
                    },
                ],
                position: None,
                style: Default::default(),
                kicad_symbol_id: None,
                source_paths: Vec::new(),
            }],
            nets_detail: vec![
                SketchNet {
                    id: "in".to_string(),
                    kind: "analog".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: vec!["R1.A".to_string()],
                    position: None,
                },
                SketchNet {
                    id: "out".to_string(),
                    kind: "analog".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: vec!["R1.B".to_string()],
                    position: None,
                },
            ],
            probes: Vec::new(),
            wire_routes: Default::default(),
            net_labels: Vec::new(),
            component_labels: Default::default(),
        }
    }

    #[test]
    fn sketch_svg_exports_nonblank_graph_artifact() {
        let (svg, summary) = sketch_svg(&snapshot_with_high_pin_block(), 960, 640).unwrap();

        assert_eq!(summary.components, 1);
        assert_eq!(summary.nets, 8);
        assert_eq!(summary.pin_anchors, 8);
        assert!(svg.starts_with("<svg "));
        assert!(svg.contains("CircuitCI Sketch - svg_high_pin"));
        assert!(svg.contains(r#"role="img""#));
        assert!(svg.contains(r#"data-kind="component" data-id="U1""#));
        assert!(svg.contains(r#"data-pin-label="U1:1""#));
        assert!(svg.contains(r#"viewBox=""#));
        assert!(svg.contains(r#"<rect "#));
        assert!(svg.contains(r#"<circle "#));
    }

    #[test]
    fn sketch_svg_exports_passive_symbols_without_pin_label_clutter() {
        let (svg, summary) = sketch_svg(&snapshot_with_passive_device_symbol(), 960, 640).unwrap();

        assert_eq!(summary.components, 1);
        assert_eq!(summary.pin_anchors, 2);
        assert!(svg.contains(r#"data-symbol="resistor""#));
        assert!(svg.contains(r#"data-component="R1" data-pin="A""#));
        assert!(svg.contains(r#"data-component="R1" data-pin="B""#));
        assert!(!svg.contains(r#"data-pin-label="R1:A""#));
        assert!(!svg.contains(r#"data-pin-label="R1:B""#));
        assert!(!svg.contains("generic.analog.resistor"));
    }

    #[test]
    fn sketch_svg_rejects_zero_dimensions() {
        let error = sketch_svg(&snapshot_with_high_pin_block(), 0, 640)
            .unwrap_err()
            .to_string();

        assert!(error.contains("width and height must be positive"));
    }
}
