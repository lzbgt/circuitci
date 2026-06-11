use crate::board_ir::{
    AnalogAggregation, AnalogAssertion, AnalogBackend, AnalogNetlistSource, AnalogProbe,
    AnalogQuantity, AnalogRelation, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::SPICE_TRANSIENT_ANALYSIS;
use super::common::validation_input_missing;
use super::spice_netlist::generate_board_netlist;

pub(super) fn validate_spice_transient(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    artifacts: &mut Vec<String>,
    waveforms: &mut Vec<String>,
    output: &Path,
) {
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_transient scenario requires an analog block.",
        );
        return;
    };

    if let Some(finding) = validate_netlist_source(bound, scenario, artifacts) {
        findings.push(finding);
        return;
    }

    if analog.model_files.is_empty()
        || analog.node_bindings.is_empty()
        || analog.pin_bindings.is_empty()
    {
        validation_input_missing(
            findings,
            scenario,
            "analog_transient requires model_files, node_bindings, and pin_bindings.",
        );
        return;
    }
    for model_file in &analog.model_files {
        let path = bound.project.source_dir.join(&model_file.path);
        if !path.is_file() {
            let mut finding = Finding::critical(
                "ANALOG_MODEL_UNAVAILABLE",
                &scenario.name,
                format!(
                    "SPICE model file {} is required for physical analog simulation.",
                    path.display()
                ),
            );
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_model_file"));
            finding.suggested_fixes.push(
                "Add sourced or bench-calibrated SPICE model files for the simulated devices."
                    .to_string(),
            );
            findings.push(finding);
            return;
        }
        if let Some(expected) = &model_file.sha256 {
            match file_sha256_hex(&path) {
                Ok(actual) if actual.eq_ignore_ascii_case(expected) => {}
                Ok(actual) => {
                    let mut finding = Finding::critical(
                        "ANALOG_MODEL_HASH_MISMATCH",
                        &scenario.name,
                        format!(
                            "SPICE model file {} does not match the declared SHA-256.",
                            path.display()
                        ),
                    );
                    finding.measured.insert("sha256".to_string(), json!(actual));
                    finding
                        .limit
                        .insert("expected_sha256".to_string(), json!(expected));
                    finding.suggested_fixes.push(
                        "Update the model file provenance or use the exact model artifact declared by the scenario.".to_string(),
                    );
                    findings.push(finding);
                    return;
                }
                Err(message) => {
                    validation_input_missing(findings, scenario, message);
                    return;
                }
            }
        }
        push_artifact(artifacts, &path);
    }
    let mut bound_nodes = BTreeSet::new();
    for binding in &analog.node_bindings {
        if !bound.project.board.nets.contains_key(&binding.net) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog node binding {} references unknown board net {}.",
                    binding.node, binding.net
                ),
            );
            return;
        }
        bound_nodes.insert(binding.node.as_str());
    }
    for binding in &analog.pin_bindings {
        if !bound_nodes.contains(binding.node.as_str()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog pin binding references unbound SPICE node {}.",
                    binding.node
                ),
            );
            return;
        }
        let Some(component) = bound
            .project
            .board
            .components
            .get(&binding.endpoint.component)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog pin binding references unknown component {}.",
                    binding.endpoint.component
                ),
            );
            return;
        };
        let Some(pin_net) = component.pins.get(&binding.endpoint.pin) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog pin binding references unknown pin {}.{}.",
                    binding.endpoint.component, binding.endpoint.pin
                ),
            );
            return;
        };
        if !analog
            .node_bindings
            .iter()
            .any(|node| node.node == binding.node && node.net == *pin_net)
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog pin binding {}.{} maps to node {}, but the board pin is on net {}.",
                    binding.endpoint.component, binding.endpoint.pin, binding.node, pin_net
                ),
            );
            return;
        }
    }

    if analog.analysis.analysis_type != "tran" {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Unsupported analog analysis type {}; only tran is accepted for this check.",
                analog.analysis.analysis_type
            ),
        );
        return;
    }
    if !analog.analysis.stop_time_us.is_finite()
        || !analog.analysis.max_step_us.is_finite()
        || analog.analysis.stop_time_us <= 0.0
        || analog.analysis.max_step_us <= 0.0
        || analog.analysis.max_step_us > analog.analysis.stop_time_us
    {
        validation_input_missing(
            findings,
            scenario,
            "analog.analysis stop_time_us and max_step_us must be finite, positive, and max_step_us must not exceed stop_time_us.",
        );
        return;
    }
    if analog.probes.is_empty() || analog.assertions.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SPICE_TRANSIENT_ANALYSIS requires probes and quantitative waveform assertions.",
        );
        return;
    }
    for probe in &analog.probes {
        if let Err(message) = validate_probe_contract(probe) {
            validation_input_missing(
                findings,
                scenario,
                format!("Analog probe {} {message}.", probe.name),
            );
            return;
        }
    }
    for assertion in &analog.assertions {
        if !analog
            .probes
            .iter()
            .any(|probe| probe.name == assertion.probe)
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog assertion {} references unknown probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            return;
        }
        if let Err(message) = validate_assertion_contract(assertion, analog.analysis.stop_time_us) {
            validation_input_missing(
                findings,
                scenario,
                format!("Analog assertion {} {message}.", assertion.name),
            );
            return;
        }
        if threshold_count(assertion) != 1 {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog assertion {} must declare exactly one finite threshold unit.",
                    assertion.name
                ),
            );
            return;
        }
        let probe = analog
            .probes
            .iter()
            .find(|probe| probe.name == assertion.probe)
            .expect("probe existence was checked above");
        if threshold_for(assertion, probe).is_none() {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog assertion {} is missing a finite threshold for {} probe {}.",
                    assertion.name,
                    quantity_name(&probe.quantity),
                    probe.name
                ),
            );
            return;
        }
        match assertion.relation {
            AnalogRelation::Below | AnalogRelation::Above => {}
        }
    }

    let selected = select_backend(&analog.backend);
    let BackendSelection::Selected(backend) = selected else {
        let mut finding = match selected {
            BackendSelection::EmbeddedUnavailable => embedded_solver_unavailable(&scenario.name),
            BackendSelection::Unavailable => {
                external_backend_unavailable(&scenario.name, &analog.backend)
            }
            BackendSelection::Selected(_) => unreachable!("handled by let-else pattern"),
        };
        finding.measured.insert(
            "requested_backend".to_string(),
            json!(backend_name(&analog.backend)),
        );
        findings.push(finding);
        return;
    };

    if backend != "ngspice" {
        let mut finding = Finding::critical(
            SPICE_TRANSIENT_ANALYSIS,
            &scenario.name,
            format!(
                "Backend {backend} was detected, but only external ngspice execution is implemented in this runtime slice."
            ),
        );
        finding
            .measured
            .insert("selected_backend".to_string(), json!(backend));
        finding
            .limit
            .insert("implemented_backend".to_string(), json!("ngspice"));
        findings.push(finding);
        return;
    }

    match run_ngspice(bound, scenario, backend, output) {
        Ok(run) => {
            for artifact in &run.artifacts {
                push_artifact(artifacts, artifact);
            }
            push_artifact(waveforms, &run.waveform);
            evaluate_waveform_assertions(scenario, &run, findings);
        }
        Err(error) => {
            for artifact in &error.artifacts {
                push_artifact(artifacts, artifact);
            }
            let mut finding =
                Finding::critical(SPICE_TRANSIENT_ANALYSIS, &scenario.name, error.message);
            finding
                .measured
                .insert("selected_backend".to_string(), json!(backend));
            finding.limit.insert(
                "required_evidence".to_string(),
                json!("ngspice_waveform_csv"),
            );
            finding.suggested_fixes.push(
                "Inspect the generated ngspice wrapper deck and solver log artifacts.".to_string(),
            );
            findings.push(finding);
        }
    }
}

struct NgspiceRun {
    artifacts: Vec<PathBuf>,
    waveform: PathBuf,
    series: WaveformSeries,
}

#[derive(Debug)]
struct WaveformSeries {
    time_s: Vec<f64>,
    values_by_probe: Vec<Vec<f64>>,
}

struct NgspiceRunError {
    message: String,
    artifacts: Vec<PathBuf>,
}

fn run_ngspice(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    backend: &str,
    output: &Path,
) -> Result<NgspiceRun, NgspiceRunError> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before run");
    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    fs::create_dir_all(&run_dir).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to create analog run directory {}: {error}",
                run_dir.display()
            ),
            Vec::new(),
        )
    })?;
    let mut artifacts = Vec::new();
    let wrapper = run_dir.join("circuitci_ngspice.cir");
    let log = run_dir.join("ngspice.log");
    let waveform = run_dir.join("waveform.csv");
    let source_netlist = prepare_source_netlist(bound, scenario, &run_dir)
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    artifacts.push(source_netlist.clone());
    let wrapper_text =
        build_ngspice_wrapper(bound, scenario, &source_netlist, Path::new("waveform.csv"))
            .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    fs::write(&wrapper, wrapper_text).map_err(|error| {
        ngspice_error(
            format!(
                "Failed to write ngspice wrapper deck {}: {error}",
                wrapper.display()
            ),
            artifacts.clone(),
        )
    })?;
    artifacts.push(wrapper.clone());

    let output = run_solver_with_timeout(backend, &wrapper, Duration::from_secs(60))
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    let mut log_text = String::new();
    log_text.push_str("COMMAND: ");
    log_text.push_str("cd ");
    log_text.push_str(
        &wrapper
            .parent()
            .map(Path::to_string_lossy)
            .unwrap_or_default(),
    );
    log_text.push_str(" && ");
    log_text.push_str(backend);
    log_text.push_str(" -b ");
    log_text.push_str(
        &wrapper
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default(),
    );
    log_text.push_str("\n\nSTDOUT:\n");
    log_text.push_str(&String::from_utf8_lossy(&output.stdout));
    log_text.push_str("\n\nSTDERR:\n");
    log_text.push_str(&String::from_utf8_lossy(&output.stderr));
    fs::write(&log, &log_text).map_err(|error| {
        ngspice_error(
            format!("Failed to write ngspice log {}: {error}", log.display()),
            artifacts.clone(),
        )
    })?;
    artifacts.push(log.clone());
    if !output.status.success() {
        return Err(ngspice_error(
            format!(
                "ngspice transient analysis exited with status {}.",
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
    if !waveform.is_file() {
        return Err(ngspice_error(
            format!(
                "ngspice completed without producing waveform export {}.",
                waveform.display()
            ),
            artifacts,
        ));
    }
    artifacts.push(waveform.clone());
    let series = parse_waveform_csv(&waveform, analog.probes.len())
        .map_err(|message| ngspice_error(message, artifacts.clone()))?;
    Ok(NgspiceRun {
        artifacts,
        waveform,
        series,
    })
}

struct SolverOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_solver_with_timeout(
    backend: &str,
    wrapper: &Path,
    timeout: Duration,
) -> Result<SolverOutput, String> {
    let working_dir = wrapper
        .parent()
        .ok_or_else(|| format!("ngspice wrapper path {} has no parent.", wrapper.display()))?;
    let deck_name = wrapper.file_name().ok_or_else(|| {
        format!(
            "ngspice wrapper path {} has no filename.",
            wrapper.display()
        )
    })?;
    let mut child = Command::new(backend)
        .current_dir(working_dir)
        .arg("-b")
        .arg(deck_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to launch ngspice backend {backend}: {error}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("Failed to collect ngspice output: {error}"))?;
                return Ok(SolverOutput {
                    status: output.status,
                    stdout: output.stdout,
                    stderr: output.stderr,
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let output = child.wait_with_output().map_err(|error| {
                    format!("Failed to collect timed-out ngspice output: {error}")
                })?;
                return Err(format!(
                    "ngspice transient analysis exceeded {} seconds and was terminated. Stdout bytes: {}, stderr bytes: {}.",
                    timeout.as_secs(),
                    output.stdout.len(),
                    output.stderr.len()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(format!("Failed while waiting for ngspice: {error}")),
        }
    }
}

fn ngspice_error(message: impl Into<String>, artifacts: Vec<PathBuf>) -> NgspiceRunError {
    NgspiceRunError {
        message: message.into(),
        artifacts,
    }
}

fn validate_netlist_source(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    artifacts: &mut Vec<String>,
) -> Option<Finding> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before netlist source validation");
    match analog.netlist_source {
        AnalogNetlistSource::File => {
            let Some(netlist) = &analog.netlist else {
                let mut finding = Finding::critical(
                    "ANALOG_NETLIST_UNAVAILABLE",
                    &scenario.name,
                    "analog.netlist is required when analog.netlist_source is file.",
                );
                finding
                    .limit
                    .insert("required_artifact".to_string(), json!("spice_netlist"));
                return Some(finding);
            };
            let netlist = bound.project.source_dir.join(netlist);
            if !netlist.is_file() {
                let mut finding = Finding::critical(
                    "ANALOG_NETLIST_UNAVAILABLE",
                    &scenario.name,
                    format!(
                        "SPICE netlist {} is required for physical analog simulation.",
                        netlist.display()
                    ),
                );
                finding
                    .limit
                    .insert("required_artifact".to_string(), json!("spice_netlist"));
                finding.suggested_fixes.push(
                    "Add a SPICE-compatible deck with device models for this board region."
                        .to_string(),
                );
                return Some(finding);
            }
            push_artifact(artifacts, &netlist);
            None
        }
        AnalogNetlistSource::GeneratedFromBoard => {
            if analog.generated.is_none() {
                return Some(Finding::critical(
                    "ANALOG_NETLIST_UNAVAILABLE",
                    &scenario.name,
                    "analog.generated is required when analog.netlist_source is generated_from_board.",
                ));
            }
            None
        }
    }
}

fn prepare_source_netlist(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    run_dir: &Path,
) -> Result<PathBuf, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before source netlist preparation");
    match analog.netlist_source {
        AnalogNetlistSource::File => {
            let netlist = analog
                .netlist
                .as_ref()
                .ok_or_else(|| "analog.netlist is required for file netlist source.".to_string())?;
            Ok(bound.project.source_dir.join(netlist))
        }
        AnalogNetlistSource::GeneratedFromBoard => {
            let path = run_dir.join("generated_board.cir");
            generate_board_netlist(bound, analog, &path)?;
            Ok(path)
        }
    }
}

fn detect_nonconvergence(log: &str) -> Option<&'static str> {
    let lower = log.to_ascii_lowercase();
    for (pattern, reason) in [
        ("timestep too small", "timestep too small"),
        ("singular matrix", "singular matrix"),
        ("convergence problem", "convergence problem"),
        (
            "doanalyses: iteration limit reached",
            "iteration limit reached",
        ),
        ("tran simulation(s) aborted", "transient simulation aborted"),
    ] {
        if lower.contains(pattern) {
            return Some(reason);
        }
    }
    None
}

fn build_ngspice_wrapper(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    netlist: &Path,
    waveform: &Path,
) -> Result<String, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before wrapper generation");
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
        if matches!(first_token, ".end" | ".tran") {
            continue;
        }
        text.push_str(&rewrite_include_line(line, include_base));
        text.push('\n');
    }
    let step_s = analog.analysis.max_step_us / 1_000_000.0;
    let stop_s = analog.analysis.stop_time_us / 1_000_000.0;
    text.push_str(".control\n");
    text.push_str("set wr_vecnames\n");
    text.push_str("set wr_singlescale\n");
    text.push_str(&format!("tran {:.12e} {:.12e}\n", step_s, stop_s));
    text.push_str("wrdata ");
    text.push_str(&waveform.to_string_lossy());
    for probe in &analog.probes {
        text.push(' ');
        text.push_str(&probe.expression);
    }
    text.push_str("\nquit\n.endc\n.end\n");
    Ok(text)
}

fn rewrite_include_line(line: &str, source_dir: &Path) -> String {
    let trimmed = line.trim_start();
    let lowercase = trimmed.to_ascii_lowercase();
    if !lowercase.starts_with(".include ") && !lowercase.starts_with(".lib ") {
        return line.to_string();
    }
    let Some((directive, rest)) = trimmed.split_once(char::is_whitespace) else {
        return line.to_string();
    };
    let path_text = rest.trim();
    let quote = if path_text.starts_with('"') {
        Some('"')
    } else if path_text.starts_with('\'') {
        Some('\'')
    } else {
        None
    };
    let path_token = quote
        .and_then(|mark| path_text[1..].split_once(mark).map(|(path, _)| path))
        .unwrap_or_else(|| path_text.split_whitespace().next().unwrap_or(path_text));
    let path = Path::new(path_token);
    if path.is_absolute() {
        return line.to_string();
    }
    let absolute = absolute_path(&source_dir.join(path))
        .unwrap_or_else(|_| normalize_path(&source_dir.join(path)));
    let indent_len = line.len() - trimmed.len();
    let indent = &line[..indent_len];
    format!("{indent}{directive} \"{}\"", absolute.to_string_lossy())
}

fn parse_waveform_csv(path: &Path, probe_count: usize) -> Result<WaveformSeries, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read waveform export {}: {error}", path.display()))?;
    let mut time_s = Vec::new();
    let mut values_by_probe = vec![Vec::new(); probe_count];
    for (index, line) in text.lines().enumerate() {
        let fields: Vec<_> = line
            .split(|character: char| character == ',' || character.is_whitespace())
            .filter(|field| !field.is_empty())
            .collect();
        if fields.is_empty() {
            continue;
        }
        let Some(time) = parse_float(fields[0]) else {
            if index == 0 {
                continue;
            }
            return Err(format!(
                "Waveform row {} in {} has non-numeric time value {}.",
                index + 1,
                path.display(),
                fields[0]
            ));
        };
        if fields.len() < probe_count + 1 {
            return Err(format!(
                "Waveform row {} in {} has {} columns, expected at least {}.",
                index + 1,
                path.display(),
                fields.len(),
                probe_count + 1
            ));
        }
        if time_s.last().is_some_and(|previous| time <= *previous) {
            return Err(format!(
                "Waveform row {} in {} has non-increasing time value {}.",
                index + 1,
                path.display(),
                fields[0]
            ));
        }
        time_s.push(time);
        for probe_index in 0..probe_count {
            let value = parse_float(fields[probe_index + 1]).ok_or_else(|| {
                format!(
                    "Waveform row {} in {} has non-numeric probe value {}.",
                    index + 1,
                    path.display(),
                    fields[probe_index + 1]
                )
            })?;
            values_by_probe[probe_index].push(value);
        }
    }
    if time_s.is_empty() {
        return Err(format!(
            "Waveform export {} has no numeric samples.",
            path.display()
        ));
    }
    Ok(WaveformSeries {
        time_s,
        values_by_probe,
    })
}

fn parse_float(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

struct AssertionThreshold {
    value: f64,
    unit: &'static str,
    limit_key: &'static str,
}

fn evaluate_waveform_assertions(
    scenario: &Scenario,
    run: &NgspiceRun,
    findings: &mut Vec<Finding>,
) {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before assertion evaluation");
    for assertion in &analog.assertions {
        let Some(probe_index) = analog
            .probes
            .iter()
            .position(|probe| probe.name == assertion.probe)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog assertion {} references unknown probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            continue;
        };
        let probe = &analog.probes[probe_index];
        let Some(threshold) = threshold_for(assertion, probe) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog assertion {} is missing a threshold for probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            continue;
        };
        let measured = match measured_assertion_value(
            assertion,
            &run.series.time_s,
            &run.series.values_by_probe[probe_index],
        ) {
            Some(value) => value,
            None => {
                let mut finding = Finding::critical(
                    SPICE_TRANSIENT_ANALYSIS,
                    &scenario.name,
                    format!(
                        "Waveform does not cover assertion {} over its requested time range.",
                        assertion.name
                    ),
                );
                finding.measured.insert(
                    "waveform".to_string(),
                    json!(normalize_artifact_path(&run.waveform)),
                );
                insert_time_limit(assertion, &mut finding);
                findings.push(finding);
                continue;
            }
        };
        let passed = match assertion.relation {
            AnalogRelation::Below => measured < threshold.value,
            AnalogRelation::Above => measured > threshold.value,
        };
        if !passed {
            let relation = match assertion.relation {
                AnalogRelation::Below => "below",
                AnalogRelation::Above => "above",
            };
            let aggregation = aggregation_label(&assertion.aggregation);
            let mut finding = Finding::critical(
                SPICE_TRANSIENT_ANALYSIS,
                &scenario.name,
                format!(
                    "Analog assertion {} failed: {aggregation} probe {} measured {:.6} {}, expected {relation} {:.6} {}{}.",
                    assertion.name,
                    assertion.probe,
                    measured,
                    threshold.unit,
                    threshold.value,
                    threshold.unit,
                    assertion_time_phrase(assertion)
                ),
            );
            finding
                .measured
                .insert(assertion.probe.clone(), json!(measured));
            finding
                .measured
                .insert(format!("{}_unit", assertion.probe), json!(threshold.unit));
            finding.measured.insert(
                format!("{}_quantity", assertion.probe),
                json!(quantity_name(&probe.quantity)),
            );
            insert_measured_time(assertion, &mut finding);
            finding.limit.insert(
                format!("{relation}{}", threshold.limit_key),
                json!(threshold.value),
            );
            finding
                .suggested_fixes
                .push("Adjust the circuit or device model so the simulated waveform meets the declared physical threshold.".to_string());
            findings.push(finding);
        }
    }
}

fn validate_probe_contract(probe: &AnalogProbe) -> Result<(), String> {
    let expression = probe
        .expression
        .trim()
        .to_ascii_lowercase()
        .replace(' ', "");
    let valid = match probe.quantity {
        AnalogQuantity::Voltage => expression.starts_with("v("),
        AnalogQuantity::Current => {
            expression.starts_with("i(")
                || expression.starts_with("-i(")
                || expression.starts_with("abs(i(")
        }
        AnalogQuantity::Power => {
            expression.contains("v(") && expression.contains("i(") && expression.contains('*')
        }
    };
    if valid {
        Ok(())
    } else {
        Err(format!(
            "expression {} is not consistent with declared {} quantity",
            probe.expression,
            quantity_name(&probe.quantity)
        ))
    }
}

fn validate_assertion_contract(
    assertion: &AnalogAssertion,
    stop_time_us: f64,
) -> Result<(), String> {
    match assertion.aggregation {
        AnalogAggregation::Sample => {
            if assertion.start_us.is_some() || assertion.end_us.is_some() {
                return Err("sample aggregation must not declare start_us or end_us".to_string());
            }
            let Some(at_us) = assertion.at_us else {
                return Err("requires at_us for sample aggregation".to_string());
            };
            if !at_us.is_finite() || at_us < 0.0 || at_us > stop_time_us {
                return Err(
                    "sample time must be finite and within the transient stop time".to_string(),
                );
            }
        }
        AnalogAggregation::Min | AnalogAggregation::Max => {
            if assertion.at_us.is_some() {
                return Err("window aggregation must not declare at_us".to_string());
            }
            let (Some(start_us), Some(end_us)) = (assertion.start_us, assertion.end_us) else {
                return Err("requires start_us and end_us for window aggregation".to_string());
            };
            if !start_us.is_finite()
                || !end_us.is_finite()
                || start_us < 0.0
                || end_us < start_us
                || end_us > stop_time_us
            {
                return Err(
                    "window bounds must be finite, ordered, and within the transient stop time"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

fn threshold_count(assertion: &AnalogAssertion) -> usize {
    [
        assertion.threshold_v,
        assertion.threshold_a,
        assertion.threshold_w,
    ]
    .into_iter()
    .filter(|threshold| threshold.is_some_and(f64::is_finite))
    .count()
}

fn threshold_for(assertion: &AnalogAssertion, probe: &AnalogProbe) -> Option<AssertionThreshold> {
    let (value, unit, limit_key) = match probe.quantity {
        AnalogQuantity::Voltage => (assertion.threshold_v?, "V", "_V"),
        AnalogQuantity::Current => (assertion.threshold_a?, "A", "_A"),
        AnalogQuantity::Power => (assertion.threshold_w?, "W", "_W"),
    };
    value.is_finite().then_some(AssertionThreshold {
        value,
        unit,
        limit_key,
    })
}

fn measured_assertion_value(
    assertion: &AnalogAssertion,
    times: &[f64],
    values: &[f64],
) -> Option<f64> {
    match assertion.aggregation {
        AnalogAggregation::Sample => interpolate_at(times, values, assertion.at_us? / 1_000_000.0),
        AnalogAggregation::Min | AnalogAggregation::Max => {
            let start = assertion.start_us? / 1_000_000.0;
            let end = assertion.end_us? / 1_000_000.0;
            aggregate_window(times, values, start, end, &assertion.aggregation)
        }
    }
}

fn aggregate_window(
    times: &[f64],
    values: &[f64],
    start: f64,
    end: f64,
    aggregation: &AnalogAggregation,
) -> Option<f64> {
    if start > end {
        return None;
    }
    let mut selected = Vec::new();
    selected.push(interpolate_at(times, values, start)?);
    for (time, value) in times.iter().copied().zip(values.iter().copied()) {
        if time > start && time < end {
            selected.push(value);
        }
    }
    selected.push(interpolate_at(times, values, end)?);
    match aggregation {
        AnalogAggregation::Min => selected.into_iter().reduce(f64::min),
        AnalogAggregation::Max => selected.into_iter().reduce(f64::max),
        AnalogAggregation::Sample => None,
    }
}

fn aggregation_label(aggregation: &AnalogAggregation) -> &'static str {
    match aggregation {
        AnalogAggregation::Sample => "sampled",
        AnalogAggregation::Min => "minimum",
        AnalogAggregation::Max => "maximum",
    }
}

fn quantity_name(quantity: &AnalogQuantity) -> &'static str {
    match quantity {
        AnalogQuantity::Voltage => "voltage",
        AnalogQuantity::Current => "current",
        AnalogQuantity::Power => "power",
    }
}

fn assertion_time_phrase(assertion: &AnalogAssertion) -> String {
    match assertion.aggregation {
        AnalogAggregation::Sample => format!(" at {} us", assertion.at_us.unwrap_or_default()),
        AnalogAggregation::Min | AnalogAggregation::Max => format!(
            " from {} us to {} us",
            assertion.start_us.unwrap_or_default(),
            assertion.end_us.unwrap_or_default()
        ),
    }
}

fn insert_time_limit(assertion: &AnalogAssertion, finding: &mut Finding) {
    match assertion.aggregation {
        AnalogAggregation::Sample => {
            if let Some(at_us) = assertion.at_us {
                finding
                    .limit
                    .insert("sample_time_us".to_string(), json!(at_us));
            }
        }
        AnalogAggregation::Min | AnalogAggregation::Max => {
            if let Some(start_us) = assertion.start_us {
                finding
                    .limit
                    .insert("start_us".to_string(), json!(start_us));
            }
            if let Some(end_us) = assertion.end_us {
                finding.limit.insert("end_us".to_string(), json!(end_us));
            }
        }
    }
}

fn insert_measured_time(assertion: &AnalogAssertion, finding: &mut Finding) {
    match assertion.aggregation {
        AnalogAggregation::Sample => {
            if let Some(at_us) = assertion.at_us {
                finding
                    .measured
                    .insert("sample_time_us".to_string(), json!(at_us));
            }
        }
        AnalogAggregation::Min | AnalogAggregation::Max => {
            if let Some(start_us) = assertion.start_us {
                finding
                    .measured
                    .insert("start_us".to_string(), json!(start_us));
            }
            if let Some(end_us) = assertion.end_us {
                finding.measured.insert("end_us".to_string(), json!(end_us));
            }
        }
    }
}

fn interpolate_at(times: &[f64], values: &[f64], target: f64) -> Option<f64> {
    if times.len() != values.len() || times.is_empty() {
        return None;
    }
    if target < times[0] || target > *times.last()? {
        return None;
    }
    for index in 0..times.len() {
        if (times[index] - target).abs() <= f64::EPSILON {
            return Some(values[index]);
        }
        if index + 1 < times.len() && times[index] <= target && target <= times[index + 1] {
            let span = times[index + 1] - times[index];
            if span <= 0.0 {
                return None;
            }
            let fraction = (target - times[index]) / span;
            return Some(values[index] + fraction * (values[index + 1] - values[index]));
        }
    }
    None
}

enum BackendSelection {
    Selected(&'static str),
    Unavailable,
    EmbeddedUnavailable,
}

fn external_backend_unavailable(scenario_name: &str, backend: &AnalogBackend) -> Finding {
    let mut finding = Finding::critical(
        "ANALOG_BACKEND_UNAVAILABLE",
        scenario_name,
        "Physical analog simulation requires ngspice, Xyce, or a linked embedded ngspice backend, but no requested solver is available.",
    );
    finding.limit.insert(
        "required_backend".to_string(),
        json!("ngspice_xyce_or_embedded_ngspice"),
    );
    finding.suggested_fixes.push(
        "Install ngspice/Xyce or build CircuitCI with a mature embedded ngspice backend."
            .to_string(),
    );
    finding.suggested_fixes.push(
        "Keep behavioral control-line checks marked as non-physical until this simulation runs."
            .to_string(),
    );
    if *backend == AnalogBackend::EmbeddedNgspice {
        finding.id = "ANALOG_EMBEDDED_SOLVER_UNAVAILABLE".to_string();
    }
    finding
}

fn embedded_solver_unavailable(scenario_name: &str) -> Finding {
    let mut finding = Finding::critical(
        "ANALOG_EMBEDDED_SOLVER_UNAVAILABLE",
        scenario_name,
        "The embedded_ngspice backend was requested, but no mature ngspice-derived engine is linked into this CircuitCI build.",
    );
    finding
        .limit
        .insert("required_backend".to_string(), json!("embedded_ngspice"));
    finding.suggested_fixes.push(
        "Vendor or link a mature ngspice-derived solver through the analog adapter; do not replace it with a partial toy SPICE subset."
            .to_string(),
    );
    finding
}

fn select_backend(requested: &AnalogBackend) -> BackendSelection {
    match requested {
        AnalogBackend::Ngspice => {
            if executable_on_path("ngspice") {
                BackendSelection::Selected("ngspice")
            } else {
                BackendSelection::Unavailable
            }
        }
        AnalogBackend::Xyce => {
            if executable_on_path("Xyce") {
                BackendSelection::Selected("Xyce")
            } else if executable_on_path("xyce") {
                BackendSelection::Selected("xyce")
            } else {
                BackendSelection::Unavailable
            }
        }
        AnalogBackend::Auto => {
            if executable_on_path("ngspice") {
                BackendSelection::Selected("ngspice")
            } else if executable_on_path("Xyce") {
                BackendSelection::Selected("Xyce")
            } else if executable_on_path("xyce") {
                BackendSelection::Selected("xyce")
            } else {
                BackendSelection::Unavailable
            }
        }
        AnalogBackend::EmbeddedNgspice => BackendSelection::EmbeddedUnavailable,
    }
}

fn backend_name(backend: &AnalogBackend) -> &'static str {
    match backend {
        AnalogBackend::Auto => "auto",
        AnalogBackend::Ngspice => "ngspice",
        AnalogBackend::Xyce => "xyce",
        AnalogBackend::EmbeddedNgspice => "embedded_ngspice",
    }
}

fn executable_on_path(binary: &str) -> bool {
    let candidate = Path::new(binary);
    if candidate.components().count() > 1 {
        return candidate.is_file();
    }
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&paths).any(|dir| {
        let path: PathBuf = dir.join(binary);
        path.is_file()
    })
}

fn file_sha256_hex(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("Failed to read model file {}: {error}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>())
}

fn safe_artifact_name(name: &str) -> String {
    let mut output = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
            output.push(character);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "scenario".to_string()
    } else {
        output
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn absolute_path(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(normalize_path(path));
    }
    Ok(normalize_path(&env::current_dir()?.join(path)))
}

fn push_artifact(artifacts: &mut Vec<String>, path: &Path) {
    let artifact = normalize_artifact_path(path);
    if !artifacts.iter().any(|existing| existing == &artifact) {
        artifacts.push(artifact);
    }
}

fn normalize_artifact_path(path: &Path) -> String {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{
        aggregate_window, interpolate_at, parse_waveform_csv, threshold_count,
        validate_assertion_contract, validate_probe_contract,
    };
    use crate::board_ir::{
        AnalogAggregation, AnalogAssertion, AnalogProbe, AnalogQuantity, AnalogRelation,
    };

    #[test]
    fn parser_skips_header_and_interpolates_samples() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("waveform.csv");
        std::fs::write(
            &path,
            "time v(boot0) v(nrst)\n0.0 0.0 0.0\n0.0001 1.0 2.0\n",
        )
        .unwrap();
        let series = parse_waveform_csv(&path, 2).unwrap();
        assert_eq!(series.time_s.len(), 2);
        assert_eq!(
            interpolate_at(&series.time_s, &series.values_by_probe[0], 0.00005).unwrap(),
            0.5
        );
    }

    #[test]
    fn parser_rejects_non_finite_probe_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("waveform.csv");
        std::fs::write(&path, "time v(boot0)\n0.0 NaN\n").unwrap();
        let error = parse_waveform_csv(&path, 1).unwrap_err();
        assert!(error.contains("non-numeric probe value"));
    }

    #[test]
    fn parser_rejects_non_increasing_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("waveform.csv");
        std::fs::write(&path, "time v(boot0)\n0.0 0.0\n0.0 1.0\n").unwrap();
        let error = parse_waveform_csv(&path, 1).unwrap_err();
        assert!(error.contains("non-increasing time"));
    }

    #[test]
    fn window_aggregation_interpolates_boundaries() {
        let times = [0.0, 1.0, 2.0, 3.0];
        let values = [0.0, 10.0, 2.0, 8.0];
        let min = aggregate_window(&times, &values, 0.5, 2.5, &AnalogAggregation::Min).unwrap();
        let max = aggregate_window(&times, &values, 0.5, 2.5, &AnalogAggregation::Max).unwrap();
        assert_eq!(min, 2.0);
        assert_eq!(max, 10.0);
    }

    #[test]
    fn window_aggregation_rejects_out_of_range_window() {
        let times = [0.0, 1.0];
        let values = [0.0, 1.0];
        assert!(aggregate_window(&times, &values, -0.1, 0.5, &AnalogAggregation::Min).is_none());
        assert!(aggregate_window(&times, &values, 0.5, 1.1, &AnalogAggregation::Max).is_none());
    }

    #[test]
    fn probe_contract_rejects_mismatched_quantity_expression() {
        let probe = AnalogProbe {
            name: "bad_current".to_string(),
            expression: "V(nrst)".to_string(),
            quantity: AnalogQuantity::Current,
        };
        assert!(validate_probe_contract(&probe).is_err());

        let probe = AnalogProbe {
            name: "base_current".to_string(),
            expression: "abs(I(VRTS))".to_string(),
            quantity: AnalogQuantity::Current,
        };
        assert!(validate_probe_contract(&probe).is_ok());
    }

    #[test]
    fn assertion_contract_rejects_contradictory_timing_and_thresholds() {
        let assertion = AnalogAssertion {
            name: "bad_sample".to_string(),
            probe: "nrst".to_string(),
            at_us: Some(100.0),
            start_us: Some(0.0),
            end_us: None,
            aggregation: AnalogAggregation::Sample,
            relation: AnalogRelation::Above,
            threshold_v: Some(1.0),
            threshold_a: None,
            threshold_w: None,
        };
        assert!(validate_assertion_contract(&assertion, 1000.0).is_err());

        let assertion = AnalogAssertion {
            name: "bad_units".to_string(),
            probe: "nrst".to_string(),
            at_us: Some(100.0),
            start_us: None,
            end_us: None,
            aggregation: AnalogAggregation::Sample,
            relation: AnalogRelation::Above,
            threshold_v: Some(1.0),
            threshold_a: Some(0.001),
            threshold_w: None,
        };
        assert_eq!(threshold_count(&assertion), 2);
    }
}
