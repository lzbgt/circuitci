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

fn write_phase_noise_project(
    dir: &std::path::Path,
    backend: &str,
    mode: &str,
    output_expression: &str,
    drive_source: &str,
    integration_start_hz: Option<f64>,
    integration_stop_hz: Option<f64>,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let phase_noise_drive_sources = if drive_source.is_empty() {
        "[]".to_string()
    } else {
        format!("[{drive_source}]")
    };
    let integration_lines = match (integration_start_hz, integration_stop_hz) {
        (Some(start), Some(stop)) => format!(
            "        phase_noise_integration_start_hz: {start}\n        phase_noise_integration_stop_hz: {stop}\n"
        ),
        (Some(start), None) => format!("        phase_noise_integration_start_hz: {start}\n"),
        (None, Some(stop)) => format!("        phase_noise_integration_stop_hz: {stop}\n"),
        (None, None) => String::new(),
    };
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: phase_noise_contract, version: 0.1.0 }}
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
  - name: rc_phase_noise
    type: analog_phase_noise
    checks: [SPICE_PHASE_NOISE_ANALYSIS]
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
        type: phase_noise
        phase_noise_mode: {mode}
        phase_noise_carrier_frequency_hz: 100000.0
        phase_noise_offset_start_hz: 10.0
        phase_noise_offset_stop_hz: 1000000.0
        phase_noise_points_per_decade: 20
        phase_noise_output_expression: {output_expression}
        phase_noise_drive_sources: {phase_noise_drive_sources}
{integration_lines}      stimuli:
        - {{ name: phase_noise_intent, description: Phase-noise planning evidence. }}
      probes:
        - {{ name: out_phase_noise, expression: V(out) }}
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
fn phase_noise_contract_is_schema_valid_and_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_phase_noise_project(
        project_dir.path(),
        "ngspice",
        "driven",
        "V(out)",
        "V1",
        Some(100.0),
        Some(100000.0),
    );
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_PHASE_NOISE_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["adapter_status"],
        "planned_not_implemented"
    );
    assert_eq!(
        report["failures"][0]["measured"]["analysis_kind"],
        "phase_noise"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][0],
        "phase_noise_spectrum"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][1],
        "phase_noise_integrated_jitter"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][2],
        "phase_noise_convergence"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][3],
        "pss_convergence"
    );
    assert_eq!(
        report["failures"][0]["limit"]["trusted_backend_status"],
        "none_available"
    );
    assert_eq!(
        report["failures"][0]["measured"]["backend_research_status"]["qucs_copen"],
        "papers_document_pnsolver_after_psssolver_but_no_public_source_repository_or_adapter_contract_found"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn phase_noise_rejects_unbound_output_expression() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_phase_noise_project(
        project_dir.path(),
        "ngspice",
        "autonomous",
        "V(missing)",
        "",
        None,
        None,
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
fn phase_noise_driven_mode_rejects_missing_generated_drive_source() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_phase_noise_project(
        project_dir.path(),
        "ngspice",
        "driven",
        "V(out)",
        "NO_SUCH_SOURCE",
        None,
        None,
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
fn phase_noise_rejects_integration_range_outside_offset_sweep() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_phase_noise_project(
        project_dir.path(),
        "ngspice",
        "autonomous",
        "V(out)",
        "",
        Some(1.0),
        Some(100000.0),
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("integration range must stay inside")
    );
}
