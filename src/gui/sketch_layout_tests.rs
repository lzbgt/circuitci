use super::*;
use crate::gui::sketch::{ProjectSnapshot, SketchNet, SketchPin};
use crate::gui::sketch_probes::{
    SketchProbe, SketchProbeAttachmentKind, SketchProbeQuantity, SketchProbeTarget,
};

fn test_component() -> SketchComponent {
    SketchComponent {
        id: "U1".to_string(),
        model: "generic.schematic.imported_component".to_string(),
        part_number: None,
        spice: None,
        pins: vec![
            SketchPin {
                pin: "1".to_string(),
                net: "in".to_string(),
            },
            SketchPin {
                pin: "2".to_string(),
                net: "out".to_string(),
            },
        ],
        position: None,
        style: SketchNodeStyle::default(),
        kicad_symbol_id: Some("Device:R".to_string()),
        source_paths: Vec::new(),
    }
}

fn layout_test_snapshot() -> ProjectSnapshot {
    ProjectSnapshot {
        name: "classical_layout".to_string(),
        components: 3,
        nets: 3,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![
            SketchComponent {
                id: "V1".to_string(),
                model: "generic.analog.voltage_source".to_string(),
                part_number: None,
                spice: None,
                pins: vec![
                    SketchPin {
                        pin: "P".to_string(),
                        net: "vcc".to_string(),
                    },
                    SketchPin {
                        pin: "N".to_string(),
                        net: "gnd".to_string(),
                    },
                ],
                position: None,
                style: SketchNodeStyle::default(),
                kicad_symbol_id: None,
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "R1".to_string(),
                model: "generic.analog.resistor".to_string(),
                part_number: None,
                spice: None,
                pins: vec![
                    SketchPin {
                        pin: "A".to_string(),
                        net: "vcc".to_string(),
                    },
                    SketchPin {
                        pin: "B".to_string(),
                        net: "sig".to_string(),
                    },
                ],
                position: None,
                style: SketchNodeStyle::default(),
                kicad_symbol_id: None,
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "C1".to_string(),
                model: "generic.analog.capacitor".to_string(),
                part_number: None,
                spice: None,
                pins: vec![
                    SketchPin {
                        pin: "A".to_string(),
                        net: "sig".to_string(),
                    },
                    SketchPin {
                        pin: "B".to_string(),
                        net: "gnd".to_string(),
                    },
                ],
                position: None,
                style: SketchNodeStyle::default(),
                kicad_symbol_id: None,
                source_paths: Vec::new(),
            },
        ],
        nets_detail: vec![
            SketchNet {
                id: "vcc".to_string(),
                kind: "power".to_string(),
                nominal_voltage: Some(5.0),
                powered: Some(true),
                connections: vec!["V1.P".to_string(), "R1.A".to_string()],
                position: None,
            },
            SketchNet {
                id: "sig".to_string(),
                kind: "digital_or_analog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["R1.B".to_string(), "C1.A".to_string()],
                position: None,
            },
            SketchNet {
                id: "gnd".to_string(),
                kind: "ground".to_string(),
                nominal_voltage: Some(0.0),
                powered: None,
                connections: vec!["V1.N".to_string(), "C1.B".to_string()],
                position: None,
            },
        ],
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
        component_labels: Default::default(),
    }
}

fn crossing_reduction_snapshot() -> ProjectSnapshot {
    ProjectSnapshot {
        name: "crossing_reduction".to_string(),
        components: 4,
        nets: 3,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![
            SketchComponent {
                id: "VTOP".to_string(),
                model: "generic.analog.voltage_source".to_string(),
                part_number: None,
                spice: None,
                pins: vec![
                    SketchPin {
                        pin: "P".to_string(),
                        net: "top".to_string(),
                    },
                    SketchPin {
                        pin: "N".to_string(),
                        net: "gnd".to_string(),
                    },
                ],
                position: None,
                style: SketchNodeStyle::default(),
                kicad_symbol_id: None,
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "VBOT".to_string(),
                model: "generic.analog.voltage_source".to_string(),
                part_number: None,
                spice: None,
                pins: vec![
                    SketchPin {
                        pin: "P".to_string(),
                        net: "bottom".to_string(),
                    },
                    SketchPin {
                        pin: "N".to_string(),
                        net: "gnd".to_string(),
                    },
                ],
                position: None,
                style: SketchNodeStyle::default(),
                kicad_symbol_id: None,
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "RBOT".to_string(),
                model: "generic.analog.resistor".to_string(),
                part_number: None,
                spice: None,
                pins: vec![
                    SketchPin {
                        pin: "A".to_string(),
                        net: "bottom".to_string(),
                    },
                    SketchPin {
                        pin: "B".to_string(),
                        net: "gnd".to_string(),
                    },
                ],
                position: None,
                style: SketchNodeStyle::default(),
                kicad_symbol_id: None,
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "RTOP".to_string(),
                model: "generic.analog.resistor".to_string(),
                part_number: None,
                spice: None,
                pins: vec![
                    SketchPin {
                        pin: "A".to_string(),
                        net: "top".to_string(),
                    },
                    SketchPin {
                        pin: "B".to_string(),
                        net: "gnd".to_string(),
                    },
                ],
                position: None,
                style: SketchNodeStyle::default(),
                kicad_symbol_id: None,
                source_paths: Vec::new(),
            },
        ],
        nets_detail: vec![
            SketchNet {
                id: "top".to_string(),
                kind: "analog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["VTOP.P".to_string(), "RTOP.A".to_string()],
                position: None,
            },
            SketchNet {
                id: "bottom".to_string(),
                kind: "analog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["VBOT.P".to_string(), "RBOT.A".to_string()],
                position: None,
            },
            SketchNet {
                id: "gnd".to_string(),
                kind: "ground".to_string(),
                nominal_voltage: Some(0.0),
                powered: None,
                connections: vec![
                    "VTOP.N".to_string(),
                    "VBOT.N".to_string(),
                    "RTOP.B".to_string(),
                    "RBOT.B".to_string(),
                ],
                position: None,
            },
        ],
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
        component_labels: Default::default(),
    }
}

#[test]
fn default_layout_uses_classical_power_signal_ground_roles() {
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(720.0, 420.0)),
        &layout_test_snapshot(),
    );
    let source = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("V1".to_string()))
        .unwrap();
    let resistor = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("R1".to_string()))
        .unwrap();
    let power = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Net("vcc".to_string()))
        .unwrap();
    let signal = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Net("sig".to_string()))
        .unwrap();
    let ground = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Net("gnd".to_string()))
        .unwrap();

    assert!(source.rect.center().x < resistor.rect.center().x);
    assert!(power.rect.center().y < signal.rect.center().y);
    assert!(signal.rect.center().y < ground.rect.center().y);
}

#[test]
fn default_layout_uses_signal_flow_ranks_for_series_paths() {
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(720.0, 420.0)),
        &layout_test_snapshot(),
    );
    let source = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("V1".to_string()))
        .unwrap();
    let resistor = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("R1".to_string()))
        .unwrap();
    let capacitor = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("C1".to_string()))
        .unwrap();

    assert!(source.rect.center().x < resistor.rect.center().x);
    assert!(resistor.rect.center().x <= capacitor.rect.center().x);
    assert!(resistor.rect.center().y < capacitor.rect.center().y);
}

#[test]
fn default_layout_reduces_parallel_branch_crossings_with_barycentric_order() {
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(720.0, 420.0)),
        &crossing_reduction_snapshot(),
    );
    let rtop = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("RTOP".to_string()))
        .unwrap();
    let rbot = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("RBOT".to_string()))
        .unwrap();

    assert!(rtop.rect.center().y < rbot.rect.center().y);
}

#[test]
fn default_layout_keeps_same_rank_shunts_readable() {
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(720.0, 420.0)),
        &crossing_reduction_snapshot(),
    );
    let rtop = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("RTOP".to_string()))
        .unwrap();
    let rbot = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("RBOT".to_string()))
        .unwrap();

    assert!(!rtop.rect.intersects(rbot.rect));
}

#[test]
fn default_layout_scales_imported_blocks_for_pin_label_spacing() {
    let mut component = test_component();
    component.id = "U99".to_string();
    component.model = "generic.schematic.imported_component".to_string();
    component.kicad_symbol_id = Some("Driver:DRV8245P".to_string());
    component.pins = (1..=12)
        .map(|index| SketchPin {
            pin: index.to_string(),
            net: format!("net_{index}"),
        })
        .collect();
    let nets = component
        .pins
        .iter()
        .map(|pin| SketchNet {
            id: pin.net.clone(),
            kind: "digital_or_analog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec![format!("{}.{}", component.id, pin.pin)],
            position: None,
        })
        .collect::<Vec<_>>();
    let snapshot = ProjectSnapshot {
        name: "imported_block".to_string(),
        components: 1,
        nets: nets.len(),
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![component],
        nets_detail: nets,
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
        component_labels: Default::default(),
    };

    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(960.0, 640.0)),
        &snapshot,
    );
    let node = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("U99".to_string()))
        .unwrap();
    let mut anchor_rows = graph
        .pin_anchors
        .iter()
        .filter(|anchor| anchor.component_id == "U99")
        .map(|anchor| anchor.label_pos.y)
        .collect::<Vec<_>>();
    anchor_rows.sort_by(|a, b| a.total_cmp(b));
    let min_gap = anchor_rows
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .fold(f32::INFINITY, f32::min);

    assert!(node.rect.height() >= 240.0);
    assert!(min_gap >= 14.0);
}

#[test]
fn classical_auto_layout_persists_positions_and_vertical_shunts() {
    let plan = classical_sketch_auto_layout(
        &layout_test_snapshot(),
        egui::vec2(720.0, 420.0),
        true,
        16.0,
    );

    assert_eq!(plan.positions.len(), 6);
    assert!(
        plan.positions
            .iter()
            .any(|(selection, _, _)| *selection == SketchSelection::Component("V1".to_string()))
    );
    assert!(
        plan.positions
            .iter()
            .any(|(selection, _, _)| *selection == SketchSelection::Net("gnd".to_string()))
    );
    assert!(plan.positions.iter().all(|(_, x, y)| {
        *x >= 0.0
            && *y >= 0.0
            && (x % 16.0).abs() <= f64::EPSILON
            && (y % 16.0).abs() <= f64::EPSILON
    }));
    assert_eq!(
        plan.styles,
        vec![(
            "C1".to_string(),
            SketchNodeStyle {
                rotation_deg: 90,
                mirrored: false,
                pin_side: SketchPinSide::Auto,
            }
        )]
    );
    assert_eq!(plan.wire_routes.len(), 6);
    assert!(plan.wire_routes.iter().any(|(source, net_id, points)| {
        source == "C1.B" && net_id == "gnd" && points.len() == 1
    }));
    assert!(
        plan.wire_routes
            .iter()
            .all(|(_, _, points)| points.iter().all(|(x, y)| *x >= 0.0 && *y >= 0.0))
    );
}

#[test]
fn classical_auto_layout_places_probe_elements_in_readout_lanes() {
    let mut snapshot = layout_test_snapshot();
    snapshot.probes.push(SketchProbe {
        element_id: Some("tran_sig_voltage".to_string()),
        attachment: SketchProbeAttachmentKind::Pin,
        source: Some("R1.B".to_string()),
        position: None,
        scenario_name: "tran".to_string(),
        probe_name: "sig_voltage".to_string(),
        expression: "V(sig)".to_string(),
        quantity: SketchProbeQuantity::Voltage,
        target: SketchProbeTarget::Net("sig".to_string()),
        assertion_names: Vec::new(),
    });
    let plan = classical_sketch_auto_layout(&snapshot, egui::vec2(720.0, 420.0), true, 16.0);

    assert_eq!(plan.probe_positions.len(), 1);
    let (element_id, x, y) = &plan.probe_positions[0];
    assert_eq!(element_id, "tran_sig_voltage");
    assert!(*x >= 0.0 && *y >= 0.0);
    assert!((x % 16.0).abs() <= f64::EPSILON);
    assert!((y % 16.0).abs() <= f64::EPSILON);

    snapshot.probes[0].position = Some(SketchPosition { x: *x, y: *y });
    for (selection, x, y) in &plan.positions {
        match selection {
            SketchSelection::Component(id) => {
                if let Some(component) = snapshot
                    .components_detail
                    .iter_mut()
                    .find(|component| component.id == *id)
                {
                    component.position = Some(SketchPosition { x: *x, y: *y });
                }
            }
            SketchSelection::Net(id) => {
                if let Some(net) = snapshot.nets_detail.iter_mut().find(|net| net.id == *id) {
                    net.position = Some(SketchPosition { x: *x, y: *y });
                }
            }
            SketchSelection::Overflow(_) => {}
        }
    }
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(720.0, 420.0)),
        &snapshot,
    );
    let probe_bounds = probe_badge_interaction_rect(graph.probe_badges.first().unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .all(|node| !probe_bounds.intersects(node.rect.expand(8.0)))
    );
}

#[test]
fn default_probe_position_avoids_existing_probe_lane() {
    let mut snapshot = layout_test_snapshot();
    snapshot.probes.push(SketchProbe {
        element_id: Some("tran_sig_voltage".to_string()),
        attachment: SketchProbeAttachmentKind::Pin,
        source: Some("R1.B".to_string()),
        position: None,
        scenario_name: "tran".to_string(),
        probe_name: "sig_voltage".to_string(),
        expression: "V(sig)".to_string(),
        quantity: SketchProbeQuantity::Voltage,
        target: SketchProbeTarget::Net("sig".to_string()),
        assertion_names: Vec::new(),
    });
    let (_, first_x, first_y) = default_probe_element_position(
        &snapshot,
        "tran_sig_voltage",
        egui::vec2(720.0, 420.0),
        true,
        16.0,
    )
    .unwrap();
    snapshot.probes[0].position = Some(SketchPosition {
        x: first_x,
        y: first_y,
    });
    snapshot.probes.push(SketchProbe {
        element_id: Some("tran_sig_voltage_2".to_string()),
        attachment: SketchProbeAttachmentKind::Pin,
        source: Some("R1.B".to_string()),
        position: None,
        scenario_name: "tran".to_string(),
        probe_name: "sig_voltage_2".to_string(),
        expression: "V(sig)".to_string(),
        quantity: SketchProbeQuantity::Voltage,
        target: SketchProbeTarget::Net("sig".to_string()),
        assertion_names: Vec::new(),
    });

    let (element_id, x, y) = default_probe_element_position(
        &snapshot,
        "tran_sig_voltage_2",
        egui::vec2(720.0, 420.0),
        true,
        16.0,
    )
    .unwrap();

    assert_eq!(element_id, "tran_sig_voltage_2");
    assert!((x % 16.0).abs() <= f64::EPSILON);
    assert!((y % 16.0).abs() <= f64::EPSILON);
    snapshot.probes[1].position = Some(SketchPosition { x, y });
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(720.0, 420.0)),
        &snapshot,
    );
    let first = graph
        .probe_badges
        .iter()
        .find(|badge| badge.probe.element_id.as_deref() == Some("tran_sig_voltage"))
        .unwrap();
    let second = graph
        .probe_badges
        .iter()
        .find(|badge| badge.probe.element_id.as_deref() == Some("tran_sig_voltage_2"))
        .unwrap();
    assert!(!probe_badge_interaction_rect(first).intersects(probe_badge_interaction_rect(second)));
}

#[test]
fn component_pin_anchors_use_matching_kicad_pin_geometry() {
    let kicad = vec![
        KiCadSymbolPinAnchor {
            pin: "1".to_string(),
            pos: egui::pos2(20.0, 40.0),
            label_pos: egui::pos2(10.0, 40.0),
            label_align: egui::Align2::RIGHT_CENTER,
        },
        KiCadSymbolPinAnchor {
            pin: "2".to_string(),
            pos: egui::pos2(80.0, 40.0),
            label_pos: egui::pos2(90.0, 40.0),
            label_align: egui::Align2::LEFT_CENTER,
        },
    ];
    let net_kinds = [("in", "analog"), ("out", "analog")]
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();

    let anchors = component_pin_anchors_from_kicad(&test_component(), &kicad, &net_kinds, 2);

    assert_eq!(anchors.len(), 2);
    assert_eq!(anchors[0].pin, "1");
    assert_eq!(anchors[0].pos, egui::pos2(20.0, 40.0));
    assert_eq!(anchors[0].label_align, egui::Align2::RIGHT_CENTER);
    assert_eq!(anchors[1].pin, "2");
    assert_eq!(anchors[1].pos, egui::pos2(80.0, 40.0));
}

#[test]
fn component_pin_anchors_ignore_unmatched_kicad_pins() {
    let kicad = vec![KiCadSymbolPinAnchor {
        pin: "3".to_string(),
        pos: egui::pos2(20.0, 40.0),
        label_pos: egui::pos2(10.0, 40.0),
        label_align: egui::Align2::RIGHT_CENTER,
    }];
    let net_kinds = std::collections::BTreeMap::new();

    let anchors = component_pin_anchors_from_kicad(&test_component(), &kicad, &net_kinds, 2);

    assert!(anchors.is_empty());
}
