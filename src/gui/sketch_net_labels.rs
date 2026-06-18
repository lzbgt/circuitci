use anyhow::{Context, Result};
use eframe::egui;

use super::CircuitCiApp;
use super::sketch::{
    ProjectSnapshot, SketchNetLabelKind, SketchPosition, SketchViewport,
    encode_edited_project_yaml, persisted_wire_route_point_from_screen_with_snap,
    screen_wire_route_point_from_persisted, validated_graph_id, with_opacity,
};

#[derive(Debug, Clone)]
pub(super) struct SketchNetLabelBadge {
    pub(super) id: String,
    pub(super) net_id: String,
    pub(super) kind: SketchNetLabelKind,
    pub(super) rect: egui::Rect,
}

impl CircuitCiApp {
    pub(super) fn apply_add_schematic_net_label_at(
        &mut self,
        canvas: egui::Rect,
        viewport: SketchViewport,
        net_id: &str,
        kind: SketchNetLabelKind,
        position: egui::Pos2,
    ) {
        let (x, y) = persisted_wire_route_point_from_screen_with_snap(
            canvas,
            position,
            viewport,
            self.sketch_snap_enabled,
            self.sketch_grid_step,
        );
        match append_schematic_net_label(&self.project_yaml, net_id, kind, SketchPosition { x, y })
        {
            Ok(updated) => {
                self.set_single_sketch_selection(Some(super::sketch::SketchSelection::Net(
                    net_id.to_string(),
                )));
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Placed {} for net {net_id}.", kind.label()),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn net_label_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        badge: &SketchNetLabelBadge,
    ) {
        ui.strong(format!("{} {}", badge.kind.label(), badge.net_id));
        if ui.button("Inspect Net").clicked() {
            self.set_single_sketch_selection(Some(super::sketch::SketchSelection::Net(
                badge.net_id.clone(),
            )));
            ui.close();
        }
        ui.separator();
        let next_kind = match badge.kind {
            SketchNetLabelKind::Local => SketchNetLabelKind::OffPage,
            SketchNetLabelKind::OffPage => SketchNetLabelKind::Local,
        };
        if ui
            .button(format!("Convert To {}", next_kind.label()))
            .clicked()
        {
            match set_schematic_net_label_kind(&self.project_yaml, &badge.id, next_kind) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!("Converted net label {} to {}.", badge.id, next_kind.label()),
                ),
                Err(error) => self.record_error(error),
            }
            ui.close();
        }
        if ui.button("Delete Label").clicked() {
            match remove_schematic_net_label(&self.project_yaml, &badge.id) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!("Removed schematic net label {}.", badge.id),
                ),
                Err(error) => self.record_error(error),
            }
            ui.close();
        }
    }
}

pub(super) fn layout_net_label_badges(
    snapshot: &ProjectSnapshot,
    canvas: egui::Rect,
    viewport: SketchViewport,
) -> Vec<SketchNetLabelBadge> {
    snapshot
        .net_labels
        .iter()
        .map(|label| {
            let pos = screen_wire_route_point_from_persisted(
                canvas,
                (label.position.x, label.position.y),
                viewport,
            );
            let width = (label.net_id.len() as f32 * 7.0 + 34.0).clamp(72.0, 220.0);
            let height = 24.0;
            SketchNetLabelBadge {
                id: label.id.clone(),
                net_id: label.net_id.clone(),
                kind: label.kind,
                rect: egui::Rect::from_center_size(pos, egui::vec2(width, height)),
            }
        })
        .collect()
}

pub(super) fn draw_net_label_badge(
    painter: &egui::Painter,
    badge: &SketchNetLabelBadge,
    hovered: bool,
    selected: bool,
    opacity: f32,
) {
    let fill = match badge.kind {
        SketchNetLabelKind::Local => egui::Color32::from_rgb(31, 70, 96),
        SketchNetLabelKind::OffPage => egui::Color32::from_rgb(85, 63, 28),
    };
    let stroke = if selected {
        egui::Stroke::new(2.0, egui::Color32::from_rgb(93, 185, 255))
    } else if hovered {
        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 196, 87))
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_rgb(165, 175, 186))
    };
    painter.rect_filled(badge.rect, 3.0, with_opacity(fill, opacity));
    painter.rect_stroke(
        badge.rect,
        3.0,
        egui::Stroke::new(stroke.width, with_opacity(stroke.color, opacity)),
        egui::StrokeKind::Inside,
    );
    if badge.kind == SketchNetLabelKind::OffPage {
        let tip = egui::pos2(badge.rect.right() - 8.0, badge.rect.center().y);
        painter.line_segment(
            [
                egui::pos2(tip.x - 7.0, tip.y - 6.0),
                egui::pos2(tip.x, tip.y),
            ],
            egui::Stroke::new(stroke.width, with_opacity(stroke.color, opacity)),
        );
        painter.line_segment(
            [
                egui::pos2(tip.x - 7.0, tip.y + 6.0),
                egui::pos2(tip.x, tip.y),
            ],
            egui::Stroke::new(stroke.width, with_opacity(stroke.color, opacity)),
        );
    }
    painter.text(
        badge.rect.left_center() + egui::vec2(9.0, 0.0),
        egui::Align2::LEFT_CENTER,
        &badge.net_id,
        egui::FontId::monospace(12.0),
        with_opacity(egui::Color32::WHITE, opacity),
    );
}

pub(super) fn hit_test_net_label_badge(
    badges: &[SketchNetLabelBadge],
    position: egui::Pos2,
) -> Option<&SketchNetLabelBadge> {
    badges
        .iter()
        .rev()
        .find(|badge| badge.rect.contains(position))
}

pub(super) fn net_label_tooltip(ui: &mut egui::Ui, badge: &SketchNetLabelBadge) {
    ui.strong(badge.kind.label());
    ui.label(format!("net: {}", badge.net_id));
    ui.label(format!("label id: {}", badge.id));
    ui.separator();
    ui.label("Click to select the underlying Board IR net.");
    ui.label("Right-click to convert or delete this schematic label.");
}

pub(super) fn append_schematic_net_label(
    text: &str,
    net_id: &str,
    kind: SketchNetLabelKind,
    position: SketchPosition,
) -> Result<String> {
    let net_id = validated_graph_id(net_id, "net")?;
    validate_position(position)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.nets.contains_key(net_id) {
        anyhow::bail!("Board IR net {net_id} was not found.");
    }
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let label_id = next_label_id(&project, net_id, kind);
    let labels = ensure_net_labels_mapping(&mut yaml)?;
    labels.insert(key(&label_id), label_value(net_id, kind, position)?);
    encode_edited_project_yaml(yaml)
}

pub(super) fn remove_schematic_net_label(text: &str, label_id: &str) -> Result<String> {
    let label_id = validated_graph_id(label_id, "net label")?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let labels = ensure_net_labels_mapping(&mut yaml)?;
    if labels.remove(key(label_id)).is_none() {
        anyhow::bail!("Schematic net label {label_id} was not found.");
    }
    encode_edited_project_yaml(yaml)
}

pub(super) fn set_schematic_net_label_kind(
    text: &str,
    label_id: &str,
    kind: SketchNetLabelKind,
) -> Result<String> {
    let label_id = validated_graph_id(label_id, "net label")?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let labels = ensure_net_labels_mapping(&mut yaml)?;
    let label = labels
        .get_mut(key(label_id))
        .with_context(|| format!("Schematic net label {label_id} was not found."))?
        .as_mapping_mut()
        .with_context(|| format!("Schematic net label {label_id} must be an object."))?;
    label.insert(
        key("kind"),
        serde_yaml_ng::Value::String(kind.as_str().to_string()),
    );
    encode_edited_project_yaml(yaml)
}

pub(super) fn remove_net_labels_for_net(yaml: &mut serde_yaml_ng::Value, net_id: &str) {
    let Some(labels) = yaml
        .as_mapping_mut()
        .and_then(|project| project.get_mut(key("board")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|board| board.get_mut(key("schematic")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|schematic| schematic.get_mut(key("net_labels")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
    else {
        return;
    };
    let to_remove: Vec<_> = labels
        .iter()
        .filter(|(_id, label)| {
            label
                .as_mapping()
                .and_then(|mapping| mapping.get(key("net")))
                .and_then(serde_yaml_ng::Value::as_str)
                == Some(net_id)
        })
        .map(|(id, _label)| id.clone())
        .collect();
    for id in to_remove {
        labels.remove(id);
    }
}

pub(super) fn rename_net_labels(yaml: &mut serde_yaml_ng::Value, old_net: &str, new_net: &str) {
    let Some(labels) = yaml
        .as_mapping_mut()
        .and_then(|project| project.get_mut(key("board")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|board| board.get_mut(key("schematic")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
        .and_then(|schematic| schematic.get_mut(key("net_labels")))
        .and_then(serde_yaml_ng::Value::as_mapping_mut)
    else {
        return;
    };
    for label in labels.values_mut() {
        let Some(mapping) = label.as_mapping_mut() else {
            continue;
        };
        if mapping
            .get(key("net"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(old_net)
        {
            mapping.insert(
                key("net"),
                serde_yaml_ng::Value::String(new_net.to_string()),
            );
        }
    }
}

fn next_label_id(
    project: &crate::board_ir::BoardProject,
    net_id: &str,
    kind: SketchNetLabelKind,
) -> String {
    let stem = match kind {
        SketchNetLabelKind::Local => "label",
        SketchNetLabelKind::OffPage => "offpage",
    };
    let base = format!("{stem}_{net_id}");
    if !project.board.schematic.net_labels.contains_key(&base) {
        return base;
    }
    let mut suffix = 2usize;
    loop {
        let candidate = format!("{base}_{suffix}");
        if !project.board.schematic.net_labels.contains_key(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn label_value(
    net_id: &str,
    kind: SketchNetLabelKind,
    position: SketchPosition,
) -> Result<serde_yaml_ng::Value> {
    let mut label = serde_yaml_ng::Mapping::new();
    label.insert(key("net"), serde_yaml_ng::Value::String(net_id.to_string()));
    label.insert(key("x"), serde_yaml_ng::to_value(position.x)?);
    label.insert(key("y"), serde_yaml_ng::to_value(position.y)?);
    label.insert(
        key("kind"),
        serde_yaml_ng::Value::String(kind.as_str().to_string()),
    );
    Ok(serde_yaml_ng::Value::Mapping(label))
}

fn ensure_net_labels_mapping(
    yaml: &mut serde_yaml_ng::Value,
) -> Result<&mut serde_yaml_ng::Mapping> {
    let project = yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let board = project
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
        .get_mut(schematic_key)
        .expect("schematic exists")
        .as_mapping_mut()
        .context("Board IR board.schematic must be an object.")?;
    let labels_key = key("net_labels");
    if !schematic.contains_key(&labels_key) {
        schematic.insert(
            labels_key.clone(),
            serde_yaml_ng::Value::Mapping(Default::default()),
        );
    }
    schematic
        .get_mut(labels_key)
        .expect("net_labels exists")
        .as_mapping_mut()
        .context("Board IR board.schematic.net_labels must be an object.")
}

fn validate_position(position: SketchPosition) -> Result<()> {
    if !position.x.is_finite() || !position.y.is_finite() {
        anyhow::bail!("Schematic net label position must be finite.");
    }
    Ok(())
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::sketch::{load_project_snapshot_from_yaml, validate_board_ir_yaml_text};

    fn project_yaml() -> &'static str {
        "project:
  name: net_label_test
  version: 0.1.0
board:
  components: {}
  nets:
    sig:
      kind: digital_or_analog
"
    }

    #[test]
    fn append_convert_and_remove_schematic_net_label() {
        let edited = append_schematic_net_label(
            project_yaml(),
            "sig",
            SketchNetLabelKind::Local,
            SketchPosition { x: 80.0, y: 96.0 },
        )
        .unwrap();
        validate_board_ir_yaml_text(&edited).unwrap();
        assert!(edited.contains("net_labels:"));
        assert!(edited.contains("label_sig:"));

        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        assert_eq!(snapshot.net_labels.len(), 1);
        assert_eq!(snapshot.net_labels[0].net_id, "sig");
        assert_eq!(snapshot.net_labels[0].kind, SketchNetLabelKind::Local);

        let converted =
            set_schematic_net_label_kind(&edited, "label_sig", SketchNetLabelKind::OffPage)
                .unwrap();
        assert!(converted.contains("kind: off_page"));
        let removed = remove_schematic_net_label(&converted, "label_sig").unwrap();
        let snapshot = load_project_snapshot_from_yaml(&removed).unwrap();
        assert!(snapshot.net_labels.is_empty());
    }

    #[test]
    fn append_schematic_net_label_rejects_unknown_net() {
        let error = append_schematic_net_label(
            project_yaml(),
            "missing",
            SketchNetLabelKind::Local,
            SketchPosition { x: 80.0, y: 96.0 },
        )
        .unwrap_err();

        assert!(error.to_string().contains("missing"));
    }
}
