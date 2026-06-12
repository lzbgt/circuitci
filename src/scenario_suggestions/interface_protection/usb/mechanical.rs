use super::super::super::{
    ScenarioSuggestion, SuggestedScenario, SuggestedTarget, USB_CONNECTOR_BODY_OVERHANG_VALID,
    USB_CONNECTOR_COMPONENT_CLEARANCE_VALID, USB_CONNECTOR_EDGE_PROXIMITY_VALID,
    USB_CONNECTOR_ENTRY_CLEARANCE_VALID, USB_CONNECTOR_ORIENTATION_VALID,
};
use super::super::{component_placement, sanitized_name};
use super::{suggested_usb_connector, usb_entry_direction};
use crate::board_ir::{BoardProject, ComponentSpec};
use crate::library::{BoundBoard, ComponentModel};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::scenario_suggestions::interface_protection) fn usb_connector_orientation_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<ScenarioSuggestion> {
    let connector = model.usb_connector.as_ref()?;
    let suggested_connector = suggested_usb_connector(bound, component_id, component, connector)?;
    let placement = component_placement(bound, component_id)?;
    placement.rotation_deg?;
    let expected_rotation = suggested_connector
        .nearest_board_edge
        .as_ref()
        .and_then(|edge| edge.expected_connector_rotation_deg);
    let mut parameters = BTreeMap::from([(
        "max_connector_rotation_error_deg".to_string(),
        serde_json::Value::Null,
    )]);
    parameters.insert(
        "expected_connector_rotation_deg".to_string(),
        expected_rotation
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
    );
    let mut required_inputs = vec![
        "Fill max_connector_rotation_error_deg from the allowed footprint-orientation tolerance."
            .to_string(),
    ];
    if expected_rotation.is_some() {
        let offset = suggested_connector
            .nearest_board_edge
            .as_ref()
            .and_then(|edge| edge.connector_entry_direction_offset_deg);
        let offset_source = suggested_connector
            .nearest_board_edge
            .as_ref()
            .and_then(|edge| edge.connector_entry_direction_offset_source.as_deref());
        required_inputs.push(offset.map_or_else(
            || "Review the inferred expected_connector_rotation_deg from nearest board-edge outward-normal evidence before making this scenario runnable.".to_string(),
            |offset_deg| {
                let source = match offset_source {
                    Some("footprint_property") => "KiCad footprint entry-direction property",
                    Some("kicad_mapping") => "KiCad mapping entry-direction offset",
                    _ => "component-model usb_connector.entry_direction_offset_deg",
                };
                format!(
                    "Review the inferred expected_connector_rotation_deg from nearest board-edge outward-normal evidence minus {source} {:.3} before making this scenario runnable.",
                    offset_deg
                )
            },
        ));
    } else {
        required_inputs.push(
            "Fill expected_connector_rotation_deg from the board-edge or enclosure-entry mechanical rule.".to_string(),
        );
    }
    Some(ScenarioSuggestion {
        id: format!("usb_connector_orientation_{}", sanitized_name(component_id)),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable: false,
        reason: format!(
            "USB connector {component_id} has imported placement rotation evidence; add a connector-orientation scenario from the mechanical board-edge rule."
        ),
        scenario: SuggestedScenario {
            name: format!("{}_usb_connector_orientation", sanitized_name(component_id)),
            scenario_type: "interface_protection".to_string(),
            checks: vec![USB_CONNECTOR_ORIENTATION_VALID.to_string()],
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
            protection_clamps: Vec::new(),
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

pub(in crate::scenario_suggestions::interface_protection) fn usb_connector_edge_proximity_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<ScenarioSuggestion> {
    let connector = model.usb_connector.as_ref()?;
    let suggested_connector = suggested_usb_connector(bound, component_id, component, connector)?;
    suggested_connector.nearest_board_edge.as_ref()?;
    let parameters = BTreeMap::from([(
        "max_connector_to_board_edge_distance_mm".to_string(),
        serde_json::Value::Null,
    )]);
    Some(ScenarioSuggestion {
        id: format!("usb_connector_edge_proximity_{}", sanitized_name(component_id)),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable: false,
        reason: format!(
            "USB connector {component_id} has imported placement and board-edge outline evidence; add a connector-to-board-edge proximity scenario."
        ),
        scenario: SuggestedScenario {
            name: format!("{}_usb_connector_edge_proximity", sanitized_name(component_id)),
            scenario_type: "interface_protection".to_string(),
            checks: vec![USB_CONNECTOR_EDGE_PROXIMITY_VALID.to_string()],
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
            protection_clamps: Vec::new(),
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
            "Fill max_connector_to_board_edge_distance_mm from the connector/enclosure mechanical rule before making this scenario runnable.".to_string(),
            "Review the nearest_board_edge evidence; sampled Edge.Cuts segments retain source provenance but approximate exact curve geometry, cutouts, panel tabs, and connector body intrusion.".to_string(),
        ],
    })
}

pub(in crate::scenario_suggestions::interface_protection) fn usb_connector_body_overhang_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<ScenarioSuggestion> {
    let connector = model.usb_connector.as_ref()?;
    let suggested_connector = suggested_usb_connector(bound, component_id, component, connector)?;
    suggested_connector
        .nearest_board_edge
        .as_ref()?
        .connector_body_overhang_mm?;
    let parameters = BTreeMap::from([(
        "max_connector_body_overhang_mm".to_string(),
        serde_json::Value::Null,
    )]);
    Some(ScenarioSuggestion {
        id: format!("usb_connector_body_overhang_{}", sanitized_name(component_id)),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable: false,
        reason: format!(
            "USB connector {component_id} has imported board-edge and mechanical footprint evidence; add a connector-body overhang scenario."
        ),
        scenario: SuggestedScenario {
            name: format!("{}_usb_connector_body_overhang", sanitized_name(component_id)),
            scenario_type: "interface_protection".to_string(),
            checks: vec![USB_CONNECTOR_BODY_OVERHANG_VALID.to_string()],
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
            protection_clamps: Vec::new(),
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
            "Fill max_connector_body_overhang_mm from connector, enclosure, and assembly mechanical constraints before making this scenario runnable.".to_string(),
            "Review the footprint body/courtyard evidence; imported straight lines, rectangles, and polygons do not model arcs, shell volume, panel cutouts, or enclosure interference.".to_string(),
        ],
    })
}

pub(in crate::scenario_suggestions::interface_protection) fn usb_connector_component_clearance_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<ScenarioSuggestion> {
    let connector = model.usb_connector.as_ref()?;
    let suggested_connector = suggested_usb_connector(bound, component_id, component, connector)?;
    suggested_connector.footprint.as_ref()?;
    let nearest_clearance = suggested_connector.nearest_component_clearance.as_ref()?;
    let nearest_clearance_mm = nearest_clearance.clearance_mm;
    let nearest_clearance_component = nearest_clearance.component.clone();
    let parameters = BTreeMap::from([(
        "min_connector_to_component_clearance_mm".to_string(),
        serde_json::Value::Null,
    )]);
    Some(ScenarioSuggestion {
        id: format!(
            "usb_connector_component_clearance_{}",
            sanitized_name(component_id)
        ),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable: false,
        reason: format!(
            "USB connector {component_id} has measured clearance {:.3} mm to nearby component {}; add a connector keepout/component-clearance scenario.",
            nearest_clearance_mm,
            nearest_clearance_component
        ),
        scenario: SuggestedScenario {
            name: format!(
                "{}_usb_connector_component_clearance",
                sanitized_name(component_id)
            ),
            scenario_type: "interface_protection".to_string(),
            checks: vec![USB_CONNECTOR_COMPONENT_CLEARANCE_VALID.to_string()],
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
            protection_clamps: Vec::new(),
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
            format!(
                "Fill min_connector_to_component_clearance_mm from the connector courtyard, cable insertion, enclosure, or assembly keepout rule after reviewing measured nearest-component clearance {:.3} mm to {}.",
                nearest_clearance_mm,
                nearest_clearance_component
            ),
            "Review imported fabrication/courtyard evidence; this is a 2D component-clearance screen and does not prove 3D shell, cable, panel, or enclosure fit.".to_string(),
        ],
    })
}

pub(in crate::scenario_suggestions::interface_protection) fn usb_connector_entry_clearance_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<ScenarioSuggestion> {
    let connector = model.usb_connector.as_ref()?;
    let suggested_connector = suggested_usb_connector(bound, component_id, component, connector)?;
    suggested_connector.footprint.as_ref()?;
    let placement = component_placement(bound, component_id)?;
    let entry_direction = usb_entry_direction(bound, component_id, placement, connector)?;
    let entry_direction_deg = entry_direction.deg;
    let obstruction_summary = suggested_connector
        .entry_clearance
        .as_ref()
        .and_then(|entry| entry.nearest_obstruction.as_ref())
        .map(|obstruction| {
            (
                obstruction.component.clone(),
                obstruction.obstruction_depth_mm,
                obstruction.obstruction_lateral_offset_mm,
            )
        });
    let suggested_depth_mm = suggested_connector
        .entry_clearance
        .as_ref()
        .and_then(|entry| entry.suggested_min_cable_entry_clearance_depth_mm);
    let parameters = BTreeMap::from([
        (
            "entry_direction_deg".to_string(),
            serde_json::Value::from(entry_direction_deg),
        ),
        (
            "min_cable_entry_clearance_depth_mm".to_string(),
            suggested_depth_mm
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null),
        ),
        (
            "cable_entry_clearance_width_mm".to_string(),
            serde_json::Value::Null,
        ),
    ]);
    Some(ScenarioSuggestion {
        id: format!(
            "usb_connector_entry_clearance_{}",
            sanitized_name(component_id)
        ),
        kind: "interface_protection".to_string(),
        confidence: "medium".to_string(),
        runnable: false,
        reason: obstruction_summary.as_ref().map_or_else(
            || format!(
                "USB connector {component_id} has imported placement rotation and mechanical footprint evidence; add a cable-entry clearance corridor scenario."
            ),
            |(component, depth_mm, lateral_offset_mm)| {
                format!(
                    "USB connector {component_id} has imported placement rotation and mechanical footprint evidence; nearest forward entry obstruction candidate is {} at {:.3} mm depth and {:.3} mm lateral offset.",
                    component,
                    depth_mm,
                    lateral_offset_mm
                )
            },
        ),
        scenario: SuggestedScenario {
            name: format!(
                "{}_usb_connector_entry_clearance",
                sanitized_name(component_id)
            ),
            scenario_type: "interface_protection".to_string(),
            checks: vec![USB_CONNECTOR_ENTRY_CLEARANCE_VALID.to_string()],
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
            protection_clamps: Vec::new(),
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
            obstruction_summary.as_ref().map_or_else(
                || {
                    if let Some(depth_mm) = suggested_depth_mm {
                        format!(
                            "Review suggested min_cable_entry_clearance_depth_mm {:.3} from connector metadata and fill cable_entry_clearance_width_mm from connector, plug, panel, enclosure, and assembly mechanical drawings before making this scenario runnable.",
                            depth_mm
                        )
                    } else {
                        "Fill min_cable_entry_clearance_depth_mm and cable_entry_clearance_width_mm from connector, plug, panel, enclosure, and assembly mechanical drawings before making this scenario runnable.".to_string()
                    }
                },
                |(component, depth_mm, lateral_offset_mm)| {
                    if let Some(suggested_depth_mm) = suggested_depth_mm {
                        format!(
                            "Review suggested min_cable_entry_clearance_depth_mm {:.3} from connector metadata and fill cable_entry_clearance_width_mm after reviewing nearest forward obstruction candidate {} at {:.3} mm depth and {:.3} mm lateral offset.",
                            suggested_depth_mm, component, depth_mm, lateral_offset_mm
                        )
                    } else {
                        format!(
                            "Fill min_cable_entry_clearance_depth_mm and cable_entry_clearance_width_mm from connector, plug, panel, enclosure, and assembly mechanical drawings after reviewing nearest forward obstruction candidate {} at {:.3} mm depth and {:.3} mm lateral offset.",
                            component, depth_mm, lateral_offset_mm
                        )
                    }
                },
            ),
            entry_direction.offset_deg.map_or_else(
                || "Review entry_direction_deg; by default it is copied from imported connector placement rotation and may need override for footprints whose zero-degree orientation is not the cable insertion direction.".to_string(),
                |offset_deg| {
                    let source = match entry_direction.source {
                        "footprint_property_offset" => "the KiCad footprint property",
                        "kicad_mapping_offset" => "the KiCad mapping",
                        _ => "the component model",
                    };
                    format!(
                        "Review entry_direction_deg; it is computed from imported connector placement rotation plus entry-direction offset {:.3} from {}.",
                        offset_deg, source
                    )
                },
            ),
            "Use 3D mechanical review for connector shell volume, plug body, cable bend radius, panel cutouts, and enclosure interference.".to_string(),
        ],
    })
}

pub(in crate::scenario_suggestions::interface_protection) fn existing_usb_connector_orientation_checks(
    project: &BoardProject,
) -> BTreeSet<String> {
    existing_target_components(project, USB_CONNECTOR_ORIENTATION_VALID)
}

pub(in crate::scenario_suggestions::interface_protection) fn existing_usb_connector_edge_proximity_checks(
    project: &BoardProject,
) -> BTreeSet<String> {
    existing_target_components(project, USB_CONNECTOR_EDGE_PROXIMITY_VALID)
}

pub(in crate::scenario_suggestions::interface_protection) fn existing_usb_connector_body_overhang_checks(
    project: &BoardProject,
) -> BTreeSet<String> {
    existing_target_components(project, USB_CONNECTOR_BODY_OVERHANG_VALID)
}

pub(in crate::scenario_suggestions::interface_protection) fn existing_usb_connector_component_clearance_checks(
    project: &BoardProject,
) -> BTreeSet<String> {
    existing_target_components(project, USB_CONNECTOR_COMPONENT_CLEARANCE_VALID)
}

pub(in crate::scenario_suggestions::interface_protection) fn existing_usb_connector_entry_clearance_checks(
    project: &BoardProject,
) -> BTreeSet<String> {
    existing_target_components(project, USB_CONNECTOR_ENTRY_CLEARANCE_VALID)
}

fn existing_target_components(project: &BoardProject, check_name: &str) -> BTreeSet<String> {
    project
        .scenarios
        .iter()
        .filter(|scenario| scenario.scenario_type == "interface_protection")
        .filter(|scenario| scenario.checks.iter().any(|check| check == check_name))
        .filter_map(|scenario| {
            scenario
                .target
                .as_ref()
                .map(|target| target.component.clone())
        })
        .collect()
}
