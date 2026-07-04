mod common;

use common::{assert_report_schema_valid, run_validation};

#[test]
fn good_interface_protection_powered_passes() {
    let report = run_validation("examples/good_interface_protection_powered/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn interface_protection_unisolated_power_domains_fail() {
    let report = run_validation("examples/bad_interface_protection_unisolated/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "U3");
    assert_eq!(failure["measured"]["side_a_powered"], false);
    assert_eq!(failure["measured"]["side_b_powered"], true);
    assert_eq!(failure["limit"]["required_unpowered_isolation"], true);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_txs0108e_unpowered_side_requires_isolation_or_oe_evidence() {
    let report = run_validation("examples/bad_ti_txs0108e_unpowered_side/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "U3");
    assert_eq!(failure["measured"]["side_a_supply_net"], "rail_a");
    assert_eq!(failure["measured"]["side_a_powered"], false);
    assert_eq!(failure["measured"]["side_b_supply_net"], "rail_b");
    assert_eq!(failure["measured"]["side_b_powered"], true);
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("does not prove the channel is disabled")
    );
    assert_eq!(failure["limit"]["enable_pin"], "OE");
    assert_eq!(failure["limit"]["required_disabled_state"], "low");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_txs0108e_oe_low_allows_one_sided_power_review() {
    let report = run_validation("examples/good_ti_txs0108e_oe_low_unpowered_side/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_txs0108e_oe_low_requires_connected_enable_pin() {
    let report = run_validation("examples/bad_ti_txs0108e_oe_low_unconnected/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == "INTERFACE_PROTECTION_REVIEW")
        .expect("interface protection finding");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("does not prove the channel is disabled")
    );
    assert_eq!(failure["limit"]["enable_pin"], "OE");
    assert_eq!(failure["limit"]["required_disabled_state"], "low");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_txs0108e_supply_order_requires_vcca_not_above_vccb() {
    let report = run_validation("examples/bad_ti_txs0108e_supply_order/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["supply_constraint"] == "vcca_lte_vccb")
        .expect("supply-order finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "U3");
    assert_eq!(failure["measured"]["lower_supply_pin"], "VCCA");
    assert_eq!(failure["measured"]["upper_supply_pin"], "VCCB");
    assert_eq!(failure["measured"]["lower_nominal_voltage_V"], 5.0);
    assert_eq!(failure["measured"]["upper_nominal_voltage_V"], 3.3);
    assert_eq!(failure["limit"]["relation"], "less_than_or_equal");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_esd_clamp_protection_passes_static_review() {
    let report = run_validation("examples/good_usb_esd_protection/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_esd_clamp_requires_declared_reference_net_kind() {
    let report = run_validation("examples/bad_usb_esd_reference/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["required_reference"] == "ground")
        .expect("ESD reference finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_shield");
    assert_eq!(
        failure["measured"]["reference_net_kind"],
        "digital_or_analog"
    );
    assert_eq!(failure["limit"]["protection_clamp"], "dp");
    assert_eq!(failure["limit"]["reference_pin"], "GND");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_esd_clamp_requires_standoff_above_protected_net_voltage() {
    let report = run_validation("examples/bad_usb_esd_standoff/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["working_voltage_max_V"] == 5.5)
        .expect("ESD standoff finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["protected_net_nominal_voltage_V"], 6.0);
    assert_eq!(failure["limit"]["protection_clamp"], "dp");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_esd_clamp_capacitance_must_fit_interface_budget() {
    let report = run_validation("examples/bad_usb_esd_line_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["max_line_capacitance_F"] == 2.0e-12)
        .expect("ESD capacitance finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["line_capacitance_F"], 1.0e-11);
    assert_eq!(failure["limit"]["protection_clamp"], "dp");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_protection_passes_when_data_and_vbus_are_clamped() {
    let report = run_validation("examples/good_usb_connector_protection/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_protection_requires_data_line_clamps() {
    let report = run_validation("examples/bad_usb_connector_missing_data_protection/project.yaml");
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    let dp = failures
        .iter()
        .find(|finding| finding["net"] == "usb_dp")
        .expect("D+ missing protection finding");
    assert_eq!(dp["id"], "USB_CONNECTOR_PROTECTION_VALID");
    assert_eq!(dp["component"], "J1");
    assert_eq!(dp["measured"]["connector_signal"], "D+");
    assert_eq!(dp["limit"]["required_protection_clamp"], true);
    let dm = failures
        .iter()
        .find(|finding| finding["net"] == "usb_dm")
        .expect("D- missing protection finding");
    assert_eq!(dm["id"], "USB_CONNECTOR_PROTECTION_VALID");
    assert_eq!(dm["measured"]["connector_signal"], "D-");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_protection_requires_vbus_clamp_when_requested() {
    let report = run_validation("examples/bad_usb_connector_missing_vbus_protection/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["net"] == "usb_vbus")
        .expect("VBUS missing protection finding");
    assert_eq!(failure["id"], "USB_CONNECTOR_PROTECTION_VALID");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["measured"]["connector_signal"], "VBUS");
    assert_eq!(failure["measured"]["connector_pin"], "VBUS");
    assert_eq!(failure["limit"]["required_protection_clamp"], true);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_protection_accepts_grounded_shield_when_requested() {
    let report = run_validation("examples/good_usb_connector_shield_ground/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_protection_requires_grounded_shield_when_requested() {
    let report = run_validation("examples/bad_usb_connector_shield_not_ground/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["required_shield_net_kind"] == "ground")
        .expect("shield grounding finding");
    assert_eq!(failure["id"], "USB_CONNECTOR_PROTECTION_VALID");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["net"], "usb_shield");
    assert_eq!(failure["measured"]["shield_pin"], "SHIELD");
    assert_eq!(failure["measured"]["shield_net_kind"], "digital_or_analog");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_protection_placement_passes_when_clamps_are_close() {
    let report = run_validation("examples/good_usb_connector_protection_placement/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_protection_placement_requires_close_data_clamps() {
    let report =
        run_validation("examples/bad_usb_connector_protection_placement_distance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    let dp = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_PROTECTION_PLACEMENT_VALID"
                && failure["measured"]["connector_signal"] == "D+"
        })
        .expect("D+ placement finding");
    assert_eq!(dp["component"], "J1");
    assert_eq!(dp["net"], "usb_dp");
    assert_eq!(dp["measured"]["protection_component"], "UESD");
    assert_eq!(dp["measured"]["distance_mm"], 6.0);
    assert_eq!(dp["limit"]["max_connector_to_protection_distance_mm"], 2.0);
    let dm = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_PROTECTION_PLACEMENT_VALID"
                && failure["measured"]["connector_signal"] == "D-"
        })
        .expect("D- placement finding");
    assert_eq!(dm["net"], "usb_dm");
    assert_eq!(dm["measured"]["distance_mm"], 6.0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_orientation_passes_within_rotation_tolerance() {
    let report = run_validation("examples/good_usb_connector_orientation/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_orientation_reports_rotation_mismatch() {
    let report = run_validation("examples/bad_usb_connector_orientation/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_ORIENTATION_VALID")
        .expect("USB connector orientation finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["measured"]["connector_rotation_deg"], 180.0);
    assert_eq!(failure["measured"]["connector_rotation_error_deg"], 180.0);
    assert_eq!(failure["limit"]["expected_connector_rotation_deg"], 0.0);
    assert_eq!(failure["limit"]["max_connector_rotation_error_deg"], 5.0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_edge_proximity_passes_when_close_to_board_edge() {
    let report = run_validation("examples/good_usb_connector_edge_proximity/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_edge_proximity_uses_footprint_body_when_available() {
    let report =
        run_validation("examples/good_usb_connector_edge_proximity_footprint/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_edge_proximity_reports_distant_connector() {
    let report = run_validation("examples/bad_usb_connector_edge_proximity/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_EDGE_PROXIMITY_VALID")
        .expect("USB connector edge proximity finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(
        failure["measured"]["connector_to_board_edge_distance_mm"],
        1.4
    );
    assert_eq!(
        failure["measured"]["connector_edge_reference"],
        "placement_center"
    );
    assert_eq!(failure["measured"]["board_edge_layer"], "Edge.Cuts");
    assert_eq!(failure["measured"]["board_edge_start_x_mm"], -0.4);
    assert_eq!(
        failure["limit"]["max_connector_to_board_edge_distance_mm"],
        0.5
    );
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_edge_proximity_reports_footprint_reference() {
    let report = run_validation("examples/bad_usb_connector_edge_proximity_footprint/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_EDGE_PROXIMITY_VALID")
        .expect("USB connector edge proximity finding");
    assert_eq!(failure["component"], "J1");
    let distance = failure["measured"]["connector_to_board_edge_distance_mm"]
        .as_f64()
        .unwrap();
    assert!((distance - 1.2).abs() < 1e-12);
    assert_eq!(
        failure["measured"]["connector_edge_reference"],
        "footprint_polygon"
    );
    assert_eq!(failure["measured"]["footprint_graphic_layer"], "F.CrtYd");
    assert_eq!(failure["measured"]["footprint_graphic_kind"], "courtyard");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_body_overhang_passes_when_within_limit() {
    let report = run_validation("examples/good_usb_connector_body_overhang/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_body_overhang_reports_excess_body_past_edge() {
    let report = run_validation("examples/bad_usb_connector_body_overhang/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_BODY_OVERHANG_VALID")
        .expect("USB connector body overhang finding");
    assert_eq!(failure["component"], "J1");
    let overhang = failure["measured"]["connector_body_overhang_mm"]
        .as_f64()
        .unwrap();
    assert!((overhang - 0.05).abs() < 1e-12);
    assert_eq!(
        failure["measured"]["connector_edge_reference"],
        "footprint_polygon"
    );
    assert_eq!(failure["measured"]["footprint_graphic_layer"], "F.CrtYd");
    assert_eq!(failure["measured"]["footprint_graphic_kind"], "courtyard");
    assert_eq!(failure["measured"]["board_edge_layer"], "Edge.Cuts");
    assert_eq!(failure["measured"]["edge_angle_deg"], 90.0);
    assert_eq!(failure["measured"]["outward_normal_deg"], 180.0);
    assert_eq!(failure["limit"]["max_connector_body_overhang_mm"], 0.02);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_body_overhang_reports_circle_reference() {
    let report = run_validation("examples/bad_usb_connector_body_overhang_circle/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_BODY_OVERHANG_VALID")
        .expect("USB connector circle body overhang finding");
    assert_eq!(failure["component"], "J1");
    let overhang = failure["measured"]["connector_body_overhang_mm"]
        .as_f64()
        .unwrap();
    assert!((overhang - 0.15).abs() < 1.0e-12);
    assert_eq!(
        failure["measured"]["connector_edge_reference"],
        "footprint_circle"
    );
    assert_eq!(failure["measured"]["footprint_graphic_layer"], "F.Fab");
    assert_eq!(failure["measured"]["footprint_graphic_kind"], "fabrication");
    assert_eq!(failure["limit"]["max_connector_body_overhang_mm"], 0.1);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_component_clearance_passes_when_neighbor_is_clear() {
    let report = run_validation("examples/good_usb_connector_component_clearance/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_component_clearance_reports_nearby_component() {
    let report = run_validation("examples/bad_usb_connector_component_clearance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_COMPONENT_CLEARANCE_VALID")
        .expect("USB connector component-clearance finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["measured"]["nearby_component"], "R1");
    let clearance = failure["measured"]["connector_to_component_clearance_mm"]
        .as_f64()
        .unwrap();
    assert!((clearance - 0.3).abs() < 1e-12);
    assert_eq!(
        failure["measured"]["connector_clearance_reference"],
        "footprint_polygon"
    );
    assert_eq!(
        failure["measured"]["nearby_component_clearance_reference"],
        "footprint_rectangle"
    );
    assert_eq!(
        failure["measured"]["nearby_component_footprint_graphic_kind"],
        "courtyard"
    );
    assert_eq!(
        failure["limit"]["min_connector_to_component_clearance_mm"],
        0.5
    );
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_entry_clearance_passes_when_corridor_is_clear() {
    let report = run_validation("examples/good_usb_connector_entry_clearance/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_entry_clearance_reports_obstruction() {
    let report = run_validation("examples/bad_usb_connector_entry_clearance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_ENTRY_CLEARANCE_VALID")
        .expect("USB connector entry-clearance finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["measured"]["obstructing_component"], "R1");
    assert_eq!(failure["measured"]["entry_direction_deg"], 0.0);
    assert_eq!(
        failure["measured"]["entry_direction_source"],
        "scenario_parameter"
    );
    assert!(failure["measured"]["entry_direction_offset_deg"].is_null());
    assert_eq!(
        failure["measured"]["obstruction_reference"],
        "footprint_rectangle"
    );
    assert_eq!(
        failure["measured"]["obstruction_footprint_graphic_kind"],
        "courtyard"
    );
    assert_eq!(failure["limit"]["min_cable_entry_clearance_depth_mm"], 2.0);
    assert_eq!(failure["limit"]["cable_entry_clearance_width_mm"], 1.0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_entry_clearance_uses_model_entry_direction_offset() {
    let report =
        run_validation("examples/bad_usb_connector_entry_clearance_model_offset/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_ENTRY_CLEARANCE_VALID")
        .expect("USB connector entry-clearance finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["measured"]["obstructing_component"], "R1");
    assert_eq!(failure["measured"]["entry_direction_deg"], 0.0);
    assert_eq!(
        failure["measured"]["entry_direction_source"],
        "component_model_offset"
    );
    assert_eq!(failure["measured"]["entry_direction_offset_deg"], 90.0);
    assert_eq!(
        failure["measured"]["obstruction_reference"],
        "footprint_rectangle"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn usb_connector_entry_clearance_uses_model_aperture_geometry() {
    let report = run_validation("examples/bad_usb_connector_entry_clearance_aperture/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_ENTRY_CLEARANCE_VALID")
        .expect("USB connector entry-clearance finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["measured"]["obstructing_component"], "R1");
    assert_eq!(
        failure["measured"]["entry_aperture_source"],
        "component_model_aperture"
    );
    assert_eq!(failure["measured"]["connector_front_projection_mm"], 0.5);
    assert_eq!(
        failure["measured"]["entry_aperture_front_projection_mm"],
        0.75
    );
    assert_eq!(
        failure["measured"]["entry_aperture_center_lateral_projection_mm"],
        1.0
    );
    assert_eq!(failure["measured"]["entry_aperture_front_offset_mm"], 0.25);
    assert_eq!(failure["measured"]["entry_aperture_lateral_offset_mm"], 1.0);
    assert_eq!(failure["measured"]["entry_aperture_width_mm"], 0.5);
    assert_eq!(
        failure["measured"]["aperture_min_effective_clearance_width_mm"],
        0.5
    );
    assert_eq!(
        failure["measured"]["effective_cable_entry_clearance_width_mm"],
        0.5
    );
    assert_eq!(failure["limit"]["cable_entry_clearance_width_mm"], 0.2);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_route_geometry_passes_for_short_data_routes() {
    let report = run_validation("examples/good_usb_connector_route_geometry/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_route_geometry_reports_length_vias_and_protection_order() {
    let report = run_validation("examples/bad_usb_connector_route_geometry/project.yaml");
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    let dp_length = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_ROUTE_GEOMETRY_VALID"
                && failure["net"] == "usb_dp"
                && failure["measured"]["route_length_mm"] == 6.0
        })
        .expect("D+ route length finding");
    assert_eq!(dp_length["component"], "J1");
    assert_eq!(dp_length["measured"]["connector_signal"], "D+");
    assert_eq!(dp_length["limit"]["max_data_line_route_length_mm"], 5.0);
    let dp_protection_distance = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_ROUTE_GEOMETRY_VALID"
                && failure["net"] == "usb_dp"
                && failure["measured"]["connector_to_protection_route_distance_mm"] == 6.0
        })
        .expect("D+ protection route distance finding");
    assert_eq!(
        dp_protection_distance["measured"]["protection_component"],
        "UESD"
    );
    assert_eq!(dp_protection_distance["measured"]["connector_pad"], "D+");
    assert_eq!(dp_protection_distance["measured"]["protection_pad"], "DP");
    assert_eq!(
        dp_protection_distance["limit"]["max_connector_to_protection_route_distance_mm"],
        2.0
    );
    assert_eq!(
        dp_protection_distance["limit"]["route_pad_contact_policy"],
        "same_net_pad_center_on_route"
    );
    let dm_vias = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_ROUTE_GEOMETRY_VALID"
                && failure["net"] == "usb_dm"
                && failure["measured"]["via_count"] == 2
        })
        .expect("D- via count finding");
    assert_eq!(dm_vias["measured"]["connector_signal"], "D-");
    assert_eq!(dm_vias["limit"]["max_data_line_via_count"], 0);
    let dp_width = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_ROUTE_GEOMETRY_VALID"
                && failure["net"] == "usb_dp"
                && failure["measured"]["route_segment_width_mm"] == 0.20
        })
        .expect("D+ route width finding");
    assert_eq!(dp_width["measured"]["connector_signal"], "D+");
    let route_width_delta = dp_width["measured"]["route_width_delta_mm"]
        .as_f64()
        .unwrap();
    assert!((route_width_delta - 0.05).abs() < 1e-12);
    assert_eq!(dp_width["limit"]["expected_data_line_width_mm"], 0.15);
    assert_eq!(dp_width["limit"]["max_data_line_width_delta_mm"], 0.01);
    let pair_length = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_ROUTE_GEOMETRY_VALID"
                && failure["measured"]["data_pair_length_mismatch_mm"] == 5.0
        })
        .expect("D+/D- length mismatch finding");
    assert_eq!(pair_length["component"], "J1");
    assert_eq!(pair_length["measured"]["dp_net"], "usb_dp");
    assert_eq!(pair_length["measured"]["dm_net"], "usb_dm");
    assert_eq!(pair_length["measured"]["dp_route_length_mm"], 6.0);
    assert_eq!(pair_length["measured"]["dm_route_length_mm"], 1.0);
    assert_eq!(
        pair_length["limit"]["max_data_pair_length_mismatch_mm"],
        0.5
    );
    let pair_vias = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_ROUTE_GEOMETRY_VALID"
                && failure["measured"]["data_pair_via_count_delta"] == 2
        })
        .expect("D+/D- via-count delta finding");
    assert_eq!(pair_vias["measured"]["dp_via_count"], 0);
    assert_eq!(pair_vias["measured"]["dm_via_count"], 2);
    assert_eq!(pair_vias["limit"]["max_data_pair_via_count_delta"], 0);
    let pair_gap = failures
        .iter()
        .find(|failure| failure["limit"]["max_data_pair_gap_delta_mm"] == 0.01)
        .expect("D+/D- gap delta finding");
    assert_eq!(
        pair_gap["measured"]["data_pair_centerline_distance_mm"],
        0.5
    );
    let measured_gap = pair_gap["measured"]["data_pair_gap_mm"].as_f64().unwrap();
    assert!((measured_gap - 0.325).abs() < 1e-12);
    let gap_delta = pair_gap["measured"]["data_pair_gap_delta_mm"]
        .as_f64()
        .unwrap();
    assert!((gap_delta - 0.175).abs() < 1e-12);
    assert_eq!(pair_gap["limit"]["expected_data_pair_gap_mm"], 0.15);
    assert_eq!(pair_gap["limit"]["max_data_pair_gap_delta_mm"], 0.01);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_return_path_passes_when_data_routes_have_ground_zone_coverage() {
    let report = run_validation("examples/good_usb_return_path/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_return_path_passes_with_imported_ground_pad_contact_evidence() {
    let report = run_validation("examples/good_usb_return_path_pad_contact/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_return_path_reports_unreferenced_data_route_length() {
    let report = run_validation("examples/bad_usb_return_path/project.yaml");
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    let failure = failures
        .iter()
        .find(|failure| failure["id"] == "USB_RETURN_PATH_VALID")
        .expect("USB return-path finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["connector_signal"], "D+");
    assert_eq!(failure["measured"]["unreferenced_route_length_mm"], 1.0);
    assert_eq!(
        failure["limit"]["max_data_line_unreferenced_length_mm"],
        0.0
    );
    assert_eq!(failure["limit"]["reference_net_kind"], "ground");
    assert_eq!(failure["limit"]["reference_zone_geometry"], "outline");
    assert_eq!(
        failure["limit"]["reference_zone_layer_policy"],
        "same_layer"
    );
    let segments = failure["measured"]["unreferenced_segments"]
        .as_array()
        .unwrap();
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0]["segment_index"], 0);
    assert_eq!(segments[0]["midpoint_x_mm"], 0.5);
    assert_eq!(segments[0]["midpoint_y_mm"], 0.0);
    assert_eq!(segments[0]["layer"], "F.Cu");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_return_path_reports_filled_zone_gap_when_required() {
    let report = run_validation("examples/bad_usb_return_path_filled_zone_gap/project.yaml");
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    let failure = failures
        .iter()
        .find(|failure| failure["id"] == "USB_RETURN_PATH_VALID")
        .expect("USB return-path filled-zone finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["connector_signal"], "D+");
    assert_eq!(failure["measured"]["unreferenced_route_length_mm"], 1.0);
    assert_eq!(
        failure["limit"]["max_data_line_unreferenced_length_mm"],
        0.0
    );
    assert_eq!(
        failure["limit"]["reference_zone_geometry"],
        "filled_polygon"
    );
    assert_eq!(
        failure["limit"]["reference_zone_layer_policy"],
        "same_layer"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn usb_return_path_reports_floating_filled_zone_when_contact_required() {
    let report = run_validation("examples/bad_usb_return_path_floating_zone/project.yaml");
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    let failure = failures
        .iter()
        .find(|failure| failure["id"] == "USB_RETURN_PATH_VALID")
        .expect("USB return-path floating-zone finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["connector_signal"], "D+");
    assert_eq!(failure["measured"]["unreferenced_route_length_mm"], 1.0);
    assert_eq!(
        failure["limit"]["reference_zone_geometry"],
        "filled_polygon"
    );
    assert_eq!(
        failure["limit"]["reference_zone_contact_policy"],
        "same_net_pad_or_via"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn usb_return_path_reports_split_filled_zone_contact_when_same_island_required() {
    let report =
        run_validation("examples/bad_usb_return_path_split_filled_zone_contact/project.yaml");
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    let failure = failures
        .iter()
        .find(|failure| failure["id"] == "USB_RETURN_PATH_VALID")
        .expect("USB return-path split filled-zone contact finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["connector_signal"], "D+");
    assert_eq!(failure["measured"]["unreferenced_route_length_mm"], 1.0);
    assert_eq!(
        failure["limit"]["reference_zone_geometry"],
        "filled_polygon"
    );
    assert_eq!(
        failure["limit"]["reference_zone_contact_policy"],
        "same_net_pad_or_via"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn usb_return_path_reports_low_filled_zone_edge_clearance() {
    let report =
        run_validation("examples/bad_usb_return_path_filled_zone_edge_clearance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    let failure = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_RETURN_PATH_VALID"
                && failure["limit"]["min_data_line_filled_zone_edge_clearance_mm"] == 0.1
        })
        .expect("USB return-path filled-zone edge-clearance finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["connector_signal"], "D+");
    assert_eq!(failure["measured"]["segment_index"], 0);
    assert_eq!(failure["measured"]["midpoint_x_mm"], 0.5);
    assert_eq!(failure["measured"]["midpoint_y_mm"], 0.0);
    assert_eq!(failure["measured"]["layer"], "F.Cu");
    let clearance = failure["measured"]["filled_zone_edge_clearance_mm"]
        .as_f64()
        .unwrap();
    assert!((clearance - 0.02).abs() < 1.0e-12);
    assert_eq!(
        failure["limit"]["reference_zone_geometry"],
        "filled_polygon"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn usb_return_path_reports_distant_stitching_via() {
    let report = run_validation("examples/bad_usb_return_path_stitching_via/project.yaml");
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    let failure = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_RETURN_PATH_VALID"
                && failure["limit"]["max_data_via_to_ground_stitch_distance_mm"] == 0.2
        })
        .expect("USB return-path stitching via finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["connector_signal"], "D+");
    assert_eq!(failure["measured"]["data_via_index"], 0);
    assert_eq!(failure["measured"]["data_via_x_mm"], 0.5);
    assert_eq!(failure["measured"]["data_via_y_mm"], 0.0);
    assert_eq!(failure["measured"]["nearest_ground_stitch_net"], "gnd");
    assert_eq!(failure["measured"]["nearest_ground_stitch_via_index"], 0);
    assert_eq!(
        failure["measured"]["nearest_ground_stitch_distance_mm"],
        1.5
    );
    assert_eq!(
        failure["limit"]["required_ground_stitch_layer_policy"],
        "same_via_layers"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn usb_vbus_route_geometry_passes_for_short_power_entry_route() {
    let report = run_validation("examples/good_usb_vbus_route_geometry/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_vbus_route_geometry_reports_length_vias_width_and_protection_order() {
    let report = run_validation("examples/bad_usb_vbus_route_geometry/project.yaml");
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    let route_length = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_VBUS_ROUTE_VALID"
                && failure["net"] == "usb_vbus"
                && failure["measured"]["route_length_mm"] == 6.0
        })
        .expect("VBUS route length finding");
    assert_eq!(route_length["component"], "J1");
    assert_eq!(route_length["measured"]["connector_signal"], "VBUS");
    assert_eq!(route_length["limit"]["max_vbus_route_length_mm"], 5.0);
    let vias = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_VBUS_ROUTE_VALID"
                && failure["net"] == "usb_vbus"
                && failure["measured"]["via_count"] == 2
        })
        .expect("VBUS via count finding");
    assert_eq!(vias["limit"]["max_vbus_via_count"], 0);
    let width = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_VBUS_ROUTE_VALID"
                && failure["net"] == "usb_vbus"
                && failure["measured"]["route_segment_width_mm"] == 0.10
        })
        .expect("VBUS route width finding");
    assert_eq!(width["limit"]["min_vbus_route_width_mm"], 0.25);
    let protection_distance = failures
        .iter()
        .find(|failure| {
            failure["id"] == "USB_VBUS_ROUTE_VALID"
                && failure["net"] == "usb_vbus"
                && failure["measured"]["connector_to_vbus_protection_route_distance_mm"] == 6.0
        })
        .expect("VBUS protection route distance finding");
    assert_eq!(
        protection_distance["measured"]["protection_component"],
        "UVBUS"
    );
    assert_eq!(protection_distance["measured"]["connector_pad"], "VBUS");
    assert_eq!(protection_distance["measured"]["protection_pad"], "VBUS");
    assert_eq!(
        protection_distance["limit"]["max_connector_to_vbus_protection_route_distance_mm"],
        2.0
    );
    assert_eq!(
        protection_distance["limit"]["vbus_route_pad_contact_policy"],
        "same_net_pad_center_on_route"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tpd2eusb30_usb_esd_passes_static_review() {
    let report = run_validation("examples/good_ti_tpd2eusb30_usb_esd/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tpd2eusb30_usb_esd_requires_standoff_above_line_voltage() {
    let report = run_validation("examples/bad_ti_tpd2eusb30_usb_esd_standoff/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["working_voltage_max_V"] == 5.5)
        .expect("TPD2EUSB30 standoff finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["protected_net_nominal_voltage_V"], 6.0);
    assert_eq!(failure["limit"]["protection_clamp"], "d1_plus");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tpd2eusb30_usb_esd_line_capacitance_must_fit_budget() {
    let report = run_validation("examples/bad_ti_tpd2eusb30_usb_esd_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["max_line_capacitance_F"] == 5.0e-13)
        .expect("TPD2EUSB30 capacitance finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["line_capacitance_F"], 7.0e-13);
    assert_eq!(failure["limit"]["protection_clamp"], "d1_plus");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_esd2can24_q1_requires_ground_reference_net() {
    let report = run_validation("examples/bad_ti_esd2can24_q1_can_esd_reference/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["required_reference"] == "ground")
        .expect("ESD2CAN24-Q1 reference finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "canh");
    assert_eq!(
        failure["measured"]["reference_net_kind"],
        "digital_or_analog"
    );
    assert_eq!(failure["limit"]["protection_clamp"], "canh");
    assert_eq!(failure["limit"]["reference_pin"], "GND");
    assert_report_schema_valid(&report);
}

#[test]
fn nexperia_prtr5v0u2x_usb_esd_passes_static_review() {
    let report = run_validation("examples/good_nexperia_prtr5v0u2x_usb_esd/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn nexperia_prtr5v0u2x_requires_power_reference_net() {
    let report = run_validation("examples/bad_nexperia_prtr5v0u2x_usb_esd_reference/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["required_reference"] == "power")
        .expect("PRTR5V0U2X reference finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(
        failure["measured"]["reference_net_kind"],
        "digital_or_analog"
    );
    assert_eq!(failure["limit"]["protection_clamp"], "io1_to_vcc");
    assert_eq!(failure["limit"]["reference_pin"], "VCC");
    assert_report_schema_valid(&report);
}

#[test]
fn nexperia_prtr5v0u2x_line_capacitance_must_fit_budget() {
    let report =
        run_validation("examples/bad_nexperia_prtr5v0u2x_usb_esd_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["max_line_capacitance_F"] == 1.0e-12)
        .expect("PRTR5V0U2X capacitance finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["line_capacitance_F"], 1.5e-12);
    assert_eq!(failure["limit"]["protection_clamp"], "io1_to_vcc");
    assert_report_schema_valid(&report);
}

#[test]
fn nexperia_pesd5v0s1ul_vbus_esd_passes_static_review() {
    let report = run_validation("examples/good_nexperia_pesd5v0s1ul_vbus_esd/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn nexperia_pesd5v0s1ul_vbus_capacitance_must_fit_budget() {
    let report = run_validation("examples/bad_nexperia_pesd5v0s1ul_vbus_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["max_line_capacitance_F"] == 100.0e-12)
        .expect("PESD5V0S1UL capacitance finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UVBUS");
    assert_eq!(failure["net"], "usb_vbus");
    assert_eq!(failure["measured"]["line_capacitance_F"], 200.0e-12);
    assert_eq!(failure["limit"]["protection_clamp"], "vbus_to_ground");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_esds552_requires_ground_reference_net() {
    let report = run_validation("examples/bad_ti_esds552_rs485_esd_reference/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["required_reference"] == "ground")
        .expect("ESDS552 reference finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "rs485_a");
    assert_eq!(
        failure["measured"]["reference_net_kind"],
        "digital_or_analog"
    );
    assert_eq!(failure["limit"]["protection_clamp"], "a");
    assert_eq!(failure["limit"]["reference_pin"], "GND");
    assert_report_schema_valid(&report);
}
