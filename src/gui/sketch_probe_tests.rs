use super::sketch::{
    SketchPosition, layout_sketch_graph, load_project_snapshot_from_yaml, sketch_graph_bounds,
};
use super::sketch_probes::{
    SketchProbe, SketchProbeAttachmentKind, SketchProbeQuantity, SketchProbeTarget,
    edit_schematic_probe_element_position, edit_schematic_probe_element_positions,
    hit_test_probe_badge, probe_badge_interaction_rect,
};
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
fn snapshot_derives_probe_badges_from_analog_probes() {
    let snapshot = load_project_snapshot_from_yaml(
        "project:
  name: probe_badge_test
  version: 0.1.0
board:
  schematic:
    probe_elements:
      tran_rail_voltage:
        scenario: tran
        probe: rail_voltage
        target: { kind: net, id: rail, attach: wire, source: R1.A }
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
            && probe.element_id == Some("tran_rail_voltage".to_string())
            && probe.attachment == SketchProbeAttachmentKind::Wire
            && probe.source.as_deref() == Some("R1.A")
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
        element_id: None,
        attachment: SketchProbeAttachmentKind::Node,
        source: None,
        position: None,
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
fn layout_places_wire_attached_probe_on_wire_midpoint() {
    let snapshot = load_project_snapshot_from_yaml(
        "project:
  name: wire_probe_layout
  version: 0.1.0
board:
  schematic:
    node_positions:
      component:R1: { x: 80, y: 120 }
      net:rail: { x: 360, y: 132 }
    probe_elements:
      tran_rail_voltage:
        scenario: tran
        probe: rail_voltage
        target: { kind: net, id: rail, attach: wire, source: R1.A }
  components:
    R1:
      model: generic.analog.resistor
      spice: { primitive: resistor, value_ohm: 1000 }
      pins: { A: rail, B: rail }
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
      generated: { ground_net: gnd, components: [R1] }
      model_files: []
      node_bindings:
        - { net: rail, node: rail }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: R1, pin: A }, node: rail }
        - { endpoint: { component: R1, pin: B }, node: rail }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      probes:
        - { name: rail_voltage, expression: 'V(rail)', quantity: voltage }
      assertions: []
",
    )
    .unwrap();
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(620.0, 360.0)),
        &snapshot,
    );
    let edge = graph
        .edges
        .iter()
        .find(|edge| edge.source == "R1.A" && edge.net_id == "rail")
        .unwrap();
    let badge = graph
        .probe_badges
        .iter()
        .find(|badge| badge.probe.element_id.as_deref() == Some("tran_rail_voltage"))
        .unwrap();

    assert!(badge.anchor.distance(edge.start.lerp(edge.end, 0.5)) < 0.01);
    assert!(badge.rect.center().y < badge.anchor.y);
}

#[test]
fn layout_places_pin_attached_probe_on_pin_anchor() {
    let mut snapshot = load_project_snapshot_from_yaml(editable_project_yaml()).unwrap();
    snapshot.probes.push(SketchProbe {
        element_id: Some("tran_net_a_voltage".to_string()),
        attachment: SketchProbeAttachmentKind::Pin,
        source: Some("R1.A".to_string()),
        position: None,
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
    let anchor = graph
        .pin_anchors
        .iter()
        .find(|anchor| anchor.component_id == "R1" && anchor.pin == "A")
        .unwrap();
    let badge = graph.probe_badges.first().unwrap();

    assert!(badge.anchor.distance(anchor.pos) < 0.01);
    assert!(hit_test_probe_badge(&graph.probe_badges, badge.rect.center()).is_some());
}

#[test]
fn placed_probe_readout_area_is_hit_testable_and_in_graph_bounds() {
    let mut snapshot = load_project_snapshot_from_yaml(editable_project_yaml()).unwrap();
    snapshot.probes.push(SketchProbe {
        element_id: Some("tran_net_a_voltage".to_string()),
        attachment: SketchProbeAttachmentKind::Pin,
        source: Some("R1.A".to_string()),
        position: None,
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
    let badge = graph.probe_badges.first().unwrap();
    let interaction = probe_badge_interaction_rect(badge);
    let bounds = sketch_graph_bounds(&graph).unwrap();

    assert!(interaction.right() > badge.rect.right() + 80.0);
    assert!(bounds.contains(interaction.right_center()));
    assert_eq!(
        hit_test_probe_badge(&graph.probe_badges, interaction.right_center())
            .unwrap()
            .probe
            .probe_name,
        "net_a_voltage"
    );
}

#[test]
fn persisted_probe_element_position_overrides_attachment_layout() {
    let yaml = "project:
  name: probe_position
  version: 0.1.0
board:
  schematic:
    probe_elements:
      tran_net_a_voltage:
        scenario: tran
        probe: net_a_voltage
        target: { kind: net, id: net_a, attach: pin, source: R1.A }
  components:
    R1:
      model: generic.analog.resistor
      pins: { A: net_a, B: gnd }
  nets:
    net_a: { kind: digital_or_analog }
    gnd: { kind: ground }
scenarios:
  - name: tran
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated: { ground_net: gnd, components: [R1] }
      model_files: []
      node_bindings:
        - { net: net_a, node: net_a }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: R1, pin: A }, node: net_a }
        - { endpoint: { component: R1, pin: B }, node: '0' }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      probes:
        - { name: net_a_voltage, expression: 'V(net_a)', quantity: voltage }
      assertions: []
";
    let edited =
        edit_schematic_probe_element_position(yaml, "tran_net_a_voltage", 144.0, 88.0).unwrap();
    let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
    let canvas = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(620.0, 360.0));
    let graph = layout_sketch_graph(canvas, &snapshot);
    let badge = graph.probe_badges.first().unwrap();

    assert_eq!(
        badge.probe.position,
        Some(SketchPosition { x: 144.0, y: 88.0 })
    );
    assert!((badge.rect.left() - 154.0).abs() < 0.01);
    assert!((badge.rect.top() - 108.0).abs() < 0.01);
}

#[test]
fn edit_schematic_probe_element_positions_batches_display_coordinates() {
    let yaml = "project:
  name: probe_position_batch
  version: 0.1.0
board:
  schematic:
    probe_elements:
      tran_a:
        scenario: tran
        probe: a
        target: { kind: net, id: a, attach: node }
      tran_b:
        scenario: tran
        probe: b
        target: { kind: net, id: b, attach: node }
  components: {}
  nets:
    a: { kind: digital_or_analog }
    b: { kind: digital_or_analog }
scenarios: []
";
    let edited = edit_schematic_probe_element_positions(
        yaml,
        &[
            ("tran_a".to_string(), 32.0, 48.0),
            ("tran_b".to_string(), 96.0, 112.0),
        ],
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();

    assert_eq!(
        project
            .board
            .schematic
            .probe_elements
            .get("tran_a")
            .unwrap()
            .x,
        Some(32.0)
    );
    assert_eq!(
        project
            .board
            .schematic
            .probe_elements
            .get("tran_b")
            .unwrap()
            .y,
        Some(112.0)
    );
}
