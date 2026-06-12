use super::{
    INTERFACE_PROTECTION_REVIEW, ScenarioSuggestion, SuggestedConditioning,
    SuggestedConditioningSide, SuggestedPlacement, SuggestedProtectionClamp, SuggestedScenario,
    SuggestedTarget, SuggestedUsbConnector, SuggestedUsbRoute, SuggestedUsbRoutePair,
    USB_CONNECTOR_PROTECTION_VALID, USB_PROTECTION_PLACEMENT_VALID, USB_ROUTE_GEOMETRY_VALID,
    sanitized_name,
};
use crate::board_ir::{
    BoardProject, ComponentPlacement, ComponentSpec, NetKind, NetLayoutRule, NetRoute,
    PlacementSide, RouteSegment,
};
use crate::library::{
    BoundBoard, ComponentModel, ProtectionClamp, ProtectionReference, SignalConditioningChannel,
    SignalConditioningKind, UsbConnector,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn interface_protection_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_interface_protection_checks(bound.project);
    let existing_usb_connectors = existing_usb_connector_protection_checks(bound.project);
    let existing_usb_placements = existing_usb_protection_placement_checks(bound.project);
    let existing_usb_routes = existing_usb_route_geometry_checks(bound.project);
    let mut suggestions = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        for channel in &model.signal_conditioning.channels {
            if existing.contains_key(&(
                component_id.clone(),
                "channel".to_string(),
                channel.name.clone(),
            )) {
                continue;
            }
            let Some(conditioning) = suggested_conditioning(bound, component_id, model, channel)
            else {
                continue;
            };
            suggestions.push(ScenarioSuggestion {
                id: format!(
                    "interface_protection_{}_{}",
                    sanitized_name(component_id),
                    sanitized_name(&channel.name)
                ),
                kind: "interface_protection".to_string(),
                confidence: "medium".to_string(),
                runnable: false,
                reason: format!(
                    "Component {component_id} model declares signal-conditioning channel {}, but no interface protection review scenario covers it.",
                    channel.name
                ),
                scenario: SuggestedScenario {
                    name: format!(
                        "{}_{}_interface_protection",
                        sanitized_name(component_id),
                        sanitized_name(&channel.name)
                    ),
                    scenario_type: "interface_protection".to_string(),
                    checks: vec![INTERFACE_PROTECTION_REVIEW.to_string()],
                    parameters: Some(BTreeMap::from([(
                        "channel".to_string(),
                        serde_json::Value::String(channel.name.clone()),
                    )])),
                    target: Some(SuggestedTarget {
                        component: component_id.clone(),
                        power_pin: None,
                        reset_pin: None,
                    }),
                    timing: None,
                    required_boot_mode: None,
                    straps: Vec::new(),
                    bootloader: None,
                    events: Vec::new(),
                    conditioning: Some(conditioning),
                    protection_clamps: Vec::new(),
                    usb_connectors: Vec::new(),
                    usb_routes: Vec::new(),
                    usb_route_pairs: Vec::new(),
                    clocks: Vec::new(),
                    reset_supervisors: Vec::new(),
                    regulators: Vec::new(),
                    pin_states: Vec::new(),
                    paths: Vec::new(),
                },
                required_inputs: vec![
                    "Confirm the signal-conditioning part datasheet supports this direction, voltage range, and unpowered-side behavior.".to_string(),
                    "Fill enable/OE/reset-state evidence when the part can disconnect or leave either side high impedance.".to_string(),
                    "Add analog_transient or GPIO_BACKDRIVE scenarios for any datasheet condition that does not guarantee isolation.".to_string(),
                ],
            });
        }
        for clamp in &model.signal_conditioning.protection_clamps {
            if existing.contains_key(&(
                component_id.clone(),
                "clamp".to_string(),
                clamp.name.clone(),
            )) {
                continue;
            }
            let Some(protection_clamp) =
                suggested_protection_clamp(bound, component_id, component, model, clamp)
            else {
                continue;
            };
            suggestions.push(ScenarioSuggestion {
                id: format!(
                    "interface_protection_{}_{}",
                    sanitized_name(component_id),
                    sanitized_name(&clamp.name)
                ),
                kind: "interface_protection".to_string(),
                confidence: "medium".to_string(),
                runnable: true,
                reason: format!(
                    "Component {component_id} model declares protection clamp {}, but no interface protection review scenario covers it.",
                    clamp.name
                ),
                scenario: SuggestedScenario {
                    name: format!(
                        "{}_{}_interface_protection",
                        sanitized_name(component_id),
                        sanitized_name(&clamp.name)
                    ),
                    scenario_type: "interface_protection".to_string(),
                    checks: vec![INTERFACE_PROTECTION_REVIEW.to_string()],
                    parameters: Some(BTreeMap::from([(
                        "clamp".to_string(),
                        serde_json::Value::String(clamp.name.clone()),
                    )])),
                    target: Some(SuggestedTarget {
                        component: component_id.clone(),
                        power_pin: None,
                        reset_pin: None,
                    }),
                    timing: None,
                    required_boot_mode: None,
                    straps: Vec::new(),
                    bootloader: None,
                    events: Vec::new(),
                    conditioning: None,
                    protection_clamps: vec![protection_clamp],
                    usb_connectors: Vec::new(),
                    usb_routes: Vec::new(),
                    usb_route_pairs: Vec::new(),
                    clocks: Vec::new(),
                    reset_supervisors: Vec::new(),
                    regulators: Vec::new(),
                    pin_states: Vec::new(),
                    paths: Vec::new(),
                },
                required_inputs: vec![
                    "Fill parameters.max_line_capacitance_F from the real interface capacitance budget when capacitance screening is required; do not use the clamp's own capacitance as the budget unless that is the actual design limit.".to_string(),
                    "Use layout, signal-integrity, and ESD-pulse validation for USB eye margin, return path, and IEC stress sign-off.".to_string(),
                ],
            });
        }
        if model.usb_connector.is_some()
            && !existing_usb_connectors.contains(component_id)
            && let Some(suggestion) =
                usb_connector_protection_suggestion(bound, component_id, component, model)
        {
            suggestions.push(suggestion);
        }
        if model.usb_connector.is_some()
            && !existing_usb_placements.contains(component_id)
            && let Some(suggestion) =
                usb_protection_placement_suggestion(bound, component_id, component, model)
        {
            suggestions.push(suggestion);
        }
        if model.usb_connector.is_some()
            && !existing_usb_routes.contains(component_id)
            && let Some(suggestion) =
                usb_route_geometry_suggestion(bound, component_id, component, model)
        {
            suggestions.push(suggestion);
        }
    }
    suggestions
}

fn usb_connector_protection_suggestion(
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
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "require_vbus_protection".to_string(),
        serde_json::Value::Bool(require_vbus_protection),
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

fn usb_protection_placement_suggestion(
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
    let parameters = BTreeMap::from([
        (
            "require_vbus_protection".to_string(),
            serde_json::Value::Bool(require_vbus_protection),
        ),
        (
            "max_connector_to_protection_distance_mm".to_string(),
            serde_json::Value::Null,
        ),
    ]);
    Some(ScenarioSuggestion {
        id: format!("usb_protection_placement_{}", sanitized_name(component_id)),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable: false,
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
        required_inputs: vec![
            "Fill parameters.max_connector_to_protection_distance_mm from the board's ESD/layout rule or datasheet/layout guidance; do not invent the limit from component coordinates.".to_string(),
            "Use PCB/layout review for routed trace order, via count, return path, shield strategy, and USB differential-pair constraints.".to_string(),
        ],
    })
}

fn usb_route_geometry_suggestion(
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
    )?;
    let dm_route = suggested_usb_route(
        bound,
        "D-",
        &suggested_connector.dm_net,
        Some(dm_clamp.component.clone()),
    )?;
    let route_pair = suggested_usb_route_pair(bound, &dp_route, &dm_route)?;
    let route_limits = suggested_usb_route_limits(bound, &dp_route.net, &dm_route.net);
    let parameters = BTreeMap::from([
        (
            "max_data_line_route_length_mm".to_string(),
            optional_number_value(route_limits.max_data_line_route_length_mm),
        ),
        (
            "max_data_line_via_count".to_string(),
            serde_json::Value::Null,
        ),
        (
            "max_data_line_width_delta_mm".to_string(),
            serde_json::Value::Null,
        ),
        (
            "max_connector_to_protection_route_distance_mm".to_string(),
            serde_json::Value::Null,
        ),
        (
            "max_component_to_route_distance_mm".to_string(),
            serde_json::Value::Null,
        ),
        (
            "max_data_pair_length_mismatch_mm".to_string(),
            optional_number_value(route_limits.max_data_pair_length_mismatch_mm),
        ),
        (
            "max_data_pair_via_count_delta".to_string(),
            serde_json::Value::Null,
        ),
        (
            "max_data_pair_gap_delta_mm".to_string(),
            serde_json::Value::Null,
        ),
    ]);
    Some(ScenarioSuggestion {
        id: format!("usb_route_geometry_{}", sanitized_name(component_id)),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable: false,
        reason: format!(
            "USB connector {component_id} has placed protection components and imported D+/D- route geometry; add route-length, via-count, and connector-to-protection route checks."
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
        required_inputs: vec![
            route_limit_required_input(
                route_limits.max_data_line_route_length_mm,
                "max_data_line_route_length_mm",
                "Fill max_data_line_route_length_mm from the board's USB layout rule or signal-integrity budget.",
            ),
            "Fill max_data_line_via_count from the board's USB routing policy; use zero when layer changes are not allowed.".to_string(),
            "Fill max_data_line_width_delta_mm when imported route width constraints should be enforced.".to_string(),
            route_limit_required_input(
                route_limits.max_data_pair_length_mismatch_mm,
                "max_data_pair_length_mismatch_mm",
                "Fill max_data_pair_length_mismatch_mm from the USB differential-pair matching rule.",
            ),
            "Fill max_data_pair_via_count_delta from the USB differential-pair matching rule.".to_string(),
            "Fill max_data_pair_gap_delta_mm when imported differential-pair gap constraints should be enforced.".to_string(),
            "Fill max_connector_to_protection_route_distance_mm and max_component_to_route_distance_mm from ESD/layout guidance before treating the route template as sign-off.".to_string(),
        ],
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SuggestedUsbRouteLimits {
    max_data_line_route_length_mm: Option<f64>,
    max_data_pair_length_mismatch_mm: Option<f64>,
}

fn suggested_usb_route_limits(
    bound: &BoundBoard<'_>,
    dp_net: &str,
    dm_net: &str,
) -> SuggestedUsbRouteLimits {
    let dp_rule = bound.project.board.layout.constraints.net_rules.get(dp_net);
    let dm_rule = bound.project.board.layout.constraints.net_rules.get(dm_net);
    SuggestedUsbRouteLimits {
        max_data_line_route_length_mm: min_rule_value(dp_rule, dm_rule, |rule| rule.length_max_mm),
        max_data_pair_length_mismatch_mm: min_rule_value(dp_rule, dm_rule, |rule| rule.skew_max_mm),
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
    Some(SuggestedUsbRoute {
        signal: signal.to_string(),
        net: net_name.to_string(),
        route_length_mm: route_length_mm(route),
        via_count: route.vias.len(),
        expected_data_line_width_mm,
        measured_data_line_width_mm: width_evidence.map(|evidence| evidence.measured_width_mm),
        data_line_width_delta_mm: width_evidence.map(|evidence| evidence.width_delta_mm),
        protection_component,
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
    route
        .segments
        .iter()
        .map(|segment| {
            let dx = segment.end.x_mm - segment.start.x_mm;
            let dy = segment.end.y_mm - segment.start.y_mm;
            dx.hypot(dy)
        })
        .sum()
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

fn suggested_usb_connector(
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
    })
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

fn suggested_conditioning(
    bound: &BoundBoard<'_>,
    component_id: &str,
    model: &ComponentModel,
    channel: &SignalConditioningChannel,
) -> Option<SuggestedConditioning> {
    let side_a = suggested_conditioning_side(
        bound,
        component_id,
        &channel.side_a_pin,
        channel.side_a_supply_pin.as_deref(),
    )?;
    let side_b = suggested_conditioning_side(
        bound,
        component_id,
        &channel.side_b_pin,
        channel.side_b_supply_pin.as_deref(),
    )?;
    if !model.ports.contains_key(&channel.side_a_pin)
        || !model.ports.contains_key(&channel.side_b_pin)
    {
        return None;
    }
    Some(SuggestedConditioning {
        component: component_id.to_string(),
        channel: channel.name.clone(),
        kind: signal_conditioning_kind_name(&channel.kind).to_string(),
        side_a,
        side_b,
        direction: channel.direction.clone(),
        unpowered_isolation: channel.unpowered_isolation,
    })
}

fn suggested_protection_clamp(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    clamp: &ProtectionClamp,
) -> Option<SuggestedProtectionClamp> {
    if !model.ports.contains_key(&clamp.protected_pin)
        || !model.ports.contains_key(&clamp.reference_pin)
    {
        return None;
    }
    let protected_net = component.pins.get(&clamp.protected_pin)?.clone();
    let reference_net = component.pins.get(&clamp.reference_pin)?.clone();
    bound.project.board.nets.get(&protected_net)?;
    bound.project.board.nets.get(&reference_net)?;
    let reference = match clamp.reference {
        ProtectionReference::Ground => "ground",
        ProtectionReference::Power => "power",
    };
    Some(SuggestedProtectionClamp {
        component: component_id.to_string(),
        clamp: clamp.name.clone(),
        protected_pin: clamp.protected_pin.clone(),
        protected_net,
        reference_pin: clamp.reference_pin.clone(),
        reference_net,
        reference: reference.to_string(),
        working_voltage_max_v: clamp.working_voltage_max_v,
        line_capacitance_f: clamp.line_capacitance_f,
        placement: suggested_placement(bound, component_id),
        distance_to_target_mm: None,
    })
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

fn component_placement<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &str,
) -> Option<&'a ComponentPlacement> {
    let placement = bound.project.board.layout.placements.get(component_id)?;
    if placement.x_mm.is_finite() && placement.y_mm.is_finite() {
        Some(placement)
    } else {
        None
    }
}

fn suggested_placement(bound: &BoundBoard<'_>, component_id: &str) -> Option<SuggestedPlacement> {
    let placement = component_placement(bound, component_id)?;
    Some(SuggestedPlacement {
        x_mm: placement.x_mm,
        y_mm: placement.y_mm,
        side: placement_side_name(&placement.side).map(str::to_string),
    })
}

fn placement_distance_mm(a: &ComponentPlacement, b: &ComponentPlacement) -> f64 {
    let dx = a.x_mm - b.x_mm;
    let dy = a.y_mm - b.y_mm;
    (dx.mul_add(dx, dy * dy)).sqrt()
}

fn placement_side_name(side: &Option<PlacementSide>) -> Option<&'static str> {
    match side {
        Some(PlacementSide::Top) => Some("top"),
        Some(PlacementSide::Bottom) => Some("bottom"),
        None => None,
    }
}

fn signal_conditioning_kind_name(kind: &SignalConditioningKind) -> &'static str {
    match kind {
        SignalConditioningKind::LevelShifter => "level_shifter",
        SignalConditioningKind::Protection => "protection",
        SignalConditioningKind::SeriesResistor => "series_resistor",
        SignalConditioningKind::BusSwitch => "bus_switch",
    }
}

fn suggested_conditioning_side(
    bound: &BoundBoard<'_>,
    component_id: &str,
    pin: &str,
    supply_pin: Option<&str>,
) -> Option<SuggestedConditioningSide> {
    let component = bound.project.board.components.get(component_id)?;
    let net = component.pins.get(pin)?.clone();
    let supply_net = supply_pin
        .and_then(|pin_name| {
            component
                .power_domains
                .get(pin_name)
                .or_else(|| component.pins.get(pin_name))
        })
        .cloned();
    Some(SuggestedConditioningSide {
        pin: pin.to_string(),
        net,
        supply_pin: supply_pin.map(str::to_string),
        supply_net,
    })
}

fn existing_interface_protection_checks(
    project: &BoardProject,
) -> BTreeMap<(String, String, String), ()> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "interface_protection"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == INTERFACE_PROTECTION_REVIEW)
        })
        .filter_map(|scenario| {
            let target = scenario.target.as_ref()?;
            if let Some(channel) = scenario
                .parameters
                .get("channel")
                .and_then(serde_yaml_ng::Value::as_str)
            {
                return Some((
                    (
                        target.component.clone(),
                        "channel".to_string(),
                        channel.to_string(),
                    ),
                    (),
                ));
            }
            let clamp = scenario
                .parameters
                .get("clamp")
                .and_then(serde_yaml_ng::Value::as_str)?;
            Some((
                (
                    target.component.clone(),
                    "clamp".to_string(),
                    clamp.to_string(),
                ),
                (),
            ))
        })
        .collect()
}

fn existing_usb_connector_protection_checks(project: &BoardProject) -> BTreeSet<String> {
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

fn existing_usb_protection_placement_checks(project: &BoardProject) -> BTreeSet<String> {
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

fn existing_usb_route_geometry_checks(project: &BoardProject) -> BTreeSet<String> {
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
