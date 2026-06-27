use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::analog_runner::{
    ModelSectionOverride, NgspiceRunError, ParameterOverride, detect_nonconvergence, ngspice_error,
    rewrite_include_line, run_solver_with_timeout, sweep_temperature_c,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};

pub(super) struct NgspiceNoiseRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) noise_spectrum: PathBuf,
    pub(super) noise_total: PathBuf,
}

pub(super) struct NgspiceNoiseRunOptions<'a, F, C>
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

pub(super) fn run_ngspice_noise<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceNoiseRunOptions<'_, F, C>,
) -> Result<NgspiceNoiseRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceNoiseRunOptions {
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
        .expect("analog was validated before noise run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog noise run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_noise.cir");
    let log = run_dir.join("ngspice_noise.log");
    let spectrum_raw = run_dir.join("noise_spectrum_raw.csv");
    let total_raw = run_dir.join("noise_total_raw.csv");
    let noise_spectrum = run_dir.join("noise_spectrum.csv");
    let noise_total = run_dir.join("noise_total.csv");

    on_progress(
        "Writing analog noise wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_noise_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("noise_spectrum_raw.csv"),
        Path::new("noise_total_raw.csv"),
        parameter_overrides,
        model_section_overrides,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice noise wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog noise backend",
        format!(
            "{} noise sweep for output node {}.",
            backend,
            analog
                .analysis
                .noise_output_node
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
                "Failed to write ngspice noise log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice noise analysis exited with status {}.",
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
    if !spectrum_raw.is_file() || !total_raw.is_file() {
        return Err(ngspice_error(
            format!(
                "ngspice completed without producing noise exports {} and {}.",
                spectrum_raw.display(),
                total_raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(spectrum_raw.clone());
    artifacts.push(total_raw.clone());

    let spectrum_csv = noise_spectrum_raw_to_csv(&spectrum_raw).map_err(|message| {
        ngspice_error(
            format!(
                "Failed to convert noise spectrum {}: {message}",
                spectrum_raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&noise_spectrum, spectrum_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write noise spectrum CSV {}: {error}",
                noise_spectrum.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(noise_spectrum.clone());

    let total_csv = noise_total_raw_to_csv(&total_raw).map_err(|message| {
        ngspice_error(
            format!(
                "Failed to convert total noise {}: {message}",
                total_raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&noise_total, total_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write total noise CSV {}: {error}",
                noise_total.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(noise_total.clone());
    on_progress(
        "Exported analog noise",
        format!(
            "Wrote {} and {}.",
            noise_spectrum.display(),
            noise_total.display()
        ),
    );
    Ok(NgspiceNoiseRun {
        artifacts,
        noise_spectrum,
        noise_total,
    })
}

fn build_ngspice_noise_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    spectrum_raw_output: &Path,
    total_raw_output: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before noise wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let output_node = analog
        .analysis
        .noise_output_node
        .as_deref()
        .ok_or_else(|| "noise_output_node is required for noise analysis".to_string())?;
    let input_source = analog
        .analysis
        .noise_input_source
        .as_deref()
        .ok_or_else(|| "noise_input_source is required for noise analysis".to_string())?;
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
        if matches!(first_token, ".end" | ".tran" | ".ac" | ".op" | ".noise") {
            continue;
        }
        let rewritten = rewrite_include_line(line, include_base);
        text.push_str(&ensure_noise_input_source_has_ac(&rewritten, input_source));
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
    let output_expr = if let Some(reference_node) = analog.analysis.noise_reference_node.as_deref()
    {
        format!("v({output_node},{reference_node})")
    } else {
        format!("v({output_node})")
    };
    let start_hz = analog
        .analysis
        .start_frequency_hz
        .ok_or_else(|| "start_frequency_hz is required for noise analysis".to_string())?;
    let stop_hz = analog
        .analysis
        .stop_frequency_hz
        .ok_or_else(|| "stop_frequency_hz is required for noise analysis".to_string())?;
    let points = analog.analysis.points_per_decade.unwrap_or(20);
    text.push_str(".control\n");
    text.push_str("set wr_vecnames\n");
    text.push_str("set wr_singlescale\n");
    text.push_str("noise ");
    text.push_str(&output_expr);
    text.push(' ');
    text.push_str(input_source);
    text.push_str(" dec ");
    text.push_str(&points.to_string());
    text.push(' ');
    text.push_str(&format!("{start_hz:.12e}"));
    text.push(' ');
    text.push_str(&format!("{stop_hz:.12e}"));
    text.push('\n');
    text.push_str("setplot noise1\nwrdata ");
    text.push_str(&spectrum_raw_output.to_string_lossy());
    text.push_str(" onoise_spectrum inoise_spectrum\n");
    text.push_str("setplot noise2\nwrdata ");
    text.push_str(&total_raw_output.to_string_lossy());
    text.push_str(" onoise_total inoise_total\nquit\n.endc\n.end\n");
    Ok(text)
}

fn ensure_noise_input_source_has_ac(line: &str, input_source: &str) -> String {
    let trimmed = line.trim_start();
    let Some(first_token) = trimmed.split_whitespace().next() else {
        return line.to_string();
    };
    if !first_token.eq_ignore_ascii_case(input_source) {
        return line.to_string();
    }
    if trimmed
        .to_ascii_lowercase()
        .split_whitespace()
        .any(|token| token == "ac")
    {
        return line.to_string();
    }
    format!("{line} AC 1")
}

fn noise_spectrum_raw_to_csv(raw: &Path) -> Result<String, String> {
    let text = fs::read_to_string(raw).map_err(|error| {
        format!(
            "Failed to read noise spectrum raw export {}: {error}",
            raw.display()
        )
    })?;
    let mut csv = "frequency_hz,onoise_v_per_sqrt_hz,inoise_v_per_sqrt_hz\n".to_string();
    let mut rows = 0usize;
    for (line_index, line) in text.lines().enumerate() {
        let fields = split_fields(line);
        if fields.is_empty() || parse_float(fields[0]).is_none() {
            continue;
        }
        if fields.len() < 3 {
            return Err(format!(
                "Noise spectrum row {} has {} columns, expected at least 3.",
                line_index + 1,
                fields.len()
            ));
        }
        let frequency = parse_float(fields[0]).ok_or_else(|| {
            format!(
                "Noise spectrum row {} has invalid frequency.",
                line_index + 1
            )
        })?;
        let onoise = parse_float(fields[fields.len() - 2]).ok_or_else(|| {
            format!(
                "Noise spectrum row {} has invalid output noise density.",
                line_index + 1
            )
        })?;
        let inoise = parse_float(fields[fields.len() - 1]).ok_or_else(|| {
            format!(
                "Noise spectrum row {} has invalid input noise density.",
                line_index + 1
            )
        })?;
        csv.push_str(&format!("{frequency:.12e},{onoise:.12e},{inoise:.12e}\n"));
        rows += 1;
    }
    if rows == 0 {
        return Err(format!(
            "Noise spectrum raw export {} has no numeric rows.",
            raw.display()
        ));
    }
    Ok(csv)
}

fn noise_total_raw_to_csv(raw: &Path) -> Result<String, String> {
    let text = fs::read_to_string(raw).map_err(|error| {
        format!(
            "Failed to read noise total raw export {}: {error}",
            raw.display()
        )
    })?;
    for (line_index, line) in text.lines().enumerate() {
        let fields = split_fields(line);
        if fields.is_empty() || parse_float(fields[0]).is_none() {
            continue;
        }
        if fields.len() < 2 {
            return Err(format!(
                "Noise total row {} has {} columns, expected at least 2.",
                line_index + 1,
                fields.len()
            ));
        }
        let output_index = fields.len().saturating_sub(2);
        let input_index = fields.len() - 1;
        let onoise = parse_float(fields[output_index]).ok_or_else(|| {
            format!(
                "Noise total row {} has invalid integrated output noise.",
                line_index + 1
            )
        })?;
        let inoise = parse_float(fields[input_index]).ok_or_else(|| {
            format!(
                "Noise total row {} has invalid integrated input noise.",
                line_index + 1
            )
        })?;
        return Ok(format!(
            "onoise_total_v,inoise_total_v\n{onoise:.12e},{inoise:.12e}\n"
        ));
    }
    Err(format!(
        "Noise total raw export {} has no numeric rows.",
        raw.display()
    ))
}

fn split_fields(line: &str) -> Vec<&str> {
    line.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

fn parse_float(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

#[cfg(test)]
mod tests {
    use super::{build_ngspice_noise_wrapper, noise_spectrum_raw_to_csv, noise_total_raw_to_csv};
    use crate::board_ir::{BoardProject, load_project};
    use crate::library::{bind_project, load_library};
    use std::path::Path;

    #[test]
    fn noise_raw_exports_convert_to_stable_csv() {
        let dir = tempfile::tempdir().unwrap();
        let spectrum = dir.path().join("spectrum.raw");
        std::fs::write(
            &spectrum,
            "frequency onoise_spectrum inoise_spectrum\n1.0e1 2.0e-9 4.0e-9\n1.0e2 3.0e-9 6.0e-9\n",
        )
        .unwrap();
        let csv = noise_spectrum_raw_to_csv(&spectrum).unwrap();
        assert_eq!(
            csv,
            "frequency_hz,onoise_v_per_sqrt_hz,inoise_v_per_sqrt_hz\n1.000000000000e1,2.000000000000e-9,4.000000000000e-9\n1.000000000000e2,3.000000000000e-9,6.000000000000e-9\n"
        );

        let total = dir.path().join("total.raw");
        std::fs::write(
            &total,
            "onoise_total onoise_total inoise_total\n2.0e-7 2.0e-7 4.0e-7\n",
        )
        .unwrap();
        let csv = noise_total_raw_to_csv(&total).unwrap();
        assert_eq!(
            csv,
            "onoise_total_v,inoise_total_v\n2.000000000000e-7,4.000000000000e-7\n"
        );
    }

    #[test]
    fn noise_wrapper_emits_noise_control_and_strips_prior_analyses() {
        let project_path = Path::new("examples/rc_lowpass_scope/project.yaml");
        let project = load_project(project_path).unwrap();
        let (library, findings) = load_library(project_path, &project);
        assert!(findings.is_empty());
        let mut project: BoardProject = project;
        let mut scenario = project.scenarios[0].clone();
        scenario.scenario_type = "analog_noise".to_string();
        scenario.checks = vec!["SPICE_NOISE_ANALYSIS".to_string()];
        let analog = scenario.analog.as_mut().unwrap();
        analog.analysis.analysis_type = "noise".to_string();
        analog.analysis.noise_output_node = Some("filtered".to_string());
        analog.analysis.noise_input_source = Some("VSIN".to_string());
        analog.analysis.start_frequency_hz = Some(10.0);
        analog.analysis.stop_frequency_hz = Some(10_000.0);
        analog.analysis.points_per_decade = Some(5);
        project.scenarios = vec![scenario.clone()];
        let bound = bind_project(&project, library, findings);

        let dir = tempfile::tempdir().unwrap();
        let netlist = dir.path().join("source.cir");
        std::fs::write(
            &netlist,
            "V1 in 0 dc 0 ac 1\n.op\n.ac dec 1 1 10\n.noise v(out) V1 dec 1 1 10\n.end\n",
        )
        .unwrap();
        let wrapper = build_ngspice_noise_wrapper(
            &bound,
            &scenario,
            &netlist,
            Path::new("noise_spectrum_raw.csv"),
            Path::new("noise_total_raw.csv"),
            &[],
            &[],
        )
        .unwrap();
        assert!(wrapper.contains("noise v(filtered) VSIN dec 5 1.000000000000e1 1.000000000000e4"));
        assert!(wrapper.contains("setplot noise1"));
        assert!(wrapper.contains("setplot noise2"));
        assert!(
            !wrapper
                .lines()
                .any(|line| line.trim_start().starts_with(".op"))
        );
        assert!(
            !wrapper
                .lines()
                .any(|line| line.trim_start().starts_with(".ac"))
        );
    }
}
