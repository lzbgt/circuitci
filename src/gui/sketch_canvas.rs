use eframe::egui;

use super::sketch::{
    self, ProjectSnapshot, SketchSelection, draw_sketch_grid, draw_sketch_node,
    draw_sketch_pin_anchor, edge_label_position, hit_test_wire, layout_sketch_graph_viewport,
    orthogonal_wire_points, persisted_node_position_from_screen_with_snap,
    snap_screen_point_to_grid, with_opacity,
};
use super::sketch_inspector::{
    default_current_probe_name_for_component, default_power_probe_name_for_component,
    default_probe_name_for_net,
};
use super::sketch_probes::{
    SketchProbeBadge, SketchProbeStatus, draw_probe_badge, hit_test_probe_badge,
    probe_assertion_status,
};
use super::waveform::{
    runtime_probe_activity_for_selection, runtime_probe_lines_for_selection,
    waveform_probe_value_for_badge,
};
use super::{CircuitCiApp, Stage, analog, sketch_bundles, sketch_hierarchy};

impl CircuitCiApp {
    pub(super) fn draw_board_graph_sized(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &ProjectSnapshot,
        desired_size: egui::Vec2,
    ) {
        let desired_size = schematic_canvas_size(desired_size);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
        self.sketch_last_canvas_rect = Some(rect);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, egui::Color32::from_gray(18));
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
            egui::StrokeKind::Inside,
        );

        if let Some(command) = self.sketch_viewport_command.take() {
            self.apply_sketch_viewport_command(rect, snapshot, command);
        }
        if let Some(target) = self.sketch_navigator_fit_target.take() {
            self.fit_sketch_navigator_target(rect, snapshot, &target);
        }
        if let Some(target) = self.sketch_hierarchy_fit_target.take() {
            self.fit_sketch_hierarchy_target(rect, snapshot, &target);
        }
        let viewport = self.sketch_viewport();
        draw_sketch_grid(
            &painter,
            rect,
            viewport,
            self.sketch_grid_enabled,
            self.sketch_grid_step,
        );
        let graph = layout_sketch_graph_viewport(rect, snapshot, viewport);
        let hierarchy_view = self.sketch_hierarchy_view(snapshot);
        if let Some(view) = &hierarchy_view {
            painter.text(
                rect.left_top() + egui::vec2(12.0, 12.0),
                egui::Align2::LEFT_TOP,
                format!("Hierarchy focus: {}", view.label()),
                egui::FontId::monospace(12.0),
                egui::Color32::from_rgb(255, 226, 145),
            );
        }
        let hierarchy_connector_badges = hierarchy_view
            .as_ref()
            .map(|view| sketch_hierarchy::layout_hierarchy_connector_badges(snapshot, &graph, view))
            .unwrap_or_default();
        let bundle_badges = sketch_bundles::layout_net_bundle_badges(snapshot, &graph);
        if let Some(action) = self.sketch_group_action.take() {
            self.apply_sketch_group_action(rect, &graph, viewport, action);
        }
        let pointer_hover = if response.hovered() {
            ui.ctx().pointer_hover_pos()
        } else {
            None
        };
        let hovered_node = pointer_hover.and_then(|position| {
            graph.nodes.iter().find(|node| {
                hierarchy_view
                    .as_ref()
                    .is_none_or(|view| view.interaction_visible(&node.selection))
                    && node.rect.contains(position)
            })
        });
        let hovered_anchor = pointer_hover.and_then(|position| {
            graph.pin_anchors.iter().find(|anchor| {
                hierarchy_view
                    .as_ref()
                    .is_none_or(|view| view.anchor_visible(anchor))
                    && anchor.pos.distance(position) <= 8.0
            })
        });
        let hovered_wire = if hovered_node.is_none() && hovered_anchor.is_none() {
            pointer_hover.and_then(|position| {
                hit_test_wire(&graph, position).filter(|edge| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.edge_visible(edge))
                })
            })
        } else {
            None
        };
        let hovered_probe_badge = pointer_hover.and_then(|position| {
            hit_test_probe_badge(&graph.probe_badges, position).filter(|badge| {
                hierarchy_view
                    .as_ref()
                    .is_none_or(|view| view.probe_badge_visible(badge))
            })
        });
        let hovered_bundle_badge = pointer_hover.and_then(|position| {
            sketch_bundles::hit_test_net_bundle_badge(&bundle_badges, position).filter(|badge| {
                hierarchy_view
                    .as_ref()
                    .is_none_or(|view| view.bundle_badge_visible(badge))
            })
        });
        let hovered_hierarchy_connector_badge = pointer_hover.and_then(|position| {
            sketch_hierarchy::hit_test_hierarchy_connector_badge(
                &hierarchy_connector_badges,
                position,
            )
        });
        let blank_canvas_hovered = hovered_node.is_none()
            && hovered_anchor.is_none()
            && hovered_wire.is_none()
            && hovered_probe_badge.is_none()
            && hovered_bundle_badge.is_none()
            && hovered_hierarchy_connector_badge.is_none();
        self.handle_sketch_viewport_input(
            ui,
            rect,
            &response,
            blank_canvas_hovered
                && !self.sketch_palette_place_armed
                && !self.sketch_library_place_armed,
        );
        let viewport = self.sketch_viewport();
        let graph = layout_sketch_graph_viewport(rect, snapshot, viewport);
        let hierarchy_connector_badges = hierarchy_view
            .as_ref()
            .map(|view| sketch_hierarchy::layout_hierarchy_connector_badges(snapshot, &graph, view))
            .unwrap_or_default();
        let bundle_badges = sketch_bundles::layout_net_bundle_badges(snapshot, &graph);
        let pointer_hover = if response.hovered() {
            ui.ctx().pointer_hover_pos()
        } else {
            None
        };
        let hovered_node = pointer_hover.and_then(|position| {
            graph.nodes.iter().find(|node| {
                hierarchy_view
                    .as_ref()
                    .is_none_or(|view| view.interaction_visible(&node.selection))
                    && node.rect.contains(position)
            })
        });
        let hovered_anchor = pointer_hover.and_then(|position| {
            graph.pin_anchors.iter().find(|anchor| {
                hierarchy_view
                    .as_ref()
                    .is_none_or(|view| view.anchor_visible(anchor))
                    && anchor.pos.distance(position) <= 8.0
            })
        });
        let hovered_wire = if hovered_node.is_none() && hovered_anchor.is_none() {
            pointer_hover.and_then(|position| {
                hit_test_wire(&graph, position).filter(|edge| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.edge_visible(edge))
                })
            })
        } else {
            None
        };
        let hovered_probe_badge = pointer_hover.and_then(|position| {
            hit_test_probe_badge(&graph.probe_badges, position).filter(|badge| {
                hierarchy_view
                    .as_ref()
                    .is_none_or(|view| view.probe_badge_visible(badge))
            })
        });
        let hovered_bundle_badge = pointer_hover.and_then(|position| {
            sketch_bundles::hit_test_net_bundle_badge(&bundle_badges, position).filter(|badge| {
                hierarchy_view
                    .as_ref()
                    .is_none_or(|view| view.bundle_badge_visible(badge))
            })
        });
        let hovered_hierarchy_connector_badge = pointer_hover.and_then(|position| {
            sketch_hierarchy::hit_test_hierarchy_connector_badge(
                &hierarchy_connector_badges,
                position,
            )
        });
        for edge in &graph.edges {
            let opacity = if let Some(view) = &hierarchy_view {
                if !view.edge_visible(edge) {
                    continue;
                }
                view.edge_opacity(edge)
            } else {
                1.0
            };
            let wire_selection = SketchSelection::Net(edge.net_id.clone());
            let selected = self.selection_is_selected(&wire_selection);
            let hovered = hovered_wire
                .is_some_and(|wire| wire.net_id == edge.net_id && wire.source == edge.source);
            draw_wire_edge(&painter, edge, selected, hovered, self.sketch_zoom, opacity);
        }
        if let Some(component_id) = &self.wire_from_component
            && let Some(pointer) = ui.ctx().pointer_hover_pos()
            && rect.contains(pointer)
            && hierarchy_view.as_ref().is_none_or(|view| {
                view.interaction_visible(&SketchSelection::Component(component_id.clone()))
            })
            && let Some(source) = wire_preview_start(&graph, component_id, &self.wire_pin_id)
        {
            let pointer = snap_screen_point_to_grid(
                rect,
                pointer,
                viewport,
                self.sketch_snap_enabled,
                self.sketch_grid_step,
            );
            draw_wire_polyline(
                &painter,
                source,
                pointer,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 196, 87)),
            );
        }
        for badge in &bundle_badges {
            let opacity = if let Some(view) = &hierarchy_view {
                if !view.bundle_badge_visible(badge) {
                    continue;
                }
                view.bundle_badge_opacity(badge)
            } else {
                1.0
            };
            let hovered = hovered_bundle_badge
                .is_some_and(|hovered| hovered.bundle.label == badge.bundle.label);
            sketch_bundles::draw_net_bundle_overlay(&painter, badge, hovered, opacity);
        }
        for node in &graph.nodes {
            let opacity = if let Some(view) = &hierarchy_view {
                if !view.interaction_visible(&node.selection) {
                    continue;
                }
                view.selection_opacity(&node.selection)
            } else {
                1.0
            };
            let selected = self.selection_is_selected(&node.selection);
            let runtime_activity = runtime_probe_activity_for_selection(
                &self.waveforms,
                self.selected_waveform,
                self.waveform_cursor_a_us,
                &node.selection,
                snapshot,
            );
            draw_sketch_node(&painter, node, selected, runtime_activity, opacity);
        }
        for anchor in &graph.pin_anchors {
            let opacity = if let Some(view) = &hierarchy_view {
                if !view.anchor_visible(anchor) {
                    continue;
                }
                view.anchor_opacity(anchor)
            } else {
                1.0
            };
            let active = self.wire_from_component.as_ref() == Some(&anchor.component_id)
                && self.wire_pin_id.trim() == anchor.pin;
            draw_sketch_pin_anchor(&painter, anchor, active, opacity);
        }
        for badge in &hierarchy_connector_badges {
            let hovered = hovered_hierarchy_connector_badge
                .is_some_and(|hovered| hovered.net_id == badge.net_id);
            sketch_hierarchy::draw_hierarchy_connector_badge(&painter, badge, hovered);
        }
        for badge in &graph.probe_badges {
            let opacity = if let Some(view) = &hierarchy_view {
                if !view.probe_badge_visible(badge) {
                    continue;
                }
                view.probe_badge_opacity(badge)
            } else {
                1.0
            };
            let hovered = hovered_probe_badge.is_some_and(|hovered| {
                hovered.probe.scenario_name == badge.probe.scenario_name
                    && hovered.probe.probe_name == badge.probe.probe_name
            });
            let status = probe_assertion_status(self.report.as_ref(), &badge.probe);
            draw_probe_badge(&painter, badge, hovered, status, opacity);
        }

        if response.clicked_by(egui::PointerButton::Primary)
            && let Some(position) = response.interact_pointer_pos()
        {
            let multi_select = ui.input(|input| input.modifiers.shift || input.modifiers.command);
            let clicked_probe_badge =
                hit_test_probe_badge(&graph.probe_badges, position).filter(|badge| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.probe_badge_visible(badge))
                });
            let clicked_hierarchy_connector_badge =
                sketch_hierarchy::hit_test_hierarchy_connector_badge(
                    &hierarchy_connector_badges,
                    position,
                );
            let clicked_bundle_badge =
                sketch_bundles::hit_test_net_bundle_badge(&bundle_badges, position).filter(
                    |badge| {
                        hierarchy_view
                            .as_ref()
                            .is_none_or(|view| view.bundle_badge_visible(badge))
                    },
                );
            let clicked_anchor = graph.pin_anchors.iter().find(|anchor| {
                hierarchy_view
                    .as_ref()
                    .is_none_or(|view| view.anchor_visible(anchor))
                    && anchor.pos.distance(position) <= 8.0
            });
            let clicked = graph
                .nodes
                .iter()
                .find(|node| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.interaction_visible(&node.selection))
                        && node.rect.contains(position)
                })
                .map(|node| node.selection.clone());
            let clicked_wire = if clicked_anchor.is_none() && clicked.is_none() {
                hit_test_wire(&graph, position).filter(|edge| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.edge_visible(edge))
                })
            } else {
                None
            };
            if (self.sketch_palette_place_armed || self.sketch_library_place_armed)
                && clicked_probe_badge.is_none()
                && clicked_hierarchy_connector_badge.is_none()
                && clicked_bundle_badge.is_none()
                && clicked_anchor.is_none()
                && clicked.is_none()
                && clicked_wire.is_none()
            {
                if self.sketch_palette_place_armed {
                    self.apply_insert_sketch_primitive_at(rect, position);
                } else {
                    self.apply_insert_selected_library_model_at(rect, position);
                }
            } else if let Some(badge) = clicked_probe_badge {
                self.open_probe_badge_in_simulation(badge);
            } else if let Some(badge) = clicked_hierarchy_connector_badge {
                self.set_single_sketch_selection(Some(SketchSelection::Net(badge.net_id.clone())));
                self.status = format!(
                    "Selected net {} with {} off-sheet endpoint(s).",
                    badge.net_id,
                    badge.external_targets.len()
                );
            } else if let Some(badge) = clicked_bundle_badge {
                self.select_net_bundle(&badge.bundle);
            } else if let Some(anchor) = clicked_anchor {
                if let Some(source_component_id) = self.wire_from_component.clone()
                    && !(source_component_id == anchor.component_id
                        && self.wire_pin_id.trim() == anchor.pin)
                {
                    self.apply_visual_pin_wire(
                        source_component_id,
                        anchor.component_id.clone(),
                        anchor.pin.clone(),
                    );
                } else {
                    self.start_visual_wire_from_anchor(anchor);
                }
            } else if let Some(SketchSelection::Net(net_id)) = &clicked
                && let Some(component_id) = self.wire_from_component.clone()
            {
                self.apply_visual_wire(component_id, net_id.clone());
            } else if let Some(edge) = clicked_wire
                && let Some(component_id) = self.wire_from_component.clone()
            {
                self.apply_visual_wire(component_id, edge.net_id.clone());
            } else if multi_select {
                if let Some(selection) = clicked {
                    self.toggle_sketch_selection(selection);
                } else if let Some(edge) = clicked_wire {
                    self.toggle_sketch_selection(SketchSelection::Net(edge.net_id.clone()));
                }
            } else if let Some(edge) = clicked_wire {
                self.set_single_sketch_selection(Some(SketchSelection::Net(edge.net_id.clone())));
                self.status = format!("Selected net {} from wire {}.", edge.net_id, edge.source);
            } else {
                self.set_single_sketch_selection(clicked);
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(position) = response.interact_pointer_pos()
        {
            let clicked_anchor = graph.pin_anchors.iter().find(|anchor| {
                hierarchy_view
                    .as_ref()
                    .is_none_or(|view| view.anchor_visible(anchor))
                    && anchor.pos.distance(position) <= 8.0
            });
            let clicked_node = graph
                .nodes
                .iter()
                .find(|node| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.interaction_visible(&node.selection))
                        && node.rect.contains(position)
                })
                .map(|node| node.selection.clone());
            if let Some(anchor) = clicked_anchor {
                self.start_visual_wire_from_anchor(anchor);
            } else if clicked_node.is_none() && ui.input(|input| input.modifiers.shift) {
                self.marquee_start = Some(position);
            } else if clicked_node.is_none() {
                self.sketch_pan_drag_active = true;
            } else if clicked_node.is_some() {
                let multi_selected_hit = clicked_node
                    .as_ref()
                    .is_some_and(|selection| self.selected_sketch_items.contains(selection));
                if !multi_selected_hit {
                    self.set_single_sketch_selection(clicked_node);
                }
            }
        }
        if let Some(start) = self.marquee_start
            && let Some(current) = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().pointer_hover_pos())
        {
            let marquee = egui::Rect::from_two_pos(start, current);
            painter.rect_filled(
                marquee,
                0.0,
                egui::Color32::from_rgba_unmultiplied(93, 185, 255, 24),
            );
            painter.rect_stroke(
                marquee,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(93, 185, 255)),
                egui::StrokeKind::Inside,
            );
        } else if response.dragged_by(egui::PointerButton::Primary)
            && !self.sketch_pan_drag_active
            && self.wire_from_component.is_none()
            && let (Some(selection), Some(position)) = (
                self.selected_sketch_item.clone(),
                response.interact_pointer_pos(),
            )
            && !matches!(selection, SketchSelection::Overflow(_))
            && let Some(node) = graph.nodes.iter().find(|node| node.selection == selection)
        {
            if self.selected_sketch_items.len() > 1
                && self.selected_sketch_items.contains(&selection)
            {
                let delta = ui.input(|input| input.pointer.delta());
                self.apply_selected_schematic_screen_delta(
                    rect,
                    &graph,
                    viewport,
                    delta,
                    "Selected sketch items moved.",
                );
            } else {
                let (x, y) = persisted_node_position_from_screen_with_snap(
                    rect,
                    position,
                    node.rect,
                    viewport,
                    self.sketch_snap_enabled,
                    self.sketch_grid_step,
                );
                self.apply_schematic_node_position(selection, x, y);
            }
        }

        if response.drag_stopped_by(egui::PointerButton::Primary)
            && let Some(start) = self.marquee_start.take()
            && let Some(end) = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().pointer_hover_pos())
        {
            self.apply_marquee_selection(egui::Rect::from_two_pos(start, end), &graph);
        }
        if response.drag_stopped_by(egui::PointerButton::Primary)
            && self.wire_from_component.is_some()
            && let Some(position) = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().pointer_hover_pos())
            && let Some(target) = wire_drag_target_at(
                &graph,
                position,
                |anchor| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.anchor_visible(anchor))
                },
                |edge| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.edge_visible(edge))
                },
                |node| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.interaction_visible(&node.selection))
                },
            )
        {
            self.apply_wire_drag_target(target);
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            self.sketch_pan_drag_active = false;
        }

        let delete_pressed = response.hovered()
            && ui.input(|input| {
                input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace)
            });
        let quick_above_pressed = response.hovered()
            && ui.input(|input| input.modifiers.shift && input.key_pressed(egui::Key::A));
        let quick_below_pressed = response.hovered()
            && ui.input(|input| input.modifiers.shift && input.key_pressed(egui::Key::B));
        let add_assertion_pressed = response.hovered()
            && ui.input(|input| !input.modifiers.shift && input.key_pressed(egui::Key::A));
        let clear_assertions_pressed =
            response.hovered() && ui.input(|input| input.key_pressed(egui::Key::X));
        let duplicate_pressed = response.hovered()
            && ui.input(|input| {
                (input.modifiers.command || input.modifiers.ctrl) && input.key_pressed(egui::Key::D)
            });
        let copy_pressed = response.hovered()
            && ui.input(|input| {
                (input.modifiers.command || input.modifiers.ctrl) && input.key_pressed(egui::Key::C)
            });
        let paste_pressed = response.hovered()
            && ui.input(|input| {
                (input.modifiers.command || input.modifiers.ctrl) && input.key_pressed(egui::Key::V)
            });
        let cancel_canvas_mode_pressed =
            response.hovered() && ui.input(|input| input.key_pressed(egui::Key::Escape));
        let requested_toolbar_paste = std::mem::take(&mut self.sketch_paste_requested);
        if cancel_canvas_mode_pressed
            && (self.sketch_palette_place_armed || self.sketch_library_place_armed)
        {
            self.sketch_palette_place_armed = false;
            self.sketch_library_place_armed = false;
            self.status = "Canvas placement canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.wire_from_component.is_some() {
            self.wire_from_component = None;
            self.status = "Wire mode canceled.".to_string();
        } else if let Some(badge) = hovered_probe_badge {
            if quick_above_pressed {
                self.apply_quick_canvas_probe_assertion(&badge.probe, "above");
            } else if quick_below_pressed {
                self.apply_quick_canvas_probe_assertion(&badge.probe, "below");
            } else if add_assertion_pressed {
                self.apply_add_canvas_probe_assertion(
                    &badge.probe.scenario_name,
                    &badge.probe.probe_name,
                );
            } else if clear_assertions_pressed {
                self.apply_remove_canvas_probe_assertions(
                    &badge.probe.scenario_name,
                    &badge.probe.probe_name,
                );
            } else if delete_pressed {
                self.apply_remove_canvas_probe(&badge.probe.scenario_name, &badge.probe.probe_name);
            }
        } else if paste_pressed {
            self.apply_paste_sketch_clipboard(rect, pointer_hover);
        } else if requested_toolbar_paste {
            self.apply_paste_sketch_clipboard(rect, None);
        } else if copy_pressed {
            self.apply_copy_selected_sketch_items();
        } else if duplicate_pressed {
            self.apply_duplicate_selected_sketch_items();
        } else if delete_pressed {
            self.apply_delete_selected_sketch_item();
        }

        if let Some(badge) = hovered_probe_badge {
            let sampled_value = waveform_probe_value_for_badge(
                &self.waveforms,
                self.selected_waveform,
                self.waveform_cursor_a_us,
                &badge.probe,
            );
            response.context_menu(|ui| {
                self.probe_badge_context_menu(ui, badge, sampled_value);
            });
            response.on_hover_ui(|ui| {
                let status = probe_assertion_status(self.report.as_ref(), &badge.probe);
                sketch_probe_badge_tooltip(ui, badge, status, sampled_value);
            });
        } else if let Some(badge) = hovered_bundle_badge {
            response.on_hover_ui(|ui| {
                sketch_bundles::net_bundle_tooltip(ui, badge);
            });
        } else if let Some(badge) = hovered_hierarchy_connector_badge {
            response.on_hover_ui(|ui| {
                sketch_hierarchy::hierarchy_connector_tooltip(ui, badge);
            });
        } else if let Some(node) = hovered_node {
            response.context_menu(|ui| {
                self.sketch_node_context_menu(ui, node, snapshot);
            });
            let runtime_lines = runtime_probe_lines_for_selection(
                &self.waveforms,
                self.selected_waveform,
                self.waveform_cursor_a_us,
                &node.selection,
                snapshot,
            );
            response.on_hover_ui(|ui| {
                sketch_hover_tooltip(ui, node, &runtime_lines);
            });
        } else if let Some(anchor) = hovered_anchor {
            response.on_hover_ui(|ui| {
                sketch_pin_hover_tooltip(ui, anchor);
            });
        } else if let Some(edge) = hovered_wire {
            response.context_menu(|ui| {
                self.sketch_wire_context_menu(ui, edge);
            });
            response.on_hover_ui(|ui| {
                sketch_wire_hover_tooltip(ui, edge);
            });
        } else {
            response.context_menu(|ui| {
                self.sketch_canvas_context_menu(ui, rect, pointer_hover);
            });
            if self.sketch_palette_place_armed || self.sketch_library_place_armed {
                let label = if self.sketch_palette_place_armed {
                    self.sketch_palette_kind.label()
                } else {
                    self.selected_library_model.as_str()
                };
                response.on_hover_text(format!(
                    "Click blank canvas to place {label}. Press Esc to cancel."
                ));
            }
        }
    }

    fn open_probe_badge_in_simulation(&mut self, badge: &SketchProbeBadge) {
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

    fn probe_badge_context_menu(
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

    fn start_visual_wire_from_anchor(&mut self, anchor: &sketch::SketchPinAnchor) {
        self.set_single_sketch_selection(Some(SketchSelection::Component(
            anchor.component_id.clone(),
        )));
        self.pin_edit_id = anchor.pin.clone();
        self.pin_edit_net = anchor.net.clone();
        self.wire_pin_id = anchor.pin.clone();
        self.wire_from_component = Some(anchor.component_id.clone());
        self.status = format!(
            "Wire mode: drag or click another pin, net, or wire to connect {}.{}.",
            anchor.component_id, anchor.pin
        );
    }

    fn apply_wire_drag_target(&mut self, target: WireDragTarget<'_>) {
        let Some(source_component_id) = self.wire_from_component.clone() else {
            return;
        };
        match target {
            WireDragTarget::Pin(anchor) => {
                if source_component_id == anchor.component_id
                    && self.wire_pin_id.trim() == anchor.pin
                {
                    return;
                }
                self.apply_visual_pin_wire(
                    source_component_id,
                    anchor.component_id.clone(),
                    anchor.pin.clone(),
                );
            }
            WireDragTarget::Net(net_id) => {
                self.apply_visual_wire(source_component_id, net_id.to_string());
            }
        }
    }

    fn sketch_node_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        node: &sketch::SketchNode,
        snapshot: &ProjectSnapshot,
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
                self.net_context_menu(ui, net_id, "Inspect Net", "Delete Net");
            }
            SketchSelection::Overflow(label) => {
                ui.strong(label);
                ui.label("Open the YAML editor or use Fit All for hidden graph items.");
            }
        }
    }

    fn sketch_canvas_context_menu(
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
                self.has_pasteable_sketch_clipboard(),
                egui::Button::new("Paste Here"),
            )
            .clicked()
        {
            self.apply_paste_sketch_clipboard(canvas, pointer_hover);
            ui.close();
        }
    }

    fn sketch_wire_context_menu(&mut self, ui: &mut egui::Ui, edge: &sketch::SketchEdge) {
        ui.strong(format!("Wire {}", edge.net_id));
        ui.label(format!("source: {}", edge.source));
        self.net_context_menu(ui, &edge.net_id, "Inspect Wire Net", "Delete Wire Net");
    }

    fn net_context_menu(
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

    fn ensure_net_probe_defaults(&mut self, net_id: &str) {
        self.ensure_canvas_probe_scenario();
        if self.analog_canvas_probe_name.trim().is_empty() {
            self.analog_canvas_probe_name = default_probe_name_for_net(net_id);
        }
    }

    fn ensure_component_probe_defaults(&mut self, component_id: &str) {
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

    fn handle_sketch_viewport_input(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        response: &egui::Response,
        blank_canvas_hovered: bool,
    ) {
        if response.drag_started_by(egui::PointerButton::Primary)
            && blank_canvas_hovered
            && !ui.input(|input| input.modifiers.shift)
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

fn zoom_viewport_around(
    current_zoom: f32,
    current_pan: egui::Vec2,
    zoom_delta: f32,
    canvas: egui::Rect,
    focus: egui::Pos2,
) -> (f32, egui::Vec2) {
    let old_zoom = current_zoom.clamp(0.25, 4.0);
    let new_zoom = (old_zoom * zoom_delta).clamp(0.25, 4.0);
    if (new_zoom - old_zoom).abs() <= f32::EPSILON {
        return (old_zoom, current_pan);
    }
    let focus = egui::pos2(
        focus.x.clamp(canvas.left(), canvas.right()),
        focus.y.clamp(canvas.top(), canvas.bottom()),
    );
    let focus_offset = focus - canvas.min;
    let logical_focus = (focus_offset - current_pan) / old_zoom;
    (new_zoom, focus_offset - logical_focus * new_zoom)
}

enum WireDragTarget<'a> {
    Pin(&'a sketch::SketchPinAnchor),
    Net(&'a str),
}

fn wire_drag_target_at<'a>(
    graph: &'a sketch::SketchGraph,
    position: egui::Pos2,
    anchor_visible: impl Fn(&sketch::SketchPinAnchor) -> bool,
    edge_visible: impl Fn(&sketch::SketchEdge) -> bool,
    node_visible: impl Fn(&sketch::SketchNode) -> bool,
) -> Option<WireDragTarget<'a>> {
    if let Some(anchor) = graph
        .pin_anchors
        .iter()
        .find(|anchor| anchor_visible(anchor) && anchor.pos.distance(position) <= 10.0)
    {
        return Some(WireDragTarget::Pin(anchor));
    }
    if let Some(SketchSelection::Net(net_id)) = graph
        .nodes
        .iter()
        .find(|node| node_visible(node) && node.rect.contains(position))
        .map(|node| &node.selection)
    {
        return Some(WireDragTarget::Net(net_id.as_str()));
    }
    hit_test_wire(graph, position)
        .filter(|edge| edge_visible(edge))
        .map(|edge| WireDragTarget::Net(edge.net_id.as_str()))
}

pub(super) fn schematic_canvas_size(available: egui::Vec2) -> egui::Vec2 {
    egui::vec2(available.x.max(560.0), available.y.max(520.0))
}

fn sketch_hover_tooltip(ui: &mut egui::Ui, node: &sketch::SketchNode, runtime_lines: &[String]) {
    ui.strong(&node.label);
    ui.label(&node.detail);
    ui.separator();
    ui.label("Runtime probes");
    if runtime_lines.is_empty() {
        ui.label("No matching waveform probe is loaded for this node.");
    } else {
        for line in runtime_lines {
            ui.monospace(line);
        }
    }
}

fn sketch_pin_hover_tooltip(ui: &mut egui::Ui, anchor: &sketch::SketchPinAnchor) {
    ui.strong(format!("{}.{}", anchor.component_id, anchor.pin));
    ui.label(format!("net: {}", anchor.net));
    ui.separator();
    ui.label("Click this pin, then click another pin or net node to wire it.");
}

fn sketch_wire_hover_tooltip(ui: &mut egui::Ui, edge: &sketch::SketchEdge) {
    ui.strong(format!("net {}", edge.net_id));
    ui.label(format!("source: {}", edge.source));
    ui.separator();
    ui.label("Click this wire to select the net; start wire mode first to connect to it.");
}

pub(super) fn component_context_pin(
    snapshot: &ProjectSnapshot,
    component_id: &str,
    preferred_pin: &str,
) -> (String, String) {
    let Some(component) = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == component_id)
    else {
        return ("P1".to_string(), String::new());
    };
    if let Some(pin) = component
        .pins
        .iter()
        .find(|pin| pin.pin == preferred_pin.trim())
    {
        return (pin.pin.clone(), pin.net.clone());
    }
    component
        .pins
        .first()
        .map(|pin| (pin.pin.clone(), pin.net.clone()))
        .unwrap_or_else(|| ("P1".to_string(), String::new()))
}

fn sketch_probe_badge_tooltip(
    ui: &mut egui::Ui,
    badge: &SketchProbeBadge,
    status: SketchProbeStatus,
    sampled_value: Option<f64>,
) {
    ui.strong(format!(
        "{} probe {}",
        badge.probe.quantity.label(),
        badge.probe.probe_name
    ));
    ui.label(format!("scenario: {}", badge.probe.scenario_name));
    ui.label(format!("expression: {}", badge.probe.expression));
    ui.label(format!("assertion status: {}", status.label()));
    if let Some(value) = sampled_value {
        ui.label(format!("cursor sample: {:.6}", value));
    } else {
        ui.label("cursor sample: no matching loaded waveform");
    }
    if !badge.probe.assertion_names.is_empty() {
        ui.label(format!(
            "assertions: {}",
            badge.probe.assertion_names.join(", ")
        ));
    }
    ui.separator();
    ui.label("Click to open this probe in the Simulation stage.");
    ui.label("Right-click to open probe actions.");
    ui.label("Press A while hovering to add an assertion from current settings.");
    ui.label("Press Shift+A while hovering to require above the cursor sample.");
    ui.label("Press Shift+B while hovering to require below the cursor sample.");
    ui.label("Press X while hovering to clear assertions for this probe.");
    ui.label("Press Delete or Backspace while hovering to remove it.");
}

fn draw_wire_edge(
    painter: &egui::Painter,
    edge: &sketch::SketchEdge,
    selected: bool,
    hovered: bool,
    zoom: f32,
    opacity: f32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let color = if selected {
        egui::Color32::from_rgb(93, 185, 255)
    } else if hovered {
        egui::Color32::from_rgb(255, 196, 87)
    } else {
        egui::Color32::from_gray(86)
    };
    let stroke_width = if selected || hovered { 2.0 } else { 1.0 };
    let color = with_opacity(color, opacity);
    let stroke = egui::Stroke::new(stroke_width, color);
    let points = orthogonal_wire_points(edge.start, edge.end);
    draw_wire_points(painter, &points, stroke);
    draw_wire_junctions(painter, &points, color, selected || hovered);
    if zoom > 0.45 || selected || hovered {
        draw_wire_label(painter, edge, selected || hovered, opacity);
    }
}

fn draw_wire_polyline(
    painter: &egui::Painter,
    start: egui::Pos2,
    end: egui::Pos2,
    stroke: egui::Stroke,
) {
    let points = orthogonal_wire_points(start, end);
    draw_wire_points(painter, &points, stroke);
}

fn draw_wire_points(painter: &egui::Painter, points: &[egui::Pos2], stroke: egui::Stroke) {
    for segment in points.windows(2) {
        painter.line_segment([segment[0], segment[1]], stroke);
    }
}

fn draw_wire_junctions(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    color: egui::Color32,
    emphasized: bool,
) {
    let radius = if emphasized { 3.0 } else { 2.2 };
    for point in points {
        painter.circle_filled(*point, radius, color);
    }
}

fn draw_wire_label(
    painter: &egui::Painter,
    edge: &sketch::SketchEdge,
    emphasized: bool,
    opacity: f32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let label = compact_wire_label(&edge.net_id);
    let pos = edge_label_position(edge);
    let width = (label.len() as f32 * 7.0 + 8.0).clamp(24.0, 128.0);
    let rect = egui::Rect::from_min_size(pos + egui::vec2(-4.0, -9.0), egui::vec2(width, 18.0));
    let fill = if emphasized {
        egui::Color32::from_rgba_unmultiplied(30, 48, 58, 232)
    } else {
        egui::Color32::from_rgba_unmultiplied(24, 24, 24, 210)
    };
    painter.rect_filled(rect, 2.0, with_opacity(fill, opacity));
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, with_opacity(egui::Color32::from_gray(76), opacity)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(10.0),
        with_opacity(egui::Color32::from_gray(225), opacity),
    );
}

fn compact_wire_label(label: &str) -> String {
    const MAX_CHARS: usize = 18;
    if label.chars().count() <= MAX_CHARS {
        return label.to_string();
    }
    let mut compact = label.chars().take(MAX_CHARS - 3).collect::<String>();
    compact.push_str("...");
    compact
}

fn wire_preview_start(
    graph: &sketch::SketchGraph,
    component_id: &str,
    pin_id: &str,
) -> Option<egui::Pos2> {
    let pin_id = pin_id.trim();
    graph
        .pin_anchors
        .iter()
        .find(|anchor| anchor.component_id == component_id && anchor.pin == pin_id)
        .map(|anchor| anchor.pos)
        .or_else(|| {
            graph
                .nodes
                .iter()
                .find(|node| node.selection == SketchSelection::Component(component_id.to_string()))
                .map(|node| node.rect.center())
        })
}

#[cfg(test)]
mod tests {
    use super::{WireDragTarget, wire_drag_target_at, zoom_viewport_around};
    use crate::gui::sketch::{
        SketchEdge, SketchGraph, SketchNode, SketchPinAnchor, SketchSelection,
    };
    use crate::gui::sketch_probes::SketchProbeBadge;
    use crate::gui::sketch_symbols::SketchSymbolKind;
    use eframe::egui;

    #[test]
    fn zoom_viewport_keeps_pointer_logical_focus_stable() {
        let canvas = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let focus = egui::pos2(300.0, 200.0);
        let pan = egui::vec2(40.0, -20.0);
        let old_zoom = 1.25;
        let logical_before = (focus - canvas.min - pan) / old_zoom;

        let (new_zoom, new_pan) = zoom_viewport_around(old_zoom, pan, 1.4, canvas, focus);
        let logical_after = (focus - canvas.min - new_pan) / new_zoom;

        assert!((logical_before.x - logical_after.x).abs() < 1e-3);
        assert!((logical_before.y - logical_after.y).abs() < 1e-3);
    }

    #[test]
    fn wire_drag_target_prefers_pin_then_net_then_wire() {
        let graph = SketchGraph {
            nodes: vec![SketchNode {
                selection: SketchSelection::Net("net_mid".to_string()),
                label: "net_mid".to_string(),
                detail: String::new(),
                symbol: SketchSymbolKind::Net,
                style: Default::default(),
                rect: egui::Rect::from_min_size(egui::pos2(90.0, 90.0), egui::vec2(80.0, 40.0)),
            }],
            pin_anchors: vec![SketchPinAnchor {
                component_id: "R1".to_string(),
                pin: "A".to_string(),
                net: "net_a".to_string(),
                pos: egui::pos2(100.0, 100.0),
                label_pos: egui::pos2(104.0, 100.0),
                label_align: egui::Align2::LEFT_CENTER,
            }],
            edges: vec![SketchEdge {
                net_id: "wire_net".to_string(),
                source: "R2.B".to_string(),
                start: egui::pos2(20.0, 200.0),
                end: egui::pos2(220.0, 200.0),
            }],
            probe_badges: Vec::<SketchProbeBadge>::new(),
        };

        match wire_drag_target_at(
            &graph,
            egui::pos2(100.0, 100.0),
            |_| true,
            |_| true,
            |_| true,
        ) {
            Some(WireDragTarget::Pin(anchor)) => assert_eq!(anchor.pin, "A"),
            _ => panic!("expected pin target"),
        }
        match wire_drag_target_at(
            &graph,
            egui::pos2(130.0, 110.0),
            |_| true,
            |_| true,
            |_| true,
        ) {
            Some(WireDragTarget::Net(net)) => assert_eq!(net, "net_mid"),
            _ => panic!("expected net target"),
        }
        match wire_drag_target_at(
            &graph,
            egui::pos2(120.0, 202.0),
            |_| true,
            |_| true,
            |_| true,
        ) {
            Some(WireDragTarget::Net(net)) => assert_eq!(net, "wire_net"),
            _ => panic!("expected wire net target"),
        }
    }
}
