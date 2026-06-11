use crate::board_ir::Scenario;
use crate::library::{BoundBoard, ProtocolBehavior, ProtocolOperation};
use crate::reports::Finding;
use serde_json::json;

use super::RESIDENT_BOOTLOADER_UPDATE_SEQUENCE;
use super::common::{target_model, validate_sender_endpoint, validation_input_missing};

pub(super) fn validate_resident_bootloader_update(
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
