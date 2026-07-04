use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::analog_dc_runner::{NgspiceDcRun, op_raw_to_operating_point_csv};
use super::analog_noise_runner::{
    NgspiceNoiseRun, noise_spectrum_raw_to_csv, noise_total_raw_to_csv,
};
use super::analog_runner::{
    ModelSectionOverride, NgspiceAcRun, NgspiceRun, NgspiceRunError, ParameterOverride,
    SolverManifestIo, SolverOutput, SolverStatus, WaveformSeries, ac_raw_to_bode_csv,
    detect_nonconvergence, ngspice_error, parse_float, parse_waveform_csv, rewrite_include_line,
    sweep_temperature_c, write_solver_manifest,
};
use super::analog_util::{absolute_path, normalize_path, safe_artifact_name};

pub(super) struct XyceTransientRunOptions<'a, F, C>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    pub(super) output: &'a Path,
    pub(super) run_subdir: Option<&'a str>,
    pub(super) parameter_overrides: &'a [ParameterOverride],
    pub(super) model_section_overrides: &'a [ModelSectionOverride],
    pub(super) operating_probe_expressions: &'a [String],
    pub(super) on_progress: F,
    pub(super) should_cancel: C,
}

pub(super) struct XyceAcRunOptions<'a, F, C>
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

pub(super) struct XyceDcRunOptions<'a, F, C>
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

pub(super) struct XyceNoiseRunOptions<'a, F, C>
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

pub(super) struct XyceSParameterRunOptions<'a, F, C>
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

pub(super) struct XyceSParameterRun {
    pub(super) artifacts: Vec<PathBuf>,
    pub(super) s_parameters: PathBuf,
}

pub(super) fn run_xyce_transient<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: XyceTransientRunOptions<'_, F, C>,
) -> Result<NgspiceRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let XyceTransientRunOptions {
        output,
        run_subdir,
        parameter_overrides,
        model_section_overrides,
        operating_probe_expressions,
        mut on_progress,
        should_cancel,
    } = options;
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before Xyce run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog Xyce run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_xyce.cir");
    let log = run_dir.join("xyce.log");
    let raw = run_dir.join("waveform_raw.csv");
    let waveform = run_dir.join("waveform.csv");
    on_progress(
        "Writing analog Xyce wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_xyce_transient_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("waveform_raw.csv"),
        parameter_overrides,
        model_section_overrides,
        operating_probe_expressions,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog Xyce backend",
        format!(
            "{} transient for {} user probe(s) and {} operating probe(s).",
            backend,
            analog.probes.len(),
            operating_probe_expressions.len()
        ),
    );
    let output = run_xyce_with_timeout(backend, &wrapper, Duration::from_secs(60), should_cancel)
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
            format!("Failed to write Xyce log {}: {error}", log.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "Xyce transient analysis exited with status {}.",
                output.status
            ),
            artifacts,
        ));
    }
    if let Some(reason) = detect_nonconvergence(&log_text) {
        return Err(ngspice_error(
            format!("Xyce reported non-convergence or numerical failure: {reason}."),
            artifacts,
        ));
    }
    if !raw.is_file() {
        return Err(ngspice_error(
            format!(
                "Xyce completed without producing transient export {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    let probe_count = analog.probes.len() + operating_probe_expressions.len();
    let series = xyce_transient_raw_to_waveform_csv(&raw, &waveform, probe_count)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(waveform.clone());
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "transient",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("xyce_transient_raw", &raw)],
        normalized_outputs: &[("transient_waveform", &waveform)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceRun {
        artifacts,
        waveform,
        series,
        user_probe_count: analog.probes.len(),
    })
}

pub(super) fn run_xyce_ac<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: XyceAcRunOptions<'_, F, C>,
) -> Result<NgspiceAcRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let XyceAcRunOptions {
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
        .expect("analog was validated before Xyce AC run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog Xyce AC run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_xyce_ac.cir");
    let log = run_dir.join("xyce_ac.log");
    let raw = run_dir.join("ac_raw.csv");
    let bode = run_dir.join("bode.csv");
    on_progress(
        "Writing analog Xyce AC wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_xyce_ac_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new("ac_raw.csv"),
        parameter_overrides,
        model_section_overrides,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce AC wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog Xyce AC backend",
        format!("{} AC sweep for {} probe(s).", backend, analog.probes.len()),
    );
    let output = run_xyce_with_timeout(backend, &wrapper, Duration::from_secs(60), should_cancel)
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
            format!("Failed to write Xyce AC log {}: {error}", log.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!("Xyce AC analysis exited with status {}.", output.status),
            artifacts,
        ));
    }
    if let Some(reason) = detect_nonconvergence(&log_text) {
        return Err(ngspice_error(
            format!("Xyce reported non-convergence or numerical failure: {reason}."),
            artifacts,
        ));
    }
    if !raw.is_file() {
        return Err(ngspice_error(
            format!(
                "Xyce completed without producing AC export {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    let bode_csv = ac_raw_to_bode_csv(&raw, analog).map_err(|message| {
        ngspice_error(
            format!(
                "Failed to convert Xyce AC response {}: {message}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&bode, bode_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce AC Bode CSV {}: {error}",
                bode.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(bode.clone());
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "ac",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("xyce_ac_raw", &raw)],
        normalized_outputs: &[("ac_bode", &bode)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceAcRun { artifacts, bode })
}

pub(super) fn run_xyce_dc<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: XyceDcRunOptions<'_, F, C>,
) -> Result<NgspiceDcRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let XyceDcRunOptions {
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
        .expect("analog was validated before Xyce DC run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog Xyce DC run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_xyce_op.cir");
    let log = run_dir.join("xyce_op.log");
    let raw = run_dir.join("operating_point_raw.csv");
    let operating_point = run_dir.join("operating_point.csv");
    on_progress(
        "Writing analog Xyce DC wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_xyce_dc_wrapper(
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
                "Failed to write Xyce DC wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog Xyce DC backend",
        format!(
            "{} operating point for {} probe(s).",
            backend,
            analog.probes.len()
        ),
    );
    let output = run_xyce_with_timeout(backend, &wrapper, Duration::from_secs(60), should_cancel)
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
            format!("Failed to write Xyce DC log {}: {error}", log.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "Xyce DC operating-point analysis exited with status {}.",
                output.status
            ),
            artifacts,
        ));
    }
    if let Some(reason) = detect_nonconvergence(&log_text) {
        return Err(ngspice_error(
            format!("Xyce reported non-convergence or numerical failure: {reason}."),
            artifacts,
        ));
    }
    if !raw.is_file() {
        return Err(ngspice_error(
            format!(
                "Xyce completed without producing operating-point export {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    let op_csv = op_raw_to_operating_point_csv(&raw, analog).map_err(|message| {
        ngspice_error(
            format!(
                "Failed to convert Xyce operating point {}: {message}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&operating_point, op_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce operating-point CSV {}: {error}",
                operating_point.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(operating_point.clone());
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
        raw_outputs: &[("xyce_operating_point_raw", &raw)],
        normalized_outputs: &[("operating_point", &operating_point)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceDcRun {
        artifacts,
        operating_point,
    })
}

pub(super) fn run_xyce_noise<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: XyceNoiseRunOptions<'_, F, C>,
) -> Result<NgspiceNoiseRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let XyceNoiseRunOptions {
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
        .expect("analog was validated before Xyce noise run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog Xyce noise run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_xyce_noise.cir");
    let log = run_dir.join("xyce_noise.log");
    let spectrum_raw = run_dir.join("noise_spectrum_raw.csv");
    let total_raw = run_dir.join("noise_total_raw.csv");
    let noise_spectrum = run_dir.join("noise_spectrum.csv");
    let noise_total = run_dir.join("noise_total.csv");
    on_progress(
        "Writing analog Xyce noise wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_xyce_noise_wrapper(
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
                "Failed to write Xyce noise wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog Xyce noise backend",
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
    let output = run_xyce_with_timeout(backend, &wrapper, Duration::from_secs(60), should_cancel)
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
            format!("Failed to write Xyce noise log {}: {error}", log.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!("Xyce noise analysis exited with status {}.", output.status),
            artifacts,
        ));
    }
    if let Some(reason) = detect_nonconvergence(&log_text) {
        return Err(ngspice_error(
            format!("Xyce reported non-convergence or numerical failure: {reason}."),
            artifacts,
        ));
    }
    if !spectrum_raw.is_file() || !total_raw.is_file() {
        return Err(ngspice_error(
            format!(
                "Xyce completed without producing noise exports {} and {}.",
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
                "Failed to convert Xyce noise spectrum {}: {message}",
                spectrum_raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&noise_spectrum, spectrum_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce noise spectrum CSV {}: {error}",
                noise_spectrum.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(noise_spectrum.clone());

    let total_csv = noise_total_raw_to_csv(&total_raw).map_err(|message| {
        ngspice_error(
            format!(
                "Failed to convert Xyce total noise {}: {message}",
                total_raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&noise_total, total_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce total noise CSV {}: {error}",
                noise_total.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(noise_total.clone());
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "noise",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[
            ("xyce_noise_spectrum_raw", &spectrum_raw),
            ("xyce_noise_total_raw", &total_raw),
        ],
        normalized_outputs: &[
            ("noise_spectrum", &noise_spectrum),
            ("noise_total", &noise_total),
        ],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(NgspiceNoiseRun {
        artifacts,
        noise_spectrum,
        noise_total,
    })
}

pub(super) fn run_xyce_sparameter<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    source_netlist: &Path,
    options: XyceSParameterRunOptions<'_, F, C>,
) -> Result<XyceSParameterRun, NgspiceRunError>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let XyceSParameterRunOptions {
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
        .expect("analog was validated before Xyce S-parameter run");
    let mut run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Some(run_subdir) = run_subdir {
        run_dir = run_dir.join(safe_artifact_name(run_subdir));
    }
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog Xyce S-parameter run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = vec![source_netlist.to_path_buf()];
    let wrapper = run_dir.join("circuitci_xyce_sparameter.cir");
    let log = run_dir.join("xyce_sparameter.log");
    let raw_name = format!(
        "s_parameters_raw.s{}p",
        analog.analysis.s_parameter_ports.len()
    );
    let raw = run_dir.join(&raw_name);
    let s_parameters = run_dir.join("s_parameters.csv");
    on_progress(
        "Writing analog Xyce S-parameter wrapper deck",
        format!("Writing {}.", wrapper.to_string_lossy()),
    );
    let wrapper_text = build_xyce_sparameter_wrapper(
        bound,
        scenario,
        source_netlist,
        Path::new(&raw_name),
        parameter_overrides,
        model_section_overrides,
    )
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce S-parameter wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    on_progress(
        "Running analog Xyce S-parameter backend",
        format!(
            "{} S-parameter sweep for {} port(s).",
            backend,
            analog.analysis.s_parameter_ports.len()
        ),
    );
    let output = run_xyce_with_timeout(backend, &wrapper, Duration::from_secs(60), should_cancel)
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
                "Failed to write Xyce S-parameter log {}: {error}",
                log.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "Xyce S-parameter analysis exited with status {}.",
                output.status
            ),
            artifacts,
        ));
    }
    if let Some(reason) = detect_nonconvergence(&log_text) {
        return Err(ngspice_error(
            format!("Xyce reported non-convergence or numerical failure: {reason}."),
            artifacts,
        ));
    }
    if !raw.is_file() {
        return Err(ngspice_error(
            format!(
                "Xyce completed without producing S-parameter export {}.",
                raw.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(raw.clone());
    let s_csv = touchstone_to_sparameter_csv(&raw, analog.analysis.s_parameter_ports.len())
        .map_err(|message| {
            ngspice_error(
                format!(
                    "Failed to convert Xyce S-parameter export {}: {message}",
                    raw.display()
                ),
                artifacts.clone(),
            )
        })?;
    let s_csv = append_sparameter_reflection_metadata(s_csv, scenario).map_err(|message| {
        ngspice_error(
            format!(
                "Failed to annotate Xyce S-parameter CSV {}: {message}",
                raw.display()
            ),
            artifacts.clone(),
        )
    })?;
    fs::write(&s_parameters, s_csv).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write Xyce S-parameter CSV {}: {error}",
                s_parameters.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(s_parameters.clone());
    let manifest = write_solver_manifest(SolverManifestIo {
        run_dir: &run_dir,
        scenario,
        requested_backend: &analog.backend,
        selected_backend: backend,
        analysis_kind: "s_parameter",
        source_netlist,
        wrapper: &wrapper,
        log: &log,
        output: &output,
        parameter_overrides,
        model_section_overrides,
        raw_outputs: &[("xyce_s_parameters_touchstone", &raw)],
        normalized_outputs: &[("s_parameters", &s_parameters)],
    })
    .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(manifest);
    Ok(XyceSParameterRun {
        artifacts,
        s_parameters,
    })
}

fn build_xyce_transient_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    raw_output: &Path,
    parameter_overrides: &[ParameterOverride],
    model_section_overrides: &[ModelSectionOverride],
    operating_probe_expressions: &[String],
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before Xyce wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for Xyce. Do not edit by hand.\n");
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
        if matches!(first_token, ".end" | ".tran" | ".print") {
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
    let step_s = analog.analysis.max_step_us / 1_000_000.0;
    let stop_s = analog.analysis.stop_time_us / 1_000_000.0;
    text.push_str(&format!(".TRAN {:.12e} {:.12e}\n", step_s, stop_s));
    text.push_str(".PRINT TRAN DELIMITER=COMMA FILE=\"");
    text.push_str(&raw_output.to_string_lossy());
    text.push('"');
    for probe in &analog.probes {
        text.push(' ');
        text.push_str(&probe.expression);
    }
    for expression in operating_probe_expressions {
        text.push(' ');
        text.push_str(expression);
    }
    text.push_str("\n.END\n");
    Ok(text)
}

fn build_xyce_ac_wrapper(
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
        .expect("analog was validated before Xyce AC wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let start_hz = analog.analysis.start_frequency_hz.ok_or_else(|| {
        "analog.analysis.start_frequency_hz is required for AC analysis.".to_string()
    })?;
    let stop_hz = analog.analysis.stop_frequency_hz.ok_or_else(|| {
        "analog.analysis.stop_frequency_hz is required for AC analysis.".to_string()
    })?;
    let points_per_decade = analog.analysis.points_per_decade.unwrap_or(20);
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for Xyce AC. Do not edit by hand.\n");
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
        if matches!(first_token, ".end" | ".tran" | ".ac" | ".print") {
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
    text.push_str(&format!(
        ".AC DEC {} {:.12e} {:.12e}\n",
        points_per_decade, start_hz, stop_hz
    ));
    text.push_str(".PRINT AC DELIMITER=COMMA FILE=\"");
    text.push_str(&raw_output.to_string_lossy());
    text.push('"');
    for probe in &analog.probes {
        text.push(' ');
        text.push_str(&probe.expression);
    }
    text.push_str("\n.END\n");
    Ok(text)
}

fn build_xyce_dc_wrapper(
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
        .expect("analog was validated before Xyce DC wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for Xyce operating point. Do not edit by hand.\n");
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
        if matches!(first_token, ".end" | ".tran" | ".ac" | ".op" | ".print") {
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
    text.push_str(".OP\n");
    text.push_str(".PRINT DC DELIMITER=COMMA FILE=\"");
    text.push_str(&raw_output.to_string_lossy());
    text.push('"');
    for probe in &analog.probes {
        text.push(' ');
        text.push_str(&probe.expression);
    }
    text.push_str("\n.END\n");
    Ok(text)
}

fn build_xyce_noise_wrapper(
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
        .expect("analog was validated before Xyce noise wrapper generation");
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
    text.push_str("* Generated by CircuitCI for Xyce noise. Do not edit by hand.\n");
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
            ".end" | ".tran" | ".ac" | ".op" | ".noise" | ".print"
        ) {
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
        format!("V({output_node},{reference_node})")
    } else {
        format!("V({output_node})")
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
    text.push_str(&format!(
        ".NOISE {output_expr} {input_source} DEC {points} {start_hz:.12e} {stop_hz:.12e}\n"
    ));
    text.push_str(".PRINT NOISE DELIMITER=COMMA FILE=\"");
    text.push_str(&spectrum_raw_output.to_string_lossy());
    text.push_str("\" ONOISE INOISE\n");
    text.push_str(".PRINT NOISE DELIMITER=COMMA FILE=\"");
    text.push_str(&total_raw_output.to_string_lossy());
    text.push_str("\" ONOISE_TOTAL INOISE_TOTAL\n.END\n");
    Ok(text)
}

fn build_xyce_sparameter_wrapper(
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
        .expect("analog was validated before Xyce S-parameter wrapper generation");
    let source = fs::read_to_string(netlist).map_err(|error| {
        format!(
            "Failed to read SPICE netlist {}: {error}",
            netlist.display()
        )
    })?;
    let start_hz = analog.analysis.start_frequency_hz.ok_or_else(|| {
        "analog.analysis.start_frequency_hz is required for S-parameter analysis.".to_string()
    })?;
    let stop_hz = analog.analysis.stop_frequency_hz.ok_or_else(|| {
        "analog.analysis.stop_frequency_hz is required for S-parameter analysis.".to_string()
    })?;
    let points_per_decade = analog.analysis.points_per_decade.unwrap_or(20);
    let mut text = String::new();
    text.push_str("* Generated by CircuitCI for Xyce S-parameter. Do not edit by hand.\n");
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
            ".end" | ".tran" | ".ac" | ".op" | ".noise" | ".lin" | ".print"
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
    for (index, port) in analog.analysis.s_parameter_ports.iter().enumerate() {
        text.push_str(&format!(
            "P{} {} {} port={} z0={:.12e}\n",
            index + 1,
            port.positive_node,
            port.negative_node,
            index + 1,
            port.reference_impedance_ohm
        ));
    }
    text.push_str(&format!(
        ".AC DEC {} {:.12e} {:.12e}\n",
        points_per_decade, start_hz, stop_hz
    ));
    text.push_str(".LIN FORMAT=TOUCHSTONE DATAFORMAT=RI SPARCALC=1 FILE=\"");
    text.push_str(&raw_output.to_string_lossy());
    text.push_str("\"\n.END\n");
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

pub(super) fn run_xyce_with_timeout(
    backend: &str,
    wrapper: &Path,
    timeout: Duration,
    should_cancel: impl Fn() -> bool,
) -> Result<SolverOutput, String> {
    let working_dir = wrapper
        .parent()
        .ok_or_else(|| format!("Xyce wrapper path {} has no parent.", wrapper.display()))?;
    let deck_name = wrapper
        .file_name()
        .ok_or_else(|| format!("Xyce wrapper path {} has no filename.", wrapper.display()))?;
    let mut child = Command::new(backend)
        .current_dir(working_dir)
        .arg(deck_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to launch Xyce backend {backend}: {error}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect Xyce output: {error}"))?;
                return Ok(SolverOutput {
                    status: SolverStatus::External(output.status),
                    command: format!(
                        "cd {} && {backend} {}",
                        working_dir.to_string_lossy(),
                        deck_name.to_string_lossy()
                    ),
                    stdout: output.stdout,
                    stderr: output.stderr,
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect timed-out Xyce output: {error}"))?;
                return Err(format!(
                    "Xyce transient analysis exceeded {} seconds and was terminated. Stdout bytes: {}, stderr bytes: {}.",
                    timeout.as_secs(),
                    output.stdout.len(),
                    output.stderr.len()
                ));
            }
            Ok(None) if should_cancel() => {
                let _ = child.kill();
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect canceled Xyce output: {error}"))?;
                return Err(format!(
                    "Xyce transient analysis was canceled and the external process was terminated. Stdout bytes: {}, stderr bytes: {}.",
                    output.stdout.len(),
                    output.stderr.len()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(format!("Failed while waiting for Xyce: {error}")),
        }
    }
}

fn xyce_transient_raw_to_waveform_csv(
    raw: &Path,
    waveform: &Path,
    probe_count: usize,
) -> Result<WaveformSeries, String> {
    let text = fs::read_to_string(raw).map_err(|error| {
        format!(
            "Failed to read Xyce transient export {}: {error}",
            raw.display()
        )
    })?;
    let mut rows = Vec::new();
    let mut time_col = None;
    let mut value_start_col = None;
    for line in text.lines() {
        let fields = split_xyce_row(line);
        if fields.is_empty() {
            continue;
        }
        if fields
            .iter()
            .any(|field| field.eq_ignore_ascii_case("time"))
        {
            if fields[0].eq_ignore_ascii_case("index") && fields.len() >= probe_count + 2 {
                time_col = Some(1);
                value_start_col = Some(2);
            } else {
                time_col = Some(0);
                value_start_col = Some(1);
            }
            continue;
        }
        let numeric: Vec<_> = fields
            .iter()
            .filter_map(|field| parse_float(field))
            .collect();
        if numeric.is_empty() {
            continue;
        }
        let (time_index, value_index) = match (time_col, value_start_col) {
            (Some(time), Some(value)) => (time, value),
            _ if numeric.len() == probe_count + 2 => (1, 2),
            _ => (0, 1),
        };
        if numeric.len() < value_index + probe_count {
            return Err(format!(
                "Xyce transient row has {} numeric column(s), expected at least {} for {} probe(s).",
                numeric.len(),
                value_index + probe_count,
                probe_count
            ));
        }
        let mut row = Vec::with_capacity(probe_count + 1);
        row.push(numeric[time_index]);
        row.extend_from_slice(&numeric[value_index..value_index + probe_count]);
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(format!(
            "Xyce transient export {} has no numeric samples.",
            raw.display()
        ));
    }
    let mut output = String::new();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&format!("{value:.12e}"));
        }
        output.push('\n');
    }
    fs::write(waveform, output).map_err(|error| {
        format!(
            "Failed to write normalized Xyce waveform {}: {error}",
            waveform.display()
        )
    })?;
    parse_waveform_csv(waveform, probe_count)
}

fn split_xyce_row(line: &str) -> Vec<&str> {
    line.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

fn touchstone_to_sparameter_csv(raw: &Path, port_count: usize) -> Result<String, String> {
    if port_count == 0 {
        return Err("S-parameter export requires at least one port.".to_string());
    }
    let text = fs::read_to_string(raw).map_err(|error| {
        format!(
            "Failed to read Xyce Touchstone export {}: {error}",
            raw.display()
        )
    })?;
    let expected_values = port_count * port_count * 2;
    let pairs = touchstone_pairs(port_count);
    let mut frequency_scale = 1.0;
    let mut data_format = "ri".to_string();
    let mut reference_impedance_ohm = 50.0;
    let mut numbers = Vec::new();
    for line in text.lines() {
        let content = line.split('!').next().unwrap_or("").trim();
        if content.is_empty() {
            continue;
        }
        if let Some(option) = content.strip_prefix('#') {
            let tokens: Vec<_> = option.split_whitespace().collect();
            if let Some(unit) = tokens.first() {
                frequency_scale = match unit.to_ascii_lowercase().as_str() {
                    "hz" => 1.0,
                    "khz" => 1.0e3,
                    "mhz" => 1.0e6,
                    "ghz" => 1.0e9,
                    other => {
                        return Err(format!(
                            "Unsupported Touchstone frequency unit {other}; expected Hz, kHz, MHz, or GHz."
                        ));
                    }
                };
            }
            for token in &tokens {
                let lower = token.to_ascii_lowercase();
                if matches!(lower.as_str(), "ri" | "ma" | "db") {
                    data_format = lower;
                }
            }
            for pair in tokens.windows(2) {
                if pair[0].eq_ignore_ascii_case("r") {
                    reference_impedance_ohm = parse_float(pair[1]).ok_or_else(|| {
                        format!(
                            "Touchstone option line declares non-numeric reference impedance {}.",
                            pair[1]
                        )
                    })?;
                    if !reference_impedance_ohm.is_finite() || reference_impedance_ohm <= 0.0 {
                        return Err(format!(
                            "Touchstone reference impedance must be finite and positive, got {reference_impedance_ohm}."
                        ));
                    }
                }
            }
            continue;
        }
        numbers.extend(split_xyce_row(content).into_iter().filter_map(parse_float));
    }
    let row_width = 1 + expected_values;
    if numbers.len() < row_width {
        return Err(format!(
            "Touchstone export has {} numeric value(s), expected at least {} for {} port(s).",
            numbers.len(),
            row_width,
            port_count
        ));
    }
    if numbers.len() % row_width != 0 {
        return Err(format!(
            "Touchstone export has {} numeric value(s), which is not an integer number of {}-column S-parameter row(s).",
            numbers.len(),
            row_width
        ));
    }

    let mut output = String::from("frequency_hz,reference_impedance_ohm");
    for (row, column) in &pairs {
        output.push_str(&format!(
            ",s{row}{column}_mag_db,s{row}{column}_phase_deg,s{row}{column}_mag_linear"
        ));
    }
    output.push('\n');
    for chunk in numbers.chunks(row_width) {
        let frequency_hz = chunk[0] * frequency_scale;
        output.push_str(&format!(
            "{frequency_hz:.12e},{reference_impedance_ohm:.12e}"
        ));
        for (index, _) in pairs.iter().enumerate() {
            let first = chunk[1 + index * 2];
            let second = chunk[1 + index * 2 + 1];
            let (mag_linear, phase_deg, mag_db) = match data_format.as_str() {
                "ri" => {
                    let mag = first.hypot(second);
                    let phase = second.atan2(first).to_degrees();
                    let db = if mag > 0.0 {
                        20.0 * mag.log10()
                    } else {
                        -300.0
                    };
                    (mag, phase, db)
                }
                "ma" => {
                    let mag = first;
                    let db = if mag > 0.0 {
                        20.0 * mag.log10()
                    } else {
                        -300.0
                    };
                    (mag, second, db)
                }
                "db" => (10.0_f64.powf(first / 20.0), second, first),
                other => {
                    return Err(format!(
                        "Unsupported Touchstone data format {other}; expected RI, MA, or DB."
                    ));
                }
            };
            output.push_str(&format!(
                ",{mag_db:.12e},{phase_deg:.12e},{mag_linear:.12e}"
            ));
        }
        output.push('\n');
    }
    Ok(output)
}

pub(super) fn append_sparameter_reflection_metadata(
    csv: String,
    scenario: &Scenario,
) -> Result<String, String> {
    let Some(analog) = scenario.analog.as_ref() else {
        return Ok(csv);
    };
    let mut requested_columns = Vec::new();
    if let Some(source) = analog.analysis.s_parameter_source_reflection {
        push_reflection_metadata_columns(&mut requested_columns, "source", source)?;
    }
    if let Some(load) = analog.analysis.s_parameter_load_reflection {
        push_reflection_metadata_columns(&mut requested_columns, "load", load)?;
    }
    if requested_columns.is_empty() {
        return Ok(csv);
    }
    let mut lines = csv.lines();
    let Some(header) = lines.next() else {
        return Ok(csv);
    };
    let header_fields: Vec<_> = header.split(',').map(str::trim).collect();
    let columns_to_add: Vec<_> = requested_columns
        .iter()
        .filter(|(name, _)| !header_fields.contains(name))
        .copied()
        .collect();
    if columns_to_add.is_empty() {
        return Ok(csv);
    }
    let mut annotated = String::new();
    annotated.push_str(header);
    for (name, _) in &columns_to_add {
        annotated.push(',');
        annotated.push_str(name);
    }
    annotated.push('\n');
    for line in lines {
        if line.trim().is_empty() {
            annotated.push('\n');
            continue;
        }
        annotated.push_str(line);
        for (_, value) in &columns_to_add {
            annotated.push(',');
            annotated.push_str(&format!("{value:.12e}"));
        }
        annotated.push('\n');
    }
    Ok(annotated)
}

fn push_reflection_metadata_columns(
    columns: &mut Vec<(&'static str, f64)>,
    role: &str,
    coefficient: crate::board_ir::AnalogSParameterReflectionCoefficient,
) -> Result<(), String> {
    if !coefficient.real.is_finite() || !coefficient.imaginary.is_finite() {
        return Err(format!(
            "S-parameter {role} reflection metadata must be finite."
        ));
    }
    if coefficient.real.mul_add(
        coefficient.real,
        coefficient.imaginary * coefficient.imaginary,
    ) >= 1.0
    {
        return Err(format!(
            "S-parameter {role} reflection metadata magnitude must be below 1."
        ));
    }
    match role {
        "source" => {
            columns.push(("source_reflection_real", coefficient.real));
            columns.push(("source_reflection_imaginary", coefficient.imaginary));
        }
        "load" => {
            columns.push(("load_reflection_real", coefficient.real));
            columns.push(("load_reflection_imaginary", coefficient.imaginary));
        }
        _ => unreachable!("reflection metadata role is fixed by caller"),
    }
    Ok(())
}

fn touchstone_pairs(port_count: usize) -> Vec<(usize, usize)> {
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

#[cfg(test)]
mod tests {
    use super::{
        append_sparameter_reflection_metadata, touchstone_to_sparameter_csv,
        xyce_transient_raw_to_waveform_csv,
    };

    #[test]
    fn xyce_transient_raw_csv_normalizes_to_waveform_csv() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("waveform_raw.csv");
        let waveform = dir.path().join("waveform.csv");
        std::fs::write(&raw, "TIME,V(out)\n0,0\n5e-6,1.2\n1e-5,1.3\n").unwrap();

        let series = xyce_transient_raw_to_waveform_csv(&raw, &waveform, 1).unwrap();

        assert_eq!(series.time_s.len(), 3);
        assert_eq!(series.values_by_probe[0][1], 1.2);
        assert!(
            std::fs::read_to_string(waveform)
                .unwrap()
                .contains("5.000000000000e-6")
        );
    }

    #[test]
    fn touchstone_to_sparameter_csv_normalizes_two_port_ri() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("s_parameters_raw.s2p");
        std::fs::write(
            &raw,
            "# Hz S RI R 50\n1.0e6 0.5 0.0 2.0 0.0 0.01 0.0 0.4 0.0\n",
        )
        .unwrap();

        let csv = touchstone_to_sparameter_csv(&raw, 2).unwrap();

        assert!(csv.starts_with(
            "frequency_hz,reference_impedance_ohm,s11_mag_db,s11_phase_deg,s11_mag_linear,s21_mag_db"
        ));
        assert!(csv.contains("1.000000000000e6,5.000000000000e1,-6.020599913280e0"));
        assert!(csv.contains("6.020599913280e0,0.000000000000e0,2.000000000000e0"));
    }

    #[test]
    fn sparameter_csv_reflection_metadata_annotation_adds_source_load_columns() {
        let scenario: crate::board_ir::Scenario = serde_yaml_ng::from_str(
            r#"
name: two_port_sparameter
type: analog_sparameter
analog:
  backend: xyce
  model_files: []
  node_bindings: []
  pin_bindings: []
  analysis:
    type: sparam
    s_parameter_source_reflection: { real: 0.2, imaginary: -0.1 }
    s_parameter_load_reflection: { real: -0.15, imaginary: 0.05 }
  stimuli: []
  probes: []
  assertions: []
"#,
        )
        .unwrap();
        let csv = concat!(
            "frequency_hz,reference_impedance_ohm,s11_mag_db\n",
            "1.000000000000e6,5.000000000000e1,-6.020599913280e0\n",
        )
        .to_string();

        let annotated = append_sparameter_reflection_metadata(csv, &scenario).unwrap();

        assert!(annotated.contains(
            "source_reflection_real,source_reflection_imaginary,load_reflection_real,load_reflection_imaginary"
        ));
        assert!(annotated.contains(
            "1.000000000000e6,5.000000000000e1,-6.020599913280e0,2.000000000000e-1,-1.000000000000e-1,-1.500000000000e-1,5.000000000000e-2"
        ));
    }
}
