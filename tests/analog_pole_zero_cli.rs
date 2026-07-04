mod common;

use common::{
    assert_report_schema_valid, assert_yaml_file_valid, binary_available, run_validation_with_path,
};
use serde_json::Value;
use std::fs;
use std::process::Command;

#[cfg(unix)]
const REAL_NGSPICE_CONFORMANCE_ENV: &str = "CIRCUITCI_RUN_REAL_NGSPICE";

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

fn write_pole_zero_project(
    dir: &std::path::Path,
    backend: &str,
    output_node: &str,
    input_source: &str,
) -> std::path::PathBuf {
    write_pole_zero_project_with_analysis_extra(dir, backend, output_node, input_source, "")
}

fn write_pole_zero_project_with_analysis_extra(
    dir: &std::path::Path,
    backend: &str,
    output_node: &str,
    input_source: &str,
    analysis_extra: &str,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: pole_zero_contract, version: 0.1.0 }}
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
    C1:
      model: generic.analog.capacitor
      pins: {{ A: out, B: gnd }}
      spice: {{ primitive: capacitor, value_f: 0.000001 }}
  nets:
    vin: {{ kind: power, nominal_voltage: 1.0, powered: true }}
    out: {{ kind: digital_or_analog }}
    gnd: {{ kind: ground }}
scenarios:
  - name: rc_pole_zero
    type: analog_pole_zero
    checks: [SPICE_POLE_ZERO_ANALYSIS]
    analog:
      backend: {backend}
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1, C1]
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
        - {{ node: out, endpoint: {{ component: C1, pin: A }} }}
        - {{ node: "0", endpoint: {{ component: C1, pin: B }} }}
      analysis:
        type: pz
        pole_zero_output_node: {output_node}
        pole_zero_reference_node: "0"
        pole_zero_input_source: {input_source}
        pole_zero_mode: poles_and_zeros
{analysis_extra}
      stimuli:
        - {{ name: pz_probe, description: Planned RC pole-zero extraction. }}
      probes:
        - {{ name: out_pz, expression: V(out) }}
      assertions: []
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy()
        ),
    )
    .unwrap();
    project
}

fn artifact_path<'a>(report: &'a Value, suffix: &str) -> &'a str {
    report["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with(suffix))
        .unwrap_or_else(|| panic!("missing artifact with suffix {suffix}"))
}

#[cfg(unix)]
fn real_ngspice_conformance_enabled() -> bool {
    if std::env::var(REAL_NGSPICE_CONFORMANCE_ENV).as_deref() != Ok("1") {
        eprintln!(
            "skipping real-ngspice pole-zero conformance; set {REAL_NGSPICE_CONFORMANCE_ENV}=1"
        );
        return false;
    }
    if !binary_available("ngspice") {
        eprintln!("skipping real-ngspice pole-zero conformance; ngspice is not on PATH");
        return false;
    }
    true
}

fn assert_pole_zero_summary_has_pole(path: &str) {
    let text = fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    let header = lines.next().unwrap_or("");
    assert_eq!(
        header,
        "output_node,reference_node,input_source,mode,root_kind,root_index,real_rad_per_s,imaginary_rad_per_s,frequency_hz"
    );
    assert!(
        lines.any(|line| line.contains(",pole,")),
        "{path} did not contain a pole row"
    );
}

#[cfg(unix)]
#[test]
fn pole_zero_backend_normalizes_summary_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '    pole(1)             : voltage, complex, 1 long [default scale]\\nall = -1.00000e+03,0.000000e+00\\n    zero(1)             : voltage, complex, 1 long\\nall = -2.00000e+03,5.000000e+02\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pole_zero_project(project_dir.path(), "ngspice", "out", "V1");
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
    let summary_path = artifact_path(&report, "pole_zero_summary.csv");
    let summary = fs::read_to_string(summary_path).unwrap();
    assert!(summary.contains(
        "output_node,reference_node,input_source,mode,root_kind,root_index,real_rad_per_s,imaginary_rad_per_s,frequency_hz"
    ));
    assert!(summary.contains("out,0,V1,poles_and_zeros,pole,1,-1.000000000000e3"));
    assert!(summary.contains("out,0,V1,poles_and_zeros,zero,1,-2.000000000000e3,5.000000000000e2"));
    assert!(artifact_path(&report, "pole_zero_raw.txt").ends_with("pole_zero_raw.txt"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(manifest["analysis"]["kind"], "pole_zero");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "pole_zero_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn pole_zero_assertions_pass_on_stable_pole_and_zero_frequency() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '    pole(1)             : voltage, complex, 1 long [default scale]\\nall = -1.00000e+03,0.000000e+00\\n    zero(1)             : voltage, complex, 1 long\\nall = -2.00000e+03,5.000000e+02\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pole_zero_project_with_analysis_extra(
        project_dir.path(),
        "ngspice",
        "out",
        "V1",
        "        pole_zero_assertions:\n        - { name: stable_pole, root_kind: pole, root_index: 1, metric: real_rad_per_s, relation: below, threshold: -5.0e2 }\n        - { name: zero_frequency_visible, root_kind: zero, root_index: 1, metric: frequency_hz, relation: above, threshold: 1.0e2 }\n",
    );
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
    assert_eq!(report["pole_zero_summaries"].as_array().unwrap().len(), 2);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn pole_zero_assertion_fails_on_root_limit() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '    pole(1)             : voltage, complex, 1 long [default scale]\\nall = -1.00000e+03,0.000000e+00\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pole_zero_project_with_analysis_extra(
        project_dir.path(),
        "ngspice",
        "out",
        "V1",
        "        pole_zero_assertions:\n        - { name: pole_too_slow, root_kind: pole, root_index: 1, metric: real_rad_per_s, relation: below, threshold: -1.5e3 }\n",
    );
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

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_POLE_ZERO_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["assertion"],
        "pole_too_slow"
    );
    assert_eq!(
        report["failures"][0]["measured"]["metric"],
        "real_rad_per_s"
    );
    assert_eq!(report["failures"][0]["limit"]["below_threshold"], -1.5e3);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn pole_zero_assertion_fails_closed_on_missing_root() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '    pole(1)             : voltage, complex, 1 long [default scale]\\nall = -1.00000e+03,0.000000e+00\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pole_zero_project_with_analysis_extra(
        project_dir.path(),
        "ngspice",
        "out",
        "V1",
        "        pole_zero_assertions:\n        - { name: second_pole_required, root_kind: pole, root_index: 2, metric: frequency_hz, relation: below, threshold: 1.0e4 }\n",
    );
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

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_POLE_ZERO_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["assertion"],
        "second_pole_required"
    );
    assert_eq!(report["failures"][0]["limit"]["required_root_index"], 2);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_ngspice_pole_zero_conformance_when_enabled() {
    if !real_ngspice_conformance_enabled() {
        return;
    }

    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pole_zero_project(project_dir.path(), "ngspice", "out", "V1");
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
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();

    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_pole_zero_summary_has_pole(artifact_path(&report, "pole_zero_summary.csv"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(manifest["analysis"]["kind"], "pole_zero");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "pole_zero_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn pole_zero_backend_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pole_zero_project(project_dir.path(), "ngspice", "out", "V1");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_POLE_ZERO_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["selected_backend"],
        "ngspice"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "ngspice_pole_zero_summary_csv"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("circuitci_ngspice_pz.cir")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("ngspice_pz.log"))
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn pole_zero_xyce_backend_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pole_zero_project(project_dir.path(), "xyce", "out", "V1");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_POLE_ZERO_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["adapter_status"],
        "planned_not_implemented"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][0],
        "pole_zero_summary"
    );
    assert_eq!(report["failures"][0]["measured"]["output_node"], "out");
    assert_eq!(report["failures"][0]["measured"]["reference_node"], "0");
    assert_eq!(report["failures"][0]["measured"]["input_source"], "V1");
    assert_eq!(report["failures"][0]["measured"]["mode"], "poles_and_zeros");
    assert!(
        report["failures"][0]["measured"]["adapter_blocker"]
            .as_str()
            .unwrap()
            .contains("does not document a native .PZ command")
    );
    assert_eq!(
        report["failures"][0]["measured"]["evidence_sources"][0],
        "docs/research/circuit_simulation_full_featured/sources/Xyce_Reference_Guide_7.8.txt"
    );
    assert_eq!(
        report["failures"][0]["limit"]["implemented_backend"],
        "ngspice"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn pole_zero_contract_rejects_unbound_output_node_before_backend_planning() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pole_zero_project(project_dir.path(), "ngspice", "missing_node", "V1");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("output node missing_node is not bound")
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn pole_zero_contract_rejects_missing_generated_input_source() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path =
        write_pole_zero_project(project_dir.path(), "ngspice", "out", "missing_source");

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
