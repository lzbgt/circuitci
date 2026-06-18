use super::CircuitCiApp;
use super::sketch::{
    SketchNodeStyle, SketchSelection, encode_edited_project_yaml, ensure_board_child_mapping_mut,
    persisted_node_position_from_screen_with_snap, schematic_node_style_mapping,
    validated_graph_id,
};
use super::sketch_spice::{SketchSpiceKind, SketchSpicePulse, default_value, draft_from_existing};
use anyhow::{Context, Result};
use eframe::egui;

#[derive(Debug, Clone)]
pub(super) struct SketchPrimitiveInsertDraft {
    pub(super) component_id: String,
    pub(super) kind: SketchSpiceKind,
    pub(super) value: f64,
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) style: SketchNodeStyle,
}

pub(super) fn insert_primitive_component(
    text: &str,
    draft: &SketchPrimitiveInsertDraft,
) -> Result<String> {
    let component_id = validated_graph_id(&draft.component_id, "component")?;
    validate_primitive_value(draft.kind, draft.value)?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;

    {
        let components = ensure_board_child_mapping_mut(&mut yaml, "components")?;
        let component_key = key(component_id);
        if components.contains_key(&component_key) {
            anyhow::bail!("Board IR component {component_id} already exists.");
        }
    }

    let (first_pin, second_pin) = draft.kind.requires_pins();
    let first_net = unique_net_id(&mut yaml, &format!("{component_id}_{first_pin}"))?;
    let second_net = unique_net_id(&mut yaml, &format!("{component_id}_{second_pin}"))?;
    insert_primitive_net(&mut yaml, &first_net, first_pin_net_kind(draft.kind))?;
    insert_primitive_net(&mut yaml, &second_net, second_pin_net_kind(draft.kind))?;
    insert_primitive_component_yaml(
        &mut yaml,
        component_id,
        draft.kind,
        draft.value,
        [
            PrimitivePinNet {
                pin: first_pin,
                net: &first_net,
            },
            PrimitivePinNet {
                pin: second_pin,
                net: &second_net,
            },
        ],
    )?;
    insert_primitive_position(&mut yaml, component_id, draft.x, draft.y)?;
    insert_primitive_style(&mut yaml, component_id, draft.style)?;

    encode_edited_project_yaml(yaml)
}

pub(super) fn next_primitive_component_id(text: &str, kind: SketchSpiceKind) -> String {
    let prefix = kind.device_prefix();
    let Ok(project) = serde_yaml_ng::from_str::<crate::board_ir::BoardProject>(text) else {
        return format!("{prefix}1");
    };
    let mut index = 1usize;
    loop {
        let candidate = format!("{prefix}{index}");
        if !project.board.components.contains_key(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

pub(super) fn primitive_model(kind: SketchSpiceKind) -> &'static str {
    match kind {
        SketchSpiceKind::Resistor => "generic.analog.resistor",
        SketchSpiceKind::Capacitor => "generic.analog.capacitor",
        SketchSpiceKind::Inductor => "generic.analog.inductor",
        SketchSpiceKind::DcVoltageSource => "generic.analog.dc_voltage_source",
        SketchSpiceKind::PulseVoltageSource => "generic.analog.pulse_voltage_source",
        SketchSpiceKind::DcCurrentSource => "generic.analog.dc_current_source",
        SketchSpiceKind::PulseCurrentSource => "generic.analog.pulse_current_source",
    }
}

fn insert_primitive_component_yaml(
    yaml: &mut serde_yaml_ng::Value,
    component_id: &str,
    kind: SketchSpiceKind,
    value: f64,
    pin_nets: [PrimitivePinNet<'_>; 2],
) -> Result<()> {
    let components = ensure_board_child_mapping_mut(yaml, "components")?;
    let mut component = serde_yaml_ng::Mapping::new();
    component.insert(
        key("model"),
        serde_yaml_ng::Value::String(primitive_model(kind).to_string()),
    );
    let mut pins = serde_yaml_ng::Mapping::new();
    for pin_net in pin_nets {
        pins.insert(
            key(pin_net.pin),
            serde_yaml_ng::Value::String(pin_net.net.to_string()),
        );
    }
    component.insert(key("pins"), serde_yaml_ng::Value::Mapping(pins));
    let mut spice_draft = draft_from_existing(component_id, None, kind);
    spice_draft.value = value;
    component.insert(
        key("spice"),
        serde_yaml_ng::Value::Mapping(primitive_spice_mapping(&spice_draft)?),
    );
    components.insert(key(component_id), serde_yaml_ng::Value::Mapping(component));
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct PrimitivePinNet<'a> {
    pin: &'a str,
    net: &'a str,
}

fn primitive_spice_mapping(
    draft: &super::sketch_spice::SketchSpiceDraft,
) -> Result<serde_yaml_ng::Mapping> {
    let mut mapping = serde_yaml_ng::Mapping::new();
    mapping.insert(
        key("primitive"),
        serde_yaml_ng::Value::String(draft.kind.label().to_string()),
    );
    match draft.kind {
        SketchSpiceKind::Resistor => insert_number(&mut mapping, "value_ohm", draft.value)?,
        SketchSpiceKind::Capacitor => insert_number(&mut mapping, "value_f", draft.value)?,
        SketchSpiceKind::Inductor => insert_number(&mut mapping, "value_h", draft.value)?,
        SketchSpiceKind::DcVoltageSource => insert_number(&mut mapping, "dc_v", draft.value)?,
        SketchSpiceKind::DcCurrentSource => insert_number(&mut mapping, "dc_a", draft.value)?,
        SketchSpiceKind::PulseVoltageSource => {
            mapping.insert(key("pulse"), pulse_mapping_value(&draft.pulse, true)?);
        }
        SketchSpiceKind::PulseCurrentSource => {
            mapping.insert(
                key("current_pulse"),
                pulse_mapping_value(&draft.pulse, false)?,
            );
        }
    }
    Ok(mapping)
}

fn insert_primitive_net(yaml: &mut serde_yaml_ng::Value, net_id: &str, kind: &str) -> Result<()> {
    let nets = ensure_board_child_mapping_mut(yaml, "nets")?;
    let mut net = serde_yaml_ng::Mapping::new();
    net.insert(key("kind"), serde_yaml_ng::Value::String(kind.to_string()));
    nets.insert(key(net_id), serde_yaml_ng::Value::Mapping(net));
    Ok(())
}

fn insert_primitive_position(
    yaml: &mut serde_yaml_ng::Value,
    component_id: &str,
    x: f64,
    y: f64,
) -> Result<()> {
    validate_coordinate(x, "x")?;
    validate_coordinate(y, "y")?;
    let project = yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let board = project
        .get_mut(key("board"))
        .context("Board IR project is missing board.")?
        .as_mapping_mut()
        .context("Board IR field board must be an object.")?;
    let schematic = ensure_child_mapping_mut(board, "schematic", "schematic")?;
    let positions = ensure_child_mapping_mut(schematic, "node_positions", "node positions")?;
    let mut position = serde_yaml_ng::Mapping::new();
    position.insert(key("x"), serde_yaml_ng::to_value(x)?);
    position.insert(key("y"), serde_yaml_ng::to_value(y)?);
    positions.insert(
        key(&format!("component:{component_id}")),
        serde_yaml_ng::Value::Mapping(position),
    );
    Ok(())
}

fn insert_primitive_style(
    yaml: &mut serde_yaml_ng::Value,
    component_id: &str,
    style: SketchNodeStyle,
) -> Result<()> {
    if style == SketchNodeStyle::default() {
        return Ok(());
    }
    let project = yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let board = project
        .get_mut(key("board"))
        .context("Board IR project is missing board.")?
        .as_mapping_mut()
        .context("Board IR field board must be an object.")?;
    let schematic = ensure_child_mapping_mut(board, "schematic", "schematic")?;
    let styles = ensure_child_mapping_mut(schematic, "node_styles", "node styles")?;
    styles.insert(
        key(&format!("component:{component_id}")),
        serde_yaml_ng::Value::Mapping(schematic_node_style_mapping(style)?),
    );
    Ok(())
}

fn unique_net_id(yaml: &mut serde_yaml_ng::Value, requested: &str) -> Result<String> {
    let base = sanitized_net_base(requested)?;
    let nets = ensure_board_child_mapping_mut(yaml, "nets")?;
    let mut candidate = base.clone();
    let mut suffix = 2usize;
    while nets.contains_key(key(&candidate)) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}

fn sanitized_net_base(requested: &str) -> Result<String> {
    let mut sanitized = String::new();
    for character in requested.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
            sanitized.push(character.to_ascii_lowercase());
        } else if !sanitized.ends_with('_') {
            sanitized.push('_');
        }
    }
    let sanitized = sanitized.trim_matches('_');
    validated_graph_id(sanitized, "net").map(str::to_string)
}

fn first_pin_net_kind(kind: SketchSpiceKind) -> &'static str {
    if kind.is_passive() {
        "digital_or_analog"
    } else {
        "power"
    }
}

fn second_pin_net_kind(kind: SketchSpiceKind) -> &'static str {
    if kind.is_passive() {
        "digital_or_analog"
    } else {
        "ground"
    }
}

fn pulse_mapping_value(pulse: &SketchSpicePulse, voltage: bool) -> Result<serde_yaml_ng::Value> {
    let mut mapping = serde_yaml_ng::Mapping::new();
    if voltage {
        insert_number(&mut mapping, "initial_v", pulse.initial)?;
        insert_number(&mut mapping, "pulsed_v", pulse.pulsed)?;
    } else {
        insert_number(&mut mapping, "initial_a", pulse.initial)?;
        insert_number(&mut mapping, "pulsed_a", pulse.pulsed)?;
    }
    insert_number(&mut mapping, "delay_us", pulse.delay_us)?;
    insert_number(&mut mapping, "rise_us", pulse.rise_us)?;
    insert_number(&mut mapping, "fall_us", pulse.fall_us)?;
    insert_number(&mut mapping, "width_us", pulse.width_us)?;
    insert_number(&mut mapping, "period_us", pulse.period_us)?;
    Ok(serde_yaml_ng::Value::Mapping(mapping))
}

fn validate_primitive_value(kind: SketchSpiceKind, value: f64) -> Result<()> {
    if matches!(
        kind,
        SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource
    ) {
        return Ok(());
    }
    if !value.is_finite() {
        anyhow::bail!("Primitive value must be finite.");
    }
    if kind.is_passive() && value <= 0.0 {
        anyhow::bail!("Passive primitive values must be greater than zero.");
    }
    Ok(())
}

fn validate_coordinate(value: f64, axis: &str) -> Result<()> {
    if !value.is_finite() {
        anyhow::bail!("Schematic {axis} position must be finite.");
    }
    Ok(())
}

fn ensure_child_mapping_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    field: &str,
    label: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let child_key = key(field);
    if !mapping.contains_key(&child_key) {
        mapping.insert(
            child_key.clone(),
            serde_yaml_ng::Value::Mapping(Default::default()),
        );
    }
    mapping
        .get_mut(&child_key)
        .expect("field was inserted when absent")
        .as_mapping_mut()
        .with_context(|| format!("Board IR {label} must be an object."))
}

fn insert_number(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: f64) -> Result<()> {
    if !value.is_finite() {
        anyhow::bail!("SPICE value {name} must be finite.");
    }
    mapping.insert(key(name), serde_yaml_ng::to_value(value)?);
    Ok(())
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

pub(super) fn default_primitive_value(kind: SketchSpiceKind) -> f64 {
    default_value(kind)
}

impl CircuitCiApp {
    pub(super) fn sketch_primitive_palette(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Primitive Palette")
            .default_open(true)
            .show(ui, |ui| {
                let previous_kind = self.sketch_palette_kind;
                egui::ComboBox::from_label("Primitive")
                    .selected_text(self.sketch_palette_kind.label())
                    .show_ui(ui, |ui| {
                        for kind in SketchSpiceKind::ALL {
                            ui.selectable_value(&mut self.sketch_palette_kind, kind, kind.label());
                        }
                    });
                if self.sketch_palette_kind != previous_kind {
                    self.sketch_palette_value = default_primitive_value(self.sketch_palette_kind);
                    self.sketch_palette_component_id =
                        next_primitive_component_id(&self.project_yaml, self.sketch_palette_kind);
                }

                ui.horizontal(|ui| {
                    ui.label("ID");
                    ui.text_edit_singleline(&mut self.sketch_palette_component_id);
                    if ui.button("Next").clicked() {
                        self.sketch_palette_component_id =
                            next_primitive_component_id(&self.project_yaml, self.sketch_palette_kind);
                    }
                });

                if palette_uses_scalar_value(self.sketch_palette_kind) {
                    ui.horizontal(|ui| {
                        ui.label(value_label(self.sketch_palette_kind));
                        ui.add(
                            egui::DragValue::new(&mut self.sketch_palette_value)
                                .speed(value_drag_speed(self.sketch_palette_kind))
                                .range(value_range(self.sketch_palette_kind))
                                .suffix(value_suffix(self.sketch_palette_kind)),
                        );
                    });
                } else {
                    ui.label(pulse_default_label(self.sketch_palette_kind));
                }

                let can_insert = !self.project_yaml.trim().is_empty();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(can_insert, egui::Button::new("Insert At View"))
                        .clicked()
                    {
                        self.apply_insert_sketch_primitive_at_view();
                    }
                    let place_label = if self.sketch_palette_place_armed {
                        "Click Canvas To Place"
                    } else {
                        "Place On Canvas"
                    };
                    let place_response = ui.add_enabled(
                        can_insert,
                        egui::Button::new(place_label).sense(egui::Sense::click_and_drag()),
                    );
                    if place_response.drag_started() {
                        self.sketch_palette_place_armed = true;
                        self.sketch_library_place_armed = false;
                        self.sketch_net_label_place_armed = false;
                        self.status = format!(
                            "Drag to blank schematic space to place {}.",
                            self.sketch_palette_kind.label()
                        );
                    } else if place_response.clicked() {
                        self.sketch_palette_place_armed = !self.sketch_palette_place_armed;
                        if self.sketch_palette_place_armed {
                            self.sketch_library_place_armed = false;
                            self.sketch_net_label_place_armed = false;
                        }
                        if self.sketch_palette_place_armed {
                            self.status = format!(
                                "Click blank schematic space to place {}.",
                                self.sketch_palette_kind.label()
                            );
                        }
                    }
                    place_response.on_hover_text(format!(
                        "Click to arm placement, or drag {} onto blank schematic space.",
                        self.sketch_palette_kind.label()
                    ));
                    if self.sketch_palette_place_armed && ui.button("Cancel").clicked() {
                        self.sketch_palette_place_armed = false;
                        self.status = "Primitive placement canceled.".to_string();
                    }
                });
                ui.small("Creates a Board IR component, two editable pin nets, SPICE evidence, and a schematic position.");
            });
    }

    fn apply_insert_sketch_primitive_at_view(&mut self) {
        let canvas = self.sketch_last_canvas_rect.unwrap_or_else(|| {
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(960.0, 640.0))
        });
        self.apply_insert_sketch_primitive_at(canvas, canvas.center());
    }

    pub(super) fn apply_insert_sketch_primitive_at(
        &mut self,
        canvas: egui::Rect,
        target: egui::Pos2,
    ) {
        if self.sketch_palette_component_id.trim().is_empty() {
            self.sketch_palette_component_id =
                next_primitive_component_id(&self.project_yaml, self.sketch_palette_kind);
        }
        let component_id = self.sketch_palette_component_id.trim().to_string();
        let (x, y) = self.palette_insert_position(canvas, target);
        let draft = SketchPrimitiveInsertDraft {
            component_id: component_id.clone(),
            kind: self.sketch_palette_kind,
            value: self.sketch_palette_value,
            x,
            y,
            style: self.placement_node_style(),
        };
        match insert_primitive_component(&self.project_yaml, &draft) {
            Ok(updated) => {
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Primitive {component_id} inserted."),
                );
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    component_id.clone(),
                )));
                self.sketch_palette_component_id =
                    next_primitive_component_id(&self.project_yaml, self.sketch_palette_kind);
                self.sketch_palette_place_armed = false;
            }
            Err(error) => self.record_error(error),
        }
    }

    fn palette_insert_position(&self, canvas: egui::Rect, target: egui::Pos2) -> (f64, f64) {
        let node_rect = egui::Rect::from_center_size(target, egui::vec2(180.0, 92.0));
        persisted_node_position_from_screen_with_snap(
            canvas,
            target,
            node_rect,
            self.sketch_viewport(),
            self.sketch_snap_enabled,
            self.sketch_grid_step,
        )
    }
}

fn palette_uses_scalar_value(kind: SketchSpiceKind) -> bool {
    !matches!(
        kind,
        SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource
    )
}

fn value_label(kind: SketchSpiceKind) -> &'static str {
    match kind {
        SketchSpiceKind::Resistor => "Resistance",
        SketchSpiceKind::Capacitor => "Capacitance",
        SketchSpiceKind::Inductor => "Inductance",
        SketchSpiceKind::DcVoltageSource => "Voltage",
        SketchSpiceKind::DcCurrentSource => "Current",
        SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource => "Pulse",
    }
}

fn value_suffix(kind: SketchSpiceKind) -> &'static str {
    match kind {
        SketchSpiceKind::Resistor => " ohm",
        SketchSpiceKind::Capacitor => " F",
        SketchSpiceKind::Inductor => " H",
        SketchSpiceKind::DcVoltageSource => " V",
        SketchSpiceKind::DcCurrentSource => " A",
        SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource => "",
    }
}

fn value_drag_speed(kind: SketchSpiceKind) -> f64 {
    match kind {
        SketchSpiceKind::Resistor => 10.0,
        SketchSpiceKind::Capacitor => 1e-7,
        SketchSpiceKind::Inductor => 1e-7,
        SketchSpiceKind::DcVoltageSource => 0.1,
        SketchSpiceKind::DcCurrentSource => 0.01,
        SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource => 1.0,
    }
}

fn value_range(kind: SketchSpiceKind) -> std::ops::RangeInclusive<f64> {
    match kind {
        SketchSpiceKind::Resistor => 1e-6..=1e12,
        SketchSpiceKind::Capacitor => 1e-15..=1e3,
        SketchSpiceKind::Inductor => 1e-12..=1e6,
        SketchSpiceKind::DcVoltageSource => -1e6..=1e6,
        SketchSpiceKind::DcCurrentSource => -1e6..=1e6,
        SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource => 0.0..=0.0,
    }
}

fn pulse_default_label(kind: SketchSpiceKind) -> &'static str {
    match kind {
        SketchSpiceKind::PulseVoltageSource => "Default pulse: 0 V to 3.3 V",
        SketchSpiceKind::PulseCurrentSource => "Default pulse: 0 A to 0.1 A",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SketchPrimitiveInsertDraft, insert_primitive_component, next_primitive_component_id,
    };
    use crate::gui::CircuitCiApp;
    use crate::gui::sketch::SketchSelection;
    use crate::gui::sketch::load_project_snapshot_from_yaml;
    use crate::gui::sketch_spice::SketchSpiceKind;
    use eframe::egui;

    fn project_yaml() -> &'static str {
        "project:
  name: palette_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins:
        A: r1_a
        B: r1_b
  nets:
    r1_a: { kind: digital_or_analog }
    r1_b: { kind: digital_or_analog }
"
    }

    #[test]
    fn inserts_passive_with_pins_nets_spice_and_position() {
        let edited = insert_primitive_component(
            project_yaml(),
            &SketchPrimitiveInsertDraft {
                component_id: "C1".to_string(),
                kind: SketchSpiceKind::Capacitor,
                value: 1e-6,
                x: 96.0,
                y: 128.0,
                style: Default::default(),
            },
        )
        .unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        let component = snapshot
            .components_detail
            .iter()
            .find(|component| component.id == "C1")
            .unwrap();
        assert_eq!(component.model, "generic.analog.capacitor");
        assert_eq!(component.pins.len(), 2);
        assert_eq!(component.position.unwrap().x, 96.0);
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let spice = project.board.components["C1"].spice.as_ref().unwrap();
        assert_eq!(spice.value_f, Some(1e-6));
        assert!(edited.contains("primitive: capacitor"));
    }

    #[test]
    fn inserts_source_with_power_and_ground_pin_nets() {
        let edited = insert_primitive_component(
            project_yaml(),
            &SketchPrimitiveInsertDraft {
                component_id: "V1".to_string(),
                kind: SketchSpiceKind::DcVoltageSource,
                value: 5.0,
                x: 10.0,
                y: 20.0,
                style: Default::default(),
            },
        )
        .unwrap();
        assert!(edited.contains("model: generic.analog.dc_voltage_source"));
        assert!(edited.contains("P: v1_p"));
        assert!(edited.contains("N: v1_n"));
        assert!(edited.contains("v1_p:\n      kind: power"));
        assert!(edited.contains("v1_n:\n      kind: ground"));
        assert!(edited.contains("dc_v: 5.0"));
    }

    #[test]
    fn inserts_primitive_with_requested_schematic_rotation() {
        let edited = insert_primitive_component(
            project_yaml(),
            &SketchPrimitiveInsertDraft {
                component_id: "L1".to_string(),
                kind: SketchSpiceKind::Inductor,
                value: 1e-3,
                x: 10.0,
                y: 20.0,
                style: crate::gui::sketch::SketchNodeStyle {
                    rotation_deg: 90,
                    ..Default::default()
                },
            },
        )
        .unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        let component = snapshot
            .components_detail
            .iter()
            .find(|component| component.id == "L1")
            .unwrap();

        assert_eq!(component.style.rotation_deg, 90);
        assert!(edited.contains("node_styles:"));
        assert!(edited.contains("component:L1:"));
    }

    #[test]
    fn next_id_skips_existing_prefix_matches() {
        assert_eq!(
            next_primitive_component_id(project_yaml(), SketchSpiceKind::Resistor),
            "R2"
        );
        assert_eq!(
            next_primitive_component_id(project_yaml(), SketchSpiceKind::Inductor),
            "L1"
        );
    }

    #[test]
    fn rejects_duplicate_component_id() {
        let error = insert_primitive_component(
            project_yaml(),
            &SketchPrimitiveInsertDraft {
                component_id: "R1".to_string(),
                kind: SketchSpiceKind::Resistor,
                value: 1000.0,
                x: 0.0,
                y: 0.0,
                style: Default::default(),
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    fn app_canvas_placement_inserts_at_clicked_position_and_disarms() {
        let canvas = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 320.0));
        let mut app = CircuitCiApp {
            project_yaml: project_yaml().to_string(),
            sketch_palette_kind: SketchSpiceKind::Resistor,
            sketch_palette_component_id: "R2".to_string(),
            sketch_palette_value: 2200.0,
            sketch_palette_place_armed: true,
            sketch_placement_rotation_deg: 90,
            sketch_snap_enabled: false,
            ..Default::default()
        };

        app.apply_insert_sketch_primitive_at(canvas, egui::pos2(300.0, 200.0));

        let snapshot = load_project_snapshot_from_yaml(&app.project_yaml).unwrap();
        let component = snapshot
            .components_detail
            .iter()
            .find(|component| component.id == "R2")
            .unwrap();
        let position = component.position.unwrap();
        assert_eq!(position.x, 210.0);
        assert_eq!(position.y, 154.0);
        assert_eq!(component.style.rotation_deg, 90);
        assert!(!app.sketch_palette_place_armed);
        assert_eq!(
            app.selected_sketch_item,
            Some(SketchSelection::Component("R2".to_string()))
        );
    }
}
