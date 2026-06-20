use super::analog::{
    AnalogCurrentProbeDraft, AnalogPowerProbeDraft, AnalogProbeDraft, AnalogProbeRemoveDraft,
    analog_scenario_choices, append_analog_current_probe, append_analog_power_probe,
    append_analog_voltage_probe, remove_analog_probe,
};
use super::sketch::{
    ProjectSnapshot, SketchNodeStyle, SketchPinSide, SketchSelection, add_component, add_net,
    assign_component_pin, connect_component_pins, edit_component_model, edit_component_part_number,
    edit_net_kind, edit_net_nominal_voltage, edit_net_powered, edit_schematic_component_style,
    edit_schematic_node_position, edit_schematic_wire_route, remove_component,
    remove_component_pin, remove_net,
};
use super::sketch_probes::{
    SketchProbeAttachmentKind, SketchProbeElementDraft, SketchProbeTarget,
    upsert_schematic_probe_element,
};
use super::sketch_rename::{rename_component, rename_net};
use super::sketch_spice::{
    SketchComponentSpice, SketchSpiceDraft, SketchSpiceKind, draft_from_existing,
    replace_component_spice,
};
use super::{CircuitCiApp, Stage};
use anyhow::Context;
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn sketch_inspector(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        ui.vertical(|ui| {
            ui.set_min_width(260.0);
            ui.heading("Inspector");
            if self.selected_sketch_items.len() > 1 {
                self.sketch_multi_selection_inspector(ui, snapshot);
            } else {
                match &self.selected_sketch_item {
                    Some(SketchSelection::Component(id)) => {
                        if let Some(component) = snapshot
                            .components_detail
                            .iter()
                            .find(|item| &item.id == id)
                        {
                            ui.strong(&component.id);
                            ui.label("Component ID");
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut self.component_rename_id);
                                if ui.button("Rename").clicked() {
                                    self.apply_component_rename(&component.id);
                                }
                            });

                            let mut model = component.model.clone();
                            ui.label("Model");
                            if ui.text_edit_singleline(&mut model).changed() {
                                self.apply_component_model_edit(&component.id, &model);
                            }

                            let mut part_number = component.part_number.clone().unwrap_or_default();
                            ui.label("Part number");
                            if ui.text_edit_singleline(&mut part_number).changed() {
                                self.apply_component_part_number_edit(&component.id, &part_number);
                            }

                            ui.separator();
                            self.component_spice_editor(
                                ui,
                                &component.id,
                                component.spice.as_ref(),
                                component.pins.iter().any(|pin| pin.pin == "A")
                                    && component.pins.iter().any(|pin| pin.pin == "B"),
                                component.pins.iter().any(|pin| pin.pin == "P")
                                    && component.pins.iter().any(|pin| pin.pin == "N"),
                            );

                            ui.separator();
                            ui.label("Symbol placement");
                            ui.horizontal(|ui| {
                                ui.label(format!("{} deg", component.style.rotation_deg));
                                if ui.button("Rotate").clicked() {
                                    let mut style = component.style;
                                    style.rotation_deg = (style.rotation_deg + 90).rem_euclid(360);
                                    self.apply_component_style_edit(&component.id, style);
                                }
                                let mirror_label = if component.style.mirrored {
                                    "Unflip"
                                } else {
                                    "Flip"
                                };
                                if ui.button(mirror_label).clicked() {
                                    let mut style = component.style;
                                    style.mirrored = !style.mirrored;
                                    self.apply_component_style_edit(&component.id, style);
                                }
                            });
                            let mut pin_side = component.style.pin_side;
                            egui::ComboBox::from_label("Pin side")
                                .selected_text(pin_side.as_str())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut pin_side, SketchPinSide::Auto, "auto");
                                    ui.selectable_value(&mut pin_side, SketchPinSide::Left, "left");
                                    ui.selectable_value(
                                        &mut pin_side,
                                        SketchPinSide::Right,
                                        "right",
                                    );
                                });
                            if pin_side != component.style.pin_side {
                                let mut style = component.style;
                                style.pin_side = pin_side;
                                self.apply_component_style_edit(&component.id, style);
                            }

                            ui.label(format!("pins: {}", component.pins.len()));
                            egui::ScrollArea::vertical()
                                .max_height(230.0)
                                .show(ui, |ui| {
                                    for pin in &component.pins {
                                        ui.monospace(format!("{} -> {}", pin.pin, pin.net));
                                    }
                                });
                            ui.separator();
                            ui.label("Pin assignment");
                            if self.pin_edit_net.is_empty()
                                && let Some(net) = snapshot.nets_detail.first()
                            {
                                self.pin_edit_net = net.id.clone();
                            }
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut self.pin_edit_id);
                                egui::ComboBox::from_id_salt("pin_edit_net")
                                    .selected_text(if self.pin_edit_net.is_empty() {
                                        "select net"
                                    } else {
                                        &self.pin_edit_net
                                    })
                                    .show_ui(ui, |ui| {
                                        for net in &snapshot.nets_detail {
                                            ui.selectable_value(
                                                &mut self.pin_edit_net,
                                                net.id.clone(),
                                                &net.id,
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Assign Pin").clicked() {
                                    self.apply_assign_component_pin(&component.id);
                                }
                                if ui.button("Remove Pin").clicked() {
                                    self.apply_remove_component_pin(&component.id);
                                }
                            });
                            ui.separator();
                            ui.label("Visual wire");
                            ui.horizontal(|ui| {
                                ui.label("pin");
                                ui.text_edit_singleline(&mut self.wire_pin_id);
                            });
                            ui.horizontal(|ui| {
                            if ui.button("Start Wire").clicked() {
                                self.wire_from_component = Some(component.id.clone());
                                if self.wire_pin_id.trim().is_empty() {
                                    self.wire_pin_id = self.pin_edit_id.clone();
                                }
                                self.status = format!(
                                    "Wire mode: click another pin or net node to connect {}.{}.",
                                    component.id,
                                    self.wire_pin_id.trim()
                                );
                            }
                            if ui.button("Cancel Wire").clicked() {
                                self.wire_from_component = None;
                            }
                        });
                            if let Some(source) = &self.wire_from_component {
                                ui.label(format!(
                                    "Active: {source}.{} -> click a pin or net",
                                    self.wire_pin_id.trim()
                                ));
                            }
                            ui.separator();
                            self.component_probe_editor(ui, &component.id);
                            if ui.button("Remove Component").clicked() {
                                self.apply_remove_component(&component.id);
                            }
                        }
                    }
                    Some(SketchSelection::Net(id)) => {
                        if let Some(net) = snapshot.nets_detail.iter().find(|item| &item.id == id) {
                            ui.strong(&net.id);
                            ui.label("Net ID");
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut self.net_rename_id);
                                if ui.button("Rename").clicked() {
                                    self.apply_net_rename(&net.id);
                                }
                            });

                            let mut kind = net.kind.clone();
                            egui::ComboBox::from_label("Kind")
                                .selected_text(&kind)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut kind, "power".to_string(), "power");
                                    ui.selectable_value(&mut kind, "ground".to_string(), "ground");
                                    ui.selectable_value(
                                        &mut kind,
                                        "digital_or_analog".to_string(),
                                        "digital_or_analog",
                                    );
                                });
                            if kind != net.kind {
                                self.apply_net_kind_edit(&net.id, &kind);
                            }

                            ui.horizontal(|ui| {
                                ui.label("Nominal voltage");
                                if let Some(voltage) = net.nominal_voltage {
                                    let mut edited = voltage;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut edited)
                                                .speed(0.1)
                                                .suffix(" V"),
                                        )
                                        .changed()
                                    {
                                        self.apply_net_nominal_voltage_edit(&net.id, Some(edited));
                                    }
                                    if ui.button("Clear").clicked() {
                                        self.apply_net_nominal_voltage_edit(&net.id, None);
                                    }
                                } else if ui.button("Set").clicked() {
                                    self.apply_net_nominal_voltage_edit(&net.id, Some(0.0));
                                }
                            });

                            let mut powered = match net.powered {
                                Some(true) => "true",
                                Some(false) => "false",
                                None => "unset",
                            }
                            .to_string();
                            egui::ComboBox::from_label("Powered")
                                .selected_text(&powered)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut powered, "unset".to_string(), "unset");
                                    ui.selectable_value(&mut powered, "true".to_string(), "true");
                                    ui.selectable_value(&mut powered, "false".to_string(), "false");
                                });
                            if powered
                                != match net.powered {
                                    Some(true) => "true",
                                    Some(false) => "false",
                                    None => "unset",
                                }
                            {
                                self.apply_net_powered_edit(
                                    &net.id,
                                    match powered.as_str() {
                                        "true" => Some(true),
                                        "false" => Some(false),
                                        _ => None,
                                    },
                                );
                            }

                            ui.label(format!("connections: {}", net.connections.len()));
                            egui::ScrollArea::vertical()
                                .max_height(230.0)
                                .show(ui, |ui| {
                                    for connection in &net.connections {
                                        ui.monospace(connection);
                                    }
                                });
                            ui.separator();
                            ui.label("Schematic labels");
                            ui.horizontal(|ui| {
                                if ui.button("Place Label").clicked()
                                    && let Some(canvas) = self.sketch_last_canvas_rect
                                {
                                    self.apply_add_schematic_net_label_at(
                                        canvas,
                                        self.sketch_viewport(),
                                        &net.id,
                                        super::sketch::SketchNetLabelKind::Local,
                                        canvas.center(),
                                    );
                                }
                                if ui.button("Place Off-Page").clicked()
                                    && let Some(canvas) = self.sketch_last_canvas_rect
                                {
                                    self.apply_add_schematic_net_label_at(
                                        canvas,
                                        self.sketch_viewport(),
                                        &net.id,
                                        super::sketch::SketchNetLabelKind::OffPage,
                                        canvas.center(),
                                    );
                                }
                            });
                            ui.separator();
                            self.net_probe_editor(ui, &net.id);
                            if ui.button("Remove Net").clicked() {
                                self.apply_remove_net(&net.id);
                            }
                        }
                    }
                    Some(SketchSelection::Overflow(label)) => {
                        ui.strong(label);
                        ui.label(
                            "Only the first visible rows are drawn to keep the graph readable.",
                        );
                        ui.label("Use the YAML editor for the complete project.");
                    }
                    None => {
                        ui.label("Select a component or net in the graph.");
                        ui.label(format!(
                            "{} components, {} nets",
                            snapshot.components_detail.len(),
                            snapshot.nets_detail.len()
                        ));
                    }
                }
            }
        });
    }

    pub(super) fn apply_component_model_edit(&mut self, component_id: &str, model: &str) {
        match edit_component_model(&self.project_yaml, component_id, model) {
            Ok(updated) => self.apply_edited_project_yaml(updated, "Component model updated."),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_component_rename(&mut self, component_id: &str) {
        let new_id = self.component_rename_id.trim().to_string();
        match rename_component(&self.project_yaml, component_id, &new_id) {
            Ok(updated) => {
                self.set_single_sketch_selection(Some(SketchSelection::Component(new_id.clone())));
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Component {component_id} renamed to {new_id}."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_add_component(&mut self) {
        match add_component(
            &self.project_yaml,
            &self.new_component_id,
            &self.new_component_model,
        ) {
            Ok(updated) => {
                let component_id = self.new_component_id.trim().to_string();
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    component_id.clone(),
                )));
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Component {component_id} added."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_remove_component(&mut self, component_id: &str) {
        match remove_component(&self.project_yaml, component_id) {
            Ok(updated) => {
                self.set_single_sketch_selection(None);
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Component {component_id} removed."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_component_part_number_edit(&mut self, component_id: &str, part_number: &str) {
        match edit_component_part_number(&self.project_yaml, component_id, part_number) {
            Ok(updated) => {
                self.apply_edited_project_yaml(updated, "Component part number updated.")
            }
            Err(error) => self.record_error(error),
        }
    }

    fn component_spice_editor(
        &mut self,
        ui: &mut egui::Ui,
        component_id: &str,
        spice: Option<&SketchComponentSpice>,
        has_passive_pins: bool,
        has_source_pins: bool,
    ) {
        ui.label("SPICE primitive");
        if spice.is_none() {
            ui.label("No component-level SPICE evidence.");
            ui.horizontal_wrapped(|ui| {
                for kind in SketchSpiceKind::ALL {
                    let enabled = if matches!(
                        kind,
                        SketchSpiceKind::Resistor
                            | SketchSpiceKind::Capacitor
                            | SketchSpiceKind::Inductor
                    ) {
                        has_passive_pins
                    } else {
                        has_source_pins
                    };
                    if ui
                        .add_enabled(enabled, egui::Button::new(format!("Add {}", kind.label())))
                        .clicked()
                    {
                        let draft = draft_from_existing(component_id, None, kind);
                        self.apply_component_spice_edit(&draft);
                    }
                }
            });
            return;
        }

        let spice = spice.expect("spice none returned");
        let mut kind = spice.kind;
        egui::ComboBox::from_label("Primitive")
            .selected_text(kind.label())
            .show_ui(ui, |ui| {
                for candidate in SketchSpiceKind::ALL {
                    ui.selectable_value(&mut kind, candidate, candidate.label());
                }
            });
        if kind != spice.kind {
            let draft = draft_from_existing(component_id, Some(spice), kind);
            self.apply_component_spice_edit(&draft);
            return;
        }

        match spice.kind {
            SketchSpiceKind::Resistor => {
                let mut value = spice.value;
                if ui
                    .add(egui::DragValue::new(&mut value).speed(1.0).suffix(" ohm"))
                    .changed()
                {
                    let mut draft = draft_from_existing(component_id, Some(spice), spice.kind);
                    draft.value = value;
                    self.apply_component_spice_edit(&draft);
                }
            }
            SketchSpiceKind::Capacitor => {
                let mut value = spice.value;
                if ui
                    .add(egui::DragValue::new(&mut value).speed(1e-7).suffix(" F"))
                    .changed()
                {
                    let mut draft = draft_from_existing(component_id, Some(spice), spice.kind);
                    draft.value = value;
                    self.apply_component_spice_edit(&draft);
                }
                ui.horizontal(|ui| {
                    ui.label("Initial voltage");
                    if let Some(initial_v) = spice.initial_v {
                        let mut edited = initial_v;
                        if ui
                            .add(egui::DragValue::new(&mut edited).speed(0.1).suffix(" V"))
                            .changed()
                        {
                            let mut draft =
                                draft_from_existing(component_id, Some(spice), spice.kind);
                            draft.initial_v = Some(edited);
                            self.apply_component_spice_edit(&draft);
                        }
                        if ui.button("Clear").clicked() {
                            let mut draft =
                                draft_from_existing(component_id, Some(spice), spice.kind);
                            draft.initial_v = None;
                            self.apply_component_spice_edit(&draft);
                        }
                    } else if ui.button("Set").clicked() {
                        let mut draft = draft_from_existing(component_id, Some(spice), spice.kind);
                        draft.initial_v = Some(0.0);
                        self.apply_component_spice_edit(&draft);
                    }
                });
            }
            SketchSpiceKind::Inductor => {
                let mut value = spice.value;
                if ui
                    .add(egui::DragValue::new(&mut value).speed(1e-7).suffix(" H"))
                    .changed()
                {
                    let mut draft = draft_from_existing(component_id, Some(spice), spice.kind);
                    draft.value = value;
                    self.apply_component_spice_edit(&draft);
                }
            }
            SketchSpiceKind::DcVoltageSource => {
                let mut value = spice.value;
                if ui
                    .add(egui::DragValue::new(&mut value).speed(0.1).suffix(" V"))
                    .changed()
                {
                    let mut draft = draft_from_existing(component_id, Some(spice), spice.kind);
                    draft.value = value;
                    self.apply_component_spice_edit(&draft);
                }
            }
            SketchSpiceKind::DcCurrentSource => {
                let mut value = spice.value;
                if ui
                    .add(egui::DragValue::new(&mut value).speed(0.01).suffix(" A"))
                    .changed()
                {
                    let mut draft = draft_from_existing(component_id, Some(spice), spice.kind);
                    draft.value = value;
                    self.apply_component_spice_edit(&draft);
                }
            }
            SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource => {
                let unit = if spice.kind == SketchSpiceKind::PulseVoltageSource {
                    " V"
                } else {
                    " A"
                };
                self.component_spice_pulse_editor(ui, component_id, spice, unit);
            }
        }
    }

    fn component_spice_pulse_editor(
        &mut self,
        ui: &mut egui::Ui,
        component_id: &str,
        spice: &SketchComponentSpice,
        unit: &str,
    ) {
        let mut pulse = spice.pulse.clone();
        let mut changed = false;
        egui::Grid::new("component_spice_pulse_editor")
            .num_columns(2)
            .show(ui, |ui| {
                changed |= pulse_field(ui, "Initial", &mut pulse.initial, 0.1, unit);
                changed |= pulse_field(ui, "Pulsed", &mut pulse.pulsed, 0.1, unit);
                changed |= pulse_field(ui, "Delay", &mut pulse.delay_us, 0.1, " us");
                changed |= pulse_field(ui, "Rise", &mut pulse.rise_us, 0.1, " us");
                changed |= pulse_field(ui, "Fall", &mut pulse.fall_us, 0.1, " us");
                changed |= pulse_field(ui, "Width", &mut pulse.width_us, 0.1, " us");
                changed |= pulse_field(ui, "Period", &mut pulse.period_us, 0.1, " us");
            });
        if changed {
            let mut draft = draft_from_existing(component_id, Some(spice), spice.kind);
            draft.pulse = pulse;
            self.apply_component_spice_edit(&draft);
        }
    }

    fn apply_component_spice_edit(&mut self, draft: &SketchSpiceDraft) {
        match replace_component_spice(&self.project_yaml, draft) {
            Ok(updated) => {
                self.apply_edited_project_yaml(updated, "Component SPICE evidence updated.")
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_assign_component_pin(&mut self, component_id: &str) {
        match assign_component_pin(
            &self.project_yaml,
            component_id,
            &self.pin_edit_id,
            &self.pin_edit_net,
        ) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Component {component_id} pin {} assigned to {}.",
                    self.pin_edit_id.trim(),
                    self.pin_edit_net.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_remove_component_pin(&mut self, component_id: &str) {
        match remove_component_pin(&self.project_yaml, component_id, &self.pin_edit_id) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Component {component_id} pin {} removed.",
                    self.pin_edit_id.trim()
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_visual_wire(&mut self, component_id: String, net_id: String) {
        self.apply_visual_wire_with_route(component_id, net_id, Vec::new());
    }

    pub(super) fn apply_visual_wire_with_route(
        &mut self,
        component_id: String,
        net_id: String,
        route_points: Vec<(f64, f64)>,
    ) {
        let source_pin_id = self.wire_pin_id.trim().to_string();
        match assign_component_pin(&self.project_yaml, &component_id, &source_pin_id, &net_id) {
            Ok(updated) => match apply_wire_route_if_present(
                updated,
                &component_id,
                &source_pin_id,
                &net_id,
                &route_points,
            ) {
                Ok(updated) => {
                    self.pin_edit_id = self.wire_pin_id.clone();
                    self.pin_edit_net = net_id.clone();
                    self.wire_from_component = None;
                    self.sketch_wire_draft.clear();
                    self.set_single_sketch_selection(Some(SketchSelection::Component(
                        component_id.clone(),
                    )));
                    self.apply_edited_project_yaml(
                        updated,
                        &format!(
                            "Visual wire assigned {component_id}.{} to {net_id}.",
                            source_pin_id
                        ),
                    );
                }
                Err(error) => self.record_error(error),
            },
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_visual_pin_wire_with_route(
        &mut self,
        source_component_id: String,
        target_component_id: String,
        target_pin_id: String,
        route_points: Vec<(f64, f64)>,
    ) {
        let source_pin_id = self.wire_pin_id.trim().to_string();
        match connect_component_pins(
            &self.project_yaml,
            &source_component_id,
            &source_pin_id,
            &target_component_id,
            &target_pin_id,
        ) {
            Ok(updated) => {
                match net_for_component_pin(&updated, &source_component_id, &source_pin_id)
                    .and_then(|net_id| {
                        apply_wire_route_if_present(
                            updated,
                            &source_component_id,
                            &source_pin_id,
                            &net_id,
                            &route_points,
                        )
                        .map(|updated| (updated, net_id))
                    }) {
                    Ok((updated, _net_id)) => {
                        self.pin_edit_id = target_pin_id.clone();
                        self.wire_from_component = None;
                        self.sketch_wire_draft.clear();
                        self.set_single_sketch_selection(Some(SketchSelection::Component(
                            target_component_id.clone(),
                        )));
                        self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Visual wire connected {source_component_id}.{source_pin_id} to {target_component_id}.{target_pin_id}."
                    ),
                );
                    }
                    Err(error) => self.record_error(error),
                }
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_remove_canvas_probe(&mut self, scenario_name: &str, probe_name: &str) {
        let draft = AnalogProbeRemoveDraft {
            scenario_name: scenario_name.to_string(),
            probe_name: probe_name.to_string(),
        };
        match remove_analog_probe(&self.project_yaml, &draft) {
            Ok(updated) => {
                if self.analog_probe_scenario == draft.scenario_name
                    && self.analog_assertion_probe == draft.probe_name
                {
                    self.analog_assertion_probe.clear();
                }
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Removed probe {} from scenario {}.",
                        draft.probe_name, draft.scenario_name
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_schematic_node_position(
        &mut self,
        selection: SketchSelection,
        x: f64,
        y: f64,
    ) {
        match edit_schematic_node_position(&self.project_yaml, &selection, x, y) {
            Ok(updated) => {
                self.set_single_sketch_selection(Some(selection));
                self.apply_edited_project_yaml(updated, "Schematic node position updated.");
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_component_style_edit(
        &mut self,
        component_id: &str,
        style: SketchNodeStyle,
    ) {
        match edit_schematic_component_style(&self.project_yaml, component_id, style) {
            Ok(updated) => {
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    component_id.to_string(),
                )));
                self.apply_edited_project_yaml(updated, "Schematic symbol style updated.");
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_add_net(&mut self) {
        match add_net(&self.project_yaml, &self.new_net_id, &self.new_net_kind) {
            Ok(updated) => {
                let net_id = self.new_net_id.trim().to_string();
                self.set_single_sketch_selection(Some(SketchSelection::Net(net_id.clone())));
                self.apply_edited_project_yaml(updated, &format!("Net {net_id} added."));
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_remove_net(&mut self, net_id: &str) {
        match remove_net(&self.project_yaml, net_id) {
            Ok(updated) => {
                self.set_single_sketch_selection(None);
                self.apply_edited_project_yaml(updated, &format!("Net {net_id} removed."));
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_net_kind_edit(&mut self, net_id: &str, kind: &str) {
        match edit_net_kind(&self.project_yaml, net_id, kind) {
            Ok(updated) => self.apply_edited_project_yaml(updated, "Net kind updated."),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_net_rename(&mut self, net_id: &str) {
        let new_id = self.net_rename_id.trim().to_string();
        match rename_net(&self.project_yaml, net_id, &new_id) {
            Ok(updated) => {
                self.set_single_sketch_selection(Some(SketchSelection::Net(new_id.clone())));
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Net {net_id} renamed to {new_id}."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_net_nominal_voltage_edit(&mut self, net_id: &str, voltage: Option<f64>) {
        match edit_net_nominal_voltage(&self.project_yaml, net_id, voltage) {
            Ok(updated) => self.apply_edited_project_yaml(updated, "Net nominal voltage updated."),
            Err(error) => self.record_error(error),
        }
    }

    fn apply_net_powered_edit(&mut self, net_id: &str, powered: Option<bool>) {
        match edit_net_powered(&self.project_yaml, net_id, powered) {
            Ok(updated) => self.apply_edited_project_yaml(updated, "Net powered flag updated."),
            Err(error) => self.record_error(error),
        }
    }

    fn net_probe_editor(&mut self, ui: &mut egui::Ui, net_id: &str) {
        ui.label("Simulation probe");
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices,
            Err(error) => {
                ui.label(format!("Analog scenarios unavailable: {error}"));
                return;
            }
        };
        if choices.is_empty() {
            ui.label("Add an analog transient scenario before inserting probes.");
            if ui.button("Seed Scenario Editor").clicked() {
                self.analog_probe_net = net_id.to_string();
                self.analog_probe_name = default_probe_name_for_net(net_id);
                self.stage = Stage::Simulation;
                self.status = format!("Simulation editor seeded for net {net_id}.");
            }
            return;
        }
        if self.analog_probe_scenario.is_empty()
            || !choices
                .iter()
                .any(|choice| choice.name == self.analog_probe_scenario)
        {
            self.analog_probe_scenario = choices[0].name.clone();
        }
        if self.analog_canvas_probe_name.is_empty() {
            self.analog_canvas_probe_name = default_probe_name_for_net(net_id);
        }
        egui::ComboBox::from_id_salt("canvas_probe_scenario")
            .selected_text(&self.analog_probe_scenario)
            .show_ui(ui, |ui| {
                for choice in &choices {
                    ui.selectable_value(
                        &mut self.analog_probe_scenario,
                        choice.name.clone(),
                        &choice.name,
                    );
                }
            });
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.analog_canvas_probe_name);
            if ui.button("Use Net").clicked() {
                self.analog_canvas_probe_name = default_probe_name_for_net(net_id);
            }
        });
        if ui.button("Add Voltage Probe").clicked() {
            self.apply_add_voltage_probe_for_net(net_id);
        }
    }

    fn component_probe_editor(&mut self, ui: &mut egui::Ui, component_id: &str) {
        ui.label("Simulation probe");
        let choices = match analog_scenario_choices(&self.project_yaml) {
            Ok(choices) => choices,
            Err(error) => {
                ui.label(format!("Analog scenarios unavailable: {error}"));
                return;
            }
        };
        if choices.is_empty() {
            ui.label("Add an analog transient scenario before inserting probes.");
            if ui.button("Open Scenario Editor").clicked() {
                self.stage = Stage::Simulation;
                self.status =
                    format!("Simulation editor opened before probing component {component_id}.");
            }
            return;
        }
        if self.analog_probe_scenario.is_empty()
            || !choices
                .iter()
                .any(|choice| choice.name == self.analog_probe_scenario)
        {
            self.analog_probe_scenario = choices[0].name.clone();
        }
        if self.analog_canvas_component_probe_name.is_empty() {
            self.analog_canvas_component_probe_name =
                default_current_probe_name_for_component(component_id);
        }
        if self.analog_canvas_component_power_probe_name.is_empty() {
            self.analog_canvas_component_power_probe_name =
                default_power_probe_name_for_component(component_id);
        }
        egui::ComboBox::from_id_salt("canvas_component_probe_scenario")
            .selected_text(&self.analog_probe_scenario)
            .show_ui(ui, |ui| {
                for choice in &choices {
                    ui.selectable_value(
                        &mut self.analog_probe_scenario,
                        choice.name.clone(),
                        &choice.name,
                    );
                }
            });
        ui.label("Current");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.analog_canvas_component_probe_name);
            if ui.button("Use Component").clicked() {
                self.analog_canvas_component_probe_name =
                    default_current_probe_name_for_component(component_id);
            }
        });
        if ui.button("Add Current Probe").clicked() {
            self.apply_add_current_probe_for_component(component_id);
        }
        ui.label("Power");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.analog_canvas_component_power_probe_name);
            if ui.button("Use Component").clicked() {
                self.analog_canvas_component_power_probe_name =
                    default_power_probe_name_for_component(component_id);
            }
        });
        if ui.button("Add Power Probe").clicked() {
            self.apply_add_power_probe_for_component(component_id);
        }
    }

    pub(super) fn apply_add_voltage_probe_for_net(&mut self, net_id: &str) -> bool {
        self.apply_add_voltage_probe_for_net_with_attachment(
            net_id,
            SketchProbeAttachmentKind::Node,
            None,
        )
    }

    pub(super) fn apply_add_voltage_probe_for_net_with_attachment(
        &mut self,
        net_id: &str,
        attachment: SketchProbeAttachmentKind,
        source: Option<String>,
    ) -> bool {
        let draft = AnalogProbeDraft {
            scenario_name: self.analog_probe_scenario.clone(),
            net_id: net_id.to_string(),
            probe_name: self.analog_canvas_probe_name.clone(),
        };
        match append_analog_voltage_probe(&self.project_yaml, &draft).and_then(|updated| {
            upsert_schematic_probe_element(
                &updated,
                &SketchProbeElementDraft {
                    element_id: schematic_probe_element_id(
                        &self.analog_probe_scenario,
                        &self.analog_canvas_probe_name,
                    ),
                    scenario_name: self.analog_probe_scenario.clone(),
                    probe_name: self.analog_canvas_probe_name.clone(),
                    target: SketchProbeTarget::Net(net_id.to_string()),
                    attachment,
                    source,
                },
            )
        }) {
            Ok(updated) => {
                self.analog_assertion_scenario = self.analog_probe_scenario.clone();
                self.analog_assertion_probe = self.analog_canvas_probe_name.trim().to_string();
                let scenario_name = self.analog_probe_scenario.clone();
                let probe_name = self.analog_canvas_probe_name.clone();
                self.remember_scope_probe_target(&scenario_name, &probe_name);
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Voltage probe {} added for net {net_id}.",
                        self.analog_canvas_probe_name.trim()
                    ),
                );
                true
            }
            Err(error) => {
                self.record_error(error);
                false
            }
        }
    }

    pub(super) fn apply_add_current_probe_for_component(&mut self, component_id: &str) -> bool {
        self.apply_add_current_probe_for_component_with_attachment(
            component_id,
            SketchProbeAttachmentKind::Node,
            None,
        )
    }

    pub(super) fn apply_add_current_probe_for_component_with_attachment(
        &mut self,
        component_id: &str,
        attachment: SketchProbeAttachmentKind,
        source: Option<String>,
    ) -> bool {
        let draft = AnalogCurrentProbeDraft {
            scenario_name: self.analog_probe_scenario.clone(),
            component_id: component_id.to_string(),
            probe_name: self.analog_canvas_component_probe_name.clone(),
        };
        match append_analog_current_probe(
            &self.project_yaml,
            std::path::Path::new(&self.project_path),
            &draft,
        )
        .and_then(|updated| {
            upsert_schematic_probe_element(
                &updated,
                &SketchProbeElementDraft {
                    element_id: schematic_probe_element_id(
                        &self.analog_probe_scenario,
                        &self.analog_canvas_component_probe_name,
                    ),
                    scenario_name: self.analog_probe_scenario.clone(),
                    probe_name: self.analog_canvas_component_probe_name.clone(),
                    target: SketchProbeTarget::Component(component_id.to_string()),
                    attachment,
                    source,
                },
            )
        }) {
            Ok(updated) => {
                self.analog_assertion_scenario = self.analog_probe_scenario.clone();
                self.analog_assertion_probe =
                    self.analog_canvas_component_probe_name.trim().to_string();
                let scenario_name = self.analog_probe_scenario.clone();
                let probe_name = self.analog_canvas_component_probe_name.clone();
                self.remember_scope_probe_target(&scenario_name, &probe_name);
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Current probe {} added for component {component_id}.",
                        self.analog_canvas_component_probe_name.trim()
                    ),
                );
                true
            }
            Err(error) => {
                self.record_error(error);
                false
            }
        }
    }

    pub(super) fn apply_add_power_probe_for_component(&mut self, component_id: &str) -> bool {
        self.apply_add_power_probe_for_component_with_attachment(
            component_id,
            SketchProbeAttachmentKind::Node,
            None,
        )
    }

    pub(super) fn apply_add_power_probe_for_component_with_attachment(
        &mut self,
        component_id: &str,
        attachment: SketchProbeAttachmentKind,
        source: Option<String>,
    ) -> bool {
        let draft = AnalogPowerProbeDraft {
            scenario_name: self.analog_probe_scenario.clone(),
            component_id: component_id.to_string(),
            probe_name: self.analog_canvas_component_power_probe_name.clone(),
        };
        match append_analog_power_probe(
            &self.project_yaml,
            std::path::Path::new(&self.project_path),
            &draft,
        )
        .and_then(|updated| {
            upsert_schematic_probe_element(
                &updated,
                &SketchProbeElementDraft {
                    element_id: schematic_probe_element_id(
                        &self.analog_probe_scenario,
                        &self.analog_canvas_component_power_probe_name,
                    ),
                    scenario_name: self.analog_probe_scenario.clone(),
                    probe_name: self.analog_canvas_component_power_probe_name.clone(),
                    target: SketchProbeTarget::Component(component_id.to_string()),
                    attachment,
                    source,
                },
            )
        }) {
            Ok(updated) => {
                self.analog_assertion_scenario = self.analog_probe_scenario.clone();
                self.analog_assertion_probe = self
                    .analog_canvas_component_power_probe_name
                    .trim()
                    .to_string();
                let scenario_name = self.analog_probe_scenario.clone();
                let probe_name = self.analog_canvas_component_power_probe_name.clone();
                self.remember_scope_probe_target(&scenario_name, &probe_name);
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Power probe {} added for component {component_id}.",
                        self.analog_canvas_component_power_probe_name.trim()
                    ),
                );
                true
            }
            Err(error) => {
                self.record_error(error);
                false
            }
        }
    }
}

fn schematic_probe_element_id(scenario_name: &str, probe_name: &str) -> String {
    format!("{}_{}", scenario_name.trim(), probe_name.trim())
}

fn apply_wire_route_if_present(
    updated_yaml: String,
    component_id: &str,
    pin_id: &str,
    net_id: &str,
    route_points: &[(f64, f64)],
) -> anyhow::Result<String> {
    if route_points.is_empty() {
        return Ok(updated_yaml);
    }
    let source = format!("{component_id}.{pin_id}");
    edit_schematic_wire_route(&updated_yaml, &source, net_id, route_points)
}

fn net_for_component_pin(text: &str, component_id: &str, pin_id: &str) -> anyhow::Result<String> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Edited project YAML is not valid Board IR.")?;
    let component = project
        .board
        .components
        .get(component_id)
        .with_context(|| format!("Board IR component {component_id} was not found."))?;
    component
        .pins
        .get(pin_id)
        .cloned()
        .with_context(|| format!("Board IR pin {component_id}.{pin_id} is not connected."))
}

fn pulse_field(ui: &mut egui::Ui, label: &str, value: &mut f64, speed: f64, suffix: &str) -> bool {
    ui.label(label);
    let changed = ui
        .add(egui::DragValue::new(value).speed(speed).suffix(suffix))
        .changed();
    ui.end_row();
    changed
}

pub(super) fn default_probe_name_for_net(net_id: &str) -> String {
    let name = sanitized_probe_stem(net_id);
    if name.is_empty() {
        "net_voltage".to_string()
    } else {
        format!("{name}_voltage")
    }
}

pub(super) fn default_current_probe_name_for_component(component_id: &str) -> String {
    let name = sanitized_probe_stem(component_id);
    if name.is_empty() {
        "component_current".to_string()
    } else {
        format!("{name}_current")
    }
}

pub(super) fn default_power_probe_name_for_component(component_id: &str) -> String {
    let name = sanitized_probe_stem(component_id);
    if name.is_empty() {
        "component_power".to_string()
    } else {
        format!("{name}_power")
    }
}

fn sanitized_probe_stem(value: &str) -> String {
    let mut name = String::new();
    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
            name.push(character);
        } else if !name.ends_with('_') {
            name.push('_');
        }
    }
    name.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        apply_wire_route_if_present, default_current_probe_name_for_component,
        default_power_probe_name_for_component, default_probe_name_for_net, net_for_component_pin,
    };

    fn wired_project_yaml() -> &'static str {
        "project:
  name: inspector_wire_route_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins:
        A: net_a
  nets:
    net_a:
      kind: digital_or_analog
"
    }

    #[test]
    fn default_probe_name_sanitizes_canvas_net_ids() {
        assert_eq!(
            default_probe_name_for_net("motor.phase-a"),
            "motor.phase-a_voltage"
        );
        assert_eq!(default_probe_name_for_net(" net/a "), "net_a_voltage");
        assert_eq!(default_probe_name_for_net("///"), "net_voltage");
    }

    #[test]
    fn default_current_probe_name_sanitizes_canvas_component_ids() {
        assert_eq!(
            default_current_probe_name_for_component("Q-2"),
            "Q-2_current"
        );
        assert_eq!(
            default_current_probe_name_for_component(" U/load "),
            "U_load_current"
        );
        assert_eq!(
            default_current_probe_name_for_component("///"),
            "component_current"
        );
    }

    #[test]
    fn default_power_probe_name_sanitizes_canvas_component_ids() {
        assert_eq!(default_power_probe_name_for_component("R1"), "R1_power");
        assert_eq!(
            default_power_probe_name_for_component(" load/r "),
            "load_r_power"
        );
        assert_eq!(
            default_power_probe_name_for_component("///"),
            "component_power"
        );
    }

    #[test]
    fn pending_wire_route_is_applied_to_connected_source_pin() {
        let net = net_for_component_pin(wired_project_yaml(), "R1", "A").unwrap();
        let edited = apply_wire_route_if_present(
            wired_project_yaml().to_string(),
            "R1",
            "A",
            &net,
            &[(120.0, 140.0), (180.0, 140.0)],
        )
        .unwrap();

        assert!(edited.contains("wire_routes:"));
        assert!(edited.contains("R1.A->net_a:"));
        assert!(edited.contains("x: 120.0"));
        assert!(edited.contains("x: 180.0"));
    }
}
