use super::{
    INTERFACE_PROTECTION_REVIEW, ScenarioSuggestion, SuggestedConditioning,
    SuggestedConditioningSide, SuggestedPlacement, SuggestedProtectionClamp, SuggestedScenario,
    SuggestedTarget, sanitized_name,
};
use crate::board_ir::{BoardProject, ComponentPlacement, ComponentSpec, PlacementSide};
use crate::library::{
    BoundBoard, ComponentModel, ProtectionClamp, ProtectionReference, SignalConditioningChannel,
    SignalConditioningKind,
};
use std::collections::BTreeMap;

mod usb;

pub(super) fn interface_protection_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_interface_protection_checks(bound.project);
    let existing_usb_connectors = usb::existing_usb_connector_protection_checks(bound.project);
    let existing_usb_placements = usb::existing_usb_protection_placement_checks(bound.project);
    let existing_usb_orientations = usb::existing_usb_connector_orientation_checks(bound.project);
    let existing_usb_edge_proximity =
        usb::existing_usb_connector_edge_proximity_checks(bound.project);
    let existing_usb_routes = usb::existing_usb_route_geometry_checks(bound.project);
    let existing_usb_vbus_routes = usb::existing_usb_vbus_route_checks(bound.project);
    let existing_usb_return_paths = usb::existing_usb_return_path_checks(bound.project);
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
                usb::usb_connector_protection_suggestion(bound, component_id, component, model)
        {
            suggestions.push(suggestion);
        }
        if model.usb_connector.is_some()
            && !existing_usb_placements.contains(component_id)
            && let Some(suggestion) =
                usb::usb_protection_placement_suggestion(bound, component_id, component, model)
        {
            suggestions.push(suggestion);
        }
        if model.usb_connector.is_some()
            && !existing_usb_orientations.contains(component_id)
            && let Some(suggestion) =
                usb::usb_connector_orientation_suggestion(bound, component_id, component, model)
        {
            suggestions.push(suggestion);
        }
        if model.usb_connector.is_some()
            && !existing_usb_edge_proximity.contains(component_id)
            && let Some(suggestion) =
                usb::usb_connector_edge_proximity_suggestion(bound, component_id, component, model)
        {
            suggestions.push(suggestion);
        }
        if model.usb_connector.is_some()
            && !existing_usb_routes.contains(component_id)
            && let Some(suggestion) =
                usb::usb_route_geometry_suggestion(bound, component_id, component, model)
        {
            suggestions.push(suggestion);
        }
        if model.usb_connector.is_some()
            && !existing_usb_vbus_routes.contains(component_id)
            && let Some(suggestion) =
                usb::usb_vbus_route_suggestion(bound, component_id, component, model)
        {
            suggestions.push(suggestion);
        }
        if model.usb_connector.is_some()
            && !existing_usb_return_paths.contains(component_id)
            && let Some(suggestion) =
                usb::usb_return_path_suggestion(bound, component_id, component, model)
        {
            suggestions.push(suggestion);
        }
    }
    suggestions
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

pub(super) fn suggested_protection_clamp(
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

pub(super) fn component_placement<'a>(
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

pub(super) fn suggested_placement(
    bound: &BoundBoard<'_>,
    component_id: &str,
) -> Option<SuggestedPlacement> {
    let placement = component_placement(bound, component_id)?;
    Some(SuggestedPlacement {
        x_mm: placement.x_mm,
        y_mm: placement.y_mm,
        side: placement_side_name(&placement.side).map(str::to_string),
        rotation_deg: placement.rotation_deg,
    })
}

pub(super) fn placement_distance_mm(a: &ComponentPlacement, b: &ComponentPlacement) -> f64 {
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
