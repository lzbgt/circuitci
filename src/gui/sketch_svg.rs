use anyhow::{Context, Result, bail};
use eframe::egui;
use std::path::Path;

use super::sketch::{
    ProjectSnapshot, SketchGraph, SketchNode, SketchPinAnchor, SketchSelection, compact_label,
    layout_sketch_graph, load_project_snapshot, sketch_graph_bounds, sketch_wire_points,
};

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
    for node in &graph.nodes {
        draw_node(&mut svg, node);
    }
    for anchor in &graph.pin_anchors {
        draw_pin_anchor(&mut svg, anchor);
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

fn draw_node(svg: &mut String, node: &SketchNode) {
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
    svg.push_str(&format!(
        r##"<rect data-kind="{kind}" data-id="{}" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="4" fill="{fill}" stroke="{stroke}" stroke-width="1.2"/>"##,
        xml_escape(&selection_id(&node.selection)),
        node.rect.min.x,
        node.rect.min.y,
        node.rect.width(),
        node.rect.height()
    ));
    svg.push('\n');
    let label = compact_label(&node.label, 28);
    svg.push_str(&format!(
        r##"<text x="{:.1}" y="{:.1}" fill="{text_fill}" font-family="monospace" font-size="13">{}</text>"##,
        node.rect.min.x + 8.0,
        node.rect.min.y + 18.0,
        xml_escape(&label)
    ));
    svg.push('\n');
    if !node.detail.is_empty() && !matches!(node.selection, SketchSelection::Net(_)) {
        let detail = compact_label(&node.detail, 34);
        svg.push_str(&format!(
            r##"<text x="{:.1}" y="{:.1}" fill="#c7ced8" font-family="monospace" font-size="11">{}</text>"##,
            node.rect.min.x + 8.0,
            node.rect.max.y - 10.0,
            xml_escape(&detail)
        ));
        svg.push('\n');
    }
}

fn draw_pin_anchor(svg: &mut String, anchor: &SketchPinAnchor) {
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
    fn sketch_svg_rejects_zero_dimensions() {
        let error = sketch_svg(&snapshot_with_high_pin_block(), 0, 640)
            .unwrap_err()
            .to_string();

        assert!(error.contains("width and height must be positive"));
    }
}
