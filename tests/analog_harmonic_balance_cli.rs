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

fn artifact_path<'a>(report: &'a Value, suffix: &str) -> &'a str {
    report["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with(suffix))
        .unwrap_or_else(|| panic!("missing artifact ending with {suffix}"))
}

fn write_harmonic_balance_project(
    dir: &std::path::Path,
    backend: &str,
    output_expression: &str,
    drive_source: &str,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: hb_contract, version: 0.1.0 }}
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
  - name: rc_harmonic_balance
    type: analog_harmonic_balance
    checks: [SPICE_HARMONIC_BALANCE_ANALYSIS]
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
        type: hb
        hb_fundamental_frequency_hz: 100000.0
        hb_output_expression: {output_expression}
        hb_harmonics: 10
        hb_drive_sources: [{drive_source}]
      stimuli:
        - {{ name: periodic_drive, description: Planned harmonic-balance spectrum extraction. }}
      probes:
        - {{ name: out_hb, expression: V(out) }}
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
fn harmonic_balance_contract_is_schema_valid_and_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path =
        write_harmonic_balance_project(project_dir.path(), "ngspice", "V(out)", "V1");
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "SPICE_HARMONIC_BALANCE_ANALYSIS"
    );
    assert_eq!(
        report["failures"][0]["measured"]["adapter_status"],
        "planned_not_implemented"
    );
    assert_eq!(
        report["failures"][0]["measured"]["analysis_kind"],
        "harmonic_balance"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][0],
        "hb_spectrum"
    );
    assert_eq!(
        report["failures"][0]["limit"]["implemented_backend"],
        "xyce"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_harmonic_balance_launch_failure_reports_solver_artifacts() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_harmonic_balance_project(project_dir.path(), "xyce", "V(out)", "V1");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "SPICE_HARMONIC_BALANCE_ANALYSIS"
    );
    assert_eq!(
        report["failures"][0]["measured"]["selected_backend"],
        "Xyce"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_evidence"],
        "xyce_hb_spectrum_csv"
    );
    let _wrapper = artifact_path(&report, "circuitci_xyce_hb.cir");
    let _log = artifact_path(&report, "xyce_hb.log");
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn explicit_xyce_harmonic_balance_normalizes_spectrum_and_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "Xyce",
        "#!/bin/sh\nprintf '%s\\n' 'FREQ,V(out)_REAL,V(out)_IMAG' '0,5.0e-1,0' '1.0e5,3.0e-1,-4.0e-1' '-1.0e5,3.0e-1,4.0e-1' > hb_spectrum_raw.csv\nexit 0\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_harmonic_balance_project(project_dir.path(), "xyce", "V(out)", "V1");

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
    let spectrum = out_dir
        .path()
        .join(artifact_path(&report, "hb_spectrum.csv"));
    let text = fs::read_to_string(spectrum).unwrap();
    let mut lines = text.lines();
    assert_eq!(
        lines.next().unwrap(),
        "output_expression,fundamental_frequency_hz,harmonic,frequency_hz,real,imaginary,magnitude,phase_deg"
    );
    assert!(
        lines.any(|line| line.contains("V(out),1.000000000000e5,1,1.000000000000e5,3.000000000000e-1,-4.000000000000e-1,5.000000000000e-1")),
        "normalized HB spectrum did not contain expected first harmonic row: {text}"
    );
    let manifest = out_dir
        .path()
        .join(artifact_path(&report, "solver_manifest.json"));
    let manifest_text = fs::read_to_string(manifest).unwrap();
    let manifest_json: Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(manifest_json["analysis"]["kind"], "harmonic_balance");
    assert!(
        manifest_json["outputs"]["normalized"]
            .as_array()
            .unwrap()
            .iter()
            .any(|output| output["kind"] == "hb_spectrum")
    );
    let wrapper = out_dir
        .path()
        .join(artifact_path(&report, "circuitci_xyce_hb.cir"));
    let wrapper_text = fs::read_to_string(wrapper).unwrap();
    assert!(wrapper_text.contains(".HB 1.000000000000e5"));
    assert!(wrapper_text.contains(".OPTIONS HBINT NUMFREQ=10"));
    assert!(wrapper_text.contains(".PRINT HB_FD FORMAT=CSV"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn harmonic_balance_contract_rejects_unbound_output_before_backend_planning() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path =
        write_harmonic_balance_project(project_dir.path(), "ngspice", "V(missing_node)", "V1");

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
fn harmonic_balance_contract_rejects_missing_generated_drive_source() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path =
        write_harmonic_balance_project(project_dir.path(), "ngspice", "V(out)", "missing_source");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("drive source missing_source is not a generated board component")
    );
    assert_report_schema_valid(&report);
}
