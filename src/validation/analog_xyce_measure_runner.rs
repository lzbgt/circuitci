use crate::board_ir::{AnalogMeasureTemplate, Scenario};
use crate::library::BoundBoard;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::analog_measure_runner::{
    NgspiceMeasureRun, measure_delay_statement, measure_raw_to_csv,
    measure_threshold_time_statement,
};
use super::analog_runner::{
    ModelSectionOverride, NgspiceRunError, ParameterOverride, SolverManifestIo,
    detect_nonconvergence, ngspice_error, normalized_frequency_sweep_type_upper,
    rewrite_include_line, sweep_temperature_c, write_solver_manifest,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};
use super::analog_xyce_runner::run_xyce_with_timeout;

pub(super) struct XyceMeasureRunOptions<'a, F, C>
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

pub(super) fn run_xyce_measure<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: XyceMeasureRunOptions<'_, F, C>,
) -> Result<NgspiceMeasureRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let XyceMeasureRunOptions {
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
        .expect("analog was validated before Xyce measure run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog Xyce measure run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_xyce_measure.cir");
    let log = run_dir.join("xyce_measure.log");
    let raw = run_dir.join("measure_raw.txt");
    let summary = run_dir.join("measure_summary.csv");

    on_progress(
        "Writing analog Xyce measure wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_xyce_measure_wrapper(
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
                "Failed to write Xyce measure wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog Xyce measure backend",
        format!(
            "{} .MEASURE {} with {} template(s).",
            backend,
            analog
                .analysis
                .measure_mode
                .as_deref()
                .unwrap_or("<missing>"),
            analog.analysis.measure_templates.len()
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
                "Failed to write Xyce measure log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "Xyce measure analysis exited with status {}.",
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
    let (measure_output_text, measure_output_artifacts) =
        xyce_measure_output_text(&run_dir, &wrapper)
            .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    for artifact in measure_output_artifacts {
        artifacts.push(artifact);
    }
    let raw_text = if measure_output_text.trim().is_empty() {
        log_text
    } else {
        format!("{log_text}\n\nXYCE_MEASURE_OUTPUT:\n{measure_output_text}")
    };
    fs::write(&raw, &raw_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce measure raw output {}: {error}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(raw.clone());
    let summary_csv = measure_raw_to_csv(&raw_text, scenario)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&summary, summary_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce measure summary CSV {}: {error}",
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
        analysis_kind: "measure",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("xyce_measure_stdout", &raw)],
        normalized_outputs: &[("measure_summary", &summary)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceMeasureRun { artifacts, summary })
}

fn xyce_measure_output_text(
    run_dir: &Path,
    wrapper: &Path,
) -> Result<(String, Vec<PathBuf>), String> {
    let Some(stem) = wrapper.file_stem().and_then(|stem| stem.to_str()) else {
        return Ok((String::new(), Vec::new()));
    };
    let mut outputs = Vec::new();
    let entries = fs::read_dir(run_dir).map_err(|error| {
        format!(
            "Failed to scan Xyce measure output directory {}: {error}",
            run_dir.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Failed to read Xyce measure output directory {}: {error}",
                run_dir.display()
            )
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with(stem) {
            continue;
        }
        let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
            continue;
        };
        if is_xyce_measure_extension(extension) {
            outputs.push(path);
        }
    }
    outputs.sort();
    let mut text = String::new();
    for output in &outputs {
        text.push_str("* Xyce measure output: ");
        text.push_str(&output.to_string_lossy());
        text.push('\n');
        text.push_str(&fs::read_to_string(output).map_err(|error| {
            format!(
                "Failed to read Xyce measure output {}: {error}",
                output.display()
            )
        })?);
        text.push('\n');
    }
    Ok((text, outputs))
}

fn is_xyce_measure_extension(extension: &str) -> bool {
    let lower = extension.to_ascii_lowercase();
    let mut chars = lower.chars();
    matches!(chars.next(), Some('m')) && chars.any(|ch| ch.is_ascii_digit())
}

fn build_xyce_measure_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before Xyce measure wrapper generation");
    let mode = analog
        .analysis
        .measure_mode
        .as_deref()
        .ok_or_else(|| "measure_mode is required for Xyce measure analysis.".to_string())?;
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for Xyce .MEASURE. Do not edit by hand.\n");
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
                | ".four"
                | ".meas"
                | ".measure"
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
    if mode == "tran" {
        text.push_str(".TRAN ");
        text.push_str(&format!(
            "{:.12e}",
            analog.analysis.max_step_us / 1_000_000.0
        ));
        text.push(' ');
        text.push_str(&format!(
            "{:.12e}",
            analog.analysis.stop_time_us / 1_000_000.0
        ));
        text.push('\n');
    } else {
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
        text.push('\n');
    }
    for template in &analog.analysis.measure_templates {
        text.push_str(&xyce_measure_template_statement(mode, template));
        text.push('\n');
    }
    text.push_str(".END\n");
    Ok(text)
}

fn xyce_measure_template_statement(mode: &str, template: &AnalogMeasureTemplate) -> String {
    let xyce_mode = mode.to_ascii_uppercase();
    if matches!(template.operation.as_str(), "delay" | "slew") {
        return measure_delay_statement(".MEASURE", &xyce_mode, template);
    }
    if template.operation == "threshold_time" {
        return measure_threshold_time_statement(".MEASURE", &xyce_mode, template);
    }
    let mut statement = String::from(".MEASURE ");
    statement.push_str(&xyce_mode);
    statement.push(' ');
    statement.push_str(&template.name);
    statement.push(' ');
    statement.push_str(&template.operation.to_ascii_uppercase());
    statement.push(' ');
    statement.push_str(&template.expression);
    if mode == "tran" {
        append_measure_window_us(&mut statement, template);
    } else {
        append_measure_window_hz(&mut statement, template);
    }
    statement
}

fn append_measure_window_us(statement: &mut String, template: &AnalogMeasureTemplate) {
    if let Some(at_us) = template.at_us {
        statement.push_str(" AT=");
        statement.push_str(&format!("{:.12e}", at_us / 1_000_000.0));
    }
    if let Some(from_us) = template.from_us {
        statement.push_str(" FROM=");
        statement.push_str(&format!("{:.12e}", from_us / 1_000_000.0));
    }
    if let Some(to_us) = template.to_us {
        statement.push_str(" TO=");
        statement.push_str(&format!("{:.12e}", to_us / 1_000_000.0));
    }
}

fn append_measure_window_hz(statement: &mut String, template: &AnalogMeasureTemplate) {
    if let Some(at_hz) = template.at_hz {
        statement.push_str(" AT=");
        statement.push_str(&format!("{at_hz:.12e}"));
    }
    if let Some(from_hz) = template.from_hz {
        statement.push_str(" FROM=");
        statement.push_str(&format!("{from_hz:.12e}"));
    }
    if let Some(to_hz) = template.to_hz {
        statement.push_str(" TO=");
        statement.push_str(&format!("{to_hz:.12e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::xyce_measure_template_statement;
    use crate::board_ir::AnalogMeasureTemplate;

    #[test]
    fn xyce_measure_template_statement_formats_transient_window() {
        let template = AnalogMeasureTemplate {
            name: "avg_out".to_string(),
            operation: "avg".to_string(),
            expression: "v(out)".to_string(),
            trigger_expression: None,
            trigger_value: None,
            target_value: None,
            trigger_edge: None,
            target_edge: None,
            trigger_count: None,
            target_count: None,
            from_us: Some(20.0),
            to_us: Some(100.0),
            at_us: None,
            from_hz: None,
            to_hz: None,
            at_hz: None,
        };

        assert_eq!(
            xyce_measure_template_statement("tran", &template),
            ".MEASURE TRAN avg_out AVG v(out) FROM=2.000000000000e-5 TO=1.000000000000e-4"
        );
    }

    #[test]
    fn xyce_measure_template_statement_formats_delay() {
        let template = AnalogMeasureTemplate {
            name: "prop_delay".to_string(),
            operation: "delay".to_string(),
            expression: "v(out)".to_string(),
            trigger_expression: Some("v(vin)".to_string()),
            trigger_value: Some(0.5),
            target_value: Some(0.5),
            trigger_edge: Some("rise".to_string()),
            target_edge: Some("rise".to_string()),
            trigger_count: Some(1),
            target_count: Some(1),
            from_us: None,
            to_us: None,
            at_us: None,
            from_hz: None,
            to_hz: None,
            at_hz: None,
        };

        assert_eq!(
            xyce_measure_template_statement("tran", &template),
            ".MEASURE TRAN prop_delay TRIG v(vin) VAL=5.000000000000e-1 RISE=1 TARG v(out) VAL=5.000000000000e-1 RISE=1"
        );
    }

    #[test]
    fn xyce_measure_template_statement_formats_slew() {
        let template = AnalogMeasureTemplate {
            name: "out_slew".to_string(),
            operation: "slew".to_string(),
            expression: "v(out)".to_string(),
            trigger_expression: None,
            trigger_value: Some(0.2),
            target_value: Some(0.8),
            trigger_edge: Some("rise".to_string()),
            target_edge: Some("rise".to_string()),
            trigger_count: Some(1),
            target_count: Some(1),
            from_us: None,
            to_us: None,
            at_us: None,
            from_hz: None,
            to_hz: None,
            at_hz: None,
        };

        assert_eq!(
            xyce_measure_template_statement("tran", &template),
            ".MEASURE TRAN out_slew TRIG v(out) VAL=2.000000000000e-1 RISE=1 TARG v(out) VAL=8.000000000000e-1 RISE=1"
        );
    }

    #[test]
    fn xyce_measure_template_statement_formats_threshold_time() {
        let template = AnalogMeasureTemplate {
            name: "out_rise_time".to_string(),
            operation: "threshold_time".to_string(),
            expression: "v(out)".to_string(),
            trigger_expression: None,
            trigger_value: None,
            target_value: Some(0.5),
            trigger_edge: None,
            target_edge: Some("rise".to_string()),
            trigger_count: None,
            target_count: Some(1),
            from_us: Some(1.0),
            to_us: Some(20.0),
            at_us: None,
            from_hz: None,
            to_hz: None,
            at_hz: None,
        };

        assert_eq!(
            xyce_measure_template_statement("tran", &template),
            ".MEASURE TRAN out_rise_time WHEN v(out)=5.000000000000e-1 FROM=1.000000000000e-6 TO=2.000000000000e-5 RISE=1"
        );
    }
}
