use crate::board_ir::{AnalogScenario, Scenario};
use crate::library::BoundBoard;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::analog_runner::{
    ModelSectionOverride, NgspiceRunError, ParameterOverride, SolverManifestIo,
    detect_nonconvergence, ngspice_error, parse_float, rewrite_include_line, sweep_temperature_c,
    write_solver_manifest,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};
use super::analog_xyce_runner::run_xyce_with_timeout;

pub(super) struct XyceHarmonicBalanceRunOptions<'a, F, C>
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

pub(super) struct XyceHarmonicBalanceRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) hb_spectrum: PathBuf,
}

pub(super) fn run_xyce_harmonic_balance<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: XyceHarmonicBalanceRunOptions<'_, F, C>,
) -> Result<XyceHarmonicBalanceRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let XyceHarmonicBalanceRunOptions {
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
        .expect("analog was validated before Xyce harmonic-balance run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog Xyce harmonic-balance run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_xyce_hb.cir");
    let log = run_dir.join("xyce_hb.log");
    let raw = run_dir.join("hb_spectrum_raw.csv");
    let hb_spectrum = run_dir.join("hb_spectrum.csv");
    on_progress(
        "Writing analog Xyce harmonic-balance wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_xyce_harmonic_balance_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("hb_spectrum_raw.csv"),
        parameter_overrides,
        model_section_overrides,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce harmonic-balance wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog Xyce harmonic-balance backend",
        format!(
            "{} harmonic balance for {} output.",
            backend,
            analog
                .analysis
                .hb_output_expression
                .as_deref()
                .unwrap_or("<missing>")
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
            format!(
                "Failed to write Xyce harmonic-balance log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "Xyce harmonic-balance analysis exited with status {}.",
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
                "Xyce completed without producing harmonic-balance export {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    let hb_csv = xyce_hb_raw_to_spectrum_csv(&raw, analog).map_err(|message| {
        ngspice_error(
            format!(
                "Failed to convert Xyce harmonic-balance export {}: {message}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&hb_spectrum, hb_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce harmonic-balance CSV {}: {error}",
                hb_spectrum.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(hb_spectrum.clone());
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "harmonic_balance",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("xyce_hb_frequency_domain_raw", &raw)],
        normalized_outputs: &[("hb_spectrum", &hb_spectrum)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(XyceHarmonicBalanceRun {
        artifacts,
        hb_spectrum,
    })
}

fn build_xyce_harmonic_balance_wrapper(
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
        .expect("analog was validated before Xyce harmonic-balance wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let fundamental_hz = analog
        .analysis
        .hb_fundamental_frequency_hz
        .ok_or_else(|| "hb_fundamental_frequency_hz is required for HB analysis".to_string())?;
    let output_expression = analog
        .analysis
        .hb_output_expression
        .as_deref()
        .ok_or_else(|| "hb_output_expression is required for HB analysis".to_string())?;
    let harmonics = analog.analysis.hb_harmonics.unwrap_or(10);
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for Xyce harmonic balance. Do not edit by hand.\n");
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
        if matches!(
            first_token,
            ".end" | ".tran" | ".ac" | ".op" | ".noise" | ".lin" | ".hb" | ".options" | ".print"
        ) {
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
    text.push_str(&format!(".HB {:.12e}\n", fundamental_hz));
    text.push_str(&format!(".OPTIONS HBINT NUMFREQ={harmonics}\n"));
    text.push_str(".PRINT HB_FD FORMAT=CSV FILE=\"");
    text.push_str(&raw_output.to_string_lossy());
    text.push_str("\" FREQ ");
    text.push_str(output_expression);
    text.push_str("\n.END\n");
    Ok(text)
}

fn xyce_hb_raw_to_spectrum_csv(raw: &Path, analog: &AnalogScenario) -> Result<String, String> {
    let text = fs::read_to_string(raw).map_err(|error| {
        format!(
            "Failed to read Xyce harmonic-balance export {}: {error}",
            raw.display()
        )
    })?;
    let output_expression = analog
        .analysis
        .hb_output_expression
        .as_deref()
        .unwrap_or("output");
    let fundamental_hz = analog.analysis.hb_fundamental_frequency_hz.ok_or_else(|| {
        "hb_fundamental_frequency_hz is required for HB normalization.".to_string()
    })?;
    let mut frequency_col = None;
    let mut value_col = None;
    let mut rows = Vec::new();
    for line in text.lines() {
        let fields = split_xyce_row(line);
        if fields.is_empty() {
            continue;
        }
        let has_freq_header = fields.iter().any(|field| {
            let normalized = normalize_header_token(field);
            normalized == "freq" || normalized == "frequency"
        });
        if has_freq_header {
            frequency_col = fields.iter().position(|field| {
                let normalized = normalize_header_token(field);
                normalized == "freq" || normalized == "frequency"
            });
            value_col = frequency_col.map(|index| index + 1);
            continue;
        }
        let numeric: Vec<_> = fields
            .iter()
            .filter_map(|field| parse_float(field))
            .collect();
        if numeric.is_empty() {
            continue;
        }
        let freq_index = frequency_col.unwrap_or(if numeric.len() >= 4 { 1 } else { 0 });
        if numeric.len() <= freq_index {
            continue;
        }
        let value_index = value_col.unwrap_or(freq_index + 1);
        if numeric.len() <= value_index {
            return Err(format!(
                "Xyce HB row has {} numeric column(s), expected a frequency and at least one value.",
                numeric.len()
            ));
        }
        let frequency_hz = numeric[freq_index];
        let real = numeric[value_index];
        let imaginary = numeric.get(value_index + 1).copied().unwrap_or(0.0);
        let magnitude = real.hypot(imaginary);
        let phase_deg = imaginary.atan2(real).to_degrees();
        let harmonic = (frequency_hz / fundamental_hz).round() as i64;
        rows.push((
            harmonic,
            frequency_hz,
            real,
            imaginary,
            magnitude,
            phase_deg,
        ));
    }
    if rows.is_empty() {
        return Err(format!(
            "Xyce harmonic-balance export {} has no numeric spectrum rows.",
            raw.display()
        ));
    }
    let mut output = String::new();
    output.push_str(
        "output_expression,fundamental_frequency_hz,harmonic,frequency_hz,real,imaginary,magnitude,phase_deg\n",
    );
    for (harmonic, frequency_hz, real, imaginary, magnitude, phase_deg) in rows {
        output.push_str(&format!(
            "{},{:.12e},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}\n",
            output_expression,
            fundamental_hz,
            harmonic,
            frequency_hz,
            real,
            imaginary,
            magnitude,
            phase_deg
        ));
    }
    Ok(output)
}

fn split_xyce_row(line: &str) -> Vec<&str> {
    line.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

fn normalize_header_token(token: &str) -> String {
    token
        .trim_matches(|character: char| {
            matches!(
                character,
                '"' | '\'' | '[' | ']' | '(' | ')' | '{' | '}' | '#'
            )
        })
        .to_ascii_lowercase()
}
