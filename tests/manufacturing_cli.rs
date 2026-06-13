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
