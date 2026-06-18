use eframe::egui;

use super::sketch::{self, SketchGraph, SketchViewport, sketch_graph_bounds, sketch_wire_points};

#[derive(Debug, Clone, Copy)]
pub(super) struct SketchMinimap {
    pub(super) rect: egui::Rect,
    content_bounds: egui::Rect,
    scale: f32,
}

impl SketchMinimap {
    pub(super) fn for_graph(canvas: egui::Rect, graph: &SketchGraph) -> Option<Self> {
        let content_bounds = sketch_graph_bounds(graph)?.expand(64.0);
        let size = egui::vec2(190.0, 140.0);
        let margin = 12.0;
        let rect = egui::Rect::from_min_size(
            egui::pos2(
                canvas.right() - size.x - margin,
                canvas.bottom() - size.y - margin,
            ),
            size,
        );
        let scale = ((rect.width() - 16.0) / content_bounds.width().max(1.0))
            .min((rect.height() - 16.0) / content_bounds.height().max(1.0))
            .max(0.001);
        Some(Self {
            rect,
            content_bounds,
            scale,
        })
    }

    pub(super) fn draw(
        &self,
        painter: &egui::Painter,
        canvas: egui::Rect,
        graph: &SketchGraph,
        viewport: SketchViewport,
    ) {
        painter.rect_filled(
            self.rect,
            4.0,
            egui::Color32::from_rgba_unmultiplied(18, 23, 28, 226),
        );
        painter.rect_stroke(
            self.rect,
            4.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(93, 112, 132)),
            egui::StrokeKind::Inside,
        );
        for edge in &graph.edges {
            let points: Vec<_> = sketch_wire_points(edge)
                .into_iter()
                .map(|point| self.map_to_minimap(point))
                .collect();
            for segment in points.windows(2) {
                painter.line_segment(
                    [segment[0], segment[1]],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(96, 112, 128)),
                );
            }
        }
        for node in &graph.nodes {
            if matches!(node.selection, sketch::SketchSelection::Overflow(_)) {
                continue;
            }
            let rect = egui::Rect::from_min_max(
                self.map_to_minimap(node.rect.min),
                self.map_to_minimap(node.rect.max),
            );
            painter.rect_filled(rect, 1.5, egui::Color32::from_rgb(73, 120, 154));
        }
        let view = self.visible_view_rect(canvas, viewport);
        painter.rect_stroke(
            view,
            2.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 196, 87)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            self.rect.left_top() + egui::vec2(8.0, 7.0),
            egui::Align2::LEFT_TOP,
            "Overview",
            egui::FontId::monospace(10.5),
            egui::Color32::from_rgb(214, 224, 235),
        );
    }

    pub(super) fn pan_for_focus(
        &self,
        canvas: egui::Rect,
        viewport: SketchViewport,
        minimap_position: egui::Pos2,
    ) -> egui::Vec2 {
        let focus = self.map_from_minimap(minimap_position);
        canvas.center() - canvas.min - (focus - canvas.min) * viewport.zoom.clamp(0.25, 4.0)
    }

    fn visible_view_rect(&self, canvas: egui::Rect, viewport: SketchViewport) -> egui::Rect {
        let zoom = viewport.zoom.clamp(0.25, 4.0);
        let logical_min = canvas.min - viewport.pan / zoom;
        let logical_max = canvas.min + (canvas.size() - viewport.pan) / zoom;
        egui::Rect::from_min_max(
            self.map_to_minimap(logical_min),
            self.map_to_minimap(logical_max),
        )
        .intersect(self.rect.shrink(5.0))
    }

    fn map_to_minimap(&self, point: egui::Pos2) -> egui::Pos2 {
        let origin = self.rect.left_top() + egui::vec2(8.0, 8.0);
        origin + (point - self.content_bounds.min) * self.scale
    }

    fn map_from_minimap(&self, point: egui::Pos2) -> egui::Pos2 {
        let origin = self.rect.left_top() + egui::vec2(8.0, 8.0);
        self.content_bounds.min + (point - origin) / self.scale
    }
}

#[cfg(test)]
mod tests {
    use super::super::sketch_symbols::SketchSymbolKind;
    use super::*;

    #[test]
    fn minimap_pan_centers_clicked_logical_point() {
        let canvas = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let graph = SketchGraph {
            nodes: vec![sketch::SketchNode {
                selection: sketch::SketchSelection::Net("sig".to_string()),
                label: "sig".to_string(),
                detail: "net".to_string(),
                symbol: SketchSymbolKind::Net,
                style: Default::default(),
                rect: egui::Rect::from_min_size(egui::pos2(100.0, 120.0), egui::vec2(80.0, 50.0)),
            }],
            pin_anchors: Vec::new(),
            edges: Vec::new(),
            probe_badges: Vec::new(),
        };
        let minimap = SketchMinimap::for_graph(canvas, &graph).unwrap();
        let target = egui::pos2(140.0, 145.0);
        let pan = minimap.pan_for_focus(
            canvas,
            SketchViewport {
                pan: egui::Vec2::ZERO,
                zoom: 1.0,
            },
            minimap.map_to_minimap(target),
        );

        assert!((pan.x - (canvas.center().x - target.x)).abs() < 0.5);
        assert!((pan.y - (canvas.center().y - target.y)).abs() < 0.5);
    }
}
