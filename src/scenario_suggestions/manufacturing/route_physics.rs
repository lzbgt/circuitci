use super::manufacturing_suggestion;
use crate::board_ir::{
    ControlledImpedanceCoupon, ControlledImpedanceCouponType,
    ControlledImpedanceDifferentialPairTarget, ControlledImpedanceNetTarget, CopperZone,
    LayoutCopper, LayoutPoint, NetKind, NetRoute, RouteSegment, RouteVia, StackupLayer,
    StackupLayerKind,
};
use crate::library::BoundBoard;
use crate::scenario_suggestions::{ScenarioSuggestion, sanitized_name};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const CONTROLLED_IMPEDANCE_GEOMETRY_VALID: &str = "CONTROLLED_IMPEDANCE_GEOMETRY_VALID";
const CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID: &str =
    "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID";
const CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID: &str =
    "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID";
const CONTROLLED_IMPEDANCE_COUPON_VALID: &str = "CONTROLLED_IMPEDANCE_COUPON_VALID";
const ADJACENT_PLANE_RETURN_PATH_VALID: &str = "ADJACENT_PLANE_RETURN_PATH_VALID";
const REFERENCE_PLANE_SLOT_CROSSING_VALID: &str = "REFERENCE_PLANE_SLOT_CROSSING_VALID";
const RETURN_PATH_STITCHING_VIA_VALID: &str = "RETURN_PATH_STITCHING_VIA_VALID";

pub(super) fn route_physics_suggestions(
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
    suggestions.extend(adjacent_plane_return_path_suggestions(bound, project_name));
    suggestions.extend(reference_plane_slot_crossing_suggestions(
        bound,
        project_name,
    ));
    suggestions.extend(return_path_stitching_via_suggestions(bound, project_name));
    suggestions
}

fn adjacent_plane_return_path_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for (net_name, net) in &bound.project.board.nets {
        if net.kind != NetKind::DigitalOrAnalog
            || manufacturing_route_check_declared_for_net(
                bound,
                ADJACENT_PLANE_RETURN_PATH_VALID,
                net_name,
            )
        {
            continue;
        }
        let Some(evidence) = adjacent_plane_return_path_evidence(bound, net_name) else {
            continue;
        };
        suggestions.push(manufacturing_suggestion(
            &format!("adjacent_plane_return_path_{}", sanitized_name(net_name)),
            true,
            &format!(
                "Route {net_name} has imported route segments, explicit stackup evidence, and sampled adjacent {} plane-zone coverage on {}.",
                evidence.reference_net, evidence.reference_layer
            ),
            &format!(
                "{}_{}_adjacent_plane_return_path",
                project_name,
                sanitized_name(net_name)
            ),
            ADJACENT_PLANE_RETURN_PATH_VALID,
            Some(BTreeMap::from([(
                "routes".to_string(),
                json!([{
                    "net": net_name,
                    "reference_net": evidence.reference_net,
                    "reference_layer": evidence.reference_layer,
                    "max_unreferenced_length_mm": 0.0
                }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn reference_plane_slot_crossing_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for (net_name, net) in &bound.project.board.nets {
        if net.kind != NetKind::DigitalOrAnalog
            || manufacturing_route_check_declared_for_net(
                bound,
                REFERENCE_PLANE_SLOT_CROSSING_VALID,
                net_name,
            )
        {
            continue;
        }
        let Some(evidence) = reference_plane_slot_crossing_evidence(bound, net_name) else {
            continue;
        };
        suggestions.push(manufacturing_suggestion(
            &format!("reference_plane_slot_crossing_{}", sanitized_name(net_name)),
            true,
            &format!(
                "Route {net_name} has imported route segments, explicit stackup evidence, adjacent {} plane-zone evidence on {}, and {} internal reference-plane gap(s) along the route centerline.",
                evidence.reference_net, evidence.reference_layer, evidence.slot_crossing_count
            ),
            &format!(
                "{}_{}_reference_plane_slot_crossing",
                project_name,
                sanitized_name(net_name)
            ),
            REFERENCE_PLANE_SLOT_CROSSING_VALID,
            Some(BTreeMap::from([(
                "routes".to_string(),
                json!([{
                    "net": net_name,
                    "reference_net": evidence.reference_net,
                    "reference_layer": evidence.reference_layer,
                    "max_slot_crossings": 0
                }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn return_path_stitching_via_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let Some(max_stitch_via_distance_mm) = bound
        .project
        .board
        .manufacturing
        .max_stitch_via_distance_mm
        .filter(|value| value.is_finite() && *value >= 0.0)
    else {
        return Vec::new();
    };
    let mut suggestions = Vec::new();
    for (net_name, net) in &bound.project.board.nets {
        if net.kind != NetKind::DigitalOrAnalog
            || manufacturing_route_check_declared_for_net(
                bound,
                RETURN_PATH_STITCHING_VIA_VALID,
                net_name,
            )
        {
            continue;
        }
        let Some(evidence) = return_path_stitching_via_evidence(bound, net_name) else {
            continue;
        };
        suggestions.push(manufacturing_suggestion(
            &format!("return_path_stitching_via_{}", sanitized_name(net_name)),
            true,
            &format!(
                "Route {net_name} has {} imported layer-transition via(s), explicit stackup evidence, {} declared {} stitching via(s), and reviewed board.manufacturing.max_stitch_via_distance_mm policy.",
                evidence.signal_via_count, evidence.reference_via_count, evidence.reference_net
            ),
            &format!(
                "{}_{}_return_path_stitching_via",
                project_name,
                sanitized_name(net_name)
            ),
            RETURN_PATH_STITCHING_VIA_VALID,
            Some(BTreeMap::from([(
                "routes".to_string(),
                json!([{
                    "net": net_name,
                    "reference_net": evidence.reference_net,
                    "max_stitch_via_distance_mm": max_stitch_via_distance_mm
                }]),
            )])),
            Vec::new(),
        ));
    }
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
        if controlled_impedance_coupon_check_declared(bound, &coupon.name)
            || !controlled_impedance_coupon_has_evidence(bound, coupon)
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
                "Controlled-impedance coupon {} has reviewed measured impedance evidence from {} and a reviewed tolerance.",
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

#[derive(Debug)]
struct AdjacentPlaneEvidence {
    reference_net: String,
    reference_layer: String,
}

#[derive(Debug)]
struct SlotCrossingEvidence {
    reference_net: String,
    reference_layer: String,
    slot_crossing_count: usize,
}

#[derive(Debug)]
struct StitchingViaEvidence {
    reference_net: String,
    signal_via_count: usize,
    reference_via_count: usize,
}

#[derive(Debug)]
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
        }
    }
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

fn adjacent_plane_return_path_evidence(
    bound: &BoundBoard<'_>,
    net_name: &str,
) -> Option<AdjacentPlaneEvidence> {
    let route = bound.project.board.layout.routes.get(net_name)?;
    if route.segments.is_empty() || bound.project.board.layout.stackup.layers.is_empty() {
        return None;
    }
    let mut reference_net = None::<String>;
    let mut reference_layer = None::<String>;
    for segment in &route.segments {
        if !usable_route_segment(segment) {
            return None;
        }
        let layer = adjacent_reference_plane(bound, &segment.layer)?;
        let net = layer.reference_net.as_ref()?;
        let zones = bound.project.board.layout.zones.get(net)?;
        if !segment_has_plane_coverage(segment, &layer.name, zones) {
            return None;
        }
        if reference_net
            .as_deref()
            .is_some_and(|current| current != net)
        {
            return None;
        }
        if reference_layer
            .as_deref()
            .is_some_and(|current| current != layer.name)
        {
            return None;
        }
        reference_net = Some(net.clone());
        reference_layer = Some(layer.name.clone());
    }
    Some(AdjacentPlaneEvidence {
        reference_net: reference_net?,
        reference_layer: reference_layer?,
    })
}

fn reference_plane_slot_crossing_evidence(
    bound: &BoundBoard<'_>,
    net_name: &str,
) -> Option<SlotCrossingEvidence> {
    let route = bound.project.board.layout.routes.get(net_name)?;
    if route.segments.is_empty() || bound.project.board.layout.stackup.layers.is_empty() {
        return None;
    }
    let mut reference_net = None::<String>;
    let mut reference_layer = None::<String>;
    let mut slot_crossing_count = 0usize;
    for segment in &route.segments {
        if !usable_route_segment(segment) {
            return None;
        }
        let layer = adjacent_reference_plane(bound, &segment.layer)?;
        let net = layer.reference_net.as_ref()?;
        let zones = bound.project.board.layout.zones.get(net)?;
        let segment_crossings = segment_slot_crossing_count(segment, &layer.name, zones)?;
        slot_crossing_count += segment_crossings;
        if reference_net
            .as_deref()
            .is_some_and(|current| current != net)
        {
            return None;
        }
        if reference_layer
            .as_deref()
            .is_some_and(|current| current != layer.name)
        {
            return None;
        }
        reference_net = Some(net.clone());
        reference_layer = Some(layer.name.clone());
    }
    if slot_crossing_count == 0 {
        return None;
    }
    Some(SlotCrossingEvidence {
        reference_net: reference_net?,
        reference_layer: reference_layer?,
        slot_crossing_count,
    })
}

fn return_path_stitching_via_evidence(
    bound: &BoundBoard<'_>,
    net_name: &str,
) -> Option<StitchingViaEvidence> {
    let route = bound.project.board.layout.routes.get(net_name)?;
    if route.vias.is_empty() || bound.project.board.layout.stackup.layers.is_empty() {
        return None;
    }
    let stackup_layers = stackup_layer_names(bound);
    let reference_net = route_reference_net(bound, route)?;
    if !route
        .vias
        .iter()
        .all(|via| usable_route_via(via, &stackup_layers))
    {
        return None;
    }
    let reference_route = bound.project.board.layout.routes.get(&reference_net)?;
    if reference_route.vias.is_empty()
        || !reference_route
            .vias
            .iter()
            .all(|via| usable_route_via(via, &stackup_layers))
    {
        return None;
    }
    let has_matching_reference_via = route.vias.iter().any(|signal_via| {
        reference_route
            .vias
            .iter()
            .any(|reference_via| via_layers_match(signal_via, reference_via))
    });
    has_matching_reference_via.then_some(StitchingViaEvidence {
        reference_net,
        signal_via_count: route.vias.len(),
        reference_via_count: reference_route.vias.len(),
    })
}

fn route_reference_net(bound: &BoundBoard<'_>, route: &NetRoute) -> Option<String> {
    let mut reference_net = None::<String>;
    for segment in &route.segments {
        if !usable_route_segment(segment) {
            return None;
        }
        let layer = adjacent_reference_plane(bound, &segment.layer)?;
        let net = layer.reference_net.as_ref()?;
        if reference_net
            .as_deref()
            .is_some_and(|current| current != net)
        {
            return None;
        }
        reference_net = Some(net.clone());
    }
    reference_net
}

fn stackup_layer_names(bound: &BoundBoard<'_>) -> BTreeSet<String> {
    bound
        .project
        .board
        .layout
        .stackup
        .layers
        .iter()
        .map(|layer| layer.name.clone())
        .collect()
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

fn usable_route_via(via: &RouteVia, stackup_layers: &BTreeSet<String>) -> bool {
    via.at.x_mm.is_finite()
        && via.at.y_mm.is_finite()
        && via.size_mm.is_finite()
        && via.size_mm > 0.0
        && via.drill_mm.is_finite()
        && via.drill_mm > 0.0
        && via.layers.len() >= 2
        && via
            .layers
            .iter()
            .all(|layer| !layer.trim().is_empty() && stackup_layers.contains(layer))
}

fn via_layers_match(signal_via: &RouteVia, reference_via: &RouteVia) -> bool {
    signal_via.layers.iter().all(|layer| {
        reference_via
            .layers
            .iter()
            .any(|candidate| candidate == layer)
    })
}

fn segment_has_plane_coverage(
    segment: &RouteSegment,
    reference_layer: &str,
    zones: &[CopperZone],
) -> bool {
    let polygons = zones
        .iter()
        .filter(|zone| zone.layer == reference_layer)
        .flat_map(zone_polygons)
        .filter(|polygon| usable_polygon(polygon))
        .collect::<Vec<_>>();
    if polygons.is_empty() {
        return false;
    }
    let samples = [
        (segment.start.x_mm, segment.start.y_mm),
        (
            (segment.start.x_mm + segment.end.x_mm) / 2.0,
            (segment.start.y_mm + segment.end.y_mm) / 2.0,
        ),
        (segment.end.x_mm, segment.end.y_mm),
    ];
    samples.iter().all(|sample| {
        polygons
            .iter()
            .any(|polygon| point_in_polygon(sample.0, sample.1, polygon))
    })
}

fn zone_polygons(zone: &CopperZone) -> Box<dyn Iterator<Item = &Vec<LayoutPoint>> + '_> {
    if zone.filled_polygons.is_empty() {
        Box::new(std::iter::once(&zone.polygon))
    } else {
        Box::new(zone.filled_polygons.iter())
    }
}

fn usable_polygon(polygon: &[LayoutPoint]) -> bool {
    polygon.len() >= 3
        && polygon
            .iter()
            .all(|point| point.x_mm.is_finite() && point.y_mm.is_finite())
}

fn segment_slot_crossing_count(
    segment: &RouteSegment,
    reference_layer: &str,
    zones: &[CopperZone],
) -> Option<usize> {
    let mut intervals = Vec::new();
    for polygon in zones
        .iter()
        .filter(|zone| zone.layer == reference_layer)
        .flat_map(zone_polygons)
        .filter(|polygon| usable_polygon(polygon))
    {
        intervals.extend(segment_polygon_coverage_intervals(segment, polygon));
    }
    let merged = merge_intervals(intervals);
    (!merged.is_empty()).then(|| {
        merged
            .windows(2)
            .filter(|pair| pair[1].0 > pair[0].1 + 1.0e-9)
            .count()
    })
}

fn segment_polygon_coverage_intervals(
    segment: &RouteSegment,
    polygon: &[LayoutPoint],
) -> Vec<(f64, f64)> {
    let mut samples = vec![0.0, 1.0];
    for current in 0..polygon.len() {
        let next = (current + 1) % polygon.len();
        if let Some(t) = segment_edge_intersection_t(segment, &polygon[current], &polygon[next])
            && t.is_finite()
            && (-1.0e-9..=1.0 + 1.0e-9).contains(&t)
        {
            samples.push(t.clamp(0.0, 1.0));
        }
    }
    samples.sort_by(f64::total_cmp);
    samples.dedup_by(|a, b| (*a - *b).abs() <= 1.0e-9);

    let mut intervals = Vec::new();
    for pair in samples.windows(2) {
        if pair[1] <= pair[0] + 1.0e-9 {
            continue;
        }
        let midpoint = (pair[0] + pair[1]) / 2.0;
        let (x, y) = point_at_t(segment, midpoint);
        if point_in_polygon(x, y, polygon) {
            intervals.push((pair[0], pair[1]));
        }
    }
    for sample in samples {
        let (x, y) = point_at_t(segment, sample);
        if point_in_polygon(x, y, polygon) {
            let start = (sample - 1.0e-9).clamp(0.0, 1.0);
            let end = (sample + 1.0e-9).clamp(0.0, 1.0);
            if end > start {
                intervals.push((start, end));
            }
        }
    }
    intervals
}

fn segment_edge_intersection_t(
    segment: &RouteSegment,
    edge_start: &LayoutPoint,
    edge_end: &LayoutPoint,
) -> Option<f64> {
    let px = segment.start.x_mm;
    let py = segment.start.y_mm;
    let rx = segment.end.x_mm - segment.start.x_mm;
    let ry = segment.end.y_mm - segment.start.y_mm;
    let qx = edge_start.x_mm;
    let qy = edge_start.y_mm;
    let sx = edge_end.x_mm - edge_start.x_mm;
    let sy = edge_end.y_mm - edge_start.y_mm;
    let denominator = cross(rx, ry, sx, sy);
    if denominator.abs() <= 1.0e-12 {
        return None;
    }
    let qpx = qx - px;
    let qpy = qy - py;
    let t = cross(qpx, qpy, sx, sy) / denominator;
    let u = cross(qpx, qpy, rx, ry) / denominator;
    ((-1.0e-9..=1.0 + 1.0e-9).contains(&t) && (-1.0e-9..=1.0 + 1.0e-9).contains(&u))
        .then_some(t.clamp(0.0, 1.0))
}

fn merge_intervals(mut intervals: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    intervals.retain(|(start, end)| start.is_finite() && end.is_finite() && *end > *start + 1.0e-9);
    intervals.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.total_cmp(&right.1))
    });
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (start, end) in intervals {
        if let Some(last) = merged.last_mut()
            && start <= last.1 + 1.0e-9
        {
            last.1 = last.1.max(end);
            continue;
        }
        merged.push((start, end));
    }
    merged
}

fn point_in_polygon(x: f64, y: f64, polygon: &[LayoutPoint]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let current_point = &polygon[current];
        let previous_point = &polygon[previous];
        if point_on_segment(x, y, current_point, previous_point) {
            return true;
        }
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

fn point_on_segment(x: f64, y: f64, start: &LayoutPoint, end: &LayoutPoint) -> bool {
    let cross_product = cross(
        x - start.x_mm,
        y - start.y_mm,
        end.x_mm - start.x_mm,
        end.y_mm - start.y_mm,
    );
    if cross_product.abs() > 1.0e-9 {
        return false;
    }
    x >= start.x_mm.min(end.x_mm) - 1.0e-9
        && x <= start.x_mm.max(end.x_mm) + 1.0e-9
        && y >= start.y_mm.min(end.y_mm) - 1.0e-9
        && y <= start.y_mm.max(end.y_mm) + 1.0e-9
}

fn point_at_t(segment: &RouteSegment, t: f64) -> (f64, f64) {
    (
        segment.start.x_mm + (segment.end.x_mm - segment.start.x_mm) * t,
        segment.start.y_mm + (segment.end.y_mm - segment.start.y_mm) * t,
    )
}

fn cross(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ax * by - ay * bx
}

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
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

fn controlled_impedance_coupon_check_declared(bound: &BoundBoard<'_>, coupon_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == CONTROLLED_IMPEDANCE_COUPON_VALID)
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
