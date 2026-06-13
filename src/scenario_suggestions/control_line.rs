use crate::board_ir::{BoardProject, ControlEffect, Endpoint, ScenarioEvent};
use crate::library::BoundBoard;
use std::collections::BTreeMap;

use super::{
    CONTROL_LINE_RELEASE_SEQUENCE, ScenarioSuggestion, SuggestedControlEffect, SuggestedEndpoint,
    SuggestedEvent, SuggestedScenario, SuggestedTarget, SuggestedTiming, sanitized_name,
};

pub(super) fn control_line_sequence_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_control_line_checks(bound.project);
    let mut suggestions = Vec::new();
    for evidence in &bound.project.board.runtime.control_line_sequences {
        let target_component = evidence.target.component.trim();
        if target_component.is_empty() || evidence.required_boot_mode.trim().is_empty() {
            continue;
        }
        let key = (
            target_component.to_string(),
            evidence.required_boot_mode.clone(),
        );
        if existing.contains_key(&key) {
            continue;
        }
        if evidence.control_effects.is_empty() || evidence.events.is_empty() {
            continue;
        }
        let Some(boot_sample_at_us) = evidence.timing.boot_sample_at_us else {
            continue;
        };
        if !evidence.timing.power_valid_at_us.is_finite()
            || evidence.timing.power_valid_at_us < 0.0
            || !evidence.timing.reset_release_at_us.is_finite()
            || evidence.timing.reset_release_at_us < 0.0
            || !boot_sample_at_us.is_finite()
            || boot_sample_at_us < 0.0
        {
            continue;
        }
        let Some(component) = bound.project.board.components.get(target_component) else {
            continue;
        };
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        if model.behavior.reset.is_none()
            || model
                .behavior
                .boot
                .as_ref()
                .and_then(|boot| boot.modes.get(&evidence.required_boot_mode))
                .is_none()
        {
            continue;
        }
        let suggestion_name = evidence.name.clone().unwrap_or_else(|| {
            format!(
                "{}_{}_control_line_release",
                sanitized_name(target_component),
                sanitized_name(&evidence.required_boot_mode)
            )
        });
        let source = evidence
            .source
            .as_deref()
            .map(|source| format!(" from {source}"))
            .unwrap_or_default();
        suggestions.push(ScenarioSuggestion {
            id: format!(
                "control_line_release_sequence_{}_{}",
                sanitized_name(target_component),
                sanitized_name(&evidence.required_boot_mode)
            ),
            kind: "reset_boot".to_string(),
            confidence: "high".to_string(),
            runnable: true,
            reason: format!(
                "Runtime evidence{source} provides a complete control-line release sequence for {target_component} boot mode {}.",
                evidence.required_boot_mode
            ),
            scenario: SuggestedScenario {
                name: suggestion_name,
                scenario_type: "control_line_sequence".to_string(),
                checks: vec![CONTROL_LINE_RELEASE_SEQUENCE.to_string()],
                parameters: None,
                target: Some(SuggestedTarget {
                    component: target_component.to_string(),
                    power_pin: evidence.target.power_pin.clone(),
                    reset_pin: evidence.target.reset_pin.clone(),
                }),
                timing: Some(SuggestedTiming {
                    power_valid_at_us: evidence.timing.power_valid_at_us,
                    reset_release_delay_us: evidence.timing.reset_release_delay_us,
                    reset_release_at_us: Some(evidence.timing.reset_release_at_us),
                    boot_sample_at_us: Some(boot_sample_at_us),
                }),
                required_boot_mode: Some(evidence.required_boot_mode.clone()),
                straps: Vec::new(),
                bootloader: None,
                control_effects: evidence
                    .control_effects
                    .iter()
                    .map(suggested_control_effect)
                    .collect(),
                events: evidence.events.iter().map(suggested_event).collect(),
                conditioning: None,
                protection_clamps: Vec::new(),
                usb_connectors: Vec::new(),
                usb_routes: Vec::new(),
                usb_route_pairs: Vec::new(),
                clocks: Vec::new(),
                reset_supervisors: Vec::new(),
                regulators: Vec::new(),
                pin_states: Vec::new(),
                paths: Vec::new(),
            },
            required_inputs: Vec::new(),
        });
    }
    suggestions
}

fn suggested_control_effect(effect: &ControlEffect) -> SuggestedControlEffect {
    SuggestedControlEffect {
        name: effect.name.clone(),
        source: suggested_endpoint(&effect.source),
        target: suggested_endpoint(&effect.target),
        asserted_state: effect.asserted_state.clone(),
        released_state: effect.released_state.clone(),
        release_delay_us: effect.release_delay_us,
    }
}

fn suggested_event(event: &ScenarioEvent) -> SuggestedEvent {
    SuggestedEvent {
        at_us: Some(event.at_us),
        action: event.action.clone(),
        from: event.from.as_ref().map(suggested_endpoint),
        to: event.to.as_ref().map(suggested_endpoint),
        bytes: event.bytes.clone(),
        line: event.line.clone(),
        asserted: event.asserted,
    }
}

fn suggested_endpoint(endpoint: &Endpoint) -> SuggestedEndpoint {
    SuggestedEndpoint {
        component: endpoint.component.clone(),
        pin: endpoint.pin.clone(),
    }
}

fn existing_control_line_checks(project: &BoardProject) -> BTreeMap<(String, String), ()> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "control_line_sequence"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == CONTROL_LINE_RELEASE_SEQUENCE)
        })
        .filter_map(|scenario| {
            Some((
                (
                    scenario.target.as_ref()?.component.clone(),
                    scenario.required_boot_mode.clone()?,
                ),
                (),
            ))
        })
        .collect()
}
