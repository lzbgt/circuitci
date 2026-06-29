use crate::board_ir::{
    ControlledImpedanceSolverResult, ControlledImpedanceSolverResultType, NetRoute, RouteSegment,
    Scenario, StackupLayer, StackupLayerKind,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;

use super::super::CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID;
use super::super::common::validation_input_missing;

const IMPEDANCE_MATCH_EPSILON_OHM: f64 = 1.0e-9;

pub(in crate::validation) fn validate_controlled_impedance_solver_result(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = solver_result_names(scenario, findings) else {
        return;
    };
    let results = &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_results;
    if results.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID requires board.manufacturing.controlled_impedance.solver_results evidence.",
        );
        return;
    }
    for name in names {
        let Some(result) = named_solver_result(results, scenario, findings, &name) else {
            return;
        };
        if !solver_result_has_valid_metadata(bound, scenario, findings, result) {
            return;
        }
        let Some(metrics) = solver_result_metrics(bound, scenario, findings, result) else {
            return;
        };
        if metrics.impedance_error_ohm > result.max_impedance_error_ohm + f64::EPSILON
            || metrics.max_width_delta_mm > result.max_route_width_delta_mm + f64::EPSILON
            || metrics.max_gap_delta_mm.is_some_and(|gap| {
                gap > result.max_route_gap_delta_mm.unwrap_or(f64::INFINITY) + f64::EPSILON
            })
        {
            findings.push(solver_result_finding(scenario, result, &metrics));
        }
    }
}

#[derive(Debug)]
struct SolverResultMetrics {
    impedance_error_ohm: f64,
    max_width_delta_mm: f64,
    max_gap_delta_mm: Option<f64>,
}

fn solver_result_names(scenario: &Scenario, findings: &mut Vec<Finding>) -> Option<Vec<String>> {
    let Some(value) = scenario.parameters.get("solver_results") else {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID requires parameters.solver_results.",
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID parameters.solver_results must be a list.",
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID parameters.solver_results must not be empty.",
        );
        return None;
    }
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        let Some(mapping) = item.as_mapping() else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID parameters.solver_results[{index}] must be an object."
                ),
            );
            return None;
        };
        let Some(value) = mapping.get("name") else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID parameters.solver_results[{index}].name is required."
                ),
            );
            return None;
        };
        let Some(name) = value
            .as_str()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID parameters.solver_results[{index}].name must be a non-empty string."
                ),
            );
            return None;
        };
        if !seen.insert(name.to_string()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID parameters.solver_results repeats result name {name}."
                ),
            );
            return None;
        }
        names.push(name.to_string());
    }
    Some(names)
}

fn named_solver_result<'a>(
    results: &'a [ControlledImpedanceSolverResult],
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    name: &str,
) -> Option<&'a ControlledImpedanceSolverResult> {
    let mut matches = results.iter().filter(|result| result.name == name);
    let Some(result) = matches.next() else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {name} is absent from board.manufacturing.controlled_impedance.solver_results."
            ),
        );
        return None;
    };
    if matches.next().is_some() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result name {name} is duplicated in board.manufacturing.controlled_impedance.solver_results."
            ),
        );
        return None;
    }
    Some(result)
}

fn solver_result_has_valid_metadata(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if result.name.trim().is_empty()
        || result.source.trim().is_empty()
        || result.solver.trim().is_empty()
        || result.stackup_revision.trim().is_empty()
        || result.route_layer.trim().is_empty()
        || result.reference_layer.trim().is_empty()
        || result.dielectric_layer.trim().is_empty()
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} must declare non-empty name, source, solver, stackup_revision, route_layer, reference_layer, and dielectric_layer.",
                result.name
            ),
        );
        return false;
    }
    if !positive(result.target_impedance_ohm)
        || !positive(result.solved_impedance_ohm)
        || !non_negative(result.max_impedance_error_ohm)
        || !positive(result.solved_width_mm)
        || !non_negative(result.max_route_width_delta_mm)
        || result.frequency_mhz.is_some_and(|value| !positive(value))
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} must declare finite positive impedances/width and non-negative error limits.",
                result.name
            ),
        );
        return false;
    }
    if !stackup_layers_match(bound, scenario, findings, result) {
        return false;
    }

    match result.result_type {
        ControlledImpedanceSolverResultType::SingleEnded => {
            let Some(net) = non_empty_option(result.net.as_deref()) else {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID single-ended result {} must declare net.",
                        result.name
                    ),
                );
                return false;
            };
            if result.first_net.is_some()
                || result.second_net.is_some()
                || result.solved_gap_mm.is_some()
                || result.max_route_gap_delta_mm.is_some()
            {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID single-ended result {} must not declare differential nets or gap metadata.",
                        result.name
                    ),
                );
                return false;
            }
            if !bound.project.board.nets.contains_key(net) {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} references undeclared net {net}.",
                        result.name
                    ),
                );
                return false;
            }
            solver_matches_single_ended_target(bound, scenario, findings, result, net)
        }
        ControlledImpedanceSolverResultType::Differential => {
            if result.net.is_some() {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID differential result {} must not declare net.",
                        result.name
                    ),
                );
                return false;
            }
            let Some(first_net) = non_empty_option(result.first_net.as_deref()) else {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID differential result {} must declare first_net.",
                        result.name
                    ),
                );
                return false;
            };
            let Some(second_net) = non_empty_option(result.second_net.as_deref()) else {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID differential result {} must declare second_net.",
                        result.name
                    ),
                );
                return false;
            };
            if first_net == second_net {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID differential result {} must declare two distinct nets.",
                        result.name
                    ),
                );
                return false;
            }
            if !positive_option(result.solved_gap_mm)
                || !non_negative_option(result.max_route_gap_delta_mm)
            {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID differential result {} must declare positive solved_gap_mm and non-negative max_route_gap_delta_mm.",
                        result.name
                    ),
                );
                return false;
            }
            for net in [first_net, second_net] {
                if !bound.project.board.nets.contains_key(net) {
                    validation_input_missing(
                        findings,
                        scenario,
                        format!(
                            "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} references undeclared net {net}.",
                            result.name
                        ),
                    );
                    return false;
                }
            }
            solver_matches_differential_target(
                bound, scenario, findings, result, first_net, second_net,
            )
        }
    }
}

fn stackup_layers_match(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let layers = &bound.project.board.layout.stackup.layers;
    if layers.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID requires board.layout.stackup.layers evidence.",
        );
        return false;
    }
    let Some(route_layer) = named_stackup_layer(layers, &result.route_layer) else {
        missing_layer(findings, scenario, result, &result.route_layer);
        return false;
    };
    let Some(reference_layer) = named_stackup_layer(layers, &result.reference_layer) else {
        missing_layer(findings, scenario, result, &result.reference_layer);
        return false;
    };
    let Some(dielectric_layer) = named_stackup_layer(layers, &result.dielectric_layer) else {
        missing_layer(findings, scenario, result, &result.dielectric_layer);
        return false;
    };
    if route_layer.kind != StackupLayerKind::Signal
        || reference_layer.kind != StackupLayerKind::Plane
        || dielectric_layer.kind != StackupLayerKind::Dielectric
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires route_layer kind signal, reference_layer kind plane, and dielectric_layer kind dielectric.",
                result.name
            ),
        );
        return false;
    }
    true
}

fn missing_layer(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    result: &ControlledImpedanceSolverResult,
    layer: &str,
) {
    validation_input_missing(
        findings,
        scenario,
        format!(
            "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} references stackup layer {layer} absent from board.layout.stackup.layers.",
            result.name
        ),
    );
}

fn solver_matches_single_ended_target(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
    net: &str,
) -> bool {
    let targets = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .nets
        .iter()
        .filter(|target| target.net == net)
        .collect::<Vec<_>>();
    if targets.len() != 1 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires exactly one reviewed board target for net {net}; found {}.",
                result.name,
                targets.len()
            ),
        );
        return false;
    }
    let target = targets[0];
    if !positive(target.target_impedance_ohm) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID board target for result {} net {net} must declare finite positive target_impedance_ohm.",
                result.name
            ),
        );
        return false;
    }
    if (result.target_impedance_ohm - target.target_impedance_ohm).abs()
        > IMPEDANCE_MATCH_EPSILON_OHM
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} target {:.3} ohm conflicts with reviewed board target {:.3} ohm for net {net}.",
                result.name, result.target_impedance_ohm, target.target_impedance_ohm
            ),
        );
        return false;
    }
    true
}

fn solver_matches_differential_target(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
    first_net: &str,
    second_net: &str,
) -> bool {
    let targets = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .differential_pairs
        .iter()
        .filter(|target| {
            unordered_pair_matches(&target.first_net, &target.second_net, first_net, second_net)
        })
        .collect::<Vec<_>>();
    if targets.len() != 1 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires exactly one reviewed board target for pair {first_net}/{second_net}; found {}.",
                result.name,
                targets.len()
            ),
        );
        return false;
    }
    let target = targets[0];
    if !positive(target.target_differential_impedance_ohm) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID board target for result {} pair {first_net}/{second_net} must declare finite positive target_differential_impedance_ohm.",
                result.name
            ),
        );
        return false;
    }
    if (result.target_impedance_ohm - target.target_differential_impedance_ohm).abs()
        > IMPEDANCE_MATCH_EPSILON_OHM
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} target {:.3} ohm conflicts with reviewed board target {:.3} ohm for pair {first_net}/{second_net}.",
                result.name, result.target_impedance_ohm, target.target_differential_impedance_ohm
            ),
        );
        return false;
    }
    true
}

fn solver_result_metrics(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> Option<SolverResultMetrics> {
    let impedance_error_ohm = (result.solved_impedance_ohm - result.target_impedance_ohm).abs();
    match result.result_type {
        ControlledImpedanceSolverResultType::SingleEnded => {
            let net = result.net.as_deref()?;
            let route = route_for_net(bound, scenario, findings, result, net)?;
            Some(SolverResultMetrics {
                impedance_error_ohm,
                max_width_delta_mm: worst_width_delta(
                    route,
                    &result.route_layer,
                    result.solved_width_mm,
                )?,
                max_gap_delta_mm: None,
            })
        }
        ControlledImpedanceSolverResultType::Differential => {
            let first_net = result.first_net.as_deref()?;
            let second_net = result.second_net.as_deref()?;
            let first_route = route_for_net(bound, scenario, findings, result, first_net)?;
            let second_route = route_for_net(bound, scenario, findings, result, second_net)?;
            Some(SolverResultMetrics {
                impedance_error_ohm,
                max_width_delta_mm: worst_width_delta(
                    first_route,
                    &result.route_layer,
                    result.solved_width_mm,
                )?
                .max(worst_width_delta(
                    second_route,
                    &result.route_layer,
                    result.solved_width_mm,
                )?),
                max_gap_delta_mm: Some(worst_gap_delta(
                    first_route,
                    second_route,
                    &result.route_layer,
                    result.solved_gap_mm?,
                )?),
            })
        }
    }
}

fn route_for_net<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
    net: &str,
) -> Option<&'a NetRoute> {
    let Some(route) = bound.project.board.layout.routes.get(net) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires imported route evidence for net {net}.",
                result.name
            ),
        );
        return None;
    };
    if route_segments(route, &result.route_layer).next().is_none() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} net {net} has no finite route segment evidence on route_layer {}.",
                result.name, result.route_layer
            ),
        );
        return None;
    }
    Some(route)
}

fn worst_width_delta(route: &NetRoute, layer: &str, solved_width_mm: f64) -> Option<f64> {
    route_segments(route, layer)
        .map(|segment| (segment.width_mm - solved_width_mm).abs())
        .max_by(|a, b| a.total_cmp(b))
}

fn worst_gap_delta(
    first_route: &NetRoute,
    second_route: &NetRoute,
    layer: &str,
    solved_gap_mm: f64,
) -> Option<f64> {
    let mut worst: Option<f64> = None;
    for first in route_segments(first_route, layer) {
        for second in route_segments(second_route, layer) {
            if let Some(gap) = parallel_overlap_gap_mm(first, second) {
                let delta = (gap - solved_gap_mm).abs();
                worst = Some(worst.map_or(delta, |current| current.max(delta)));
            }
        }
    }
    worst
}

fn route_segments<'a>(
    route: &'a NetRoute,
    layer: &'a str,
) -> impl Iterator<Item = &'a RouteSegment> + 'a {
    route
        .segments
        .iter()
        .filter(move |segment| segment.layer == layer && usable_route_segment(segment))
}

fn usable_route_segment(segment: &RouteSegment) -> bool {
    segment.width_mm.is_finite()
        && segment.width_mm > 0.0
        && segment.start.x_mm.is_finite()
        && segment.start.y_mm.is_finite()
        && segment.end.x_mm.is_finite()
        && segment.end.y_mm.is_finite()
        && ((segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm))
            > 0.0
}

fn parallel_overlap_gap_mm(first: &RouteSegment, second: &RouteSegment) -> Option<f64> {
    let first_dx = first.end.x_mm - first.start.x_mm;
    let first_dy = first.end.y_mm - first.start.y_mm;
    let second_dx = second.end.x_mm - second.start.x_mm;
    let second_dy = second.end.y_mm - second.start.y_mm;
    let first_len = first_dx.hypot(first_dy);
    let second_len = second_dx.hypot(second_dy);
    if first_len <= 0.0 || second_len <= 0.0 {
        return None;
    }
    let first_unit = (first_dx / first_len, first_dy / first_len);
    let second_unit = (second_dx / second_len, second_dy / second_len);
    let cross = (first_unit.0 * second_unit.1 - first_unit.1 * second_unit.0).abs();
    if cross > 1.0e-6 {
        return None;
    }
    let origin_delta = (
        second.start.x_mm - first.start.x_mm,
        second.start.y_mm - first.start.y_mm,
    );
    let normal_gap = (origin_delta.0 * first_unit.1 - origin_delta.1 * first_unit.0).abs();
    let first_start: f64 = 0.0;
    let first_end = first_len;
    let second_start = origin_delta.0 * first_unit.0 + origin_delta.1 * first_unit.1;
    let second_end = second_start
        + second_len * (first_unit.0 * second_unit.0 + first_unit.1 * second_unit.1).signum();
    let second_min = second_start.min(second_end);
    let second_max = second_start.max(second_end);
    if first_end.min(second_max) - first_start.max(second_min) <= 0.0 {
        return None;
    }
    Some((normal_gap - (first.width_mm + second.width_mm) / 2.0).max(0.0))
}

fn solver_result_finding(
    scenario: &Scenario,
    result: &ControlledImpedanceSolverResult,
    metrics: &SolverResultMetrics,
) -> Finding {
    let mut finding = Finding::critical(
        CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID,
        scenario.name.clone(),
        format!(
            "Controlled-impedance solver result {} is outside reviewed impedance or route-geometry limits.",
            result.name
        ),
    );
    finding
        .measured
        .insert("result".to_string(), json!(result.name));
    finding
        .measured
        .insert("source".to_string(), json!(result.source));
    finding
        .measured
        .insert("solver".to_string(), json!(result.solver));
    if let Some(version) = &result.solver_version {
        finding
            .measured
            .insert("solver_version".to_string(), json!(version));
    }
    finding.measured.insert(
        "stackup_revision".to_string(),
        json!(result.stackup_revision),
    );
    finding
        .measured
        .insert("route_layer".to_string(), json!(result.route_layer));
    finding.measured.insert(
        "solved_impedance_ohm".to_string(),
        json!(result.solved_impedance_ohm),
    );
    finding.measured.insert(
        "impedance_error_ohm".to_string(),
        json!(metrics.impedance_error_ohm),
    );
    finding.measured.insert(
        "max_route_width_delta_mm".to_string(),
        json!(metrics.max_width_delta_mm),
    );
    if let Some(gap_delta) = metrics.max_gap_delta_mm {
        finding
            .measured
            .insert("max_route_gap_delta_mm".to_string(), json!(gap_delta));
    }
    finding.limit.insert(
        "max_impedance_error_ohm".to_string(),
        json!(result.max_impedance_error_ohm),
    );
    finding.limit.insert(
        "max_route_width_delta_mm".to_string(),
        json!(result.max_route_width_delta_mm),
    );
    if let Some(gap_limit) = result.max_route_gap_delta_mm {
        finding
            .limit
            .insert("max_route_gap_delta_mm".to_string(), json!(gap_limit));
    }
    finding.suggested_fixes.push(
        "Review the solver setup, controlled-impedance geometry, and imported route evidence before treating the impedance target as signed off.".to_string(),
    );
    finding
}

fn named_stackup_layer<'a>(layers: &'a [StackupLayer], name: &str) -> Option<&'a StackupLayer> {
    layers.iter().find(|layer| layer.name == name)
}

fn unordered_pair_matches(first_a: &str, second_a: &str, first_b: &str, second_b: &str) -> bool {
    (first_a == first_b && second_a == second_b) || (first_a == second_b && second_a == first_b)
}

fn non_empty_option(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn non_negative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn positive_option(value: Option<f64>) -> bool {
    value.is_some_and(positive)
}

fn non_negative_option(value: Option<f64>) -> bool {
    value.is_some_and(non_negative)
}
