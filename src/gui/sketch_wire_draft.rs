use eframe::egui;

use super::sketch::{
    SketchViewport, persisted_wire_route_point_from_screen_with_snap,
    screen_wire_route_point_from_persisted,
};

#[derive(Clone, Debug, Default)]
pub(super) struct SketchWireDraft {
    points: Vec<(f64, f64)>,
}

impl SketchWireDraft {
    pub(super) fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.points.len()
    }

    pub(super) fn points(&self) -> &[(f64, f64)] {
        &self.points
    }

    pub(super) fn screen_points(
        &self,
        canvas: egui::Rect,
        viewport: SketchViewport,
    ) -> Vec<egui::Pos2> {
        self.points
            .iter()
            .map(|point| screen_wire_route_point_from_persisted(canvas, *point, viewport))
            .collect()
    }

    pub(super) fn push_screen_point(
        &mut self,
        canvas: egui::Rect,
        viewport: SketchViewport,
        point: egui::Pos2,
        snap_enabled: bool,
        grid_step: f32,
    ) {
        let point = persisted_wire_route_point_from_screen_with_snap(
            canvas,
            point,
            viewport,
            snap_enabled,
            grid_step,
        );
        if self
            .points
            .last()
            .is_none_or(|last| point_distance_sq(*last, point) > 0.25)
        {
            self.points.push(point);
        }
    }

    pub(super) fn pop_point(&mut self) -> bool {
        self.points.pop().is_some()
    }

    pub(super) fn clear(&mut self) {
        self.points.clear();
    }
}

fn point_distance_sq(left: (f64, f64), right: (f64, f64)) -> f64 {
    let dx = left.0 - right.0;
    let dy = left.1 - right.1;
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_wire_bends_store_schematic_coordinates() {
        let canvas = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(400.0, 300.0));
        let viewport = SketchViewport {
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
        };
        let mut draft = SketchWireDraft::default();

        draft.push_screen_point(canvas, viewport, egui::pos2(110.0, 170.0), false, 20.0);

        assert_eq!(draft.points(), &[(100.0, 150.0)]);
        assert_eq!(
            draft.screen_points(canvas, viewport),
            vec![egui::pos2(110.0, 170.0)]
        );
    }
}
