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

fn write_periodic_ac_project(
    dir: &std::path::Path,
    backend: &str,
    mode: &str,
    output_expression: &str,
    input_source: &str,
    start_frequency_hz: f64,
    stop_frequency_hz: f64,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: periodic_ac_contract, version: 0.1.0 }}
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
    VINAC:
      model: generic.analog.dc_voltage_source
      pins: {{ P: in, N: gnd }}
      spice: {{ primitive: dc_voltage_source, dc_v: 0.01 }}
    R1:
      model: generic.analog.resistor
      pins: {{ A: in, B: out }}
      spice: {{ primitive: resistor, value_ohm: 1000 }}
    C1:
      model: generic.analog.capacitor
      pins: {{ A: out, B: gnd }}
      spice: {{ primitive: capacitor, value_f: 0.000001 }}
  nets:
    vin: {{ kind: digital_or_analog }}
    in: {{ kind: power, nominal_voltage: 0.01, powered: true }}
    out: {{ kind: digital_or_analog }}
    gnd: {{ kind: ground }}
scenarios:
  - name: rc_periodic_ac
    type: analog_periodic_ac
    checks: [SPICE_PERIODIC_AC_ANALYSIS]
    analog:
      backend: {backend}
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, VINAC, R1, C1]
      model_files: []
      node_bindings:
        - {{ node: vin, net: vin }}
        - {{ node: in, net: in }}
        - {{ node: out, net: out }}
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: vin, endpoint: {{ component: V1, pin: P }} }}
        - {{ node: "0", endpoint: {{ component: V1, pin: N }} }}
        - {{ node: in, endpoint: {{ component: VINAC, pin: P }} }}
        - {{ node: "0", endpoint: {{ component: VINAC, pin: N }} }}
        - {{ node: in, endpoint: {{ component: R1, pin: A }} }}
        - {{ node: out, endpoint: {{ component: R1, pin: B }} }}
        - {{ node: out, endpoint: {{ component: C1, pin: A }} }}
        - {{ node: "0", endpoint: {{ component: C1, pin: B }} }}
      analysis:
        type: pac
        pac_mode: {mode}
        pac_carrier_frequency_hz: 100000.0
        pac_start_frequency_hz: {start_frequency_hz}
        pac_stop_frequency_hz: {stop_frequency_hz}
        pac_points_per_decade: 20
        pac_output_expression: {output_expression}
        pac_input_source: {input_source}
        pac_sidebands: 2
        pac_drive_sources: [V1]
      stimuli:
        - {{ name: periodic_ac_intent, description: Periodic small-signal planning evidence. }}
      probes:
        - {{ name: out_pac, expression: V(out) }}
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
fn periodic_ac_contract_is_schema_valid_and_fails_closed_with_planning_evidence() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_periodic_ac_project(
        project_dir.path(),
        "ngspice",
        "pac",
        "V(out)",
        "VINAC",
        10.0,
        1_000_000.0,
    );
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_PERIODIC_AC_ANALYSIS");
    assert_eq!(
        report["failures"][0]["measured"]["adapter_status"],
        "planned_not_implemented"
    );
    assert_eq!(
        report["failures"][0]["measured"]["analysis_kind"],
        "periodic_ac"
    );
    assert_eq!(report["failures"][0]["measured"]["pac_mode"], "pac");
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][0],
        "pac_response"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][1],
        "pac_sidebands"
    );
    assert_eq!(
        report["failures"][0]["measured"]["required_normalized_outputs"][2],
        "pac_convergence"
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
        report["failures"][0]["measured"]["backend_research_status"]["ngspice"],
        "manual_mentions_pac_as_future_pss_downstream_analysis_but_pss_is_experimental_autonomous_only_and_no_pac_command_contract_is_documented"
    );
    assert_eq!(
        report["failures"][0]["measured"]["source_notes"]["xyce_reference"],
        "sources/Xyce_Reference_Guide_7.8.txt"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn periodic_ac_rejects_unbound_output_expression() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_periodic_ac_project(
        project_dir.path(),
        "ngspice",
        "pac",
        "V(missing)",
        "VINAC",
        10.0,
        1_000_000.0,
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
fn periodic_ac_rejects_missing_generated_input_source() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_periodic_ac_project(
        project_dir.path(),
        "ngspice",
        "pxf",
        "V(out)",
        "NO_SUCH_SOURCE",
        10.0,
        1_000_000.0,
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
fn periodic_ac_rejects_invalid_frequency_window() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let project_path = write_periodic_ac_project(
        project_dir.path(),
        "ngspice",
        "pac",
        "V(out)",
        "VINAC",
        1000.0,
        100.0,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("pac_stop_frequency_hz greater than pac_start_frequency_hz")
    );
}
