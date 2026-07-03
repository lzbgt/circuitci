mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation_with_path};
use serde_json::Value;
use sha2::{Digest, Sha256};
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

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_osdi_files(dir: &std::path::Path) -> (String, String) {
    let source = b"`include \"disciplines.vams\"\nmodule tiny_resistor(p, n); endmodule\n";
    let artifact = b"not-a-real-osdi-binary-but-stable-test-content\n";
    fs::write(dir.join("tiny_resistor.va"), source).unwrap();
    fs::write(dir.join("tiny_resistor.osdi"), artifact).unwrap();
    (sha256_hex(source), sha256_hex(artifact))
}

fn write_model_compiler_project(
    dir: &std::path::Path,
    source_sha256: Option<&str>,
    artifact_sha256: Option<&str>,
    compiler: Option<&str>,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let source_sha256 = source_sha256
        .map(|sha| format!("          source_sha256: {sha}\n"))
        .unwrap_or_default();
    let artifact_sha256 = artifact_sha256
        .map(|sha| format!("          sha256: {sha}\n"))
        .unwrap_or_default();
    let compiler = compiler
        .map(|compiler| format!("          compiler: {compiler}\n"))
        .unwrap_or_default();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: model_compiler_contract, version: 0.1.0 }}
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
  nets:
    vin: {{ kind: power, nominal_voltage: 1.0, powered: true }}
    out: {{ kind: digital_or_analog }}
    gnd: {{ kind: ground }}
scenarios:
  - name: model_compiler_pss
    type: analog_pss
    checks: [SPICE_PSS_ANALYSIS]
    analog:
      backend: ngspice
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1]
      model_files:
        - path: tiny_resistor.osdi
{artifact_sha256}          artifact_format: osdi_shared_object
          source_path: tiny_resistor.va
{source_sha256}{compiler}          compiler_version: 23.5.0-test
          compiler_command: openvaf tiny_resistor.va -o tiny_resistor.osdi
      node_bindings:
        - {{ node: vin, net: vin }}
        - {{ node: out, net: out }}
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: vin, endpoint: {{ component: V1, pin: P }} }}
        - {{ node: "0", endpoint: {{ component: V1, pin: N }} }}
        - {{ node: vin, endpoint: {{ component: R1, pin: A }} }}
        - {{ node: out, endpoint: {{ component: R1, pin: B }} }}
      analysis:
        type: pss
        pss_mode: driven
        pss_frequency_guess_hz: 100000.0
        pss_stabilization_time_us: 100.0
        pss_output_expression: V(out)
        pss_drive_sources: [V1]
      stimuli:
        - {{ name: model_compiler, description: OpenVAF provenance planning evidence. }}
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
fn openvaf_osdi_model_provenance_is_schema_valid_and_reaches_analysis_planning() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let project_path = write_model_compiler_project(
        project_dir.path(),
        Some(&source_sha),
        Some(&artifact_sha),
        Some("openvaf"),
    );
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_PSS_ANALYSIS");
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("tiny_resistor.va"))
    );
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("tiny_resistor.osdi"))
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_requires_source_hash_pin() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let (_source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let project_path = write_model_compiler_project(
        project_dir.path(),
        None,
        Some(&artifact_sha),
        Some("openvaf"),
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_MODEL_COMPILER_PROVENANCE_MISSING"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_field"],
        "source_sha256"
    );
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_rejects_source_hash_mismatch() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let (_source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let wrong_source_sha = "0000000000000000000000000000000000000000000000000000000000000000";
    let project_path = write_model_compiler_project(
        project_dir.path(),
        Some(wrong_source_sha),
        Some(&artifact_sha),
        Some("openvaf"),
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_MODEL_SOURCE_HASH_MISMATCH"
    );
    assert_eq!(
        report["failures"][0]["limit"]["expected_sha256"],
        wrong_source_sha
    );
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_requires_openvaf_compiler_identity() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let project_path = write_model_compiler_project(
        project_dir.path(),
        Some(&source_sha),
        Some(&artifact_sha),
        None,
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_MODEL_COMPILER_PROVENANCE_MISSING"
    );
    assert_eq!(report["failures"][0]["limit"]["required_field"], "compiler");
}
