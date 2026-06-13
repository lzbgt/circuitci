use crate::board_ir::{
    ComponentPlacement, ComponentSpec, NetRoute, Scenario, UsbVbusRouteLayoutRule,
};
use crate::library::{BoundBoard, UsbConnector};
use crate::reports::Finding;
use crate::validation::common::validation_input_missing;

use super::super::usb_connector::{valid_component_placement, valid_protection_clamps_for_net};
use super::super::usb_connector_geometry::placement_is_finite;
use super::geometry::{
    PlacementPoint, pad_to_route_distance_mm, route_distance_between_pads,
    route_distance_between_placements, route_length_mm, validate_route_shape,
};
use super::vbus_findings::*;
use super::{
    RouteDistanceLimits, narrowest_route_segment, optional_bool_parameter,
    optional_integer_parameter, optional_nonnegative_parameter, optional_positive_parameter,
    required_positive_parameter, route_pad_for_pin, validate_optional_constraint_integer_rule,
    validate_optional_constraint_nonnegative_rule, validate_optional_constraint_positive_rule,
    validate_optional_constraint_source,
};

pub(super) fn validate_usb_vbus_route(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(max_route_length_mm) =
        required_positive_parameter(scenario, "max_vbus_route_length_mm", findings)
    else {
        return;
    };
    let vbus_rule = &bound.project.board.layout.constraints.usb_vbus_route;
    let Some(max_via_count) = optional_vbus_integer_parameter_or_rule(
        scenario,
        vbus_rule,
        "max_vbus_via_count",
        vbus_rule.max_vbus_via_count,
        findings,
    ) else {
        return;
    };
    let Some(max_protection_route_distance_mm) = optional_vbus_positive_parameter_or_rule(
        scenario,
        vbus_rule,
        "max_connector_to_vbus_protection_route_distance_mm",
        vbus_rule.max_connector_to_vbus_protection_route_distance_mm,
        findings,
    ) else {
        return;
    };
    let Some(max_component_to_route_distance_mm) = optional_vbus_positive_parameter_or_rule(
        scenario,
        vbus_rule,
        "max_component_to_route_distance_mm",
        vbus_rule.max_component_to_route_distance_mm,
        findings,
    ) else {
        return;
    };
    let Some(min_vbus_route_width_mm) = optional_vbus_nonnegative_parameter_or_rule(
        scenario,
        vbus_rule,
        "min_vbus_route_width_mm",
        vbus_rule.min_vbus_route_width_mm,
        findings,
    ) else {
        return;
    };
    let Some(require_vbus_route_pad_contact_evidence) = optional_vbus_bool_parameter_or_rule(
        scenario,
        "require_vbus_route_pad_contact_evidence",
        vbus_rule,
        vbus_rule.require_vbus_route_pad_contact_evidence,
        findings,
    ) else {
        return;
    };

    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_VBUS_ROUTE_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_vbus_route_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB VBUS route target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_vbus_route_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB VBUS route target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(connector) = &model.usb_connector else {
        findings.push(usb_vbus_route_metadata_finding(
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
    let pin = &connector.vbus_pin;
    let Some(net_name) = component.pins.get(pin) else {
        findings.push(usb_vbus_route_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector {} VBUS pin {pin} is not connected, so VBUS route geometry cannot be checked.",
                target.component
            ),
            "missing_pin",
            pin,
        ));
        return;
    };
    if !bound.project.board.nets.contains_key(net_name) {
        findings.push(usb_vbus_route_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector {} VBUS net {net_name} is not declared, so VBUS route geometry cannot be checked.",
                target.component
            ),
            "missing_net",
            net_name,
        ));
        return;
    }
    let Some(route) = bound.project.board.layout.routes.get(net_name) else {
        findings.push(usb_vbus_route_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector {} VBUS net {net_name} has no board.layout.routes entry.",
                target.component
            ),
            "missing_route",
            net_name,
        ));
        return;
    };
    if let Err(message) = validate_route_shape(route) {
        findings.push(usb_vbus_route_metadata_finding(
            scenario,
            &target.component,
            message,
            "route_geometry",
            net_name,
        ));
        return;
    }

    let route_length_mm = route_length_mm(route);
    if route_length_mm > max_route_length_mm {
        findings.push(usb_vbus_route_length_finding(
            scenario,
            &target.component,
            net_name,
            route_length_mm,
            max_route_length_mm,
        ));
    }
    let via_count = route.vias.len();
    if let Some(max_via_count) = max_via_count
        && via_count > max_via_count
    {
        findings.push(usb_vbus_route_via_count_finding(
            scenario,
            &target.component,
            net_name,
            via_count,
            max_via_count,
        ));
    }
    if let Some(min_width_mm) = min_vbus_route_width_mm
        && let Some((segment_index, measured_width_mm)) = narrowest_route_segment(route)
        && measured_width_mm < min_width_mm
    {
        findings.push(usb_vbus_route_width_finding(
            scenario,
            &target.component,
            net_name,
            segment_index,
            measured_width_mm,
            min_width_mm,
        ));
    }
    validate_vbus_protection_route_distance(
        bound,
        scenario,
        VbusRouteProtectionCheck {
            connector_id: &target.component,
            component,
            connector,
            connector_placement,
            net_name,
            route,
            max_protection_route_distance_mm,
            max_component_to_route_distance_mm,
            require_route_pad_contact_evidence: require_vbus_route_pad_contact_evidence,
        },
        findings,
    );
}

struct VbusRouteProtectionCheck<'a> {
    connector_id: &'a str,
    component: &'a ComponentSpec,
    connector: &'a UsbConnector,
    connector_placement: &'a ComponentPlacement,
    net_name: &'a str,
    route: &'a NetRoute,
    max_protection_route_distance_mm: Option<f64>,
    max_component_to_route_distance_mm: Option<f64>,
    require_route_pad_contact_evidence: bool,
}

fn validate_vbus_protection_route_distance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: VbusRouteProtectionCheck<'_>,
    findings: &mut Vec<Finding>,
) {
    let (Some(max_protection_route_distance_mm), Some(max_component_to_route_distance_mm)) = (
        check.max_protection_route_distance_mm,
        check.max_component_to_route_distance_mm,
    ) else {
        return;
    };
    let distance_limits = RouteDistanceLimits {
        max_protection_route_distance_mm,
        max_component_to_route_distance_mm,
    };
    if check.require_route_pad_contact_evidence {
        validate_vbus_protection_route_distance_from_pads(
            bound,
            scenario,
            &check,
            distance_limits,
            findings,
        );
        return;
    }

    let protections = valid_protection_clamps_for_net(bound, check.connector_id, check.net_name);
    if protections.is_empty() {
        findings.push(usb_vbus_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} VBUS net {} has no valid protection clamp for route-order validation.",
                check.connector_id, check.net_name
            ),
            "required_vbus_protection_clamp",
            check.net_name,
        ));
        return;
    }
    let connector_point = PlacementPoint::from(check.connector_placement);
    let mut nearest = None;
    let mut missing_placements = Vec::new();
    let mut off_route = Vec::new();
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
            missing_placements.push(protection.component_id.to_string());
            continue;
        }
        let protection_point = PlacementPoint::from(protection_placement);
        let Some(route_distance) = route_distance_between_placements(
            check.route,
            connector_point,
            protection_point,
            distance_limits.max_component_to_route_distance_mm,
        ) else {
            off_route.push(protection.component_id.to_string());
            continue;
        };
        if nearest
            .as_ref()
            .is_none_or(|(_, distance): &(&str, f64)| route_distance < *distance)
        {
            nearest = Some((protection.component_id, route_distance));
        }
    }
    let Some((protection_component, route_distance_mm)) = nearest else {
        findings.push(usb_vbus_route_no_protection_path_finding(
            scenario,
            check.connector_id,
            check.net_name,
            &missing_placements,
            &off_route,
            distance_limits.max_component_to_route_distance_mm,
        ));
        return;
    };
    if route_distance_mm > distance_limits.max_protection_route_distance_mm {
        findings.push(usb_vbus_route_protection_distance_finding(
            scenario,
            check.connector_id,
            check.net_name,
            protection_component,
            route_distance_mm,
            distance_limits.max_protection_route_distance_mm,
        ));
    }
}

fn validate_vbus_protection_route_distance_from_pads(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: &VbusRouteProtectionCheck<'_>,
    distance_limits: RouteDistanceLimits,
    findings: &mut Vec<Finding>,
) {
    let protections = valid_protection_clamps_for_net(bound, check.connector_id, check.net_name);
    if protections.is_empty() {
        findings.push(usb_vbus_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} VBUS net {} has no valid protection clamp for route-order validation.",
                check.connector_id, check.net_name
            ),
            "required_vbus_protection_clamp",
            check.net_name,
        ));
        return;
    }

    let connector_pin = &check.connector.vbus_pin;
    if check.component.pins.get(connector_pin).map(String::as_str) != Some(check.net_name) {
        findings.push(usb_vbus_route_pad_metadata_finding(
            scenario,
            UsbVbusRoutePadMetadataEvidence {
                connector_id: check.connector_id,
                net: check.net_name,
                pad_component: check.connector_id,
                pad_pin: connector_pin,
                field: "connector_pin_net",
            },
            format!(
                "USB connector {} VBUS pin {connector_pin} is not connected to net {}.",
                check.connector_id, check.net_name
            ),
        ));
        return;
    }

    let Some(connector_pad) =
        route_pad_for_pin(bound, check.connector_id, connector_pin, check.net_name)
    else {
        findings.push(usb_vbus_route_pad_metadata_finding(
            scenario,
            UsbVbusRoutePadMetadataEvidence {
                connector_id: check.connector_id,
                net: check.net_name,
                pad_component: check.connector_id,
                pad_pin: connector_pin,
                field: "missing_connector_vbus_route_pad",
            },
            format!(
                "USB connector {} VBUS pin {connector_pin} has no matching board.layout.pads evidence on net {}.",
                check.connector_id, check.net_name
            ),
        ));
        return;
    };
    if pad_to_route_distance_mm(
        check.route,
        connector_pad,
        distance_limits.max_component_to_route_distance_mm,
    )
    .is_none_or(|distance_mm| distance_mm > distance_limits.max_component_to_route_distance_mm)
    {
        findings.push(usb_vbus_route_pad_metadata_finding(
            scenario,
            UsbVbusRoutePadMetadataEvidence {
                connector_id: check.connector_id,
                net: check.net_name,
                pad_component: check.connector_id,
                pad_pin: connector_pin,
                field: "connector_vbus_pad_off_route",
            },
            format!(
                "USB connector {} VBUS pad {connector_pin} is not on the imported route for net {} within {:.3} mm.",
                check.connector_id,
                check.net_name,
                distance_limits.max_component_to_route_distance_mm
            ),
        ));
        return;
    }

    let mut nearest = None;
    let mut missing_pads = Vec::new();
    let mut off_route_pads = Vec::new();
    for protection in &protections {
        let protection_pin = &protection.clamp.protected_pin;
        let Some(protection_pad) = route_pad_for_pin(
            bound,
            protection.component_id,
            protection_pin,
            check.net_name,
        ) else {
            missing_pads.push(format!("{}.{}", protection.component_id, protection_pin));
            continue;
        };
        let Some(route_distance) = route_distance_between_pads(
            check.route,
            connector_pad,
            protection_pad,
            distance_limits.max_component_to_route_distance_mm,
        ) else {
            off_route_pads.push(format!("{}.{}", protection.component_id, protection_pin));
            continue;
        };
        if nearest
            .as_ref()
            .is_none_or(|(_, _, distance): &(&str, &str, f64)| route_distance < *distance)
        {
            nearest = Some((
                protection.component_id,
                protection_pin.as_str(),
                route_distance,
            ));
        }
    }
    let Some((protection_component, protection_pin, route_distance_mm)) = nearest else {
        findings.push(usb_vbus_route_no_protection_pad_path_finding(
            scenario,
            UsbVbusRoutePadPathEvidence {
                connector_id: check.connector_id,
                net: check.net_name,
                connector_pin,
                missing_pads: &missing_pads,
                off_route_pads: &off_route_pads,
                max_pad_to_route_distance_mm: distance_limits.max_component_to_route_distance_mm,
            },
        ));
        return;
    };
    if route_distance_mm > distance_limits.max_protection_route_distance_mm {
        findings.push(usb_vbus_route_protection_pad_distance_finding(
            scenario,
            check.connector_id,
            check.net_name,
            UsbVbusRoutePadDistanceEvidence {
                connector_pin,
                protection_component,
                protection_pin,
                route_distance_mm,
                max_route_distance_mm: distance_limits.max_protection_route_distance_mm,
            },
        ));
    }
}

fn optional_vbus_nonnegative_parameter_or_rule(
    scenario: &Scenario,
    rule: &UsbVbusRouteLayoutRule,
    name: &str,
    rule_value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<Option<f64>> {
    match optional_nonnegative_parameter(scenario, name, findings)? {
        Some(value) => Some(Some(value)),
        None => validate_optional_constraint_nonnegative_rule(
            scenario,
            "board.layout.constraints.usb_vbus_route",
            rule.source.as_deref(),
            name,
            rule_value,
            findings,
        ),
    }
}

fn optional_vbus_positive_parameter_or_rule(
    scenario: &Scenario,
    rule: &UsbVbusRouteLayoutRule,
    name: &str,
    rule_value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<Option<f64>> {
    match optional_positive_parameter(scenario, name, findings)? {
        Some(value) => Some(Some(value)),
        None => validate_optional_constraint_positive_rule(
            scenario,
            "board.layout.constraints.usb_vbus_route",
            rule.source.as_deref(),
            name,
            rule_value,
            findings,
        ),
    }
}

fn optional_vbus_integer_parameter_or_rule(
    scenario: &Scenario,
    rule: &UsbVbusRouteLayoutRule,
    name: &str,
    rule_value: Option<usize>,
    findings: &mut Vec<Finding>,
) -> Option<Option<usize>> {
    match optional_integer_parameter(scenario, name, findings)? {
        Some(value) => Some(Some(value)),
        None => validate_optional_constraint_integer_rule(
            scenario,
            "board.layout.constraints.usb_vbus_route",
            rule.source.as_deref(),
            rule_value,
            findings,
        ),
    }
}

fn optional_vbus_bool_parameter_or_rule(
    scenario: &Scenario,
    name: &str,
    rule: &UsbVbusRouteLayoutRule,
    rule_value: Option<bool>,
    findings: &mut Vec<Finding>,
) -> Option<bool> {
    let scenario_value = optional_bool_parameter(scenario, name, findings)?;
    if scenario
        .parameters
        .get(name)
        .is_some_and(|raw| !raw.is_null())
    {
        return Some(scenario_value);
    }
    validate_optional_constraint_source(
        scenario,
        "board.layout.constraints.usb_vbus_route",
        rule.source.as_deref(),
        rule_value.is_some(),
        findings,
    )?;
    Some(rule_value.unwrap_or(scenario_value))
}
