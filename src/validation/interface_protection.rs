use crate::board_ir::{NetKind, PinLogicState, Scenario};
use crate::library::{
    BoundBoard, ProtectionClamp, ProtectionReference, SignalConditioningChannel,
    SignalSupplyConstraint, SignalSupplyRelation, UsbConnector,
};
use crate::reports::Finding;
use serde_json::json;

use super::INTERFACE_PROTECTION_REVIEW;
use super::USB_CONNECTOR_PROTECTION_VALID;
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
    if let Some(clamp_name) = scenario
        .parameters
        .get("clamp")
        .and_then(serde_yaml_ng::Value::as_str)
    {
        validate_protection_clamp(bound, scenario, &target.component, clamp_name, findings);
        return;
    }

    let Some(channel_name) = scenario
        .parameters
        .get("channel")
        .and_then(serde_yaml_ng::Value::as_str)
    else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.channel or parameters.clamp is required.",
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

pub(super) fn validate_usb_connector_protection(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_CONNECTOR_PROTECTION_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(connector) = &model.usb_connector else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no usb_connector metadata.",
                target.component, component.model
            ),
            "usb_connector",
            &component.model,
        ));
        return;
    };

    validate_usb_connector_pin(
        bound,
        scenario,
        &target.component,
        component,
        connector,
        UsbConnectorSignal::Dp,
        findings,
    );
    validate_usb_connector_pin(
        bound,
        scenario,
        &target.component,
        component,
        connector,
        UsbConnectorSignal::Dm,
        findings,
    );

    if scenario_bool_parameter(scenario, "require_vbus_protection").unwrap_or(false) {
        validate_usb_connector_pin(
            bound,
            scenario,
            &target.component,
            component,
            connector,
            UsbConnectorSignal::Vbus,
            findings,
        );
    }
}

#[derive(Debug, Clone, Copy)]
enum UsbConnectorSignal {
    Dp,
    Dm,
    Vbus,
}

impl UsbConnectorSignal {
    fn label(self) -> &'static str {
        match self {
            Self::Dp => "D+",
            Self::Dm => "D-",
            Self::Vbus => "VBUS",
        }
    }

    fn pin(self, connector: &UsbConnector) -> &str {
        match self {
            Self::Dp => &connector.dp_pin,
            Self::Dm => &connector.dm_pin,
            Self::Vbus => &connector.vbus_pin,
        }
    }
}

fn validate_usb_connector_pin(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    connector_id: &str,
    component: &crate::board_ir::ComponentSpec,
    connector: &UsbConnector,
    signal: UsbConnectorSignal,
    findings: &mut Vec<Finding>,
) {
    let pin = signal.pin(connector);
    let Some(net_name) = component.pins.get(pin) else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} {} pin {pin} is not connected.",
                signal.label()
            ),
            "missing_pin",
            pin,
        ));
        return;
    };
    if !bound.project.board.nets.contains_key(net_name) {
        findings.push(usb_connector_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} {} net {net_name} is not declared.",
                signal.label()
            ),
            "missing_net",
            net_name,
        ));
        return;
    }
    if let Some(protection) = find_valid_clamp_for_net(bound, connector_id, net_name) {
        if let Some(min_standoff_v) = scenario_numeric_parameter(
            scenario,
            match signal {
                UsbConnectorSignal::Vbus => "vbus_working_voltage_min_V",
                UsbConnectorSignal::Dp | UsbConnectorSignal::Dm => "data_working_voltage_min_V",
            },
            findings,
        ) && let Some(working_voltage_max_v) = protection.clamp.working_voltage_max_v
            && working_voltage_max_v < min_standoff_v
        {
            findings.push(usb_connector_standoff_finding(
                scenario,
                connector_id,
                signal,
                net_name,
                &protection,
                working_voltage_max_v,
                min_standoff_v,
            ));
        }
        return;
    }
    findings.push(usb_connector_missing_protection_finding(
        scenario,
        connector_id,
        signal,
        pin,
        net_name,
    ));
}

struct ResolvedUsbProtection<'a> {
    component_id: &'a str,
    clamp: &'a ProtectionClamp,
    reference_net_name: &'a str,
    reference_net_kind: &'a NetKind,
}

fn find_valid_clamp_for_net<'a>(
    bound: &'a BoundBoard<'_>,
    connector_id: &str,
    net_name: &str,
) -> Option<ResolvedUsbProtection<'a>> {
    for (component_id, component) in &bound.project.board.components {
        if component_id == connector_id {
            continue;
        }
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        for clamp in &model.signal_conditioning.protection_clamps {
            let Some(protected_net) = component.pins.get(&clamp.protected_pin) else {
                continue;
            };
            if protected_net != net_name {
                continue;
            }
            let Some(reference_net_name) = component.pins.get(&clamp.reference_pin) else {
                continue;
            };
            let Some(reference_net) = bound.project.board.nets.get(reference_net_name) else {
                continue;
            };
            let expected_kind = match clamp.reference {
                ProtectionReference::Ground => NetKind::Ground,
                ProtectionReference::Power => NetKind::Power,
            };
            if reference_net.kind == expected_kind {
                return Some(ResolvedUsbProtection {
                    component_id,
                    clamp,
                    reference_net_name,
                    reference_net_kind: &reference_net.kind,
                });
            }
        }
    }
    None
}

fn validate_protection_clamp(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    component_id: &str,
    clamp_name: &str,
    findings: &mut Vec<Finding>,
) {
    let Some(component) = bound.project.board.components.get(component_id) else {
        findings.push(metadata_finding(
            scenario,
            component_id,
            format!("Interface protection target component {component_id} is not declared."),
            "component",
            component_id,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(metadata_finding(
            scenario,
            component_id,
            format!(
                "Interface protection target component {component_id} model {} is not loaded.",
                component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(clamp) = model
        .signal_conditioning
        .protection_clamps
        .iter()
        .find(|clamp| clamp.name == clamp_name)
    else {
        findings.push(metadata_finding(
            scenario,
            component_id,
            format!(
                "Component {component_id} model {} has no signal_conditioning protection_clamp {clamp_name}.",
                component.model
            ),
            "clamp",
            clamp_name,
        ));
        return;
    };

    let Some(protected_net_name) = component.pins.get(&clamp.protected_pin) else {
        findings.push(protection_clamp_metadata_finding(
            scenario,
            component_id,
            clamp,
            format!(
                "Protection clamp {} protected pin {} is not connected.",
                clamp.name, clamp.protected_pin
            ),
            "missing_protected_pin",
            &clamp.protected_pin,
        ));
        return;
    };
    let Some(protected_net) = bound.project.board.nets.get(protected_net_name) else {
        findings.push(protection_clamp_metadata_finding(
            scenario,
            component_id,
            clamp,
            format!(
                "Protection clamp {} protected net {} is not declared.",
                clamp.name, protected_net_name
            ),
            "missing_protected_net",
            protected_net_name,
        ));
        return;
    };
    let Some(reference_net_name) = component.pins.get(&clamp.reference_pin) else {
        findings.push(protection_clamp_metadata_finding(
            scenario,
            component_id,
            clamp,
            format!(
                "Protection clamp {} reference pin {} is not connected.",
                clamp.name, clamp.reference_pin
            ),
            "missing_reference_pin",
            &clamp.reference_pin,
        ));
        return;
    };
    let Some(reference_net) = bound.project.board.nets.get(reference_net_name) else {
        findings.push(protection_clamp_metadata_finding(
            scenario,
            component_id,
            clamp,
            format!(
                "Protection clamp {} reference net {} is not declared.",
                clamp.name, reference_net_name
            ),
            "missing_reference_net",
            reference_net_name,
        ));
        return;
    };
    let expected_kind = match clamp.reference {
        ProtectionReference::Ground => NetKind::Ground,
        ProtectionReference::Power => NetKind::Power,
    };
    if reference_net.kind != expected_kind {
        findings.push(protection_reference_finding(
            scenario,
            component_id,
            clamp,
            reference_net_name,
            &reference_net.kind,
        ));
    }

    if let Some(working_voltage_max_v) = clamp.working_voltage_max_v {
        if !working_voltage_max_v.is_finite() || working_voltage_max_v <= 0.0 {
            findings.push(protection_clamp_metadata_finding(
                scenario,
                component_id,
                clamp,
                format!(
                    "Protection clamp {} working_voltage_max_V must be finite and positive.",
                    clamp.name
                ),
                "working_voltage_max_V",
                &clamp.name,
            ));
        } else if let Some(nominal_voltage_v) = protected_net.nominal_voltage
            && nominal_voltage_v.is_finite()
            && nominal_voltage_v > working_voltage_max_v
        {
            findings.push(protection_working_voltage_finding(
                scenario,
                component_id,
                clamp,
                protected_net_name,
                nominal_voltage_v,
                working_voltage_max_v,
            ));
        }
    }

    if let Some(line_capacitance_f) = clamp.line_capacitance_f {
        if !line_capacitance_f.is_finite() || line_capacitance_f < 0.0 {
            findings.push(protection_clamp_metadata_finding(
                scenario,
                component_id,
                clamp,
                format!(
                    "Protection clamp {} line_capacitance_F must be finite and non-negative.",
                    clamp.name
                ),
                "line_capacitance_F",
                &clamp.name,
            ));
            return;
        }
        if let Some(max_line_capacitance_f) =
            scenario_numeric_parameter(scenario, "max_line_capacitance_F", findings)
            && line_capacitance_f > max_line_capacitance_f
        {
            findings.push(protection_capacitance_finding(
                scenario,
                component_id,
                clamp,
                protected_net_name,
                line_capacitance_f,
                max_line_capacitance_f,
            ));
        }
    }
}

fn scenario_numeric_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let value = scenario.parameters.get(name)?;
    let Some(number) = value.as_f64() else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be numeric when declared."),
        );
        return None;
    };
    if !number.is_finite() || number < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be finite and non-negative."),
        );
        return None;
    }
    Some(number)
}

fn scenario_bool_parameter(scenario: &Scenario, name: &str) -> Option<bool> {
    scenario
        .parameters
        .get(name)
        .and_then(serde_yaml_ng::Value::as_bool)
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

fn protection_clamp_metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    clamp: &ProtectionClamp,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(INTERFACE_PROTECTION_REVIEW, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding
        .limit
        .insert("protection_clamp".to_string(), json!(clamp.name));
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Declare the protection clamp pins and reference net before using this review check.".to_string(),
        "Do not treat a clamp-only protection part as validated until its datasheet standoff voltage and capacitance are modeled.".to_string(),
    ];
    finding
}

fn protection_reference_finding(
    scenario: &Scenario,
    component_id: &str,
    clamp: &ProtectionClamp,
    reference_net: &str,
    actual_kind: &NetKind,
) -> Finding {
    let expected = match clamp.reference {
        ProtectionReference::Ground => "ground",
        ProtectionReference::Power => "power",
    };
    let actual = match actual_kind {
        NetKind::Power => "power",
        NetKind::Ground => "ground",
        NetKind::DigitalOrAnalog => "digital_or_analog",
    };
    let mut finding = Finding::critical(
        INTERFACE_PROTECTION_REVIEW,
        &scenario.name,
        format!(
            "Protection clamp {} on component {} reference pin {} is connected to {actual} net {}, expected {expected}.",
            clamp.name, component_id, clamp.reference_pin, reference_net
        ),
    );
    finding.component = Some(component_id.to_string());
    finding.net.replace(reference_net.to_string());
    finding
        .measured
        .insert("reference_net_kind".to_string(), json!(actual));
    finding
        .limit
        .insert("protection_clamp".to_string(), json!(clamp.name));
    finding
        .limit
        .insert("reference_pin".to_string(), json!(clamp.reference_pin));
    finding
        .limit
        .insert("required_reference".to_string(), json!(expected));
    finding.suggested_fixes = vec![
        format!(
            "Connect {}.{} to a declared {expected} net according to the protection device datasheet.",
            component_id, clamp.reference_pin
        ),
        "Use a different clamp model if this part references another rail instead of ground."
            .to_string(),
    ];
    finding
}

fn protection_working_voltage_finding(
    scenario: &Scenario,
    component_id: &str,
    clamp: &ProtectionClamp,
    protected_net: &str,
    nominal_voltage_v: f64,
    working_voltage_max_v: f64,
) -> Finding {
    let mut finding = Finding::critical(
        INTERFACE_PROTECTION_REVIEW,
        &scenario.name,
        format!(
            "Protection clamp {} on component {} sees protected net {} at {:.6} V, above standoff limit {:.6} V.",
            clamp.name, component_id, protected_net, nominal_voltage_v, working_voltage_max_v
        ),
    );
    finding.component = Some(component_id.to_string());
    finding.net = Some(protected_net.to_string());
    finding.measured.insert(
        "protected_net_nominal_voltage_V".to_string(),
        json!(nominal_voltage_v),
    );
    finding
        .limit
        .insert("protection_clamp".to_string(), json!(clamp.name));
    finding.limit.insert(
        "working_voltage_max_V".to_string(),
        json!(working_voltage_max_v),
    );
    finding.suggested_fixes = vec![
        "Select an ESD/protection device with reverse standoff voltage above the protected signal's normal voltage.".to_string(),
        "Correct the protected net nominal_voltage if this scenario represents a lower-voltage operating state.".to_string(),
    ];
    finding
}

fn protection_capacitance_finding(
    scenario: &Scenario,
    component_id: &str,
    clamp: &ProtectionClamp,
    protected_net: &str,
    line_capacitance_f: f64,
    max_line_capacitance_f: f64,
) -> Finding {
    let mut finding = Finding::critical(
        INTERFACE_PROTECTION_REVIEW,
        &scenario.name,
        format!(
            "Protection clamp {} on component {} has {:.3e} F line capacitance on {}, above interface limit {:.3e} F.",
            clamp.name, component_id, line_capacitance_f, protected_net, max_line_capacitance_f
        ),
    );
    finding.component = Some(component_id.to_string());
    finding.net = Some(protected_net.to_string());
    finding
        .measured
        .insert("line_capacitance_F".to_string(), json!(line_capacitance_f));
    finding
        .limit
        .insert("protection_clamp".to_string(), json!(clamp.name));
    finding.limit.insert(
        "max_line_capacitance_F".to_string(),
        json!(max_line_capacitance_f),
    );
    finding.suggested_fixes = vec![
        "Select a lower-capacitance ESD/protection device for this interface.".to_string(),
        "Raise max_line_capacitance_F only when the interface budget and signal-integrity analysis allow the added capacitance.".to_string(),
    ];
    finding
}

fn usb_connector_metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(USB_CONNECTOR_PROTECTION_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Declare usb_connector metadata and connect every required USB connector pin before using this protection check.".to_string(),
        "Use explicit protection-clamp models on exposed USB nets instead of treating connector exposure as implicitly protected.".to_string(),
    ];
    finding
}

fn usb_connector_missing_protection_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    pin: &str,
    net: &str,
) -> Finding {
    let mut finding = Finding::critical(
        USB_CONNECTOR_PROTECTION_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} pin {pin} on net {net} has no valid protection clamp coverage.",
            signal.label()
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_pin".to_string(), json!(pin));
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .limit
        .insert("required_protection_clamp".to_string(), json!(true));
    finding.suggested_fixes = vec![
        format!(
            "Add a datasheet-backed ESD/protection component whose protected pin connects to USB connector {connector_id}.{pin} net {net}."
        ),
        "Place the protection device close to the USB connector in PCB layout and add explicit clamp-review scenarios for standoff voltage and capacitance.".to_string(),
    ];
    finding
}

fn usb_connector_standoff_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    protection: &ResolvedUsbProtection<'_>,
    working_voltage_max_v: f64,
    min_standoff_v: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_CONNECTOR_PROTECTION_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} is protected by {}.{}, but clamp standoff {:.6} V is below required {:.6} V.",
            signal.label(),
            protection.component_id,
            protection.clamp.name,
            working_voltage_max_v,
            min_standoff_v
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding.measured.insert(
        "protection_component".to_string(),
        json!(protection.component_id),
    );
    finding
        .measured
        .insert("protection_clamp".to_string(), json!(protection.clamp.name));
    finding.measured.insert(
        "reference_pin".to_string(),
        json!(protection.clamp.reference_pin),
    );
    finding.measured.insert(
        "reference_net".to_string(),
        json!(protection.reference_net_name),
    );
    finding.measured.insert(
        "reference_net_kind".to_string(),
        json!(format!("{:?}", protection.reference_net_kind).to_lowercase()),
    );
    finding.measured.insert(
        "working_voltage_max_V".to_string(),
        json!(working_voltage_max_v),
    );
    finding.limit.insert(
        "required_working_voltage_min_V".to_string(),
        json!(min_standoff_v),
    );
    finding.suggested_fixes = vec![
        "Select a protection device whose reverse standoff voltage covers the exposed USB connector operating voltage.".to_string(),
        "Use separate VBUS-rated protection for the VBUS pin when the data-line ESD part is not rated for the power rail.".to_string(),
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
