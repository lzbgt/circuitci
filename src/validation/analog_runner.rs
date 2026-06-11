use crate::board_ir::{AnalogBackend, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::analog_operating_limits::OperatingLimitProbe;
use super::analog_util::{absolute_path, executable_on_path, normalize_path, safe_artifact_name};

pub(super) struct NgspiceRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) waveform: PathBuf,
    pub(super) series: WaveformSeries,
    pub(super) user_probe_count: usize,
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

pub(super) fn run_ngspice(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    output: &Path,
    source_netlist: &Path,
    operating_probes: &[OperatingLimitProbe],
) -> Result<NgspiceRun, NgspiceRunError> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before run");
    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
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
    let wrapper_text = build_ngspice_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("waveform.csv"),
        operating_probes,
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

    let output = run_solver_with_timeout(backend, &wrapper, Duration::from_secs(60))
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    let mut log_text = String::new();
    log_text.push_str("COMMAND: ");
    log_text.push_str("cd ");
    log_text.push_str(
        &wrapper
            .parent()
            .map(Path::to_string_lossy)
            .unwrap_or_default(),
    );
    log_text.push_str(" && ");
    log_text.push_str(backend);
    log_text.push_str(" -b ");
    log_text.push_str(
        &wrapper
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default(),
    );
    log_text.push_str("\n\nSTDOUT:\n");
    log_text.push_str(&String::from_utf8_lossy(&output.stdout));
    log_text.push_str("\n\nSTDERR:\n");
    log_text.push_str(&String::from_utf8_lossy(&output.stderr));
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
    let probe_count = analog.probes.len() + operating_probes.len();
    let series = parse_waveform_csv(&waveform, probe_count)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    Ok(NgspiceRun {
        artifacts,
        waveform,
        series,
        user_probe_count: analog.probes.len(),
    })
}

struct SolverOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_solver_with_timeout(
    backend: &str,
    wrapper: &Path,
    timeout: Duration,
) -> Result<SolverOutput, String> {
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
                    status: output.status,
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
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(format!("Failed while waiting for ngspice: {error}")),
        }
    }
}

fn ngspice_error(message: impl Into<String>, artifacts: Vec<PathBuf>) -> NgspiceRunError {
    NgspiceRunError {
        message: message.into(),
        artifacts,
    }
}

fn detect_nonconvergence(log: &str) -> Option<&'static str> {
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
    operating_probes: &[OperatingLimitProbe],
) -> Result<String, String> {
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
    let step_s = analog.analysis.max_step_us / 1_000_000.0;
    let stop_s = analog.analysis.stop_time_us / 1_000_000.0;
    text.push_str(".control\n");
    text.push_str("set wr_vecnames\n");
    text.push_str("set wr_singlescale\n");
    text.push_str(&format!("tran {:.12e} {:.12e}\n", step_s, stop_s));
    text.push_str("wrdata ");
    text.push_str(&waveform.to_string_lossy());
    for probe in &analog.probes {
        text.push(' ');
        text.push_str(&probe.expression);
    }
    for probe in operating_probes {
        text.push(' ');
        text.push_str(&probe.expression);
    }
    text.push_str("\nquit\n.endc\n.end\n");
    Ok(text)
}

fn rewrite_include_line(line: &str, source_dir: &Path) -> String {
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

fn parse_waveform_csv(path: &Path, probe_count: usize) -> Result<WaveformSeries, String> {
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

fn parse_float(value: &str) -> Option<f64> {
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

pub(super) fn select_backend(requested: &AnalogBackend) -> BackendSelection {
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
            } else if executable_on_path("Xyce") {
                BackendSelection::Selected("Xyce")
            } else if executable_on_path("xyce") {
                BackendSelection::Selected("xyce")
            } else {
                BackendSelection::Unavailable
            }
        }
        AnalogBackend::EmbeddedNgspice => BackendSelection::EmbeddedUnavailable,
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

#[cfg(test)]
mod tests {
    use super::{
        build_ngspice_wrapper, detect_nonconvergence, parse_waveform_csv, rewrite_include_line,
    };
    use crate::board_ir::load_project;
    use crate::library::{bind_project, load_library};
    use crate::validation::analog_assertions::interpolate_at;
    use crate::validation::analog_operating_limits::operating_limit_probes;
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
    fn wrapper_keeps_user_probes_before_operating_probes() {
        let project_path = Path::new("examples/bad_mosfet_overcurrent/project.yaml");
        let project = load_project(project_path).unwrap();
        let (library, findings) = load_library(project_path, &project);
        assert!(findings.is_empty());
        let bound = bind_project(&project, library, findings);
        let scenario = &project.scenarios[0];
        let operating = operating_limit_probes(&bound, scenario);
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
            &operating.probes,
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
    fn include_rewriting_absolutizes_relative_model_paths() {
        let dir = tempfile::tempdir().unwrap();
        let rewritten = rewrite_include_line(".include models/device.lib", dir.path());
        assert!(rewritten.starts_with(".include \""));
        assert!(rewritten.ends_with("models/device.lib\""));

        let absolute = rewrite_include_line(".lib /tmp/model.lib", dir.path());
        assert_eq!(absolute, ".lib /tmp/model.lib");
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
