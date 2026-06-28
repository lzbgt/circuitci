use crate::board_ir::{NetRoute, RouteSegment, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::CONTROLLED_IMPEDANCE_GEOMETRY_VALID;
use super::super::common::validation_input_missing;

pub(in crate::validation) fn validate_controlled_impedance_geometry(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(rules) = controlled_impedance_rules(bound, scenario, findings) else {
        return;
    };
    for rule in rules.single_ended {
        validate_single_ended_rule(bound, scenario, findings, rule);
    }
    for rule in rules.differential_pairs {
        validate_differential_rule(bound, scenario, findings, rule);
    }
}

#[derive(Debug, Default)]
struct ControlledImpedanceRules {
    single_ended: Vec<SingleEndedRule>,
    differential_pairs: Vec<DifferentialPairRule>,
}

#[derive(Debug)]
struct SingleEndedRule {
    net: String,
    source: String,
    target_impedance_ohm: f64,
    expected_width_mm: f64,
    max_width_error_mm: f64,
}

#[derive(Debug)]
struct DifferentialPairRule {
    first_net: String,
    second_net: String,
    source: String,
    target_differential_impedance_ohm: f64,
    expected_width_mm: f64,
    expected_gap_mm: f64,
    max_width_error_mm: f64,
    max_gap_error_mm: f64,
}

fn controlled_impedance_rules(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<ControlledImpedanceRules> {
    let mut rules = ControlledImpedanceRules::default();
    if let Some(items) = optional_sequence(scenario, findings, "nets")? {
        for (index, item) in items.iter().enumerate() {
            rules.single_ended.push(parse_single_ended_rule(
                bound, scenario, findings, index, item,
            )?);
        }
    }
    if let Some(items) = optional_sequence(scenario, findings, "differential_pairs")? {
        for (index, item) in items.iter().enumerate() {
            rules.differential_pairs.push(parse_differential_pair_rule(
                bound, scenario, findings, index, item,
            )?);
        }
    }
    if rules.single_ended.is_empty() && rules.differential_pairs.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_GEOMETRY_VALID requires parameters.nets or parameters.differential_pairs.",
        );
        return None;
    }
    Some(rules)
}

fn optional_sequence<'a>(
    scenario: &'a Scenario,
    findings: &mut Vec<Finding>,
    key: &str,
) -> Option<Option<&'a Vec<serde_yaml_ng::Value>>>
where
    serde_yaml_ng::Value: 'a,
{
    let Some(value) = scenario.parameters.get(key) else {
        return Some(None);
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            format!("CONTROLLED_IMPEDANCE_GEOMETRY_VALID parameters.{key} must be a list."),
        );
        return None;
    };
    Some(Some(items))
}

fn parse_single_ended_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    index: usize,
    item: &serde_yaml_ng::Value,
) -> Option<SingleEndedRule> {
    let mapping = rule_mapping(scenario, findings, "nets", index, item)?;
    let net = required_string(scenario, findings, mapping, "nets", index, "net")?;
    require_declared_net(bound, scenario, findings, "nets", index, &net)?;
    Some(SingleEndedRule {
        net,
        source: required_string(scenario, findings, mapping, "nets", index, "source")?,
        target_impedance_ohm: required_positive_number(
            scenario,
            findings,
            mapping,
            "nets",
            index,
            "target_impedance_ohm",
        )?,
        expected_width_mm: required_positive_number(
            scenario,
            findings,
            mapping,
            "nets",
            index,
            "expected_width_mm",
        )?,
        max_width_error_mm: required_non_negative_number(
            scenario,
            findings,
            mapping,
            "nets",
            index,
            "max_width_error_mm",
        )?,
    })
}

fn parse_differential_pair_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    index: usize,
    item: &serde_yaml_ng::Value,
) -> Option<DifferentialPairRule> {
    let mapping = rule_mapping(scenario, findings, "differential_pairs", index, item)?;
    let first_net = required_string(
        scenario,
        findings,
        mapping,
        "differential_pairs",
        index,
        "first_net",
    )?;
    let second_net = required_string(
        scenario,
        findings,
        mapping,
        "differential_pairs",
        index,
        "second_net",
    )?;
    if first_net == second_net {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_GEOMETRY_VALID parameters.differential_pairs[{index}] must name two distinct nets."
            ),
        );
        return None;
    }
    require_declared_net(
        bound,
        scenario,
        findings,
        "differential_pairs",
        index,
        &first_net,
    )?;
    require_declared_net(
        bound,
        scenario,
        findings,
        "differential_pairs",
        index,
        &second_net,
    )?;
    Some(DifferentialPairRule {
        first_net,
        second_net,
        source: required_string(
            scenario,
            findings,
            mapping,
            "differential_pairs",
            index,
            "source",
        )?,
        target_differential_impedance_ohm: required_positive_number(
            scenario,
            findings,
            mapping,
            "differential_pairs",
            index,
            "target_differential_impedance_ohm",
        )?,
        expected_width_mm: required_positive_number(
            scenario,
            findings,
            mapping,
            "differential_pairs",
            index,
            "expected_width_mm",
        )?,
        expected_gap_mm: required_non_negative_number(
            scenario,
            findings,
            mapping,
            "differential_pairs",
            index,
            "expected_gap_mm",
        )?,
        max_width_error_mm: required_non_negative_number(
            scenario,
            findings,
            mapping,
            "differential_pairs",
            index,
            "max_width_error_mm",
        )?,
        max_gap_error_mm: required_non_negative_number(
            scenario,
            findings,
            mapping,
            "differential_pairs",
            index,
            "max_gap_error_mm",
        )?,
    })
}

fn rule_mapping<'a>(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    list_key: &str,
    index: usize,
    item: &'a serde_yaml_ng::Value,
) -> Option<&'a serde_yaml_ng::Mapping> {
    let Some(mapping) = item.as_mapping() else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_GEOMETRY_VALID parameters.{list_key}[{index}] must be an object."
            ),
        );
        return None;
    };
    Some(mapping)
}

fn required_string(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
    list_key: &str,
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
                "CONTROLLED_IMPEDANCE_GEOMETRY_VALID parameters.{list_key}[{index}].{key} must be a non-empty string."
            ),
        );
    }
    value
}

fn required_positive_number(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
    list_key: &str,
    index: usize,
    key: &str,
) -> Option<f64> {
    let value = required_number(scenario, findings, mapping, list_key, index, key)?;
    if value <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_GEOMETRY_VALID parameters.{list_key}[{index}].{key} must be positive."
            ),
        );
        return None;
    }
    Some(value)
}

fn required_non_negative_number(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
    list_key: &str,
    index: usize,
    key: &str,
) -> Option<f64> {
    let value = required_number(scenario, findings, mapping, list_key, index, key)?;
    if value < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_GEOMETRY_VALID parameters.{list_key}[{index}].{key} must be non-negative."
            ),
        );
        return None;
    }
    Some(value)
}

fn required_number(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
    list_key: &str,
    index: usize,
    key: &str,
) -> Option<f64> {
    let value = mapping
        .get(serde_yaml_ng::Value::String(key.to_string()))
        .and_then(serde_yaml_ng::Value::as_f64)
        .filter(|value| value.is_finite());
    if value.is_none() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_GEOMETRY_VALID parameters.{list_key}[{index}].{key} must be a finite number."
            ),
        );
    }
    value
}

fn require_declared_net(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    list_key: &str,
    index: usize,
    net: &str,
) -> Option<()> {
    if bound.project.board.nets.contains_key(net) {
        return Some(());
    }
    validation_input_missing(
        findings,
        scenario,
        format!(
            "CONTROLLED_IMPEDANCE_GEOMETRY_VALID parameters.{list_key}[{index}] references undeclared net {net}."
        ),
    );
    None
}

fn validate_single_ended_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: SingleEndedRule,
) {
    let Some(route) = route_for_net(bound, scenario, findings, &rule.net) else {
        return;
    };
    let Some(worst_width) = worst_width_error(route, &rule.net, rule.expected_width_mm) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_GEOMETRY_VALID net {} route has no finite segment width evidence.",
                rule.net
            ),
        );
        return;
    };
    if worst_width.width_error_mm > rule.max_width_error_mm + f64::EPSILON {
        findings.push(single_ended_width_finding(scenario, &rule, worst_width));
    }
}

fn validate_differential_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: DifferentialPairRule,
) {
    let Some(first_route) = route_for_net(bound, scenario, findings, &rule.first_net) else {
        return;
    };
    let Some(second_route) = route_for_net(bound, scenario, findings, &rule.second_net) else {
        return;
    };
    let first_width = worst_width_error(first_route, &rule.first_net, rule.expected_width_mm);
    let second_width = worst_width_error(second_route, &rule.second_net, rule.expected_width_mm);
    let worst_width = [first_width, second_width]
        .into_iter()
        .flatten()
        .max_by(|left, right| left.width_error_mm.total_cmp(&right.width_error_mm));
    let Some(worst_width) = worst_width else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_GEOMETRY_VALID differential pair {}/{} has no finite segment width evidence.",
                rule.first_net, rule.second_net
            ),
        );
        return;
    };
    let Some(worst_gap) = worst_pair_gap_error(
        first_route,
        second_route,
        &rule.first_net,
        &rule.second_net,
        rule.expected_gap_mm,
    ) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_GEOMETRY_VALID differential pair {}/{} has no parallel overlapping same-layer route evidence for gap measurement.",
                rule.first_net, rule.second_net
            ),
        );
        return;
    };
    if worst_width.width_error_mm > rule.max_width_error_mm + f64::EPSILON
        || worst_gap.gap_error_mm > rule.max_gap_error_mm + f64::EPSILON
    {
        findings.push(differential_pair_finding(
            scenario,
            &rule,
            worst_width,
            worst_gap,
        ));
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
                "CONTROLLED_IMPEDANCE_GEOMETRY_VALID net {net} has no board.layout.routes entry."
            ),
        );
        return None;
    };
    if let Err(message) = validate_route_shape(route) {
        validation_input_missing(
            findings,
            scenario,
            format!("CONTROLLED_IMPEDANCE_GEOMETRY_VALID net {net} {message}"),
        );
        return None;
    }
    Some(route)
}

fn validate_route_shape(route: &NetRoute) -> Result<(), &'static str> {
    if route.segments.is_empty() {
        return Err("route must include at least one segment.");
    }
    for segment in &route.segments {
        if !segment.start.x_mm.is_finite()
            || !segment.start.y_mm.is_finite()
            || !segment.end.x_mm.is_finite()
            || !segment.end.y_mm.is_finite()
        {
            return Err("route segment endpoints must be finite.");
        }
        if !segment.width_mm.is_finite() || segment.width_mm <= 0.0 {
            return Err("route segment width_mm must be finite and positive.");
        }
        if segment.layer.trim().is_empty() {
            return Err("route segment layer must be non-empty.");
        }
        if segment_length_mm(segment) <= f64::EPSILON {
            return Err("route segment length must be greater than zero.");
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct WidthEvidence {
    net: String,
    segment_index: usize,
    layer: String,
    measured_width_mm: f64,
    width_error_mm: f64,
    start: (f64, f64),
    end: (f64, f64),
}

fn worst_width_error(route: &NetRoute, net: &str, expected_width_mm: f64) -> Option<WidthEvidence> {
    route
        .segments
        .iter()
        .enumerate()
        .filter(|(_, segment)| segment.width_mm.is_finite())
        .map(|(segment_index, segment)| WidthEvidence {
            net: net.to_string(),
            segment_index,
            layer: segment.layer.clone(),
            measured_width_mm: segment.width_mm,
            width_error_mm: (segment.width_mm - expected_width_mm).abs(),
            start: (segment.start.x_mm, segment.start.y_mm),
            end: (segment.end.x_mm, segment.end.y_mm),
        })
        .max_by(|left, right| left.width_error_mm.total_cmp(&right.width_error_mm))
}

#[derive(Debug, Clone)]
struct GapEvidence {
    first_segment_index: usize,
    second_segment_index: usize,
    layer: String,
    measured_gap_mm: f64,
    gap_error_mm: f64,
}

fn worst_pair_gap_error(
    first_route: &NetRoute,
    second_route: &NetRoute,
    _first_net: &str,
    _second_net: &str,
    expected_gap_mm: f64,
) -> Option<GapEvidence> {
    let mut worst = None;
    for (first_segment_index, first_segment) in first_route.segments.iter().enumerate() {
        for (second_segment_index, second_segment) in second_route.segments.iter().enumerate() {
            if first_segment.layer != second_segment.layer {
                continue;
            }
            let Some(measured_gap_mm) = parallel_overlap_gap_mm(first_segment, second_segment)
            else {
                continue;
            };
            let gap_error_mm = (measured_gap_mm - expected_gap_mm).abs();
            let evidence = GapEvidence {
                first_segment_index,
                second_segment_index,
                layer: first_segment.layer.clone(),
                measured_gap_mm,
                gap_error_mm,
            };
            if worst
                .as_ref()
                .is_none_or(|current: &GapEvidence| gap_error_mm > current.gap_error_mm)
            {
                worst = Some(evidence);
            }
        }
    }
    worst
}

fn parallel_overlap_gap_mm(first: &RouteSegment, second: &RouteSegment) -> Option<f64> {
    let first_dx = first.end.x_mm - first.start.x_mm;
    let first_dy = first.end.y_mm - first.start.y_mm;
    let second_dx = second.end.x_mm - second.start.x_mm;
    let second_dy = second.end.y_mm - second.start.y_mm;
    let first_len = first_dx.hypot(first_dy);
    let second_len = second_dx.hypot(second_dy);
    if first_len <= f64::EPSILON || second_len <= f64::EPSILON {
        return None;
    }
    let first_unit_x = first_dx / first_len;
    let first_unit_y = first_dy / first_len;
    let second_unit_x = second_dx / second_len;
    let second_unit_y = second_dy / second_len;
    let cross = (first_unit_x * second_unit_y - first_unit_y * second_unit_x).abs();
    if cross > 1.0e-6 {
        return None;
    }
    let projection_a = (second.start.x_mm - first.start.x_mm) * first_unit_x
        + (second.start.y_mm - first.start.y_mm) * first_unit_y;
    let projection_b = (second.end.x_mm - first.start.x_mm) * first_unit_x
        + (second.end.y_mm - first.start.y_mm) * first_unit_y;
    let overlap_start = projection_a.min(projection_b).max(0.0);
    let overlap_end = projection_a.max(projection_b).min(first_len);
    if overlap_end - overlap_start <= f64::EPSILON {
        return None;
    }
    let centerline_distance_mm = ((second.start.x_mm - first.start.x_mm) * first_unit_y
        - (second.start.y_mm - first.start.y_mm) * first_unit_x)
        .abs();
    Some(centerline_distance_mm - (first.width_mm + second.width_mm) / 2.0)
}

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
}

fn single_ended_width_finding(
    scenario: &Scenario,
    rule: &SingleEndedRule,
    width: WidthEvidence,
) -> Finding {
    let mut finding = Finding::critical(
        CONTROLLED_IMPEDANCE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "Controlled-impedance net {} has {:.3} mm route width, outside {:.3} mm +/- {:.3} mm declared target for {:.1} ohm evidence.",
            rule.net,
            width.measured_width_mm,
            rule.expected_width_mm,
            rule.max_width_error_mm,
            rule.target_impedance_ohm
        ),
    );
    finding.measured.insert("net".to_string(), json!(rule.net));
    finding
        .measured
        .insert("target_source".to_string(), json!(rule.source));
    finding.measured.insert(
        "target_impedance_ohm".to_string(),
        json!(rule.target_impedance_ohm),
    );
    insert_width_evidence(&mut finding, "route", &width);
    finding.limit.insert(
        "expected_width_mm".to_string(),
        json!(rule.expected_width_mm),
    );
    finding.limit.insert(
        "max_width_error_mm".to_string(),
        json!(rule.max_width_error_mm),
    );
    finding.suggested_fixes = vec![
        "Adjust the routed width to match the reviewed controlled-impedance stackup target.".to_string(),
        "Update the declared expected_width_mm only after reviewing the board stackup or fabricator impedance table.".to_string(),
        "Do not treat this check as an impedance solver; it only verifies imported geometry against explicit target evidence.".to_string(),
    ];
    finding
}

fn differential_pair_finding(
    scenario: &Scenario,
    rule: &DifferentialPairRule,
    width: WidthEvidence,
    gap: GapEvidence,
) -> Finding {
    let mut finding = Finding::critical(
        CONTROLLED_IMPEDANCE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "Controlled-impedance differential pair {}/{} has route width error {:.3} mm or gap error {:.3} mm outside declared geometry tolerances for {:.1} ohm evidence.",
            rule.first_net,
            rule.second_net,
            width.width_error_mm,
            gap.gap_error_mm,
            rule.target_differential_impedance_ohm
        ),
    );
    finding
        .measured
        .insert("first_net".to_string(), json!(rule.first_net));
    finding
        .measured
        .insert("second_net".to_string(), json!(rule.second_net));
    finding
        .measured
        .insert("target_source".to_string(), json!(rule.source));
    finding.measured.insert(
        "target_differential_impedance_ohm".to_string(),
        json!(rule.target_differential_impedance_ohm),
    );
    insert_width_evidence(&mut finding, "worst_width", &width);
    finding
        .measured
        .insert("gap_layer".to_string(), json!(gap.layer));
    finding.measured.insert(
        "first_gap_route_segment_index".to_string(),
        json!(gap.first_segment_index),
    );
    finding.measured.insert(
        "second_gap_route_segment_index".to_string(),
        json!(gap.second_segment_index),
    );
    finding
        .measured
        .insert("measured_gap_mm".to_string(), json!(gap.measured_gap_mm));
    finding
        .measured
        .insert("gap_error_mm".to_string(), json!(gap.gap_error_mm));
    finding.measured.insert(
        "width_violation".to_string(),
        json!(width.width_error_mm > rule.max_width_error_mm + f64::EPSILON),
    );
    finding.measured.insert(
        "gap_violation".to_string(),
        json!(gap.gap_error_mm > rule.max_gap_error_mm + f64::EPSILON),
    );
    finding.limit.insert(
        "expected_width_mm".to_string(),
        json!(rule.expected_width_mm),
    );
    finding
        .limit
        .insert("expected_gap_mm".to_string(), json!(rule.expected_gap_mm));
    finding.limit.insert(
        "max_width_error_mm".to_string(),
        json!(rule.max_width_error_mm),
    );
    finding
        .limit
        .insert("max_gap_error_mm".to_string(), json!(rule.max_gap_error_mm));
    finding.suggested_fixes = vec![
        "Adjust the differential-pair route width and spacing to match the reviewed controlled-impedance stackup target.".to_string(),
        "Update expected_width_mm or expected_gap_mm only from reviewed stackup or fabricator impedance evidence.".to_string(),
        "Use a field solver or fabricator impedance coupon for final sign-off; this check only compares imported geometry to declared targets.".to_string(),
    ];
    finding
}

fn insert_width_evidence(finding: &mut Finding, prefix: &str, width: &WidthEvidence) {
    finding
        .measured
        .insert(format!("{prefix}_net"), json!(width.net));
    finding.measured.insert(
        format!("{prefix}_segment_index"),
        json!(width.segment_index),
    );
    finding
        .measured
        .insert(format!("{prefix}_layer"), json!(width.layer));
    finding.measured.insert(
        format!("{prefix}_measured_width_mm"),
        json!(width.measured_width_mm),
    );
    finding.measured.insert(
        format!("{prefix}_width_error_mm"),
        json!(width.width_error_mm),
    );
    finding.measured.insert(
        format!("{prefix}_segment_start"),
        json!({ "x_mm": width.start.0, "y_mm": width.start.1 }),
    );
    finding.measured.insert(
        format!("{prefix}_segment_end"),
        json!({ "x_mm": width.end.0, "y_mm": width.end.1 }),
    );
}
