use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use libloading::Library;
use std::env;
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::ptr;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::analog_runner::{SolverOutput, SolverStatus, uses_capacitor_initial_conditions};

pub(super) struct EmbeddedCommands {
    pub(super) tran: String,
    pub(super) wrdata: String,
}

impl EmbeddedCommands {
    pub(super) fn new(
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

pub(super) fn run_embedded_ngspice(
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

pub(super) fn embedded_ngspice_available() -> bool {
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
