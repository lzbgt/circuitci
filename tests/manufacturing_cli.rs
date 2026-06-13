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
fn drill_annular_ring_passes_when_copper_flash_is_large_enough() {
    let report = run_validation("examples/good_drill_annular_ring/project.yaml");
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
