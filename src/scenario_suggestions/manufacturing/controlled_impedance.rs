use super::manufacturing_suggestion;
use crate::board_ir::{
    ControlledImpedanceCoupon, ControlledImpedanceCouponType,
    ControlledImpedanceDifferentialPairTarget, ControlledImpedanceNetTarget,
    ControlledImpedanceSolverResult, ControlledImpedanceSolverResultType, LayoutCopper, NetKind,
    NetRoute, RouteSegment, StackupLayer, StackupLayerKind,
};
use crate::library::BoundBoard;
use crate::scenario_suggestions::{ScenarioSuggestion, sanitized_name};
use serde_json::json;
use std::collections::BTreeMap;

const CONTROLLED_IMPEDANCE_GEOMETRY_VALID: &str = "CONTROLLED_IMPEDANCE_GEOMETRY_VALID";
const CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID: &str =
    "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID";
const CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID: &str =
    "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID";
const CONTROLLED_IMPEDANCE_COUPON_VALID: &str = "CONTROLLED_IMPEDANCE_COUPON_VALID";
const CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID: &str = "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID";
const CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID: &str =
    "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID";
const CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID: &str = "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID";

pub(super) fn controlled_impedance_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    suggestions.extend(controlled_impedance_geometry_suggestions(
        bound,
        project_name,
    ));
    suggestions.extend(controlled_impedance_stackup_evidence_suggestions(
        bound,
        project_name,
    ));
    suggestions.extend(controlled_impedance_solder_mask_loading_suggestions(
        bound,
        project_name,
    ));
    suggestions.extend(controlled_impedance_coupon_suggestions(bound, project_name));
    suggestions.extend(controlled_impedance_coupon_batch_suggestions(
        bound,
        project_name,
    ));
    suggestions.extend(controlled_impedance_coupon_trace_correlation_suggestions(
        bound,
        project_name,
    ));
    suggestions.extend(controlled_impedance_solver_result_suggestions(
        bound,
        project_name,
    ));
    suggestions
}

fn controlled_impedance_geometry_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    let targets = &bound.project.board.manufacturing.controlled_impedance;
    for target in &targets.nets {
        if !controlled_impedance_net_target_has_evidence(bound, target)
            || controlled_impedance_net_check_declared(bound, &target.net)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!("controlled_impedance_{}", sanitized_name(&target.net)),
            true,
            &format!(
                "Net {} has reviewed board.manufacturing.controlled_impedance target evidence from {} and imported route-width evidence.",
                target.net, target.source
            ),
            &format!(
                "{}_{}_controlled_impedance",
                project_name,
                sanitized_name(&target.net)
            ),
            CONTROLLED_IMPEDANCE_GEOMETRY_VALID,
            Some(BTreeMap::from([(
                "nets".to_string(),
                json!([{
                    "net": target.net,
                    "source": target.source,
                    "target_impedance_ohm": target.target_impedance_ohm,
                    "expected_width_mm": target.expected_width_mm,
                    "max_width_error_mm": target.max_width_error_mm
                }]),
            )])),
            Vec::new(),
        ));
    }
    for target in &targets.differential_pairs {
        if !controlled_impedance_pair_target_has_evidence(bound, target)
            || controlled_impedance_pair_check_declared(
                bound,
                &target.first_net,
                &target.second_net,
            )
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "controlled_impedance_{}_{}",
                sanitized_name(&target.first_net),
                sanitized_name(&target.second_net)
            ),
            true,
            &format!(
                "Differential pair {}/{} has reviewed board.manufacturing.controlled_impedance target evidence from {}, imported route-width evidence, and parallel same-layer gap evidence.",
                target.first_net, target.second_net, target.source
            ),
            &format!(
                "{}_{}_{}_controlled_impedance",
                project_name,
                sanitized_name(&target.first_net),
                sanitized_name(&target.second_net)
            ),
            CONTROLLED_IMPEDANCE_GEOMETRY_VALID,
            Some(BTreeMap::from([(
                "differential_pairs".to_string(),
                json!([{
                    "first_net": target.first_net,
                    "second_net": target.second_net,
                    "source": target.source,
                    "target_differential_impedance_ohm": target.target_differential_impedance_ohm,
                    "expected_width_mm": target.expected_width_mm,
                    "expected_gap_mm": target.expected_gap_mm,
                    "max_width_error_mm": target.max_width_error_mm,
                    "max_gap_error_mm": target.max_gap_error_mm
                }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn controlled_impedance_stackup_evidence_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    let targets = &bound.project.board.manufacturing.controlled_impedance;
    for target in &targets.nets {
        if target.net.trim().is_empty()
            || !bound
                .project
                .board
                .nets
                .get(&target.net)
                .is_some_and(|net| net.kind == NetKind::DigitalOrAnalog)
            || manufacturing_route_check_declared_for_net(
                bound,
                CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID,
                &target.net,
            )
        {
            continue;
        }
        let Some(evidence) = controlled_impedance_stackup_evidence_for_net(bound, &target.net)
        else {
            continue;
        };
        suggestions.push(manufacturing_suggestion(
            &format!(
                "controlled_impedance_stackup_{}",
                sanitized_name(&target.net)
            ),
            true,
            &format!(
                "Net {} has reviewed controlled-impedance target evidence from {}, imported route evidence on {}, and explicit stackup copper/dielectric metadata.",
                target.net, target.source, evidence.route_layer
            ),
            &format!(
                "{}_{}_controlled_impedance_stackup",
                project_name,
                sanitized_name(&target.net)
            ),
            CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID,
            Some(BTreeMap::from([(
                "routes".to_string(),
                json!([stackup_route_parameter(&target.net, &evidence)]),
            )])),
            Vec::new(),
        ));
    }

    for target in &targets.differential_pairs {
        if target.first_net == target.second_net
            || target.first_net.trim().is_empty()
            || target.second_net.trim().is_empty()
            || !bound
                .project
                .board
                .nets
                .get(&target.first_net)
                .is_some_and(|net| net.kind == NetKind::DigitalOrAnalog)
            || !bound
                .project
                .board
                .nets
                .get(&target.second_net)
                .is_some_and(|net| net.kind == NetKind::DigitalOrAnalog)
            || manufacturing_route_check_declared_for_net(
                bound,
                CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID,
                &target.first_net,
            )
            || manufacturing_route_check_declared_for_net(
                bound,
                CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID,
                &target.second_net,
            )
        {
            continue;
        }
        let Some(first_evidence) =
            controlled_impedance_stackup_evidence_for_net(bound, &target.first_net)
        else {
            continue;
        };
        let Some(second_evidence) =
            controlled_impedance_stackup_evidence_for_net(bound, &target.second_net)
        else {
            continue;
        };
        suggestions.push(manufacturing_suggestion(
            &format!(
                "controlled_impedance_stackup_{}_{}",
                sanitized_name(&target.first_net),
                sanitized_name(&target.second_net)
            ),
            true,
            &format!(
                "Differential pair {}/{} has reviewed controlled-impedance target evidence from {}, imported route evidence, and explicit stackup copper/dielectric metadata.",
                target.first_net, target.second_net, target.source
            ),
            &format!(
                "{}_{}_{}_controlled_impedance_stackup",
                project_name,
                sanitized_name(&target.first_net),
                sanitized_name(&target.second_net)
            ),
            CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID,
            Some(BTreeMap::from([(
                "routes".to_string(),
                json!([
                    stackup_route_parameter(&target.first_net, &first_evidence),
                    stackup_route_parameter(&target.second_net, &second_evidence)
                ]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn controlled_impedance_solder_mask_loading_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    let targets = &bound.project.board.manufacturing.controlled_impedance;
    for target in &targets.nets {
        if manufacturing_route_check_declared_for_net(
            bound,
            CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID,
            &target.net,
        ) {
            continue;
        }
        let Some(evidence) = controlled_impedance_net_mask_evidence(bound, target) else {
            continue;
        };
        suggestions.push(manufacturing_suggestion(
            &format!(
                "controlled_impedance_solder_mask_{}",
                sanitized_name(&target.net)
            ),
            true,
            &format!(
                "Net {} has reviewed controlled-impedance solder-mask loading policy from {} and imported route plus solder-mask opening evidence.",
                target.net, evidence.source
            ),
            &format!(
                "{}_{}_controlled_impedance_solder_mask",
                project_name,
                sanitized_name(&target.net)
            ),
            CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID,
            Some(BTreeMap::from([(
                "routes".to_string(),
                json!([mask_route_parameter(&target.net, &evidence)]),
            )])),
            Vec::new(),
        ));
    }

    for target in &targets.differential_pairs {
        if manufacturing_route_check_declared_for_net(
            bound,
            CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID,
            &target.first_net,
        ) || manufacturing_route_check_declared_for_net(
            bound,
            CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID,
            &target.second_net,
        ) {
            continue;
        }
        let Some(first_evidence) =
            controlled_impedance_pair_mask_evidence(bound, target, &target.first_net)
        else {
            continue;
        };
        let Some(second_evidence) =
            controlled_impedance_pair_mask_evidence(bound, target, &target.second_net)
        else {
            continue;
        };
        suggestions.push(manufacturing_suggestion(
            &format!(
                "controlled_impedance_solder_mask_{}_{}",
                sanitized_name(&target.first_net),
                sanitized_name(&target.second_net)
            ),
            true,
            &format!(
                "Differential pair {}/{} has reviewed controlled-impedance solder-mask loading policy from {} and imported route plus solder-mask opening evidence.",
                target.first_net, target.second_net, first_evidence.source
            ),
            &format!(
                "{}_{}_{}_controlled_impedance_solder_mask",
                project_name,
                sanitized_name(&target.first_net),
                sanitized_name(&target.second_net)
            ),
            CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID,
            Some(BTreeMap::from([(
                "routes".to_string(),
                json!([
                    mask_route_parameter(&target.first_net, &first_evidence),
                    mask_route_parameter(&target.second_net, &second_evidence)
                ]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn controlled_impedance_coupon_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for coupon in &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .coupons
    {
        if controlled_impedance_coupon_check_declared(
            bound,
            CONTROLLED_IMPEDANCE_COUPON_VALID,
            &coupon.name,
        ) || !controlled_impedance_coupon_has_evidence(bound, coupon)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "controlled_impedance_coupon_{}",
                sanitized_name(&coupon.name)
            ),
            true,
            &format!(
                "Controlled-impedance coupon {} has reviewed measured impedance evidence from {}, a reviewed tolerance, and a matching board controlled-impedance target.",
                coupon.name, coupon.source
            ),
            &format!(
                "{}_{}_controlled_impedance_coupon",
                project_name,
                sanitized_name(&coupon.name)
            ),
            CONTROLLED_IMPEDANCE_COUPON_VALID,
            Some(BTreeMap::from([(
                "coupons".to_string(),
                json!([{ "name": coupon.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn controlled_impedance_coupon_batch_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for coupon in &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .coupons
    {
        if controlled_impedance_coupon_check_declared(
            bound,
            CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID,
            &coupon.name,
        ) || !controlled_impedance_coupon_batch_has_evidence(bound, coupon)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "controlled_impedance_coupon_batch_{}",
                sanitized_name(&coupon.name)
            ),
            true,
            &format!(
                "Controlled-impedance coupon {} has reviewed batch sample evidence from {}, explicit batch acceptance limits, and a matching board controlled-impedance target.",
                coupon.name, coupon.source
            ),
            &format!(
                "{}_{}_controlled_impedance_coupon_batch",
                project_name,
                sanitized_name(&coupon.name)
            ),
            CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID,
            Some(BTreeMap::from([(
                "coupons".to_string(),
                json!([{ "name": coupon.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn controlled_impedance_coupon_trace_correlation_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for coupon in &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .coupons
    {
        if controlled_impedance_coupon_check_declared(
            bound,
            CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID,
            &coupon.name,
        ) || !controlled_impedance_coupon_trace_correlation_has_evidence(bound, coupon)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "controlled_impedance_coupon_trace_correlation_{}",
                sanitized_name(&coupon.name)
            ),
            true,
            &format!(
                "Controlled-impedance coupon {} has reviewed lot/panel/stackup trace-correlation metadata from {}, imported route evidence, and a matching board controlled-impedance target.",
                coupon.name, coupon.source
            ),
            &format!(
                "{}_{}_controlled_impedance_coupon_trace_correlation",
                project_name,
                sanitized_name(&coupon.name)
            ),
            CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID,
            Some(BTreeMap::from([(
                "coupons".to_string(),
                json!([{ "name": coupon.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn controlled_impedance_solver_result_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for result in &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_results
    {
        if controlled_impedance_solver_result_check_declared(bound, &result.name)
            || !controlled_impedance_solver_result_has_evidence(bound, result)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "controlled_impedance_solver_result_{}",
                sanitized_name(&result.name)
            ),
            true,
            &format!(
                "Controlled-impedance solver result {} has reviewed source evidence from {}, a source-backed solver artifact digest, matching board target metadata, explicit stackup layers, and imported route geometry.",
                result.name, result.source
            ),
            &format!(
                "{}_{}_controlled_impedance_solver_result",
                project_name,
                sanitized_name(&result.name)
            ),
            CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID,
            Some(BTreeMap::from([(
                "solver_results".to_string(),
                json!([{ "name": result.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

struct ControlledImpedanceStackupEvidence {
    route_layer: String,
    reference_layer: String,
    dielectric_layer: String,
}

#[derive(Debug)]
struct ControlledImpedanceMaskEvidence {
    route_layer: String,
    solder_mask_layer: String,
    expected_solder_mask_state: String,
    source: String,
}

fn controlled_impedance_net_target_has_evidence(
    bound: &BoundBoard<'_>,
    target: &ControlledImpedanceNetTarget,
) -> bool {
    !target.net.trim().is_empty()
        && !target.source.trim().is_empty()
        && positive_finite(target.target_impedance_ohm)
        && positive_finite(target.expected_width_mm)
        && non_negative_finite(target.max_width_error_mm)
        && bound
            .project
            .board
            .nets
            .get(&target.net)
            .is_some_and(|net| net.kind == NetKind::DigitalOrAnalog)
        && bound
            .project
            .board
            .layout
            .routes
            .get(&target.net)
            .is_some_and(route_has_valid_segments)
}

fn controlled_impedance_pair_target_has_evidence(
    bound: &BoundBoard<'_>,
    target: &ControlledImpedanceDifferentialPairTarget,
) -> bool {
    if target.first_net == target.second_net
        || target.first_net.trim().is_empty()
        || target.second_net.trim().is_empty()
        || target.source.trim().is_empty()
        || !positive_finite(target.target_differential_impedance_ohm)
        || !positive_finite(target.expected_width_mm)
        || !positive_finite(target.expected_gap_mm)
        || !non_negative_finite(target.max_width_error_mm)
        || !non_negative_finite(target.max_gap_error_mm)
        || !bound
            .project
            .board
            .nets
            .get(&target.first_net)
            .is_some_and(|net| net.kind == NetKind::DigitalOrAnalog)
        || !bound
            .project
            .board
            .nets
            .get(&target.second_net)
            .is_some_and(|net| net.kind == NetKind::DigitalOrAnalog)
    {
        return false;
    }
    let Some(first_route) = bound.project.board.layout.routes.get(&target.first_net) else {
        return false;
    };
    let Some(second_route) = bound.project.board.layout.routes.get(&target.second_net) else {
        return false;
    };
    route_has_valid_segments(first_route)
        && route_has_valid_segments(second_route)
        && routes_have_parallel_gap_evidence(first_route, second_route)
}

fn controlled_impedance_stackup_evidence_for_net(
    bound: &BoundBoard<'_>,
    net_name: &str,
) -> Option<ControlledImpedanceStackupEvidence> {
    let route = bound.project.board.layout.routes.get(net_name)?;
    if route.segments.is_empty() || bound.project.board.layout.stackup.layers.is_empty() {
        return None;
    }
    let mut evidence = None::<ControlledImpedanceStackupEvidence>;
    for segment in &route.segments {
        if !usable_route_segment(segment) {
            return None;
        }
        let route_layer = stackup_layer_by_name(bound, &segment.layer)?;
        if route_layer.kind != StackupLayerKind::Signal
            || !stackup_copper_layer_has_evidence(route_layer)
        {
            return None;
        }
        let reference_layer = adjacent_reference_plane(bound, &segment.layer)?;
        if !stackup_copper_layer_has_evidence(reference_layer) {
            return None;
        }
        let dielectric_layer =
            dielectric_between_layers(bound, &segment.layer, &reference_layer.name)?;
        if !stackup_dielectric_layer_has_evidence(dielectric_layer) {
            return None;
        }
        let segment_evidence = ControlledImpedanceStackupEvidence {
            route_layer: route_layer.name.clone(),
            reference_layer: reference_layer.name.clone(),
            dielectric_layer: dielectric_layer.name.clone(),
        };
        if evidence.as_ref().is_some_and(|current| {
            current.route_layer != segment_evidence.route_layer
                || current.reference_layer != segment_evidence.reference_layer
                || current.dielectric_layer != segment_evidence.dielectric_layer
        }) {
            return None;
        }
        evidence = Some(segment_evidence);
    }
    evidence
}

fn stackup_route_parameter(
    net_name: &str,
    evidence: &ControlledImpedanceStackupEvidence,
) -> serde_json::Value {
    json!({
        "net": net_name,
        "route_layer": evidence.route_layer,
        "reference_layer": evidence.reference_layer,
        "dielectric_layer": evidence.dielectric_layer
    })
}

fn controlled_impedance_net_mask_evidence(
    bound: &BoundBoard<'_>,
    target: &ControlledImpedanceNetTarget,
) -> Option<ControlledImpedanceMaskEvidence> {
    if !controlled_impedance_net_target_has_evidence(bound, target) {
        return None;
    }
    controlled_impedance_mask_evidence_for_net(
        bound,
        &target.net,
        target.solder_mask_state.as_deref(),
        target.solder_mask_layer.as_deref(),
        target
            .solder_mask_source
            .as_deref()
            .or(Some(target.source.as_str())),
    )
}

fn controlled_impedance_pair_mask_evidence(
    bound: &BoundBoard<'_>,
    target: &ControlledImpedanceDifferentialPairTarget,
    net: &str,
) -> Option<ControlledImpedanceMaskEvidence> {
    if !controlled_impedance_pair_target_has_evidence(bound, target) {
        return None;
    }
    controlled_impedance_mask_evidence_for_net(
        bound,
        net,
        target.solder_mask_state.as_deref(),
        target.solder_mask_layer.as_deref(),
        target
            .solder_mask_source
            .as_deref()
            .or(Some(target.source.as_str())),
    )
}

fn controlled_impedance_mask_evidence_for_net(
    bound: &BoundBoard<'_>,
    net_name: &str,
    solder_mask_state: Option<&str>,
    solder_mask_layer: Option<&str>,
    source: Option<&str>,
) -> Option<ControlledImpedanceMaskEvidence> {
    let expected_solder_mask_state = match solder_mask_state? {
        "covered" => "covered",
        "opened" => "opened",
        _ => return None,
    };
    let solder_mask_layer = solder_mask_layer
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let source = source.map(str::trim).filter(|value| !value.is_empty())?;
    if !solder_mask_layer_has_opening_evidence(
        &bound.project.board.layout.solder_mask,
        solder_mask_layer,
    ) {
        return None;
    }
    let route = bound.project.board.layout.routes.get(net_name)?;
    if !route_has_valid_segments(route) {
        return None;
    }
    let mut route_layer = None::<&str>;
    for segment in &route.segments {
        if route_layer.is_some_and(|current| current != segment.layer) {
            return None;
        }
        route_layer = Some(&segment.layer);
    }
    Some(ControlledImpedanceMaskEvidence {
        route_layer: route_layer?.to_string(),
        solder_mask_layer: solder_mask_layer.to_string(),
        expected_solder_mask_state: expected_solder_mask_state.to_string(),
        source: source.to_string(),
    })
}

fn solder_mask_layer_has_opening_evidence(mask: &LayoutCopper, layer: &str) -> bool {
    mask.features
        .iter()
        .any(|feature| feature.layer == layer && feature.polarity == "dark")
        || mask
            .segments
            .iter()
            .any(|segment| segment.layer == layer && segment.polarity == "dark")
        || mask
            .regions
            .iter()
            .any(|region| region.layer == layer && region.polarity == "dark")
}

fn mask_route_parameter(
    net_name: &str,
    evidence: &ControlledImpedanceMaskEvidence,
) -> serde_json::Value {
    json!({
        "net": net_name,
        "route_layer": evidence.route_layer,
        "solder_mask_layer": evidence.solder_mask_layer,
        "expected_solder_mask_state": evidence.expected_solder_mask_state,
        "source": evidence.source
    })
}

fn controlled_impedance_coupon_has_evidence(
    bound: &BoundBoard<'_>,
    coupon: &ControlledImpedanceCoupon,
) -> bool {
    if coupon.name.trim().is_empty()
        || coupon.source.trim().is_empty()
        || !positive_finite(coupon.target_impedance_ohm)
        || !positive_finite(coupon.measured_impedance_ohm)
        || !non_negative_finite(coupon.max_impedance_error_ohm)
    {
        return false;
    }
    match coupon.coupon_type {
        ControlledImpedanceCouponType::SingleEnded => coupon
            .net
            .as_deref()
            .map(str::trim)
            .filter(|net| !net.is_empty())
            .is_some_and(|net| {
                coupon.first_net.is_none()
                    && coupon.second_net.is_none()
                    && bound.project.board.nets.contains_key(net)
                    && matching_single_ended_coupon_target(bound, coupon, net)
            }),
        ControlledImpedanceCouponType::Differential => {
            if coupon.net.is_some() {
                return false;
            }
            let Some(first_net) = coupon
                .first_net
                .as_deref()
                .map(str::trim)
                .filter(|net| !net.is_empty())
            else {
                return false;
            };
            let Some(second_net) = coupon
                .second_net
                .as_deref()
                .map(str::trim)
                .filter(|net| !net.is_empty())
            else {
                return false;
            };
            first_net != second_net
                && bound.project.board.nets.contains_key(first_net)
                && bound.project.board.nets.contains_key(second_net)
                && matching_differential_coupon_target(bound, coupon, first_net, second_net)
        }
    }
}

fn controlled_impedance_coupon_batch_has_evidence(
    bound: &BoundBoard<'_>,
    coupon: &ControlledImpedanceCoupon,
) -> bool {
    controlled_impedance_coupon_has_evidence(bound, coupon)
        && coupon.min_batch_sample_count.is_some_and(|value| value > 0)
        && coupon
            .max_batch_mean_impedance_error_ohm
            .is_some_and(non_negative_finite)
        && coupon
            .max_batch_sample_impedance_error_ohm
            .is_some_and(non_negative_finite)
        && coupon.max_batch_stddev_ohm.is_some_and(non_negative_finite)
        && !coupon.samples.is_empty()
        && coupon.samples.iter().all(|sample| {
            !sample.name.trim().is_empty()
                && !sample.source.trim().is_empty()
                && positive_finite(sample.measured_impedance_ohm)
        })
}

fn controlled_impedance_coupon_trace_correlation_has_evidence(
    bound: &BoundBoard<'_>,
    coupon: &ControlledImpedanceCoupon,
) -> bool {
    controlled_impedance_coupon_has_evidence(bound, coupon)
        && coupon
            .process_lot
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && coupon
            .panel_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && coupon
            .stackup_revision
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && coupon
            .coupon_trace_layer
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && coupon.coupon_trace_width_mm.is_some_and(positive_finite)
        && coupon
            .max_trace_width_delta_mm
            .is_some_and(non_negative_finite)
        && match coupon.coupon_type {
            ControlledImpedanceCouponType::SingleEnded => {
                coupon.coupon_trace_gap_mm.is_none()
                    && coupon.max_trace_gap_delta_mm.is_none()
                    && coupon.net.as_deref().is_some_and(|net| {
                        bound
                            .project
                            .board
                            .layout
                            .routes
                            .get(net)
                            .is_some_and(route_has_valid_segments)
                    })
            }
            ControlledImpedanceCouponType::Differential => {
                coupon.coupon_trace_gap_mm.is_some_and(positive_finite)
                    && coupon
                        .max_trace_gap_delta_mm
                        .is_some_and(non_negative_finite)
                    && coupon.first_net.as_deref().is_some_and(|first_net| {
                        coupon.second_net.as_deref().is_some_and(|second_net| {
                            let Some(first_route) =
                                bound.project.board.layout.routes.get(first_net)
                            else {
                                return false;
                            };
                            let Some(second_route) =
                                bound.project.board.layout.routes.get(second_net)
                            else {
                                return false;
                            };
                            route_has_valid_segments(first_route)
                                && route_has_valid_segments(second_route)
                                && routes_have_parallel_gap_evidence(first_route, second_route)
                        })
                    })
            }
        }
}

fn controlled_impedance_solver_result_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    !result.name.trim().is_empty()
        && !result.source.trim().is_empty()
        && !result.solver.trim().is_empty()
        && result
            .solver_artifact_uri
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_artifact_sha256
            .as_deref()
            .is_some_and(|value| is_sha256_hex(value.trim()))
        && !result.stackup_revision.trim().is_empty()
        && !result.route_layer.trim().is_empty()
        && !result.reference_layer.trim().is_empty()
        && !result.dielectric_layer.trim().is_empty()
        && positive_finite(result.target_impedance_ohm)
        && positive_finite(result.solved_impedance_ohm)
        && non_negative_finite(result.max_impedance_error_ohm)
        && positive_finite(result.solved_width_mm)
        && non_negative_finite(result.max_route_width_delta_mm)
        && result.frequency_mhz.is_none_or(positive_finite)
        && controlled_impedance_solver_input_deck_has_evidence(result)
        && controlled_impedance_solver_sweep_has_evidence(result)
        && solver_stackup_has_evidence(bound, result)
        && match result.result_type {
            ControlledImpedanceSolverResultType::SingleEnded => result
                .net
                .as_deref()
                .map(str::trim)
                .filter(|net| !net.is_empty())
                .is_some_and(|net| {
                    result.first_net.is_none()
                        && result.second_net.is_none()
                        && result.solved_gap_mm.is_none()
                        && result.max_route_gap_delta_mm.is_none()
                        && result.input_gap_mm.is_none()
                        && bound.project.board.nets.contains_key(net)
                        && matching_single_ended_solver_target(bound, result, net)
                        && bound
                            .project
                            .board
                            .layout
                            .routes
                            .get(net)
                            .is_some_and(|route| {
                                route_has_layer_segments(route, &result.route_layer)
                            })
                }),
            ControlledImpedanceSolverResultType::Differential => {
                if result.net.is_some()
                    || !result.solved_gap_mm.is_some_and(positive_finite)
                    || !result
                        .max_route_gap_delta_mm
                        .is_some_and(non_negative_finite)
                    || (controlled_impedance_solver_input_deck_policy_requested(result)
                        && !result.input_gap_mm.is_some_and(positive_finite))
                {
                    return false;
                }
                let Some(first_net) = result
                    .first_net
                    .as_deref()
                    .map(str::trim)
                    .filter(|net| !net.is_empty())
                else {
                    return false;
                };
                let Some(second_net) = result
                    .second_net
                    .as_deref()
                    .map(str::trim)
                    .filter(|net| !net.is_empty())
                else {
                    return false;
                };
                first_net != second_net
                    && bound.project.board.nets.contains_key(first_net)
                    && bound.project.board.nets.contains_key(second_net)
                    && matching_differential_solver_target(bound, result, first_net, second_net)
                    && bound
                        .project
                        .board
                        .layout
                        .routes
                        .get(first_net)
                        .is_some_and(|first_route| {
                            bound
                                .project
                                .board
                                .layout
                                .routes
                                .get(second_net)
                                .is_some_and(|second_route| {
                                    route_has_layer_segments(first_route, &result.route_layer)
                                        && route_has_layer_segments(
                                            second_route,
                                            &result.route_layer,
                                        )
                                        && routes_have_parallel_gap_evidence(
                                            first_route,
                                            second_route,
                                        )
                                })
                        })
            }
        }
}

fn controlled_impedance_solver_input_deck_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.solver_input_deck_uri.is_some()
        || result.solver_input_deck_sha256.is_some()
        || result.input_stackup_revision.is_some()
        || result.input_route_layer.is_some()
        || result.input_reference_layer.is_some()
        || result.input_dielectric_layer.is_some()
        || result.input_width_mm.is_some()
        || result.input_gap_mm.is_some()
        || result.input_frequency_mhz.is_some()
}

fn controlled_impedance_solver_input_deck_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_input_deck_policy_requested(result) {
        return true;
    }
    result
        .solver_input_deck_uri
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_input_deck_sha256
            .as_deref()
            .is_some_and(|value| is_sha256_hex(value.trim()))
        && result
            .input_stackup_revision
            .as_deref()
            .is_some_and(|value| value.trim() == result.stackup_revision)
        && result
            .input_route_layer
            .as_deref()
            .is_some_and(|value| value.trim() == result.route_layer)
        && result
            .input_reference_layer
            .as_deref()
            .is_some_and(|value| value.trim() == result.reference_layer)
        && result
            .input_dielectric_layer
            .as_deref()
            .is_some_and(|value| value.trim() == result.dielectric_layer)
        && result
            .input_width_mm
            .is_some_and(|value| (value - result.solved_width_mm).abs() <= f64::EPSILON)
        && match result.result_type {
            ControlledImpedanceSolverResultType::SingleEnded => result.input_gap_mm.is_none(),
            ControlledImpedanceSolverResultType::Differential => {
                if let (Some(input_gap), Some(solved_gap)) =
                    (result.input_gap_mm, result.solved_gap_mm)
                {
                    (input_gap - solved_gap).abs() <= f64::EPSILON
                } else {
                    false
                }
            }
        }
        && match (result.frequency_mhz, result.input_frequency_mhz) {
            (Some(frequency), Some(input_frequency)) => {
                (input_frequency - frequency).abs() <= f64::EPSILON
            }
            (Some(_), None) => false,
            (None, Some(input_frequency)) => positive_finite(input_frequency),
            (None, None) => true,
        }
}

fn controlled_impedance_solver_sweep_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if result.min_solver_sample_count.is_none()
        && result.max_solver_frequency_step_mhz.is_none()
        && result.required_solver_corners.is_empty()
    {
        return true;
    }
    if result
        .min_solver_sample_count
        .is_some_and(|count| count == 0)
        || result
            .max_solver_frequency_step_mhz
            .is_some_and(|step| !positive_finite(step))
        || result.samples.is_empty()
    {
        return false;
    }
    let mut required_corners = std::collections::BTreeSet::new();
    for corner in &result.required_solver_corners {
        let corner = corner.trim();
        if corner.is_empty() || !required_corners.insert(corner.to_string()) {
            return false;
        }
    }
    let mut sample_names = std::collections::BTreeSet::new();
    let mut sample_corners = std::collections::BTreeSet::new();
    let mut corner_frequencies: std::collections::BTreeMap<String, Vec<f64>> =
        std::collections::BTreeMap::new();
    for sample in &result.samples {
        if sample.name.trim().is_empty()
            || sample.source.trim().is_empty()
            || sample.corner.trim().is_empty()
            || !positive_finite(sample.frequency_mhz)
            || !positive_finite(sample.solved_impedance_ohm)
            || !sample_names.insert(sample.name.trim().to_string())
            || (sample.solved_impedance_ohm - result.target_impedance_ohm).abs()
                > result.max_impedance_error_ohm + f64::EPSILON
        {
            return false;
        }
        let corner = sample.corner.trim().to_string();
        sample_corners.insert(corner.clone());
        corner_frequencies
            .entry(corner)
            .or_default()
            .push(sample.frequency_mhz);
    }
    if let Some(min_count) = result.min_solver_sample_count
        && result.samples.len() < min_count
    {
        return false;
    }
    if required_corners
        .iter()
        .any(|corner| !sample_corners.contains(corner))
    {
        return false;
    }
    if let Some(max_step) = result.max_solver_frequency_step_mhz {
        for frequencies in corner_frequencies.values_mut() {
            if frequencies.len() < 2 {
                return false;
            }
            frequencies.sort_by(|a, b| a.total_cmp(b));
            if frequencies
                .windows(2)
                .any(|window| window[1] - window[0] > max_step + f64::EPSILON)
            {
                return false;
            }
        }
    }
    true
}

fn solver_stackup_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let layers = &bound.project.board.layout.stackup.layers;
    layers
        .iter()
        .find(|layer| layer.name == result.route_layer)
        .is_some_and(|layer| layer.kind == StackupLayerKind::Signal)
        && layers
            .iter()
            .find(|layer| layer.name == result.reference_layer)
            .is_some_and(|layer| layer.kind == StackupLayerKind::Plane)
        && layers
            .iter()
            .find(|layer| layer.name == result.dielectric_layer)
            .is_some_and(|layer| layer.kind == StackupLayerKind::Dielectric)
}

fn route_has_layer_segments(route: &NetRoute, layer: &str) -> bool {
    route
        .segments
        .iter()
        .any(|segment| segment.layer == layer && usable_route_segment(segment))
}

fn matching_single_ended_solver_target(
    bound: &BoundBoard<'_>,
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
    targets.len() == 1
        && positive_finite(targets[0].target_impedance_ohm)
        && (targets[0].target_impedance_ohm - result.target_impedance_ohm).abs() <= 1.0e-9
}

fn matching_differential_solver_target(
    bound: &BoundBoard<'_>,
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
    targets.len() == 1
        && positive_finite(targets[0].target_differential_impedance_ohm)
        && (targets[0].target_differential_impedance_ohm - result.target_impedance_ohm).abs()
            <= 1.0e-9
}

fn matching_single_ended_coupon_target(
    bound: &BoundBoard<'_>,
    coupon: &ControlledImpedanceCoupon,
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
    targets.len() == 1
        && positive_finite(targets[0].target_impedance_ohm)
        && (targets[0].target_impedance_ohm - coupon.target_impedance_ohm).abs() <= 1.0e-9
}

fn matching_differential_coupon_target(
    bound: &BoundBoard<'_>,
    coupon: &ControlledImpedanceCoupon,
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
    targets.len() == 1
        && positive_finite(targets[0].target_differential_impedance_ohm)
        && (targets[0].target_differential_impedance_ohm - coupon.target_impedance_ohm).abs()
            <= 1.0e-9
}

fn unordered_pair_matches(
    first: &str,
    second: &str,
    expected_first: &str,
    expected_second: &str,
) -> bool {
    (first == expected_first && second == expected_second)
        || (first == expected_second && second == expected_first)
}

fn positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn non_negative_finite(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn route_has_valid_segments(route: &NetRoute) -> bool {
    !route.segments.is_empty() && route.segments.iter().all(usable_route_segment)
}

fn routes_have_parallel_gap_evidence(first_route: &NetRoute, second_route: &NetRoute) -> bool {
    first_route.segments.iter().any(|first| {
        second_route.segments.iter().any(|second| {
            first.layer == second.layer
                && parallel_overlap_gap_mm(first, second).is_some_and(f64::is_finite)
        })
    })
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

fn adjacent_reference_plane<'a>(
    bound: &'a BoundBoard<'_>,
    route_layer: &str,
) -> Option<&'a StackupLayer> {
    let layers = &bound.project.board.layout.stackup.layers;
    let route_index = layers.iter().position(|layer| layer.name == route_layer)?;
    let mut candidates = Vec::new();
    for direction in [-1, 1] {
        if let Some(layer) = nearest_conductive_layer(layers, route_index, direction)
            && layer.kind == StackupLayerKind::Plane
            && layer.reference_net.as_ref().is_some_and(|net| {
                bound
                    .project
                    .board
                    .nets
                    .get(net)
                    .is_some_and(|spec| spec.kind == NetKind::Ground)
            })
        {
            candidates.push(layer);
        }
    }
    (candidates.len() == 1).then(|| candidates[0])
}

fn nearest_conductive_layer(
    layers: &[StackupLayer],
    route_index: usize,
    direction: isize,
) -> Option<&StackupLayer> {
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

fn stackup_layer_by_name<'a>(
    bound: &'a BoundBoard<'_>,
    layer_name: &str,
) -> Option<&'a StackupLayer> {
    bound
        .project
        .board
        .layout
        .stackup
        .layers
        .iter()
        .find(|layer| layer.name == layer_name)
}

fn dielectric_between_layers<'a>(
    bound: &'a BoundBoard<'_>,
    route_layer: &str,
    reference_layer: &str,
) -> Option<&'a StackupLayer> {
    let layers = &bound.project.board.layout.stackup.layers;
    let route_index = layers.iter().position(|layer| layer.name == route_layer)?;
    let reference_index = layers
        .iter()
        .position(|layer| layer.name == reference_layer)?;
    let low = route_index.min(reference_index);
    let high = route_index.max(reference_index);
    let candidates = layers[(low + 1)..high]
        .iter()
        .filter(|layer| layer.kind == StackupLayerKind::Dielectric)
        .collect::<Vec<_>>();
    (candidates.len() == 1).then(|| candidates[0])
}

fn stackup_copper_layer_has_evidence(layer: &StackupLayer) -> bool {
    layer
        .copper_thickness_um
        .is_some_and(|value| value.is_finite() && value > 0.0)
        && layer
            .source
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn stackup_dielectric_layer_has_evidence(layer: &StackupLayer) -> bool {
    layer
        .thickness_mm
        .is_some_and(|value| value.is_finite() && value > 0.0)
        && layer
            .dielectric_constant
            .is_some_and(|value| value.is_finite() && value > 0.0)
        && layer
            .material
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && layer
            .source
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
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

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn manufacturing_route_check_declared_for_net(
    bound: &BoundBoard<'_>,
    check: &str,
    net_name: &str,
) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario.checks.iter().any(|declared| declared == check)
            && scenario
                .parameters
                .get("routes")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|routes| {
                    routes.iter().any(|route| {
                        route.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("net".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(net_name)
                    })
                })
    })
}

fn controlled_impedance_net_check_declared(bound: &BoundBoard<'_>, net_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == CONTROLLED_IMPEDANCE_GEOMETRY_VALID)
            && scenario
                .parameters
                .get("nets")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|nets| {
                    nets.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("net".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(net_name)
                    })
                })
    })
}

fn controlled_impedance_pair_check_declared(
    bound: &BoundBoard<'_>,
    first_net: &str,
    second_net: &str,
) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == CONTROLLED_IMPEDANCE_GEOMETRY_VALID)
            && scenario
                .parameters
                .get("differential_pairs")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|pairs| {
                    pairs.iter().any(|item| {
                        let Some(mapping) = item.as_mapping() else {
                            return false;
                        };
                        let declared_first = mapping
                            .get(serde_yaml_ng::Value::String("first_net".to_string()))
                            .and_then(serde_yaml_ng::Value::as_str);
                        let declared_second = mapping
                            .get(serde_yaml_ng::Value::String("second_net".to_string()))
                            .and_then(serde_yaml_ng::Value::as_str);
                        matches!(
                            (declared_first, declared_second),
                            (Some(left), Some(right))
                                if (left == first_net && right == second_net)
                                    || (left == second_net && right == first_net)
                        )
                    })
                })
    })
}

fn controlled_impedance_coupon_check_declared(
    bound: &BoundBoard<'_>,
    check_id: &str,
    coupon_name: &str,
) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario.checks.iter().any(|declared| declared == check_id)
            && scenario
                .parameters
                .get("coupons")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|coupons| {
                    coupons.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(coupon_name)
                    })
                })
    })
}

fn controlled_impedance_solver_result_check_declared(
    bound: &BoundBoard<'_>,
    result_name: &str,
) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID)
            && scenario
                .parameters
                .get("solver_results")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|results| {
                    results.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(result_name)
                    })
                })
    })
}
