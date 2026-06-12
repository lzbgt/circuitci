use crate::board_ir::{BoardProject, NetKind};
use crate::library::{BoundBoard, PortKind};
use serde::Serialize;
use std::collections::BTreeMap;

const POWER_TREE_VALID: &str = "POWER_TREE_VALID";
const RESET_RELEASE_AFTER_POWER_VALID: &str = "RESET_RELEASE_AFTER_POWER_VALID";

#[derive(Debug, Serialize)]
pub struct ScenarioSuggestionReport {
    pub schema_version: String,
    pub project: String,
    pub suggestions: Vec<ScenarioSuggestion>,
}

#[derive(Debug, Serialize)]
pub struct ScenarioSuggestion {
    pub id: String,
    pub kind: String,
    pub confidence: String,
    pub runnable: bool,
    pub reason: String,
    pub scenario: SuggestedScenario,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_inputs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedScenario {
    pub name: String,
    #[serde(rename = "type")]
    pub scenario_type: String,
    pub checks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SuggestedTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<SuggestedTiming>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedTarget {
    pub component: String,
    pub power_pin: String,
    pub reset_pin: String,
}

#[derive(Debug, Serialize)]
pub struct SuggestedTiming {
    pub power_valid_at_us: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_release_delay_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_release_at_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_sample_at_us: Option<f64>,
}

pub fn suggest_scenarios(bound: &BoundBoard<'_>) -> ScenarioSuggestionReport {
    let mut suggestions = Vec::new();
    if should_suggest_power_tree(bound.project) {
        suggestions.push(power_tree_suggestion(bound.project));
    }
    suggestions.extend(reset_release_suggestions(bound));
    ScenarioSuggestionReport {
        schema_version: "0.1.0".to_string(),
        project: bound.project.project.name.clone(),
        suggestions,
    }
}

fn should_suggest_power_tree(project: &BoardProject) -> bool {
    let has_power_net = project
        .board
        .nets
        .values()
        .any(|net| net.kind == NetKind::Power);
    let already_declared = project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "power_tree"
            && scenario
                .checks
                .iter()
                .any(|check| check == POWER_TREE_VALID)
    });
    has_power_net && !already_declared
}

fn power_tree_suggestion(project: &BoardProject) -> ScenarioSuggestion {
    ScenarioSuggestion {
        id: "power_tree_valid".to_string(),
        kind: "power_tree".to_string(),
        confidence: "high".to_string(),
        runnable: true,
        reason: "Project declares power nets but no POWER_TREE_VALID scenario.".to_string(),
        scenario: SuggestedScenario {
            name: format!("{}_power_tree", sanitized_name(&project.project.name)),
            scenario_type: "power_tree".to_string(),
            checks: vec![POWER_TREE_VALID.to_string()],
            target: None,
            timing: None,
        },
        required_inputs: Vec::new(),
    }
}

fn reset_release_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_reset_checks(bound.project);
    let mut suggestions = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        if existing.contains_key(component_id) {
            continue;
        }
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let Some(reset) = &model.behavior.reset else {
            continue;
        };
        if !component.pins.contains_key(&reset.pin) {
            continue;
        }
        let Some((power_pin, power_valid_at_us)) =
            model.ports.iter().find_map(|(pin_name, port)| {
                if port.kind != PortKind::ElectricalPower {
                    return None;
                }
                let net_name = component
                    .power_domains
                    .get(pin_name)
                    .or_else(|| component.pins.get(pin_name))
                    .or(component.power_domain.as_ref())?;
                let net = bound.project.board.nets.get(net_name)?;
                let power_valid_at_us = net.power_valid_at_us?;
                if power_valid_at_us.is_finite() && power_valid_at_us >= 0.0 {
                    Some((pin_name.clone(), power_valid_at_us))
                } else {
                    None
                }
            })
        else {
            continue;
        };
        let boot_sample_at_us = model
            .behavior
            .boot
            .as_ref()
            .and_then(|boot| boot.sample_time_after_reset_release_us)
            .map(|delay_us| power_valid_at_us + delay_us);
        suggestions.push(ScenarioSuggestion {
            id: format!("reset_release_after_power_valid_{}", sanitized_name(component_id)),
            kind: "reset_boot".to_string(),
            confidence: "medium".to_string(),
            runnable: false,
            reason: format!(
                "Component {component_id} has reset behavior and target rail power_valid_at_us, but no RESET_RELEASE_AFTER_POWER_VALID scenario."
            ),
            scenario: SuggestedScenario {
                name: format!("{}_reset_release_after_power", sanitized_name(component_id)),
                scenario_type: "reset_boot".to_string(),
                checks: vec![RESET_RELEASE_AFTER_POWER_VALID.to_string()],
                target: Some(SuggestedTarget {
                    component: component_id.clone(),
                    power_pin,
                    reset_pin: reset.pin.clone(),
                }),
                timing: Some(SuggestedTiming {
                    power_valid_at_us,
                    reset_release_delay_us: Some(0.0),
                    reset_release_at_us: None,
                    boot_sample_at_us,
                }),
            },
            required_inputs: vec![
                "Fill timing.reset_release_at_us from reset supervisor, RC, control-line, or analog waveform evidence before validation.".to_string(),
                "Keep timing.power_valid_at_us equal to the target rail power_valid_at_us or remove duplicated stale timing.".to_string(),
            ],
        });
    }
    suggestions
}

fn existing_reset_checks(project: &BoardProject) -> BTreeMap<String, ()> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "reset_boot"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == RESET_RELEASE_AFTER_POWER_VALID)
        })
        .filter_map(|scenario| {
            scenario
                .target
                .as_ref()
                .map(|target| (target.component.clone(), ()))
        })
        .collect()
}

fn sanitized_name(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            out.push('_');
            last_was_separator = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "scenario".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::sanitized_name;

    #[test]
    fn sanitizes_scenario_names() {
        assert_eq!(sanitized_name("UM STM32L4"), "um_stm32l4");
        assert_eq!(sanitized_name("U1"), "u1");
        assert_eq!(sanitized_name("!!!"), "scenario");
    }
}
