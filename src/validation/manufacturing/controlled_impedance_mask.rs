use crate::board_ir::{
    LayoutCopper, LayoutCopperFeature, LayoutCopperRegion, LayoutCopperSegment, LayoutPoint,
    NetRoute, RouteSegment, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID;
use super::super::common::validation_input_missing;

pub(in crate::validation) fn validate_controlled_impedance_solder_mask_loading(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(rules) = mask_rules(bound, scenario, findings) else {
        return;
    };
    for rule in rules {
        validate_mask_rule(bound, scenario, findings, rule);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedMaskState {
    Covered,
    Opened,
}

#[derive(Debug)]
struct MaskRule {
    net: String,
    route_layer: String,
    solder_mask_layer: String,
    expected_state: ExpectedMaskState,
    source: String,
}

fn mask_rules(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<Vec<MaskRule>> {
    let Some(value) = scenario.parameters.get("routes") else {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID requires parameters.routes.",
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID parameters.routes must be a list.",
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID parameters.routes must not be empty.",
        );
        return None;
    }

    let mut rules = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Some(mapping) = item.as_mapping() else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID parameters.routes[{index}] must be an object."
                ),
            );
            return None;
        };
        let net = required_string(scenario, findings, mapping, index, "net")?;
        if !bound.project.board.nets.contains_key(&net) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID parameters.routes[{index}] references undeclared net {net}."
                ),
            );
            return None;
        }
        rules.push(MaskRule {
            net,
            route_layer: required_string(scenario, findings, mapping, index, "route_layer")?,
            solder_mask_layer: required_string(
                scenario,
                findings,
                mapping,
                index,
                "solder_mask_layer",
            )?,
            expected_state: required_mask_state(
                scenario,
                findings,
                mapping,
                index,
                "expected_solder_mask_state",
            )?,
            source: required_string(scenario, findings, mapping, index, "source")?,
        });
    }
    Some(rules)
}

fn validate_mask_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: MaskRule,
) {
    let Some(route) = route_for_net(bound, scenario, findings, &rule.net) else {
        return;
    };
    let segments = route
        .segments
        .iter()
        .enumerate()
        .filter(|(_, segment)| segment.layer == rule.route_layer)
        .collect::<Vec<_>>();
    if segments.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID net {} has no finite route segment evidence on route_layer {}.",
                rule.net, rule.route_layer
            ),
        );
        return;
    }
    let mask = &bound.project.board.layout.solder_mask;
    if !solder_mask_layer_has_opening_evidence(mask, &rule.solder_mask_layer) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID requires imported dark solder-mask opening evidence on solder_mask_layer {}.",
                rule.solder_mask_layer
            ),
        );
        return;
    }
    for (segment_index, segment) in segments {
        for (sample_index, sample) in segment_samples(segment).into_iter().enumerate() {
            let opened = solder_mask_opening_contains_point(mask, &rule.solder_mask_layer, &sample);
            match (rule.expected_state, opened) {
                (ExpectedMaskState::Covered, true) | (ExpectedMaskState::Opened, false) => {
                    findings.push(mask_loading_finding(
                        scenario,
                        &rule,
                        segment_index,
                        sample_index,
                        &sample,
                        opened,
                    ));
                }
                _ => {}
            }
        }
    }
}

fn required_string(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
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
            format!(
                "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID parameters.routes[{index}].{key} must be a non-empty string."
            ),
        );
    }
    value
}

fn required_mask_state(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
    index: usize,
    key: &str,
) -> Option<ExpectedMaskState> {
    let raw = required_string(scenario, findings, mapping, index, key)?;
    match raw.as_str() {
        "covered" => Some(ExpectedMaskState::Covered),
        "opened" => Some(ExpectedMaskState::Opened),
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID parameters.routes[{index}].{key} must be covered or opened."
                ),
            );
            None
        }
    }
}

fn route_for_net<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    net: &str,
) -> Option<&'a NetRoute> {
    let Some(route) = bound.project.board.layout.routes.get(net) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID net {net} has no board.layout.routes entry."
            ),
        );
        return None;
    };
    if route.segments.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID net {net} route must include at least one segment."
            ),
        );
        return None;
    }
    for segment in &route.segments {
        if !usable_route_segment(segment) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID net {net} route segments must have finite endpoints, positive width_mm, non-empty layer, and positive length."
                ),
            );
            return None;
        }
    }
    Some(route)
}

fn usable_route_segment(segment: &RouteSegment) -> bool {
    segment.start.x_mm.is_finite()
        && segment.start.y_mm.is_finite()
        && segment.end.x_mm.is_finite()
        && segment.end.y_mm.is_finite()
        && segment.width_mm.is_finite()
        && segment.width_mm > 0.0
        && !segment.layer.trim().is_empty()
        && segment_length_mm(segment) > f64::EPSILON
}

fn segment_samples(segment: &RouteSegment) -> [LayoutPoint; 3] {
    [
        point_at_t(segment, 0.0),
        point_at_t(segment, 0.5),
        point_at_t(segment, 1.0),
    ]
}

fn point_at_t(segment: &RouteSegment, t: f64) -> LayoutPoint {
    LayoutPoint {
        x_mm: segment.start.x_mm + (segment.end.x_mm - segment.start.x_mm) * t,
        y_mm: segment.start.y_mm + (segment.end.y_mm - segment.start.y_mm) * t,
    }
}

fn solder_mask_layer_has_opening_evidence(mask: &LayoutCopper, layer: &str) -> bool {
    mask.features
        .iter()
        .any(|feature| is_dark_on_layer(&feature.layer, &feature.polarity, layer))
        || mask
            .segments
            .iter()
            .any(|segment| is_dark_on_layer(&segment.layer, &segment.polarity, layer))
        || mask
            .regions
            .iter()
            .any(|region| is_dark_on_layer(&region.layer, &region.polarity, layer))
}

fn solder_mask_opening_contains_point(
    mask: &LayoutCopper,
    layer: &str,
    point: &LayoutPoint,
) -> bool {
    mask.features
        .iter()
        .any(|feature| feature_contains_point(feature, layer, point))
        || mask
            .segments
            .iter()
            .any(|segment| segment_contains_point(segment, layer, point))
        || mask
            .regions
            .iter()
            .any(|region| region_contains_point(region, layer, point))
}

fn feature_contains_point(feature: &LayoutCopperFeature, layer: &str, point: &LayoutPoint) -> bool {
    if !is_dark_on_layer(&feature.layer, &feature.polarity, layer)
        || !feature.at.x_mm.is_finite()
        || !feature.at.y_mm.is_finite()
        || !feature.size.x_mm.is_finite()
        || !feature.size.y_mm.is_finite()
        || feature.size.x_mm <= 0.0
        || feature.size.y_mm <= 0.0
    {
        return false;
    }
    let dx = (point.x_mm - feature.at.x_mm).abs();
    let dy = (point.y_mm - feature.at.y_mm).abs();
    match feature.shape.as_str() {
        "circle" => dx.hypot(dy) <= feature.size.x_mm.min(feature.size.y_mm) / 2.0 + f64::EPSILON,
        "oval" => point_in_axis_aligned_oval(dx, dy, feature.size.x_mm, feature.size.y_mm),
        _ => {
            dx <= feature.size.x_mm / 2.0 + f64::EPSILON
                && dy <= feature.size.y_mm / 2.0 + f64::EPSILON
        }
    }
}

fn point_in_axis_aligned_oval(dx: f64, dy: f64, width_mm: f64, height_mm: f64) -> bool {
    let radius = width_mm.min(height_mm) / 2.0;
    let half_major = width_mm.max(height_mm) / 2.0;
    if radius <= f64::EPSILON {
        return false;
    }
    if width_mm >= height_mm {
        let straight_half = half_major - radius;
        if dx <= straight_half + f64::EPSILON && dy <= radius + f64::EPSILON {
            return true;
        }
        (dx - straight_half).max(0.0).hypot(dy) <= radius + f64::EPSILON
    } else {
        let straight_half = half_major - radius;
        if dy <= straight_half + f64::EPSILON && dx <= radius + f64::EPSILON {
            return true;
        }
        dx.hypot((dy - straight_half).max(0.0)) <= radius + f64::EPSILON
    }
}

fn segment_contains_point(segment: &LayoutCopperSegment, layer: &str, point: &LayoutPoint) -> bool {
    if !is_dark_on_layer(&segment.layer, &segment.polarity, layer)
        || !segment.start.x_mm.is_finite()
        || !segment.start.y_mm.is_finite()
        || !segment.end.x_mm.is_finite()
        || !segment.end.y_mm.is_finite()
        || !segment.width_mm.is_finite()
        || segment.width_mm <= 0.0
    {
        return false;
    }
    point_to_segment_distance_mm(point, &segment.start, &segment.end)
        <= segment.width_mm / 2.0 + f64::EPSILON
}

fn region_contains_point(region: &LayoutCopperRegion, layer: &str, point: &LayoutPoint) -> bool {
    is_dark_on_layer(&region.layer, &region.polarity, layer)
        && region.points.len() >= 3
        && region
            .points
            .iter()
            .all(|point| point.x_mm.is_finite() && point.y_mm.is_finite())
        && point_in_polygon(point, &region.points)
}

fn is_dark_on_layer(object_layer: &str, polarity: &str, expected_layer: &str) -> bool {
    object_layer == expected_layer && polarity == "dark"
}

fn point_to_segment_distance_mm(
    point: &LayoutPoint,
    start: &LayoutPoint,
    end: &LayoutPoint,
) -> f64 {
    let dx = end.x_mm - start.x_mm;
    let dy = end.y_mm - start.y_mm;
    let len2 = dx * dx + dy * dy;
    if len2 <= f64::EPSILON {
        return (point.x_mm - start.x_mm).hypot(point.y_mm - start.y_mm);
    }
    let t =
        (((point.x_mm - start.x_mm) * dx + (point.y_mm - start.y_mm) * dy) / len2).clamp(0.0, 1.0);
    let projection = LayoutPoint {
        x_mm: start.x_mm + t * dx,
        y_mm: start.y_mm + t * dy,
    };
    (point.x_mm - projection.x_mm).hypot(point.y_mm - projection.y_mm)
}

fn point_in_polygon(point: &LayoutPoint, polygon: &[LayoutPoint]) -> bool {
    let mut inside = false;
    for (start, end) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        let crosses = ((start.y_mm > point.y_mm) != (end.y_mm > point.y_mm))
            && (point.x_mm
                < (end.x_mm - start.x_mm) * (point.y_mm - start.y_mm) / (end.y_mm - start.y_mm)
                    + start.x_mm);
        if crosses {
            inside = !inside;
        }
    }
    inside
}

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
}

fn mask_loading_finding(
    scenario: &Scenario,
    rule: &MaskRule,
    segment_index: usize,
    sample_index: usize,
    sample: &LayoutPoint,
    opened: bool,
) -> Finding {
    let measured_state = if opened { "opened" } else { "covered" };
    let expected_state = match rule.expected_state {
        ExpectedMaskState::Covered => "covered",
        ExpectedMaskState::Opened => "opened",
    };
    let mut finding = Finding::critical(
        CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID,
        &scenario.name,
        format!(
            "Controlled-impedance route {} has solder-mask state {} at sampled route point, but reviewed evidence requires {}.",
            rule.net, measured_state, expected_state
        ),
    );
    finding.measured.insert("net".to_string(), json!(rule.net));
    finding
        .measured
        .insert("route_layer".to_string(), json!(rule.route_layer));
    finding.measured.insert(
        "solder_mask_layer".to_string(),
        json!(rule.solder_mask_layer),
    );
    finding
        .measured
        .insert("target_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("route_segment_index".to_string(), json!(segment_index));
    finding
        .measured
        .insert("sample_index".to_string(), json!(sample_index));
    finding
        .measured
        .insert("sample_x_mm".to_string(), json!(sample.x_mm));
    finding
        .measured
        .insert("sample_y_mm".to_string(), json!(sample.y_mm));
    finding.measured.insert(
        "measured_solder_mask_state".to_string(),
        json!(measured_state),
    );
    finding.limit.insert(
        "expected_solder_mask_state".to_string(),
        json!(expected_state),
    );
    finding.suggested_fixes = vec![
        "Review whether the controlled-impedance target assumed solder-mask-covered or solder-mask-opened routing.".to_string(),
        "Adjust solder-mask openings or the reviewed solder_mask_state metadata so imported mask evidence matches the stackup target.".to_string(),
        "Use a field solver or fabricator coupon for final impedance sign-off; this check only compares imported mask geometry against reviewed mask-loading policy.".to_string(),
    ];
    finding
}
