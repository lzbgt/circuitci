use crate::board_ir::{ComponentPlacement, LayoutPoint, NetRoute, RouteSegment, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use crate::validation::BUS_PROTECTION_PLACEMENT_VALID;
use serde_json::json;

use super::super::common::validation_input_missing;
use super::required_scenario_numeric_parameter;

const EPSILON_MM: f64 = 1.0e-9;

pub(super) fn validate_bus_protection_placement(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(line_a_net) = required_string_parameter(scenario, "line_a_net", findings) else {
        return;
    };
    let Some(line_b_net) = required_string_parameter(scenario, "line_b_net", findings) else {
        return;
    };
    let Some(reference_component) =
        required_string_parameter(scenario, "reference_component", findings)
    else {
        return;
    };
    let Some(checked_component) =
        required_string_parameter(scenario, "checked_component", findings)
    else {
        return;
    };
    let Some(max_reference_to_checked_route_distance_mm) = required_positive_parameter(
        scenario,
        "max_reference_to_checked_route_distance_mm",
        findings,
    ) else {
        return;
    };
    let Some(max_component_to_route_distance_mm) =
        required_positive_parameter(scenario, "max_component_to_route_distance_mm", findings)
    else {
        return;
    };

    if line_a_net == line_b_net {
        findings.push(input_finding(
            scenario,
            "Bus placement line_a_net and line_b_net must be different.".to_string(),
        ));
        return;
    }
    if !bound.project.board.nets.contains_key(line_a_net) {
        findings.push(input_finding(
            scenario,
            format!("Bus placement line_a_net {line_a_net} is not declared."),
        ));
        return;
    }
    if !bound.project.board.nets.contains_key(line_b_net) {
        findings.push(input_finding(
            scenario,
            format!("Bus placement line_b_net {line_b_net} is not declared."),
        ));
        return;
    }

    let Some(reference_placement) =
        placement_evidence(bound, scenario, reference_component, findings)
    else {
        return;
    };
    let Some(checked_placement) = placement_evidence(bound, scenario, checked_component, findings)
    else {
        return;
    };

    let Some(line_a_route) = route_evidence(bound, scenario, line_a_net, findings) else {
        return;
    };
    let Some(line_b_route) = route_evidence(bound, scenario, line_b_net, findings) else {
        return;
    };

    let reference_point = PlacementPoint::from(reference_placement);
    let checked_point = PlacementPoint::from(checked_placement);
    let Some(line_a_distance_mm) = route_distance_between_points(
        line_a_route,
        reference_point,
        checked_point,
        max_component_to_route_distance_mm,
    ) else {
        findings.push(off_route_finding(
            scenario,
            checked_component,
            line_a_net,
            reference_component,
            max_component_to_route_distance_mm,
        ));
        return;
    };
    let Some(line_b_distance_mm) = route_distance_between_points(
        line_b_route,
        reference_point,
        checked_point,
        max_component_to_route_distance_mm,
    ) else {
        findings.push(off_route_finding(
            scenario,
            checked_component,
            line_b_net,
            reference_component,
            max_component_to_route_distance_mm,
        ));
        return;
    };

    let worst_route_distance_mm = line_a_distance_mm.max(line_b_distance_mm);
    if worst_route_distance_mm > max_reference_to_checked_route_distance_mm {
        findings.push(route_distance_finding(
            scenario,
            checked_component,
            line_a_net,
            line_b_net,
            reference_component,
            line_a_distance_mm,
            line_b_distance_mm,
            max_reference_to_checked_route_distance_mm,
            max_component_to_route_distance_mm,
        ));
    }
}

fn required_string_parameter<'a>(
    scenario: &'a Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<&'a str> {
    let Some(raw) = scenario.parameters.get(name) else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} is required."),
        );
        return None;
    };
    let Some(value) = raw.as_str() else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be a string."),
        );
        return None;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must not be blank."),
        );
        return None;
    }
    Some(trimmed)
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

fn placement_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    component_id: &str,
    findings: &mut Vec<Finding>,
) -> Option<&'a ComponentPlacement> {
    let Some(placement) = bound.project.board.layout.placements.get(component_id) else {
        findings.push(input_finding(
            scenario,
            format!("Bus placement component {component_id} has no board.layout.placements entry."),
        ));
        return None;
    };
    if !placement.x_mm.is_finite() || !placement.y_mm.is_finite() {
        findings.push(input_finding(
            scenario,
            format!("Bus placement component {component_id} coordinates must be finite."),
        ));
        return None;
    }
    Some(placement)
}

fn route_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    net: &str,
    findings: &mut Vec<Finding>,
) -> Option<&'a NetRoute> {
    let Some(route) = bound.project.board.layout.routes.get(net) else {
        findings.push(input_finding(
            scenario,
            format!("Bus placement net {net} has no board.layout.routes entry."),
        ));
        return None;
    };
    if let Err(message) = validate_route_shape(route, net) {
        findings.push(input_finding(scenario, message));
        return None;
    }
    Some(route)
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

#[derive(Debug, Clone)]
struct RouteProjection {
    distance_to_point_mm: f64,
    cumulative_route_distance_mm: f64,
}

fn validate_route_shape(route: &NetRoute, net: &str) -> Result<(), String> {
    if route.segments.is_empty() {
        return Err(format!(
            "Bus placement route for {net} must include at least one segment."
        ));
    }
    for segment in &route.segments {
        let start = PlacementPoint::from(&segment.start);
        let end = PlacementPoint::from(&segment.end);
        if !point_is_finite(start) || !point_is_finite(end) {
            return Err(format!(
                "Bus placement route for {net} has non-finite endpoints."
            ));
        }
        if segment.width_mm <= 0.0 || !segment.width_mm.is_finite() {
            return Err(format!(
                "Bus placement route for {net} has non-positive or non-finite width_mm."
            ));
        }
        if segment.layer.trim().is_empty() {
            return Err(format!(
                "Bus placement route for {net} has a segment with blank layer."
            ));
        }
        if segment_length_mm(segment) <= EPSILON_MM {
            return Err(format!(
                "Bus placement route for {net} has a zero-length segment."
            ));
        }
    }
    for pair in route.segments.windows(2) {
        let left = PlacementPoint::from(&pair[0].end);
        let right = PlacementPoint::from(&pair[1].start);
        if point_distance_mm(left, right) > EPSILON_MM {
            return Err(format!(
                "Bus placement route for {net} must be an ordered continuous polyline."
            ));
        }
    }
    Ok(())
}

fn route_distance_between_points(
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
    Some(
        (from_projection.cumulative_route_distance_mm - to_projection.cumulative_route_distance_mm)
            .abs(),
    )
}

fn nearest_projection(route: &NetRoute, point: PlacementPoint) -> Option<RouteProjection> {
    let mut best = None;
    let mut cumulative_route_distance_mm = 0.0;
    for segment in &route.segments {
        let start = PlacementPoint::from(&segment.start);
        let end = PlacementPoint::from(&segment.end);
        let segment_length_mm = segment_length_mm(segment);
        let Some((distance_along_segment_mm, projected)) =
            project_point_to_segment(point, start, end)
        else {
            cumulative_route_distance_mm += segment_length_mm;
            continue;
        };
        let distance_to_point_mm = point_distance_mm(point, projected);
        let projection = RouteProjection {
            distance_to_point_mm,
            cumulative_route_distance_mm: cumulative_route_distance_mm + distance_along_segment_mm,
        };
        if best.as_ref().is_none_or(|current: &RouteProjection| {
            distance_to_point_mm < current.distance_to_point_mm
        }) {
            best = Some(projection);
        }
        cumulative_route_distance_mm += segment_length_mm;
    }
    best
}

fn project_point_to_segment(
    point: PlacementPoint,
    start: PlacementPoint,
    end: PlacementPoint,
) -> Option<(f64, PlacementPoint)> {
    let dx = end.x_mm - start.x_mm;
    let dy = end.y_mm - start.y_mm;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= EPSILON_MM {
        return None;
    }
    let raw_t = ((point.x_mm - start.x_mm) * dx + (point.y_mm - start.y_mm) * dy) / length_squared;
    let t = raw_t.clamp(0.0, 1.0);
    let projected = PlacementPoint {
        x_mm: start.x_mm + t * dx,
        y_mm: start.y_mm + t * dy,
    };
    Some((t * length_squared.sqrt(), projected))
}

fn point_is_finite(point: PlacementPoint) -> bool {
    point.x_mm.is_finite() && point.y_mm.is_finite()
}

fn point_distance_mm(left: PlacementPoint, right: PlacementPoint) -> f64 {
    (left.x_mm - right.x_mm).hypot(left.y_mm - right.y_mm)
}

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    point_distance_mm(
        PlacementPoint::from(&segment.start),
        PlacementPoint::from(&segment.end),
    )
}

fn input_finding(scenario: &Scenario, message: String) -> Finding {
    let mut finding = Finding::critical(BUS_PROTECTION_PLACEMENT_VALID, &scenario.name, message);
    finding.suggested_fixes = vec![
        "Add explicit board.layout.placements and ordered board.layout.routes evidence for the bus reference and checked component.".to_string(),
        "Keep BUS_PROTECTION_PLACEMENT_VALID scenario limits tied to the selected layout policy or datasheet/application-note guidance.".to_string(),
    ];
    finding
}

fn off_route_finding(
    scenario: &Scenario,
    component_id: &str,
    net: &str,
    reference_component: &str,
    max_component_to_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        BUS_PROTECTION_PLACEMENT_VALID,
        &scenario.name,
        format!(
            "Bus placement component {component_id} or reference component {reference_component} is not on route {net} within {max_component_to_route_distance_mm:.3} mm."
        ),
    );
    finding.component = Some(component_id.to_string());
    finding.measured.insert("net".to_string(), json!(net));
    finding.measured.insert(
        "reference_component".to_string(),
        json!(reference_component),
    );
    finding.limit.insert(
        "max_component_to_route_distance_mm".to_string(),
        json!(max_component_to_route_distance_mm),
    );
    finding.suggested_fixes = vec![
        "Place the protection or termination component directly on the routed bus pair instead of behind a long stub.".to_string(),
        "If this is a pre-layout contract, update board.layout.routes to match the intended ordered route path.".to_string(),
    ];
    finding
}

#[allow(clippy::too_many_arguments)]
fn route_distance_finding(
    scenario: &Scenario,
    component_id: &str,
    line_a_net: &str,
    line_b_net: &str,
    reference_component: &str,
    line_a_route_distance_mm: f64,
    line_b_route_distance_mm: f64,
    max_reference_to_checked_route_distance_mm: f64,
    max_component_to_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        BUS_PROTECTION_PLACEMENT_VALID,
        &scenario.name,
        format!(
            "Bus placement component {component_id} is too far from {reference_component}: route distances are {line_a_route_distance_mm:.3} mm on {line_a_net} and {line_b_route_distance_mm:.3} mm on {line_b_net}, limit {max_reference_to_checked_route_distance_mm:.3} mm."
        ),
    );
    finding.component = Some(component_id.to_string());
    finding
        .measured
        .insert("line_a_net".to_string(), json!(line_a_net));
    finding
        .measured
        .insert("line_b_net".to_string(), json!(line_b_net));
    finding.measured.insert(
        "reference_component".to_string(),
        json!(reference_component),
    );
    finding.measured.insert(
        "line_a_route_distance_mm".to_string(),
        json!(line_a_route_distance_mm),
    );
    finding.measured.insert(
        "line_b_route_distance_mm".to_string(),
        json!(line_b_route_distance_mm),
    );
    finding.limit.insert(
        "max_reference_to_checked_route_distance_mm".to_string(),
        json!(max_reference_to_checked_route_distance_mm),
    );
    finding.limit.insert(
        "max_component_to_route_distance_mm".to_string(),
        json!(max_component_to_route_distance_mm),
    );
    finding.suggested_fixes = vec![
        "Move the checked protection or termination component closer to the connector/transceiver reference point on both bus traces.".to_string(),
        "Use the imported routed board geometry to prove the final placement before fabrication.".to_string(),
    ];
    finding
}
