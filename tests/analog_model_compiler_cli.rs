mod common;

use common::{
    assert_report_schema_valid, assert_yaml_file_valid, binary_available, run_validation_with_path,
    run_validation_with_path_and_env,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;

#[cfg(unix)]
const REAL_NGSPICE_OSDI_CONFORMANCE_ENV: &str = "CIRCUITCI_RUN_REAL_NGSPICE_OSDI";

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

#[cfg(unix)]
fn fake_openvaf_builder(dir: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("openvaf");
    fs::write(
        &path,
        "#!/bin/sh\nprintf 'not-a-real-osdi-binary-but-stable-test-content\\n' > tiny_resistor.osdi\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
}

#[cfg(unix)]
fn fake_openvaf_failure(dir: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("openvaf");
    fs::write(
        &path,
        "#!/bin/sh\necho 'openvaf compile failed' >&2\nexit 7\n",
    )
    .unwrap();
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
    write_model_compiler_project_with_command(
        dir,
        source_sha256,
        artifact_sha256,
        compiler,
        "openvaf tiny_resistor.va -o tiny_resistor.osdi",
    )
}

fn write_model_compiler_project_with_command(
    dir: &std::path::Path,
    source_sha256: Option<&str>,
    artifact_sha256: Option<&str>,
    compiler: Option<&str>,
    compiler_command: &str,
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
          compiler_command: {compiler_command}
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

fn write_model_compiler_transient_project(
    dir: &std::path::Path,
    source_sha256: &str,
    artifact_sha256: &str,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let project = dir.join("project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: model_compiler_osdi_load, version: 0.1.0 }}
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
  nets:
    vin: {{ kind: power, nominal_voltage: 1.0, powered: true }}
    out: {{ kind: digital_or_analog }}
    gnd: {{ kind: ground }}
scenarios:
  - name: model_compiler_transient
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: ngspice
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1]
      model_files:
        - path: tiny_resistor.osdi
          sha256: {artifact_sha256}
          artifact_format: osdi_shared_object
          source_path: tiny_resistor.va
          source_sha256: {source_sha256}
          compiler: openvaf
          compiler_version: 23.5.0-test
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
        type: tran
        stop_time_us: 2.0
        max_step_us: 1.0
      stimuli: []
      probes:
        - {{ name: out, expression: V(out) }}
      assertions:
        - {{ name: out_above_threshold, probe: out, at_us: 1.0, relation: above, threshold_v: 0.4 }}
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy(),
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
        .unwrap_or_else(|| panic!("missing artifact ending with {suffix}"))
}

fn run_validation_with_path_retaining_output(
    project: &str,
    path: &std::path::Path,
) -> (tempfile::TempDir, Value) {
    let out_dir = tempfile::tempdir().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            project,
            "--profile",
            "iot_basic_v0",
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .env("PATH", path)
        .status()
        .unwrap();
    assert!(status.success());
    let report =
        serde_json::from_str(&fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();
    (out_dir, report)
}

fn run_validation_retaining_output_with_env(
    project: &str,
    envs: &[(&str, &str)],
) -> (tempfile::TempDir, Value) {
    let out_dir = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_circuitci"));
    command.args([
        "validate",
        project,
        "--profile",
        "iot_basic_v0",
        "--output",
        out_dir.path().to_str().unwrap(),
    ]);
    for (key, value) in envs {
        command.env(key, value);
    }
    let status = command.status().unwrap();
    assert!(status.success());
    let report =
        serde_json::from_str(&fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();
    (out_dir, report)
}

#[cfg(unix)]
fn real_ngspice_osdi_conformance_enabled() -> bool {
    if std::env::var(REAL_NGSPICE_OSDI_CONFORMANCE_ENV).as_deref() != Ok("1") {
        eprintln!(
            "skipping real-ngspice OSDI conformance; set {REAL_NGSPICE_OSDI_CONFORMANCE_ENV}=1"
        );
        return false;
    }
    if !binary_available("ngspice") {
        eprintln!("skipping real-ngspice OSDI conformance; ngspice is not on PATH");
        return false;
    }
    if !binary_available("openvaf") {
        eprintln!("skipping real-ngspice OSDI conformance; openvaf is not on PATH");
        return false;
    }
    if !ngspice_has_pre_osdi_command() {
        eprintln!("skipping real-ngspice OSDI conformance; ngspice does not accept pre_osdi");
        return false;
    }
    true
}

#[cfg(unix)]
fn ngspice_has_pre_osdi_command() -> bool {
    let dir = tempfile::tempdir().unwrap();
    let deck = dir.path().join("probe.cir");
    fs::write(
        &deck,
        ".control\npre_osdi \"missing-osdi-probe.osdi\"\nquit\n.endc\n.end\n",
    )
    .unwrap();
    let output = Command::new("ngspice")
        .arg("-b")
        .arg(deck.file_name().unwrap())
        .current_dir(dir.path())
        .output();
    let Ok(output) = output else {
        return false;
    };
    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_ascii_lowercase();
    !(log.contains("no such command")
        || log.contains("unknown command")
        || log.contains("undefined command"))
}

#[cfg(unix)]
fn write_real_openvaf_fixture(dir: &std::path::Path) -> (String, String) {
    let source = b"`include \"disciplines.vams\"\nmodule tiny_resistor(p, n);\n  inout p, n;\n  electrical p, n;\n  parameter real r = 1000.0 from (0:inf);\n  analog begin\n    I(p, n) <+ V(p, n) / r;\n  end\nendmodule\n";
    fs::write(dir.join("tiny_resistor.va"), source).unwrap();
    let output = Command::new("openvaf")
        .args(["tiny_resistor.va", "-o", "tiny_resistor.osdi"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "openvaf fixture compile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact = fs::read(dir.join("tiny_resistor.osdi")).unwrap();
    (sha256_hex(source), sha256_hex(&artifact))
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
fn openvaf_osdi_model_is_loaded_with_ngspice_pre_osdi() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'time v(out)\\n0.0 0.5\\n0.000001 0.5\\n' > waveform.csv\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let project_path =
        write_model_compiler_transient_project(project_dir.path(), &source_sha, &artifact_sha);

    let (_out_dir, report) =
        run_validation_with_path_retaining_output(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "pass", "{report:#}");
    let wrapper = fs::read_to_string(artifact_path(&report, "circuitci_ngspice.cir")).unwrap();
    assert!(wrapper.contains("pre_osdi \""));
    assert!(wrapper.contains("tiny_resistor.osdi"));
    assert!(!wrapper.contains(".include \""));
    let generated = fs::read_to_string(artifact_path(&report, "generated_board.cir")).unwrap();
    assert!(!generated.contains("tiny_resistor.osdi"));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["inputs"]["model_files"][0]["artifact_format"],
        "osdi_shared_object"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_reports_ngspice_without_osdi_support() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\necho 'Error: no such command pre_osdi' >&2\nexit 1\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let project_path =
        write_model_compiler_transient_project(project_dir.path(), &source_sha, &artifact_sha);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_TRANSIENT_ANALYSIS");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("OSDI model loading failed")
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_ngspice_osdi_conformance_compiles_and_loads_openvaf_fixture() {
    if !real_ngspice_osdi_conformance_enabled() {
        return;
    }
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_real_openvaf_fixture(project_dir.path());
    fs::remove_file(project_dir.path().join("tiny_resistor.osdi")).unwrap();
    let project_path =
        write_model_compiler_transient_project(project_dir.path(), &source_sha, &artifact_sha);

    let (_out_dir, report) = run_validation_retaining_output_with_env(
        project_path.to_str().unwrap(),
        &[("CIRCUITCI_RUN_OPENVAF_BUILDS", "1")],
    );

    assert_eq!(report["result"], "pass", "{report:#}");
    let rebuilt = fs::read(project_dir.path().join("tiny_resistor.osdi")).unwrap();
    assert_eq!(sha256_hex(&rebuilt), artifact_sha);
    let wrapper = fs::read_to_string(artifact_path(&report, "circuitci_ngspice.cir")).unwrap();
    assert!(wrapper.contains("pre_osdi \""));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["backend"]["selected"], "ngspice");
    assert_eq!(
        manifest["inputs"]["model_files"][0]["artifact_format"],
        "osdi_shared_object"
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

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_reports_build_plan_when_artifact_is_missing() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    fs::remove_file(project_dir.path().join("tiny_resistor.osdi")).unwrap();
    let project_path = write_model_compiler_project(
        project_dir.path(),
        Some(&source_sha),
        Some(&artifact_sha),
        Some("openvaf"),
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_MODEL_COMPILER_ARTIFACT_UNAVAILABLE"
    );
    assert_eq!(
        report["failures"][0]["measured"]["compiler_command"],
        "openvaf tiny_resistor.va -o tiny_resistor.osdi"
    );
    assert_eq!(
        report["failures"][0]["measured"]["compiler_available_on_path"],
        false
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_build_step"],
        "openvaf_compile_osdi_shared_object"
    );
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_reports_build_plan_when_artifact_hash_is_stale() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, _artifact_sha) = write_osdi_files(project_dir.path());
    let wrong_artifact_sha = "1111111111111111111111111111111111111111111111111111111111111111";
    let project_path = write_model_compiler_project(
        project_dir.path(),
        Some(&source_sha),
        Some(wrong_artifact_sha),
        Some("openvaf"),
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_MODEL_COMPILER_ARTIFACT_HASH_MISMATCH"
    );
    assert_eq!(
        report["failures"][0]["limit"]["expected_sha256"],
        wrong_artifact_sha
    );
    assert_eq!(
        report["failures"][0]["limit"]["output_path"],
        "tiny_resistor.osdi"
    );
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_can_rebuild_missing_artifact_when_opted_in() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    fake_openvaf_builder(fake_path.path());
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    fs::remove_file(project_dir.path().join("tiny_resistor.osdi")).unwrap();
    let project_path = write_model_compiler_project(
        project_dir.path(),
        Some(&source_sha),
        Some(&artifact_sha),
        Some("openvaf"),
    );

    let report = run_validation_with_path_and_env(
        project_path.to_str().unwrap(),
        fake_path.path(),
        &[("CIRCUITCI_RUN_OPENVAF_BUILDS", "1")],
    );

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_PSS_ANALYSIS");
    let rebuilt = fs::read(project_dir.path().join("tiny_resistor.osdi")).unwrap();
    assert_eq!(sha256_hex(&rebuilt), artifact_sha);
    assert!(
        report["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("tiny_resistor.osdi"))
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_can_rebuild_hash_stale_artifact_when_opted_in() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    fake_openvaf_builder(fake_path.path());
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    fs::write(
        project_dir.path().join("tiny_resistor.osdi"),
        b"stale artifact\n",
    )
    .unwrap();
    let project_path = write_model_compiler_project(
        project_dir.path(),
        Some(&source_sha),
        Some(&artifact_sha),
        Some("openvaf"),
    );

    let report = run_validation_with_path_and_env(
        project_path.to_str().unwrap(),
        fake_path.path(),
        &[("CIRCUITCI_RUN_OPENVAF_BUILDS", "1")],
    );

    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_PSS_ANALYSIS");
    let rebuilt = fs::read(project_dir.path().join("tiny_resistor.osdi")).unwrap();
    assert_eq!(sha256_hex(&rebuilt), artifact_sha);
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_reports_failed_opt_in_compiler_execution() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    fake_openvaf_failure(fake_path.path());
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    fs::remove_file(project_dir.path().join("tiny_resistor.osdi")).unwrap();
    let project_path = write_model_compiler_project(
        project_dir.path(),
        Some(&source_sha),
        Some(&artifact_sha),
        Some("openvaf"),
    );

    let report = run_validation_with_path_and_env(
        project_path.to_str().unwrap(),
        fake_path.path(),
        &[("CIRCUITCI_RUN_OPENVAF_BUILDS", "1")],
    );

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_MODEL_COMPILER_BUILD_FAILED"
    );
    assert!(
        report["failures"][0]["measured"]["stderr"]
            .as_str()
            .unwrap()
            .contains("openvaf compile failed")
    );
    assert_eq!(
        report["failures"][0]["measured"]["compiler_available_on_path"],
        true
    );
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_requires_command_to_reference_source_and_output() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable(fake_path.path(), "ngspice");
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let project_path = write_model_compiler_project_with_command(
        project_dir.path(),
        Some(&source_sha),
        Some(&artifact_sha),
        Some("openvaf"),
        "openvaf other.va -o other.osdi",
    );

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_MODEL_COMPILER_COMMAND_MISMATCH"
    );
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("tiny_resistor.va")
    );
}
