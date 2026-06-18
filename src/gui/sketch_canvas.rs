use eframe::egui;

use super::sketch::{
    self, ProjectSnapshot, SketchNodeStyle, SketchPinSide, SketchSelection, draw_sketch_grid,
    draw_sketch_node, draw_sketch_pin_anchor, edit_schematic_component_styles,
    edit_schematic_wire_route, hit_test_wire, layout_sketch_graph, layout_sketch_graph_viewport,
    persisted_node_position_from_screen_with_snap,
    persisted_wire_route_point_from_screen_with_snap, remove_schematic_wire_route,
    snap_screen_point_to_grid,
};
use super::sketch_canvas_interaction::{
    SketchSelectionBoxMode, WireDragTarget, hit_test_wire_route_handle, schematic_canvas_size,
    wire_drag_target_at, wire_route_insert_index, zoom_viewport_around,
};
use super::sketch_canvas_interaction::{next_pin_side, normalize_canvas_rotation};
use super::sketch_canvas_render::{
    draw_pending_wire_route_handles, draw_placement_ghost, draw_wire_drag_target, draw_wire_edge,
    draw_wire_junctions, draw_wire_points, draw_wire_route_handles, draw_wire_route_preview,
    placement_ghost_rect, placement_ghost_size, sketch_hover_tooltip, sketch_pin_hover_tooltip,
    sketch_probe_badge_tooltip, sketch_wire_hover_tooltip, sketch_wire_route_handle_tooltip,
    wire_preview_start,
};
use super::sketch_inspector::{
    default_current_probe_name_for_component, default_power_probe_name_for_component,
    default_probe_name_for_net,
};
use super::sketch_probes::{draw_probe_badge, hit_test_probe_badge, probe_assertion_status};
use super::sketch_routes;
use super::waveform::{
    runtime_probe_activity_for_selection, runtime_probe_lines_for_selection,
    waveform_probe_value_for_badge,
};
use super::{
    CircuitCiApp, analog, sketch_bundles, sketch_component_labels, sketch_connectivity,
    sketch_hierarchy, sketch_minimap, sketch_net_labels,
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
        let minimap_graph = layout_sketch_graph(rect, snapshot);
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
        let component_label_badges = sketch_component_labels::layout_component_label_badges(
            snapshot,
            &graph,
            rect,
            viewport,
            self.sketch_reference_labels_visible,
            self.sketch_value_labels_visible,
        );
        let minimap = sketch_minimap::SketchMinimap::for_graph(rect, &minimap_graph);
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
        let pointer_over_minimap = pointer_hover
            .is_some_and(|position| minimap.is_some_and(|map| map.rect.contains(position)));
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
        let hovered_component_label_badge = pointer_hover.and_then(|position| {
            sketch_component_labels::hit_test_component_label_badge(
                &component_label_badges,
                position,
            )
            .filter(|badge| {
                hierarchy_view.as_ref().is_none_or(|view| {
                    view.interaction_visible(&SketchSelection::Component(
                        badge.component_id.clone(),
                    ))
                })
            })
        });
        let blank_canvas_hovered = hovered_node.is_none()
            && hovered_anchor.is_none()
            && hovered_route_handle.is_none()
            && hovered_wire.is_none()
            && hovered_probe_badge.is_none()
            && hovered_bundle_badge.is_none()
            && hovered_hierarchy_connector_badge.is_none()
            && hovered_net_label_badge.is_none()
            && hovered_component_label_badge.is_none()
            && !pointer_over_minimap;
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
        let component_label_badges = sketch_component_labels::layout_component_label_badges(
            snapshot,
            &graph,
            rect,
            viewport,
            self.sketch_reference_labels_visible,
            self.sketch_value_labels_visible,
        );
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
        let hovered_component_label_badge = pointer_hover.and_then(|position| {
            sketch_component_labels::hit_test_component_label_badge(
                &component_label_badges,
                position,
            )
            .filter(|badge| {
                hierarchy_view.as_ref().is_none_or(|view| {
                    view.interaction_visible(&SketchSelection::Component(
                        badge.component_id.clone(),
                    ))
                })
            })
        });
        let placement_target_clear = hovered_node.is_none()
            && hovered_anchor.is_none()
            && hovered_wire.is_none()
            && hovered_probe_badge.is_none()
            && hovered_bundle_badge.is_none()
            && hovered_hierarchy_connector_badge.is_none()
            && hovered_net_label_badge.is_none()
            && hovered_component_label_badge.is_none();
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
        for badge in &component_label_badges {
            let dragged = self
                .sketch_component_label_drag
                .as_ref()
                .filter(|drag| drag.component_id == badge.component_id && drag.kind == badge.kind);
            let badge = if let Some(drag) = dragged {
                let mut badge = badge.clone();
                badge.rect = egui::Rect::from_center_size(drag.current_center, badge.rect.size());
                std::borrow::Cow::Owned(badge)
            } else {
                std::borrow::Cow::Borrowed(badge)
            };
            let badge = badge.as_ref();
            let selection = SketchSelection::Component(badge.component_id.clone());
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
            let hovered = hovered_component_label_badge.is_some_and(|hovered| {
                hovered.component_id == badge.component_id && hovered.kind == badge.kind
            });
            let selected = self.selection_is_selected(&selection);
            sketch_component_labels::draw_component_label_badge(
                &painter, badge, hovered, selected, opacity,
            );
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
            let hovered = hovered_anchor.is_some_and(|hovered| {
                hovered.component_id == anchor.component_id && hovered.pin == anchor.pin
            });
            let component_selected = self
                .selection_is_selected(&SketchSelection::Component(anchor.component_id.clone()));
            draw_sketch_pin_anchor(
                &painter,
                anchor,
                active || target || connected,
                hovered || active || target || connected || component_selected,
                opacity,
            );
        }
        if let Some(target) = &wire_drag_target {
            draw_wire_drag_target(&painter, target);
        }
        if let Some(minimap) = minimap {
            minimap.draw(&painter, rect, &minimap_graph, viewport);
            let over_minimap =
                pointer_hover.is_some_and(|position| minimap.rect.contains(position));
            if over_minimap
                && ui.input(|input| input.pointer.primary_clicked() || input.pointer.primary_down())
                && let Some(position) = pointer_hover
            {
                self.sketch_pan = minimap.pan_for_focus(rect, viewport, position);
                self.status = "Schematic overview panned viewport.".to_string();
            }
        }
        if (self.sketch_palette_place_armed
            || self.sketch_library_place_armed
            || self.sketch_net_label_place_armed)
            && let Some(pointer) = pointer_hover
            && rect.contains(pointer)
        {
            let label = self.canvas_placement_label();
            let style = self.canvas_placement_style();
            let ghost = placement_ghost_rect(
                rect,
                pointer,
                viewport,
                self.sketch_snap_enabled,
                self.sketch_grid_step,
                placement_ghost_size(&label, style),
            );
            draw_placement_ghost(&painter, ghost, &label, placement_target_clear, style);
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
            && !pointer_over_minimap
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
            let clicked_component_label_badge =
                sketch_component_labels::hit_test_component_label_badge(
                    &component_label_badges,
                    position,
                )
                .filter(|badge| {
                    hierarchy_view.as_ref().is_none_or(|view| {
                        view.interaction_visible(&SketchSelection::Component(
                            badge.component_id.clone(),
                        ))
                    })
                });
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
            let clicked_wire = if clicked_anchor.is_none()
                && clicked_component_label_badge.is_none()
                && clicked.is_none()
            {
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
            } else if let Some(badge) = clicked_component_label_badge {
                if response.double_clicked_by(egui::PointerButton::Primary) {
                    match badge.kind {
                        sketch_component_labels::SketchComponentLabelKind::Reference => {
                            self.begin_component_id_inline_edit(&badge.component_id);
                        }
                        sketch_component_labels::SketchComponentLabelKind::Value => {
                            self.begin_component_value_inline_edit(snapshot, &badge.component_id);
                        }
                    }
                } else {
                    self.set_single_sketch_selection(Some(SketchSelection::Component(
                        badge.component_id.clone(),
                    )));
                    self.status = format!(
                        "Selected component {} from schematic label.",
                        badge.component_id
                    );
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
            } else if let Some(SketchSelection::Component(component_id)) = &clicked
                && response.double_clicked_by(egui::PointerButton::Primary)
            {
                self.begin_component_default_inline_edit(snapshot, component_id);
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
            && !pointer_over_minimap
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
            && !pointer_over_minimap
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
            let clicked_component_label_badge =
                sketch_component_labels::hit_test_component_label_badge(
                    &component_label_badges,
                    position,
                )
                .filter(|badge| {
                    hierarchy_view.as_ref().is_none_or(|view| {
                        view.interaction_visible(&SketchSelection::Component(
                            badge.component_id.clone(),
                        ))
                    })
                });
            let clicked_wire = if clicked_anchor.is_none()
                && clicked_node.is_none()
                && clicked_net_label_badge.is_none()
                && clicked_component_label_badge.is_none()
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
                && let Some(badge) = clicked_component_label_badge
            {
                self.sketch_component_label_drag = Some(super::SketchComponentLabelDrag {
                    component_id: badge.component_id.clone(),
                    kind: badge.kind,
                    current_center: badge.rect.center(),
                });
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    badge.component_id.clone(),
                )));
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
            } else if clicked_node.is_none()
                && let Some(mode) =
                    ui.input(|input| SketchSelectionBoxMode::from_modifiers(input.modifiers))
            {
                if ui.input(|input| input.key_down(egui::Key::L)) {
                    self.sketch_selection_lasso_drag = Some(super::SketchSelectionLassoDrag {
                        points: vec![position],
                        mode,
                    });
                } else {
                    self.sketch_selection_box_drag = Some(super::SketchSelectionBoxDrag {
                        start: position,
                        mode,
                    });
                }
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
        if let Some(selection_box) = &self.sketch_selection_box_drag
            && let Some(current) = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().pointer_hover_pos())
        {
            let marquee = egui::Rect::from_two_pos(selection_box.start, current);
            painter.rect_filled(marquee, 0.0, selection_box.mode.fill());
            painter.rect_stroke(
                marquee,
                0.0,
                egui::Stroke::new(1.0, selection_box.mode.stroke()),
                egui::StrokeKind::Inside,
            );
            painter.text(
                marquee.left_top() + egui::vec2(6.0, 6.0),
                egui::Align2::LEFT_TOP,
                selection_box.mode.label(),
                egui::FontId::monospace(11.0),
                selection_box.mode.stroke(),
            );
        } else if let Some(lasso) = &mut self.sketch_selection_lasso_drag
            && let Some(current) = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().pointer_hover_pos())
        {
            if lasso
                .points
                .last()
                .is_none_or(|point| point.distance(current) >= 3.0)
            {
                lasso.points.push(current);
            }
            if lasso.points.len() >= 2 {
                let stroke = egui::Stroke::new(1.5, lasso.mode.stroke());
                painter.add(egui::Shape::line(lasso.points.clone(), stroke));
                if let (Some(first), Some(last)) = (lasso.points.first(), lasso.points.last()) {
                    painter.line_segment([*last, *first], stroke);
                    painter.circle_filled(*first, 3.0, lasso.mode.stroke());
                }
                painter.text(
                    current + egui::vec2(8.0, 8.0),
                    egui::Align2::LEFT_TOP,
                    format!("{} lasso", lasso.mode.label()),
                    egui::FontId::monospace(11.0),
                    lasso.mode.stroke(),
                );
            }
        } else if response.dragged_by(egui::PointerButton::Primary)
            && let Some(position) = response.interact_pointer_pos()
            && let Some(drag) = &mut self.sketch_component_label_drag
        {
            drag.current_center = snap_screen_point_to_grid(
                rect,
                position,
                viewport,
                self.sketch_snap_enabled,
                self.sketch_grid_step,
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
            && self.sketch_component_label_drag.is_none()
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
            && let Some(selection_box) = self.sketch_selection_box_drag.take()
            && let Some(end) = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().pointer_hover_pos())
        {
            self.apply_marquee_selection(
                egui::Rect::from_two_pos(selection_box.start, end),
                &graph,
                selection_box.mode,
            );
        }
        if response.drag_stopped_by(egui::PointerButton::Primary)
            && let Some(mut lasso) = self.sketch_selection_lasso_drag.take()
        {
            if let Some(end) = response
                .interact_pointer_pos()
                .or_else(|| ui.ctx().pointer_hover_pos())
                && lasso
                    .points
                    .last()
                    .is_none_or(|point| point.distance(end) >= 3.0)
            {
                lasso.points.push(end);
            }
            self.apply_lasso_selection(&lasso.points, &graph, lasso.mode);
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
            && let Some(drag) = self.sketch_component_label_drag.take()
            && let Some(badge) = component_label_badges
                .iter()
                .find(|badge| badge.component_id == drag.component_id && badge.kind == drag.kind)
        {
            self.apply_move_schematic_component_label_to(
                rect,
                viewport,
                badge,
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
        let rotate_clockwise_pressed = response.hovered()
            && ui.input(|input| !input.modifiers.shift && input.key_pressed(egui::Key::R));
        let rotate_counter_clockwise_pressed = response.hovered()
            && ui.input(|input| input.modifiers.shift && input.key_pressed(egui::Key::R));
        let flip_pressed = response.hovered()
            && ui.input(|input| !input.modifiers.shift && input.key_pressed(egui::Key::F));
        let cycle_pin_side_pressed = response.hovered()
            && ui.input(|input| input.modifiers.shift && input.key_pressed(egui::Key::F));
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
        } else if cancel_canvas_mode_pressed && self.sketch_selection_box_drag.is_some() {
            self.sketch_selection_box_drag = None;
            self.status = "Selection box canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.sketch_selection_lasso_drag.is_some() {
            self.sketch_selection_lasso_drag = None;
            self.status = "Selection lasso canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.sketch_wire_route_drag.is_some() {
            self.sketch_wire_route_drag = None;
            self.status = "Wire route edit canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.sketch_net_label_drag.is_some() {
            self.sketch_net_label_drag = None;
            self.status = "Net label move canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.sketch_component_label_drag.is_some() {
            self.sketch_component_label_drag = None;
            self.status = "Component label move canceled.".to_string();
        } else if rotate_clockwise_pressed && self.component_placement_armed() {
            self.rotate_canvas_placement(90);
        } else if rotate_counter_clockwise_pressed && self.component_placement_armed() {
            self.rotate_canvas_placement(-90);
        } else if flip_pressed && self.component_placement_armed() {
            self.flip_canvas_placement();
        } else if cycle_pin_side_pressed && self.component_placement_armed() {
            self.cycle_canvas_placement_pin_side();
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
        } else if rotate_clockwise_pressed {
            self.apply_rotate_selected_sketch_components(90);
        } else if rotate_counter_clockwise_pressed {
            self.apply_rotate_selected_sketch_components(-90);
        } else if flip_pressed {
            self.apply_flip_selected_sketch_components();
        } else if cycle_pin_side_pressed {
            self.apply_cycle_selected_sketch_pin_side();
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
        } else if let Some(badge) = hovered_component_label_badge {
            response.context_menu(|ui| {
                self.component_label_context_menu(ui, badge, snapshot);
            });
            response.on_hover_ui(|ui| {
                sketch_component_labels::component_label_tooltip(ui, badge);
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
        self.sketch_selection_quick_toolbar(ui, rect, &graph);
        self.sketch_net_label_inline_editor(ui, &net_label_badges, snapshot);
        self.sketch_component_inline_editor(ui, &graph);
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
        let mut label = if self.sketch_palette_place_armed {
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
        };
        let rotation = self.canvas_placement_rotation_deg();
        if rotation != 0 && (self.sketch_palette_place_armed || self.sketch_library_place_armed) {
            label = format!("{label} {rotation} deg");
        }
        if self.component_placement_armed() && self.sketch_placement_mirrored {
            label = format!("{label} flipped");
        }
        label
    }

    fn canvas_placement_rotation_deg(&self) -> i32 {
        if self.sketch_palette_place_armed || self.sketch_library_place_armed {
            normalize_canvas_rotation(self.sketch_placement_rotation_deg)
        } else {
            0
        }
    }

    fn canvas_placement_style(&self) -> SketchNodeStyle {
        if self.component_placement_armed() {
            self.placement_node_style()
        } else {
            SketchNodeStyle::default()
        }
    }

    fn component_placement_armed(&self) -> bool {
        self.sketch_palette_place_armed || self.sketch_library_place_armed
    }

    fn rotate_canvas_placement(&mut self, delta_deg: i32) {
        self.sketch_placement_rotation_deg =
            normalize_canvas_rotation(self.sketch_placement_rotation_deg + delta_deg);
        self.status = format!(
            "Canvas placement rotation set to {} deg.",
            self.sketch_placement_rotation_deg
        );
    }

    fn flip_canvas_placement(&mut self) {
        self.sketch_placement_mirrored = !self.sketch_placement_mirrored;
        let state = if self.sketch_placement_mirrored {
            "flipped"
        } else {
            "unflipped"
        };
        self.status = format!("Canvas placement {state}.");
    }

    fn cycle_canvas_placement_pin_side(&mut self) {
        self.sketch_placement_pin_side = next_pin_side(self.sketch_placement_pin_side);
        self.status = format!(
            "Canvas placement pin side set to {}.",
            self.sketch_placement_pin_side.as_str()
        );
    }

    pub(super) fn placement_node_style(&self) -> SketchNodeStyle {
        SketchNodeStyle {
            rotation_deg: normalize_canvas_rotation(self.sketch_placement_rotation_deg),
            mirrored: self.sketch_placement_mirrored,
            pin_side: self.sketch_placement_pin_side,
        }
    }

    pub(super) fn sketch_placement_orientation_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Orientation");
            ui.label(format!(
                "{} deg",
                normalize_canvas_rotation(self.sketch_placement_rotation_deg)
            ));
            if ui.button("Rotate").on_hover_text("Shortcut: R").clicked() {
                self.rotate_canvas_placement(90);
            }
            let flip_label = if self.sketch_placement_mirrored {
                "Unflip"
            } else {
                "Flip"
            };
            if ui.button(flip_label).on_hover_text("Shortcut: F").clicked() {
                self.flip_canvas_placement();
            }
        });
        let mut pin_side = self.sketch_placement_pin_side;
        egui::ComboBox::from_label("Placement pins")
            .selected_text(pin_side.as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut pin_side, SketchPinSide::Auto, "auto");
                ui.selectable_value(&mut pin_side, SketchPinSide::Left, "left");
                ui.selectable_value(&mut pin_side, SketchPinSide::Right, "right");
            });
        if pin_side != self.sketch_placement_pin_side {
            self.sketch_placement_pin_side = pin_side;
            self.status = format!(
                "Canvas placement pin side set to {}.",
                self.sketch_placement_pin_side.as_str()
            );
        }
        ui.small(
            "Canvas shortcuts: R rotate, Shift+R rotate back, F flip, Shift+F cycle pin side.",
        );
    }

    fn apply_rotate_selected_sketch_components(&mut self, delta_deg: i32) {
        self.apply_transform_selected_sketch_component_styles("rotating", "Rotated", |style| {
            style.rotation_deg = normalize_canvas_rotation(style.rotation_deg + delta_deg);
        });
    }

    fn apply_flip_selected_sketch_components(&mut self) {
        self.apply_transform_selected_sketch_component_styles("flipping", "Flipped", |style| {
            style.mirrored = !style.mirrored;
        });
    }

    fn apply_cycle_selected_sketch_pin_side(&mut self) {
        self.apply_transform_selected_sketch_component_styles(
            "changing pin side for",
            "Changed pin side for",
            |style| style.pin_side = next_pin_side(style.pin_side),
        );
    }

    fn apply_transform_selected_sketch_component_styles(
        &mut self,
        empty_verb: &str,
        applied_verb: &str,
        mut transform: impl FnMut(&mut SketchNodeStyle),
    ) {
        let Some(snapshot) = &self.project_snapshot else {
            return;
        };
        let selected_components = self
            .selected_sketch_items
            .iter()
            .chain(self.selected_sketch_item.iter())
            .filter_map(|selection| match selection {
                SketchSelection::Component(component_id) => Some(component_id.clone()),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();
        if selected_components.is_empty() {
            self.status = format!("Select one or more components before {empty_verb}.");
            return;
        }
        let mut edits = Vec::with_capacity(selected_components.len());
        for component_id in selected_components {
            let Some(component) = snapshot
                .components_detail
                .iter()
                .find(|component| component.id == component_id)
            else {
                continue;
            };
            let mut style = component.style;
            transform(&mut style);
            edits.push((component_id, style));
        }
        if edits.is_empty() {
            return;
        }
        match edit_schematic_component_styles(&self.project_yaml, &edits) {
            Ok(updated) => {
                let count = edits.len();
                self.apply_edited_project_yaml(
                    updated,
                    &format!("{applied_verb} {count} selected component(s)."),
                );
            }
            Err(error) => self.record_error(error),
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
            && ui.input(|input| SketchSelectionBoxMode::from_modifiers(input.modifiers).is_none())
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
