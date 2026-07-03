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

fn write_measure_project(
    dir: &std::path::Path,
    backend: &str,
    mode: &str,
    statement: &str,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let analysis_extra = if mode == "tran" {
        "        stop_time_us: 100.0\n        max_step_us: 0.1\n".to_string()
    } else {
        "        start_frequency_hz: 10.0\n        stop_frequency_hz: 100000.0\n        points_per_decade: 10\n".to_string()
    };
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: measure_contract, version: 0.1.0 }}
libraries:
  - {libs}
board:
  components:
    V1:
      model: generic.analog.pulse_voltage_source
      pins: {{ P: vin, N: gnd }}
      spice:
        primitive: pulse_voltage_source
        pulse:
          initial_v: 0.0
          pulsed_v: 1.0
          delay_us: 0.0
          rise_us: 0.1
          fall_us: 0.1
          width_us: 5.0
          period_us: 10.0
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
  - name: rc_measure
    type: analog_measure
    checks: [SPICE_MEASURE_ANALYSIS]
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
        type: measure
        measure_mode: {mode}
{analysis_extra}        measure_statements:
          - name: avg_out
            statement: "{statement}"
      stimuli:
        - {{ name: scalar_measure, description: Scalar measurement extraction. }}
      probes:
        - {{ name: out_measure, expression: V(out) }}
      assertions: []
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy()
        ),
    )
    .unwrap();
    project
}

fn write_measure_template_project(
    dir: &std::path::Path,
    backend: &str,
    mode: &str,
    template_yaml: &str,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let analysis_extra = if mode == "tran" {
        "        stop_time_us: 100.0\n        max_step_us: 0.1\n".to_string()
    } else {
        "        start_frequency_hz: 10.0\n        stop_frequency_hz: 100000.0\n        points_per_decade: 10\n".to_string()
    };
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: measure_template_contract, version: 0.1.0 }}
libraries:
  - {libs}
board:
  components:
    V1:
      model: generic.analog.pulse_voltage_source
      pins: {{ P: vin, N: gnd }}
      spice:
        primitive: pulse_voltage_source
        pulse:
          initial_v: 0.0
          pulsed_v: 1.0
          delay_us: 0.0
          rise_us: 0.1
          fall_us: 0.1
          width_us: 5.0
          period_us: 10.0
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
  - name: rc_measure_template
    type: analog_measure
    checks: [SPICE_MEASURE_ANALYSIS]
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
        type: measure
        measure_mode: {mode}
{analysis_extra}        measure_templates:
{template_yaml}
      stimuli: []
      probes:
        - {{ name: out_measure, expression: V(out) }}
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
            "skipping real-ngspice measure conformance; set {REAL_NGSPICE_CONFORMANCE_ENV}=1"
        );
        return false;
    }
    if !binary_available("ngspice") {
        eprintln!("skipping real-ngspice measure conformance; ngspice is not on PATH");
        return false;
    }
    true
}

#[cfg(unix)]
fn real_xyce_measure_conformance_enabled() -> bool {
    if std::env::var(REAL_XYCE_CONFORMANCE_ENV).as_deref() != Ok("1") {
        eprintln!("skipping real-Xyce measure conformance; set {REAL_XYCE_CONFORMANCE_ENV}=1");
        return false;
    }
    if !binary_available("Xyce") && !binary_available("xyce") {
        eprintln!("skipping real-Xyce measure conformance; Xyce/xyce is not on PATH");
        return false;
    }
    true
}

#[cfg(unix)]
#[test]
fn measure_backend_normalizes_summary_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '%s\\n' 'avg_out = 5.10001e-01 from= 2.00000e-05 to= 1.00000e-04'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_project(
        project_dir.path(),
        "ngspice",
        "tran",
        "meas tran avg_out AVG v(out) FROM=20u TO=100u",
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
    let summary = fs::read_to_string(artifact_path(&report, "measure_summary.csv")).unwrap();
    assert!(summary.contains("measurement,mode,value,raw_line"));
    assert!(summary.contains("avg_out,tran,5.100010000000e-1"));
    assert!(artifact_path(&report, "measure_raw.txt").ends_with("measure_raw.txt"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(manifest["analysis"]["kind"], "measure");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "measure_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_template_backend_generates_statement_and_normalizes_summary() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '%s\\n' 'avg_out = 5.10001e-01 from= 2.00000e-05 to= 1.00000e-04'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "ngspice",
        "tran",
        "          - name: avg_out\n            operation: avg\n            expression: v(out)\n            from_us: 20.0\n            to_us: 100.0\n",
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
    let summary = fs::read_to_string(artifact_path(&report, "measure_summary.csv")).unwrap();
    assert!(summary.contains("avg_out,tran,5.100010000000e-1"));
    let wrapper =
        fs::read_to_string(artifact_path(&report, "circuitci_ngspice_measure.cir")).unwrap();
    assert!(
        wrapper
            .contains("meas tran avg_out AVG v(out) FROM=2.000000000000e-5 TO=1.000000000000e-4")
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_delay_template_generates_trig_targ_statement() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '%s\\n' 'prop_delay = 1.23400e-06 targ= 2.00000e-06 trig= 7.66000e-07'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "ngspice",
        "tran",
        "          - name: prop_delay\n            operation: delay\n            expression: v(out)\n            trigger_expression: v(vin)\n            trigger_value: 0.5\n            target_value: 0.5\n            trigger_edge: rise\n            target_edge: rise\n            trigger_count: 1\n            target_count: 1\n",
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
    let summary = fs::read_to_string(artifact_path(&report, "measure_summary.csv")).unwrap();
    assert!(summary.contains("prop_delay,tran,1.234000000000e-6"));
    let wrapper =
        fs::read_to_string(artifact_path(&report, "circuitci_ngspice_measure.cir")).unwrap();
    assert!(
        wrapper.contains(
            "meas tran prop_delay TRIG v(vin) VAL=5.000000000000e-1 RISE=1 TARG v(out) VAL=5.000000000000e-1 RISE=1"
        )
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_slew_and_threshold_templates_generate_portable_statements() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '%s\\n' 'out_slew = 8.00000e-07 targ= 1.80000e-06 trig= 1.00000e-06' 'out_rise_time = 1.50000e-06'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "ngspice",
        "tran",
        "          - name: out_slew\n            operation: slew\n            expression: v(out)\n            trigger_value: 0.2\n            target_value: 0.8\n            trigger_edge: rise\n            target_edge: rise\n          - name: out_rise_time\n            operation: threshold_time\n            expression: v(out)\n            target_value: 0.5\n            target_edge: rise\n            target_count: 1\n            from_us: 1.0\n            to_us: 20.0\n",
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
    let summary = fs::read_to_string(artifact_path(&report, "measure_summary.csv")).unwrap();
    assert!(summary.contains("out_slew,tran,8.000000000000e-7"));
    assert!(summary.contains("out_rise_time,tran,1.500000000000e-6"));
    let wrapper =
        fs::read_to_string(artifact_path(&report, "circuitci_ngspice_measure.cir")).unwrap();
    assert!(
        wrapper.contains(
            "meas tran out_slew TRIG v(out) VAL=2.000000000000e-1 RISE=1 TARG v(out) VAL=8.000000000000e-1 RISE=1"
        )
    );
    assert!(
        wrapper.contains(
            "meas tran out_rise_time WHEN v(out)=5.000000000000e-1 FROM=1.000000000000e-6 TO=2.000000000000e-5 RISE=1"
        )
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_ac_mode_accepts_frequency_domain_statement() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '%s\\n' 'avg_out = -3.01030e+00 at= 1.00000e+03'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_project(
        project_dir.path(),
        "ngspice",
        "ac",
        "meas ac avg_out FIND vdb(out) AT=1k",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_ngspice_measure_conformance_when_enabled() {
    if !real_ngspice_conformance_enabled() {
        return;
    }

    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_project(
        project_dir.path(),
        "ngspice",
        "tran",
        "meas tran avg_out AVG v(out) FROM=20u TO=100u",
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
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();

    assert_eq!(report["result"], "pass");
    let summary = fs::read_to_string(artifact_path(&report, "measure_summary.csv")).unwrap();
    assert!(summary.contains("avg_out,tran,"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_xyce_backend_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_project(
        project_dir.path(),
        "xyce",
        "tran",
        "meas tran avg_out AVG v(out) FROM=20u TO=100u",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_MEASURE_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["adapter_status"],
        "planned_not_implemented"
    );
    assert_eq!(
        report["failures"][0]["limit"]["implemented_backend"],
        "ngspice_or_xyce_templates"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "measure_templates_or_ngspice_raw_measure_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_xyce_template_backend_normalizes_summary_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\n{ printf '%s\\n' 'INDEX avg_out'; printf '%s\\n' '0 5.10001e-01'; } > circuitci_xyce_measure.mt0\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "xyce",
        "tran",
        "          - name: avg_out\n            operation: avg\n            expression: v(out)\n            from_us: 20.0\n            to_us: 100.0\n",
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
    let summary = fs::read_to_string(artifact_path(&report, "measure_summary.csv")).unwrap();
    assert!(summary.contains("avg_out,tran,5.100010000000e-1"));
    assert!(artifact_path(&report, "circuitci_xyce_measure.mt0").ends_with(".mt0"));
    let wrapper = fs::read_to_string(artifact_path(&report, "circuitci_xyce_measure.cir")).unwrap();
    assert!(
        wrapper.contains(
            ".MEASURE TRAN avg_out AVG v(out) FROM=2.000000000000e-5 TO=1.000000000000e-4"
        )
    );
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["analysis"]["kind"], "measure");
    assert_eq!(manifest["outputs"]["raw"][0]["kind"], "xyce_measure_stdout");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "measure_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_xyce_delay_template_normalizes_summary() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\n{ printf '%s\\n' 'INDEX prop_delay'; printf '%s\\n' '0 1.23400e-06'; } > circuitci_xyce_measure.mt0\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "xyce",
        "tran",
        "          - name: prop_delay\n            operation: delay\n            expression: v(out)\n            trigger_expression: v(vin)\n            trigger_value: 0.5\n            target_value: 0.5\n            trigger_edge: rise\n            target_edge: rise\n",
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
    let summary = fs::read_to_string(artifact_path(&report, "measure_summary.csv")).unwrap();
    assert!(summary.contains("prop_delay,tran,1.234000000000e-6"));
    let wrapper = fs::read_to_string(artifact_path(&report, "circuitci_xyce_measure.cir")).unwrap();
    assert!(
        wrapper.contains(
            ".MEASURE TRAN prop_delay TRIG v(vin) VAL=5.000000000000e-1 RISE=1 TARG v(out) VAL=5.000000000000e-1 RISE=1"
        )
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_xyce_slew_and_threshold_templates_normalize_summary() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\n{ printf '%s\\n' 'INDEX out_slew out_rise_time'; printf '%s\\n' '0 8.00000e-07 1.50000e-06'; } > circuitci_xyce_measure.mt0\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "xyce",
        "tran",
        "          - name: out_slew\n            operation: slew\n            expression: v(out)\n            trigger_value: 0.2\n            target_value: 0.8\n            trigger_edge: rise\n            target_edge: rise\n          - name: out_rise_time\n            operation: threshold_time\n            expression: v(out)\n            target_value: 0.5\n            target_edge: rise\n            target_count: 1\n            from_us: 1.0\n            to_us: 20.0\n",
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
    let summary = fs::read_to_string(artifact_path(&report, "measure_summary.csv")).unwrap();
    assert!(summary.contains("out_slew,tran,8.000000000000e-7"));
    assert!(summary.contains("out_rise_time,tran,1.500000000000e-6"));
    let wrapper = fs::read_to_string(artifact_path(&report, "circuitci_xyce_measure.cir")).unwrap();
    assert!(
        wrapper.contains(
            ".MEASURE TRAN out_slew TRIG v(out) VAL=2.000000000000e-1 RISE=1 TARG v(out) VAL=8.000000000000e-1 RISE=1"
        )
    );
    assert!(
        wrapper.contains(
            ".MEASURE TRAN out_rise_time WHEN v(out)=5.000000000000e-1 FROM=1.000000000000e-6 TO=2.000000000000e-5 RISE=1"
        )
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_xyce_measure_template_conformance_when_enabled() {
    if !real_xyce_measure_conformance_enabled() {
        return;
    }

    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "xyce",
        "tran",
        "          - name: avg_out\n            operation: avg\n            expression: v(out)\n            from_us: 20.0\n            to_us: 100.0\n",
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
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();

    assert_eq!(report["result"], "pass");
    let summary = fs::read_to_string(artifact_path(&report, "measure_summary.csv")).unwrap();
    assert!(summary.contains("measurement,mode,value,raw_line"));
    assert!(summary.contains("avg_out,tran,"));
    let raw = fs::read_to_string(artifact_path(&report, "measure_raw.txt")).unwrap();
    assert!(raw.contains("avg_out"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert!(manifest["backend"]["selected"] == "Xyce" || manifest["backend"]["selected"] == "xyce");
    assert_eq!(manifest["analysis"]["kind"], "measure");
    assert_eq!(manifest["outputs"]["raw"][0]["kind"], "xyce_measure_stdout");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "measure_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_xyce_template_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "xyce",
        "tran",
        "          - name: avg_out\n            operation: avg\n            expression: v(out)\n            from_us: 20.0\n            to_us: 100.0\n",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_MEASURE_ANALYSIS");
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "xyce_measure_summary_csv"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("circuitci_xyce_measure.cir")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| { artifact.as_str().unwrap().ends_with("xyce_measure.log") })
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_rejects_unbound_statement_node() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_project(
        project_dir.path(),
        "ngspice",
        "tran",
        "meas tran avg_out AVG v(missing) FROM=20u TO=100u",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("unbound node missing")
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_template_rejects_unbound_expression_node() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "ngspice",
        "tran",
        "          - name: avg_out\n            operation: avg\n            expression: v(missing)\n            from_us: 20.0\n            to_us: 100.0\n",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("unbound node missing")
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_delay_template_rejects_unbound_trigger_node() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "ngspice",
        "tran",
        "          - name: prop_delay\n            operation: delay\n            expression: v(out)\n            trigger_expression: v(missing)\n            trigger_value: 0.5\n            target_value: 0.5\n            trigger_edge: rise\n            target_edge: rise\n",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("unbound node missing")
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_template_requires_find_location() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_template_project(
        project_dir.path(),
        "ngspice",
        "tran",
        "          - name: find_out\n            operation: find\n            expression: v(out)\n",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("requires at_us or at_hz")
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn measure_backend_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_measure_project(
        project_dir.path(),
        "ngspice",
        "tran",
        "meas tran avg_out AVG v(out) FROM=20u TO=100u",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_MEASURE_ANALYSIS");
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "ngspice_measure_summary_csv"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("circuitci_ngspice_measure.cir")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| { artifact.as_str().unwrap().ends_with("ngspice_measure.log") })
    );
    assert_report_schema_valid(&report);
}
