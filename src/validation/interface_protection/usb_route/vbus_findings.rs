use crate::reports::Finding;
use crate::validation::USB_VBUS_ROUTE_VALID;
use serde_json::json;

use crate::board_ir::Scenario;

pub(super) fn usb_vbus_route_metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(USB_VBUS_ROUTE_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Import PCB route geometry with import-kicad-pcb before declaring USB_VBUS_ROUTE_VALID."
            .to_string(),
        "Declare VBUS route limits from the board USB power/layout rule instead of inferring them from coordinates.".to_string(),
    ];
    finding
}

pub(super) fn usb_vbus_route_length_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    route_length_mm: f64,
    max_route_length_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} route length {:.3} mm exceeds limit {:.3} mm.",
            route_length_mm, max_route_length_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding
        .measured
        .insert("route_length_mm".to_string(), json!(route_length_mm));
    finding.limit.insert(
        "max_vbus_route_length_mm".to_string(),
        json!(max_route_length_mm),
    );
    finding.suggested_fixes = vec![
        "Shorten the USB VBUS route or move the connector/protection/power-entry path closer together.".to_string(),
        "Use a board-specific USB power-layout rule for max_vbus_route_length_mm.".to_string(),
    ];
    finding
}

pub(super) fn usb_vbus_route_via_count_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    via_count: usize,
    max_via_count: usize,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} has {via_count} vias, above limit {max_via_count}."
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding
        .measured
        .insert("via_count".to_string(), json!(via_count));
    finding
        .limit
        .insert("max_vbus_via_count".to_string(), json!(max_via_count));
    finding.suggested_fixes = vec![
        "Reduce USB VBUS layer changes before the protection/power-entry stage or relax max_vbus_via_count only with layout justification.".to_string(),
        "Use a separate power-path/current-capacity review for VBUS copper sizing and fuse behavior.".to_string(),
    ];
    finding
}

pub(super) fn usb_vbus_route_width_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    segment_index: usize,
    measured_width_mm: f64,
    min_width_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} segment {segment_index} width {:.3} mm is below minimum {:.3} mm.",
            measured_width_mm, min_width_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding
        .measured
        .insert("segment_index".to_string(), json!(segment_index));
    finding.measured.insert(
        "route_segment_width_mm".to_string(),
        json!(measured_width_mm),
    );
    finding
        .limit
        .insert("min_vbus_route_width_mm".to_string(), json!(min_width_mm));
    finding.suggested_fixes = vec![
        "Widen the USB VBUS route to satisfy the board's USB power-entry layout rule.".to_string(),
        "Keep current-capacity and temperature-rise sign-off in a separate power-layout review."
            .to_string(),
    ];
    finding
}

pub(super) fn usb_vbus_route_no_protection_path_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    missing_placements: &[String],
    off_route_components: &[String],
    max_component_to_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} has no protection component with usable route-distance evidence."
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
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
        "Place the USB VBUS protection component on the routed VBUS net near the connector and import updated PCB route geometry.".to_string(),
        "Check that component placement coordinates and route coordinates share the same PCB coordinate system.".to_string(),
    ];
    finding
}

pub(super) fn usb_vbus_route_pad_metadata_finding(
    scenario: &Scenario,
    evidence: UsbVbusRoutePadMetadataEvidence<'_>,
    message: String,
) -> Finding {
    let mut finding = Finding::critical(USB_VBUS_ROUTE_VALID, &scenario.name, message);
    finding.component = Some(evidence.connector_id.to_string());
    finding.net = Some(evidence.net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding
        .measured
        .insert("pad_component".to_string(), json!(evidence.pad_component));
    finding
        .measured
        .insert("pad_pin".to_string(), json!(evidence.pad_pin));
    finding
        .limit
        .insert(evidence.field.to_string(), json!(evidence.pad_pin));
    finding.limit.insert(
        "vbus_route_pad_contact_policy".to_string(),
        json!("same_net_pad_center_on_route"),
    );
    finding.suggested_fixes = vec![
        "Import PCB pad evidence with import-kicad-pcb before enabling require_vbus_route_pad_contact_evidence.".to_string(),
        "Check that the USB connector and VBUS protection footprint pad names match the component model pins and that both pads share the routed VBUS net.".to_string(),
    ];
    finding
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UsbVbusRoutePadMetadataEvidence<'a> {
    pub(super) connector_id: &'a str,
    pub(super) net: &'a str,
    pub(super) pad_component: &'a str,
    pub(super) pad_pin: &'a str,
    pub(super) field: &'a str,
}

pub(super) fn usb_vbus_route_no_protection_pad_path_finding(
    scenario: &Scenario,
    evidence: UsbVbusRoutePadPathEvidence<'_>,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {} VBUS net {} has no protection pad with usable route-distance evidence.",
            evidence.connector_id, evidence.net
        ),
    );
    finding.component = Some(evidence.connector_id.to_string());
    finding.net = Some(evidence.net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding
        .measured
        .insert("connector_pad".to_string(), json!(evidence.connector_pin));
    finding.measured.insert(
        "vbus_protection_pads_missing".to_string(),
        json!(evidence.missing_pads),
    );
    finding.measured.insert(
        "vbus_protection_pads_off_route".to_string(),
        json!(evidence.off_route_pads),
    );
    finding.limit.insert(
        "max_component_to_route_distance_mm".to_string(),
        json!(evidence.max_pad_to_route_distance_mm),
    );
    finding.limit.insert(
        "vbus_route_pad_contact_policy".to_string(),
        json!("same_net_pad_center_on_route"),
    );
    finding.suggested_fixes = vec![
        "Route connector VBUS through the protection device pad before continuing downstream, then import updated PCB route and pad evidence.".to_string(),
        "Check VBUS protection footprint pad names, net names, and copper layers when the protection pad exists but is not on the imported route.".to_string(),
    ];
    finding
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UsbVbusRoutePadPathEvidence<'a> {
    pub(super) connector_id: &'a str,
    pub(super) net: &'a str,
    pub(super) connector_pin: &'a str,
    pub(super) missing_pads: &'a [String],
    pub(super) off_route_pads: &'a [String],
    pub(super) max_pad_to_route_distance_mm: f64,
}

pub(super) fn usb_vbus_route_protection_distance_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    protection_component: &str,
    route_distance_mm: f64,
    max_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} reaches protection component {protection_component} after {:.3} mm of route, exceeding limit {:.3} mm.",
            route_distance_mm, max_route_distance_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding.measured.insert(
        "protection_component".to_string(),
        json!(protection_component),
    );
    finding.measured.insert(
        "connector_to_vbus_protection_route_distance_mm".to_string(),
        json!(route_distance_mm),
    );
    finding.limit.insert(
        "max_connector_to_vbus_protection_route_distance_mm".to_string(),
        json!(max_route_distance_mm),
    );
    finding.suggested_fixes = vec![
        "Move the VBUS protection component closer to the USB connector along the routed VBUS path.".to_string(),
        "Route connector VBUS through the protection/power-entry component before continuing to downstream loads.".to_string(),
    ];
    finding
}

pub(super) fn usb_vbus_route_protection_pad_distance_finding(
    scenario: &Scenario,
    connector_id: &str,
    net: &str,
    evidence: UsbVbusRoutePadDistanceEvidence<'_>,
) -> Finding {
    let mut finding = Finding::critical(
        USB_VBUS_ROUTE_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} VBUS net {net} reaches protection pad {}.{} after {:.3} mm of route, exceeding limit {:.3} mm.",
            evidence.protection_component,
            evidence.protection_pin,
            evidence.route_distance_mm,
            evidence.max_route_distance_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!("VBUS"));
    finding
        .measured
        .insert("connector_pad".to_string(), json!(evidence.connector_pin));
    finding.measured.insert(
        "protection_component".to_string(),
        json!(evidence.protection_component),
    );
    finding
        .measured
        .insert("protection_pad".to_string(), json!(evidence.protection_pin));
    finding.measured.insert(
        "connector_to_vbus_protection_route_distance_mm".to_string(),
        json!(evidence.route_distance_mm),
    );
    finding.limit.insert(
        "max_connector_to_vbus_protection_route_distance_mm".to_string(),
        json!(evidence.max_route_distance_mm),
    );
    finding.limit.insert(
        "vbus_route_pad_contact_policy".to_string(),
        json!("same_net_pad_center_on_route"),
    );
    finding.suggested_fixes = vec![
        "Move the VBUS protection pad closer to the USB connector VBUS pad along the routed VBUS path.".to_string(),
        "Route connector VBUS through the protection/power-entry pad before continuing to downstream loads.".to_string(),
    ];
    finding
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UsbVbusRoutePadDistanceEvidence<'a> {
    pub(super) connector_pin: &'a str,
    pub(super) protection_component: &'a str,
    pub(super) protection_pin: &'a str,
    pub(super) route_distance_mm: f64,
    pub(super) max_route_distance_mm: f64,
}
