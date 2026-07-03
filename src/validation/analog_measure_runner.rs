use crate::board_ir::{AnalogMeasureTemplate, Scenario};
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
            "{} .MEASURE {} with {} measurement(s).",
            backend,
            analog
                .analysis
                .measure_mode
                .as_deref()
                .unwrap_or("<missing>"),
            analog.analysis.measure_statements.len() + analog.analysis.measure_templates.len()
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
    for template in &analog.analysis.measure_templates {
        text.push_str(&measure_template_statement(mode, template));
        text.push('\n');
    }
    text.push_str("quit\n.endc\n.end\n");
    Ok(text)
}

pub(super) fn measure_raw_to_csv(log: &str, scenario: &Scenario) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before measure parsing");
    let mode = analog.analysis.measure_mode.as_deref().unwrap_or("");
    let names = measure_names(scenario);
    let mut rows = Vec::new();
    let mut found_names = BTreeSet::new();
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
        let normalized = name.to_ascii_lowercase();
        if found_names.insert(normalized) {
            rows.push((name.to_string(), value, line.trim().to_string()));
        }
    }
    if rows.len() != names.len() {
        for (name, value, raw_line) in measure_table_rows(log, &names) {
            let normalized = name.to_ascii_lowercase();
            if found_names.insert(normalized) {
                rows.push((name, value, raw_line));
            }
        }
    }
    if rows.len() != names.len() {
        let missing: Vec<&str> = names
            .iter()
            .map(String::as_str)
            .filter(|name| !found_names.contains(&name.to_ascii_lowercase()))
            .collect();
        return Err(format!(
            ".MEASURE output did not contain scalar result(s) for {}.",
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

fn measure_table_rows(log: &str, names: &[String]) -> Vec<(String, f64, String)> {
    let lines: Vec<&str> = log.lines().collect();
    let mut rows = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let header_tokens = measure_tokens(line);
        if header_tokens.is_empty() {
            continue;
        }
        let matches: Vec<(&String, usize)> = names
            .iter()
            .filter_map(|name| {
                header_tokens
                    .iter()
                    .position(|token| token.eq_ignore_ascii_case(name))
                    .map(|position| (name, position))
            })
            .collect();
        if matches.is_empty() {
            continue;
        }
        let Some(value_line) = lines[index + 1..]
            .iter()
            .map(|candidate| candidate.trim())
            .find(|candidate| !candidate.is_empty())
        else {
            continue;
        };
        let value_tokens = measure_tokens(value_line);
        for (name, header_position) in matches {
            let value_token = if value_tokens.len() == header_tokens.len() {
                value_tokens.get(header_position)
            } else if value_tokens.len() + 1 == header_tokens.len() && header_position > 0 {
                value_tokens.get(header_position - 1)
            } else {
                value_tokens.get(header_position)
            };
            let Some(value_token) = value_token else {
                continue;
            };
            let Some(value) = parse_number(value_token) else {
                continue;
            };
            rows.push((
                name.clone(),
                value,
                format!("{name} = {value_token} ({value_line})"),
            ));
        }
    }
    rows
}

fn measure_names(scenario: &Scenario) -> Vec<String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before measure-name expansion");
    analog
        .analysis
        .measure_statements
        .iter()
        .map(|statement| statement.name.clone())
        .chain(
            analog
                .analysis
                .measure_templates
                .iter()
                .map(|template| template.name.clone()),
        )
        .collect()
}

fn normalize_measure_statement(statement: &str) -> String {
    let trimmed = statement.trim();
    if let Some(rest) = trimmed.strip_prefix('.') {
        rest.trim_start().to_string()
    } else {
        trimmed.to_string()
    }
}

pub(super) fn measure_template_statement(mode: &str, template: &AnalogMeasureTemplate) -> String {
    let mut statement = String::from("meas ");
    statement.push_str(mode);
    statement.push(' ');
    statement.push_str(&template.name);
    statement.push(' ');
    statement.push_str(&template.operation.to_ascii_uppercase());
    statement.push(' ');
    statement.push_str(&template.expression);
    if mode == "tran" {
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
    } else {
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
    statement
}

fn parse_number(value: &str) -> Option<f64> {
    value
        .trim_matches(|ch: char| ch == ',' || ch == ';')
        .parse()
        .ok()
}

fn measure_tokens(line: &str) -> Vec<&str> {
    line.split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|token| !token.is_empty())
        .collect()
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
    use super::{measure_raw_to_csv, measure_template_statement};
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

    #[test]
    fn measure_raw_to_csv_extracts_xyce_table_scalars() {
        let csv = measure_raw_to_csv(
            "CircuitCI Xyce measure export\nINDEX avg_out max_out\n0 5.10001e-01 9.93663e-01\n",
            &scenario(),
        )
        .unwrap();

        assert!(csv.contains("avg_out,tran,5.100010000000e-1"));
        assert!(csv.contains("max_out,tran,9.936630000000e-1"));
    }

    #[test]
    fn measure_template_statement_formats_transient_window() {
        let scenario = scenario();
        let template = crate::board_ir::AnalogMeasureTemplate {
            name: "avg_out".to_string(),
            operation: "avg".to_string(),
            expression: "v(out)".to_string(),
            from_us: Some(20.0),
            to_us: Some(100.0),
            at_us: None,
            from_hz: None,
            to_hz: None,
            at_hz: None,
        };

        assert_eq!(
            measure_template_statement(
                scenario
                    .analog
                    .as_ref()
                    .unwrap()
                    .analysis
                    .measure_mode
                    .as_deref()
                    .unwrap(),
                &template
            ),
            "meas tran avg_out AVG v(out) FROM=2.000000000000e-5 TO=1.000000000000e-4"
        );
    }
}
