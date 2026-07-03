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

fn write_sensitivity_project(
    dir: &std::path::Path,
    backend: &str,
    output_expression: &str,
    mode: &str,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    let ac_fields = if mode == "ac" {
        r#"
        start_frequency_hz: 100.0
        stop_frequency_hz: 100000.0
        points_per_decade: 20"#
    } else {
        ""
    };
    fs::write(
        &project,
        format!(
            r#"project: {{ name: sensitivity_contract, version: 0.1.0 }}
libraries:
  - {libs}
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins: {{ P: vin, N: gnd }}
      spice: {{ primitive: dc_voltage_source, dc_v: 1.0 }}
    R1:
      model: generic.analog.resistor
      pins: {{ A: vin, B: out }}
      spice: {{ primitive: resistor, value_ohm: 1000 }}
    R2:
      model: generic.analog.resistor
      pins: {{ A: out, B: gnd }}
      spice: {{ primitive: resistor, value_ohm: 1000 }}
  nets:
    vin: {{ kind: power, nominal_voltage: 1.0, powered: true }}
    out: {{ kind: digital_or_analog }}
    gnd: {{ kind: ground }}
scenarios:
  - name: divider_sensitivity
    type: analog_sensitivity
    checks: [SPICE_SENSITIVITY_ANALYSIS]
    analog:
      backend: {backend}
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1, R2]
      model_files: []
      node_bindings:
        - {{ node: vin, net: vin }}
        - {{ node: out, net: out }}
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: vin, endpoint: {{ component: V1, pin: P }} }}
        - {{ node: "0", endpoint: {{ component: V1, pin: N }} }}
        - {{ node: vin, endpoint: {{ component: R1, pin: A }} }}
        - {{ node: out, endpoint: {{ component: R1, pin: B }} }}
        - {{ node: out, endpoint: {{ component: R2, pin: A }} }}
        - {{ node: "0", endpoint: {{ component: R2, pin: B }} }}
      analysis:
        type: sens
        sensitivity_output_expression: {output_expression}
        sensitivity_mode: {mode}{ac_fields}
        sensitivity_filters: [R1, R2]
      stimuli:
        - {{ name: sensitivity_probe, description: Planned sensitivity extraction. }}
      probes:
        - {{ name: out_sensitivity, expression: V(out) }}
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
fn sensitivity_contract_is_schema_valid_and_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project(project_dir.path(), "ngspice", "V(out)", "dc");
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_SENSITIVITY_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["adapter_status"],
        "planned_not_implemented"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][0],
        "sensitivity_summary"
    );
    assert_eq!(
        report["failures"][0]["measured"]["output_expression"],
        "V(out)"
    );
    assert_eq!(report["failures"][0]["measured"]["mode"], "dc");
    assert_eq!(report["failures"][0]["measured"]["filters"][0], "R1");
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "sensitivity_summary_csv_or_json"
    );
    assert_eq!(
        report["failures"][0]["limit"]["implemented_backend"],
        "none_yet"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn ac_sensitivity_contract_requires_and_reports_frequency_bounds() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project(project_dir.path(), "ngspice", "V(out)", "ac");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_SENSITIVITY_ANALYSIS");
    assert_eq!(report["failures"][0]["measured"]["mode"], "ac");
    assert_eq!(
        report["failures"][0]["measured"]["frequency_start_hz"],
        100.0
    );
    assert_eq!(
        report["failures"][0]["measured"]["frequency_stop_hz"],
        100000.0
    );
    assert_eq!(report["failures"][0]["measured"]["points_per_decade"], 20);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sensitivity_contract_rejects_unbound_output_node_before_backend_planning() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path =
        write_sensitivity_project(project_dir.path(), "ngspice", "V(missing_node)", "dc");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("unbound node missing_node")
    );
    assert_report_schema_valid(&report);
}
