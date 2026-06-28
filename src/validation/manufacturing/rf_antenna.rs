use crate::board_ir::{
    LayoutCopperFeature, LayoutCopperRegion, LayoutCopperSegment, LayoutPoint,
    RfAntennaKeepoutRule, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::RF_ANTENNA_KEEPOUT_VALID;
use super::super::common::validation_input_missing;
use super::geometry::{
    copper_feature_to_polygon_clearance_mm, copper_segment_to_polygon_clearance_mm,
    point_inside_polygon, polygon_to_polygon_clearance_mm, validate_copper_feature_geometry,
    validate_copper_region_geometry, validate_copper_segment_geometry,
};

pub(in crate::validation) fn validate_rf_antenna_keepout(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = keepout_names(scenario, findings) else {
        return;
    };
    for name in names {
        let Some(rule) = keepout_rule(bound, scenario, findings, &name) else {
            return;
        };
        validate_keepout_rule(bound, scenario, findings, rule);
    }
}

fn keepout_names(scenario: &Scenario, findings: &mut Vec<Finding>) -> Option<Vec<String>> {
    let Some(value) = scenario.parameters.get("keepouts") else {
        validation_input_missing(
            findings,
            scenario,
            "RF_ANTENNA_KEEPOUT_VALID requires parameters.keepouts.",
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            "RF_ANTENNA_KEEPOUT_VALID parameters.keepouts must be a list.",
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "RF_ANTENNA_KEEPOUT_VALID parameters.keepouts must not be empty.",
        );
        return None;
    }
    let mut names = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(mapping) = item.as_mapping() else {
            validation_input_missing(
                findings,
                scenario,
                format!("RF_ANTENNA_KEEPOUT_VALID parameters.keepouts[{index}] must be an object."),
            );
            return None;
        };
        let name = mapping
            .get(serde_yaml_ng::Value::String("name".to_string()))
            .and_then(serde_yaml_ng::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let Some(name) = name else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_KEEPOUT_VALID parameters.keepouts[{index}].name must be a non-empty string."
                ),
            );
            return None;
        };
        names.push(name);
    }
    Some(names)
}

fn keepout_rule<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    name: &str,
) -> Option<&'a RfAntennaKeepoutRule> {
    let matches = bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .keepouts
        .iter()
        .filter(|rule| rule.name == name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [rule] => Some(*rule),
        [] => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_KEEPOUT_VALID keepout {name} is absent from board.layout.constraints.rf_antenna.keepouts."
                ),
            );
            None
        }
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_KEEPOUT_VALID keepout {name} is ambiguous in board.layout.constraints.rf_antenna.keepouts."
                ),
            );
            None
        }
    }
}

fn validate_keepout_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &RfAntennaKeepoutRule,
) {
    if let Err(message) = validate_keepout_metadata(bound, rule) {
        validation_input_missing(findings, scenario, message);
        return;
    }

    let mut comparable_count = 0usize;
    for (index, feature) in bound
        .project
        .board
        .layout
        .copper
        .features
        .iter()
        .enumerate()
    {
        if !copper_feature_is_comparable(rule, feature) {
            continue;
        }
        comparable_count += 1;
        if let Err(message) = validate_copper_feature_geometry(feature, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(clearance_mm) = copper_feature_to_polygon_clearance_mm(feature, &rule.polygon)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_KEEPOUT_VALID keepout {} cannot compare copper feature {index}.",
                    rule.name
                ),
            );
            continue;
        };
        if clearance_mm + f64::EPSILON < rule.min_copper_clearance_mm {
            findings.push(feature_keepout_finding(
                scenario,
                rule,
                feature,
                index,
                clearance_mm,
            ));
        }
    }
    for (index, segment) in bound
        .project
        .board
        .layout
        .copper
        .segments
        .iter()
        .enumerate()
    {
        if !copper_segment_is_comparable(rule, segment) {
            continue;
        }
        comparable_count += 1;
        if let Err(message) = validate_copper_segment_geometry(segment, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let clearance_mm = copper_segment_to_polygon_clearance_mm(segment, &rule.polygon);
        if clearance_mm + f64::EPSILON < rule.min_copper_clearance_mm {
            findings.push(segment_keepout_finding(
                scenario,
                rule,
                segment,
                index,
                clearance_mm,
            ));
        }
    }
    for (index, region) in bound.project.board.layout.copper.regions.iter().enumerate() {
        if !copper_region_is_comparable(rule, region) {
            continue;
        }
        comparable_count += 1;
        if let Err(message) = validate_copper_region_geometry(region, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let clearance_mm = polygon_to_polygon_clearance_mm(&region.points, &rule.polygon);
        if clearance_mm + f64::EPSILON < rule.min_copper_clearance_mm {
            findings.push(region_keepout_finding(
                scenario,
                rule,
                region,
                index,
                clearance_mm,
            ));
        }
    }

    if comparable_count == 0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "RF_ANTENNA_KEEPOUT_VALID keepout {} has no comparable board.layout.copper evidence on layer {}.",
                rule.name, rule.layer
            ),
        );
    }
}

fn validate_keepout_metadata(
    bound: &BoundBoard<'_>,
    rule: &RfAntennaKeepoutRule,
) -> Result<(), String> {
    if rule.name.trim().is_empty() {
        return Err("RF_ANTENNA_KEEPOUT_VALID keepout name must be non-empty.".to_string());
    }
    if rule.layer.trim().is_empty() {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {} layer must be non-empty.",
            rule.name
        ));
    }
    if !rule.min_copper_clearance_mm.is_finite() || rule.min_copper_clearance_mm < 0.0 {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {} min_copper_clearance_mm must be finite and non-negative.",
            rule.name
        ));
    }
    if rule.source.trim().is_empty() {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {} source must be non-empty.",
            rule.name
        ));
    }
    if let Some(net) = rule.antenna_net.as_deref()
        && !bound.project.board.nets.contains_key(net)
    {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {} antenna_net {net} is absent from board.nets.",
            rule.name
        ));
    }
    validate_polygon(&rule.name, &rule.polygon)
}

fn validate_polygon(name: &str, polygon: &[LayoutPoint]) -> Result<(), String> {
    if polygon.len() < 3 {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {name} polygon must contain at least three points."
        ));
    }
    if polygon
        .iter()
        .any(|point| !point.x_mm.is_finite() || !point.y_mm.is_finite())
    {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {name} polygon points must be finite."
        ));
    }
    let area = polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
        .map(|(left, right)| left.x_mm * right.y_mm - right.x_mm * left.y_mm)
        .sum::<f64>()
        .abs()
        / 2.0;
    if area <= f64::EPSILON {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {name} polygon must be non-degenerate."
        ));
    }
    Ok(())
}

fn copper_feature_is_comparable(
    rule: &RfAntennaKeepoutRule,
    feature: &LayoutCopperFeature,
) -> bool {
    feature.layer == rule.layer && !same_antenna_net(rule, feature.net.as_deref())
}

fn copper_segment_is_comparable(
    rule: &RfAntennaKeepoutRule,
    segment: &LayoutCopperSegment,
) -> bool {
    segment.layer == rule.layer && !same_antenna_net(rule, segment.net.as_deref())
}

fn copper_region_is_comparable(rule: &RfAntennaKeepoutRule, region: &LayoutCopperRegion) -> bool {
    region.layer == rule.layer && !same_antenna_net(rule, region.net.as_deref())
}

fn same_antenna_net(rule: &RfAntennaKeepoutRule, net: Option<&str>) -> bool {
    matches!((rule.antenna_net.as_deref(), net), (Some(antenna), Some(candidate)) if antenna == candidate)
}

fn feature_keepout_finding(
    scenario: &Scenario,
    rule: &RfAntennaKeepoutRule,
    feature: &LayoutCopperFeature,
    index: usize,
    clearance_mm: f64,
) -> Finding {
    let mut finding = base_keepout_finding(scenario, rule, "feature", index, clearance_mm);
    finding
        .measured
        .insert("copper_feature_shape".to_string(), json!(feature.shape));
    finding
        .measured
        .insert("copper_feature_at".to_string(), point_json(&feature.at));
    finding.measured.insert(
        "copper_feature_size".to_string(),
        json!({
            "x_mm": feature.size.x_mm,
            "y_mm": feature.size.y_mm
        }),
    );
    insert_optional_copper_owner(
        &mut finding,
        feature.net.as_deref(),
        feature.component.as_deref(),
    );
    finding
}

fn segment_keepout_finding(
    scenario: &Scenario,
    rule: &RfAntennaKeepoutRule,
    segment: &LayoutCopperSegment,
    index: usize,
    clearance_mm: f64,
) -> Finding {
    let mut finding = base_keepout_finding(scenario, rule, "segment", index, clearance_mm);
    finding.measured.insert(
        "copper_segment_start".to_string(),
        point_json(&segment.start),
    );
    finding
        .measured
        .insert("copper_segment_end".to_string(), point_json(&segment.end));
    finding.measured.insert(
        "copper_segment_width_mm".to_string(),
        json!(segment.width_mm),
    );
    insert_optional_copper_owner(
        &mut finding,
        segment.net.as_deref(),
        segment.component.as_deref(),
    );
    finding
}

fn region_keepout_finding(
    scenario: &Scenario,
    rule: &RfAntennaKeepoutRule,
    region: &LayoutCopperRegion,
    index: usize,
    clearance_mm: f64,
) -> Finding {
    let mut finding = base_keepout_finding(scenario, rule, "region", index, clearance_mm);
    finding.measured.insert(
        "copper_region_point_count".to_string(),
        json!(region.points.len()),
    );
    let intrudes = region
        .points
        .iter()
        .any(|point| point_inside_polygon(point, &rule.polygon))
        || rule
            .polygon
            .iter()
            .any(|point| point_inside_polygon(point, &region.points));
    finding.measured.insert(
        "copper_region_intrudes_keepout".to_string(),
        json!(intrudes),
    );
    insert_optional_copper_owner(
        &mut finding,
        region.net.as_deref(),
        region.component.as_deref(),
    );
    finding
}

fn base_keepout_finding(
    scenario: &Scenario,
    rule: &RfAntennaKeepoutRule,
    copper_kind: &str,
    copper_index: usize,
    clearance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        RF_ANTENNA_KEEPOUT_VALID,
        &scenario.name,
        format!(
            "RF antenna keepout {} has {copper_kind} copper clearance {:.3} mm below the reviewed {:.3} mm limit.",
            rule.name, clearance_mm, rule.min_copper_clearance_mm
        ),
    );
    finding
        .measured
        .insert("keepout_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("keepout_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("keepout_layer".to_string(), json!(rule.layer));
    finding.measured.insert(
        "keepout_polygon_point_count".to_string(),
        json!(rule.polygon.len()),
    );
    finding
        .measured
        .insert("copper_kind".to_string(), json!(copper_kind));
    finding
        .measured
        .insert("copper_index".to_string(), json!(copper_index));
    finding
        .measured
        .insert("clearance_mm".to_string(), json!(clearance_mm));
    if let Some(net) = &rule.antenna_net {
        finding
            .measured
            .insert("antenna_net".to_string(), json!(net));
    }
    finding.limit.insert(
        "min_copper_clearance_mm".to_string(),
        json!(rule.min_copper_clearance_mm),
    );
    finding.suggested_fixes = vec![
        "Move non-antenna copper outside the reviewed antenna keepout polygon.".to_string(),
        "Update the keepout only from antenna datasheet, module layout-guide, or RF review evidence.".to_string(),
        "Use RF simulation or measurement for final antenna performance; this check only screens imported copper geometry against explicit keepout metadata.".to_string(),
    ];
    finding
}

fn point_json(point: &LayoutPoint) -> serde_json::Value {
    json!({
        "x_mm": point.x_mm,
        "y_mm": point.y_mm
    })
}

fn insert_optional_copper_owner(finding: &mut Finding, net: Option<&str>, component: Option<&str>) {
    if let Some(net) = net {
        finding
            .measured
            .insert("copper_net".to_string(), json!(net));
    }
    if let Some(component) = component {
        finding
            .measured
            .insert("copper_component".to_string(), json!(component));
    }
}
