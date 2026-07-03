use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::analog_runner::{
    ModelSectionOverride, NgspiceRunError, ParameterOverride, SolverManifestIo,
    detect_nonconvergence, ngspice_error, rewrite_include_line, run_solver_with_timeout,
    sweep_temperature_c, write_solver_manifest,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};

pub(super) struct NgspiceDcSweepRunOptions<'a, F, C>
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

pub(super) struct NgspiceDcSweepRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) sweep: PathBuf,
}

pub(super) fn run_ngspice_dc_sweep<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceDcSweepRunOptions<'_, F, C>,
) -> Result<NgspiceDcSweepRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceDcSweepRunOptions {
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
        .expect("analog was validated before DC sweep run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog DC sweep run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_dc_sweep.cir");
    let log = run_dir.join("ngspice_dc_sweep.log");
    let raw = run_dir.join("dc_sweep_raw.csv");
    let sweep = run_dir.join("dc_sweep.csv");

    on_progress(
        "Writing analog DC sweep wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_dc_sweep_wrapper(
        bound,
        scenario,
        source_netlist,
        &raw,
        parameter_overrides,
        model_section_overrides,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write analog DC sweep wrapper {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running ngspice DC sweep",
        format!("Backend {backend} will write {}.", raw.to_string_lossy()),
    );
    let output_result = run_solver_with_timeout(
        backend,
        &wrapper,
        Duration::from_secs(30),
        None,
        should_cancel,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    let mut log_text = String::new();
    log_text.push_str(&String::from_utf8_lossy(&output_result.stdout));
    if !output_result.stderr.is_empty() {
        log_text.push_str("\n[stderr]\n");
        log_text.push_str(&String::from_utf8_lossy(&output_result.stderr));
    }
    fs::write(&log, &log_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice DC sweep log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());

    if !output_result.status.success() {
        let reason = detect_nonconvergence(&log_text).unwrap_or("solver process failed");
        return Err(ngspice_error(
            format!(
                "ngspice DC sweep did not complete successfully ({reason}); see {}.",
                log.display()
            ),
            artifacts,
        ));
    }
    if !raw.is_file() {
        return Err(ngspice_error(
            format!(
                "ngspice DC sweep completed but did not write raw sweep export {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    let sweep_csv = dc_sweep_raw_to_csv(&raw, analog).map_err(|message| {
        ngspice_error(
            format!(
                "Failed to normalize ngspice DC sweep export {}: {message}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&sweep, sweep_csv).map_err(|error| {
        ngspice_error(
            format!("Failed to write DC sweep CSV {}: {error}", sweep.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(sweep.clone());
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "dc_sweep",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output_result,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("ngspice_dc_sweep_raw", &raw)],
        normalized_outputs: &[("dc_sweep", &sweep)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceDcSweepRun { artifacts, sweep })
}

fn build_ngspice_dc_sweep_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    source_netlist: &Path,
    raw_output: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before DC sweep wrapper generation");
    let source_text = fs::read_to_string(source_netlist).map_err(|error| {
        format!(
            "Failed to read source netlist {} for DC sweep wrapper: {error}",
            source_netlist.display()
        )
    })?;
    let source_dir = source_netlist.parent().ok_or_else(|| {
        format!(
            "Source netlist {} has no parent directory.",
            source_netlist.display()
        )
    })?;
    let mut text = String::from("* CircuitCI ngspice DC sweep wrapper\n");
    for line in source_text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower == ".end"
            || lower == ".op"
            || lower.starts_with(".tran ")
            || lower.starts_with(".ac ")
            || lower.starts_with(".dc ")
            || lower.starts_with(".noise ")
        {
            continue;
        }
        text.push_str(&rewrite_include_line(line, source_dir));
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
            text.push_str(&format!("{temperature_c:.12e}"));
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
    let analysis = &analog.analysis;
    let source = analysis
        .dc_sweep_source
        .as_deref()
        .ok_or_else(|| "dc_sweep_source is required before wrapper generation.".to_string())?;
    let start = analysis
        .dc_sweep_start
        .ok_or_else(|| "dc_sweep_start is required before wrapper generation.".to_string())?;
    let stop = analysis
        .dc_sweep_stop
        .ok_or_else(|| "dc_sweep_stop is required before wrapper generation.".to_string())?;
    let step = analysis
        .dc_sweep_step
        .ok_or_else(|| "dc_sweep_step is required before wrapper generation.".to_string())?;
    text.push_str(".control\n");
    text.push_str("set wr_vecnames\n");
    text.push_str("set wr_singlescale\n");
    text.push_str(&format!(
        "dc {source} {start:.12e} {stop:.12e} {step:.12e}\n"
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

pub(super) fn dc_sweep_raw_to_csv(
    raw: &Path,
    analog: &crate::board_ir::AnalogScenario,
) -> Result<String, String> {
    let text = fs::read_to_string(raw).map_err(|error| {
        format!(
            "Failed to read DC sweep raw export {}: {error}",
            raw.display()
        )
    })?;
    let source = analog
        .analysis
        .dc_sweep_source
        .as_deref()
        .unwrap_or("source");
    let mut csv = String::from("sweep_source,sweep_value,probe,value\n");
    let expected_columns = analog.probes.len() + 1;
    let mut row_count = 0usize;
    for (line_index, line) in text.lines().enumerate() {
        let fields: Vec<_> = line
            .split(|character: char| character == ',' || character.is_whitespace())
            .filter(|field| !field.is_empty())
            .collect();
        if fields.is_empty() {
            continue;
        }
        let Some(sweep_value) = parse_float(fields[0]) else {
            if row_count == 0 {
                continue;
            }
            return Err(format!(
                "DC sweep row {} has non-numeric sweep value {}.",
                line_index + 1,
                fields[0]
            ));
        };
        if fields.len() < expected_columns {
            return Err(format!(
                "DC sweep row {} has {} columns, expected at least {}.",
                line_index + 1,
                fields.len(),
                expected_columns
            ));
        }
        for (probe_index, probe) in analog.probes.iter().enumerate() {
            let field_index = probe_index + 1;
            let value = parse_float(fields[field_index]).ok_or_else(|| {
                format!(
                    "DC sweep row {} has non-numeric value {} for probe {}.",
                    line_index + 1,
                    fields[field_index],
                    probe.name
                )
            })?;
            csv.push_str(source);
            csv.push(',');
            csv.push_str(&format!("{sweep_value:.12e}"));
            csv.push(',');
            csv.push_str(&sanitize_csv_field(&probe.name));
            csv.push(',');
            csv.push_str(&format!("{value:.12e}"));
            csv.push('\n');
        }
        row_count += 1;
    }
    if row_count == 0 {
        return Err(format!(
            "DC sweep raw export {} has no numeric rows.",
            raw.display()
        ));
    }
    Ok(csv)
}

fn parse_float(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

fn sanitize_csv_field(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch == ',' || ch == '\n' || ch == '\r' {
                '_'
            } else {
                ch
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::dc_sweep_raw_to_csv;
    use crate::board_ir::BoardProject;
    use std::fs;

    #[test]
    fn dc_sweep_raw_export_normalizes_probe_rows() {
        let board: BoardProject = serde_yaml_ng::from_str(
            r#"
project: { name: dc_sweep_parse, version: 0.1.0 }
board: { components: {}, nets: {} }
scenarios:
  - name: sweep
    type: analog_dc_sweep
    checks: [SPICE_DC_SWEEP_ANALYSIS]
    analog:
      backend: ngspice
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis:
        type: dc_sweep
        dc_sweep_source: V1
        dc_sweep_start: 0.0
        dc_sweep_stop: 1.0
        dc_sweep_step: 0.5
      stimuli: []
      probes:
        - { name: out, expression: V(out) }
      assertions: []
"#,
        )
        .unwrap();
        let analog = board.scenarios[0].analog.as_ref().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let raw = temp.path().join("dc_sweep_raw.csv");
        fs::write(&raw, "Index out\n0.0 0.0\n0.5 0.25\n1.0 0.5\n").unwrap();

        let csv = dc_sweep_raw_to_csv(&raw, analog).unwrap();

        assert!(csv.contains("sweep_source,sweep_value,probe,value"));
        assert!(csv.contains("V1,5.000000000000e-1,out,2.500000000000e-1"));
    }
}
