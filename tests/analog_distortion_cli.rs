mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation_with_path};
use serde_json::Value;
use std::fs;

#[cfg(unix)]
fn fake_executable(dir: &std::path::Path, name: &str) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    fs::write(&path, "#!/bin/sh\nexit 99\n").unwrap();
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

#[cfg(unix)]
#[test]
fn distortion_contract_is_schema_valid_and_fails_closed_with_planning_evidence() {
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
            f2_over_f1: Some(0.9),
            stop_frequency_hz: 1_000_000.0,
        },
    );
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

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
        "manual_documents_disto_command_and_distof1_distof2_source_keywords_but_circuitci_normalizer_not_implemented_yet"
    );
    assert_eq!(
        report["failures"][0]["measured"]["source_notes"]["ngspice_manual"],
        "sources/ngspice_manual.xhtml"
    );
    assert_eq!(
        report["failures"][0]["limit"]["trusted_backend_status"],
        "none_available"
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
