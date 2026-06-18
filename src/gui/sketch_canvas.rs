use eframe::egui;

use super::sketch::{
    self, ProjectSnapshot, SketchSelection, draw_sketch_grid, draw_sketch_node,
    draw_sketch_pin_anchor, edge_label_position, edit_schematic_wire_route, hit_test_wire,
    layout_sketch_graph_viewport, persisted_node_position_from_screen_with_snap,
    persisted_wire_route_point_from_screen_with_snap, remove_schematic_wire_route,
    sketch_wire_points, snap_screen_point_to_grid, with_opacity,
};
use super::sketch_inspector::{
    default_current_probe_name_for_component, default_power_probe_name_for_component,
    default_probe_name_for_net,
};
use super::sketch_probes::{
    SketchProbeBadge, SketchProbeStatus, draw_probe_badge, hit_test_probe_badge,
    probe_assertion_status,
};
use super::sketch_routes;
use super::waveform::{
    runtime_probe_activity_for_selection, runtime_probe_lines_for_selection,
    waveform_probe_value_for_badge,
};
use super::{
    CircuitCiApp, analog, sketch_bundles, sketch_connectivity, sketch_hierarchy, sketch_net_labels,
};

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
        let net_label_badges = sketch_net_labels::layout_net_label_badges(snapshot, rect, viewport);
        let connectivity_highlight =
            sketch_connectivity::SketchConnectivityHighlight::from_selection(
                self.selected_sketch_items
                    .iter()
                    .chain(self.selected_sketch_item.iter()),
            );
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
        let hovered_route_handle = if hovered_node.is_none() && hovered_anchor.is_none() {
            pointer_hover.and_then(|position| {
                hit_test_wire_route_handle(&graph, position).filter(|(edge, _)| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.edge_visible(edge))
                })
            })
        } else {
            None
        };
        let hovered_wire =
            if hovered_node.is_none() && hovered_anchor.is_none() && hovered_route_handle.is_none()
            {
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
        let hovered_net_label_badge = pointer_hover.and_then(|position| {
            sketch_net_labels::hit_test_net_label_badge(&net_label_badges, position).filter(
                |badge| {
                    hierarchy_view.as_ref().is_none_or(|view| {
                        view.interaction_visible(&SketchSelection::Net(badge.net_id.clone()))
                    })
                },
            )
        });
        let blank_canvas_hovered = hovered_node.is_none()
            && hovered_anchor.is_none()
            && hovered_route_handle.is_none()
            && hovered_wire.is_none()
            && hovered_probe_badge.is_none()
            && hovered_bundle_badge.is_none()
            && hovered_hierarchy_connector_badge.is_none()
            && hovered_net_label_badge.is_none();
        self.handle_sketch_viewport_input(
            ui,
            rect,
            &response,
            blank_canvas_hovered
                && !self.sketch_palette_place_armed
                && !self.sketch_library_place_armed
                && !self.sketch_net_label_place_armed,
        );
        let viewport = self.sketch_viewport();
        let graph = layout_sketch_graph_viewport(rect, snapshot, viewport);
        let hierarchy_connector_badges = hierarchy_view
            .as_ref()
            .map(|view| sketch_hierarchy::layout_hierarchy_connector_badges(snapshot, &graph, view))
            .unwrap_or_default();
        let bundle_badges = sketch_bundles::layout_net_bundle_badges(snapshot, &graph);
        let net_label_badges = sketch_net_labels::layout_net_label_badges(snapshot, rect, viewport);
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
        let hovered_route_handle = if hovered_node.is_none() && hovered_anchor.is_none() {
            pointer_hover.and_then(|position| {
                hit_test_wire_route_handle(&graph, position).filter(|(edge, _)| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.edge_visible(edge))
                })
            })
        } else {
            None
        };
        let hovered_wire =
            if hovered_node.is_none() && hovered_anchor.is_none() && hovered_route_handle.is_none()
            {
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
        let hovered_net_label_badge = pointer_hover.and_then(|position| {
            sketch_net_labels::hit_test_net_label_badge(&net_label_badges, position).filter(
                |badge| {
                    hierarchy_view.as_ref().is_none_or(|view| {
                        view.interaction_visible(&SketchSelection::Net(badge.net_id.clone()))
                    })
                },
            )
        });
        let placement_target_clear = hovered_node.is_none()
            && hovered_anchor.is_none()
            && hovered_wire.is_none()
            && hovered_probe_badge.is_none()
            && hovered_bundle_badge.is_none()
            && hovered_hierarchy_connector_badge.is_none()
            && hovered_net_label_badge.is_none();
        let wire_drag_target = if let Some(component_id) = &self.wire_from_component
            && let Some(position) = pointer_hover
            && rect.contains(position)
            && hierarchy_view.as_ref().is_none_or(|view| {
                view.interaction_visible(&SketchSelection::Component(component_id.clone()))
            }) {
            wire_drag_target_at(
                &graph,
                &net_label_badges,
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
                |badge| {
                    hierarchy_view.as_ref().is_none_or(|view| {
                        view.interaction_visible(&SketchSelection::Net(badge.net_id.clone()))
                    })
                },
            )
            .filter(|target| !target.is_source_pin(component_id.as_str(), self.wire_pin_id.trim()))
        } else {
            None
        };
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
            let target = wire_drag_target
                .as_ref()
                .is_some_and(|target| target.matches_edge(edge));
            draw_wire_edge(
                &painter,
                edge,
                selected,
                hovered || target,
                self.sketch_zoom,
                opacity,
            );
            if selected || hovered || target || !edge.route.is_empty() {
                draw_wire_route_handles(&painter, edge, hovered_route_handle, opacity);
            }
        }
        if let Some(drag) = &self.sketch_wire_route_drag
            && let Some(edge) = graph
                .edges
                .iter()
                .find(|edge| edge.net_id == drag.net_id && edge.source == drag.source)
        {
            draw_wire_route_preview(&painter, edge, &drag.points);
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
            let end = wire_drag_target
                .as_ref()
                .map(WireDragTarget::snap_position)
                .unwrap_or(pointer);
            let route_points = self.sketch_wire_draft.screen_points(rect, viewport);
            let points = sketch_routes::wire_points(source, &route_points, end);
            draw_wire_points(
                &painter,
                &points,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 196, 87)),
            );
            draw_wire_junctions(
                &painter,
                &points,
                egui::Color32::from_rgb(255, 196, 87),
                true,
            );
            draw_pending_wire_route_handles(&painter, &route_points);
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
        for badge in &net_label_badges {
            let dragged = self
                .sketch_net_label_drag
                .as_ref()
                .filter(|drag| drag.label_id == badge.id);
            let badge = if let Some(drag) = dragged {
                let mut badge = badge.clone();
                badge.rect = egui::Rect::from_center_size(drag.current_center, badge.rect.size());
                std::borrow::Cow::Owned(badge)
            } else {
                std::borrow::Cow::Borrowed(badge)
            };
            let badge = badge.as_ref();
            let selection = SketchSelection::Net(badge.net_id.clone());
            let opacity = if let Some(view) = &hierarchy_view {
                if !view.interaction_visible(&selection) {
                    continue;
                }
                view.selection_opacity(&selection)
            } else {
                1.0
            };
            if opacity <= 0.0 {
                continue;
            }
            let hovered = hovered_net_label_badge.is_some_and(|hovered| hovered.id == badge.id);
            let selected = self.selection_is_selected(&selection);
            sketch_net_labels::draw_net_label_badge(&painter, badge, hovered, selected, opacity);
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
            let connected = connectivity_highlight.anchor_connected(anchor);
            let target = wire_drag_target.as_ref().is_some_and(|target| {
                matches!(
                    target,
                    WireDragTarget::Pin {
                        component_id,
                        pin,
                        ..
                    } if component_id == &anchor.component_id && pin == &anchor.pin
                )
            });
            draw_sketch_pin_anchor(&painter, anchor, active || target || connected, opacity);
        }
        if let Some(target) = &wire_drag_target {
            draw_wire_drag_target(&painter, target);
        }
        if (self.sketch_palette_place_armed
            || self.sketch_library_place_armed
            || self.sketch_net_label_place_armed)
            && let Some(pointer) = pointer_hover
            && rect.contains(pointer)
        {
            let label = self.canvas_placement_label();
            let ghost = placement_ghost_rect(
                rect,
                pointer,
                viewport,
                self.sketch_snap_enabled,
                self.sketch_grid_step,
                placement_ghost_size(&label),
            );
            draw_placement_ghost(&painter, ghost, &label, placement_target_clear);
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

        let mut placement_applied = false;
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
            let clicked_net_label_badge =
                sketch_net_labels::hit_test_net_label_badge(&net_label_badges, position).filter(
                    |badge| {
                        hierarchy_view.as_ref().is_none_or(|view| {
                            view.interaction_visible(&SketchSelection::Net(badge.net_id.clone()))
                        })
                    },
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
            if (self.sketch_palette_place_armed
                || self.sketch_library_place_armed
                || self.sketch_net_label_place_armed)
                && placement_target_clear
            {
                if self.sketch_palette_place_armed {
                    self.apply_insert_sketch_primitive_at(rect, position);
                } else if self.sketch_library_place_armed {
                    self.apply_insert_selected_library_model_at(rect, position);
                } else {
                    self.apply_add_or_create_schematic_net_label_at(rect, viewport, position);
                }
                placement_applied = true;
            } else if let Some(badge) = clicked_probe_badge {
                self.open_probe_badge_in_simulation(badge);
            } else if let Some(badge) = clicked_hierarchy_connector_badge {
                self.set_single_sketch_selection(Some(SketchSelection::Net(badge.net_id.clone())));
                self.status = format!(
                    "Selected net {} with {} off-sheet endpoint(s).",
                    badge.net_id,
                    badge.external_targets.len()
                );
            } else if let Some(badge) = clicked_net_label_badge {
                if let Some(component_id) = self.wire_from_component.clone() {
                    let route_points = self.pending_wire_route_points();
                    self.apply_visual_wire_with_route(
                        component_id,
                        badge.net_id.clone(),
                        route_points,
                    );
                } else if response.double_clicked_by(egui::PointerButton::Primary) {
                    self.begin_net_label_inline_edit(badge);
                } else {
                    self.set_single_sketch_selection(Some(SketchSelection::Net(
                        badge.net_id.clone(),
                    )));
                    self.status = format!("Selected net {} from schematic label.", badge.net_id);
                }
            } else if let Some(badge) = clicked_bundle_badge {
                self.select_net_bundle(&badge.bundle);
            } else if let Some(anchor) = clicked_anchor {
                if let Some(source_component_id) = self.wire_from_component.clone()
                    && !(source_component_id == anchor.component_id
                        && self.wire_pin_id.trim() == anchor.pin)
                {
                    let route_points = self.pending_wire_route_points();
                    self.apply_visual_pin_wire_with_route(
                        source_component_id,
                        anchor.component_id.clone(),
                        anchor.pin.clone(),
                        route_points,
                    );
                } else {
                    self.start_visual_wire_from_anchor(anchor);
                }
            } else if let Some(SketchSelection::Net(net_id)) = &clicked
                && let Some(component_id) = self.wire_from_component.clone()
            {
                let route_points = self.pending_wire_route_points();
                self.apply_visual_wire_with_route(component_id, net_id.clone(), route_points);
            } else if let Some(edge) = clicked_wire
                && let Some(component_id) = self.wire_from_component.clone()
            {
                let route_points = self.pending_wire_route_points();
                self.apply_visual_wire_with_route(component_id, edge.net_id.clone(), route_points);
            } else if self.wire_from_component.is_some() && placement_target_clear {
                self.add_pending_wire_route_point(rect, viewport, position);
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
        if !placement_applied
            && (self.sketch_palette_place_armed
                || self.sketch_library_place_armed
                || self.sketch_net_label_place_armed)
            && placement_target_clear
            && ui.input(|input| input.pointer.any_released())
            && let Some(position) = pointer_hover
            && rect.contains(position)
        {
            if self.sketch_palette_place_armed {
                self.apply_insert_sketch_primitive_at(rect, position);
            } else if self.sketch_library_place_armed {
                self.apply_insert_selected_library_model_at(rect, position);
            } else {
                self.apply_add_or_create_schematic_net_label_at(rect, viewport, position);
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
            let clicked_net_label_badge =
                sketch_net_labels::hit_test_net_label_badge(&net_label_badges, position).filter(
                    |badge| {
                        hierarchy_view.as_ref().is_none_or(|view| {
                            view.interaction_visible(&SketchSelection::Net(badge.net_id.clone()))
                        })
                    },
                );
            let clicked_wire = if clicked_anchor.is_none()
                && clicked_node.is_none()
                && clicked_net_label_badge.is_none()
            {
                hit_test_wire(&graph, position).filter(|edge| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.edge_visible(edge))
                })
            } else {
                None
            };
            if let Some(anchor) = clicked_anchor {
                self.start_visual_wire_from_anchor(anchor);
            } else if self.wire_from_component.is_none()
                && !self.sketch_palette_place_armed
                && !self.sketch_library_place_armed
                && !self.sketch_net_label_place_armed
                && let Some((edge, point_index)) = hovered_route_handle
            {
                self.sketch_wire_route_drag = Some(super::SketchWireRouteDrag {
                    net_id: edge.net_id.clone(),
                    source: edge.source.clone(),
                    points: edge.route.clone(),
                    point_index,
                });
                self.set_single_sketch_selection(Some(SketchSelection::Net(edge.net_id.clone())));
            } else if self.wire_from_component.is_none()
                && !self.sketch_palette_place_armed
                && !self.sketch_library_place_armed
                && !self.sketch_net_label_place_armed
                && let Some(badge) = clicked_net_label_badge
            {
                self.sketch_net_label_drag = Some(super::SketchNetLabelDrag {
                    label_id: badge.id.clone(),
                    net_id: badge.net_id.clone(),
                    current_center: badge.rect.center(),
                });
                self.set_single_sketch_selection(Some(SketchSelection::Net(badge.net_id.clone())));
            } else if self.wire_from_component.is_none()
                && !self.sketch_palette_place_armed
                && !self.sketch_library_place_armed
                && !self.sketch_net_label_place_armed
                && let Some(edge) = clicked_wire
            {
                let preview = snap_screen_point_to_grid(
                    rect,
                    position,
                    viewport,
                    self.sketch_snap_enabled,
                    self.sketch_grid_step,
                );
                let mut points = edge.route.clone();
                let point_index = wire_route_insert_index(edge, preview);
                points.insert(point_index, preview);
                self.sketch_wire_route_drag = Some(super::SketchWireRouteDrag {
                    net_id: edge.net_id.clone(),
                    source: edge.source.clone(),
                    points,
                    point_index,
                });
                self.set_single_sketch_selection(Some(SketchSelection::Net(edge.net_id.clone())));
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
            && let Some(position) = response.interact_pointer_pos()
            && let Some(drag) = &mut self.sketch_wire_route_drag
        {
            if let Some(point) = drag.points.get_mut(drag.point_index) {
                *point = snap_screen_point_to_grid(
                    rect,
                    position,
                    viewport,
                    self.sketch_snap_enabled,
                    self.sketch_grid_step,
                );
            }
        } else if response.dragged_by(egui::PointerButton::Primary)
            && let Some(position) = response.interact_pointer_pos()
            && let Some(drag) = &mut self.sketch_net_label_drag
        {
            drag.current_center = snap_screen_point_to_grid(
                rect,
                position,
                viewport,
                self.sketch_snap_enabled,
                self.sketch_grid_step,
            );
        } else if response.dragged_by(egui::PointerButton::Primary)
            && !self.sketch_pan_drag_active
            && self.wire_from_component.is_none()
            && self.sketch_wire_route_drag.is_none()
            && self.sketch_net_label_drag.is_none()
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
            && let Some(drag) = self.sketch_wire_route_drag.take()
        {
            self.apply_schematic_wire_route(rect, viewport, drag);
        }
        if response.drag_stopped_by(egui::PointerButton::Primary)
            && let Some(drag) = self.sketch_net_label_drag.take()
        {
            self.apply_move_schematic_net_label_to(
                rect,
                viewport,
                &drag.label_id,
                &drag.net_id,
                drag.current_center,
            );
        }
        if response.drag_stopped_by(egui::PointerButton::Primary)
            && self.wire_from_component.is_some()
            && let Some(position) = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().pointer_hover_pos())
            && let Some(target) = wire_drag_target_at(
                &graph,
                &net_label_badges,
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
                |badge| {
                    hierarchy_view.as_ref().is_none_or(|view| {
                        view.interaction_visible(&SketchSelection::Net(badge.net_id.clone()))
                    })
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
            && (self.sketch_palette_place_armed
                || self.sketch_library_place_armed
                || self.sketch_net_label_place_armed)
        {
            self.sketch_palette_place_armed = false;
            self.sketch_library_place_armed = false;
            self.sketch_net_label_place_armed = false;
            self.status = "Canvas placement canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.wire_from_component.is_some() {
            self.wire_from_component = None;
            self.sketch_wire_draft.clear();
            self.status = "Wire mode canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.sketch_wire_route_drag.is_some() {
            self.sketch_wire_route_drag = None;
            self.status = "Wire route edit canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.sketch_net_label_drag.is_some() {
            self.sketch_net_label_drag = None;
            self.status = "Net label move canceled.".to_string();
        } else if delete_pressed
            && self.wire_from_component.is_some()
            && !self.sketch_wire_draft.is_empty()
        {
            self.remove_last_pending_wire_route_point();
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
        } else if let Some(badge) = hovered_net_label_badge {
            response.context_menu(|ui| {
                self.net_label_context_menu(ui, badge, &net_label_badges, rect);
            });
            response.on_hover_ui(|ui| {
                sketch_net_labels::net_label_tooltip(ui, badge);
            });
        } else if let Some(node) = hovered_node {
            response.context_menu(|ui| {
                self.sketch_node_context_menu(ui, node, snapshot, rect, viewport, pointer_hover);
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
        } else if let Some((edge, index)) = hovered_route_handle {
            response.context_menu(|ui| {
                self.sketch_wire_context_menu(ui, rect, viewport, edge, pointer_hover, Some(index));
            });
            response.on_hover_ui(|ui| {
                sketch_wire_route_handle_tooltip(ui, edge, index);
            });
        } else if let Some(edge) = hovered_wire {
            response.context_menu(|ui| {
                self.sketch_wire_context_menu(ui, rect, viewport, edge, pointer_hover, None);
            });
            response.on_hover_ui(|ui| {
                sketch_wire_hover_tooltip(ui, edge);
            });
        } else {
            response.context_menu(|ui| {
                self.sketch_canvas_context_menu(ui, rect, pointer_hover);
            });
            if self.sketch_palette_place_armed
                || self.sketch_library_place_armed
                || self.sketch_net_label_place_armed
            {
                let label = if self.sketch_palette_place_armed {
                    self.sketch_palette_kind.label()
                } else if self.sketch_net_label_place_armed {
                    self.sketch_net_label_kind.label()
                } else {
                    self.selected_library_model.as_str()
                };
                response.on_hover_text(format!(
                    "Click blank canvas to place {label}. Press Esc to cancel."
                ));
            }
        }
        self.sketch_net_label_inline_editor(ui, &net_label_badges, snapshot);
    }

    fn start_visual_wire_from_anchor(&mut self, anchor: &sketch::SketchPinAnchor) {
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

    fn apply_wire_drag_target(&mut self, target: WireDragTarget) {
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

    fn add_pending_wire_route_point(
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

    fn remove_last_pending_wire_route_point(&mut self) {
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

    fn pending_wire_route_points(&self) -> Vec<(f64, f64)> {
        self.sketch_wire_draft.points().to_vec()
    }

    fn apply_schematic_wire_route(
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

    fn canvas_placement_label(&self) -> String {
        if self.sketch_palette_place_armed {
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

pub(super) fn zoom_viewport_around(
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

#[derive(Debug, Clone, PartialEq)]
pub(super) enum WireDragTarget {
    Pin {
        component_id: String,
        pin: String,
        net: String,
        pos: egui::Pos2,
    },
    NetNode {
        net_id: String,
        rect: egui::Rect,
    },
    NetLabel {
        net_id: String,
        label_id: String,
        rect: egui::Rect,
    },
    Wire {
        net_id: String,
        source: String,
        start: egui::Pos2,
        end: egui::Pos2,
        snap: egui::Pos2,
    },
}

impl WireDragTarget {
    fn snap_position(&self) -> egui::Pos2 {
        match self {
            WireDragTarget::Pin { pos, .. } => *pos,
            WireDragTarget::NetNode { rect, .. } => rect.center(),
            WireDragTarget::NetLabel { rect, .. } => rect.center(),
            WireDragTarget::Wire { snap, .. } => *snap,
        }
    }

    fn is_source_pin(&self, component_id: &str, pin: &str) -> bool {
        matches!(
            self,
            WireDragTarget::Pin {
                component_id: target_component_id,
                pin: target_pin,
                ..
            } if target_component_id == component_id && target_pin == pin
        )
    }

    fn matches_edge(&self, edge: &sketch::SketchEdge) -> bool {
        matches!(
            self,
            WireDragTarget::Wire {
                net_id,
                source,
                ..
            } if net_id == &edge.net_id && source == &edge.source
        )
    }
}

pub(super) fn wire_drag_target_at(
    graph: &sketch::SketchGraph,
    net_label_badges: &[sketch_net_labels::SketchNetLabelBadge],
    position: egui::Pos2,
    anchor_visible: impl Fn(&sketch::SketchPinAnchor) -> bool,
    edge_visible: impl Fn(&sketch::SketchEdge) -> bool,
    node_visible: impl Fn(&sketch::SketchNode) -> bool,
    label_visible: impl Fn(&sketch_net_labels::SketchNetLabelBadge) -> bool,
) -> Option<WireDragTarget> {
    if let Some(anchor) = graph
        .pin_anchors
        .iter()
        .find(|anchor| anchor_visible(anchor) && anchor.pos.distance(position) <= 10.0)
    {
        return Some(WireDragTarget::Pin {
            component_id: anchor.component_id.clone(),
            pin: anchor.pin.clone(),
            net: anchor.net.clone(),
            pos: anchor.pos,
        });
    }
    if let Some(badge) = sketch_net_labels::hit_test_net_label_badge(net_label_badges, position)
        .filter(|badge| label_visible(badge))
    {
        return Some(WireDragTarget::NetLabel {
            net_id: badge.net_id.clone(),
            label_id: badge.id.clone(),
            rect: badge.rect,
        });
    }
    if let Some(node) = graph
        .nodes
        .iter()
        .find(|node| node_visible(node) && node.rect.contains(position))
        && let SketchSelection::Net(net_id) = &node.selection
    {
        return Some(WireDragTarget::NetNode {
            net_id: net_id.clone(),
            rect: node.rect,
        });
    }
    hit_test_wire(graph, position)
        .filter(|edge| edge_visible(edge))
        .map(|edge| WireDragTarget::Wire {
            net_id: edge.net_id.clone(),
            source: edge.source.clone(),
            start: edge.start,
            end: edge.end,
            snap: closest_point_on_edge(position, edge),
        })
}

pub(super) fn closest_point_on_edge(position: egui::Pos2, edge: &sketch::SketchEdge) -> egui::Pos2 {
    sketch_routes::closest_point_on_polyline(position, &sketch_wire_points(edge))
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
    ui.label("Click this wire to select the net; drag it to shape the schematic route.");
    ui.label("Start wire mode first to connect another pin to it.");
    if !edge.route.is_empty() {
        ui.label("Right-click to insert route handles or clear the custom schematic route.");
    } else {
        ui.label("Right-click to insert a route handle at the pointer.");
    }
}

fn sketch_wire_route_handle_tooltip(ui: &mut egui::Ui, edge: &sketch::SketchEdge, index: usize) {
    ui.strong(format!("wire route handle {}", index + 1));
    ui.label(format!("net: {}", edge.net_id));
    ui.label(format!("source: {}", edge.source));
    ui.separator();
    ui.label("Drag this handle to refine the schematic route.");
    ui.label("Right-click to delete this handle or clear the custom route.");
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
    let points = sketch_wire_points(edge);
    draw_wire_points(painter, &points, stroke);
    draw_wire_junctions(painter, &points, color, selected || hovered);
    if zoom > 0.45 || selected || hovered {
        draw_wire_label(painter, edge, selected || hovered, opacity);
    }
}

fn draw_wire_route_handles(
    painter: &egui::Painter,
    edge: &sketch::SketchEdge,
    hovered: Option<(&sketch::SketchEdge, usize)>,
    opacity: f32,
) {
    if edge.route.is_empty() {
        return;
    }
    let opacity = opacity.clamp(0.0, 1.0);
    for (index, point) in edge.route.iter().enumerate() {
        let hovered = hovered.is_some_and(|(hovered_edge, hovered_index)| {
            hovered_index == index
                && hovered_edge.net_id == edge.net_id
                && hovered_edge.source == edge.source
        });
        let fill = if hovered {
            egui::Color32::from_rgb(255, 196, 87)
        } else {
            egui::Color32::from_rgb(32, 126, 223)
        };
        painter.circle_filled(
            *point,
            if hovered { 5.5 } else { 4.5 },
            with_opacity(fill, opacity),
        );
        painter.circle_stroke(
            *point,
            if hovered { 7.5 } else { 6.5 },
            egui::Stroke::new(1.5, with_opacity(egui::Color32::WHITE, opacity)),
        );
    }
}

fn draw_wire_route_preview(
    painter: &egui::Painter,
    edge: &sketch::SketchEdge,
    route_points: &[egui::Pos2],
) {
    let points = sketch_routes::wire_points(edge.start, route_points, edge.end);
    draw_wire_points(
        painter,
        &points,
        egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 196, 87)),
    );
    draw_wire_junctions(
        painter,
        &points,
        egui::Color32::from_rgb(255, 196, 87),
        true,
    );
}

fn draw_pending_wire_route_handles(painter: &egui::Painter, route_points: &[egui::Pos2]) {
    for point in route_points {
        painter.circle_filled(*point, 4.5, egui::Color32::from_rgb(255, 196, 87));
        painter.circle_stroke(
            *point,
            6.5,
            egui::Stroke::new(1.5, egui::Color32::from_rgb(36, 36, 36)),
        );
    }
}

pub(super) fn hit_test_wire_route_handle(
    graph: &sketch::SketchGraph,
    position: egui::Pos2,
) -> Option<(&sketch::SketchEdge, usize)> {
    graph
        .edges
        .iter()
        .flat_map(|edge| {
            edge.route
                .iter()
                .enumerate()
                .map(move |(index, point)| (edge, index, point.distance(position)))
        })
        .filter(|(_, _, distance)| *distance <= 8.0)
        .min_by(|(_, _, left), (_, _, right)| left.total_cmp(right))
        .map(|(edge, index, _)| (edge, index))
}

pub(super) fn wire_route_insert_index(edge: &sketch::SketchEdge, position: egui::Pos2) -> usize {
    sketch_routes::route_insert_index(edge.start, &edge.route, edge.end, position)
}

fn draw_wire_drag_target(painter: &egui::Painter, target: &WireDragTarget) {
    let color = egui::Color32::from_rgb(255, 196, 87);
    match target {
        WireDragTarget::Pin { pos, .. } => {
            painter.circle_filled(
                *pos,
                7.5,
                egui::Color32::from_rgba_unmultiplied(255, 196, 87, 44),
            );
            painter.circle_stroke(*pos, 9.0, egui::Stroke::new(2.0, color));
        }
        WireDragTarget::NetNode { rect, .. } => {
            let rect = rect.expand(5.0);
            painter.rect_filled(
                rect,
                4.0,
                egui::Color32::from_rgba_unmultiplied(255, 196, 87, 24),
            );
            painter.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(2.0, color),
                egui::StrokeKind::Inside,
            );
        }
        WireDragTarget::NetLabel { rect, .. } => {
            let rect = rect.expand(4.0);
            painter.rect_filled(
                rect,
                4.0,
                egui::Color32::from_rgba_unmultiplied(255, 196, 87, 28),
            );
            painter.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(2.0, color),
                egui::StrokeKind::Inside,
            );
        }
        WireDragTarget::Wire { snap, .. } => {
            painter.circle_filled(
                *snap,
                5.5,
                egui::Color32::from_rgba_unmultiplied(255, 196, 87, 80),
            );
            painter.circle_stroke(*snap, 7.0, egui::Stroke::new(2.0, color));
        }
    }
}

pub(super) fn placement_ghost_rect(
    canvas: egui::Rect,
    pointer: egui::Pos2,
    viewport: sketch::SketchViewport,
    snap_enabled: bool,
    grid_step: f32,
    size: egui::Vec2,
) -> egui::Rect {
    let center = snap_screen_point_to_grid(canvas, pointer, viewport, snap_enabled, grid_step);
    egui::Rect::from_center_size(center, size)
}

fn placement_ghost_size(label: &str) -> egui::Vec2 {
    let width = (label.chars().count() as f32 * 7.0 + 56.0).clamp(120.0, 220.0);
    egui::vec2(width, 72.0)
}

fn draw_placement_ghost(
    painter: &egui::Painter,
    rect: egui::Rect,
    label: &str,
    target_clear: bool,
) {
    let accent = if target_clear {
        egui::Color32::from_rgb(95, 190, 255)
    } else {
        egui::Color32::from_rgb(255, 116, 116)
    };
    let fill = if target_clear {
        egui::Color32::from_rgba_unmultiplied(38, 88, 112, 72)
    } else {
        egui::Color32::from_rgba_unmultiplied(112, 38, 38, 64)
    };
    painter.rect_filled(rect, 5.0, fill);
    painter.rect_stroke(
        rect,
        5.0,
        egui::Stroke::new(2.0, accent),
        egui::StrokeKind::Inside,
    );
    let pin_y = rect.center().y;
    painter.circle_filled(egui::pos2(rect.left(), pin_y), 4.0, accent);
    painter.circle_filled(egui::pos2(rect.right(), pin_y), 4.0, accent);
    let text = if target_clear {
        label.to_string()
    } else {
        format!("{label} blocked")
    };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        compact_placement_label(&text),
        egui::FontId::monospace(11.0),
        egui::Color32::from_gray(238),
    );
}

fn compact_placement_label(label: &str) -> String {
    const MAX_CHARS: usize = 22;
    if label.chars().count() <= MAX_CHARS {
        return label.to_string();
    }
    let mut compact = label.chars().take(MAX_CHARS - 3).collect::<String>();
    compact.push_str("...");
    compact
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
