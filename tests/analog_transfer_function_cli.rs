mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation_with_path};
use serde_json::Value;
use std::fs;
use std::process::Command;

#[cfg(unix)]
fn fake_executable(dir: &std::path::Path, name: &str) {
    fake_executable_with_body(dir, name, "#!/bin/sh\nexit 99\n");
}

#[cfg(unix)]
fn fake_executable_with_body(dir: &std::path::Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
}

fn write_transfer_function_project(
    dir: &std::path::Path,
    backend: &str,
    input_source: &str,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: transfer_function_contract, version: 0.1.0 }}
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
  - name: divider_tf
    type: analog_transfer_function
    checks: [SPICE_TRANSFER_FUNCTION_ANALYSIS]
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
        type: tf
        transfer_output_expression: V(out)
        transfer_input_source: {input_source}
      stimuli:
        - {{ name: dc_gain, description: Small-signal transfer from V1 to out. }}
      probes:
        - {{ name: out_gain, expression: V(out) }}
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
fn transfer_function_backend_normalizes_summary_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'transfer_function = 5.000000e-001\\noutput_impedance_at_v(out) = 5.000000e+002\\nv1#input_impedance = 2.000000e+003\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_transfer_function_project(project_dir.path(), "ngspice", "V1");
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);
    fs::create_dir_all("out").unwrap();
    let out_dir = tempfile::tempdir_in("out").unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            project_path.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .env("PATH", fake_path.path())
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();

    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    let artifacts = report["artifacts"].as_array().unwrap();
    let summary_path = artifacts
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with("transfer_function_summary.csv"))
        .unwrap();
    let summary = fs::read_to_string(summary_path).unwrap();
    assert!(summary.contains(
        "output_expression,input_source,transfer_function_gain,input_resistance_ohm,output_resistance_ohm"
    ));
    assert!(summary.contains("V(out),V1,5.000000000000e-1"));
    assert!(summary.contains("2.000000000000e3,5.000000000000e2"));
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("transfer_function_raw.txt")
    }));
    let manifest_path = artifacts
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with("solver_manifest.json"))
        .unwrap();
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(manifest["analysis"]["kind"], "transfer_function");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "transfer_function_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn transfer_function_backend_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_transfer_function_project(project_dir.path(), "ngspice", "V1");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "SPICE_TRANSFER_FUNCTION_ANALYSIS"
    );
    assert_eq!(
        report["failures"][0]["measured"]["selected_backend"],
        "ngspice"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "ngspice_transfer_function_summary_csv"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("circuitci_ngspice_tf.cir")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("ngspice_tf.log"))
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn transfer_function_xyce_backend_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_transfer_function_project(project_dir.path(), "xyce", "V1");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "SPICE_TRANSFER_FUNCTION_ANALYSIS"
    );
    assert_eq!(
        report["failures"][0]["measured"]["adapter_status"],
        "planned_not_implemented"
    );
    assert_eq!(
        report["failures"][0]["limit"]["implemented_backend"],
        "ngspice"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn transfer_function_contract_rejects_missing_generated_input_source() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path =
        write_transfer_function_project(project_dir.path(), "ngspice", "missing_source");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("input source missing_source is not a generated board component")
    );
    assert_report_schema_valid(&report);
}
