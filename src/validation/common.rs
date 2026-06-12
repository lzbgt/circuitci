use crate::board_ir::{BoardProject, Endpoint, Scenario};
use crate::library::{BoundBoard, ComponentModel, Port, PortKind};
use crate::reports::{EndpointPair, Finding};
use serde_json::json;

pub(super) fn validate_sender_endpoint(
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
    if let Err(message) =
        validate_kicad_pin_direction(bound, from, PinDirection::Output, "sender endpoint")
    {
        let mut finding = Finding::critical(rule_id, &scenario.name, message);
        finding.component = Some(from.component.clone());
        finding.suggested_fixes = vec![
            "Use a schematic pin whose KiCad electrical type is output-capable for the sender."
                .to_string(),
            "Correct the KiCad symbol pin electrical type only when the symbol metadata is wrong."
                .to_string(),
        ];
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

#[derive(Debug, Clone, Copy)]
pub(super) enum PinDirection {
    Input,
    Output,
}

pub(super) fn validate_kicad_pin_direction(
    bound: &BoundBoard<'_>,
    endpoint: &Endpoint,
    required: PinDirection,
    role: &str,
) -> Result<(), String> {
    let Some(electrical_type) = kicad_pin_electrical_type(bound, endpoint) else {
        return Ok(());
    };
    if kicad_pin_type_supports_direction(electrical_type, required) {
        return Ok(());
    }
    let required_text = match required {
        PinDirection::Input => "input-capable",
        PinDirection::Output => "output-capable",
    };
    Err(format!(
        "{role} {}.{} has KiCad electrical type {}, which is not {required_text}.",
        endpoint.component, endpoint.pin, electrical_type
    ))
}

pub(super) fn kicad_pin_direction_capable(
    bound: &BoundBoard<'_>,
    endpoint: &Endpoint,
    required: PinDirection,
) -> bool {
    kicad_pin_electrical_type(bound, endpoint)
        .is_none_or(|electrical_type| kicad_pin_type_supports_direction(electrical_type, required))
}

fn kicad_pin_electrical_type<'a>(
    bound: &'a BoundBoard<'_>,
    endpoint: &Endpoint,
) -> Option<&'a str> {
    bound
        .project
        .board
        .components
        .get(&endpoint.component)
        .and_then(|component| component.source.as_ref())
        .and_then(|source| source.board_pin_electrical_types.get(&endpoint.pin))
        .map(String::as_str)
}

fn kicad_pin_type_supports_direction(electrical_type: &str, required: PinDirection) -> bool {
    let normalized = normalize_kicad_pin_electrical_type(electrical_type);
    match required {
        PinDirection::Input => {
            matches!(normalized.as_str(), "input" | "bidirectional" | "tri_state")
        }
        PinDirection::Output => matches!(
            normalized.as_str(),
            "output"
                | "bidirectional"
                | "tri_state"
                | "power_out"
                | "open_collector"
                | "open_emitter"
        ),
    }
}

fn normalize_kicad_pin_electrical_type(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

pub(super) fn shared_net<'a>(
    project: &'a BoardProject,
    driver: &Endpoint,
    victim: &Endpoint,
) -> Option<&'a str> {
    let driver_net = project.net_for_pin(&driver.component, &driver.pin)?;
    let victim_net = project.net_for_pin(&victim.component, &victim.pin)?;
    (driver_net == victim_net).then_some(driver_net)
}

pub(super) fn model_port<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &str,
    pin: &str,
) -> Option<(&'a ComponentModel, &'a Port)> {
    let component = bound.project.board.components.get(component_id)?;
    let model = bound.library.get(&component.model)?;
    let port = model.ports.get(pin)?;
    Some((model, port))
}

pub(super) fn component_pin_connected(bound: &BoundBoard<'_>, endpoint: &Endpoint) -> bool {
    bound
        .project
        .board
        .components
        .get(&endpoint.component)
        .is_some_and(|component| component.pins.contains_key(&endpoint.pin))
}

pub(super) fn component_power_voltage(
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

pub(super) fn target_model<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
) -> Option<(String, &'a ComponentModel)> {
    let target = scenario.target.as_ref()?;
    let component = bound.project.board.components.get(&target.component)?;
    let model = bound.library.get(&component.model)?;
    Some((target.component.clone(), model))
}

pub(super) fn validation_input_missing(
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

pub(super) fn normalize_state(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
