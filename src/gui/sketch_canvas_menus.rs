use eframe::egui;

use super::sketch::{self, ProjectSnapshot, SketchSelection, edge_label_position};
use super::sketch_canvas::component_context_pin;
use super::sketch_probes::SketchProbeBadge;
use super::{CircuitCiApp, Stage};

impl CircuitCiApp {
    pub(super) fn open_probe_badge_in_simulation(&mut self, badge: &SketchProbeBadge) {
        self.analog_probe_scenario = badge.probe.scenario_name.clone();
        self.analog_assertion_scenario = badge.probe.scenario_name.clone();
        self.analog_assertion_probe = badge.probe.probe_name.clone();
        self.stage = Stage::Simulation;
        self.status = format!(
            "Selected {} probe {} from scenario {}.",
            badge.probe.quantity.label(),
            badge.probe.probe_name,
            badge.probe.scenario_name
        );
    }

    pub(super) fn probe_badge_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        badge: &SketchProbeBadge,
        sampled_value: Option<f64>,
    ) {
        ui.strong(format!(
            "{} probe {}",
            badge.probe.quantity.label(),
            badge.probe.probe_name
        ));
        if ui.button("Open in Simulation").clicked() {
            self.open_probe_badge_in_simulation(badge);
            ui.close();
        }
        ui.separator();
        if ui.button("Add Assertion From Settings").clicked() {
            self.apply_add_canvas_probe_assertion(
                &badge.probe.scenario_name,
                &badge.probe.probe_name,
            );
            ui.close();
        }
        if ui
            .add_enabled(
                sampled_value.is_some(),
                egui::Button::new("Quick Require Above Cursor Sample"),
            )
            .clicked()
        {
            self.apply_quick_canvas_probe_assertion(&badge.probe, "above");
            ui.close();
        }
        if ui
            .add_enabled(
                sampled_value.is_some(),
                egui::Button::new("Quick Require Below Cursor Sample"),
            )
            .clicked()
        {
            self.apply_quick_canvas_probe_assertion(&badge.probe, "below");
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(
                !badge.probe.assertion_names.is_empty(),
                egui::Button::new("Clear Probe Assertions"),
            )
            .clicked()
        {
            self.apply_remove_canvas_probe_assertions(
                &badge.probe.scenario_name,
                &badge.probe.probe_name,
            );
            ui.close();
        }
        if ui.button("Remove Probe").clicked() {
            self.apply_remove_canvas_probe(&badge.probe.scenario_name, &badge.probe.probe_name);
            ui.close();
        }
    }

    pub(super) fn sketch_node_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        node: &sketch::SketchNode,
        snapshot: &ProjectSnapshot,
        canvas: egui::Rect,
        viewport: sketch::SketchViewport,
        pointer_hover: Option<egui::Pos2>,
    ) {
        match &node.selection {
            SketchSelection::Component(component_id) => {
                ui.strong(format!("Component {component_id}"));
                if ui.button("Inspect Component").clicked() {
                    self.set_single_sketch_selection(Some(node.selection.clone()));
                    ui.close();
                }
                if ui.button("Duplicate Component").clicked() {
                    self.set_single_sketch_selection(Some(node.selection.clone()));
                    self.apply_duplicate_selected_sketch_items();
                    ui.close();
                }
                if ui.button("Copy Component").clicked() {
                    self.set_single_sketch_selection(Some(node.selection.clone()));
                    self.apply_copy_selected_sketch_items();
                    ui.close();
                }
                if ui.button("Start Wire From Pin").clicked() {
                    let (pin, net) =
                        component_context_pin(snapshot, component_id, &self.wire_pin_id);
                    self.set_single_sketch_selection(Some(node.selection.clone()));
                    self.pin_edit_id = pin.clone();
                    self.pin_edit_net = net;
                    self.wire_pin_id = pin.clone();
                    self.wire_from_component = Some(component_id.clone());
                    self.status = format!(
                        "Wire mode: click another pin, net, or wire to connect {component_id}.{pin}."
                    );
                    ui.close();
                }
                ui.separator();
                if ui.button("Add Current Probe").clicked() {
                    self.ensure_component_probe_defaults(component_id);
                    self.apply_add_current_probe_for_component(component_id);
                    ui.close();
                }
                if ui.button("Add Power Probe").clicked() {
                    self.ensure_component_probe_defaults(component_id);
                    self.apply_add_power_probe_for_component(component_id);
                    ui.close();
                }
                ui.separator();
                if ui.button("Delete Component").clicked() {
                    self.set_single_sketch_selection(Some(node.selection.clone()));
                    self.apply_delete_selected_sketch_item();
                    ui.close();
                }
            }
            SketchSelection::Net(net_id) => {
                ui.strong(format!("Net {net_id}"));
                let target = pointer_hover.unwrap_or_else(|| node.rect.center());
                if ui.button("Place Net Label Here").clicked() {
                    self.apply_add_schematic_net_label_at(
                        canvas,
                        viewport,
                        net_id,
                        sketch::SketchNetLabelKind::Local,
                        target,
                    );
                    ui.close();
                }
                if ui.button("Place Off-Page Connector Here").clicked() {
                    self.apply_add_schematic_net_label_at(
                        canvas,
                        viewport,
                        net_id,
                        sketch::SketchNetLabelKind::OffPage,
                        target,
                    );
                    ui.close();
                }
                ui.separator();
                self.net_context_menu(ui, net_id, "Inspect Net", "Delete Net");
            }
            SketchSelection::Overflow(label) => {
                ui.strong(label);
                ui.label("Open the YAML editor or use Fit All for hidden graph items.");
            }
        }
    }

    pub(super) fn sketch_canvas_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        canvas: egui::Rect,
        pointer_hover: Option<egui::Pos2>,
    ) {
        ui.strong("Canvas");
        if ui
            .add_enabled(
                !self.project_yaml.trim().is_empty(),
                egui::Button::new(format!("Place {}", self.sketch_palette_kind.label())),
            )
            .clicked()
        {
            let target = pointer_hover.unwrap_or_else(|| canvas.center());
            self.apply_insert_sketch_primitive_at(canvas, target);
            ui.close();
        }
        if !self.selected_library_model.trim().is_empty()
            && ui
                .add_enabled(
                    !self.project_yaml.trim().is_empty()
                        && !self.new_component_id.trim().is_empty(),
                    egui::Button::new(format!("Place {}", self.selected_library_model)),
                )
                .clicked()
        {
            let target = pointer_hover.unwrap_or_else(|| canvas.center());
            self.apply_insert_selected_library_model_at(canvas, target);
            ui.close();
        }
        if ui
            .add_enabled(
                !self.project_yaml.trim().is_empty()
                    && !self.sketch_net_label_net_id.trim().is_empty(),
                egui::Button::new(format!(
                    "Place {} {}",
                    self.sketch_net_label_kind.label(),
                    self.sketch_net_label_net_id.trim()
                )),
            )
            .clicked()
        {
            let target = pointer_hover.unwrap_or_else(|| canvas.center());
            self.apply_add_or_create_schematic_net_label_at(canvas, self.sketch_viewport(), target);
            ui.close();
        }
        if ui
            .add_enabled(
                self.has_pasteable_sketch_clipboard(),
                egui::Button::new("Paste Here"),
            )
            .clicked()
        {
            self.apply_paste_sketch_clipboard(canvas, pointer_hover);
            ui.close();
        }
    }

    pub(super) fn sketch_wire_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        canvas: egui::Rect,
        viewport: sketch::SketchViewport,
        edge: &sketch::SketchEdge,
        pointer_hover: Option<egui::Pos2>,
        route_handle_index: Option<usize>,
    ) {
        ui.strong(format!("Wire {}", edge.net_id));
        ui.label(format!("source: {}", edge.source));
        if let Some(target) = pointer_hover {
            if ui.button("Place Net Label Here").clicked() {
                self.apply_add_schematic_net_label_at(
                    canvas,
                    viewport,
                    &edge.net_id,
                    sketch::SketchNetLabelKind::Local,
                    target,
                );
                ui.close();
            }
            if ui.button("Place Off-Page Connector Here").clicked() {
                self.apply_add_schematic_net_label_at(
                    canvas,
                    viewport,
                    &edge.net_id,
                    sketch::SketchNetLabelKind::OffPage,
                    target,
                );
                ui.close();
            }
            ui.separator();
        }
        if ui
            .add_enabled(
                pointer_hover.is_some(),
                egui::Button::new("Insert Route Handle Here"),
            )
            .clicked()
        {
            let target = pointer_hover.unwrap_or_else(|| edge_label_position(edge));
            self.apply_insert_schematic_wire_route_point(canvas, viewport, edge, target);
            ui.close();
        }
        if let Some(index) = route_handle_index
            && ui.button("Delete Route Handle").clicked()
        {
            self.apply_delete_schematic_wire_route_point(canvas, viewport, edge, index);
            ui.close();
        }
        if ui
            .add_enabled(
                !edge.route.is_empty(),
                egui::Button::new("Clear Custom Route"),
            )
            .clicked()
        {
            self.apply_remove_schematic_wire_route(edge);
            ui.close();
        }
        ui.separator();
        self.net_context_menu(ui, &edge.net_id, "Inspect Wire Net", "Delete Wire Net");
    }

    pub(super) fn net_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        net_id: &str,
        inspect_label: &str,
        delete_label: &str,
    ) {
        if ui.button(inspect_label).clicked() {
            self.set_single_sketch_selection(Some(SketchSelection::Net(net_id.to_string())));
            ui.close();
        }
        if let Some(component_id) = self.wire_from_component.clone()
            && ui.button("Connect Active Wire Here").clicked()
        {
            self.apply_visual_wire(component_id, net_id.to_string());
            ui.close();
        }
        ui.separator();
        if ui.button("Add Voltage Probe").clicked() {
            self.ensure_net_probe_defaults(net_id);
            self.apply_add_voltage_probe_for_net(net_id);
            ui.close();
        }
        ui.separator();
        if ui.button(delete_label).clicked() {
            self.set_single_sketch_selection(Some(SketchSelection::Net(net_id.to_string())));
            self.apply_delete_selected_sketch_item();
            ui.close();
        }
    }
}
