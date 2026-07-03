mod common;

use common::{
    assert_report_schema_valid, run_validation_with_path, run_validation_with_path_and_env,
};
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

fn analog_backend_project(project: &str, backend: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let text = fs::read_to_string(project)
        .unwrap()
        .replace("backend: auto", &format!("backend: {backend}"))
        .replace("../../libs", &repo.join("libs").to_string_lossy())
        .replace("../../models", &repo.join("models").to_string_lossy());
    let path = dir.path().join("project.yaml");
    fs::write(&path, text).unwrap();
    (dir, path)
}

fn write_xyce_rc_project(dir: &std::path::Path) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: xyce_rc_smoke, version: 0.1.0 }}
libraries:
  - {libs}
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins: {{ P: vin, N: gnd }}
      spice: {{ primitive: dc_voltage_source, dc_v: 3.3 }}
    R1:
      model: generic.analog.resistor
      pins: {{ A: vin, B: out }}
      spice: {{ primitive: resistor, value_ohm: 1000 }}
    C1:
      model: generic.analog.capacitor
      pins: {{ A: out, B: gnd }}
      spice: {{ primitive: capacitor, value_f: 0.000001 }}
  nets:
    vin: {{ kind: power, nominal_voltage: 3.3, powered: true }}
    out: {{ kind: digital_or_analog }}
    gnd: {{ kind: ground }}
scenarios:
  - name: xyce_rc_transient
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: xyce
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
      analysis: {{ type: tran, stop_time_us: 10, max_step_us: 1 }}
      stimuli: []
      probes:
        - {{ name: out, expression: V(out) }}
      assertions:
        - {{ name: out_rises, probe: out, at_us: 5, relation: above, threshold_v: 0.5 }}
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy()
        ),
    )
    .unwrap();
    project
}

fn write_xyce_ac_rc_project(dir: &std::path::Path) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: xyce_ac_smoke, version: 0.1.0 }}
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
  - name: xyce_rc_ac
    type: analog_ac
    checks: [SPICE_AC_ANALYSIS]
    analog:
      backend: xyce
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
      analysis: {{ type: ac, start_frequency_hz: 10.0, stop_frequency_hz: 100000.0, points_per_decade: 10 }}
      stimuli:
        - {{ name: unity_ac_drive, description: Fake Xyce fixture exports a normalized 1 V AC response. }}
      probes:
        - {{ name: out, expression: V(out) }}
      assertions:
        - {{ name: out_gain_at_1khz_below_minus_1db, probe: out, aggregation: gain_db_at_frequency, relation: below, at_hz: 1000.0, threshold_db: -1.0 }}
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy()
        ),
    )
    .unwrap();
    project
}

fn write_xyce_dc_divider_project(dir: &std::path::Path) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: xyce_dc_smoke, version: 0.1.0 }}
libraries:
  - {libs}
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins: {{ P: vin, N: gnd }}
      spice: {{ primitive: dc_voltage_source, dc_v: 5.0 }}
    R1:
      model: generic.analog.resistor
      pins: {{ A: vin, B: midpoint }}
      spice: {{ primitive: resistor, value_ohm: 10000 }}
    R2:
      model: generic.analog.resistor
      pins: {{ A: midpoint, B: gnd }}
      spice: {{ primitive: resistor, value_ohm: 10000 }}
  nets:
    vin: {{ kind: power, nominal_voltage: 5.0, powered: true }}
    midpoint: {{ kind: digital_or_analog, nominal_voltage: 2.5 }}
    gnd: {{ kind: ground }}
scenarios:
  - name: xyce_divider_dc
    type: analog_dc
    checks: [SPICE_DC_ANALYSIS]
    analog:
      backend: xyce
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1, R2]
      model_files: []
      node_bindings:
        - {{ node: vin, net: vin }}
        - {{ node: midpoint, net: midpoint }}
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: vin, endpoint: {{ component: V1, pin: P }} }}
        - {{ node: "0", endpoint: {{ component: V1, pin: N }} }}
        - {{ node: vin, endpoint: {{ component: R1, pin: A }} }}
        - {{ node: midpoint, endpoint: {{ component: R1, pin: B }} }}
        - {{ node: midpoint, endpoint: {{ component: R2, pin: A }} }}
        - {{ node: "0", endpoint: {{ component: R2, pin: B }} }}
      analysis: {{ type: op }}
      stimuli:
        - {{ name: dc_divider_bias, description: Fake Xyce fixture exports 5 V input and 2.5 V midpoint. }}
      probes:
        - {{ name: vin, expression: V(vin) }}
        - {{ name: midpoint, expression: V(midpoint) }}
      assertions:
        - {{ name: vin_above_4_9v, probe: vin, aggregation: operating_point, relation: above, threshold_v: 4.9 }}
        - {{ name: midpoint_above_2_4v, probe: midpoint, aggregation: operating_point, relation: above, threshold_v: 2.4 }}
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy()
        ),
    )
    .unwrap();
    project
}

fn write_xyce_noise_divider_project(dir: &std::path::Path) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: xyce_noise_smoke, version: 0.1.0 }}
libraries:
  - {libs}
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins: {{ P: vin, N: gnd }}
      spice: {{ primitive: dc_voltage_source, dc_v: 5.0 }}
    R1:
      model: generic.analog.resistor
      pins: {{ A: vin, B: midpoint }}
      spice: {{ primitive: resistor, value_ohm: 10000 }}
    R2:
      model: generic.analog.resistor
      pins: {{ A: midpoint, B: gnd }}
      spice: {{ primitive: resistor, value_ohm: 10000 }}
  nets:
    vin: {{ kind: power, nominal_voltage: 5.0, powered: true }}
    midpoint: {{ kind: digital_or_analog, nominal_voltage: 2.5 }}
    gnd: {{ kind: ground }}
scenarios:
  - name: xyce_divider_noise
    type: analog_noise
    checks: [SPICE_NOISE_ANALYSIS]
    analog:
      backend: xyce
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1, R2]
      model_files: []
      node_bindings:
        - {{ node: vin, net: vin }}
        - {{ node: midpoint, net: midpoint }}
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: vin, endpoint: {{ component: V1, pin: P }} }}
        - {{ node: "0", endpoint: {{ component: V1, pin: N }} }}
        - {{ node: vin, endpoint: {{ component: R1, pin: A }} }}
        - {{ node: midpoint, endpoint: {{ component: R1, pin: B }} }}
        - {{ node: midpoint, endpoint: {{ component: R2, pin: A }} }}
        - {{ node: "0", endpoint: {{ component: R2, pin: B }} }}
      analysis:
        type: noise
        start_frequency_hz: 10.0
        stop_frequency_hz: 100000.0
        points_per_decade: 20
        noise_output_node: midpoint
        noise_input_source: V1
      stimuli:
        - {{ name: divider_noise_band, description: Fake Xyce fixture exports spectrum and integrated noise. }}
      probes:
        - {{ name: onoise, expression: V(midpoint) }}
        - {{ name: inoise, expression: V(vin) }}
      assertions:
        - {{ name: output_density_1khz_below_10nv, probe: onoise, aggregation: output_noise_density_at_frequency, relation: below, at_hz: 1000.0, threshold_v_per_sqrt_hz: 1.0e-8 }}
        - {{ name: output_rms_noise_below_3_5uv, probe: onoise, aggregation: integrated_output_noise, relation: below, threshold_v: 3.5e-6 }}
        - {{ name: input_referred_rms_noise_below_7uv, probe: inoise, aggregation: integrated_input_noise, relation: below, threshold_v: 7.0e-6 }}
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy()
        ),
    )
    .unwrap();
    project
}

#[cfg(unix)]
#[test]
fn auto_backend_keeps_xyce_explicit_until_conformance_enabled() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let (_project_dir, project_path) =
        analog_backend_project("examples/good_mosfet_low_side_switch/project.yaml", "auto");

    let missing_library = fake_path.path().join("missing-libngspice.dylib");
    let report = run_validation_with_path_and_env(
        project_path.to_str().unwrap(),
        fake_path.path(),
        &[("CIRCUITCI_LIBNGSPICE", missing_library.to_str().unwrap())],
    );

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    assert_eq!(
        report["failures"][0]["measured"]["requested_backend"],
        "auto"
    );
    assert!(report["waveforms"].as_array().unwrap().is_empty());
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_backend_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let (_project_dir, project_path) =
        analog_backend_project("examples/good_mosfet_low_side_switch/project.yaml", "xyce");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_TRANSIENT_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["selected_backend"],
        "Xyce"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "xyce_transient_waveform_csv"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert_eq!(
        artifacts
            .iter()
            .filter(|artifact| artifact.as_str().unwrap().ends_with("circuitci_xyce.cir"))
            .count(),
        1
    );
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("xyce.log"))
    );
    assert!(report["waveforms"].as_array().unwrap().is_empty());
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_backend_normalizes_transient_waveform_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf 'TIME,V(out)\\n0,0\\n5e-6,1.2\\n1e-5,1.3\\n' > waveform_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_xyce_rc_project(project_dir.path());

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
    assert!(waveforms[0].as_str().unwrap().ends_with("waveform.csv"));
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("waveform_raw.csv"))
    );
    let manifest_path = artifacts
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with("solver_manifest.json"))
        .unwrap();
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["outputs"]["raw"][0]["kind"], "xyce_transient_raw");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "transient_waveform"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_ac_backend_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let (_project_dir, project_path) = analog_backend_project(
        "examples/good_generated_rc_lowpass_bode_observation/project.yaml",
        "xyce",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_AC_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["selected_backend"],
        "Xyce"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "xyce_ac_bode_csv"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("circuitci_xyce_ac.cir")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("xyce_ac.log"))
    );
    assert!(report["waveforms"].as_array().unwrap().is_empty());
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_ac_backend_normalizes_bode_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf 'FREQ,Re(V(out)),Im(V(out))\\n10,1,0\\n1000,0.5,0\\n100000,0.01,0\\n' > ac_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_xyce_ac_rc_project(project_dir.path());
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
    assert!(waveforms[0].as_str().unwrap().ends_with("bode.csv"));
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("ac_raw.csv"))
    );
    let manifest_path = artifacts
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with("solver_manifest.json"))
        .unwrap();
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["analysis"]["kind"], "ac");
    assert_eq!(manifest["outputs"]["raw"][0]["kind"], "xyce_ac_raw");
    assert_eq!(manifest["outputs"]["normalized"][0]["kind"], "ac_bode");
    let bode_path = waveforms[0].as_str().unwrap();
    let bode = fs::read_to_string(bode_path).unwrap();
    assert!(bode.contains("out_mag_db"));
    assert!(bode.contains("-6.020599913280e0"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_dc_backend_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let (_project_dir, project_path) =
        analog_backend_project("examples/good_dc_bias_observation/project.yaml", "xyce");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_DC_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["selected_backend"],
        "Xyce"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "xyce_operating_point_csv"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("circuitci_xyce_op.cir")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("xyce_op.log"))
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_dc_backend_normalizes_operating_point_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf 'INDEX,V(vin),V(midpoint)\\n0,5.0,2.5\\n' > operating_point_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_xyce_dc_divider_project(project_dir.path());
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
    assert!(report["waveforms"].as_array().unwrap().is_empty());
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("operating_point_raw.csv")
    }));
    let operating_point_path = artifacts
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with("operating_point.csv"))
        .unwrap();
    let operating_point = fs::read_to_string(operating_point_path).unwrap();
    assert!(operating_point.contains("vin,midpoint"));
    assert!(operating_point.contains("5.000000000000e0,2.500000000000e0"));
    let manifest_path = artifacts
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with("solver_manifest.json"))
        .unwrap();
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["analysis"]["kind"], "operating_point");
    assert_eq!(
        manifest["outputs"]["raw"][0]["kind"],
        "xyce_operating_point_raw"
    );
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "operating_point"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_noise_backend_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let (_project_dir, project_path) =
        analog_backend_project("examples/good_noise_observation/project.yaml", "xyce");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_NOISE_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["selected_backend"],
        "Xyce"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "xyce_noise_csv"
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("circuitci_xyce_noise.cir")
    }));
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("xyce_noise.log"))
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_noise_backend_normalizes_spectrum_total_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf 'FREQ,ONOISE,INOISE\\n10,2e-9,4e-9\\n1000,3e-9,6e-9\\n100000,1e-9,2e-9\\n' > noise_spectrum_raw.csv\nprintf 'INDEX,ONOISE_TOTAL,INOISE_TOTAL\\n0,2e-7,4e-7\\n' > noise_total_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_xyce_noise_divider_project(project_dir.path());
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
    assert!(
        waveforms[0]
            .as_str()
            .unwrap()
            .ends_with("noise_spectrum.csv")
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("noise_total.csv"))
    );
    let spectrum = fs::read_to_string(waveforms[0].as_str().unwrap()).unwrap();
    assert!(spectrum.contains("frequency_hz,onoise_v_per_sqrt_hz,inoise_v_per_sqrt_hz"));
    assert!(spectrum.contains("1.000000000000e3,3.000000000000e-9,6.000000000000e-9"));
    let manifest_path = artifacts
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with("solver_manifest.json"))
        .unwrap();
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["analysis"]["kind"], "noise");
    assert_eq!(
        manifest["outputs"]["raw"][0]["kind"],
        "xyce_noise_spectrum_raw"
    );
    assert_eq!(
        manifest["outputs"]["raw"][1]["kind"],
        "xyce_noise_total_raw"
    );
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "noise_spectrum"
    );
    assert_eq!(manifest["outputs"]["normalized"][1]["kind"], "noise_total");
    assert_report_schema_valid(&report);
}
