mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, binary_available};
use serde_json::Value;
use std::fs;
use std::process::Command;

#[cfg(unix)]
const REAL_NGSPICE_CONFORMANCE_ENV: &str = "CIRCUITCI_RUN_REAL_NGSPICE";

#[cfg(unix)]
const REAL_XYCE_CONFORMANCE_ENV: &str = "CIRCUITCI_RUN_REAL_XYCE";

#[cfg(unix)]
fn fake_executable_with_body(dir: &std::path::Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
}

fn write_dc_sweep_project(
    dir: &std::path::Path,
    backend: &str,
    assertion_threshold: f64,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: dc_sweep_contract, version: 0.1.0 }}
libraries:
  - {libs}
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins: {{ P: vin, N: gnd }}
      spice: {{ primitive: dc_voltage_source, dc_v: 0.0 }}
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
  - name: divider_dc_sweep
    type: analog_dc_sweep
    checks: [SPICE_DC_SWEEP_ANALYSIS]
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
        type: dc_sweep
        dc_sweep_source: V1
        dc_sweep_start: 0.0
        dc_sweep_stop: 1.0
        dc_sweep_step: 0.5
        dc_sweep_assertions:
          - name: out_max_below_limit
            probe: out_voltage
            aggregation: max
            relation: below
            threshold: {assertion_threshold}
            unit: V
      stimuli:
        - {{ name: input_sweep, description: DC input source sweep. }}
      probes:
        - {{ name: out_voltage, expression: V(out), quantity: voltage }}
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
            "skipping real-ngspice DC sweep conformance; set {REAL_NGSPICE_CONFORMANCE_ENV}=1"
        );
        return false;
    }
    if !binary_available("ngspice") {
        eprintln!("skipping real-ngspice DC sweep conformance; ngspice is not on PATH");
        return false;
    }
    true
}

#[cfg(unix)]
fn real_xyce_conformance_enabled() -> bool {
    if std::env::var(REAL_XYCE_CONFORMANCE_ENV).as_deref() != Ok("1") {
        eprintln!("skipping real-Xyce DC sweep conformance; set {REAL_XYCE_CONFORMANCE_ENV}=1");
        return false;
    }
    if !binary_available("Xyce") && !binary_available("xyce") {
        eprintln!("skipping real-Xyce DC sweep conformance; Xyce/xyce is not on PATH");
        return false;
    }
    true
}

#[cfg(unix)]
#[test]
fn dc_sweep_backend_normalizes_curve_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\n/bin/cat > dc_sweep_raw.csv <<'EOF'\nIndex out\n0.0 0.0\n0.5 0.25\n1.0 0.5\nEOF\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_dc_sweep_project(project_dir.path(), "ngspice", 0.7);
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
    assert_report_schema_valid(&report);
    assert!(report["failures"].as_array().unwrap().is_empty());
    let sweep = artifact_path(&report, "dc_sweep.csv");
    let text = fs::read_to_string(sweep).unwrap();
    assert!(text.contains("sweep_source,sweep_value,probe,value"));
    assert!(text.contains("V1,1.000000000000e0,out_voltage,5.000000000000e-1"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["analysis"]["kind"], "dc_sweep");
    assert_eq!(manifest["outputs"]["normalized"][0]["kind"], "dc_sweep");
}

#[cfg(unix)]
#[test]
fn xyce_dc_sweep_backend_normalizes_curve_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\n/bin/cat > dc_sweep_raw.csv <<'EOF'\nIndex,out\n0.0,0.0\n0.5,0.25\n1.0,0.5\nEOF\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_dc_sweep_project(project_dir.path(), "xyce", 0.7);
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
    assert_report_schema_valid(&report);
    assert!(report["failures"].as_array().unwrap().is_empty());
    assert!(artifact_path(&report, "circuitci_xyce_dc_sweep.cir").ends_with(".cir"));
    let sweep = fs::read_to_string(artifact_path(&report, "dc_sweep.csv")).unwrap();
    assert!(sweep.contains("V1,1.000000000000e0,out_voltage,5.000000000000e-1"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["analysis"]["kind"], "dc_sweep");
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["outputs"]["raw"][0]["kind"], "xyce_dc_sweep_raw");
    assert_eq!(manifest["outputs"]["normalized"][0]["kind"], "dc_sweep");
}

#[cfg(unix)]
#[test]
fn auto_dc_sweep_backend_uses_xyce_when_ngspice_is_absent() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\n/bin/cat > dc_sweep_raw.csv <<'EOF'\nIndex,out\n0.0,0.0\n0.5,0.25\n1.0,0.5\nEOF\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_dc_sweep_project(project_dir.path(), "auto", 0.7);
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
    assert_report_schema_valid(&report);
    assert!(report["failures"].as_array().unwrap().is_empty());
    let sweep = fs::read_to_string(artifact_path(&report, "dc_sweep.csv")).unwrap();
    assert!(sweep.contains("V1,1.000000000000e0,out_voltage,5.000000000000e-1"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["requested"], "auto");
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["analysis"]["kind"], "dc_sweep");
    assert_eq!(manifest["outputs"]["raw"][0]["kind"], "xyce_dc_sweep_raw");
    assert_eq!(manifest["outputs"]["normalized"][0]["kind"], "dc_sweep");
}

#[cfg(unix)]
#[test]
fn real_ngspice_dc_sweep_conformance_when_enabled() {
    if !real_ngspice_conformance_enabled() {
        return;
    }
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_dc_sweep_project(project_dir.path(), "ngspice", 0.7);
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
    assert_report_schema_valid(&report);
    assert!(report["failures"].as_array().unwrap().is_empty());
    let sweep = fs::read_to_string(artifact_path(&report, "dc_sweep.csv")).unwrap();
    assert!(sweep.contains("sweep_source,sweep_value,probe,value"));
    assert!(sweep.lines().skip(1).count() >= 3);
}

#[cfg(unix)]
#[test]
fn real_xyce_dc_sweep_conformance_when_enabled() {
    if !real_xyce_conformance_enabled() {
        return;
    }
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_dc_sweep_project(project_dir.path(), "xyce", 0.7);
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
    assert_report_schema_valid(&report);
    assert!(report["failures"].as_array().unwrap().is_empty());
    let sweep = fs::read_to_string(artifact_path(&report, "dc_sweep.csv")).unwrap();
    assert!(sweep.contains("sweep_source,sweep_value,probe,value"));
    assert!(sweep.lines().skip(1).count() >= 3);
}

#[cfg(unix)]
#[test]
fn dc_sweep_assertion_failure_reports_normalized_curve() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\n/bin/cat > dc_sweep_raw.csv <<'EOF'\nIndex out\n0.0 0.0\n0.5 0.25\n1.0 0.5\nEOF\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_dc_sweep_project(project_dir.path(), "ngspice", 0.4);
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
    assert_report_schema_valid(&report);
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SPICE_DC_SWEEP_ANALYSIS");
    assert_eq!(failure["measured"]["assertion"], "out_max_below_limit");
    assert_eq!(failure["measured"]["probe"], "out_voltage");
    assert!(
        failure["measured"]["dc_sweep"]
            .as_str()
            .unwrap()
            .ends_with("dc_sweep.csv")
    );
}
