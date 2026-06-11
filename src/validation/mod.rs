use crate::board_ir::{BoardProject, Endpoint, PinLogicState, PinMode, Scenario};
use crate::library::{
    BoundBoard, ComponentModel, Port, PortKind, ProtocolBehavior, ProtocolOperation,
};
use crate::reports::{EndpointPair, Finding, Limitation};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const GPIO_BACKDRIVE: &str = "GPIO_BACKDRIVE";
const RESET_RELEASE_AFTER_POWER_VALID: &str = "RESET_RELEASE_AFTER_POWER_VALID";
const BOOT_STRAP_DEFINED: &str = "BOOT_STRAP_DEFINED";
const UART_BOOTLOADER_SYNC: &str = "UART_BOOTLOADER_SYNC";
const RESIDENT_BOOTLOADER_UPDATE_SEQUENCE: &str = "RESIDENT_BOOTLOADER_UPDATE_SEQUENCE";
const SUPPORTED_SCENARIO_TYPES: &[&str] = &[
    "gpio_backdrive",
    "reset_boot",
    "serial_programming",
    "firmware_update",
];

pub fn validate(bound: &BoundBoard<'_>) -> (Vec<Finding>, Vec<Limitation>) {
    let mut findings = bound.findings.clone();
    let mut limitations = model_quality_limitations(bound);
    let mut added_backdrive_limitation = false;
    let mut added_protocol_limitation = false;

    for scenario in &bound.project.scenarios {
        if !SUPPORTED_SCENARIO_TYPES.contains(&scenario.scenario_type.as_str()) {
            limitations.push(Limitation {
                id: "UNSUPPORTED_SCENARIO".to_string(),
                scope: format!("scenario:{}", scenario.name),
                confidence: "low".to_string(),
                blocking: true,
                message: format!(
                    "Scenario type {} is not implemented in this runtime.",
                    scenario.scenario_type
                ),
            });
        }

        let mut seen = BTreeSet::new();
        for check in &scenario.checks {
            if !seen.insert(check) {
                continue;
            }
            match check.as_str() {
                GPIO_BACKDRIVE if scenario.scenario_type == "gpio_backdrive" => {
                    if !added_backdrive_limitation {
                        limitations.push(Limitation {
                            id: "SIMPLE_BACKDRIVE_MODEL".to_string(),
                            scope: "validation:GPIO_BACKDRIVE".to_string(),
                            confidence: "medium".to_string(),
                            blocking: false,
                            message: "GPIO_BACKDRIVE uses a deterministic behavioral approximation, not an analog solver waveform.".to_string(),
                        });
                        added_backdrive_limitation = true;
                    }
                    validate_backdrive(bound, scenario, &mut findings)
                }
                RESET_RELEASE_AFTER_POWER_VALID if scenario.scenario_type == "reset_boot" => {
                    validate_reset_release(bound, scenario, &mut findings)
                }
                BOOT_STRAP_DEFINED if scenario.scenario_type == "reset_boot" => {
                    validate_boot_straps(bound, scenario, &mut findings)
                }
                UART_BOOTLOADER_SYNC if scenario.scenario_type == "serial_programming" => {
                    validate_uart_bootloader_sync(bound, scenario, &mut findings)
                }
                RESIDENT_BOOTLOADER_UPDATE_SEQUENCE
                    if scenario.scenario_type == "firmware_update" =>
                {
                    if !added_protocol_limitation {
                        limitations.push(Limitation {
                            id: "ABSTRACT_PROTOCOL_TRACE".to_string(),
                            scope: format!("validation:{RESIDENT_BOOTLOADER_UPDATE_SEQUENCE}"),
                            confidence: "medium".to_string(),
                            blocking: false,
                            message: "Resident protocol validation checks declared transaction traces; it does not execute firmware, decode raw frames, recompute CRCs, or prove HIL behavior.".to_string(),
                        });
                        added_protocol_limitation = true;
                    }
                    validate_resident_bootloader_update(bound, scenario, &mut findings)
                }
                GPIO_BACKDRIVE
                | RESET_RELEASE_AFTER_POWER_VALID
                | BOOT_STRAP_DEFINED
                | UART_BOOTLOADER_SYNC
                | RESIDENT_BOOTLOADER_UPDATE_SEQUENCE => findings.push(Finding::critical(
                    "CHECK_SCENARIO_TYPE_MISMATCH",
                    &scenario.name,
                    format!(
                        "Check {check} is not valid for scenario type {}.",
                        scenario.scenario_type
                    ),
                )),
                other => limitations.push(Limitation {
                    id: "UNSUPPORTED_CHECK".to_string(),
                    scope: format!("scenario:{}:check:{other}", scenario.name),
                    confidence: "low".to_string(),
                    blocking: true,
                    message: format!("Check {other} is not implemented in this runtime."),
                }),
            }
        }
    }

    (findings, limitations)
}

fn model_quality_limitations(bound: &BoundBoard<'_>) -> Vec<Limitation> {
    let mut limitations = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let source = model.model_quality.source.as_str();
        let confidence = model.model_quality.confidence.as_str();
        if matches!(source, "estimated" | "generic") || confidence == "low" {
            limitations.push(Limitation {
                id: "LOW_CONFIDENCE_MODEL".to_string(),
                scope: format!("component:{component_id}:model:{}", model.component_id),
                confidence: model.model_quality.confidence.clone(),
                blocking: false,
                message: format!(
                    "Component {component_id} uses {} model {} with {} confidence.",
                    model.model_quality.source, model.component_id, model.model_quality.confidence
                ),
            });
        }
    }
    limitations
}

fn validate_backdrive(bound: &BoundBoard<'_>, scenario: &Scenario, findings: &mut Vec<Finding>) {
    let diode_drop_v = scenario
        .parameters
        .get("diode_drop_V")
        .and_then(serde_yaml_ng::Value::as_f64)
        .unwrap_or(0.3);

    for path in &scenario.paths {
        let Some(driver_state) = scenario
            .pin_states
            .iter()
            .find(|state| state.component == path.driver.component && state.pin == path.driver.pin)
        else {
            findings.push(Finding::warning(
                "PIN_STATE_MISSING",
                &scenario.name,
                format!(
                    "Missing pin state for driver {}.{}.",
                    path.driver.component, path.driver.pin
                ),
            ));
            continue;
        };
        if driver_state.mode != PinMode::Output || driver_state.state != Some(PinLogicState::High) {
            continue;
        }

        let Some(victim_state) = scenario
            .pin_states
            .iter()
            .find(|state| state.component == path.victim.component && state.pin == path.victim.pin)
        else {
            findings.push(Finding::warning(
                "PIN_STATE_MISSING",
                &scenario.name,
                format!(
                    "Missing pin state for victim {}.{}.",
                    path.victim.component, path.victim.pin
                ),
            ));
            continue;
        };
        if victim_state.mode != PinMode::Input {
            continue;
        }

        let Some(net) = shared_net(bound.project, &path.driver, &path.victim) else {
            findings.push(Finding::warning(
                "BACKDRIVE_PATH_NET_MISMATCH",
                &scenario.name,
                format!(
                    "Backdrive path {}.{} -> {}.{} is not on one shared net.",
                    path.driver.component, path.driver.pin, path.victim.component, path.victim.pin
                ),
            ));
            continue;
        };

        let Some((driver_model, driver_port)) =
            model_port(bound, &path.driver.component, &path.driver.pin)
        else {
            findings.push(Finding::warning(
                "DRIVER_PORT_NOT_FOUND",
                &scenario.name,
                format!(
                    "Driver port {}.{} is unresolved.",
                    path.driver.component, path.driver.pin
                ),
            ));
            continue;
        };
        let Some((victim_model, victim_port)) =
            model_port(bound, &path.victim.component, &path.victim.pin)
        else {
            findings.push(Finding::warning(
                "VICTIM_PORT_NOT_FOUND",
                &scenario.name,
                format!(
                    "Victim port {}.{} is unresolved.",
                    path.victim.component, path.victim.pin
                ),
            ));
            continue;
        };

        if !matches!(
            driver_port.kind,
            PortKind::DigitalElectricalOutput | PortKind::DigitalElectricalIo
        ) {
            findings.push(Finding::warning(
                "DRIVER_KIND_INVALID",
                &scenario.name,
                format!(
                    "Driver {}.{} is not an output-capable port.",
                    path.driver.component, path.driver.pin
                ),
            ));
            continue;
        }
        if !matches!(
            victim_port.kind,
            PortKind::DigitalElectricalInput | PortKind::DigitalElectricalIo
        ) {
            findings.push(Finding::warning(
                "VICTIM_KIND_INVALID",
                &scenario.name,
                format!(
                    "Victim {}.{} is not an input-capable port.",
                    path.victim.component, path.victim.pin
                ),
            ));
            continue;
        }

        let Some(driver_high_v) = driver_port.electrical.drive_high_voltage_v else {
            missing_electrical(
                findings,
                &scenario.name,
                "drive_high_voltage_V",
                &path.driver,
            );
            continue;
        };
        let Some(source_ohm) = driver_port.electrical.source_impedance_ohm else {
            missing_electrical(
                findings,
                &scenario.name,
                "source_impedance_ohm",
                &path.driver,
            );
            continue;
        };
        let Some(limit_a) = victim_port.electrical.injection_current_limit_a else {
            missing_electrical(
                findings,
                &scenario.name,
                "injection_current_limit_A",
                &path.victim,
            );
            continue;
        };
        let Some(victim_rail_v) =
            component_power_voltage(bound, &path.victim.component, victim_model)
        else {
            findings.push(Finding::warning(
                "VICTIM_POWER_UNKNOWN",
                &scenario.name,
                format!(
                    "Victim component {} power voltage is unknown.",
                    path.victim.component
                ),
            ));
            continue;
        };
        let Some(driver_rail_v) =
            component_power_voltage(bound, &path.driver.component, driver_model)
        else {
            findings.push(Finding::warning(
                "DRIVER_POWER_UNKNOWN",
                &scenario.name,
                format!(
                    "Driver component {} power voltage is unknown.",
                    path.driver.component
                ),
            ));
            continue;
        };
        if driver_rail_v <= 0.0 {
            continue;
        }

        let effective_ohm = source_ohm + path.series_resistance_ohm;
        if effective_ohm <= 0.0 {
            findings.push(Finding::warning(
                "INVALID_BACKDRIVE_RESISTANCE",
                &scenario.name,
                "Backdrive effective resistance must be greater than zero.",
            ));
            continue;
        }
        let injection_current_a =
            ((driver_high_v - victim_rail_v - diode_drop_v).max(0.0)) / effective_ohm;
        if injection_current_a > limit_a {
            let mut measured = BTreeMap::new();
            measured.insert(
                "injection_current_A".to_string(),
                json!(injection_current_a),
            );
            measured.insert("driver_high_voltage_V".to_string(), json!(driver_high_v));
            measured.insert("victim_rail_voltage_V".to_string(), json!(victim_rail_v));
            measured.insert("effective_resistance_ohm".to_string(), json!(effective_ohm));
            let mut limit = BTreeMap::new();
            limit.insert("injection_current_A".to_string(), json!(limit_a));

            let mut finding = Finding::critical(
                GPIO_BACKDRIVE,
                &scenario.name,
                format!(
                    "Powered component {}.{} drives unpowered component {}.{} on net {net}.",
                    path.driver.component, path.driver.pin, path.victim.component, path.victim.pin
                ),
            );
            finding.component = Some(path.victim.component.clone());
            finding.net = Some(net.to_string());
            finding.endpoints = Some(EndpointPair {
                driver: path.driver.clone(),
                victim: path.victim.clone(),
            });
            finding.measured = measured;
            finding.limit = limit;
            finding.suggested_fixes = vec![
                "Add a series resistor sized to keep injection current below the receiving pin limit.".to_string(),
                "Add a bus switch or isolation device.".to_string(),
                "Ensure both components are in the same powered domain before driving the net.".to_string(),
                "Configure the driving pin as high impedance while the receiving component is unpowered.".to_string(),
            ];
            findings.push(finding);
        }
    }
}

fn validate_resident_bootloader_update(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some((target_component, model)) = target_model(bound, scenario) else {
        validation_input_missing(
            findings,
            scenario,
            "firmware_update target component and model are required.",
        );
        return;
    };
    let Some(protocol_scenario) = &scenario.protocol else {
        validation_input_missing(findings, scenario, "protocol block is required.");
        return;
    };
    if protocol_scenario
        .component
        .as_ref()
        .is_some_and(|component| component != &target_component)
    {
        validation_input_missing(
            findings,
            scenario,
            "protocol.component must match target.component.",
        );
        return;
    }
    let Some(protocol) = model.behavior.protocols.get(&protocol_scenario.name) else {
        protocol_finding(
            findings,
            scenario,
            &target_component,
            format!(
                "Component model {} does not declare protocol {}.",
                model.component_id, protocol_scenario.name
            ),
            None,
        );
        return;
    };
    let Some(flow) = protocol.flows.get(&protocol_scenario.flow) else {
        protocol_finding(
            findings,
            scenario,
            &target_component,
            format!(
                "Protocol {} does not declare flow {}.",
                protocol_scenario.name, protocol_scenario.flow
            ),
            None,
        );
        return;
    };

    if let Some(transport_interface) = &protocol.transport_interface {
        let Some(sender) = &protocol_scenario.sender else {
            validation_input_missing(
                findings,
                scenario,
                "protocol.sender is required when a transport_interface is declared.",
            );
            return;
        };
        let Some(bootloader) = &model.behavior.bootloader else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Component model {} does not declare transport interfaces.",
                    model.component_id
                ),
            );
            return;
        };
        let Some(interface) = bootloader.interfaces.get(transport_interface) else {
            protocol_finding(
                findings,
                scenario,
                &target_component,
                format!(
                    "Protocol transport interface {transport_interface} is not declared by model {}.",
                    model.component_id
                ),
                None,
            );
            return;
        };
        if let Err(finding) = validate_sender_endpoint(
            bound,
            scenario,
            RESIDENT_BOOTLOADER_UPDATE_SEQUENCE,
            sender,
            &target_component,
            &interface.rx_pin,
        ) {
            findings.push(*finding);
            return;
        }
    }

    let events: Vec<_> = scenario
        .events
        .iter()
        .filter(|event| event.action == "protocol_request")
        .collect();
    if events.len() != scenario.events.len() {
        protocol_finding(
            findings,
            scenario,
            &target_component,
            "firmware_update scenarios only support protocol_request events.",
            None,
        );
        return;
    }
    if events.is_empty() {
        validation_input_missing(findings, scenario, "protocol_request events are required.");
        return;
    }

    let ok_result = protocol.frame.ok_result.unwrap_or(0);
    for event in &events {
        let Some(operation_name) = &event.operation else {
            validation_input_missing(
                findings,
                scenario,
                "protocol_request.operation is required.",
            );
            return;
        };
        let Some(operation) = protocol.operations.get(operation_name) else {
            protocol_finding(
                findings,
                scenario,
                &target_component,
                format!("Operation {operation_name} is not declared by the protocol model."),
                None,
            );
            return;
        };
        if event.result_code != Some(ok_result) {
            let mut finding = base_protocol_finding(
                scenario,
                &target_component,
                format!("Operation {operation_name} did not return the expected success result."),
            );
            finding
                .measured
                .insert("result_code".to_string(), json!(event.result_code));
            finding
                .limit
                .insert("ok_result".to_string(), json!(ok_result));
            findings.push(finding);
            return;
        }
        if !payload_len_valid(protocol, operation, event.payload_len) {
            let mut finding = base_protocol_finding(
                scenario,
                &target_component,
                format!("Operation {operation_name} payload length is outside model limits."),
            );
            finding
                .measured
                .insert("payload_len".to_string(), json!(event.payload_len));
            if let Some(max_payload_len) = protocol.frame.max_payload_len {
                finding
                    .limit
                    .insert("max_payload_len".to_string(), json!(max_payload_len));
            }
            if let Some(payload) = &operation.payload {
                if let Some(len) = payload.len {
                    finding.limit.insert("payload_len".to_string(), json!(len));
                }
                if let Some(min_len) = payload.min_len {
                    finding
                        .limit
                        .insert("min_payload_len".to_string(), json!(min_len));
                }
                if let Some(max_len) = payload.max_len {
                    finding
                        .limit
                        .insert("max_payload_len_for_operation".to_string(), json!(max_len));
                }
            }
            findings.push(finding);
            return;
        }
        if !activate_mode_valid(operation, event.activate_mode.as_deref()) {
            let mut finding = base_protocol_finding(
                scenario,
                &target_component,
                format!("Operation {operation_name} activate mode is not declared by the model."),
            );
            finding
                .measured
                .insert("activate_mode".to_string(), json!(event.activate_mode));
            findings.push(finding);
            return;
        }
    }

    if let Err(message) = flow_matches(protocol, scenario, &events) {
        protocol_finding(
            findings,
            scenario,
            &target_component,
            message,
            Some(&protocol_scenario.flow),
        );
        return;
    }

    if let Err(message) = transfer_trace_valid(protocol, scenario, &events) {
        protocol_finding(findings, scenario, &target_component, message, None);
        return;
    }

    let expected_final_state = protocol_scenario
        .expected_final_state
        .as_ref()
        .or(flow.final_state.as_ref());
    if let Some(expected_final_state) = expected_final_state {
        let observed = events.iter().rev().find_map(|event| event.state.as_ref());
        if observed != Some(expected_final_state) {
            let mut finding = base_protocol_finding(
                scenario,
                &target_component,
                "Final observed protocol state does not match the expected state.",
            );
            finding
                .measured
                .insert("observed_final_state".to_string(), json!(observed));
            finding.limit.insert(
                "expected_final_state".to_string(),
                json!(expected_final_state),
            );
            findings.push(finding);
        }
    }
}

fn payload_len_valid(
    protocol: &ProtocolBehavior,
    operation: &ProtocolOperation,
    payload_len: Option<u64>,
) -> bool {
    let Some(payload_len) = payload_len else {
        return operation.payload.is_none();
    };
    if protocol
        .frame
        .max_payload_len
        .is_some_and(|max| payload_len > max)
    {
        return false;
    }
    let Some(payload) = &operation.payload else {
        return true;
    };
    if payload.len.is_some_and(|len| payload_len != len) {
        return false;
    }
    if payload.min_len.is_some_and(|min| payload_len < min) {
        return false;
    }
    if payload.max_len.is_some_and(|max| payload_len > max) {
        return false;
    }
    true
}

fn activate_mode_valid(operation: &ProtocolOperation, activate_mode: Option<&str>) -> bool {
    let Some(activate_mode) = activate_mode else {
        return true;
    };
    operation
        .payload
        .as_ref()
        .is_some_and(|payload| payload.values.contains_key(activate_mode))
}

fn flow_matches(
    protocol: &ProtocolBehavior,
    scenario: &Scenario,
    events: &[&crate::board_ir::ScenarioEvent],
) -> Result<(), String> {
    let protocol_scenario = scenario.protocol.as_ref().expect("protocol exists");
    let flow = protocol
        .flows
        .get(&protocol_scenario.flow)
        .expect("flow exists");
    let mut event_index = 0;
    for phase in &flow.phases {
        if !protocol.operations.contains_key(&phase.operation) {
            return Err(format!(
                "Flow {} references undeclared operation {}.",
                protocol_scenario.flow, phase.operation
            ));
        }
        let mut matched = 0usize;
        while events
            .get(event_index)
            .and_then(|event| event.operation.as_ref())
            == Some(&phase.operation)
        {
            matched += 1;
            event_index += 1;
            if phase.repeat.is_none() {
                break;
            }
        }
        match phase.repeat.as_deref() {
            Some("zero_or_more") => {}
            Some("one_or_more") if matched > 0 => {}
            Some("one_or_more") => {
                return Err(format!(
                    "Flow {} requires at least one {} operation.",
                    protocol_scenario.flow, phase.operation
                ));
            }
            Some(other) => {
                return Err(format!(
                    "Flow {} uses unsupported repeat mode {other}.",
                    protocol_scenario.flow
                ));
            }
            None if matched == 1 => {}
            None => {
                return Err(format!(
                    "Flow {} expected operation {} at position {}.",
                    protocol_scenario.flow,
                    phase.operation,
                    event_index + 1
                ));
            }
        }
    }
    if event_index != events.len() {
        return Err(format!(
            "Flow {} did not consume all protocol events.",
            protocol_scenario.flow
        ));
    }
    Ok(())
}

fn transfer_trace_valid(
    protocol: &ProtocolBehavior,
    scenario: &Scenario,
    events: &[&crate::board_ir::ScenarioEvent],
) -> Result<(), String> {
    let protocol_scenario = scenario.protocol.as_ref().expect("protocol exists");
    let package_size = protocol_scenario
        .package_size_bytes
        .ok_or_else(|| "protocol.package_size_bytes is required.".to_string())?;
    if protocol_scenario.package_sha256.is_none() {
        return Err("protocol.package_sha256 is required.".to_string());
    }

    let mut chunks = Vec::new();
    let mut saw_start = false;
    let mut saw_finish = false;
    for event in events {
        let Some(operation_name) = &event.operation else {
            continue;
        };
        let Some(operation) = protocol.operations.get(operation_name) else {
            continue;
        };
        match operation.role.as_deref() {
            Some("start_transfer") => saw_start = true,
            Some("finish_transfer") => saw_finish = true,
            Some("data_chunk") => {
                let offset = event
                    .offset
                    .ok_or_else(|| format!("Data operation {operation_name} is missing offset."))?;
                let chunk_len = event.chunk_len.ok_or_else(|| {
                    format!("Data operation {operation_name} is missing chunk_len.")
                })?;
                let payload = operation.payload.as_ref().ok_or_else(|| {
                    format!("Data operation {operation_name} lacks payload metadata.")
                })?;
                let overhead = payload.overhead_len.ok_or_else(|| {
                    format!("Data operation {operation_name} lacks overhead_len metadata.")
                })?;
                let expected_payload_len = overhead
                    .checked_add(chunk_len)
                    .ok_or_else(|| "Data payload length overflows u64.".to_string())?;
                if event.payload_len != Some(expected_payload_len) {
                    return Err(format!(
                        "Data operation {operation_name} payload length does not equal overhead_len + chunk_len."
                    ));
                }
                let end = offset
                    .checked_add(chunk_len)
                    .ok_or_else(|| "Data chunk offset overflows u64.".to_string())?;
                if end > package_size {
                    return Err(format!(
                        "Data operation {operation_name} writes beyond package_size_bytes."
                    ));
                }
                chunks.push((offset, end));
            }
            _ => {}
        }
    }
    if !saw_start {
        return Err("Transfer start operation is missing.".to_string());
    }
    if !saw_finish {
        return Err("Transfer finish operation is missing.".to_string());
    }
    if chunks.is_empty() {
        return Err("At least one data chunk operation is required.".to_string());
    }
    chunks.sort_unstable();
    let mut cursor = 0;
    for (start, end) in chunks {
        if start != cursor {
            return Err("Data chunks do not completely cover package_size_bytes.".to_string());
        }
        cursor = end;
    }
    if cursor != package_size {
        return Err("Data chunks do not completely cover package_size_bytes.".to_string());
    }
    Ok(())
}

fn protocol_finding(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    target_component: &str,
    message: impl Into<String>,
    flow: Option<&str>,
) {
    let mut finding = base_protocol_finding(scenario, target_component, message);
    if let Some(flow) = flow {
        finding.limit.insert("flow".to_string(), json!(flow));
    }
    findings.push(finding);
}

fn base_protocol_finding(
    scenario: &Scenario,
    target_component: &str,
    message: impl Into<String>,
) -> Finding {
    let mut finding =
        Finding::critical(RESIDENT_BOOTLOADER_UPDATE_SEQUENCE, &scenario.name, message);
    finding.component = Some(target_component.to_string());
    finding.suggested_fixes = vec![
        "Align the protocol scenario trace with the component model protocol flow.".to_string(),
        "Correct payload lengths, result codes, chunk coverage, or sender connectivity before relying on the update path.".to_string(),
    ];
    finding
}

fn validate_reset_release(
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

fn validate_reset_target_assertions(
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

fn validate_boot_straps(bound: &BoundBoard<'_>, scenario: &Scenario, findings: &mut Vec<Finding>) {
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

fn validate_uart_bootloader_sync(
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

fn validate_sender_endpoint(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    rule_id: &str,
    from: &Endpoint,
    target_component: &str,
    target_rx_pin: &str,
) -> Result<(), Box<Finding>> {
    let Some((_sender_model, sender_port)) = model_port(bound, &from.component, &from.pin) else {
        let mut finding = Finding::critical(
            rule_id,
            &scenario.name,
            format!(
                "Sender endpoint {}.{} is unresolved.",
                from.component, from.pin
            ),
        );
        finding.component = Some(from.component.clone());
        finding.suggested_fixes = vec![
            "Use a sender component and pin declared in the board and component model.".to_string(),
        ];
        return Err(Box::new(finding));
    };
    if !matches!(
        sender_port.kind,
        PortKind::DigitalElectricalOutput | PortKind::DigitalElectricalIo
    ) {
        let mut finding = Finding::critical(
            rule_id,
            &scenario.name,
            format!(
                "Sender endpoint {}.{} is not output-capable.",
                from.component, from.pin
            ),
        );
        finding.component = Some(from.component.clone());
        finding.suggested_fixes =
            vec!["Use the USB-UART transmit pin as the event sender.".to_string()];
        return Err(Box::new(finding));
    }

    let target_endpoint = Endpoint {
        component: target_component.to_string(),
        pin: target_rx_pin.to_string(),
    };
    if shared_net(bound.project, from, &target_endpoint).is_none() {
        let mut finding = Finding::critical(
            rule_id,
            &scenario.name,
            format!(
                "Sender {}.{} is not connected to target RX {}.{}.",
                from.component, from.pin, target_component, target_rx_pin
            ),
        );
        finding.component = Some(target_component.to_string());
        finding.endpoints = Some(EndpointPair {
            driver: from.clone(),
            victim: target_endpoint,
        });
        finding
            .limit
            .insert("target_rx_pin".to_string(), json!(target_rx_pin));
        finding.suggested_fixes = vec![
            "Cross USB-UART TXD to the target bootloader RX pin.".to_string(),
            "Correct the board netlist if the schematic already has the intended connection."
                .to_string(),
        ];
        return Err(Box::new(finding));
    }
    Ok(())
}

fn shared_net<'a>(
    project: &'a BoardProject,
    driver: &Endpoint,
    victim: &Endpoint,
) -> Option<&'a str> {
    let driver_net = project.net_for_pin(&driver.component, &driver.pin)?;
    let victim_net = project.net_for_pin(&victim.component, &victim.pin)?;
    (driver_net == victim_net).then_some(driver_net)
}

fn model_port<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &str,
    pin: &str,
) -> Option<(&'a ComponentModel, &'a Port)> {
    let component = bound.project.board.components.get(component_id)?;
    let model = bound.library.get(&component.model)?;
    let port = model.ports.get(pin)?;
    Some((model, port))
}

fn component_power_voltage(
    bound: &BoundBoard<'_>,
    component_id: &str,
    model: &ComponentModel,
) -> Option<f64> {
    let component = bound.project.board.components.get(component_id)?;
    let power_port = model
        .ports
        .iter()
        .find(|(_, port)| port.kind == PortKind::ElectricalPower)
        .map(|(name, _)| name)?;
    let net_name = component
        .power_domains
        .get(power_port)
        .or_else(|| component.pins.get(power_port))
        .or(component.power_domain.as_ref())?;
    let net = bound.project.board.nets.get(net_name)?;
    match net.powered {
        Some(true) => net.nominal_voltage,
        Some(false) => Some(0.0),
        None => None,
    }
}

fn missing_electrical(
    findings: &mut Vec<Finding>,
    scenario: &str,
    field: &str,
    endpoint: &Endpoint,
) {
    findings.push(Finding::warning(
        "ELECTRICAL_METADATA_MISSING",
        scenario,
        format!(
            "Missing {field} for {}.{}.",
            endpoint.component, endpoint.pin
        ),
    ));
}

fn target_model<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
) -> Option<(String, &'a ComponentModel)> {
    let target = scenario.target.as_ref()?;
    let component = bound.project.board.components.get(&target.component)?;
    let model = bound.library.get(&component.model)?;
    Some((target.component.clone(), model))
}

fn validation_input_missing(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    message: impl Into<String>,
) {
    let mut finding = Finding::critical("VALIDATION_INPUT_MISSING", &scenario.name, message);
    finding.suggested_fixes = vec![
        "Add the missing scenario or component-model data required by the declared check."
            .to_string(),
        "Do not declare a validation check until its required inputs are modeled.".to_string(),
    ];
    findings.push(finding);
}

fn normalize_state(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
