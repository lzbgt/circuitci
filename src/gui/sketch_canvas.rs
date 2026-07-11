use eframe::egui;

use super::sketch::{
    ProjectSnapshot, SketchSelection, draw_sketch_grid, draw_sketch_node, draw_sketch_pin_anchor,
    hit_test_wire, layout_sketch_graph, layout_sketch_graph_viewport,
    persisted_node_position_from_screen, persisted_node_position_from_screen_with_snap,
    runtime_scope_chip_rect, snap_screen_point_to_grid,
};
use super::sketch_canvas_hits::{
    SketchCanvasHitContext, hover_targets as collect_hover_targets, position_hits_interactive_item,
    runtime_scope_activity_targets,
};
use super::sketch_canvas_interaction::{
    SketchSelectionBoxMode, WireDragTarget, schematic_canvas_size, wire_drag_target_at,
    wire_route_insert_index,
};
use super::sketch_canvas_render::{
    draw_pending_wire_route_handles, draw_placement_ghost, draw_snap_feedback,
    draw_wire_drag_target, draw_wire_edge, draw_wire_junctions, draw_wire_points,
    draw_wire_route_handles, draw_wire_route_preview, placement_ghost_rect, placement_ghost_size,
    sketch_hover_tooltip, sketch_pin_hover_tooltip, sketch_probe_badge_tooltip,
    sketch_wire_hover_tooltip, sketch_wire_route_handle_tooltip, wire_preview_start,
};
use super::sketch_probes::{
    SketchProbe, SketchProbeRuntimeReadout, SketchProbeTarget, draw_probe_badge,
    hit_test_probe_badge, probe_assertion_status,
};
use super::sketch_routes;
use super::sketch_scope_feedback::{
    ScopeProbeToolHoverInput, draw_scope_probe_tool_feedback, scope_probe_tool_hover_feedback,
};
use super::sketch_scope_tools::{SketchScopeProbePlacement, SketchScopeProbeTool};
use super::waveform::{
    runtime_probe_activity_for_selection, runtime_probe_lines_for_selection,
    runtime_scope_probe_frequency_label, runtime_scope_probe_sample_label,
    runtime_scope_probe_sparkline_points, runtime_scope_probe_target_for_selection,
    waveform_probe_value_for_badge, waveform_time_range_for_view,
};
use super::{
    CircuitCiApp, ScopeProbeTarget, sketch_alignment, sketch_bundles, sketch_component_labels,
    sketch_connectivity, sketch_hierarchy, sketch_minimap, sketch_net_labels,
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
        let bundle_badges = if self.sketch_net_bundles_visible {
            sketch_bundles::layout_net_bundle_badges(snapshot, &graph)
        } else {
            Vec::new()
        };
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
        let selection_frame_response = self.sketch_selection_frame(ui, rect, &graph);
        if selection_frame_response
            .as_ref()
            .is_some_and(|response| response.drag_started_by(egui::PointerButton::Primary))
            && let Some(pointer_start) = selection_frame_response
                .as_ref()
                .and_then(|response| response.interact_pointer_pos())
        {
            let node_starts = graph
                .nodes
                .iter()
                .filter(|node| self.selected_sketch_items.contains(&node.selection))
                .filter(|node| !matches!(node.selection, SketchSelection::Overflow(_)))
                .map(|node| (node.selection.clone(), node.rect))
                .collect::<Vec<_>>();
            if !node_starts.is_empty() {
                self.sketch_group_frame_drag = Some(super::SketchGroupFrameDrag {
                    pointer_start,
                    last_applied_delta: egui::Vec2::ZERO,
                    node_starts,
                });
            }
        }
        let pointer_hover = if response.hovered() {
            ui.ctx().pointer_hover_pos()
        } else {
            None
        };
        let hit_context = SketchCanvasHitContext {
            graph: &graph,
            hierarchy_view: hierarchy_view.as_ref(),
            bundle_badges: &bundle_badges,
            hierarchy_connector_badges: &hierarchy_connector_badges,
            net_label_badges: &net_label_badges,
            component_label_badges: &component_label_badges,
            minimap: minimap.as_ref(),
            waveforms: &self.waveforms,
            selected_waveform: self.selected_waveform,
            waveform_cursor_a_us: self.waveform_cursor_a_us,
            snapshot,
            runtime_scope_overlay_visible: self.sketch_runtime_scope_overlay_visible,
        };
        let hover_targets = collect_hover_targets(&hit_context, pointer_hover);
        let pointer_over_minimap = hover_targets.pointer_over_minimap;
        let blank_canvas_hovered = hover_targets.blank_canvas_hovered();
        if hover_targets.runtime_scope_node.is_some() {
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::PointingHand);
        }
        let placement_armed = self.sketch_palette_place_armed
            || self.sketch_library_place_armed
            || self.sketch_net_label_place_armed;
        let scope_probe_tool_armed = self.scope_probe_tool_armed();
        let pan_drag_start_allowed = !placement_armed
            && !scope_probe_tool_armed
            && ui.input(|input| {
                input.pointer.press_origin().is_some_and(|origin| {
                    rect.contains(origin) && !position_hits_interactive_item(&hit_context, origin)
                })
            });
        self.handle_sketch_viewport_input(
            ui,
            rect,
            &response,
            pan_drag_start_allowed,
            blank_canvas_hovered && !placement_armed && !scope_probe_tool_armed,
        );
        let viewport = self.sketch_viewport();
        let graph = layout_sketch_graph_viewport(rect, snapshot, viewport);
        let hierarchy_connector_badges = hierarchy_view
            .as_ref()
            .map(|view| sketch_hierarchy::layout_hierarchy_connector_badges(snapshot, &graph, view))
            .unwrap_or_default();
        let bundle_badges = if self.sketch_net_bundles_visible {
            sketch_bundles::layout_net_bundle_badges(snapshot, &graph)
        } else {
            Vec::new()
        };
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
        let hit_context = SketchCanvasHitContext {
            graph: &graph,
            hierarchy_view: hierarchy_view.as_ref(),
            bundle_badges: &bundle_badges,
            hierarchy_connector_badges: &hierarchy_connector_badges,
            net_label_badges: &net_label_badges,
            component_label_badges: &component_label_badges,
            minimap: minimap.as_ref(),
            waveforms: &self.waveforms,
            selected_waveform: self.selected_waveform,
            waveform_cursor_a_us: self.waveform_cursor_a_us,
            snapshot,
            runtime_scope_overlay_visible: self.sketch_runtime_scope_overlay_visible,
        };
        let hover_targets = collect_hover_targets(&hit_context, pointer_hover);
        let runtime_scope_activity_targets = runtime_scope_activity_targets(&hit_context);
        let hovered_node = hover_targets.node;
        let hovered_anchor = hover_targets.anchor;
        let hovered_route_handle = hover_targets.route_handle;
        let hovered_wire = hover_targets.wire;
        let hovered_probe_badge = hover_targets.probe_badge;
        let hovered_bundle_badge = hover_targets.bundle_badge;
        let hovered_hierarchy_connector_badge = hover_targets.hierarchy_connector_badge;
        let hovered_net_label_badge = hover_targets.net_label_badge;
        let hovered_component_label_badge = hover_targets.component_label_badge;
        let placement_target_clear = hover_targets.placement_target_clear();
        let scope_probe_feedback = self.active_scope_probe_tool().map(|tool| {
            scope_probe_tool_hover_feedback(ScopeProbeToolHoverInput {
                tool,
                hovered_probe_badge,
                hovered_anchor,
                hovered_net_label_badge,
                hovered_component_label_badge,
                hovered_wire,
                hovered_node,
                pointer_hover,
            })
        });
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
        let net_label_y_offsets = super::sketch_render::sketch_net_label_y_offsets(&graph);
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
            let runtime_activity = self
                .sketch_runtime_scope_overlay_visible
                .then(|| {
                    runtime_probe_activity_for_selection(
                        &self.waveforms,
                        self.selected_waveform,
                        self.waveform_cursor_a_us,
                        &node.selection,
                        snapshot,
                    )
                })
                .flatten();
            let runtime_scope_chip_hovered = self.sketch_runtime_scope_overlay_visible
                && hover_targets.runtime_scope_chip_hovered(&node.selection);
            let label_y_offset = match &node.selection {
                SketchSelection::Net(net_id) => {
                    net_label_y_offsets.get(net_id).copied().unwrap_or_default()
                }
                _ => 0.0,
            };
            draw_sketch_node(
                &painter,
                node,
                selected,
                runtime_activity,
                runtime_scope_chip_hovered,
                label_y_offset,
                opacity,
            );
        }
        self.sketch_runtime_scope_activity_legend(ui, rect, &runtime_scope_activity_targets);
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
        if let Some(feedback) = &scope_probe_feedback {
            draw_scope_probe_tool_feedback(&painter, feedback);
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
            let (ghost, alignment_guides, alignment_snapped) = if self.component_placement_armed() {
                self.aligned_component_placement_rect(&graph, rect, pointer, viewport)
            } else {
                (
                    placement_ghost_rect(
                        rect,
                        pointer,
                        viewport,
                        self.sketch_snap_enabled,
                        self.sketch_grid_step,
                        placement_ghost_size(&label, style),
                    ),
                    sketch_alignment::SketchAlignmentGuides {
                        vertical: None,
                        horizontal: None,
                    },
                    false,
                )
            };
            draw_placement_ghost(&painter, ghost, &label, placement_target_clear, style);
            draw_snap_feedback(
                &painter,
                ghost.center(),
                placement_target_clear,
                self.sketch_snap_enabled,
            );
            sketch_alignment::draw_alignment_guides(&painter, rect, alignment_guides);
            if alignment_snapped {
                painter.text(
                    ghost.left_bottom() + egui::vec2(0.0, 15.0),
                    egui::Align2::LEFT_CENTER,
                    "Guide snap",
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_rgb(99, 224, 172),
                );
            }
        }
        if let Some(group_drag) = &self.sketch_group_frame_drag {
            if let Some(pointer) = ui.ctx().pointer_interact_pos()
                && rect.contains(pointer)
            {
                let snapped = snap_screen_point_to_grid(
                    rect,
                    pointer,
                    viewport,
                    self.sketch_snap_enabled,
                    self.sketch_grid_step,
                );
                draw_snap_feedback(&painter, snapped, true, self.sketch_snap_enabled);
                let delta = sketch_alignment::snap_delta_to_guides(
                    &graph,
                    &group_drag.node_starts,
                    pointer - group_drag.pointer_start,
                    self.sketch_guide_snap_enabled,
                );
                if let Some(bounds) =
                    sketch_alignment::moved_selection_bounds(&group_drag.node_starts, delta)
                {
                    let excluded = group_drag
                        .node_starts
                        .iter()
                        .map(|(selection, _)| selection.clone())
                        .collect();
                    sketch_alignment::draw_alignment_guides(
                        &painter,
                        rect,
                        sketch_alignment::guides_for_rect(&graph, bounds, &excluded),
                    );
                }
            }
        } else if response.dragged_by(egui::PointerButton::Primary)
            && !self.sketch_pan_drag_active
            && self.wire_from_component.is_none()
            && self.sketch_wire_route_drag.is_none()
            && self.sketch_net_label_drag.is_none()
            && self.sketch_component_label_drag.is_none()
            && self.sketch_probe_element_drag.is_none()
            && self.sketch_selection_box_drag.is_none()
            && self.sketch_selection_lasso_drag.is_none()
        {
            let mut excluded = self.selected_sketch_items.clone();
            if let Some(selection) = &self.selected_sketch_item {
                excluded.insert(selection.clone());
            }
            let selected_bounds = sketch_alignment::selection_bounds(
                graph
                    .nodes
                    .iter()
                    .filter(|node| excluded.contains(&node.selection)),
            );
            if let Some(bounds) = selected_bounds {
                sketch_alignment::draw_alignment_guides(
                    &painter,
                    rect,
                    sketch_alignment::guides_for_rect(&graph, bounds, &excluded),
                );
            }
        }
        for badge in &hierarchy_connector_badges {
            let hovered = hovered_hierarchy_connector_badge
                .is_some_and(|hovered| hovered.net_id == badge.net_id);
            sketch_hierarchy::draw_hierarchy_connector_badge(&painter, badge, hovered);
        }
        for badge in &graph.probe_badges {
            let badge = if let Some(drag) = self
                .sketch_probe_element_drag
                .as_ref()
                .filter(|drag| badge.probe.element_id.as_deref() == Some(drag.element_id.as_str()))
            {
                let mut badge = badge.clone();
                badge.rect = egui::Rect::from_center_size(drag.current_center, badge.rect.size());
                std::borrow::Cow::Owned(badge)
            } else {
                std::borrow::Cow::Borrowed(badge)
            };
            let badge = badge.as_ref();
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
            let runtime = badge
                .probe
                .element_id
                .as_ref()
                .and_then(|_| self.probe_badge_runtime_readout(&badge.probe));
            draw_probe_badge(&painter, badge, hovered, status, runtime.as_ref(), opacity);
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
            let clicked_runtime_scope_target = self
                .sketch_runtime_scope_overlay_visible
                .then(|| {
                    graph
                        .nodes
                        .iter()
                        .find(|node| {
                            hierarchy_view
                                .as_ref()
                                .is_none_or(|view| view.interaction_visible(&node.selection))
                                && runtime_scope_chip_rect(node).contains(position)
                                && runtime_probe_activity_for_selection(
                                    &self.waveforms,
                                    self.selected_waveform,
                                    self.waveform_cursor_a_us,
                                    &node.selection,
                                    snapshot,
                                )
                                .is_some()
                        })
                        .and_then(|node| {
                            runtime_scope_probe_target_for_selection(
                                &self.waveforms,
                                self.selected_waveform,
                                &node.selection,
                                snapshot,
                            )
                        })
                })
                .flatten();
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
            let scope_tool_target = if let Some(badge) = clicked_probe_badge.as_ref() {
                match &badge.probe.target {
                    SketchProbeTarget::Component(component_id) => {
                        Some(SketchScopeProbePlacement::node(SketchSelection::Component(
                            component_id.clone(),
                        )))
                    }
                    SketchProbeTarget::Net(net_id) => Some(SketchScopeProbePlacement::node(
                        SketchSelection::Net(net_id.clone()),
                    )),
                }
            } else if let Some(anchor) = clicked_anchor {
                Some(match self.sketch_scope_probe_tool {
                    Some(SketchScopeProbeTool::Voltage) => SketchScopeProbePlacement::pin(
                        SketchSelection::Net(anchor.net.clone()),
                        &anchor.component_id,
                        &anchor.pin,
                    ),
                    Some(SketchScopeProbeTool::Current | SketchScopeProbeTool::Power) => {
                        SketchScopeProbePlacement::pin(
                            SketchSelection::Component(anchor.component_id.clone()),
                            &anchor.component_id,
                            &anchor.pin,
                        )
                    }
                    None => SketchScopeProbePlacement::pin(
                        SketchSelection::Component(anchor.component_id.clone()),
                        &anchor.component_id,
                        &anchor.pin,
                    ),
                })
            } else if let Some(badge) = clicked_net_label_badge.as_ref() {
                Some(SketchScopeProbePlacement::node(SketchSelection::Net(
                    badge.net_id.clone(),
                )))
            } else if let Some(badge) = clicked_component_label_badge.as_ref() {
                Some(SketchScopeProbePlacement::node(SketchSelection::Component(
                    badge.component_id.clone(),
                )))
            } else if let Some(edge) = clicked_wire.as_ref() {
                Some(SketchScopeProbePlacement::wire(
                    edge.net_id.clone(),
                    edge.source.clone(),
                ))
            } else {
                clicked.clone().map(SketchScopeProbePlacement::node)
            };
            if self.apply_scope_probe_tool_to_selection(scope_tool_target) {
                placement_applied = true;
            } else if (self.sketch_palette_place_armed
                || self.sketch_library_place_armed
                || self.sketch_net_label_place_armed)
                && placement_target_clear
            {
                let (target, snap_enabled) = if self.component_placement_armed() {
                    let (aligned, _, alignment_snapped) =
                        self.aligned_component_placement_rect(&graph, rect, position, viewport);
                    (
                        aligned.center(),
                        self.sketch_snap_enabled && !alignment_snapped,
                    )
                } else {
                    (position, self.sketch_snap_enabled)
                };
                if self.sketch_palette_place_armed {
                    self.apply_insert_sketch_primitive_at_with_snap(rect, target, snap_enabled);
                } else if self.sketch_library_place_armed {
                    self.apply_insert_selected_library_model_at_with_snap(
                        rect,
                        target,
                        snap_enabled,
                    );
                } else {
                    self.apply_add_or_create_schematic_net_label_at(rect, viewport, target);
                }
                placement_applied = true;
            } else if let Some(target) = clicked_runtime_scope_target {
                self.open_scope_probe_target(target);
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
            let (target, snap_enabled) = if self.component_placement_armed() {
                let (aligned, _, alignment_snapped) =
                    self.aligned_component_placement_rect(&graph, rect, position, viewport);
                (
                    aligned.center(),
                    self.sketch_snap_enabled && !alignment_snapped,
                )
            } else {
                (position, self.sketch_snap_enabled)
            };
            if self.sketch_palette_place_armed {
                self.apply_insert_sketch_primitive_at_with_snap(rect, target, snap_enabled);
            } else if self.sketch_library_place_armed {
                self.apply_insert_selected_library_model_at_with_snap(rect, target, snap_enabled);
            } else {
                self.apply_add_or_create_schematic_net_label_at(rect, viewport, target);
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary)
            && !self.sketch_pan_drag_active
            && !self.scope_probe_tool_armed()
            && !pointer_over_minimap
            && self.sketch_group_frame_drag.is_none()
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
            let clicked_probe_badge =
                hit_test_probe_badge(&graph.probe_badges, position).filter(|badge| {
                    hierarchy_view
                        .as_ref()
                        .is_none_or(|view| view.probe_badge_visible(badge))
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
                && let Some(badge) = clicked_probe_badge
                && let Some(element_id) = badge.probe.element_id.as_deref()
            {
                self.sketch_probe_element_drag = Some(super::SketchProbeElementDrag {
                    element_id: element_id.to_string(),
                    current_center: badge.rect.center(),
                });
                self.status = format!("Moving probe element {element_id}.");
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
        let group_drag_update = self
            .sketch_group_frame_drag
            .as_ref()
            .and_then(|group_drag| {
                if !ui.input(|input| input.pointer.primary_down()) {
                    return None;
                }
                let position = ui.ctx().pointer_interact_pos()?;
                let raw_delta = position - group_drag.pointer_start;
                let delta = sketch_alignment::snap_delta_to_guides(
                    &graph,
                    &group_drag.node_starts,
                    raw_delta,
                    self.sketch_guide_snap_enabled,
                );
                if (delta - group_drag.last_applied_delta).length_sq() <= f32::EPSILON {
                    return None;
                }
                Some((
                    group_drag.node_starts.clone(),
                    delta,
                    (delta - raw_delta).length_sq() <= f32::EPSILON,
                ))
            });
        if let Some((node_starts, delta, use_grid_snap)) = group_drag_update {
            self.apply_schematic_node_rect_delta_with_snap(
                rect,
                viewport,
                &node_starts,
                delta,
                "Selected sketch group moved.",
                self.sketch_snap_enabled && use_grid_snap,
            );
            if let Some(group_drag) = &mut self.sketch_group_frame_drag {
                group_drag.last_applied_delta = delta;
            }
        } else if let Some(selection_box) = &self.sketch_selection_box_drag
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
            && let Some(position) = response.interact_pointer_pos()
            && let Some(drag) = &mut self.sketch_probe_element_drag
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
            && self.sketch_probe_element_drag.is_none()
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
                let node_starts = graph
                    .nodes
                    .iter()
                    .filter(|node| self.selected_sketch_items.contains(&node.selection))
                    .map(|node| (node.selection.clone(), node.rect))
                    .collect::<Vec<_>>();
                let raw_delta = delta;
                let delta = sketch_alignment::snap_delta_to_guides(
                    &graph,
                    &node_starts,
                    raw_delta,
                    self.sketch_guide_snap_enabled,
                );
                self.apply_selected_schematic_screen_delta_with_snap(
                    rect,
                    &graph,
                    viewport,
                    delta,
                    "Selected sketch items moved.",
                    self.sketch_snap_enabled && (delta - raw_delta).length_sq() <= f32::EPSILON,
                );
            } else {
                let proposed_center = snap_screen_point_to_grid(
                    rect,
                    position,
                    viewport,
                    self.sketch_snap_enabled,
                    self.sketch_grid_step,
                );
                let proposed_rect = egui::Rect::from_center_size(proposed_center, node.rect.size());
                let excluded = std::collections::BTreeSet::from([selection.clone()]);
                let guides = sketch_alignment::guides_for_rect(&graph, proposed_rect, &excluded);
                let snapped_rect = sketch_alignment::snap_rect_to_guides(
                    proposed_rect,
                    guides,
                    self.sketch_guide_snap_enabled,
                );
                let (x, y) = if snapped_rect != proposed_rect {
                    persisted_node_position_from_screen(
                        rect,
                        snapped_rect.center(),
                        node.rect,
                        viewport,
                    )
                } else {
                    persisted_node_position_from_screen_with_snap(
                        rect,
                        position,
                        node.rect,
                        viewport,
                        self.sketch_snap_enabled,
                        self.sketch_grid_step,
                    )
                };
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
            && let Some(drag) = self.sketch_probe_element_drag.take()
            && let Some(badge) = graph
                .probe_badges
                .iter()
                .find(|badge| badge.probe.element_id.as_deref() == Some(drag.element_id.as_str()))
        {
            self.apply_move_schematic_probe_element_to(rect, viewport, badge, drag.current_center);
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
        if self.sketch_group_frame_drag.is_some() && !ui.input(|input| input.pointer.primary_down())
        {
            self.sketch_group_frame_drag = None;
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
        let scope_tool_shortcut = if response.hovered() {
            ui.input(SketchScopeProbeTool::from_unmodified_shortcut)
        } else {
            None
        };
        let requested_toolbar_paste = std::mem::take(&mut self.sketch_paste_requested);
        if let Some(tool) = scope_tool_shortcut {
            self.toggle_scope_probe_tool(tool);
        } else if cancel_canvas_mode_pressed && self.scope_probe_tool_armed() {
            self.cancel_scope_probe_tool();
        } else if cancel_canvas_mode_pressed
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
        } else if cancel_canvas_mode_pressed && self.sketch_group_frame_drag.is_some() {
            self.sketch_group_frame_drag = None;
            self.status = "Group move canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.sketch_wire_route_drag.is_some() {
            self.sketch_wire_route_drag = None;
            self.status = "Wire route edit canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.sketch_net_label_drag.is_some() {
            self.sketch_net_label_drag = None;
            self.status = "Net label move canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.sketch_component_label_drag.is_some() {
            self.sketch_component_label_drag = None;
            self.status = "Component label move canceled.".to_string();
        } else if cancel_canvas_mode_pressed && self.sketch_probe_element_drag.is_some() {
            self.sketch_probe_element_drag = None;
            self.status = "Probe element move canceled.".to_string();
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
            let runtime_lines = if self.sketch_runtime_scope_overlay_visible {
                runtime_probe_lines_for_selection(
                    &self.waveforms,
                    self.selected_waveform,
                    self.waveform_cursor_a_us,
                    &node.selection,
                    snapshot,
                )
            } else {
                Vec::new()
            };
            let runtime_scope_chip_hovered = self.sketch_runtime_scope_overlay_visible
                && hover_targets.runtime_scope_chip_hovered(&node.selection);
            response.on_hover_ui(|ui| {
                sketch_hover_tooltip(ui, node, &runtime_lines);
                if runtime_scope_chip_hovered {
                    ui.separator();
                    ui.label("Click the scope chip to open the matching loaded trace.");
                }
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

    fn probe_badge_runtime_readout(
        &self,
        probe: &SketchProbe,
    ) -> Option<SketchProbeRuntimeReadout> {
        let target = ScopeProbeTarget {
            scenario_name: probe.scenario_name.clone(),
            probe_name: probe.probe_name.clone(),
        };
        for waveform_index in 0..self.waveforms.len() {
            let Some(sample_label) = runtime_scope_probe_sample_label(
                &self.waveforms,
                waveform_index,
                self.waveform_cursor_a_us,
                &target,
            ) else {
                continue;
            };
            let sparkline_points =
                runtime_scope_probe_sparkline_points(&self.waveforms, waveform_index, &target, 24)
                    .unwrap_or_default();
            let cursor_fraction = waveform_time_range_for_view(&self.waveforms, waveform_index)
                .and_then(|(start_us, end_us)| {
                    (end_us > start_us).then_some(
                        ((self.waveform_cursor_a_us - start_us) / (end_us - start_us))
                            .clamp(0.0, 1.0) as f32,
                    )
                });
            return Some(SketchProbeRuntimeReadout {
                sample_label,
                frequency_label: runtime_scope_probe_frequency_label(
                    &self.waveforms,
                    waveform_index,
                    &target,
                ),
                sparkline_points,
                cursor_fraction,
            });
        }
        None
    }
}
