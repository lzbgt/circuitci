use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::MODEL_QUALITY_REQUIRED;

pub(super) fn validate_model_quality_required(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(components) = required_components(scenario, findings) else {
        return;
    };
    let Some(allowed_sources) = string_list_parameter(scenario, "allowed_sources", findings) else {
        return;
    };
    if allowed_sources.is_empty() {
        missing_input(
            scenario,
            "parameters.allowed_sources",
            "Set allowed_sources to at least one acceptable model_quality.source value.",
            findings,
        );
        return;
    }
    let Some(min_confidence) = string_parameter(scenario, "min_confidence", findings) else {
        return;
    };
    let Some(min_confidence_rank) = confidence_rank(&min_confidence) else {
        missing_input(
            scenario,
            "parameters.min_confidence",
            "Set min_confidence to low, medium, or high.",
            findings,
        );
        return;
    };

    for component_id in components {
        let Some(component) = bound.project.board.components.get(&component_id) else {
            missing_component(scenario, &component_id, findings);
            continue;
        };
        let Some(model) = bound.library.get(&component.model) else {
            let mut finding = Finding::critical(
                MODEL_QUALITY_REQUIRED,
                &scenario.name,
                "Component model is not available for model-quality sign-off.",
            );
            finding.component = Some(component_id.clone());
            finding
                .measured
                .insert("model".to_string(), json!(component.model));
            finding.suggested_fixes = vec![
                "Add the missing component model to one of the project libraries.".to_string(),
                "Bind the component to a source-backed model before fabrication sign-off."
                    .to_string(),
            ];
            findings.push(finding);
            continue;
        };

        let actual_source = model.model_quality.source.as_str();
        let actual_confidence = model.model_quality.confidence.as_str();
        let source_ok = allowed_sources
            .iter()
            .any(|allowed| allowed == actual_source);
        let confidence_ok = confidence_rank(actual_confidence)
            .map(|rank| rank >= min_confidence_rank)
            .unwrap_or(false);
        if source_ok && confidence_ok {
            continue;
        }

        let mut finding = Finding::critical(
            MODEL_QUALITY_REQUIRED,
            &scenario.name,
            "Component model quality does not meet the scenario sign-off policy.",
        );
        finding.component = Some(component_id.clone());
        finding
            .measured
            .insert("model".to_string(), json!(model.component_id));
        finding
            .measured
            .insert("model_source".to_string(), json!(actual_source));
        finding
            .measured
            .insert("model_confidence".to_string(), json!(actual_confidence));
        finding
            .limit
            .insert("allowed_sources".to_string(), json!(allowed_sources));
        finding
            .limit
            .insert("min_confidence".to_string(), json!(min_confidence));
        finding.suggested_fixes = vec![
            "Replace the placeholder model with a datasheet-backed or measured component model."
                .to_string(),
            "If this is a deliberate design envelope, keep it out of fabrication sign-off scenarios."
                .to_string(),
        ];
        findings.push(finding);
    }
}

fn required_components(scenario: &Scenario, findings: &mut Vec<Finding>) -> Option<Vec<String>> {
    if let Some(components) = string_list_parameter(scenario, "components", findings) {
        if components.is_empty() {
            missing_input(
                scenario,
                "parameters.components",
                "Set components to at least one board component id.",
                findings,
            );
            return None;
        }
        return Some(components);
    }
    if let Some(target) = &scenario.target {
        return Some(vec![target.component.clone()]);
    }
    missing_input(
        scenario,
        "parameters.components",
        "Set parameters.components or scenario.target.component for model-quality sign-off.",
        findings,
    );
    None
}

fn string_parameter(
    scenario: &Scenario,
    key: &'static str,
    findings: &mut Vec<Finding>,
) -> Option<String> {
    let Some(value) = scenario.parameters.get(key) else {
        missing_input(
            scenario,
            &format!("parameters.{key}"),
            &format!("Set parameters.{key} for model-quality sign-off."),
            findings,
        );
        return None;
    };
    let Some(value) = value.as_str() else {
        missing_input(
            scenario,
            &format!("parameters.{key}"),
            &format!("Set parameters.{key} to a string."),
            findings,
        );
        return None;
    };
    let value = value.trim();
    if value.is_empty() {
        missing_input(
            scenario,
            &format!("parameters.{key}"),
            &format!("Set parameters.{key} to a non-empty string."),
            findings,
        );
        return None;
    }
    Some(value.to_string())
}

fn string_list_parameter(
    scenario: &Scenario,
    key: &'static str,
    findings: &mut Vec<Finding>,
) -> Option<Vec<String>> {
    let value = scenario.parameters.get(key)?;
    let Some(items) = value.as_sequence() else {
        missing_input(
            scenario,
            &format!("parameters.{key}"),
            &format!("Set parameters.{key} to a list of strings."),
            findings,
        );
        return None;
    };
    let mut strings = Vec::with_capacity(items.len());
    for item in items {
        let Some(item) = item.as_str() else {
            missing_input(
                scenario,
                &format!("parameters.{key}"),
                &format!("Set every parameters.{key} entry to a string."),
                findings,
            );
            return None;
        };
        let item = item.trim();
        if item.is_empty() {
            missing_input(
                scenario,
                &format!("parameters.{key}"),
                &format!("Set every parameters.{key} entry to a non-empty string."),
                findings,
            );
            return None;
        }
        strings.push(item.to_string());
    }
    Some(strings)
}

fn confidence_rank(confidence: &str) -> Option<u8> {
    match confidence.trim() {
        "low" => Some(0),
        "medium" => Some(1),
        "high" => Some(2),
        _ => None,
    }
}

fn missing_component(scenario: &Scenario, component_id: &str, findings: &mut Vec<Finding>) {
    let mut finding = Finding::critical(
        MODEL_QUALITY_REQUIRED,
        &scenario.name,
        "Model-quality sign-off references a component that is not present on the board.",
    );
    finding.component = Some(component_id.to_string());
    finding
        .measured
        .insert("missing_component".to_string(), json!(component_id));
    finding.suggested_fixes = vec![
        "Fix parameters.components to reference an existing board component.".to_string(),
        "Add the selected component to the board before enabling this sign-off gate.".to_string(),
    ];
    findings.push(finding);
}

fn missing_input(scenario: &Scenario, input: &str, fix: &str, findings: &mut Vec<Finding>) {
    let mut finding = Finding::critical(
        MODEL_QUALITY_REQUIRED,
        &scenario.name,
        "Model-quality validation is missing required sign-off policy input.",
    );
    finding
        .measured
        .insert("missing_input".to_string(), json!(input));
    finding.suggested_fixes = vec![fix.to_string()];
    findings.push(finding);
}
