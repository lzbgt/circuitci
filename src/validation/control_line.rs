use crate::board_ir::{ControlEffect, Scenario};
use crate::library::{BoundBoard, PortKind};
use crate::reports::Finding;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

use super::CONTROL_LINE_RELEASE_SEQUENCE;
use super::common::{
    PinDirection, component_pin_connected, model_port, normalize_state, target_model,
    validate_kicad_pin_direction, validation_input_missing,
};
use super::target_contract::validate_reset_target_assertions;

pub(super) fn validate_control_line_release(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some((target_component, model)) = target_model(bound, scenario) else {
        validation_input_missing(
            findings,
            scenario,
            "control_line_sequence target component and model are required.",
        );
        return;
    };
    validate_reset_target_assertions(bound, scenario, findings);
    let Some(timing) = &scenario.timing else {
        validation_input_missing(
            findings,
            scenario,
            "control_line_sequence timing is required.",
        );
        return;
    };
    let Some(boot_sample_at_us) = timing.boot_sample_at_us else {
        validation_input_missing(
            findings,
            scenario,
            "control_line_sequence timing.boot_sample_at_us is required.",
        );
        return;
    };
    let Some(required_boot_mode) = &scenario.required_boot_mode else {
        validation_input_missing(findings, scenario, "required_boot_mode is required.");
        return;
    };
    let Some(boot) = &model.behavior.boot else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Component model {} does not declare boot modes.",
                model.component_id
            ),
        );
        return;
    };
    let Some(mode) = boot.modes.get(required_boot_mode) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Component model {} does not declare boot mode {}.",
                model.component_id, required_boot_mode
            ),
        );
        return;
    };
    let Some(reset) = &model.behavior.reset else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Component model {} does not declare reset behavior.",
                model.component_id
            ),
        );
        return;
    };
    if scenario.control_effects.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "control_effects are required for CONTROL_LINE_RELEASE_SEQUENCE.",
        );
        return;
    }

    let mut names = BTreeSet::new();
    let mut target_effects = BTreeMap::new();
    for effect in &scenario.control_effects {
        if !names.insert(effect.name.as_str()) {
            control_line_finding(
                findings,
                scenario,
                &target_component,
                format!("Duplicate control effect name {}.", effect.name),
                None,
            );
            return;
        }
        if effect.release_delay_us < 0.0 {
            control_line_finding(
                findings,
                scenario,
                &target_component,
                format!(
                    "Control effect {} has a negative release delay.",
                    effect.name
                ),
                None,
            );
            return;
        }
        if let Err(message) = validate_control_effect_endpoint(bound, &target_component, effect) {
            control_line_finding(findings, scenario, &target_component, message, None);
            return;
        }
        if target_effects
            .insert(effect.target.pin.as_str(), effect)
            .is_some()
        {
            control_line_finding(
                findings,
                scenario,
                &target_component,
                format!(
                    "Multiple control effects target {}.{}.",
                    effect.target.component, effect.target.pin
                ),
                None,
            );
            return;
        }
    }

    let reset_pin = scenario
        .target
        .as_ref()
        .and_then(|target| target.reset_pin.as_deref())
        .unwrap_or(reset.pin.as_str());
    let reset_released_state = match normalize_state(&reset.active).as_str() {
        "low" => "high",
        "high" => "low",
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!("Reset active polarity {} is unsupported.", reset.active),
            );
            return;
        }
    };
    let Some(reset_effect) = target_effects.get(reset_pin).copied() else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "No control effect derives reset pin {}.{}.",
                target_component, reset_pin
            ),
        );
        return;
    };
    for sample_time in [timing.reset_release_at_us, boot_sample_at_us] {
        let derived = match derive_control_state_at(scenario, reset_effect, sample_time) {
            Ok(state) => state,
            Err(message) => {
                validation_input_missing(findings, scenario, message);
                return;
            }
        };
        if derived != reset_released_state {
            let mut finding = base_control_line_finding(
                scenario,
                &target_component,
                format!(
                    "Derived reset state {}.{} is not released at {} us.",
                    target_component, reset_pin, sample_time
                ),
            );
            finding
                .measured
                .insert(format!("derived_{reset_pin}"), json!(derived));
            finding
                .measured
                .insert("sample_time_us".to_string(), json!(sample_time));
            finding
                .limit
                .insert(format!("released_{reset_pin}"), json!(reset_released_state));
            findings.push(finding);
            return;
        }
    }

    for requirement in &mode.straps {
        let Some(effect) = target_effects.get(requirement.pin.as_str()).copied() else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "No control effect derives boot strap {}.{}.",
                    target_component, requirement.pin
                ),
            );
            return;
        };
        let derived = match derive_control_state_at(scenario, effect, boot_sample_at_us) {
            Ok(state) => state,
            Err(message) => {
                validation_input_missing(findings, scenario, message);
                return;
            }
        };
        let required_state = normalize_state(&requirement.required_state);
        if derived != required_state {
            let mut finding = base_control_line_finding(
                scenario,
                &target_component,
                format!(
                    "Derived boot strap {}.{} is not valid for boot mode {}.",
                    target_component, requirement.pin, required_boot_mode
                ),
            );
            finding
                .measured
                .insert("required_boot_mode".to_string(), json!(required_boot_mode));
            finding
                .measured
                .insert(format!("derived_{}", requirement.pin), json!(derived));
            finding
                .measured
                .insert("sample_time_us".to_string(), json!(boot_sample_at_us));
            finding.limit.insert(
                format!("required_{}", requirement.pin),
                json!(required_state),
            );
            finding.suggested_fixes = vec![
                "Release or assert the host control line early enough for the target strap to settle before sampling.".to_string(),
                "Reduce the modeled release delay with a discharge or bleed path when supported by the circuit.".to_string(),
                "Use a deterministic reset or boot-control topology when host line choreography is not reliable.".to_string(),
            ];
            findings.push(finding);
            return;
        }
    }
}

fn validate_control_effect_endpoint(
    bound: &BoundBoard<'_>,
    target_component: &str,
    effect: &ControlEffect,
) -> Result<(), String> {
    let Some((_source_model, source_port)) =
        model_port(bound, &effect.source.component, &effect.source.pin)
    else {
        return Err(format!(
            "Control effect {} source {}.{} is unresolved.",
            effect.name, effect.source.component, effect.source.pin
        ));
    };
    if !component_pin_connected(bound, &effect.source) {
        return Err(format!(
            "Control effect {} source {}.{} is not connected in the board pin map.",
            effect.name, effect.source.component, effect.source.pin
        ));
    }
    if !matches!(
        source_port.kind,
        PortKind::DigitalElectricalOutput | PortKind::DigitalElectricalIo
    ) {
        return Err(format!(
            "Control effect {} source {}.{} is not output-capable.",
            effect.name, effect.source.component, effect.source.pin
        ));
    }
    if let Err(message) =
        validate_kicad_pin_direction(bound, &effect.source, PinDirection::Output, "source")
    {
        return Err(format!("Control effect {} {message}", effect.name));
    }
    if effect.target.component != target_component {
        return Err(format!(
            "Control effect {} target {}.{} is not on target component {}.",
            effect.name, effect.target.component, effect.target.pin, target_component
        ));
    }
    let Some((_target_model, target_port)) =
        model_port(bound, &effect.target.component, &effect.target.pin)
    else {
        return Err(format!(
            "Control effect {} target {}.{} is unresolved.",
            effect.name, effect.target.component, effect.target.pin
        ));
    };
    if !component_pin_connected(bound, &effect.target) {
        return Err(format!(
            "Control effect {} target {}.{} is not connected in the board pin map.",
            effect.name, effect.target.component, effect.target.pin
        ));
    }
    if !matches!(
        target_port.kind,
        PortKind::DigitalElectricalInput | PortKind::DigitalElectricalIo
    ) {
        return Err(format!(
            "Control effect {} target {}.{} is not input-capable.",
            effect.name, effect.target.component, effect.target.pin
        ));
    }
    if let Err(message) =
        validate_kicad_pin_direction(bound, &effect.target, PinDirection::Input, "target")
    {
        return Err(format!("Control effect {} {message}", effect.name));
    }
    Ok(())
}

fn derive_control_state_at(
    scenario: &Scenario,
    effect: &ControlEffect,
    sample_time_us: f64,
) -> Result<String, String> {
    let mut last_event = None;
    for event in &scenario.events {
        if event.action == "control_line"
            && event.line.as_deref() == Some(effect.name.as_str())
            && event.at_us <= sample_time_us
        {
            last_event = Some(event);
        }
    }
    let Some(event) = last_event else {
        return Err(format!(
            "Control effect {} has no control_line event at or before {} us.",
            effect.name, sample_time_us
        ));
    };
    let Some(asserted) = event.asserted else {
        return Err(format!(
            "Control effect {} event at {} us is missing asserted.",
            effect.name, event.at_us
        ));
    };
    if asserted || sample_time_us - event.at_us < effect.release_delay_us {
        Ok(normalize_state(&effect.asserted_state))
    } else {
        Ok(normalize_state(&effect.released_state))
    }
}

fn control_line_finding(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    target_component: &str,
    message: impl Into<String>,
    line: Option<&str>,
) {
    let mut finding = base_control_line_finding(scenario, target_component, message);
    if let Some(line) = line {
        finding.limit.insert("line".to_string(), json!(line));
    }
    findings.push(finding);
}

fn base_control_line_finding(
    scenario: &Scenario,
    target_component: &str,
    message: impl Into<String>,
) -> Finding {
    let mut finding = Finding::critical(CONTROL_LINE_RELEASE_SEQUENCE, &scenario.name, message);
    finding.component = Some(target_component.to_string());
    finding.suggested_fixes = vec![
        "Provide explicit control-line events before each evaluated sample time.".to_string(),
        "Adjust line release timing or modeled release delay so reset and boot straps settle before sampling.".to_string(),
    ];
    finding
}
