use super::sketch::{
    SketchEdge, SketchGraph, SketchNode, SketchPinAnchor, SketchSelection, SketchViewport,
    sketch_wire_points,
};
use super::sketch_canvas::{
    WireDragTarget, closest_point_on_edge, hit_test_wire_route_handle, wire_drag_target_at,
    wire_route_insert_index, zoom_viewport_around,
};
use super::sketch_canvas_render::placement_ghost_rect;
use super::sketch_net_labels::SketchNetLabelBadge;
use super::sketch_probes::SketchProbeBadge;
use super::sketch_symbols::SketchSymbolKind;
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
            route: Vec::new(),
        }],
        probe_badges: Vec::<SketchProbeBadge>::new(),
    };

    match wire_drag_target_at(
        &graph,
        &[],
        egui::pos2(100.0, 100.0),
        |_| true,
        |_| true,
        |_| true,
        |_| true,
    ) {
        Some(WireDragTarget::Pin { pin, pos, .. }) => {
            assert_eq!(pin, "A");
            assert_eq!(pos, egui::pos2(100.0, 100.0));
        }
        _ => panic!("expected pin target"),
    }
    match wire_drag_target_at(
        &graph,
        &[],
        egui::pos2(130.0, 110.0),
        |_| true,
        |_| true,
        |_| true,
        |_| true,
    ) {
        Some(WireDragTarget::NetNode { net_id, rect }) => {
            assert_eq!(net_id, "net_mid");
            assert!(rect.contains(egui::pos2(130.0, 110.0)));
        }
        _ => panic!("expected net target"),
    }
    match wire_drag_target_at(
        &graph,
        &[],
        egui::pos2(120.0, 202.0),
        |_| true,
        |_| true,
        |_| true,
        |_| true,
    ) {
        Some(WireDragTarget::Wire { net_id, snap, .. }) => {
            assert_eq!(net_id, "wire_net");
            assert_eq!(snap, egui::pos2(120.0, 200.0));
        }
        _ => panic!("expected wire net target"),
    }
}

#[test]
fn wire_drag_target_accepts_net_label_badges_before_wires() {
    let graph = SketchGraph {
        nodes: vec![SketchNode {
            selection: SketchSelection::Net("node_net".to_string()),
            label: "node_net".to_string(),
            detail: String::new(),
            symbol: SketchSymbolKind::Net,
            style: Default::default(),
            rect: egui::Rect::from_center_size(egui::pos2(120.0, 100.0), egui::vec2(90.0, 36.0)),
        }],
        pin_anchors: Vec::new(),
        edges: vec![SketchEdge {
            net_id: "wire_net".to_string(),
            source: "R2.B".to_string(),
            start: egui::pos2(20.0, 100.0),
            end: egui::pos2(220.0, 100.0),
            route: Vec::new(),
        }],
        probe_badges: Vec::<SketchProbeBadge>::new(),
    };
    let labels = vec![SketchNetLabelBadge {
        id: "label_sig".to_string(),
        net_id: "sig".to_string(),
        kind: super::sketch::SketchNetLabelKind::Local,
        rect: egui::Rect::from_center_size(egui::pos2(120.0, 100.0), egui::vec2(80.0, 24.0)),
    }];

    match wire_drag_target_at(
        &graph,
        &labels,
        egui::pos2(120.0, 100.0),
        |_| true,
        |_| true,
        |_| true,
        |_| true,
    ) {
        Some(WireDragTarget::NetLabel {
            net_id,
            label_id,
            rect,
        }) => {
            assert_eq!(net_id, "sig");
            assert_eq!(label_id, "label_sig");
            assert!(rect.contains(egui::pos2(120.0, 100.0)));
        }
        other => panic!("expected net label target, got {other:?}"),
    }
}

#[test]
fn wire_drag_snap_uses_nearest_orthogonal_segment() {
    let edge = SketchEdge {
        net_id: "net_a".to_string(),
        source: "R1.A".to_string(),
        start: egui::pos2(20.0, 20.0),
        end: egui::pos2(220.0, 120.0),
        route: Vec::new(),
    };
    let snap = closest_point_on_edge(egui::pos2(150.0, 75.0), &edge);
    assert_eq!(snap, egui::pos2(120.0, 75.0));
}

#[test]
fn route_handle_hit_test_targets_nearest_custom_waypoint() {
    let graph = SketchGraph {
        nodes: Vec::new(),
        pin_anchors: Vec::new(),
        edges: vec![SketchEdge {
            net_id: "net_a".to_string(),
            source: "R1.A".to_string(),
            start: egui::pos2(20.0, 20.0),
            end: egui::pos2(220.0, 120.0),
            route: vec![egui::pos2(80.0, 40.0), egui::pos2(160.0, 96.0)],
        }],
        probe_badges: Vec::<SketchProbeBadge>::new(),
    };

    let (edge, index) = hit_test_wire_route_handle(&graph, egui::pos2(162.0, 98.0)).unwrap();

    assert_eq!(edge.net_id, "net_a");
    assert_eq!(index, 1);
    assert!(hit_test_wire_route_handle(&graph, egui::pos2(120.0, 120.0)).is_none());
}

#[test]
fn wire_route_insert_index_targets_nearest_segment() {
    let edge = SketchEdge {
        net_id: "net_a".to_string(),
        source: "R1.A".to_string(),
        start: egui::pos2(20.0, 20.0),
        end: egui::pos2(220.0, 120.0),
        route: vec![egui::pos2(80.0, 40.0), egui::pos2(160.0, 96.0)],
    };

    let mut points = edge.route.clone();
    let index = wire_route_insert_index(&edge, egui::pos2(118.0, 66.0));
    points.insert(index, egui::pos2(118.0, 66.0));

    assert_eq!(index, 1);
    assert_eq!(points[1], egui::pos2(118.0, 66.0));
}

#[test]
fn routed_wire_points_are_orthogonal_between_custom_handles() {
    let edge = SketchEdge {
        net_id: "net_a".to_string(),
        source: "R1.A".to_string(),
        start: egui::pos2(20.0, 20.0),
        end: egui::pos2(220.0, 120.0),
        route: vec![egui::pos2(80.0, 40.0), egui::pos2(160.0, 96.0)],
    };

    let points = sketch_wire_points(&edge);

    assert!(points.contains(&egui::pos2(80.0, 40.0)));
    assert!(points.contains(&egui::pos2(160.0, 96.0)));
    assert!(points.windows(2).all(|segment| {
        (segment[0].x - segment[1].x).abs() <= 0.5 || (segment[0].y - segment[1].y).abs() <= 0.5
    }));
}

#[test]
fn placement_ghost_snaps_in_logical_canvas_space() {
    let canvas = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(640.0, 360.0));
    let viewport = SketchViewport {
        zoom: 1.5,
        pan: egui::vec2(30.0, -12.0),
    };
    let rect = placement_ghost_rect(
        canvas,
        egui::pos2(157.0, 112.0),
        viewport,
        true,
        32.0,
        egui::vec2(180.0, 92.0),
    );

    assert_eq!(rect.size(), egui::vec2(180.0, 92.0));
    assert_eq!(rect.center(), egui::pos2(136.0, 104.0));
}
