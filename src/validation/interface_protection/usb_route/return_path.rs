use crate::board_ir::{CopperZone, LayoutPad, NetKind, NetRoute, RouteVia, Scenario};
use crate::library::{BoundBoard, UsbConnector};
use crate::reports::Finding;
use crate::validation::common::validation_input_missing;

use super::super::UsbConnectorSignal;
use super::findings::{
    UsbReturnPathClearanceEvidence, UsbReturnPathEvidence, UsbReturnPathSegmentEvidence,
    UsbReturnPathStitchViaEvidence, usb_return_path_filled_zone_clearance_finding,
    usb_return_path_metadata_finding, usb_return_path_stitch_via_finding,
    usb_return_path_unreferenced_finding,
};
use super::geometry::{
    PlacementPoint, point_clearance_to_filled_zone_edge, point_inside_filled_zone,
    point_inside_zone_outline, points_inside_same_filled_zone_polygon, segment_length_mm,
    segment_midpoint, validate_route_shape, validate_zone_outline,
};
use super::{
    optional_bool_parameter, optional_nonnegative_parameter, required_nonnegative_parameter,
};

pub(super) fn validate_usb_return_path(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(max_unreferenced_length_mm) =
        required_nonnegative_parameter(scenario, "max_data_line_unreferenced_length_mm", findings)
    else {
        return;
    };
    let Some(max_data_via_to_ground_stitch_distance_mm) = optional_nonnegative_parameter(
        scenario,
        "max_data_via_to_ground_stitch_distance_mm",
        findings,
    ) else {
        return;
    };
    let Some(require_filled_zone_coverage) =
        optional_bool_parameter(scenario, "require_filled_zone_coverage", findings)
    else {
        return;
    };
    let Some(min_data_line_filled_zone_edge_clearance_mm) = optional_nonnegative_parameter(
        scenario,
        "min_data_line_filled_zone_edge_clearance_mm",
        findings,
    ) else {
        return;
    };
    let Some(require_ground_zone_contact_evidence) =
        optional_bool_parameter(scenario, "require_ground_zone_contact_evidence", findings)
    else {
        return;
    };

    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_RETURN_PATH_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_return_path_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB return-path target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_return_path_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB return-path target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(connector) = &model.usb_connector else {
        findings.push(usb_return_path_metadata_finding(
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
    let Some(ground_zones) = ground_reference_zones(bound, scenario, &target.component, findings)
    else {
        return;
    };

    for signal in [UsbConnectorSignal::Dp, UsbConnectorSignal::Dm] {
        validate_usb_return_path_for_signal(
            bound,
            scenario,
            UsbReturnPathSignalCheck {
                connector_id: &target.component,
                component,
                connector,
                signal,
                ground_zones: &ground_zones,
                max_unreferenced_length_mm,
                max_data_via_to_ground_stitch_distance_mm,
                require_filled_zone_coverage,
                min_data_line_filled_zone_edge_clearance_mm,
                require_ground_zone_contact_evidence,
            },
            findings,
        );
    }
}

struct UsbReturnPathSignalCheck<'a> {
    connector_id: &'a str,
    component: &'a crate::board_ir::ComponentSpec,
    connector: &'a UsbConnector,
    signal: UsbConnectorSignal,
    ground_zones: &'a [GroundZoneRef<'a>],
    max_unreferenced_length_mm: f64,
    max_data_via_to_ground_stitch_distance_mm: Option<f64>,
    require_filled_zone_coverage: bool,
    min_data_line_filled_zone_edge_clearance_mm: Option<f64>,
    require_ground_zone_contact_evidence: bool,
}

#[derive(Debug, Clone, Copy)]
struct GroundZoneRef<'a> {
    net_name: &'a str,
    zone: &'a CopperZone,
}

fn validate_usb_return_path_for_signal(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: UsbReturnPathSignalCheck<'_>,
    findings: &mut Vec<Finding>,
) {
    let pin = check.signal.pin(check.connector);
    let Some(net_name) = check.component.pins.get(pin) else {
        findings.push(usb_return_path_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} pin {pin} is not connected, so return-path coverage cannot be checked.",
                check.connector_id,
                check.signal.label()
            ),
            "missing_pin",
            pin,
        ));
        return;
    };
    if !bound.project.board.nets.contains_key(net_name) {
        findings.push(usb_return_path_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} net {net_name} is not declared, so return-path coverage cannot be checked.",
                check.connector_id,
                check.signal.label()
            ),
            "missing_net",
            net_name,
        ));
        return;
    }
    let Some(route) = bound.project.board.layout.routes.get(net_name) else {
        findings.push(usb_return_path_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} net {net_name} has no board.layout.routes entry.",
                check.connector_id,
                check.signal.label()
            ),
            "missing_route",
            net_name,
        ));
        return;
    };
    if let Err(message) = validate_route_shape(route) {
        findings.push(usb_return_path_metadata_finding(
            scenario,
            check.connector_id,
            message,
            "route_geometry",
            net_name,
        ));
        return;
    }
    let mut unreferenced_segments = Vec::new();
    let mut unreferenced_length_mm = 0.0;
    for (segment_index, segment) in route.segments.iter().enumerate() {
        let midpoint = segment_midpoint(segment);
        let referenced = check.ground_zones.iter().any(|ground_zone| {
            ground_zone_references_point(
                bound,
                midpoint,
                &segment.layer,
                ground_zone,
                check.require_filled_zone_coverage,
                check.require_ground_zone_contact_evidence,
            )
        });
        if referenced {
            continue;
        }
        let segment_length_mm = segment_length_mm(segment);
        unreferenced_length_mm += segment_length_mm;
        unreferenced_segments.push(UsbReturnPathSegmentEvidence {
            segment_index,
            segment_length_mm,
            midpoint_x_mm: midpoint.x_mm,
            midpoint_y_mm: midpoint.y_mm,
            layer: segment.layer.clone(),
        });
    }
    if unreferenced_length_mm > check.max_unreferenced_length_mm {
        findings.push(usb_return_path_unreferenced_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            UsbReturnPathEvidence {
                unreferenced_length_mm,
                max_unreferenced_length_mm: check.max_unreferenced_length_mm,
                unreferenced_segments: &unreferenced_segments,
                require_filled_zone_coverage: check.require_filled_zone_coverage,
                require_ground_zone_contact_evidence: check.require_ground_zone_contact_evidence,
            },
        ));
    }
    if let Some(max_distance_mm) = check.max_data_via_to_ground_stitch_distance_mm {
        validate_usb_return_path_stitch_vias(
            bound,
            scenario,
            &check,
            net_name,
            route,
            max_distance_mm,
            findings,
        );
    }
    if let Some(min_clearance_mm) = check.min_data_line_filled_zone_edge_clearance_mm {
        validate_usb_return_path_filled_zone_clearance(
            scenario,
            &check,
            net_name,
            route,
            min_clearance_mm,
            findings,
        );
    }
}

fn point_inside_ground_reference(
    midpoint: PlacementPoint,
    zone: &CopperZone,
    require_filled_zone_coverage: bool,
) -> bool {
    if require_filled_zone_coverage {
        point_inside_filled_zone(midpoint, zone)
    } else {
        point_inside_zone_outline(midpoint, zone)
    }
}

fn ground_zone_references_point(
    bound: &BoundBoard<'_>,
    midpoint: PlacementPoint,
    route_layer: &str,
    ground_zone: &GroundZoneRef<'_>,
    require_filled_zone_coverage: bool,
    require_ground_zone_contact_evidence: bool,
) -> bool {
    if ground_zone.zone.layer != route_layer {
        return false;
    }
    if !point_inside_ground_reference(midpoint, ground_zone.zone, require_filled_zone_coverage) {
        return false;
    }
    if !require_ground_zone_contact_evidence {
        return true;
    }
    ground_zone_has_contact_evidence(bound, midpoint, ground_zone, require_filled_zone_coverage)
}

fn ground_zone_has_contact_evidence(
    bound: &BoundBoard<'_>,
    covered_point: PlacementPoint,
    ground_zone: &GroundZoneRef<'_>,
    require_filled_zone_coverage: bool,
) -> bool {
    ground_pads(bound, ground_zone.net_name).any(|pad| {
        let contact_point = PlacementPoint::from(&pad.at);
        pad_layers_include(&pad.layers, &ground_zone.zone.layer)
            && contact_proves_ground_reference(
                covered_point,
                contact_point,
                ground_zone.zone,
                require_filled_zone_coverage,
            )
    }) || ground_route_vias(bound, ground_zone.net_name).any(|via| {
        let contact_point = PlacementPoint::from(&via.at);
        via_layers_include(&via.layers, &ground_zone.zone.layer)
            && contact_proves_ground_reference(
                covered_point,
                contact_point,
                ground_zone.zone,
                require_filled_zone_coverage,
            )
    })
}

fn contact_proves_ground_reference(
    covered_point: PlacementPoint,
    contact_point: PlacementPoint,
    zone: &CopperZone,
    require_filled_zone_coverage: bool,
) -> bool {
    if require_filled_zone_coverage {
        points_inside_same_filled_zone_polygon(covered_point, contact_point, zone)
    } else {
        point_inside_zone_outline(contact_point, zone)
    }
}

fn ground_pads<'a>(
    bound: &'a BoundBoard<'_>,
    ground_net_name: &'a str,
) -> impl Iterator<Item = &'a LayoutPad> + 'a {
    bound
        .project
        .board
        .layout
        .pads
        .values()
        .flat_map(|component_pads| component_pads.values())
        .filter(move |pad| pad.net == ground_net_name)
        .filter(|pad| pad.at.x_mm.is_finite() && pad.at.y_mm.is_finite())
}

fn ground_route_vias<'a>(
    bound: &'a BoundBoard<'_>,
    ground_net_name: &'a str,
) -> impl Iterator<Item = &'a RouteVia> + 'a {
    bound
        .project
        .board
        .layout
        .routes
        .get(ground_net_name)
        .into_iter()
        .flat_map(|route| route.vias.iter())
        .filter(|via| via.at.x_mm.is_finite() && via.at.y_mm.is_finite())
}

fn pad_layers_include(layers: &[String], zone_layer: &str) -> bool {
    layers.iter().any(|layer| layer_matches(layer, zone_layer))
}

fn via_layers_include(layers: &[String], zone_layer: &str) -> bool {
    layers.iter().any(|layer| layer_matches(layer, zone_layer))
}

fn layer_matches(candidate: &str, zone_layer: &str) -> bool {
    candidate == zone_layer || (candidate == "*.Cu" && zone_layer.ends_with(".Cu"))
}

fn validate_usb_return_path_stitch_vias(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: &UsbReturnPathSignalCheck<'_>,
    net_name: &str,
    route: &NetRoute,
    max_distance_mm: f64,
    findings: &mut Vec<Finding>,
) {
    let ground_vias = ground_stitch_vias(bound);
    for (via_index, via) in route.vias.iter().enumerate() {
        let nearest = nearest_matching_ground_via(via, &ground_vias);
        if nearest
            .as_ref()
            .is_none_or(|candidate| candidate.distance_mm > max_distance_mm)
        {
            findings.push(usb_return_path_stitch_via_finding(
                scenario,
                UsbReturnPathStitchViaEvidence {
                    connector_id: check.connector_id,
                    signal: check.signal,
                    net: net_name,
                    data_via_index: via_index,
                    data_via: via,
                    nearest,
                    max_distance_mm,
                },
            ));
        }
    }
}

fn validate_usb_return_path_filled_zone_clearance(
    scenario: &Scenario,
    check: &UsbReturnPathSignalCheck<'_>,
    net_name: &str,
    route: &NetRoute,
    min_clearance_mm: f64,
    findings: &mut Vec<Finding>,
) {
    for (segment_index, segment) in route.segments.iter().enumerate() {
        let midpoint = segment_midpoint(segment);
        let clearance_mm = check
            .ground_zones
            .iter()
            .filter(|ground_zone| ground_zone.zone.layer == segment.layer)
            .filter_map(|ground_zone| {
                point_clearance_to_filled_zone_edge(midpoint, ground_zone.zone)
            })
            .max_by(|left, right| left.total_cmp(right));
        if clearance_mm.is_some_and(|clearance_mm| clearance_mm >= min_clearance_mm) {
            continue;
        }
        findings.push(usb_return_path_filled_zone_clearance_finding(
            scenario,
            UsbReturnPathClearanceEvidence {
                connector_id: check.connector_id,
                signal: check.signal,
                net: net_name,
                segment_index,
                segment_length_mm: segment_length_mm(segment),
                midpoint_x_mm: midpoint.x_mm,
                midpoint_y_mm: midpoint.y_mm,
                layer: &segment.layer,
                clearance_mm,
                min_clearance_mm,
            },
        ));
    }
}

#[derive(Debug, Clone, Copy)]
struct GroundStitchViaRef<'a> {
    net_name: &'a str,
    via_index: usize,
    via: &'a RouteVia,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::validation::interface_protection::usb_route) struct GroundStitchViaCandidate<'a> {
    pub(in crate::validation::interface_protection::usb_route) ground_net: &'a str,
    pub(in crate::validation::interface_protection::usb_route) ground_via_index: usize,
    pub(in crate::validation::interface_protection::usb_route) distance_mm: f64,
}

fn ground_stitch_vias<'a>(bound: &'a BoundBoard<'_>) -> Vec<GroundStitchViaRef<'a>> {
    let mut vias = Vec::new();
    for (net_name, route) in &bound.project.board.layout.routes {
        let Some(net) = bound.project.board.nets.get(net_name) else {
            continue;
        };
        if net.kind != NetKind::Ground {
            continue;
        }
        for (via_index, via) in route.vias.iter().enumerate() {
            vias.push(GroundStitchViaRef {
                net_name,
                via_index,
                via,
            });
        }
    }
    vias
}

fn nearest_matching_ground_via<'a>(
    data_via: &RouteVia,
    ground_vias: &'a [GroundStitchViaRef<'a>],
) -> Option<GroundStitchViaCandidate<'a>> {
    ground_vias
        .iter()
        .filter(|ground_via| via_layers_match(data_via, ground_via.via))
        .map(|ground_via| GroundStitchViaCandidate {
            ground_net: ground_via.net_name,
            ground_via_index: ground_via.via_index,
            distance_mm: via_distance_mm(data_via, ground_via.via),
        })
        .min_by(|left, right| left.distance_mm.total_cmp(&right.distance_mm))
}

fn via_layers_match(data_via: &RouteVia, ground_via: &RouteVia) -> bool {
    if data_via.layers.is_empty() || ground_via.layers.is_empty() {
        return true;
    }
    data_via.layers.iter().all(|layer| {
        ground_via
            .layers
            .iter()
            .any(|ground_layer| ground_layer == layer)
    })
}

fn via_distance_mm(first: &RouteVia, second: &RouteVia) -> f64 {
    let dx = first.at.x_mm - second.at.x_mm;
    let dy = first.at.y_mm - second.at.y_mm;
    dx.hypot(dy)
}

fn ground_reference_zones<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    connector_id: &str,
    findings: &mut Vec<Finding>,
) -> Option<Vec<GroundZoneRef<'a>>> {
    let mut zones = Vec::new();
    for (net_name, zone_list) in &bound.project.board.layout.zones {
        let Some(net) = bound.project.board.nets.get(net_name) else {
            continue;
        };
        if net.kind != NetKind::Ground {
            continue;
        }
        for zone in zone_list {
            if let Err(message) = validate_zone_outline(zone) {
                findings.push(usb_return_path_metadata_finding(
                    scenario,
                    connector_id,
                    message,
                    "ground_zone",
                    net_name,
                ));
                return None;
            }
            zones.push(GroundZoneRef { net_name, zone });
        }
    }
    if zones.is_empty() {
        findings.push(usb_return_path_metadata_finding(
            scenario,
            connector_id,
            "USB return-path validation requires at least one board.layout.zones entry whose net kind is ground.".to_string(),
            "missing_ground_zone",
            "ground",
        ));
        return None;
    }
    Some(zones)
}
