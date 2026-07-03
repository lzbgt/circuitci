use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::analog_runner::{
    ModelSectionOverride, NgspiceRun, NgspiceRunError, ParameterOverride, SolverManifestIo,
    SolverOutput, SolverStatus, WaveformSeries, detect_nonconvergence, ngspice_error, parse_float,
    parse_waveform_csv, rewrite_include_line, sweep_temperature_c, write_solver_manifest,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};

pub(super) struct XyceTransientRunOptions<'a, F, C>
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

pub(super) fn run_xyce_transient<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: XyceTransientRunOptions<'_, F, C>,
) -> Result<NgspiceRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let XyceTransientRunOptions {
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
        .expect("analog was validated before Xyce run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog Xyce run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_xyce.cir");
    let log = run_dir.join("xyce.log");
    let raw = run_dir.join("waveform_raw.csv");
    let waveform = run_dir.join("waveform.csv");
    on_progress(
        "Writing analog Xyce wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_xyce_transient_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("waveform_raw.csv"),
        parameter_overrides,
        model_section_overrides,
        operating_probe_expressions,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog Xyce backend",
        format!(
            "{} transient for {} user probe(s) and {} operating probe(s).",
            backend,
            analog.probes.len(),
            operating_probe_expressions.len()
        ),
    );
    let output = run_xyce_with_timeout(backend, &wrapper, Duration::from_secs(60), should_cancel)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    let mut log_text = String::new();
    log_text.push_str("COMMAND: ");
    log_text.push_str(&output.command);
    log_text.push_str("\n\nSTDOUT:\n");
    log_text.push_str(&String::from_utf8_lossy(&output.stdout));
    log_text.push_str("\n\nSTDERR:\n");
    log_text.push_str(&String::from_utf8_lossy(&output.stderr));
    fs::write(&log, &log_text).map_err(|error| {
        ngspice_error(
            format!("Failed to write Xyce log {}: {error}", log.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "Xyce transient analysis exited with status {}.",
                output.status
            ),
            artifacts,
        ));
    }
    if let Some(reason) = detect_nonconvergence(&log_text) {
        return Err(ngspice_error(
            format!("Xyce reported non-convergence or numerical failure: {reason}."),
            artifacts,
        ));
    }
    if !raw.is_file() {
        return Err(ngspice_error(
            format!(
                "Xyce completed without producing transient export {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    let probe_count = analog.probes.len() + operating_probe_expressions.len();
    let series = xyce_transient_raw_to_waveform_csv(&raw, &waveform, probe_count)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(waveform.clone());
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
        raw_outputs: &[("xyce_transient_raw", &raw)],
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

fn build_xyce_transient_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    raw_output: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
    operating_probe_expressions: &[String],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before Xyce wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for Xyce. Do not edit by hand.\n");
    text.push_str("* Source netlist: ");
    text.push_str(&netlist.to_string_lossy());
    text.push('\n');
    let include_base = netlist.parent().unwrap_or(&bound.project.source_dir);
    let mut in_control_block = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let directive = trimmed.to_ascii_lowercase();
        let first_token = directive.split_whitespace().next().unwrap_or("");
        if first_token == ".control" {
            in_control_block = true;
            continue;
        }
        if in_control_block {
            if first_token == ".endc" {
                in_control_block = false;
            }
            continue;
        }
        if matches!(first_token, ".end" | ".tran" | ".print") {
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
    let step_s = analog.analysis.max_step_us / 1_000_000.0;
    let stop_s = analog.analysis.stop_time_us / 1_000_000.0;
    text.push_str(&format!(".TRAN {:.12e} {:.12e}\n", step_s, stop_s));
    text.push_str(".PRINT TRAN FORMAT=CSV FILE=\"");
    text.push_str(&raw_output.to_string_lossy());
    text.push('"');
    for probe in &analog.probes {
        text.push(' ');
        text.push_str(&probe.expression);
    }
    for expression in operating_probe_expressions {
        text.push(' ');
        text.push_str(expression);
    }
    text.push_str("\n.END\n");
    Ok(text)
}

fn run_xyce_with_timeout(
    backend: &str,
    wrapper: &Path,
    timeout: Duration,
    should_cancel: impl Fn() -> bool,
) -> Result<SolverOutput, String> {
    let working_dir = wrapper
        .parent()
        .ok_or_else(|| format!("Xyce wrapper path {} has no parent.", wrapper.display()))?;
    let deck_name = wrapper
        .file_name()
        .ok_or_else(|| format!("Xyce wrapper path {} has no filename.", wrapper.display()))?;
    let mut child = Command::new(backend)
        .current_dir(working_dir)
        .arg(deck_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to launch Xyce backend {backend}: {error}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect Xyce output: {error}"))?;
                return Ok(SolverOutput {
                    status: SolverStatus::External(output.status),
                    command: format!(
                        "cd {} && {backend} {}",
                        working_dir.to_string_lossy(),
                        deck_name.to_string_lossy()
                    ),
                    stdout: output.stdout,
                    stderr: output.stderr,
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect timed-out Xyce output: {error}"))?;
                return Err(format!(
                    "Xyce transient analysis exceeded {} seconds and was terminated. Stdout bytes: {}, stderr bytes: {}.",
                    timeout.as_secs(),
                    output.stdout.len(),
                    output.stderr.len()
                ));
            }
            Ok(None) if should_cancel() => {
                let _ = child.kill();
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect canceled Xyce output: {error}"))?;
                return Err(format!(
                    "Xyce transient analysis was canceled and the external process was terminated. Stdout bytes: {}, stderr bytes: {}.",
                    output.stdout.len(),
                    output.stderr.len()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(format!("Failed while waiting for Xyce: {error}")),
        }
    }
}

fn xyce_transient_raw_to_waveform_csv(
    raw: &Path,
    waveform: &Path,
    probe_count: usize,
) -> Result<WaveformSeries, String> {
    let text = fs::read_to_string(raw).map_err(|error| {
        format!(
            "Failed to read Xyce transient export {}: {error}",
            raw.display()
        )
    })?;
    let mut rows = Vec::new();
    let mut time_col = None;
    let mut value_start_col = None;
    for line in text.lines() {
        let fields = split_xyce_row(line);
        if fields.is_empty() {
            continue;
        }
        if fields
            .iter()
            .any(|field| field.eq_ignore_ascii_case("time"))
        {
            if fields[0].eq_ignore_ascii_case("index") && fields.len() >= probe_count + 2 {
                time_col = Some(1);
                value_start_col = Some(2);
            } else {
                time_col = Some(0);
                value_start_col = Some(1);
            }
            continue;
        }
        let numeric: Vec<_> = fields
            .iter()
            .filter_map(|field| parse_float(field))
            .collect();
        if numeric.is_empty() {
            continue;
        }
        let (time_index, value_index) = match (time_col, value_start_col) {
            (Some(time), Some(value)) => (time, value),
            _ if numeric.len() == probe_count + 2 => (1, 2),
            _ => (0, 1),
        };
        if numeric.len() < value_index + probe_count {
            return Err(format!(
                "Xyce transient row has {} numeric column(s), expected at least {} for {} probe(s).",
                numeric.len(),
                value_index + probe_count,
                probe_count
            ));
        }
        let mut row = Vec::with_capacity(probe_count + 1);
        row.push(numeric[time_index]);
        row.extend_from_slice(&numeric[value_index..value_index + probe_count]);
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(format!(
            "Xyce transient export {} has no numeric samples.",
            raw.display()
        ));
    }
    let mut output = String::new();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&format!("{value:.12e}"));
        }
        output.push('\n');
    }
    fs::write(waveform, output).map_err(|error| {
        format!(
            "Failed to write normalized Xyce waveform {}: {error}",
            waveform.display()
        )
    })?;
    parse_waveform_csv(waveform, probe_count)
}

fn split_xyce_row(line: &str) -> Vec<&str> {
    line.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::xyce_transient_raw_to_waveform_csv;

    #[test]
    fn xyce_transient_raw_csv_normalizes_to_waveform_csv() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("waveform_raw.csv");
        let waveform = dir.path().join("waveform.csv");
        std::fs::write(&raw, "TIME,V(out)\n0,0\n5e-6,1.2\n1e-5,1.3\n").unwrap();

        let series = xyce_transient_raw_to_waveform_csv(&raw, &waveform, 1).unwrap();

        assert_eq!(series.time_s.len(), 3);
        assert_eq!(series.values_by_probe[0][1], 1.2);
        assert!(
            std::fs::read_to_string(waveform)
                .unwrap()
                .contains("5.000000000000e-6")
        );
    }
}
