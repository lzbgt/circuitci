use super::CircuitCiApp;
use super::sketch::{ProjectSnapshot, SketchComponent, SketchNet, SketchPin};
use super::sketch::{
    SketchNodeStyle, SketchPinSide, SketchPosition, SketchSelection, SketchViewport, add_component,
    add_component_with_ports, add_net, assign_component_pin, connect_component_pins,
    edit_schematic_component_style, edit_schematic_node_position, edit_schematic_node_positions,
    layout_sketch_graph, layout_sketch_graph_viewport, load_project_snapshot_from_yaml,
    orthogonal_wire_points, persisted_node_position_from_screen,
    persisted_node_position_from_screen_with_snap, remove_component, remove_component_pin,
    remove_net, sketch_graph_bounds, snap_screen_point_to_grid, validate_board_ir_yaml_text,
};
use super::sketch_symbols::SketchSymbolKind;
use eframe::egui;

fn editable_project_yaml() -> &'static str {
    "project:
  name: gui_graph_edit_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins:
        A: net_a
        B: gnd
  nets:
    net_a:
      kind: digital_or_analog
    gnd:
      kind: ground
"
}

#[test]
fn add_and_remove_component_emit_valid_yaml() {
    let edited = add_component(
        editable_project_yaml(),
        "U2",
        "generic.schematic.imported_component",
    )
    .unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("U2:"));
    assert!(edited.contains("generic.schematic.imported_component"));

    let edited = remove_component(&edited, "U2").unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(!edited.contains("U2:"));
}

#[test]
fn add_component_with_ports_creates_default_pin_nets() {
    let ports = vec![
        ("VIN".to_string(), "electrical_power".to_string()),
        ("GND".to_string(), "electrical_ground".to_string()),
        ("OUT".to_string(), "digital_electrical_output".to_string()),
    ];
    let edited = add_component_with_ports(
        editable_project_yaml(),
        "U2",
        "vendor.example.power_stage",
        &ports,
    )
    .unwrap();

    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("U2:"));
    assert!(edited.contains("VIN: u2_vin"));
    assert!(edited.contains("GND: u2_gnd"));
    assert!(edited.contains("OUT: u2_out"));
    assert!(edited.contains("u2_vin:\n      kind: power"));
    assert!(edited.contains("u2_gnd:\n      kind: ground"));
    assert!(edited.contains("u2_out:\n      kind: digital_or_analog"));
}

#[test]
fn add_component_with_ports_suffixes_existing_generated_net() {
    let ports = vec![("VIN".to_string(), "electrical_power".to_string())];
    let project = add_net(editable_project_yaml(), "u2_vin", "power").unwrap();
    let edited =
        add_component_with_ports(&project, "U2", "vendor.example.power_stage", &ports).unwrap();

    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("VIN: u2_vin_2"));
}

#[test]
fn layout_assigns_common_component_symbol_kinds() {
    let snapshot = ProjectSnapshot {
        name: "symbols".to_string(),
        components: 7,
        nets: 1,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![
            SketchComponent {
                id: "R1".to_string(),
                model: "generic.analog.resistor".to_string(),
                part_number: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
            },
            SketchComponent {
                id: "C1".to_string(),
                model: "generic.analog.capacitor".to_string(),
                part_number: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
            },
            SketchComponent {
                id: "L1".to_string(),
                model: "generic.analog.inductor".to_string(),
                part_number: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
            },
            SketchComponent {
                id: "D1".to_string(),
                model: "generic.analog.diode".to_string(),
                part_number: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
            },
            SketchComponent {
                id: "V1".to_string(),
                model: "generic.analog.voltage_source".to_string(),
                part_number: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
            },
            SketchComponent {
                id: "J1".to_string(),
                model: "vendor.example.connector".to_string(),
                part_number: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
            },
            SketchComponent {
                id: "U1".to_string(),
                model: "vendor.example.controller".to_string(),
                part_number: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
            },
        ],
        nets_detail: vec![SketchNet {
            id: "gnd".to_string(),
            kind: "ground".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: Vec::new(),
            position: None,
        }],
    };

    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(620.0, 900.0)),
        &snapshot,
    );
    let symbols_by_label: std::collections::BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|node| (node.label.as_str(), node.symbol))
        .collect();

    assert_eq!(symbols_by_label["R1"], SketchSymbolKind::Resistor);
    assert_eq!(symbols_by_label["C1"], SketchSymbolKind::Capacitor);
    assert_eq!(symbols_by_label["L1"], SketchSymbolKind::Inductor);
    assert_eq!(symbols_by_label["D1"], SketchSymbolKind::Diode);
    assert_eq!(symbols_by_label["V1"], SketchSymbolKind::Source);
    assert_eq!(symbols_by_label["J1"], SketchSymbolKind::Connector);
    assert_eq!(symbols_by_label["U1"], SketchSymbolKind::Ic);
    assert_eq!(symbols_by_label["gnd"], SketchSymbolKind::Net);
}

#[test]
fn edit_schematic_component_style_emits_valid_yaml() {
    let edited = edit_schematic_component_style(
        editable_project_yaml(),
        "R1",
        SketchNodeStyle {
            rotation_deg: 90,
            mirrored: true,
            pin_side: SketchPinSide::Left,
        },
    )
    .unwrap();

    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("node_styles:"));
    assert!(edited.contains("component:R1:"));
    assert!(edited.contains("rotation_deg: 90"));
    assert!(edited.contains("mirrored: true"));
    assert!(edited.contains("pin_side: left"));
}

#[test]
fn layout_uses_schematic_style_for_left_pin_anchors() {
    let edited = edit_schematic_component_style(
        editable_project_yaml(),
        "R1",
        SketchNodeStyle {
            rotation_deg: 180,
            mirrored: false,
            pin_side: SketchPinSide::Left,
        },
    )
    .unwrap();
    let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(620.0, 340.0)),
        &snapshot,
    );
    let component = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("R1".to_string()))
        .unwrap();
    let anchor = graph
        .pin_anchors
        .iter()
        .find(|anchor| anchor.component_id == "R1" && anchor.pin == "A")
        .unwrap();

    assert_eq!(component.style.rotation_deg, 180);
    assert_eq!(component.style.pin_side, SketchPinSide::Left);
    assert_eq!(anchor.pos.x, component.rect.left());
    assert_eq!(anchor.label_align, egui::Align2::LEFT_CENTER);
}

#[test]
fn add_and_remove_unreferenced_net_emit_valid_yaml() {
    let edited = add_net(editable_project_yaml(), "sense_new", "digital_or_analog").unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("sense_new:"));

    let edited = remove_net(&edited, "sense_new").unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(!edited.contains("sense_new:"));
}

#[test]
fn remove_referenced_net_fails_closed() {
    let error = remove_net(editable_project_yaml(), "net_a").unwrap_err();
    assert!(error.to_string().contains("R1.A"));
}

#[test]
fn assign_and_remove_component_pin_emit_valid_yaml() {
    let edited = add_net(editable_project_yaml(), "sense_new", "digital_or_analog").unwrap();
    let edited = assign_component_pin(&edited, "R1", "SENSE", "sense_new").unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("SENSE: sense_new"));

    let edited = remove_component_pin(&edited, "R1", "SENSE").unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(!edited.contains("SENSE: sense_new"));
}

#[test]
fn assign_component_pin_rejects_missing_net() {
    let error =
        assign_component_pin(editable_project_yaml(), "R1", "SENSE", "missing_net").unwrap_err();
    assert!(error.to_string().contains("missing_net"));
}

#[test]
fn connect_component_pins_reuses_source_net() {
    let project = add_component(
        editable_project_yaml(),
        "U2",
        "generic.schematic.imported_component",
    )
    .unwrap();
    let project = assign_component_pin(&project, "U2", "P1", "gnd").unwrap();
    let edited = connect_component_pins(&project, "R1", "A", "U2", "P1").unwrap();

    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("A: net_a"));
    assert!(edited.contains("P1: net_a"));
}

#[test]
fn connect_component_pins_creates_net_when_both_pins_are_unbound() {
    let project = add_component(
        editable_project_yaml(),
        "U2",
        "generic.schematic.imported_component",
    )
    .unwrap();
    let edited = connect_component_pins(&project, "R1", "SENSE", "U2", "P1").unwrap();

    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("SENSE: net_r1_sense"));
    assert!(edited.contains("P1: net_r1_sense"));
    assert!(edited.contains("net_r1_sense:\n      kind: digital_or_analog"));
}

#[test]
fn add_component_rejects_unsafe_gui_id() {
    let error = add_component(
        editable_project_yaml(),
        "bad id",
        "generic.schematic.imported_component",
    )
    .unwrap_err();
    assert!(error.to_string().contains("unsupported characters"));
}

#[test]
fn edit_schematic_node_position_emits_valid_yaml() {
    let edited = edit_schematic_node_position(
        editable_project_yaml(),
        &SketchSelection::Component("R1".to_string()),
        42.0,
        84.0,
    )
    .unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("component:R1"));
    assert!(edited.contains("x: 42"));
    assert!(edited.contains("y: 84"));
}

#[test]
fn edit_schematic_node_positions_batches_valid_yaml() {
    let edited = edit_schematic_node_positions(
        editable_project_yaml(),
        &[
            (SketchSelection::Component("R1".to_string()), 42.0, 24.0),
            (SketchSelection::Net("net_a".to_string()), 220.0, 36.0),
        ],
    )
    .unwrap();

    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("component:R1"));
    assert!(edited.contains("net:net_a"));
    assert!(edited.contains("x: 220"));
}

#[test]
fn sketch_graph_layout_uses_saved_node_position() {
    let snapshot = ProjectSnapshot {
        name: "positioned_graph".to_string(),
        components: 1,
        nets: 1,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![SketchComponent {
            id: "R1".to_string(),
            model: "generic.analog.resistor".to_string(),
            part_number: None,
            position: Some(SketchPosition { x: 50.0, y: 70.0 }),
            pins: vec![SketchPin {
                pin: "A".to_string(),
                net: "net_a".to_string(),
            }],
            style: SketchNodeStyle::default(),
        }],
        nets_detail: vec![SketchNet {
            id: "net_a".to_string(),
            kind: "digital_or_analog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec!["R1.A".to_string()],
            position: Some(SketchPosition { x: 310.0, y: 70.0 }),
        }],
    };
    let graph = layout_sketch_graph(
        eframe::egui::Rect::from_min_size(
            eframe::egui::pos2(10.0, 20.0),
            eframe::egui::vec2(640.0, 320.0),
        ),
        &snapshot,
    );
    let component = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("R1".to_string()))
        .unwrap();
    assert_eq!(component.rect.left(), 60.0);
    assert_eq!(component.rect.top(), 90.0);
    assert_eq!(graph.edges.len(), 1);
}

#[test]
fn sketch_graph_layout_renders_pin_anchors() {
    let snapshot = ProjectSnapshot {
        name: "pin_graph".to_string(),
        components: 1,
        nets: 2,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![SketchComponent {
            id: "U1".to_string(),
            model: "vendor.example.dual_pin".to_string(),
            part_number: None,
            position: None,
            pins: vec![
                SketchPin {
                    pin: "VIN".to_string(),
                    net: "vin".to_string(),
                },
                SketchPin {
                    pin: "GND".to_string(),
                    net: "gnd".to_string(),
                },
            ],
            style: SketchNodeStyle::default(),
        }],
        nets_detail: vec![
            SketchNet {
                id: "vin".to_string(),
                kind: "power".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["U1.VIN".to_string()],
                position: None,
            },
            SketchNet {
                id: "gnd".to_string(),
                kind: "ground".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["U1.GND".to_string()],
                position: None,
            },
        ],
    };
    let graph = layout_sketch_graph(
        eframe::egui::Rect::from_min_size(
            eframe::egui::pos2(0.0, 0.0),
            eframe::egui::vec2(720.0, 360.0),
        ),
        &snapshot,
    );

    assert_eq!(graph.pin_anchors.len(), 2);
    let vin_anchor = graph
        .pin_anchors
        .iter()
        .find(|anchor| anchor.component_id == "U1" && anchor.pin == "VIN")
        .unwrap();
    assert_eq!(vin_anchor.net, "vin");
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.start.distance(vin_anchor.pos) < 0.01)
    );
}

#[test]
fn sketch_graph_viewport_transforms_nodes_and_edges() {
    let snapshot = ProjectSnapshot {
        name: "viewport_graph".to_string(),
        components: 1,
        nets: 1,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![SketchComponent {
            id: "R1".to_string(),
            model: "generic.analog.resistor".to_string(),
            part_number: None,
            position: Some(SketchPosition { x: 20.0, y: 30.0 }),
            pins: vec![SketchPin {
                pin: "A".to_string(),
                net: "net_a".to_string(),
            }],
            style: SketchNodeStyle::default(),
        }],
        nets_detail: vec![SketchNet {
            id: "net_a".to_string(),
            kind: "digital_or_analog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec!["R1.A".to_string()],
            position: Some(SketchPosition { x: 300.0, y: 30.0 }),
        }],
    };
    let canvas = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(640.0, 320.0));
    let graph = layout_sketch_graph_viewport(
        canvas,
        &snapshot,
        SketchViewport {
            pan: egui::vec2(12.0, -8.0),
            zoom: 2.0,
        },
    );
    let component = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("R1".to_string()))
        .unwrap();

    assert_eq!(component.rect.left(), 62.0);
    assert_eq!(component.rect.top(), 62.0);
    assert!(component.rect.width() > 250.0);
    assert_eq!(graph.edges.len(), 1);
}

#[test]
fn sketch_graph_bounds_excludes_overflow_hints() {
    let snapshot = ProjectSnapshot {
        name: "bounds".to_string(),
        components: 1,
        nets: 1,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![SketchComponent {
            id: "U1".to_string(),
            model: "generic.ic".to_string(),
            part_number: None,
            position: Some(SketchPosition { x: 20.0, y: 30.0 }),
            pins: vec![SketchPin {
                pin: "OUT".to_string(),
                net: "net_a".to_string(),
            }],
            style: SketchNodeStyle::default(),
        }],
        nets_detail: vec![SketchNet {
            id: "net_a".to_string(),
            kind: "digital_or_analog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec!["U1.OUT".to_string()],
            position: Some(SketchPosition { x: 360.0, y: 90.0 }),
        }],
    };
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
    let graph = layout_sketch_graph(canvas, &snapshot);
    let bounds = sketch_graph_bounds(&graph).unwrap();

    assert!(bounds.left() <= 20.0);
    assert!(bounds.right() >= 360.0);
    assert!(bounds.bottom() >= 90.0);
}

#[test]
fn persisted_node_position_inverts_viewport_transform() {
    let canvas = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(640.0, 320.0));
    let viewport = SketchViewport {
        pan: egui::vec2(12.0, -8.0),
        zoom: 2.0,
    };
    let screen_node = egui::Rect::from_min_size(egui::pos2(62.0, 62.0), egui::vec2(300.0, 184.0));
    let (x, y) = persisted_node_position_from_screen(
        canvas,
        egui::pos2(62.0 + 150.0, 62.0 + 92.0),
        screen_node,
        viewport,
    );

    assert_eq!(x, 20.0);
    assert_eq!(y, 30.0);
}

#[test]
fn persisted_node_position_snaps_to_grid_after_viewport_inverse() {
    let canvas = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(640.0, 320.0));
    let viewport = SketchViewport {
        pan: egui::vec2(12.0, -8.0),
        zoom: 2.0,
    };
    let screen_node = egui::Rect::from_min_size(egui::pos2(62.0, 62.0), egui::vec2(300.0, 184.0));
    let (x, y) = persisted_node_position_from_screen_with_snap(
        canvas,
        egui::pos2(62.0 + 150.0, 62.0 + 92.0),
        screen_node,
        viewport,
        true,
        16.0,
    );

    assert_eq!(x, 16.0);
    assert_eq!(y, 32.0);
}

#[test]
fn screen_point_snap_rounds_in_logical_canvas_space() {
    let canvas = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(640.0, 320.0));
    let viewport = SketchViewport {
        pan: egui::vec2(12.0, -8.0),
        zoom: 2.0,
    };
    let snapped = snap_screen_point_to_grid(canvas, egui::pos2(89.0, 73.0), viewport, true, 16.0);

    assert_eq!(snapped, egui::pos2(86.0, 66.0));
}

#[test]
fn orthogonal_wire_points_use_midpoint_route() {
    let points = orthogonal_wire_points(egui::pos2(10.0, 20.0), egui::pos2(90.0, 70.0));

    assert_eq!(
        points,
        vec![
            egui::pos2(10.0, 20.0),
            egui::pos2(50.0, 20.0),
            egui::pos2(50.0, 70.0),
            egui::pos2(90.0, 70.0),
        ]
    );
}

#[test]
fn multi_selected_items_delete_as_one_validated_edit() {
    let yaml = "project:
  name: gui_delete_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins:
        A: net_a
        B: gnd
  nets:
    net_a:
      kind: digital_or_analog
    gnd:
      kind: ground
    loose:
      kind: digital_or_analog
";
    let mut app = CircuitCiApp {
        project_yaml: yaml.to_string(),
        project_snapshot: Some(load_project_snapshot_from_yaml(yaml).unwrap()),
        ..CircuitCiApp::default()
    };
    app.selected_sketch_items
        .insert(SketchSelection::Component("R1".to_string()));
    app.selected_sketch_items
        .insert(SketchSelection::Net("loose".to_string()));

    app.apply_delete_selected_sketch_item();

    validate_board_ir_yaml_text(&app.project_yaml).unwrap();
    assert!(!app.project_yaml.contains("R1:"));
    assert!(!app.project_yaml.contains("loose:"));
    assert!(app.project_yaml.contains("gnd:"));
    assert!(app.selected_sketch_items.is_empty());
    assert_eq!(app.project_yaml_undo.len(), 1);
}

#[test]
fn group_screen_delta_moves_selected_items_as_one_edit() {
    let yaml = "project:
  name: gui_group_move_test
  version: 0.1.0
board:
  components:
    U1:
      model: generic.ic
      pins:
        OUT: net_a
    U2:
      model: generic.ic
      pins:
        IN: net_a
  nets:
    net_a:
      kind: digital_or_analog
  schematic:
    node_positions:
      component:U1:
        x: 20.0
        y: 30.0
      component:U2:
        x: 260.0
        y: 30.0
";
    let snapshot = load_project_snapshot_from_yaml(yaml).unwrap();
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
    let viewport = SketchViewport {
        pan: egui::Vec2::ZERO,
        zoom: 1.0,
    };
    let graph = layout_sketch_graph_viewport(canvas, &snapshot, viewport);
    let mut app = CircuitCiApp {
        project_yaml: yaml.to_string(),
        project_snapshot: Some(snapshot),
        ..CircuitCiApp::default()
    };
    app.selected_sketch_items
        .insert(SketchSelection::Component("U1".to_string()));
    app.selected_sketch_items
        .insert(SketchSelection::Component("U2".to_string()));

    app.apply_selected_schematic_screen_delta(
        canvas,
        &graph,
        viewport,
        egui::vec2(12.0, 8.0),
        "moved",
    );

    validate_board_ir_yaml_text(&app.project_yaml).unwrap();
    assert!(app.project_yaml.contains("x: 32.0"));
    assert!(app.project_yaml.contains("x: 272.0"));
    assert!(app.project_yaml.contains("y: 32.0"));
    assert_eq!(app.selected_sketch_items.len(), 2);
    assert_eq!(app.project_yaml_undo.len(), 1);
}

#[test]
fn fit_sketch_content_places_transformed_bounds_inside_canvas() {
    let snapshot = ProjectSnapshot {
        name: "fit".to_string(),
        components: 2,
        nets: 1,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![
            SketchComponent {
                id: "U1".to_string(),
                model: "generic.ic".to_string(),
                part_number: None,
                position: Some(SketchPosition { x: 20.0, y: 40.0 }),
                pins: vec![SketchPin {
                    pin: "OUT".to_string(),
                    net: "far_net".to_string(),
                }],
                style: SketchNodeStyle::default(),
            },
            SketchComponent {
                id: "U2".to_string(),
                model: "generic.ic".to_string(),
                part_number: None,
                position: Some(SketchPosition { x: 820.0, y: 420.0 }),
                pins: vec![SketchPin {
                    pin: "IN".to_string(),
                    net: "far_net".to_string(),
                }],
                style: SketchNodeStyle::default(),
            },
        ],
        nets_detail: vec![SketchNet {
            id: "far_net".to_string(),
            kind: "digital_or_analog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec!["U1.OUT".to_string(), "U2.IN".to_string()],
            position: Some(SketchPosition { x: 460.0, y: 240.0 }),
        }],
    };
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
    let mut app = CircuitCiApp::default();

    app.fit_sketch_content(canvas, &snapshot);
    let graph = layout_sketch_graph_viewport(canvas, &snapshot, app.sketch_viewport());
    let bounds = sketch_graph_bounds(&graph).unwrap();
    let viewport = canvas.shrink(24.0);

    assert!(app.sketch_zoom < 1.0);
    assert!(viewport.contains(bounds.left_top()));
    assert!(viewport.contains(bounds.right_bottom()));
}
