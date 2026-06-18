use anyhow::{Context, Result};
use eframe::egui;

use super::CircuitCiApp;
use super::sketch::{
    ProjectSnapshot, SketchNetLabelKind, SketchPosition, SketchSelection, SketchViewport,
    encode_edited_project_yaml, ensure_board_child_mapping_mut, normalized_net_kind,
    persisted_wire_route_point_from_screen_with_snap, screen_wire_route_point_from_persisted,
    validated_graph_id, with_opacity,
};
use super::sketch_rename::rename_net;

#[derive(Debug, Clone)]
pub(super) struct SketchNetLabelBadge {
    pub(super) id: String,
    pub(super) net_id: String,
    pub(super) kind: SketchNetLabelKind,
    pub(super) rect: egui::Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NetLabelInlineAction {
    Apply,
    Cancel,
}

impl CircuitCiApp {
    pub(super) fn sketch_net_label_panel(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        ui.collapsing("Named Net Labels", |ui| {
            ui.horizontal(|ui| {
                ui.label("Net");
                ui.text_edit_singleline(&mut self.sketch_net_label_net_id);
            });
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("sketch_net_label_kind")
                    .selected_text(self.sketch_net_label_kind.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.sketch_net_label_kind,
                            SketchNetLabelKind::Local,
                            SketchNetLabelKind::Local.label(),
                        );
                        ui.selectable_value(
                            &mut self.sketch_net_label_kind,
                            SketchNetLabelKind::OffPage,
                            SketchNetLabelKind::OffPage.label(),
                        );
                    });
                egui::ComboBox::from_id_salt("sketch_net_label_net_kind")
                    .selected_text(&self.sketch_net_label_net_kind)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.sketch_net_label_net_kind,
                            "digital_or_analog".to_string(),
                            "digital_or_analog",
                        );
                        ui.selectable_value(
                            &mut self.sketch_net_label_net_kind,
                            "power".to_string(),
                            "power",
                        );
                        ui.selectable_value(
                            &mut self.sketch_net_label_net_kind,
                            "ground".to_string(),
                            "ground",
                        );
                    });
            });
            let net_id = self.sketch_net_label_net_id.trim();
            let net_exists = snapshot.nets_detail.iter().any(|net| net.id == net_id);
            if net_id.is_empty() {
                ui.label("Type a Board IR net ID to place a label.");
            } else if net_exists {
                ui.label("Existing net will be reused.");
            } else {
                ui.label("Missing net will be created with the selected kind.");
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !self.project_yaml.trim().is_empty(),
                        egui::Button::new("Place At View Center"),
                    )
                    .clicked()
                    && let Some(canvas) = self.sketch_last_canvas_rect
                {
                    self.apply_add_or_create_schematic_net_label_at(
                        canvas,
                        self.sketch_viewport(),
                        canvas.center(),
                    );
                }
                let place_label = if self.sketch_net_label_place_armed {
                    "Placement Armed"
                } else {
                    "Place On Canvas"
                };
                if ui
                    .add_enabled(
                        !self.project_yaml.trim().is_empty(),
                        egui::Button::new(place_label),
                    )
                    .clicked()
                {
                    self.sketch_net_label_place_armed = !self.sketch_net_label_place_armed;
                    if self.sketch_net_label_place_armed {
                        self.sketch_palette_place_armed = false;
                        self.sketch_library_place_armed = false;
                        self.status =
                            "Click blank schematic space to place a named net label.".to_string();
                    }
                }
                if self.sketch_net_label_place_armed && ui.button("Cancel").clicked() {
                    self.sketch_net_label_place_armed = false;
                    self.status = "Named net label placement canceled.".to_string();
                }
            });
            if let Some(SketchSelection::Net(net_id)) = self.selected_sketch_item.clone() {
                ui.horizontal(|ui| {
                    if ui.button("Load Selected Net").clicked() {
                        self.sketch_net_label_net_id = net_id.clone();
                    }
                    if ui
                        .add_enabled(
                            !self.sketch_net_label_net_id.trim().is_empty()
                                && self.sketch_net_label_net_id.trim() != net_id,
                            egui::Button::new("Rename Selected To Typed"),
                        )
                        .clicked()
                    {
                        self.apply_rename_selected_net_to_typed_label(&net_id);
                    }
                });
            }
        });
    }

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

    pub(super) fn apply_add_or_create_schematic_net_label_at(
        &mut self,
        canvas: egui::Rect,
        viewport: SketchViewport,
        position: egui::Pos2,
    ) {
        let (x, y) = persisted_wire_route_point_from_screen_with_snap(
            canvas,
            position,
            viewport,
            self.sketch_snap_enabled,
            self.sketch_grid_step,
        );
        let net_id = self.sketch_net_label_net_id.trim().to_string();
        match append_or_create_schematic_net_label(
            &self.project_yaml,
            &net_id,
            &self.sketch_net_label_net_kind,
            self.sketch_net_label_kind,
            SketchPosition { x, y },
        ) {
            Ok((updated, created_net)) => {
                self.sketch_net_label_place_armed = false;
                self.set_single_sketch_selection(Some(super::sketch::SketchSelection::Net(
                    net_id.clone(),
                )));
                let status = if created_net {
                    format!(
                        "Created net {net_id} and placed {}.",
                        self.sketch_net_label_kind.label()
                    )
                } else {
                    format!(
                        "Placed {} for existing net {net_id}.",
                        self.sketch_net_label_kind.label()
                    )
                };
                self.apply_edited_project_yaml(updated, &status);
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_rename_selected_net_to_typed_label(&mut self, selected_net: &str) {
        let new_id = self.sketch_net_label_net_id.trim().to_string();
        match rename_net(&self.project_yaml, selected_net, &new_id) {
            Ok(updated) => {
                self.set_single_sketch_selection(Some(super::sketch::SketchSelection::Net(
                    new_id.clone(),
                )));
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Net {selected_net} renamed to {new_id}."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn begin_net_label_inline_edit(&mut self, badge: &SketchNetLabelBadge) {
        self.sketch_net_label_edit = Some(super::SketchNetLabelEdit {
            label_id: badge.id.clone(),
            original_net_id: badge.net_id.clone(),
            draft_net_id: badge.net_id.clone(),
            draft_kind: badge.kind,
        });
        self.sketch_net_label_net_id = badge.net_id.clone();
        self.sketch_net_label_kind = badge.kind;
        self.status = format!("Editing schematic net label {}.", badge.id);
    }

    pub(super) fn sketch_net_label_inline_editor(
        &mut self,
        ui: &mut egui::Ui,
        badges: &[SketchNetLabelBadge],
        snapshot: &ProjectSnapshot,
    ) {
        let Some(edit) = self.sketch_net_label_edit.clone() else {
            return;
        };
        let Some(badge) = badges.iter().find(|badge| badge.id == edit.label_id) else {
            self.sketch_net_label_edit = None;
            return;
        };

        let mut draft_net_id = edit.draft_net_id.clone();
        let mut draft_kind = edit.draft_kind;
        let mut action = None;
        let editor_pos = badge.rect.right_top() + egui::vec2(8.0, -4.0);
        egui::Area::new(egui::Id::new(("net_label_edit", &edit.label_id)))
            .order(egui::Order::Foreground)
            .fixed_pos(editor_pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(240.0);
                    ui.strong("Edit Net Label");
                    ui.horizontal(|ui| {
                        ui.label("Net");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut draft_net_id).desired_width(150.0),
                        );
                        if response.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter))
                        {
                            action = Some(NetLabelInlineAction::Apply);
                        }
                    });
                    egui::ComboBox::from_id_salt(("inline_net_label_kind", &edit.label_id))
                        .selected_text(draft_kind.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut draft_kind,
                                SketchNetLabelKind::Local,
                                SketchNetLabelKind::Local.label(),
                            );
                            ui.selectable_value(
                                &mut draft_kind,
                                SketchNetLabelKind::OffPage,
                                SketchNetLabelKind::OffPage.label(),
                            );
                        });
                    let trimmed = draft_net_id.trim();
                    let matches: Vec<_> = snapshot
                        .nets_detail
                        .iter()
                        .filter(|net| !trimmed.is_empty() && net.id.contains(trimmed))
                        .take(5)
                        .map(|net| net.id.clone())
                        .collect();
                    if !matches.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Use");
                            for net in matches {
                                if ui.small_button(&net).clicked() {
                                    draft_net_id = net;
                                }
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !draft_net_id.trim().is_empty(),
                                egui::Button::new("Apply"),
                            )
                            .clicked()
                        {
                            action = Some(NetLabelInlineAction::Apply);
                        }
                        if ui.button("Cancel").clicked() {
                            action = Some(NetLabelInlineAction::Cancel);
                        }
                    });
                });
            });

        if let Some(edit) = &mut self.sketch_net_label_edit {
            edit.draft_net_id = draft_net_id;
            edit.draft_kind = draft_kind;
        }
        match action {
            Some(NetLabelInlineAction::Apply) => self.apply_net_label_inline_edit(),
            Some(NetLabelInlineAction::Cancel) => {
                self.sketch_net_label_edit = None;
                self.status = "Canceled schematic net label edit.".to_string();
            }
            None => {}
        }
    }

    fn apply_net_label_inline_edit(&mut self) {
        let Some(edit) = self.sketch_net_label_edit.clone() else {
            return;
        };
        let new_net_id = edit.draft_net_id.trim().to_string();
        let net_exists = self
            .project_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.nets_detail.iter().any(|net| net.id == new_net_id));
        let result = if new_net_id == edit.original_net_id {
            set_schematic_net_label_kind(&self.project_yaml, &edit.label_id, edit.draft_kind)
        } else if net_exists {
            set_schematic_net_label_target_and_kind(
                &self.project_yaml,
                &edit.label_id,
                &new_net_id,
                edit.draft_kind,
            )
        } else {
            rename_net(&self.project_yaml, &edit.original_net_id, &new_net_id).and_then(|updated| {
                set_schematic_net_label_kind(&updated, &edit.label_id, edit.draft_kind)
            })
        };
        match result {
            Ok(updated) => {
                self.sketch_net_label_edit = None;
                self.sketch_net_label_net_id = new_net_id.clone();
                self.sketch_net_label_kind = edit.draft_kind;
                self.set_single_sketch_selection(Some(SketchSelection::Net(new_net_id.clone())));
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Updated schematic net label {}.", edit.label_id),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_move_schematic_net_label_to(
        &mut self,
        canvas: egui::Rect,
        viewport: SketchViewport,
        label_id: &str,
        net_id: &str,
        position: egui::Pos2,
    ) {
        let (x, y) = persisted_wire_route_point_from_screen_with_snap(
            canvas,
            position,
            viewport,
            self.sketch_snap_enabled,
            self.sketch_grid_step,
        );
        match edit_schematic_net_label_position(
            &self.project_yaml,
            label_id,
            SketchPosition { x, y },
        ) {
            Ok(updated) => {
                self.set_single_sketch_selection(Some(super::sketch::SketchSelection::Net(
                    net_id.to_string(),
                )));
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Moved schematic net label {label_id}."),
                );
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn net_label_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        badge: &SketchNetLabelBadge,
        badges: &[SketchNetLabelBadge],
        canvas: egui::Rect,
    ) {
        ui.strong(format!("{} {}", badge.kind.label(), badge.net_id));
        if ui.button("Inspect Net").clicked() {
            self.set_single_sketch_selection(Some(super::sketch::SketchSelection::Net(
                badge.net_id.clone(),
            )));
            ui.close();
        }
        if ui.button("Edit Label").clicked() {
            self.begin_net_label_inline_edit(badge);
            ui.close();
        }
        let peer_count = badges
            .iter()
            .filter(|peer| peer.net_id == badge.net_id && peer.id != badge.id)
            .count();
        if ui
            .add_enabled(peer_count > 0, egui::Button::new("Next Peer Label"))
            .clicked()
        {
            self.focus_next_peer_net_label(badge, badges, canvas);
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

    fn focus_next_peer_net_label(
        &mut self,
        current: &SketchNetLabelBadge,
        badges: &[SketchNetLabelBadge],
        canvas: egui::Rect,
    ) {
        let Some(next) = next_peer_net_label(current, badges) else {
            return;
        };
        self.sketch_pan += canvas.center() - next.rect.center();
        self.set_single_sketch_selection(Some(SketchSelection::Net(next.net_id.clone())));
        self.status = format!(
            "Centered peer {} for net {}.",
            next.kind.label(),
            next.net_id
        );
    }
}

fn next_peer_net_label<'a>(
    current: &SketchNetLabelBadge,
    badges: &'a [SketchNetLabelBadge],
) -> Option<&'a SketchNetLabelBadge> {
    let peers: Vec<_> = badges
        .iter()
        .filter(|badge| badge.net_id == current.net_id)
        .collect();
    let current_index = peers.iter().position(|badge| badge.id == current.id)?;
    peers.get((current_index + 1) % peers.len()).copied()
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
    ui.label("Double-click to edit or retarget this label.");
    ui.label("Right-click to edit, convert, or delete this schematic label.");
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

pub(super) fn append_or_create_schematic_net_label(
    text: &str,
    net_id: &str,
    net_kind: &str,
    kind: SketchNetLabelKind,
    position: SketchPosition,
) -> Result<(String, bool)> {
    let net_id = validated_graph_id(net_id, "net")?;
    let net_kind = normalized_net_kind(net_kind)?;
    validate_position(position)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let created_net = !project.board.nets.contains_key(net_id);
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    if created_net {
        let nets = ensure_board_child_mapping_mut(&mut yaml, "nets")?;
        let mut net = serde_yaml_ng::Mapping::new();
        net.insert(
            key("kind"),
            serde_yaml_ng::Value::String(net_kind.to_string()),
        );
        nets.insert(key(net_id), serde_yaml_ng::Value::Mapping(net));
    }
    let label_id = next_label_id(&project, net_id, kind);
    let labels = ensure_net_labels_mapping(&mut yaml)?;
    labels.insert(key(&label_id), label_value(net_id, kind, position)?);
    Ok((encode_edited_project_yaml(yaml)?, created_net))
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

pub(super) fn set_schematic_net_label_target_and_kind(
    text: &str,
    label_id: &str,
    net_id: &str,
    kind: SketchNetLabelKind,
) -> Result<String> {
    let label_id = validated_graph_id(label_id, "net label")?;
    let net_id = validated_graph_id(net_id, "net")?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.nets.contains_key(net_id) {
        anyhow::bail!("Board IR net {net_id} was not found.");
    }
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let labels = ensure_net_labels_mapping(&mut yaml)?;
    let label = labels
        .get_mut(key(label_id))
        .with_context(|| format!("Schematic net label {label_id} was not found."))?
        .as_mapping_mut()
        .with_context(|| format!("Schematic net label {label_id} must be an object."))?;
    label.insert(key("net"), serde_yaml_ng::Value::String(net_id.to_string()));
    label.insert(
        key("kind"),
        serde_yaml_ng::Value::String(kind.as_str().to_string()),
    );
    encode_edited_project_yaml(yaml)
}

pub(super) fn edit_schematic_net_label_position(
    text: &str,
    label_id: &str,
    position: SketchPosition,
) -> Result<String> {
    let label_id = validated_graph_id(label_id, "net label")?;
    validate_position(position)?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let labels = ensure_net_labels_mapping(&mut yaml)?;
    let label = labels
        .get_mut(key(label_id))
        .with_context(|| format!("Schematic net label {label_id} was not found."))?
        .as_mapping_mut()
        .with_context(|| format!("Schematic net label {label_id} must be an object."))?;
    label.insert(key("x"), serde_yaml_ng::to_value(position.x)?);
    label.insert(key("y"), serde_yaml_ng::to_value(position.y)?);
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
    other:
      kind: power
"
    }

    fn badge(id: &str, net_id: &str) -> SketchNetLabelBadge {
        SketchNetLabelBadge {
            id: id.to_string(),
            net_id: net_id.to_string(),
            kind: SketchNetLabelKind::Local,
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(24.0)),
        }
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
        let moved = edit_schematic_net_label_position(
            &converted,
            "label_sig",
            SketchPosition { x: 128.0, y: 160.0 },
        )
        .unwrap();
        let snapshot = load_project_snapshot_from_yaml(&moved).unwrap();
        assert_eq!(snapshot.net_labels[0].position.x, 128.0);
        assert_eq!(snapshot.net_labels[0].position.y, 160.0);
        let removed = remove_schematic_net_label(&moved, "label_sig").unwrap();
        let snapshot = load_project_snapshot_from_yaml(&removed).unwrap();
        assert!(snapshot.net_labels.is_empty());
    }

    #[test]
    fn next_peer_net_label_cycles_same_net_labels_only() {
        let labels = [
            badge("label_sig", "sig"),
            badge("label_other", "other"),
            badge("offpage_sig", "sig"),
        ];

        let next = next_peer_net_label(&labels[0], &labels).unwrap();
        assert_eq!(next.id, "offpage_sig");
        let wrapped = next_peer_net_label(next, &labels).unwrap();
        assert_eq!(wrapped.id, "label_sig");
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

    #[test]
    fn set_schematic_net_label_target_and_kind_retargets_existing_net() {
        let labeled = append_schematic_net_label(
            project_yaml(),
            "sig",
            SketchNetLabelKind::Local,
            SketchPosition { x: 80.0, y: 96.0 },
        )
        .unwrap();

        let edited = set_schematic_net_label_target_and_kind(
            &labeled,
            "label_sig",
            "other",
            SketchNetLabelKind::OffPage,
        )
        .unwrap();

        validate_board_ir_yaml_text(&edited).unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        assert_eq!(snapshot.net_labels.len(), 1);
        assert_eq!(snapshot.net_labels[0].id, "label_sig");
        assert_eq!(snapshot.net_labels[0].net_id, "other");
        assert_eq!(snapshot.net_labels[0].kind, SketchNetLabelKind::OffPage);
        assert!(snapshot.nets_detail.iter().any(|net| net.id == "sig"));
        assert!(snapshot.nets_detail.iter().any(|net| net.id == "other"));
    }

    #[test]
    fn set_schematic_net_label_target_and_kind_rejects_missing_net() {
        let labeled = append_schematic_net_label(
            project_yaml(),
            "sig",
            SketchNetLabelKind::Local,
            SketchPosition { x: 80.0, y: 96.0 },
        )
        .unwrap();

        let error = set_schematic_net_label_target_and_kind(
            &labeled,
            "label_sig",
            "missing",
            SketchNetLabelKind::Local,
        )
        .unwrap_err();

        assert!(error.to_string().contains("missing"));
    }

    #[test]
    fn append_or_create_schematic_net_label_creates_missing_net() {
        let (edited, created) = append_or_create_schematic_net_label(
            project_yaml(),
            "typed_bus",
            "power",
            SketchNetLabelKind::OffPage,
            SketchPosition { x: 128.0, y: 160.0 },
        )
        .unwrap();

        assert!(created);
        validate_board_ir_yaml_text(&edited).unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        let net = snapshot
            .nets_detail
            .iter()
            .find(|net| net.id == "typed_bus")
            .expect("created net is present");
        assert_eq!(net.kind, "power");
        assert_eq!(snapshot.net_labels.len(), 1);
        assert_eq!(snapshot.net_labels[0].id, "offpage_typed_bus");
        assert_eq!(snapshot.net_labels[0].net_id, "typed_bus");
        assert_eq!(snapshot.net_labels[0].kind, SketchNetLabelKind::OffPage);
    }

    #[test]
    fn append_or_create_schematic_net_label_reuses_existing_net() {
        let (edited, created) = append_or_create_schematic_net_label(
            project_yaml(),
            "sig",
            "ground",
            SketchNetLabelKind::Local,
            SketchPosition { x: 32.0, y: 48.0 },
        )
        .unwrap();

        assert!(!created);
        validate_board_ir_yaml_text(&edited).unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        let net = snapshot
            .nets_detail
            .iter()
            .find(|net| net.id == "sig")
            .expect("existing net is present");
        assert_eq!(net.kind, "digital_or_analog");
        assert_eq!(snapshot.net_labels.len(), 1);
        assert_eq!(snapshot.net_labels[0].id, "label_sig");
    }

    #[test]
    fn append_or_create_schematic_net_label_rejects_unknown_net_kind() {
        let error = append_or_create_schematic_net_label(
            project_yaml(),
            "typed_bus",
            "mystery",
            SketchNetLabelKind::Local,
            SketchPosition { x: 32.0, y: 48.0 },
        )
        .unwrap_err();

        assert!(error.to_string().contains("Unsupported net kind"));
    }
}
