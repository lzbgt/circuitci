use crate::board_ir::{CopperZone, NetRoute, RouteSegment, Scenario, StackupLayerKind};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::common::validation_input_missing;
use super::super::{ADJACENT_PLANE_RETURN_PATH_VALID, REFERENCE_PLANE_SLOT_CROSSING_VALID};

pub(in crate::validation) fn validate_adjacent_plane_return_path(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(rules) = route_rules(bound, scenario, findings) else {
        return;
    };
    for rule in rules {
        validate_rule(bound, scenario, findings, rule);
    }
}

pub(in crate::validation) fn validate_reference_plane_slot_crossing(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(rules) = slot_crossing_rules(bound, scenario, findings) else {
        return;
    };
    for rule in rules {
        validate_slot_crossing_rule(bound, scenario, findings, rule);
    }
}

#[derive(Debug)]
struct RouteRule {
    net: String,
    reference_net: String,
    max_unreferenced_length_mm: f64,
    reference_layer: Option<String>,
}

#[derive(Debug)]
struct RouteMeasurement {
    total_length_mm: f64,
    unreferenced_length_mm: f64,
    unreferenced_segment_count: usize,
    first_unreferenced_segment_index: Option<usize>,
    first_unreferenced_route_layer: Option<String>,
    first_unreferenced_reference_layer: Option<String>,
    reference_layers: Vec<String>,
}

#[derive(Debug)]
struct SlotCrossingRule {
    net: String,
    reference_net: String,
    max_slot_crossings: usize,
    reference_layer: Option<String>,
}

#[derive(Debug)]
struct SlotCrossingMeasurement {
    total_route_length_mm: f64,
    slot_crossing_count: usize,
    first_slot_segment_index: Option<usize>,
    first_slot_route_layer: Option<String>,
    first_slot_reference_layer: Option<String>,
    first_slot_start_mm: Option<f64>,
    first_slot_end_mm: Option<f64>,
    reference_layers: Vec<String>,
}

fn route_rules(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<Vec<RouteRule>> {
    let Some(value) = scenario.parameters.get("routes") else {
        validation_input_missing(
            findings,
            scenario,
            "ADJACENT_PLANE_RETURN_PATH_VALID requires parameters.routes.",
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            "ADJACENT_PLANE_RETURN_PATH_VALID parameters.routes must be a list.",
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "ADJACENT_PLANE_RETURN_PATH_VALID parameters.routes must not be empty.",
        );
        return None;
    }
    let mut rules = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let mapping = match item.as_mapping() {
            Some(mapping) => mapping,
            None => {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "ADJACENT_PLANE_RETURN_PATH_VALID parameters.routes[{index}] must be an object."
                    ),
                );
                return None;
            }
        };
        let net = required_string(scenario, findings, mapping, index, "net")?;
        let reference_net = required_string(scenario, findings, mapping, index, "reference_net")?;
        require_declared_net(bound, scenario, findings, index, &net)?;
        require_declared_net(bound, scenario, findings, index, &reference_net)?;
        let max_unreferenced_length_mm = required_non_negative_number(
            scenario,
            findings,
            mapping,
            index,
            "max_unreferenced_length_mm",
        )?;
        let reference_layer = optional_string(mapping, "reference_layer");
        rules.push(RouteRule {
            net,
            reference_net,
            max_unreferenced_length_mm,
            reference_layer,
        });
    }
    Some(rules)
}

fn slot_crossing_rules(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<Vec<SlotCrossingRule>> {
    let Some(value) = scenario.parameters.get("routes") else {
        validation_input_missing(
            findings,
            scenario,
            "REFERENCE_PLANE_SLOT_CROSSING_VALID requires parameters.routes.",
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            "REFERENCE_PLANE_SLOT_CROSSING_VALID parameters.routes must be a list.",
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "REFERENCE_PLANE_SLOT_CROSSING_VALID parameters.routes must not be empty.",
        );
        return None;
    }
    let mut rules = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let mapping = match item.as_mapping() {
            Some(mapping) => mapping,
            None => {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "REFERENCE_PLANE_SLOT_CROSSING_VALID parameters.routes[{index}] must be an object."
                    ),
                );
                return None;
            }
        };
        let net = required_string_for_check(
            scenario,
            findings,
            mapping,
            REFERENCE_PLANE_SLOT_CROSSING_VALID,
            index,
            "net",
        )?;
        let reference_net = required_string_for_check(
            scenario,
            findings,
            mapping,
            REFERENCE_PLANE_SLOT_CROSSING_VALID,
            index,
            "reference_net",
        )?;
        require_declared_net_for_check(
            bound,
            scenario,
            findings,
            REFERENCE_PLANE_SLOT_CROSSING_VALID,
            index,
            &net,
        )?;
        require_declared_net_for_check(
            bound,
            scenario,
            findings,
            REFERENCE_PLANE_SLOT_CROSSING_VALID,
            index,
            &reference_net,
        )?;
        let max_slot_crossings = required_non_negative_integer(
            scenario,
            findings,
            mapping,
            index,
            "max_slot_crossings",
        )?;
        let reference_layer = optional_string(mapping, "reference_layer");
        rules.push(SlotCrossingRule {
            net,
            reference_net,
            max_slot_crossings,
            reference_layer,
        });
    }
    Some(rules)
}

fn validate_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: RouteRule,
) {
    if bound.project.board.layout.stackup.layers.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "ADJACENT_PLANE_RETURN_PATH_VALID requires board.layout.stackup.layers evidence.",
        );
        return;
    }
    let Some(route) = route_for_net(
        bound,
        scenario,
        findings,
        ADJACENT_PLANE_RETURN_PATH_VALID,
        &rule.net,
    ) else {
        return;
    };
    let Some(zones) = bound.project.board.layout.zones.get(&rule.reference_net) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "ADJACENT_PLANE_RETURN_PATH_VALID reference net {} has no board.layout.zones entry.",
                rule.reference_net
            ),
        );
        return;
    };
    let Some(measurement) = measure_route(bound, scenario, findings, &rule, route, zones) else {
        return;
    };
    if measurement.unreferenced_length_mm > rule.max_unreferenced_length_mm + f64::EPSILON {
        findings.push(return_path_finding(scenario, &rule, measurement));
    }
}

fn validate_slot_crossing_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: SlotCrossingRule,
) {
    if bound.project.board.layout.stackup.layers.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "REFERENCE_PLANE_SLOT_CROSSING_VALID requires board.layout.stackup.layers evidence.",
        );
        return;
    }
    let Some(route) = route_for_net(
        bound,
        scenario,
        findings,
        REFERENCE_PLANE_SLOT_CROSSING_VALID,
        &rule.net,
    ) else {
        return;
    };
    let Some(zones) = bound.project.board.layout.zones.get(&rule.reference_net) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "REFERENCE_PLANE_SLOT_CROSSING_VALID reference net {} has no board.layout.zones entry.",
                rule.reference_net
            ),
        );
        return;
    };
    let Some(measurement) = measure_slot_crossings(bound, scenario, findings, &rule, route, zones)
    else {
        return;
    };
    if measurement.slot_crossing_count > rule.max_slot_crossings {
        findings.push(slot_crossing_finding(scenario, &rule, measurement));
    }
}

fn route_for_net<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    net: &str,
) -> Option<&'a NetRoute> {
    let Some(route) = bound.project.board.layout.routes.get(net) else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} net {net} has no board.layout.routes entry."),
        );
        return None;
    };
    if route.segments.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} net {net} route must include segments."),
        );
        return None;
    }
    for (index, segment) in route.segments.iter().enumerate() {
        if !segment.start.x_mm.is_finite()
            || !segment.start.y_mm.is_finite()
            || !segment.end.x_mm.is_finite()
            || !segment.end.y_mm.is_finite()
        {
            validation_input_missing(
                findings,
                scenario,
                format!("{check_id} net {net} route segment {index} endpoints must be finite."),
            );
            return None;
        }
        if !segment.width_mm.is_finite() || segment.width_mm <= 0.0 {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "{check_id} net {net} route segment {index} width_mm must be finite and positive."
                ),
            );
            return None;
        }
        if segment.layer.trim().is_empty() || segment_length_mm(segment) <= f64::EPSILON {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "{check_id} net {net} route segment {index} must have a non-empty layer and non-zero length."
                ),
            );
            return None;
        }
    }
    Some(route)
}

fn measure_route(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &RouteRule,
    route: &NetRoute,
    zones: &[CopperZone],
) -> Option<RouteMeasurement> {
    let mut measurement = RouteMeasurement {
        total_length_mm: 0.0,
        unreferenced_length_mm: 0.0,
        unreferenced_segment_count: 0,
        first_unreferenced_segment_index: None,
        first_unreferenced_route_layer: None,
        first_unreferenced_reference_layer: None,
        reference_layers: Vec::new(),
    };
    for (index, segment) in route.segments.iter().enumerate() {
        let length = segment_length_mm(segment);
        measurement.total_length_mm += length;
        let reference_layer = resolve_reference_layer(
            bound,
            scenario,
            findings,
            ADJACENT_PLANE_RETURN_PATH_VALID,
            &rule.reference_net,
            rule.reference_layer.as_deref(),
            &segment.layer,
        )?;
        if !measurement.reference_layers.contains(&reference_layer) {
            measurement.reference_layers.push(reference_layer.clone());
        }
        let Some(has_coverage) = segment_has_plane_coverage(segment, &reference_layer, zones)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "ADJACENT_PLANE_RETURN_PATH_VALID reference net {} has no usable zone polygon on layer {reference_layer}.",
                    rule.reference_net
                ),
            );
            return None;
        };
        if !has_coverage {
            measurement.unreferenced_length_mm += length;
            measurement.unreferenced_segment_count += 1;
            if measurement.first_unreferenced_segment_index.is_none() {
                measurement.first_unreferenced_segment_index = Some(index);
                measurement.first_unreferenced_route_layer = Some(segment.layer.clone());
                measurement.first_unreferenced_reference_layer = Some(reference_layer);
            }
        }
    }
    Some(measurement)
}

fn measure_slot_crossings(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &SlotCrossingRule,
    route: &NetRoute,
    zones: &[CopperZone],
) -> Option<SlotCrossingMeasurement> {
    let mut measurement = SlotCrossingMeasurement {
        total_route_length_mm: 0.0,
        slot_crossing_count: 0,
        first_slot_segment_index: None,
        first_slot_route_layer: None,
        first_slot_reference_layer: None,
        first_slot_start_mm: None,
        first_slot_end_mm: None,
        reference_layers: Vec::new(),
    };
    for (index, segment) in route.segments.iter().enumerate() {
        let segment_length = segment_length_mm(segment);
        measurement.total_route_length_mm += segment_length;
        let reference_layer = resolve_reference_layer(
            bound,
            scenario,
            findings,
            REFERENCE_PLANE_SLOT_CROSSING_VALID,
            &rule.reference_net,
            rule.reference_layer.as_deref(),
            &segment.layer,
        )?;
        if !measurement.reference_layers.contains(&reference_layer) {
            measurement.reference_layers.push(reference_layer.clone());
        }
        let Some(intervals) = segment_plane_coverage_intervals(segment, &reference_layer, zones)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "REFERENCE_PLANE_SLOT_CROSSING_VALID reference net {} has no usable zone polygon on layer {reference_layer}.",
                    rule.reference_net
                ),
            );
            return None;
        };
        if intervals.is_empty() {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "REFERENCE_PLANE_SLOT_CROSSING_VALID net {} segment {index} has no sampled reference-plane coverage on layer {reference_layer}.",
                    rule.net
                ),
            );
            return None;
        }
        for (start_t, end_t) in internal_uncovered_gaps(&intervals) {
            measurement.slot_crossing_count += 1;
            if measurement.first_slot_segment_index.is_none() {
                measurement.first_slot_segment_index = Some(index);
                measurement.first_slot_route_layer = Some(segment.layer.clone());
                measurement.first_slot_reference_layer = Some(reference_layer.clone());
                measurement.first_slot_start_mm = Some(start_t * segment_length);
                measurement.first_slot_end_mm = Some(end_t * segment_length);
            }
        }
    }
    Some(measurement)
}

fn resolve_reference_layer(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    reference_net: &str,
    reference_layer: Option<&str>,
    route_layer: &str,
) -> Option<String> {
    let layers = &bound.project.board.layout.stackup.layers;
    let Some(route_index) = layers.iter().position(|layer| layer.name == route_layer) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "{check_id} route layer {route_layer} is absent from board.layout.stackup.layers."
            ),
        );
        return None;
    };
    if let Some(reference_layer) = reference_layer {
        let Some(layer) = layers.iter().find(|layer| layer.name == reference_layer) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "{check_id} reference_layer {reference_layer} is absent from board.layout.stackup.layers."
                ),
            );
            return None;
        };
        if layer.kind != StackupLayerKind::Plane
            || layer.reference_net.as_deref() != Some(reference_net)
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "{check_id} reference_layer {reference_layer} must be a plane for reference_net {reference_net}."
                ),
            );
            return None;
        }
        return Some(reference_layer.to_string());
    }
    let mut candidates = Vec::new();
    if let Some(layer) = nearest_conductive_layer(layers, route_index, -1)
        && layer.kind == StackupLayerKind::Plane
        && layer.reference_net.as_deref() == Some(reference_net)
    {
        candidates.push(layer.name.clone());
    }
    if let Some(layer) = nearest_conductive_layer(layers, route_index, 1)
        && layer.kind == StackupLayerKind::Plane
        && layer.reference_net.as_deref() == Some(reference_net)
    {
        candidates.push(layer.name.clone());
    }
    match candidates.len() {
        1 => candidates.into_iter().next(),
        0 => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "{check_id} route layer {route_layer} has no adjacent explicit plane for reference_net {reference_net}."
                ),
            );
            None
        }
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "{check_id} route layer {route_layer} has multiple adjacent planes for reference_net {reference_net}; set parameters.routes[].reference_layer."
                ),
            );
            None
        }
    }
}

fn nearest_conductive_layer(
    layers: &[crate::board_ir::StackupLayer],
    route_index: usize,
    direction: isize,
) -> Option<&crate::board_ir::StackupLayer> {
    let mut index = route_index as isize + direction;
    while index >= 0 && (index as usize) < layers.len() {
        let layer = &layers[index as usize];
        if layer.kind != StackupLayerKind::Dielectric {
            return Some(layer);
        }
        index += direction;
    }
    None
}

fn segment_has_plane_coverage(
    segment: &RouteSegment,
    reference_layer: &str,
    zones: &[CopperZone],
) -> Option<bool> {
    let polygons = zones
        .iter()
        .filter(|zone| zone.layer == reference_layer)
        .flat_map(zone_polygons)
        .filter(|polygon| usable_polygon(polygon))
        .collect::<Vec<_>>();
    if polygons.is_empty() {
        return None;
    }
    let samples = [
        (segment.start.x_mm, segment.start.y_mm),
        (
            (segment.start.x_mm + segment.end.x_mm) / 2.0,
            (segment.start.y_mm + segment.end.y_mm) / 2.0,
        ),
        (segment.end.x_mm, segment.end.y_mm),
    ];
    Some(samples.iter().all(|sample| {
        polygons
            .iter()
            .any(|polygon| point_in_polygon(sample.0, sample.1, polygon))
    }))
}

fn segment_plane_coverage_intervals(
    segment: &RouteSegment,
    reference_layer: &str,
    zones: &[CopperZone],
) -> Option<Vec<(f64, f64)>> {
    let polygons = zones
        .iter()
        .filter(|zone| zone.layer == reference_layer)
        .flat_map(zone_polygons)
        .filter(|polygon| usable_polygon(polygon))
        .collect::<Vec<_>>();
    if polygons.is_empty() {
        return None;
    }
    let mut intervals = Vec::new();
    for polygon in polygons {
        intervals.extend(segment_polygon_coverage_intervals(segment, polygon));
    }
    Some(merge_intervals(intervals))
}

fn segment_polygon_coverage_intervals(
    segment: &RouteSegment,
    polygon: &[crate::board_ir::LayoutPoint],
) -> Vec<(f64, f64)> {
    let mut stops = vec![0.0, 1.0];
    for edge_index in 0..polygon.len() {
        let next_index = (edge_index + 1) % polygon.len();
        let first = &polygon[edge_index];
        let second = &polygon[next_index];
        stops.extend(segment_edge_intersections_t(segment, first, second));
    }
    stops.sort_by(f64::total_cmp);
    stops.dedup_by(|left, right| (*left - *right).abs() <= 1.0e-9);
    let mut intervals = Vec::new();
    for pair in stops.windows(2) {
        let start = pair[0].clamp(0.0, 1.0);
        let end = pair[1].clamp(0.0, 1.0);
        if end - start <= 1.0e-9 {
            continue;
        }
        let midpoint = (start + end) / 2.0;
        let (x, y) = point_at_t(segment, midpoint);
        if point_in_polygon(x, y, polygon) {
            intervals.push((start, end));
        }
    }
    intervals
}

fn segment_edge_intersections_t(
    segment: &RouteSegment,
    first: &crate::board_ir::LayoutPoint,
    second: &crate::board_ir::LayoutPoint,
) -> Vec<f64> {
    let p_x = segment.start.x_mm;
    let p_y = segment.start.y_mm;
    let r_x = segment.end.x_mm - segment.start.x_mm;
    let r_y = segment.end.y_mm - segment.start.y_mm;
    let q_x = first.x_mm;
    let q_y = first.y_mm;
    let s_x = second.x_mm - first.x_mm;
    let s_y = second.y_mm - first.y_mm;
    let denominator = cross(r_x, r_y, s_x, s_y);
    let qmp_x = q_x - p_x;
    let qmp_y = q_y - p_y;
    if denominator.abs() <= 1.0e-12 {
        if cross(qmp_x, qmp_y, r_x, r_y).abs() > 1.0e-9 {
            return Vec::new();
        }
        let route_len_sq = r_x * r_x + r_y * r_y;
        if route_len_sq <= f64::EPSILON {
            return Vec::new();
        }
        let first_t = ((q_x - p_x) * r_x + (q_y - p_y) * r_y) / route_len_sq;
        let second_t = ((second.x_mm - p_x) * r_x + (second.y_mm - p_y) * r_y) / route_len_sq;
        return [first_t, second_t]
            .into_iter()
            .filter(|t| *t >= -1.0e-9 && *t <= 1.0 + 1.0e-9)
            .map(|t| t.clamp(0.0, 1.0))
            .collect();
    }
    let t = cross(qmp_x, qmp_y, s_x, s_y) / denominator;
    let u = cross(qmp_x, qmp_y, r_x, r_y) / denominator;
    if (-1.0e-9..=1.0 + 1.0e-9).contains(&t) && (-1.0e-9..=1.0 + 1.0e-9).contains(&u) {
        vec![t.clamp(0.0, 1.0)]
    } else {
        Vec::new()
    }
}

fn merge_intervals(mut intervals: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    intervals.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (start, end) in intervals {
        if end - start <= 1.0e-9 {
            continue;
        }
        match merged.last_mut() {
            Some(last) if start <= last.1 + 1.0e-9 => {
                last.1 = last.1.max(end);
            }
            _ => merged.push((start, end)),
        }
    }
    merged
}

fn internal_uncovered_gaps(intervals: &[(f64, f64)]) -> Vec<(f64, f64)> {
    intervals
        .windows(2)
        .filter_map(|pair| {
            let start = pair[0].1;
            let end = pair[1].0;
            (end - start > 1.0e-9).then_some((start, end))
        })
        .collect()
}

fn point_at_t(segment: &RouteSegment, t: f64) -> (f64, f64) {
    (
        segment.start.x_mm + (segment.end.x_mm - segment.start.x_mm) * t,
        segment.start.y_mm + (segment.end.y_mm - segment.start.y_mm) * t,
    )
}

fn cross(first_x: f64, first_y: f64, second_x: f64, second_y: f64) -> f64 {
    first_x * second_y - first_y * second_x
}

fn zone_polygons(
    zone: &CopperZone,
) -> Box<dyn Iterator<Item = &Vec<crate::board_ir::LayoutPoint>> + '_> {
    if zone.filled_polygons.is_empty() {
        Box::new(std::iter::once(&zone.polygon))
    } else {
        Box::new(zone.filled_polygons.iter())
    }
}

fn usable_polygon(polygon: &[crate::board_ir::LayoutPoint]) -> bool {
    polygon.len() >= 3
        && polygon
            .iter()
            .all(|point| point.x_mm.is_finite() && point.y_mm.is_finite())
}

fn point_in_polygon(x: f64, y: f64, polygon: &[crate::board_ir::LayoutPoint]) -> bool {
    if polygon.iter().enumerate().any(|(index, point)| {
        let next = &polygon[(index + 1) % polygon.len()];
        point_on_segment(x, y, point, next)
    }) {
        return true;
    }
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let current_point = &polygon[current];
        let previous_point = &polygon[previous];
        let intersects = ((current_point.y_mm > y) != (previous_point.y_mm > y))
            && (x
                < (previous_point.x_mm - current_point.x_mm) * (y - current_point.y_mm)
                    / (previous_point.y_mm - current_point.y_mm)
                    + current_point.x_mm);
        if intersects {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn point_on_segment(
    x: f64,
    y: f64,
    first: &crate::board_ir::LayoutPoint,
    second: &crate::board_ir::LayoutPoint,
) -> bool {
    let dx = second.x_mm - first.x_mm;
    let dy = second.y_mm - first.y_mm;
    let cross_product = cross(x - first.x_mm, y - first.y_mm, dx, dy).abs();
    if cross_product > 1.0e-9 {
        return false;
    }
    let dot = (x - first.x_mm) * dx + (y - first.y_mm) * dy;
    let len_sq = dx * dx + dy * dy;
    dot >= -1.0e-9 && dot <= len_sq + 1.0e-9
}

fn return_path_finding(
    scenario: &Scenario,
    rule: &RouteRule,
    measurement: RouteMeasurement,
) -> Finding {
    let mut finding = Finding::critical(
        ADJACENT_PLANE_RETURN_PATH_VALID,
        &scenario.name,
        format!(
            "Route net {} has {:.3} mm without sampled adjacent {} plane coverage, above the {:.3} mm declared limit.",
            rule.net,
            measurement.unreferenced_length_mm,
            rule.reference_net,
            rule.max_unreferenced_length_mm
        ),
    );
    finding.measured.insert("net".to_string(), json!(rule.net));
    finding
        .measured
        .insert("reference_net".to_string(), json!(rule.reference_net));
    finding.measured.insert(
        "total_route_length_mm".to_string(),
        json!(measurement.total_length_mm),
    );
    finding.measured.insert(
        "unreferenced_route_length_mm".to_string(),
        json!(measurement.unreferenced_length_mm),
    );
    finding.measured.insert(
        "unreferenced_segment_count".to_string(),
        json!(measurement.unreferenced_segment_count),
    );
    finding.measured.insert(
        "reference_layers".to_string(),
        json!(measurement.reference_layers),
    );
    if let Some(index) = measurement.first_unreferenced_segment_index {
        finding.measured.insert(
            "first_unreferenced_route_segment_index".to_string(),
            json!(index),
        );
    }
    if let Some(layer) = measurement.first_unreferenced_route_layer {
        finding
            .measured
            .insert("first_unreferenced_route_layer".to_string(), json!(layer));
    }
    if let Some(layer) = measurement.first_unreferenced_reference_layer {
        finding.measured.insert(
            "first_unreferenced_reference_layer".to_string(),
            json!(layer),
        );
    }
    finding.limit.insert(
        "max_unreferenced_length_mm".to_string(),
        json!(rule.max_unreferenced_length_mm),
    );
    finding.suggested_fixes = vec![
        "Route the net over an explicit adjacent reference plane zone or restore the plane under the affected route segment.".to_string(),
        "Add reviewed stackup and plane-zone evidence before using this rule for return-path sign-off.".to_string(),
        "Use SI/EMI analysis for final return-current behavior; this rule only screens imported route and plane-coverage evidence.".to_string(),
    ];
    finding
}

fn slot_crossing_finding(
    scenario: &Scenario,
    rule: &SlotCrossingRule,
    measurement: SlotCrossingMeasurement,
) -> Finding {
    let mut finding = Finding::critical(
        REFERENCE_PLANE_SLOT_CROSSING_VALID,
        &scenario.name,
        format!(
            "Route net {} crosses {} internal gaps in adjacent {} plane coverage, above the {} declared limit.",
            rule.net, measurement.slot_crossing_count, rule.reference_net, rule.max_slot_crossings
        ),
    );
    finding.measured.insert("net".to_string(), json!(rule.net));
    finding
        .measured
        .insert("reference_net".to_string(), json!(rule.reference_net));
    finding.measured.insert(
        "total_route_length_mm".to_string(),
        json!(measurement.total_route_length_mm),
    );
    finding.measured.insert(
        "slot_crossing_count".to_string(),
        json!(measurement.slot_crossing_count),
    );
    finding.measured.insert(
        "reference_layers".to_string(),
        json!(measurement.reference_layers),
    );
    if let Some(index) = measurement.first_slot_segment_index {
        finding
            .measured
            .insert("first_slot_route_segment_index".to_string(), json!(index));
    }
    if let Some(layer) = measurement.first_slot_route_layer {
        finding
            .measured
            .insert("first_slot_route_layer".to_string(), json!(layer));
    }
    if let Some(layer) = measurement.first_slot_reference_layer {
        finding
            .measured
            .insert("first_slot_reference_layer".to_string(), json!(layer));
    }
    if let Some(distance) = measurement.first_slot_start_mm {
        finding
            .measured
            .insert("first_slot_start_mm".to_string(), json!(distance));
    }
    if let Some(distance) = measurement.first_slot_end_mm {
        finding
            .measured
            .insert("first_slot_end_mm".to_string(), json!(distance));
    }
    finding.limit.insert(
        "max_slot_crossings".to_string(),
        json!(rule.max_slot_crossings),
    );
    finding.suggested_fixes = vec![
        "Reroute the signal so it stays over one continuous explicit reference-plane zone.".to_string(),
        "Restore or stitch the reference plane across the slot only after reviewing the board stackup and return-current path.".to_string(),
        "Use SI/EMI analysis for final sign-off; this check only screens imported route and reference-plane zone geometry.".to_string(),
    ];
    finding
}

fn required_string(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
    index: usize,
    key: &str,
) -> Option<String> {
    required_string_for_check(
        scenario,
        findings,
        mapping,
        ADJACENT_PLANE_RETURN_PATH_VALID,
        index,
        key,
    )
}

fn required_string_for_check(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
    check_id: &str,
    index: usize,
    key: &str,
) -> Option<String> {
    let value = mapping
        .get(serde_yaml_ng::Value::String(key.to_string()))
        .and_then(serde_yaml_ng::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if value.is_none() {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.routes[{index}].{key} must be a non-empty string."),
        );
    }
    value
}

fn optional_string(mapping: &serde_yaml_ng::Mapping, key: &str) -> Option<String> {
    mapping
        .get(serde_yaml_ng::Value::String(key.to_string()))
        .and_then(serde_yaml_ng::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn required_non_negative_number(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
    index: usize,
    key: &str,
) -> Option<f64> {
    let value = mapping
        .get(serde_yaml_ng::Value::String(key.to_string()))
        .and_then(serde_yaml_ng::Value::as_f64)
        .filter(|value| value.is_finite());
    match value {
        Some(value) if value >= 0.0 => Some(value),
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "ADJACENT_PLANE_RETURN_PATH_VALID parameters.routes[{index}].{key} must be a finite non-negative number."
                ),
            );
            None
        }
    }
}

fn required_non_negative_integer(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
    index: usize,
    key: &str,
) -> Option<usize> {
    let value = mapping
        .get(serde_yaml_ng::Value::String(key.to_string()))
        .and_then(serde_yaml_ng::Value::as_u64);
    let Some(value) = value else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "REFERENCE_PLANE_SLOT_CROSSING_VALID parameters.routes[{index}].{key} must be a non-negative integer."
            ),
        );
        return None;
    };
    usize::try_from(value).ok().or_else(|| {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "REFERENCE_PLANE_SLOT_CROSSING_VALID parameters.routes[{index}].{key} is too large for this runtime."
            ),
        );
        None
    })
}

fn require_declared_net(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    index: usize,
    net: &str,
) -> Option<()> {
    require_declared_net_for_check(
        bound,
        scenario,
        findings,
        ADJACENT_PLANE_RETURN_PATH_VALID,
        index,
        net,
    )
}

fn require_declared_net_for_check(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    index: usize,
    net: &str,
) -> Option<()> {
    if bound.project.board.nets.contains_key(net) {
        return Some(());
    }
    validation_input_missing(
        findings,
        scenario,
        format!("{check_id} parameters.routes[{index}] references undeclared net {net}."),
    );
    None
}

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
}
