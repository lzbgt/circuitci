use anyhow::{Context, Result};
use eframe::egui;

use super::CircuitCiApp;
use super::sketch::{
    ProjectSnapshot, SketchComponent, SketchGraph, SketchPosition, SketchSelection, SketchViewport,
    encode_edited_project_yaml, screen_wire_route_point_from_persisted, validated_graph_id,
};
use super::sketch_render::with_opacity;
use super::sketch_spice::SketchSpiceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum SketchComponentLabelKind {
    Reference,
    Value,
}

impl SketchComponentLabelKind {
    fn field(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::Value => "value",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Reference => "Reference",
            Self::Value => "Value",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct SketchComponentLabelBadge {
    pub(super) component_id: String,
    pub(super) kind: SketchComponentLabelKind,
    pub(super) text: String,
    pub(super) rect: egui::Rect,
}

impl CircuitCiApp {
    pub(super) fn apply_move_schematic_component_label_to(
        &mut self,
        canvas: egui::Rect,
        viewport: SketchViewport,
        badge: &SketchComponentLabelBadge,
        center: egui::Pos2,
    ) {
        let position = persisted_component_label_position(
            canvas,
            center,
            viewport,
            self.sketch_snap_enabled,
            self.sketch_grid_step,
        );
        match edit_schematic_component_label_position(
            &self.project_yaml,
            &badge.component_id,
            badge.kind,
            position,
        ) {
            Ok(updated) => {
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Component {} {} label moved.",
                        badge.component_id,
                        badge.kind.label()
                    ),
                );
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    badge.component_id.clone(),
                )));
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_reset_schematic_component_label(
        &mut self,
        badge: &SketchComponentLabelBadge,
    ) {
        match remove_schematic_component_label_position(
            &self.project_yaml,
            &badge.component_id,
            badge.kind,
        ) {
            Ok(updated) => {
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Component {} {} label reset.",
                        badge.component_id,
                        badge.kind.label()
                    ),
                );
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    badge.component_id.clone(),
                )));
            }
            Err(error) => self.record_error(error),
        }
    }
}

pub(super) fn layout_component_label_badges(
    snapshot: &ProjectSnapshot,
    graph: &SketchGraph,
    canvas: egui::Rect,
    viewport: SketchViewport,
) -> Vec<SketchComponentLabelBadge> {
    let mut badges = Vec::new();
    for component in &snapshot.components_detail {
        let Some(node) = graph.nodes.iter().find(
            |node| matches!(&node.selection, SketchSelection::Component(id) if id == &component.id),
        ) else {
            continue;
        };
        let label_positions = snapshot.component_labels.get(&component.id);
        let reference_center = label_positions
            .and_then(|labels| labels.reference)
            .map(|position| screen_position(canvas, viewport, position))
            .unwrap_or_else(|| node.rect.left_top() + egui::vec2(34.0, -12.0));
        badges.push(label_badge(
            &component.id,
            SketchComponentLabelKind::Reference,
            component.id.clone(),
            reference_center,
        ));

        if let Some(value) = component_value_label(component) {
            let value_center = label_positions
                .and_then(|labels| labels.value)
                .map(|position| screen_position(canvas, viewport, position))
                .unwrap_or_else(|| node.rect.left_bottom() + egui::vec2(42.0, 14.0));
            badges.push(label_badge(
                &component.id,
                SketchComponentLabelKind::Value,
                value,
                value_center,
            ));
        }
    }
    badges
}

pub(super) fn draw_component_label_badge(
    painter: &egui::Painter,
    badge: &SketchComponentLabelBadge,
    hovered: bool,
    selected: bool,
    opacity: f32,
) {
    let fill = match badge.kind {
        SketchComponentLabelKind::Reference => egui::Color32::from_rgb(34, 48, 66),
        SketchComponentLabelKind::Value => egui::Color32::from_rgb(55, 50, 32),
    };
    let stroke = if selected {
        egui::Stroke::new(2.0, egui::Color32::from_rgb(93, 185, 255))
    } else if hovered {
        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 196, 87))
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 160, 170))
    };
    painter.rect_filled(badge.rect, 3.0, with_opacity(fill, opacity));
    painter.rect_stroke(
        badge.rect,
        3.0,
        egui::Stroke::new(stroke.width, with_opacity(stroke.color, opacity)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        badge.rect.center(),
        egui::Align2::CENTER_CENTER,
        &badge.text,
        egui::FontId::monospace(11.5),
        with_opacity(egui::Color32::WHITE, opacity),
    );
}

pub(super) fn hit_test_component_label_badge(
    badges: &[SketchComponentLabelBadge],
    position: egui::Pos2,
) -> Option<&SketchComponentLabelBadge> {
    badges
        .iter()
        .rev()
        .find(|badge| badge.rect.contains(position))
}

pub(super) fn component_label_tooltip(ui: &mut egui::Ui, badge: &SketchComponentLabelBadge) {
    ui.strong(format!(
        "Component {} {}",
        badge.component_id,
        badge.kind.label()
    ));
    ui.label(&badge.text);
    ui.separator();
    ui.label("Click to select the component.");
    ui.label("Double-click to edit this label's underlying Board IR value.");
    ui.label("Drag to reposition this schematic label.");
}

pub(super) fn edit_schematic_component_label_position(
    text: &str,
    component_id: &str,
    kind: SketchComponentLabelKind,
    position: SketchPosition,
) -> Result<String> {
    let component_id = validated_graph_id(component_id, "component")?;
    validate_position(position)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.components.contains_key(component_id) {
        anyhow::bail!("Board IR component {component_id} was not found.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let labels = ensure_component_label_mapping(&mut yaml, component_id)?;
    labels.insert(key(kind.field()), position_value(position)?);
    encode_edited_project_yaml(yaml)
}

pub(super) fn remove_schematic_component_label_position(
    text: &str,
    component_id: &str,
    kind: SketchComponentLabelKind,
) -> Result<String> {
    let component_id = validated_graph_id(component_id, "component")?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.components.contains_key(component_id) {
        anyhow::bail!("Board IR component {component_id} was not found.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let Some(labels) = yaml
        .as_mapping_mut()
        .and_then(|project| project.get_mut(key("board")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|board| board.get_mut(key("schematic")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|schematic| schematic.get_mut(key("component_labels")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|labels| labels.get_mut(key(component_id)))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
    else {
        return encode_edited_project_yaml(yaml);
    };
    labels.remove(key(kind.field()));
    encode_edited_project_yaml(yaml)
}

pub(super) fn rename_component_labels(
    yaml: &mut serde_yaml_ng::Value,
    old_component: &str,
    new_component: &str,
) {
    let Some(labels) = yaml
        .as_mapping_mut()
        .and_then(|project| project.get_mut(key("board")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|board| board.get_mut(key("schematic")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|schematic| schematic.get_mut(key("component_labels")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
    else {
        return;
    };
    if let Some(value) = labels.remove(key(old_component)) {
        labels.insert(key(new_component), value);
    }
}

fn persisted_component_label_position(
    canvas: egui::Rect,
    screen_position: egui::Pos2,
    viewport: SketchViewport,
    snap_enabled: bool,
    grid_step: f32,
) -> SketchPosition {
    let (x, y) = super::sketch::persisted_wire_route_point_from_screen_with_snap(
        canvas,
        screen_position,
        viewport,
        snap_enabled,
        grid_step,
    );
    SketchPosition { x, y }
}

fn screen_position(
    canvas: egui::Rect,
    viewport: SketchViewport,
    position: SketchPosition,
) -> egui::Pos2 {
    screen_wire_route_point_from_persisted(canvas, (position.x, position.y), viewport)
}

fn label_badge(
    component_id: &str,
    kind: SketchComponentLabelKind,
    text: String,
    center: egui::Pos2,
) -> SketchComponentLabelBadge {
    let width = (text.chars().count() as f32 * 7.0 + 18.0).clamp(44.0, 180.0);
    SketchComponentLabelBadge {
        component_id: component_id.to_string(),
        kind,
        text,
        rect: egui::Rect::from_center_size(center, egui::vec2(width, 22.0)),
    }
}

fn component_value_label(component: &SketchComponent) -> Option<String> {
    let spice = component.spice.as_ref()?;
    match spice.kind {
        SketchSpiceKind::Resistor => Some(format_engineering(spice.value, "ohm")),
        SketchSpiceKind::Capacitor => Some(format_engineering(spice.value, "F")),
        SketchSpiceKind::Inductor => Some(format_engineering(spice.value, "H")),
        SketchSpiceKind::DcVoltageSource => Some(format_engineering(spice.value, "V")),
        SketchSpiceKind::DcCurrentSource => Some(format_engineering(spice.value, "A")),
        SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource => None,
    }
}

fn format_engineering(value: f64, unit: &str) -> String {
    if !value.is_finite() {
        return format!("{value} {unit}");
    }
    let abs = value.abs();
    let (scale, suffix) = if abs >= 1e6 {
        (1e6, "M")
    } else if abs >= 1e3 {
        (1e3, "k")
    } else if abs >= 1.0 || abs == 0.0 {
        (1.0, "")
    } else if abs >= 1e-3 {
        (1e-3, "m")
    } else if abs >= 1e-6 {
        (1e-6, "u")
    } else if abs >= 1e-9 {
        (1e-9, "n")
    } else {
        (1e-12, "p")
    };
    let scaled = value / scale;
    let text = if scaled.abs() >= 100.0 || scaled.fract().abs() < 1e-9 {
        format!("{scaled:.0}")
    } else if scaled.abs() >= 10.0 {
        format!("{scaled:.1}")
    } else {
        format!("{scaled:.2}")
    };
    format!("{} {}{}", trim_number(&text), suffix, unit)
}

fn trim_number(text: &str) -> &str {
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() { "0" } else { trimmed }
}

fn validate_position(position: SketchPosition) -> Result<()> {
    if !position.x.is_finite() || !position.y.is_finite() {
        anyhow::bail!("Schematic component label position must be finite.");
    }
    Ok(())
}

fn ensure_component_label_mapping<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    component_id: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let board = yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?
        .get_mut(key("board"))
        .context("Board IR project is missing board.")?
        .as_mapping_mut()
        .context("Board IR field board must be an object.")?;
    let schematic_key = key("schematic");
    if !board.contains_key(&schematic_key) {
        board.insert(
            schematic_key.clone(),
            serde_yaml_ng::Value::Mapping(Default::default()),
        );
    }
    let schematic = board
        .get_mut(&schematic_key)
        .expect("schematic exists")
        .as_mapping_mut()
        .context("Board IR board.schematic must be an object.")?;
    let component_labels_key = key("component_labels");
    if !schematic.contains_key(&component_labels_key) {
        schematic.insert(
            component_labels_key.clone(),
            serde_yaml_ng::Value::Mapping(Default::default()),
        );
    }
    let component_labels = schematic
        .get_mut(&component_labels_key)
        .expect("component labels exists")
        .as_mapping_mut()
        .context("Board IR board.schematic.component_labels must be an object.")?;
    let component_key = key(component_id);
    if !component_labels.contains_key(&component_key) {
        component_labels.insert(
            component_key.clone(),
            serde_yaml_ng::Value::Mapping(Default::default()),
        );
    }
    component_labels
        .get_mut(&component_key)
        .expect("component label entry exists")
        .as_mapping_mut()
        .with_context(|| {
            format!("Board IR schematic component_labels.{component_id} must be an object.")
        })
}

fn position_value(position: SketchPosition) -> Result<serde_yaml_ng::Value> {
    let mut mapping = serde_yaml_ng::Mapping::new();
    mapping.insert(key("x"), serde_yaml_ng::to_value(position.x)?);
    mapping.insert(key("y"), serde_yaml_ng::to_value(position.y)?);
    Ok(serde_yaml_ng::Value::Mapping(mapping))
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        SketchComponentLabelKind, edit_schematic_component_label_position, format_engineering,
        remove_schematic_component_label_position,
    };
    use crate::gui::sketch::{SketchPosition, load_project_snapshot_from_yaml};

    fn project_yaml() -> &'static str {
        "project:
  name: component_label_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      spice: { primitive: resistor, value_ohm: 1000 }
      pins: { A: in, B: out }
  nets:
    in: { kind: digital_or_analog }
    out: { kind: digital_or_analog }
"
    }

    #[test]
    fn formats_scalar_component_values_for_schematic_labels() {
        assert_eq!(format_engineering(1000.0, "ohm"), "1 kohm");
        assert_eq!(format_engineering(4.7e-6, "F"), "4.7 uF");
        assert_eq!(format_engineering(0.0, "V"), "0 V");
    }

    #[test]
    fn edits_component_label_position_metadata() {
        let edited = edit_schematic_component_label_position(
            project_yaml(),
            "R1",
            SketchComponentLabelKind::Value,
            SketchPosition { x: 120.0, y: 64.0 },
        )
        .unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        let labels = snapshot.component_labels.get("R1").unwrap();

        assert_eq!(labels.value.unwrap().x, 120.0);
        assert_eq!(labels.value.unwrap().y, 64.0);
        assert!(labels.reference.is_none());
    }

    #[test]
    fn removes_component_label_position_metadata() {
        let edited = edit_schematic_component_label_position(
            project_yaml(),
            "R1",
            SketchComponentLabelKind::Reference,
            SketchPosition { x: 8.0, y: 16.0 },
        )
        .unwrap();
        let edited = remove_schematic_component_label_position(
            &edited,
            "R1",
            SketchComponentLabelKind::Reference,
        )
        .unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();

        assert!(
            snapshot
                .component_labels
                .get("R1")
                .is_none_or(|labels| labels.reference.is_none())
        );
    }
}
