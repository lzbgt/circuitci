use crate::board_ir::{AnalogBackend, AnalogRelation, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::env;
use std::path::{Component, Path, PathBuf};

use super::SPICE_TRANSIENT_ANALYSIS;
use super::common::validation_input_missing;

pub(super) fn validate_spice_transient(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    artifacts: &mut Vec<String>,
) {
    let Some(analog) = &scenario.analog else {
        validation_input_missing(
            findings,
            scenario,
            "analog_transient scenario requires an analog block.",
        );
        return;
    };

    let netlist = bound.project.source_dir.join(&analog.netlist);
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
            "Add a SPICE-compatible deck with device models for this board region.".to_string(),
        );
        findings.push(finding);
        return;
    }
    push_artifact(artifacts, &netlist);

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
    if analog.analysis.stop_time_us <= 0.0 || analog.analysis.max_step_us <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "analog.analysis stop_time_us and max_step_us must be positive.",
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
        if assertion.at_us < 0.0 {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog assertion {} has a negative sample time.",
                    assertion.name
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

    let mut finding = Finding::critical(
        SPICE_TRANSIENT_ANALYSIS,
        &scenario.name,
        "SPICE backend detection succeeded, but transient execution and waveform assertion parsing are not implemented in this runtime yet.",
    );
    finding
        .measured
        .insert("selected_backend".to_string(), json!(backend));
    finding
        .measured
        .insert("netlist".to_string(), json!(netlist.display().to_string()));
    finding.limit.insert(
        "required_evidence".to_string(),
        json!("transient_waveform_assertions"),
    );
    finding.suggested_fixes.push(
        "Implement backend invocation, solver-log capture, waveform export parsing, and assertion evaluation before marking physical analog acceptance as pass."
            .to_string(),
    );
    findings.push(finding);
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
