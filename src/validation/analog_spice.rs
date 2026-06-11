use crate::board_ir::{AnalogNetlistSource, AnalogRelation, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::SPICE_TRANSIENT_ANALYSIS;
use super::analog_assertions::{
    evaluate_waveform_assertions, quantity_name, threshold_count, threshold_for,
    validate_assertion_contract, validate_probe_contract,
};
use super::analog_operating_limits::{
    evaluate_operating_limits, operating_limit_probes, operating_probe_expressions,
};
use super::analog_runner::{
    BackendSelection, backend_name, embedded_solver_unavailable, external_backend_unavailable,
    run_ngspice, select_backend,
};
use super::analog_soa::evaluate_soa_limits;
use super::analog_util::{file_sha256_hex, push_artifact, safe_artifact_name};
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

    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    if let Err(error) = fs::create_dir_all(&run_dir) {
        findings.push(Finding::critical(
            SPICE_TRANSIENT_ANALYSIS,
            &scenario.name,
            format!(
                "Failed to create analog run directory {}: {error}",
                run_dir.display()
            ),
        ));
        return;
    }
    let source_netlist = match prepare_source_netlist(bound, scenario, &run_dir) {
        Ok(source_netlist) => {
            push_artifact(artifacts, &source_netlist);
            source_netlist
        }
        Err(message) => {
            let mut finding = Finding::critical(SPICE_TRANSIENT_ANALYSIS, &scenario.name, message);
            finding
                .limit
                .insert("required_artifact".to_string(), json!("spice_netlist"));
            finding.suggested_fixes.push(
                "Fix the generated Board IR to SPICE contract before selecting a solver backend."
                    .to_string(),
            );
            findings.push(finding);
            return;
        }
    };
    let operating_limits = operating_limit_probes(bound, scenario);
    if !operating_limits.metadata_findings.is_empty() {
        findings.extend(operating_limits.metadata_findings);
        return;
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

    let operating_expressions = operating_probe_expressions(&operating_limits);
    match run_ngspice(
        bound,
        scenario,
        backend,
        output,
        &source_netlist,
        &operating_expressions,
    ) {
        Ok(run) => {
            for artifact in &run.artifacts {
                push_artifact(artifacts, artifact);
            }
            push_artifact(waveforms, &run.waveform);
            evaluate_waveform_assertions(scenario, &run, findings);
            evaluate_operating_limits(scenario, &run, &operating_limits.probes, findings);
            evaluate_soa_limits(scenario, &run, &operating_limits, findings);
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
