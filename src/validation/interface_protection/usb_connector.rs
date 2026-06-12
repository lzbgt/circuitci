use crate::board_ir::{
    ComponentPlacement, ComponentSpec, LayoutFootprintRectangle, LayoutFootprintSegment,
    LayoutPoint, LayoutSegment, NetKind, Scenario,
};
use crate::library::{BoundBoard, ProtectionClamp, ProtectionReference, UsbConnector};
use crate::reports::Finding;

use super::super::common::validation_input_missing;
use super::usb_connector_findings::*;
use super::{
    required_scenario_numeric_parameter, scenario_bool_parameter, scenario_numeric_parameter,
};

pub(super) fn validate_usb_connector_orientation(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(expected_rotation_deg) =
        required_scenario_numeric_parameter(scenario, "expected_connector_rotation_deg", findings)
    else {
        return;
    };
    let Some(max_error_deg) =
        required_scenario_numeric_parameter(scenario, "max_connector_rotation_error_deg", findings)
    else {
        return;
    };
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_CONNECTOR_ORIENTATION_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_orientation_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector orientation target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_orientation_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector orientation target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    if model.usb_connector.is_none() {
        findings.push(usb_orientation_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no usb_connector metadata.",
                target.component, component.model
            ),
            "usb_connector",
            "missing",
        ));
        return;
    }
    let Some(placement) = valid_component_placement(bound, scenario, &target.component, findings)
    else {
        return;
    };
    let Some(actual_rotation_deg) = placement.rotation_deg else {
        findings.push(usb_orientation_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector {} placement has no rotation_deg evidence.",
                target.component
            ),
            "rotation_deg",
            "missing",
        ));
        return;
    };
    if !actual_rotation_deg.is_finite() {
        findings.push(usb_orientation_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector {} placement rotation_deg must be finite.",
                target.component
            ),
            "rotation_deg",
            "non_finite",
        ));
        return;
    }
    let rotation_error_deg = angular_error_deg(actual_rotation_deg, expected_rotation_deg);
    if rotation_error_deg > max_error_deg {
        findings.push(usb_orientation_finding(
            scenario,
            &target.component,
            placement,
            actual_rotation_deg,
            expected_rotation_deg,
            rotation_error_deg,
            max_error_deg,
        ));
    }
}

pub(super) fn validate_usb_connector_edge_proximity(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(max_distance_mm) = required_scenario_numeric_parameter(
        scenario,
        "max_connector_to_board_edge_distance_mm",
        findings,
    ) else {
        return;
    };
    if max_distance_mm <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.max_connector_to_board_edge_distance_mm must be greater than zero.",
        );
        return;
    }
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_CONNECTOR_EDGE_PROXIMITY_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_edge_proximity_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector edge-proximity target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_edge_proximity_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector edge-proximity target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    if model.usb_connector.is_none() {
        findings.push(usb_edge_proximity_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no usb_connector metadata.",
                target.component, component.model
            ),
            "usb_connector",
            "missing",
        ));
        return;
    }
    let Some(placement) = valid_component_placement(bound, scenario, &target.component, findings)
    else {
        return;
    };
    let Some(edge) = nearest_board_edge(bound, &target.component, placement) else {
        findings.push(usb_edge_proximity_metadata_finding(
            scenario,
            &target.component,
            "USB connector edge proximity requires at least one usable board.layout.outline.segments entry.".to_string(),
            "outline",
            "missing",
        ));
        return;
    };
    if edge.distance_mm > max_distance_mm {
        findings.push(usb_edge_proximity_finding(
            scenario,
            &target.component,
            placement,
            &edge,
            max_distance_mm,
        ));
    }
}

pub(super) fn validate_usb_connector_protection(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_CONNECTOR_PROTECTION_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(connector) = &model.usb_connector else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no usb_connector metadata.",
                target.component, component.model
            ),
            "usb_connector",
            &component.model,
        ));
        return;
    };

    validate_usb_connector_pin(
        bound,
        scenario,
        &target.component,
        component,
        connector,
        UsbConnectorSignal::Dp,
        findings,
    );
    validate_usb_connector_pin(
        bound,
        scenario,
        &target.component,
        component,
        connector,
        UsbConnectorSignal::Dm,
        findings,
    );

    if scenario_bool_parameter(scenario, "require_vbus_protection").unwrap_or(false) {
        validate_usb_connector_pin(
            bound,
            scenario,
            &target.component,
            component,
            connector,
            UsbConnectorSignal::Vbus,
            findings,
        );
    }
    if scenario_bool_parameter(scenario, "require_shield_ground").unwrap_or(false) {
        validate_usb_connector_shield_ground(
            bound,
            scenario,
            &target.component,
            component,
            connector,
            findings,
        );
    }
}

pub(super) fn validate_usb_protection_placement(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(max_distance_mm) = required_scenario_numeric_parameter(
        scenario,
        "max_connector_to_protection_distance_mm",
        findings,
    ) else {
        return;
    };
    if max_distance_mm <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.max_connector_to_protection_distance_mm must be greater than zero.",
        );
        return;
    }
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_PROTECTION_PLACEMENT_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_placement_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB placement target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_placement_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB placement target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(connector) = &model.usb_connector else {
        findings.push(usb_placement_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no usb_connector metadata.",
                target.component, component.model
            ),
            "usb_connector",
            &component.model,
        ));
        return;
    };
    let Some(connector_placement) =
        valid_component_placement(bound, scenario, &target.component, findings)
    else {
        return;
    };
    validate_usb_protection_placement_for_pin(
        bound,
        scenario,
        UsbPlacementPinCheck {
            connector_id: &target.component,
            component,
            connector,
            connector_placement,
            signal: UsbConnectorSignal::Dp,
            max_distance_mm,
        },
        findings,
    );
    validate_usb_protection_placement_for_pin(
        bound,
        scenario,
        UsbPlacementPinCheck {
            connector_id: &target.component,
            component,
            connector,
            connector_placement,
            signal: UsbConnectorSignal::Dm,
            max_distance_mm,
        },
        findings,
    );
    if scenario_bool_parameter(scenario, "require_vbus_protection").unwrap_or(false) {
        validate_usb_protection_placement_for_pin(
            bound,
            scenario,
            UsbPlacementPinCheck {
                connector_id: &target.component,
                component,
                connector,
                connector_placement,
                signal: UsbConnectorSignal::Vbus,
                max_distance_mm,
            },
            findings,
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum UsbConnectorSignal {
    Dp,
    Dm,
    Vbus,
}

impl UsbConnectorSignal {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Dp => "D+",
            Self::Dm => "D-",
            Self::Vbus => "VBUS",
        }
    }

    pub(super) fn pin(self, connector: &UsbConnector) -> &str {
        match self {
            Self::Dp => &connector.dp_pin,
            Self::Dm => &connector.dm_pin,
            Self::Vbus => &connector.vbus_pin,
        }
    }
}

fn validate_usb_connector_pin(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    connector_id: &str,
    component: &ComponentSpec,
    connector: &UsbConnector,
    signal: UsbConnectorSignal,
    findings: &mut Vec<Finding>,
) {
    let pin = signal.pin(connector);
    let Some(net_name) = component.pins.get(pin) else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} {} pin {pin} is not connected.",
                signal.label()
            ),
            "missing_pin",
            pin,
        ));
        return;
    };
    if !bound.project.board.nets.contains_key(net_name) {
        findings.push(usb_connector_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} {} net {net_name} is not declared.",
                signal.label()
            ),
            "missing_net",
            net_name,
        ));
        return;
    }
    if let Some(protection) = find_valid_clamp_for_net(bound, connector_id, net_name) {
        if let Some(min_standoff_v) = scenario_numeric_parameter(
            scenario,
            match signal {
                UsbConnectorSignal::Vbus => "vbus_working_voltage_min_V",
                UsbConnectorSignal::Dp | UsbConnectorSignal::Dm => "data_working_voltage_min_V",
            },
            findings,
        ) && let Some(working_voltage_max_v) = protection.clamp.working_voltage_max_v
            && working_voltage_max_v < min_standoff_v
        {
            findings.push(usb_connector_standoff_finding(
                scenario,
                connector_id,
                signal,
                net_name,
                &protection,
                working_voltage_max_v,
                min_standoff_v,
            ));
        }
        return;
    }
    findings.push(usb_connector_missing_protection_finding(
        scenario,
        connector_id,
        signal,
        pin,
        net_name,
    ));
}

fn validate_usb_connector_shield_ground(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    connector_id: &str,
    component: &ComponentSpec,
    connector: &UsbConnector,
    findings: &mut Vec<Finding>,
) {
    let Some(shield_pin) = connector.shield_pin.as_deref() else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} has no shield_pin metadata, but require_shield_ground is true."
            ),
            "shield_pin",
            connector_id,
        ));
        return;
    };
    let Some(shield_net_name) = component.pins.get(shield_pin) else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} shield pin {shield_pin} is not connected, but require_shield_ground is true."
            ),
            "missing_shield_pin",
            shield_pin,
        ));
        return;
    };
    let Some(shield_net) = bound.project.board.nets.get(shield_net_name) else {
        findings.push(usb_connector_metadata_finding(
            scenario,
            connector_id,
            format!("USB connector {connector_id} shield net {shield_net_name} is not declared."),
            "missing_shield_net",
            shield_net_name,
        ));
        return;
    };
    if shield_net.kind != NetKind::Ground {
        findings.push(usb_connector_shield_ground_finding(
            scenario,
            connector_id,
            shield_pin,
            shield_net_name,
            &shield_net.kind,
        ));
    }
}

pub(super) struct ResolvedUsbProtection<'a> {
    pub(super) component_id: &'a str,
    pub(super) clamp: &'a ProtectionClamp,
    pub(super) reference_net_name: &'a str,
    pub(super) reference_net_kind: &'a NetKind,
}

struct UsbPlacementPinCheck<'a> {
    connector_id: &'a str,
    component: &'a ComponentSpec,
    connector: &'a UsbConnector,
    connector_placement: &'a ComponentPlacement,
    signal: UsbConnectorSignal,
    max_distance_mm: f64,
}

pub(super) struct UsbPlacementDistanceEvidence<'a> {
    pub(super) scenario: &'a Scenario,
    pub(super) connector_id: &'a str,
    pub(super) signal: UsbConnectorSignal,
    pub(super) net: &'a str,
    pub(super) protection: &'a ResolvedUsbProtection<'a>,
    pub(super) connector_placement: &'a ComponentPlacement,
    pub(super) protection_placement: &'a ComponentPlacement,
    pub(super) distance_mm: f64,
    pub(super) max_distance_mm: f64,
}

pub(super) struct UsbBoardEdgeDistanceEvidence<'a> {
    pub(super) distance_mm: f64,
    pub(super) edge: &'a LayoutSegment,
    pub(super) connector_reference: UsbBoardEdgeConnectorReference<'a>,
}

#[derive(Clone, Copy)]
pub(super) enum UsbBoardEdgeConnectorReference<'a> {
    PlacementCenter,
    FootprintSegment { layer: &'a str, kind: &'a str },
    FootprintRectangle { layer: &'a str, kind: &'a str },
}

impl UsbBoardEdgeConnectorReference<'_> {
    pub(super) fn label(&self) -> &'static str {
        match self {
            UsbBoardEdgeConnectorReference::PlacementCenter => "placement_center",
            UsbBoardEdgeConnectorReference::FootprintSegment { .. } => "footprint_segment",
            UsbBoardEdgeConnectorReference::FootprintRectangle { .. } => "footprint_rectangle",
        }
    }

    pub(super) fn footprint_layer(&self) -> Option<&str> {
        match self {
            UsbBoardEdgeConnectorReference::PlacementCenter => None,
            UsbBoardEdgeConnectorReference::FootprintSegment { layer, .. }
            | UsbBoardEdgeConnectorReference::FootprintRectangle { layer, .. } => Some(layer),
        }
    }

    pub(super) fn footprint_kind(&self) -> Option<&str> {
        match self {
            UsbBoardEdgeConnectorReference::PlacementCenter => None,
            UsbBoardEdgeConnectorReference::FootprintSegment { kind, .. }
            | UsbBoardEdgeConnectorReference::FootprintRectangle { kind, .. } => Some(kind),
        }
    }
}

fn find_valid_clamp_for_net<'a>(
    bound: &'a BoundBoard<'_>,
    connector_id: &str,
    net_name: &str,
) -> Option<ResolvedUsbProtection<'a>> {
    valid_protection_clamps_for_net(bound, connector_id, net_name)
        .into_iter()
        .next()
}

pub(super) fn valid_protection_clamps_for_net<'a>(
    bound: &'a BoundBoard<'_>,
    connector_id: &str,
    net_name: &str,
) -> Vec<ResolvedUsbProtection<'a>> {
    let mut protections = Vec::new();
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
                protections.push(ResolvedUsbProtection {
                    component_id,
                    clamp,
                    reference_net_name,
                    reference_net_kind: &reference_net.kind,
                });
            }
        }
    }
    protections
}

fn validate_usb_protection_placement_for_pin(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: UsbPlacementPinCheck<'_>,
    findings: &mut Vec<Finding>,
) {
    let connector_id = check.connector_id;
    let connector_placement = check.connector_placement;
    let signal = check.signal;
    let max_distance_mm = check.max_distance_mm;
    let component = check.component;
    let connector = check.connector;
    let pin = signal.pin(connector);
    let Some(net_name) = component.pins.get(pin) else {
        findings.push(usb_placement_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} {} pin {pin} is not connected, so protection placement cannot be checked.",
                signal.label()
            ),
            "missing_pin",
            pin,
        ));
        return;
    };
    if !bound.project.board.nets.contains_key(net_name) {
        findings.push(usb_placement_metadata_finding(
            scenario,
            connector_id,
            format!(
                "USB connector {connector_id} {} net {net_name} is not declared, so protection placement cannot be checked.",
                signal.label()
            ),
            "missing_net",
            net_name,
        ));
        return;
    }
    let protections = valid_protection_clamps_for_net(bound, connector_id, net_name);
    if protections.is_empty() {
        findings.push(usb_placement_missing_protection_finding(
            scenario,
            connector_id,
            signal,
            pin,
            net_name,
        ));
        return;
    }
    let mut nearest: Option<(&ResolvedUsbProtection<'_>, &ComponentPlacement, f64)> = None;
    let mut missing_placements = Vec::new();
    for protection in &protections {
        let Some(protection_placement) = bound
            .project
            .board
            .layout
            .placements
            .get(protection.component_id)
        else {
            missing_placements.push(protection.component_id.to_string());
            continue;
        };
        if !placement_is_finite(protection_placement) {
            findings.push(usb_placement_metadata_finding(
                scenario,
                protection.component_id,
                format!(
                    "USB protection component {} placement must have finite x_mm and y_mm.",
                    protection.component_id
                ),
                "placement",
                protection.component_id,
            ));
            continue;
        }
        let distance_mm = placement_distance_mm(connector_placement, protection_placement);
        if nearest
            .as_ref()
            .is_none_or(|(_, _, nearest_distance)| distance_mm < *nearest_distance)
        {
            nearest = Some((protection, protection_placement, distance_mm));
        }
    }
    let Some((protection, protection_placement, distance_mm)) = nearest else {
        findings.push(usb_placement_missing_protection_placement_finding(
            scenario,
            connector_id,
            signal,
            net_name,
            &missing_placements,
        ));
        return;
    };
    if distance_mm > max_distance_mm {
        findings.push(usb_placement_distance_finding(
            UsbPlacementDistanceEvidence {
                scenario,
                connector_id,
                signal,
                net: net_name,
                protection,
                connector_placement,
                protection_placement,
                distance_mm,
                max_distance_mm,
            },
        ));
    }
}

pub(super) fn valid_component_placement<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    component_id: &str,
    findings: &mut Vec<Finding>,
) -> Option<&'a ComponentPlacement> {
    let Some(placement) = bound.project.board.layout.placements.get(component_id) else {
        findings.push(usb_placement_metadata_finding(
            scenario,
            component_id,
            format!("Component {component_id} has no board.layout.placements entry."),
            "placement",
            component_id,
        ));
        return None;
    };
    if !placement_is_finite(placement) {
        findings.push(usb_placement_metadata_finding(
            scenario,
            component_id,
            format!("Component {component_id} placement must have finite x_mm and y_mm."),
            "placement",
            component_id,
        ));
        return None;
    }
    Some(placement)
}

pub(super) fn placement_is_finite(placement: &ComponentPlacement) -> bool {
    placement.x_mm.is_finite() && placement.y_mm.is_finite()
}

fn placement_distance_mm(a: &ComponentPlacement, b: &ComponentPlacement) -> f64 {
    let dx = a.x_mm - b.x_mm;
    let dy = a.y_mm - b.y_mm;
    dx.hypot(dy)
}

fn nearest_board_edge<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &'a str,
    placement: &ComponentPlacement,
) -> Option<UsbBoardEdgeDistanceEvidence<'a>> {
    bound
        .project
        .board
        .layout
        .outline
        .segments
        .iter()
        .filter(|segment| outline_segment_is_usable(segment))
        .map(|edge| {
            let (distance_mm, connector_reference) =
                connector_to_edge_distance(bound, component_id, placement, edge);
            UsbBoardEdgeDistanceEvidence {
                distance_mm,
                edge,
                connector_reference,
            }
        })
        .min_by(|left, right| left.distance_mm.total_cmp(&right.distance_mm))
}

fn connector_to_edge_distance<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &'a str,
    placement: &ComponentPlacement,
    edge: &LayoutSegment,
) -> (f64, UsbBoardEdgeConnectorReference<'a>) {
    let mut best_distance = placement_to_segment_distance_mm(placement, edge);
    let mut best_reference = UsbBoardEdgeConnectorReference::PlacementCenter;
    let Some(footprint) = bound.project.board.layout.footprints.get(component_id) else {
        return (best_distance, best_reference);
    };

    for segment in &footprint.segments {
        if !mechanical_footprint_kind(&segment.kind) {
            continue;
        }
        let Some(distance_mm) = footprint_segment_to_edge_distance_mm(segment, edge) else {
            continue;
        };
        if distance_mm < best_distance {
            best_distance = distance_mm;
            best_reference = UsbBoardEdgeConnectorReference::FootprintSegment {
                layer: &segment.layer,
                kind: &segment.kind,
            };
        }
    }

    for rectangle in &footprint.rectangles {
        if !mechanical_footprint_kind(&rectangle.kind) {
            continue;
        }
        let Some(distance_mm) = footprint_rectangle_to_edge_distance_mm(rectangle, edge) else {
            continue;
        };
        if distance_mm < best_distance {
            best_distance = distance_mm;
            best_reference = UsbBoardEdgeConnectorReference::FootprintRectangle {
                layer: &rectangle.layer,
                kind: &rectangle.kind,
            };
        }
    }

    (best_distance, best_reference)
}

fn mechanical_footprint_kind(kind: &str) -> bool {
    matches!(kind, "fabrication" | "courtyard")
}

fn outline_segment_is_usable(segment: &LayoutSegment) -> bool {
    point_is_finite(&segment.start)
        && point_is_finite(&segment.end)
        && (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
            > f64::EPSILON
}

fn point_is_finite(point: &LayoutPoint) -> bool {
    point.x_mm.is_finite() && point.y_mm.is_finite()
}

fn placement_to_segment_distance_mm(
    placement: &ComponentPlacement,
    segment: &LayoutSegment,
) -> f64 {
    point_to_segment_distance_mm(
        placement.x_mm,
        placement.y_mm,
        segment.start.x_mm,
        segment.start.y_mm,
        segment.end.x_mm,
        segment.end.y_mm,
    )
}

fn footprint_segment_to_edge_distance_mm(
    segment: &LayoutFootprintSegment,
    edge: &LayoutSegment,
) -> Option<f64> {
    if !point_is_finite(&segment.start)
        || !point_is_finite(&segment.end)
        || segment_length_mm(&segment.start, &segment.end) <= f64::EPSILON
    {
        return None;
    }
    Some(segment_to_segment_distance_mm(
        &segment.start,
        &segment.end,
        &edge.start,
        &edge.end,
    ))
}

fn footprint_rectangle_to_edge_distance_mm(
    rectangle: &LayoutFootprintRectangle,
    edge: &LayoutSegment,
) -> Option<f64> {
    if !point_is_finite(&rectangle.start) || !point_is_finite(&rectangle.end) {
        return None;
    }
    let min_x = rectangle.start.x_mm.min(rectangle.end.x_mm);
    let max_x = rectangle.start.x_mm.max(rectangle.end.x_mm);
    let min_y = rectangle.start.y_mm.min(rectangle.end.y_mm);
    let max_y = rectangle.start.y_mm.max(rectangle.end.y_mm);
    if (max_x - min_x).abs() <= f64::EPSILON || (max_y - min_y).abs() <= f64::EPSILON {
        return None;
    }
    let corners = [
        LayoutPoint {
            x_mm: min_x,
            y_mm: min_y,
        },
        LayoutPoint {
            x_mm: max_x,
            y_mm: min_y,
        },
        LayoutPoint {
            x_mm: max_x,
            y_mm: max_y,
        },
        LayoutPoint {
            x_mm: min_x,
            y_mm: max_y,
        },
    ];
    Some(
        (0..corners.len())
            .map(|index| {
                let next_index = (index + 1) % corners.len();
                segment_to_segment_distance_mm(
                    &corners[index],
                    &corners[next_index],
                    &edge.start,
                    &edge.end,
                )
            })
            .fold(f64::INFINITY, f64::min),
    )
}

fn segment_length_mm(start: &LayoutPoint, end: &LayoutPoint) -> f64 {
    (end.x_mm - start.x_mm).hypot(end.y_mm - start.y_mm)
}

fn segment_to_segment_distance_mm(
    a_start: &LayoutPoint,
    a_end: &LayoutPoint,
    b_start: &LayoutPoint,
    b_end: &LayoutPoint,
) -> f64 {
    if segments_intersect(a_start, a_end, b_start, b_end) {
        return 0.0;
    }
    [
        point_to_segment_distance_mm(
            a_start.x_mm,
            a_start.y_mm,
            b_start.x_mm,
            b_start.y_mm,
            b_end.x_mm,
            b_end.y_mm,
        ),
        point_to_segment_distance_mm(
            a_end.x_mm,
            a_end.y_mm,
            b_start.x_mm,
            b_start.y_mm,
            b_end.x_mm,
            b_end.y_mm,
        ),
        point_to_segment_distance_mm(
            b_start.x_mm,
            b_start.y_mm,
            a_start.x_mm,
            a_start.y_mm,
            a_end.x_mm,
            a_end.y_mm,
        ),
        point_to_segment_distance_mm(
            b_end.x_mm,
            b_end.y_mm,
            a_start.x_mm,
            a_start.y_mm,
            a_end.x_mm,
            a_end.y_mm,
        ),
    ]
    .into_iter()
    .fold(f64::INFINITY, f64::min)
}

fn segments_intersect(
    a_start: &LayoutPoint,
    a_end: &LayoutPoint,
    b_start: &LayoutPoint,
    b_end: &LayoutPoint,
) -> bool {
    let d1 = cross_product(a_start, a_end, b_start);
    let d2 = cross_product(a_start, a_end, b_end);
    let d3 = cross_product(b_start, b_end, a_start);
    let d4 = cross_product(b_start, b_end, a_end);
    if ((d1 > f64::EPSILON && d2 < -f64::EPSILON) || (d1 < -f64::EPSILON && d2 > f64::EPSILON))
        && ((d3 > f64::EPSILON && d4 < -f64::EPSILON) || (d3 < -f64::EPSILON && d4 > f64::EPSILON))
    {
        return true;
    }
    (d1.abs() <= f64::EPSILON && point_on_segment(b_start, a_start, a_end))
        || (d2.abs() <= f64::EPSILON && point_on_segment(b_end, a_start, a_end))
        || (d3.abs() <= f64::EPSILON && point_on_segment(a_start, b_start, b_end))
        || (d4.abs() <= f64::EPSILON && point_on_segment(a_end, b_start, b_end))
}

fn cross_product(origin: &LayoutPoint, end: &LayoutPoint, point: &LayoutPoint) -> f64 {
    (end.x_mm - origin.x_mm) * (point.y_mm - origin.y_mm)
        - (end.y_mm - origin.y_mm) * (point.x_mm - origin.x_mm)
}

fn point_on_segment(point: &LayoutPoint, start: &LayoutPoint, end: &LayoutPoint) -> bool {
    point.x_mm >= start.x_mm.min(end.x_mm) - f64::EPSILON
        && point.x_mm <= start.x_mm.max(end.x_mm) + f64::EPSILON
        && point.y_mm >= start.y_mm.min(end.y_mm) - f64::EPSILON
        && point.y_mm <= start.y_mm.max(end.y_mm) + f64::EPSILON
}

fn point_to_segment_distance_mm(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f64::EPSILON {
        return (px - ax).hypot(py - ay);
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / length_sq).clamp(0.0, 1.0);
    let nearest_x = ax + t * dx;
    let nearest_y = ay + t * dy;
    (px - nearest_x).hypot(py - nearest_y)
}

fn normalize_rotation_deg(rotation_deg: f64) -> f64 {
    rotation_deg.rem_euclid(360.0)
}

fn angular_error_deg(actual_deg: f64, expected_deg: f64) -> f64 {
    let delta = (normalize_rotation_deg(actual_deg) - normalize_rotation_deg(expected_deg)).abs();
    delta.min(360.0 - delta)
}
