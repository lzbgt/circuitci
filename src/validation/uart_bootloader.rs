use crate::board_ir::Scenario;
use crate::library::{BoundBoard, PortKind};
use crate::reports::Finding;
use serde_json::json;

use super::UART_BOOTLOADER_SYNC;
use super::common::{model_port, target_model, validate_sender_endpoint, validation_input_missing};
use super::target_contract::validate_boot_straps;

pub(super) fn validate_uart_bootloader_sync(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some((target_component, model)) = target_model(bound, scenario) else {
        validation_input_missing(
            findings,
            scenario,
            "serial_programming target component and model are required.",
        );
        return;
    };
    let Some(bootloader) = &scenario.bootloader else {
        validation_input_missing(findings, scenario, "bootloader block is required.");
        return;
    };
    if bootloader
        .component
        .as_ref()
        .is_some_and(|component| component != &target_component)
    {
        validation_input_missing(
            findings,
            scenario,
            "bootloader.component must match target.component.",
        );
        return;
    }
    let Some(behavior) = &model.behavior.bootloader else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Component model {} does not declare bootloader interfaces.",
                model.component_id
            ),
        );
        return;
    };
    let Some(interface) = behavior.interfaces.get(&bootloader.interface) else {
        let mut finding = Finding::critical(
            UART_BOOTLOADER_SYNC,
            &scenario.name,
            format!(
                "Component model {} does not support bootloader interface {}.",
                model.component_id, bootloader.interface
            ),
        );
        finding.component = Some(target_component.clone());
        finding
            .limit
            .insert("interface".to_string(), json!(bootloader.interface));
        finding.suggested_fixes = vec![
            "Select a bootloader interface supported by the target component model.".to_string(),
            "Add a model-library bootloader interface only when the datasheet supports it."
                .to_string(),
        ];
        findings.push(finding);
        return;
    };
    let Some((_target_model, target_rx_port)) =
        model_port(bound, &target_component, &interface.rx_pin)
    else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Target RX pin {}.{} is not declared and connected.",
                target_component, interface.rx_pin
            ),
        );
        return;
    };
    if !matches!(
        target_rx_port.kind,
        PortKind::DigitalElectricalInput | PortKind::DigitalElectricalIo
    ) {
        let mut finding = Finding::critical(
            UART_BOOTLOADER_SYNC,
            &scenario.name,
            format!(
                "Target RX pin {}.{} is not input-capable.",
                target_component, interface.rx_pin
            ),
        );
        finding.component = Some(target_component.clone());
        finding
            .limit
            .insert("rx_pin".to_string(), json!(interface.rx_pin));
        findings.push(finding);
        return;
    }
    if bootloader
        .sync_byte
        .is_some_and(|sync_byte| sync_byte != interface.sync_byte)
        || bootloader
            .expected_response
            .is_some_and(|ack_byte| ack_byte != interface.ack_byte)
    {
        let mut finding = Finding::critical(
            UART_BOOTLOADER_SYNC,
            &scenario.name,
            "Scenario bootloader sync/ACK bytes conflict with the target model.",
        );
        finding.component = Some(target_component.clone());
        finding
            .measured
            .insert("interface".to_string(), json!(bootloader.interface));
        finding
            .limit
            .insert("sync_byte".to_string(), json!(interface.sync_byte));
        finding
            .limit
            .insert("expected_response".to_string(), json!(interface.ack_byte));
        finding.suggested_fixes = vec![
            "Use the sync and ACK bytes declared by the component model datasheet metadata."
                .to_string(),
        ];
        findings.push(finding);
        return;
    }

    let min_event_time = scenario
        .timing
        .as_ref()
        .and_then(|timing| timing.boot_sample_at_us)
        .unwrap_or(0.0);
    let mut sync_candidate_problem = None;
    let sync_event_found = scenario.events.iter().any(|event| {
        if event.action != "uart_send"
            || event.at_us < min_event_time
            || event.bytes != [interface.sync_byte]
        {
            return false;
        }
        let Some(to) = &event.to else {
            return false;
        };
        if to.component != target_component || to.pin != interface.rx_pin {
            return false;
        }

        match validate_uart_sender(bound, scenario, event, &target_component, &interface.rx_pin) {
            Ok(()) => true,
            Err(finding) => {
                sync_candidate_problem.get_or_insert(*finding);
                false
            }
        }
    });
    if !sync_event_found {
        if let Some(finding) = sync_candidate_problem {
            findings.push(finding);
            return;
        }
        let mut finding = Finding::critical(
            UART_BOOTLOADER_SYNC,
            &scenario.name,
            format!(
                "No UART bootloader sync byte was sent to {}.{}.",
                target_component, interface.rx_pin
            ),
        );
        finding.component = Some(target_component.clone());
        finding
            .measured
            .insert("interface".to_string(), json!(bootloader.interface));
        finding
            .measured
            .insert("sync_event_found".to_string(), json!(false));
        finding
            .limit
            .insert("sync_byte".to_string(), json!(interface.sync_byte));
        finding
            .limit
            .insert("expected_response".to_string(), json!(interface.ack_byte));
        finding
            .limit
            .insert("rx_pin".to_string(), json!(interface.rx_pin));
        if let Some(required_boot_mode) = &scenario.required_boot_mode {
            finding
                .limit
                .insert("required_boot_mode".to_string(), json!(required_boot_mode));
        }
        finding.suggested_fixes = vec![
            "Send the model-declared bootloader sync byte to the target RX pin after boot sampling."
                .to_string(),
            "Check USB-UART TX/RX crossing and target boot mode straps.".to_string(),
        ];
        findings.push(finding);
        return;
    }

    if let Some(required_boot_mode) = &scenario.required_boot_mode {
        let mut local_findings = Vec::new();
        validate_boot_straps(bound, scenario, &mut local_findings);
        if !local_findings.is_empty() {
            findings.extend(local_findings);
        }
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
        if !boot.modes.contains_key(required_boot_mode) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Component model {} does not declare boot mode {}.",
                    model.component_id, required_boot_mode
                ),
            );
        }
    }
}

fn validate_uart_sender(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    event: &crate::board_ir::ScenarioEvent,
    target_component: &str,
    target_rx_pin: &str,
) -> Result<(), Box<Finding>> {
    let Some(from) = &event.from else {
        let mut finding = Finding::critical(
            UART_BOOTLOADER_SYNC,
            &scenario.name,
            "UART bootloader sync event is missing the sender endpoint.",
        );
        finding.component = Some(target_component.to_string());
        finding.limit.insert(
            "required_sender_endpoint".to_string(),
            json!("event.from.component and event.from.pin"),
        );
        finding.suggested_fixes = vec![
            "Declare the USB-UART TX endpoint that sends the sync byte.".to_string(),
            "Ensure the sender endpoint is connected to the target RX net.".to_string(),
        ];
        return Err(Box::new(finding));
    };
    validate_sender_endpoint(
        bound,
        scenario,
        UART_BOOTLOADER_SYNC,
        from,
        target_component,
        target_rx_pin,
    )
}
