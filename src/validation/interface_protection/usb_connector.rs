use crate::board_ir::{
    ComponentPlacement, ComponentSpec, NetKind, Scenario, UsbConnectorLayoutRule,
};
use crate::library::{BoundBoard, ProtectionClamp, ProtectionReference, UsbConnector};
use crate::reports::Finding;

use super::super::common::validation_input_missing;
use super::usb_connector_findings::*;
use super::usb_connector_geometry::{
    angular_error_deg, nearest_board_edge, nearest_body_overhang_edge, placement_distance_mm,
    placement_is_finite,
};
use super::{
    required_scenario_numeric_parameter, scenario_bool_parameter, scenario_numeric_parameter,
};

pub(super) fn validate_usb_connector_orientation(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(expected_rotation_deg) =
        required_scenario_numeric_parameter(scenario, "expected_connector_rotation_deg", findings)
    else {
        return;
    };
    let rule = &bound.project.board.layout.constraints.usb_connector;
    let Some(max_error_deg) = required_usb_connector_nonnegative_parameter(
        scenario,
        rule,
        "max_connector_rotation_error_deg",
        rule.max_connector_rotation_error_deg,
        findings,
    ) else {
        return;
    };
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_CONNECTOR_ORIENTATION_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_orientation_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector orientation target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_orientation_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector orientation target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    if model.usb_connector.is_none() {
        findings.push(usb_orientation_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no usb_connector metadata.",
                target.component, component.model
            ),
            "usb_connector",
            "missing",
        ));
        return;
    }
    let Some(placement) = valid_component_placement(bound, scenario, &target.component, findings)
    else {
        return;
    };
    let Some(actual_rotation_deg) = placement.rotation_deg else {
        findings.push(usb_orientation_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector {} placement has no rotation_deg evidence.",
                target.component
            ),
            "rotation_deg",
            "missing",
        ));
        return;
    };
    if !actual_rotation_deg.is_finite() {
        findings.push(usb_orientation_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector {} placement rotation_deg must be finite.",
                target.component
            ),
            "rotation_deg",
            "non_finite",
        ));
        return;
    }
    let rotation_error_deg = angular_error_deg(actual_rotation_deg, expected_rotation_deg);
    if rotation_error_deg > max_error_deg {
        findings.push(usb_orientation_finding(
            scenario,
            &target.component,
            placement,
            actual_rotation_deg,
            expected_rotation_deg,
            rotation_error_deg,
            max_error_deg,
        ));
    }
}

pub(super) fn validate_usb_connector_edge_proximity(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let rule = &bound.project.board.layout.constraints.usb_connector;
    let Some(max_distance_mm) = required_usb_connector_nonnegative_parameter(
        scenario,
        rule,
        "max_connector_to_board_edge_distance_mm",
        rule.max_connector_to_board_edge_distance_mm,
        findings,
    ) else {
        return;
    };
    if max_distance_mm <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.max_connector_to_board_edge_distance_mm must be greater than zero.",
        );
        return;
    }
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_CONNECTOR_EDGE_PROXIMITY_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_edge_proximity_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector edge-proximity target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_edge_proximity_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector edge-proximity target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    if model.usb_connector.is_none() {
        findings.push(usb_edge_proximity_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no usb_connector metadata.",
                target.component, component.model
            ),
            "usb_connector",
            "missing",
        ));
        return;
    }
    let Some(placement) = valid_component_placement(bound, scenario, &target.component, findings)
    else {
        return;
    };
    let Some(edge) = nearest_board_edge(bound, &target.component, placement) else {
        findings.push(usb_edge_proximity_metadata_finding(
            scenario,
            &target.component,
            "USB connector edge proximity requires at least one usable board.layout.outline.segments entry.".to_string(),
            "outline",
            "missing",
        ));
        return;
    };
    if edge.distance_mm > max_distance_mm {
        findings.push(usb_edge_proximity_finding(
            scenario,
            &target.component,
            placement,
            &edge,
            max_distance_mm,
        ));
    }
}

pub(super) fn validate_usb_connector_body_overhang(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let rule = &bound.project.board.layout.constraints.usb_connector;
    let Some(max_overhang_mm) = required_usb_connector_nonnegative_parameter(
        scenario,
        rule,
        "max_connector_body_overhang_mm",
        rule.max_connector_body_overhang_mm,
        findings,
    ) else {
        return;
    };
    if max_overhang_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.max_connector_body_overhang_mm must be zero or greater.",
        );
        return;
    }
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_CONNECTOR_BODY_OVERHANG_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_body_overhang_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector body-overhang target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_body_overhang_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector body-overhang target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    if model.usb_connector.is_none() {
        findings.push(usb_body_overhang_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no usb_connector metadata.",
                target.component, component.model
            ),
            "usb_connector",
            "missing",
        ));
        return;
    }
    if valid_component_placement(bound, scenario, &target.component, findings).is_none() {
        return;
    }
    let Some(edge) = nearest_body_overhang_edge(bound, &target.component) else {
        findings.push(usb_body_overhang_metadata_finding(
            scenario,
            &target.component,
            "USB connector body overhang requires usable board.layout.outline.segments and imported fabrication/courtyard footprint graphics.".to_string(),
            "body_overhang_evidence",
            "missing",
        ));
        return;
    };
    if edge.body_overhang_mm > max_overhang_mm {
        findings.push(usb_body_overhang_finding(
            scenario,
            &target.component,
            &edge,
            max_overhang_mm,
        ));
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
    if scenario_bool_parameter(scenario, "require_shield_ground").unwrap_or(false) {
        validate_usb_connector_shield_ground(
            bound,
            scenario,
            &target.component,
            component,
            connector,
            findings,
        );
    }
}

pub(super) fn validate_usb_protection_placement(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let rule = &bound.project.board.layout.constraints.usb_connector;
    let Some(max_distance_mm) = required_usb_connector_nonnegative_parameter(
        scenario,
        rule,
        "max_connector_to_protection_distance_mm",
        rule.max_connector_to_protection_distance_mm,
        findings,
    ) else {
        return;
    };
    if max_distance_mm <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.max_connector_to_protection_distance_mm must be greater than zero.",
        );
        return;
    }
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_PROTECTION_PLACEMENT_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_placement_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB placement target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_placement_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB placement target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(connector) = &model.usb_connector else {
        findings.push(usb_placement_metadata_finding(
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
    let Some(connector_placement) =
        valid_component_placement(bound, scenario, &target.component, findings)
    else {
        return;
    };
    validate_usb_protection_placement_for_pin(
        bound,
        scenario,
        UsbPlacementPinCheck {
            connector_id: &target.component,
            component,
            connector,
            connector_placement,
            signal: UsbConnectorSignal::Dp,
            max_distance_mm,
        },
        findings,
    );
    validate_usb_protection_placement_for_pin(
        bound,
        scenario,
        UsbPlacementPinCheck {
            connector_id: &target.component,
            component,
            connector,
            connector_placement,
            signal: UsbConnectorSignal::Dm,
            max_distance_mm,
        },
        findings,
    );
    if scenario_bool_parameter(scenario, "require_vbus_protection").unwrap_or(false) {
        validate_usb_protection_placement_for_pin(
            bound,
            scenario,
            UsbPlacementPinCheck {
                connector_id: &target.component,
                component,
                connector,
                connector_placement,
                signal: UsbConnectorSignal::Vbus,
                max_distance_mm,
            },
            findings,
        );
    }
}

pub(super) fn required_usb_connector_nonnegative_parameter(
    scenario: &Scenario,
    rule: &UsbConnectorLayoutRule,
    name: &str,
    rule_value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if let Some(raw) = scenario.parameters.get(name)
        && !raw.is_null()
    {
        let Some(value) = raw.as_f64() else {
            validation_input_missing(
                findings,
                scenario,
                format!("interface_protection parameters.{name} must be numeric when declared."),
            );
            return None;
        };
        if !value.is_finite() || value < 0.0 {
            validation_input_missing(
                findings,
                scenario,
                format!("interface_protection parameters.{name} must be finite and non-negative."),
            );
            return None;
        }
        return Some(value);
    }
    let Some(value) = rule_value else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "interface_protection parameters.{name} or board.layout.constraints.usb_connector.{name} is required."
            ),
        );
        return None;
    };
    if !value.is_finite() || value < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "board.layout.constraints.usb_connector.{name} must be finite and non-negative."
            ),
        );
        return None;
    }
    if rule.source.as_deref().is_some_and(str::is_empty) {
        validation_input_missing(
            findings,
            scenario,
            "board.layout.constraints.usb_connector.source must not be empty when declared.",
        );
        return None;
    }
    Some(value)
}

#[derive(Debug, Clone, Copy)]
pub(super) enum UsbConnectorSignal {
    Dp,
    Dm,
    Vbus,
}

impl UsbConnectorSignal {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Dp => "D+",
            Self::Dm => "D-",
            Self::Vbus => "VBUS",
        }
    }

    pub(super) fn pin(self, connector: &UsbConnector) -> &str {
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
    component: &ComponentSpec,
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

fn validate_usb_connector_shield_ground(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    connector_id: &str,
    component: &ComponentSpec,
    connector: &UsbConnector,
    findings: &mut Vec<Finding>,
) {
    let Some(shield_pin) = connector.shield_pin.as_deref() else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} has no shield_pin metadata, but require_shield_ground is true."
            ),
            "shield_pin",
            connector_id,
        ));
        return;
    };
    let Some(shield_net_name) = component.pins.get(shield_pin) else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} shield pin {shield_pin} is not connected, but require_shield_ground is true."
            ),
            "missing_shield_pin",
            shield_pin,
        ));
        return;
    };
    let Some(shield_net) = bound.project.board.nets.get(shield_net_name) else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            connector_id,
            format!("USB connector {connector_id} shield net {shield_net_name} is not declared."),
            "missing_shield_net",
            shield_net_name,
        ));
        return;
    };
    if shield_net.kind != NetKind::Ground {
        findings.push(usb_connector_shield_ground_finding(
            scenario,
            connector_id,
            shield_pin,
            shield_net_name,
            &shield_net.kind,
        ));
    }
}

pub(super) struct ResolvedUsbProtection<'a> {
    pub(super) component_id: &'a str,
    pub(super) clamp: &'a ProtectionClamp,
    pub(super) reference_net_name: &'a str,
    pub(super) reference_net_kind: &'a NetKind,
}

struct UsbPlacementPinCheck<'a> {
    connector_id: &'a str,
    component: &'a ComponentSpec,
    connector: &'a UsbConnector,
    connector_placement: &'a ComponentPlacement,
    signal: UsbConnectorSignal,
    max_distance_mm: f64,
}

pub(super) struct UsbPlacementDistanceEvidence<'a> {
    pub(super) scenario: &'a Scenario,
    pub(super) connector_id: &'a str,
    pub(super) signal: UsbConnectorSignal,
    pub(super) net: &'a str,
    pub(super) protection: &'a ResolvedUsbProtection<'a>,
    pub(super) connector_placement: &'a ComponentPlacement,
    pub(super) protection_placement: &'a ComponentPlacement,
    pub(super) distance_mm: f64,
    pub(super) max_distance_mm: f64,
}

fn find_valid_clamp_for_net<'a>(
    bound: &'a BoundBoard<'_>,
    connector_id: &str,
    net_name: &str,
) -> Option<ResolvedUsbProtection<'a>> {
    valid_protection_clamps_for_net(bound, connector_id, net_name)
        .into_iter()
        .next()
}

pub(super) fn valid_protection_clamps_for_net<'a>(
    bound: &'a BoundBoard<'_>,
    connector_id: &str,
    net_name: &str,
) -> Vec<ResolvedUsbProtection<'a>> {
    let mut protections = Vec::new();
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
                protections.push(ResolvedUsbProtection {
                    component_id,
                    clamp,
                    reference_net_name,
                    reference_net_kind: &reference_net.kind,
                });
            }
        }
    }
    protections
}

fn validate_usb_protection_placement_for_pin(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: UsbPlacementPinCheck<'_>,
    findings: &mut Vec<Finding>,
) {
    let connector_id = check.connector_id;
    let connector_placement = check.connector_placement;
    let signal = check.signal;
    let max_distance_mm = check.max_distance_mm;
    let component = check.component;
    let connector = check.connector;
    let pin = signal.pin(connector);
    let Some(net_name) = component.pins.get(pin) else {
        findings.push(usb_placement_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} {} pin {pin} is not connected, so protection placement cannot be checked.",
                signal.label()
            ),
            "missing_pin",
            pin,
        ));
        return;
    };
    if !bound.project.board.nets.contains_key(net_name) {
        findings.push(usb_placement_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} {} net {net_name} is not declared, so protection placement cannot be checked.",
                signal.label()
            ),
            "missing_net",
            net_name,
        ));
        return;
    }
    let protections = valid_protection_clamps_for_net(bound, connector_id, net_name);
    if protections.is_empty() {
        findings.push(usb_placement_missing_protection_finding(
            scenario,
            connector_id,
            signal,
            pin,
            net_name,
        ));
        return;
    }
    let mut nearest: Option<(&ResolvedUsbProtection<'_>, &ComponentPlacement, f64)> = None;
    let mut missing_placements = Vec::new();
    for protection in &protections {
        let Some(protection_placement) = bound
            .project
            .board
            .layout
            .placements
            .get(protection.component_id)
        else {
            missing_placements.push(protection.component_id.to_string());
            continue;
        };
        if !placement_is_finite(protection_placement) {
            findings.push(usb_placement_metadata_finding(
                scenario,
                protection.component_id,
                format!(
                    "USB protection component {} placement must have finite x_mm and y_mm.",
                    protection.component_id
                ),
                "placement",
                protection.component_id,
            ));
            continue;
        }
        let distance_mm = placement_distance_mm(connector_placement, protection_placement);
        if nearest
            .as_ref()
            .is_none_or(|(_, _, nearest_distance)| distance_mm < *nearest_distance)
        {
            nearest = Some((protection, protection_placement, distance_mm));
        }
    }
    let Some((protection, protection_placement, distance_mm)) = nearest else {
        findings.push(usb_placement_missing_protection_placement_finding(
            scenario,
            connector_id,
            signal,
            net_name,
            &missing_placements,
        ));
        return;
    };
    if distance_mm > max_distance_mm {
        findings.push(usb_placement_distance_finding(
            UsbPlacementDistanceEvidence {
                scenario,
                connector_id,
                signal,
                net: net_name,
                protection,
                connector_placement,
                protection_placement,
                distance_mm,
                max_distance_mm,
            },
        ));
    }
}

pub(super) fn valid_component_placement<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    component_id: &str,
    findings: &mut Vec<Finding>,
) -> Option<&'a ComponentPlacement> {
    let Some(placement) = bound.project.board.layout.placements.get(component_id) else {
        findings.push(usb_placement_metadata_finding(
            scenario,
            component_id,
            format!("Component {component_id} has no board.layout.placements entry."),
            "placement",
            component_id,
        ));
        return None;
    };
    if !placement_is_finite(placement) {
        findings.push(usb_placement_metadata_finding(
            scenario,
            component_id,
            format!("Component {component_id} placement must have finite x_mm and y_mm."),
            "placement",
            component_id,
        ));
        return None;
    }
    Some(placement)
}
