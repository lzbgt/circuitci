use crate::board_ir::{AnalogNetlistSource, AnalogRelation, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
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
    BackendSelection, NgspiceRunOptions, ParameterOverride, backend_name,
    embedded_solver_unavailable, external_backend_unavailable, run_ngspice, select_backend,
};
use super::analog_soa::evaluate_soa_limits;
use super::analog_util::{file_sha256_hex, push_artifact, safe_artifact_name};
use super::common::validation_input_missing;
use super::spice_netlist::generate_board_netlist;

const MAX_ANALOG_SWEEP_CORNERS: usize = 64;

pub(super) struct AnalogTransientSinks<'a> {
    pub(super) findings: &'a mut Vec<Finding>,
    pub(super) artifacts: &'a mut Vec<String>,
    pub(super) waveforms: &'a mut Vec<String>,
}

pub(super) fn validate_spice_transient_with_progress<F, C>(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    sinks: &mut AnalogTransientSinks<'_>,
    output: &Path,
    mut on_progress: F,
    should_cancel: C,
) where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let findings = &mut *sinks.findings;
    let artifacts = &mut *sinks.artifacts;
    let waveforms = &mut *sinks.waveforms;
    on_progress(
        "Preparing analog transient",
        format!("Checking analog scenario {}.", scenario.name),
    );
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_transient scenario requires an analog block.",
        );
        return;
    };
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }

    on_progress(
        "Checking analog model evidence",
        format!(
            "{} model file(s), {} node binding(s), {} pin binding(s).",
            analog.model_files.len(),
            analog.node_bindings.len(),
            analog.pin_bindings.len()
        ),
    );
    if let Some(finding) = validate_netlist_source(bound, scenario, artifacts) {
        findings.push(finding);
        return;
    }

    if analog.node_bindings.is_empty() || analog.pin_bindings.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "analog_transient requires node_bindings and pin_bindings.",
        );
        return;
    }
    if should_cancel() {
        push_canceled_finding(findings, scenario);
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
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
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
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
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
    if analog.probes.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SPICE_TRANSIENT_ANALYSIS requires at least one waveform probe.",
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
    if analog.assertions.is_empty() {
        let mut finding = Finding::info(
            "ANALOG_ASSERTIONS_ABSENT",
            &scenario.name,
            "SPICE transient solved and exported probes, but no quantitative waveform assertions were declared.",
        );
        finding.limit.insert(
            "required_for_signoff".to_string(),
            json!("Add voltage/current/power assertions for the board behavior being verified."),
        );
        findings.push(finding);
    }
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }
    let run_plans = match analog_run_plans(analog) {
        Ok(run_plans) => run_plans,
        Err(message) => {
            validation_input_missing(findings, scenario, message);
            return;
        }
    };

    let run_dir = output
        .join("analog")
        .join(safe_artifact_name(&scenario.name));
    on_progress(
        "Preparing analog run directory",
        format!("Creating {}.", run_dir.to_string_lossy()),
    );
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
    if should_cancel() {
        push_canceled_finding(findings, scenario);
        return;
    }
    on_progress(
        "Preparing analog deck",
        format!(
            "Resolving {} netlist source.",
            netlist_source_name(&analog.netlist_source)
        ),
    );
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
    on_progress(
        "Preparing analog operating probes",
        format!("Collecting operating-limit probes for {}.", scenario.name),
    );
    let operating_limits = operating_limit_probes(bound, scenario);
    if !operating_limits.metadata_findings.is_empty() {
        findings.extend(operating_limits.metadata_findings);
        return;
    }

    on_progress(
        "Selecting analog backend",
        format!("Requested backend {}.", backend_name(&analog.backend)),
    );
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

    if !matches!(backend, "ngspice" | "embedded_ngspice") {
        let mut finding = Finding::critical(
            SPICE_TRANSIENT_ANALYSIS,
            &scenario.name,
            format!(
                "Backend {backend} was detected, but only ngspice-compatible execution is implemented in this runtime slice."
            ),
        );
        finding
            .measured
            .insert("selected_backend".to_string(), json!(backend));
        finding.limit.insert(
            "implemented_backend".to_string(),
            json!("ngspice_or_embedded_ngspice"),
        );
        findings.push(finding);
        return;
    }

    let operating_expressions = operating_probe_expressions(&operating_limits);
    for run_plan in run_plans {
        if should_cancel() {
            push_canceled_finding(findings, scenario);
            return;
        }
        on_progress("Running analog input corner", run_plan.progress_label());
        match run_ngspice(
            bound,
            scenario,
            backend,
            &source_netlist,
            NgspiceRunOptions {
                output,
                run_subdir: run_plan.run_subdir.as_deref(),
                parameter_overrides: &run_plan.parameter_overrides,
                operating_probe_expressions: &operating_expressions,
                on_progress: &mut on_progress,
                should_cancel: &should_cancel,
            },
        ) {
            Ok(run) => {
                let finding_start = findings.len();
                on_progress(
                    "Recording analog artifacts",
                    format!(
                        "{} artifact(s), waveform {}.",
                        run.artifacts.len(),
                        run.waveform.to_string_lossy()
                    ),
                );
                for artifact in &run.artifacts {
                    push_artifact(artifacts, artifact);
                }
                push_artifact(waveforms, &run.waveform);
                on_progress(
                    "Evaluating analog assertions",
                    format!(
                        "{} user probe(s), {} assertion(s).",
                        run.user_probe_count,
                        analog.assertions.len()
                    ),
                );
                evaluate_waveform_assertions(scenario, &run, findings);
                on_progress(
                    "Evaluating analog limits",
                    format!("{} operating probe(s).", operating_limits.probes.len()),
                );
                evaluate_operating_limits(scenario, &run, &operating_limits.probes, findings);
                evaluate_soa_limits(scenario, &run, &operating_limits, findings);
                tag_corner_findings(findings, finding_start, &run_plan);
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
                tag_corner_finding(&mut finding, &run_plan);
                finding.suggested_fixes.push(
                    "Inspect the generated ngspice wrapper deck and solver log artifacts."
                        .to_string(),
                );
                findings.push(finding);
            }
        }
    }
}

fn push_canceled_finding(findings: &mut Vec<Finding>, scenario: &Scenario) {
    findings.push(Finding::critical(
        "VALIDATION_CANCELED",
        &scenario.name,
        "Analog transient validation was canceled before completion.",
    ));
}

#[derive(Debug, Clone)]
struct AnalogRunPlan {
    sweep_name: Option<String>,
    corner_name: String,
    run_subdir: Option<String>,
    parameter_overrides: Vec<ParameterOverride>,
}

impl AnalogRunPlan {
    fn nominal() -> Self {
        Self {
            sweep_name: None,
            corner_name: "nominal".to_string(),
            run_subdir: None,
            parameter_overrides: Vec::new(),
        }
    }

    fn progress_label(&self) -> String {
        if let Some(sweep_name) = &self.sweep_name {
            format!(
                "{} / {} with {} override(s).",
                sweep_name,
                self.corner_name,
                self.parameter_overrides.len()
            )
        } else {
            "nominal input set.".to_string()
        }
    }
}

fn analog_run_plans(
    analog: &crate::board_ir::AnalogScenario,
) -> Result<Vec<AnalogRunPlan>, String> {
    if analog.sweeps.is_empty() {
        return Ok(vec![AnalogRunPlan::nominal()]);
    }
    let mut plans = Vec::new();
    for sweep in &analog.sweeps {
        if sweep.name.trim().is_empty() {
            return Err("analog sweep names must not be empty.".to_string());
        }
        if sweep.parameters.is_empty() {
            return Err(format!(
                "Analog sweep {} must declare at least one parameter.",
                sweep.name
            ));
        }
        let mut seen = BTreeSet::new();
        let mut parameter_values = Vec::new();
        for parameter in &sweep.parameters {
            if !valid_spice_parameter_name(&parameter.name) {
                return Err(format!(
                    "Analog sweep {} has invalid SPICE parameter name {}.",
                    sweep.name, parameter.name
                ));
            }
            if !seen.insert(parameter.name.clone()) {
                return Err(format!(
                    "Analog sweep {} declares parameter {} more than once.",
                    sweep.name, parameter.name
                ));
            }
            if parameter.values.is_empty() {
                return Err(format!(
                    "Analog sweep {} parameter {} must declare at least one value.",
                    sweep.name, parameter.name
                ));
            }
            if parameter.values.iter().any(|value| !value.is_finite()) {
                return Err(format!(
                    "Analog sweep {} parameter {} contains a non-finite value.",
                    sweep.name, parameter.name
                ));
            }
            parameter_values.push((parameter.name.clone(), parameter.values.clone()));
        }
        let mut combinations = Vec::new();
        expand_parameter_combinations(&parameter_values, 0, Vec::new(), &mut combinations)?;
        for (index, overrides) in combinations.into_iter().enumerate() {
            if plans.len() >= MAX_ANALOG_SWEEP_CORNERS {
                return Err(format!(
                    "Analog sweeps exceed the {MAX_ANALOG_SWEEP_CORNERS}-corner execution cap."
                ));
            }
            let corner_name = format!("corner_{:03}", index + 1);
            plans.push(AnalogRunPlan {
                sweep_name: Some(sweep.name.clone()),
                run_subdir: Some(format!("{}_{}", sweep.name, corner_name)),
                corner_name,
                parameter_overrides: overrides,
            });
        }
    }
    Ok(plans)
}

fn expand_parameter_combinations(
    parameters: &[(String, Vec<f64>)],
    index: usize,
    current: Vec<ParameterOverride>,
    output: &mut Vec<Vec<ParameterOverride>>,
) -> Result<(), String> {
    if output.len() >= MAX_ANALOG_SWEEP_CORNERS {
        return Err(format!(
            "Analog sweeps exceed the {MAX_ANALOG_SWEEP_CORNERS}-corner execution cap."
        ));
    }
    let Some((name, values)) = parameters.get(index) else {
        output.push(current);
        return Ok(());
    };
    for value in values {
        let mut next = current.clone();
        next.push(ParameterOverride {
            name: name.clone(),
            value: *value,
        });
        expand_parameter_combinations(parameters, index + 1, next, output)?;
    }
    Ok(())
}

fn valid_spice_parameter_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn tag_corner_findings(findings: &mut [Finding], start: usize, run_plan: &AnalogRunPlan) {
    for finding in findings.iter_mut().skip(start) {
        tag_corner_finding(finding, run_plan);
    }
}

fn tag_corner_finding(finding: &mut Finding, run_plan: &AnalogRunPlan) {
    if let Some(sweep_name) = &run_plan.sweep_name {
        finding
            .measured
            .insert("analog_sweep".to_string(), json!(sweep_name));
        finding
            .measured
            .insert("analog_corner".to_string(), json!(run_plan.corner_name));
        finding.measured.insert(
            "analog_parameters".to_string(),
            json!(parameter_override_map(&run_plan.parameter_overrides)),
        );
    }
}

fn parameter_override_map(overrides: &[ParameterOverride]) -> BTreeMap<String, f64> {
    overrides
        .iter()
        .map(|override_| (override_.name.clone(), override_.value))
        .collect()
}

fn netlist_source_name(source: &AnalogNetlistSource) -> &'static str {
    match source {
        AnalogNetlistSource::File => "file-backed",
        AnalogNetlistSource::GeneratedFromBoard => "generated-from-Board",
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

#[cfg(test)]
mod tests {
    use super::{
        AnalogRunPlan, MAX_ANALOG_SWEEP_CORNERS, ParameterOverride, analog_run_plans,
        tag_corner_finding,
    };
    use crate::board_ir::BoardProject;
    use crate::reports::Finding;

    fn rc_sweep_project_yaml(parameter_yaml: &str) -> String {
        format!(
            r#"
project:
  name: sweep_test
  version: 0.1.0
board:
  name: sweep_test
  components: {{}}
  nets: {{}}
scenarios:
  - name: analog_sweep
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: file
      netlist: deck.cir
      model_files: []
      node_bindings:
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: "0", endpoint: {{ component: U1, pin: GND }} }}
      analysis:
        type: tran
        stop_time_us: 1000.0
        max_step_us: 1.0
      stimuli: []
      sweeps:
        - name: rc_tolerance
          parameters:
{parameter_yaml}
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"#
        )
    }

    #[test]
    fn analog_run_plans_expand_parameter_sweeps() {
        let yaml = rc_sweep_project_yaml(
            r#"            - name: RIN_VALUE
              values: [950.0, 1000.0]
            - name: COUT_VALUE
              values: [0.000000095, 0.0000001]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 4);
        assert_eq!(plans[0].sweep_name.as_deref(), Some("rc_tolerance"));
        assert_eq!(plans[0].corner_name, "corner_001");
        assert_eq!(
            plans[0].run_subdir.as_deref(),
            Some("rc_tolerance_corner_001")
        );
        assert_eq!(plans[0].parameter_overrides[0].name, "RIN_VALUE");
        assert_eq!(plans[0].parameter_overrides[0].value, 950.0);
        assert_eq!(plans[0].parameter_overrides[1].name, "COUT_VALUE");
        assert_eq!(plans[0].parameter_overrides[1].value, 0.000000095);
        assert_eq!(plans[3].parameter_overrides[0].value, 1000.0);
        assert_eq!(plans[3].parameter_overrides[1].value, 0.0000001);
    }

    #[test]
    fn analog_run_plans_reject_invalid_parameter_names() {
        let yaml = rc_sweep_project_yaml(
            r#"            - name: 1BAD
              values: [1.0]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let error = analog_run_plans(analog).unwrap_err();

        assert!(error.contains("invalid SPICE parameter name"));
    }

    #[test]
    fn analog_run_plans_enforce_corner_cap() {
        let values = (0..=MAX_ANALOG_SWEEP_CORNERS)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let yaml = rc_sweep_project_yaml(&format!(
            r#"            - name: RIN_VALUE
              values: [{values}]
"#
        ));
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let error = analog_run_plans(analog).unwrap_err();

        assert!(error.contains("64-corner execution cap"));
    }

    #[test]
    fn sweep_corner_tags_findings_with_parameter_values() {
        let run_plan = AnalogRunPlan {
            sweep_name: Some("rc_tolerance".to_string()),
            corner_name: "corner_007".to_string(),
            run_subdir: Some("rc_tolerance_corner_007".to_string()),
            parameter_overrides: vec![ParameterOverride {
                name: "RIN_VALUE".to_string(),
                value: 1050.0,
            }],
        };
        let mut finding = Finding::critical(
            "SPICE_TRANSIENT_ANALYSIS",
            "rc_lowpass",
            "filtered output exceeded the attenuation limit",
        );

        tag_corner_finding(&mut finding, &run_plan);

        assert_eq!(finding.measured["analog_sweep"], "rc_tolerance");
        assert_eq!(finding.measured["analog_corner"], "corner_007");
        assert_eq!(finding.measured["analog_parameters"]["RIN_VALUE"], 1050.0);
    }
}
