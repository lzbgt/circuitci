use crate::board_ir::{Scenario, SpicePrimitive};
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

pub(super) struct NgspicePoleZeroRunOptions<'a, F, C>
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

pub(super) struct NgspicePoleZeroRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) summary: PathBuf,
}

pub(super) fn run_ngspice_pole_zero<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspicePoleZeroRunOptions<'_, F, C>,
) -> Result<NgspicePoleZeroRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspicePoleZeroRunOptions {
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
        .expect("analog was validated before pole-zero run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog pole-zero run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_pz.cir");
    let log = run_dir.join("ngspice_pz.log");
    let raw = run_dir.join("pole_zero_raw.txt");
    let summary = run_dir.join("pole_zero_summary.csv");

    on_progress(
        "Writing analog pole-zero wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_pole_zero_wrapper(
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
                "Failed to write ngspice pole-zero wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog pole-zero backend",
        format!(
            "{} .PZ {} / {}.",
            backend,
            analog
                .analysis
                .pole_zero_output_node
                .as_deref()
                .unwrap_or("<missing>"),
            analog
                .analysis
                .pole_zero_input_source
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
                "Failed to write ngspice pole-zero log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice pole-zero analysis exited with status {}.",
                output.status
            ),
            artifacts,
        ));
    }
    fs::write(&raw, &log_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice pole-zero raw output {}: {error}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(raw.clone());
    let summary_csv = pole_zero_raw_to_csv(&log_text, scenario)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    if let Some(reason) = detect_nonconvergence(&log_text)
        && log_text
            .to_ascii_lowercase()
            .contains("pz simulation(s) aborted")
    {
        return Err(ngspice_error(
            format!("ngspice reported non-convergence or numerical failure: {reason}."),
            artifacts,
        ));
    }
    fs::write(&summary, summary_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write pole-zero summary CSV {}: {error}",
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
        analysis_kind: "pole_zero",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("ngspice_pole_zero_stdout", &raw)],
        normalized_outputs: &[("pole_zero_summary", &summary)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspicePoleZeroRun { artifacts, summary })
}

fn build_ngspice_pole_zero_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before pole-zero wrapper generation");
    let output_node = analog
        .analysis
        .pole_zero_output_node
        .as_deref()
        .ok_or_else(|| "pole_zero_output_node is required for .PZ analysis.".to_string())?;
    let reference_node = analog
        .analysis
        .pole_zero_reference_node
        .as_deref()
        .ok_or_else(|| "pole_zero_reference_node is required for .PZ analysis.".to_string())?;
    let input_source = analog
        .analysis
        .pole_zero_input_source
        .as_deref()
        .ok_or_else(|| "pole_zero_input_source is required for .PZ analysis.".to_string())?;
    let mode = analog
        .analysis
        .pole_zero_mode
        .as_deref()
        .ok_or_else(|| "pole_zero_mode is required for .PZ analysis.".to_string())?;
    let (input_positive_node, input_negative_node) = input_source_nodes(scenario, input_source)?;
    let transfer_kind = input_source_transfer_kind(bound, input_source)?;
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for ngspice .PZ. Do not edit by hand.\n");
    text.push_str("* Source netlist: ");
    text.push_str(&netlist.to_string_lossy());
    text.push('\n');
    let include_base = netlist.parent().unwrap_or(&bound.project.source_dir);
    let mut in_control_block = false;
    let source_tokens = source_element_tokens(input_source);
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
            ".end" | ".tran" | ".ac" | ".op" | ".noise" | ".lin" | ".tf" | ".pz" | ".print"
        ) {
            continue;
        }
        if source_tokens.iter().any(|token| token == first_token) {
            text.push_str("* CircuitCI omitted PZ input source element: ");
            text.push_str(line);
            text.push('\n');
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
    text.push_str("pz ");
    text.push_str(&input_positive_node);
    text.push(' ');
    text.push_str(&input_negative_node);
    text.push(' ');
    text.push_str(output_node);
    text.push(' ');
    text.push_str(reference_node);
    text.push(' ');
    text.push_str(transfer_kind);
    text.push(' ');
    text.push_str(ngspice_mode_token(mode));
    text.push_str("\nsetplot pz1\ndisplay\nprint all\nquit\n.endc\n.end\n");
    Ok(text)
}

fn input_source_nodes(scenario: &Scenario, input_source: &str) -> Result<(String, String), String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before pole-zero node resolution");
    let node_for_pin = |pin: &str| {
        analog
            .pin_bindings
            .iter()
            .find(|binding| {
                binding.endpoint.component == input_source && binding.endpoint.pin == pin
            })
            .map(|binding| binding.node.clone())
    };
    let positive = node_for_pin("P").ok_or_else(|| {
        format!("analog_pole_zero input source {input_source} is missing bound P pin.")
    })?;
    let negative = node_for_pin("N").ok_or_else(|| {
        format!("analog_pole_zero input source {input_source} is missing bound N pin.")
    })?;
    Ok((positive, negative))
}

fn input_source_transfer_kind(
    bound: &BoundBoard<'_>,
    input_source: &str,
) -> Result<&'static str, String> {
    let component = bound
        .project
        .board
        .components
        .get(input_source)
        .ok_or_else(|| {
            format!("analog_pole_zero input source {input_source} is not on the board.")
        })?;
    let primitive = component
        .spice
        .as_ref()
        .map(|spice| &spice.primitive)
        .ok_or_else(|| {
            format!(
                "analog_pole_zero input source {input_source} requires a SPICE source primitive."
            )
        })?;
    match primitive {
        SpicePrimitive::DcVoltageSource | SpicePrimitive::PulseVoltageSource => Ok("vol"),
        SpicePrimitive::DcCurrentSource | SpicePrimitive::PulseCurrentSource => Ok("cur"),
        SpicePrimitive::Resistor | SpicePrimitive::Capacitor | SpicePrimitive::Inductor => {
            Err(format!(
                "analog_pole_zero input source {input_source} must be a voltage or current source."
            ))
        }
    }
}

fn source_element_tokens(input_source: &str) -> Vec<String> {
    let lower = input_source.to_ascii_lowercase();
    let mut tokens = vec![lower.clone()];
    for prefix in ["v", "i"] {
        if !lower.starts_with(prefix) {
            tokens.push(format!("{prefix}{lower}"));
        }
    }
    tokens
}

fn ngspice_mode_token(mode: &str) -> &'static str {
    match mode {
        "poles" => "pol",
        "zeros" => "zer",
        "poles_and_zeros" => "pz",
        _ => "pz",
    }
}

fn pole_zero_raw_to_csv(log: &str, scenario: &Scenario) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before pole-zero parsing");
    let output_node = analog
        .analysis
        .pole_zero_output_node
        .as_deref()
        .unwrap_or("");
    let reference_node = analog
        .analysis
        .pole_zero_reference_node
        .as_deref()
        .unwrap_or("");
    let input_source = analog
        .analysis
        .pole_zero_input_source
        .as_deref()
        .unwrap_or("");
    let mode = analog.analysis.pole_zero_mode.as_deref().unwrap_or("");
    let mut roots = Vec::new();
    let mut pending: Option<(String, usize)> = None;
    for line in log.lines() {
        if let Some(root) = parse_root_label(line) {
            pending = Some(root);
        }
        if let Some((real, imaginary)) = parse_complex_value(line)
            && let Some((kind, index)) = pending.take()
        {
            roots.push((kind, index, real, imaginary));
        }
    }
    if roots.is_empty() {
        return Err("ngspice .PZ output did not contain pole or zero values.".to_string());
    }
    let mut text = String::from(
        "output_node,reference_node,input_source,mode,root_kind,root_index,real_rad_per_s,imaginary_rad_per_s,frequency_hz\n",
    );
    for (kind, index, real, imaginary) in roots {
        let frequency_hz = real.hypot(imaginary) / std::f64::consts::TAU;
        text.push_str(&format!(
            "{},{},{},{},{},{},{:.12e},{:.12e},{:.12e}\n",
            csv_escape(output_node),
            csv_escape(reference_node),
            csv_escape(input_source),
            csv_escape(mode),
            kind,
            index,
            real,
            imaginary,
            frequency_hz
        ));
    }
    Ok(text)
}

fn parse_root_label(line: &str) -> Option<(String, usize)> {
    let lower = line.to_ascii_lowercase();
    let kind = if lower.contains("pole(") {
        "pole"
    } else if lower.contains("zero(") {
        "zero"
    } else {
        return None;
    };
    let start = lower.find('(')? + 1;
    let end = lower[start..].find(')')? + start;
    let index = lower[start..end].trim().parse().ok()?;
    Some((kind.to_string(), index))
}

fn parse_complex_value(line: &str) -> Option<(f64, f64)> {
    let (_, value) = line.split_once('=')?;
    let mut fields = value.trim().split(',');
    let real = fields.next()?.trim().parse().ok()?;
    let imaginary = fields.next()?.trim().parse().ok()?;
    Some((real, imaginary))
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
    use super::pole_zero_raw_to_csv;
    use crate::board_ir::Scenario;

    #[test]
    fn pole_zero_raw_to_csv_extracts_ngspice_roots() {
        let scenario: Scenario = serde_yaml_ng::from_str(
            r#"
name: pz
type: analog_pole_zero
analog:
  backend: ngspice
  netlist_source: generated_from_board
  model_files: []
  node_bindings: []
  pin_bindings: []
  analysis:
    type: pz
    pole_zero_output_node: out
    pole_zero_reference_node: "0"
    pole_zero_input_source: V1
    pole_zero_mode: poles_and_zeros
  stimuli: []
  probes: []
  assertions: []
"#,
        )
        .unwrap();
        let csv = pole_zero_raw_to_csv(
            "    pole(1)             : voltage, complex, 1 long [default scale]\nall = -1.00000e+03,0.000000e+00\n    zero(1)             : voltage, complex, 1 long\nall = -2.00000e+03,5.000000e+02\n",
            &scenario,
        )
        .unwrap();

        assert!(csv.contains("out,0,V1,poles_and_zeros,pole,1,-1.000000000000e3"));
        assert!(csv.contains("out,0,V1,poles_and_zeros,zero,1,-2.000000000000e3,5.000000000000e2"));
    }
}
