use super::*;
use std::path::Path;

#[test]
fn append_analog_sparameter_scenario_emits_valid_two_port_yaml() {
    let draft = AnalogSParameterScenarioDraft {
        name: "gui_sparam".to_string(),
        ground_net: "gnd".to_string(),
        port1_net: "port1".to_string(),
        port2_net: "port2".to_string(),
        probe_name: "s11".to_string(),
        start_frequency_hz: 1.0e6,
        stop_frequency_hz: 1.0e9,
        points_per_decade: 25,
        reference_impedance_ohm: 50.0,
    };
    let edited = append_analog_sparameter_scenario_with_project_path(
        sparameter_project_yaml(),
        Path::new("examples/generated_sparameter/project.yaml"),
        &draft,
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let scenario = &project.scenarios[0];

    assert_eq!(scenario.name, "gui_sparam");
    assert_eq!(scenario.scenario_type, "analog_sparameter");
    assert_eq!(
        scenario.checks,
        vec!["SPICE_S_PARAMETER_ANALYSIS".to_string()]
    );
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.backend, crate::board_ir::AnalogBackend::Auto);
    assert_eq!(analog.generated.as_ref().unwrap().ground_net, "gnd");
    assert_eq!(analog.analysis.analysis_type, "sparam");
    assert_eq!(analog.analysis.start_frequency_hz, Some(1.0e6));
    assert_eq!(analog.analysis.stop_frequency_hz, Some(1.0e9));
    assert_eq!(analog.analysis.points_per_decade, Some(25));
    let ports = &analog.analysis.s_parameter_ports;
    assert_eq!(ports.len(), 2);
    assert_eq!(ports[0].name, "p1");
    assert_eq!(ports[0].positive_node, "port1");
    assert_eq!(ports[0].negative_node, "0");
    assert_eq!(ports[0].reference_impedance_ohm, 50.0);
    assert_eq!(ports[1].name, "p2");
    assert_eq!(ports[1].positive_node, "port2");
    assert_eq!(ports[1].negative_node, "0");
    assert_eq!(ports[1].reference_impedance_ohm, 50.0);
    assert_eq!(analog.probes[0].name, "s11");
    assert_eq!(analog.probes[0].expression, "S(p1,p1)");
}

#[test]
fn append_analog_sparameter_scenario_rejects_duplicate_port_nets() {
    let draft = AnalogSParameterScenarioDraft {
        name: "gui_sparam".to_string(),
        ground_net: "gnd".to_string(),
        port1_net: "port1".to_string(),
        port2_net: "port1".to_string(),
        probe_name: "s11".to_string(),
        start_frequency_hz: 1.0e6,
        stop_frequency_hz: 1.0e9,
        points_per_decade: 25,
        reference_impedance_ohm: 50.0,
    };
    let error = append_analog_sparameter_scenario_with_project_path(
        sparameter_project_yaml(),
        Path::new("examples/generated_sparameter/project.yaml"),
        &draft,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("S-parameter ports must use distinct nets")
    );
}

fn sparameter_project_yaml() -> &'static str {
    r#"
project:
  name: sparameter_run_setup_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 50.0
      pins:
        A: port1
        B: port2
    R2:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 50.0
      pins:
        A: port2
        B: gnd
  nets:
    port1:
      kind: digital_or_analog
    port2:
      kind: digital_or_analog
    gnd:
      kind: ground
"#
}
