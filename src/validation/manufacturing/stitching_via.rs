use crate::board_ir::{NetRoute, RouteVia, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;

use super::super::RETURN_PATH_STITCHING_VIA_VALID;
use super::super::common::validation_input_missing;

pub(in crate::validation) fn validate_return_path_stitching_via(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(rules) = stitching_rules(bound, scenario, findings) else {
        return;
    };
    for rule in rules {
        validate_rule(bound, scenario, findings, rule);
    }
}

#[derive(Debug)]
struct StitchingRule {
    net: String,
    reference_net: String,
    max_stitch_via_distance_mm: f64,
}

#[derive(Debug)]
struct StitchingViolation<'a> {
    signal_via_index: usize,
    signal_via: &'a RouteVia,
    nearest: Option<ReferenceViaCandidate<'a>>,
}

#[derive(Debug, Clone, Copy)]
struct ReferenceViaCandidate<'a> {
    via_index: usize,
    via: &'a RouteVia,
    distance_mm: f64,
}

fn stitching_rules(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<Vec<StitchingRule>> {
    let Some(value) = scenario.parameters.get("routes") else {
        validation_input_missing(
            findings,
            scenario,
            "RETURN_PATH_STITCHING_VIA_VALID requires parameters.routes.",
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            "RETURN_PATH_STITCHING_VIA_VALID parameters.routes must be a list.",
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "RETURN_PATH_STITCHING_VIA_VALID parameters.routes must not be empty.",
        );
        return None;
    }
    let mut rules = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(mapping) = item.as_mapping() else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RETURN_PATH_STITCHING_VIA_VALID parameters.routes[{index}] must be an object."
                ),
            );
            return None;
        };
        let net = required_string(scenario, findings, mapping, index, "net")?;
        let reference_net = required_string(scenario, findings, mapping, index, "reference_net")?;
        if net == reference_net {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RETURN_PATH_STITCHING_VIA_VALID parameters.routes[{index}] must name distinct net and reference_net values."
                ),
            );
            return None;
        }
        require_declared_net(bound, scenario, findings, index, &net)?;
        require_declared_net(bound, scenario, findings, index, &reference_net)?;
        rules.push(StitchingRule {
            net,
            reference_net,
            max_stitch_via_distance_mm: required_non_negative_number(
                scenario,
                findings,
                mapping,
                index,
                "max_stitch_via_distance_mm",
            )?,
        });
    }
    Some(rules)
}

fn validate_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: StitchingRule,
) {
    let Some(stackup_layers) = explicit_stackup_layers(bound, scenario, findings) else {
        return;
    };
    let Some(route) = route_for_net(bound, scenario, findings, &rule.net) else {
        return;
    };
    let Some(reference_route) = route_for_net(bound, scenario, findings, &rule.reference_net)
    else {
        return;
    };
    if validate_route_vias(scenario, findings, &rule.net, route, &stackup_layers).is_none()
        || validate_route_vias(
            scenario,
            findings,
            &rule.reference_net,
            reference_route,
            &stackup_layers,
        )
        .is_none()
    {
        return;
    }
    if route.vias.is_empty() {
        return;
    }
    if reference_route.vias.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "RETURN_PATH_STITCHING_VIA_VALID reference net {} has no board.layout.routes vias evidence.",
                rule.reference_net
            ),
        );
        return;
    }

    for (signal_via_index, signal_via) in route.vias.iter().enumerate() {
        let nearest = nearest_matching_reference_via(signal_via, reference_route);
        if nearest
            .as_ref()
            .is_none_or(|candidate| candidate.distance_mm > rule.max_stitch_via_distance_mm)
        {
            findings.push(stitching_finding(
                scenario,
                &rule,
                StitchingViolation {
                    signal_via_index,
                    signal_via,
                    nearest,
                },
                route.vias.len(),
                reference_route.vias.len(),
            ));
        }
    }
}

fn explicit_stackup_layers(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<BTreeSet<String>> {
    let layers = &bound.project.board.layout.stackup.layers;
    if layers.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "RETURN_PATH_STITCHING_VIA_VALID requires board.layout.stackup.layers evidence.",
        );
        return None;
    }
    Some(layers.iter().map(|layer| layer.name.clone()).collect())
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
            format!("RETURN_PATH_STITCHING_VIA_VALID net {net} has no board.layout.routes entry."),
        );
        return None;
    };
    Some(route)
}

fn validate_route_vias(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    net: &str,
    route: &NetRoute,
    stackup_layers: &BTreeSet<String>,
) -> Option<()> {
    for (via_index, via) in route.vias.iter().enumerate() {
        if !via.at.x_mm.is_finite()
            || !via.at.y_mm.is_finite()
            || !via.size_mm.is_finite()
            || via.size_mm <= 0.0
            || !via.drill_mm.is_finite()
            || via.drill_mm <= 0.0
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RETURN_PATH_STITCHING_VIA_VALID net {net} via {via_index} must have finite coordinates and positive size/drill evidence."
                ),
            );
            return None;
        }
        if via.layers.len() < 2 {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RETURN_PATH_STITCHING_VIA_VALID net {net} via {via_index} must declare at least two explicit layers."
                ),
            );
            return None;
        }
        for layer in &via.layers {
            if !stackup_layers.contains(layer) {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "RETURN_PATH_STITCHING_VIA_VALID net {net} via {via_index} references layer {layer} outside board.layout.stackup.layers."
                    ),
                );
                return None;
            }
        }
    }
    Some(())
}

fn nearest_matching_reference_via<'a>(
    signal_via: &RouteVia,
    reference_route: &'a NetRoute,
) -> Option<ReferenceViaCandidate<'a>> {
    reference_route
        .vias
        .iter()
        .enumerate()
        .filter(|(_, reference_via)| via_layers_match(signal_via, reference_via))
        .map(|(via_index, via)| ReferenceViaCandidate {
            via_index,
            via,
            distance_mm: via_distance_mm(signal_via, via),
        })
        .min_by(|left, right| left.distance_mm.total_cmp(&right.distance_mm))
}

fn via_layers_match(signal_via: &RouteVia, reference_via: &RouteVia) -> bool {
    signal_via.layers.iter().all(|layer| {
        reference_via
            .layers
            .iter()
            .any(|candidate| candidate == layer)
    })
}

fn via_distance_mm(first: &RouteVia, second: &RouteVia) -> f64 {
    (first.at.x_mm - second.at.x_mm).hypot(first.at.y_mm - second.at.y_mm)
}

fn stitching_finding(
    scenario: &Scenario,
    rule: &StitchingRule,
    violation: StitchingViolation<'_>,
    signal_via_count: usize,
    reference_via_count: usize,
) -> Finding {
    let mut finding = Finding::critical(
        RETURN_PATH_STITCHING_VIA_VALID,
        scenario.name.clone(),
        format!(
            "Route {} via {} has no {} stitching via within {:.3} mm with a matching layer span.",
            rule.net,
            violation.signal_via_index,
            rule.reference_net,
            rule.max_stitch_via_distance_mm
        ),
    );
    finding.net = Some(rule.net.clone());
    finding.measured.insert("net".to_string(), json!(rule.net));
    finding
        .measured
        .insert("reference_net".to_string(), json!(rule.reference_net));
    finding
        .measured
        .insert("signal_via_count".to_string(), json!(signal_via_count));
    finding.measured.insert(
        "reference_via_count".to_string(),
        json!(reference_via_count),
    );
    finding.measured.insert(
        "signal_via_index".to_string(),
        json!(violation.signal_via_index),
    );
    finding.measured.insert(
        "signal_via_x_mm".to_string(),
        json!(violation.signal_via.at.x_mm),
    );
    finding.measured.insert(
        "signal_via_y_mm".to_string(),
        json!(violation.signal_via.at.y_mm),
    );
    finding.measured.insert(
        "signal_via_layers".to_string(),
        json!(violation.signal_via.layers),
    );
    if let Some(nearest) = violation.nearest {
        finding.measured.insert(
            "nearest_reference_via_index".to_string(),
            json!(nearest.via_index),
        );
        finding.measured.insert(
            "nearest_reference_via_distance_mm".to_string(),
            json!(nearest.distance_mm),
        );
        finding.measured.insert(
            "nearest_reference_via_x_mm".to_string(),
            json!(nearest.via.at.x_mm),
        );
        finding.measured.insert(
            "nearest_reference_via_y_mm".to_string(),
            json!(nearest.via.at.y_mm),
        );
        finding.measured.insert(
            "nearest_reference_via_layers".to_string(),
            json!(nearest.via.layers),
        );
    }
    finding.limit.insert(
        "max_stitch_via_distance_mm".to_string(),
        json!(rule.max_stitch_via_distance_mm),
    );
    finding.limit.insert(
        "matching_layer_policy".to_string(),
        json!("signal_via_layers_subset"),
    );
    finding.suggested_fixes = vec![
        "Add a nearby stitching via on the declared reference net with the same explicit layer span, then re-import the PCB evidence.".to_string(),
        "If the transition intentionally uses another return structure, encode that reviewed reference-net evidence explicitly before using this screen for sign-off.".to_string(),
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
                "RETURN_PATH_STITCHING_VIA_VALID parameters.routes[{index}].{key} must be a non-empty string."
            ),
        );
    }
    value
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
        .filter(|value| value.is_finite() && *value >= 0.0);
    if value.is_none() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "RETURN_PATH_STITCHING_VIA_VALID parameters.routes[{index}].{key} must be a finite non-negative number."
            ),
        );
    }
    value
}

fn require_declared_net(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
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
            "RETURN_PATH_STITCHING_VIA_VALID parameters.routes[{index}] references undeclared net {net}."
        ),
    );
    None
}
