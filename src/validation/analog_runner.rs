use crate::board_ir::{AnalogBackend, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use libloading::Library;
use serde_json::json;
use std::env;
use std::ffi::{CStr, CString};
use std::fs;
use std::io::ErrorKind;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::ptr;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use super::analog_util::{
    absolute_path, executable_on_path, normalize_artifact_path, normalize_path, safe_artifact_name,
};

pub(super) const ANALOG_SOLVER_MANIFEST_SCHEMA: &str = "circuitci.analog_solver_manifest.v0.1";

#[derive(Debug, Clone)]
pub(super) struct ParameterOverride {
    pub(super) name: String,
    pub(super) value: f64,
}

#[derive(Debug, Clone)]
pub(super) struct ModelSectionOverride {
    pub(super) path: String,
    pub(super) section: String,
}

pub(super) struct NgspiceRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) waveform: PathBuf,
    pub(super) series: WaveformSeries,
    pub(super) user_probe_count: usize,
}

pub(super) struct NgspiceAcRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) bode: PathBuf,
}

#[derive(Debug)]
pub(super) struct WaveformSeries {
    pub(super) time_s: Vec<f64>,
    pub(super) values_by_probe: Vec<Vec<f64>>,
}

pub(super) struct NgspiceRunError {
    pub(super) message: String,
    pub(super) artifacts: Vec<PathBuf>,
}

pub(super) struct NgspiceRunOptions<'a, F, C>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    pub(super) output: &'a Path,
    pub(super) run_subdir: Option<&'a str>,
    pub(super) parameter_overrides: &'a [ParameterOverride],
    pub(super) model_section_overrides: &'a [ModelSectionOverride],
    pub(super) operating_probe_expressions: &'a [String],
    pub(super) on_progress: F,
    pub(super) should_cancel: C,
}

struct NgspiceWrapperOptions<'a> {
    parameter_overrides: &'a [ParameterOverride],
    model_section_overrides: &'a [ModelSectionOverride],
    operating_probe_expressions: &'a [String],
    include_control: bool,
    include_quit: bool,
}

pub(super) struct NgspiceAcRunOptions<'a, F, C>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    pub(super) output: &'a Path,
    pub(super) run_subdir: Option<&'a str>,
    pub(super) parameter_overrides: &'a [ParameterOverride],
    pub(super) model_section_overrides: &'a [ModelSectionOverride],
    pub(super) on_progress: F,
    pub(super) should_cancel: C,
}

pub(super) fn run_ngspice<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceRunOptions<'_, F, C>,
) -> Result<NgspiceRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceRunOptions {
        output,
        run_subdir,
        parameter_overrides,
        model_section_overrides,
        operating_probe_expressions,
        mut on_progress,
        should_cancel,
    } = options;
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice.cir");
    let log = run_dir.join("ngspice.log");
    let waveform = run_dir.join("waveform.csv");
    let embedded_backend = backend == "embedded_ngspice";
    on_progress(
        "Writing analog wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("waveform.csv"),
        NgspiceWrapperOptions {
            parameter_overrides,
            model_section_overrides,
            operating_probe_expressions,
            include_control: !embedded_backend,
            include_quit: !embedded_backend,
        },
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    let embedded_commands =
        EmbeddedCommands::new(bound, scenario, &waveform, operating_probe_expressions);
    on_progress(
        "Running analog backend",
        format!(
            "{} transient for {} user probe(s) and {} operating probe(s).",
            backend,
            analog.probes.len(),
            operating_probe_expressions.len()
        ),
    );
    let output = run_solver_with_timeout(
        backend,
        &wrapper,
        Duration::from_secs(60),
        Some(&embedded_commands),
        should_cancel,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    let mut log_text = String::new();
    log_text.push_str("COMMAND: ");
    log_text.push_str(&output.command);
    log_text.push_str("\n\nSTDOUT:\n");
    log_text.push_str(&String::from_utf8_lossy(&output.stdout));
    log_text.push_str("\n\nSTDERR:\n");
    log_text.push_str(&String::from_utf8_lossy(&output.stderr));
    on_progress(
        "Writing analog solver log",
        format!("Writing {}.", log.to_string_lossy()),
    );
    fs::write(&log, &log_text).map_err(|error| {
        ngspice_error(
            format!("Failed to write ngspice log {}: {error}", log.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice transient analysis exited with status {}.",
                output.status
            ),
            artifacts,
        ));
    }
    if let Some(reason) = detect_nonconvergence(&log_text) {
        return Err(ngspice_error(
            format!("ngspice reported non-convergence or numerical failure: {reason}."),
            artifacts,
        ));
    }
    if !waveform.is_file() {
        return Err(ngspice_error(
            format!(
                "ngspice completed without producing waveform export {}.",
                waveform.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(waveform.clone());
    let probe_count = analog.probes.len() + operating_probe_expressions.len();
    on_progress(
        "Loading analog waveform",
        format!(
            "Reading {} with {} column(s).",
            waveform.to_string_lossy(),
            probe_count
        ),
    );
    let series = parse_waveform_csv(&waveform, probe_count)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    on_progress(
        "Loaded analog waveform",
        format!("{} sample(s).", series.time_s.len()),
    );
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "transient",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[],
        normalized_outputs: &[("transient_waveform", &waveform)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceRun {
        artifacts,
        waveform,
        series,
        user_probe_count: analog.probes.len(),
    })
}

pub(super) fn run_ngspice_ac<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceAcRunOptions<'_, F, C>,
) -> Result<NgspiceAcRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceAcRunOptions {
        output,
        run_subdir,
        parameter_overrides,
        model_section_overrides,
        mut on_progress,
        should_cancel,
    } = options;
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before AC run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog AC run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_ac.cir");
    let log = run_dir.join("ngspice_ac.log");
    let raw = run_dir.join("ac_raw.csv");
    let bode = run_dir.join("bode.csv");
    on_progress(
        "Writing analog AC wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_ac_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("ac_raw.csv"),
        parameter_overrides,
        model_section_overrides,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice AC wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog AC backend",
        format!("{} AC sweep for {} probe(s).", backend, analog.probes.len()),
    );
    let output = run_solver_with_timeout(
        backend,
        &wrapper,
        Duration::from_secs(60),
        None,
        should_cancel,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    let mut log_text = String::new();
    log_text.push_str("COMMAND: ");
    log_text.push_str(&output.command);
    log_text.push_str("\n\nSTDOUT:\n");
    log_text.push_str(&String::from_utf8_lossy(&output.stdout));
    log_text.push_str("\n\nSTDERR:\n");
    log_text.push_str(&String::from_utf8_lossy(&output.stderr));
    on_progress(
        "Writing analog AC solver log",
        format!("Writing {}.", log.to_string_lossy()),
    );
    fs::write(&log, &log_text).map_err(|error| {
        ngspice_error(
            format!("Failed to write ngspice AC log {}: {error}", log.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!("ngspice AC analysis exited with status {}.", output.status),
            artifacts,
        ));
    }
    if let Some(reason) = detect_nonconvergence(&log_text) {
        return Err(ngspice_error(
            format!("ngspice reported non-convergence or numerical failure: {reason}."),
            artifacts,
        ));
    }
    if !raw.is_file() {
        return Err(ngspice_error(
            format!(
                "ngspice completed without producing AC export {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    on_progress(
        "Loading analog AC response",
        format!("Reading {}.", raw.to_string_lossy()),
    );
    let bode_csv = ac_raw_to_bode_csv(&raw, analog).map_err(|message| {
        ngspice_error(
            format!("Failed to convert AC response {}: {message}", raw.display()),
            artifacts.clone(),
        )
    })?;
    fs::write(&bode, bode_csv).map_err(|error| {
        ngspice_error(
            format!("Failed to write AC Bode CSV {}: {error}", bode.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(bode.clone());
    on_progress(
        "Exported analog AC response",
        format!("Wrote {}.", bode.to_string_lossy()),
    );
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "ac",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("ngspice_ac_raw", &raw)],
        normalized_outputs: &[("ac_bode", &bode)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceAcRun { artifacts, bode })
}

pub(super) struct SolverOutput {
    pub(super) status: SolverStatus,
    pub(super) command: String,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

pub(super) enum SolverStatus {
    External(ExitStatus),
    Embedded(i32),
}

impl SolverStatus {
    pub(super) fn success(&self) -> bool {
        match self {
            Self::External(status) => status.success(),
            Self::Embedded(code) => *code == 0,
        }
    }
}

pub(super) struct EmbeddedCommands {
    tran: String,
    wrdata: String,
}

impl EmbeddedCommands {
    fn new(
        bound: &BoundBoard<'_>,
        scenario: &Scenario,
        waveform: &Path,
        operating_probe_expressions: &[String],
    ) -> Self {
        let analog = scenario
            .analog
            .as_ref()
            .expect("analog was validated before embedded command generation");
        let step_s = analog.analysis.max_step_us / 1_000_000.0;
        let stop_s = analog.analysis.stop_time_us / 1_000_000.0;
        let mut wrdata = String::new();
        wrdata.push_str("wrdata ");
        wrdata.push_str(&waveform.to_string_lossy());
        for probe in &analog.probes {
            wrdata.push(' ');
            wrdata.push_str(&probe.expression);
        }
        for expression in operating_probe_expressions {
            wrdata.push(' ');
            wrdata.push_str(expression);
        }
        let uic = if uses_capacitor_initial_conditions(bound, scenario) {
            " uic"
        } else {
            ""
        };
        Self {
            tran: format!("tran {:.12e} {:.12e}{uic}", step_s, stop_s),
            wrdata,
        }
    }
}

impl std::fmt::Display for SolverStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::External(status) => write!(formatter, "{status}"),
            Self::Embedded(code) => write!(formatter, "embedded libngspice return code {code}"),
        }
    }
}

pub(super) fn run_solver_with_timeout(
    backend: &str,
    wrapper: &Path,
    timeout: Duration,
    embedded_commands: Option<&EmbeddedCommands>,
    should_cancel: impl Fn() -> bool,
) -> Result<SolverOutput, String> {
    if backend == "embedded_ngspice" {
        let commands = embedded_commands
            .ok_or_else(|| "embedded ngspice execution requires transient commands".to_string())?;
        return run_embedded_ngspice(wrapper, commands, timeout);
    }
    let working_dir = wrapper
        .parent()
        .ok_or_else(|| format!("ngspice wrapper path {} has no parent.", wrapper.display()))?;
    let deck_name = wrapper.file_name().ok_or_else(|| {
        format!(
            "ngspice wrapper path {} has no filename.",
            wrapper.display()
        )
    })?;
    let mut child = Command::new(backend)
        .current_dir(working_dir)
        .arg("-b")
        .arg(deck_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to launch ngspice backend {backend}: {error}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect ngspice output: {error}"))?;
                return Ok(SolverOutput {
                    status: SolverStatus::External(output.status),
                    command: format!(
                        "cd {} && {backend} -b {}",
                        working_dir.to_string_lossy(),
                        deck_name.to_string_lossy()
                    ),
                    stdout: output.stdout,
                    stderr: output.stderr,
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let output = child.wait_with_output().map_err(|error| {
                    format!("Failed to collect timed-out ngspice output: {error}")
                })?;
                return Err(format!(
                    "ngspice transient analysis exceeded {} seconds and was terminated. Stdout bytes: {}, stderr bytes: {}.",
                    timeout.as_secs(),
                    output.stdout.len(),
                    output.stderr.len()
                ));
            }
            Ok(None) if should_cancel() => {
                let _ = child.kill();
                let output = child.wait_with_output().map_err(|error| {
                    format!("Failed to collect canceled ngspice output: {error}")
                })?;
                return Err(format!(
                    "ngspice transient analysis was canceled and the external process was terminated. Stdout bytes: {}, stderr bytes: {}.",
                    output.stdout.len(),
                    output.stderr.len()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(format!("Failed while waiting for ngspice: {error}")),
        }
    }
}

type SendCharCallback = unsafe extern "C" fn(*mut c_char, c_int, *mut c_void) -> c_int;
type SendStatCallback = unsafe extern "C" fn(*mut c_char, c_int, *mut c_void) -> c_int;
type ControlledExitCallback = unsafe extern "C" fn(c_int, bool, bool, c_int, *mut c_void) -> c_int;
type SendDataCallback = unsafe extern "C" fn(*mut c_void, c_int, c_int, *mut c_void) -> c_int;
type SendInitDataCallback = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void) -> c_int;
type BgThreadRunningCallback = unsafe extern "C" fn(bool, c_int, *mut c_void) -> c_int;

type NgSpiceInit = unsafe extern "C" fn(
    Option<SendCharCallback>,
    Option<SendStatCallback>,
    Option<ControlledExitCallback>,
    Option<SendDataCallback>,
    Option<SendInitDataCallback>,
    Option<BgThreadRunningCallback>,
    *mut c_void,
) -> c_int;
type NgSpiceCommand = unsafe extern "C" fn(*mut c_char) -> c_int;
type NgSpiceCirc = unsafe extern "C" fn(*mut *mut c_char) -> c_int;

struct EmbeddedNgspice {
    _library: Library,
    init: NgSpiceInit,
    command: NgSpiceCommand,
    circ: NgSpiceCirc,
    path: String,
}

struct EmbeddedLog {
    stdout: String,
    status: String,
    controlled_exit: Option<EmbeddedExit>,
    background_events: Vec<bool>,
}

struct EmbeddedExit {
    status: c_int,
    immediate: bool,
    quit: bool,
    ident: c_int,
}

static EMBEDDED_NGSPICE_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn run_embedded_ngspice(
    wrapper: &Path,
    commands: &EmbeddedCommands,
    _timeout: Duration,
) -> Result<SolverOutput, String> {
    let _guard = EMBEDDED_NGSPICE_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "embedded ngspice global mutex is poisoned".to_string())?;
    let engine = load_embedded_ngspice()?;
    let mut log = EmbeddedLog {
        stdout: String::new(),
        status: String::new(),
        controlled_exit: None,
        background_events: Vec::new(),
    };
    let user_data = &mut log as *mut EmbeddedLog as *mut c_void;
    let init_code = unsafe {
        (engine.init)(
            Some(embedded_send_char),
            Some(embedded_send_stat),
            Some(embedded_controlled_exit),
            None,
            None,
            Some(embedded_background_running),
            user_data,
        )
    };
    if init_code != 0 {
        return Err(format!(
            "Failed to initialize embedded libngspice {}: return code {init_code}.",
            engine.path
        ));
    }
    let mut circuit = c_circuit_lines(wrapper)?;
    let circ_code = unsafe { (engine.circ)(circuit.as_mut_ptr()) };
    let vecnames_code = if circ_code == 0 {
        run_embedded_command(&engine, "set wr_vecnames")?
    } else {
        circ_code
    };
    let singlescale_code = if vecnames_code == 0 {
        run_embedded_command(&engine, "set wr_singlescale")?
    } else {
        vecnames_code
    };
    let tran_code = if singlescale_code == 0 {
        run_embedded_command(&engine, &commands.tran)?
    } else {
        singlescale_code
    };
    let wrdata_code = if tran_code == 0 {
        run_embedded_command(&engine, &commands.wrdata)?
    } else {
        tran_code
    };
    let destroy_code = run_embedded_command(&engine, "destroy all").unwrap_or(wrdata_code);
    let mut stdout = String::new();
    stdout.push_str("LIBNGSPICE: ");
    stdout.push_str(&engine.path);
    stdout.push_str("\nCIRCUIT: ");
    stdout.push_str(&wrapper.to_string_lossy());
    stdout.push_str("\nTRAN: ");
    stdout.push_str(&commands.tran);
    stdout.push_str("\nWRDATA: ");
    stdout.push_str(&commands.wrdata);
    stdout.push_str(&format!(
        "\nRETURN_CODES: circ={circ_code} wr_vecnames={vecnames_code} wr_singlescale={singlescale_code} tran={tran_code} wrdata={wrdata_code} destroy={destroy_code}\n"
    ));
    stdout.push_str("\nCALLBACK_OUTPUT:\n");
    stdout.push_str(&log.stdout);
    stdout.push_str("\nCALLBACK_STATUS:\n");
    stdout.push_str(&log.status);
    if let Some(exit) = log.controlled_exit {
        stdout.push_str(&format!(
            "\nCONTROLLED_EXIT: status={} immediate={} quit={} ident={}\n",
            exit.status, exit.immediate, exit.quit, exit.ident
        ));
    }
    if !log.background_events.is_empty() {
        stdout.push_str("BACKGROUND_EVENTS:");
        for event in log.background_events {
            stdout.push_str(if event { " running" } else { " stopped" });
        }
        stdout.push('\n');
    }
    Ok(SolverOutput {
        status: SolverStatus::Embedded(
            [
                circ_code,
                vecnames_code,
                singlescale_code,
                tran_code,
                wrdata_code,
            ]
            .into_iter()
            .find(|code| *code != 0)
            .unwrap_or(0),
        ),
        command: format!("embedded_ngspice circ {}", wrapper.to_string_lossy()),
        stdout: stdout.into_bytes(),
        stderr: Vec::new(),
    })
}

struct EmbeddedCircuit {
    _lines: Vec<CString>,
    pointers: Vec<*mut c_char>,
}

impl EmbeddedCircuit {
    fn as_mut_ptr(&mut self) -> *mut *mut c_char {
        self.pointers.as_mut_ptr()
    }
}

fn c_circuit_lines(wrapper: &Path) -> Result<EmbeddedCircuit, String> {
    let source = fs::read_to_string(wrapper).map_err(|error| {
        format!(
            "Failed to read embedded ngspice circuit deck {}: {error}",
            wrapper.display()
        )
    })?;
    let mut lines = Vec::new();
    for line in source.lines() {
        lines.push(CString::new(line).map_err(|_| {
            format!(
                "Embedded ngspice circuit deck {} contains an interior NUL.",
                wrapper.display()
            )
        })?);
    }
    let mut pointers: Vec<*mut c_char> = lines
        .iter_mut()
        .map(|line| line.as_ptr().cast_mut())
        .collect();
    pointers.push(ptr::null_mut());
    Ok(EmbeddedCircuit {
        _lines: lines,
        pointers,
    })
}

fn run_embedded_command(engine: &EmbeddedNgspice, command: &str) -> Result<c_int, String> {
    let command = CString::new(command)
        .map_err(|_| format!("Embedded ngspice command contains an interior NUL: {command}"))?;
    let mut command = command.into_bytes_with_nul();
    Ok(unsafe { (engine.command)(command.as_mut_ptr().cast()) })
}

fn load_embedded_ngspice() -> Result<EmbeddedNgspice, String> {
    let mut errors = Vec::new();
    for candidate in embedded_ngspice_candidates() {
        let library = match unsafe { Library::new(&candidate) } {
            Ok(library) => library,
            Err(error) => {
                errors.push(format!("{candidate}: {error}"));
                continue;
            }
        };
        let init = unsafe { load_symbol::<NgSpiceInit>(&library, b"ngSpice_Init\0")? };
        let command = unsafe { load_symbol::<NgSpiceCommand>(&library, b"ngSpice_Command\0")? };
        let circ = unsafe { load_symbol::<NgSpiceCirc>(&library, b"ngSpice_Circ\0")? };
        return Ok(EmbeddedNgspice {
            _library: library,
            init,
            command,
            circ,
            path: candidate,
        });
    }
    Err(format!(
        "No usable libngspice shared library found. Tried: {}",
        errors.join("; ")
    ))
}

unsafe fn load_symbol<T: Copy>(library: &Library, name: &[u8]) -> Result<T, String> {
    let symbol = unsafe { library.get::<T>(name) }.map_err(|error| {
        format!(
            "Failed to resolve libngspice symbol {}: {error}",
            String::from_utf8_lossy(name).trim_end_matches('\0')
        )
    })?;
    Ok(*symbol)
}

fn embedded_ngspice_candidates() -> Vec<String> {
    if let Ok(path) = env::var("CIRCUITCI_LIBNGSPICE")
        && !path.trim().is_empty()
    {
        return vec![path];
    }
    let mut candidates = Vec::new();
    candidates.extend(
        [
            "libngspice.dylib",
            "libngspice.so",
            "libngspice.so.0",
            "ngspice.dll",
            "/opt/homebrew/lib/libngspice.dylib",
            "/opt/homebrew/opt/libngspice/lib/libngspice.dylib",
            "/usr/local/lib/libngspice.dylib",
            "/usr/lib/libngspice.so",
            "/usr/local/lib/libngspice.so",
        ]
        .iter()
        .map(|candidate| (*candidate).to_string()),
    );
    candidates
}

fn embedded_ngspice_available() -> bool {
    load_embedded_ngspice().is_ok()
}

unsafe extern "C" fn embedded_send_char(
    text: *mut c_char,
    _ident: c_int,
    user_data: *mut c_void,
) -> c_int {
    if let (Some(log), Some(message)) = (embedded_log(user_data), c_string(text)) {
        log.stdout.push_str(message);
        if !message.ends_with('\n') {
            log.stdout.push('\n');
        }
    }
    0
}

unsafe extern "C" fn embedded_send_stat(
    text: *mut c_char,
    _ident: c_int,
    user_data: *mut c_void,
) -> c_int {
    if let (Some(log), Some(message)) = (embedded_log(user_data), c_string(text)) {
        log.status.push_str(message);
        if !message.ends_with('\n') {
            log.status.push('\n');
        }
    }
    0
}

unsafe extern "C" fn embedded_controlled_exit(
    status: c_int,
    immediate: bool,
    quit: bool,
    ident: c_int,
    user_data: *mut c_void,
) -> c_int {
    if let Some(log) = embedded_log(user_data) {
        log.controlled_exit = Some(EmbeddedExit {
            status,
            immediate,
            quit,
            ident,
        });
    }
    0
}

unsafe extern "C" fn embedded_background_running(
    running: bool,
    _ident: c_int,
    user_data: *mut c_void,
) -> c_int {
    if let Some(log) = embedded_log(user_data) {
        log.background_events.push(running);
    }
    0
}

fn embedded_log(user_data: *mut c_void) -> Option<&'static mut EmbeddedLog> {
    if user_data.is_null() {
        return None;
    }
    Some(unsafe { &mut *(user_data as *mut EmbeddedLog) })
}

fn c_string(text: *mut c_char) -> Option<&'static str> {
    if text.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(text) }.to_str().ok()
}

pub(super) fn ngspice_error(
    message: impl Into<String>,
    artifacts: Vec<PathBuf>,
) -> NgspiceRunError {
    NgspiceRunError {
        message: message.into(),
        artifacts,
    }
}

pub(super) fn detect_nonconvergence(log: &str) -> Option<&'static str> {
    let lower = log.to_ascii_lowercase();
    for (pattern, reason) in [
        ("timestep too small", "timestep too small"),
        ("singular matrix", "singular matrix"),
        ("convergence problem", "convergence problem"),
        (
            "doanalyses: iteration limit reached",
            "iteration limit reached",
        ),
        ("tran simulation(s) aborted", "transient simulation aborted"),
    ] {
        if lower.contains(pattern) {
            return Some(reason);
        }
    }
    None
}

fn build_ngspice_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    waveform: &Path,
    options: NgspiceWrapperOptions<'_>,
) -> Result<String, String> {
    let NgspiceWrapperOptions {
        parameter_overrides,
        model_section_overrides,
        operating_probe_expressions,
        include_control,
        include_quit,
    } = options;
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI. Do not edit by hand.\n");
    text.push_str("* Source netlist: ");
    text.push_str(&netlist.to_string_lossy());
    text.push('\n');
    let include_base = netlist.parent().unwrap_or(&bound.project.source_dir);
    for line in source.lines() {
        let trimmed = line.trim_start();
        let directive = trimmed.to_ascii_lowercase();
        let first_token = directive.split_whitespace().next().unwrap_or("");
        if matches!(first_token, ".end" | ".tran") {
            continue;
        }
        text.push_str(&rewrite_include_line(line, include_base));
        text.push('\n');
    }
    if !parameter_overrides.is_empty() {
        text.push_str("* CircuitCI sweep parameter overrides.\n");
        for override_ in parameter_overrides {
            text.push_str(".param ");
            text.push_str(&override_.name);
            text.push('=');
            text.push_str(&format!("{:.12e}", override_.value));
            text.push('\n');
        }
        if let Some(temperature_c) = sweep_temperature_c(parameter_overrides) {
            text.push_str(".temp ");
            text.push_str(&format!("{:.12e}", temperature_c));
            text.push('\n');
        }
    }
    if !model_section_overrides.is_empty() {
        text.push_str("* CircuitCI sweep model section overrides.\n");
        for override_ in model_section_overrides {
            let path = Path::new(&override_.path);
            let absolute = if path.is_absolute() {
                normalize_path(path)
            } else {
                absolute_path(&bound.project.source_dir.join(path))
                    .unwrap_or_else(|_| normalize_path(&bound.project.source_dir.join(path)))
            };
            text.push_str(".lib \"");
            text.push_str(&absolute.to_string_lossy());
            text.push_str("\" ");
            text.push_str(&override_.section);
            text.push('\n');
        }
    }
    if !include_control {
        text.push_str(".end\n");
        return Ok(text);
    }
    let step_s = analog.analysis.max_step_us / 1_000_000.0;
    let stop_s = analog.analysis.stop_time_us / 1_000_000.0;
    let uic = if uses_capacitor_initial_conditions(bound, scenario) {
        " uic"
    } else {
        ""
    };
    text.push_str(".control\n");
    text.push_str("set wr_vecnames\n");
    text.push_str("set wr_singlescale\n");
    text.push_str(&format!("tran {:.12e} {:.12e}{uic}\n", step_s, stop_s));
    text.push_str("wrdata ");
    text.push_str(&waveform.to_string_lossy());
    for probe in &analog.probes {
        text.push(' ');
        text.push_str(&probe.expression);
    }
    for expression in operating_probe_expressions {
        text.push(' ');
        text.push_str(expression);
    }
    if include_quit {
        text.push_str("\nquit");
    }
    text.push_str("\n.endc\n.end\n");
    Ok(text)
}

fn build_ngspice_ac_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    raw_output: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before AC wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let start_hz = analog.analysis.start_frequency_hz.ok_or_else(|| {
        "analog.analysis.start_frequency_hz is required for AC analysis.".to_string()
    })?;
    let stop_hz = analog.analysis.stop_frequency_hz.ok_or_else(|| {
        "analog.analysis.stop_frequency_hz is required for AC analysis.".to_string()
    })?;
    let points_per_decade = analog.analysis.points_per_decade.unwrap_or(20);
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI. Do not edit by hand.\n");
    text.push_str("* Source netlist: ");
    text.push_str(&netlist.to_string_lossy());
    text.push('\n');
    let include_base = netlist.parent().unwrap_or(&bound.project.source_dir);
    for line in source.lines() {
        let trimmed = line.trim_start();
        let directive = trimmed.to_ascii_lowercase();
        let first_token = directive.split_whitespace().next().unwrap_or("");
        if matches!(first_token, ".end" | ".tran" | ".ac") {
            continue;
        }
        text.push_str(&rewrite_include_line(line, include_base));
        text.push('\n');
    }
    if !parameter_overrides.is_empty() {
        text.push_str("* CircuitCI sweep parameter overrides.\n");
        for override_ in parameter_overrides {
            text.push_str(".param ");
            text.push_str(&override_.name);
            text.push('=');
            text.push_str(&format!("{:.12e}", override_.value));
            text.push('\n');
        }
        if let Some(temperature_c) = sweep_temperature_c(parameter_overrides) {
            text.push_str(".temp ");
            text.push_str(&format!("{:.12e}", temperature_c));
            text.push('\n');
        }
    }
    if !model_section_overrides.is_empty() {
        text.push_str("* CircuitCI sweep model section overrides.\n");
        for override_ in model_section_overrides {
            let path = Path::new(&override_.path);
            let absolute = if path.is_absolute() {
                normalize_path(path)
            } else {
                absolute_path(&bound.project.source_dir.join(path))
                    .unwrap_or_else(|_| normalize_path(&bound.project.source_dir.join(path)))
            };
            text.push_str(".lib \"");
            text.push_str(&absolute.to_string_lossy());
            text.push_str("\" ");
            text.push_str(&override_.section);
            text.push('\n');
        }
    }
    text.push_str(".control\n");
    text.push_str("set wr_vecnames\n");
    text.push_str("set wr_singlescale\n");
    text.push_str(&format!(
        "ac dec {} {:.12e} {:.12e}\n",
        points_per_decade, start_hz, stop_hz
    ));
    text.push_str("wrdata ");
    text.push_str(&raw_output.to_string_lossy());
    for probe in &analog.probes {
        text.push(' ');
        text.push_str(&probe.expression);
    }
    text.push_str("\nquit\n.endc\n.end\n");
    Ok(text)
}

pub(super) fn ac_raw_to_bode_csv(
    raw: &Path,
    analog: &crate::board_ir::AnalogScenario,
) -> Result<String, String> {
    let text = fs::read_to_string(raw)
        .map_err(|error| format!("Failed to read AC raw export {}: {error}", raw.display()))?;
    let mut csv = String::from("frequency_hz");
    for probe in &analog.probes {
        let name = sanitize_csv_column(&probe.name);
        csv.push_str(&format!(",{name}_mag_db,{name}_phase_deg,{name}_mag"));
    }
    csv.push('\n');
    let expected_columns = 1 + analog.probes.len() * 2;
    let mut row_count = 0usize;
    for (line_index, line) in text.lines().enumerate() {
        let fields: Vec<_> = line
            .split(|character: char| character == ',' || character.is_whitespace())
            .filter(|field| !field.is_empty())
            .collect();
        if fields.is_empty() {
            continue;
        }
        let Some(frequency_hz) = parse_float(fields[0]) else {
            if row_count == 0 {
                continue;
            }
            return Err(format!(
                "AC row {} has non-numeric frequency value {}.",
                line_index + 1,
                fields[0]
            ));
        };
        if fields.len() < expected_columns {
            return Err(format!(
                "AC row {} has {} columns, expected at least {}.",
                line_index + 1,
                fields.len(),
                expected_columns
            ));
        }
        csv.push_str(&format!("{frequency_hz:.12e}"));
        for probe_index in 0..analog.probes.len() {
            let real_index = 1 + probe_index * 2;
            let imag_index = real_index + 1;
            let real = parse_float(fields[real_index]).ok_or_else(|| {
                format!(
                    "AC row {} has non-numeric real value {}.",
                    line_index + 1,
                    fields[real_index]
                )
            })?;
            let imag = parse_float(fields[imag_index]).ok_or_else(|| {
                format!(
                    "AC row {} has non-numeric imaginary value {}.",
                    line_index + 1,
                    fields[imag_index]
                )
            })?;
            let magnitude = real.hypot(imag);
            let magnitude_db = if magnitude > 0.0 {
                20.0 * magnitude.log10()
            } else {
                -300.0
            };
            let phase_deg = imag.atan2(real).to_degrees();
            csv.push_str(&format!(
                ",{magnitude_db:.12e},{phase_deg:.12e},{magnitude:.12e}"
            ));
        }
        csv.push('\n');
        row_count += 1;
    }
    if row_count == 0 {
        return Err(format!(
            "AC raw export {} has no numeric rows.",
            raw.display()
        ));
    }
    Ok(csv)
}

fn sanitize_csv_column(name: &str) -> String {
    let mut output = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "probe".to_string()
    } else {
        output
    }
}

pub(super) fn sweep_temperature_c(parameter_overrides: &[ParameterOverride]) -> Option<f64> {
    parameter_overrides
        .iter()
        .find(|override_| {
            override_.name.eq_ignore_ascii_case("TEMP_C")
                || override_.name.eq_ignore_ascii_case("TEMPERATURE_C")
        })
        .map(|override_| override_.value)
}

fn uses_capacitor_initial_conditions(bound: &BoundBoard<'_>, scenario: &Scenario) -> bool {
    let Some(analog) = &scenario.analog else {
        return false;
    };
    let Some(generated) = &analog.generated else {
        return false;
    };
    generated.components.iter().any(|component_id| {
        bound
            .project
            .board
            .components
            .get(component_id)
            .and_then(|component| component.spice.as_ref())
            .is_some_and(|spice| {
                matches!(spice.primitive, crate::board_ir::SpicePrimitive::Capacitor)
                    && spice.initial_v.is_some()
            })
    })
}

pub(super) fn rewrite_include_line(line: &str, source_dir: &Path) -> String {
    let trimmed = line.trim_start();
    let lowercase = trimmed.to_ascii_lowercase();
    if !lowercase.starts_with(".include ") && !lowercase.starts_with(".lib ") {
        return line.to_string();
    }
    let Some((directive, rest)) = trimmed.split_once(char::is_whitespace) else {
        return line.to_string();
    };
    let path_text = rest.trim();
    let quote = if path_text.starts_with('"') {
        Some('"')
    } else if path_text.starts_with('\'') {
        Some('\'')
    } else {
        None
    };
    let path_token = quote
        .and_then(|mark| path_text[1..].split_once(mark).map(|(path, _)| path))
        .unwrap_or_else(|| path_text.split_whitespace().next().unwrap_or(path_text));
    let path = Path::new(path_token);
    if path.is_absolute() {
        return line.to_string();
    }
    let absolute = absolute_path(&source_dir.join(path))
        .unwrap_or_else(|_| normalize_path(&source_dir.join(path)));
    let indent_len = line.len() - trimmed.len();
    let indent = &line[..indent_len];
    format!("{indent}{directive} \"{}\"", absolute.to_string_lossy())
}

pub(super) fn parse_waveform_csv(
    path: &Path,
    probe_count: usize,
) -> Result<WaveformSeries, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read waveform export {}: {error}", path.display()))?;
    let mut time_s = Vec::new();
    let mut values_by_probe = vec![Vec::new(); probe_count];
    for (index, line) in text.lines().enumerate() {
        let fields: Vec<_> = line
            .split(|character: char| character == ',' || character.is_whitespace())
            .filter(|field| !field.is_empty())
            .collect();
        if fields.is_empty() {
            continue;
        }
        let Some(time) = parse_float(fields[0]) else {
            if index == 0 {
                continue;
            }
            return Err(format!(
                "Waveform row {} in {} has non-numeric time value {}.",
                index + 1,
                path.display(),
                fields[0]
            ));
        };
        if fields.len() < probe_count + 1 {
            return Err(format!(
                "Waveform row {} in {} has {} columns, expected at least {}.",
                index + 1,
                path.display(),
                fields.len(),
                probe_count + 1
            ));
        }
        if time_s.last().is_some_and(|previous| time <= *previous) {
            return Err(format!(
                "Waveform row {} in {} has non-increasing time value {}.",
                index + 1,
                path.display(),
                fields[0]
            ));
        }
        time_s.push(time);
        for probe_index in 0..probe_count {
            let value = parse_float(fields[probe_index + 1]).ok_or_else(|| {
                format!(
                    "Waveform row {} in {} has non-numeric probe value {}.",
                    index + 1,
                    path.display(),
                    fields[probe_index + 1]
                )
            })?;
            values_by_probe[probe_index].push(value);
        }
    }
    if time_s.is_empty() {
        return Err(format!(
            "Waveform export {} has no numeric samples.",
            path.display()
        ));
    }
    Ok(WaveformSeries {
        time_s,
        values_by_probe,
    })
}

pub(super) fn parse_float(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

pub(super) enum BackendSelection {
    Selected(&'static str),
    Unavailable,
    EmbeddedUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AnalogRuntimeFeature {
    Transient,
    Ac,
    Dc,
    DcSweep,
    Noise,
    SParameter,
    TransferFunction,
    PoleZero,
    Sensitivity,
    Fourier,
    HarmonicBalance,
    PeriodicSteadyState,
    PhaseNoise,
    Measure,
}

impl AnalogRuntimeFeature {
    fn supports_embedded_ngspice(self) -> bool {
        matches!(self, Self::Transient)
    }
}

pub(super) fn external_backend_unavailable(
    scenario_name: &str,
    backend: &AnalogBackend,
) -> Finding {
    let mut finding = Finding::critical(
        "ANALOG_BACKEND_UNAVAILABLE",
        scenario_name,
        "Physical analog simulation requires ngspice, Xyce, or a linked embedded ngspice backend, but no requested solver is available.",
    );
    finding.limit.insert(
        "required_backend".to_string(),
        json!("ngspice_xyce_or_embedded_ngspice"),
    );
    finding.suggested_fixes.push(
        "Install ngspice/Xyce or build CircuitCI with a mature embedded ngspice backend."
            .to_string(),
    );
    finding.suggested_fixes.push(
        "Keep behavioral control-line checks marked as non-physical until this simulation runs."
            .to_string(),
    );
    if *backend == AnalogBackend::EmbeddedNgspice {
        finding.id = "ANALOG_EMBEDDED_SOLVER_UNAVAILABLE".to_string();
    }
    finding
}

pub(super) fn embedded_solver_unavailable(scenario_name: &str) -> Finding {
    let mut finding = Finding::critical(
        "ANALOG_EMBEDDED_SOLVER_UNAVAILABLE",
        scenario_name,
        "The embedded_ngspice backend was requested, but no mature ngspice-derived engine is linked into this CircuitCI build.",
    );
    finding
        .limit
        .insert("required_backend".to_string(), json!("embedded_ngspice"));
    finding.suggested_fixes.push(
        "Vendor or link a mature ngspice-derived solver through the analog adapter; do not replace it with a partial toy SPICE subset."
            .to_string(),
    );
    finding
}

pub(super) fn select_backend_for_feature(
    requested: &AnalogBackend,
    feature: AnalogRuntimeFeature,
) -> BackendSelection {
    match requested {
        AnalogBackend::Ngspice => {
            if executable_on_path("ngspice") {
                BackendSelection::Selected("ngspice")
            } else {
                BackendSelection::Unavailable
            }
        }
        AnalogBackend::Xyce => {
            if executable_on_path("Xyce") {
                BackendSelection::Selected("Xyce")
            } else if executable_on_path("xyce") {
                BackendSelection::Selected("xyce")
            } else {
                BackendSelection::Unavailable
            }
        }
        AnalogBackend::Auto => {
            if executable_on_path("ngspice") {
                BackendSelection::Selected("ngspice")
            } else if feature.supports_embedded_ngspice() && embedded_ngspice_available() {
                BackendSelection::Selected("embedded_ngspice")
            } else {
                BackendSelection::Unavailable
            }
        }
        AnalogBackend::EmbeddedNgspice => {
            if !feature.supports_embedded_ngspice() {
                BackendSelection::EmbeddedUnavailable
            } else if embedded_ngspice_available() {
                BackendSelection::Selected("embedded_ngspice")
            } else {
                BackendSelection::EmbeddedUnavailable
            }
        }
    }
}

pub(super) fn backend_name(backend: &AnalogBackend) -> &'static str {
    match backend {
        AnalogBackend::Auto => "auto",
        AnalogBackend::Ngspice => "ngspice",
        AnalogBackend::Xyce => "xyce",
        AnalogBackend::EmbeddedNgspice => "embedded_ngspice",
    }
}

pub(super) struct SolverManifestIo<'a> {
    pub(super) run_dir: &'a Path,
    pub(super) scenario: &'a Scenario,
    pub(super) requested_backend: &'a AnalogBackend,
    pub(super) selected_backend: &'a str,
    pub(super) analysis_kind: &'a str,
    pub(super) source_netlist: &'a Path,
    pub(super) wrapper: &'a Path,
    pub(super) log: &'a Path,
    pub(super) output: &'a SolverOutput,
    pub(super) parameter_overrides: &'a [ParameterOverride],
    pub(super) model_section_overrides: &'a [ModelSectionOverride],
    pub(super) raw_outputs: &'a [(&'a str, &'a Path)],
    pub(super) normalized_outputs: &'a [(&'a str, &'a Path)],
}

pub(super) fn write_solver_manifest(io: SolverManifestIo<'_>) -> Result<PathBuf, String> {
    let analog = io
        .scenario
        .analog
        .as_ref()
        .expect("analog was validated before solver manifest generation");
    let manifest = io.run_dir.join("solver_manifest.json");
    let parameter_overrides: Vec<_> = io
        .parameter_overrides
        .iter()
        .map(|override_| {
            json!({
                "name": override_.name,
                "value": override_.value,
            })
        })
        .collect();
    let model_section_overrides: Vec<_> = io
        .model_section_overrides
        .iter()
        .map(|override_| {
            json!({
                "path": override_.path,
                "section": override_.section,
            })
        })
        .collect();
    let raw_outputs = path_entries(io.raw_outputs);
    let normalized_outputs = path_entries(io.normalized_outputs);
    let model_files: Vec<_> = analog
        .model_files
        .iter()
        .map(|model_file| {
            json!({
                "path": model_file.path,
                "sha256": model_file.sha256,
            })
        })
        .collect();
    let value = json!({
        "schema_version": ANALOG_SOLVER_MANIFEST_SCHEMA,
        "scenario": io.scenario.name,
        "scenario_type": io.scenario.scenario_type,
        "analysis": {
            "kind": io.analysis_kind,
            "type": analog.analysis.analysis_type,
        },
        "backend": {
            "requested": backend_name(io.requested_backend),
            "selected": io.selected_backend,
        },
        "execution": {
            "command": io.output.command,
            "status": io.output.status.to_string(),
            "success": io.output.status.success(),
            "stdout_bytes": io.output.stdout.len(),
            "stderr_bytes": io.output.stderr.len(),
        },
        "inputs": {
            "source_netlist": normalize_artifact_path(io.source_netlist),
            "wrapper": normalize_artifact_path(io.wrapper),
            "model_files": model_files,
            "parameter_overrides": parameter_overrides,
            "model_section_overrides": model_section_overrides,
        },
        "outputs": {
            "log": normalize_artifact_path(io.log),
            "raw": raw_outputs,
            "normalized": normalized_outputs,
        },
    });
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("Failed to serialize analog solver manifest: {error}"))?;
    fs::write(&manifest, text).map_err(|error| {
        format!(
            "Failed to write analog solver manifest {}: {error}",
            manifest.display()
        )
    })?;
    Ok(manifest)
}

fn path_entries(entries: &[(&str, &Path)]) -> Vec<serde_json::Value> {
    entries
        .iter()
        .map(|(kind, path)| {
            json!({
                "kind": kind,
                "path": normalize_artifact_path(path),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        ModelSectionOverride, NgspiceWrapperOptions, ParameterOverride, ac_raw_to_bode_csv,
        build_ngspice_wrapper, detect_nonconvergence, parse_waveform_csv, rewrite_include_line,
    };
    use crate::board_ir::{BoardProject, load_project};
    use crate::library::{bind_project, load_library};
    use crate::validation::analog_operating_limits::{
        operating_limit_probes, operating_probe_expressions,
    };
    use crate::validation::analog_waveform_measurements::interpolate_at;
    use std::path::Path;

    #[test]
    fn parser_skips_header_and_interpolates_samples() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("waveform.csv");
        std::fs::write(
            &path,
            "time v(boot0) v(nrst)
0.0 0.0 0.0
0.0001 1.0 2.0
",
        )
        .unwrap();
        let series = parse_waveform_csv(&path, 2).unwrap();
        assert_eq!(series.time_s.len(), 2);
        assert_eq!(
            interpolate_at(&series.time_s, &series.values_by_probe[0], 0.00005).unwrap(),
            0.5
        );
    }

    #[test]
    fn parser_rejects_non_finite_probe_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("waveform.csv");
        std::fs::write(
            &path,
            "time v(boot0)
0.0 NaN
",
        )
        .unwrap();
        let error = parse_waveform_csv(&path, 1).unwrap_err();
        assert!(error.contains("non-numeric probe value"));
    }

    #[test]
    fn parser_rejects_non_increasing_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("waveform.csv");
        std::fs::write(
            &path,
            "time v(boot0)
0.0 0.0
0.0 1.0
",
        )
        .unwrap();
        let error = parse_waveform_csv(&path, 1).unwrap_err();
        assert!(error.contains("non-increasing time"));
    }

    #[test]
    fn ac_raw_export_converts_to_bode_columns() {
        let project: BoardProject = serde_yaml_ng::from_str(
            r#"
project:
  name: ac_test
  version: 0.1.0
board:
  components: {}
  nets: {}
scenarios:
  - name: ac_response
    type: analog_ac
    checks: [SPICE_AC_ANALYSIS]
    analog:
      backend: auto
      netlist_source: file
      netlist: deck_ac.cir
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis:
        type: ac
        start_frequency_hz: 10.0
        stop_frequency_hz: 100000.0
      stimuli: []
      probes:
        - name: input
          expression: V(input)
        - name: filtered
          expression: V(filtered)
      assertions: []
"#,
        )
        .unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ac_raw.csv");
        std::fs::write(
            &path,
            "frequency v(input) v(input) v(filtered) v(filtered)
1.000000e3 1.0 0.0 0.5 -0.5
",
        )
        .unwrap();

        let bode = ac_raw_to_bode_csv(&path, analog).unwrap();

        assert!(bode.starts_with(
            "frequency_hz,input_mag_db,input_phase_deg,input_mag,filtered_mag_db,filtered_phase_deg,filtered_mag"
        ));
        assert!(bode.contains("1.000000000000e3"));
        assert!(bode.contains("-3.010299956640e0"));
        assert!(bode.contains("-4.500000000000e1"));
    }

    #[test]
    fn wrapper_keeps_user_probes_before_operating_probes() {
        let project_path = Path::new("examples/bad_mosfet_overcurrent/project.yaml");
        let project = load_project(project_path).unwrap();
        let (library, findings) = load_library(project_path, &project);
        assert!(findings.is_empty());
        let bound = bind_project(&project, library, findings);
        let scenario = &project.scenarios[0];
        let operating = operating_limit_probes(&bound, scenario);
        let operating_expressions = operating_probe_expressions(&operating);
        assert!(
            operating
                .probes
                .iter()
                .any(|probe| probe.expression == "abs(I(VCCI_M1))")
        );

        let dir = tempfile::tempdir().unwrap();
        let netlist = dir.path().join("source.cir");
        std::fs::write(&netlist, "VDD vdd 0 5\n.end\n").unwrap();
        let wrapper = build_ngspice_wrapper(
            &bound,
            scenario,
            &netlist,
            Path::new("waveform.csv"),
            NgspiceWrapperOptions {
                parameter_overrides: &[],
                model_section_overrides: &[],
                operating_probe_expressions: &operating_expressions,
                include_control: true,
                include_quit: true,
            },
        )
        .unwrap();
        let wrdata = wrapper
            .lines()
            .find(|line| line.starts_with("wrdata "))
            .unwrap();
        let user_probe = wrdata.find("V(sw)").unwrap();
        let operating_probe = wrdata.find("abs(I(VCCI_M1))").unwrap();
        assert!(user_probe < operating_probe);
    }

    #[test]
    fn wrapper_injects_parameter_overrides() {
        let project_path = Path::new("examples/rc_lowpass_scope/project.yaml");
        let project = load_project(project_path).unwrap();
        let (library, findings) = load_library(project_path, &project);
        assert!(findings.is_empty());
        let bound = bind_project(&project, library, findings);
        let scenario = &project.scenarios[0];

        let dir = tempfile::tempdir().unwrap();
        let netlist = dir.path().join("source.cir");
        std::fs::write(&netlist, "R1 in out {R_LOAD}\n.end\n").unwrap();
        let wrapper = build_ngspice_wrapper(
            &bound,
            scenario,
            &netlist,
            Path::new("waveform.csv"),
            NgspiceWrapperOptions {
                parameter_overrides: &[ParameterOverride {
                    name: "R_LOAD".to_string(),
                    value: 1234.0,
                }],
                model_section_overrides: &[],
                operating_probe_expressions: &[],
                include_control: true,
                include_quit: true,
            },
        )
        .unwrap();
        assert!(wrapper.contains(".param R_LOAD=1.234000000000e3"));
        assert!(wrapper.contains("R1 in out {R_LOAD}"));
    }

    #[test]
    fn wrapper_maps_temp_c_override_to_ngspice_temperature() {
        let project_path = Path::new("examples/rc_lowpass_scope/project.yaml");
        let project = load_project(project_path).unwrap();
        let (library, findings) = load_library(project_path, &project);
        assert!(findings.is_empty());
        let bound = bind_project(&project, library, findings);
        let scenario = &project.scenarios[0];

        let dir = tempfile::tempdir().unwrap();
        let netlist = dir.path().join("source.cir");
        std::fs::write(&netlist, "R1 in out 1k\n.end\n").unwrap();
        let wrapper = build_ngspice_wrapper(
            &bound,
            scenario,
            &netlist,
            Path::new("waveform.csv"),
            NgspiceWrapperOptions {
                parameter_overrides: &[ParameterOverride {
                    name: "TEMP_C".to_string(),
                    value: 85.0,
                }],
                model_section_overrides: &[],
                operating_probe_expressions: &[],
                include_control: true,
                include_quit: true,
            },
        )
        .unwrap();
        assert!(wrapper.contains(".param TEMP_C=8.500000000000e1"));
        assert!(wrapper.contains(".temp 8.500000000000e1"));
    }

    #[test]
    fn wrapper_injects_model_section_overrides() {
        let project_path = Path::new("examples/rc_lowpass_scope/project.yaml");
        let project = load_project(project_path).unwrap();
        let (library, findings) = load_library(project_path, &project);
        assert!(findings.is_empty());
        let bound = bind_project(&project, library, findings);
        let scenario = &project.scenarios[0];

        let dir = tempfile::tempdir().unwrap();
        let netlist = dir.path().join("source.cir");
        std::fs::write(&netlist, "X1 in out vendor_device\n.end\n").unwrap();
        let wrapper = build_ngspice_wrapper(
            &bound,
            scenario,
            &netlist,
            Path::new("waveform.csv"),
            NgspiceWrapperOptions {
                parameter_overrides: &[],
                model_section_overrides: &[ModelSectionOverride {
                    path: "models/vendor.lib".to_string(),
                    section: "slow".to_string(),
                }],
                operating_probe_expressions: &[],
                include_control: true,
                include_quit: true,
            },
        )
        .unwrap();
        assert!(wrapper.lines().any(|line| {
            line.starts_with(".lib \"") && line.ends_with("models/vendor.lib\" slow")
        }));
    }

    #[test]
    fn include_rewriting_absolutizes_relative_model_paths() {
        let dir = tempfile::tempdir().unwrap();
        let rewritten = rewrite_include_line(".include models/device.lib", dir.path());
        assert!(rewritten.starts_with(".include \""));
        assert!(rewritten.ends_with("models/device.lib\""));

        let absolute = rewrite_include_line(".lib /tmp/model.lib", dir.path());
        assert_eq!(absolute, ".lib /tmp/model.lib");
    }

    #[cfg(unix)]
    #[test]
    fn external_solver_cancellation_kills_child_process() {
        use std::os::unix::fs::PermissionsExt;
        use std::time::{Duration, Instant};

        let dir = tempfile::tempdir().unwrap();
        let wrapper = dir.path().join("wrapper.cir");
        std::fs::write(&wrapper, ".end\n").unwrap();
        let solver = dir.path().join("fake_solver.sh");
        std::fs::write(&solver, "#!/bin/sh\nsleep 10\n").unwrap();
        let mut permissions = std::fs::metadata(&solver).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&solver, permissions).unwrap();

        let started = Instant::now();
        let solver = solver.to_string_lossy().into_owned();
        let error = match super::run_solver_with_timeout(
            &solver,
            &wrapper,
            Duration::from_secs(30),
            None,
            || true,
        ) {
            Ok(_) => panic!("canceled solver should not complete successfully"),
            Err(error) => error,
        };

        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(error.contains("was canceled"));
    }

    #[test]
    fn nonconvergence_detection_matches_ngspice_failure_text() {
        assert_eq!(
            detect_nonconvergence("Warning: timestep too small; trouble with node x"),
            Some("timestep too small")
        );
        assert_eq!(
            detect_nonconvergence("doAnalyses: iteration limit reached"),
            Some("iteration limit reached")
        );
        assert_eq!(detect_nonconvergence("analysis completed"), None);
    }
}
