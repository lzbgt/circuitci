use crate::board_ir::{
    ComponentPlacement, LayoutPoint, NetLayoutRule, NetRoute, RouteSegment, Scenario,
};
use crate::library::{BoundBoard, UsbConnector};
use crate::reports::Finding;
use serde_json::json;

use super::{
    UsbConnectorSignal, placement_is_finite, required_scenario_numeric_parameter,
    valid_component_placement, valid_protection_clamps_for_net,
};
use crate::validation::USB_ROUTE_GEOMETRY_VALID;
use crate::validation::common::validation_input_missing;

const EPSILON_MM: f64 = 1.0e-9;

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

fn worst_route_width_delta(route: &NetRoute, expected_width_mm: f64) -> Option<(usize, f64, f64)> {
    route
        .segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            let delta = (segment.width_mm - expected_width_mm).abs();
            (index, segment.width_mm, delta)
        })
        .max_by(|left, right| left.2.total_cmp(&right.2))
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

#[derive(Debug, Clone, Copy)]
struct UsbPairGapEvidence {
    dp_segment_index: usize,
    dm_segment_index: usize,
    centerline_distance_mm: f64,
    measured_gap_mm: f64,
    expected_gap_mm: f64,
    gap_delta_mm: f64,
}

fn worst_pair_gap_delta(
    dp_route: &NetRoute,
    dm_route: &NetRoute,
    expected_gap_mm: f64,
) -> Option<UsbPairGapEvidence> {
    let mut worst = None;
    for (dp_index, dp_segment) in dp_route.segments.iter().enumerate() {
        for (dm_index, dm_segment) in dm_route.segments.iter().enumerate() {
            let Some((centerline_distance_mm, measured_gap_mm)) =
                parallel_overlap_gap_mm(dp_segment, dm_segment)
            else {
                continue;
            };
            let gap_delta_mm = (measured_gap_mm - expected_gap_mm).abs();
            let evidence = UsbPairGapEvidence {
                dp_segment_index: dp_index,
                dm_segment_index: dm_index,
                centerline_distance_mm,
                measured_gap_mm,
                expected_gap_mm,
                gap_delta_mm,
            };
            if worst
                .as_ref()
                .is_none_or(|current: &UsbPairGapEvidence| gap_delta_mm > current.gap_delta_mm)
            {
                worst = Some(evidence);
            }
        }
    }
    worst
}

fn parallel_overlap_gap_mm(
    dp_segment: &RouteSegment,
    dm_segment: &RouteSegment,
) -> Option<(f64, f64)> {
    let dp_start = PlacementPoint::from(&dp_segment.start);
    let dp_end = PlacementPoint::from(&dp_segment.end);
    let dm_start = PlacementPoint::from(&dm_segment.start);
    let dm_end = PlacementPoint::from(&dm_segment.end);
    let dp_dx = dp_end.x_mm - dp_start.x_mm;
    let dp_dy = dp_end.y_mm - dp_start.y_mm;
    let dm_dx = dm_end.x_mm - dm_start.x_mm;
    let dm_dy = dm_end.y_mm - dm_start.y_mm;
    let dp_len = dp_dx.hypot(dp_dy);
    let dm_len = dm_dx.hypot(dm_dy);
    if dp_len <= EPSILON_MM || dm_len <= EPSILON_MM {
        return None;
    }
    let dp_unit_x = dp_dx / dp_len;
    let dp_unit_y = dp_dy / dp_len;
    let dm_unit_x = dm_dx / dm_len;
    let dm_unit_y = dm_dy / dm_len;
    let cross = (dp_unit_x * dm_unit_y - dp_unit_y * dm_unit_x).abs();
    if cross > 1.0e-6 {
        return None;
    }
    let projection_a =
        (dm_start.x_mm - dp_start.x_mm) * dp_unit_x + (dm_start.y_mm - dp_start.y_mm) * dp_unit_y;
    let projection_b =
        (dm_end.x_mm - dp_start.x_mm) * dp_unit_x + (dm_end.y_mm - dp_start.y_mm) * dp_unit_y;
    let overlap_start = projection_a.min(projection_b).max(0.0);
    let overlap_end = projection_a.max(projection_b).min(dp_len);
    if overlap_end - overlap_start <= EPSILON_MM {
        return None;
    }
    let centerline_distance_mm = ((dm_start.x_mm - dp_start.x_mm) * dp_unit_y
        - (dm_start.y_mm - dp_start.y_mm) * dp_unit_x)
        .abs();
    let measured_gap_mm =
        centerline_distance_mm - (dp_segment.width_mm + dm_segment.width_mm) / 2.0;
    Some((centerline_distance_mm, measured_gap_mm))
}

#[derive(Debug, Clone, Copy)]
struct PlacementPoint {
    x_mm: f64,
    y_mm: f64,
}

impl From<&ComponentPlacement> for PlacementPoint {
    fn from(placement: &ComponentPlacement) -> Self {
        Self {
            x_mm: placement.x_mm,
            y_mm: placement.y_mm,
        }
    }
}

impl From<&LayoutPoint> for PlacementPoint {
    fn from(point: &LayoutPoint) -> Self {
        Self {
            x_mm: point.x_mm,
            y_mm: point.y_mm,
        }
    }
}

fn validate_route_shape(route: &NetRoute) -> Result<(), String> {
    if route.segments.is_empty() {
        return Err("USB route geometry must include at least one segment.".to_string());
    }
    for segment in &route.segments {
        let start = PlacementPoint::from(&segment.start);
        let end = PlacementPoint::from(&segment.end);
        if !point_is_finite(start) || !point_is_finite(end) {
            return Err("USB route segment endpoints must be finite.".to_string());
        }
        if segment.width_mm <= 0.0 || !segment.width_mm.is_finite() {
            return Err("USB route segment width_mm must be finite and positive.".to_string());
        }
        if segment.layer.trim().is_empty() {
            return Err("USB route segment layer must be non-empty.".to_string());
        }
        if point_distance_mm(start, end) <= EPSILON_MM {
            return Err("USB route segment length must be greater than zero.".to_string());
        }
    }
    for via in &route.vias {
        let at = PlacementPoint::from(&via.at);
        if !point_is_finite(at) {
            return Err("USB route via coordinate must be finite.".to_string());
        }
        if via.size_mm <= 0.0 || !via.size_mm.is_finite() {
            return Err("USB route via size_mm must be finite and positive.".to_string());
        }
        if via.drill_mm <= 0.0 || !via.drill_mm.is_finite() {
            return Err("USB route via drill_mm must be finite and positive.".to_string());
        }
    }
    Ok(())
}

fn route_length_mm(route: &NetRoute) -> f64 {
    route
        .segments
        .iter()
        .map(|segment| {
            point_distance_mm(
                PlacementPoint::from(&segment.start),
                PlacementPoint::from(&segment.end),
            )
        })
        .sum()
}

fn route_distance_between_placements(
    route: &NetRoute,
    from: PlacementPoint,
    to: PlacementPoint,
    max_point_to_route_distance_mm: f64,
) -> Option<f64> {
    let from_projection = nearest_projection(route, from)?;
    let to_projection = nearest_projection(route, to)?;
    if from_projection.distance_to_point_mm > max_point_to_route_distance_mm
        || to_projection.distance_to_point_mm > max_point_to_route_distance_mm
    {
        return None;
    }
    shortest_route_distance_mm(route, &from_projection, &to_projection)
}

#[derive(Debug, Clone, Copy)]
struct Projection {
    segment_index: usize,
    t: f64,
    point: PlacementPoint,
    distance_to_point_mm: f64,
}

fn nearest_projection(route: &NetRoute, point: PlacementPoint) -> Option<Projection> {
    route
        .segments
        .iter()
        .enumerate()
        .filter_map(|(segment_index, segment)| {
            let start = PlacementPoint::from(&segment.start);
            let end = PlacementPoint::from(&segment.end);
            project_point_to_segment(point, start, end).map(|(t, projected)| Projection {
                segment_index,
                t,
                point: projected,
                distance_to_point_mm: point_distance_mm(point, projected),
            })
        })
        .min_by(|left, right| {
            left.distance_to_point_mm
                .total_cmp(&right.distance_to_point_mm)
        })
}

fn project_point_to_segment(
    point: PlacementPoint,
    start: PlacementPoint,
    end: PlacementPoint,
) -> Option<(f64, PlacementPoint)> {
    let dx = end.x_mm - start.x_mm;
    let dy = end.y_mm - start.y_mm;
    let length_squared = dx.mul_add(dx, dy * dy);
    if length_squared <= EPSILON_MM {
        return None;
    }
    let raw_t = ((point.x_mm - start.x_mm) * dx + (point.y_mm - start.y_mm) * dy) / length_squared;
    let t = raw_t.clamp(0.0, 1.0);
    Some((
        t,
        PlacementPoint {
            x_mm: start.x_mm + t * dx,
            y_mm: start.y_mm + t * dy,
        },
    ))
}

fn shortest_route_distance_mm(
    route: &NetRoute,
    from_projection: &Projection,
    to_projection: &Projection,
) -> Option<f64> {
    let mut graph = RouteGraph::default();
    for (segment_index, segment) in route.segments.iter().enumerate() {
        add_segment_to_graph(
            &mut graph,
            segment_index,
            segment,
            from_projection,
            to_projection,
        );
    }
    let from_node = graph.find_node(from_projection.point)?;
    let to_node = graph.find_node(to_projection.point)?;
    graph.shortest_distance(from_node, to_node)
}

#[derive(Default)]
struct RouteGraph {
    nodes: Vec<PlacementPoint>,
    edges: Vec<Vec<(usize, f64)>>,
}

impl RouteGraph {
    fn node_for(&mut self, point: PlacementPoint) -> usize {
        if let Some(index) = self.find_node(point) {
            return index;
        }
        let index = self.nodes.len();
        self.nodes.push(point);
        self.edges.push(Vec::new());
        index
    }

    fn find_node(&self, point: PlacementPoint) -> Option<usize> {
        self.nodes
            .iter()
            .position(|candidate| points_equal(*candidate, point))
    }

    fn connect(&mut self, a: usize, b: usize, distance_mm: f64) {
        if a == b || distance_mm <= EPSILON_MM {
            return;
        }
        self.edges[a].push((b, distance_mm));
        self.edges[b].push((a, distance_mm));
    }

    fn shortest_distance(&self, start: usize, end: usize) -> Option<f64> {
        let mut distances = vec![f64::INFINITY; self.nodes.len()];
        let mut visited = vec![false; self.nodes.len()];
        distances[start] = 0.0;
        loop {
            let Some(current) = distances
                .iter()
                .enumerate()
                .filter(|(index, _)| !visited[*index])
                .min_by(|(_, left), (_, right)| left.total_cmp(right))
                .map(|(index, _)| index)
            else {
                break;
            };
            if current == end {
                return Some(distances[current]);
            }
            visited[current] = true;
            for (next, edge_distance) in &self.edges[current] {
                let candidate = distances[current] + edge_distance;
                if candidate < distances[*next] {
                    distances[*next] = candidate;
                }
            }
        }
        distances[end].is_finite().then_some(distances[end])
    }
}

fn add_segment_to_graph(
    graph: &mut RouteGraph,
    segment_index: usize,
    segment: &RouteSegment,
    from_projection: &Projection,
    to_projection: &Projection,
) {
    let start = PlacementPoint::from(&segment.start);
    let end = PlacementPoint::from(&segment.end);
    let mut points = vec![(0.0, start), (1.0, end)];
    if from_projection.segment_index == segment_index {
        points.push((from_projection.t, from_projection.point));
    }
    if to_projection.segment_index == segment_index {
        points.push((to_projection.t, to_projection.point));
    }
    points.sort_by(|left, right| left.0.total_cmp(&right.0));
    points.dedup_by(|left, right| points_equal(left.1, right.1));
    for window in points.windows(2) {
        let first = graph.node_for(window[0].1);
        let second = graph.node_for(window[1].1);
        graph.connect(first, second, point_distance_mm(window[0].1, window[1].1));
    }
}

fn point_is_finite(point: PlacementPoint) -> bool {
    point.x_mm.is_finite() && point.y_mm.is_finite()
}

fn points_equal(a: PlacementPoint, b: PlacementPoint) -> bool {
    (a.x_mm - b.x_mm).abs() <= EPSILON_MM && (a.y_mm - b.y_mm).abs() <= EPSILON_MM
}

fn point_distance_mm(a: PlacementPoint, b: PlacementPoint) -> f64 {
    let dx = a.x_mm - b.x_mm;
    let dy = a.y_mm - b.y_mm;
    dx.hypot(dy)
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

fn usb_route_metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(USB_ROUTE_GEOMETRY_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Import PCB route geometry with import-kicad-pcb before declaring USB_ROUTE_GEOMETRY_VALID.".to_string(),
        "Declare route limits from the board USB/layout rule instead of inferring them from coordinates.".to_string(),
    ];
    finding
}

fn usb_route_length_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    route_length_mm: f64,
    max_route_length_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} route length {:.3} mm exceeds limit {:.3} mm.",
            signal.label(),
            route_length_mm,
            max_route_length_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .measured
        .insert("route_length_mm".to_string(), json!(route_length_mm));
    finding.limit.insert(
        "max_data_line_route_length_mm".to_string(),
        json!(max_route_length_mm),
    );
    finding.suggested_fixes = vec![
        "Shorten the USB data-line route or move the connector/protected device closer together."
            .to_string(),
        "Use a board-specific USB layout rule for max_data_line_route_length_mm.".to_string(),
    ];
    finding
}

fn usb_route_via_count_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    via_count: usize,
    max_via_count: usize,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} has {via_count} vias, above limit {max_via_count}.",
            signal.label()
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .measured
        .insert("via_count".to_string(), json!(via_count));
    finding
        .limit
        .insert("max_data_line_via_count".to_string(), json!(max_via_count));
    finding.suggested_fixes = vec![
        "Reduce USB data-line layer changes or relax max_data_line_via_count only with layout/SI justification.".to_string(),
        "Keep D+ and D- via usage symmetric when the board route must change layers.".to_string(),
    ];
    finding
}

fn usb_route_width_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    evidence: UsbRouteWidthEvidence,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} segment {} width {:.3} mm differs from route rule {:.3} mm by {:.3} mm, above tolerance {:.3} mm.",
            signal.label(),
            evidence.segment_index,
            evidence.measured_width_mm,
            evidence.expected_width_mm,
            evidence.width_delta_mm,
            evidence.max_width_delta_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .measured
        .insert("segment_index".to_string(), json!(evidence.segment_index));
    finding.measured.insert(
        "route_segment_width_mm".to_string(),
        json!(evidence.measured_width_mm),
    );
    finding.measured.insert(
        "route_width_delta_mm".to_string(),
        json!(evidence.width_delta_mm),
    );
    finding.limit.insert(
        "expected_data_line_width_mm".to_string(),
        json!(evidence.expected_width_mm),
    );
    finding.limit.insert(
        "max_data_line_width_delta_mm".to_string(),
        json!(evidence.max_width_delta_mm),
    );
    finding.suggested_fixes = vec![
        "Update the routed USB data-line width to match the imported PCB route rule.".to_string(),
        "If the route intentionally necks down, encode that exception as a more specific board rule instead of relaxing the global USB route check.".to_string(),
    ];
    finding
}

#[derive(Debug, Clone, Copy)]
struct UsbRouteWidthEvidence {
    segment_index: usize,
    measured_width_mm: f64,
    expected_width_mm: f64,
    width_delta_mm: f64,
    max_width_delta_mm: f64,
}

fn usb_route_no_protection_path_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    missing_placements: &[String],
    off_route_components: &[String],
    max_component_to_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} has no protection component with usable route-distance evidence.",
            signal.label()
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
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
        "Place the USB ESD component on the routed USB net near the connector and import updated PCB route geometry.".to_string(),
        "Check that component placement coordinates and route coordinates share the same PCB coordinate system.".to_string(),
    ];
    finding
}

fn usb_pair_gap_unmeasured_finding(
    scenario: &Scenario,
    connector_id: &str,
    dp_net: &str,
    dm_net: &str,
    expected_gap_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} D+/D- nets {dp_net}/{dm_net} have no overlapping parallel routed segments for diff-pair gap validation."
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.measured.insert("dp_net".to_string(), json!(dp_net));
    finding.measured.insert("dm_net".to_string(), json!(dm_net));
    finding.limit.insert(
        "expected_data_pair_gap_mm".to_string(),
        json!(expected_gap_mm),
    );
    finding.suggested_fixes = vec![
        "Route USB D+ and D- as overlapping parallel segments where differential-pair gap can be measured.".to_string(),
        "Import updated PCB route geometry after routing the data lines as a differential pair.".to_string(),
    ];
    finding
}

fn usb_pair_gap_finding(
    scenario: &Scenario,
    connector_id: &str,
    dp_net: &str,
    dm_net: &str,
    evidence: UsbPairGapEvidence,
    max_gap_delta_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} D+/D- edge gap {:.3} mm differs from route rule {:.3} mm by {:.3} mm, above tolerance {:.3} mm.",
            evidence.measured_gap_mm,
            evidence.expected_gap_mm,
            evidence.gap_delta_mm,
            max_gap_delta_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.measured.insert("dp_net".to_string(), json!(dp_net));
    finding.measured.insert("dm_net".to_string(), json!(dm_net));
    finding.measured.insert(
        "dp_segment_index".to_string(),
        json!(evidence.dp_segment_index),
    );
    finding.measured.insert(
        "dm_segment_index".to_string(),
        json!(evidence.dm_segment_index),
    );
    finding.measured.insert(
        "data_pair_centerline_distance_mm".to_string(),
        json!(evidence.centerline_distance_mm),
    );
    finding.measured.insert(
        "data_pair_gap_mm".to_string(),
        json!(evidence.measured_gap_mm),
    );
    finding.measured.insert(
        "data_pair_gap_delta_mm".to_string(),
        json!(evidence.gap_delta_mm),
    );
    finding.limit.insert(
        "expected_data_pair_gap_mm".to_string(),
        json!(evidence.expected_gap_mm),
    );
    finding.limit.insert(
        "max_data_pair_gap_delta_mm".to_string(),
        json!(max_gap_delta_mm),
    );
    finding.suggested_fixes = vec![
        "Route D+ and D- with the imported differential-pair gap or update the board rule if the impedance target changed.".to_string(),
        "Avoid local spreading or necking of only one member of the USB data pair unless captured by a more specific layout rule.".to_string(),
    ];
    finding
}

fn usb_route_protection_distance_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    protection_component: &str,
    route_distance_mm: f64,
    max_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} reaches protection component {protection_component} after {:.3} mm of route, exceeding limit {:.3} mm.",
            signal.label(),
            route_distance_mm,
            max_route_distance_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding.measured.insert(
        "protection_component".to_string(),
        json!(protection_component),
    );
    finding.measured.insert(
        "connector_to_protection_route_distance_mm".to_string(),
        json!(route_distance_mm),
    );
    finding.limit.insert(
        "max_connector_to_protection_route_distance_mm".to_string(),
        json!(max_route_distance_mm),
    );
    finding.suggested_fixes = vec![
        "Move the ESD component closer to the USB connector along the routed data line.".to_string(),
        "Route connector pins through the protection device before continuing to the USB transceiver.".to_string(),
    ];
    finding
}

fn usb_pair_length_mismatch_finding(
    scenario: &Scenario,
    connector_id: &str,
    evidence: UsbPairLengthEvidence<'_>,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} D+/D- route length mismatch {:.3} mm exceeds limit {:.3} mm.",
            evidence.length_mismatch_mm, evidence.max_length_mismatch_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding
        .measured
        .insert("dp_net".to_string(), json!(evidence.dp_net));
    finding
        .measured
        .insert("dm_net".to_string(), json!(evidence.dm_net));
    finding.measured.insert(
        "dp_route_length_mm".to_string(),
        json!(evidence.dp_length_mm),
    );
    finding.measured.insert(
        "dm_route_length_mm".to_string(),
        json!(evidence.dm_length_mm),
    );
    finding.measured.insert(
        "data_pair_length_mismatch_mm".to_string(),
        json!(evidence.length_mismatch_mm),
    );
    finding.limit.insert(
        "max_data_pair_length_mismatch_mm".to_string(),
        json!(evidence.max_length_mismatch_mm),
    );
    finding.suggested_fixes = vec![
        "Length-match the USB D+ and D- routes within the board's USB routing rule.".to_string(),
        "Route D+ and D- as a pair and avoid unnecessary jogs or detours on only one line."
            .to_string(),
    ];
    finding
}

#[derive(Debug, Clone, Copy)]
struct UsbPairLengthEvidence<'a> {
    dp_net: &'a str,
    dm_net: &'a str,
    dp_length_mm: f64,
    dm_length_mm: f64,
    length_mismatch_mm: f64,
    max_length_mismatch_mm: f64,
}

fn usb_pair_via_delta_finding(
    scenario: &Scenario,
    connector_id: &str,
    evidence: UsbPairViaEvidence<'_>,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} D+/D- via-count delta {} exceeds limit {}.",
            evidence.via_count_delta, evidence.max_via_count_delta
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding
        .measured
        .insert("dp_net".to_string(), json!(evidence.dp_net));
    finding
        .measured
        .insert("dm_net".to_string(), json!(evidence.dm_net));
    finding
        .measured
        .insert("dp_via_count".to_string(), json!(evidence.dp_via_count));
    finding
        .measured
        .insert("dm_via_count".to_string(), json!(evidence.dm_via_count));
    finding.measured.insert(
        "data_pair_via_count_delta".to_string(),
        json!(evidence.via_count_delta),
    );
    finding.limit.insert(
        "max_data_pair_via_count_delta".to_string(),
        json!(evidence.max_via_count_delta),
    );
    finding.suggested_fixes = vec![
        "Keep D+ and D- layer changes symmetric when vias are unavoidable.".to_string(),
        "Remove unnecessary vias from one side of the USB pair or add the matching transition only when the layout stackup requires it.".to_string(),
    ];
    finding
}

#[derive(Debug, Clone, Copy)]
struct UsbPairViaEvidence<'a> {
    dp_net: &'a str,
    dm_net: &'a str,
    dp_via_count: usize,
    dm_via_count: usize,
    via_count_delta: usize,
    max_via_count_delta: usize,
}
