use super::project::{PendingProjectAction, gui_project_examples};
use super::sketch::{
    DEFAULT_SKETCH_GRID_STEP, ProjectSnapshot, SketchComponent, SketchNet, SketchNodeStyle,
    SketchPin, SketchPinSide, SketchSelection, edit_component_model, edit_component_part_number,
    edit_net_kind, edit_net_nominal_voltage, edit_net_powered, layout_sketch_graph,
    validate_board_ir_yaml_text, wire_route_key,
};
use super::sketch_canvas_render::component_context_pin;
use super::{CircuitCiApp, ScopeProbeTarget, SketchSnapMode, SketchViewportCommand, Stage, egui};
use std::path::Path;

fn editable_project_yaml() -> &'static str {
    "project:
  name: gui_editor_test
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

fn analog_scope_project_yaml() -> &'static str {
    "project:
  name: gui_scope_toolbar_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      spice:
        primitive: dc_voltage_source
        dc_v: 5.0
      pins:
        P: rail
        N: gnd
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
  - name: gui_transient
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1]
      model_files: []
      node_bindings:
        - { net: rail, node: rail }
        - { net: out, node: out }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: V1, pin: P }, node: rail }
        - { endpoint: { component: V1, pin: N }, node: '0' }
        - { endpoint: { component: R1, pin: A }, node: rail }
        - { endpoint: { component: R1, pin: B }, node: out }
      analysis: { type: tran, stop_time_us: 100, max_step_us: 1 }
      stimuli: []
      probes: []
      assertions: []
"
}

fn gui_project_example_by_id(id: &str) -> super::project::GuiProjectExample {
    gui_project_examples()
        .iter()
        .copied()
        .find(|example| example.id == id)
        .unwrap()
}

#[test]
fn board_ir_editor_accepts_minimal_project_yaml() {
    validate_board_ir_yaml_text(
        "project:
  name: gui_editor_test
  version: 0.1.0
board:
  components: {}
  nets: {}
",
    )
    .unwrap();
}

#[test]
fn board_ir_editor_rejects_invalid_project_yaml() {
    let error = validate_board_ir_yaml_text(
        "project:
  name: gui_editor_test
",
    )
    .unwrap_err();
    assert!(error.to_string().contains("Board IR"));
}

#[test]
fn classical_auto_layout_action_persists_positions_and_fit_command() {
    let yaml = editable_project_yaml();
    let mut app = CircuitCiApp {
        project_yaml: yaml.to_string(),
        project_snapshot: Some(super::sketch::load_project_snapshot_from_yaml(yaml).unwrap()),
        ..Default::default()
    };

    app.apply_classical_sketch_auto_layout();

    assert!(app.project_yaml_dirty);
    assert_eq!(
        app.sketch_viewport_command,
        Some(SketchViewportCommand::FitAll)
    );
    assert!(app.status.contains("Classical auto layout positioned"));
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let positions = &project.board.schematic.node_positions;
    assert!(positions.contains_key("component:R1"));
    assert!(positions.contains_key("net:net_a"));
    assert!(positions.contains_key("net:gnd"));
    assert!(
        project
            .board
            .schematic
            .wire_routes
            .contains_key("R1.A->net_a")
    );
    assert!(
        project
            .board
            .schematic
            .wire_routes
            .contains_key("R1.B->gnd")
    );
    let style = project
        .board
        .schematic
        .node_styles
        .get("component:R1")
        .unwrap();
    assert_eq!(style.rotation_deg, Some(90));
}

#[test]
fn scope_voltage_action_creates_probe_and_opens_scopes() {
    let yaml = analog_scope_project_yaml();
    let mut app = CircuitCiApp {
        project_yaml: yaml.to_string(),
        project_snapshot: Some(super::sketch::load_project_snapshot_from_yaml(yaml).unwrap()),
        ..Default::default()
    };

    app.open_or_create_scope_voltage_probe_for_net_with_attachment(
        "out",
        super::sketch_probes::SketchProbeAttachmentKind::Wire,
        Some("R1.B".to_string()),
    );

    assert_eq!(app.stage, Stage::Simulation);
    assert_eq!(
        app.pending_scope_probe,
        Some(ScopeProbeTarget {
            scenario_name: "gui_transient".to_string(),
            probe_name: "out_voltage".to_string(),
        })
    );
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let probes = &project.scenarios[0].analog.as_ref().unwrap().probes;
    assert!(probes.iter().any(|probe| {
        probe.name == "out_voltage"
            && probe.expression == "V(out)"
            && probe.quantity == crate::board_ir::AnalogQuantity::Voltage
    }));
    let element = project
        .board
        .schematic
        .probe_elements
        .get("gui_transient_out_voltage")
        .unwrap();
    assert_eq!(element.scenario, "gui_transient");
    assert_eq!(element.probe, "out_voltage");
    assert!(matches!(
        element.target.kind,
        crate::board_ir::SchematicProbeElementTargetKind::Net
    ));
    assert_eq!(element.target.id, "out");
    assert!(matches!(
        element.target.attach,
        Some(crate::board_ir::SchematicProbeAttachmentKind::Wire)
    ));
    assert_eq!(element.target.source.as_deref(), Some("R1.B"));
    assert!(element.x.is_some_and(f64::is_finite));
    assert!(element.y.is_some_and(f64::is_finite));
}

#[test]
fn scope_component_actions_create_current_and_power_probes() {
    let yaml = analog_scope_project_yaml();
    let mut app = CircuitCiApp {
        project_yaml: yaml.to_string(),
        project_snapshot: Some(super::sketch::load_project_snapshot_from_yaml(yaml).unwrap()),
        ..Default::default()
    };

    app.open_or_create_scope_component_probe(
        "V1",
        super::sketch_probes::SketchProbeQuantity::Current,
    );
    assert_eq!(app.stage, Stage::Simulation);
    assert_eq!(
        app.pending_scope_probe,
        Some(ScopeProbeTarget {
            scenario_name: "gui_transient".to_string(),
            probe_name: "V1_current".to_string(),
        })
    );
    app.open_or_create_scope_component_probe_with_attachment(
        "V1",
        super::sketch_probes::SketchProbeQuantity::Power,
        super::sketch_probes::SketchProbeAttachmentKind::Pin,
        Some("V1.P".to_string()),
    );
    assert_eq!(
        app.pending_scope_probe,
        Some(ScopeProbeTarget {
            scenario_name: "gui_transient".to_string(),
            probe_name: "V1_power".to_string(),
        })
    );

    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let probes = &project.scenarios[0].analog.as_ref().unwrap().probes;
    assert!(probes.iter().any(|probe| {
        probe.name == "V1_current"
            && probe.expression == "I(V1)"
            && probe.quantity == crate::board_ir::AnalogQuantity::Current
    }));
    assert!(probes.iter().any(|probe| {
        probe.name == "V1_power"
            && probe.expression == "V(rail,0)*I(V1)"
            && probe.quantity == crate::board_ir::AnalogQuantity::Power
    }));
    let current = project
        .board
        .schematic
        .probe_elements
        .get("gui_transient_V1_current")
        .unwrap();
    assert_eq!(current.probe, "V1_current");
    assert!(current.x.is_some_and(f64::is_finite));
    assert!(current.y.is_some_and(f64::is_finite));
    assert!(matches!(
        current.target.kind,
        crate::board_ir::SchematicProbeElementTargetKind::Component
    ));
    assert_eq!(current.target.id, "V1");
    let power = project
        .board
        .schematic
        .probe_elements
        .get("gui_transient_V1_power")
        .unwrap();
    assert_eq!(power.probe, "V1_power");
    assert!(power.x.is_some_and(f64::is_finite));
    assert!(power.y.is_some_and(f64::is_finite));
    assert!(matches!(
        power.target.kind,
        crate::board_ir::SchematicProbeElementTargetKind::Component
    ));
    assert_eq!(power.target.id, "V1");
    assert!(matches!(
        power.target.attach,
        Some(crate::board_ir::SchematicProbeAttachmentKind::Pin)
    ));
    assert_eq!(power.target.source.as_deref(), Some("V1.P"));
}

#[test]
fn armed_scope_voltage_tool_creates_probe_from_net_click() {
    let yaml = analog_scope_project_yaml();
    let mut app = CircuitCiApp {
        project_yaml: yaml.to_string(),
        project_snapshot: Some(super::sketch::load_project_snapshot_from_yaml(yaml).unwrap()),
        ..Default::default()
    };

    app.arm_scope_probe_tool(super::sketch_scope_tools::SketchScopeProbeTool::Voltage);
    assert!(app.apply_scope_probe_tool_to_selection(Some(
        super::sketch_scope_tools::SketchScopeProbePlacement::node(SketchSelection::Net(
            "out".to_string(),
        ))
    )));

    assert_eq!(app.stage, Stage::Simulation);
    assert!(!app.scope_probe_tool_armed());
    assert_eq!(
        app.pending_scope_probe,
        Some(ScopeProbeTarget {
            scenario_name: "gui_transient".to_string(),
            probe_name: "out_voltage".to_string(),
        })
    );
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    assert!(
        project.scenarios[0]
            .analog
            .as_ref()
            .unwrap()
            .probes
            .iter()
            .any(|probe| probe.name == "out_voltage")
    );
}

#[test]
fn armed_scope_current_tool_rejects_net_without_mutation() {
    let yaml = analog_scope_project_yaml();
    let mut app = CircuitCiApp {
        project_yaml: yaml.to_string(),
        project_snapshot: Some(super::sketch::load_project_snapshot_from_yaml(yaml).unwrap()),
        ..Default::default()
    };

    app.arm_scope_probe_tool(super::sketch_scope_tools::SketchScopeProbeTool::Current);
    assert!(app.apply_scope_probe_tool_to_selection(Some(
        super::sketch_scope_tools::SketchScopeProbePlacement::node(SketchSelection::Net(
            "out".to_string(),
        ))
    )));

    assert_ne!(app.stage, Stage::Simulation);
    assert!(app.scope_probe_tool_armed());
    assert_eq!(app.project_yaml, yaml);
    assert!(app.status.contains("needs a component"));
}

#[test]
fn ne555_scope_example_import_preset_sets_scope_ready_spice_fields() {
    let mut app = CircuitCiApp::default();

    app.use_ne555_scope_example_import_preset();

    assert_eq!(
        app.import_spice_deck_path,
        "examples/ne555_astable_scope_smoke/deck.cir"
    );
    assert_eq!(
        app.import_spice_output_path,
        "out/gui_import/ne555_astable_scope.project.yaml"
    );
    assert_eq!(app.import_spice_project_name, "ne555_astable_scope");
    assert_eq!(app.import_spice_backend, "auto");
    assert_eq!(app.import_spice_stop_time_us, 5_000.0);
    assert_eq!(app.import_spice_max_step_us, 2.0);
    assert!(app.status.contains("NE555 astable scope example"));
}

#[test]
fn ne555_scope_example_shortcut_loads_direct_project() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("ne555_astable_scope"), None);

    assert_eq!(
        app.project_path,
        "examples/ne555_astable_scope_smoke/project.yaml"
    );
    assert!(app.project_yaml.contains("name: ne555_astable_scope"));
    assert!(!app.project_yaml_dirty);
    let snapshot = app.project_snapshot.as_ref().unwrap();
    assert_eq!(snapshot.name, "ne555_astable_scope");
    assert_eq!(snapshot.components, 5);
    assert_eq!(snapshot.nets, 4);
    assert_eq!(snapshot.scenarios, 1);
    assert_eq!(app.stage, Stage::Sketch);
    assert_eq!(
        app.sketch_viewport_command,
        Some(SketchViewportCommand::FitAll)
    );
    assert!(app.status.contains("fitting routed schematic"));
}

#[test]
fn scope_examples_load_routed_schematic_edges() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("ne555_astable_scope"), None);
    let snapshot = app.project_snapshot.as_ref().unwrap();

    assert_eq!(snapshot.wire_routes.len(), 10);
    assert_eq!(
        snapshot
            .wire_routes
            .get(&wire_route_key("VCC.P", "vcc"))
            .unwrap()[0],
        super::sketch::SketchPosition { x: 80.0, y: 48.0 }
    );
    assert_eq!(
        snapshot
            .wire_routes
            .get(&wire_route_key("RLOAD.B", "gnd"))
            .unwrap()
            .last()
            .copied(),
        Some(super::sketch::SketchPosition { x: 192.0, y: 512.0 })
    );
    assert!(
        snapshot
            .wire_routes
            .contains_key(&wire_route_key("RTIM.B", "timing"))
    );
    let vout = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == "VOUT")
        .unwrap();
    assert_eq!(
        vout.kicad_symbol_id.as_deref(),
        Some("Simulation_SPICE:VPULSE")
    );
    let ctim = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == "CTIM")
        .unwrap();
    assert_eq!(ctim.kicad_symbol_id.as_deref(), Some("Device:C"));
    assert_eq!(
        ctim.style,
        SketchNodeStyle {
            rotation_deg: 90,
            mirrored: false,
            pin_side: SketchPinSide::Auto,
        }
    );

    app.request_project_example_load(gui_project_example_by_id("rc_lowpass_scope"), None);
    let snapshot = app.project_snapshot.as_ref().unwrap();

    assert_eq!(snapshot.wire_routes.len(), 6);
    assert_eq!(app.stage, Stage::Sketch);
    assert_eq!(
        app.sketch_viewport_command,
        Some(SketchViewportCommand::FitAll)
    );
    assert_eq!(
        snapshot
            .wire_routes
            .get(&wire_route_key("VSIN.N", "gnd"))
            .unwrap()
            .last()
            .copied(),
        Some(super::sketch::SketchPosition { x: 80.0, y: 512.0 })
    );
    assert!(
        snapshot
            .wire_routes
            .contains_key(&wire_route_key("RIN.B", "filtered"))
    );
    assert!(
        snapshot
            .wire_routes
            .contains_key(&wire_route_key("COUT.B", "gnd"))
    );
    let source = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == "VSIN")
        .unwrap();
    assert_eq!(
        source.kicad_symbol_id.as_deref(),
        Some("Simulation_SPICE:VSIN")
    );
    let capacitor = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == "COUT")
        .unwrap();
    assert_eq!(capacitor.kicad_symbol_id.as_deref(), Some("Device:C"));
    assert_eq!(
        capacitor.style,
        SketchNodeStyle {
            rotation_deg: 90,
            mirrored: false,
            pin_side: SketchPinSide::Auto,
        }
    );

    app.request_project_example_load(
        gui_project_example_by_id("comparator_threshold_scope"),
        None,
    );
    let snapshot = app.project_snapshot.as_ref().unwrap();

    assert_eq!(snapshot.wire_routes.len(), 13);
    assert_eq!(app.stage, Stage::Sketch);
    assert_eq!(
        app.sketch_viewport_command,
        Some(SketchViewportCommand::FitAll)
    );
    assert!(
        snapshot
            .wire_routes
            .contains_key(&wire_route_key("XU1.OUT", "output"))
    );
    assert!(
        snapshot
            .wire_routes
            .contains_key(&wire_route_key("VREF.P", "reference"))
    );
    let comparator = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == "XU1")
        .unwrap();
    assert_eq!(
        comparator.kicad_symbol_id.as_deref(),
        Some("Comparator:LMV331")
    );
    let load = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == "RLOAD")
        .unwrap();
    assert_eq!(load.kicad_symbol_id.as_deref(), Some("Device:R"));

    app.request_project_example_load(gui_project_example_by_id("opamp_buffer_scope"), None);
    let snapshot = app.project_snapshot.as_ref().unwrap();

    assert_eq!(snapshot.wire_routes.len(), 11);
    assert_eq!(app.stage, Stage::Sketch);
    assert_eq!(
        app.sketch_viewport_command,
        Some(SketchViewportCommand::FitAll)
    );
    assert!(
        snapshot
            .wire_routes
            .contains_key(&wire_route_key("XU1.OUT", "output"))
    );
    assert!(
        snapshot
            .wire_routes
            .contains_key(&wire_route_key("XU1.INN", "output"))
    );
    let opamp = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == "XU1")
        .unwrap();
    assert_eq!(
        opamp.kicad_symbol_id.as_deref(),
        Some("Amplifier_Operational:LMV321")
    );
    let load = snapshot
        .components_detail
        .iter()
        .find(|component| component.id == "RLOAD")
        .unwrap();
    assert_eq!(load.kicad_symbol_id.as_deref(), Some("Device:R"));
}

#[test]
fn gui_project_example_registry_lists_ne555_scope_fixture() {
    let examples = gui_project_examples();

    assert_eq!(examples.len(), 14);
    let example = gui_project_example_by_id("ne555_astable_scope");
    assert_eq!(example.id, "ne555_astable_scope");
    assert_eq!(example.category, "Timer");
    assert_eq!(example.open_label, "Open NE555 Scope Example");
    assert_eq!(example.run_label, "Open NE555 + Run Scopes");
    assert_eq!(
        example.summary,
        "Astable-style timer output with timing-node and source-current traces."
    );
    assert_eq!(
        example.project_path,
        "examples/ne555_astable_scope_smoke/project.yaml"
    );
    assert_eq!(example.project_name, "ne555_astable_scope");
    assert_eq!(
        example.expected_traces,
        &["v(out)", "v(timing)", "v(vcc)", "i(VCC)", "i(VOUT)"]
    );
}

#[test]
fn gui_project_example_registry_lists_rc_lowpass_scope_fixture() {
    let example = gui_project_example_by_id("rc_lowpass_scope");

    assert_eq!(example.category, "Filter");
    assert_eq!(example.open_label, "Open RC Low-Pass Scope Example");
    assert_eq!(example.run_label, "Open RC Low-Pass + Run Scopes");
    assert_eq!(
        example.summary,
        "1 kHz sine into a first-order low-pass for input/output comparison."
    );
    assert_eq!(
        example.project_path,
        "examples/rc_lowpass_scope/project.yaml"
    );
    assert_eq!(example.project_name, "rc_lowpass_scope");
    assert_eq!(
        example.expected_traces,
        &["v(input)", "v(filtered)", "i(VSIN)"]
    );
    assert_eq!(
        example.expected_frequency,
        "1.00 kHz sine, fc about 1.59 kHz"
    );
}

#[test]
fn gui_project_example_registry_lists_rc_monte_carlo_bode_fixture() {
    let example = gui_project_example_by_id("rc_monte_carlo_bode");

    assert_eq!(example.category, "Yield");
    assert_eq!(example.open_label, "Open RC Monte Carlo Example");
    assert_eq!(example.run_label, "Open RC Monte Carlo + Run Observations");
    assert_eq!(
        example.summary,
        "Generated RC low-pass Bode run with sampled R/C tolerances and yield checks."
    );
    assert_eq!(
        example.project_path,
        "examples/good_generated_rc_lowpass_monte_carlo_observation/project.yaml"
    );
    assert_eq!(
        example.project_name,
        "good_generated_rc_lowpass_monte_carlo_observation"
    );
    assert_eq!(
        example.expected_traces,
        &[
            "input_gain_db",
            "filtered_gain_db",
            "filtered_phase_deg",
            "filtered_mag"
        ]
    );
    assert_eq!(
        example.expected_frequency,
        "5 sampled R/C tolerance Bode runs with yield and P5 margin criteria"
    );
    assert_eq!(example.observation_preset_component, None);
}

#[test]
fn gui_project_example_registry_lists_comparator_threshold_scope_fixture() {
    let example = gui_project_example_by_id("comparator_threshold_scope");

    assert_eq!(example.category, "Comparator");
    assert_eq!(example.open_label, "Open Comparator Threshold Example");
    assert_eq!(example.run_label, "Open Comparator + Run Scopes");
    assert_eq!(
        example.summary,
        "Pulse input against a DC reference for output-state threshold checks."
    );
    assert_eq!(
        example.project_path,
        "examples/comparator_threshold_scope/project.yaml"
    );
    assert_eq!(example.project_name, "comparator_threshold_scope");
    assert_eq!(
        example.expected_traces,
        &["v(input)", "v(reference)", "v(output)", "v(vcc)"]
    );
    assert_eq!(
        example.expected_frequency,
        "80 us input pulse crossing a 1.2 V reference"
    );
    assert_eq!(example.observation_preset_component, Some("XU1"));
}

#[test]
fn gui_project_example_registry_lists_opamp_buffer_scope_fixture() {
    let example = gui_project_example_by_id("opamp_buffer_scope");

    assert_eq!(example.category, "Op-Amp");
    assert_eq!(example.open_label, "Open Op-Amp Buffer Example");
    assert_eq!(example.run_label, "Open Op-Amp Buffer + Run Scopes");
    assert_eq!(
        example.summary,
        "Unity-gain buffer tracking a pulse input with output settling checks."
    );
    assert_eq!(
        example.project_path,
        "examples/good_ideal_opamp_buffer/project.yaml"
    );
    assert_eq!(example.project_name, "good_ideal_opamp_buffer");
    assert_eq!(
        example.expected_traces,
        &["v(input)", "v(output)", "v(vcc)"]
    );
    assert_eq!(
        example.expected_frequency,
        "80 us input pulse through unity feedback"
    );
    assert_eq!(example.observation_preset_component, Some("XU1"));
}

#[test]
fn gui_project_example_registry_lists_ap2112k_ldo_scope_fixture() {
    let example = gui_project_example_by_id("ap2112k_ldo_scope");

    assert_eq!(example.category, "Regulator");
    assert_eq!(example.open_label, "Open AP2112K LDO Example");
    assert_eq!(example.run_label, "Open AP2112K + Run Scopes");
    assert_eq!(
        example.summary,
        "Enabled 3.3 V LDO rail with load-current and output-window checks."
    );
    assert_eq!(
        example.project_path,
        "examples/good_ap2112k_3v3_ldo_observation/project.yaml"
    );
    assert_eq!(example.project_name, "good_ap2112k_3v3_ldo_observation");
    assert_eq!(
        example.expected_traces,
        &["v_usb", "v_en", "v_rail3v3", "i_load"]
    );
    assert_eq!(
        example.expected_frequency,
        "5 V enabled input, 3.3 V regulated load rail"
    );
    assert_eq!(example.observation_preset_component, Some("UREG"));
}

#[test]
fn gui_project_example_registry_lists_tps22918_load_switch_scope_fixture() {
    let example = gui_project_example_by_id("tps22918_load_switch_scope");

    assert_eq!(example.category, "Load Switch");
    assert_eq!(example.open_label, "Open TPS22918 Load Switch Example");
    assert_eq!(example.run_label, "Open TPS22918 + Run Scopes");
    assert_eq!(
        example.summary,
        "Enabled 5 V load switch path with switched-rail and load-current checks."
    );
    assert_eq!(
        example.project_path,
        "examples/good_tps22918_load_switch_observation/project.yaml"
    );
    assert_eq!(
        example.project_name,
        "good_tps22918_load_switch_observation"
    );
    assert_eq!(
        example.expected_traces,
        &["v_usb", "v_on", "v_switched5v", "i_load"]
    );
    assert_eq!(
        example.expected_frequency,
        "5 V enabled load switch into a 1 kOhm load"
    );
    assert_eq!(example.observation_preset_component, Some("USW"));
}

#[test]
fn gui_project_example_registry_lists_mcp73831_charger_scope_fixture() {
    let example = gui_project_example_by_id("mcp73831_charger_scope");

    assert_eq!(example.category, "Charger");
    assert_eq!(example.open_label, "Open MCP73831 Charger Example");
    assert_eq!(example.run_label, "Open MCP73831 + Run Scopes");
    assert_eq!(
        example.summary,
        "USB-powered Li-Ion charger with PROG-current and VBAT checks."
    );
    assert_eq!(
        example.project_path,
        "examples/good_mcp73831_charger_observation/project.yaml"
    );
    assert_eq!(example.project_name, "good_mcp73831_charger_observation");
    assert_eq!(example.expected_traces, &["v_usb", "v_bat", "i_charge"]);
    assert_eq!(
        example.expected_frequency,
        "5 V USB input, 10 kOhm PROG resistor, and 100 mA charge observation"
    );
    assert_eq!(example.observation_preset_component, Some("UCHG"));
}

#[test]
fn gui_project_example_registry_lists_bq24075_power_path_scope_fixture() {
    let example = gui_project_example_by_id("bq24075_power_path_scope");

    assert_eq!(example.category, "Power Path");
    assert_eq!(example.open_label, "Open BQ24075 Power Path Example");
    assert_eq!(example.run_label, "Open BQ24075 + Run Scopes");
    assert_eq!(
        example.summary,
        "Adapter-powered charger with OUT rail and BAT charge-current checks."
    );
    assert_eq!(
        example.project_path,
        "examples/good_bq24075_power_path_observation/project.yaml"
    );
    assert_eq!(example.project_name, "good_bq24075_power_path_observation");
    assert_eq!(
        example.expected_traces,
        &["v_adapter", "v_sysout", "v_bat", "i_charge", "i_sys_load"]
    );
    assert_eq!(
        example.expected_frequency,
        "6 V adapter input, 5.5 V OUT path, and 450 mA ISET charge observation"
    );
    assert_eq!(example.observation_preset_component, Some("UCHG"));
}

#[test]
fn gui_project_example_registry_lists_bq25798_nvdc_scope_fixture() {
    let example = gui_project_example_by_id("bq25798_nvdc_scope");

    assert_eq!(example.category, "Power Path");
    assert_eq!(example.open_label, "Open BQ25798 NVDC Example");
    assert_eq!(example.run_label, "Open BQ25798 + Run Scopes");
    assert_eq!(
        example.summary,
        "20 V adapter buck-boost/NVDC charger observation with SYS and BAT checks."
    );
    assert_eq!(
        example.project_path,
        "examples/good_bq25798_nvdc_observation/project.yaml"
    );
    assert_eq!(example.project_name, "good_bq25798_nvdc_observation");
    assert_eq!(
        example.expected_traces,
        &["v_adapter", "v_sysout", "v_bat", "i_charge", "i_sys_load"]
    );
    assert_eq!(
        example.expected_frequency,
        "20 V adapter input, 12 V SYS rail, and 2 A programmed charge observation"
    );
    assert_eq!(example.observation_preset_component, None);
}

#[test]
fn gui_project_example_registry_lists_tlv803_reset_scope_fixture() {
    let example = gui_project_example_by_id("tlv803_reset_scope");

    assert_eq!(example.category, "Reset");
    assert_eq!(example.open_label, "Open TLV803 Reset Example");
    assert_eq!(example.run_label, "Open TLV803 + Run Scopes");
    assert_eq!(
        example.summary,
        "Reset-supervisor threshold release from a pulsed 3.3 V rail."
    );
    assert_eq!(
        example.project_path,
        "examples/good_tlv803ea29_reset_observation/project.yaml"
    );
    assert_eq!(example.project_name, "good_tlv803ea29_reset_observation");
    assert_eq!(example.expected_traces, &["v_rail", "reset_n"]);
    assert_eq!(
        example.expected_frequency,
        "3.3 V rail ramp with reset release"
    );
    assert_eq!(example.observation_preset_component, Some("URESET"));
}

#[test]
fn gui_project_example_registry_lists_loop_stability_bode_fixture() {
    let example = gui_project_example_by_id("loop_stability_bode_scope");

    assert_eq!(example.category, "Stability");
    assert_eq!(example.open_label, "Open Loop Stability Bode Example");
    assert_eq!(example.run_label, "Open Loop Stability + Run Scopes");
    assert_eq!(
        example.summary,
        "Open-loop Bode response with executable phase and gain margin checks."
    );
    assert_eq!(
        example.project_path,
        "examples/loop_stability_bode_scope/project.yaml"
    );
    assert_eq!(example.project_name, "loop_stability_bode_scope");
    assert_eq!(
        example.expected_traces,
        &["loop_mag_db", "loop_phase_deg", "loop_mag"]
    );
    assert_eq!(
        example.expected_frequency,
        "Bode loop gain with phase margin >45 deg and gain margin >6 dB"
    );
    assert_eq!(example.observation_preset_component, None);
}

#[test]
fn gui_project_example_registry_lists_dc_bias_fixture() {
    let example = gui_project_example_by_id("dc_bias_observation");

    assert_eq!(example.category, "Bias");
    assert_eq!(example.open_label, "Open DC Bias Example");
    assert_eq!(example.run_label, "Open DC Bias + Run Observations");
    assert_eq!(
        example.summary,
        "Generated operating-point divider bias with resistor-tolerance margin checks."
    );
    assert_eq!(
        example.project_path,
        "examples/good_dc_bias_observation/project.yaml"
    );
    assert_eq!(example.project_name, "good_dc_bias_observation");
    assert_eq!(example.expected_traces, &["vin", "midpoint"]);
    assert_eq!(
        example.expected_frequency,
        "DC operating point with 9 divider-tolerance corners"
    );
    assert_eq!(example.observation_preset_component, None);
}

#[test]
fn gui_project_example_registry_lists_noise_observation_fixture() {
    let example = gui_project_example_by_id("noise_observation");

    assert_eq!(example.category, "Noise");
    assert_eq!(example.open_label, "Open Noise Observation Example");
    assert_eq!(example.run_label, "Open Noise + Run Observations");
    assert_eq!(
        example.summary,
        "Generated divider noise density and integrated RMS noise checks."
    );
    assert_eq!(
        example.project_path,
        "examples/good_noise_observation/project.yaml"
    );
    assert_eq!(example.project_name, "good_noise_observation");
    assert_eq!(
        example.expected_traces,
        &[
            "onoise_density",
            "inoise_density",
            "onoise_total",
            "inoise_total"
        ]
    );
    assert_eq!(
        example.expected_frequency,
        "10 Hz to 100 kHz divider output and input-referred RMS noise"
    );
    assert_eq!(example.observation_preset_component, None);
}

#[test]
fn gui_project_example_picker_defaults_and_falls_back_to_valid_entry() {
    let mut app = CircuitCiApp::default();

    assert_eq!(app.selected_project_example().id, "ne555_astable_scope");
    assert_eq!(
        app.selected_project_example().observation_preset_component,
        None
    );
    app.selected_project_example_id = "rc_lowpass_scope".to_string();
    assert_eq!(app.selected_project_example().id, "rc_lowpass_scope");
    assert_eq!(
        app.selected_project_example().observation_preset_component,
        None
    );
    app.selected_project_example_id = "rc_monte_carlo_bode".to_string();
    assert_eq!(app.selected_project_example().id, "rc_monte_carlo_bode");
    app.selected_project_example_id = "comparator_threshold_scope".to_string();
    assert_eq!(
        app.selected_project_example().id,
        "comparator_threshold_scope"
    );
    app.selected_project_example_id = "opamp_buffer_scope".to_string();
    assert_eq!(app.selected_project_example().id, "opamp_buffer_scope");
    app.selected_project_example_id = "ap2112k_ldo_scope".to_string();
    assert_eq!(app.selected_project_example().id, "ap2112k_ldo_scope");
    app.selected_project_example_id = "tps22918_load_switch_scope".to_string();
    assert_eq!(
        app.selected_project_example().id,
        "tps22918_load_switch_scope"
    );
    app.selected_project_example_id = "mcp73831_charger_scope".to_string();
    assert_eq!(app.selected_project_example().id, "mcp73831_charger_scope");
    app.selected_project_example_id = "bq24075_power_path_scope".to_string();
    assert_eq!(
        app.selected_project_example().id,
        "bq24075_power_path_scope"
    );
    app.selected_project_example_id = "tlv803_reset_scope".to_string();
    assert_eq!(app.selected_project_example().id, "tlv803_reset_scope");
    app.selected_project_example_id = "loop_stability_bode_scope".to_string();
    assert_eq!(
        app.selected_project_example().id,
        "loop_stability_bode_scope"
    );
    app.selected_project_example_id = "noise_observation".to_string();
    assert_eq!(app.selected_project_example().id, "noise_observation");
    app.selected_project_example_id = "deleted_example".to_string();
    assert_eq!(app.selected_project_example().id, "ne555_astable_scope");
}

#[test]
fn opamp_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("opamp_buffer_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert!(app.project_yaml_dirty);
    assert_eq!(app.stage, Stage::Sketch);
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("XU1".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "xu1_observation");
    assert!(app.status.contains("Generated observation preset"));
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "xu1_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_xu1_inn"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_xu1_inn_tracks_input_high_below")
    );
}

#[test]
fn comparator_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(
        gui_project_example_by_id("comparator_threshold_scope"),
        None,
    );

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(app.analog_generated_scenario, "xu1_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "xu1_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_xu1_out"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_xu1_out_positive_input_low_state")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_xu1_out_positive_input_high_state")
    );
}

#[test]
fn ap2112k_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("ap2112k_ldo_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UREG".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "ureg_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "ureg_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_ureg_vout")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureg_vout_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureg_vout_max_voltage")
    );
}

#[test]
fn tps22918_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(
        gui_project_example_by_id("tps22918_load_switch_scope"),
        None,
    );

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("USW".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "usw_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "usw_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_usw_vout"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_usw_vout_enabled_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_usw_vout_enabled_max_voltage")
    );
}

#[test]
fn mcp73831_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("mcp73831_charger_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UCHG".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uchg_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uchg_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uchg_vbat")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uchg_vbat_regulation_ceiling")
    );
}

#[test]
fn bq24075_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("bq24075_power_path_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UCHG".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uchg_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uchg_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uchg_out"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uchg_bat_regulation_ceiling")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uchg_out_power_path_ceiling")
    );
}

#[test]
fn tlv803_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tlv803_reset_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("URESET".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "ureset_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "ureset_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_ureset_reset")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureset_reset_asserted_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureset_reset_released_high")
    );
}

#[test]
fn ne555_scope_example_workflow_declines_observation_preset_without_model_target() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("ne555_astable_scope"), None);

    assert!(!app.create_scope_example_observation_preset());
    assert!(!app.project_yaml_dirty);
    assert!(app.status.contains("does not declare"));
}

#[test]
fn ne555_scope_example_workflow_status_is_contextual() {
    let mut app = CircuitCiApp::default();
    assert!(app.scope_example_workflow_status().is_none());

    app.request_project_example_load(gui_project_example_by_id("ne555_astable_scope"), None);
    let status = app.scope_example_workflow_status().unwrap();

    assert_eq!(status.title, "NE555 Scope Workflow");
    assert_eq!(status.state, "Ready");
    assert_eq!(
        status.expected_traces,
        &["v(out)", "v(timing)", "v(vcc)", "i(VCC)", "i(VOUT)"]
    );
    assert_eq!(status.expected_frequency, "about 1.46 kHz");
    assert!(status.action.contains("Run + Scopes"));
}

#[test]
fn ne555_scope_example_shortcut_uses_unsaved_project_guard() {
    let mut app = CircuitCiApp {
        project_yaml_dirty: true,
        ..Default::default()
    };

    app.request_project_example_load(gui_project_example_by_id("ne555_astable_scope"), None);

    assert_eq!(
        app.pending_project_action,
        Some(PendingProjectAction::LoadProjectSummary {
            path: "examples/ne555_astable_scope_smoke/project.yaml".to_string()
        })
    );
    assert!(app.status.contains("Confirm unsaved changes"));
    assert_ne!(
        app.project_path,
        "examples/ne555_astable_scope_smoke/project.yaml"
    );
}

#[test]
fn ne555_scope_example_run_shortcut_loads_and_starts_scopes_validation() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load_and_run_scopes(
        gui_project_example_by_id("ne555_astable_scope"),
        None,
    );

    assert_eq!(
        app.project_path,
        "examples/ne555_astable_scope_smoke/project.yaml"
    );
    assert_eq!(app.stage, Stage::Simulation);
    assert!(app.background_job_elapsed_secs().is_some());
    assert_eq!(app.sketch_viewport_command, None);
    assert!(app.status.contains("Running validation in Scopes"));
    let status = app.scope_example_workflow_status().unwrap();
    assert_eq!(status.state, "Validation running");
    assert!(status.action.contains("waveform loading"));
    assert!(app.project_yaml.contains("name: ne555_astable_scope"));
    let snapshot = app.project_snapshot.as_ref().unwrap();
    assert_eq!(snapshot.name, "ne555_astable_scope");
}

#[test]
fn ne555_scope_example_workflow_run_action_starts_scopes_validation() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("ne555_astable_scope"), None);

    assert!(app.run_scope_example_workflow_scopes());
    assert_eq!(app.stage, Stage::Simulation);
    assert!(app.background_job_elapsed_secs().is_some());
    assert!(app.status.contains("Running validation in Scopes"));
    let status = app.scope_example_workflow_status().unwrap();
    assert_eq!(status.state, "Validation running");
}

#[test]
fn ne555_scope_example_workflow_activity_action_opens_sketch_overlay() {
    let mut app = CircuitCiApp {
        stage: Stage::Simulation,
        sketch_runtime_scope_overlay_visible: false,
        ..Default::default()
    };

    app.request_project_example_load(gui_project_example_by_id("ne555_astable_scope"), None);

    assert!(app.open_scope_example_workflow_activity());
    assert_eq!(app.stage, Stage::Sketch);
    assert!(app.sketch_runtime_scope_overlay_visible);
    assert!(app.sketch_scope_activity_window_open);
    assert!(app.status.contains("Scope Activity"));
}

#[test]
fn ne555_scope_example_run_shortcut_uses_unsaved_project_guard() {
    let mut app = CircuitCiApp {
        project_yaml_dirty: true,
        ..Default::default()
    };

    app.request_project_example_load_and_run_scopes(
        gui_project_example_by_id("ne555_astable_scope"),
        None,
    );

    assert_eq!(
        app.pending_project_action,
        Some(PendingProjectAction::LoadProjectSummaryAndRunScopes {
            path: "examples/ne555_astable_scope_smoke/project.yaml".to_string()
        })
    );
    assert!(app.status.contains("Confirm unsaved changes"));
    assert_ne!(app.stage, Stage::Simulation);
    assert!(app.background_job_elapsed_secs().is_none());
}

#[test]
fn validate_from_gui_emits_phase_progress() {
    let output = tempfile::tempdir().unwrap();
    let mut stages = Vec::new();

    let (report, markdown) = super::validate_from_gui(
        Path::new("examples/good_current_source_load/project.yaml"),
        "default",
        output.path(),
        |stage, _detail| stages.push(stage.to_string()),
        || false,
    )
    .unwrap();

    assert_eq!(report.result, "pass");
    assert!(markdown.contains("# CircuitCI Report"));
    for expected in [
        "Loading project",
        "Loading models",
        "Binding models",
        "Running validation",
        "Preparing analog transient",
        "Checking analog model evidence",
        "Preparing analog deck",
        "Selecting analog backend",
        "Writing analog wrapper deck",
        "Running analog backend",
        "Loading analog waveform",
        "Evaluating analog assertions",
        "Applying profile coverage",
        "Assembling report",
        "Writing report",
        "Loading markdown report",
    ] {
        assert!(stages.iter().any(|stage| stage == expected), "{expected}");
    }
}

#[test]
fn suggest_from_gui_cancellation_stops_before_yaml_output() {
    let error = super::suggest_from_gui_with_cancel(
        Path::new("examples/scenario_suggestions_power_reset/project.yaml"),
        "default",
        || true,
    )
    .unwrap_err();

    assert!(error.to_string().contains("canceled"));
}

#[test]
fn board_ir_component_form_edits_emit_valid_yaml() {
    let edited =
        edit_component_model(editable_project_yaml(), "R1", "vendor.test.resistor").unwrap();
    let edited = edit_component_part_number(&edited, "R1", "RC0603FR-0710KL").unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("vendor.test.resistor"));
    assert!(edited.contains("RC0603FR-0710KL"));
}

#[test]
fn board_ir_net_form_edits_emit_valid_yaml() {
    let edited = edit_net_kind(editable_project_yaml(), "net_a", "power").unwrap();
    let edited = edit_net_nominal_voltage(&edited, "net_a", Some(3.3)).unwrap();
    let edited = edit_net_powered(&edited, "net_a", Some(true)).unwrap();
    validate_board_ir_yaml_text(&edited).unwrap();
    assert!(edited.contains("kind: power"));
    assert!(edited.contains("nominal_voltage: 3.3"));
    assert!(edited.contains("powered: true"));
}

#[test]
fn component_context_pin_prefers_existing_wire_pin_then_first_pin() {
    let snapshot = ProjectSnapshot {
        name: "graph".to_string(),
        components: 1,
        nets: 2,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![SketchComponent {
            id: "U1".to_string(),
            model: "generic.ic".to_string(),
            part_number: None,
            spice: None,
            position: None,
            pins: vec![
                SketchPin {
                    pin: "VCC".to_string(),
                    net: "rail".to_string(),
                },
                SketchPin {
                    pin: "GND".to_string(),
                    net: "gnd".to_string(),
                },
            ],
            style: SketchNodeStyle::default(),
            kicad_symbol_id: None,
            source_paths: Vec::new(),
        }],
        nets_detail: Vec::new(),
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
        component_labels: Default::default(),
    };

    assert_eq!(
        component_context_pin(&snapshot, "U1", " GND "),
        ("GND".to_string(), "gnd".to_string())
    );
    assert_eq!(
        component_context_pin(&snapshot, "U1", "OUT"),
        ("VCC".to_string(), "rail".to_string())
    );
    assert_eq!(
        component_context_pin(&snapshot, "MISSING", "OUT"),
        ("P1".to_string(), String::new())
    );
}

#[test]
fn sketch_graph_layout_connects_component_to_net() {
    let snapshot = ProjectSnapshot {
        name: "graph".to_string(),
        components: 1,
        nets: 1,
        scenarios: 0,
        libraries: Vec::new(),
        components_detail: vec![SketchComponent {
            id: "R1".to_string(),
            model: "generic.analog.resistor".to_string(),
            part_number: None,
            spice: None,
            position: None,
            pins: vec![SketchPin {
                pin: "A".to_string(),
                net: "net_a".to_string(),
            }],
            style: SketchNodeStyle::default(),
            kicad_symbol_id: None,
            source_paths: Vec::new(),
        }],
        nets_detail: vec![SketchNet {
            id: "net_a".to_string(),
            kind: "DigitalOrAnalog".to_string(),
            nominal_voltage: None,
            powered: None,
            connections: vec!["R1.A".to_string()],
            position: None,
        }],
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
        component_labels: Default::default(),
    };
    let graph = layout_sketch_graph(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0)),
        &snapshot,
    );
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
}

#[test]
fn sketch_snap_mode_maps_grid_and_guide_flags() {
    let mut app = CircuitCiApp::default();

    app.set_sketch_snap_mode(SketchSnapMode::Free);
    assert_eq!(app.sketch_snap_mode(), SketchSnapMode::Free);
    assert!(!app.sketch_snap_enabled);
    assert!(!app.sketch_guide_snap_enabled);

    app.set_sketch_snap_mode(SketchSnapMode::Grid);
    assert_eq!(app.sketch_snap_mode(), SketchSnapMode::Grid);
    assert!(app.sketch_snap_enabled);
    assert!(!app.sketch_guide_snap_enabled);

    app.set_sketch_snap_mode(SketchSnapMode::Guides);
    assert_eq!(app.sketch_snap_mode(), SketchSnapMode::Guides);
    assert!(!app.sketch_snap_enabled);
    assert!(app.sketch_guide_snap_enabled);

    app.set_sketch_snap_mode(SketchSnapMode::GridAndGuides);
    assert_eq!(app.sketch_snap_mode(), SketchSnapMode::GridAndGuides);
    assert!(app.sketch_snap_enabled);
    assert!(app.sketch_guide_snap_enabled);
}

#[test]
fn sketch_grid_step_normalizes_toolbar_input() {
    let mut app = CircuitCiApp {
        sketch_grid_step: f32::NAN,
        ..Default::default()
    };
    app.normalize_sketch_grid_step();
    assert_eq!(app.sketch_grid_step, DEFAULT_SKETCH_GRID_STEP);

    app.sketch_grid_step = 1.0;
    app.normalize_sketch_grid_step();
    assert_eq!(app.sketch_grid_step, 4.0);

    app.sketch_grid_step = 128.0;
    app.normalize_sketch_grid_step();
    assert_eq!(app.sketch_grid_step, 96.0);
}

#[test]
fn run_plus_scopes_transition_opens_simulation_stage() {
    let mut app = CircuitCiApp {
        stage: Stage::Sketch,
        pending_scope_probe: Some(ScopeProbeTarget {
            scenario_name: "astable".to_string(),
            probe_name: "v_out".to_string(),
        }),
        ..Default::default()
    };

    app.open_scopes_for_running_validation();

    assert_eq!(app.stage, Stage::Simulation);
    assert_eq!(app.status, "Running validation in Scopes.");
    assert!(
        app.diagnostics
            .iter()
            .any(|line| line.contains("Run + Scopes opened the Scopes workspace"))
    );
    assert_eq!(
        app.pending_scope_probe,
        Some(ScopeProbeTarget {
            scenario_name: "astable".to_string(),
            probe_name: "v_out".to_string(),
        })
    );
}
