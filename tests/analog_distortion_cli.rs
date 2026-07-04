mod common;

use common::{
    assert_report_schema_valid, assert_yaml_file_valid, binary_available, run_validation_with_path,
};
use serde_json::Value;
use std::fs;
use std::process::Command;

const REAL_NGSPICE_DISTORTION_ENV: &str = "CIRCUITCI_RUN_REAL_NGSPICE_DISTO";

#[cfg(unix)]
fn fake_executable(dir: &std::path::Path, name: &str) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    fs::write(&path, "#!/bin/sh\nexit 99\n").unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
}

#[cfg(unix)]
fn fake_ngspice_distortion(dir: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("ngspice");
    fs::write(
        &path,
        r#"#!/bin/sh
printf '%s\n' 'CIRCUITCI_DISTORTION_COMPONENT im_f1_plus_f2'
printf '%s\n' 'Index   frequency       v(out)'
printf '%s\n' '0 1.000000e+03 2.000000e-03, -3.000000e-04'
printf '%s\n' '1 1.000000e+04 4.000000e-03, 5.000000e-04'
printf '%s\n' 'CIRCUITCI_DISTORTION_COMPONENT im_f1_minus_f2'
printf '%s\n' 'Index   frequency       v(out)'
printf '%s\n' '0 1.000000e+03 1.000000e-03, 0.000000e+00'
printf '%s\n' 'CIRCUITCI_DISTORTION_COMPONENT im_2f1_minus_f2'
printf '%s\n' 'Index   frequency       v(out)'
printf '%s\n' '0 1.000000e+03 5.000000e-04, 0.000000e+00'
exit 0
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
}

struct DistortionProjectOptions<'a> {
    backend: &'a str,
    mode: &'a str,
    output_expression: &'a str,
    f1_source: &'a str,
    f2_source: &'a str,
    f2_over_f1: Option<f64>,
    stop_frequency_hz: f64,
}

fn write_distortion_project(
    dir: &std::path::Path,
    options: DistortionProjectOptions<'_>,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let f2_sources = if options.f2_source.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", options.f2_source)
    };
    let ratio_line = options
        .f2_over_f1
        .map(|ratio| format!("        distortion_f2_over_f1: {ratio}\n"))
        .unwrap_or_default();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: distortion_contract, version: 0.1.0 }}
libraries:
  - {libs}
board:
  components:
    VIN1:
      model: generic.analog.dc_voltage_source
      pins: {{ P: in1, N: gnd }}
      spice: {{ primitive: dc_voltage_source, dc_v: 0.01 }}
    VIN2:
      model: generic.analog.dc_voltage_source
      pins: {{ P: in2, N: gnd }}
      spice: {{ primitive: dc_voltage_source, dc_v: 0.002 }}
    R1:
      model: generic.analog.resistor
      pins: {{ A: in1, B: out }}
      spice: {{ primitive: resistor, value_ohm: 1000 }}
    R2:
      model: generic.analog.resistor
      pins: {{ A: in2, B: out }}
      spice: {{ primitive: resistor, value_ohm: 2000 }}
    C1:
      model: generic.analog.capacitor
      pins: {{ A: out, B: gnd }}
      spice: {{ primitive: capacitor, value_f: 0.000001 }}
  nets:
    in1: {{ kind: power, nominal_voltage: 0.01, powered: true }}
    in2: {{ kind: power, nominal_voltage: 0.002, powered: true }}
    out: {{ kind: digital_or_analog }}
    gnd: {{ kind: ground }}
scenarios:
  - name: rc_distortion
    type: analog_distortion
    checks: [SPICE_DISTORTION_ANALYSIS]
    analog:
      backend: {backend}
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [VIN1, VIN2, R1, R2, C1]
      model_files: []
      node_bindings:
        - {{ node: in1, net: in1 }}
        - {{ node: in2, net: in2 }}
        - {{ node: out, net: out }}
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: in1, endpoint: {{ component: VIN1, pin: P }} }}
        - {{ node: "0", endpoint: {{ component: VIN1, pin: N }} }}
        - {{ node: in2, endpoint: {{ component: VIN2, pin: P }} }}
        - {{ node: "0", endpoint: {{ component: VIN2, pin: N }} }}
        - {{ node: in1, endpoint: {{ component: R1, pin: A }} }}
        - {{ node: out, endpoint: {{ component: R1, pin: B }} }}
        - {{ node: in2, endpoint: {{ component: R2, pin: A }} }}
        - {{ node: out, endpoint: {{ component: R2, pin: B }} }}
        - {{ node: out, endpoint: {{ component: C1, pin: A }} }}
        - {{ node: "0", endpoint: {{ component: C1, pin: B }} }}
      analysis:
        type: disto
        distortion_mode: {mode}
        distortion_start_frequency_hz: 10.0
        distortion_stop_frequency_hz: {stop_frequency_hz}
        distortion_points_per_decade: 20
        distortion_output_expression: {output_expression}
        distortion_f1_sources: [{f1_source}]
        distortion_f2_sources: {f2_sources}
{ratio_line}      stimuli:
        - {{ name: distortion_intent, description: Small-signal distortion planning evidence. }}
      probes:
        - {{ name: out_disto, expression: V(out) }}
      assertions: []
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy(),
            backend = options.backend,
            mode = options.mode,
            stop_frequency_hz = options.stop_frequency_hz,
            output_expression = options.output_expression,
            f1_source = options.f1_source,
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

fn real_ngspice_distortion_enabled() -> bool {
    if std::env::var_os(REAL_NGSPICE_DISTORTION_ENV).as_deref() != Some(std::ffi::OsStr::new("1")) {
        eprintln!(
            "skipping real-ngspice distortion conformance; set {REAL_NGSPICE_DISTORTION_ENV}=1"
        );
        return false;
    }
    if !binary_available("ngspice") {
        eprintln!("skipping real-ngspice distortion conformance; ngspice is not on PATH");
        return false;
    }
    true
}

fn write_file_distortion_project(dir: &std::path::Path) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let netlist = dir.join("diode_disto.cir");
    fs::write(
        &netlist,
        r#"* diode distortion conformance
VIN out 0 DC 0.2
D1 out 0 DMOD
.model DMOD D(Is=1e-14 N=1)
.end
"#,
    )
    .unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: diode_distortion_conformance, version: 0.1.0 }}
libraries:
  - {libs}
board:
  components:
    VIN:
      model: generic.analog.dc_voltage_source
      pins: {{ P: out, N: gnd }}
    D1:
      model: generic.analog.switching_diode
      pins: {{ A: out, K: gnd }}
  nets:
    out: {{ kind: power, nominal_voltage: 0.2, powered: true }}
    gnd: {{ kind: ground }}
scenarios:
  - name: diode_distortion
    type: analog_distortion
    checks: [SPICE_DISTORTION_ANALYSIS]
    analog:
      backend: ngspice
      netlist_source: file
      netlist: diode_disto.cir
      model_files: []
      node_bindings:
        - {{ node: out, net: out }}
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: out, endpoint: {{ component: VIN, pin: P }} }}
        - {{ node: "0", endpoint: {{ component: VIN, pin: N }} }}
      analysis:
        type: disto
        distortion_mode: harmonic
        distortion_start_frequency_hz: 1000.0
        distortion_stop_frequency_hz: 10000.0
        distortion_points_per_decade: 3
        distortion_output_expression: I(VIN)
        distortion_f1_sources: [VIN]
      stimuli: []
      probes: []
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
fn distortion_contract_runs_ngspice_and_normalizes_spectrum() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_ngspice_distortion(fake_path.path());
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_distortion_project(
        project_dir.path(),
        DistortionProjectOptions {
            backend: "ngspice",
            mode: "intermodulation",
            output_expression: "V(out)",
            f1_source: "VIN1",
            f2_source: "VIN2",
            f2_over_f1: Some(0.9),
            stop_frequency_hz: 1_000_000.0,
        },
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
    assert!(report["failures"].as_array().unwrap().is_empty());
    let spectrum = artifact_path(&report, "distortion_spectrum.csv");
    let spectrum_text = fs::read_to_string(spectrum).unwrap();
    assert!(
        spectrum_text
            .contains("im_f1_plus_f2,1.000000000000e3,V(out),2.000000000000e-3,-3.000000000000e-4")
    );
    let summary = artifact_path(&report, "distortion_summary.csv");
    let summary_text = fs::read_to_string(summary).unwrap();
    assert!(summary_text.contains("im_f1_plus_f2,V(out),2,4.031128874149e-3,1.000000000000e4"));
    let convergence: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "distortion_convergence.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(convergence["status"], "pass");
    assert_eq!(convergence["analysis"], "distortion");
    let wrapper = artifact_path(&report, "circuitci_ngspice_disto.cir");
    let wrapper_text = fs::read_to_string(wrapper).unwrap();
    assert!(wrapper_text.contains("VIN1 in1 0 DC 0.01 DISTOF1 1.0 0.0"));
    assert!(wrapper_text.contains("VIN2 in2 0 DC 0.002 DISTOF2 1.0 0.0"));
    assert!(
        wrapper_text.contains("disto dec 20 1.000000000000e1 1.000000000000e6 9.000000000000e-1")
    );
    assert!(wrapper_text.contains("setplot disto3"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn distortion_xyce_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "Xyce");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_distortion_project(
        project_dir.path(),
        DistortionProjectOptions {
            backend: "xyce",
            mode: "harmonic",
            output_expression: "V(out)",
            f1_source: "VIN1",
            f2_source: "",
            f2_over_f1: None,
            stop_frequency_hz: 1_000_000.0,
        },
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_DISTORTION_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["adapter_status"],
        "planned_not_implemented"
    );
    assert_eq!(
        report["failures"][0]["measured"]["analysis_kind"],
        "distortion"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][0],
        "distortion_spectrum"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][1],
        "distortion_summary"
    );
    assert_eq!(
        report["failures"][0]["measured"]["backend_research_status"]["ngspice"],
        "manual_and_source_document_disto_plots_and_circuitci_ngspice_adapter_is_enabled"
    );
    assert_eq!(
        report["failures"][0]["measured"]["source_notes"]["ngspice_disto_source"],
        "sources/ngspice_source_distoan.c.gz"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_ngspice_distortion_conformance_when_enabled() {
    if !real_ngspice_distortion_enabled() {
        return;
    }
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_file_distortion_project(project_dir.path());
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
    let spectrum = fs::read_to_string(artifact_path(&report, "distortion_spectrum.csv")).unwrap();
    assert!(spectrum.contains("h2,"));
    assert!(spectrum.contains("h3,"));
    let wrapper =
        fs::read_to_string(artifact_path(&report, "circuitci_ngspice_disto.cir")).unwrap();
    assert!(wrapper.contains("VIN out 0 DC 0.2 DISTOF1 1.0 0.0"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(manifest["analysis"]["kind"], "distortion");
    assert_eq!(
        manifest["outputs"]["normalized"][2]["kind"],
        "distortion_convergence"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn distortion_rejects_unbound_output_expression() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_distortion_project(
        project_dir.path(),
        DistortionProjectOptions {
            backend: "ngspice",
            mode: "harmonic",
            output_expression: "V(missing)",
            f1_source: "VIN1",
            f2_source: "",
            f2_over_f1: None,
            stop_frequency_hz: 1_000_000.0,
        },
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
}

#[cfg(unix)]
#[test]
fn distortion_rejects_missing_generated_f1_source() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_distortion_project(
        project_dir.path(),
        DistortionProjectOptions {
            backend: "ngspice",
            mode: "harmonic",
            output_expression: "V(out)",
            f1_source: "NO_SUCH_SOURCE",
            f2_source: "",
            f2_over_f1: None,
            stop_frequency_hz: 1_000_000.0,
        },
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("NO_SUCH_SOURCE is not a generated board component")
    );
}

#[cfg(unix)]
#[test]
fn distortion_rejects_intermodulation_without_f2_ratio() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_distortion_project(
        project_dir.path(),
        DistortionProjectOptions {
            backend: "ngspice",
            mode: "intermodulation",
            output_expression: "V(out)",
            f1_source: "VIN1",
            f2_source: "VIN2",
            f2_over_f1: None,
            stop_frequency_hz: 1_000_000.0,
        },
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("requires finite distortion_f2_over_f1")
    );
}

#[cfg(unix)]
#[test]
fn distortion_rejects_invalid_frequency_window() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_distortion_project(
        project_dir.path(),
        DistortionProjectOptions {
            backend: "ngspice",
            mode: "harmonic",
            output_expression: "V(out)",
            f1_source: "VIN1",
            f2_source: "",
            f2_over_f1: None,
            stop_frequency_hz: 1.0,
        },
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("distortion_stop_frequency_hz greater")
    );
}
