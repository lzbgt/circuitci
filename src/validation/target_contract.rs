use crate::board_ir::Scenario;
use crate::library::{BoundBoard, PortKind};
use crate::reports::Finding;
use serde_json::json;

use super::common::{normalize_state, target_model, validation_input_missing};
use super::{BOOT_STRAP_DEFINED, RESET_RELEASE_AFTER_POWER_VALID};

pub(super) fn validate_reset_release(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "reset_boot target.component is required.",
        );
        return;
    };
    validate_reset_target_assertions(bound, scenario, findings);
    let Some(timing) = &scenario.timing else {
        validation_input_missing(findings, scenario, "reset_boot timing is required.");
        return;
    };

    let margin_us = timing.reset_release_at_us - timing.power_valid_at_us;
    if margin_us < 0.0 {
        let mut finding = Finding::critical(
            RESET_RELEASE_AFTER_POWER_VALID,
            &scenario.name,
            format!(
                "Reset releases before power is valid for component {}.",
                target.component
            ),
        );
        finding.component = Some(target.component.clone());
        finding.measured.insert(
            "power_valid_at_us".to_string(),
            json!(timing.power_valid_at_us),
        );
        finding.measured.insert(
            "reset_release_at_us".to_string(),
            json!(timing.reset_release_at_us),
        );
        finding
            .measured
            .insert("margin_us".to_string(), json!(margin_us));
        finding.limit.insert(
            "reset_release_not_before_power_valid".to_string(),
            json!(true),
        );
        finding.suggested_fixes = vec![
            "Delay reset release until the MCU operating rail is valid.".to_string(),
            "Increase reset RC delay or use a supervisor IC.".to_string(),
            "Tie reset release to regulator power-good when available.".to_string(),
        ];
        findings.push(finding);
    }
}

pub(super) fn validate_reset_target_assertions(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        validation_input_missing(
            findings,
            scenario,
            format!("Target component {} is not declared.", target.component),
        );
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        validation_input_missing(
            findings,
            scenario,
            format!("Target component {} model is unresolved.", target.component),
        );
        return;
    };

    if let Some(reset_pin) = &target.reset_pin {
        if let Some(reset) = &model.behavior.reset {
            if reset.pin != *reset_pin {
                let mut finding = Finding::critical(
                    "TARGET_RESET_PIN_MISMATCH",
                    &scenario.name,
                    format!(
                        "Scenario reset pin {}.{} does not match model reset pin {}.",
                        target.component, reset_pin, reset.pin
                    ),
                );
                finding.component = Some(target.component.clone());
                finding
                    .measured
                    .insert("scenario_reset_pin".to_string(), json!(reset_pin));
                finding
                    .limit
                    .insert("model_reset_pin".to_string(), json!(reset.pin));
                finding.suggested_fixes = vec![
                    "Use the reset pin declared by the target component model.".to_string(),
                    "Correct the component model only when the datasheet identifies a different reset pin."
                        .to_string(),
                ];
                findings.push(finding);
            }
        } else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Component model {} does not declare reset behavior.",
                    model.component_id
                ),
            );
        }

        match model.ports.get(reset_pin) {
            Some(port)
                if matches!(
                    port.kind,
                    PortKind::DigitalElectricalInput | PortKind::DigitalElectricalIo
                ) => {}
            Some(_) => findings.push(Finding::critical(
                "TARGET_RESET_PIN_KIND_INVALID",
                &scenario.name,
                format!(
                    "Scenario reset pin {}.{} is not input-capable.",
                    target.component, reset_pin
                ),
            )),
            None => validation_input_missing(
                findings,
                scenario,
                format!(
                    "Scenario reset pin {}.{} is not declared by model {}.",
                    target.component, reset_pin, model.component_id
                ),
            ),
        }

        if !component.pins.contains_key(reset_pin) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Scenario reset pin {}.{} is not connected in the board pin map.",
                    target.component, reset_pin
                ),
            );
        }
    }

    if let Some(power_pin) = &target.power_pin {
        match model.ports.get(power_pin) {
            Some(port) if port.kind == PortKind::ElectricalPower => {}
            Some(_) => findings.push(Finding::critical(
                "TARGET_POWER_PIN_KIND_INVALID",
                &scenario.name,
                format!(
                    "Scenario power pin {}.{} is not an electrical power port.",
                    target.component, power_pin
                ),
            )),
            None => validation_input_missing(
                findings,
                scenario,
                format!(
                    "Scenario power pin {}.{} is not declared by model {}.",
                    target.component, power_pin, model.component_id
                ),
            ),
        }

        let rail = component
            .power_domains
            .get(power_pin)
            .or_else(|| component.pins.get(power_pin))
            .or(component.power_domain.as_ref());
        if rail.is_none() {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Scenario power pin {}.{} does not resolve to a board rail.",
                    target.component, power_pin
                ),
            );
        }
    }
}

pub(super) fn validate_boot_straps(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some((target_component, model)) = target_model(bound, scenario) else {
        validation_input_missing(
            findings,
            scenario,
            "reset_boot target component and model are required for boot strap validation.",
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

    for requirement in &mode.straps {
        let observed = scenario
            .straps
            .iter()
            .find(|strap| strap.component == target_component && strap.pin == requirement.pin);
        let observed_state = observed.map(|strap| normalize_state(&strap.actual));
        let required_state = normalize_state(&requirement.required_state);
        let failed = match observed_state.as_deref() {
            None | Some("floating" | "undefined") => true,
            Some(actual) => actual != required_state,
        };
        if failed {
            let mut finding = Finding::critical(
                BOOT_STRAP_DEFINED,
                &scenario.name,
                format!(
                    "Boot strap {}.{} is not valid for boot mode {}.",
                    target_component, requirement.pin, required_boot_mode
                ),
            );
            finding.component = Some(target_component.clone());
            if let Some(net) = observed.and_then(|strap| strap.net.clone()) {
                finding.net = Some(net);
            }
            finding
                .measured
                .insert("required_boot_mode".to_string(), json!(required_boot_mode));
            finding.measured.insert(
                format!("observed_{}", requirement.pin),
                json!(observed_state.unwrap_or_else(|| "missing".to_string())),
            );
            finding.limit.insert(
                format!("required_{}", requirement.pin),
                json!(required_state),
            );
            finding.suggested_fixes = vec![
                "Set the boot strap resistor network to the required state during sampling."
                    .to_string(),
                "Avoid leaving boot strap pins floating or in the undefined region.".to_string(),
                "Check reset timing so straps are stable before the boot sample time.".to_string(),
            ];
            findings.push(finding);
        }
    }
}
