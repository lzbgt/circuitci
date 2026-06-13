use crate::board_ir::{ComponentPlacement, LayoutPad, NetLayoutRule, NetRoute, Scenario};
use crate::library::{BoundBoard, UsbConnector};
use crate::reports::Finding;

use super::required_scenario_numeric_parameter;
use super::usb_connector::{
    UsbConnectorSignal, placement_is_finite, valid_component_placement,
    valid_protection_clamps_for_net,
};
use crate::validation::common::validation_input_missing;

mod findings;
mod geometry;
mod return_path;
mod vbus_findings;

use findings::*;
use geometry::{
    PlacementPoint, pad_to_route_distance_mm, route_distance_between_pads,
    route_distance_between_placements, route_length_mm, validate_route_shape, worst_pair_gap_delta,
    worst_route_width_delta,
};
use vbus_findings::*;

pub(super) fn validate_usb_route_geometry(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(max_route_length_mm) =
        required_positive_parameter(scenario, "max_data_line_route_length_mm", findings)
    else {
        return;
    };
    let Some(max_via_count) =
        optional_integer_parameter(scenario, "max_data_line_via_count", findings)
    else {
        return;
    };
    let Some(max_protection_route_distance_mm) = optional_positive_parameter(
        scenario,
        "max_connector_to_protection_route_distance_mm",
        findings,
    ) else {
        return;
    };
    let Some(max_component_to_route_distance_mm) =
        optional_positive_parameter(scenario, "max_component_to_route_distance_mm", findings)
    else {
        return;
    };
    let Some(max_pair_length_mismatch_mm) =
        required_nonnegative_parameter(scenario, "max_data_pair_length_mismatch_mm", findings)
    else {
        return;
    };
    let Some(max_pair_via_count_delta) =
        optional_integer_parameter(scenario, "max_data_pair_via_count_delta", findings)
    else {
        return;
    };
    let Some(max_data_line_width_delta_mm) =
        optional_nonnegative_parameter(scenario, "max_data_line_width_delta_mm", findings)
    else {
        return;
    };
    let Some(max_data_pair_gap_delta_mm) =
        optional_nonnegative_parameter(scenario, "max_data_pair_gap_delta_mm", findings)
    else {
        return;
    };
    let Some(require_route_pad_contact_evidence) =
        optional_bool_parameter(scenario, "require_route_pad_contact_evidence", findings)
    else {
        return;
    };

    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_ROUTE_GEOMETRY_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_route_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB route target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_route_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB route target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(connector) = &model.usb_connector else {
        findings.push(usb_route_metadata_finding(
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

    for signal in [UsbConnectorSignal::Dp, UsbConnectorSignal::Dm] {
        validate_usb_route_for_signal(
            bound,
            scenario,
            UsbRouteSignalCheck {
                connector_id: &target.component,
                component,
                connector,
                connector_placement,
                signal,
                max_route_length_mm,
                max_via_count,
                max_data_line_width_delta_mm,
                max_protection_route_distance_mm,
                max_component_to_route_distance_mm,
                require_route_pad_contact_evidence,
            },
            findings,
        );
    }
    validate_usb_pair_consistency(
        bound,
        scenario,
        &target.component,
        UsbPairRouteTarget {
            component,
            connector,
        },
        UsbPairLimits {
            max_length_mismatch_mm: max_pair_length_mismatch_mm,
            max_via_count_delta: max_pair_via_count_delta,
            max_gap_delta_mm: max_data_pair_gap_delta_mm,
        },
        findings,
    );
}

pub(super) fn validate_usb_return_path(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    return_path::validate_usb_return_path(bound, scenario, findings);
}

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
    let Some(max_via_count) = required_integer_parameter(scenario, "max_vbus_via_count", findings)
    else {
        return;
    };
    let Some(max_protection_route_distance_mm) = required_positive_parameter(
        scenario,
        "max_connector_to_vbus_protection_route_distance_mm",
        findings,
    ) else {
        return;
    };
    let Some(max_component_to_route_distance_mm) =
        required_positive_parameter(scenario, "max_component_to_route_distance_mm", findings)
    else {
        return;
    };
    let Some(min_vbus_route_width_mm) =
        optional_nonnegative_parameter(scenario, "min_vbus_route_width_mm", findings)
    else {
        return;
    };
    let Some(require_vbus_route_pad_contact_evidence) = optional_bool_parameter(
        scenario,
        "require_vbus_route_pad_contact_evidence",
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
    if via_count > max_via_count {
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

struct UsbRouteSignalCheck<'a> {
    connector_id: &'a str,
    component: &'a crate::board_ir::ComponentSpec,
    connector: &'a UsbConnector,
    connector_placement: &'a ComponentPlacement,
    signal: UsbConnectorSignal,
    max_route_length_mm: f64,
    max_via_count: Option<usize>,
    max_data_line_width_delta_mm: Option<f64>,
    max_protection_route_distance_mm: Option<f64>,
    max_component_to_route_distance_mm: Option<f64>,
    require_route_pad_contact_evidence: bool,
}

struct VbusRouteProtectionCheck<'a> {
    connector_id: &'a str,
    component: &'a crate::board_ir::ComponentSpec,
    connector: &'a UsbConnector,
    connector_placement: &'a ComponentPlacement,
    net_name: &'a str,
    route: &'a NetRoute,
    max_protection_route_distance_mm: f64,
    max_component_to_route_distance_mm: f64,
    require_route_pad_contact_evidence: bool,
}

#[derive(Debug, Clone, Copy)]
struct RouteDistanceLimits {
    max_protection_route_distance_mm: f64,
    max_component_to_route_distance_mm: f64,
}

fn validate_usb_route_for_signal(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: UsbRouteSignalCheck<'_>,
    findings: &mut Vec<Finding>,
) {
    let pin = check.signal.pin(check.connector);
    let Some(net_name) = check.component.pins.get(pin) else {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} pin {pin} is not connected, so route geometry cannot be checked.",
                check.connector_id,
                check.signal.label()
            ),
            "missing_pin",
            pin,
        ));
        return;
    };
    if !bound.project.board.nets.contains_key(net_name) {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} net {net_name} is not declared, so route geometry cannot be checked.",
                check.connector_id,
                check.signal.label()
            ),
            "missing_net",
            net_name,
        ));
        return;
    }
    let Some(route) = bound.project.board.layout.routes.get(net_name) else {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} net {net_name} has no board.layout.routes entry.",
                check.connector_id,
                check.signal.label()
            ),
            "missing_route",
            net_name,
        ));
        return;
    };
    if let Err(message) = validate_route_shape(route) {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            message,
            "route_geometry",
            net_name,
        ));
        return;
    }

    let route_length_mm = route_length_mm(route);
    if route_length_mm > check.max_route_length_mm {
        findings.push(usb_route_length_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            route_length_mm,
            check.max_route_length_mm,
        ));
    }
    let via_count = route.vias.len();
    if let Some(max_via_count) = check.max_via_count
        && via_count > max_via_count
    {
        findings.push(usb_route_via_count_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            via_count,
            max_via_count,
        ));
    }
    if let Some(max_width_delta_mm) = check.max_data_line_width_delta_mm {
        validate_route_width_against_rule(
            bound,
            scenario,
            &check,
            net_name,
            route,
            max_width_delta_mm,
            findings,
        );
    }
    validate_protection_route_distance(bound, scenario, &check, net_name, route, findings);
}

fn validate_protection_route_distance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: &UsbRouteSignalCheck<'_>,
    net_name: &str,
    route: &NetRoute,
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
        validate_protection_route_distance_from_pads(
            bound,
            scenario,
            check,
            net_name,
            route,
            distance_limits,
            findings,
        );
        return;
    }

    let protections = valid_protection_clamps_for_net(bound, check.connector_id, net_name);
    if protections.is_empty() {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} net {net_name} has no valid protection clamp for route-order validation.",
                check.connector_id,
                check.signal.label()
            ),
            "required_protection_clamp",
            net_name,
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
            route,
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
        findings.push(usb_route_no_protection_path_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            &missing_placements,
            &off_route,
            distance_limits.max_component_to_route_distance_mm,
        ));
        return;
    };
    if route_distance_mm > distance_limits.max_protection_route_distance_mm {
        findings.push(usb_route_protection_distance_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            protection_component,
            route_distance_mm,
            distance_limits.max_protection_route_distance_mm,
        ));
    }
}

fn validate_protection_route_distance_from_pads(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: &UsbRouteSignalCheck<'_>,
    net_name: &str,
    route: &NetRoute,
    distance_limits: RouteDistanceLimits,
    findings: &mut Vec<Finding>,
) {
    let protections = valid_protection_clamps_for_net(bound, check.connector_id, net_name);
    if protections.is_empty() {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} net {net_name} has no valid protection clamp for route-order validation.",
                check.connector_id,
                check.signal.label()
            ),
            "required_protection_clamp",
            net_name,
        ));
        return;
    }

    let connector_pin = check.signal.pin(check.connector);
    let Some(connector_pad) = route_pad_for_pin(bound, check.connector_id, connector_pin, net_name)
    else {
        findings.push(usb_route_pad_metadata_finding(
            scenario,
            UsbRoutePadMetadataEvidence {
                connector_id: check.connector_id,
                signal: check.signal,
                net: net_name,
                pad_component: check.connector_id,
                pad_pin: connector_pin,
                field: "missing_connector_route_pad",
            },
            format!(
                "USB connector {} {} pin {connector_pin} has no matching board.layout.pads evidence on net {net_name}.",
                check.connector_id,
                check.signal.label()
            ),
        ));
        return;
    };
    if pad_to_route_distance_mm(
        route,
        connector_pad,
        distance_limits.max_component_to_route_distance_mm,
    )
    .is_none_or(|distance_mm| distance_mm > distance_limits.max_component_to_route_distance_mm)
    {
        findings.push(usb_route_pad_metadata_finding(
            scenario,
            UsbRoutePadMetadataEvidence {
                connector_id: check.connector_id,
                signal: check.signal,
                net: net_name,
                pad_component: check.connector_id,
                pad_pin: connector_pin,
                field: "connector_pad_off_route",
            },
            format!(
                "USB connector {} {} pad {connector_pin} is not on the imported route for net {net_name} within {:.3} mm.",
                check.connector_id,
                check.signal.label(),
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
        let Some(protection_pad) =
            route_pad_for_pin(bound, protection.component_id, protection_pin, net_name)
        else {
            missing_pads.push(format!("{}.{}", protection.component_id, protection_pin));
            continue;
        };
        let Some(route_distance) = route_distance_between_pads(
            route,
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
        findings.push(usb_route_no_protection_pad_path_finding(
            scenario,
            UsbRoutePadPathEvidence {
                connector_id: check.connector_id,
                signal: check.signal,
                net: net_name,
                connector_pin,
                missing_pads: &missing_pads,
                off_route_pads: &off_route_pads,
                max_pad_to_route_distance_mm: distance_limits.max_component_to_route_distance_mm,
            },
        ));
        return;
    };
    if route_distance_mm > distance_limits.max_protection_route_distance_mm {
        findings.push(usb_route_protection_pad_distance_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            UsbRoutePadDistanceEvidence {
                connector_pin,
                protection_component,
                protection_pin,
                route_distance_mm,
                max_route_distance_mm: distance_limits.max_protection_route_distance_mm,
            },
        ));
    }
}

fn route_pad_for_pin<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &str,
    pin: &str,
    net_name: &str,
) -> Option<&'a LayoutPad> {
    let pads = bound.project.board.layout.pads.get(component_id)?;
    if let Some(pad) = pads.get(pin)
        && pad.net == net_name
        && pad.at.x_mm.is_finite()
        && pad.at.y_mm.is_finite()
    {
        return Some(pad);
    }
    let mut matching_pads = pads
        .values()
        .filter(|pad| pad.net == net_name && pad.at.x_mm.is_finite() && pad.at.y_mm.is_finite());
    let pad = matching_pads.next()?;
    matching_pads.next().is_none().then_some(pad)
}

fn validate_vbus_protection_route_distance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: VbusRouteProtectionCheck<'_>,
    findings: &mut Vec<Finding>,
) {
    if check.require_route_pad_contact_evidence {
        validate_vbus_protection_route_distance_from_pads(bound, scenario, &check, findings);
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
            check.max_component_to_route_distance_mm,
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
            check.max_component_to_route_distance_mm,
        ));
        return;
    };
    if route_distance_mm > check.max_protection_route_distance_mm {
        findings.push(usb_vbus_route_protection_distance_finding(
            scenario,
            check.connector_id,
            check.net_name,
            protection_component,
            route_distance_mm,
            check.max_protection_route_distance_mm,
        ));
    }
}

fn validate_vbus_protection_route_distance_from_pads(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: &VbusRouteProtectionCheck<'_>,
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
        check.max_component_to_route_distance_mm,
    )
    .is_none_or(|distance_mm| distance_mm > check.max_component_to_route_distance_mm)
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
                check.max_component_to_route_distance_mm
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
            check.max_component_to_route_distance_mm,
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
                max_pad_to_route_distance_mm: check.max_component_to_route_distance_mm,
            },
        ));
        return;
    };
    if route_distance_mm > check.max_protection_route_distance_mm {
        findings.push(usb_vbus_route_protection_pad_distance_finding(
            scenario,
            check.connector_id,
            check.net_name,
            UsbVbusRoutePadDistanceEvidence {
                connector_pin,
                protection_component,
                protection_pin,
                route_distance_mm,
                max_route_distance_mm: check.max_protection_route_distance_mm,
            },
        ));
    }
}

fn validate_route_width_against_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: &UsbRouteSignalCheck<'_>,
    net_name: &str,
    route: &NetRoute,
    max_width_delta_mm: f64,
    findings: &mut Vec<Finding>,
) {
    let Some(rule) = bound
        .project
        .board
        .layout
        .constraints
        .net_rules
        .get(net_name)
    else {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {connector_id} {} net {net_name} has no board.layout.constraints.net_rules entry for width validation.",
                check.signal.label(),
                connector_id = check.connector_id
            ),
            "missing_route_constraint",
            net_name,
        ));
        return;
    };
    let Some(expected_width_mm) = expected_usb_data_width_mm(rule) else {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {connector_id} {} net {net_name} route rule has no diff_pair_width_mm or track_width_mm.",
                check.signal.label(),
                connector_id = check.connector_id
            ),
            "missing_route_width_constraint",
            net_name,
        ));
        return;
    };
    let Some((segment_index, measured_width_mm, width_delta_mm)) =
        worst_route_width_delta(route, expected_width_mm)
    else {
        return;
    };
    if width_delta_mm > max_width_delta_mm {
        findings.push(usb_route_width_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            UsbRouteWidthEvidence {
                segment_index,
                measured_width_mm,
                expected_width_mm,
                width_delta_mm,
                max_width_delta_mm,
            },
        ));
    }
}

fn validate_usb_pair_consistency(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    connector_id: &str,
    target: UsbPairRouteTarget<'_>,
    limits: UsbPairLimits,
    findings: &mut Vec<Finding>,
) {
    let Some((dp_net, dp_route)) = route_for_signal(
        bound,
        target.component,
        target.connector,
        UsbConnectorSignal::Dp,
    ) else {
        return;
    };
    let Some((dm_net, dm_route)) = route_for_signal(
        bound,
        target.component,
        target.connector,
        UsbConnectorSignal::Dm,
    ) else {
        return;
    };
    if validate_route_shape(dp_route).is_err() || validate_route_shape(dm_route).is_err() {
        return;
    }
    let dp_length_mm = route_length_mm(dp_route);
    let dm_length_mm = route_length_mm(dm_route);
    let length_mismatch_mm = (dp_length_mm - dm_length_mm).abs();
    if length_mismatch_mm > limits.max_length_mismatch_mm {
        findings.push(usb_pair_length_mismatch_finding(
            scenario,
            connector_id,
            UsbPairLengthEvidence {
                dp_net,
                dm_net,
                dp_length_mm,
                dm_length_mm,
                length_mismatch_mm,
                max_length_mismatch_mm: limits.max_length_mismatch_mm,
            },
        ));
    }
    let dp_via_count = dp_route.vias.len();
    let dm_via_count = dm_route.vias.len();
    let via_count_delta = dp_via_count.abs_diff(dm_via_count);
    if let Some(max_via_count_delta) = limits.max_via_count_delta
        && via_count_delta > max_via_count_delta
    {
        findings.push(usb_pair_via_delta_finding(
            scenario,
            connector_id,
            UsbPairViaEvidence {
                dp_net,
                dm_net,
                dp_via_count,
                dm_via_count,
                via_count_delta,
                max_via_count_delta,
            },
        ));
    }
    if let Some(max_gap_delta_mm) = limits.max_gap_delta_mm {
        validate_usb_pair_gap(
            bound,
            scenario,
            connector_id,
            UsbPairRoutes {
                dp_net,
                dp_route,
                dm_net,
                dm_route,
            },
            max_gap_delta_mm,
            findings,
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct UsbPairRouteTarget<'a> {
    component: &'a crate::board_ir::ComponentSpec,
    connector: &'a UsbConnector,
}

#[derive(Debug, Clone, Copy)]
struct UsbPairLimits {
    max_length_mismatch_mm: f64,
    max_via_count_delta: Option<usize>,
    max_gap_delta_mm: Option<f64>,
}

fn route_for_signal<'a>(
    bound: &'a BoundBoard<'_>,
    component: &'a crate::board_ir::ComponentSpec,
    connector: &UsbConnector,
    signal: UsbConnectorSignal,
) -> Option<(&'a str, &'a NetRoute)> {
    let net_name = component.pins.get(signal.pin(connector))?;
    let route = bound.project.board.layout.routes.get(net_name)?;
    Some((net_name, route))
}

fn expected_usb_data_width_mm(rule: &NetLayoutRule) -> Option<f64> {
    rule.diff_pair_width_mm.or(rule.track_width_mm)
}

fn narrowest_route_segment(route: &NetRoute) -> Option<(usize, f64)> {
    route
        .segments
        .iter()
        .enumerate()
        .map(|(index, segment)| (index, segment.width_mm))
        .min_by(|left, right| left.1.total_cmp(&right.1))
}

fn validate_usb_pair_gap(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    connector_id: &str,
    routes: UsbPairRoutes<'_>,
    max_gap_delta_mm: f64,
    findings: &mut Vec<Finding>,
) {
    let expected_gap_mm = match (
        route_rule_gap(bound, routes.dp_net),
        route_rule_gap(bound, routes.dm_net),
    ) {
        (Some(dp_gap), Some(dm_gap)) => dp_gap.min(dm_gap),
        (Some(gap), None) | (None, Some(gap)) => gap,
        (None, None) => {
            findings.push(usb_route_metadata_finding(
                scenario,
                connector_id,
                format!(
                    "USB connector {connector_id} D+/D- nets {}/{} have no diff_pair_gap_mm route constraint.",
                    routes.dp_net, routes.dm_net
                ),
                "missing_diff_pair_gap_constraint",
                routes.dp_net,
            ));
            return;
        }
    };
    let Some(gap) = worst_pair_gap_delta(routes.dp_route, routes.dm_route, expected_gap_mm) else {
        findings.push(usb_pair_gap_unmeasured_finding(
            scenario,
            connector_id,
            routes.dp_net,
            routes.dm_net,
            expected_gap_mm,
        ));
        return;
    };
    if gap.gap_delta_mm > max_gap_delta_mm {
        findings.push(usb_pair_gap_finding(
            scenario,
            connector_id,
            routes.dp_net,
            routes.dm_net,
            gap,
            max_gap_delta_mm,
        ));
    }
}

#[derive(Debug, Clone, Copy)]
struct UsbPairRoutes<'a> {
    dp_net: &'a str,
    dp_route: &'a NetRoute,
    dm_net: &'a str,
    dm_route: &'a NetRoute,
}

fn route_rule_gap(bound: &BoundBoard<'_>, net_name: &str) -> Option<f64> {
    bound
        .project
        .board
        .layout
        .constraints
        .net_rules
        .get(net_name)
        .and_then(|rule| rule.diff_pair_gap_mm)
}

fn required_positive_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let value = required_scenario_numeric_parameter(scenario, name, findings)?;
    if value <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be greater than zero."),
        );
        return None;
    }
    Some(value)
}

fn required_nonnegative_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    required_scenario_numeric_parameter(scenario, name, findings)
}

fn optional_nonnegative_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<Option<f64>> {
    let Some(raw) = scenario.parameters.get(name) else {
        return Some(None);
    };
    if raw.is_null() {
        return Some(None);
    }
    let Some(value) = raw.as_f64() else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be a number."),
        );
        return None;
    };
    if !value.is_finite() || value < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be non-negative."),
        );
        return None;
    }
    Some(Some(value))
}

fn optional_positive_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<Option<f64>> {
    let Some(raw) = scenario.parameters.get(name) else {
        return Some(None);
    };
    if raw.is_null() {
        return Some(None);
    }
    let Some(value) = raw.as_f64() else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be a number."),
        );
        return None;
    };
    if !value.is_finite() || value <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be greater than zero."),
        );
        return None;
    }
    Some(Some(value))
}

fn optional_integer_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<Option<usize>> {
    let Some(raw) = scenario.parameters.get(name) else {
        return Some(None);
    };
    if raw.is_null() {
        return Some(None);
    }
    let Some(value) = raw.as_f64() else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be a number."),
        );
        return None;
    };
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > usize::MAX as f64 {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be a non-negative integer."),
        );
        return None;
    }
    Some(Some(value as usize))
}

fn optional_bool_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<bool> {
    let Some(raw) = scenario.parameters.get(name) else {
        return Some(false);
    };
    if raw.is_null() {
        return Some(false);
    }
    let Some(value) = raw.as_bool() else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be a boolean."),
        );
        return None;
    };
    Some(value)
}

fn required_integer_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<usize> {
    let value = required_scenario_numeric_parameter(scenario, name, findings)?;
    if value.fract() != 0.0 || value > usize::MAX as f64 {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be a non-negative integer."),
        );
        return None;
    }
    Some(value as usize)
}
