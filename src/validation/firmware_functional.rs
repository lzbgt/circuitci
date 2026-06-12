use crate::board_ir::{
    FirmwareBackend, FirmwareBuildSpec, FirmwareScenario, PinLogicState, PinMode, PinState,
    QemuFirmwareOptions, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::FUNCTIONAL_MCU_FIRMWARE;
use super::analog_util::{executable_on_path, push_artifact, safe_artifact_name};
use super::common::{target_model, validation_input_missing};

const DEFAULT_QEMU_EXECUTABLE: &str = "qemu-system-arm";
const DEFAULT_FIRMWARE_BUILD_TIMEOUT_MS: u64 = 120_000;
const DEFAULT_QEMU_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_PIN_TRACE_PREFIX: &str = "CIRCUITCI_PIN ";

pub(super) fn validate_functional_mcu_firmware(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    artifacts: &mut Vec<String>,
    output: &Path,
) {
    let Some((target_component, model)) = target_model(bound, scenario) else {
        validation_input_missing(
            findings,
            scenario,
            "firmware_in_loop target component and model are required.",
        );
        return;
    };
    let Some(firmware) = &scenario.firmware else {
        validation_input_missing(
            findings,
            scenario,
            "firmware_in_loop firmware block is required.",
        );
        return;
    };
    if firmware.image.trim().is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "firmware_in_loop firmware.image is required.",
        );
        return;
    }
    if firmware.expected_pin_states.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "firmware_in_loop firmware.expected_pin_states must declare board-facing pin behavior to validate.",
        );
        return;
    }

    match selected_backend(firmware) {
        SelectedFirmwareBackend::Qemu => validate_qemu_firmware(
            FirmwareValidationContext {
                bound,
                scenario,
                firmware,
                target_component: &target_component,
                target_model: model.component_id.as_str(),
                output,
            },
            findings,
            artifacts,
        ),
        SelectedFirmwareBackend::Unavailable(message) => {
            findings.push(unavailable_finding(
                scenario,
                &target_component,
                model.component_id.as_str(),
                firmware,
                message,
            ));
        }
    }
}

struct FirmwareValidationContext<'a> {
    bound: &'a BoundBoard<'a>,
    scenario: &'a Scenario,
    firmware: &'a FirmwareScenario,
    target_component: &'a str,
    target_model: &'a str,
    output: &'a Path,
}

fn validate_qemu_firmware(
    context: FirmwareValidationContext<'_>,
    findings: &mut Vec<Finding>,
    artifacts: &mut Vec<String>,
) {
    let FirmwareValidationContext {
        bound,
        scenario,
        firmware,
        target_component,
        target_model,
        output,
    } = context;

    let scenario_dir = output.join(format!("firmware_{}", safe_artifact_name(&scenario.name)));
    if let Err(error) = fs::create_dir_all(&scenario_dir) {
        findings.push(unavailable_finding(
            scenario,
            target_component,
            target_model,
            firmware,
            format!(
                "Failed to create firmware artifact directory {}: {error}",
                scenario_dir.display()
            ),
        ));
        return;
    }

    let build_finding = firmware.build.as_ref().and_then(|build| {
        run_firmware_build(
            FirmwareBuildContext {
                bound,
                scenario,
                target_component,
                target_model,
                firmware,
                scenario_dir: &scenario_dir,
            },
            build,
            artifacts,
        )
    });
    if let Some(finding) = build_finding {
        findings.push(finding);
        return;
    }

    let Some(machine) = firmware
        .machine
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        validation_input_missing(
            findings,
            scenario,
            "firmware_in_loop firmware.machine is required for QEMU functional MCU validation.",
        );
        return;
    };
    let image = firmware_image_path(bound, &firmware.image);
    if !image.is_file() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "firmware_in_loop firmware.image {} is not a readable file.",
                image.display()
            ),
        );
        return;
    }

    let qemu = firmware.qemu.as_ref();
    let executable = qemu
        .and_then(|options| options.executable.as_deref())
        .unwrap_or(DEFAULT_QEMU_EXECUTABLE);
    if !executable_on_path(executable) {
        findings.push(unavailable_finding(
            scenario,
            target_component,
            target_model,
            firmware,
            format!("QEMU executable {executable} is not available on PATH."),
        ));
        return;
    }

    let timeout = Duration::from_millis(
        qemu.and_then(|options| options.timeout_ms)
            .unwrap_or(DEFAULT_QEMU_TIMEOUT_MS),
    );
    let result = run_qemu(executable, machine, &image, qemu, timeout);
    let log = scenario_dir.join("qemu.log");
    let log_result = write_qemu_log(&log, &result);
    if log_result.is_ok() {
        push_artifact(artifacts, &log);
    }

    let run = match result {
        Ok(run) => run,
        Err(message) => {
            findings.push(qemu_failure_finding(
                scenario,
                target_component,
                target_model,
                firmware,
                message,
                log_result.err(),
            ));
            return;
        }
    };
    if !run.status.success() {
        findings.push(qemu_failure_finding(
            scenario,
            target_component,
            target_model,
            firmware,
            format!("QEMU exited with status {}.", exit_status_text(&run.status)),
            log_result.err(),
        ));
        return;
    }

    let prefix = qemu
        .and_then(|options| options.pin_trace_prefix.as_deref())
        .unwrap_or(DEFAULT_PIN_TRACE_PREFIX);
    let observations = match parse_pin_observations(&run.stdout, &run.stderr, prefix) {
        Ok(observations) => observations,
        Err(message) => {
            findings.push(pin_trace_failure_finding(
                scenario,
                target_component,
                target_model,
                firmware,
                message,
            ));
            return;
        }
    };
    if observations.is_empty() {
        findings.push(pin_trace_failure_finding(
            scenario,
            target_component,
            target_model,
            firmware,
            format!("QEMU output did not include any {prefix:?} pin observations."),
        ));
        return;
    }

    for expected in &firmware.expected_pin_states {
        let key = PinKey::from(expected);
        let Some(observed) = observations.get(&key) else {
            findings.push(pin_mismatch_finding(
                scenario,
                target_component,
                target_model,
                firmware,
                expected,
                None,
                "Expected pin behavior was not observed in QEMU output.",
            ));
            continue;
        };
        if observed.mode != expected.mode || observed.state != expected.state {
            findings.push(pin_mismatch_finding(
                scenario,
                target_component,
                target_model,
                firmware,
                expected,
                Some(observed),
                "Observed QEMU pin behavior did not match the expected board-facing state.",
            ));
        }
    }
}

enum SelectedFirmwareBackend {
    Qemu,
    Unavailable(String),
}

fn selected_backend(firmware: &FirmwareScenario) -> SelectedFirmwareBackend {
    match firmware.backend {
        FirmwareBackend::Qemu => SelectedFirmwareBackend::Qemu,
        FirmwareBackend::Renode => SelectedFirmwareBackend::Unavailable(
            "Renode functional MCU backend is not integrated in this runtime.".to_string(),
        ),
        FirmwareBackend::Auto => {
            let executable = firmware
                .qemu
                .as_ref()
                .and_then(|options| options.executable.as_deref())
                .unwrap_or(DEFAULT_QEMU_EXECUTABLE);
            if firmware.machine.is_some() && executable_on_path(executable) {
                SelectedFirmwareBackend::Qemu
            } else {
                SelectedFirmwareBackend::Unavailable(
                    "No functional MCU firmware backend is selectable; provide QEMU machine metadata or configure a supported backend.".to_string(),
                )
            }
        }
    }
}

struct FirmwareBuildContext<'a> {
    bound: &'a BoundBoard<'a>,
    scenario: &'a Scenario,
    target_component: &'a str,
    target_model: &'a str,
    firmware: &'a FirmwareScenario,
    scenario_dir: &'a Path,
}

fn run_firmware_build(
    context: FirmwareBuildContext<'_>,
    build: &FirmwareBuildSpec,
    artifacts: &mut Vec<String>,
) -> Option<Finding> {
    let FirmwareBuildContext {
        bound,
        scenario,
        target_component,
        target_model,
        firmware,
        scenario_dir,
    } = context;

    if build.command.is_empty() {
        return Some(unavailable_finding(
            scenario,
            target_component,
            target_model,
            firmware,
            "firmware.build.command must contain at least one argv entry.",
        ));
    }
    let working_dir = resolve_project_path(bound, build.working_dir.as_deref().unwrap_or("."));
    if !working_dir.is_dir() {
        return Some(unavailable_finding(
            scenario,
            target_component,
            target_model,
            firmware,
            format!(
                "firmware.build.working_dir {} is not a directory.",
                working_dir.display()
            ),
        ));
    }
    let executable = resolve_command_executable(&working_dir, &build.command[0]);
    let timeout = Duration::from_millis(
        build
            .timeout_ms
            .unwrap_or(DEFAULT_FIRMWARE_BUILD_TIMEOUT_MS),
    );
    let result = run_command_with_timeout(
        &executable,
        &build.command[1..],
        &working_dir,
        timeout,
        format_build_command(&working_dir, &build.command),
    );
    let log = scenario_dir.join("firmware_build.log");
    let log_result = write_process_log(&log, "firmware_build", &result);
    if log_result.is_ok() {
        push_artifact(artifacts, &log);
    }
    let run = match result {
        Ok(run) => run,
        Err(message) => {
            return Some(build_failure_finding(
                scenario,
                target_component,
                target_model,
                firmware,
                message,
                log_result.err(),
            ));
        }
    };
    if !run.status.success() {
        return Some(build_failure_finding(
            scenario,
            target_component,
            target_model,
            firmware,
            format!(
                "firmware.build command exited with status {}.",
                exit_status_text(&run.status)
            ),
            log_result.err(),
        ));
    }
    for output in &build.outputs {
        let path = resolve_against(&working_dir, output);
        if !path.is_file() {
            return Some(build_failure_finding(
                scenario,
                target_component,
                target_model,
                firmware,
                format!("firmware.build output {} was not produced.", path.display()),
                None,
            ));
        }
        push_artifact(artifacts, &path);
    }
    None
}

fn run_qemu(
    executable: &str,
    machine: &str,
    image: &Path,
    options: Option<&QemuFirmwareOptions>,
    timeout: Duration,
) -> Result<QemuRun, String> {
    let mut command = Command::new(executable);
    command
        .arg("-M")
        .arg(machine)
        .arg("-kernel")
        .arg(image)
        .arg("-nographic")
        .arg("-semihosting");
    if let Some(options) = options {
        command.args(&options.extra_args);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let command_line = format_qemu_command(executable, machine, image, options);
    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to launch QEMU backend {executable}: {error}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect QEMU output: {error}"))?;
                return Ok(QemuRun {
                    command_line,
                    status: output.status,
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect timed-out QEMU output: {error}"))?;
                return Err(format!(
                    "QEMU firmware validation exceeded {} ms and was terminated. Stdout bytes: {}, stderr bytes: {}.",
                    timeout.as_millis(),
                    output.stdout.len(),
                    output.stderr.len()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(format!("Failed while waiting for QEMU: {error}")),
        }
    }
}

fn parse_pin_observations(
    stdout: &str,
    stderr: &str,
    prefix: &str,
) -> Result<BTreeMap<PinKey, ObservedPin>, String> {
    let mut observations = BTreeMap::new();
    for line in stdout.lines().chain(stderr.lines()) {
        let Some(rest) = line.trim().strip_prefix(prefix) else {
            continue;
        };
        let observed = parse_pin_observation(rest.trim())?;
        let key = PinKey {
            component: observed.component.clone(),
            pin: observed.pin.clone(),
        };
        if let Some(previous) = observations.get(&key) {
            if previous != &observed {
                return Err(format!(
                    "Conflicting QEMU pin observations for {}.{}.",
                    key.component, key.pin
                ));
            }
        } else {
            observations.insert(key, observed);
        }
    }
    Ok(observations)
}

fn parse_pin_observation(text: &str) -> Result<ObservedPin, String> {
    let mut parts = text.split_whitespace();
    let endpoint = parts
        .next()
        .ok_or_else(|| "Pin observation is missing endpoint.".to_string())?;
    let (component, pin) = endpoint
        .split_once('.')
        .ok_or_else(|| format!("Pin observation endpoint {endpoint:?} must be COMPONENT.PIN."))?;
    if component.is_empty() || pin.is_empty() {
        return Err(format!(
            "Pin observation endpoint {endpoint:?} must include non-empty component and pin."
        ));
    }
    let mut mode = None;
    let mut state = None;
    for part in parts {
        let (key, value) = part
            .split_once('=')
            .ok_or_else(|| format!("Pin observation field {part:?} must use key=value syntax."))?;
        match key {
            "mode" => mode = Some(parse_pin_mode(value)?),
            "state" => state = Some(parse_pin_state(value)?),
            _ => return Err(format!("Unsupported pin observation field {key:?}.")),
        }
    }
    Ok(ObservedPin {
        component: component.to_string(),
        pin: pin.to_string(),
        mode: mode
            .ok_or_else(|| format!("Pin observation for {component}.{pin} is missing mode=..."))?,
        state,
    })
}

fn parse_pin_mode(value: &str) -> Result<PinMode, String> {
    match value {
        "input" => Ok(PinMode::Input),
        "output" => Ok(PinMode::Output),
        "high_z" => Ok(PinMode::HighZ),
        _ => Err(format!(
            "Unsupported pin mode {value:?} in QEMU observation."
        )),
    }
}

fn parse_pin_state(value: &str) -> Result<PinLogicState, String> {
    match value {
        "high" => Ok(PinLogicState::High),
        "low" => Ok(PinLogicState::Low),
        "z" => Ok(PinLogicState::Z),
        _ => Err(format!(
            "Unsupported pin state {value:?} in QEMU observation."
        )),
    }
}

fn firmware_image_path(bound: &BoundBoard<'_>, image: &str) -> PathBuf {
    resolve_project_path(bound, image)
}

fn resolve_project_path(bound: &BoundBoard<'_>, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        bound.project.source_dir.join(path)
    }
}

fn resolve_against(base: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn resolve_command_executable(working_dir: &Path, executable: &str) -> PathBuf {
    let path = Path::new(executable);
    if path.is_absolute() || path.components().count() > 1 {
        resolve_against(working_dir, executable)
    } else {
        path.to_path_buf()
    }
}

fn run_command_with_timeout(
    executable: &Path,
    args: &[String],
    working_dir: &Path,
    timeout: Duration,
    command_line: String,
) -> Result<QemuRun, String> {
    let mut command = Command::new(executable);
    command.current_dir(working_dir).args(args);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        format!(
            "Failed to launch firmware build command {}: {error}",
            executable.display()
        )
    })?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect firmware build output: {error}"))?;
                return Ok(QemuRun {
                    command_line,
                    status: output.status,
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let output = child.wait_with_output().map_err(|error| {
                    format!("Failed to collect timed-out firmware build output: {error}")
                })?;
                return Err(format!(
                    "firmware.build command exceeded {} ms and was terminated. Stdout bytes: {}, stderr bytes: {}.",
                    timeout.as_millis(),
                    output.stdout.len(),
                    output.stderr.len()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => {
                return Err(format!(
                    "Failed while waiting for firmware build command: {error}"
                ));
            }
        }
    }
}

fn write_qemu_log(path: &Path, result: &Result<QemuRun, String>) -> Result<(), String> {
    write_process_log(path, "qemu", result)
}

fn write_process_log(
    path: &Path,
    label: &str,
    result: &Result<QemuRun, String>,
) -> Result<(), String> {
    let mut log = String::new();
    match result {
        Ok(run) => {
            log.push_str(&format!("kind: {label}\n"));
            log.push_str(&format!("command: {}\n", run.command_line));
            log.push_str(&format!(
                "exit_status: {}\n\n",
                exit_status_text(&run.status)
            ));
            log.push_str("stdout:\n");
            log.push_str(&run.stdout);
            log.push_str("\n\nstderr:\n");
            log.push_str(&run.stderr);
        }
        Err(message) => {
            log.push_str("error:\n");
            log.push_str(message);
            log.push('\n');
        }
    }
    fs::write(path, log).map_err(|error| format!("Failed to write {label} log: {error}"))
}

fn format_qemu_command(
    executable: &str,
    machine: &str,
    image: &Path,
    options: Option<&QemuFirmwareOptions>,
) -> String {
    let mut parts = vec![
        executable.to_string(),
        "-M".to_string(),
        machine.to_string(),
        "-kernel".to_string(),
        image.display().to_string(),
        "-nographic".to_string(),
        "-semihosting".to_string(),
    ];
    if let Some(options) = options {
        parts.extend(options.extra_args.clone());
    }
    parts.join(" ")
}

fn format_build_command(working_dir: &Path, command: &[String]) -> String {
    format!("cd {} && {}", working_dir.display(), command.join(" "))
}

fn exit_status_text(status: &ExitStatus) -> String {
    status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "terminated_by_signal".to_string())
}

fn unavailable_finding(
    scenario: &Scenario,
    target_component: &str,
    target_model: &str,
    firmware: &FirmwareScenario,
    message: impl Into<String>,
) -> Finding {
    let mut finding = base_finding(
        scenario,
        target_component,
        target_model,
        firmware,
        message.into(),
    );
    finding.suggested_fixes = vec![
        "Add a supported functional MCU firmware backend, such as QEMU or Renode, before relying on firmware-in-loop results.".to_string(),
        "Keep transistor-level MCU internals out of this check; validate board-facing pin behavior, reset/boot state, and peripheral effects instead.".to_string(),
        "Use existing reset/boot, protocol, control-line, and analog SPICE checks for narrower validation until firmware-in-loop support covers this target.".to_string(),
    ];
    finding
}

fn qemu_failure_finding(
    scenario: &Scenario,
    target_component: &str,
    target_model: &str,
    firmware: &FirmwareScenario,
    message: impl Into<String>,
    log_error: Option<String>,
) -> Finding {
    let mut finding = base_finding(
        scenario,
        target_component,
        target_model,
        firmware,
        message.into(),
    );
    if let Some(log_error) = log_error {
        finding
            .measured
            .insert("artifact_error".to_string(), json!(log_error));
    }
    finding.suggested_fixes = vec![
        "Check the QEMU machine, firmware image format, and any explicit qemu.extra_args.".to_string(),
        "Ensure the functional firmware model emits CIRCUITCI_PIN observations for board-facing pins.".to_string(),
    ];
    finding
}

fn build_failure_finding(
    scenario: &Scenario,
    target_component: &str,
    target_model: &str,
    firmware: &FirmwareScenario,
    message: impl Into<String>,
    log_error: Option<String>,
) -> Finding {
    let mut finding = base_finding(
        scenario,
        target_component,
        target_model,
        firmware,
        message.into(),
    );
    if let Some(log_error) = log_error {
        finding
            .measured
            .insert("artifact_error".to_string(), json!(log_error));
    }
    finding.suggested_fixes = vec![
        "Fix firmware.build.command, working_dir, or declared outputs so the functional firmware image is produced before validation.".to_string(),
        "Use an explicit project build script, such as a peer STM32 CMake wrapper, instead of assuming the MCU compiler is globally on PATH.".to_string(),
    ];
    finding
}

fn pin_trace_failure_finding(
    scenario: &Scenario,
    target_component: &str,
    target_model: &str,
    firmware: &FirmwareScenario,
    message: impl Into<String>,
) -> Finding {
    let mut finding = base_finding(
        scenario,
        target_component,
        target_model,
        firmware,
        message.into(),
    );
    finding.suggested_fixes = vec![
        "Instrument the functional firmware model or QEMU board model to emit CIRCUITCI_PIN COMPONENT.PIN mode=<mode> state=<state> observations.".to_string(),
        "Keep observations at the MCU package boundary; do not substitute transistor-level MCU internals for pin behavior.".to_string(),
    ];
    finding
}

fn pin_mismatch_finding(
    scenario: &Scenario,
    target_component: &str,
    target_model: &str,
    firmware: &FirmwareScenario,
    expected: &PinState,
    observed: Option<&ObservedPin>,
    message: impl Into<String>,
) -> Finding {
    let mut finding = base_finding(
        scenario,
        target_component,
        target_model,
        firmware,
        message.into(),
    );
    finding
        .measured
        .insert("pin_component".to_string(), json!(expected.component));
    finding
        .measured
        .insert("pin".to_string(), json!(expected.pin));
    if let Some(observed) = observed {
        finding.measured.insert(
            "observed_mode".to_string(),
            json!(pin_mode_text(&observed.mode)),
        );
        finding.measured.insert(
            "observed_state".to_string(),
            json!(observed.state.as_ref().map(pin_state_text)),
        );
    }
    finding.limit.insert(
        "expected_mode".to_string(),
        json!(pin_mode_text(&expected.mode)),
    );
    finding.limit.insert(
        "expected_state".to_string(),
        json!(expected.state.as_ref().map(pin_state_text)),
    );
    finding.suggested_fixes = vec![
        "Fix firmware pin configuration or board mapping so the functional MCU model drives the expected package pin behavior.".to_string(),
        "If the expected state is wrong, update the scenario with the intended board-facing pin behavior.".to_string(),
    ];
    finding
}

fn base_finding(
    scenario: &Scenario,
    target_component: &str,
    target_model: &str,
    firmware: &FirmwareScenario,
    message: String,
) -> Finding {
    let mut finding = Finding::critical(FUNCTIONAL_MCU_FIRMWARE, &scenario.name, message);
    finding.component = Some(target_component.to_string());
    finding
        .measured
        .insert("target_component".to_string(), json!(target_component));
    finding
        .measured
        .insert("target_model".to_string(), json!(target_model));
    finding.measured.insert(
        "backend".to_string(),
        json!(firmware_backend(&firmware.backend)),
    );
    finding
        .measured
        .insert("firmware_image".to_string(), json!(firmware.image));
    if let Some(machine) = &firmware.machine {
        finding
            .measured
            .insert("machine".to_string(), json!(machine));
    }
    finding.measured.insert(
        "expected_pin_states".to_string(),
        json!(firmware.expected_pin_states.len()),
    );
    finding.limit.insert(
        "functional_blackbox_boundary".to_string(),
        json!("firmware-visible peripherals and board-facing pin behavior"),
    );
    finding
        .limit
        .insert("transistor_level_mcu_required".to_string(), json!(false));
    finding
}

fn firmware_backend(backend: &FirmwareBackend) -> &'static str {
    match backend {
        FirmwareBackend::Auto => "auto",
        FirmwareBackend::Renode => "renode",
        FirmwareBackend::Qemu => "qemu",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedPin {
    component: String,
    pin: String,
    mode: PinMode,
    state: Option<PinLogicState>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PinKey {
    component: String,
    pin: String,
}

impl From<&PinState> for PinKey {
    fn from(value: &PinState) -> Self {
        Self {
            component: value.component.clone(),
            pin: value.pin.clone(),
        }
    }
}

struct QemuRun {
    command_line: String,
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

fn pin_mode_text(mode: &PinMode) -> &'static str {
    match mode {
        PinMode::Input => "input",
        PinMode::Output => "output",
        PinMode::HighZ => "high_z",
    }
}

fn pin_state_text(state: &PinLogicState) -> &'static str {
    match state {
        PinLogicState::High => "high",
        PinLogicState::Low => "low",
        PinLogicState::Z => "z",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pin_observation_lines() {
        let observations = parse_pin_observations(
            "noise\nCIRCUITCI_PIN U1.TX mode=output state=high\n",
            "",
            DEFAULT_PIN_TRACE_PREFIX,
        )
        .unwrap();
        let observed = observations
            .get(&PinKey {
                component: "U1".to_string(),
                pin: "TX".to_string(),
            })
            .unwrap();
        assert_eq!(observed.mode, PinMode::Output);
        assert_eq!(observed.state, Some(PinLogicState::High));
    }

    #[test]
    fn rejects_conflicting_pin_observations() {
        let error = parse_pin_observations(
            "CIRCUITCI_PIN U1.TX mode=output state=high\nCIRCUITCI_PIN U1.TX mode=output state=low\n",
            "",
            DEFAULT_PIN_TRACE_PREFIX,
        )
        .unwrap_err();
        assert!(error.contains("Conflicting QEMU pin observations"));
    }
}
