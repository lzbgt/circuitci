mod common;

use common::{
    assert_report_schema_valid, assert_yaml_file_valid, binary_available, run_validation_with_path,
};
use serde_json::Value;
use std::fs;
use std::process::Command;

const REAL_NGSPICE_CONFORMANCE_ENV: &str = "CIRCUITCI_RUN_REAL_NGSPICE";
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
fn real_ngspice_sparameter_conformance_enabled() -> bool {
    if std::env::var(REAL_NGSPICE_CONFORMANCE_ENV).as_deref() != Ok("1") {
        eprintln!(
            "skipping real ngspice S-parameter conformance; set {REAL_NGSPICE_CONFORMANCE_ENV}=1 to run"
        );
        return false;
    }
    if !binary_available("ngspice") {
        eprintln!("skipping real ngspice S-parameter conformance; ngspice is not on PATH");
        return false;
    }
    true
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
            header
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .any(|actual| actual.eq_ignore_ascii_case(column)),
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
fn ngspice_sparameter_backend_normalizes_smatrix_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'frequency_hz,s_1_1_real,s_1_1_imaginary,s_2_1_real,s_2_1_imaginary,s_1_2_real,s_1_2_imaginary,s_2_2_real,s_2_2_imaginary\\n1.0e6,0.5,0,0.1,0,0.1,0,0.4,0\\n1.0e9,0.2,0,0.15,0,0.15,0,0.3,0\\n' > s_parameters_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project(project_dir.path(), "ngspice", "port1");
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
    let s_parameters_path = waveform_path(&report, "s_parameters.csv");
    assert_csv_has_header(
        s_parameters_path,
        &["s11_mag_db", "s21_mag_db", "s12_mag_db", "s22_mag_db"],
    );
    assert!(artifact_path(&report, "s_parameter_summary.csv").ends_with("s_parameter_summary.csv"));
    assert_eq!(report["s_parameter_summaries"].as_array().unwrap().len(), 4);
    let manifest_path = artifact_path(&report, "solver_manifest.json");
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["analysis"]["kind"], "s_parameter");
    assert_eq!(manifest["outputs"]["normalized"][0]["kind"], "s_parameters");
    assert_eq!(
        manifest["outputs"]["normalized"].as_array().unwrap().len(),
        1
    );
    let markdown = fs::read_to_string(out_dir.path().join("report.md")).unwrap();
    assert!(markdown.contains("## S-Parameter Summary"));
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
    assert!(s_parameters.contains(
        "frequency_hz,reference_impedance_ohm,s11_mag_db,s11_phase_deg,s11_mag_linear,s21_mag_db"
    ));
    assert!(s_parameters.contains("1.000000000000e6,5.000000000000e1,-6.020599913280e0"));
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
fn auto_sparameter_backend_uses_xyce_when_ngspice_is_absent() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.5 0.0 2.0 0.0 0.01 0.0 0.4 0.0\\n1.0e9 0.2 0.0 1.5 0.0 0.02 0.0 0.3 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "auto",
        "port1",
        r#"        s_parameter_assertions:
          - name: s11_return_loss_floor
            parameter: s11
            metric: return_loss_db
            aggregation: min
            relation: above
            threshold: 3.0
        s_parameter_network_assertions:
          - name: passive_two_port
            metric: passivity_max_singular_value
            relation: below
            threshold: 3.0
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
    let s_parameters_path = waveform_path(&report, "s_parameters.csv");
    assert_csv_has_header(
        s_parameters_path,
        &["s11_mag_db", "s21_mag_db", "s12_mag_db", "s22_mag_db"],
    );
    artifact_path(&report, "s_parameter_summary.csv");
    artifact_path(&report, "s_parameter_network_summary.csv");
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["requested"], "auto");
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
          - name: s11_mismatch_loss_ceiling
            parameter: s11
            metric: mismatch_loss_db
            aggregation: max
            relation: below
            threshold: 1.3
            unit: dB
          - name: s21_insertion_loss_ceiling
            parameter: s21
            metric: insertion_loss_db
            aggregation: max
            relation: below
            threshold: 0.0
            unit: dB
          - name: s11_impedance_ceiling
            parameter: s11
            metric: impedance_magnitude_ohm
            aggregation: max
            relation: below
            threshold: 151.0
            unit: ohm
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
            "max_mismatch_loss_db",
            "min_group_delay_s",
            "max_group_delay_s",
            "min_impedance_real_ohm",
            "max_impedance_magnitude_ohm",
        ],
    );
    let summaries = report["s_parameter_summaries"].as_array().unwrap();
    let s11 = summaries
        .iter()
        .find(|row| row["parameter"] == "s11")
        .expect("s11 summary row");
    assert!(s11["min_return_loss_db"].as_f64().unwrap() > 6.0);
    assert!((s11["max_vswr"].as_f64().unwrap() - 3.0).abs() < 1.0e-9);
    assert!((s11["max_mismatch_loss_db"].as_f64().unwrap() - 1.249387366083).abs() < 1.0e-9);
    assert_eq!(s11["max_group_delay_s"], 0.0);
    assert!((s11["min_impedance_real_ohm"].as_f64().unwrap() - 75.0).abs() < 1.0e-9);
    assert!((s11["max_impedance_magnitude_ohm"].as_f64().unwrap() - 150.0).abs() < 1.0e-9);
    let s21 = summaries
        .iter()
        .find(|row| row["parameter"] == "s21")
        .expect("s21 summary row");
    assert!(s21["max_insertion_loss_db"].as_f64().unwrap() < 0.0);
    assert!(s21["max_vswr"].is_null());
    assert!(s21["max_mismatch_loss_db"].is_null());
    assert_eq!(s21["max_group_delay_s"], 0.0);
    assert!(s21["max_impedance_magnitude_ohm"].is_null());
    let markdown = fs::read_to_string(out_dir.path().join("report.md")).unwrap();
    assert!(markdown.contains("## S-Parameter Summary"));
    assert!(markdown.contains("`s11`"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_assertion_fails_on_impedance_limit() {
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
          - name: s11_impedance_ceiling
            parameter: s11
            metric: impedance_magnitude_ohm
            aggregation: max
            relation: below
            threshold: 120.0
            unit: ohm
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "s11_impedance_ceiling")
        .expect("impedance assertion failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(failure["measured"]["metric"], "impedance_magnitude_ohm");
    assert_eq!(failure["measured"]["aggregation"], "max");
    assert!(failure["measured"]["value"].as_f64().unwrap() > 120.0);
    assert_eq!(failure["limit"]["below_threshold"], 120.0);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_assertion_fails_on_mismatch_loss_limit() {
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
          - name: s11_mismatch_loss_ceiling
            parameter: s11
            metric: mismatch_loss_db
            aggregation: max
            relation: below
            threshold: 1.0
            unit: dB
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "s11_mismatch_loss_ceiling")
        .expect("mismatch-loss assertion failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(failure["measured"]["metric"], "mismatch_loss_db");
    assert_eq!(failure["measured"]["aggregation"], "max");
    assert!(failure["measured"]["value"].as_f64().unwrap() > 1.0);
    assert_eq!(failure["limit"]["below_threshold"], 1.0);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_group_delay_assertions_pass_and_fail_from_phase_slope() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.5 0.0 2.0 0.0 0.01 0.0 0.4 0.0\\n1.0e9 0.2 0.0 0.0 -1.5 0.02 0.0 0.3 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_assertions:
          - name: s21_group_delay_below_300ps
            parameter: s21
            metric: group_delay_s
            aggregation: max
            relation: below
            threshold: 3.0e-10
            unit: s
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
    let summaries = report["s_parameter_summaries"].as_array().unwrap();
    let s21 = summaries
        .iter()
        .find(|row| row["parameter"] == "s21")
        .expect("s21 summary row");
    let group_delay = s21["max_group_delay_s"].as_f64().unwrap();
    assert!((2.4e-10..2.6e-10).contains(&group_delay));
    let markdown = fs::read_to_string(out_dir.path().join("report.md")).unwrap();
    assert!(markdown.contains("group_delay_s="));
    assert_report_schema_valid(&report);

    let failing_project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_assertions:
          - name: s21_group_delay_below_100ps
            parameter: s21
            metric: group_delay_s
            aggregation: max
            relation: below
            threshold: 1.0e-10
"#,
    );
    let report = run_validation_with_path(failing_project_path.to_str().unwrap(), fake_path.path());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "s21_group_delay_below_100ps")
        .expect("group-delay assertion failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(failure["measured"]["metric"], "group_delay_s");
    assert_eq!(failure["measured"]["aggregation"], "max");
    assert!(failure["measured"]["value"].as_f64().unwrap() > 1.0e-10);
    assert_eq!(failure["limit"]["below_threshold"], 1.0e-10);
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
        r#"        s_parameter_source_reflection: { real: 0.2, imaginary: 0.0 }
        s_parameter_load_reflection: { real: 0.1, imaginary: 0.0 }
        s_parameter_network_assertions:
          - name: reciprocal_two_port
            metric: reciprocity_error_linear
            relation: below
            threshold: 0.001
          - name: passive_two_port
            metric: passivity_max_singular_value
            relation: below
            threshold: 0.91
          - name: stable_rollet_k
            metric: rollet_k_min
            relation: above
            threshold: 1.0
          - name: stable_delta
            metric: stability_delta_magnitude_max
            relation: below
            threshold: 1.0
          - name: available_gain_floor
            metric: maximum_available_gain_db_min
            relation: above
            threshold: -5.0
          - name: stable_gain_floor
            metric: maximum_stable_gain_db_min
            relation: above
            threshold: -0.5
          - name: unilateral_gain_floor
            metric: maximum_unilateral_gain_db_min
            relation: above
            threshold: -5.0
          - name: transducer_gain_floor
            metric: transducer_gain_db_min
            relation: above
            threshold: -5.0
          - name: source_available_gain_floor
            metric: available_gain_db_min
            relation: above
            threshold: -5.0
          - name: load_operating_gain_floor
            metric: operating_gain_db_min
            relation: above
            threshold: -5.0
"#,
    );

    let out_dir = tempfile::tempdir().unwrap();
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
    artifact_path(&report, "s_parameter_network_summary.csv");
    let summaries = report["s_parameter_network_summaries"].as_array().unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0]["port_count"], 2);
    assert_eq!(summaries[0]["row_count"], 2);
    assert!(
        summaries[0]["max_reciprocity_error_linear"]
            .as_f64()
            .unwrap()
            < 1.0e-3
    );
    assert!(
        summaries[0]["max_passivity_singular_value"]
            .as_f64()
            .unwrap()
            < 0.91
    );
    assert!(summaries[0]["min_rollet_k"].as_f64().unwrap() > 1.0);
    assert!(
        summaries[0]["max_stability_delta_magnitude"]
            .as_f64()
            .unwrap()
            < 1.0
    );
    assert!(
        summaries[0]["min_maximum_available_gain_db"]
            .as_f64()
            .unwrap()
            > -5.0
    );
    assert!(summaries[0]["min_maximum_stable_gain_db"].as_f64().unwrap() > -0.5);
    assert!(
        summaries[0]["min_maximum_unilateral_gain_db"]
            .as_f64()
            .unwrap()
            > -5.0
    );
    assert_eq!(summaries[0]["source_reflection_real"], 0.2);
    assert_eq!(summaries[0]["source_reflection_imaginary"], 0.0);
    assert_eq!(summaries[0]["load_reflection_real"], 0.1);
    assert_eq!(summaries[0]["load_reflection_imaginary"], 0.0);
    assert!(summaries[0]["min_transducer_gain_db"].as_f64().unwrap() > -5.0);
    assert!(summaries[0]["min_available_gain_db"].as_f64().unwrap() > -5.0);
    assert!(summaries[0]["min_operating_gain_db"].as_f64().unwrap() > -5.0);
    let markdown = fs::read_to_string(out_dir.path().join("report.md")).unwrap();
    assert!(markdown.contains("## S-Parameter Network Summary"));
    assert!(markdown.contains("max_reciprocity_error="));
    assert!(markdown.contains("max_passivity_singular_value="));
    assert!(markdown.contains("min_rollet_k="));
    assert!(markdown.contains("max_stability_delta_magnitude="));
    assert!(markdown.contains("min_maximum_available_gain_db="));
    assert!(markdown.contains("min_maximum_stable_gain_db="));
    assert!(markdown.contains("min_maximum_unilateral_gain_db="));
    assert!(markdown.contains("min_transducer_gain_db="));
    assert!(markdown.contains("min_available_gain_db="));
    assert!(markdown.contains("min_operating_gain_db="));
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
fn sparameter_noise_assertions_fail_closed_until_sp_noise_summary_exists() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.1 0.0 0.8 0.0 0.02 0.0 0.1 0.0\\n1.0e9 0.1 0.0 0.7 0.0 0.02 0.0 0.1 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_noise_assertions:
          - name: nf_limit
            metric: noise_figure_db_max
            relation: below
            threshold: 3.0
          - name: nfmin_limit
            metric: minimum_noise_figure_db_max
            relation: below
            threshold: 1.5
          - name: rn_limit
            metric: equivalent_noise_resistance_ohm_max
            relation: below
            threshold: 10.0
          - name: gamma_opt_limit
            metric: optimum_source_reflection_magnitude_max
            relation: below
            threshold: 0.5
"#,
    );
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    assert_eq!(failures.len(), 4);
    let nf_failure = failures
        .iter()
        .find(|failure| failure["measured"]["assertion"] == "nf_limit")
        .unwrap();
    assert_eq!(nf_failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(nf_failure["measured"]["metric"], "noise_figure_db_max");
    assert_eq!(
        nf_failure["measured"]["backend_status"],
        "planned_not_implemented"
    );
    assert!(
        nf_failure["measured"]["s_parameters"]
            .as_str()
            .unwrap()
            .ends_with("s_parameters.csv")
    );
    assert!(
        nf_failure["measured"]["adapter_blocker"]
            .as_str()
            .unwrap()
            .contains("No trusted non-ngspice RF SP-noise backend path")
    );
    assert_eq!(
        nf_failure["measured"]["evidence_sources"][0],
        "docs/research/circuit_simulation_full_featured/sparameter_noise_backend_evidence.md"
    );
    assert_eq!(
        nf_failure["limit"]["required_normalized_output"],
        "s_parameter_noise_summary"
    );
    assert_eq!(
        nf_failure["limit"]["required_backend_feature"],
        "ngspice_sp_donoise_two_port_noise_outputs"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn auto_sparameter_noise_assertions_keep_xyce_noise_boundary() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.1 0.0 0.8 0.0 0.02 0.0 0.1 0.0\\n1.0e9 0.1 0.0 0.7 0.0 0.02 0.0 0.1 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "auto",
        "port1",
        r#"        s_parameter_noise_assertions:
          - name: nf_limit
            metric: noise_figure_db_max
            relation: below
            threshold: 3.0
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(failure["measured"]["assertion"], "nf_limit");
    assert_eq!(failure["measured"]["metric"], "noise_figure_db_max");
    assert_eq!(
        failure["measured"]["backend_status"],
        "planned_not_implemented"
    );
    assert!(
        failure["measured"]["adapter_blocker"]
            .as_str()
            .unwrap()
            .contains("No trusted non-ngspice RF SP-noise backend path")
    );
    assert_eq!(
        failure["limit"]["required_normalized_output"],
        "s_parameter_noise_summary"
    );
    assert!(
        failure["measured"]["s_parameters"]
            .as_str()
            .unwrap()
            .ends_with("s_parameters.csv")
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn ngspice_sparameter_noise_assertions_evaluate_summary_artifact() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'frequency_hz,s_1_1_real,s_1_1_imaginary,s_2_1_real,s_2_1_imaginary,s_1_2_real,s_1_2_imaginary,s_2_2_real,s_2_2_imaginary\\n1.0e6,0.5,0,0.1,0,0.1,0,0.4,0\\n1.0e9,0.2,0,0.15,0,0.15,0,0.3,0\\n' > s_parameters_raw.csv\nprintf 'frequency_hz,nf_db,nfmin_db,rn_ohm,sopt_real,sopt_imaginary\\n1.0e6,2.0,1.0,4.0,0.3,0.4\\n1.0e9,3.0,1.5,6.0,0.1,0.2\\n' > s_parameter_noise_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "ngspice",
        "port1",
        r#"        s_parameter_assertions:
          - name: s11_return_loss_floor
            parameter: s11
            metric: return_loss_db
            aggregation: min
            relation: above
            threshold: 3.0
        s_parameter_network_assertions:
          - name: passive_two_port
            metric: passivity_max_singular_value
            relation: below
            threshold: 1.0
        s_parameter_noise_assertions:
          - name: nf_limit
            metric: noise_figure_db_max
            relation: below
            threshold: 2.5
          - name: nfmin_limit
            metric: minimum_noise_figure_db_max
            relation: below
            threshold: 2.0
          - name: rn_limit
            metric: equivalent_noise_resistance_ohm_max
            relation: below
            threshold: 10.0
          - name: gamma_opt_limit
            metric: optimum_source_reflection_magnitude_max
            relation: below
            threshold: 0.6
"#,
    );

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
    let s_parameters_path = artifact_path(&report, "s_parameters.csv");
    assert_csv_has_header(
        s_parameters_path,
        &["s11_mag_db", "s21_mag_db", "s12_mag_db", "s22_mag_db"],
    );
    let s_summary_path = artifact_path(&report, "s_parameter_summary.csv");
    assert_csv_has_header(
        s_summary_path,
        &[
            "min_return_loss_db",
            "max_return_loss_db",
            "min_vswr",
            "max_vswr",
        ],
    );
    let network_summary_path = artifact_path(&report, "s_parameter_network_summary.csv");
    assert_csv_has_header(
        network_summary_path,
        &[
            "max_passivity_singular_value",
            "frequency_hz_at_max_passivity",
        ],
    );
    let summary_path = artifact_path(&report, "s_parameter_noise_summary.csv");
    assert_csv_has_header(
        summary_path,
        &[
            "max_noise_figure_db",
            "max_minimum_noise_figure_db",
            "max_equivalent_noise_resistance_ohm",
            "max_optimum_source_reflection_magnitude",
        ],
    );
    let summary = fs::read_to_string(summary_path).unwrap();
    assert!(summary.contains("3.000000000000e0,1.000000000000e9"));
    assert!(summary.contains("5.000000000000e-1,1.000000000000e6"));
    let failures = report["failures"].as_array().unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0]["measured"]["assertion"], "nf_limit");
    assert_eq!(failures[0]["measured"]["metric"], "noise_figure_db_max");
    assert_eq!(failures[0]["measured"]["measured"], 3.0);
    assert!(
        failures[0]["measured"]["s_parameter_noise_summary"]
            .as_str()
            .unwrap()
            .ends_with("s_parameter_noise_summary.csv")
    );
    let manifest_path = artifact_path(&report, "solver_manifest.json");
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["analysis"]["kind"], "s_parameter_noise");
    assert_eq!(manifest["outputs"]["normalized"][0]["kind"], "s_parameters");
    assert_eq!(
        manifest["outputs"]["normalized"][1]["kind"],
        "s_parameter_noise_summary"
    );
    let waveforms = report["waveforms"].as_array().unwrap();
    assert!(
        waveforms
            .iter()
            .any(|path| path.as_str().unwrap().ends_with("s_parameters.csv"))
    );
    let s_summaries = report["s_parameter_summaries"].as_array().unwrap();
    assert!(s_summaries.iter().any(|row| row["parameter"] == "s11"));
    let network_summaries = report["s_parameter_network_summaries"].as_array().unwrap();
    assert_eq!(network_summaries.len(), 1);
    let summaries = report["s_parameter_noise_summaries"].as_array().unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0]["row_count"], 2);
    assert_eq!(summaries[0]["max_noise_figure_db"], 3.0);
    assert_eq!(summaries[0]["max_equivalent_noise_resistance_ohm"], 6.0);
    assert_eq!(summaries[0]["max_optimum_source_reflection_magnitude"], 0.5);
    let markdown = fs::read_to_string(out_dir.path().join("report.md")).unwrap();
    assert!(markdown.contains("## S-Parameter Noise Summary"));
    assert!(markdown.contains("noise_figure_db_max=3.000000e0"));
    assert!(markdown.contains("optimum_source_reflection_magnitude_max=5.000000e-1"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_network_assertion_fails_on_rollet_k_limit() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.9 0.0 2.0 0.0 0.5 0.0 0.9 0.0\\n1.0e9 0.8 0.0 1.5 0.0 0.4 0.0 0.8 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_network_assertions:
          - name: active_stability_margin
            metric: rollet_k_min
            relation: above
            threshold: 1.0
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "active_stability_margin")
        .expect("Rollet K assertion failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(failure["measured"]["metric"], "rollet_k_min");
    assert!(failure["measured"]["value"].as_f64().unwrap() < 1.0);
    assert_eq!(failure["limit"]["above_threshold"], 1.0);
    let summaries = report["s_parameter_network_summaries"].as_array().unwrap();
    assert!(summaries[0]["min_rollet_k"].as_f64().unwrap() < 1.0);
    assert!(
        summaries[0]["max_stability_delta_magnitude"]
            .as_f64()
            .unwrap()
            < 1.0
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_network_assertion_fails_on_maximum_available_gain_limit() {
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
          - name: available_gain_floor
            metric: maximum_available_gain_db_min
            relation: above
            threshold: -1.0
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "available_gain_floor")
        .expect("maximum available gain assertion failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(
        failure["measured"]["metric"],
        "maximum_available_gain_db_min"
    );
    assert_eq!(failure["measured"]["unit"], "dB");
    assert!(failure["measured"]["value"].as_f64().unwrap() < -1.0);
    assert_eq!(failure["limit"]["above_threshold"], -1.0);
    let summaries = report["s_parameter_network_summaries"].as_array().unwrap();
    assert!(
        summaries[0]["min_maximum_available_gain_db"]
            .as_f64()
            .unwrap()
            < -1.0
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_network_assertion_fails_when_maximum_available_gain_unavailable() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 0.1 0.0 1.0 0.0 0.0 0.0 0.1 0.0\\n1.0e9 0.05 0.0 1.0 0.0 0.0 0.0 0.05 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_network_assertions:
          - name: available_gain_floor
            metric: maximum_available_gain_db_min
            relation: above
            threshold: 0.0
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "available_gain_floor")
        .expect("maximum available gain unavailable failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(
        failure["measured"]["metric"],
        "maximum_available_gain_db_min"
    );
    assert_eq!(
        failure["limit"]["required_metric"],
        "maximum_available_gain_db_min"
    );
    let summaries = report["s_parameter_network_summaries"].as_array().unwrap();
    assert!(summaries[0]["min_maximum_available_gain_db"].is_null());
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_network_assertion_fails_on_maximum_unilateral_gain_limit() {
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
          - name: unilateral_gain_floor
            metric: maximum_unilateral_gain_db_min
            relation: above
            threshold: -1.0
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "unilateral_gain_floor")
        .expect("maximum unilateral gain assertion failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(
        failure["measured"]["metric"],
        "maximum_unilateral_gain_db_min"
    );
    assert_eq!(failure["measured"]["unit"], "dB");
    assert!(failure["measured"]["value"].as_f64().unwrap() < -1.0);
    assert_eq!(failure["limit"]["above_threshold"], -1.0);
    let summaries = report["s_parameter_network_summaries"].as_array().unwrap();
    assert!(
        summaries[0]["min_maximum_unilateral_gain_db"]
            .as_f64()
            .unwrap()
            < -1.0
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_network_assertion_fails_when_maximum_unilateral_gain_unavailable() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '# Hz S RI R 50\\n1.0e6 1.0 0.0 0.8 0.0 0.8001 0.0 0.1 0.0\\n1.0e9 1.0 0.0 0.7 0.0 0.70005 0.0 0.05 0.0\\n' > s_parameters_raw.s2p\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "port1",
        r#"        s_parameter_network_assertions:
          - name: unilateral_gain_floor
            metric: maximum_unilateral_gain_db_min
            relation: above
            threshold: 0.0
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "unilateral_gain_floor")
        .expect("maximum unilateral gain unavailable failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(
        failure["measured"]["metric"],
        "maximum_unilateral_gain_db_min"
    );
    assert_eq!(
        failure["limit"]["required_metric"],
        "maximum_unilateral_gain_db_min"
    );
    let summaries = report["s_parameter_network_summaries"].as_array().unwrap();
    assert!(summaries[0]["min_maximum_unilateral_gain_db"].is_null());
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_network_assertion_fails_on_transducer_gain_limit() {
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
        r#"        s_parameter_source_reflection: { real: 0.2, imaginary: 0.0 }
        s_parameter_load_reflection: { real: 0.1, imaginary: 0.0 }
        s_parameter_network_assertions:
          - name: transducer_gain_floor
            metric: transducer_gain_db_min
            relation: above
            threshold: -1.0
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "transducer_gain_floor")
        .expect("transducer gain assertion failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(failure["measured"]["metric"], "transducer_gain_db_min");
    assert_eq!(failure["measured"]["unit"], "dB");
    assert!(failure["measured"]["value"].as_f64().unwrap() < -1.0);
    assert_eq!(failure["limit"]["above_threshold"], -1.0);
    let summaries = report["s_parameter_network_summaries"].as_array().unwrap();
    assert!(summaries[0]["min_transducer_gain_db"].as_f64().unwrap() < -1.0);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sparameter_network_assertion_fails_when_available_gain_lacks_source_reflection() {
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
          - name: source_available_gain_floor
            metric: available_gain_db_min
            relation: above
            threshold: 0.0
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"]["assertion"] == "source_available_gain_floor")
        .expect("available gain unavailable failure");
    assert_eq!(failure["id"], "SPICE_S_PARAMETER_ANALYSIS");
    assert_eq!(failure["measured"]["metric"], "available_gain_db_min");
    assert_eq!(failure["limit"]["required_metric"], "available_gain_db_min");
    let summaries = report["s_parameter_network_summaries"].as_array().unwrap();
    assert!(summaries[0]["source_reflection_real"].is_null());
    assert!(summaries[0]["min_available_gain_db"].is_null());
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_ngspice_sparameter_conformance_normalizes_smatrix_when_enabled() {
    if !real_ngspice_sparameter_conformance_enabled() {
        return;
    }
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project(project_dir.path(), "ngspice", "port1");
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
            "reference_impedance_ohm",
            "s11_mag_db",
            "s11_phase_deg",
            "s11_mag_linear",
            "s21_mag_db",
            "s21_phase_deg",
            "s21_mag_linear",
        ],
    );
    assert_csv_has_header(
        artifact_path(&report, "s_parameter_summary.csv"),
        &["parameter", "min_mag_db", "max_mag_linear"],
    );
    artifact_path(&report, "s_parameters_raw.csv");
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(manifest["analysis"]["kind"], "s_parameter");
    assert_eq!(
        manifest["outputs"]["raw"][0]["kind"],
        "ngspice_s_parameters_raw"
    );
    assert_eq!(manifest["outputs"]["normalized"][0]["kind"], "s_parameters");
    assert_eq!(
        manifest["outputs"]["normalized"].as_array().unwrap().len(),
        1
    );
    assert!(
        report["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|artifact| artifact.as_str())
            .all(|artifact| !artifact.ends_with("s_parameter_noise_summary.csv"))
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_ngspice_sparameter_noise_conformance_when_enabled() {
    if !real_ngspice_sparameter_conformance_enabled() {
        return;
    }
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sparameter_project_with_analysis_extra(
        project_dir.path(),
        "ngspice",
        "port1",
        r#"        s_parameter_assertions:
          - name: s11_magnitude_available
            parameter: s11
            metric: magnitude_linear
            aggregation: max
            relation: below
            threshold: 2.0
        s_parameter_network_assertions:
          - name: passive_network_available
            metric: passivity_max_singular_value
            relation: below
            threshold: 2.0
        s_parameter_noise_assertions:
          - name: nf_available
            metric: noise_figure_db_max
            relation: below
            threshold: 1000000.0
          - name: nfmin_available
            metric: minimum_noise_figure_db_max
            relation: below
            threshold: 1000000.0
          - name: rn_available
            metric: equivalent_noise_resistance_ohm_max
            relation: below
            threshold: 1000000000000.0
          - name: gamma_opt_available
            metric: optimum_source_reflection_magnitude_max
            relation: below
            threshold: 1000000.0
"#,
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
        &["s11_mag_db", "s21_mag_db", "s12_mag_db", "s22_mag_db"],
    );
    assert_csv_has_header(
        artifact_path(&report, "s_parameter_summary.csv"),
        &["parameter", "min_mag_linear", "max_mag_linear"],
    );
    assert_csv_has_header(
        artifact_path(&report, "s_parameter_network_summary.csv"),
        &["max_passivity_singular_value"],
    );
    assert_csv_has_header(
        artifact_path(&report, "s_parameter_noise_raw.csv"),
        &["nf", "nfmin", "rn", "sopt"],
    );
    assert_csv_has_header(
        artifact_path(&report, "s_parameter_noise_summary.csv"),
        &[
            "max_noise_figure_db",
            "max_minimum_noise_figure_db",
            "max_equivalent_noise_resistance_ohm",
            "max_optimum_source_reflection_magnitude",
        ],
    );
    let summaries = report["s_parameter_noise_summaries"].as_array().unwrap();
    assert_eq!(summaries.len(), 1);
    assert!(summaries[0]["row_count"].as_u64().unwrap() > 0);
    assert!(
        summaries[0]["max_noise_figure_db"]
            .as_f64()
            .unwrap()
            .is_finite()
    );
    assert!(
        summaries[0]["max_minimum_noise_figure_db"]
            .as_f64()
            .unwrap()
            .is_finite()
    );
    assert!(
        summaries[0]["max_equivalent_noise_resistance_ohm"]
            .as_f64()
            .unwrap()
            .is_finite()
    );
    assert!(
        summaries[0]["max_optimum_source_reflection_magnitude"]
            .as_f64()
            .unwrap()
            .is_finite()
    );
    let s_summaries = report["s_parameter_summaries"].as_array().unwrap();
    assert!(s_summaries.iter().any(|row| row["parameter"] == "s11"));
    let network_summaries = report["s_parameter_network_summaries"].as_array().unwrap();
    assert_eq!(network_summaries.len(), 1);
    assert!(
        network_summaries[0]["max_passivity_singular_value"]
            .as_f64()
            .unwrap()
            < 2.0
    );
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(manifest["analysis"]["kind"], "s_parameter_noise");
    assert_eq!(
        manifest["outputs"]["raw"][0]["kind"],
        "ngspice_s_parameters_raw"
    );
    assert_eq!(
        manifest["outputs"]["raw"][1]["kind"],
        "ngspice_s_parameter_noise_raw"
    );
    assert_eq!(manifest["outputs"]["normalized"][0]["kind"], "s_parameters");
    assert_eq!(
        manifest["outputs"]["normalized"][1]["kind"],
        "s_parameter_noise_summary"
    );
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
