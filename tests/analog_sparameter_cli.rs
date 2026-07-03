mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation_with_path};
use serde_json::Value;
use std::fs;

#[cfg(unix)]
fn fake_executable(dir: &std::path::Path, name: &str) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    fs::write(&path, "#!/bin/sh\nexit 99\n").unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
}

fn write_sparameter_project(dir: &std::path::Path, port_positive_node: &str) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: xyce_sparameter_contract, version: 0.1.0 }}
libraries:
  - {libs}
board:
  components:
    R1:
      model: generic.analog.resistor
      pins: {{ A: port1, B: port2 }}
      spice: {{ primitive: resistor, value_ohm: 50 }}
    R2:
      model: generic.analog.resistor
      pins: {{ A: port2, B: gnd }}
      spice: {{ primitive: resistor, value_ohm: 50 }}
  nets:
    port1: {{ kind: digital_or_analog }}
    port2: {{ kind: digital_or_analog }}
    gnd: {{ kind: ground }}
scenarios:
  - name: two_port_sparameter
    type: analog_sparameter
    checks: [SPICE_S_PARAMETER_ANALYSIS]
    analog:
      backend: xyce
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [R1, R2]
      model_files: []
      node_bindings:
        - {{ node: port1, net: port1 }}
        - {{ node: port2, net: port2 }}
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: port1, endpoint: {{ component: R1, pin: A }} }}
        - {{ node: port2, endpoint: {{ component: R1, pin: B }} }}
        - {{ node: port2, endpoint: {{ component: R2, pin: A }} }}
        - {{ node: "0", endpoint: {{ component: R2, pin: B }} }}
      analysis:
        type: sparam
        start_frequency_hz: 1000000.0
        stop_frequency_hz: 1000000000.0
        points_per_decade: 20
        s_parameter_ports:
          - {{ name: p1, positive_node: {port_positive_node}, negative_node: "0", reference_impedance_ohm: 50.0 }}
          - {{ name: p2, positive_node: port2, negative_node: "0", reference_impedance_ohm: 50.0 }}
      stimuli:
        - {{ name: two_port_sweep, description: Planned two-port S-parameter sweep. }}
      probes:
        - {{ name: s11, expression: "S(p1,p1)" }}
      assertions: []
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy()
        ),
    )
    .unwrap();
    project
}

#[cfg(unix)]
#[test]
fn sparameter_contract_is_schema_valid_and_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project(project_dir.path(), "port1");
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["adapter_status"],
        "planned_not_implemented"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][0],
        "s_parameters"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "s_parameters_csv_or_touchstone"
    );
    assert_eq!(report["failures"][0]["measured"]["port_count"], 2);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_contract_rejects_unbound_port_node_before_backend_planning() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project(project_dir.path(), "missing_node");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("positive_node missing_node is not bound")
    );
    assert_report_schema_valid(&report);
}
