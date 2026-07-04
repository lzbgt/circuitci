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

fn write_fourier_project(
    dir: &std::path::Path,
    backend: &str,
    output_expression: &str,
    stop_time_us: f64,
    fundamental_frequency_hz: f64,
    fourier_assertions: &str,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: fourier_contract, version: 0.1.0 }}
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
  - name: rc_fourier
    type: analog_fourier
    checks: [SPICE_FOURIER_ANALYSIS]
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
        type: fourier
        stop_time_us: {stop_time_us}
        max_step_us: 0.1
        fourier_fundamental_frequency_hz: {fundamental_frequency_hz}
        fourier_output_expression: {output_expression}
        fourier_harmonics: 10
{fourier_assertions}      stimuli:
        - {{ name: square_wave, description: RC pulse response harmonic extraction. }}
      probes:
        - {{ name: out_fourier, expression: V(out) }}
      assertions: []
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy(),
            fourier_assertions = fourier_assertions,
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
            "skipping real-ngspice Fourier conformance; set {REAL_NGSPICE_CONFORMANCE_ENV}=1"
        );
        return false;
    }
    if !binary_available("ngspice") {
        eprintln!("skipping real-ngspice Fourier conformance; ngspice is not on PATH");
        return false;
    }
    true
}

fn assert_fourier_summary_has_fundamental(path: &str) {
    let text = fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    let header = lines.next().unwrap_or("");
    assert_eq!(
        header,
        "output_expression,fundamental_frequency_hz,reported_harmonics,harmonic,frequency_hz,magnitude,phase_deg,normalized_magnitude,normalized_phase_deg,thd_percent,grid_size,interpolation_degree,periods"
    );
    assert!(
        lines.any(|line| line.contains(",1,1.000000000000e5,")),
        "{path} did not contain a fundamental harmonic row"
    );
}

#[cfg(unix)]
#[test]
fn fourier_backend_normalizes_summary_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '%s\\n' 'Fourier analysis for v(out):' '  No. Harmonics: 5, THD: 18.5435 %, Gridsize: 200, Interpolation Degree: 1, No. Periods: 1' '' 'Harmonic Frequency   Magnitude   Phase       Norm. Mag   Norm. Phase' '-------- ---------   ---------   -----       ---------   -----------' ' 0       0           0.509986    0           0           0' ' 1       100000      0.538779    -35.733     1           0' ' 2       200000      0.0124232   31.3212     0.0230581   67.0541'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_fourier_project(
        project_dir.path(),
        "ngspice",
        "V(out)",
        100.0,
        100_000.0,
        "",
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
    let summary_path = artifact_path(&report, "fourier_summary.csv");
    let summary = fs::read_to_string(summary_path).unwrap();
    assert!(summary.contains(
        "output_expression,fundamental_frequency_hz,reported_harmonics,harmonic,frequency_hz,magnitude,phase_deg,normalized_magnitude,normalized_phase_deg,thd_percent,grid_size,interpolation_degree,periods"
    ));
    assert!(summary.contains("V(out),1.000000000000e5,5,1,1.000000000000e5,5.387790000000e-1"));
    assert!(summary.contains(",1.854350000000e1,200,1,1"));
    let fourier_summaries = report["fourier_summaries"].as_array().unwrap();
    assert_eq!(fourier_summaries.len(), 3);
    assert_eq!(fourier_summaries[0]["output_expression"], "V(out)");
    assert_eq!(fourier_summaries[1]["harmonic"], 1);
    assert_eq!(fourier_summaries[1]["magnitude"], 5.38779e-1);
    assert_eq!(fourier_summaries[2]["harmonic"], 2);
    assert_eq!(fourier_summaries[2]["normalized_magnitude"], 2.30581e-2);
    assert_eq!(fourier_summaries[2]["thd_percent"], 18.5435);
    assert!(artifact_path(&report, "fourier_raw.txt").ends_with("fourier_raw.txt"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(manifest["analysis"]["kind"], "fourier");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "fourier_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn fourier_assertions_pass_on_harmonic_and_thd_metrics() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '%s\\n' 'Fourier analysis for v(out):' '  No. Harmonics: 5, THD: 18.5435 %, Gridsize: 200, Interpolation Degree: 1, No. Periods: 1' '' 'Harmonic Frequency   Magnitude   Phase       Norm. Mag   Norm. Phase' '-------- ---------   ---------   -----       ---------   -----------' ' 0       0           0.509986    0           0           0' ' 1       100000      0.538779    -35.733     1           0' ' 2       200000      0.0124232   31.3212     0.0230581   67.0541'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_fourier_project(
        project_dir.path(),
        "ngspice",
        "V(out)",
        100.0,
        100_000.0,
        r#"        fourier_assertions:
          - name: second_harmonic_ratio_below_limit
            harmonic: 2
            metric: normalized_magnitude
            relation: below
            threshold: 0.03
            unit: ratio
          - name: thd_below_limit
            metric: thd_percent
            relation: below
            threshold: 20.0
            unit: percent
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
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn fourier_assertion_fails_on_normalized_harmonic_limit() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '%s\\n' 'Fourier analysis for v(out):' '  No. Harmonics: 5, THD: 18.5435 %, Gridsize: 200, Interpolation Degree: 1, No. Periods: 1' '' 'Harmonic Frequency   Magnitude   Phase       Norm. Mag   Norm. Phase' '-------- ---------   ---------   -----       ---------   -----------' ' 0       0           0.509986    0           0           0' ' 1       100000      0.538779    -35.733     1           0' ' 2       200000      0.0124232   31.3212     0.0230581   67.0541'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_fourier_project(
        project_dir.path(),
        "ngspice",
        "V(out)",
        100.0,
        100_000.0,
        r#"        fourier_assertions:
          - name: second_harmonic_ratio_below_limit
            harmonic: 2
            metric: normalized_magnitude
            relation: below
            threshold: 0.01
            unit: ratio
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

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_FOURIER_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["assertion"],
        "second_harmonic_ratio_below_limit"
    );
    assert_eq!(report["failures"][0]["measured"]["harmonic"], 2);
    assert_eq!(
        report["failures"][0]["measured"]["metric"],
        "normalized_magnitude"
    );
    assert_eq!(report["failures"][0]["measured"]["value"], 2.30581e-2);
    assert_eq!(report["failures"][0]["limit"]["below_threshold"], 0.01);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn fourier_assertion_fails_closed_on_missing_harmonic() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf '%s\\n' 'Fourier analysis for v(out):' '  No. Harmonics: 5, THD: 18.5435 %, Gridsize: 200, Interpolation Degree: 1, No. Periods: 1' '' 'Harmonic Frequency   Magnitude   Phase       Norm. Mag   Norm. Phase' '-------- ---------   ---------   -----       ---------   -----------' ' 0       0           0.509986    0           0           0' ' 1       100000      0.538779    -35.733     1           0' ' 2       200000      0.0124232   31.3212     0.0230581   67.0541'\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_fourier_project(
        project_dir.path(),
        "ngspice",
        "V(out)",
        100.0,
        100_000.0,
        r#"        fourier_assertions:
          - name: tenth_harmonic_below_limit
            harmonic: 10
            metric: magnitude
            relation: below
            threshold: 0.001
"#,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_FOURIER_ANALYSIS");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("missing normalized harmonic 10")
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_ngspice_fourier_conformance_when_enabled() {
    if !real_ngspice_conformance_enabled() {
        return;
    }

    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_fourier_project(
        project_dir.path(),
        "ngspice",
        "V(out)",
        100.0,
        100_000.0,
        "",
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
    assert_fourier_summary_has_fundamental(artifact_path(&report, "fourier_summary.csv"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(manifest["analysis"]["kind"], "fourier");
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn fourier_xyce_backend_normalizes_summary_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '%s\\n' 'Fourier analysis for V(out):' '  No. Harmonics: 9, THD: 12.5 %, Gridsize: 200, Interpolation Degree: 1, No. Periods: 1' '' 'Harmonic Frequency   Magnitude   Phase       Norm. Mag   Norm. Phase' '-------- ---------   ---------   -----       ---------   -----------' ' 0       0           0.500000    0           0           0' ' 1       100000      0.250000    -45.0       1           0' > \"$1.four0\"\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_fourier_project(
        project_dir.path(),
        "xyce",
        "V(out)",
        100.0,
        100_000.0,
        r#"        fourier_assertions:
          - name: fundamental_present
            harmonic: 1
            metric: magnitude
            relation: below
            threshold: 1.0
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
        .env("PATH", fake_path.path())
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();

    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert!(artifact_path(&report, "circuitci_xyce_fourier.cir.four0").ends_with(".four0"));
    let summary_path = artifact_path(&report, "fourier_summary.csv");
    assert_fourier_summary_has_fundamental(summary_path);
    assert_eq!(report["fourier_summaries"].as_array().unwrap().len(), 2);
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["analysis"]["kind"], "fourier");
    assert_eq!(manifest["outputs"]["raw"][0]["kind"], "xyce_fourier_raw");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "fourier_summary"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn fourier_backend_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_fourier_project(
        project_dir.path(),
        "ngspice",
        "V(out)",
        100.0,
        100_000.0,
        "",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_FOURIER_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["selected_backend"],
        "ngspice"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "ngspice_fourier_summary_csv"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("circuitci_ngspice_fourier.cir")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| { artifact.as_str().unwrap().ends_with("ngspice_fourier.log") })
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn fourier_contract_rejects_unbound_output_node_before_backend_planning() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_fourier_project(
        project_dir.path(),
        "ngspice",
        "V(missing_node)",
        100.0,
        100_000.0,
        "",
    );

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

#[cfg(unix)]
#[test]
fn fourier_contract_requires_transient_window_to_cover_period() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path =
        write_fourier_project(project_dir.path(), "ngspice", "V(out)", 1.0, 100_000.0, "");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("cover at least one fundamental period")
    );
    assert_report_schema_valid(&report);
}
