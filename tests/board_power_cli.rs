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
        failure["measured"]["obstruction_reference"],
        "footprint_rectangle"
    );
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
fn good_power_tree_board_passes() {
    let report = run_validation("examples/good_power_tree_board/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn good_load_switch_power_tree_passes_with_enable_evidence() {
    let report = run_validation("examples/good_load_switch_power_tree/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn load_switch_powered_output_requires_enable_evidence() {
    let report = run_validation("examples/bad_load_switch_missing_enable/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("required_enabled_state").is_some())
        .expect("expected load switch enable finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USW");
    assert_eq!(failure["net"], "sensor_3v3");
    assert_eq!(failure["measured"]["control_state"], "missing");
    assert_eq!(failure["limit"]["control_pin"], "EN");
    assert_eq!(failure["limit"]["required_enabled_state"], "high");
    assert_report_schema_valid(&report);
}

#[test]
fn load_switch_disabled_control_cannot_power_output_rail() {
    let report = run_validation("examples/bad_load_switch_disabled_output_powered/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("required_enabled_state").is_some())
        .expect("expected load switch enable finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USW");
    assert_eq!(failure["net"], "sensor_3v3");
    assert_eq!(failure["measured"]["control_state"], "low");
    assert_eq!(failure["limit"]["required_enabled_state"], "high");
    assert_report_schema_valid(&report);
}

#[test]
fn load_switch_output_current_limit_fails() {
    let report = run_validation("examples/bad_load_switch_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("load_switch_max_output_current_A")
                .is_some()
        })
        .expect("expected load switch output current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USW");
    assert_eq!(failure["net"], "sensor_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 0.07);
    assert_eq!(failure["limit"]["load_switch_max_output_current_A"], 0.05);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tps22918_load_switch_power_tree_passes_with_on_evidence() {
    let report = run_validation("examples/good_ti_tps22918_load_switch/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tps22918_powered_output_requires_on_evidence() {
    let report = run_validation("examples/bad_ti_tps22918_missing_on/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("required_enabled_state").is_some())
        .expect("expected TPS22918 ON evidence finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USW");
    assert_eq!(failure["net"], "sensor_3v3");
    assert_eq!(failure["measured"]["control_state"], "missing");
    assert_eq!(failure["limit"]["control_pin"], "ON");
    assert_eq!(failure["limit"]["required_enabled_state"], "high");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tps22918_output_current_uses_datasheet_limit() {
    let report = run_validation("examples/bad_ti_tps22918_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("load_switch_max_output_current_A")
                .is_some()
        })
        .expect("expected TPS22918 output current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USW");
    assert_eq!(failure["net"], "sensor_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 2.1);
    assert_eq!(failure["limit"]["load_switch_max_output_current_A"], 2.0);
    assert_report_schema_valid(&report);
}

#[test]
fn microchip_mcp73831_usb_charger_passes_current_budget() {
    let report = run_validation("examples/good_microchip_mcp73831_usb_charger/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn microchip_mcp73831_charge_current_must_fit_usb_budget() {
    let report = run_validation("examples/bad_microchip_mcp73831_usb_budget/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("input_supply_current_limit_A")
                .is_some()
        })
        .expect("expected charger input budget finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UCHG");
    assert_eq!(failure["net"], "usb_5v");
    assert_eq!(failure["measured"]["programmed_charge_current_A"], 0.5);
    assert_eq!(failure["limit"]["input_supply_current_limit_A"], 0.1);
    assert_report_schema_valid(&report);
}

#[test]
fn microchip_mcp73831_charge_current_uses_datasheet_limit() {
    let report = run_validation("examples/bad_microchip_mcp73831_charge_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("battery_charger_max_charge_current_A")
                .is_some()
        })
        .expect("expected charger current limit finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UCHG");
    assert_eq!(failure["net"], "battery");
    assert_eq!(failure["measured"]["programmed_charge_current_A"], 0.6);
    assert_eq!(
        failure["limit"]["battery_charger_max_charge_current_A"],
        0.5
    );
    assert_report_schema_valid(&report);
}

#[test]
fn microchip_mcp73831_requires_programmed_charge_current() {
    let report = run_validation("examples/bad_microchip_mcp73831_missing_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("required_component_parameter")
                .is_some()
        })
        .expect("expected missing charger current parameter finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UCHG");
    assert_eq!(
        failure["limit"]["required_component_parameter"],
        "programmed_charge_current_A"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn good_power_mux_usb_selected_passes_with_reverse_blocking() {
    let report = run_validation("examples/good_power_mux_usb_selected/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn power_mux_requires_selected_input_parameter() {
    let report = run_validation("examples/bad_power_mux_missing_selection/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("required_component_parameter")
                .is_some()
        })
        .expect("expected selected input parameter finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UMUX");
    assert_eq!(
        failure["limit"]["required_component_parameter"],
        "selected_input"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn power_mux_selected_input_must_be_powered() {
    let report = run_validation("examples/bad_power_mux_selected_unpowered/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("selected_input_powered").is_some())
        .expect("expected selected unpowered input finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UMUX");
    assert_eq!(failure["net"], "sys");
    assert_eq!(failure["measured"]["selected_input"], "battery");
    assert_eq!(failure["measured"]["selected_input_powered"], false);
    assert_eq!(failure["limit"]["selected_input_powered"], true);
    assert_report_schema_valid(&report);
}

#[test]
fn power_mux_inactive_unpowered_input_requires_reverse_blocking() {
    let report = run_validation("examples/bad_power_mux_backfeed/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("required_reverse_blocking").is_some())
        .expect("expected reverse-blocking finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UMUX");
    assert_eq!(failure["net"], "battery");
    assert_eq!(failure["measured"]["inactive_input"], "battery");
    assert_eq!(failure["measured"]["inactive_input_powered"], false);
    assert_eq!(failure["measured"]["output_powered"], true);
    assert_eq!(failure["limit"]["required_reverse_blocking"], true);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tps2115a_power_mux_passes_with_selected_source_and_reverse_blocking() {
    let report = run_validation("examples/good_ti_tps2115a_power_mux/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tps2115a_output_current_uses_datasheet_limit() {
    let report = run_validation("examples/bad_ti_tps2115a_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("power_mux_max_output_current_A")
                .is_some()
        })
        .expect("expected power mux output current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UMUX");
    assert_eq!(failure["net"], "sys_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 1.2);
    assert_eq!(failure["limit"]["power_mux_max_output_current_A"], 1.0);
    assert_report_schema_valid(&report);
}

#[test]
fn good_clock_source_crystal_passes_load_capacitance_check() {
    let report = run_validation("examples/good_clock_source_crystal/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn clock_source_load_capacitance_fails_outside_crystal_range() {
    let report = run_validation("examples/bad_clock_source_load_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == "CLOCK_SOURCE_VALID")
        .expect("expected clock source finding");
    assert_eq!(failure["component"], "U1");
    assert_eq!(failure["measured"]["crystal_component"], "Y1");
    assert_eq!(failure["measured"]["input_load_capacitance_F"], 8.0e-12);
    assert_eq!(failure["measured"]["output_load_capacitance_F"], 8.0e-12);
    assert_eq!(failure["measured"]["stray_capacitance_F"], 2.0e-12);
    assert_eq!(failure["measured"]["effective_load_capacitance_F"], 6.0e-12);
    assert_eq!(failure["limit"]["crystal_load_capacitance_min_F"], 10.0e-12);
    assert_eq!(failure["limit"]["crystal_load_capacitance_max_F"], 15.0e-12);
    assert_report_schema_valid(&report);
}

#[test]
fn power_tree_overvoltage_fails() {
    let report = run_validation("examples/bad_power_tree_overvoltage/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "POWER_TREE_VALID");
    assert_eq!(report["failures"][0]["component"], "U1");
    assert_eq!(report["failures"][0]["net"], "rail_5v");
    assert_eq!(report["failures"][0]["measured"]["nominal_voltage_V"], 5.0);
    assert_eq!(
        report["failures"][0]["limit"]["operating_voltage_maximum_V"],
        3.6
    );
    assert_report_schema_valid(&report);
}

#[test]
fn wch_ch340c_power_overvoltage_uses_datasheet_limit() {
    let report = run_validation("examples/bad_wch_ch340c_power_overvoltage/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["component"] == "U5" && finding["limit"]["operating_voltage_maximum_V"] == 5.3
        })
        .expect("CH340C VCC finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["measured"]["nominal_voltage_V"], 6.0);
    assert_eq!(failure["limit"]["operating_voltage_maximum_V"], 5.3);
    assert_report_schema_valid(&report);
}

#[test]
fn silabs_cp2102n_vdd_overvoltage_uses_datasheet_limit() {
    let report = run_validation("examples/bad_silabs_cp2102n_vdd_overvoltage/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["component"] == "U6" && finding["limit"]["operating_voltage_maximum_V"] == 3.6
        })
        .expect("CP2102N VDD finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["measured"]["nominal_voltage_V"], 5.0);
    assert_eq!(failure["limit"]["operating_voltage_maximum_V"], 3.6);
    assert_report_schema_valid(&report);
}

#[test]
fn io_voltage_vih_mismatch_fails() {
    let report = run_validation("examples/bad_io_voltage_vih_mismatch/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("receiver_vih_min_V").is_some())
        .expect("VIH mismatch finding");
    assert_eq!(failure["id"], "IO_VOLTAGE_COMPATIBLE");
    assert_eq!(failure["component"], "UMCU");
    assert_eq!(failure["net"], "sensor_irq");
    assert_eq!(failure["measured"]["driver_high_voltage_V"], 1.8);
    assert_eq!(failure["limit"]["receiver_vih_min_V"], 2.4);
    assert_report_schema_valid(&report);
}

#[test]
fn io_voltage_respects_imported_kicad_pin_direction_metadata() {
    let report = run_validation("examples/good_io_voltage_kicad_direction_metadata/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn io_voltage_backfeed_clamp_current_fails() {
    let report = run_validation("examples/bad_io_voltage_backfeed_clamp/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"].get("injection_current_A").is_some())
        .expect("clamp-current finding");
    assert_eq!(failure["id"], "IO_VOLTAGE_COMPATIBLE");
    assert_eq!(failure["component"], "UMCU");
    assert_eq!(failure["net"], "wake_line");
    assert_eq!(failure["measured"]["driver_high_voltage_V"], 5.0);
    assert_eq!(failure["measured"]["receiver_rail_voltage_V"], 3.3);
    let injection_current_a = failure["measured"]["injection_current_A"].as_f64().unwrap();
    assert!((injection_current_a - 0.014).abs() < 1e-12);
    assert_eq!(failure["limit"]["injection_current_A"], 0.001);
    assert_report_schema_valid(&report);
}

#[test]
fn boot_strap_bias_divider_passes() {
    let report = run_validation("examples/good_bootstrap_bias_divider/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn boot_strap_bias_threshold_fails() {
    let report = run_validation("examples/bad_bootstrap_bias_threshold/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "BOOT_STRAP_BIAS_VALID");
    assert_eq!(failure["component"], "U1");
    assert_eq!(failure["net"], "boot0");
    assert_eq!(failure["measured"]["required_boot_mode"], "bootloader");
    assert_eq!(failure["measured"]["strap_voltage_V"], 1.65);
    assert_eq!(failure["limit"]["required_BOOT0"], "high");
    assert_eq!(failure["limit"]["vih_min_V"], 2.0);
    assert_report_schema_valid(&report);
}

#[test]
fn boot_strap_bias_current_limit_fails() {
    let report = run_validation("examples/bad_bootstrap_bias_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("max_strap_bias_current_A").is_some())
        .expect("strap current finding");
    assert_eq!(failure["id"], "BOOT_STRAP_BIAS_VALID");
    assert_eq!(failure["component"], "U1");
    assert_eq!(failure["net"], "boot0");
    assert_eq!(failure["measured"]["strap_voltage_V"], 3.0);
    let current_a = failure["measured"]["strap_bias_current_A"]
        .as_f64()
        .unwrap();
    assert!((current_a - 0.0003).abs() < 1e-12);
    assert_eq!(failure["limit"]["max_strap_bias_current_A"], 0.0001);
    assert_report_schema_valid(&report);
}

#[test]
fn power_tree_current_budget_fails() {
    let report = run_validation("examples/bad_power_tree_current_budget/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "POWER_TREE_VALID");
    assert_eq!(report["failures"][0]["net"], "rail_3v3");
    assert_eq!(
        report["failures"][0]["measured"]["declared_load_current_A"],
        0.05
    );
    assert_eq!(
        report["failures"][0]["limit"]["supply_current_limit_A"],
        0.04
    );
    assert_report_schema_valid(&report);
}

#[test]
fn good_regulator_power_tree_passes() {
    let report = run_validation("examples/good_regulator_power_tree/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn good_reset_supervisor_threshold_passes() {
    let report = run_validation("examples/good_reset_supervisor_threshold/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tlv803ea29_reset_supervisor_threshold_passes() {
    let report = run_validation("examples/good_ti_tlv803ea29_reset_supervisor/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tlv803ea29_threshold_above_nominal_rail_fails() {
    let report = run_validation("examples/bad_ti_tlv803ea29_nominal_rail/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("reset_supervisor_threshold_max_V")
                .is_some()
        })
        .expect("expected TLV803EA29 threshold finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USUP");
    assert_eq!(failure["net"], "rail_2v9");
    assert_eq!(failure["measured"]["monitored_nominal_voltage_V"], 2.9);
    assert_eq!(failure["limit"]["reset_supervisor_threshold_max_V"], 2.9886);
    assert_report_schema_valid(&report);
}

#[test]
fn reset_supervisor_threshold_below_load_minimum_fails() {
    let report = run_validation("examples/bad_reset_supervisor_threshold_too_low/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["id"] == "POWER_TREE_VALID"
                && finding["measured"]
                    .get("reset_supervisor_threshold_min_V")
                    .is_some()
        })
        .expect("expected reset-supervisor threshold finding");
    assert_eq!(failure["component"], "USUP_LOW_THRESHOLD");
    assert_eq!(failure["net"], "nrst");
    assert_eq!(failure["measured"]["reset_supervisor_threshold_min_V"], 2.4);
    assert_eq!(failure["measured"]["monitored_load_component"], "U1");
    assert_eq!(failure["measured"]["monitored_load_pin"], "VDD");
    assert_eq!(failure["limit"]["load_operating_voltage_min_V"], 2.7);
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_dropout_fails() {
    let report = run_validation("examples/bad_regulator_dropout/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("dropout_voltage_V").is_some())
        .expect("expected regulator dropout finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["input_voltage_V"], 3.4);
    assert_eq!(failure["measured"]["output_voltage_V"], 3.3);
    assert_eq!(failure["limit"]["dropout_voltage_V"], 0.3);
    assert_report_schema_valid(&report);
}

#[test]
fn diodes_ap2112k_3v3_regulator_passes_static_power_tree() {
    let report = run_validation("examples/good_diodes_ap2112k_3v3_regulator/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn diodes_ap2112k_3v3_dropout_uses_datasheet_limit() {
    let report = run_validation("examples/bad_diodes_ap2112k_3v3_dropout/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("dropout_voltage_V").is_some())
        .expect("expected AP2112 dropout finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["input_voltage_V"], 3.6);
    assert_eq!(failure["measured"]["output_voltage_V"], 3.3);
    let margin = failure["measured"]["dropout_margin_V"].as_f64().unwrap();
    assert!((margin - 0.3).abs() < 1e-12);
    assert_eq!(failure["limit"]["dropout_voltage_V"], 0.4);
    assert_report_schema_valid(&report);
}

#[test]
fn diodes_ap2112k_3v3_output_current_uses_datasheet_limit() {
    let report = run_validation("examples/bad_diodes_ap2112k_3v3_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_max_output_current_A")
                .is_some()
        })
        .expect("expected AP2112 output-current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 0.65);
    assert_eq!(failure["limit"]["regulator_max_output_current_A"], 0.6);
    assert_report_schema_valid(&report);
}

#[test]
fn diodes_ap2112k_3v3_output_capacitance_uses_datasheet_requirement() {
    let report = run_validation("examples/bad_diodes_ap2112k_3v3_output_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_output_capacitance_min_F")
                .is_some()
        })
        .expect("expected AP2112 output capacitance finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["support_capacitance_F"], 0.00000047);
    assert_eq!(failure["measured"]["support_capacitors"][0], "COUT");
    assert_eq!(
        failure["limit"]["regulator_output_capacitance_min_F"],
        0.000001
    );
    assert_eq!(failure["limit"]["power_conversion_pin"], "VOUT");
    assert_report_schema_valid(&report);
}

#[test]
fn ams1117_3v3_regulator_passes_static_power_tree() {
    let report = run_validation("examples/good_ams1117_3v3_regulator/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ams1117_3v3_dropout_uses_datasheet_limit() {
    let report = run_validation("examples/bad_ams1117_3v3_dropout/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("dropout_voltage_V").is_some())
        .expect("expected AMS1117 dropout finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["input_voltage_V"], 4.2);
    assert_eq!(failure["measured"]["output_voltage_V"], 3.3);
    let margin = failure["measured"]["dropout_margin_V"].as_f64().unwrap();
    assert!((margin - 0.9).abs() < 1e-12);
    assert_eq!(failure["limit"]["dropout_voltage_V"], 1.3);
    assert_report_schema_valid(&report);
}

#[test]
fn ams1117_3v3_minimum_load_uses_datasheet_requirement() {
    let report = run_validation("examples/bad_ams1117_3v3_minimum_load/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_min_output_current_A")
                .is_some()
        })
        .expect("expected AMS1117 minimum-load finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(
        failure["measured"]["declared_minimum_output_load_current_A"],
        0.002
    );
    assert_eq!(failure["limit"]["regulator_min_output_current_A"], 0.01);
    assert_report_schema_valid(&report);
}

#[test]
fn ams1117_3v3_output_current_uses_datasheet_regulation_limit() {
    let report = run_validation("examples/bad_ams1117_3v3_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_max_output_current_A")
                .is_some()
        })
        .expect("expected AMS1117 output-current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 0.85);
    assert_eq!(failure["limit"]["regulator_max_output_current_A"], 0.8);
    assert_report_schema_valid(&report);
}

#[test]
fn ams1117_3v3_output_capacitance_uses_datasheet_requirement() {
    let report = run_validation("examples/bad_ams1117_3v3_output_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_output_capacitance_min_F")
                .is_some()
        })
        .expect("expected AMS1117 output capacitance finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["support_capacitance_F"], 0.000010);
    assert_eq!(failure["measured"]["support_capacitors"][0], "COUT");
    assert_eq!(
        failure["limit"]["regulator_output_capacitance_min_F"],
        0.000022
    );
    assert_eq!(failure["limit"]["power_conversion_pin"], "VOUT");
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_output_current_fails() {
    let report = run_validation("examples/bad_regulator_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_max_output_current_A")
                .is_some()
        })
        .expect("expected regulator output current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 0.05);
    assert_eq!(failure["limit"]["regulator_max_output_current_A"], 0.04);
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_conversion_metadata_fails_closed() {
    let report = run_validation("examples/bad_regulator_conversion_pin/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("power_conversion_field").is_some())
        .expect("expected power_conversion metadata finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["limit"]["power_conversion_field"], "output_pin");
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_startup_sequence_fails() {
    let report = run_validation("examples/bad_regulator_startup_sequence/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("earliest_output_power_valid_at_us")
                .is_some()
        })
        .expect("expected regulator startup timing finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["input_power_valid_at_us"], 800.0);
    assert_eq!(failure["measured"]["output_power_valid_at_us"], 1200.0);
    assert_eq!(failure["measured"]["startup_delay_us"], 1000.0);
    assert_eq!(
        failure["limit"]["earliest_output_power_valid_at_us"],
        1800.0
    );
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_startup_missing_timing_fails_closed() {
    let report = run_validation("examples/bad_regulator_startup_missing_timing/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("required_rail_timing_field").is_some())
        .expect("expected missing rail timing finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(
        failure["limit"]["required_rail_timing_field"],
        "output_power_valid_at_us"
    );
    assert_report_schema_valid(&report);
}
