mod common;

use common::{assert_report_schema_valid, run_validation};

#[test]
fn drill_to_board_edge_clearance_passes_when_hole_is_far_enough() {
    let report = run_validation("examples/good_drill_to_board_edge_clearance/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn drill_to_board_edge_clearance_fails_external_edge_violation() {
    let report = run_validation("examples/bad_drill_to_board_edge_clearance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID");
    assert_eq!(failure["measured"]["drill_index"], 0);
    assert_eq!(failure["measured"]["drill_x_mm"], 0.8);
    assert_eq!(failure["measured"]["drill_mm"], 1.0);
    assert_eq!(failure["measured"]["clearance_mm"], 0.30000000000000004);
    assert_eq!(failure["measured"]["board_edge_boundary_role"], "external");
    assert_eq!(
        failure["measured"]["board_edge_source_primitive"],
        "gerber_linear"
    );
    assert_eq!(failure["limit"]["min_drill_edge_clearance_mm"], 0.5);
    assert_report_schema_valid(&report);
}

#[test]
fn drill_to_board_edge_clearance_treats_cutouts_as_board_edges() {
    let report = run_validation("examples/bad_drill_to_cutout_edge_clearance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID");
    assert_eq!(failure["measured"]["drill_plating"], "plated");
    assert_eq!(failure["measured"]["drill_tool"], "T02");
    assert_eq!(failure["measured"]["board_edge_boundary_role"], "cutout");
    assert_eq!(failure["measured"]["board_edge_contour_index"], 1);
    assert_eq!(failure["limit"]["min_drill_edge_clearance_mm"], 0.4);
    assert_report_schema_valid(&report);
}

#[test]
fn drill_diameter_passes_with_jlc_process_defaults() {
    let report = run_validation("examples/good_drill_diameter_jlc_process/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn drill_diameter_fails_with_jlc_process_defaults() {
    let report = run_validation("examples/bad_drill_diameter_jlc_process/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["summary"]["critical"], 2);
    let failures = report["failures"].as_array().unwrap();
    assert!(
        failures
            .iter()
            .all(|failure| failure["id"] == "DRILL_DIAMETER_VALID")
    );
    assert_eq!(failures[0]["measured"]["drill_index"], 0);
    assert_eq!(failures[0]["measured"]["drill_mm"], 0.1);
    assert_eq!(failures[0]["measured"]["drill_plating"], "plated");
    assert_eq!(failures[0]["limit"]["min_drill_diameter_mm"], 0.15);
    assert_eq!(failures[0]["limit"]["max_drill_diameter_mm"], 6.3);
    assert_eq!(failures[1]["measured"]["drill_index"], 1);
    assert_eq!(failures[1]["measured"]["drill_mm"], 6.4);
    assert_eq!(failures[1]["measured"]["drill_plating"], "non_plated");
    assert_eq!(failures[1]["limit"]["min_drill_diameter_mm"], 0.15);
    assert_eq!(failures[1]["limit"]["max_drill_diameter_mm"], 6.3);
    assert_report_schema_valid(&report);
}

#[test]
fn slot_to_board_edge_clearance_passes_when_slot_is_far_enough() {
    let report = run_validation("examples/good_slot_to_board_edge_clearance/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn slot_to_board_edge_clearance_fails_external_edge_violation() {
    let report = run_validation("examples/bad_slot_to_board_edge_clearance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SLOT_TO_BOARD_EDGE_CLEARANCE_VALID");
    assert_eq!(failure["measured"]["slot_index"], 0);
    assert_eq!(failure["measured"]["slot_start"]["x_mm"], 0.8);
    assert_eq!(failure["measured"]["slot_width_mm"], 1.0);
    assert_eq!(failure["measured"]["slot_radius_mm"], 0.5);
    assert_eq!(failure["measured"]["clearance_mm"], 0.30000000000000004);
    assert_eq!(
        failure["measured"]["slot_centerline_to_board_edge_distance_mm"],
        0.8
    );
    assert_eq!(failure["measured"]["slot_plating"], "plated");
    assert_eq!(failure["measured"]["slot_tool"], "T08");
    assert_eq!(failure["measured"]["source_slot_index"], 0);
    assert_eq!(failure["measured"]["board_edge_boundary_role"], "external");
    assert_eq!(failure["limit"]["min_slot_edge_clearance_mm"], 0.5);
    assert_report_schema_valid(&report);
}

#[test]
fn slot_width_passes_with_jlc_process_defaults() {
    let report = run_validation("examples/good_slot_width_jlc_process/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn slot_width_fails_with_jlc_process_defaults() {
    let report = run_validation("examples/bad_slot_width_jlc_process/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["summary"]["critical"], 3);
    let failures = report["failures"].as_array().unwrap();
    assert!(
        failures
            .iter()
            .all(|failure| failure["id"] == "SLOT_WIDTH_VALID")
    );
    assert_eq!(failures[0]["measured"]["slot_process"], "plated");
    assert_eq!(failures[0]["measured"]["slot_width_mm"], 0.6);
    assert_eq!(failures[0]["limit"]["min_slot_width_mm"], 0.65);
    assert_eq!(failures[1]["measured"]["slot_process"], "non_plated");
    assert_eq!(failures[1]["measured"]["slot_width_mm"], 0.9);
    assert_eq!(failures[1]["limit"]["min_slot_width_mm"], 1.0);
    assert_eq!(failures[2]["measured"]["slot_process"], "unknown_plating");
    assert_eq!(failures[2]["measured"]["slot_width_mm"], 0.9);
    assert_eq!(failures[2]["limit"]["min_slot_width_mm"], 1.0);
    assert_report_schema_valid(&report);
}

#[test]
fn drill_annular_ring_passes_when_copper_flash_is_large_enough() {
    let report = run_validation("examples/good_drill_annular_ring/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn drill_annular_ring_passes_when_required_layers_have_copper_flashes() {
    let report = run_validation("examples/good_drill_annular_ring_required_layers/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn drill_annular_ring_fails_when_copper_flash_is_too_small() {
    let report = run_validation("examples/bad_drill_annular_ring/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "DRILL_ANNULAR_RING_VALID");
    assert_eq!(failure["measured"]["drill_index"], 0);
    assert_eq!(failure["measured"]["drill_mm"], 0.6);
    assert_eq!(failure["measured"]["annular_ring_mm"], 0.10000000000000003);
    assert_eq!(failure["measured"]["copper_feature_index"], 0);
    assert_eq!(failure["measured"]["copper_feature_layer"], "F.Cu");
    assert_eq!(failure["measured"]["copper_feature_aperture"], "D10");
    assert_eq!(failure["measured"]["copper_feature_shape"], "circle");
    assert_eq!(failure["measured"]["copper_feature_size_x_mm"], 0.8);
    assert_eq!(failure["limit"]["min_annular_ring_mm"], 0.2);
    assert_eq!(
        failure["limit"]["max_drill_to_copper_center_offset_mm"],
        0.05
    );
    assert_report_schema_valid(&report);
}

#[test]
fn drill_annular_ring_uses_fabrication_process_list_default() {
    let report = run_validation("examples/bad_drill_annular_ring_jlc_via_process/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "DRILL_ANNULAR_RING_VALID");
    let annular_ring_mm = failure["measured"]["annular_ring_mm"].as_f64().unwrap();
    assert!((annular_ring_mm - 0.045).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["min_annular_ring_mm"], 0.05);
    assert_report_schema_valid(&report);
}

#[test]
fn drill_annular_ring_fails_when_plated_drill_has_no_matching_copper_flash() {
    let report = run_validation("examples/bad_drill_annular_ring_missing_copper/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "DRILL_ANNULAR_RING_VALID");
    assert_eq!(failure["measured"]["drill_index"], 0);
    assert_eq!(failure["measured"]["drill_plating"], "unknown");
    assert_eq!(failure["limit"]["min_annular_ring_mm"], 0.2);
    assert_eq!(
        failure["limit"]["max_drill_to_copper_center_offset_mm"],
        0.05
    );
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("no co-located")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn drill_annular_ring_fails_when_required_layer_has_no_copper_flash() {
    let report =
        run_validation("examples/bad_drill_annular_ring_missing_required_layer/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "DRILL_ANNULAR_RING_VALID");
    assert_eq!(failure["measured"]["drill_net"], "GND");
    assert_eq!(failure["measured"]["required_copper_layer"], "B.Cu");
    assert_eq!(failure["limit"]["min_annular_ring_mm"], 0.2);
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("required layer B.Cu")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn drill_annular_ring_fails_when_drill_and_copper_owners_conflict() {
    let report = run_validation("examples/bad_drill_annular_ring_owner_mismatch/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "DRILL_ANNULAR_RING_VALID");
    assert_eq!(failure["measured"]["drill_owner_kind"], "pad");
    assert_eq!(failure["measured"]["drill_net"], "GND");
    assert_eq!(failure["measured"]["drill_component"], "J1");
    assert_eq!(failure["measured"]["drill_pin"], "1");
    assert_eq!(failure["measured"]["copper_feature_net"], "VBUS");
    assert_eq!(failure["measured"]["drill_copper_owner_mismatch"], true);
    assert!(failure["message"].as_str().unwrap().contains("wrong owner"));
    assert_report_schema_valid(&report);
}

#[test]
fn drill_annular_ring_fails_when_same_net_pad_owners_conflict() {
    let report =
        run_validation("examples/bad_drill_annular_ring_same_net_owner_mismatch/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "DRILL_ANNULAR_RING_VALID");
    assert_eq!(failure["measured"]["drill_net"], "GND");
    assert_eq!(failure["measured"]["drill_owner_kind"], "pad");
    assert_eq!(failure["measured"]["drill_component"], "J1");
    assert_eq!(failure["measured"]["drill_pin"], "1");
    assert_eq!(failure["measured"]["copper_feature_net"], "GND");
    assert_eq!(failure["measured"]["copper_feature_owner_kind"], "pad");
    assert_eq!(failure["measured"]["copper_feature_component"], "J1");
    assert_eq!(failure["measured"]["copper_feature_pin"], "2");
    assert_eq!(failure["measured"]["drill_copper_owner_mismatch"], true);
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("different pad/via owner")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn copper_to_board_edge_clearance_passes_for_far_flash_and_trace() {
    let report = run_validation("examples/good_copper_to_board_edge_clearance/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_to_board_edge_clearance_fails_for_near_flash() {
    let report = run_validation("examples/bad_copper_feature_to_board_edge_clearance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID");
    assert_eq!(failure["measured"]["copper_kind"], "feature");
    assert_eq!(failure["measured"]["copper_feature_index"], 0);
    assert_eq!(failure["measured"]["copper_feature_shape"], "circle");
    assert_eq!(failure["measured"]["clearance_mm"], 0.10000000000000003);
    assert_eq!(failure["measured"]["board_edge_boundary_role"], "external");
    assert_eq!(failure["limit"]["min_copper_edge_clearance_mm"], 0.25);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_to_board_edge_clearance_fails_for_near_trace_segment() {
    let report = run_validation("examples/bad_copper_segment_to_board_edge_clearance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID");
    assert_eq!(failure["measured"]["copper_kind"], "segment");
    assert_eq!(failure["measured"]["copper_segment_index"], 0);
    assert_eq!(failure["measured"]["copper_segment_width_mm"], 0.4);
    assert_eq!(
        failure["measured"]["trace_centerline_to_board_edge_distance_mm"],
        0.4
    );
    assert_eq!(failure["measured"]["clearance_mm"], 0.2);
    assert_eq!(failure["limit"]["min_copper_edge_clearance_mm"], 0.25);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_to_board_edge_clearance_fails_for_near_region() {
    let report = run_validation("examples/bad_copper_region_to_board_edge_clearance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID");
    assert_eq!(failure["measured"]["copper_kind"], "region");
    assert_eq!(failure["measured"]["copper_region_index"], 0);
    assert_eq!(failure["measured"]["copper_region_layer"], "F.Cu");
    assert_eq!(
        failure["measured"]["copper_region_source_primitive"],
        "gerber_region"
    );
    assert_eq!(failure["measured"]["copper_region_point_count"], 4);
    assert_eq!(failure["measured"]["clearance_mm"], 0.1);
    assert_eq!(failure["limit"]["min_copper_edge_clearance_mm"], 0.25);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_spacing_passes_for_far_or_different_layer_copper() {
    let report = run_validation("examples/good_copper_spacing/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_spacing_passes_for_touching_same_net_copper() {
    let report = run_validation("examples/good_copper_same_net_touching/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_mask_opening_passes_when_opening_expands_copper_flash() {
    let report = run_validation("examples/good_solder_mask_opening/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_mask_opening_passes_for_segment_opening() {
    let report = run_validation("examples/good_solder_mask_opening_segment/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_mask_opening_fails_when_opening_is_missing() {
    let report = run_validation("examples/bad_solder_mask_opening_missing/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_MASK_OPENING_VALID");
    assert_eq!(failure["measured"]["copper_feature_index"], 0);
    assert_eq!(failure["measured"]["copper_feature_layer"], "F.Cu");
    assert_eq!(failure["measured"]["expected_solder_mask_layer"], "F.Mask");
    assert_eq!(failure["limit"]["min_mask_expansion_mm"], 0.05);
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("no co-located solder-mask opening")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn solder_mask_opening_fails_when_opening_is_undersized() {
    let report = run_validation("examples/bad_solder_mask_opening_undersized/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_MASK_OPENING_VALID");
    assert_eq!(failure["measured"]["copper_feature_layer"], "B.Cu");
    assert_eq!(failure["measured"]["solder_mask_feature_layer"], "B.Mask");
    assert_eq!(failure["measured"]["solder_mask_feature_shape"], "oval");
    let measured_min_expansion = failure["measured"]["measured_min_mask_expansion_mm"]
        .as_f64()
        .unwrap();
    assert!((measured_min_expansion - 0.03).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["min_mask_expansion_mm"], 0.05);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_mask_opening_uses_fabrication_process_default() {
    let report = run_validation("examples/bad_solder_mask_opening_jlc_process/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_MASK_OPENING_VALID");
    assert_eq!(failure["limit"]["min_mask_expansion_mm"], 0.05);
    assert_eq!(failure["limit"]["max_copper_to_mask_center_offset_mm"], 0.1);
    assert_report_schema_valid(&report);
}

#[test]
fn explicit_manufacturing_parameter_overrides_fabrication_process_default() {
    let report = run_validation("examples/good_solder_mask_opening_explicit_override/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn unknown_fabrication_process_fails_closed_when_needed_for_required_parameter() {
    let report = run_validation("examples/bad_unknown_fabrication_process/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("unsupported process preset 'unknown_fab_process'")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn solder_mask_opening_fails_for_region_opening_expansion() {
    let report = run_validation("examples/bad_solder_mask_opening_region_undersized/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_MASK_OPENING_VALID");
    assert_eq!(failure["measured"]["solder_mask_kind"], "region");
    assert_eq!(failure["measured"]["solder_mask_region_layer"], "F.Mask");
    assert_eq!(failure["measured"]["solder_mask_region_point_count"], 4);
    let measured_min_expansion = failure["measured"]["measured_min_mask_expansion_mm"]
        .as_f64()
        .unwrap();
    assert!((measured_min_expansion + 0.04_f64.hypot(0.04)).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["min_mask_expansion_mm"], 0.05);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_mask_dam_passes_when_openings_are_far_enough() {
    let report = run_validation("examples/good_solder_mask_dam/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_mask_dam_fails_when_openings_leave_thin_web() {
    let report = run_validation("examples/bad_solder_mask_dam/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_MASK_DAM_VALID");
    assert_eq!(failure["measured"]["solder_mask_layer"], "F.Mask");
    assert_eq!(failure["measured"]["first_solder_mask_kind"], "feature");
    assert_eq!(failure["measured"]["first_solder_mask_feature_index"], 0);
    assert_eq!(
        failure["measured"]["first_solder_mask_feature_owner_kind"],
        "pad"
    );
    assert_eq!(
        failure["measured"]["first_solder_mask_feature_component"],
        "U1"
    );
    assert_eq!(failure["measured"]["first_solder_mask_feature_pin"], "1");
    assert_eq!(failure["measured"]["second_solder_mask_kind"], "feature");
    assert_eq!(failure["measured"]["second_solder_mask_feature_index"], 1);
    assert_eq!(
        failure["measured"]["second_solder_mask_feature_owner_kind"],
        "pad"
    );
    assert_eq!(
        failure["measured"]["second_solder_mask_feature_component"],
        "U1"
    );
    assert_eq!(failure["measured"]["second_solder_mask_feature_pin"], "2");
    let dam_width = failure["measured"]["solder_mask_dam_width_mm"]
        .as_f64()
        .unwrap();
    assert!((dam_width - 0.08).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["min_solder_mask_dam_mm"], 0.15);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_mask_dam_uses_fabrication_process_default() {
    let report = run_validation("examples/bad_solder_mask_dam_jlc_process/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_MASK_DAM_VALID");
    let dam_width = failure["measured"]["solder_mask_dam_width_mm"]
        .as_f64()
        .unwrap();
    assert!((dam_width - 0.08).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["min_solder_mask_dam_mm"], 0.1);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_mask_dam_fails_for_non_flash_openings() {
    let report = run_validation("examples/bad_solder_mask_dam_segment_region/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_MASK_DAM_VALID");
    assert_eq!(failure["measured"]["solder_mask_layer"], "F.Mask");
    assert_eq!(failure["measured"]["first_solder_mask_kind"], "segment");
    assert_eq!(failure["measured"]["first_solder_mask_segment_index"], 0);
    assert_eq!(
        failure["measured"]["first_solder_mask_segment_owner_kind"],
        "pad"
    );
    assert_eq!(
        failure["measured"]["first_solder_mask_segment_component"],
        "U1"
    );
    assert_eq!(failure["measured"]["first_solder_mask_segment_pin"], "1");
    assert_eq!(
        failure["measured"]["first_solder_mask_segment_width_mm"],
        0.2
    );
    assert_eq!(failure["measured"]["second_solder_mask_kind"], "region");
    assert_eq!(failure["measured"]["second_solder_mask_region_index"], 0);
    assert_eq!(
        failure["measured"]["second_solder_mask_region_owner_kind"],
        "pad"
    );
    assert_eq!(
        failure["measured"]["second_solder_mask_region_component"],
        "U1"
    );
    assert_eq!(failure["measured"]["second_solder_mask_region_pin"], "2");
    assert_eq!(
        failure["measured"]["second_solder_mask_region_point_count"],
        4
    );
    let dam_width = failure["measured"]["solder_mask_dam_width_mm"]
        .as_f64()
        .unwrap();
    assert!((dam_width - 0.12).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["min_solder_mask_dam_mm"], 0.15);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_opening_passes_when_area_ratio_is_in_range() {
    let report = run_validation("examples/good_solder_paste_opening/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_opening_passes_for_segment_opening() {
    let report = run_validation("examples/good_solder_paste_opening_segment/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_opening_passes_for_windowed_openings() {
    let report = run_validation("examples/good_solder_paste_opening_windowed/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_opening_fails_when_opening_is_missing() {
    let report = run_validation("examples/bad_solder_paste_opening_missing/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_PASTE_OPENING_VALID");
    assert_eq!(failure["measured"]["copper_feature_index"], 0);
    assert_eq!(failure["measured"]["copper_feature_layer"], "F.Cu");
    assert_eq!(
        failure["measured"]["expected_solder_paste_layer"],
        "F.Paste"
    );
    assert_eq!(failure["limit"]["min_paste_area_ratio"], 0.7);
    assert_eq!(failure["limit"]["max_paste_area_ratio"], 1.0);
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("no co-located solder-paste opening")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_opening_fails_when_opening_is_undersized() {
    let report = run_validation("examples/bad_solder_paste_opening_undersized/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_PASTE_OPENING_VALID");
    assert_eq!(failure["measured"]["solder_paste_feature_layer"], "F.Paste");
    assert_eq!(failure["measured"]["solder_paste_feature_shape"], "rect");
    let area_ratio = failure["measured"]["solder_paste_area_ratio"]
        .as_f64()
        .unwrap();
    assert!((area_ratio - 0.4375).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["min_paste_area_ratio"], 0.7);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_opening_fails_when_opening_is_oversized() {
    let report = run_validation("examples/bad_solder_paste_opening_oversized/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_PASTE_OPENING_VALID");
    let area_ratio = failure["measured"]["solder_paste_area_ratio"]
        .as_f64()
        .unwrap();
    assert!((area_ratio - 1.2375).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["max_paste_area_ratio"], 1.0);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_opening_reports_aggregate_windowed_area() {
    let report =
        run_validation("examples/bad_solder_paste_opening_windowed_oversized/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_PASTE_OPENING_VALID");
    assert_eq!(failure["measured"]["solder_paste_opening_count"], 4);
    assert_eq!(failure["measured"]["solder_paste_opening_area_mm2"], 4.0);
    let area_ratio = failure["measured"]["solder_paste_area_ratio"]
        .as_f64()
        .unwrap();
    assert!((area_ratio - 0.25).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["max_paste_area_ratio"], 0.24);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_opening_fails_for_region_opening_area() {
    let report = run_validation("examples/bad_solder_paste_opening_region_oversized/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_PASTE_OPENING_VALID");
    assert_eq!(failure["measured"]["solder_paste_kind"], "region");
    assert_eq!(failure["measured"]["solder_paste_region_index"], 0);
    assert_eq!(failure["measured"]["solder_paste_region_point_count"], 4);
    let area_ratio = failure["measured"]["solder_paste_area_ratio"]
        .as_f64()
        .unwrap();
    assert!((area_ratio - 1.5).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["max_paste_area_ratio"], 1.0);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_spacing_passes_when_openings_are_far_enough() {
    let report = run_validation("examples/good_solder_paste_spacing/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_spacing_fails_when_openings_are_too_close() {
    let report = run_validation("examples/bad_solder_paste_spacing/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_PASTE_SPACING_VALID");
    assert_eq!(failure["measured"]["solder_paste_layer"], "F.Paste");
    assert_eq!(failure["measured"]["first_solder_paste_kind"], "feature");
    assert_eq!(failure["measured"]["first_solder_paste_feature_index"], 0);
    assert_eq!(
        failure["measured"]["first_solder_paste_feature_owner_kind"],
        "pad"
    );
    assert_eq!(
        failure["measured"]["first_solder_paste_feature_component"],
        "U1"
    );
    assert_eq!(failure["measured"]["first_solder_paste_feature_pin"], "1");
    assert_eq!(failure["measured"]["second_solder_paste_kind"], "feature");
    assert_eq!(failure["measured"]["second_solder_paste_feature_index"], 1);
    assert_eq!(
        failure["measured"]["second_solder_paste_feature_owner_kind"],
        "pad"
    );
    assert_eq!(
        failure["measured"]["second_solder_paste_feature_component"],
        "U1"
    );
    assert_eq!(failure["measured"]["second_solder_paste_feature_pin"], "2");
    let spacing = failure["measured"]["solder_paste_spacing_mm"]
        .as_f64()
        .unwrap();
    assert!((spacing - 0.08).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["min_solder_paste_spacing_mm"], 0.15);
    assert_report_schema_valid(&report);
}

#[test]
fn solder_paste_spacing_fails_for_non_flash_openings() {
    let report = run_validation("examples/bad_solder_paste_spacing_segment_region/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SOLDER_PASTE_SPACING_VALID");
    assert_eq!(failure["measured"]["first_solder_paste_kind"], "segment");
    assert_eq!(failure["measured"]["first_solder_paste_segment_index"], 0);
    assert_eq!(
        failure["measured"]["first_solder_paste_segment_owner_kind"],
        "pad"
    );
    assert_eq!(
        failure["measured"]["first_solder_paste_segment_component"],
        "U1"
    );
    assert_eq!(failure["measured"]["first_solder_paste_segment_pin"], "1");
    assert_eq!(
        failure["measured"]["first_solder_paste_segment_width_mm"],
        0.2
    );
    assert_eq!(failure["measured"]["second_solder_paste_kind"], "region");
    assert_eq!(failure["measured"]["second_solder_paste_region_index"], 0);
    assert_eq!(
        failure["measured"]["second_solder_paste_region_owner_kind"],
        "pad"
    );
    assert_eq!(
        failure["measured"]["second_solder_paste_region_component"],
        "U1"
    );
    assert_eq!(failure["measured"]["second_solder_paste_region_pin"], "2");
    assert_eq!(
        failure["measured"]["second_solder_paste_region_point_count"],
        4
    );
    let spacing = failure["measured"]["solder_paste_spacing_mm"]
        .as_f64()
        .unwrap();
    assert!((spacing - 0.12).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["min_solder_paste_spacing_mm"], 0.15);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_spacing_fails_for_near_flashes() {
    let report = run_validation("examples/bad_copper_feature_spacing/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "COPPER_SPACING_VALID");
    assert_eq!(failure["measured"]["first_copper_kind"], "feature");
    assert_eq!(failure["measured"]["first_copper_feature_index"], 0);
    assert_eq!(failure["measured"]["second_copper_kind"], "feature");
    assert_eq!(failure["measured"]["second_copper_feature_index"], 1);
    assert_eq!(failure["measured"]["copper_layer"], "F.Cu");
    assert_eq!(failure["measured"]["clearance_mm"], 0.19999999999999996);
    assert_eq!(failure["limit"]["min_copper_spacing_mm"], 0.25);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_spacing_fails_for_overlapping_different_net_copper() {
    let report = run_validation("examples/bad_copper_different_net_overlap/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "COPPER_SPACING_VALID");
    assert_eq!(failure["measured"]["first_copper_kind"], "feature");
    assert_eq!(failure["measured"]["first_copper_feature_net"], "VBUS");
    assert_eq!(
        failure["measured"]["first_copper_feature_island_id"],
        "F_Cu_VBUS_0"
    );
    assert_eq!(failure["measured"]["second_copper_kind"], "feature");
    assert_eq!(failure["measured"]["second_copper_feature_net"], "GND");
    assert_eq!(
        failure["measured"]["second_copper_feature_island_id"],
        "F_Cu_GND_0"
    );
    assert_eq!(failure["measured"]["clearance_mm"], 0.0);
    assert_eq!(failure["limit"]["min_copper_spacing_mm"], 0.25);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_spacing_fails_for_overlapping_different_island_copper() {
    let report = run_validation("examples/bad_copper_different_island_overlap/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "COPPER_SPACING_VALID");
    assert_eq!(
        failure["measured"]["first_copper_feature_island_id"],
        "F_Cu_island_0"
    );
    assert_eq!(
        failure["measured"]["second_copper_feature_island_id"],
        "F_Cu_island_1"
    );
    assert_eq!(failure["measured"]["clearance_mm"], 0.0);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_spacing_fails_for_near_flash_and_trace() {
    let report = run_validation("examples/bad_copper_feature_segment_spacing/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "COPPER_SPACING_VALID");
    assert_eq!(failure["measured"]["first_copper_kind"], "feature");
    assert_eq!(failure["measured"]["first_copper_feature_shape"], "rect");
    assert_eq!(failure["measured"]["second_copper_kind"], "segment");
    assert_eq!(failure["measured"]["second_copper_segment_width_mm"], 0.2);
    assert_eq!(failure["measured"]["clearance_mm"], 0.35);
    assert_eq!(failure["limit"]["min_copper_spacing_mm"], 0.4);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_spacing_fails_for_near_traces() {
    let report = run_validation("examples/bad_copper_segment_spacing/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "COPPER_SPACING_VALID");
    assert_eq!(failure["measured"]["first_copper_kind"], "segment");
    assert_eq!(failure["measured"]["first_copper_segment_index"], 0);
    assert_eq!(failure["measured"]["second_copper_kind"], "segment");
    assert_eq!(failure["measured"]["second_copper_segment_index"], 1);
    assert_eq!(failure["measured"]["clearance_mm"], 0.19999999999999993);
    assert_eq!(failure["limit"]["min_copper_spacing_mm"], 0.25);
    assert_report_schema_valid(&report);
}

#[test]
fn copper_spacing_fails_for_near_flash_and_region() {
    let report = run_validation("examples/bad_copper_region_spacing/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "COPPER_SPACING_VALID");
    assert_eq!(failure["measured"]["first_copper_kind"], "feature");
    assert_eq!(failure["measured"]["first_copper_feature_index"], 0);
    assert_eq!(failure["measured"]["second_copper_kind"], "region");
    assert_eq!(failure["measured"]["second_copper_region_index"], 0);
    assert_eq!(
        failure["measured"]["second_copper_region_source_primitive"],
        "gerber_region"
    );
    assert_eq!(failure["measured"]["copper_layer"], "F.Cu");
    let clearance = failure["measured"]["clearance_mm"].as_f64().unwrap();
    assert!((clearance - 0.15).abs() < 1e-9);
    assert_eq!(failure["limit"]["min_copper_spacing_mm"], 0.25);
    assert_report_schema_valid(&report);
}
