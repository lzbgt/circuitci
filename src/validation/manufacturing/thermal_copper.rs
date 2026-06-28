use crate::board_ir::{
    LayoutCopperFeature, LayoutCopperRegion, LayoutCopperSegment, Scenario, ThermalCopperRule,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::THERMAL_COPPER_AREA_VALID;
use super::super::common::validation_input_missing;
use super::geometry::{
    validate_copper_feature_geometry, validate_copper_region_geometry,
    validate_copper_segment_geometry,
};

pub(in crate::validation) fn validate_thermal_copper_area(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = thermal_rule_names(scenario, findings) else {
        return;
    };
    for name in names {
        let Some(rule) = thermal_rule(bound, scenario, findings, &name) else {
            return;
        };
        validate_thermal_rule(bound, scenario, findings, rule);
    }
}

fn thermal_rule_names(scenario: &Scenario, findings: &mut Vec<Finding>) -> Option<Vec<String>> {
    let Some(value) = scenario.parameters.get("thermal_copper") else {
        validation_input_missing(
            findings,
            scenario,
            "THERMAL_COPPER_AREA_VALID requires parameters.thermal_copper.",
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            "THERMAL_COPPER_AREA_VALID parameters.thermal_copper must be a list.",
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "THERMAL_COPPER_AREA_VALID parameters.thermal_copper must not be empty.",
        );
        return None;
    }
    let mut names = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(mapping) = item.as_mapping() else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_COPPER_AREA_VALID parameters.thermal_copper[{index}] must be an object."
                ),
            );
            return None;
        };
        let Some(name) = mapping
            .get(serde_yaml_ng::Value::String("name".to_string()))
            .and_then(serde_yaml_ng::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_COPPER_AREA_VALID parameters.thermal_copper[{index}].name must be a non-empty string."
                ),
            );
            return None;
        };
        names.push(name);
    }
    Some(names)
}

fn thermal_rule<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    name: &str,
) -> Option<&'a ThermalCopperRule> {
    let matches = bound
        .project
        .board
        .manufacturing
        .thermal_copper
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
                    "THERMAL_COPPER_AREA_VALID thermal copper rule {name} is absent from board.manufacturing.thermal_copper."
                ),
            );
            None
        }
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_COPPER_AREA_VALID thermal copper rule {name} is ambiguous in board.manufacturing.thermal_copper."
                ),
            );
            None
        }
    }
}

fn validate_thermal_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &ThermalCopperRule,
) {
    if let Err(message) = validate_rule_metadata(bound, rule) {
        validation_input_missing(findings, scenario, message);
        return;
    }

    let copper = &bound.project.board.layout.copper;
    let mut evidence = ThermalAreaEvidence::default();
    for (index, feature) in copper.features.iter().enumerate() {
        if !thermal_feature_matches(rule, feature) {
            continue;
        }
        if let Err(message) = validate_copper_feature_geometry(feature, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(area_mm2) = feature_area_mm2(feature) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_COPPER_AREA_VALID thermal copper rule {} cannot measure unsupported copper feature shape {} at board.layout.copper.features[{index}].",
                    rule.name, feature.shape
                ),
            );
            continue;
        };
        evidence.feature_area_mm2 += area_mm2;
        evidence.feature_count += 1;
    }
    for (index, segment) in copper.segments.iter().enumerate() {
        if !thermal_segment_matches(rule, segment) {
            continue;
        }
        if let Err(message) = validate_copper_segment_geometry(segment, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        evidence.segment_area_mm2 += segment_area_mm2(segment);
        evidence.segment_count += 1;
    }
    for (index, region) in copper.regions.iter().enumerate() {
        if !thermal_region_matches(rule, region) {
            continue;
        }
        if let Err(message) = validate_copper_region_geometry(region, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        evidence.region_area_mm2 += region_area_mm2(region);
        evidence.region_count += 1;
    }

    if evidence.object_count() == 0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_COPPER_AREA_VALID thermal copper rule {} has no comparable board.layout.copper evidence for component {}, nets {:?}, and layers {:?}.",
                rule.name, rule.component, rule.nets, rule.layers
            ),
        );
        return;
    }

    let total_area_mm2 = evidence.total_area_mm2();
    if total_area_mm2 + f64::EPSILON < rule.min_copper_area_mm2 {
        findings.push(thermal_area_finding(
            scenario,
            rule,
            &evidence,
            total_area_mm2,
        ));
    }
}

fn validate_rule_metadata(bound: &BoundBoard<'_>, rule: &ThermalCopperRule) -> Result<(), String> {
    if rule.name.trim().is_empty() {
        return Err(
            "THERMAL_COPPER_AREA_VALID thermal copper rule name must be non-empty.".to_string(),
        );
    }
    if !bound.project.board.components.contains_key(&rule.component) {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} component {} is absent from board.components.",
            rule.name, rule.component
        ));
    }
    if rule.source.trim().is_empty() {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} source must be non-empty.",
            rule.name
        ));
    }
    if !rule.power_loss_w.is_finite() || rule.power_loss_w <= 0.0 {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} power_loss_w must be finite and positive.",
            rule.name
        ));
    }
    if !rule.min_copper_area_mm2.is_finite() || rule.min_copper_area_mm2 <= 0.0 {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} min_copper_area_mm2 must be finite and positive.",
            rule.name
        ));
    }
    for net in &rule.nets {
        if !bound.project.board.nets.contains_key(net) {
            return Err(format!(
                "THERMAL_COPPER_AREA_VALID thermal copper rule {} net {net} is absent from board.nets.",
                rule.name
            ));
        }
    }
    Ok(())
}

fn thermal_feature_matches(rule: &ThermalCopperRule, feature: &LayoutCopperFeature) -> bool {
    copper_matches(
        rule,
        feature.component.as_deref(),
        feature.net.as_deref(),
        &feature.layer,
    )
}

fn thermal_segment_matches(rule: &ThermalCopperRule, segment: &LayoutCopperSegment) -> bool {
    copper_matches(
        rule,
        segment.component.as_deref(),
        segment.net.as_deref(),
        &segment.layer,
    )
}

fn thermal_region_matches(rule: &ThermalCopperRule, region: &LayoutCopperRegion) -> bool {
    copper_matches(
        rule,
        region.component.as_deref(),
        region.net.as_deref(),
        &region.layer,
    )
}

fn copper_matches(
    rule: &ThermalCopperRule,
    component: Option<&str>,
    net: Option<&str>,
    layer: &str,
) -> bool {
    if !rule.layers.is_empty() && !rule.layers.iter().any(|candidate| candidate == layer) {
        return false;
    }
    let component_match = component == Some(rule.component.as_str());
    let net_match = net.is_some_and(|candidate| rule.nets.iter().any(|net| net == candidate));
    component_match || (!rule.nets.is_empty() && net_match)
}

fn feature_area_mm2(feature: &LayoutCopperFeature) -> Option<f64> {
    let x = feature.size.x_mm;
    let y = feature.size.y_mm;
    if !x.is_finite() || !y.is_finite() || x <= 0.0 || y <= 0.0 {
        return None;
    }
    match feature.shape.as_str() {
        "rect" | "rectangle" => Some(x * y),
        "circle" => Some(std::f64::consts::PI * (x.min(y) / 2.0).powi(2)),
        "oval" | "roundrect" => Some(oval_area_mm2(x, y)),
        _ => None,
    }
}

fn oval_area_mm2(x_mm: f64, y_mm: f64) -> f64 {
    let major = x_mm.max(y_mm);
    let minor = x_mm.min(y_mm);
    (major - minor) * minor + std::f64::consts::PI * (minor / 2.0).powi(2)
}

fn segment_area_mm2(segment: &LayoutCopperSegment) -> f64 {
    let length_mm =
        (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm);
    length_mm * segment.width_mm
}

fn region_area_mm2(region: &LayoutCopperRegion) -> f64 {
    region
        .points
        .iter()
        .zip(region.points.iter().cycle().skip(1))
        .take(region.points.len())
        .map(|(left, right)| left.x_mm * right.y_mm - right.x_mm * left.y_mm)
        .sum::<f64>()
        .abs()
        / 2.0
}

#[derive(Debug, Default)]
struct ThermalAreaEvidence {
    feature_area_mm2: f64,
    segment_area_mm2: f64,
    region_area_mm2: f64,
    feature_count: usize,
    segment_count: usize,
    region_count: usize,
}

impl ThermalAreaEvidence {
    fn total_area_mm2(&self) -> f64 {
        self.feature_area_mm2 + self.segment_area_mm2 + self.region_area_mm2
    }

    fn object_count(&self) -> usize {
        self.feature_count + self.segment_count + self.region_count
    }
}

fn thermal_area_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    evidence: &ThermalAreaEvidence,
    total_area_mm2: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_COPPER_AREA_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} measured {:.3} mm^2 of explicit copper evidence, below the reviewed {:.3} mm^2 minimum.",
            rule.name, total_area_mm2, rule.min_copper_area_mm2
        ),
    );
    finding.component = Some(rule.component.clone());
    finding
        .measured
        .insert("thermal_copper_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("thermal_copper_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("component".to_string(), json!(rule.component));
    finding
        .measured
        .insert("power_loss_w".to_string(), json!(rule.power_loss_w));
    finding
        .measured
        .insert("nets".to_string(), json!(rule.nets));
    finding
        .measured
        .insert("layers".to_string(), json!(rule.layers));
    finding
        .measured
        .insert("copper_area_mm2".to_string(), json!(total_area_mm2));
    finding.measured.insert(
        "copper_feature_area_mm2".to_string(),
        json!(evidence.feature_area_mm2),
    );
    finding.measured.insert(
        "copper_segment_area_mm2".to_string(),
        json!(evidence.segment_area_mm2),
    );
    finding.measured.insert(
        "copper_region_area_mm2".to_string(),
        json!(evidence.region_area_mm2),
    );
    finding.measured.insert(
        "copper_object_count".to_string(),
        json!(evidence.object_count()),
    );
    finding.limit.insert(
        "min_copper_area_mm2".to_string(),
        json!(rule.min_copper_area_mm2),
    );
    finding.suggested_fixes = vec![
        "Increase explicit copper area tied to the component or reviewed thermal nets/layers, then re-import the layout evidence.".to_string(),
        "If the loss or copper-area requirement changed, update board.manufacturing.thermal_copper from the reviewed thermal note instead of relying on this screen as a thermal solver.".to_string(),
    ];
    finding
}
