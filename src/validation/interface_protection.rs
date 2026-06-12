use crate::board_ir::{NetKind, Scenario};
use crate::library::{BoundBoard, SignalConditioningChannel};
use crate::reports::Finding;
use serde_json::json;

use super::INTERFACE_PROTECTION_REVIEW;
use super::common::validation_input_missing;

pub(super) fn validate_interface_protection(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required.",
        );
        return;
    };
    let Some(channel_name) = scenario
        .parameters
        .get("channel")
        .and_then(serde_yaml_ng::Value::as_str)
    else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.channel is required.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(metadata_finding(
            scenario,
            &target.component,
            format!(
                "Interface protection target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(metadata_finding(
            scenario,
            &target.component,
            format!(
                "Interface protection target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(channel) = model
        .signal_conditioning
        .channels
        .iter()
        .find(|channel| channel.name == channel_name)
    else {
        findings.push(metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no signal_conditioning channel {}.",
                target.component, component.model, channel_name
            ),
            "channel",
            channel_name,
        ));
        return;
    };

    let Some(side_a) = resolve_side(
        bound,
        scenario,
        &target.component,
        channel,
        Side::A,
        findings,
    ) else {
        return;
    };
    let Some(side_b) = resolve_side(
        bound,
        scenario,
        &target.component,
        channel,
        Side::B,
        findings,
    ) else {
        return;
    };
    if side_a.powered == side_b.powered {
        return;
    }
    match channel.unpowered_isolation {
        Some(true) => {}
        Some(false) => findings.push(unpowered_isolation_finding(
            scenario,
            &target.component,
            channel,
            &side_a,
            &side_b,
            "datasheet metadata says unpowered_isolation is false",
        )),
        None => findings.push(unpowered_isolation_finding(
            scenario,
            &target.component,
            channel,
            &side_a,
            &side_b,
            "datasheet metadata does not declare unpowered_isolation",
        )),
    }
}

#[derive(Debug, Clone, Copy)]
enum Side {
    A,
    B,
}

struct ResolvedSide {
    pin: String,
    net: String,
    supply_pin: String,
    supply_net: String,
    powered: bool,
}

fn resolve_side(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    component_id: &str,
    channel: &SignalConditioningChannel,
    side: Side,
    findings: &mut Vec<Finding>,
) -> Option<ResolvedSide> {
    let component = bound.project.board.components.get(component_id)?;
    let (pin, supply_pin) = match side {
        Side::A => (&channel.side_a_pin, channel.side_a_supply_pin.as_deref()),
        Side::B => (&channel.side_b_pin, channel.side_b_supply_pin.as_deref()),
    };
    let side_name = match side {
        Side::A => "side_a",
        Side::B => "side_b",
    };
    let Some(net) = component.pins.get(pin) else {
        findings.push(metadata_finding(
            scenario,
            component_id,
            format!(
                "Interface protection channel {} {side_name} pin {} is not connected.",
                channel.name, pin
            ),
            "missing_pin",
            pin,
        ));
        return None;
    };
    let Some(supply_pin) = supply_pin else {
        findings.push(metadata_finding(
            scenario,
            component_id,
            format!(
                "Interface protection channel {} {side_name} is missing supply pin metadata.",
                channel.name
            ),
            "missing_supply_pin",
            side_name,
        ));
        return None;
    };
    let Some(supply_net_name) = component
        .power_domains
        .get(supply_pin)
        .or_else(|| component.pins.get(supply_pin))
    else {
        findings.push(metadata_finding(
            scenario,
            component_id,
            format!(
                "Interface protection channel {} {side_name} supply pin {} does not resolve to a net.",
                channel.name, supply_pin
            ),
            "missing_supply_net",
            supply_pin,
        ));
        return None;
    };
    let Some(supply_net) = bound.project.board.nets.get(supply_net_name) else {
        findings.push(metadata_finding(
            scenario,
            component_id,
            format!(
                "Interface protection channel {} {side_name} supply net {} is not declared.",
                channel.name, supply_net_name
            ),
            "missing_supply_net",
            supply_net_name,
        ));
        return None;
    };
    if supply_net.kind != NetKind::Power {
        findings.push(metadata_finding(
            scenario,
            component_id,
            format!(
                "Interface protection channel {} {side_name} supply net {} is not a power net.",
                channel.name, supply_net_name
            ),
            "invalid_supply_net",
            supply_net_name,
        ));
        return None;
    }
    let Some(powered) = supply_net.powered else {
        findings.push(metadata_finding(
            scenario,
            component_id,
            format!(
                "Interface protection channel {} {side_name} supply net {} is missing powered state.",
                channel.name, supply_net_name
            ),
            "missing_supply_powered",
            supply_net_name,
        ));
        return None;
    };
    Some(ResolvedSide {
        pin: pin.clone(),
        net: net.clone(),
        supply_pin: supply_pin.to_string(),
        supply_net: supply_net_name.clone(),
        powered,
    })
}

fn metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(INTERFACE_PROTECTION_REVIEW, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Declare the signal-conditioning channel, pins, supply pins, and rail powered states before using this review check.".to_string(),
        "Do not treat an interface protection part as validated until its datasheet conditions are modeled.".to_string(),
    ];
    finding
}

fn unpowered_isolation_finding(
    scenario: &Scenario,
    component_id: &str,
    channel: &SignalConditioningChannel,
    side_a: &ResolvedSide,
    side_b: &ResolvedSide,
    reason: &str,
) -> Finding {
    let mut finding = Finding::critical(
        INTERFACE_PROTECTION_REVIEW,
        &scenario.name,
        format!(
            "Interface protection channel {} on component {} connects powered/unpowered domains, but {reason}.",
            channel.name, component_id
        ),
    );
    finding.component = Some(component_id.to_string());
    finding
        .measured
        .insert("side_a_pin".to_string(), json!(side_a.pin));
    finding
        .measured
        .insert("side_a_net".to_string(), json!(side_a.net));
    finding
        .measured
        .insert("side_a_supply_pin".to_string(), json!(side_a.supply_pin));
    finding
        .measured
        .insert("side_a_supply_net".to_string(), json!(side_a.supply_net));
    finding
        .measured
        .insert("side_a_powered".to_string(), json!(side_a.powered));
    finding
        .measured
        .insert("side_b_pin".to_string(), json!(side_b.pin));
    finding
        .measured
        .insert("side_b_net".to_string(), json!(side_b.net));
    finding
        .measured
        .insert("side_b_supply_pin".to_string(), json!(side_b.supply_pin));
    finding
        .measured
        .insert("side_b_supply_net".to_string(), json!(side_b.supply_net));
    finding
        .measured
        .insert("side_b_powered".to_string(), json!(side_b.powered));
    finding
        .limit
        .insert("required_unpowered_isolation".to_string(), json!(true));
    finding.suggested_fixes = vec![
        "Use a level shifter, bus switch, or protection device whose datasheet guarantees powered-to-unpowered isolation for this rail state.".to_string(),
        "Hold the driving side high impedance until both sides are powered, or add an explicit isolation switch controlled by a valid power-good signal.".to_string(),
        "Add a GPIO_BACKDRIVE or analog_transient scenario for the unpowered condition when the datasheet does not guarantee isolation.".to_string(),
    ];
    finding
}
