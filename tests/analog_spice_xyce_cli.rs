mod common;

use common::{assert_report_schema_valid, binary_available, run_validation_with_path};
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

#[cfg(unix)]
fn real_xyce_conformance_enabled() -> bool {
    if std::env::var(REAL_XYCE_CONFORMANCE_ENV).as_deref() != Ok("1") {
        eprintln!("skipping real Xyce conformance; set {REAL_XYCE_CONFORMANCE_ENV}=1 to run");
        return false;
    }
    if !binary_available("Xyce") && !binary_available("xyce") {
        eprintln!("skipping real Xyce conformance; Xyce/xyce is not on PATH");
        return false;
    }
    true
}

#[cfg(unix)]
fn run_real_xyce_project(project_path: &std::path::Path) -> (tempfile::TempDir, Value) {
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
    (out_dir, report)
}

#[cfg(unix)]
fn run_project_with_path(
    project_path: &std::path::Path,
    path: &std::path::Path,
) -> (tempfile::TempDir, Value) {
    run_project_with_path_and_env(project_path, path, &[])
}

#[cfg(unix)]
fn run_project_with_path_and_env(
    project_path: &std::path::Path,
    path: &std::path::Path,
    envs: &[(&str, &str)],
) -> (tempfile::TempDir, Value) {
    fs::create_dir_all("out").unwrap();
    let out_dir = tempfile::tempdir_in("out").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_circuitci"));
    command
        .args([
            "validate",
            project_path.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .env("PATH", path);
    for (key, value) in envs {
        command.env(key, value);
    }
    let status = command.status().unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();
    (out_dir, report)
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

fn use_auto_backend(project_path: &std::path::Path) {
    let text = fs::read_to_string(project_path).unwrap();
    assert!(text.contains("backend: xyce"));
    fs::write(project_path, text.replace("backend: xyce", "backend: auto")).unwrap();
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
fn auto_backend_uses_xyce_for_transient_when_ngspice_is_absent() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf 'TIME,V(out)\\n0,0\\n5e-6,1.2\\n1e-5,1.3\\n' > waveform_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_xyce_rc_project(project_dir.path());
    use_auto_backend(&project_path);
    let missing_library = fake_path.path().join("missing-libngspice.dylib");

    let (_out_dir, report) = run_project_with_path_and_env(
        &project_path,
        fake_path.path(),
        &[("CIRCUITCI_LIBNGSPICE", missing_library.to_str().unwrap())],
    );

    assert_eq!(report["result"], "pass");
    waveform_path(&report, "waveform.csv");
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["requested"], "auto");
    assert_eq!(manifest["backend"]["selected"], "Xyce");
    assert_eq!(manifest["analysis"]["kind"], "transient");
    assert_eq!(manifest["outputs"]["raw"][0]["kind"], "xyce_transient_raw");
    assert_eq!(
        manifest["outputs"]["normalized"][0]["kind"],
        "transient_waveform"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn auto_backend_uses_xyce_for_ac_dc_noise_when_ngspice_is_absent() {
    let fake_path = tempfile::tempdir().unwrap();
    let project_root = tempfile::tempdir().unwrap();

    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf 'FREQ,Re(V(out)),Im(V(out))\\n10,1,0\\n1000,0.5,0\\n100000,0.01,0\\n' > ac_raw.csv\nexit 0\n",
    );
    let ac_dir = project_root.path().join("ac");
    fs::create_dir_all(&ac_dir).unwrap();
    let ac_project = write_xyce_ac_rc_project(&ac_dir);
    use_auto_backend(&ac_project);
    let (_ac_out, ac_report) = run_project_with_path(&ac_project, fake_path.path());
    assert_eq!(ac_report["result"], "pass");
    let ac_manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&ac_report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(ac_manifest["backend"]["requested"], "auto");
    assert_eq!(ac_manifest["backend"]["selected"], "Xyce");
    assert_eq!(ac_manifest["analysis"]["kind"], "ac");
    assert_eq!(ac_manifest["outputs"]["normalized"][0]["kind"], "ac_bode");
    assert_report_schema_valid(&ac_report);

    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf 'INDEX,V(vin),V(midpoint)\\n0,5.0,2.5\\n' > operating_point_raw.csv\nexit 0\n",
    );
    let dc_dir = project_root.path().join("dc");
    fs::create_dir_all(&dc_dir).unwrap();
    let dc_project = write_xyce_dc_divider_project(&dc_dir);
    use_auto_backend(&dc_project);
    let (_dc_out, dc_report) = run_project_with_path(&dc_project, fake_path.path());
    assert_eq!(dc_report["result"], "pass");
    let dc_manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&dc_report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(dc_manifest["backend"]["requested"], "auto");
    assert_eq!(dc_manifest["backend"]["selected"], "Xyce");
    assert_eq!(dc_manifest["analysis"]["kind"], "operating_point");
    assert_eq!(
        dc_manifest["outputs"]["normalized"][0]["kind"],
        "operating_point"
    );
    assert_report_schema_valid(&dc_report);

    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf 'FREQ,ONOISE,INOISE\\n10,2e-9,4e-9\\n1000,3e-9,6e-9\\n100000,1e-9,2e-9\\n' > noise_spectrum_raw.csv\nprintf 'INDEX,ONOISE_TOTAL,INOISE_TOTAL\\n0,2e-7,4e-7\\n' > noise_total_raw.csv\nexit 0\n",
    );
    let noise_dir = project_root.path().join("noise");
    fs::create_dir_all(&noise_dir).unwrap();
    let noise_project = write_xyce_noise_divider_project(&noise_dir);
    use_auto_backend(&noise_project);
    let (_noise_out, noise_report) = run_project_with_path(&noise_project, fake_path.path());
    assert_eq!(noise_report["result"], "pass");
    let noise_manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&noise_report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(noise_manifest["backend"]["requested"], "auto");
    assert_eq!(noise_manifest["backend"]["selected"], "Xyce");
    assert_eq!(noise_manifest["analysis"]["kind"], "noise");
    assert_eq!(
        noise_manifest["outputs"]["normalized"][0]["kind"],
        "noise_spectrum"
    );
    assert_eq!(
        noise_manifest["outputs"]["normalized"][1]["kind"],
        "noise_total"
    );
    assert_report_schema_valid(&noise_report);
}

#[cfg(unix)]
#[test]
fn real_xyce_conformance_normalizes_supported_analysis_contracts_when_enabled() {
    if !real_xyce_conformance_enabled() {
        return;
    }

    let project_root = tempfile::tempdir().unwrap();

    let transient_dir = project_root.path().join("transient");
    fs::create_dir_all(&transient_dir).unwrap();
    let transient_project = write_xyce_rc_project(&transient_dir);
    let (_transient_out, transient_report) = run_real_xyce_project(&transient_project);
    assert_eq!(transient_report["result"], "pass");
    assert_eq!(transient_report["summary"]["critical"], 0);
    assert_csv_has_header(
        waveform_path(&transient_report, "waveform.csv"),
        &["time_s", "out"],
    );
    let transient_manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&transient_report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(transient_manifest["backend"]["selected"], "Xyce");
    assert_eq!(transient_manifest["analysis"]["kind"], "transient");
    assert_eq!(
        transient_manifest["outputs"]["normalized"][0]["kind"],
        "transient_waveform"
    );
    assert_report_schema_valid(&transient_report);

    let ac_dir = project_root.path().join("ac");
    fs::create_dir_all(&ac_dir).unwrap();
    let ac_project = write_xyce_ac_rc_project(&ac_dir);
    let (_ac_out, ac_report) = run_real_xyce_project(&ac_project);
    assert_eq!(ac_report["result"], "pass");
    assert_eq!(ac_report["summary"]["critical"], 0);
    assert_csv_has_header(
        waveform_path(&ac_report, "bode.csv"),
        &[
            "frequency_hz",
            "out_mag_db",
            "out_phase_deg",
            "out_mag_linear",
        ],
    );
    let ac_manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&ac_report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(ac_manifest["backend"]["selected"], "Xyce");
    assert_eq!(ac_manifest["analysis"]["kind"], "ac");
    assert_eq!(ac_manifest["outputs"]["normalized"][0]["kind"], "ac_bode");
    assert_report_schema_valid(&ac_report);

    let dc_dir = project_root.path().join("dc");
    fs::create_dir_all(&dc_dir).unwrap();
    let dc_project = write_xyce_dc_divider_project(&dc_dir);
    let (_dc_out, dc_report) = run_real_xyce_project(&dc_project);
    assert_eq!(dc_report["result"], "pass");
    assert_eq!(dc_report["summary"]["critical"], 0);
    assert_csv_has_header(
        artifact_path(&dc_report, "operating_point.csv"),
        &["vin", "midpoint"],
    );
    let dc_manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&dc_report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(dc_manifest["backend"]["selected"], "Xyce");
    assert_eq!(dc_manifest["analysis"]["kind"], "operating_point");
    assert_eq!(
        dc_manifest["outputs"]["normalized"][0]["kind"],
        "operating_point"
    );
    assert_report_schema_valid(&dc_report);

    let noise_dir = project_root.path().join("noise");
    fs::create_dir_all(&noise_dir).unwrap();
    let noise_project = write_xyce_noise_divider_project(&noise_dir);
    let (_noise_out, noise_report) = run_real_xyce_project(&noise_project);
    assert_eq!(noise_report["result"], "pass");
    assert_eq!(noise_report["summary"]["critical"], 0);
    assert_csv_has_header(
        waveform_path(&noise_report, "noise_spectrum.csv"),
        &[
            "frequency_hz",
            "onoise_v_per_sqrt_hz",
            "inoise_v_per_sqrt_hz",
        ],
    );
    assert_csv_has_header(
        artifact_path(&noise_report, "noise_total.csv"),
        &["onoise_total_v", "inoise_total_v"],
    );
    let noise_manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&noise_report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(noise_manifest["backend"]["selected"], "Xyce");
    assert_eq!(noise_manifest["analysis"]["kind"], "noise");
    assert_eq!(
        noise_manifest["outputs"]["normalized"][0]["kind"],
        "noise_spectrum"
    );
    assert_eq!(
        noise_manifest["outputs"]["normalized"][1]["kind"],
        "noise_total"
    );
    assert_report_schema_valid(&noise_report);
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
fn explicit_xyce_ac_backend_evaluates_group_delay_assertions() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf 'FREQ,Re(V(out)),Im(V(out))\\n10,1,0\\n1000,0,-1\\n100000,-1,0\\n' > ac_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_xyce_ac_rc_project(project_dir.path());
    let text = fs::read_to_string(&project_path).unwrap().replace(
        "        - { name: out_gain_at_1khz_below_minus_1db, probe: out, aggregation: gain_db_at_frequency, relation: below, at_hz: 1000.0, threshold_db: -1.0 }",
        "        - { name: out_group_delay_below_10us, probe: out, aggregation: group_delay_s_at_frequency, relation: below, at_hz: 1000.0, threshold_s: 1.0e-5 }",
    );
    fs::write(&project_path, text).unwrap();
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
fn explicit_xyce_ac_backend_fails_group_delay_limits() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf 'FREQ,Re(V(out)),Im(V(out))\\n10,1,0\\n1000,0,-1\\n100000,-1,0\\n' > ac_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_xyce_ac_rc_project(project_dir.path());
    let text = fs::read_to_string(&project_path).unwrap().replace(
        "        - { name: out_gain_at_1khz_below_minus_1db, probe: out, aggregation: gain_db_at_frequency, relation: below, at_hz: 1000.0, threshold_db: -1.0 }",
        "        - { name: out_group_delay_below_1us, probe: out, aggregation: group_delay_s_at_frequency, relation: below, at_hz: 1000.0, threshold_s: 1.0e-6 }",
    );
    fs::write(&project_path, text).unwrap();
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
    assert_eq!(report["failures"][0]["id"], "SPICE_AC_ANALYSIS");
    assert_eq!(report["failures"][0]["measured"]["quantity"], "group_delay");
    assert!(report["failures"][0]["measured"]["out"].as_f64().unwrap() > 1.0e-6);
    assert_eq!(
        report["failures"][0]["measured"]["decision_threshold_s"],
        1.0e-6
    );
    assert_eq!(report["failures"][0]["limit"]["below_s"], 1.0e-6);
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
