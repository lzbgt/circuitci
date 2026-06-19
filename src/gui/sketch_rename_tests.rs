use super::sketch::{SketchPinSide, load_project_snapshot_from_yaml};
use super::sketch_rename::{rename_component, rename_net};

fn rename_project_yaml() -> &'static str {
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

#[test]
fn rename_component_updates_board_schematic_and_generated_analog_refs() {
    let edited = rename_component(rename_project_yaml(), "R1", "R_SENSE").unwrap();
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
    let edited = rename_net(rename_project_yaml(), "out", "sense_out").unwrap();
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
    let edited = rename_net(rename_project_yaml(), "gnd", "pgnd").unwrap();
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
