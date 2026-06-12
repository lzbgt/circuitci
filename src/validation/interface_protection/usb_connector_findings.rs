use crate::board_ir::NetKind;
use crate::reports::Finding;
use serde_json::json;

use super::{
    ResolvedUsbProtection, UsbConnectorSignal, UsbPlacementDistanceEvidence, placement_side_name,
};
use crate::board_ir::ComponentPlacement;
use crate::validation::{
    USB_CONNECTOR_ORIENTATION_VALID, USB_CONNECTOR_PROTECTION_VALID, USB_PROTECTION_PLACEMENT_VALID,
};

pub(super) fn usb_connector_metadata_finding(
    scenario: &crate::board_ir::Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(USB_CONNECTOR_PROTECTION_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Declare usb_connector metadata and connect every required USB connector pin before using this protection check.".to_string(),
        "Use explicit protection-clamp models on exposed USB nets instead of treating connector exposure as implicitly protected.".to_string(),
    ];
    finding
}

pub(super) fn usb_connector_missing_protection_finding(
    scenario: &crate::board_ir::Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    pin: &str,
    net: &str,
) -> Finding {
    let mut finding = Finding::critical(
        USB_CONNECTOR_PROTECTION_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} pin {pin} on net {net} has no valid protection clamp coverage.",
            signal.label()
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_pin".to_string(), json!(pin));
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .limit
        .insert("required_protection_clamp".to_string(), json!(true));
    finding.suggested_fixes = vec![
        format!(
            "Add a datasheet-backed ESD/protection component whose protected pin connects to USB connector {connector_id}.{pin} net {net}."
        ),
        "Place the protection device close to the USB connector in PCB layout and add explicit clamp-review scenarios for standoff voltage and capacitance.".to_string(),
    ];
    finding
}

pub(super) fn usb_connector_standoff_finding(
    scenario: &crate::board_ir::Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    protection: &ResolvedUsbProtection<'_>,
    working_voltage_max_v: f64,
    min_standoff_v: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_CONNECTOR_PROTECTION_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} is protected by {}.{}, but clamp standoff {:.6} V is below required {:.6} V.",
            signal.label(),
            protection.component_id,
            protection.clamp.name,
            working_voltage_max_v,
            min_standoff_v
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding.measured.insert(
        "protection_component".to_string(),
        json!(protection.component_id),
    );
    finding
        .measured
        .insert("protection_clamp".to_string(), json!(protection.clamp.name));
    finding.measured.insert(
        "reference_pin".to_string(),
        json!(protection.clamp.reference_pin),
    );
    finding.measured.insert(
        "reference_net".to_string(),
        json!(protection.reference_net_name),
    );
    finding.measured.insert(
        "reference_net_kind".to_string(),
        json!(net_kind_name(protection.reference_net_kind)),
    );
    finding.measured.insert(
        "working_voltage_max_V".to_string(),
        json!(working_voltage_max_v),
    );
    finding.limit.insert(
        "required_working_voltage_min_V".to_string(),
        json!(min_standoff_v),
    );
    finding.suggested_fixes = vec![
        "Select a protection device whose reverse standoff voltage covers the exposed USB connector operating voltage.".to_string(),
        "Use separate VBUS-rated protection for the VBUS pin when the data-line ESD part is not rated for the power rail.".to_string(),
    ];
    finding
}

pub(super) fn usb_connector_shield_ground_finding(
    scenario: &crate::board_ir::Scenario,
    connector_id: &str,
    shield_pin: &str,
    shield_net: &str,
    actual_kind: &NetKind,
) -> Finding {
    let actual = net_kind_name(actual_kind);
    let mut finding = Finding::critical(
        USB_CONNECTOR_PROTECTION_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} shield pin {shield_pin} is connected to {actual} net {shield_net}, expected ground because require_shield_ground is true."
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(shield_net.to_string());
    finding
        .measured
        .insert("shield_pin".to_string(), json!(shield_pin));
    finding
        .measured
        .insert("shield_net".to_string(), json!(shield_net));
    finding
        .measured
        .insert("shield_net_kind".to_string(), json!(actual));
    finding
        .limit
        .insert("required_shield_net_kind".to_string(), json!("ground"));
    finding.suggested_fixes = vec![
        "Connect the USB shield pin to a declared ground/chassis strategy net when require_shield_ground is used.".to_string(),
        "If the design intentionally uses an RC, ferrite, spark gap, or chassis-only shield strategy, model that strategy explicitly before using this simplified ground check.".to_string(),
    ];
    finding
}

pub(super) fn usb_placement_metadata_finding(
    scenario: &crate::board_ir::Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(USB_PROTECTION_PLACEMENT_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Add board.layout.placements entries with finite x_mm/y_mm for the USB connector and protection components.".to_string(),
        "Use placement data extracted from the PCB design before declaring USB_PROTECTION_PLACEMENT_VALID.".to_string(),
    ];
    finding
}

pub(super) fn usb_placement_missing_protection_finding(
    scenario: &crate::board_ir::Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    pin: &str,
    net: &str,
) -> Finding {
    let mut finding = Finding::critical(
        USB_PROTECTION_PLACEMENT_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} pin {pin} on net {net} has no valid protection component to place near the connector.",
            signal.label()
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_pin".to_string(), json!(pin));
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .limit
        .insert("required_protection_clamp".to_string(), json!(true));
    finding.suggested_fixes = vec![
        format!(
            "Add a datasheet-backed ESD/protection component on USB connector {connector_id}.{pin} net {net} before checking placement."
        ),
        "Then place the protection component close enough to the connector to satisfy max_connector_to_protection_distance_mm.".to_string(),
    ];
    finding
}

pub(super) fn usb_placement_missing_protection_placement_finding(
    scenario: &crate::board_ir::Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    missing_components: &[String],
) -> Finding {
    let mut finding = Finding::critical(
        USB_PROTECTION_PLACEMENT_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} has protection components but none have usable placement evidence.",
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
        json!(missing_components),
    );
    finding
        .limit
        .insert("required_protection_placement".to_string(), json!(true));
    finding.suggested_fixes = vec![
        "Add board.layout.placements entries for the USB protection component candidates."
            .to_string(),
        "Extract placement from the PCB layout instead of relying only on schematic connectivity."
            .to_string(),
    ];
    finding
}

pub(super) fn usb_placement_distance_finding(
    evidence: UsbPlacementDistanceEvidence<'_>,
) -> Finding {
    let connector_id = evidence.connector_id;
    let signal = evidence.signal;
    let net = evidence.net;
    let protection = evidence.protection;
    let connector_placement = evidence.connector_placement;
    let protection_placement = evidence.protection_placement;
    let distance_mm = evidence.distance_mm;
    let max_distance_mm = evidence.max_distance_mm;
    let mut finding = Finding::critical(
        USB_PROTECTION_PLACEMENT_VALID,
        &evidence.scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} is protected by {}.{}, but placement distance {:.3} mm exceeds limit {:.3} mm.",
            signal.label(),
            protection.component_id,
            protection.clamp.name,
            distance_mm,
            max_distance_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding.measured.insert(
        "protection_component".to_string(),
        json!(protection.component_id),
    );
    finding
        .measured
        .insert("protection_clamp".to_string(), json!(protection.clamp.name));
    finding
        .measured
        .insert("distance_mm".to_string(), json!(distance_mm));
    finding.measured.insert(
        "connector_x_mm".to_string(),
        json!(connector_placement.x_mm),
    );
    finding.measured.insert(
        "connector_y_mm".to_string(),
        json!(connector_placement.y_mm),
    );
    if let Some(side) = placement_side_name(&connector_placement.side) {
        finding
            .measured
            .insert("connector_side".to_string(), json!(side));
    }
    finding.measured.insert(
        "protection_x_mm".to_string(),
        json!(protection_placement.x_mm),
    );
    finding.measured.insert(
        "protection_y_mm".to_string(),
        json!(protection_placement.y_mm),
    );
    if let Some(side) = placement_side_name(&protection_placement.side) {
        finding
            .measured
            .insert("protection_side".to_string(), json!(side));
    }
    finding.limit.insert(
        "max_connector_to_protection_distance_mm".to_string(),
        json!(max_distance_mm),
    );
    finding.suggested_fixes = vec![
        format!(
            "Move protection component {} closer to USB connector {connector_id} on the protected net {net}.",
            protection.component_id
        ),
        "Keep the ESD current path short and low-inductance; use PCB/layout review for trace order, via count, return path, and shield strategy.".to_string(),
    ];
    finding
}

pub(super) fn usb_orientation_metadata_finding(
    scenario: &crate::board_ir::Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(USB_CONNECTOR_ORIENTATION_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Import PCB component placement rotation with import-kicad-pcb before declaring USB_CONNECTOR_ORIENTATION_VALID.".to_string(),
        "Use explicit board-edge or mechanical review evidence to set expected_connector_rotation_deg and max_connector_rotation_error_deg.".to_string(),
    ];
    finding
}

pub(super) fn usb_orientation_finding(
    scenario: &crate::board_ir::Scenario,
    connector_id: &str,
    placement: &ComponentPlacement,
    actual_rotation_deg: f64,
    expected_rotation_deg: f64,
    rotation_error_deg: f64,
    max_error_deg: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_CONNECTOR_ORIENTATION_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} placement rotation {:.3} deg differs from expected {:.3} deg by {:.3} deg, exceeding limit {:.3} deg.",
            actual_rotation_deg, expected_rotation_deg, rotation_error_deg, max_error_deg
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.measured.insert(
        "connector_rotation_deg".to_string(),
        json!(actual_rotation_deg),
    );
    finding.measured.insert(
        "connector_rotation_error_deg".to_string(),
        json!(rotation_error_deg),
    );
    finding
        .measured
        .insert("connector_x_mm".to_string(), json!(placement.x_mm));
    finding
        .measured
        .insert("connector_y_mm".to_string(), json!(placement.y_mm));
    if let Some(side) = placement_side_name(&placement.side) {
        finding
            .measured
            .insert("connector_side".to_string(), json!(side));
    }
    finding.limit.insert(
        "expected_connector_rotation_deg".to_string(),
        json!(expected_rotation_deg),
    );
    finding.limit.insert(
        "max_connector_rotation_error_deg".to_string(),
        json!(max_error_deg),
    );
    finding.suggested_fixes = vec![
        format!(
            "Rotate or remap USB connector {connector_id} footprint so its imported placement rotation matches the intended board-edge orientation."
        ),
        "If the board intentionally uses a different connector entry direction, update expected_connector_rotation_deg from the mechanical/layout rule and rerun validation.".to_string(),
    ];
    finding
}

fn net_kind_name(kind: &NetKind) -> &'static str {
    match kind {
        NetKind::Power => "power",
        NetKind::Ground => "ground",
        NetKind::DigitalOrAnalog => "digital_or_analog",
    }
}
