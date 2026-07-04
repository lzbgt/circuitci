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

fn write_sensitivity_project(
    dir: &std::path::Path,
    backend: &str,
    output_expression: &str,
    mode: &str,
) -> std::path::PathBuf {
    write_sensitivity_project_with_analysis_extra(dir, backend, output_expression, mode, "")
}

fn write_sensitivity_project_with_analysis_extra(
    dir: &std::path::Path,
    backend: &str,
    output_expression: &str,
    mode: &str,
    analysis_extra: &str,
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
{analysis_extra}
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
fn real_ngspice_conformance_enabled() -> bool {
    if std::env::var(REAL_NGSPICE_CONFORMANCE_ENV).as_deref() != Ok("1") {
        eprintln!(
            "skipping real-ngspice sensitivity conformance; set {REAL_NGSPICE_CONFORMANCE_ENV}=1"
        );
        return false;
    }
    if !binary_available("ngspice") {
        eprintln!("skipping real-ngspice sensitivity conformance; ngspice is not on PATH");
        return false;
    }
    true
}

#[cfg(unix)]
fn real_xyce_sensitivity_conformance_enabled() -> bool {
    if std::env::var(REAL_XYCE_CONFORMANCE_ENV).as_deref() != Ok("1") {
        eprintln!("skipping real-Xyce sensitivity conformance; set {REAL_XYCE_CONFORMANCE_ENV}=1");
        return false;
    }
    if !binary_available("Xyce") && !binary_available("xyce") {
        eprintln!("skipping real-Xyce sensitivity conformance; Xyce/xyce is not on PATH");
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
        .unwrap_or_else(|| panic!("missing artifact with suffix {suffix}"))
}

fn assert_selected_xyce(manifest: &Value) {
    assert!(
        manifest["backend"]["selected"]
            .as_str()
            .unwrap()
            .eq_ignore_ascii_case("xyce")
    );
}

fn assert_sensitivity_summary_has_dc_rows(path: &str) {
    let text = fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    assert_eq!(
        lines.next().unwrap_or(""),
        "output_expression,mode,parameter,frequency_hz,sensitivity_real,sensitivity_imaginary,sensitivity_magnitude"
    );
    assert!(
        lines.any(|line| line.contains("V(out),dc,")),
        "{path} did not contain a DC sensitivity row"
    );
}

fn assert_sensitivity_summary_has_ac_rows(path: &str) {
    let text = fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    assert_eq!(
        lines.next().unwrap_or(""),
        "output_expression,mode,parameter,frequency_hz,sensitivity_real,sensitivity_imaginary,sensitivity_magnitude"
    );
    assert!(
        lines.any(|line| line.contains("V(out),ac,")),
        "{path} did not contain an AC sensitivity row"
    );
}

#[cfg(unix)]
#[test]
fn sensitivity_backend_normalizes_summary_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'r1 = -2.50000e-04\\nr2 = 2.499998e-04\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project(project_dir.path(), "ngspice", "V(out)", "dc");
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
    let summary_path = artifact_path(&report, "sensitivity_summary.csv");
    let summary = fs::read_to_string(summary_path).unwrap();
    assert!(summary.contains(
        "output_expression,mode,parameter,frequency_hz,sensitivity_real,sensitivity_imaginary,sensitivity_magnitude"
    ));
    assert!(summary.contains("V(out),dc,r1,,-2.500000000000e-4,0.000000000000e0"));
    assert!(summary.contains("V(out),dc,r2,,2.499998000000e-4,0.000000000000e0"));
    assert!(artifact_path(&report, "sensitivity_raw.txt").ends_with("sensitivity_raw.txt"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(manifest["analysis"]["kind"], "sensitivity");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "sensitivity_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_ngspice_sensitivity_conformance_when_enabled() {
    if !real_ngspice_conformance_enabled() {
        return;
    }

    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project(project_dir.path(), "ngspice", "V(out)", "dc");
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
    assert_sensitivity_summary_has_dc_rows(artifact_path(&report, "sensitivity_summary.csv"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "sensitivity_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_ngspice_ac_sensitivity_conformance_when_enabled() {
    if !real_ngspice_conformance_enabled() {
        return;
    }

    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project(project_dir.path(), "ngspice", "V(out)", "ac");
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
    assert_sensitivity_summary_has_ac_rows(artifact_path(&report, "sensitivity_summary.csv"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "sensitivity_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_xyce_sensitivity_conformance_when_enabled() {
    if !real_xyce_sensitivity_conformance_enabled() {
        return;
    }

    let project_root = tempfile::tempdir().unwrap();

    let dc_dir = project_root.path().join("dc");
    fs::create_dir_all(&dc_dir).unwrap();
    let dc_project = write_sensitivity_project(&dc_dir, "xyce", "V(out)", "dc");
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&dc_project, &validator);
    fs::create_dir_all("out").unwrap();
    let dc_out = tempfile::tempdir_in("out").unwrap();
    let dc_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            dc_project.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            dc_out.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(dc_status.success());
    let dc_report: Value =
        serde_json::from_str(&fs::read_to_string(dc_out.path().join("report.json")).unwrap())
            .unwrap();
    assert_eq!(dc_report["result"], "pass");
    assert_sensitivity_summary_has_dc_rows(artifact_path(&dc_report, "sensitivity_summary.csv"));
    let dc_manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&dc_report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_selected_xyce(&dc_manifest);
    assert_eq!(dc_manifest["analysis"]["kind"], "sensitivity");
    assert_eq!(
        dc_manifest["outputs"]["raw"][0]["kind"],
        "xyce_sensitivity_csv"
    );
    assert_report_schema_valid(&dc_report);

    let ac_dir = project_root.path().join("ac");
    fs::create_dir_all(&ac_dir).unwrap();
    let ac_project = write_sensitivity_project(&ac_dir, "xyce", "V(out)", "ac");
    assert_yaml_file_valid(&ac_project, &validator);
    let ac_out = tempfile::tempdir_in("out").unwrap();
    let ac_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            ac_project.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            ac_out.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(ac_status.success());
    let ac_report: Value =
        serde_json::from_str(&fs::read_to_string(ac_out.path().join("report.json")).unwrap())
            .unwrap();
    assert_eq!(ac_report["result"], "pass");
    assert_sensitivity_summary_has_ac_rows(artifact_path(&ac_report, "sensitivity_summary.csv"));
    let ac_manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&ac_report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_selected_xyce(&ac_manifest);
    assert_eq!(ac_manifest["analysis"]["kind"], "sensitivity");
    assert_eq!(
        ac_manifest["outputs"]["normalized"][0]["kind"],
        "sensitivity_summary"
    );
    assert_report_schema_valid(&ac_report);
}

#[cfg(unix)]
#[test]
fn ac_sensitivity_backend_reports_frequency_rows() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'Index   frequency       r1\\n0 1.000000e+02 -2.50000e-04, 1.000000e-06\\nIndex   frequency       r2\\n0 1.000000e+02 2.499998e-04, -0.000000e+00\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project(project_dir.path(), "ngspice", "V(out)", "ac");

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
    let summary = fs::read_to_string(artifact_path(&report, "sensitivity_summary.csv")).unwrap();
    assert!(summary.contains("V(out),ac,r1,1.000000000000e2,-2.500000000000e-4"));
    assert!(summary.contains("V(out),ac,r2,1.000000000000e2,2.499998000000e-4"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sensitivity_assertions_pass_on_dc_summary_rows() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'r1 = -2.50000e-04\\nr2 = 2.499998e-04\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project_with_analysis_extra(
        project_dir.path(),
        "ngspice",
        "V(out)",
        "dc",
        r#"
        sensitivity_assertions:
          - name: r1_sensitivity_below_limit
            parameter: r1
            metric: sensitivity_magnitude
            relation: below
            threshold: 1.0e-3
            unit: V/ohm"#,
    );
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_eq!(report["sensitivity_summaries"].as_array().unwrap().len(), 2);
    assert_eq!(report["sensitivity_summaries"][0]["parameter"], "r1");
    assert_eq!(
        report["sensitivity_summaries"][0]["frequency_hz"],
        Value::Null
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sensitivity_assertions_pass_on_ac_summary_rows() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'Index   frequency       r1\\n0 1.000000e+02 -2.50000e-04, 1.000000e-06\\nIndex   frequency       r2\\n0 1.000000e+02 2.499998e-04, -0.000000e+00\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project_with_analysis_extra(
        project_dir.path(),
        "ngspice",
        "V(out)",
        "ac",
        r#"
        sensitivity_assertions:
          - name: r1_real_below_zero
            parameter: r1
            frequency_hz: 100.0
            metric: sensitivity_real
            relation: below
            threshold: 0.0
            unit: V/ohm"#,
    );
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_eq!(report["sensitivity_summaries"][0]["frequency_hz"], 100.0);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sensitivity_assertion_fails_on_limit_violation() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'r1 = -2.50000e-04\\nr2 = 2.499998e-04\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project_with_analysis_extra(
        project_dir.path(),
        "ngspice",
        "V(out)",
        "dc",
        r#"
        sensitivity_assertions:
          - name: r1_sensitivity_too_high
            parameter: r1
            metric: sensitivity_magnitude
            relation: below
            threshold: 1.0e-5"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_SENSITIVITY_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["assertion"],
        "r1_sensitivity_too_high"
    );
    assert_eq!(
        report["failures"][0]["measured"]["metric"],
        "sensitivity_magnitude"
    );
    assert_eq!(report["failures"][0]["limit"]["below_threshold"], 1.0e-5);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sensitivity_assertion_fails_closed_on_missing_parameter() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'r1 = -2.50000e-04\\nr2 = 2.499998e-04\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project_with_analysis_extra(
        project_dir.path(),
        "ngspice",
        "V(out)",
        "dc",
        r#"
        sensitivity_assertions:
          - name: missing_parameter
            parameter: r3
            metric: sensitivity_magnitude
            relation: below
            threshold: 1.0"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_SENSITIVITY_ANALYSIS");
    assert_eq!(report["failures"][0]["limit"]["required_parameter"], "r3");
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn ac_sensitivity_assertion_requires_frequency_when_parameter_is_ambiguous() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'Index   frequency       r1\\n0 1.000000e+02 -2.50000e-04, 1.000000e-06\\n1 1.000000e+03 -1.50000e-04, 2.000000e-06\\n'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project_with_analysis_extra(
        project_dir.path(),
        "ngspice",
        "V(out)",
        "ac",
        r#"
        sensitivity_assertions:
          - name: ambiguous_r1
            parameter: r1
            metric: sensitivity_magnitude
            relation: below
            threshold: 1.0"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_SENSITIVITY_ANALYSIS");
    assert_eq!(
        report["failures"][0]["limit"]["required_field"],
        "frequency_hz"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sensitivity_xyce_backend_normalizes_summary_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '%s\\n' 'Index,{V(out)},d_{V(out)}/d_R1:R_dir,d_{V(out)}/d_R2:R_dir' '0,5.000000e-01,-2.500000e-04,2.500000e-04' > sensitivity_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project_with_analysis_extra(
        project_dir.path(),
        "xyce",
        "V(out)",
        "dc",
        r#"
        sensitivity_assertions:
          - name: r1_sensitivity_below_limit
            parameter: R1
            metric: sensitivity_magnitude
            relation: below
            threshold: 1.0e-3"#,
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
    let wrapper = fs::read_to_string(artifact_path(&report, "circuitci_xyce_sens.cir")).unwrap();
    assert!(wrapper.contains(".SENS objfunc={V(out)} param=R1:R,R2:R"));
    assert!(wrapper.contains(".PRINT SENS FORMAT=CSV sensitivity_raw.csv"));
    let summary = fs::read_to_string(artifact_path(&report, "sensitivity_summary.csv")).unwrap();
    assert!(summary.contains("V(out),dc,R1,,-2.500000000000e-4,0.000000000000e0"));
    assert!(summary.contains("V(out),dc,R2,,2.500000000000e-4,0.000000000000e0"));
    assert!(artifact_path(&report, "sensitivity_raw.csv").ends_with("sensitivity_raw.csv"));
    assert_eq!(report["sensitivity_summaries"].as_array().unwrap().len(), 2);
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["analysis"]["kind"], "sensitivity");
    assert_eq!(
        manifest["outputs"]["raw"][0]["kind"],
        "xyce_sensitivity_csv"
    );
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "sensitivity_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn sensitivity_backend_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_sensitivity_project(project_dir.path(), "ngspice", "V(out)", "dc");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_SENSITIVITY_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["selected_backend"],
        "ngspice"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "ngspice_sensitivity_summary_csv"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("circuitci_ngspice_sens.cir")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("ngspice_sens.log"))
    );
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
