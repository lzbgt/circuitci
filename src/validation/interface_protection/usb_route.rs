use crate::board_ir::{ComponentPlacement, NetLayoutRule, NetRoute, Scenario};
use crate::library::{BoundBoard, UsbConnector};
use crate::reports::Finding;
use crate::validation::USB_VBUS_ROUTE_VALID;
use serde_json::json;

use super::{
    UsbConnectorSignal, placement_is_finite, required_scenario_numeric_parameter,
    valid_component_placement, valid_protection_clamps_for_net,
};
use crate::validation::common::validation_input_missing;

mod findings;
mod geometry;
mod return_path;

use findings::*;
use geometry::{
    PlacementPoint, route_distance_between_placements, route_length_mm, validate_route_shape,
    worst_pair_gap_delta, worst_route_width_delta,
};

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
        required_integer_parameter(scenario, "max_data_line_via_count", findings)
    else {
        return;
    };
    let Some(max_protection_route_distance_mm) = required_positive_parameter(
        scenario,
        "max_connector_to_protection_route_distance_mm",
        findings,
    ) else {
        return;
    };
    let Some(max_component_to_route_distance_mm) =
        required_positive_parameter(scenario, "max_component_to_route_distance_mm", findings)
    else {
        return;
    };
    let Some(max_pair_length_mismatch_mm) =
        required_nonnegative_parameter(scenario, "max_data_pair_length_mismatch_mm", findings)
    else {
        return;
    };
    let Some(max_pair_via_count_delta) =
        required_integer_parameter(scenario, "max_data_pair_via_count_delta", findings)
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
            connector_placement,
            net_name,
            route,
            max_protection_route_distance_mm,
            max_component_to_route_distance_mm,
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
    max_via_count: usize,
    max_data_line_width_delta_mm: Option<f64>,
    max_protection_route_distance_mm: f64,
    max_component_to_route_distance_mm: f64,
}

struct VbusRouteProtectionCheck<'a> {
    connector_id: &'a str,
    connector_placement: &'a ComponentPlacement,
    net_name: &'a str,
    route: &'a NetRoute,
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
    if via_count > check.max_via_count {
        findings.push(usb_route_via_count_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            via_count,
            check.max_via_count,
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
        findings.push(usb_route_no_protection_path_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            &missing_placements,
            &off_route,
            check.max_component_to_route_distance_mm,
        ));
        return;
    };
    if route_distance_mm > check.max_protection_route_distance_mm {
        findings.push(usb_route_protection_distance_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            protection_component,
            route_distance_mm,
            check.max_protection_route_distance_mm,
        ));
    }
}

fn validate_vbus_protection_route_distance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: VbusRouteProtectionCheck<'_>,
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
    if via_count_delta > limits.max_via_count_delta {
        findings.push(usb_pair_via_delta_finding(
            scenario,
            connector_id,
            UsbPairViaEvidence {
                dp_net,
                dm_net,
                dp_via_count,
                dm_via_count,
                via_count_delta,
                max_via_count_delta: limits.max_via_count_delta,
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
    max_via_count_delta: usize,
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

fn usb_vbus_route_metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(USB_VBUS_ROUTE_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Import PCB route geometry with import-kicad-pcb before declaring USB_VBUS_ROUTE_VALID."
            .to_string(),
        "Declare VBUS route limits from the board USB power/layout rule instead of inferring them from coordinates.".to_string(),
    ];
    finding
}

fn usb_vbus_route_length_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    route_length_mm: f64,
    max_route_length_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} route length {:.3} mm exceeds limit {:.3} mm.",
            route_length_mm, max_route_length_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding
        .measured
        .insert("route_length_mm".to_string(), json!(route_length_mm));
    finding.limit.insert(
        "max_vbus_route_length_mm".to_string(),
        json!(max_route_length_mm),
    );
    finding.suggested_fixes = vec![
        "Shorten the USB VBUS route or move the connector/protection/power-entry path closer together.".to_string(),
        "Use a board-specific USB power-layout rule for max_vbus_route_length_mm.".to_string(),
    ];
    finding
}

fn usb_vbus_route_via_count_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    via_count: usize,
    max_via_count: usize,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} has {via_count} vias, above limit {max_via_count}."
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding
        .measured
        .insert("via_count".to_string(), json!(via_count));
    finding
        .limit
        .insert("max_vbus_via_count".to_string(), json!(max_via_count));
    finding.suggested_fixes = vec![
        "Reduce USB VBUS layer changes before the protection/power-entry stage or relax max_vbus_via_count only with layout justification.".to_string(),
        "Use a separate power-path/current-capacity review for VBUS copper sizing and fuse behavior.".to_string(),
    ];
    finding
}

fn usb_vbus_route_width_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    segment_index: usize,
    measured_width_mm: f64,
    min_width_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} segment {segment_index} width {:.3} mm is below minimum {:.3} mm.",
            measured_width_mm, min_width_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding
        .measured
        .insert("segment_index".to_string(), json!(segment_index));
    finding.measured.insert(
        "route_segment_width_mm".to_string(),
        json!(measured_width_mm),
    );
    finding
        .limit
        .insert("min_vbus_route_width_mm".to_string(), json!(min_width_mm));
    finding.suggested_fixes = vec![
        "Widen the USB VBUS route to satisfy the board's USB power-entry layout rule.".to_string(),
        "Keep current-capacity and temperature-rise sign-off in a separate power-layout review."
            .to_string(),
    ];
    finding
}

fn usb_vbus_route_no_protection_path_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    missing_placements: &[String],
    off_route_components: &[String],
    max_component_to_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} has no protection component with usable route-distance evidence."
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding.measured.insert(
        "protection_components_without_placement".to_string(),
        json!(missing_placements),
    );
    finding.measured.insert(
        "protection_components_off_route".to_string(),
        json!(off_route_components),
    );
    finding.limit.insert(
        "max_component_to_route_distance_mm".to_string(),
        json!(max_component_to_route_distance_mm),
    );
    finding.suggested_fixes = vec![
        "Place the USB VBUS protection component on the routed VBUS net near the connector and import updated PCB route geometry.".to_string(),
        "Check that component placement coordinates and route coordinates share the same PCB coordinate system.".to_string(),
    ];
    finding
}

fn usb_vbus_route_protection_distance_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    protection_component: &str,
    route_distance_mm: f64,
    max_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} reaches protection component {protection_component} after {:.3} mm of route, exceeding limit {:.3} mm.",
            route_distance_mm, max_route_distance_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding.measured.insert(
        "protection_component".to_string(),
        json!(protection_component),
    );
    finding.measured.insert(
        "connector_to_vbus_protection_route_distance_mm".to_string(),
        json!(route_distance_mm),
    );
    finding.limit.insert(
        "max_connector_to_vbus_protection_route_distance_mm".to_string(),
        json!(max_route_distance_mm),
    );
    finding.suggested_fixes = vec![
        "Move the VBUS protection component closer to the USB connector along the routed VBUS path.".to_string(),
        "Route connector VBUS through the protection/power-entry component before continuing to downstream loads.".to_string(),
    ];
    finding
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

fn optional_bool_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<bool> {
    let Some(raw) = scenario.parameters.get(name) else {
        return Some(false);
    };
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
