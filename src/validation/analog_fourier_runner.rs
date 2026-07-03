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

pub(super) struct NgspiceFourierRunOptions<'a, F, C>
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

pub(super) struct NgspiceFourierRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) summary: PathBuf,
}

pub(super) fn run_ngspice_fourier<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceFourierRunOptions<'_, F, C>,
) -> Result<NgspiceFourierRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceFourierRunOptions {
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
        .expect("analog was validated before Fourier run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog Fourier run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_fourier.cir");
    let log = run_dir.join("ngspice_fourier.log");
    let raw = run_dir.join("fourier_raw.txt");
    let summary = run_dir.join("fourier_summary.csv");

    on_progress(
        "Writing analog Fourier wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_fourier_wrapper(
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
                "Failed to write ngspice Fourier wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog Fourier backend",
        format!(
            "{} .FOUR {} at {:.12e} Hz.",
            backend,
            analog
                .analysis
                .fourier_output_expression
                .as_deref()
                .unwrap_or("<missing>"),
            analog
                .analysis
                .fourier_fundamental_frequency_hz
                .unwrap_or_default()
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
                "Failed to write ngspice Fourier log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice Fourier analysis exited with status {}.",
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
                "Failed to write ngspice Fourier raw output {}: {error}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(raw.clone());
    let summary_csv = fourier_raw_to_csv(&log_text, scenario)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&summary, summary_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Fourier summary CSV {}: {error}",
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
        analysis_kind: "fourier",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("ngspice_fourier_stdout", &raw)],
        normalized_outputs: &[("fourier_summary", &summary)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceFourierRun { artifacts, summary })
}

fn build_ngspice_fourier_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before Fourier wrapper generation");
    let output_expression = analog
        .analysis
        .fourier_output_expression
        .as_deref()
        .ok_or_else(|| "fourier_output_expression is required for .FOUR analysis.".to_string())?;
    let fundamental_hz = analog
        .analysis
        .fourier_fundamental_frequency_hz
        .ok_or_else(|| {
            "fourier_fundamental_frequency_hz is required for .FOUR analysis.".to_string()
        })?;
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for ngspice .FOUR. Do not edit by hand.\n");
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
    let requested_harmonics = analog.analysis.fourier_harmonics.unwrap_or(10);
    let nfreqs = requested_harmonics.saturating_add(1);
    text.push_str(".control\n");
    push_ngspice_osdi_load_commands(&mut text, bound, scenario)?;
    text.push_str("set nfreqs=");
    text.push_str(&nfreqs.to_string());
    text.push_str("\ntran ");
    text.push_str(&format!(
        "{:.12e}",
        analog.analysis.max_step_us / 1_000_000.0
    ));
    text.push(' ');
    text.push_str(&format!(
        "{:.12e}",
        analog.analysis.stop_time_us / 1_000_000.0
    ));
    text.push_str("\nfourier ");
    text.push_str(&format!("{fundamental_hz:.12e}"));
    text.push(' ');
    text.push_str(output_expression);
    text.push_str("\nquit\n.endc\n.end\n");
    Ok(text)
}

fn fourier_raw_to_csv(log: &str, scenario: &Scenario) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before Fourier parsing");
    let output_expression = analog
        .analysis
        .fourier_output_expression
        .as_deref()
        .unwrap_or("");
    let fundamental_hz = analog
        .analysis
        .fourier_fundamental_frequency_hz
        .unwrap_or_default();
    let mut rows = Vec::new();
    let mut metadata = FourierMetadata::default();
    let mut in_target_block = false;
    let mut in_table = false;
    for line in log.lines() {
        let trimmed = line.trim();
        if let Some(expr) = trimmed
            .strip_prefix("Fourier analysis for ")
            .and_then(|value| value.strip_suffix(':'))
        {
            in_target_block = expr.trim().eq_ignore_ascii_case(output_expression.trim());
            in_table = false;
            continue;
        }
        if !in_target_block {
            continue;
        }
        if trimmed.starts_with("No. Harmonics:") {
            metadata = parse_metadata(trimmed);
            continue;
        }
        if trimmed.starts_with("Harmonic") && trimmed.contains("Frequency") {
            in_table = true;
            continue;
        }
        if !in_table || trimmed.is_empty() || trimmed.starts_with('-') {
            continue;
        }
        let fields: Vec<&str> = trimmed.split_whitespace().collect();
        if fields.len() < 6 {
            if !rows.is_empty() {
                in_table = false;
            }
            continue;
        }
        let Some(harmonic) = parse_u32(fields[0]) else {
            if !rows.is_empty() {
                in_table = false;
            }
            continue;
        };
        let Some(frequency_hz) = parse_number(fields[1]) else {
            continue;
        };
        let Some(magnitude) = parse_number(fields[2]) else {
            continue;
        };
        let Some(phase_deg) = parse_number(fields[3]) else {
            continue;
        };
        let Some(normalized_magnitude) = parse_number(fields[4]) else {
            continue;
        };
        let Some(normalized_phase_deg) = parse_number(fields[5]) else {
            continue;
        };
        rows.push(FourierRow {
            harmonic,
            frequency_hz,
            magnitude,
            phase_deg,
            normalized_magnitude,
            normalized_phase_deg,
        });
    }
    if rows.is_empty() {
        return Err(format!(
            "ngspice .FOUR output did not contain normalized harmonic rows for {output_expression}."
        ));
    }
    let mut csv = String::from(
        "output_expression,fundamental_frequency_hz,reported_harmonics,harmonic,frequency_hz,magnitude,phase_deg,normalized_magnitude,normalized_phase_deg,thd_percent,grid_size,interpolation_degree,periods\n",
    );
    for row in rows {
        csv.push_str(&csv_escape(output_expression));
        csv.push(',');
        csv.push_str(&format!("{fundamental_hz:.12e}"));
        csv.push(',');
        push_optional_u32(&mut csv, metadata.reported_harmonics);
        csv.push(',');
        csv.push_str(&row.harmonic.to_string());
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.frequency_hz));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.magnitude));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.phase_deg));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.normalized_magnitude));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.normalized_phase_deg));
        csv.push(',');
        push_optional_f64(&mut csv, metadata.thd_percent);
        csv.push(',');
        push_optional_u32(&mut csv, metadata.grid_size);
        csv.push(',');
        push_optional_u32(&mut csv, metadata.interpolation_degree);
        csv.push(',');
        push_optional_u32(&mut csv, metadata.periods);
        csv.push('\n');
    }
    Ok(csv)
}

#[derive(Default)]
struct FourierMetadata {
    reported_harmonics: Option<u32>,
    thd_percent: Option<f64>,
    grid_size: Option<u32>,
    interpolation_degree: Option<u32>,
    periods: Option<u32>,
}

struct FourierRow {
    harmonic: u32,
    frequency_hz: f64,
    magnitude: f64,
    phase_deg: f64,
    normalized_magnitude: f64,
    normalized_phase_deg: f64,
}

fn parse_metadata(line: &str) -> FourierMetadata {
    let mut metadata = FourierMetadata::default();
    for part in line.split(',') {
        let Some((key, value)) = part.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_end_matches('%').trim();
        match key {
            "No. Harmonics" => metadata.reported_harmonics = parse_u32(value),
            "THD" => metadata.thd_percent = parse_number(value),
            "Gridsize" => metadata.grid_size = parse_u32(value),
            "Interpolation Degree" => metadata.interpolation_degree = parse_u32(value),
            "No. Periods" => metadata.periods = parse_u32(value),
            _ => {}
        }
    }
    metadata
}

fn parse_number(value: &str) -> Option<f64> {
    value
        .trim_matches(|ch: char| ch == ',' || ch == ';' || ch == '%')
        .parse()
        .ok()
}

fn parse_u32(value: &str) -> Option<u32> {
    value
        .trim_matches(|ch: char| ch == ',' || ch == ';')
        .parse()
        .ok()
}

fn push_optional_f64(csv: &mut String, value: Option<f64>) {
    if let Some(value) = value {
        csv.push_str(&format!("{value:.12e}"));
    }
}

fn push_optional_u32(csv: &mut String, value: Option<u32>) {
    if let Some(value) = value {
        csv.push_str(&value.to_string());
    }
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
    use super::fourier_raw_to_csv;
    use crate::board_ir::Scenario;

    fn scenario() -> Scenario {
        serde_yaml_ng::from_str(
            r#"
name: fourier
type: analog_fourier
analog:
  backend: ngspice
  netlist_source: generated_from_board
  model_files: []
  node_bindings: []
  pin_bindings: []
  analysis:
    type: fourier
    stop_time_us: 100.0
    max_step_us: 0.1
    fourier_fundamental_frequency_hz: 100000.0
    fourier_output_expression: V(out)
    fourier_harmonics: 4
  stimuli: []
  probes: []
  assertions: []
"#,
        )
        .unwrap()
    }

    #[test]
    fn fourier_raw_to_csv_extracts_harmonic_table() {
        let csv = fourier_raw_to_csv(
            r#"
Fourier analysis for v(out):
  No. Harmonics: 5, THD: 18.5435 %, Gridsize: 200, Interpolation Degree: 1, No. Periods: 1

Harmonic Frequency   Magnitude   Phase       Norm. Mag   Norm. Phase
-------- ---------   ---------   -----       ---------   -----------
 0       0           0.509986    0           0           0
 1       100000      0.538779    -35.733     1           0
 2       200000      0.0124232   31.3212     0.0230581   67.0541
"#,
            &scenario(),
        )
        .unwrap();

        assert!(csv.contains("V(out),1.000000000000e5,5,0,0.000000000000e0,5.099860000000e-1"));
        assert!(csv.contains("V(out),1.000000000000e5,5,1,1.000000000000e5,5.387790000000e-1,-3.573300000000e1,1.000000000000e0"));
        assert!(csv.contains(",1.854350000000e1,200,1,1"));
    }
}
