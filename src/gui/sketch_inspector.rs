use super::analog::{
    AnalogCurrentProbeDraft, AnalogPowerProbeDraft, AnalogProbeDraft, AnalogProbeRemoveDraft,
    analog_scenario_choices, append_analog_current_probe, append_analog_power_probe,
    append_analog_voltage_probe, remove_analog_probe,
};
use super::sketch::{
    ProjectSnapshot, SketchNodeStyle, SketchPinSide, SketchSelection, add_component, add_net,
    assign_component_pin, connect_component_pins, edit_component_model, edit_component_part_number,
    edit_net_kind, edit_net_nominal_voltage, edit_net_powered, edit_schematic_component_style,
    edit_schematic_node_position, remove_component, remove_component_pin, remove_net,
};
use super::{CircuitCiApp, Stage};
use eframe::egui;

impl CircuitCiApp {
    pub(super) fn sketch_inspector(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        ui.vertical(|ui| {
            ui.set_min_width(260.0);
            ui.heading("Inspector");
            match &self.selected_sketch_item {
                Some(SketchSelection::Component(id)) => {
                    if let Some(component) = snapshot
                        .components_detail
                        .iter()
                        .find(|item| &item.id == id)
                    {
                        ui.strong(&component.id);
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
                                ui.selectable_value(&mut pin_side, SketchPinSide::Right, "right");
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
                                    .add(egui::DragValue::new(&mut edited).speed(0.1).suffix(" V"))
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
                        self.net_probe_editor(ui, &net.id);
                        if ui.button("Remove Net").clicked() {
                            self.apply_remove_net(&net.id);
                        }
                    }
                }
                Some(SketchSelection::Overflow(label)) => {
                    ui.strong(label);
                    ui.label("Only the first visible rows are drawn to keep the graph readable.");
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
        });
    }

    pub(super) fn apply_component_model_edit(&mut self, component_id: &str, model: &str) {
        match edit_component_model(&self.project_yaml, component_id, model) {
            Ok(updated) => self.apply_edited_project_yaml(updated, "Component model updated."),
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
        match assign_component_pin(
            &self.project_yaml,
            &component_id,
            &self.wire_pin_id,
            &net_id,
        ) {
            Ok(updated) => {
                self.pin_edit_id = self.wire_pin_id.clone();
                self.pin_edit_net = net_id.clone();
                self.wire_from_component = None;
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    component_id.clone(),
                )));
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Visual wire assigned {component_id}.{} to {net_id}.",
                        self.wire_pin_id.trim()
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_visual_pin_wire(
        &mut self,
        source_component_id: String,
        target_component_id: String,
        target_pin_id: String,
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
                self.pin_edit_id = target_pin_id.clone();
                self.wire_from_component = None;
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

    fn apply_component_style_edit(&mut self, component_id: &str, style: SketchNodeStyle) {
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

    pub(super) fn apply_add_voltage_probe_for_net(&mut self, net_id: &str) {
        let draft = AnalogProbeDraft {
            scenario_name: self.analog_probe_scenario.clone(),
            net_id: net_id.to_string(),
            probe_name: self.analog_canvas_probe_name.clone(),
        };
        match append_analog_voltage_probe(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.analog_assertion_scenario = self.analog_probe_scenario.clone();
                self.analog_assertion_probe = self.analog_canvas_probe_name.trim().to_string();
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Voltage probe {} added for net {net_id}.",
                        self.analog_canvas_probe_name.trim()
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_add_current_probe_for_component(&mut self, component_id: &str) {
        let draft = AnalogCurrentProbeDraft {
            scenario_name: self.analog_probe_scenario.clone(),
            component_id: component_id.to_string(),
            probe_name: self.analog_canvas_component_probe_name.clone(),
        };
        match append_analog_current_probe(
            &self.project_yaml,
            std::path::Path::new(&self.project_path),
            &draft,
        ) {
            Ok(updated) => {
                self.analog_assertion_scenario = self.analog_probe_scenario.clone();
                self.analog_assertion_probe =
                    self.analog_canvas_component_probe_name.trim().to_string();
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Current probe {} added for component {component_id}.",
                        self.analog_canvas_component_probe_name.trim()
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_add_power_probe_for_component(&mut self, component_id: &str) {
        let draft = AnalogPowerProbeDraft {
            scenario_name: self.analog_probe_scenario.clone(),
            component_id: component_id.to_string(),
            probe_name: self.analog_canvas_component_power_probe_name.clone(),
        };
        match append_analog_power_probe(
            &self.project_yaml,
            std::path::Path::new(&self.project_path),
            &draft,
        ) {
            Ok(updated) => {
                self.analog_assertion_scenario = self.analog_probe_scenario.clone();
                self.analog_assertion_probe = self
                    .analog_canvas_component_power_probe_name
                    .trim()
                    .to_string();
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Power probe {} added for component {component_id}.",
                        self.analog_canvas_component_power_probe_name.trim()
                    ),
                );
            }
            Err(error) => self.record_error(error),
        }
    }
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
        default_current_probe_name_for_component, default_power_probe_name_for_component,
        default_probe_name_for_net,
    };

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
}
