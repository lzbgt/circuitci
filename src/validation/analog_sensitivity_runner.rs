use crate::board_ir::{Scenario, SpicePrimitive};
use crate::library::BoundBoard;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::analog_runner::{
    ModelSectionOverride, NgspiceRunError, ParameterOverride, SolverManifestIo,
    detect_nonconvergence, ngspice_error, normalized_frequency_sweep_type,
    normalized_frequency_sweep_type_upper, push_ngspice_osdi_load_commands, rewrite_include_line,
    run_solver_with_timeout, sweep_temperature_c, write_solver_manifest,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};
use super::analog_xyce_runner::run_xyce_with_timeout;

pub(super) struct NgspiceSensitivityRunOptions<'a, F, C>
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

pub(super) struct XyceSensitivityRunOptions<'a, F, C>
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

pub(super) struct NgspiceSensitivityRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) summary: PathBuf,
}

pub(super) fn run_ngspice_sensitivity<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceSensitivityRunOptions<'_, F, C>,
) -> Result<NgspiceSensitivityRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceSensitivityRunOptions {
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
        .expect("analog was validated before sensitivity run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog sensitivity run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_sens.cir");
    let log = run_dir.join("ngspice_sens.log");
    let raw = run_dir.join("sensitivity_raw.txt");
    let summary = run_dir.join("sensitivity_summary.csv");

    on_progress(
        "Writing analog sensitivity wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_sensitivity_wrapper(
        bound,
        scenario,
        source_netlist,
        parameter_overrides,
        model_section_overrides,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice sensitivity wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog sensitivity backend",
        format!(
            "{} .SENS {}.",
            backend,
            analog
                .analysis
                .sensitivity_output_expression
                .as_deref()
                .unwrap_or("<missing>")
        ),
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
    fs::write(&log, &log_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice sensitivity log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice sensitivity analysis exited with status {}.",
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
    fs::write(&raw, &log_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice sensitivity raw output {}: {error}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(raw.clone());
    let summary_csv = sensitivity_raw_to_csv(&log_text, scenario)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&summary, summary_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write sensitivity summary CSV {}: {error}",
                summary.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(summary.clone());
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "sensitivity",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("ngspice_sensitivity_stdout", &raw)],
        normalized_outputs: &[("sensitivity_summary", &summary)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceSensitivityRun { artifacts, summary })
}

pub(super) fn run_xyce_sensitivity<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: XyceSensitivityRunOptions<'_, F, C>,
) -> Result<NgspiceSensitivityRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let XyceSensitivityRunOptions {
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
        .expect("analog was validated before Xyce sensitivity run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog Xyce sensitivity run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_xyce_sens.cir");
    let log = run_dir.join("xyce_sens.log");
    let raw = run_dir.join("sensitivity_raw.csv");
    let summary = run_dir.join("sensitivity_summary.csv");
    let parameter_map = xyce_sensitivity_parameter_map(bound, scenario)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;

    on_progress(
        "Writing analog Xyce sensitivity wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_xyce_sensitivity_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("sensitivity_raw.csv"),
        &parameter_map,
        parameter_overrides,
        model_section_overrides,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce sensitivity wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog Xyce sensitivity backend",
        format!(
            "{} .SENS {}.",
            backend,
            analog
                .analysis
                .sensitivity_output_expression
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
                "Failed to write Xyce sensitivity log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "Xyce sensitivity analysis exited with status {}.",
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
                "Xyce completed without producing sensitivity output {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    let raw_text = fs::read_to_string(&raw).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to read Xyce sensitivity output {}: {error}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    let summary_csv = xyce_sensitivity_csv_to_summary(&raw_text, scenario, &parameter_map)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&summary, summary_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write sensitivity summary CSV {}: {error}",
                summary.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(summary.clone());
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "sensitivity",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("xyce_sensitivity_csv", &raw)],
        normalized_outputs: &[("sensitivity_summary", &summary)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceSensitivityRun { artifacts, summary })
}

fn build_ngspice_sensitivity_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before sensitivity wrapper generation");
    let output_expression = analog
        .analysis
        .sensitivity_output_expression
        .as_deref()
        .ok_or_else(|| {
            "sensitivity_output_expression is required for .SENS analysis.".to_string()
        })?;
    let mode = analog
        .analysis
        .sensitivity_mode
        .as_deref()
        .ok_or_else(|| "sensitivity_mode is required for .SENS analysis.".to_string())?;
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for ngspice .SENS. Do not edit by hand.\n");
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
            ".end"
                | ".tran"
                | ".ac"
                | ".op"
                | ".noise"
                | ".lin"
                | ".tf"
                | ".pz"
                | ".sens"
                | ".print"
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
    text.push_str(".control\n");
    push_ngspice_osdi_load_commands(&mut text, bound, scenario)?;
    text.push_str("sens ");
    text.push_str(output_expression);
    for filter in &analog.analysis.sensitivity_filters {
        text.push(' ');
        text.push_str(filter);
    }
    if mode == "ac" {
        let sweep_type = normalized_frequency_sweep_type(analog.analysis.sweep_type.as_deref())?;
        text.push_str(" ac ");
        text.push_str(sweep_type);
        text.push(' ');
        text.push_str(
            &analog
                .analysis
                .points_per_decade
                .expect("AC points_per_decade was validated")
                .to_string(),
        );
        text.push(' ');
        text.push_str(&format!(
            "{:.12e}",
            analog
                .analysis
                .start_frequency_hz
                .expect("AC start frequency was validated")
        ));
        text.push(' ');
        text.push_str(&format!(
            "{:.12e}",
            analog
                .analysis
                .stop_frequency_hz
                .expect("AC stop frequency was validated")
        ));
    }
    text.push_str("\nsetplot sens1\ndisplay\nprint all\nquit\n.endc\n.end\n");
    Ok(text)
}

fn build_xyce_sensitivity_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    output_file: &Path,
    parameter_map: &[XyceSensitivityParameter],
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before Xyce sensitivity wrapper generation");
    let output_expression = analog
        .analysis
        .sensitivity_output_expression
        .as_deref()
        .ok_or_else(|| {
            "sensitivity_output_expression is required for .SENS analysis.".to_string()
        })?;
    let mode = analog
        .analysis
        .sensitivity_mode
        .as_deref()
        .ok_or_else(|| "sensitivity_mode is required for .SENS analysis.".to_string())?;
    if parameter_map.is_empty() {
        return Err(
            "Xyce .SENS requires sensitivity_filters[] to name explicit sensitivity parameters."
                .to_string(),
        );
    }
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for Xyce .SENS. Do not edit by hand.\n");
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
            ".end"
                | ".tran"
                | ".ac"
                | ".op"
                | ".noise"
                | ".lin"
                | ".tf"
                | ".pz"
                | ".sens"
                | ".print"
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
    if mode == "ac" {
        let sweep_type =
            normalized_frequency_sweep_type_upper(analog.analysis.sweep_type.as_deref())?;
        text.push_str(".AC ");
        text.push_str(sweep_type);
        text.push(' ');
        text.push_str(
            &analog
                .analysis
                .points_per_decade
                .expect("AC points_per_decade was validated")
                .to_string(),
        );
        text.push(' ');
        text.push_str(&format!(
            "{:.12e}",
            analog
                .analysis
                .start_frequency_hz
                .expect("AC start frequency was validated")
        ));
        text.push(' ');
        text.push_str(&format!(
            "{:.12e}",
            analog
                .analysis
                .stop_frequency_hz
                .expect("AC stop frequency was validated")
        ));
        text.push_str("\n.SENS acobjfunc={");
    } else {
        text.push_str(".OP\n.SENS objfunc={");
    }
    text.push_str(output_expression);
    text.push_str("} param=");
    for (index, parameter) in parameter_map.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        text.push_str(&parameter.xyce_parameter);
    }
    text.push_str("\n.PRINT SENS FORMAT=CSV ");
    text.push_str(&output_file.to_string_lossy());
    text.push_str("\n.end\n");
    Ok(text)
}

fn sensitivity_raw_to_csv(log: &str, scenario: &Scenario) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before sensitivity parsing");
    let output_expression = analog
        .analysis
        .sensitivity_output_expression
        .as_deref()
        .unwrap_or("");
    let mode = analog.analysis.sensitivity_mode.as_deref().unwrap_or("");
    let rows = if mode == "ac" {
        parse_ac_sensitivity_rows(log)
    } else {
        parse_dc_sensitivity_rows(log, &analog.analysis.sensitivity_filters)
    };
    if rows.is_empty() {
        return Err(format!(
            ".SENS output did not contain normalized {mode} sensitivity rows."
        ));
    }
    Ok(sensitivity_rows_to_csv(output_expression, mode, rows))
}

fn xyce_sensitivity_csv_to_summary(
    csv_text: &str,
    scenario: &Scenario,
    parameter_map: &[XyceSensitivityParameter],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before sensitivity parsing");
    let output_expression = analog
        .analysis
        .sensitivity_output_expression
        .as_deref()
        .unwrap_or("");
    let mode = analog.analysis.sensitivity_mode.as_deref().unwrap_or("");
    let mut lines = csv_text.lines().filter(|line| !line.trim().is_empty());
    let Some(header_line) = lines.next() else {
        return Err("Xyce .SENS CSV output was empty.".to_string());
    };
    let header = split_csv_line(header_line)
        .ok_or_else(|| "Xyce .SENS CSV header is malformed.".to_string())?;
    let frequency_col = header
        .iter()
        .position(|field| field.trim().eq_ignore_ascii_case("FREQ"));
    let mut value_columns = Vec::new();
    for (index, field) in header.iter().enumerate() {
        if is_xyce_non_sensitivity_column(field, output_expression) {
            continue;
        }
        let Some(xyce_parameter) = xyce_parameter_from_sensitivity_header(field) else {
            continue;
        };
        let parameter = parameter_map
            .iter()
            .find(|candidate| {
                candidate
                    .xyce_parameter
                    .eq_ignore_ascii_case(&xyce_parameter)
            })
            .map(|candidate| candidate.declared_parameter.clone())
            .unwrap_or(xyce_parameter);
        value_columns.push((index, parameter));
    }
    if value_columns.is_empty() {
        return Err("Xyce .SENS CSV output did not contain sensitivity columns.".to_string());
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_line(line) else {
            continue;
        };
        let frequency_hz = frequency_col
            .and_then(|index| fields.get(index))
            .and_then(|value| parse_number(value));
        for (index, parameter) in &value_columns {
            let Some(value) = fields.get(*index).and_then(|value| parse_number(value)) else {
                continue;
            };
            rows.push(SensitivityRow {
                parameter: parameter.clone(),
                frequency_hz,
                real: value,
                imaginary: 0.0,
            });
        }
    }
    if rows.is_empty() {
        return Err(
            "Xyce .SENS CSV output did not contain normalized sensitivity rows.".to_string(),
        );
    }
    Ok(sensitivity_rows_to_csv(output_expression, mode, rows))
}

fn sensitivity_rows_to_csv(
    output_expression: &str,
    mode: &str,
    rows: Vec<SensitivityRow>,
) -> String {
    let mut csv = String::from(
        "output_expression,mode,parameter,frequency_hz,sensitivity_real,sensitivity_imaginary,sensitivity_magnitude\n",
    );
    for row in rows {
        csv.push_str(&csv_escape(output_expression));
        csv.push(',');
        csv.push_str(&csv_escape(mode));
        csv.push(',');
        csv.push_str(&csv_escape(&row.parameter));
        csv.push(',');
        if let Some(frequency_hz) = row.frequency_hz {
            csv.push_str(&format!("{frequency_hz:.12e}"));
        }
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.real));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.imaginary));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.magnitude()));
        csv.push('\n');
    }
    csv
}

#[derive(Debug)]
struct SensitivityRow {
    parameter: String,
    frequency_hz: Option<f64>,
    real: f64,
    imaginary: f64,
}

impl SensitivityRow {
    fn magnitude(&self) -> f64 {
        (self
            .real
            .mul_add(self.real, self.imaginary * self.imaginary))
        .sqrt()
    }
}

fn parse_dc_sensitivity_rows(log: &str, filters: &[String]) -> Vec<SensitivityRow> {
    let mut rows = Vec::new();
    for line in log.lines() {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let parameter = name.trim();
        if parameter.is_empty()
            || parameter.contains(char::is_whitespace)
            || parameter.eq_ignore_ascii_case("command")
        {
            continue;
        }
        if !filters.is_empty()
            && !filters
                .iter()
                .any(|filter| filter.eq_ignore_ascii_case(parameter))
        {
            continue;
        }
        let Some(real) = parse_number(value.split_whitespace().next().unwrap_or("")) else {
            continue;
        };
        rows.push(SensitivityRow {
            parameter: parameter.to_string(),
            frequency_hz: None,
            real,
            imaginary: 0.0,
        });
    }
    rows
}

fn parse_ac_sensitivity_rows(log: &str) -> Vec<SensitivityRow> {
    let mut rows = Vec::new();
    let mut current_parameter: Option<String> = None;
    for line in log.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() >= 3
            && fields[0].eq_ignore_ascii_case("index")
            && fields[1].eq_ignore_ascii_case("frequency")
        {
            current_parameter = Some(fields[2].to_string());
            continue;
        }
        let Some(parameter) = current_parameter.as_ref() else {
            continue;
        };
        if fields.len() < 4 || fields[0].parse::<usize>().is_err() {
            continue;
        }
        let Some(frequency_hz) = parse_number(fields[1]) else {
            continue;
        };
        let Some(real) = parse_number(fields[2]) else {
            continue;
        };
        let Some(imaginary) = parse_number(fields[3]) else {
            continue;
        };
        rows.push(SensitivityRow {
            parameter: parameter.clone(),
            frequency_hz: Some(frequency_hz),
            real,
            imaginary,
        });
    }
    rows
}

fn parse_number(value: &str) -> Option<f64> {
    value
        .trim_matches(|ch: char| ch == ',' || ch == ';')
        .parse()
        .ok()
}

#[derive(Debug, Clone)]
struct XyceSensitivityParameter {
    declared_parameter: String,
    xyce_parameter: String,
}

fn xyce_sensitivity_parameter_map(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
) -> Result<Vec<XyceSensitivityParameter>, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before Xyce sensitivity parameter mapping");
    if analog.analysis.sensitivity_filters.is_empty() {
        return Err(
            "Xyce .SENS requires sensitivity_filters[] to name explicit sensitivity parameters."
                .to_string(),
        );
    }
    let mut parameters = Vec::new();
    for filter in &analog.analysis.sensitivity_filters {
        let declared = filter.trim();
        if declared.is_empty() {
            continue;
        }
        let xyce_parameter = if declared.contains(':') {
            declared.to_string()
        } else if let Some(component) = bound.project.board.components.get(declared) {
            match component.spice.as_ref().map(|spec| &spec.primitive) {
                Some(SpicePrimitive::Resistor) => format!("{declared}:R"),
                Some(SpicePrimitive::Capacitor) => format!("{declared}:C"),
                Some(SpicePrimitive::Inductor) => format!("{declared}:L"),
                _ => declared.to_string(),
            }
        } else {
            declared.to_string()
        };
        parameters.push(XyceSensitivityParameter {
            declared_parameter: declared.to_string(),
            xyce_parameter,
        });
    }
    if parameters.is_empty() {
        return Err(
            "Xyce .SENS requires sensitivity_filters[] to name explicit sensitivity parameters."
                .to_string(),
        );
    }
    Ok(parameters)
}

fn is_xyce_non_sensitivity_column(field: &str, output_expression: &str) -> bool {
    let trimmed = field.trim().trim_matches('"');
    trimmed.eq_ignore_ascii_case("INDEX")
        || trimmed.eq_ignore_ascii_case("TIME")
        || trimmed.eq_ignore_ascii_case("FREQ")
        || trimmed.eq_ignore_ascii_case(output_expression)
        || trimmed.eq_ignore_ascii_case(&format!("{{{output_expression}}}"))
}

fn xyce_parameter_from_sensitivity_header(field: &str) -> Option<String> {
    let mut value = field.trim().trim_matches('"').trim();
    if let Some((_, tail)) = value.rsplit_once("/d_") {
        value = tail;
    } else if let Some(tail) = value.strip_prefix("d_") {
        value = tail;
    } else if let Some(tail) = value.strip_prefix("D_") {
        value = tail;
    } else {
        return None;
    }
    value = value
        .strip_suffix("_dir")
        .or_else(|| value.strip_suffix("_DIR"))
        .or_else(|| value.strip_suffix("_adj"))
        .or_else(|| value.strip_suffix("_ADJ"))
        .unwrap_or(value);
    let value = value.trim().trim_matches('{').trim_matches('}').trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn split_csv_line(line: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(field.trim().to_string());
                field.clear();
            }
            _ => field.push(ch),
        }
    }
    if in_quotes {
        return None;
    }
    fields.push(field.trim().to_string());
    Some(fields)
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::sensitivity_raw_to_csv;
    use crate::board_ir::Scenario;

    fn scenario(mode: &str) -> Scenario {
        serde_yaml_ng::from_str(&format!(
            r#"
name: sens
type: analog_sensitivity
analog:
  backend: ngspice
  netlist_source: generated_from_board
  model_files: []
  node_bindings: []
  pin_bindings: []
  analysis:
    type: sens
    sensitivity_output_expression: V(out)
    sensitivity_mode: {mode}
    sensitivity_filters: [R1, R2]
  stimuli: []
  probes: []
  assertions: []
"#
        ))
        .unwrap()
    }

    #[test]
    fn sensitivity_raw_to_csv_extracts_dc_assignments() {
        let csv = sensitivity_raw_to_csv("r1 = -2.50000e-04\nr2 = 2.499998e-04\n", &scenario("dc"))
            .unwrap();

        assert!(csv.contains("V(out),dc,r1,,-2.500000000000e-4,0.000000000000e0"));
        assert!(csv.contains("V(out),dc,r2,,2.499998000000e-4,0.000000000000e0"));
    }

    #[test]
    fn sensitivity_raw_to_csv_extracts_ac_tables() {
        let csv = sensitivity_raw_to_csv(
            "Index   frequency       r1\n0 1.000000e+01 -2.50000e-04, 1.000000e-06\nIndex   frequency       r2\n0 1.000000e+01 2.499998e-04, -0.000000e+00\n",
            &scenario("ac"),
        )
        .unwrap();

        assert!(csv.contains("V(out),ac,r1,1.000000000000e1,-2.500000000000e-4,1.000000000000e-6"));
        assert!(csv.contains("V(out),ac,r2,1.000000000000e1,2.499998000000e-4,-0.000000000000e0"));
    }
}
