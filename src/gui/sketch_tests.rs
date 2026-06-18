use super::sketch::{ProjectSnapshot, SketchComponent, SketchNet, SketchPin};
use super::sketch::{
    SketchNodeStyle, SketchPinSide, SketchPosition, SketchSelection, SketchViewport, add_component,
    add_component_with_ports, add_net, assign_component_pin, connect_component_pins,
    edge_label_position, edit_schematic_component_style, edit_schematic_component_styles,
    edit_schematic_node_position, edit_schematic_node_positions, edit_schematic_wire_route,
    hit_test_wire, layout_sketch_graph, layout_sketch_graph_viewport,
    load_project_snapshot_from_yaml, orthogonal_wire_points, persisted_node_position_from_screen,
    persisted_node_position_from_screen_with_snap, remove_component, remove_component_pin,
    remove_net, remove_schematic_wire_route, sketch_graph_bounds, sketch_wire_points,
    snap_screen_point_to_grid, validate_board_ir_yaml_text, wire_route_key,
};
use super::sketch_canvas::schematic_canvas_size;
use super::sketch_duplicate::duplicate_components_with_local_nets;
use super::sketch_probes::{
    SketchProbe, SketchProbeQuantity, SketchProbeTarget, hit_test_probe_badge,
};
use super::sketch_rename::{rename_component, rename_net};
use super::sketch_symbols::SketchSymbolKind;
use super::{CircuitCiApp, SketchViewportCommand};
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

fn probe_badge_project_yaml() -> &'static str {
    "project:
  name: probe_badge_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000
      pins:
        A: rail
        B: out
  nets:
    rail:
      kind: power
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
  schematic:
    node_positions:
      component:R1: { x: 10.0, y: 20.0 }
      net:out: { x: 90.0, y: 120.0 }
    node_styles:
      component:R1: { rotation_deg: 90, mirrored: true, pin_side: left }
    wire_routes:
      R1.B->out:
        points:
          - { x: 140.0, y: 150.0 }
    net_labels:
      label_out:
        net: out
        x: 168.0
        y: 196.0
        kind: off_page
scenarios:
  - name: tran
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [R1]
      model_files: []
      node_bindings:
        - { net: rail, node: rail_5v }
        - { net: out, node: out }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: R1, pin: A }, node: rail_5v }
        - { endpoint: { component: R1, pin: B }, node: out }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      probes:
        - { name: rail_voltage, expression: 'V(rail_5v)', quantity: voltage }
        - { name: r1_current, expression: 'I(VCCI_R1)', quantity: current }
        - { name: r1_power, expression: 'V(rail_5v,out)*I(VCCI_R1)', quantity: power }
      assertions:
        - { name: rail_voltage_min, probe: rail_voltage, at_us: 5, relation: above, threshold_v: 4.5 }
"
}

fn component_position(snapshot: &ProjectSnapshot, component_id: &str) -> (f64, f64) {
    let position = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == component_id)
        .and_then(|component| component.position)
        .unwrap_or_else(|| panic!("component {component_id} has no schematic position"));
    (position.x, position.y)
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

    let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(620.0, 340.0)),
        &snapshot,
    );
    let pin_kinds = graph
        .pin_anchors
        .iter()
        .filter(|anchor| anchor.component_id == "U2")
        .map(|anchor| (anchor.pin.as_str(), anchor.kind.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(pin_kinds["VIN"], "power");
    assert_eq!(pin_kinds["GND"], "ground");
    assert_eq!(pin_kinds["OUT"], "digital_or_analog");
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
fn schematic_canvas_size_prefers_model_editor_space() {
    assert_eq!(
        schematic_canvas_size(egui::vec2(1200.0, 720.0)),
        egui::vec2(1200.0, 720.0)
    );
    assert_eq!(
        schematic_canvas_size(egui::vec2(320.0, 240.0)),
        egui::vec2(560.0, 520.0)
    );
}

#[test]
fn duplicate_components_copies_local_nets_and_offsets_positions() {
    let yaml = "project:
  name: duplicate_test
  version: 0.1.0
board:
  schematic:
    node_positions:
      component:R1: { x: 10.0, y: 20.0 }
      component:C1: { x: 20.0, y: 80.0 }
      net:local: { x: 160.0, y: 40.0 }
    node_styles:
      component:R1: { rotation_deg: 90, mirrored: true, pin_side: left }
  components:
    R1:
      model: generic.analog.resistor
      part_number: RC0603
      source:
        format: kicad_schematic
        instances:
          - { project: imported.kicad_sch, path: /sheet, reference: R1, unit: 1 }
      pins:
        A: local
        B: gnd
    C1:
      model: generic.analog.capacitor
      pins:
        A: local
        B: gnd
    U1:
      model: generic.ic
      pins:
        GND: gnd
  nets:
    local:
      kind: digital_or_analog
      nominal_voltage: 1.2
    gnd:
      kind: ground
";
    let (edited, selections) = duplicate_components_with_local_nets(
        yaml,
        &["R1".to_string(), "C1".to_string()],
        egui::vec2(32.0, 48.0),
    )
    .unwrap();

    validate_board_ir_yaml_text(&edited).unwrap();
    let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
    assert!(snapshot.components_detail.iter().any(|component| {
        component.id == "R2"
            && component.part_number.as_deref() == Some("RC0603")
            && component.source_paths.is_empty()
            && component.pins.iter().any(|pin| pin.net == "local_copy")
            && component.pins.iter().any(|pin| pin.net == "gnd")
            && component.position.is_some_and(|position| {
                (position.x - 42.0).abs() < f64::EPSILON && (position.y - 68.0).abs() < f64::EPSILON
            })
            && component.style.rotation_deg == 90
            && component.style.mirrored
            && component.style.pin_side == SketchPinSide::Left
    }));
    assert!(snapshot.components_detail.iter().any(|component| {
        component.id == "C2" && component.pins.iter().any(|pin| pin.net == "local_copy")
    }));
    assert!(snapshot.nets_detail.iter().any(|net| {
        net.id == "local_copy"
            && net.nominal_voltage == Some(1.2)
            && net.position.is_some_and(|position| {
                (position.x - 192.0).abs() < f64::EPSILON
                    && (position.y - 88.0).abs() < f64::EPSILON
            })
    }));
    assert!(!snapshot.nets_detail.iter().any(|net| net.id == "gnd_copy"));
    assert!(selections.contains(&SketchSelection::Component("R2".to_string())));
    assert!(selections.contains(&SketchSelection::Component("C2".to_string())));
    assert!(selections.contains(&SketchSelection::Net("local_copy".to_string())));
}

#[test]
fn duplicate_component_keeps_external_nets_shared() {
    let yaml = "project:
  name: duplicate_external_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins:
        A: net_a
        B: gnd
    U1:
      model: generic.ic
      pins:
        IN: net_a
        GND: gnd
  nets:
    net_a:
      kind: digital_or_analog
    gnd:
      kind: ground
";
    let (edited, selections) =
        duplicate_components_with_local_nets(yaml, &["R1".to_string()], egui::vec2(32.0, 32.0))
            .unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
    let r2 = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == "R2")
        .unwrap();

    assert!(r2.pins.iter().any(|pin| pin.net == "net_a"));
    assert!(r2.pins.iter().any(|pin| pin.net == "gnd"));
    assert!(
        !snapshot
            .nets_detail
            .iter()
            .any(|net| net.id == "net_a_copy")
    );
    assert_eq!(
        selections,
        vec![SketchSelection::Component("R2".to_string())]
    );
}

#[test]
fn copy_paste_components_places_duplicate_group_at_target() {
    let yaml = "project:
  name: paste_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins:
        A: local
        B: gnd
    C1:
      model: generic.analog.capacitor
      pins:
        A: local
        B: gnd
  nets:
    local:
      kind: digital_or_analog
    gnd:
      kind: ground
  schematic:
    node_positions:
      component:R1:
        x: 20.0
        y: 30.0
      component:C1:
        x: 160.0
        y: 30.0
      net:local:
        x: 90.0
        y: 140.0
";
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 360.0));
    let target = egui::pos2(320.0, 240.0);
    let mut app = CircuitCiApp {
        project_yaml: yaml.to_string(),
        project_snapshot: Some(load_project_snapshot_from_yaml(yaml).unwrap()),
        ..CircuitCiApp::default()
    };
    app.selected_sketch_items
        .insert(SketchSelection::Component("R1".to_string()));
    app.selected_sketch_items
        .insert(SketchSelection::Component("C1".to_string()));

    app.apply_copy_selected_sketch_items();
    app.apply_paste_sketch_clipboard(canvas, Some(target));

    validate_board_ir_yaml_text(&app.project_yaml).unwrap();
    let snapshot = load_project_snapshot_from_yaml(&app.project_yaml).unwrap();
    assert!(
        snapshot
            .components_detail
            .iter()
            .any(|component| component.id == "R2"
                && component.pins.iter().any(|pin| pin.net == "local_copy"))
    );
    assert!(
        snapshot
            .components_detail
            .iter()
            .any(|component| component.id == "C2"
                && component.pins.iter().any(|pin| pin.net == "local_copy"))
    );
    assert!(
        app.selected_sketch_items
            .contains(&SketchSelection::Component("R2".to_string()))
    );
    assert!(
        app.selected_sketch_items
            .contains(&SketchSelection::Net("local_copy".to_string()))
    );
    assert_eq!(app.sketch_clipboard_components, vec!["C1", "R1"]);
    assert_eq!(app.project_yaml_undo.len(), 1);

    let graph = layout_sketch_graph_viewport(canvas, &snapshot, app.sketch_viewport());
    let pasted_bounds = graph
        .nodes
        .iter()
        .filter(|node| app.selected_sketch_items.contains(&node.selection))
        .map(|node| node.rect)
        .reduce(|accumulator, rect| accumulator.union(rect))
        .unwrap();
    assert!(pasted_bounds.center().distance(target) <= app.sketch_grid_step);
}

#[test]
fn snapshot_derives_probe_badges_from_analog_probes() {
    let snapshot = load_project_snapshot_from_yaml(
        "project:
  name: probe_badge_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000
      pins:
        A: rail
        B: out
  nets:
    rail:
      kind: power
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
scenarios:
  - name: tran
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [R1]
      model_files: []
      node_bindings:
        - { net: rail, node: rail_5v }
        - { net: out, node: out }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: R1, pin: A }, node: rail_5v }
        - { endpoint: { component: R1, pin: B }, node: out }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      probes:
        - { name: rail_voltage, expression: 'V(rail_5v)', quantity: voltage }
        - { name: r1_current, expression: 'I(VCCI_R1)', quantity: current }
        - { name: r1_power, expression: 'V(rail_5v,out)*I(VCCI_R1)', quantity: power }
      assertions:
        - { name: rail_voltage_min, probe: rail_voltage, at_us: 5, relation: above, threshold_v: 4.5 }
",
    )
    .unwrap();

    assert_eq!(snapshot.probes.len(), 3);
    assert!(snapshot.probes.iter().any(|probe| {
        probe.probe_name == "rail_voltage"
            && probe.quantity.label() == "V"
            && probe.assertion_names == ["rail_voltage_min".to_string()]
            && matches!(probe.target, SketchProbeTarget::Net(ref id) if id == "rail")
    }));
    assert!(snapshot.probes.iter().any(|probe| {
        probe.probe_name == "r1_current"
            && probe.quantity.label() == "I"
            && matches!(probe.target, SketchProbeTarget::Component(ref id) if id == "R1")
    }));
    assert!(snapshot.probes.iter().any(|probe| {
        probe.probe_name == "r1_power"
            && probe.quantity.label() == "P"
            && matches!(probe.target, SketchProbeTarget::Component(ref id) if id == "R1")
    }));
}

#[test]
fn layout_places_hit_testable_probe_badges() {
    let mut snapshot = load_project_snapshot_from_yaml(editable_project_yaml()).unwrap();
    snapshot.probes.push(SketchProbe {
        scenario_name: "tran".to_string(),
        probe_name: "net_a_voltage".to_string(),
        expression: "V(net_a)".to_string(),
        quantity: SketchProbeQuantity::Voltage,
        target: SketchProbeTarget::Net("net_a".to_string()),
        assertion_names: Vec::new(),
    });
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(620.0, 360.0)),
        &snapshot,
    );

    assert_eq!(graph.probe_badges.len(), 1);
    let badge =
        hit_test_probe_badge(&graph.probe_badges, graph.probe_badges[0].rect.center()).unwrap();
    assert_eq!(badge.probe.probe_name, "net_a_voltage");
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
                spice: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "C1".to_string(),
                model: "generic.analog.capacitor".to_string(),
                part_number: None,
                spice: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "L1".to_string(),
                model: "generic.analog.inductor".to_string(),
                part_number: None,
                spice: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "D1".to_string(),
                model: "generic.analog.diode".to_string(),
                part_number: None,
                spice: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "V1".to_string(),
                model: "generic.analog.voltage_source".to_string(),
                part_number: None,
                spice: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "J1".to_string(),
                model: "vendor.example.connector".to_string(),
                part_number: None,
                spice: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "U1".to_string(),
                model: "vendor.example.controller".to_string(),
                part_number: None,
                spice: None,
                pins: Vec::new(),
                position: None,
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
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
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
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
fn edit_schematic_component_styles_rotates_multiple_components_in_one_yaml_edit() {
    let with_c1 = add_component(editable_project_yaml(), "C1", "generic.analog.capacitor").unwrap();
    let edited = edit_schematic_component_styles(
        &with_c1,
        &[
            (
                "R1".to_string(),
                SketchNodeStyle {
                    rotation_deg: 90,
                    ..Default::default()
                },
            ),
            (
                "C1".to_string(),
                SketchNodeStyle {
                    rotation_deg: 270,
                    ..Default::default()
                },
            ),
        ],
    )
    .unwrap();

    validate_board_ir_yaml_text(&edited).unwrap();
    let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
    let by_id: std::collections::BTreeMap<_, _> = snapshot
        .components_detail
        .iter()
        .map(|component| (component.id.as_str(), component.style.rotation_deg))
        .collect();
    assert_eq!(by_id["R1"], 90);
    assert_eq!(by_id["C1"], 270);
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
            spice: None,
            position: Some(SketchPosition { x: 50.0, y: 70.0 }),
            pins: vec![SketchPin {
                pin: "A".to_string(),
                net: "net_a".to_string(),
            }],
            style: SketchNodeStyle::default(),
            source_paths: Vec::new(),
        }],
        nets_detail: vec![SketchNet {
            id: "net_a".to_string(),
            kind: "digital_or_analog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec!["R1.A".to_string()],
            position: Some(SketchPosition { x: 310.0, y: 70.0 }),
        }],
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
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
            spice: None,
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
            source_paths: Vec::new(),
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
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
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
            spice: None,
            position: Some(SketchPosition { x: 20.0, y: 30.0 }),
            pins: vec![SketchPin {
                pin: "A".to_string(),
                net: "net_a".to_string(),
            }],
            style: SketchNodeStyle::default(),
            source_paths: Vec::new(),
        }],
        nets_detail: vec![SketchNet {
            id: "net_a".to_string(),
            kind: "digital_or_analog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec!["R1.A".to_string()],
            position: Some(SketchPosition { x: 300.0, y: 30.0 }),
        }],
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
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
            spice: None,
            position: Some(SketchPosition { x: 20.0, y: 30.0 }),
            pins: vec![SketchPin {
                pin: "OUT".to_string(),
                net: "net_a".to_string(),
            }],
            style: SketchNodeStyle::default(),
            source_paths: Vec::new(),
        }],
        nets_detail: vec![SketchNet {
            id: "net_a".to_string(),
            kind: "digital_or_analog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec!["U1.OUT".to_string()],
            position: Some(SketchPosition { x: 360.0, y: 90.0 }),
        }],
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
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
fn sketch_graph_edges_carry_net_metadata_for_wire_inspection() {
    let snapshot = load_project_snapshot_from_yaml(editable_project_yaml()).unwrap();
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
    let graph = layout_sketch_graph(canvas, &snapshot);
    let edge = graph
        .edges
        .iter()
        .find(|edge| edge.net_id == "net_a" && edge.source == "R1.A")
        .unwrap();
    let hit = hit_test_wire(&graph, edge_label_position(edge)).unwrap();

    assert_eq!(hit.net_id, "net_a");
    assert_eq!(hit.source, "R1.A");
}

#[test]
fn sketch_graph_edges_use_schematic_wire_routes_for_display() {
    let yaml = "project:
  name: routed_wire_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins: { A: net_a, B: gnd }
  nets:
    net_a: { kind: digital_or_analog }
    gnd: { kind: ground }
  schematic:
    node_positions:
      component:R1: { x: 10.0, y: 20.0 }
      net:net_a: { x: 300.0, y: 100.0 }
    wire_routes:
      R1.A->net_a:
        points:
          - { x: 180.0, y: 140.0 }
";
    let snapshot = load_project_snapshot_from_yaml(yaml).unwrap();
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
    let graph = layout_sketch_graph(canvas, &snapshot);
    let edge = graph
        .edges
        .iter()
        .find(|edge| edge.net_id == "net_a" && edge.source == "R1.A")
        .unwrap();
    let points = sketch_wire_points(edge);

    assert_eq!(edge.route, vec![egui::pos2(180.0, 140.0)]);
    assert!(points.contains(&egui::pos2(180.0, 140.0)));
    let hit = hit_test_wire(&graph, egui::pos2(180.0, 140.0)).unwrap();
    assert_eq!(hit.source, "R1.A");
}

#[test]
fn edit_schematic_wire_route_emits_valid_display_metadata() {
    let edited =
        edit_schematic_wire_route(editable_project_yaml(), "R1.A", "net_a", &[(128.0, 144.0)])
            .unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("wire_routes:"));
    assert!(edited.contains("R1.A->net_a:"));

    let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
    assert_eq!(
        snapshot.wire_routes.get(&wire_route_key("R1.A", "net_a")),
        Some(&vec![SketchPosition { x: 128.0, y: 144.0 }])
    );
}

#[test]
fn edit_schematic_wire_route_rejects_wrong_net() {
    let error =
        edit_schematic_wire_route(editable_project_yaml(), "R1.A", "gnd", &[(128.0, 144.0)])
            .unwrap_err();

    assert!(error.to_string().contains("not gnd"));
}

#[test]
fn remove_schematic_wire_route_clears_only_display_metadata() {
    let routed =
        edit_schematic_wire_route(editable_project_yaml(), "R1.A", "net_a", &[(128.0, 144.0)])
            .unwrap();
    let edited = remove_schematic_wire_route(&routed, "R1.A", "net_a").unwrap();

    validate_board_ir_yaml_text(&edited).unwrap();
    let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
    assert!(snapshot.wire_routes.is_empty());
    assert!(snapshot.components_detail.iter().any(|component| {
        component.id == "R1"
            && component
                .pins
                .iter()
                .any(|pin| pin.pin == "A" && pin.net == "net_a")
    }));
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
fn group_alignment_actions_update_selected_edges_and_centers() {
    let yaml = "project:
  name: gui_group_align_test
  version: 0.1.0
board:
  components:
    U1:
      model: generic.ic
    U2:
      model: generic.ic
  nets: {}
  schematic:
    node_positions:
      component:U1:
        x: 20.0
        y: 30.0
      component:U2:
        x: 260.0
        y: 120.0
";
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 360.0));
    let viewport = SketchViewport {
        pan: egui::Vec2::ZERO,
        zoom: 1.0,
    };
    let mut app = CircuitCiApp {
        project_yaml: yaml.to_string(),
        project_snapshot: Some(load_project_snapshot_from_yaml(yaml).unwrap()),
        sketch_snap_enabled: false,
        ..CircuitCiApp::default()
    };
    app.selected_sketch_items
        .insert(SketchSelection::Component("U1".to_string()));
    app.selected_sketch_items
        .insert(SketchSelection::Component("U2".to_string()));

    let graph =
        layout_sketch_graph_viewport(canvas, app.project_snapshot.as_ref().unwrap(), viewport);
    app.apply_sketch_group_action(
        canvas,
        &graph,
        viewport,
        super::SketchGroupAction::AlignRight,
    );
    let snapshot = load_project_snapshot_from_yaml(&app.project_yaml).unwrap();
    assert_eq!(component_position(&snapshot, "U1"), (260.0, 30.0));
    assert_eq!(component_position(&snapshot, "U2"), (260.0, 120.0));

    let graph = layout_sketch_graph_viewport(canvas, &snapshot, viewport);
    app.apply_sketch_group_action(
        canvas,
        &graph,
        viewport,
        super::SketchGroupAction::AlignBottom,
    );
    let snapshot = load_project_snapshot_from_yaml(&app.project_yaml).unwrap();
    assert_eq!(component_position(&snapshot, "U1"), (260.0, 120.0));
    assert_eq!(component_position(&snapshot, "U2"), (260.0, 120.0));

    let graph = layout_sketch_graph_viewport(canvas, &snapshot, viewport);
    app.apply_sketch_group_action(
        canvas,
        &graph,
        viewport,
        super::SketchGroupAction::AlignCenterX,
    );
    let graph =
        layout_sketch_graph_viewport(canvas, app.project_snapshot.as_ref().unwrap(), viewport);
    app.apply_sketch_group_action(
        canvas,
        &graph,
        viewport,
        super::SketchGroupAction::AlignCenterY,
    );
    validate_board_ir_yaml_text(&app.project_yaml).unwrap();
    assert_eq!(app.project_yaml_undo.len(), 3);
}

#[test]
fn group_distribution_actions_space_selected_centers_evenly() {
    let yaml = "project:
  name: gui_group_distribute_test
  version: 0.1.0
board:
  components:
    U1:
      model: generic.ic
    U2:
      model: generic.ic
    U3:
      model: generic.ic
  nets: {}
  schematic:
    node_positions:
      component:U1:
        x: 20.0
        y: 40.0
      component:U2:
        x: 300.0
        y: 120.0
      component:U3:
        x: 140.0
        y: 260.0
";
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 420.0));
    let viewport = SketchViewport {
        pan: egui::Vec2::ZERO,
        zoom: 1.0,
    };
    let mut app = CircuitCiApp {
        project_yaml: yaml.to_string(),
        project_snapshot: Some(load_project_snapshot_from_yaml(yaml).unwrap()),
        sketch_snap_enabled: false,
        ..CircuitCiApp::default()
    };
    for component_id in ["U1", "U2", "U3"] {
        app.selected_sketch_items
            .insert(SketchSelection::Component(component_id.to_string()));
    }

    let graph =
        layout_sketch_graph_viewport(canvas, app.project_snapshot.as_ref().unwrap(), viewport);
    app.apply_sketch_group_action(
        canvas,
        &graph,
        viewport,
        super::SketchGroupAction::DistributeHorizontal,
    );
    let snapshot = load_project_snapshot_from_yaml(&app.project_yaml).unwrap();
    assert_eq!(component_position(&snapshot, "U1"), (20.0, 40.0));
    assert_eq!(component_position(&snapshot, "U3"), (160.0, 260.0));
    assert_eq!(component_position(&snapshot, "U2"), (300.0, 120.0));

    let graph = layout_sketch_graph_viewport(canvas, &snapshot, viewport);
    app.apply_sketch_group_action(
        canvas,
        &graph,
        viewport,
        super::SketchGroupAction::DistributeVertical,
    );
    let snapshot = load_project_snapshot_from_yaml(&app.project_yaml).unwrap();
    assert_eq!(component_position(&snapshot, "U1"), (20.0, 40.0));
    assert_eq!(component_position(&snapshot, "U2"), (300.0, 150.0));
    assert_eq!(component_position(&snapshot, "U3"), (160.0, 260.0));
    assert_eq!(app.project_yaml_undo.len(), 2);
}

#[test]
fn sketch_layout_keeps_offscreen_rows_pannable_for_navigator_fit() {
    let components_detail = (0..8)
        .map(|index| SketchComponent {
            id: format!("U{index}"),
            model: "generic.ic".to_string(),
            part_number: None,
            spice: None,
            pins: vec![SketchPin {
                pin: "OUT".to_string(),
                net: "net_a".to_string(),
            }],
            position: None,
            style: SketchNodeStyle::default(),
            source_paths: Vec::new(),
        })
        .collect();
    let snapshot = ProjectSnapshot {
        name: "pannable".to_string(),
        components: 8,
        nets: 1,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail,
        nets_detail: vec![SketchNet {
            id: "net_a".to_string(),
            kind: "digital_or_analog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec!["U7.OUT".to_string()],
            position: None,
        }],
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
    };
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 220.0));

    let graph = layout_sketch_graph(canvas, &snapshot);
    let last = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("U7".to_string()))
        .unwrap();

    assert!(last.rect.top() > canvas.bottom());
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
                spice: None,
                position: Some(SketchPosition { x: 20.0, y: 40.0 }),
                pins: vec![SketchPin {
                    pin: "OUT".to_string(),
                    net: "far_net".to_string(),
                }],
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "U2".to_string(),
                model: "generic.ic".to_string(),
                part_number: None,
                spice: None,
                position: Some(SketchPosition { x: 820.0, y: 420.0 }),
                pins: vec![SketchPin {
                    pin: "IN".to_string(),
                    net: "far_net".to_string(),
                }],
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
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
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
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

#[test]
fn fit_selected_sketch_content_places_selection_inside_canvas() {
    let snapshot = ProjectSnapshot {
        name: "fit_selection".to_string(),
        components: 2,
        nets: 1,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![
            SketchComponent {
                id: "U1".to_string(),
                model: "generic.ic".to_string(),
                part_number: None,
                spice: None,
                position: Some(SketchPosition { x: 20.0, y: 40.0 }),
                pins: vec![SketchPin {
                    pin: "OUT".to_string(),
                    net: "far_net".to_string(),
                }],
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
            },
            SketchComponent {
                id: "U2".to_string(),
                model: "generic.ic".to_string(),
                part_number: None,
                spice: None,
                position: Some(SketchPosition { x: 880.0, y: 520.0 }),
                pins: vec![SketchPin {
                    pin: "IN".to_string(),
                    net: "far_net".to_string(),
                }],
                style: SketchNodeStyle::default(),
                source_paths: Vec::new(),
            },
        ],
        nets_detail: vec![SketchNet {
            id: "far_net".to_string(),
            kind: "digital_or_analog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec!["U1.OUT".to_string(), "U2.IN".to_string()],
            position: Some(SketchPosition { x: 460.0, y: 280.0 }),
        }],
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
    };
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
    let mut app = CircuitCiApp {
        sketch_zoom: 3.0,
        sketch_pan: egui::vec2(-900.0, -400.0),
        ..Default::default()
    };
    app.set_single_sketch_selection(Some(SketchSelection::Net("far_net".to_string())));

    app.apply_sketch_viewport_command(canvas, &snapshot, SketchViewportCommand::FitSelection);

    let graph = layout_sketch_graph_viewport(canvas, &snapshot, app.sketch_viewport());
    let net_node = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Net("far_net".to_string()))
        .unwrap();
    let viewport = canvas.shrink(24.0);
    assert!(viewport.contains(net_node.rect.center()));
    assert!(app.status.starts_with("Fit 1 selected"));
}

#[test]
fn home_viewport_command_resets_zoom_and_pan() {
    let snapshot = ProjectSnapshot {
        name: "home".to_string(),
        components: 0,
        nets: 0,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: Vec::new(),
        nets_detail: Vec::new(),
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
    };
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
    let mut app = CircuitCiApp {
        sketch_zoom: 2.4,
        sketch_pan: egui::vec2(-180.0, 95.0),
        ..Default::default()
    };

    app.apply_sketch_viewport_command(canvas, &snapshot, SketchViewportCommand::Home);

    assert_eq!(app.sketch_zoom, 1.0);
    assert_eq!(app.sketch_pan, egui::Vec2::ZERO);
}

#[test]
fn rename_component_updates_board_schematic_and_generated_analog_refs() {
    let edited = rename_component(probe_badge_project_yaml(), "R1", "R_SENSE").unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let board = &project.board;

    assert!(!board.components.contains_key("R1"));
    assert!(board.components.contains_key("R_SENSE"));

    let scenario = &project.scenarios[0];
    let analog = scenario.analog.as_ref().unwrap();
    let generated = analog.generated.as_ref().unwrap();
    assert_eq!(generated.components, ["R_SENSE".to_string()]);
    assert!(analog.pin_bindings.iter().all(|binding| {
        binding.endpoint.component == "R_SENSE" || binding.endpoint.component != "R1"
    }));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| { probe.name == "r1_current" && probe.expression == "I(VCCI_R_SENSE)" })
    );
    assert!(analog.probes.iter().any(|probe| {
        probe.name == "r1_power" && probe.expression == "V(rail_5v,out)*I(VCCI_R_SENSE)"
    }));

    let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
    assert!(snapshot.components_detail.iter().any(|item| {
        item.id == "R_SENSE"
            && item.style.rotation_deg == 90
            && item.style.mirrored
            && item.style.pin_side == SketchPinSide::Left
    }));
    assert!(
        snapshot
            .components_detail
            .iter()
            .all(|item| item.id != "R1")
    );
    assert!(snapshot.wire_routes.contains_key("R_SENSE.B->out"));
    assert!(!snapshot.wire_routes.contains_key("R1.B->out"));
}

#[test]
fn rename_net_updates_pins_schematic_and_generated_analog_refs() {
    let edited = rename_net(probe_badge_project_yaml(), "out", "sense_out").unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let board = &project.board;

    assert!(!board.nets.contains_key("out"));
    assert!(board.nets.contains_key("sense_out"));
    assert_eq!(
        board.components.get("R1").unwrap().pins.get("B").unwrap(),
        "sense_out"
    );

    let analog = project.scenarios[0].analog.as_ref().unwrap();
    assert!(
        analog
            .node_bindings
            .iter()
            .any(|binding| { binding.net == "sense_out" && binding.node == "out" })
    );
    assert!(
        analog
            .node_bindings
            .iter()
            .all(|binding| binding.net != "out")
    );

    let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
    assert!(snapshot.wire_routes.contains_key("R1.B->sense_out"));
    assert!(!snapshot.wire_routes.contains_key("R1.B->out"));
    assert_eq!(snapshot.net_labels[0].net_id, "sense_out");
    assert!(snapshot.nets_detail.iter().any(|item| {
        item.id == "sense_out"
            && item
                .position
                .as_ref()
                .is_some_and(|position| position.x == 90.0 && position.y == 120.0)
    }));
    assert!(snapshot.nets_detail.iter().all(|item| item.id != "out"));
}

#[test]
fn rename_component_updates_source_branch_probe_expression() {
    let edited = rename_component(
        "project:
  name: source_rename_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.voltage_source
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
      pins: { P: rail, N: gnd }
  nets:
    rail: { kind: power }
    gnd: { kind: ground }
scenarios:
  - name: tran
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated: { ground_net: gnd, components: [V1] }
      model_files: []
      node_bindings:
        - { net: rail, node: rail }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: V1, pin: P }, node: rail }
        - { endpoint: { component: V1, pin: N }, node: '0' }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      probes:
        - { name: source_current, expression: 'I(V1)', quantity: current }
      assertions: []
",
        "V1",
        "VBUS",
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let analog = project.scenarios[0].analog.as_ref().unwrap();

    assert!(
        analog
            .probes
            .iter()
            .any(|probe| { probe.name == "source_current" && probe.expression == "I(VBUS)" })
    );
    assert!(analog.pin_bindings.iter().all(|binding| {
        binding.endpoint.component == "VBUS" || binding.endpoint.component != "V1"
    }));
}

#[test]
fn rename_component_does_not_rewrite_branch_prefix_collisions() {
    let edited = rename_component(
        "project:
  name: branch_prefix_collision_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      spice: { primitive: resistor, value_ohm: 1000 }
      pins: { A: rail, B: gnd }
    R10:
      model: generic.analog.resistor
      spice: { primitive: resistor, value_ohm: 1000 }
      pins: { A: rail, B: gnd }
  nets:
    rail: { kind: power }
    gnd: { kind: ground }
scenarios:
  - name: tran
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated: { ground_net: gnd, components: [R1, R10] }
      model_files: []
      node_bindings:
        - { net: rail, node: rail }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: R1, pin: A }, node: rail }
        - { endpoint: { component: R1, pin: B }, node: '0' }
        - { endpoint: { component: R10, pin: A }, node: rail }
        - { endpoint: { component: R10, pin: B }, node: '0' }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      probes:
        - { name: r1_current, expression: 'I(VCCI_R1)', quantity: current }
        - { name: r10_current, expression: 'I(VCCI_R10)', quantity: current }
      assertions: []
",
        "R1",
        "R_SENSE",
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let analog = project.scenarios[0].analog.as_ref().unwrap();

    assert!(
        analog
            .probes
            .iter()
            .any(|probe| { probe.name == "r1_current" && probe.expression == "I(VCCI_R_SENSE)" })
    );
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| { probe.name == "r10_current" && probe.expression == "I(VCCI_R10)" })
    );
}

#[test]
fn rename_ground_net_updates_generated_ground_binding() {
    let edited = rename_net(probe_badge_project_yaml(), "gnd", "pgnd").unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let analog = project.scenarios[0].analog.as_ref().unwrap();

    assert_eq!(analog.generated.as_ref().unwrap().ground_net, "pgnd");
    assert!(
        analog
            .node_bindings
            .iter()
            .any(|binding| { binding.net == "pgnd" && binding.node == "0" })
    );
    assert!(
        analog
            .node_bindings
            .iter()
            .all(|binding| binding.net != "gnd")
    );
}
