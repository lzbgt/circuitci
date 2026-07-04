use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::analog_runner::{
    ModelSectionOverride, NgspiceRunError, ParameterOverride, SolverManifestIo,
    detect_nonconvergence, ngspice_error, push_ngspice_osdi_load_commands, rewrite_include_line,
    run_solver_with_timeout, sweep_temperature_c, write_solver_manifest,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};

pub(super) struct NgspiceDistortionRunOptions<'a, F, C>
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

pub(super) struct NgspiceDistortionRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) spectrum: PathBuf,
    pub(super) summary: PathBuf,
    pub(super) convergence: PathBuf,
}

pub(super) fn run_ngspice_distortion<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceDistortionRunOptions<'_, F, C>,
) -> Result<NgspiceDistortionRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceDistortionRunOptions {
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
        .expect("analog was validated before distortion run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog distortion run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_disto.cir");
    let log = run_dir.join("ngspice_disto.log");
    let raw = run_dir.join("distortion_raw.txt");
    let spectrum = run_dir.join("distortion_spectrum.csv");
    let summary = run_dir.join("distortion_summary.csv");
    let convergence = run_dir.join("distortion_convergence.json");

    on_progress(
        "Writing analog distortion wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_distortion_wrapper(
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
                "Failed to write ngspice distortion wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog distortion backend",
        format!(
            "{} .DISTO {}.",
            backend,
            analog
                .analysis
                .distortion_output_expression
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
                "Failed to write ngspice distortion log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice distortion analysis exited with status {}.",
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
                "Failed to write ngspice distortion raw output {}: {error}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(raw.clone());
    let rows = parse_distortion_rows(&log_text)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&spectrum, distortion_spectrum_csv(&rows, scenario)).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write distortion spectrum CSV {}: {error}",
                spectrum.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(spectrum.clone());
    fs::write(&summary, distortion_summary_csv(&rows, scenario)).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write distortion summary CSV {}: {error}",
                summary.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(summary.clone());
    fs::write(
        &convergence,
        distortion_convergence_json(&rows, scenario, backend)?,
    )
    .map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write distortion convergence JSON {}: {error}",
                convergence.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(convergence.clone());
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "distortion",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("ngspice_distortion_stdout", &raw)],
        normalized_outputs: &[
            ("distortion_spectrum", &spectrum),
            ("distortion_summary", &summary),
            ("distortion_convergence", &convergence),
        ],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceDistortionRun {
        artifacts,
        spectrum,
        summary,
        convergence,
    })
}

fn build_ngspice_distortion_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before distortion wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {} for distortion wrapper: {error}",
            netlist.display()
        )
    })?;
    let output_expression = analog
        .analysis
        .distortion_output_expression
        .as_deref()
        .ok_or_else(|| "distortion_output_expression is required.".to_string())?;
    let mode = analog
        .analysis
        .distortion_mode
        .as_deref()
        .unwrap_or("harmonic");
    let include_base = netlist.parent().unwrap_or(&bound.project.source_dir);
    let source_marks = distortion_source_marks(analog);
    let mut seen_sources = BTreeSet::new();
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for ngspice .DISTO. Do not edit by hand.\n");
    text.push_str("* Source netlist: ");
    text.push_str(&netlist.to_string_lossy());
    text.push('\n');
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
                | ".disto"
                | ".four"
                | ".print"
        ) {
            continue;
        }
        let rewritten = rewrite_include_line(line, include_base);
        let annotated = annotate_distortion_source(&rewritten, &source_marks, &mut seen_sources);
        text.push_str(&annotated);
        text.push('\n');
    }
    let missing: Vec<_> = source_marks
        .keys()
        .filter(|source| !seen_sources.contains(*source))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "ngspice .DISTO wrapper could not annotate source lines for {}.",
            missing.join(", ")
        ));
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
    text.push_str("disto dec ");
    text.push_str(
        &analog
            .analysis
            .distortion_points_per_decade
            .expect("distortion points were validated")
            .to_string(),
    );
    text.push(' ');
    text.push_str(&format!(
        "{:.12e}",
        analog
            .analysis
            .distortion_start_frequency_hz
            .expect("distortion start frequency was validated")
    ));
    text.push(' ');
    text.push_str(&format!(
        "{:.12e}",
        analog
            .analysis
            .distortion_stop_frequency_hz
            .expect("distortion stop frequency was validated")
    ));
    if mode == "intermodulation" {
        text.push(' ');
        text.push_str(&format!(
            "{:.12e}",
            analog
                .analysis
                .distortion_f2_over_f1
                .expect("distortion f2/f1 ratio was validated")
        ));
    }
    text.push('\n');
    for (plot, component) in distortion_plot_components(mode)? {
        text.push_str("setplot ");
        text.push_str(plot);
        text.push_str("\necho CIRCUITCI_DISTORTION_COMPONENT ");
        text.push_str(component);
        text.push_str("\nprint ");
        text.push_str(output_expression);
        text.push('\n');
    }
    text.push_str("quit\n.endc\n.end\n");
    Ok(text)
}

fn distortion_source_marks(analog: &crate::board_ir::AnalogScenario) -> BTreeMap<String, String> {
    let mut marks: BTreeMap<String, String> = BTreeMap::new();
    for source in &analog.analysis.distortion_f1_sources {
        marks
            .entry(source.clone())
            .and_modify(|mark| mark.push_str(" DISTOF1 1.0 0.0"))
            .or_insert_with(|| "DISTOF1 1.0 0.0".to_string());
    }
    for source in &analog.analysis.distortion_f2_sources {
        marks
            .entry(source.clone())
            .and_modify(|mark| mark.push_str(" DISTOF2 1.0 0.0"))
            .or_insert_with(|| "DISTOF2 1.0 0.0".to_string());
    }
    marks
}

fn annotate_distortion_source(
    line: &str,
    source_marks: &BTreeMap<String, String>,
    seen_sources: &mut BTreeSet<String>,
) -> String {
    let Some(first_token) = line.split_whitespace().next() else {
        return line.to_string();
    };
    for (source, mark) in source_marks {
        if source_token_matches(first_token, source) {
            seen_sources.insert(source.clone());
            return format!("{line} {mark}");
        }
    }
    line.to_string()
}

fn source_token_matches(token: &str, source: &str) -> bool {
    let candidates = [
        source.to_string(),
        format!("V{}", element_suffix(source)),
        format!("I{}", element_suffix(source)),
    ];
    candidates
        .iter()
        .any(|candidate| token.eq_ignore_ascii_case(candidate))
}

fn element_suffix(component_id: &str) -> String {
    let mut suffix = String::new();
    for character in component_id.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            suffix.push(character);
        } else {
            suffix.push('_');
        }
    }
    if suffix.is_empty() {
        suffix.push('X');
    }
    suffix
}

fn distortion_plot_components(mode: &str) -> Result<Vec<(&'static str, &'static str)>, String> {
    match mode {
        "harmonic" => Ok(vec![("disto1", "h2"), ("disto2", "h3")]),
        "intermodulation" => Ok(vec![
            ("disto1", "im_f1_plus_f2"),
            ("disto2", "im_f1_minus_f2"),
            ("disto3", "im_2f1_minus_f2"),
        ]),
        _ => Err(format!("unsupported distortion mode {mode}")),
    }
}

fn parse_distortion_rows(log: &str) -> Result<Vec<DistortionRow>, String> {
    let mut rows = Vec::new();
    let mut component: Option<String> = None;
    let mut in_table = false;
    for line in log.lines() {
        if let Some(raw_component) = line.trim().strip_prefix("CIRCUITCI_DISTORTION_COMPONENT ") {
            component = Some(raw_component.trim().to_string());
            in_table = false;
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() >= 3
            && fields[0].eq_ignore_ascii_case("index")
            && fields[1].eq_ignore_ascii_case("frequency")
        {
            in_table = true;
            continue;
        }
        if !in_table || fields.len() < 4 || fields[0].parse::<usize>().is_err() {
            continue;
        }
        let Some(component) = component.as_ref() else {
            continue;
        };
        let Some(frequency_hz) = parse_number(fields[1]) else {
            continue;
        };
        let Some(real) = parse_number(fields[2]) else {
            continue;
        };
        let Some(imaginary) = parse_number(fields[3]) else {
            continue;
        };
        rows.push(DistortionRow {
            component: component.clone(),
            frequency_hz,
            real,
            imaginary,
        });
    }
    if rows.is_empty() {
        return Err("ngspice .DISTO output did not contain distortion rows.".to_string());
    }
    Ok(rows)
}

#[derive(Clone, Debug)]
struct DistortionRow {
    component: String,
    frequency_hz: f64,
    real: f64,
    imaginary: f64,
}

impl DistortionRow {
    fn magnitude(&self) -> f64 {
        (self
            .real
            .mul_add(self.real, self.imaginary * self.imaginary))
        .sqrt()
    }

    fn phase_degrees(&self) -> f64 {
        self.imaginary.atan2(self.real).to_degrees()
    }
}

fn distortion_spectrum_csv(rows: &[DistortionRow], scenario: &Scenario) -> String {
    let output_expression = scenario
        .analog
        .as_ref()
        .and_then(|analog| analog.analysis.distortion_output_expression.as_deref())
        .unwrap_or("");
    let mut csv = String::from(
        "component,frequency_hz,output_expression,real,imaginary,magnitude,phase_degrees\n",
    );
    for row in rows {
        csv.push_str(&csv_escape(&row.component));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.frequency_hz));
        csv.push(',');
        csv.push_str(&csv_escape(output_expression));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.real));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.imaginary));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.magnitude()));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", row.phase_degrees()));
        csv.push('\n');
    }
    csv
}

fn distortion_summary_csv(rows: &[DistortionRow], scenario: &Scenario) -> String {
    let output_expression = scenario
        .analog
        .as_ref()
        .and_then(|analog| analog.analysis.distortion_output_expression.as_deref())
        .unwrap_or("");
    let mut by_component: BTreeMap<&str, Vec<&DistortionRow>> = BTreeMap::new();
    for row in rows {
        by_component.entry(&row.component).or_default().push(row);
    }
    let mut csv =
        String::from("component,output_expression,row_count,max_magnitude,frequency_hz_at_max\n");
    for (component, component_rows) in by_component {
        let max_row = component_rows
            .iter()
            .copied()
            .max_by(|left, right| left.magnitude().total_cmp(&right.magnitude()))
            .expect("component group is nonempty");
        csv.push_str(&csv_escape(component));
        csv.push(',');
        csv.push_str(&csv_escape(output_expression));
        csv.push(',');
        csv.push_str(&component_rows.len().to_string());
        csv.push(',');
        csv.push_str(&format!("{:.12e}", max_row.magnitude()));
        csv.push(',');
        csv.push_str(&format!("{:.12e}", max_row.frequency_hz));
        csv.push('\n');
    }
    csv
}

fn distortion_convergence_json(
    rows: &[DistortionRow],
    scenario: &Scenario,
    backend: &str,
) -> Result<String, NgspiceRunError> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before distortion convergence generation");
    let mut components = BTreeSet::new();
    for row in rows {
        components.insert(row.component.clone());
    }
    serde_json::to_string_pretty(&json!({
        "status": "pass",
        "backend": backend,
        "analysis": "distortion",
        "mode": analog.analysis.distortion_mode.as_deref().unwrap_or("harmonic"),
        "row_count": rows.len(),
        "components": components.into_iter().collect::<Vec<_>>(),
        "output_expression": analog.analysis.distortion_output_expression,
        "nonconvergence_detected": false,
    }))
    .map_err(|error| {
        ngspice_error(
            format!("Failed to encode distortion convergence JSON: {error}"),
            Vec::new(),
        )
    })
}

fn parse_number(value: &str) -> Option<f64> {
    value
        .trim_matches(|ch: char| ch == ',' || ch == ';' || ch == '\u{c}')
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
    use super::{distortion_spectrum_csv, distortion_summary_csv, parse_distortion_rows};
    use crate::board_ir::Scenario;

    fn scenario() -> Scenario {
        serde_yaml_ng::from_str(
            r#"
name: disto
type: analog_distortion
analog:
  backend: ngspice
  netlist_source: generated_from_board
  model_files: []
  node_bindings: []
  pin_bindings: []
  analysis:
    type: disto
    distortion_output_expression: V(out)
  stimuli: []
  probes: []
  assertions: []
"#,
        )
        .unwrap()
    }

    #[test]
    fn parses_printed_distortion_tables() {
        let rows = parse_distortion_rows(
            "CIRCUITCI_DISTORTION_COMPONENT h2\nIndex frequency v(out)\n0 1.0e3 2.0e-3, -3.0e-4\nCIRCUITCI_DISTORTION_COMPONENT h3\nIndex frequency v(out)\n0 1.0e3 4.0e-5, 0.0e0\n",
        )
        .unwrap();

        assert_eq!(rows.len(), 2);
        let spectrum = distortion_spectrum_csv(&rows, &scenario());
        assert!(spectrum.contains("h2,1.000000000000e3,V(out),2.000000000000e-3"));
        let summary = distortion_summary_csv(&rows, &scenario());
        assert!(summary.contains("h3,V(out),1,4.000000000000e-5"));
    }
}
