use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::analog_runner::{
    ModelSectionOverride, NgspiceRunError, ParameterOverride, SolverManifestIo,
    detect_nonconvergence, ngspice_error, rewrite_include_line, run_solver_with_timeout,
    sweep_temperature_c, write_solver_manifest,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};

pub(super) struct NgspiceMeasureRunOptions<'a, F, C>
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

pub(super) struct NgspiceMeasureRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) summary: PathBuf,
}

pub(super) fn run_ngspice_measure<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceMeasureRunOptions<'_, F, C>,
) -> Result<NgspiceMeasureRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceMeasureRunOptions {
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
        .expect("analog was validated before measure run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog measure run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_measure.cir");
    let log = run_dir.join("ngspice_measure.log");
    let raw = run_dir.join("measure_raw.txt");
    let summary = run_dir.join("measure_summary.csv");

    on_progress(
        "Writing analog measure wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_measure_wrapper(
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
                "Failed to write ngspice measure wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog measure backend",
        format!(
            "{} .MEASURE {} with {} statement(s).",
            backend,
            analog
                .analysis
                .measure_mode
                .as_deref()
                .unwrap_or("<missing>"),
            analog.analysis.measure_statements.len()
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
                "Failed to write ngspice measure log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice measure analysis exited with status {}.",
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
                "Failed to write ngspice measure raw output {}: {error}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(raw.clone());
    let summary_csv = measure_raw_to_csv(&log_text, scenario)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&summary, summary_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write measure summary CSV {}: {error}",
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
        raw_outputs: &[("ngspice_measure_stdout", &raw)],
        normalized_outputs: &[("measure_summary", &summary)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceMeasureRun { artifacts, summary })
}

fn build_ngspice_measure_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before measure wrapper generation");
    let mode = analog
        .analysis
        .measure_mode
        .as_deref()
        .ok_or_else(|| "measure_mode is required for .MEASURE analysis.".to_string())?;
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for ngspice .MEASURE. Do not edit by hand.\n");
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
    text.push_str(".control\n");
    if mode == "tran" {
        text.push_str("tran ");
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
        text.push_str("ac dec ");
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
    for statement in &analog.analysis.measure_statements {
        text.push_str(&normalize_measure_statement(&statement.statement));
        text.push('\n');
    }
    text.push_str("quit\n.endc\n.end\n");
    Ok(text)
}

fn measure_raw_to_csv(log: &str, scenario: &Scenario) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before measure parsing");
    let mode = analog.analysis.measure_mode.as_deref().unwrap_or("");
    let names: BTreeSet<&str> = analog
        .analysis
        .measure_statements
        .iter()
        .map(|statement| statement.name.as_str())
        .collect();
    let mut rows = Vec::new();
    for line in log.lines() {
        let Some((name, rest)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if !names
            .iter()
            .any(|expected| expected.eq_ignore_ascii_case(name))
        {
            continue;
        }
        let Some(value) = parse_number(rest.split_whitespace().next().unwrap_or("")) else {
            continue;
        };
        rows.push((name.to_string(), value, line.trim().to_string()));
    }
    if rows.len() != names.len() {
        let found: BTreeSet<String> = rows.iter().map(|row| row.0.to_ascii_lowercase()).collect();
        let missing: Vec<&str> = names
            .iter()
            .copied()
            .filter(|name| !found.contains(&name.to_ascii_lowercase()))
            .collect();
        return Err(format!(
            "ngspice .MEASURE output did not contain scalar result(s) for {}.",
            missing.join(", ")
        ));
    }
    let mut csv = String::from("measurement,mode,value,raw_line\n");
    for (name, value, raw_line) in rows {
        csv.push_str(&csv_escape(&name));
        csv.push(',');
        csv.push_str(&csv_escape(mode));
        csv.push(',');
        csv.push_str(&format!("{value:.12e}"));
        csv.push(',');
        csv.push_str(&csv_escape(&raw_line));
        csv.push('\n');
    }
    Ok(csv)
}

fn normalize_measure_statement(statement: &str) -> String {
    let trimmed = statement.trim();
    if let Some(rest) = trimmed.strip_prefix('.') {
        rest.trim_start().to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_number(value: &str) -> Option<f64> {
    value
        .trim_matches(|ch: char| ch == ',' || ch == ';')
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
    use super::measure_raw_to_csv;
    use crate::board_ir::Scenario;

    fn scenario() -> Scenario {
        serde_yaml_ng::from_str(
            r#"
name: measure
type: analog_measure
analog:
  backend: ngspice
  netlist_source: generated_from_board
  model_files: []
  node_bindings: []
  pin_bindings: []
  analysis:
    type: measure
    measure_mode: tran
    stop_time_us: 100.0
    max_step_us: 0.1
    measure_statements:
      - name: avg_out
        statement: meas tran avg_out AVG v(out) FROM=20u TO=100u
      - name: max_out
        statement: meas tran max_out MAX v(out) FROM=0 TO=100u
  stimuli: []
  probes: []
  assertions: []
"#,
        )
        .unwrap()
    }

    #[test]
    fn measure_raw_to_csv_extracts_declared_scalars() {
        let csv = measure_raw_to_csv(
            "avg_out = 5.10001e-01 from= 2.0e-05 to= 1.0e-04\nmax_out = 9.93663e-01 at= 9.51000e-05\n",
            &scenario(),
        )
        .unwrap();

        assert!(csv.contains("avg_out,tran,5.100010000000e-1"));
        assert!(csv.contains("max_out,tran,9.936630000000e-1"));
    }
}
