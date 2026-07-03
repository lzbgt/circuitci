use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::analog_runner::{
    ModelSectionOverride, NgspiceRunError, ParameterOverride, SolverManifestIo,
    detect_nonconvergence, ngspice_error, push_ngspice_osdi_load_commands, rewrite_include_line,
    run_solver_with_timeout, sweep_temperature_c, write_solver_manifest,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};

pub(super) struct NgspiceTransferFunctionRunOptions<'a, F, C>
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

pub(super) struct NgspiceTransferFunctionRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) summary: PathBuf,
}

pub(super) fn run_ngspice_transfer_function<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceTransferFunctionRunOptions<'_, F, C>,
) -> Result<NgspiceTransferFunctionRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceTransferFunctionRunOptions {
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
        .expect("analog was validated before transfer-function run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog transfer-function run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_tf.cir");
    let log = run_dir.join("ngspice_tf.log");
    let raw = run_dir.join("transfer_function_raw.txt");
    let summary = run_dir.join("transfer_function_summary.csv");

    on_progress(
        "Writing analog transfer-function wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_transfer_function_wrapper(
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
                "Failed to write ngspice transfer-function wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog transfer-function backend",
        format!(
            "{} .TF {} / {}.",
            backend,
            analog
                .analysis
                .transfer_output_expression
                .as_deref()
                .unwrap_or("<missing>"),
            analog
                .analysis
                .transfer_input_source
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
                "Failed to write ngspice transfer-function log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice transfer-function analysis exited with status {}.",
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
                "Failed to write ngspice transfer-function raw output {}: {error}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(raw.clone());
    let summary_csv = transfer_function_raw_to_csv(&log_text, scenario)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&summary, summary_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write transfer-function summary CSV {}: {error}",
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
        analysis_kind: "transfer_function",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("ngspice_transfer_function_stdout", &raw)],
        normalized_outputs: &[("transfer_function_summary", &summary)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceTransferFunctionRun { artifacts, summary })
}

fn build_ngspice_transfer_function_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before transfer-function wrapper generation");
    let output_expression = analog
        .analysis
        .transfer_output_expression
        .as_deref()
        .ok_or_else(|| "transfer_output_expression is required for .TF analysis.".to_string())?;
    let input_source = analog
        .analysis
        .transfer_input_source
        .as_deref()
        .ok_or_else(|| "transfer_input_source is required for .TF analysis.".to_string())?;
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for ngspice .TF. Do not edit by hand.\n");
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
            ".end" | ".tran" | ".ac" | ".op" | ".noise" | ".lin" | ".tf" | ".print"
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
    push_ngspice_osdi_control_block(&mut text, bound, scenario)?;
    text.push_str(".tf ");
    text.push_str(output_expression);
    text.push(' ');
    text.push_str(input_source);
    text.push_str("\n.end\n");
    Ok(text)
}

fn push_ngspice_osdi_control_block(
    text: &mut String,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
) -> Result<(), String> {
    let before = text.len();
    push_ngspice_osdi_load_commands(text, bound, scenario)?;
    if text.len() == before {
        return Ok(());
    }
    let commands = text.split_off(before);
    text.push_str(".control\n");
    text.push_str(&commands);
    text.push_str(".endc\n");
    Ok(())
}

fn transfer_function_raw_to_csv(log: &str, scenario: &Scenario) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before transfer-function parsing");
    let output_expression = analog
        .analysis
        .transfer_output_expression
        .as_deref()
        .unwrap_or("");
    let input_source = analog
        .analysis
        .transfer_input_source
        .as_deref()
        .unwrap_or("");
    let mut transfer_gain = None;
    let mut output_resistance = None;
    let mut input_resistance = None;
    for line in log.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("transfer_function") {
            transfer_gain = parse_assignment_value(line).or(transfer_gain);
        } else if lower.contains("output_impedance") || lower.contains("output_resistance") {
            output_resistance = parse_assignment_value(line).or(output_resistance);
        } else if lower.contains("input_impedance") || lower.contains("input_resistance") {
            input_resistance = parse_assignment_value(line).or(input_resistance);
        }
    }
    let transfer_gain = transfer_gain.ok_or_else(|| {
        "ngspice .TF output did not contain a transfer_function value.".to_string()
    })?;
    let output_resistance = output_resistance.ok_or_else(|| {
        "ngspice .TF output did not contain output impedance/resistance.".to_string()
    })?;
    let input_resistance = input_resistance.ok_or_else(|| {
        "ngspice .TF output did not contain input impedance/resistance.".to_string()
    })?;
    Ok(format!(
        "output_expression,input_source,transfer_function_gain,input_resistance_ohm,output_resistance_ohm\n{},{},{:.12e},{:.12e},{:.12e}\n",
        csv_escape(output_expression),
        csv_escape(input_source),
        transfer_gain,
        input_resistance,
        output_resistance
    ))
}

fn parse_assignment_value(line: &str) -> Option<f64> {
    line.split('=')
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
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
    use super::transfer_function_raw_to_csv;
    use crate::board_ir::Scenario;

    #[test]
    fn transfer_function_raw_to_csv_extracts_ngspice_scalars() {
        let scenario: Scenario = serde_yaml_ng::from_str(
            r#"
name: tf
type: analog_transfer_function
analog:
  backend: ngspice
  netlist_source: generated_from_board
  model_files: []
  node_bindings: []
  pin_bindings: []
  analysis:
    type: tf
    transfer_output_expression: V(out)
    transfer_input_source: V1
  stimuli: []
  probes: []
  assertions: []
"#,
        )
        .unwrap();
        let csv = transfer_function_raw_to_csv(
            "transfer_function = 5.000000e-001\noutput_impedance_at_v(out) = 5.000000e+002\nv1#input_impedance = 2.000000e+003\n",
            &scenario,
        )
        .unwrap();

        assert!(csv.contains("V(out),V1,5.000000000000e-1"));
        assert!(csv.contains("2.000000000000e3,5.000000000000e2"));
    }
}
