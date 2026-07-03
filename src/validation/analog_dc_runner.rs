use crate::board_ir::{AnalogScenario, Scenario};
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

pub(super) struct NgspiceDcRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) operating_point: PathBuf,
}

pub(super) struct NgspiceDcRunOptions<'a, F, C>
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

pub(super) fn run_ngspice_dc<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceDcRunOptions<'_, F, C>,
) -> Result<NgspiceDcRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceDcRunOptions {
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
        .expect("analog was validated before DC run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog DC run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_op.cir");
    let log = run_dir.join("ngspice_op.log");
    let raw = run_dir.join("operating_point_raw.csv");
    let operating_point = run_dir.join("operating_point.csv");
    on_progress(
        "Writing analog DC wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_dc_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("operating_point_raw.csv"),
        parameter_overrides,
        model_section_overrides,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice DC wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog DC backend",
        format!(
            "{} operating point for {} probe(s).",
            backend,
            analog.probes.len()
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
    on_progress(
        "Writing analog DC solver log",
        format!("Writing {}.", log.to_string_lossy()),
    );
    fs::write(&log, &log_text).map_err(|error| {
        ngspice_error(
            format!("Failed to write ngspice DC log {}: {error}", log.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice DC operating-point analysis exited with status {}.",
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
    if !raw.is_file() {
        return Err(ngspice_error(
            format!(
                "ngspice completed without producing operating-point export {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    on_progress(
        "Loading analog operating point",
        format!("Reading {}.", raw.to_string_lossy()),
    );
    let op_csv = op_raw_to_operating_point_csv(&raw, analog).map_err(|message| {
        ngspice_error(
            format!(
                "Failed to convert operating point {}: {message}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&operating_point, op_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write operating-point CSV {}: {error}",
                operating_point.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(operating_point.clone());
    on_progress(
        "Exported analog operating point",
        format!("Wrote {}.", operating_point.to_string_lossy()),
    );
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "operating_point",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("ngspice_operating_point_raw", &raw)],
        normalized_outputs: &[("operating_point", &operating_point)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceDcRun {
        artifacts,
        operating_point,
    })
}

fn build_ngspice_dc_wrapper(
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
        .expect("analog was validated before DC wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI. Do not edit by hand.\n");
    text.push_str("* Source netlist: ");
    text.push_str(&netlist.to_string_lossy());
    text.push('\n');
    let include_base = netlist.parent().unwrap_or(&bound.project.source_dir);
    for line in source.lines() {
        let trimmed = line.trim_start();
        let directive = trimmed.to_ascii_lowercase();
        let first_token = directive.split_whitespace().next().unwrap_or("");
        if matches!(first_token, ".end" | ".tran" | ".ac" | ".op") {
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
    text.push_str("set wr_vecnames\n");
    text.push_str("set wr_singlescale\n");
    text.push_str("op\n");
    text.push_str("wrdata ");
    text.push_str(&raw_output.to_string_lossy());
    for probe in &analog.probes {
        text.push(' ');
        text.push_str(&probe.expression);
    }
    text.push_str("\nquit\n.endc\n.end\n");
    Ok(text)
}

fn op_raw_to_operating_point_csv(raw: &Path, analog: &AnalogScenario) -> Result<String, String> {
    let text = fs::read_to_string(raw).map_err(|error| {
        format!(
            "Failed to read operating-point raw export {}: {error}",
            raw.display()
        )
    })?;
    let mut csv = String::new();
    for (index, probe) in analog.probes.iter().enumerate() {
        if index > 0 {
            csv.push(',');
        }
        csv.push_str(&sanitize_csv_column(&probe.name));
    }
    csv.push('\n');
    let expected_columns = analog.probes.len() + 1;
    for (line_index, line) in text.lines().enumerate() {
        let fields: Vec<_> = line
            .split(|character: char| character == ',' || character.is_whitespace())
            .filter(|field| !field.is_empty())
            .collect();
        if fields.is_empty() {
            continue;
        }
        if parse_float(fields[0]).is_none() {
            continue;
        }
        if fields.len() < expected_columns {
            return Err(format!(
                "Operating-point row {} has {} columns, expected at least {}.",
                line_index + 1,
                fields.len(),
                expected_columns
            ));
        }
        for probe_index in 0..analog.probes.len() {
            let field_index = probe_index + 1;
            let value = parse_float(fields[field_index]).ok_or_else(|| {
                format!(
                    "Operating-point row {} has non-numeric probe value {}.",
                    line_index + 1,
                    fields[field_index]
                )
            })?;
            if probe_index > 0 {
                csv.push(',');
            }
            csv.push_str(&format!("{value:.12e}"));
        }
        csv.push('\n');
        return Ok(csv);
    }
    Err(format!(
        "Operating-point raw export {} has no numeric rows.",
        raw.display()
    ))
}

fn sanitize_csv_column(name: &str) -> String {
    let mut output = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "probe".to_string()
    } else {
        output
    }
}

fn parse_float(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

#[cfg(test)]
mod tests {
    use super::{build_ngspice_dc_wrapper, op_raw_to_operating_point_csv};
    use crate::board_ir::BoardProject;
    use crate::library::{bind_project, load_library};
    use std::path::Path;

    #[test]
    fn op_raw_export_converts_to_operating_point_columns() {
        let project: BoardProject = serde_yaml_ng::from_str(
            r#"
project:
  name: op_test
  version: 0.1.0
board:
  components: {}
  nets: {}
scenarios:
  - name: bias
    type: analog_dc
    checks: [SPICE_DC_ANALYSIS]
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis: { type: op }
      stimuli: []
      probes:
        - { name: vin, expression: V(in) }
        - { name: vout, expression: V(out) }
      assertions: []
"#,
        )
        .unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("op_raw.csv");
        std::fs::write(
            &path,
            " in              v(in)           v(out)\n 5.00000000e+00  5.00000000e+00  2.50000000e+00\n",
        )
        .unwrap();
        let csv = op_raw_to_operating_point_csv(&path, analog).unwrap();
        assert_eq!(csv, "vin,vout\n5.000000000000e0,2.500000000000e0\n");
    }

    #[test]
    fn dc_wrapper_runs_op_and_strips_existing_analysis_cards() {
        let project: BoardProject = serde_yaml_ng::from_str(
            r#"
project:
  name: op_wrapper
  version: 0.1.0
board:
  components: {}
  nets: {}
scenarios:
  - name: bias
    type: analog_dc
    checks: [SPICE_DC_ANALYSIS]
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis: { type: op }
      stimuli: []
      probes:
        - { name: out, expression: V(out) }
      assertions: []
"#,
        )
        .unwrap();
        let (library, findings) = load_library(Path::new("project.yaml"), &project);
        assert!(findings.is_empty());
        let bound = bind_project(&project, library, findings);
        let dir = tempfile::tempdir().unwrap();
        let deck = dir.path().join("deck.cir");
        std::fs::write(&deck, "* source\nV1 out 0 DC 2.5\n.op\n.end\n").unwrap();
        let wrapper = build_ngspice_dc_wrapper(
            &bound,
            &project.scenarios[0],
            &deck,
            Path::new("operating_point_raw.csv"),
            &[],
            &[],
        )
        .unwrap();
        assert!(wrapper.contains("\nop\n"));
        assert!(wrapper.contains("wrdata operating_point_raw.csv V(out)"));
        assert!(!wrapper.contains("\n.op\n"));
    }
}
