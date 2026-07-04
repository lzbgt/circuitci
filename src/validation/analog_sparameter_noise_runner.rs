use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::analog_runner::{
    ModelSectionOverride, NgspiceRunError, ParameterOverride, SolverManifestIo,
    detect_nonconvergence, ngspice_error, parse_float, push_ngspice_osdi_load_commands,
    rewrite_include_line, run_solver_with_timeout, sweep_temperature_c, write_solver_manifest,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};
use super::analog_xyce_runner::append_sparameter_reflection_metadata;

pub(super) struct NgspiceSParameterNoiseRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) s_parameters: PathBuf,
    pub(super) noise_summary: PathBuf,
}

pub(super) struct NgspiceSParameterNoiseRunOptions<'a, F, C>
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

pub(super) fn run_ngspice_sparameter_noise<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: NgspiceSParameterNoiseRunOptions<'_, F, C>,
) -> Result<NgspiceSParameterNoiseRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let NgspiceSParameterNoiseRunOptions {
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
        .expect("analog was validated before ngspice SP-noise run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog S-parameter noise run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_ngspice_sparameter_noise.cir");
    let log = run_dir.join("ngspice_sparameter_noise.log");
    let raw_s_parameters = run_dir.join("s_parameters_raw.csv");
    let s_parameters = run_dir.join("s_parameters.csv");
    let raw = run_dir.join("s_parameter_noise_raw.csv");
    let noise_summary = run_dir.join("s_parameter_noise_summary.csv");

    on_progress(
        "Writing analog S-parameter noise wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_ngspice_sparameter_noise_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("s_parameters_raw.csv"),
        Path::new("s_parameter_noise_raw.csv"),
        parameter_overrides,
        model_section_overrides,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice S-parameter noise wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog S-parameter noise backend",
        format!(
            "{} SP-noise sweep for {} port(s).",
            backend,
            analog.analysis.s_parameter_ports.len()
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
                "Failed to write ngspice S-parameter noise log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice S-parameter noise analysis exited with status {}.",
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
    if !raw_s_parameters.is_file() {
        return Err(ngspice_error(
            format!(
                "ngspice completed without producing S-parameter export {}.",
                raw_s_parameters.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw_s_parameters.clone());
    let s_csv = ngspice_sparameter_raw_to_csv(
        &raw_s_parameters,
        analog.analysis.s_parameter_ports.len(),
        scenario,
    )
    .map_err(|message| {
        ngspice_error(
            format!(
                "Failed to normalize ngspice S-parameter export {}: {message}",
                raw_s_parameters.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&s_parameters, s_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice S-parameter CSV {}: {error}",
                s_parameters.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(s_parameters.clone());
    if !raw.is_file() {
        return Err(ngspice_error(
            format!(
                "ngspice completed without producing S-parameter noise export {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    let summary_csv = s_parameter_noise_raw_to_summary_csv(&raw).map_err(|message| {
        ngspice_error(
            format!(
                "Failed to normalize S-parameter noise export {}: {message}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&noise_summary, summary_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write S-parameter noise summary {}: {error}",
                noise_summary.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(noise_summary.clone());
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "s_parameter_noise",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[
            ("ngspice_s_parameters_raw", &raw_s_parameters),
            ("ngspice_s_parameter_noise_raw", &raw),
        ],
        normalized_outputs: &[
            ("s_parameters", &s_parameters),
            ("s_parameter_noise_summary", &noise_summary),
        ],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceSParameterNoiseRun {
        artifacts,
        s_parameters,
        noise_summary,
    })
}

fn build_ngspice_sparameter_noise_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    s_parameters_raw: &Path,
    raw_output: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before ngspice SP-noise wrapper generation");
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
        if matches!(
            first_token,
            ".end" | ".tran" | ".ac" | ".op" | ".noise" | ".sp" | ".lin" | ".print"
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
    for (index, port) in analog.analysis.s_parameter_ports.iter().enumerate() {
        text.push_str(&format!(
            "V_CIRCUITCI_P{} {} {} dc 0 ac 1 portnum {} z0 {:.12e}\n",
            index + 1,
            port.positive_node,
            port.negative_node,
            index + 1,
            port.reference_impedance_ohm
        ));
    }
    let start_hz = analog
        .analysis
        .start_frequency_hz
        .ok_or_else(|| "start_frequency_hz is required for SP-noise analysis".to_string())?;
    let stop_hz = analog
        .analysis
        .stop_frequency_hz
        .ok_or_else(|| "stop_frequency_hz is required for SP-noise analysis".to_string())?;
    let points = analog.analysis.points_per_decade.unwrap_or(20);
    text.push_str(".control\n");
    push_ngspice_osdi_load_commands(&mut text, bound, scenario)?;
    text.push_str("set wr_vecnames\nset wr_singlescale\n");
    text.push_str(&format!(
        "sp dec {points} {start_hz:.12e} {stop_hz:.12e} 1\n"
    ));
    text.push_str("wrdata ");
    text.push_str(&s_parameters_raw.to_string_lossy());
    for (row, column) in sparameter_pairs(analog.analysis.s_parameter_ports.len()) {
        text.push_str(&format!(" S_{row}_{column}"));
    }
    text.push('\n');
    text.push_str("wrdata ");
    text.push_str(&raw_output.to_string_lossy());
    text.push_str(" NF NFmin Rn SOpt\nquit\n.endc\n.end\n");
    Ok(text)
}

pub(super) fn ngspice_sparameter_raw_to_csv(
    raw: &Path,
    port_count: usize,
    scenario: &Scenario,
) -> Result<String, String> {
    let text = fs::read_to_string(raw).map_err(|error| {
        format!(
            "failed to read ngspice S-parameter raw CSV {}: {error}",
            raw.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "ngspice S-parameter export is empty".to_string())?;
    let columns = split_fields(header);
    let frequency = find_column(&columns, &["frequency_hz", "frequency", "freq"])
        .ok_or_else(|| "ngspice S-parameter export lacks a frequency column".to_string())?;
    let pairs = sparameter_pairs(port_count);
    let mut term_columns = Vec::new();
    for (row, column) in &pairs {
        let term = format!("s{row}{column}");
        let ngspice_term = format!("s_{row}_{column}");
        term_columns.push(
            find_sparameter_term_columns(&columns, &term, &ngspice_term).ok_or_else(|| {
                format!("ngspice S-parameter export lacks {ngspice_term} real/imaginary columns")
            })?,
        );
    }

    let mut output = String::from("frequency_hz,reference_impedance_ohm");
    for (row, column) in &pairs {
        output.push_str(&format!(
            ",s{row}{column}_mag_db,s{row}{column}_phase_deg,s{row}{column}_mag_linear"
        ));
    }
    output.push('\n');
    let reference_impedance_ohm = scenario
        .analog
        .as_ref()
        .and_then(|analog| analog.analysis.s_parameter_ports.first())
        .map(|port| port.reference_impedance_ohm)
        .unwrap_or(50.0);
    if !reference_impedance_ohm.is_finite() || reference_impedance_ohm <= 0.0 {
        return Err(format!(
            "ngspice S-parameter export has invalid reference impedance {reference_impedance_ohm}"
        ));
    }
    let mut row_count = 0usize;
    let mut previous_frequency = None;
    for (line_index, line) in lines.enumerate() {
        let fields = split_fields(line);
        let frequency_hz = parse_column(&fields, frequency, line_index, "frequency")?;
        if frequency_hz <= 0.0 {
            return Err(format!(
                "ngspice S-parameter row {} has non-positive frequency",
                line_index + 2
            ));
        }
        if previous_frequency.is_some_and(|previous| frequency_hz <= previous) {
            return Err(format!(
                "ngspice S-parameter row {} has duplicate or non-increasing frequency",
                line_index + 2
            ));
        }
        previous_frequency = Some(frequency_hz);
        output.push_str(&format!(
            "{frequency_hz:.12e},{reference_impedance_ohm:.12e}"
        ));
        for columns in &term_columns {
            let real = parse_column(&fields, columns.real, line_index, "S real")?;
            let imaginary = parse_column(&fields, columns.imaginary, line_index, "S imaginary")?;
            let magnitude = real.hypot(imaginary);
            let phase_deg = imaginary.atan2(real).to_degrees();
            let magnitude_db = if magnitude > 0.0 {
                20.0 * magnitude.log10()
            } else {
                -300.0
            };
            output.push_str(&format!(
                ",{magnitude_db:.12e},{phase_deg:.12e},{magnitude:.12e}"
            ));
        }
        output.push('\n');
        row_count += 1;
    }
    if row_count == 0 {
        return Err("ngspice S-parameter export has no data rows".to_string());
    }
    append_sparameter_reflection_metadata(output, scenario)
}

#[derive(Debug, Clone, Copy)]
struct SParameterTermColumns {
    real: usize,
    imaginary: usize,
}

fn find_sparameter_term_columns(
    columns: &[String],
    compact_term: &str,
    ngspice_term: &str,
) -> Option<SParameterTermColumns> {
    let real_names = [
        format!("{compact_term}_real"),
        format!("{compact_term}_re"),
        format!("{ngspice_term}_real"),
        format!("{ngspice_term}_re"),
    ];
    let imaginary_names = [
        format!("{compact_term}_imaginary"),
        format!("{compact_term}_imag"),
        format!("{compact_term}_im"),
        format!("{ngspice_term}_imaginary"),
        format!("{ngspice_term}_imag"),
        format!("{ngspice_term}_im"),
    ];
    if let (Some(real), Some(imaginary)) = (
        columns
            .iter()
            .position(|column| real_names.contains(column)),
        columns
            .iter()
            .position(|column| imaginary_names.contains(column)),
    ) {
        return Some(SParameterTermColumns { real, imaginary });
    }
    let repeated: Vec<_> = columns
        .iter()
        .enumerate()
        .filter_map(|(index, column)| {
            (column == compact_term || column == ngspice_term).then_some(index)
        })
        .collect();
    (repeated.len() >= 2).then_some(SParameterTermColumns {
        real: repeated[0],
        imaginary: repeated[1],
    })
}

fn sparameter_pairs(port_count: usize) -> Vec<(usize, usize)> {
    if port_count == 2 {
        return vec![(1, 1), (2, 1), (1, 2), (2, 2)];
    }
    let mut pairs = Vec::with_capacity(port_count * port_count);
    for row in 1..=port_count {
        for column in 1..=port_count {
            pairs.push((row, column));
        }
    }
    pairs
}

pub(super) fn s_parameter_noise_raw_to_summary_csv(raw: &Path) -> Result<String, String> {
    let text = fs::read_to_string(raw).map_err(|error| {
        format!(
            "failed to read S-parameter noise raw CSV {}: {error}",
            raw.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "S-parameter noise export is empty".to_string())?;
    let columns = split_fields(header);
    let frequency = find_column(&columns, &["frequency_hz", "frequency", "freq"])
        .ok_or_else(|| "S-parameter noise export lacks a frequency column".to_string())?;
    let nf = find_column(&columns, &["noise_figure_db", "nf_db", "nf"])
        .ok_or_else(|| "S-parameter noise export lacks NF/noise figure".to_string())?;
    let nfmin = find_column(&columns, &["minimum_noise_figure_db", "nfmin_db", "nfmin"])
        .ok_or_else(|| "S-parameter noise export lacks NFmin".to_string())?;
    let rn = find_column(
        &columns,
        &["equivalent_noise_resistance_ohm", "rn_ohm", "rn"],
    )
    .ok_or_else(|| "S-parameter noise export lacks Rn".to_string())?;
    let sopt_mag = find_column(
        &columns,
        &[
            "optimum_source_reflection_magnitude",
            "sopt_magnitude",
            "sopt_mag",
        ],
    );
    let sopt_real = find_column(&columns, &["sopt_real", "sopt_re", "sopt"]);
    let sopt_imaginary = find_column(&columns, &["sopt_imaginary", "sopt_im"]);
    if sopt_mag.is_none() && (sopt_real.is_none() || sopt_imaginary.is_none()) {
        return Err(
            "S-parameter noise export lacks SOpt magnitude or real/imaginary columns".into(),
        );
    }

    let mut row_count = 0usize;
    let mut min_frequency_hz = f64::INFINITY;
    let mut max_frequency_hz = f64::NEG_INFINITY;
    let mut max_nf = (f64::NEG_INFINITY, f64::NAN);
    let mut max_nfmin = (f64::NEG_INFINITY, f64::NAN);
    let mut max_rn = (f64::NEG_INFINITY, f64::NAN);
    let mut max_sopt = (f64::NEG_INFINITY, f64::NAN);
    for (line_index, line) in lines.enumerate() {
        let fields = split_fields(line);
        let frequency_hz = parse_column(&fields, frequency, line_index, "frequency")?;
        if frequency_hz <= 0.0 {
            return Err(format!(
                "S-parameter noise row {} has non-positive frequency",
                line_index + 2
            ));
        }
        min_frequency_hz = min_frequency_hz.min(frequency_hz);
        max_frequency_hz = max_frequency_hz.max(frequency_hz);
        update_max(
            &mut max_nf,
            parse_column(&fields, nf, line_index, "NF")?,
            frequency_hz,
        );
        update_max(
            &mut max_nfmin,
            parse_column(&fields, nfmin, line_index, "NFmin")?,
            frequency_hz,
        );
        update_max(
            &mut max_rn,
            parse_column(&fields, rn, line_index, "Rn")?,
            frequency_hz,
        );
        let sopt = if let Some(index) = sopt_mag {
            parse_column(&fields, index, line_index, "SOpt magnitude")?
        } else {
            let real = parse_column(&fields, sopt_real.unwrap(), line_index, "SOpt real")?;
            let imaginary = parse_column(
                &fields,
                sopt_imaginary.unwrap_or(sopt_real.unwrap() + 1),
                line_index,
                "SOpt imaginary",
            )?;
            real.hypot(imaginary)
        };
        if sopt < 0.0 {
            return Err(format!(
                "S-parameter noise row {} has negative SOpt magnitude",
                line_index + 2
            ));
        }
        update_max(&mut max_sopt, sopt, frequency_hz);
        row_count += 1;
    }
    if row_count == 0 {
        return Err("S-parameter noise export has no data rows".to_string());
    }
    Ok(format!(
        "row_count,min_frequency_hz,max_frequency_hz,max_noise_figure_db,frequency_hz_at_max_noise_figure,max_minimum_noise_figure_db,frequency_hz_at_max_minimum_noise_figure,max_equivalent_noise_resistance_ohm,frequency_hz_at_max_equivalent_noise_resistance,max_optimum_source_reflection_magnitude,frequency_hz_at_max_optimum_source_reflection_magnitude\n{row_count},{min_frequency_hz:.12e},{max_frequency_hz:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}\n",
        max_nf.0, max_nf.1, max_nfmin.0, max_nfmin.1, max_rn.0, max_rn.1, max_sopt.0, max_sopt.1,
    ))
}

fn split_fields(line: &str) -> Vec<String> {
    line.split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|field| !field.is_empty())
        .map(|field| {
            field
                .trim_matches(|ch| matches!(ch, '"' | '\'' | '(' | ')' | '[' | ']'))
                .to_ascii_lowercase()
        })
        .collect()
}

fn find_column(columns: &[String], names: &[&str]) -> Option<usize> {
    columns
        .iter()
        .position(|column| names.iter().any(|name| column == name))
}

fn parse_column(
    fields: &[String],
    index: usize,
    line_index: usize,
    label: &str,
) -> Result<f64, String> {
    let field = fields.get(index).ok_or_else(|| {
        format!(
            "S-parameter noise row {} lacks {label} column",
            line_index + 2
        )
    })?;
    let value = parse_float(field).ok_or_else(|| {
        format!(
            "S-parameter noise row {} has invalid {label}",
            line_index + 2
        )
    })?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!(
            "S-parameter noise row {} has non-finite {label}",
            line_index + 2
        ))
    }
}

fn update_max(target: &mut (f64, f64), value: f64, frequency_hz: f64) {
    if value > target.0 {
        *target = (value, frequency_hz);
    }
}

#[cfg(test)]
mod tests {
    use super::{ngspice_sparameter_raw_to_csv, s_parameter_noise_raw_to_summary_csv};

    #[test]
    fn s_parameter_noise_raw_summary_accepts_sopt_real_imaginary() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("s_parameter_noise_raw.csv");
        std::fs::write(
            &raw,
            "frequency_hz,nf_db,nfmin_db,rn_ohm,sopt_real,sopt_imaginary\n1e6,2.0,1.0,4.0,0.3,0.4\n1e9,3.0,1.5,6.0,0.1,0.2\n",
        )
        .unwrap();

        let summary = s_parameter_noise_raw_to_summary_csv(&raw).unwrap();

        assert!(summary.contains("2,1.000000000000e6,1.000000000000e9"));
        assert!(summary.contains("3.000000000000e0,1.000000000000e9"));
        assert!(summary.contains("6.000000000000e0,1.000000000000e9"));
        assert!(summary.contains("5.000000000000e-1,1.000000000000e6"));
    }

    #[test]
    fn ngspice_sparameter_raw_normalizes_explicit_real_imaginary_columns() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("s_parameters_raw.csv");
        std::fs::write(
            &raw,
            "frequency_hz,s_1_1_real,s_1_1_imaginary,s_2_1_real,s_2_1_imaginary,s_1_2_real,s_1_2_imaginary,s_2_2_real,s_2_2_imaginary\n1e6,0.5,0,0.1,0,0.1,0,0.4,0\n1e9,0.2,0,0.15,0,0.15,0,0.3,0\n",
        )
        .unwrap();
        let scenario = sparameter_scenario();

        let csv = ngspice_sparameter_raw_to_csv(&raw, 2, &scenario).unwrap();

        assert!(csv.starts_with("frequency_hz,reference_impedance_ohm,s11_mag_db"));
        assert!(csv.contains("-6.020599913280e0,0.000000000000e0,5.000000000000e-1"));
        assert!(csv.contains("source_reflection_real"));
    }

    #[test]
    fn ngspice_sparameter_raw_normalizes_repeated_complex_vector_headers() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("s_parameters_raw.csv");
        std::fs::write(
            &raw,
            "frequency s_1_1 s_1_1 s_2_1 s_2_1 s_1_2 s_1_2 s_2_2 s_2_2\n1e6 0.5 0 0.1 0 0.1 0 0.4 0\n1e9 0.2 0 0.15 0 0.15 0 0.3 0\n",
        )
        .unwrap();
        let scenario = sparameter_scenario();

        let csv = ngspice_sparameter_raw_to_csv(&raw, 2, &scenario).unwrap();

        assert!(csv.contains("s21_mag_db"));
        assert!(csv.contains("-2.000000000000e1,0.000000000000e0,1.000000000000e-1"));
    }

    fn sparameter_scenario() -> crate::board_ir::Scenario {
        serde_yaml_ng::from_str(
            "name: two_port_sparameter
type: analog_sparameter
analog:
  backend: ngspice
  netlist_source: generated_from_board
  model_files: []
  node_bindings: []
  pin_bindings: []
  analysis:
    type: sparam
    start_frequency_hz: 1.0e6
    stop_frequency_hz: 1.0e9
    points_per_decade: 20
    s_parameter_source_reflection: { real: 0.2, imaginary: -0.1 }
    s_parameter_ports:
      - { name: p1, positive_node: port1, negative_node: '0', reference_impedance_ohm: 50.0 }
      - { name: p2, positive_node: port2, negative_node: '0', reference_impedance_ohm: 50.0 }
  stimuli: []
  probes: []
  assertions: []
",
        )
        .unwrap()
    }
}
