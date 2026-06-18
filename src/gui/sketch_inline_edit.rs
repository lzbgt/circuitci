use anyhow::{Context, Result};
use eframe::egui;

use super::CircuitCiApp;
use super::sketch::{ProjectSnapshot, SketchGraph, SketchSelection};
use super::sketch_rename::rename_component;
use super::sketch_spice::{
    SketchComponentSpice, SketchSpiceKind, draft_from_existing, replace_component_spice,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum SketchComponentInlineEditMode {
    ComponentId,
    ScalarValue,
}

#[derive(Debug, Clone)]
pub(super) struct SketchComponentInlineEdit {
    component_id: String,
    mode: SketchComponentInlineEditMode,
    draft: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineEditAction {
    Apply,
    Cancel,
}

impl CircuitCiApp {
    pub(super) fn begin_component_default_inline_edit(
        &mut self,
        snapshot: &ProjectSnapshot,
        component_id: &str,
    ) {
        if component_scalar_spice(snapshot, component_id).is_some() {
            self.begin_component_value_inline_edit(snapshot, component_id);
        } else {
            self.begin_component_id_inline_edit(component_id);
        }
    }

    pub(super) fn begin_component_id_inline_edit(&mut self, component_id: &str) {
        self.sketch_component_inline_edit = Some(SketchComponentInlineEdit {
            component_id: component_id.to_string(),
            mode: SketchComponentInlineEditMode::ComponentId,
            draft: component_id.to_string(),
        });
        self.component_rename_id = component_id.to_string();
        self.status = format!("Editing component {component_id} ID on schematic.");
    }

    pub(super) fn begin_component_value_inline_edit(
        &mut self,
        snapshot: &ProjectSnapshot,
        component_id: &str,
    ) {
        let Some(spice) = component_scalar_spice(snapshot, component_id) else {
            self.status = format!(
                "Component {component_id} has no single-value SPICE primitive for inline editing."
            );
            return;
        };
        self.sketch_component_inline_edit = Some(SketchComponentInlineEdit {
            component_id: component_id.to_string(),
            mode: SketchComponentInlineEditMode::ScalarValue,
            draft: format_scalar_value(spice.value),
        });
        self.status = format!("Editing component {component_id} value on schematic.");
    }

    pub(super) fn sketch_component_inline_editor(
        &mut self,
        ui: &mut egui::Ui,
        graph: &SketchGraph,
    ) {
        let Some(edit) = self.sketch_component_inline_edit.clone() else {
            return;
        };
        let Some(node) = graph.nodes.iter().find(|node| {
            matches!(&node.selection, SketchSelection::Component(id) if id == &edit.component_id)
        }) else {
            self.sketch_component_inline_edit = None;
            return;
        };

        let mut draft = edit.draft.clone();
        let mut action = None;
        let title = match edit.mode {
            SketchComponentInlineEditMode::ComponentId => "Edit Component ID",
            SketchComponentInlineEditMode::ScalarValue => "Edit Component Value",
        };
        let label = match edit.mode {
            SketchComponentInlineEditMode::ComponentId => "ID",
            SketchComponentInlineEditMode::ScalarValue => "Value",
        };
        let editor_pos = node.rect.right_top() + egui::vec2(8.0, 0.0);
        egui::Area::new(egui::Id::new((
            "component_inline_edit",
            &edit.component_id,
            edit.mode,
        )))
        .order(egui::Order::Foreground)
        .fixed_pos(editor_pos)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(240.0);
                ui.strong(title);
                ui.horizontal(|ui| {
                    ui.label(label);
                    let response =
                        ui.add(egui::TextEdit::singleline(&mut draft).desired_width(150.0));
                    if response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        action = Some(InlineEditAction::Apply);
                    }
                });
                if edit.mode == SketchComponentInlineEditMode::ScalarValue {
                    ui.label("Supports engineering suffixes such as 4.7k, 100n, 1u, 10m, 2M.");
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!draft.trim().is_empty(), egui::Button::new("Apply"))
                        .clicked()
                    {
                        action = Some(InlineEditAction::Apply);
                    }
                    if ui.button("Cancel").clicked() {
                        action = Some(InlineEditAction::Cancel);
                    }
                });
            });
        });

        if let Some(edit) = &mut self.sketch_component_inline_edit {
            edit.draft = draft;
        }
        match action {
            Some(InlineEditAction::Apply) => self.apply_component_inline_edit(),
            Some(InlineEditAction::Cancel) => {
                self.sketch_component_inline_edit = None;
                self.status = "Canceled schematic component edit.".to_string();
            }
            None => {}
        }
    }

    fn apply_component_inline_edit(&mut self) {
        let Some(edit) = self.sketch_component_inline_edit.clone() else {
            return;
        };
        let result = match edit.mode {
            SketchComponentInlineEditMode::ComponentId => {
                rename_component(&self.project_yaml, &edit.component_id, edit.draft.trim())
            }
            SketchComponentInlineEditMode::ScalarValue => {
                edit_component_inline_value(&self.project_yaml, &edit.component_id, &edit.draft)
            }
        };
        match result {
            Ok(updated) => {
                self.sketch_component_inline_edit = None;
                let selected_id = if edit.mode == SketchComponentInlineEditMode::ComponentId {
                    edit.draft.trim().to_string()
                } else {
                    edit.component_id.clone()
                };
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    selected_id.clone(),
                )));
                let message = match edit.mode {
                    SketchComponentInlineEditMode::ComponentId => {
                        format!("Component {} renamed to {selected_id}.", edit.component_id)
                    }
                    SketchComponentInlineEditMode::ScalarValue => {
                        format!("Component {} value updated.", edit.component_id)
                    }
                };
                self.apply_edited_project_yaml(updated, &message);
            }
            Err(error) => self.record_error(error),
        }
    }
}

pub(super) fn component_supports_inline_value(spice: Option<&SketchComponentSpice>) -> bool {
    spice.is_some_and(|spice| scalar_value_kind(spice.kind))
}

fn component_scalar_spice<'a>(
    snapshot: &'a ProjectSnapshot,
    component_id: &str,
) -> Option<&'a SketchComponentSpice> {
    snapshot
        .components_detail
        .iter()
        .find(|component| component.id == component_id)
        .and_then(|component| component.spice.as_ref())
        .filter(|spice| scalar_value_kind(spice.kind))
}

fn scalar_value_kind(kind: SketchSpiceKind) -> bool {
    matches!(
        kind,
        SketchSpiceKind::Resistor
            | SketchSpiceKind::Capacitor
            | SketchSpiceKind::Inductor
            | SketchSpiceKind::DcVoltageSource
            | SketchSpiceKind::DcCurrentSource
    )
}

fn edit_component_inline_value(text: &str, component_id: &str, value_text: &str) -> Result<String> {
    let value = parse_engineering_value(value_text)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let component = project
        .board
        .components
        .get(component_id)
        .with_context(|| format!("Component {component_id} was not found."))?;
    let spice = component
        .spice
        .as_ref()
        .with_context(|| format!("Component {component_id} has no SPICE evidence."))?;
    let existing = SketchComponentSpice::from_board(spice);
    if !scalar_value_kind(existing.kind) {
        anyhow::bail!(
            "Component {component_id} uses {} which needs the inspector multi-field editor.",
            existing.kind.label()
        );
    }
    let mut draft = draft_from_existing(component_id, Some(&existing), existing.kind);
    draft.value = value;
    replace_component_spice(text, &draft)
}

fn parse_engineering_value(input: &str) -> Result<f64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Value cannot be empty.");
    }
    if let Ok(value) = trimmed.parse::<f64>() {
        return finite_value(value);
    }

    let mut numeric_end = 0;
    let mut previous = '\0';
    for (index, character) in trimmed.char_indices() {
        let numeric = character.is_ascii_digit()
            || matches!(character, '+' | '-' | '.')
            || matches!(character, 'e' | 'E')
            || (matches!(character, '+' | '-') && matches!(previous, 'e' | 'E'));
        if numeric {
            numeric_end = index + character.len_utf8();
            previous = character;
        } else {
            break;
        }
    }
    let number = trimmed[..numeric_end]
        .trim()
        .parse::<f64>()
        .with_context(|| format!("Value {input:?} is not a number."))?;
    let suffix = trimmed[numeric_end..].trim();
    let multiplier = engineering_multiplier(suffix)?;
    finite_value(number * multiplier)
}

fn engineering_multiplier(suffix: &str) -> Result<f64> {
    if suffix.is_empty() {
        return Ok(1.0);
    }
    let lower = suffix.to_ascii_lowercase();
    let (multiplier, unit_suffix) = if lower.starts_with("meg") {
        (1e6, suffix.get(3..).unwrap_or_default())
    } else if let Some(rest) = suffix.strip_prefix('M') {
        (1e6, rest)
    } else if let Some(rest) = suffix
        .strip_prefix('k')
        .or_else(|| suffix.strip_prefix('K'))
    {
        (1e3, rest)
    } else if let Some(rest) = suffix.strip_prefix('m') {
        (1e-3, rest)
    } else if let Some(rest) = suffix
        .strip_prefix('u')
        .or_else(|| suffix.strip_prefix('µ'))
    {
        (1e-6, rest)
    } else if let Some(rest) = suffix.strip_prefix('n') {
        (1e-9, rest)
    } else if let Some(rest) = suffix.strip_prefix('p') {
        (1e-12, rest)
    } else {
        (1.0, suffix)
    };
    let unit = unit_suffix.trim().to_ascii_lowercase();
    if unit.is_empty()
        || matches!(
            unit.as_str(),
            "r" | "ohm" | "ohms" | "ω" | "f" | "h" | "v" | "a"
        )
    {
        Ok(multiplier)
    } else {
        anyhow::bail!("Unsupported value suffix {suffix:?}.");
    }
}

fn finite_value(value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        anyhow::bail!("Value must be finite.")
    }
}

fn format_scalar_value(value: f64) -> String {
    format!("{value}")
}

#[cfg(test)]
mod tests {
    use super::{edit_component_inline_value, parse_engineering_value};

    fn project_yaml() -> &'static str {
        "project:
  name: inline_component_edit_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      spice: { primitive: resistor, value_ohm: 1000 }
      pins: { A: rail, B: out }
    V1:
      model: generic.analog.dc_voltage_source
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
      pins: { P: rail, N: gnd }
  nets:
    rail: { kind: power }
    out: { kind: digital_or_analog }
    gnd: { kind: ground }
"
    }

    #[test]
    fn parses_engineering_value_suffixes() {
        assert_eq!(parse_engineering_value("4.7k").unwrap(), 4700.0);
        assert_close(parse_engineering_value("100nF").unwrap(), 100e-9);
        assert_eq!(parse_engineering_value("1u").unwrap(), 1e-6);
        assert_eq!(parse_engineering_value("10mA").unwrap(), 10e-3);
        assert_eq!(parse_engineering_value("2M").unwrap(), 2e6);
        assert_eq!(parse_engineering_value("3megohm").unwrap(), 3e6);
    }

    fn assert_close(left: f64, right: f64) {
        assert!(
            (left - right).abs() <= right.abs().max(1.0) * 1e-12,
            "{left} != {right}"
        );
    }

    #[test]
    fn inline_value_edit_updates_passive_spice_value() {
        let edited = edit_component_inline_value(project_yaml(), "R1", "4.7k").unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();

        assert_eq!(
            project
                .board
                .components
                .get("R1")
                .unwrap()
                .spice
                .as_ref()
                .unwrap()
                .value_ohm,
            Some(4700.0)
        );
    }

    #[test]
    fn inline_value_edit_updates_dc_source_value() {
        let edited = edit_component_inline_value(project_yaml(), "V1", "3.3V").unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();

        assert_eq!(
            project
                .board
                .components
                .get("V1")
                .unwrap()
                .spice
                .as_ref()
                .unwrap()
                .dc_v,
            Some(3.3)
        );
    }
}
