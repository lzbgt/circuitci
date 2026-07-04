mod common;

use common::{
    assert_report_schema_valid, assert_yaml_file_valid, binary_available, run_validation_with_path,
};
use serde_json::Value;
use std::fs;
use std::process::Command;

const REAL_XYCE_CONFORMANCE_ENV: &str = "CIRCUITCI_RUN_REAL_XYCE";

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

#[cfg(unix)]
fn real_xyce_sparameter_conformance_enabled() -> bool {
    if std::env::var(REAL_XYCE_CONFORMANCE_ENV).as_deref() != Ok("1") {
        eprintln!(
            "skipping real Xyce S-parameter conformance; set {REAL_XYCE_CONFORMANCE_ENV}=1 to run"
        );
        return false;
    }
    if !binary_available("Xyce") && !binary_available("xyce") {
        eprintln!("skipping real Xyce S-parameter conformance; Xyce/xyce is not on PATH");
        return false;
    }
    true
}

fn artifact_path<'a>(report: &'a Value, suffix: &str) -> &'a str {
    report["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with(suffix))
        .unwrap_or_else(|| panic!("missing artifact ending with {suffix}"))
}

fn waveform_path<'a>(report: &'a Value, suffix: &str) -> &'a str {
    report["waveforms"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|waveform| waveform.as_str())
        .find(|waveform| waveform.ends_with(suffix))
        .unwrap_or_else(|| panic!("missing waveform ending with {suffix}"))
}

fn assert_csv_has_header(path: &str, expected: &[&str]) {
    let text = fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    let header = lines.next().unwrap_or("");
    for column in expected {
        assert!(
            header.split(',').any(|actual| actual == *column),
            "{path} header {header:?} missing column {column}"
        );
    }
    assert!(lines.next().is_some(), "{path} has no data rows");
}

fn write_sparameter_project(
    dir: &std::path::Path,
    backend: &str,
    port_positive_node: &str,
) -> std::path::PathBuf {
    write_sparameter_project_with_analysis_extra(dir, backend, port_positive_node, "")
}

fn write_sparameter_project_with_analysis_extra(
    dir: &std::path::Path,
    backend: &str,
    port_positive_node: &str,
    analysis_extra: &str,
) -> std::path::PathBuf {
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
      backend: {backend}
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
{analysis_extra}
      stimuli:
        - {{ name: two_port_sweep, description: Planned two-port S-parameter sweep. }}
      probes:
        - {{ name: s11, expression: "S(p1,p1)" }}
      assertions: []
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy(),
            analysis_extra = analysis_extra
        ),
    )
    .unwrap();
    project
}

#[cfg(unix)]
#[test]
fn sparameter_contract_is_schema_valid_and_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project(project_dir.path(), "ngspice", "port1");
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
        "xyce_s_parameters_csv_or_touchstone"
    );
    assert_eq!(report["failures"][0]["measured"]["port_count"], 2);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_sparameter_backend_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project(project_dir.path(), "xyce", "port1");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["selected_backend"],
        "Xyce"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "xyce_s_parameters_csv_or_touchstone"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("circuitci_xyce_sparameter.cir")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| { artifact.as_str().unwrap().ends_with("xyce_sparameter.log") })
    );
    assert!(report["waveforms"].as_array().unwrap().is_empty());
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_sparameter_backend_normalizes_touchstone_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.5 0.0 2.0 0.0 0.01 0.0 0.4 0.0\\n1.0e9 0.2 0.0 1.5 0.0 0.02 0.0 0.3 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project(project_dir.path(), "xyce", "port1");
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
    let waveforms = report["waveforms"].as_array().unwrap();
    assert_eq!(waveforms.len(), 1);
    assert!(waveforms[0].as_str().unwrap().ends_with("s_parameters.csv"));
    let s_parameters = fs::read_to_string(waveforms[0].as_str().unwrap()).unwrap();
    assert!(
        s_parameters.contains("frequency_hz,s11_mag_db,s11_phase_deg,s11_mag_linear,s21_mag_db")
    );
    assert!(s_parameters.contains("1.000000000000e6,-6.020599913280e0"));
    assert!(s_parameters.contains("3.521825181114e0,0.000000000000e0,1.500000000000e0"));
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(
        artifacts
            .iter()
            .any(|artifact| { artifact.as_str().unwrap().ends_with("s_parameters_raw.s2p") })
    );
    let manifest_path = artifacts
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with("solver_manifest.json"))
        .unwrap();
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["analysis"]["kind"], "s_parameter");
    assert_eq!(
        manifest["outputs"]["raw"][0]["kind"],
        "xyce_s_parameters_touchstone"
    );
    assert_eq!(manifest["outputs"]["normalized"][0]["kind"], "s_parameters");
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_assertions_pass_and_project_summary_rows() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.5 0.0 2.0 0.0 0.01 0.0 0.4 0.0\\n1.0e9 0.2 0.0 1.5 0.0 0.02 0.0 0.3 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_assertions:
          - name: s11_return_loss_floor
            parameter: s11
            metric: return_loss_db
            aggregation: min
            relation: above
            threshold: 6.0
            unit: dB
          - name: s11_vswr_ceiling
            parameter: s11
            metric: vswr
            aggregation: max
            relation: below
            threshold: 3.1
          - name: s21_insertion_loss_ceiling
            parameter: s21
            metric: insertion_loss_db
            aggregation: max
            relation: below
            threshold: 0.0
            unit: dB
"#,
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

    assert_eq!(report["result"], "pass");
    let summary_path = artifact_path(&report, "s_parameter_summary.csv");
    assert_csv_has_header(
        summary_path,
        &[
            "parameter",
            "min_return_loss_db",
            "max_insertion_loss_db",
            "max_vswr",
        ],
    );
    let summaries = report["s_parameter_summaries"].as_array().unwrap();
    let s11 = summaries
        .iter()
        .find(|row| row["parameter"] == "s11")
        .expect("s11 summary row");
    assert!(s11["min_return_loss_db"].as_f64().unwrap() > 6.0);
    assert!((s11["max_vswr"].as_f64().unwrap() - 3.0).abs() < 1.0e-9);
    let s21 = summaries
        .iter()
        .find(|row| row["parameter"] == "s21")
        .expect("s21 summary row");
    assert!(s21["max_insertion_loss_db"].as_f64().unwrap() < 0.0);
    assert!(s21["max_vswr"].is_null());
    let markdown = fs::read_to_string(out_dir.path().join("report.md")).unwrap();
    assert!(markdown.contains("## S-Parameter Summary"));
    assert!(markdown.contains("`s11`"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_assertion_fails_on_return_loss_limit() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.5 0.0 2.0 0.0 0.01 0.0 0.4 0.0\\n1.0e9 0.2 0.0 1.5 0.0 0.02 0.0 0.3 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_assertions:
          - name: s11_return_loss_floor
            parameter: s11
            metric: return_loss_db
            aggregation: min
            relation: above
            threshold: 10.0
            unit: dB
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "s11_return_loss_floor")
        .expect("return-loss assertion failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(failure["measured"]["metric"], "return_loss_db");
    assert_eq!(failure["measured"]["aggregation"], "min");
    assert_eq!(failure["limit"]["above_threshold"], 10.0);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_assertion_rejects_vswr_on_transmission_term() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_assertions:
          - name: invalid_vswr
            parameter: s21
            metric: vswr
            aggregation: max
            relation: below
            threshold: 2.0
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("requires a reflection parameter")
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_network_assertions_pass_with_reciprocal_passive_two_port() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.1 0.0 0.8 0.0 0.8001 0.0 0.1 0.0\\n1.0e9 0.05 0.0 0.7 0.0 0.70005 0.0 0.05 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_network_assertions:
          - name: reciprocal_two_port
            metric: reciprocity_error_linear
            relation: below
            threshold: 0.001
          - name: passive_two_port
            metric: passivity_max_singular_value
            relation: below
            threshold: 0.91
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "pass");
    artifact_path(&report, "s_parameter_network_summary.csv");
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_network_assertion_fails_on_passivity_limit() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.5 0.0 2.0 0.0 0.01 0.0 0.4 0.0\\n1.0e9 0.2 0.0 1.5 0.0 0.02 0.0 0.3 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_network_assertions:
          - name: passive_two_port
            metric: passivity_max_singular_value
            relation: below
            threshold: 1.0
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "passive_two_port")
        .expect("passivity assertion failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(
        failure["measured"]["metric"],
        "passivity_max_singular_value"
    );
    assert!(failure["measured"]["value"].as_f64().unwrap() > 1.0);
    assert_eq!(failure["limit"]["below_threshold"], 1.0);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_xyce_sparameter_conformance_normalizes_touchstone_when_enabled() {
    if !real_xyce_sparameter_conformance_enabled() {
        return;
    }
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project(project_dir.path(), "xyce", "port1");
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
    assert_csv_has_header(
        waveform_path(&report, "s_parameters.csv"),
        &[
            "frequency_hz",
            "s11_mag_db",
            "s11_phase_deg",
            "s11_mag_linear",
            "s21_mag_db",
            "s21_phase_deg",
            "s21_mag_linear",
        ],
    );
    artifact_path(&report, "s_parameters_raw.s2p");
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["analysis"]["kind"], "s_parameter");
    assert_eq!(
        manifest["outputs"]["raw"][0]["kind"],
        "xyce_s_parameters_touchstone"
    );
    assert_eq!(manifest["outputs"]["normalized"][0]["kind"], "s_parameters");
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_contract_rejects_unbound_port_node_before_backend_planning() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project(project_dir.path(), "xyce", "missing_node");

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
