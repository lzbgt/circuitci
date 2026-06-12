use crate::board_ir::{NetKind, PinLogicState, Scenario};
use crate::library::{
    BoundBoard, SignalConditioningChannel, SignalSupplyConstraint, SignalSupplyRelation,
};
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
    validate_supply_constraints(bound, scenario, &target.component, findings);
    if side_a.powered == side_b.powered {
        return;
    }
    match channel.unpowered_isolation {
        Some(true) => {}
        Some(false) => {
            if channel_disabled_by_observation(bound, scenario, &target.component, channel) {
                return;
            }
            findings.push(unpowered_isolation_finding(
                scenario,
                &target.component,
                channel,
                &side_a,
                &side_b,
                "datasheet metadata says unpowered_isolation is false and the scenario does not prove the channel is disabled",
            ));
        }
        None => {
            if channel_disabled_by_observation(bound, scenario, &target.component, channel) {
                return;
            }
            findings.push(unpowered_isolation_finding(
                scenario,
                &target.component,
                channel,
                &side_a,
                &side_b,
                "datasheet metadata does not declare unpowered_isolation and the scenario does not prove the channel is disabled",
            ));
        }
    }
}

fn channel_disabled_by_observation(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    component_id: &str,
    channel: &SignalConditioningChannel,
) -> bool {
    let Some(enable_pin) = &channel.enable_pin else {
        return false;
    };
    let Some(component) = bound.project.board.components.get(component_id) else {
        return false;
    };
    if !component.pins.contains_key(enable_pin) {
        return false;
    }
    let Some(disabled_state) = channel
        .disabled_state
        .as_deref()
        .and_then(parse_logic_state)
    else {
        return false;
    };
    scenario.pin_states.iter().any(|state| {
        state.component == component_id
            && state.pin == *enable_pin
            && state.state.as_ref() == Some(&disabled_state)
    })
}

fn parse_logic_state(value: &str) -> Option<PinLogicState> {
    match value.trim().to_ascii_lowercase().as_str() {
        "high" => Some(PinLogicState::High),
        "low" => Some(PinLogicState::Low),
        "z" => Some(PinLogicState::Z),
        _ => None,
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

fn validate_supply_constraints(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    component_id: &str,
    findings: &mut Vec<Finding>,
) {
    let Some(component) = bound.project.board.components.get(component_id) else {
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        return;
    };
    for constraint in &model.signal_conditioning.supply_constraints {
        let Some(lower) = resolve_constraint_supply(
            bound,
            scenario,
            component_id,
            constraint,
            &constraint.lower_supply_pin,
            "lower_supply_pin",
            findings,
        ) else {
            continue;
        };
        let Some(upper) = resolve_constraint_supply(
            bound,
            scenario,
            component_id,
            constraint,
            &constraint.upper_supply_pin,
            "upper_supply_pin",
            findings,
        ) else {
            continue;
        };
        if !lower.powered || !upper.powered {
            continue;
        }
        let Some(lower_voltage_v) = lower.nominal_voltage_v else {
            findings.push(supply_constraint_metadata_finding(
                scenario,
                component_id,
                constraint,
                format!(
                    "Signal-conditioning supply constraint {} cannot be checked because rail {} has no nominal_voltage.",
                    constraint.name, lower.net
                ),
                "missing_nominal_voltage_pin",
                &constraint.lower_supply_pin,
            ));
            continue;
        };
        let Some(upper_voltage_v) = upper.nominal_voltage_v else {
            findings.push(supply_constraint_metadata_finding(
                scenario,
                component_id,
                constraint,
                format!(
                    "Signal-conditioning supply constraint {} cannot be checked because rail {} has no nominal_voltage.",
                    constraint.name, upper.net
                ),
                "missing_nominal_voltage_pin",
                &constraint.upper_supply_pin,
            ));
            continue;
        };
        match constraint.relation {
            SignalSupplyRelation::LessThanOrEqual => {
                if lower_voltage_v > upper_voltage_v {
                    findings.push(supply_constraint_violation_finding(
                        scenario,
                        component_id,
                        constraint,
                        &lower,
                        &upper,
                        lower_voltage_v,
                        upper_voltage_v,
                    ));
                }
            }
        }
    }
}

fn resolve_constraint_supply(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    component_id: &str,
    constraint: &SignalSupplyConstraint,
    supply_pin: &str,
    field: &str,
    findings: &mut Vec<Finding>,
) -> Option<ResolvedSupply> {
    let component = bound.project.board.components.get(component_id)?;
    let Some(supply_net_name) = component
        .power_domains
        .get(supply_pin)
        .or_else(|| component.pins.get(supply_pin))
    else {
        findings.push(supply_constraint_metadata_finding(
            scenario,
            component_id,
            constraint,
            format!(
                "Signal-conditioning supply constraint {} pin {} does not resolve to a net.",
                constraint.name, supply_pin
            ),
            field,
            supply_pin,
        ));
        return None;
    };
    let Some(supply_net) = bound.project.board.nets.get(supply_net_name) else {
        findings.push(supply_constraint_metadata_finding(
            scenario,
            component_id,
            constraint,
            format!(
                "Signal-conditioning supply constraint {} net {} is not declared.",
                constraint.name, supply_net_name
            ),
            "missing_supply_net",
            supply_net_name,
        ));
        return None;
    };
    if supply_net.kind != NetKind::Power {
        findings.push(supply_constraint_metadata_finding(
            scenario,
            component_id,
            constraint,
            format!(
                "Signal-conditioning supply constraint {} net {} is not a power net.",
                constraint.name, supply_net_name
            ),
            "invalid_supply_net",
            supply_net_name,
        ));
        return None;
    }
    let Some(powered) = supply_net.powered else {
        findings.push(supply_constraint_metadata_finding(
            scenario,
            component_id,
            constraint,
            format!(
                "Signal-conditioning supply constraint {} rail {} is missing powered state.",
                constraint.name, supply_net_name
            ),
            "missing_supply_powered",
            supply_net_name,
        ));
        return None;
    };
    Some(ResolvedSupply {
        pin: supply_pin.to_string(),
        net: supply_net_name.clone(),
        powered,
        nominal_voltage_v: supply_net.nominal_voltage,
    })
}

struct ResolvedSupply {
    pin: String,
    net: String,
    powered: bool,
    nominal_voltage_v: Option<f64>,
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

fn supply_constraint_metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    constraint: &SignalSupplyConstraint,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(INTERFACE_PROTECTION_REVIEW, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding
        .limit
        .insert("supply_constraint".to_string(), json!(constraint.name));
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Connect every constrained supply pin to a declared power rail before using this interface protection check.".to_string(),
        "Declare nominal_voltage and powered state for constrained rails so datasheet supply-order rules can be checked.".to_string(),
    ];
    finding
}

fn supply_constraint_violation_finding(
    scenario: &Scenario,
    component_id: &str,
    constraint: &SignalSupplyConstraint,
    lower: &ResolvedSupply,
    upper: &ResolvedSupply,
    lower_voltage_v: f64,
    upper_voltage_v: f64,
) -> Finding {
    let mut finding = Finding::critical(
        INTERFACE_PROTECTION_REVIEW,
        &scenario.name,
        format!(
            "Signal-conditioning supply constraint {} on component {} requires {} <= {}, but {:.6} V > {:.6} V.",
            constraint.name,
            component_id,
            constraint.lower_supply_pin,
            constraint.upper_supply_pin,
            lower_voltage_v,
            upper_voltage_v
        ),
    );
    finding.component = Some(component_id.to_string());
    finding
        .measured
        .insert("lower_supply_pin".to_string(), json!(lower.pin));
    finding
        .measured
        .insert("lower_supply_net".to_string(), json!(lower.net));
    finding.measured.insert(
        "lower_nominal_voltage_V".to_string(),
        json!(lower_voltage_v),
    );
    finding
        .measured
        .insert("upper_supply_pin".to_string(), json!(upper.pin));
    finding
        .measured
        .insert("upper_supply_net".to_string(), json!(upper.net));
    finding.measured.insert(
        "upper_nominal_voltage_V".to_string(),
        json!(upper_voltage_v),
    );
    finding
        .limit
        .insert("supply_constraint".to_string(), json!(constraint.name));
    finding.limit.insert(
        "relation".to_string(),
        json!(match constraint.relation {
            SignalSupplyRelation::LessThanOrEqual => "less_than_or_equal",
        }),
    );
    finding.limit.insert(
        "lower_supply_pin".to_string(),
        json!(constraint.lower_supply_pin),
    );
    finding.limit.insert(
        "upper_supply_pin".to_string(),
        json!(constraint.upper_supply_pin),
    );
    finding.suggested_fixes = vec![
        format!(
            "Reassign the rails or choose a compatible level shifter so {} is not above {} during operation.",
            constraint.lower_supply_pin, constraint.upper_supply_pin
        ),
        "If this condition is only transient, add a timing or analog scenario that proves the part remains disabled until the supply-order rule is satisfied.".to_string(),
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
    if let Some(enable_pin) = &channel.enable_pin {
        finding
            .limit
            .insert("enable_pin".to_string(), json!(enable_pin));
    }
    if let Some(disabled_state) = &channel.disabled_state {
        finding
            .limit
            .insert("required_disabled_state".to_string(), json!(disabled_state));
    }
    finding.suggested_fixes = vec![
        "Use a level shifter, bus switch, or protection device whose datasheet guarantees powered-to-unpowered isolation for this rail state.".to_string(),
        "Hold the driving side high impedance until both sides are powered, or add an explicit isolation switch controlled by a valid power-good signal.".to_string(),
        "Add a GPIO_BACKDRIVE or analog_transient scenario for the unpowered condition when the datasheet does not guarantee isolation.".to_string(),
    ];
    if let (Some(enable_pin), Some(disabled_state)) = (&channel.enable_pin, &channel.disabled_state)
    {
        finding.suggested_fixes.insert(
            0,
            format!(
                "Prove {component_id}.{enable_pin} is held {disabled_state} whenever only one side of channel {} is powered, or change the control circuit so that state is guaranteed by power sequencing.",
                channel.name
            ),
        );
    }
    finding
}
