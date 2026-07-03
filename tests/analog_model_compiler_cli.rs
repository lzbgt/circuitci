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
const REAL_XYCE_ADMS_PLUGIN_CONFORMANCE_ENV: &str = "CIRCUITCI_RUN_REAL_XYCE_ADMS_PLUGIN";

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

fn write_model_package_lock(
    dir: &std::path::Path,
    artifact_id: &str,
    artifact_path: &str,
    artifact_sha256: &str,
    artifact_format: &str,
    compiler: &str,
) -> String {
    let lock = format!(
        r#"package:
  name: org.circuitci.test.tiny_resistor
  version: 1.0.0
artifacts:
  - id: {artifact_id}
    path: {artifact_path}
    sha256: {artifact_sha256}
    artifact_format: {artifact_format}
    compiler: {compiler}
"#
    );
    fs::write(dir.join("compact_model.lock.yaml"), lock.as_bytes()).unwrap();
    sha256_hex(lock.as_bytes())
}

fn write_model_package_registry(
    dir: &std::path::Path,
    entry_id: &str,
    artifact_id: &str,
    lock_sha256: &str,
) -> String {
    let registry = format!(
        r#"packages:
  - id: {entry_id}
    package:
      name: org.circuitci.test.tiny_resistor
      version: 1.0.0
    artifact_id: {artifact_id}
    lock_path: compact_model.lock.yaml
    lock_sha256: {lock_sha256}
"#
    );
    fs::write(dir.join("compact_model_registry.yaml"), registry.as_bytes()).unwrap();
    sha256_hex(registry.as_bytes())
}

fn add_model_package_lock_to_project(
    project_path: &std::path::Path,
    artifact_id: &str,
    lock_sha256: &str,
) {
    let project = fs::read_to_string(project_path).unwrap();
    let updated = project.replace(
        "          compiler_command: openvaf tiny_resistor.va -o tiny_resistor.osdi\n",
        &format!(
            "          compiler_command: openvaf tiny_resistor.va -o tiny_resistor.osdi\n          model_package_name: org.circuitci.test.tiny_resistor\n          model_package_version: 1.0.0\n          model_package_artifact_id: {artifact_id}\n          model_package_lock_path: compact_model.lock.yaml\n          model_package_lock_sha256: {lock_sha256}\n"
        ),
    );
    fs::write(project_path, updated).unwrap();
}

fn add_model_package_registry_to_project(
    project_path: &std::path::Path,
    entry_id: &str,
    registry_sha256: &str,
) {
    let project = fs::read_to_string(project_path).unwrap();
    let updated = project.replace(
        "          compiler_command: openvaf tiny_resistor.va -o tiny_resistor.osdi\n",
        &format!(
            "          compiler_command: openvaf tiny_resistor.va -o tiny_resistor.osdi\n          model_package_registry_path: compact_model_registry.yaml\n          model_package_registry_sha256: {registry_sha256}\n          model_package_registry_entry: {entry_id}\n"
        ),
    );
    fs::write(project_path, updated).unwrap();
}

fn write_xyce_adms_plugin_files(dir: &std::path::Path) -> (String, String, String) {
    let source = b"`include \"disciplines.vams\"\nmodule tiny_xyce_resistor(p, n); endmodule\n";
    let plugin = b"not-a-real-xyce-plugin-but-stable-test-content\n";
    let conformance = br#"{"solver":"xyce","plugin":"tiny_xyce_plugin.so","status":"planned"}"#;
    fs::write(dir.join("tiny_xyce_resistor.va"), source).unwrap();
    fs::write(dir.join("tiny_xyce_plugin.so"), plugin).unwrap();
    fs::write(dir.join("xyce_plugin_conformance.json"), conformance).unwrap();
    (
        sha256_hex(source),
        sha256_hex(plugin),
        sha256_hex(conformance),
    )
}

fn write_xyce_adms_plugin_project(
    dir: &std::path::Path,
    source_sha256: &str,
    plugin_sha256: &str,
    conformance_sha256: &str,
    configure_options: &[&str],
) -> std::path::PathBuf {
    write_xyce_adms_plugin_project_with_commands(
        dir,
        source_sha256,
        plugin_sha256,
        conformance_sha256,
        configure_options,
        "buildxyceplugin tiny_xyce_resistor.va tiny_xyce_plugin.so",
        "Xyce -plugin tiny_xyce_plugin.so circuit.cir",
    )
}

fn write_xyce_adms_plugin_project_with_commands(
    dir: &std::path::Path,
    source_sha256: &str,
    plugin_sha256: &str,
    conformance_sha256: &str,
    configure_options: &[&str],
    compiler_command: &str,
    plugin_load_command: &str,
) -> std::path::PathBuf {
    let repo = std::env::current_dir().unwrap();
    let configure_options = configure_options
        .iter()
        .map(|option| format!("            - {option}\n"))
        .collect::<String>();
    let project = dir.join("xyce_plugin_project.yaml");
    fs::write(
        &project,
        format!(
            r#"project: {{ name: xyce_adms_plugin_contract, version: 0.1.0 }}
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
  - name: xyce_adms_plugin_planning
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: xyce
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1]
      model_files:
        - path: tiny_xyce_plugin.so
          sha256: {plugin_sha256}
          artifact_format: xyce_adms_plugin
          source_path: tiny_xyce_resistor.va
          source_sha256: {source_sha256}
          compiler: xyce_adms
          compiler_version: xyce-7.8-adms-test
          compiler_command: {compiler_command}
          plugin_load_command: {plugin_load_command}
          xyce_version: 7.8-test
          xyce_adms_template_revision: xyce-7.8-utils-ADMS-test
          xyce_configure_options:
{configure_options}          conformance_artifact: xyce_plugin_conformance.json
          conformance_sha256: {conformance_sha256}
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
        stop_time_us: 1.0
        max_step_us: 0.5
      stimuli: []
      probes:
        - {{ name: out, expression: V(out) }}
      assertions: []
"#,
            libs = repo.join("libs/generic/analog").to_string_lossy(),
        ),
    )
    .unwrap();
    project
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
    run_validation_with_path_and_env_retaining_output(project, path, &[])
}

fn run_validation_with_path_and_env_retaining_output(
    project: &str,
    path: &std::path::Path,
    envs: &[(&str, &str)],
) -> (tempfile::TempDir, Value) {
    let out_dir = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_circuitci"));
    command
        .args([
            "validate",
            project,
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
fn xyce_binary() -> Option<&'static str> {
    if binary_available("Xyce") {
        Some("Xyce")
    } else if binary_available("xyce") {
        Some("xyce")
    } else {
        None
    }
}

#[cfg(unix)]
fn real_xyce_adms_plugin_conformance_enabled() -> bool {
    if std::env::var(REAL_XYCE_ADMS_PLUGIN_CONFORMANCE_ENV).as_deref() != Ok("1") {
        eprintln!(
            "skipping real-Xyce ADMS plugin conformance; set {REAL_XYCE_ADMS_PLUGIN_CONFORMANCE_ENV}=1"
        );
        return false;
    }
    if xyce_binary().is_none() {
        eprintln!("skipping real-Xyce ADMS plugin conformance; Xyce/xyce is not on PATH");
        return false;
    }
    if !binary_available("buildxyceplugin") {
        eprintln!("skipping real-Xyce ADMS plugin conformance; buildxyceplugin is not on PATH");
        return false;
    }
    true
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
fn write_real_xyce_adms_plugin_fixture(dir: &std::path::Path) -> (String, String, String) {
    let source = b"`include \"disciplines.vams\"\n`include \"constants.vams\"\n\n`define attr(txt) (*txt*)\n\nmodule rlc (p,n) `attr(xyceSpiceDeviceName=\"RLC\" xyceLevelNumber=\"1\");\n  electrical p,n;\n  inout p,n;\n  electrical internal1, internal2;\n\n  parameter real L=1e-3 from (0:inf) `attr(info=\"Inductance\" type=\"instance\");\n  parameter real R=1e3 from (0:inf) `attr(info=\"Resistance\" type=\"instance\");\n  parameter real C=1e-12 from (0:inf) `attr(info=\"Capacitance\" type=\"instance\");\n  real InductorCurrent;\n  real CapacitorCharge;\n\n  analog begin\n    I(p,internal1) <+ V(p,internal1)/R;\n    CapacitorCharge = V(internal1,internal2)*C;\n    I(internal1,internal2) <+ ddt(CapacitorCharge);\n    InductorCurrent=I(internal2,n);\n    V(internal2,n) <+ L*ddt(InductorCurrent);\n  end\nendmodule\n";
    fs::write(dir.join("tiny_xyce_resistor.va"), source).unwrap();
    let build_output = Command::new("buildxyceplugin")
        .args(["-o", "tiny_xyce_plugin", "tiny_xyce_resistor.va", "."])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        build_output.status.success(),
        "buildxyceplugin fixture compile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build_output.stdout),
        String::from_utf8_lossy(&build_output.stderr)
    );
    let built_plugin = find_xyce_plugin_output(dir);
    let normalized_plugin = dir.join("tiny_xyce_plugin.so");
    if built_plugin != normalized_plugin {
        fs::copy(&built_plugin, &normalized_plugin).unwrap();
    }
    let deck = b"Test of Xyce ADMS plugin load\nV1 1 0 SIN (5v 5v 20MEG)\nYrlc rlc1 1 0 R=1kohm L=1mH C=1pf\n.tran 1n 4u\n.print tran v(1) I(v1)\n.end\n";
    fs::write(dir.join("rlc_series.cir"), deck).unwrap();
    let xyce = xyce_binary().expect("real Xyce conformance requires Xyce binary");
    let conformance_output = Command::new(xyce)
        .args([
            "-plugin",
            normalized_plugin.to_str().unwrap(),
            "rlc_series.cir",
        ])
        .current_dir(dir)
        .output()
        .unwrap();
    let conformance_log = format!(
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        conformance_output.status,
        String::from_utf8_lossy(&conformance_output.stdout),
        String::from_utf8_lossy(&conformance_output.stderr)
    );
    fs::write(dir.join("xyce_plugin_conformance.json"), &conformance_log).unwrap();
    assert!(
        conformance_output.status.success(),
        "Xyce plugin conformance run failed\n{conformance_log}"
    );
    let plugin = fs::read(normalized_plugin).unwrap();
    (
        sha256_hex(source),
        sha256_hex(&plugin),
        sha256_hex(conformance_log.as_bytes()),
    )
}

#[cfg(unix)]
fn find_xyce_plugin_output(dir: &std::path::Path) -> std::path::PathBuf {
    for candidate in [
        dir.join("tiny_xyce_plugin.so"),
        dir.join("libtiny_xyce_plugin.so"),
        dir.join(".libs/tiny_xyce_plugin.so"),
        dir.join(".libs/libtiny_xyce_plugin.so"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    for subdir in [dir.to_path_buf(), dir.join(".libs")] {
        if let Ok(entries) = fs::read_dir(&subdir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                if path.is_file()
                    && file_name.ends_with(".so")
                    && file_name.contains("tiny_xyce_plugin")
                {
                    return path;
                }
            }
        }
    }
    panic!("buildxyceplugin did not produce a tiny_xyce_plugin shared library");
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

    let (out_dir, report) =
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
    let provenance = &manifest["inputs"]["model_file_provenance"][0];
    assert_eq!(provenance["model_file"], "tiny_resistor.osdi");
    assert_eq!(provenance["source_sha256_declared"], source_sha);
    assert_eq!(provenance["source_sha256_actual"], source_sha);
    assert_eq!(provenance["artifact_sha256_declared"], artifact_sha);
    assert_eq!(provenance["artifact_sha256_actual"], artifact_sha);
    assert_eq!(provenance["rebuild_mode"], "prebuilt_verified");
    assert_eq!(provenance["produced_by_circuitci"], false);
    let report_provenance = &report["model_file_provenance"][0];
    assert_eq!(report_provenance["scenario"], "model_compiler_transient");
    assert_eq!(report_provenance["analysis"], "transient");
    assert_eq!(report_provenance["backend"], "ngspice");
    assert_eq!(report_provenance["model_file"], "tiny_resistor.osdi");
    assert_eq!(report_provenance["source_sha256_actual"], source_sha);
    assert_eq!(report_provenance["artifact_sha256_actual"], artifact_sha);
    assert_eq!(report_provenance["rebuild_mode"], "prebuilt_verified");
    assert_eq!(report_provenance["produced_by_circuitci"], false);
    let markdown = fs::read_to_string(out_dir.path().join("report.md")).unwrap();
    assert!(markdown.contains("## Model File Provenance"));
    assert!(markdown.contains("`tiny_resistor.osdi`"));
    assert!(markdown.contains("`prebuilt_verified`"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_package_lock_is_recorded_in_manifest_and_report() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'time v(out)\\n0.0 0.5\\n0.000001 0.5\\n' > waveform.csv\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let lock_sha = write_model_package_lock(
        project_dir.path(),
        "tiny_resistor_osdi",
        "tiny_resistor.osdi",
        &artifact_sha,
        "osdi_shared_object",
        "openvaf",
    );
    let project_path =
        write_model_compiler_transient_project(project_dir.path(), &source_sha, &artifact_sha);
    add_model_package_lock_to_project(&project_path, "tiny_resistor_osdi", &lock_sha);

    let (out_dir, report) =
        run_validation_with_path_retaining_output(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "pass", "{report:#}");
    assert!(
        report["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact
                .as_str()
                .unwrap()
                .ends_with("compact_model.lock.yaml"))
    );
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    let provenance = &manifest["inputs"]["model_file_provenance"][0];
    assert_eq!(
        provenance["model_package_name"],
        "org.circuitci.test.tiny_resistor"
    );
    assert_eq!(provenance["model_package_version"], "1.0.0");
    assert_eq!(
        provenance["model_package_artifact_id"],
        "tiny_resistor_osdi"
    );
    assert_eq!(
        provenance["model_package_lock_path"],
        "compact_model.lock.yaml"
    );
    assert_eq!(provenance["model_package_lock_sha256"], lock_sha);
    let report_provenance = &report["model_file_provenance"][0];
    assert_eq!(
        report_provenance["model_package_name"],
        "org.circuitci.test.tiny_resistor"
    );
    assert_eq!(
        report_provenance["model_package_artifact_id"],
        "tiny_resistor_osdi"
    );
    let markdown = fs::read_to_string(out_dir.path().join("report.md")).unwrap();
    assert!(markdown.contains("Package: `org.circuitci.test.tiny_resistor`"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_package_registry_import_is_recorded_in_manifest_and_report() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'time v(out)\\n0.0 0.5\\n0.000001 0.5\\n' > waveform.csv\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let lock_sha = write_model_package_lock(
        project_dir.path(),
        "tiny_resistor_osdi",
        "tiny_resistor.osdi",
        &artifact_sha,
        "osdi_shared_object",
        "openvaf",
    );
    let registry_sha = write_model_package_registry(
        project_dir.path(),
        "tiny_resistor_qualified_osdi",
        "tiny_resistor_osdi",
        &lock_sha,
    );
    let project_path =
        write_model_compiler_transient_project(project_dir.path(), &source_sha, &artifact_sha);
    add_model_package_registry_to_project(
        &project_path,
        "tiny_resistor_qualified_osdi",
        &registry_sha,
    );

    let (out_dir, report) =
        run_validation_with_path_retaining_output(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "pass", "{report:#}");
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("compact_model.lock.yaml")
    }));
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("compact_model_registry.yaml")
    }));
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    let provenance = &manifest["inputs"]["model_file_provenance"][0];
    assert_eq!(
        provenance["model_package_registry_path"],
        "compact_model_registry.yaml"
    );
    assert_eq!(provenance["model_package_registry_sha256"], registry_sha);
    assert_eq!(
        provenance["model_package_registry_entry"],
        "tiny_resistor_qualified_osdi"
    );
    let report_provenance = &report["model_file_provenance"][0];
    assert_eq!(
        report_provenance["model_package_registry_entry"],
        "tiny_resistor_qualified_osdi"
    );
    let markdown = fs::read_to_string(out_dir.path().join("report.md")).unwrap();
    assert!(markdown.contains("Registry: `compact_model_registry.yaml`"));
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_package_registry_rejects_explicit_metadata_mismatch() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'time v(out)\\n0.0 0.5\\n0.000001 0.5\\n' > waveform.csv\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let lock_sha = write_model_package_lock(
        project_dir.path(),
        "tiny_resistor_osdi",
        "tiny_resistor.osdi",
        &artifact_sha,
        "osdi_shared_object",
        "openvaf",
    );
    let registry_sha = write_model_package_registry(
        project_dir.path(),
        "tiny_resistor_qualified_osdi",
        "tiny_resistor_osdi",
        &lock_sha,
    );
    let project_path =
        write_model_compiler_transient_project(project_dir.path(), &source_sha, &artifact_sha);
    add_model_package_registry_to_project(
        &project_path,
        "tiny_resistor_qualified_osdi",
        &registry_sha,
    );
    let project = fs::read_to_string(&project_path).unwrap();
    fs::write(
        &project_path,
        project.replace(
            "          model_package_registry_entry: tiny_resistor_qualified_osdi\n",
            "          model_package_registry_entry: tiny_resistor_qualified_osdi\n          model_package_version: 2.0.0\n",
        ),
    )
    .unwrap();

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_MODEL_PACKAGE_REGISTRY_ENTRY_MISMATCH"
    );
    assert_eq!(
        report["failures"][0]["limit"]["mismatch"],
        "model_package_version"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_model_package_lock_rejects_mismatched_artifact_path() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'time v(out)\\n0.0 0.5\\n0.000001 0.5\\n' > waveform.csv\n",
    );
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let lock_sha = write_model_package_lock(
        project_dir.path(),
        "tiny_resistor_osdi",
        "other_resistor.osdi",
        &artifact_sha,
        "osdi_shared_object",
        "openvaf",
    );
    let project_path =
        write_model_compiler_transient_project(project_dir.path(), &source_sha, &artifact_sha);
    add_model_package_lock_to_project(&project_path, "tiny_resistor_osdi", &lock_sha);

    let report = run_validation_with_path(project_path.to_str().unwrap(), fake_path.path());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_MODEL_PACKAGE_LOCK_ARTIFACT_MISMATCH"
    );
    assert_eq!(report["failures"][0]["limit"]["mismatch"], "artifact_path");
    assert_eq!(
        report["failures"][0]["measured"]["model_package_artifact_id"],
        "tiny_resistor_osdi"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn openvaf_osdi_model_rejects_explicit_xyce_backend() {
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    let project_path =
        write_model_compiler_transient_project(project_dir.path(), &source_sha, &artifact_sha);
    let project = fs::read_to_string(&project_path)
        .unwrap()
        .replace("backend: ngspice", "backend: xyce");
    fs::write(&project_path, project).unwrap();

    let report = common::run_validation(project_path.to_str().unwrap());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == "ANALOG_MODEL_COMPILER_BACKEND_UNSUPPORTED")
        .expect("missing backend compatibility finding");
    assert_eq!(failure["measured"]["model_file"], "tiny_resistor.osdi");
    assert_eq!(failure["measured"]["requested_backend"], "xyce");
    assert_eq!(
        failure["limit"]["supported_backend"],
        "external_ngspice_with_pre_osdi"
    );
    assert_eq!(report["model_file_provenance"].as_array().unwrap().len(), 0);
    assert_report_schema_valid(&report);
}

#[test]
fn xyce_adms_plugin_contract_is_planned_not_executed() {
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, plugin_sha, conformance_sha) =
        write_xyce_adms_plugin_files(project_dir.path());
    let project_path = write_xyce_adms_plugin_project(
        project_dir.path(),
        &source_sha,
        &plugin_sha,
        &conformance_sha,
        &["--enable-shared", "--enable-xyce-shareable"],
    );
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&project_path, &validator);

    let report = common::run_validation(project_path.to_str().unwrap());

    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == "ANALOG_MODEL_COMPILER_XYCE_PLUGIN_UNSUPPORTED")
        .expect("missing Xyce/ADMS plugin planning finding");
    assert_eq!(failure["measured"]["model_file"], "tiny_xyce_plugin.so");
    assert_eq!(failure["measured"]["artifact_format"], "xyce_adms_plugin");
    assert_eq!(failure["measured"]["compiler"], "xyce_adms");
    assert_eq!(
        failure["measured"]["plugin_load_command"],
        "Xyce -plugin tiny_xyce_plugin.so circuit.cir"
    );
    assert_eq!(
        failure["limit"]["required_backend_adapter"],
        "xyce_adms_plugin_loader"
    );
    assert_eq!(
        failure["limit"]["required_conformance"],
        "real_xyce_plugin_load"
    );
    assert!(
        report["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact
                .as_str()
                .unwrap()
                .ends_with("tiny_xyce_resistor.va"))
    );
    assert!(
        report["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("tiny_xyce_plugin.so"))
    );
    assert!(
        report["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact
                .as_str()
                .unwrap()
                .ends_with("xyce_plugin_conformance.json"))
    );
    assert_report_schema_valid(&report);
}

#[test]
fn xyce_adms_plugin_contract_requires_shareable_xyce_build_options() {
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, plugin_sha, conformance_sha) =
        write_xyce_adms_plugin_files(project_dir.path());
    let project_path = write_xyce_adms_plugin_project(
        project_dir.path(),
        &source_sha,
        &plugin_sha,
        &conformance_sha,
        &["--enable-shared"],
    );

    let report = common::run_validation(project_path.to_str().unwrap());

    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_MODEL_COMPILER_PROVENANCE_MISSING"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_field"],
        "xyce_configure_options"
    );
    assert_eq!(
        report["failures"][0]["limit"]["required_configure_option"],
        "--enable-xyce-shareable"
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn real_xyce_adms_plugin_conformance_builds_loads_and_records_contract_when_enabled() {
    if !real_xyce_adms_plugin_conformance_enabled() {
        return;
    }
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, plugin_sha, conformance_sha) =
        write_real_xyce_adms_plugin_fixture(project_dir.path());
    let project_path = write_xyce_adms_plugin_project_with_commands(
        project_dir.path(),
        &source_sha,
        &plugin_sha,
        &conformance_sha,
        &["--enable-shared", "--enable-xyce-shareable"],
        "buildxyceplugin -o tiny_xyce_plugin tiny_xyce_resistor.va .",
        "Xyce -plugin tiny_xyce_plugin.so rlc_series.cir",
    );

    let report = common::run_validation(project_path.to_str().unwrap());

    assert_eq!(report["result"], "fail", "{report:#}");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == "ANALOG_MODEL_COMPILER_XYCE_PLUGIN_UNSUPPORTED")
        .expect("missing Xyce/ADMS plugin planning finding");
    assert_eq!(
        failure["measured"]["compiler_command"],
        "buildxyceplugin -o tiny_xyce_plugin tiny_xyce_resistor.va ."
    );
    assert_eq!(
        failure["measured"]["plugin_load_command"],
        "Xyce -plugin tiny_xyce_plugin.so rlc_series.cir"
    );
    assert_eq!(
        failure["measured"]["conformance_artifact"],
        "xyce_plugin_conformance.json"
    );
    assert!(
        report["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact
                .as_str()
                .unwrap()
                .ends_with("xyce_plugin_conformance.json"))
    );
    assert_report_schema_valid(&report);
}

#[cfg(unix)]
#[test]
fn openvaf_osdi_rebuild_is_recorded_in_solver_manifest() {
    let fake_path = tempfile::tempdir().unwrap();
    fake_executable_with_body(
        fake_path.path(),
        "ngspice",
        "#!/bin/sh\nprintf 'time v(out)\\n0.0 0.5\\n0.000001 0.5\\n' > waveform.csv\n",
    );
    fake_openvaf_builder(fake_path.path());
    let project_dir = tempfile::tempdir().unwrap();
    let (source_sha, artifact_sha) = write_osdi_files(project_dir.path());
    fs::remove_file(project_dir.path().join("tiny_resistor.osdi")).unwrap();
    let project_path =
        write_model_compiler_transient_project(project_dir.path(), &source_sha, &artifact_sha);

    let (_out_dir, report) = run_validation_with_path_and_env_retaining_output(
        project_path.to_str().unwrap(),
        fake_path.path(),
        &[("CIRCUITCI_RUN_OPENVAF_BUILDS", "1")],
    );

    assert_eq!(report["result"], "pass", "{report:#}");
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(artifact_path(&report, "solver_manifest.json")).unwrap(),
    )
    .unwrap();
    let provenance = &manifest["inputs"]["model_file_provenance"][0];
    assert_eq!(provenance["model_file"], "tiny_resistor.osdi");
    assert_eq!(provenance["source_sha256_declared"], source_sha);
    assert_eq!(provenance["source_sha256_actual"], source_sha);
    assert_eq!(provenance["artifact_sha256_declared"], artifact_sha);
    assert_eq!(provenance["artifact_sha256_actual"], artifact_sha);
    assert_eq!(provenance["compiler_available_on_path"], true);
    assert_eq!(provenance["build_env_enabled"], true);
    assert_eq!(provenance["rebuild_mode"], "rebuilt_missing_artifact");
    assert_eq!(provenance["produced_by_circuitci"], true);
    let report_provenance = &report["model_file_provenance"][0];
    assert!(
        report_provenance["manifest"]
            .as_str()
            .unwrap()
            .ends_with("solver_manifest.json")
    );
    assert_eq!(report_provenance["compiler_available_on_path"], true);
    assert_eq!(report_provenance["build_env_enabled"], true);
    assert_eq!(
        report_provenance["rebuild_mode"],
        "rebuilt_missing_artifact"
    );
    assert_eq!(report_provenance["produced_by_circuitci"], true);
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
