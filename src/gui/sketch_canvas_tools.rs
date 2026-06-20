use eframe::egui;

use super::sketch::{
    self, SketchNodeStyle, SketchPinSide, SketchSelection, edit_schematic_component_styles,
    edit_schematic_wire_route, persisted_node_position_from_screen_with_snap,
    persisted_wire_route_point_from_screen_with_snap, remove_schematic_wire_route,
    snap_screen_point_to_grid,
};
use super::sketch_canvas_interaction::{
    SketchSelectionBoxMode, WireDragTarget, next_pin_side, normalize_canvas_rotation,
    wire_route_insert_index, zoom_viewport_around,
};
use super::sketch_canvas_render::{placement_ghost_rect, placement_ghost_size};
use super::sketch_inspector::{
    default_current_probe_name_for_component, default_power_probe_name_for_component,
    default_probe_name_for_net,
};
use super::sketch_probes::{SketchProbeBadge, edit_schematic_probe_element_position};
use super::{CircuitCiApp, analog, sketch_alignment};

impl CircuitCiApp {
    pub(super) fn start_visual_wire_from_anchor(&mut self, anchor: &sketch::SketchPinAnchor) {
        self.set_single_sketch_selection(Some(SketchSelection::Component(
            anchor.component_id.clone(),
        )));
        self.pin_edit_id = anchor.pin.clone();
        self.pin_edit_net = anchor.net.clone();
        self.wire_pin_id = anchor.pin.clone();
        self.wire_from_component = Some(anchor.component_id.clone());
        self.sketch_wire_draft.clear();
        self.status = format!(
            "Wire mode: click blank space for bends, then click or release on a pin, net, or wire to connect {}.{}.",
            anchor.component_id, anchor.pin
        );
    }

    pub(super) fn apply_wire_drag_target(&mut self, target: WireDragTarget) {
        let Some(source_component_id) = self.wire_from_component.clone() else {
            return;
        };
        let route_points = self.pending_wire_route_points();
        match target {
            WireDragTarget::Pin {
                component_id, pin, ..
            } => {
                if source_component_id == component_id && self.wire_pin_id.trim() == pin {
                    return;
                }
                self.apply_visual_pin_wire_with_route(
                    source_component_id,
                    component_id,
                    pin,
                    route_points,
                );
            }
            WireDragTarget::NetNode { net_id, .. }
            | WireDragTarget::NetLabel { net_id, .. }
            | WireDragTarget::Wire { net_id, .. } => {
                self.apply_visual_wire_with_route(source_component_id, net_id, route_points);
            }
        }
    }

    pub(super) fn add_pending_wire_route_point(
        &mut self,
        canvas: egui::Rect,
        viewport: sketch::SketchViewport,
        position: egui::Pos2,
    ) {
        let before = self.sketch_wire_draft.len();
        self.sketch_wire_draft.push_screen_point(
            canvas,
            viewport,
            position,
            self.sketch_snap_enabled,
            self.sketch_grid_step,
        );
        if self.sketch_wire_draft.len() > before {
            self.status = format!(
                "Wire route bend {} added. Click a pin, net, or wire to finish.",
                self.sketch_wire_draft.len()
            );
        }
    }

    pub(super) fn remove_last_pending_wire_route_point(&mut self) {
        if self.sketch_wire_draft.pop_point() {
            self.status = if self.sketch_wire_draft.is_empty() {
                "Wire route bend removed.".to_string()
            } else {
                format!(
                    "Wire route bend removed; {} remain.",
                    self.sketch_wire_draft.len()
                )
            };
        }
    }

    pub(super) fn pending_wire_route_points(&self) -> Vec<(f64, f64)> {
        self.sketch_wire_draft.points().to_vec()
    }

    pub(super) fn apply_schematic_wire_route(
        &mut self,
        canvas: egui::Rect,
        viewport: sketch::SketchViewport,
        drag: super::SketchWireRouteDrag,
    ) {
        let points: Vec<_> = drag
            .points
            .iter()
            .map(|point| {
                persisted_wire_route_point_from_screen_with_snap(
                    canvas,
                    *point,
                    viewport,
                    self.sketch_snap_enabled,
                    self.sketch_grid_step,
                )
            })
            .collect();
        match edit_schematic_wire_route(&self.project_yaml, &drag.source, &drag.net_id, &points) {
            Ok(updated) => {
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Wire route {} -> {} edited.", drag.source, drag.net_id),
                );
                self.set_single_sketch_selection(Some(SketchSelection::Net(drag.net_id)));
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_remove_schematic_wire_route(&mut self, edge: &sketch::SketchEdge) {
        match remove_schematic_wire_route(&self.project_yaml, &edge.source, &edge.net_id) {
            Ok(updated) => {
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Wire route {} -> {} cleared.", edge.source, edge.net_id),
                );
                self.set_single_sketch_selection(Some(SketchSelection::Net(edge.net_id.clone())));
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_move_schematic_probe_element_to(
        &mut self,
        canvas: egui::Rect,
        viewport: sketch::SketchViewport,
        badge: &SketchProbeBadge,
        current_center: egui::Pos2,
    ) {
        let Some(element_id) = badge.probe.element_id.as_deref() else {
            self.status = "Only placed schematic probe elements can be moved.".to_string();
            return;
        };
        let (x, y) = persisted_node_position_from_screen_with_snap(
            canvas,
            current_center,
            badge.rect,
            viewport,
            self.sketch_snap_enabled,
            self.sketch_grid_step,
        );
        match edit_schematic_probe_element_position(&self.project_yaml, element_id, x, y) {
            Ok(updated) => {
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Probe element {element_id} moved."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_insert_schematic_wire_route_point(
        &mut self,
        canvas: egui::Rect,
        viewport: sketch::SketchViewport,
        edge: &sketch::SketchEdge,
        position: egui::Pos2,
    ) {
        let point = snap_screen_point_to_grid(
            canvas,
            position,
            viewport,
            self.sketch_snap_enabled,
            self.sketch_grid_step,
        );
        let mut points = edge.route.clone();
        points.insert(wire_route_insert_index(edge, point), point);
        self.apply_schematic_wire_route_points(canvas, viewport, edge, points);
    }

    pub(super) fn apply_delete_schematic_wire_route_point(
        &mut self,
        canvas: egui::Rect,
        viewport: sketch::SketchViewport,
        edge: &sketch::SketchEdge,
        point_index: usize,
    ) {
        if point_index >= edge.route.len() {
            return;
        }
        let mut points = edge.route.clone();
        points.remove(point_index);
        if points.is_empty() {
            self.apply_remove_schematic_wire_route(edge);
        } else {
            self.apply_schematic_wire_route_points(canvas, viewport, edge, points);
        }
    }

    fn apply_schematic_wire_route_points(
        &mut self,
        canvas: egui::Rect,
        viewport: sketch::SketchViewport,
        edge: &sketch::SketchEdge,
        points: Vec<egui::Pos2>,
    ) {
        self.apply_schematic_wire_route(
            canvas,
            viewport,
            super::SketchWireRouteDrag {
                net_id: edge.net_id.clone(),
                source: edge.source.clone(),
                points,
                point_index: 0,
            },
        );
    }

    pub(super) fn canvas_placement_label(&self) -> String {
        let mut label = if self.sketch_palette_place_armed {
            self.sketch_palette_kind.label().to_string()
        } else if self.sketch_net_label_place_armed {
            let net_id = self.sketch_net_label_net_id.trim();
            if net_id.is_empty() {
                self.sketch_net_label_kind.label().to_string()
            } else {
                format!("{} {}", self.sketch_net_label_kind.label(), net_id)
            }
        } else if self.selected_library_model.trim().is_empty() {
            "Library component".to_string()
        } else {
            self.selected_library_model.clone()
        };
        let rotation = self.canvas_placement_rotation_deg();
        if rotation != 0 && (self.sketch_palette_place_armed || self.sketch_library_place_armed) {
            label = format!("{label} {rotation} deg");
        }
        if self.component_placement_armed() && self.sketch_placement_mirrored {
            label = format!("{label} flipped");
        }
        label
    }

    fn canvas_placement_rotation_deg(&self) -> i32 {
        if self.sketch_palette_place_armed || self.sketch_library_place_armed {
            normalize_canvas_rotation(self.sketch_placement_rotation_deg)
        } else {
            0
        }
    }

    pub(super) fn canvas_placement_style(&self) -> SketchNodeStyle {
        if self.component_placement_armed() {
            self.placement_node_style()
        } else {
            SketchNodeStyle::default()
        }
    }

    pub(super) fn component_placement_armed(&self) -> bool {
        self.sketch_palette_place_armed || self.sketch_library_place_armed
    }

    pub(super) fn aligned_component_placement_rect(
        &self,
        graph: &sketch::SketchGraph,
        canvas: egui::Rect,
        pointer: egui::Pos2,
        viewport: sketch::SketchViewport,
    ) -> (egui::Rect, sketch_alignment::SketchAlignmentGuides, bool) {
        let label = self.canvas_placement_label();
        let style = self.canvas_placement_style();
        let base = placement_ghost_rect(
            canvas,
            pointer,
            viewport,
            self.sketch_snap_enabled,
            self.sketch_grid_step,
            placement_ghost_size(&label, style),
        );
        let excluded = std::collections::BTreeSet::new();
        let guides = sketch_alignment::guides_for_rect(graph, base, &excluded);
        let snapped =
            sketch_alignment::snap_rect_to_guides(base, guides, self.sketch_guide_snap_enabled);
        (snapped, guides, snapped != base)
    }

    pub(super) fn rotate_canvas_placement(&mut self, delta_deg: i32) {
        self.sketch_placement_rotation_deg =
            normalize_canvas_rotation(self.sketch_placement_rotation_deg + delta_deg);
        self.status = format!(
            "Canvas placement rotation set to {} deg.",
            self.sketch_placement_rotation_deg
        );
    }

    pub(super) fn flip_canvas_placement(&mut self) {
        self.sketch_placement_mirrored = !self.sketch_placement_mirrored;
        let state = if self.sketch_placement_mirrored {
            "flipped"
        } else {
            "unflipped"
        };
        self.status = format!("Canvas placement {state}.");
    }

    pub(super) fn cycle_canvas_placement_pin_side(&mut self) {
        self.sketch_placement_pin_side = next_pin_side(self.sketch_placement_pin_side);
        self.status = format!(
            "Canvas placement pin side set to {}.",
            self.sketch_placement_pin_side.as_str()
        );
    }

    pub(super) fn placement_node_style(&self) -> SketchNodeStyle {
        SketchNodeStyle {
            rotation_deg: normalize_canvas_rotation(self.sketch_placement_rotation_deg),
            mirrored: self.sketch_placement_mirrored,
            pin_side: self.sketch_placement_pin_side,
        }
    }

    pub(super) fn sketch_placement_orientation_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Orientation");
            ui.label(format!(
                "{} deg",
                normalize_canvas_rotation(self.sketch_placement_rotation_deg)
            ));
            if ui.button("Rotate").on_hover_text("Shortcut: R").clicked() {
                self.rotate_canvas_placement(90);
            }
            let flip_label = if self.sketch_placement_mirrored {
                "Unflip"
            } else {
                "Flip"
            };
            if ui.button(flip_label).on_hover_text("Shortcut: F").clicked() {
                self.flip_canvas_placement();
            }
        });
        let mut pin_side = self.sketch_placement_pin_side;
        egui::ComboBox::from_label("Placement pins")
            .selected_text(pin_side.as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut pin_side, SketchPinSide::Auto, "auto");
                ui.selectable_value(&mut pin_side, SketchPinSide::Left, "left");
                ui.selectable_value(&mut pin_side, SketchPinSide::Right, "right");
            });
        if pin_side != self.sketch_placement_pin_side {
            self.sketch_placement_pin_side = pin_side;
            self.status = format!(
                "Canvas placement pin side set to {}.",
                self.sketch_placement_pin_side.as_str()
            );
        }
        ui.small(
            "Canvas shortcuts: R rotate, Shift+R rotate back, F flip, Shift+F cycle pin side.",
        );
    }

    pub(super) fn apply_rotate_selected_sketch_components(&mut self, delta_deg: i32) {
        self.apply_transform_selected_sketch_component_styles("rotating", "Rotated", |style| {
            style.rotation_deg = normalize_canvas_rotation(style.rotation_deg + delta_deg);
        });
    }

    pub(super) fn apply_flip_selected_sketch_components(&mut self) {
        self.apply_transform_selected_sketch_component_styles("flipping", "Flipped", |style| {
            style.mirrored = !style.mirrored;
        });
    }

    pub(super) fn apply_cycle_selected_sketch_pin_side(&mut self) {
        self.apply_transform_selected_sketch_component_styles(
            "changing pin side for",
            "Changed pin side for",
            |style| style.pin_side = next_pin_side(style.pin_side),
        );
    }

    fn apply_transform_selected_sketch_component_styles(
        &mut self,
        empty_verb: &str,
        applied_verb: &str,
        mut transform: impl FnMut(&mut SketchNodeStyle),
    ) {
        let Some(snapshot) = &self.project_snapshot else {
            return;
        };
        let selected_components = self
            .selected_sketch_items
            .iter()
            .chain(self.selected_sketch_item.iter())
            .filter_map(|selection| match selection {
                SketchSelection::Component(component_id) => Some(component_id.clone()),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();
        if selected_components.is_empty() {
            self.status = format!("Select one or more components before {empty_verb}.");
            return;
        }
        let mut edits = Vec::with_capacity(selected_components.len());
        for component_id in selected_components {
            let Some(component) = snapshot
                .components_detail
                .iter()
                .find(|component| component.id == component_id)
            else {
                continue;
            };
            let mut style = component.style;
            transform(&mut style);
            edits.push((component_id, style));
        }
        if edits.is_empty() {
            return;
        }
        match edit_schematic_component_styles(&self.project_yaml, &edits) {
            Ok(updated) => {
                let count = edits.len();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("{applied_verb} {count} selected component(s)."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn ensure_net_probe_defaults(&mut self, net_id: &str) {
        self.ensure_canvas_probe_scenario();
        if self.analog_canvas_probe_name.trim().is_empty() {
            self.analog_canvas_probe_name = default_probe_name_for_net(net_id);
        }
    }

    pub(super) fn ensure_component_probe_defaults(&mut self, component_id: &str) {
        self.ensure_canvas_probe_scenario();
        if self.analog_canvas_component_probe_name.trim().is_empty() {
            self.analog_canvas_component_probe_name =
                default_current_probe_name_for_component(component_id);
        }
        if self
            .analog_canvas_component_power_probe_name
            .trim()
            .is_empty()
        {
            self.analog_canvas_component_power_probe_name =
                default_power_probe_name_for_component(component_id);
        }
    }

    fn ensure_canvas_probe_scenario(&mut self) {
        let Ok(choices) = analog::analog_scenario_choices(&self.project_yaml) else {
            return;
        };
        if (self.analog_probe_scenario.is_empty()
            || !choices
                .iter()
                .any(|choice| choice.name == self.analog_probe_scenario))
            && let Some(choice) = choices.first()
        {
            self.analog_probe_scenario = choice.name.clone();
        }
    }

    pub(super) fn handle_sketch_viewport_input(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        response: &egui::Response,
        pan_drag_start_allowed: bool,
        blank_canvas_hovered: bool,
    ) {
        if response.drag_started_by(egui::PointerButton::Primary)
            && pan_drag_start_allowed
            && ui.input(|input| SketchSelectionBoxMode::from_modifiers(input.modifiers).is_none())
        {
            self.sketch_pan_drag_active = true;
        }
        if response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
            || self.sketch_pan_drag_active
        {
            let delta = ui.input(|input| input.pointer.delta());
            self.sketch_pan += delta;
        }

        if response.hovered() {
            let (zoom_delta, scroll_delta, pointer) = ui.input(|input| {
                (
                    input.zoom_delta(),
                    input.smooth_scroll_delta,
                    input.pointer.hover_pos(),
                )
            });
            if (zoom_delta - 1.0).abs() > f32::EPSILON {
                self.zoom_sketch_canvas(zoom_delta, rect, pointer.unwrap_or(rect.center()));
            } else if blank_canvas_hovered && scroll_delta != egui::Vec2::ZERO {
                self.sketch_pan += scroll_delta;
            }
        }
    }

    fn zoom_sketch_canvas(&mut self, zoom_delta: f32, canvas: egui::Rect, focus: egui::Pos2) {
        let (new_zoom, new_pan) =
            zoom_viewport_around(self.sketch_zoom, self.sketch_pan, zoom_delta, canvas, focus);
        self.sketch_zoom = new_zoom;
        self.sketch_pan = new_pan;
    }
}
