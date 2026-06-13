use super::super::{
    ScenarioSuggestion, SuggestedProtectionClamp, SuggestedScenario, SuggestedTarget,
    SuggestedUsbConnector, SuggestedUsbRoute, SuggestedUsbRoutePair,
    USB_CONNECTOR_PROTECTION_VALID, USB_PROTECTION_PLACEMENT_VALID, USB_RETURN_PATH_VALID,
    USB_ROUTE_GEOMETRY_VALID, USB_VBUS_ROUTE_VALID,
};
use super::{
    component_placement, placement_distance_mm, sanitized_name, suggested_placement,
    suggested_protection_clamp,
};
use crate::board_ir::{
    BoardProject, ComponentPlacement, ComponentSpec, NetKind, NetLayoutRule, NetRoute, RouteSegment,
};
use crate::library::{BoundBoard, ComponentModel, ProtectionReference, UsbConnector};
use std::collections::{BTreeMap, BTreeSet};

mod edge_evidence;
mod mechanical;
mod route_evidence;

use edge_evidence::{
    entry_clearance_evidence, nearest_board_edge_evidence, nearest_component_clearance_evidence,
    suggested_footprint,
};
pub(in crate::scenario_suggestions::interface_protection) use mechanical::{
    existing_usb_connector_body_overhang_checks, existing_usb_connector_component_clearance_checks,
    existing_usb_connector_edge_proximity_checks, existing_usb_connector_entry_clearance_checks,
    existing_usb_connector_orientation_checks, usb_connector_body_overhang_suggestion,
    usb_connector_component_clearance_suggestion, usb_connector_edge_proximity_suggestion,
    usb_connector_entry_clearance_suggestion, usb_connector_orientation_suggestion,
};
use route_evidence::{
    GroundReferenceGeometry, GroundZoneEvidence, ground_zone_outlines,
    ground_zones_have_filled_polygons, pad_to_route_distance_mm,
    return_path_filled_zone_clearance_segments, return_path_unreferenced_segments,
    route_distance_between_pads_mm, route_ground_zone_contacts, suggested_usb_route_pad,
};

pub(super) fn usb_connector_protection_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<ScenarioSuggestion> {
    let connector = model.usb_connector.as_ref()?;
    let suggested_connector = suggested_usb_connector(bound, component_id, component, connector)?;
    let dp_protection =
        suggested_valid_protection_clamp_for_net(bound, component_id, &suggested_connector.dp_net);
    let dm_protection =
        suggested_valid_protection_clamp_for_net(bound, component_id, &suggested_connector.dm_net);
    let vbus_protection = suggested_connector
        .vbus_net
        .as_deref()
        .and_then(|net| suggested_valid_protection_clamp_for_net(bound, component_id, net));
    let mut protection_clamps = Vec::new();
    if let Some(clamp) = dp_protection {
        protection_clamps.push(clamp);
    }
    if let Some(clamp) = dm_protection {
        protection_clamps.push(clamp);
    }
    if let Some(clamp) = vbus_protection {
        protection_clamps.push(clamp);
    }
    let require_vbus_protection = suggested_connector
        .vbus_net
        .as_deref()
        .and_then(|net| bound.project.board.nets.get(net))
        .is_some_and(|net| net.kind == NetKind::Power);
    let require_shield_ground = suggested_connector.shield_net.is_some();
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "require_vbus_protection".to_string(),
        serde_json::Value::Bool(require_vbus_protection),
    );
    parameters.insert(
        "require_shield_ground".to_string(),
        serde_json::Value::Bool(require_shield_ground),
    );
    if let Some(data_voltage) = max_nominal_voltage(
        bound,
        [
            suggested_connector.dp_net.as_str(),
            suggested_connector.dm_net.as_str(),
        ],
    ) {
        parameters.insert(
            "data_working_voltage_min_V".to_string(),
            serde_json::Value::from(data_voltage),
        );
    }
    if let Some(vbus_net) = suggested_connector.vbus_net.as_deref()
        && let Some(vbus_voltage) = max_nominal_voltage(bound, [vbus_net])
    {
        parameters.insert(
            "vbus_working_voltage_min_V".to_string(),
            serde_json::Value::from(vbus_voltage),
        );
    }
    let mut required_inputs = vec![
        "Use PCB/layout validation for USB connector placement, protection placement, return path, shield strategy, and differential routing.".to_string(),
        "Use clamp-specific INTERFACE_PROTECTION_REVIEW scenarios when capacitance budget, standoff, or reference-net evidence needs independent sign-off.".to_string(),
    ];
    if protection_clamps.is_empty() {
        required_inputs.insert(
            0,
            "Add datasheet-backed ESD/protection components on USB connector exposed nets; this runnable template will fail until coverage exists.".to_string(),
        );
    } else if protection_clamps.len() < 2 {
        required_inputs.insert(
            0,
            "Review missing USB D+ or D- protection coverage; this runnable template will fail if either data line has no valid clamp.".to_string(),
        );
    } else if require_vbus_protection && suggested_connector.vbus_net.is_some() {
        let has_vbus_clamp = protection_clamps.iter().any(|clamp| {
            suggested_connector
                .vbus_net
                .as_ref()
                .is_some_and(|net| clamp.protected_net == *net)
        });
        if !has_vbus_clamp {
            required_inputs.insert(
                0,
                "Review missing USB VBUS protection coverage; require_vbus_protection is true because VBUS is connected to a declared power net.".to_string(),
            );
        }
    }
    if require_shield_ground {
        required_inputs.push(
            "Review whether the simplified require_shield_ground check matches the board's intended USB shield strategy; model RC, ferrite, chassis-only, or spark-gap bonding explicitly before using this as EMC sign-off.".to_string(),
        );
    }
    Some(ScenarioSuggestion {
        id: format!("usb_connector_protection_{}", sanitized_name(component_id)),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable: true,
        reason: format!(
            "USB connector {component_id} exposes D+/D-/VBUS nets; add a connector-level protection coverage scenario."
        ),
        scenario: SuggestedScenario {
            name: format!("{}_usb_connector_protection", sanitized_name(component_id)),
            scenario_type: "interface_protection".to_string(),
            checks: vec![USB_CONNECTOR_PROTECTION_VALID.to_string()],
            parameters: Some(parameters),
            target: Some(SuggestedTarget {
                component: component_id.to_string(),
                power_pin: None,
                reset_pin: None,
            }),
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
            conditioning: None,
            protection_clamps,
            usb_connectors: vec![suggested_connector],
            usb_routes: Vec::new(),
            usb_route_pairs: Vec::new(),
            clocks: Vec::new(),
            reset_supervisors: Vec::new(),
            regulators: Vec::new(),
            pin_states: Vec::new(),
            paths: Vec::new(),
        },
        required_inputs,
    })
}

pub(super) fn usb_protection_placement_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<ScenarioSuggestion> {
    let connector = model.usb_connector.as_ref()?;
    let suggested_connector = suggested_usb_connector(bound, component_id, component, connector)?;
    let connector_placement = component_placement(bound, component_id)?;
    let require_vbus_protection = suggested_connector
        .vbus_net
        .as_deref()
        .and_then(|net| bound.project.board.nets.get(net))
        .is_some_and(|net| net.kind == NetKind::Power);
    let mut protection_clamps = vec![
        nearest_placed_protection_clamp_for_net(
            bound,
            component_id,
            &suggested_connector.dp_net,
            connector_placement,
        )?,
        nearest_placed_protection_clamp_for_net(
            bound,
            component_id,
            &suggested_connector.dm_net,
            connector_placement,
        )?,
    ];
    if require_vbus_protection {
        let vbus_net = suggested_connector.vbus_net.as_deref()?;
        protection_clamps.push(nearest_placed_protection_clamp_for_net(
            bound,
            component_id,
            vbus_net,
            connector_placement,
        )?);
    }
    let connector_rule = &bound.project.board.layout.constraints.usb_connector;
    let max_distance_mm = connector_rule.max_connector_to_protection_distance_mm;
    let runnable = max_distance_mm.is_some();
    let parameters = BTreeMap::from([
        (
            "require_vbus_protection".to_string(),
            serde_json::Value::Bool(require_vbus_protection),
        ),
        (
            "max_connector_to_protection_distance_mm".to_string(),
            optional_number_value(max_distance_mm),
        ),
    ]);
    Some(ScenarioSuggestion {
        id: format!("usb_protection_placement_{}", sanitized_name(component_id)),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable,
        reason: format!(
            "USB connector {component_id} and connected protection components have placement evidence; add a connector-to-protection distance scenario."
        ),
        scenario: SuggestedScenario {
            name: format!("{}_usb_protection_placement", sanitized_name(component_id)),
            scenario_type: "interface_protection".to_string(),
            checks: vec![USB_PROTECTION_PLACEMENT_VALID.to_string()],
            parameters: Some(parameters),
            target: Some(SuggestedTarget {
                component: component_id.to_string(),
                power_pin: None,
                reset_pin: None,
            }),
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
            conditioning: None,
            protection_clamps,
            usb_connectors: vec![suggested_connector],
            usb_routes: Vec::new(),
            usb_route_pairs: Vec::new(),
            clocks: Vec::new(),
            reset_supervisors: Vec::new(),
            regulators: Vec::new(),
            pin_states: Vec::new(),
            paths: Vec::new(),
        },
        required_inputs: if runnable {
            Vec::new()
        } else {
            vec![
                "Fill board.layout.constraints.usb_connector.max_connector_to_protection_distance_mm from the board's ESD/layout rule or datasheet/layout guidance; do not invent the limit from component coordinates.".to_string(),
                "Use PCB/layout review for routed trace order, via count, return path, shield strategy, and USB differential-pair constraints.".to_string(),
            ]
        },
    })
}

pub(super) fn usb_route_geometry_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<ScenarioSuggestion> {
    let connector = model.usb_connector.as_ref()?;
    let suggested_connector = suggested_usb_connector(bound, component_id, component, connector)?;
    let connector_placement = component_placement(bound, component_id)?;
    let dp_clamp = nearest_placed_protection_clamp_for_net(
        bound,
        component_id,
        &suggested_connector.dp_net,
        connector_placement,
    )?;
    let dm_clamp = nearest_placed_protection_clamp_for_net(
        bound,
        component_id,
        &suggested_connector.dm_net,
        connector_placement,
    )?;
    let dp_route = suggested_usb_route(
        bound,
        "D+",
        &suggested_connector.dp_net,
        Some(dp_clamp.component.clone()),
        Some((component_id, connector.dp_pin.as_str())),
        Some((dp_clamp.component.as_str(), dp_clamp.protected_pin.as_str())),
    )?;
    let dm_route = suggested_usb_route(
        bound,
        "D-",
        &suggested_connector.dm_net,
        Some(dm_clamp.component.clone()),
        Some((component_id, connector.dm_pin.as_str())),
        Some((dm_clamp.component.as_str(), dm_clamp.protected_pin.as_str())),
    )?;
    let route_pair = suggested_usb_route_pair(bound, &dp_route, &dm_route)?;
    let route_limits = suggested_usb_route_limits(bound, &dp_route.net, &dm_route.net);
    let runnable = route_limits.max_data_line_route_length_mm.is_some()
        && route_limits.max_data_pair_length_mismatch_mm.is_some();
    let parameters = BTreeMap::from([
        (
            "max_data_line_route_length_mm".to_string(),
            optional_number_value(route_limits.max_data_line_route_length_mm),
        ),
        (
            "max_data_line_via_count".to_string(),
            optional_usize_value(route_limits.max_data_line_via_count),
        ),
        (
            "max_data_line_width_delta_mm".to_string(),
            optional_number_value(route_limits.max_data_line_width_delta_mm),
        ),
        (
            "max_connector_to_protection_route_distance_mm".to_string(),
            optional_number_value(route_limits.max_connector_to_protection_route_distance_mm),
        ),
        (
            "max_component_to_route_distance_mm".to_string(),
            optional_number_value(route_limits.max_component_to_route_distance_mm),
        ),
        (
            "max_data_pair_length_mismatch_mm".to_string(),
            optional_number_value(route_limits.max_data_pair_length_mismatch_mm),
        ),
        (
            "max_data_pair_via_count_delta".to_string(),
            optional_usize_value(route_limits.max_data_pair_via_count_delta),
        ),
        (
            "max_data_pair_gap_delta_mm".to_string(),
            optional_number_value(route_limits.max_data_pair_gap_delta_mm),
        ),
        (
            "require_route_pad_contact_evidence".to_string(),
            optional_bool_value(route_limits.require_route_pad_contact_evidence),
        ),
    ]);
    Some(ScenarioSuggestion {
        id: format!("usb_route_geometry_{}", sanitized_name(component_id)),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable,
        reason: format!(
            "USB connector {component_id} has placed protection components and imported D+/D- route geometry; add route-length and differential-pair matching checks."
        ),
        scenario: SuggestedScenario {
            name: format!("{}_usb_route_geometry", sanitized_name(component_id)),
            scenario_type: "interface_protection".to_string(),
            checks: vec![USB_ROUTE_GEOMETRY_VALID.to_string()],
            parameters: Some(parameters),
            target: Some(SuggestedTarget {
                component: component_id.to_string(),
                power_pin: None,
                reset_pin: None,
            }),
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
            conditioning: None,
            protection_clamps: vec![dp_clamp, dm_clamp],
            usb_connectors: vec![suggested_connector],
            usb_routes: vec![dp_route, dm_route],
            usb_route_pairs: vec![route_pair],
            clocks: Vec::new(),
            reset_supervisors: Vec::new(),
            regulators: Vec::new(),
            pin_states: Vec::new(),
            paths: Vec::new(),
        },
        required_inputs: if runnable {
            Vec::new()
        } else {
            let mut inputs = Vec::new();
            if route_limits.max_data_line_route_length_mm.is_none() {
                inputs.push(
                    "Fill max_data_line_route_length_mm from the board's USB layout rule or signal-integrity budget.".to_string(),
                );
            }
            if route_limits.max_data_pair_length_mismatch_mm.is_none() {
                inputs.push(
                    "Fill max_data_pair_length_mismatch_mm from the USB differential-pair matching rule.".to_string(),
                );
            }
            inputs.push(
                "Optional USB route checks can be enabled later with via-count, width-delta, pair-gap, pad-contact, and connector-to-protection route-distance limits.".to_string(),
            );
            inputs
        },
    })
}

pub(super) fn usb_vbus_route_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<ScenarioSuggestion> {
    let connector = model.usb_connector.as_ref()?;
    let suggested_connector = suggested_usb_connector(bound, component_id, component, connector)?;
    let vbus_net = suggested_connector.vbus_net.as_deref()?;
    if bound
        .project
        .board
        .nets
        .get(vbus_net)
        .is_none_or(|net| net.kind != NetKind::Power)
    {
        return None;
    }
    let connector_placement = component_placement(bound, component_id)?;
    let vbus_clamp = nearest_placed_protection_clamp_for_net(
        bound,
        component_id,
        vbus_net,
        connector_placement,
    )?;
    let vbus_route = suggested_usb_vbus_route(
        bound,
        vbus_net,
        Some(vbus_clamp.component.clone()),
        Some((component_id, connector.vbus_pin.as_str())),
        Some((
            vbus_clamp.component.as_str(),
            vbus_clamp.protected_pin.as_str(),
        )),
    )?;
    let route_limit = bound
        .project
        .board
        .layout
        .constraints
        .net_rules
        .get(vbus_net)
        .and_then(|rule| rule.length_max_mm);
    let min_width = vbus_route.expected_vbus_route_width_mm;
    let parameters = BTreeMap::from([
        (
            "max_vbus_route_length_mm".to_string(),
            optional_number_value(route_limit),
        ),
        ("max_vbus_via_count".to_string(), serde_json::Value::Null),
        (
            "min_vbus_route_width_mm".to_string(),
            optional_number_value(min_width),
        ),
        (
            "max_connector_to_vbus_protection_route_distance_mm".to_string(),
            serde_json::Value::Null,
        ),
        (
            "max_component_to_route_distance_mm".to_string(),
            serde_json::Value::Null,
        ),
        (
            "require_vbus_route_pad_contact_evidence".to_string(),
            serde_json::Value::Null,
        ),
    ]);
    let runnable = route_limit.is_some();
    Some(ScenarioSuggestion {
        id: format!("usb_vbus_route_{}", sanitized_name(component_id)),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable,
        reason: format!(
            "USB connector {component_id} has VBUS route and protection evidence; add VBUS power-entry route checks."
        ),
        scenario: SuggestedScenario {
            name: format!("{}_usb_vbus_route", sanitized_name(component_id)),
            scenario_type: "interface_protection".to_string(),
            checks: vec![USB_VBUS_ROUTE_VALID.to_string()],
            parameters: Some(parameters),
            target: Some(SuggestedTarget {
                component: component_id.to_string(),
                power_pin: None,
                reset_pin: None,
            }),
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
            conditioning: None,
            protection_clamps: vec![vbus_clamp],
            usb_connectors: vec![suggested_connector],
            usb_routes: vec![vbus_route],
            usb_route_pairs: Vec::new(),
            clocks: Vec::new(),
            reset_supervisors: Vec::new(),
            regulators: Vec::new(),
            pin_states: Vec::new(),
            paths: Vec::new(),
        },
        required_inputs: if runnable {
            Vec::new()
        } else {
            vec![
                route_limit_required_input(
                    route_limit,
                    "max_vbus_route_length_mm",
                    "Fill max_vbus_route_length_mm from the board's USB power-entry layout rule.",
                ),
                "Optional VBUS route checks can be enabled later with via-count, width, pad-contact, and connector-to-protection route-distance limits.".to_string(),
            ]
        },
    })
}

pub(super) fn usb_return_path_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<ScenarioSuggestion> {
    let connector = model.usb_connector.as_ref()?;
    let suggested_connector = suggested_usb_connector(bound, component_id, component, connector)?;
    let ground_zones = ground_zone_outlines(bound);
    if ground_zones.is_empty() {
        return None;
    }
    let dp_route = suggested_usb_route_with_return_path(
        bound,
        "D+",
        &suggested_connector.dp_net,
        &ground_zones,
    )?;
    let dm_route = suggested_usb_route_with_return_path(
        bound,
        "D-",
        &suggested_connector.dm_net,
        &ground_zones,
    )?;
    let measured_unreferenced_length_mm = dp_route
        .unreferenced_route_length_mm
        .unwrap_or(0.0)
        .max(dm_route.unreferenced_route_length_mm.unwrap_or(0.0));
    let return_path_rule = &bound.project.board.layout.constraints.usb_return_path;
    let max_unreferenced_length_mm = return_path_rule.max_data_line_unreferenced_length_mm;
    let runnable = max_unreferenced_length_mm.is_some();
    Some(ScenarioSuggestion {
        id: format!("usb_return_path_{}", sanitized_name(component_id)),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable,
        reason: format!(
            "USB connector {component_id} has imported D+/D- route geometry and same-layer ground-zone outline evidence; add static return-path coverage checks."
        ),
        scenario: SuggestedScenario {
            name: format!("{}_usb_return_path", sanitized_name(component_id)),
            scenario_type: "interface_protection".to_string(),
            checks: vec![USB_RETURN_PATH_VALID.to_string()],
            parameters: Some(BTreeMap::from([
                (
                    "max_data_line_unreferenced_length_mm".to_string(),
                    optional_number_value(max_unreferenced_length_mm),
                ),
                (
                    "max_data_via_to_ground_stitch_distance_mm".to_string(),
                    optional_number_value(
                        return_path_rule.max_data_via_to_ground_stitch_distance_mm,
                    ),
                ),
                (
                    "require_filled_zone_coverage".to_string(),
                    optional_bool_value(return_path_rule.require_filled_zone_coverage),
                ),
                (
                    "min_data_line_filled_zone_edge_clearance_mm".to_string(),
                    optional_number_value(
                        return_path_rule.min_data_line_filled_zone_edge_clearance_mm,
                    ),
                ),
                (
                    "require_ground_zone_contact_evidence".to_string(),
                    optional_bool_value(return_path_rule.require_ground_zone_contact_evidence),
                ),
            ])),
            target: Some(SuggestedTarget {
                component: component_id.to_string(),
                power_pin: None,
                reset_pin: None,
            }),
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
            conditioning: None,
            protection_clamps: Vec::new(),
            usb_connectors: vec![suggested_connector],
            usb_routes: vec![dp_route, dm_route],
            usb_route_pairs: Vec::new(),
            clocks: Vec::new(),
            reset_supervisors: Vec::new(),
            regulators: Vec::new(),
            pin_states: Vec::new(),
            paths: Vec::new(),
        },
        required_inputs: if runnable {
            Vec::new()
        } else {
            vec![
                format!(
                    "Fill board.layout.constraints.usb_return_path.max_data_line_unreferenced_length_mm from the board's USB return-path/layout rule after reviewing measured uncovered length {measured_unreferenced_length_mm:.3} mm."
                ),
                "Optional USB return-path checks can be enabled later with stitching-via distance, filled-zone coverage, filled-zone edge clearance, and ground-zone contact evidence policy.".to_string(),
                "Treat this as a same-layer ground-zone screen only; adjacent planes, filled-zone island continuity, impedance, and EMI need more specific evidence.".to_string(),
            ]
        },
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SuggestedUsbRouteLimits {
    max_data_line_route_length_mm: Option<f64>,
    max_data_line_via_count: Option<usize>,
    max_data_line_width_delta_mm: Option<f64>,
    max_connector_to_protection_route_distance_mm: Option<f64>,
    max_component_to_route_distance_mm: Option<f64>,
    max_data_pair_length_mismatch_mm: Option<f64>,
    max_data_pair_via_count_delta: Option<usize>,
    max_data_pair_gap_delta_mm: Option<f64>,
    require_route_pad_contact_evidence: Option<bool>,
}

fn suggested_usb_route_limits(
    bound: &BoundBoard<'_>,
    dp_net: &str,
    dm_net: &str,
) -> SuggestedUsbRouteLimits {
    let dp_rule = bound.project.board.layout.constraints.net_rules.get(dp_net);
    let dm_rule = bound.project.board.layout.constraints.net_rules.get(dm_net);
    let usb_route = &bound.project.board.layout.constraints.usb_route;
    SuggestedUsbRouteLimits {
        max_data_line_route_length_mm: min_rule_value(dp_rule, dm_rule, |rule| rule.length_max_mm),
        max_data_line_via_count: usb_route.max_data_line_via_count,
        max_data_line_width_delta_mm: usb_route.max_data_line_width_delta_mm,
        max_connector_to_protection_route_distance_mm: usb_route
            .max_connector_to_protection_route_distance_mm,
        max_component_to_route_distance_mm: usb_route.max_component_to_route_distance_mm,
        max_data_pair_length_mismatch_mm: min_rule_value(dp_rule, dm_rule, |rule| rule.skew_max_mm),
        max_data_pair_via_count_delta: usb_route.max_data_pair_via_count_delta,
        max_data_pair_gap_delta_mm: usb_route.max_data_pair_gap_delta_mm,
        require_route_pad_contact_evidence: usb_route.require_route_pad_contact_evidence,
    }
}

fn min_rule_value(
    first: Option<&NetLayoutRule>,
    second: Option<&NetLayoutRule>,
    value: impl Fn(&NetLayoutRule) -> Option<f64>,
) -> Option<f64> {
    match (first.and_then(&value), second.and_then(value)) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn expected_usb_data_width_mm(rule: &NetLayoutRule) -> Option<f64> {
    rule.diff_pair_width_mm.or(rule.track_width_mm)
}

fn optional_number_value(value: Option<f64>) -> serde_json::Value {
    value.map_or(serde_json::Value::Null, serde_json::Value::from)
}

fn optional_usize_value(value: Option<usize>) -> serde_json::Value {
    value.map_or(serde_json::Value::Null, serde_json::Value::from)
}

fn optional_bool_value(value: Option<bool>) -> serde_json::Value {
    value.map_or(serde_json::Value::Null, serde_json::Value::from)
}

fn route_limit_required_input(value: Option<f64>, field: &str, fallback: &str) -> String {
    if value.is_some() {
        format!(
            "Review imported KiCad routing constraint {field} before treating the route template as sign-off."
        )
    } else {
        fallback.to_string()
    }
}

fn suggested_usb_route(
    bound: &BoundBoard<'_>,
    signal: &str,
    net_name: &str,
    protection_component: Option<String>,
    connector_pad_ref: Option<(&str, &str)>,
    protection_pad_ref: Option<(&str, &str)>,
) -> Option<SuggestedUsbRoute> {
    let route = bound.project.board.layout.routes.get(net_name)?;
    if route.segments.is_empty() {
        return None;
    }
    let expected_data_line_width_mm = bound
        .project
        .board
        .layout
        .constraints
        .net_rules
        .get(net_name)
        .and_then(expected_usb_data_width_mm);
    let width_evidence = expected_data_line_width_mm
        .and_then(|expected_width_mm| route_width_delta(route, expected_width_mm));
    let connector_pad = connector_pad_ref.and_then(|(component_id, pin)| {
        suggested_usb_route_pad(bound, component_id, pin, net_name)
    });
    let protection_pad = protection_pad_ref.and_then(|(component_id, pin)| {
        suggested_usb_route_pad(bound, component_id, pin, net_name)
    });
    let connector_pad_to_route_distance_mm = connector_pad
        .as_ref()
        .and_then(|pad| pad_to_route_distance_mm(route, pad));
    let protection_pad_to_route_distance_mm = protection_pad
        .as_ref()
        .and_then(|pad| pad_to_route_distance_mm(route, pad));
    let connector_to_protection_pad_route_distance_mm =
        connector_pad.as_ref().and_then(|connector_pad| {
            protection_pad.as_ref().and_then(|protection_pad| {
                route_distance_between_pads_mm(route, connector_pad, protection_pad)
            })
        });
    Some(SuggestedUsbRoute {
        signal: signal.to_string(),
        net: net_name.to_string(),
        route_length_mm: route_length_mm(route),
        via_count: route.vias.len(),
        expected_data_line_width_mm,
        measured_data_line_width_mm: width_evidence.map(|evidence| evidence.measured_width_mm),
        data_line_width_delta_mm: width_evidence.map(|evidence| evidence.width_delta_mm),
        expected_vbus_route_width_mm: None,
        measured_vbus_route_width_min_mm: None,
        protection_component,
        connector_pad,
        protection_pad,
        connector_pad_to_route_distance_mm,
        protection_pad_to_route_distance_mm,
        connector_to_protection_pad_route_distance_mm,
        unreferenced_route_length_mm: None,
        unreferenced_segments: None,
        filled_unreferenced_route_length_mm: None,
        filled_unreferenced_segments: None,
        filled_zone_edge_clearance_min_mm: None,
        filled_zone_edge_clearance_segments: None,
        ground_zone_contacts: None,
        filled_ground_zone_contacts: None,
    })
}

fn suggested_usb_vbus_route(
    bound: &BoundBoard<'_>,
    net_name: &str,
    protection_component: Option<String>,
    connector_pad_ref: Option<(&str, &str)>,
    protection_pad_ref: Option<(&str, &str)>,
) -> Option<SuggestedUsbRoute> {
    let route = bound.project.board.layout.routes.get(net_name)?;
    if route.segments.is_empty() {
        return None;
    }
    let expected_vbus_route_width_mm = bound
        .project
        .board
        .layout
        .constraints
        .net_rules
        .get(net_name)
        .and_then(|rule| rule.track_width_mm);
    let connector_pad = connector_pad_ref.and_then(|(component_id, pin)| {
        suggested_usb_route_pad(bound, component_id, pin, net_name)
    });
    let protection_pad = protection_pad_ref.and_then(|(component_id, pin)| {
        suggested_usb_route_pad(bound, component_id, pin, net_name)
    });
    let connector_pad_to_route_distance_mm = connector_pad
        .as_ref()
        .and_then(|pad| pad_to_route_distance_mm(route, pad));
    let protection_pad_to_route_distance_mm = protection_pad
        .as_ref()
        .and_then(|pad| pad_to_route_distance_mm(route, pad));
    let connector_to_protection_pad_route_distance_mm =
        connector_pad.as_ref().and_then(|connector_pad| {
            protection_pad.as_ref().and_then(|protection_pad| {
                route_distance_between_pads_mm(route, connector_pad, protection_pad)
            })
        });
    Some(SuggestedUsbRoute {
        signal: "VBUS".to_string(),
        net: net_name.to_string(),
        route_length_mm: route_length_mm(route),
        via_count: route.vias.len(),
        expected_data_line_width_mm: None,
        measured_data_line_width_mm: None,
        data_line_width_delta_mm: None,
        expected_vbus_route_width_mm,
        measured_vbus_route_width_min_mm: narrowest_route_width_mm(route),
        protection_component,
        connector_pad,
        protection_pad,
        connector_pad_to_route_distance_mm,
        protection_pad_to_route_distance_mm,
        connector_to_protection_pad_route_distance_mm,
        unreferenced_route_length_mm: None,
        unreferenced_segments: None,
        filled_unreferenced_route_length_mm: None,
        filled_unreferenced_segments: None,
        filled_zone_edge_clearance_min_mm: None,
        filled_zone_edge_clearance_segments: None,
        ground_zone_contacts: None,
        filled_ground_zone_contacts: None,
    })
}

fn suggested_usb_route_with_return_path(
    bound: &BoundBoard<'_>,
    signal: &str,
    net_name: &str,
    ground_zones: &[GroundZoneEvidence<'_>],
) -> Option<SuggestedUsbRoute> {
    let route = bound.project.board.layout.routes.get(net_name)?;
    if route.segments.is_empty() {
        return None;
    }
    let (unreferenced_route_length_mm, unreferenced_segments) =
        return_path_unreferenced_segments(route, ground_zones, GroundReferenceGeometry::Outline);
    let (filled_unreferenced_route_length_mm, filled_unreferenced_segments) =
        if ground_zones_have_filled_polygons(ground_zones) {
            let (length, segments) = return_path_unreferenced_segments(
                route,
                ground_zones,
                GroundReferenceGeometry::FilledPolygon,
            );
            (Some(length), Some(segments))
        } else {
            (None, None)
        };
    let (filled_zone_edge_clearance_min_mm, filled_zone_edge_clearance_segments) =
        if ground_zones_have_filled_polygons(ground_zones) {
            let segments = return_path_filled_zone_clearance_segments(route, ground_zones);
            let min_clearance = segments
                .iter()
                .filter_map(|segment| segment.filled_zone_edge_clearance_mm)
                .min_by(|left, right| left.total_cmp(right));
            (min_clearance, Some(segments))
        } else {
            (None, None)
        };
    let ground_zone_contacts =
        route_ground_zone_contacts(bound, route, ground_zones, GroundReferenceGeometry::Outline);
    let filled_ground_zone_contacts = if ground_zones_have_filled_polygons(ground_zones) {
        Some(route_ground_zone_contacts(
            bound,
            route,
            ground_zones,
            GroundReferenceGeometry::FilledPolygon,
        ))
    } else {
        None
    };
    Some(SuggestedUsbRoute {
        signal: signal.to_string(),
        net: net_name.to_string(),
        route_length_mm: route_length_mm(route),
        via_count: route.vias.len(),
        expected_data_line_width_mm: None,
        measured_data_line_width_mm: None,
        data_line_width_delta_mm: None,
        expected_vbus_route_width_mm: None,
        measured_vbus_route_width_min_mm: None,
        protection_component: None,
        connector_pad: None,
        protection_pad: None,
        connector_pad_to_route_distance_mm: None,
        protection_pad_to_route_distance_mm: None,
        connector_to_protection_pad_route_distance_mm: None,
        unreferenced_route_length_mm: Some(unreferenced_route_length_mm),
        unreferenced_segments: Some(unreferenced_segments),
        filled_unreferenced_route_length_mm,
        filled_unreferenced_segments,
        filled_zone_edge_clearance_min_mm,
        filled_zone_edge_clearance_segments,
        ground_zone_contacts: Some(ground_zone_contacts),
        filled_ground_zone_contacts,
    })
}

fn suggested_usb_route_pair(
    bound: &BoundBoard<'_>,
    dp_route: &SuggestedUsbRoute,
    dm_route: &SuggestedUsbRoute,
) -> Option<SuggestedUsbRoutePair> {
    if dp_route.signal != "D+" || dm_route.signal != "D-" {
        return None;
    }
    let expected_data_pair_gap_mm = min_rule_value(
        bound
            .project
            .board
            .layout
            .constraints
            .net_rules
            .get(&dp_route.net),
        bound
            .project
            .board
            .layout
            .constraints
            .net_rules
            .get(&dm_route.net),
        |rule| rule.diff_pair_gap_mm,
    );
    let gap_evidence = expected_data_pair_gap_mm.and_then(|expected_gap_mm| {
        let dp_route_geometry = bound.project.board.layout.routes.get(&dp_route.net)?;
        let dm_route_geometry = bound.project.board.layout.routes.get(&dm_route.net)?;
        pair_gap_delta(dp_route_geometry, dm_route_geometry, expected_gap_mm)
    });
    Some(SuggestedUsbRoutePair {
        dp_net: dp_route.net.clone(),
        dm_net: dm_route.net.clone(),
        dp_route_length_mm: dp_route.route_length_mm,
        dm_route_length_mm: dm_route.route_length_mm,
        data_pair_length_mismatch_mm: (dp_route.route_length_mm - dm_route.route_length_mm).abs(),
        dp_via_count: dp_route.via_count,
        dm_via_count: dm_route.via_count,
        data_pair_via_count_delta: dp_route.via_count.abs_diff(dm_route.via_count),
        expected_data_pair_gap_mm,
        measured_data_pair_gap_mm: gap_evidence.map(|evidence| evidence.measured_gap_mm),
        data_pair_gap_delta_mm: gap_evidence.map(|evidence| evidence.gap_delta_mm),
    })
}

fn route_length_mm(route: &NetRoute) -> f64 {
    route.segments.iter().map(segment_length_mm).sum()
}

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    let dx = segment.end.x_mm - segment.start.x_mm;
    let dy = segment.end.y_mm - segment.start.y_mm;
    dx.hypot(dy)
}

#[derive(Debug, Clone, Copy)]
struct RouteWidthDelta {
    measured_width_mm: f64,
    width_delta_mm: f64,
}

fn route_width_delta(route: &NetRoute, expected_width_mm: f64) -> Option<RouteWidthDelta> {
    route
        .segments
        .iter()
        .map(|segment| RouteWidthDelta {
            measured_width_mm: segment.width_mm,
            width_delta_mm: (segment.width_mm - expected_width_mm).abs(),
        })
        .max_by(|left, right| left.width_delta_mm.total_cmp(&right.width_delta_mm))
}

fn narrowest_route_width_mm(route: &NetRoute) -> Option<f64> {
    route
        .segments
        .iter()
        .map(|segment| segment.width_mm)
        .min_by(f64::total_cmp)
}

#[derive(Debug, Clone, Copy)]
struct PairGapDelta {
    measured_gap_mm: f64,
    gap_delta_mm: f64,
}

fn pair_gap_delta(
    dp_route: &NetRoute,
    dm_route: &NetRoute,
    expected_gap_mm: f64,
) -> Option<PairGapDelta> {
    let mut worst = None;
    for dp_segment in &dp_route.segments {
        for dm_segment in &dm_route.segments {
            let Some(measured_gap_mm) = parallel_overlap_gap_mm(dp_segment, dm_segment) else {
                continue;
            };
            let gap_delta_mm = (measured_gap_mm - expected_gap_mm).abs();
            let evidence = PairGapDelta {
                measured_gap_mm,
                gap_delta_mm,
            };
            if worst
                .as_ref()
                .is_none_or(|current: &PairGapDelta| gap_delta_mm > current.gap_delta_mm)
            {
                worst = Some(evidence);
            }
        }
    }
    worst
}

fn parallel_overlap_gap_mm(dp_segment: &RouteSegment, dm_segment: &RouteSegment) -> Option<f64> {
    let dp_dx = dp_segment.end.x_mm - dp_segment.start.x_mm;
    let dp_dy = dp_segment.end.y_mm - dp_segment.start.y_mm;
    let dm_dx = dm_segment.end.x_mm - dm_segment.start.x_mm;
    let dm_dy = dm_segment.end.y_mm - dm_segment.start.y_mm;
    let dp_len = dp_dx.hypot(dp_dy);
    let dm_len = dm_dx.hypot(dm_dy);
    if dp_len <= f64::EPSILON || dm_len <= f64::EPSILON {
        return None;
    }
    let dp_unit_x = dp_dx / dp_len;
    let dp_unit_y = dp_dy / dp_len;
    let dm_unit_x = dm_dx / dm_len;
    let dm_unit_y = dm_dy / dm_len;
    let cross = (dp_unit_x * dm_unit_y - dp_unit_y * dm_unit_x).abs();
    if cross > 1.0e-6 {
        return None;
    }
    let projection_a = (dm_segment.start.x_mm - dp_segment.start.x_mm) * dp_unit_x
        + (dm_segment.start.y_mm - dp_segment.start.y_mm) * dp_unit_y;
    let projection_b = (dm_segment.end.x_mm - dp_segment.start.x_mm) * dp_unit_x
        + (dm_segment.end.y_mm - dp_segment.start.y_mm) * dp_unit_y;
    let overlap_start = projection_a.min(projection_b).max(0.0);
    let overlap_end = projection_a.max(projection_b).min(dp_len);
    if overlap_end - overlap_start <= f64::EPSILON {
        return None;
    }
    let centerline_distance_mm = ((dm_segment.start.x_mm - dp_segment.start.x_mm) * dp_unit_y
        - (dm_segment.start.y_mm - dp_segment.start.y_mm) * dp_unit_x)
        .abs();
    Some(centerline_distance_mm - (dp_segment.width_mm + dm_segment.width_mm) / 2.0)
}

pub(super) fn suggested_usb_connector(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    connector: &UsbConnector,
) -> Option<SuggestedUsbConnector> {
    let dp_net = connected_declared_net(bound, component, &connector.dp_pin)?.to_string();
    let dm_net = connected_declared_net(bound, component, &connector.dm_pin)?.to_string();
    let gnd_net = connected_declared_net(bound, component, &connector.gnd_pin)?.to_string();
    let vbus_net =
        connected_declared_net(bound, component, &connector.vbus_pin).map(str::to_string);
    let shield_net = connector
        .shield_pin
        .as_deref()
        .and_then(|pin| connected_declared_net(bound, component, pin))
        .map(str::to_string);
    let entry_direction = component_placement(bound, component_id)
        .and_then(|placement| usb_entry_direction(bound, component_id, placement, connector));
    Some(SuggestedUsbConnector {
        component: component_id.to_string(),
        standard: connector.standard.clone(),
        vbus_pin: connector.vbus_pin.clone(),
        vbus_net,
        dp_pin: connector.dp_pin.clone(),
        dp_net,
        dm_pin: connector.dm_pin.clone(),
        dm_net,
        gnd_pin: connector.gnd_pin.clone(),
        gnd_net,
        shield_pin: connector.shield_pin.clone(),
        shield_net,
        placement: suggested_placement(bound, component_id),
        footprint: suggested_footprint(bound, component_id),
        nearest_board_edge: nearest_board_edge_evidence(bound, component_id, connector),
        nearest_component_clearance: nearest_component_clearance_evidence(bound, component_id),
        entry_clearance: entry_direction.and_then(|entry_direction| {
            entry_clearance_evidence(
                bound,
                component_id,
                connector,
                entry_direction.deg,
                entry_direction.source,
                entry_direction.offset_deg,
            )
        }),
    })
}

pub(super) struct UsbEntryDirection {
    pub deg: f64,
    pub source: &'static str,
    pub offset_deg: Option<f64>,
}

pub(super) fn usb_entry_direction(
    bound: &BoundBoard<'_>,
    component_id: &str,
    placement: &ComponentPlacement,
    connector: &UsbConnector,
) -> Option<UsbEntryDirection> {
    let rotation_deg = placement.rotation_deg?;
    let layout_offset_deg = bound
        .project
        .board
        .layout
        .footprints
        .get(component_id)
        .and_then(|footprint| footprint.entry_direction.as_ref())
        .and_then(|entry_direction| entry_direction.offset_deg);
    let offset_deg = layout_offset_deg.or(connector.entry_direction_offset_deg);
    let deg = normalize_rotation_deg(rotation_deg + offset_deg.unwrap_or(0.0));
    deg.is_finite().then_some(UsbEntryDirection {
        deg,
        source: if layout_offset_deg.is_some() {
            usb_entry_direction_source(bound, component_id)
        } else if offset_deg.is_some() {
            "component_model_offset"
        } else {
            "placement_rotation"
        },
        offset_deg,
    })
}

fn usb_entry_direction_source(bound: &BoundBoard<'_>, component_id: &str) -> &'static str {
    if bound
        .project
        .board
        .layout
        .footprints
        .get(component_id)
        .and_then(|footprint| footprint.entry_direction.as_ref())
        .and_then(|entry_direction| entry_direction.source.as_deref())
        == Some("kicad_footprint_property")
    {
        "footprint_property_offset"
    } else {
        "kicad_mapping_offset"
    }
}

fn normalize_rotation_deg(rotation_deg: f64) -> f64 {
    rotation_deg.rem_euclid(360.0)
}

fn connected_declared_net<'a>(
    bound: &BoundBoard<'_>,
    component: &'a ComponentSpec,
    pin: &str,
) -> Option<&'a str> {
    let net = component.pins.get(pin)?;
    bound.project.board.nets.get(net)?;
    Some(net)
}

fn max_nominal_voltage<'a>(
    bound: &BoundBoard<'_>,
    nets: impl IntoIterator<Item = &'a str>,
) -> Option<f64> {
    nets.into_iter()
        .filter_map(|net| bound.project.board.nets.get(net)?.nominal_voltage)
        .filter(|voltage| voltage.is_finite())
        .reduce(f64::max)
}

fn suggested_valid_protection_clamp_for_net(
    bound: &BoundBoard<'_>,
    connector_id: &str,
    net_name: &str,
) -> Option<SuggestedProtectionClamp> {
    for (component_id, component) in &bound.project.board.components {
        if component_id == connector_id {
            continue;
        }
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        for clamp in &model.signal_conditioning.protection_clamps {
            let Some(protected_net) = component.pins.get(&clamp.protected_pin) else {
                continue;
            };
            if protected_net != net_name {
                continue;
            }
            let Some(reference_net_name) = component.pins.get(&clamp.reference_pin) else {
                continue;
            };
            let Some(reference_net) = bound.project.board.nets.get(reference_net_name) else {
                continue;
            };
            let expected_kind = match clamp.reference {
                ProtectionReference::Ground => NetKind::Ground,
                ProtectionReference::Power => NetKind::Power,
            };
            if reference_net.kind == expected_kind {
                return suggested_protection_clamp(bound, component_id, component, model, clamp);
            }
        }
    }
    None
}

fn nearest_placed_protection_clamp_for_net(
    bound: &BoundBoard<'_>,
    connector_id: &str,
    net_name: &str,
    connector_placement: &ComponentPlacement,
) -> Option<SuggestedProtectionClamp> {
    let mut nearest: Option<(SuggestedProtectionClamp, f64)> = None;
    for (component_id, component) in &bound.project.board.components {
        if component_id == connector_id {
            continue;
        }
        let Some(protection_placement) = component_placement(bound, component_id) else {
            continue;
        };
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        for clamp in &model.signal_conditioning.protection_clamps {
            let Some(protected_net) = component.pins.get(&clamp.protected_pin) else {
                continue;
            };
            if protected_net != net_name {
                continue;
            }
            let Some(reference_net_name) = component.pins.get(&clamp.reference_pin) else {
                continue;
            };
            let Some(reference_net) = bound.project.board.nets.get(reference_net_name) else {
                continue;
            };
            let expected_kind = match clamp.reference {
                ProtectionReference::Ground => NetKind::Ground,
                ProtectionReference::Power => NetKind::Power,
            };
            if reference_net.kind != expected_kind {
                continue;
            }
            let distance_mm = placement_distance_mm(connector_placement, protection_placement);
            let Some(mut suggested) =
                suggested_protection_clamp(bound, component_id, component, model, clamp)
            else {
                continue;
            };
            suggested.distance_to_target_mm = Some(distance_mm);
            if nearest
                .as_ref()
                .is_none_or(|(_, nearest_distance)| distance_mm < *nearest_distance)
            {
                nearest = Some((suggested, distance_mm));
            }
        }
    }
    nearest.map(|(suggested, _)| suggested)
}

pub(super) fn existing_usb_connector_protection_checks(project: &BoardProject) -> BTreeSet<String> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "interface_protection"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == USB_CONNECTOR_PROTECTION_VALID)
        })
        .filter_map(|scenario| {
            scenario
                .target
                .as_ref()
                .map(|target| target.component.clone())
        })
        .collect()
}

pub(super) fn existing_usb_protection_placement_checks(project: &BoardProject) -> BTreeSet<String> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "interface_protection"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == USB_PROTECTION_PLACEMENT_VALID)
        })
        .filter_map(|scenario| {
            scenario
                .target
                .as_ref()
                .map(|target| target.component.clone())
        })
        .collect()
}

pub(super) fn existing_usb_route_geometry_checks(project: &BoardProject) -> BTreeSet<String> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "interface_protection"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == USB_ROUTE_GEOMETRY_VALID)
        })
        .filter_map(|scenario| {
            scenario
                .target
                .as_ref()
                .map(|target| target.component.clone())
        })
        .collect()
}

pub(super) fn existing_usb_vbus_route_checks(project: &BoardProject) -> BTreeSet<String> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "interface_protection"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == USB_VBUS_ROUTE_VALID)
        })
        .filter_map(|scenario| {
            scenario
                .target
                .as_ref()
                .map(|target| target.component.clone())
        })
        .collect()
}

pub(super) fn existing_usb_return_path_checks(project: &BoardProject) -> BTreeSet<String> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "interface_protection"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == USB_RETURN_PATH_VALID)
        })
        .filter_map(|scenario| {
            scenario
                .target
                .as_ref()
                .map(|target| target.component.clone())
        })
        .collect()
}
