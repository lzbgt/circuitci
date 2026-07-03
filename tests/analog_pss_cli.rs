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

fn write_pss_project(
    dir: &std::path::Path,
    backend: &str,
    mode: &str,
    output_expression: &str,
    drive_source: &str,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let pss_drive_sources = if drive_source.is_empty() {
        "[]".to_string()
    } else {
        format!("[{drive_source}]")
    };
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: pss_contract, version: 0.1.0 }}
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
  - name: rc_periodic_steady_state
    type: analog_pss
    checks: [SPICE_PSS_ANALYSIS]
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
        type: pss
        pss_mode: {mode}
        pss_frequency_guess_hz: 100000.0
        pss_stabilization_time_us: 100.0
        pss_periods: 2
        pss_output_expression: {output_expression}
        pss_drive_sources: {pss_drive_sources}
        pss_residual_tolerance: 0.000001
        pss_state_error_tolerance: 0.000001
        pss_max_iterations: 80
      stimuli:
        - {{ name: pss_seed, description: Periodic steady-state planning evidence. }}
      probes:
        - {{ name: out_pss, expression: V(out) }}
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
fn pss_contract_is_schema_valid_and_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pss_project(project_dir.path(), "ngspice", "driven", "V(out)", "V1");
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_PSS_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["adapter_status"],
        "planned_not_implemented"
    );
    assert_eq!(
        report["failures"][0]["measured"]["analysis_kind"],
        "periodic_steady_state"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][0],
        "pss_waveform"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][1],
        "pss_spectrum"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][2],
        "pss_convergence"
    );
    assert_eq!(
        report["failures"][0]["limit"]["implemented_backend"],
        "none_yet"
    );
    assert_eq!(
        report["failures"][0]["limit"]["trusted_backend_status"],
        "none_available"
    );
    assert_eq!(
        report["failures"][0]["measured"]["backend_research_status"]["xyce"],
        "no_distinct_pss_command_in_xyce_7_8_docs_hb_only_for_current_runtime"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn pss_rejects_unbound_output_expression() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path =
        write_pss_project(project_dir.path(), "ngspice", "driven", "V(missing)", "V1");

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
fn pss_driven_mode_rejects_missing_generated_drive_source() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pss_project(
        project_dir.path(),
        "ngspice",
        "driven",
        "V(out)",
        "NO_SUCH_SOURCE",
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
fn autonomous_pss_can_plan_without_drive_sources() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_pss_project(project_dir.path(), "ngspice", "autonomous", "V(out)", "");

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_PSS_ANALYSIS");
    assert_eq!(report["failures"][0]["measured"]["pss_mode"], "autonomous");
}
