use crate::board_ir::{FirmwareBackend, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::FUNCTIONAL_MCU_FIRMWARE;
use super::common::{target_model, validation_input_missing};

pub(super) fn validate_functional_mcu_firmware(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some((target_component, model)) = target_model(bound, scenario) else {
        validation_input_missing(
            findings,
            scenario,
            "firmware_in_loop target component and model are required.",
        );
        return;
    };
    let Some(firmware) = &scenario.firmware else {
        validation_input_missing(
            findings,
            scenario,
            "firmware_in_loop firmware block is required.",
        );
        return;
    };
    if firmware.image.trim().is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "firmware_in_loop firmware.image is required.",
        );
        return;
    }
    if firmware.expected_pin_states.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "firmware_in_loop firmware.expected_pin_states must declare board-facing pin behavior to validate.",
        );
        return;
    }

    let mut finding = Finding::critical(
        FUNCTIONAL_MCU_FIRMWARE,
        &scenario.name,
        "Functional MCU firmware validation requires a firmware runtime backend, but no Renode/QEMU adapter is available in this runtime.",
    );
    finding.component = Some(target_component.clone());
    finding
        .measured
        .insert("target_component".to_string(), json!(target_component));
    finding
        .measured
        .insert("target_model".to_string(), json!(model.component_id));
    finding.measured.insert(
        "backend".to_string(),
        json!(firmware_backend(&firmware.backend)),
    );
    finding
        .measured
        .insert("firmware_image".to_string(), json!(firmware.image));
    if let Some(machine) = &firmware.machine {
        finding
            .measured
            .insert("machine".to_string(), json!(machine));
    }
    finding.measured.insert(
        "expected_pin_states".to_string(),
        json!(firmware.expected_pin_states.len()),
    );
    finding.limit.insert(
        "functional_blackbox_boundary".to_string(),
        json!("firmware-visible peripherals and board-facing pin behavior"),
    );
    finding
        .limit
        .insert("transistor_level_mcu_required".to_string(), json!(false));
    finding.suggested_fixes = vec![
        "Add a supported functional MCU firmware backend, such as a Renode or QEMU adapter, before relying on firmware-in-loop results.".to_string(),
        "Keep transistor-level MCU internals out of this check; validate board-facing pin behavior, reset/boot state, and peripheral effects instead.".to_string(),
        "Use existing reset/boot, protocol, control-line, and analog SPICE checks for narrower validation until firmware-in-loop support is available.".to_string(),
    ];
    findings.push(finding);
}

fn firmware_backend(backend: &FirmwareBackend) -> &'static str {
    match backend {
        FirmwareBackend::Auto => "auto",
        FirmwareBackend::Renode => "renode",
        FirmwareBackend::Qemu => "qemu",
    }
}
