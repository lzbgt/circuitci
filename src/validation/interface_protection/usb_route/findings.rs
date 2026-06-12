use crate::board_ir::RouteVia;
use crate::board_ir::Scenario;
use crate::reports::Finding;
use crate::validation::{USB_RETURN_PATH_VALID, USB_ROUTE_GEOMETRY_VALID};

use super::GroundStitchViaCandidate;
use serde::Serialize;
use serde_json::json;

use super::UsbConnectorSignal;
use super::geometry::UsbPairGapEvidence;

pub(super) fn usb_route_metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(USB_ROUTE_GEOMETRY_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Import PCB route geometry with import-kicad-pcb before declaring USB_ROUTE_GEOMETRY_VALID.".to_string(),
        "Declare route limits from the board USB/layout rule instead of inferring them from coordinates.".to_string(),
    ];
    finding
}

pub(super) fn usb_route_length_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    route_length_mm: f64,
    max_route_length_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} route length {:.3} mm exceeds limit {:.3} mm.",
            signal.label(),
            route_length_mm,
            max_route_length_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .measured
        .insert("route_length_mm".to_string(), json!(route_length_mm));
    finding.limit.insert(
        "max_data_line_route_length_mm".to_string(),
        json!(max_route_length_mm),
    );
    finding.suggested_fixes = vec![
        "Shorten the USB data-line route or move the connector/protected device closer together."
            .to_string(),
        "Use a board-specific USB layout rule for max_data_line_route_length_mm.".to_string(),
    ];
    finding
}

pub(super) fn usb_route_via_count_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    via_count: usize,
    max_via_count: usize,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} has {via_count} vias, above limit {max_via_count}.",
            signal.label()
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .measured
        .insert("via_count".to_string(), json!(via_count));
    finding
        .limit
        .insert("max_data_line_via_count".to_string(), json!(max_via_count));
    finding.suggested_fixes = vec![
        "Reduce USB data-line layer changes or relax max_data_line_via_count only with layout/SI justification.".to_string(),
        "Keep D+ and D- via usage symmetric when the board route must change layers.".to_string(),
    ];
    finding
}

pub(super) fn usb_route_width_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    evidence: UsbRouteWidthEvidence,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} segment {} width {:.3} mm differs from route rule {:.3} mm by {:.3} mm, above tolerance {:.3} mm.",
            signal.label(),
            evidence.segment_index,
            evidence.measured_width_mm,
            evidence.expected_width_mm,
            evidence.width_delta_mm,
            evidence.max_width_delta_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .measured
        .insert("segment_index".to_string(), json!(evidence.segment_index));
    finding.measured.insert(
        "route_segment_width_mm".to_string(),
        json!(evidence.measured_width_mm),
    );
    finding.measured.insert(
        "route_width_delta_mm".to_string(),
        json!(evidence.width_delta_mm),
    );
    finding.limit.insert(
        "expected_data_line_width_mm".to_string(),
        json!(evidence.expected_width_mm),
    );
    finding.limit.insert(
        "max_data_line_width_delta_mm".to_string(),
        json!(evidence.max_width_delta_mm),
    );
    finding.suggested_fixes = vec![
        "Update the routed USB data-line width to match the imported PCB route rule.".to_string(),
        "If the route intentionally necks down, encode that exception as a more specific board rule instead of relaxing the global USB route check.".to_string(),
    ];
    finding
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UsbRouteWidthEvidence {
    pub(super) segment_index: usize,
    pub(super) measured_width_mm: f64,
    pub(super) expected_width_mm: f64,
    pub(super) width_delta_mm: f64,
    pub(super) max_width_delta_mm: f64,
}

pub(super) fn usb_route_no_protection_path_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    missing_placements: &[String],
    off_route_components: &[String],
    max_component_to_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} has no protection component with usable route-distance evidence.",
            signal.label()
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding.measured.insert(
        "protection_components_without_placement".to_string(),
        json!(missing_placements),
    );
    finding.measured.insert(
        "protection_components_off_route".to_string(),
        json!(off_route_components),
    );
    finding.limit.insert(
        "max_component_to_route_distance_mm".to_string(),
        json!(max_component_to_route_distance_mm),
    );
    finding.suggested_fixes = vec![
        "Place the USB ESD component on the routed USB net near the connector and import updated PCB route geometry.".to_string(),
        "Check that component placement coordinates and route coordinates share the same PCB coordinate system.".to_string(),
    ];
    finding
}

pub(super) fn usb_pair_gap_unmeasured_finding(
    scenario: &Scenario,
    connector_id: &str,
    dp_net: &str,
    dm_net: &str,
    expected_gap_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} D+/D- nets {dp_net}/{dm_net} have no overlapping parallel routed segments for diff-pair gap validation."
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.measured.insert("dp_net".to_string(), json!(dp_net));
    finding.measured.insert("dm_net".to_string(), json!(dm_net));
    finding.limit.insert(
        "expected_data_pair_gap_mm".to_string(),
        json!(expected_gap_mm),
    );
    finding.suggested_fixes = vec![
        "Route USB D+ and D- as overlapping parallel segments where differential-pair gap can be measured.".to_string(),
        "Import updated PCB route geometry after routing the data lines as a differential pair.".to_string(),
    ];
    finding
}

pub(super) fn usb_pair_gap_finding(
    scenario: &Scenario,
    connector_id: &str,
    dp_net: &str,
    dm_net: &str,
    evidence: UsbPairGapEvidence,
    max_gap_delta_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} D+/D- edge gap {:.3} mm differs from route rule {:.3} mm by {:.3} mm, above tolerance {:.3} mm.",
            evidence.measured_gap_mm,
            evidence.expected_gap_mm,
            evidence.gap_delta_mm,
            max_gap_delta_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.measured.insert("dp_net".to_string(), json!(dp_net));
    finding.measured.insert("dm_net".to_string(), json!(dm_net));
    finding.measured.insert(
        "dp_segment_index".to_string(),
        json!(evidence.dp_segment_index),
    );
    finding.measured.insert(
        "dm_segment_index".to_string(),
        json!(evidence.dm_segment_index),
    );
    finding.measured.insert(
        "data_pair_centerline_distance_mm".to_string(),
        json!(evidence.centerline_distance_mm),
    );
    finding.measured.insert(
        "data_pair_gap_mm".to_string(),
        json!(evidence.measured_gap_mm),
    );
    finding.measured.insert(
        "data_pair_gap_delta_mm".to_string(),
        json!(evidence.gap_delta_mm),
    );
    finding.limit.insert(
        "expected_data_pair_gap_mm".to_string(),
        json!(evidence.expected_gap_mm),
    );
    finding.limit.insert(
        "max_data_pair_gap_delta_mm".to_string(),
        json!(max_gap_delta_mm),
    );
    finding.suggested_fixes = vec![
        "Route D+ and D- with the imported differential-pair gap or update the board rule if the impedance target changed.".to_string(),
        "Avoid local spreading or necking of only one member of the USB data pair unless captured by a more specific layout rule.".to_string(),
    ];
    finding
}

pub(super) fn usb_route_protection_distance_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    protection_component: &str,
    route_distance_mm: f64,
    max_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} reaches protection component {protection_component} after {:.3} mm of route, exceeding limit {:.3} mm.",
            signal.label(),
            route_distance_mm,
            max_route_distance_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding.measured.insert(
        "protection_component".to_string(),
        json!(protection_component),
    );
    finding.measured.insert(
        "connector_to_protection_route_distance_mm".to_string(),
        json!(route_distance_mm),
    );
    finding.limit.insert(
        "max_connector_to_protection_route_distance_mm".to_string(),
        json!(max_route_distance_mm),
    );
    finding.suggested_fixes = vec![
        "Move the ESD component closer to the USB connector along the routed data line.".to_string(),
        "Route connector pins through the protection device before continuing to the USB transceiver.".to_string(),
    ];
    finding
}

pub(super) fn usb_pair_length_mismatch_finding(
    scenario: &Scenario,
    connector_id: &str,
    evidence: UsbPairLengthEvidence<'_>,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} D+/D- route length mismatch {:.3} mm exceeds limit {:.3} mm.",
            evidence.length_mismatch_mm, evidence.max_length_mismatch_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding
        .measured
        .insert("dp_net".to_string(), json!(evidence.dp_net));
    finding
        .measured
        .insert("dm_net".to_string(), json!(evidence.dm_net));
    finding.measured.insert(
        "dp_route_length_mm".to_string(),
        json!(evidence.dp_length_mm),
    );
    finding.measured.insert(
        "dm_route_length_mm".to_string(),
        json!(evidence.dm_length_mm),
    );
    finding.measured.insert(
        "data_pair_length_mismatch_mm".to_string(),
        json!(evidence.length_mismatch_mm),
    );
    finding.limit.insert(
        "max_data_pair_length_mismatch_mm".to_string(),
        json!(evidence.max_length_mismatch_mm),
    );
    finding.suggested_fixes = vec![
        "Length-match the USB D+ and D- routes within the board's USB routing rule.".to_string(),
        "Route D+ and D- as a pair and avoid unnecessary jogs or detours on only one line."
            .to_string(),
    ];
    finding
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UsbPairLengthEvidence<'a> {
    pub(super) dp_net: &'a str,
    pub(super) dm_net: &'a str,
    pub(super) dp_length_mm: f64,
    pub(super) dm_length_mm: f64,
    pub(super) length_mismatch_mm: f64,
    pub(super) max_length_mismatch_mm: f64,
}

pub(super) fn usb_pair_via_delta_finding(
    scenario: &Scenario,
    connector_id: &str,
    evidence: UsbPairViaEvidence<'_>,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} D+/D- via-count delta {} exceeds limit {}.",
            evidence.via_count_delta, evidence.max_via_count_delta
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding
        .measured
        .insert("dp_net".to_string(), json!(evidence.dp_net));
    finding
        .measured
        .insert("dm_net".to_string(), json!(evidence.dm_net));
    finding
        .measured
        .insert("dp_via_count".to_string(), json!(evidence.dp_via_count));
    finding
        .measured
        .insert("dm_via_count".to_string(), json!(evidence.dm_via_count));
    finding.measured.insert(
        "data_pair_via_count_delta".to_string(),
        json!(evidence.via_count_delta),
    );
    finding.limit.insert(
        "max_data_pair_via_count_delta".to_string(),
        json!(evidence.max_via_count_delta),
    );
    finding.suggested_fixes = vec![
        "Keep D+ and D- layer changes symmetric when vias are unavoidable.".to_string(),
        "Remove unnecessary vias from one side of the USB pair or add the matching transition only when the layout stackup requires it.".to_string(),
    ];
    finding
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UsbPairViaEvidence<'a> {
    pub(super) dp_net: &'a str,
    pub(super) dm_net: &'a str,
    pub(super) dp_via_count: usize,
    pub(super) dm_via_count: usize,
    pub(super) via_count_delta: usize,
    pub(super) max_via_count_delta: usize,
}

pub(super) fn usb_return_path_metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(USB_RETURN_PATH_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Import KiCad PCB copper zone evidence with import-kicad-pcb before declaring USB_RETURN_PATH_VALID.".to_string(),
        "Declare max_data_line_unreferenced_length_mm from the board's USB return-path/layout rule.".to_string(),
    ];
    finding
}

pub(super) fn usb_return_path_unreferenced_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    evidence: UsbReturnPathEvidence<'_>,
) -> Finding {
    let mut finding = Finding::critical(
        USB_RETURN_PATH_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} has {:.3} mm of routed data-line length without same-layer ground-zone outline coverage, above limit {:.3} mm.",
            signal.label(),
            evidence.unreferenced_length_mm,
            evidence.max_unreferenced_length_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding.measured.insert(
        "unreferenced_route_length_mm".to_string(),
        json!(evidence.unreferenced_length_mm),
    );
    finding.measured.insert(
        "unreferenced_segments".to_string(),
        json!(evidence.unreferenced_segments),
    );
    finding.limit.insert(
        "max_data_line_unreferenced_length_mm".to_string(),
        json!(evidence.max_unreferenced_length_mm),
    );
    finding
        .limit
        .insert("reference_net_kind".to_string(), json!("ground"));
    finding.limit.insert(
        "reference_zone_layer_policy".to_string(),
        json!("same_layer"),
    );
    finding.suggested_fixes = vec![
        "Add or extend same-layer ground-zone reference coverage under the affected USB data-route segment, then import the updated PCB.".to_string(),
        "If the design intentionally references an adjacent plane or controlled stackup instead, model that return-path evidence with a more specific rule instead of using this same-layer outline check as sign-off.".to_string(),
    ];
    finding
}

pub(super) fn usb_return_path_stitch_via_finding(
    scenario: &Scenario,
    evidence: UsbReturnPathStitchViaEvidence<'_>,
) -> Finding {
    let nearest_distance = evidence.nearest.map(|candidate| candidate.distance_mm);
    let message = if let Some(distance_mm) = nearest_distance {
        format!(
            "USB connector {} {} net {} data via {} is {:.3} mm from the nearest matching ground stitch via, above limit {:.3} mm.",
            evidence.connector_id,
            evidence.signal.label(),
            evidence.net,
            evidence.data_via_index,
            distance_mm,
            evidence.max_distance_mm
        )
    } else {
        format!(
            "USB connector {} {} net {} data via {} has no matching ground stitch via evidence.",
            evidence.connector_id,
            evidence.signal.label(),
            evidence.net,
            evidence.data_via_index
        )
    };
    let mut finding = Finding::critical(USB_RETURN_PATH_VALID, &scenario.name, message);
    finding.component = Some(evidence.connector_id.to_string());
    finding.net = Some(evidence.net.to_string());
    finding.measured.insert(
        "connector_signal".to_string(),
        json!(evidence.signal.label()),
    );
    finding
        .measured
        .insert("data_via_index".to_string(), json!(evidence.data_via_index));
    finding.measured.insert(
        "data_via_x_mm".to_string(),
        json!(evidence.data_via.at.x_mm),
    );
    finding.measured.insert(
        "data_via_y_mm".to_string(),
        json!(evidence.data_via.at.y_mm),
    );
    finding.measured.insert(
        "data_via_layers".to_string(),
        json!(evidence.data_via.layers),
    );
    if let Some(candidate) = evidence.nearest {
        finding.measured.insert(
            "nearest_ground_stitch_net".to_string(),
            json!(candidate.ground_net),
        );
        finding.measured.insert(
            "nearest_ground_stitch_via_index".to_string(),
            json!(candidate.ground_via_index),
        );
        finding.measured.insert(
            "nearest_ground_stitch_distance_mm".to_string(),
            json!(candidate.distance_mm),
        );
    }
    finding.limit.insert(
        "max_data_via_to_ground_stitch_distance_mm".to_string(),
        json!(evidence.max_distance_mm),
    );
    finding
        .limit
        .insert("reference_net_kind".to_string(), json!("ground"));
    finding.limit.insert(
        "required_ground_stitch_layer_policy".to_string(),
        json!("same_via_layers"),
    );
    finding.suggested_fixes = vec![
        "Add a nearby ground stitching via that spans the same USB data-via layer transition, then import the updated PCB.".to_string(),
        "If the stackup uses a different controlled return-path transition, model that evidence with a more specific rule instead of using this stitching-via screen as sign-off.".to_string(),
    ];
    finding
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UsbReturnPathStitchViaEvidence<'a> {
    pub(super) connector_id: &'a str,
    pub(super) signal: UsbConnectorSignal,
    pub(super) net: &'a str,
    pub(super) data_via_index: usize,
    pub(super) data_via: &'a RouteVia,
    pub(super) nearest: Option<GroundStitchViaCandidate<'a>>,
    pub(super) max_distance_mm: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct UsbReturnPathSegmentEvidence {
    pub(super) segment_index: usize,
    pub(super) segment_length_mm: f64,
    pub(super) midpoint_x_mm: f64,
    pub(super) midpoint_y_mm: f64,
    pub(super) layer: String,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UsbReturnPathEvidence<'a> {
    pub(super) unreferenced_length_mm: f64,
    pub(super) max_unreferenced_length_mm: f64,
    pub(super) unreferenced_segments: &'a [UsbReturnPathSegmentEvidence],
}
