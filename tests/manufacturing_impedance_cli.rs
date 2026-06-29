mod common;

use common::{assert_report_schema_valid, run_validation};

#[test]
fn controlled_impedance_geometry_passes_for_declared_width_and_gap_targets() {
    let (_dir, project_path) = write_impedance_project(
        r#"      nets:
        - net: RF
          source: fab_stackup_table_rev_a
          target_impedance_ohm: 50
          expected_width_mm: 0.20
          max_width_error_mm: 0.03
      differential_pairs:
        - first_net: DP
          second_net: DM
          source: fab_stackup_table_rev_a
          target_differential_impedance_ohm: 90
          expected_width_mm: 0.15
          expected_gap_mm: 0.20
          max_width_error_mm: 0.02
          max_gap_error_mm: 0.03
"#,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_geometry_fails_for_single_ended_width_error() {
    let (_dir, project_path) = write_impedance_project(
        r#"      nets:
        - net: RF
          source: fab_stackup_table_rev_a
          target_impedance_ohm: 50
          expected_width_mm: 0.20
          max_width_error_mm: 0.01
"#,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_GEOMETRY_VALID");
    assert_eq!(failure["measured"]["net"], "RF");
    assert_eq!(
        failure["measured"]["target_source"],
        "fab_stackup_table_rev_a"
    );
    assert_eq!(failure["measured"]["target_impedance_ohm"], 50.0);
    assert_eq!(failure["measured"]["route_net"], "RF");
    assert_eq!(failure["measured"]["route_layer"], "F.Cu");
    assert_eq!(failure["measured"]["route_measured_width_mm"], 0.18);
    let width_error = failure["measured"]["route_width_error_mm"]
        .as_f64()
        .unwrap();
    assert!((width_error - 0.02).abs() < 1.0e-12);
    assert_eq!(failure["limit"]["expected_width_mm"], 0.20);
    assert_eq!(failure["limit"]["max_width_error_mm"], 0.01);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_geometry_fails_for_differential_gap_error() {
    let (_dir, project_path) = write_impedance_project(
        r#"      differential_pairs:
        - first_net: DP
          second_net: DM
          source: fab_stackup_table_rev_a
          target_differential_impedance_ohm: 90
          expected_width_mm: 0.15
          expected_gap_mm: 0.12
          max_width_error_mm: 0.02
          max_gap_error_mm: 0.03
"#,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_GEOMETRY_VALID");
    assert_eq!(failure["measured"]["first_net"], "DP");
    assert_eq!(failure["measured"]["second_net"], "DM");
    assert_eq!(
        failure["measured"]["target_source"],
        "fab_stackup_table_rev_a"
    );
    assert_eq!(
        failure["measured"]["target_differential_impedance_ohm"],
        90.0
    );
    assert!(["DP", "DM"].contains(&failure["measured"]["worst_width_net"].as_str().unwrap()));
    assert_eq!(failure["measured"]["gap_layer"], "F.Cu");
    let measured_gap = failure["measured"]["measured_gap_mm"].as_f64().unwrap();
    let gap_error = failure["measured"]["gap_error_mm"].as_f64().unwrap();
    assert!((measured_gap - 0.2).abs() < 1.0e-12);
    assert!((gap_error - 0.08).abs() < 1.0e-12);
    assert_eq!(failure["measured"]["width_violation"], false);
    assert_eq!(failure["measured"]["gap_violation"], true);
    assert_eq!(failure["limit"]["expected_gap_mm"], 0.12);
    assert_eq!(failure["limit"]["max_gap_error_mm"], 0.03);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_geometry_fails_closed_without_parallel_pair_evidence() {
    let (_dir, project_path) = write_impedance_project(
        r#"      differential_pairs:
        - first_net: DP
          second_net: DM
          source: fab_stackup_table_rev_a
          target_differential_impedance_ohm: 90
          expected_width_mm: 0.15
          expected_gap_mm: 0.20
          max_width_error_mm: 0.02
          max_gap_error_mm: 0.03
"#,
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          end: { x_mm: 10.0, y_mm: 0.35 }",
        "          end: { x_mm: 0.0, y_mm: 10.0 }",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("has no parallel overlapping same-layer route evidence")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_stackup_evidence_passes_for_explicit_layer_metadata() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      routes:
        - net: RF
          route_layer: F.Cu
          reference_layer: In1.GND
          dielectric_layer: prepreg_1
"#,
        "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_stackup_evidence_fails_for_non_between_dielectric() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      routes:
        - net: RF
          route_layer: F.Cu
          reference_layer: In1.GND
          dielectric_layer: core_1
"#,
        "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID");
    assert_eq!(failure["measured"]["net"], "RF");
    assert_eq!(failure["measured"]["route_layer"], "F.Cu");
    assert_eq!(failure["measured"]["reference_layer"], "In1.GND");
    assert_eq!(failure["measured"]["dielectric_layer"], "core_1");
    assert_eq!(failure["measured"]["route_layer_index"], 0);
    assert_eq!(failure["measured"]["reference_layer_index"], 2);
    assert_eq!(failure["measured"]["dielectric_layer_index"], 3);
    assert_eq!(failure["measured"]["route_copper_thickness_um"], 35.0);
    assert_eq!(failure["measured"]["reference_copper_thickness_um"], 17.5);
    assert_eq!(failure["measured"]["dielectric_thickness_mm"], 0.60);
    assert_eq!(failure["measured"]["dielectric_constant"], 4.2);
    assert_eq!(failure["measured"]["dielectric_material"], "FR-4 core");
    assert_eq!(
        failure["limit"]["dielectric_layer_must_be_between_route_and_reference"],
        true
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_stackup_evidence_fails_closed_without_copper_thickness() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      routes:
        - net: RF
          route_layer: F.Cu
          reference_layer: In1.GND
          dielectric_layer: prepreg_1
"#,
        "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace("          copper_thickness_um: 35.0\n", "");
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("stackup layer F.Cu must declare finite positive copper_thickness_um")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solder_mask_loading_passes_for_covered_route() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      routes:
        - net: RF
          route_layer: F.Cu
          solder_mask_layer: F.Mask
          expected_solder_mask_state: covered
          source: fab_stackup_table_rev_a
"#,
        "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solder_mask_loading_passes_for_opened_route() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      routes:
        - net: RF
          route_layer: F.Cu
          solder_mask_layer: F.Mask
          expected_solder_mask_state: opened
          source: fab_stackup_table_rev_a
"#,
        "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "at: { x_mm: 20.0, y_mm: 20.0 }",
        "at: { x_mm: 5.0, y_mm: 2.0 }",
    );
    project = project.replace(
        "size: { x_mm: 1.0, y_mm: 1.0 }",
        "size: { x_mm: 11.0, y_mm: 1.0 }",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solder_mask_loading_fails_for_opening_on_covered_route() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      routes:
        - net: RF
          route_layer: F.Cu
          solder_mask_layer: F.Mask
          expected_solder_mask_state: covered
          source: fab_stackup_table_rev_a
"#,
        "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "at: { x_mm: 20.0, y_mm: 20.0 }",
        "at: { x_mm: 5.0, y_mm: 2.0 }",
    );
    project = project.replace(
        "size: { x_mm: 1.0, y_mm: 1.0 }",
        "size: { x_mm: 11.0, y_mm: 1.0 }",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(
        failure["id"],
        "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID"
    );
    assert_eq!(failure["measured"]["net"], "RF");
    assert_eq!(failure["measured"]["route_layer"], "F.Cu");
    assert_eq!(failure["measured"]["solder_mask_layer"], "F.Mask");
    assert_eq!(
        failure["measured"]["target_source"],
        "fab_stackup_table_rev_a"
    );
    assert_eq!(failure["measured"]["measured_solder_mask_state"], "opened");
    assert_eq!(failure["limit"]["expected_solder_mask_state"], "covered");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solder_mask_loading_fails_closed_without_mask_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      routes:
        - net: RF
          route_layer: F.Cu
          solder_mask_layer: F.Mask
          expected_solder_mask_state: covered
          source: fab_stackup_table_rev_a
"#,
        "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    let mask_block = r#"    solder_mask:
      features:
        - at: { x_mm: 20.0, y_mm: 20.0 }
          layer: F.Mask
          polarity: dark
          net: RF
          source_primitive: gerber_flash
          source_primitive_index: 0
          aperture: D10
          shape: rect
          size: { x_mm: 1.0, y_mm: 1.0 }
"#;
    project = project.replace(mask_block, "");
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("requires imported dark solder-mask opening evidence")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_passes_for_reviewed_measurement_within_tolerance() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: rf_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_fails_for_out_of_tolerance_measurement() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: dp_dm_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_COUPON_VALID");
    assert_eq!(failure["measured"]["coupon_name"], "dp_dm_coupon");
    assert_eq!(failure["measured"]["coupon_type"], "differential");
    assert_eq!(failure["measured"]["first_net"], "DP");
    assert_eq!(failure["measured"]["second_net"], "DM");
    assert_eq!(failure["measured"]["source"], "fab_coupon_report_rev_b");
    assert_eq!(failure["measured"]["target_impedance_ohm"], 90.0);
    assert_eq!(failure["measured"]["measured_impedance_ohm"], 96.0);
    assert_eq!(failure["measured"]["impedance_error_ohm"], 6.0);
    assert_eq!(failure["limit"]["max_impedance_error_ohm"], 5.0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_fails_closed_without_named_coupon_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: missing_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("coupon missing_coupon is absent")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_fails_closed_without_matching_board_target() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: rf_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        r#"      nets:
        - net: RF
          source: fab_stackup_table_rev_a
          target_impedance_ohm: 50
          expected_width_mm: 0.20
          max_width_error_mm: 0.03
"#,
        "",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("requires exactly one reviewed board controlled-impedance target for RF")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_fails_closed_for_mismatched_board_target() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: dp_dm_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          target_differential_impedance_ohm: 90",
        "          target_differential_impedance_ohm: 85",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("conflicts with reviewed board differential target")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_batch_passes_for_reviewed_samples_within_limits() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: rf_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_batch_fails_for_out_of_limit_statistics() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: rf_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          measured_impedance_ohm: 51.2\n        - name: dp_dm_coupon",
        "          measured_impedance_ohm: 54.0\n        - name: dp_dm_coupon",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID");
    assert_eq!(failure["measured"]["coupon_name"], "rf_coupon");
    assert_eq!(failure["measured"]["coupon_type"], "single_ended");
    assert_eq!(failure["measured"]["sample_count"], 3);
    assert_eq!(failure["measured"]["target_impedance_ohm"], 50.0);
    assert_eq!(failure["limit"]["min_batch_sample_count"], 3);
    assert_eq!(failure["limit"]["max_batch_mean_impedance_error_ohm"], 1.5);
    assert_eq!(
        failure["limit"]["max_batch_sample_impedance_error_ohm"],
        2.0
    );
    assert_eq!(failure["limit"]["max_batch_stddev_ohm"], 0.5);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_batch_fails_closed_without_reviewed_limits() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: rf_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace("          min_batch_sample_count: 3\n", "");
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("requires reviewed min_batch_sample_count")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_trace_correlation_passes_for_reviewed_route_match() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: rf_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_trace_correlation_fails_for_route_width_mismatch() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: rf_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace("            width_mm: 0.18", "            width_mm: 0.24");
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(
        failure["id"],
        "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID"
    );
    assert_eq!(failure["measured"]["coupon_name"], "rf_coupon");
    assert_eq!(failure["measured"]["process_lot"], "lot_2026_06_a");
    assert_eq!(failure["measured"]["panel_id"], "panel_7");
    assert_eq!(failure["measured"]["stackup_revision"], "stackup_rev_a");
    assert_eq!(failure["measured"]["coupon_trace_layer"], "F.Cu");
    assert_eq!(failure["measured"]["measured_width_mm"], 0.24);
    assert_eq!(failure["limit"]["coupon_trace_width_mm"], 0.20);
    assert_eq!(failure["limit"]["max_trace_width_delta_mm"], 0.03);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_coupon_trace_correlation_fails_closed_without_process_tags() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      coupons:
        - name: rf_coupon
"#,
        "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace("          process_lot: lot_2026_06_a\n", "");
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("requires reviewed process_lot")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_for_reviewed_solver_and_route_match() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_with_signed_artifact_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_artifact_signature_uri: artifacts/solver/rf_solver_result.sig\n          solver_artifact_signature_sha256: 1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef\n          solver_artifact_signer: si_review_key_2026\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_for_partial_signed_artifact_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_artifact_signature_uri: artifacts/solver/rf_solver_result.sig\n          solver_artifact_signer: si_review_key_2026\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("solver_artifact_signature_sha256")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_with_stackup_signoff_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          stackup_revision: stackup_rev_a\n",
        "          stackup_revision: stackup_rev_a\n          stackup_signoff_source: fabricator_stackup_review_rev_a\n          fabricator_stackup_revision: stackup_rev_a\n          stackup_signoff_artifact_uri: artifacts/fabricator/stackup_signoff_rev_a.pdf\n          stackup_signoff_artifact_sha256: 111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_for_partial_stackup_signoff_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          stackup_revision: stackup_rev_a\n",
        "          stackup_revision: stackup_rev_a\n          stackup_signoff_source: fabricator_stackup_review_rev_a\n          fabricator_stackup_revision: stackup_rev_a\n          stackup_signoff_artifact_uri: artifacts/fabricator/stackup_signoff_rev_a.pdf\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("stackup_signoff_artifact_sha256")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_for_stale_stackup_signoff_revision() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          stackup_revision: stackup_rev_a\n",
        "          stackup_revision: stackup_rev_a\n          stackup_signoff_source: fabricator_stackup_review_rev_old\n          fabricator_stackup_revision: stackup_rev_old\n          stackup_signoff_artifact_uri: artifacts/fabricator/stackup_signoff_rev_old.pdf\n          stackup_signoff_artifact_sha256: 111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("fabricator_stackup_revision")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_with_material_library_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          input_etch_compensation_um: 8.0\n",
        "          input_etch_compensation_um: 8.0\n          solver_material_library: reviewed_stackup_materials\n          solver_material_library_revision: rev_a\n          solver_material_library_artifact_uri: artifacts/solver/material_library_rev_a.json\n          solver_material_library_artifact_sha256: abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd\n          input_material_library: reviewed_stackup_materials\n          input_material_library_revision: rev_a\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_material_library_artifact_content_gap() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          input_etch_compensation_um: 8.0\n",
        "          input_etch_compensation_um: 8.0\n          solver_material_library: reviewed_stackup_materials\n          solver_material_library_revision: rev_a\n          solver_material_library_artifact_uri: artifacts/solver/material_library_rev_a.json\n          solver_material_library_artifact_sha256: abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd\n          input_material_library: reviewed_stackup_materials\n          input_material_library_revision: rev_a\n",
    );
    project = project.replace(
        "          corners: [nominal, high_dk]\n",
        "          corners: [nominal]\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("does not declare required corner high_dk")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_material_library_mismatch() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          input_etch_compensation_um: 8.0\n",
        "          input_etch_compensation_um: 8.0\n          solver_material_library: reviewed_stackup_materials\n          solver_material_library_revision: rev_a\n          solver_material_library_artifact_uri: artifacts/solver/material_library_rev_a.json\n          solver_material_library_artifact_sha256: abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd\n          input_material_library: stale_stackup_materials\n          input_material_library_revision: rev_a\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID");
    assert_eq!(
        failure["measured"]["solver_material_library"],
        "reviewed_stackup_materials"
    );
    assert_eq!(
        failure["measured"]["input_material_library"],
        "stale_stackup_materials"
    );
    assert!(
        failure["measured"]["input_deck_mismatches"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("solver_material_library"))
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_for_partial_material_library_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          input_etch_compensation_um: 8.0\n",
        "          input_etch_compensation_um: 8.0\n          solver_material_library: reviewed_stackup_materials\n          solver_material_library_revision: rev_a\n          input_material_library: reviewed_stackup_materials\n          input_material_library_revision: rev_a\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("material-library metadata")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_with_material_corner_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          input_etch_compensation_um: 8.0\n",
        "          input_etch_compensation_um: 8.0\n          solver_material_library: reviewed_stackup_materials\n          solver_material_library_revision: rev_a\n          solver_material_library_artifact_uri: artifacts/solver/material_library_rev_a.json\n          solver_material_library_artifact_sha256: abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd\n          input_material_library: reviewed_stackup_materials\n          input_material_library_revision: rev_a\n",
    );
    project = project.replace(
        "              solved_impedance_ohm: 49.5\n        - name: dp_dm_solver_result\n",
        "              solved_impedance_ohm: 49.5\n          material_corners:\n            - name: rf_solver_nominal_material\n              source: solver_material_library_rev_a\n              corner: nominal\n              dielectric_layer: prepreg_1\n              material: FR-4 prepreg\n              dielectric_constant: 4.1\n              nominal_dielectric_constant: 4.1\n              material_library: reviewed_stackup_materials\n              material_library_revision: rev_a\n            - name: rf_solver_high_dk_material\n              source: solver_material_library_rev_a\n              corner: high_dk\n              dielectric_layer: prepreg_1\n              material: FR-4 prepreg\n              dielectric_constant: 4.4\n              nominal_dielectric_constant: 4.1\n              material_library: reviewed_stackup_materials\n              material_library_revision: rev_a\n        - name: dp_dm_solver_result\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_for_material_corner_stackup_mismatch() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          input_etch_compensation_um: 8.0\n",
        "          input_etch_compensation_um: 8.0\n          solver_material_library: reviewed_stackup_materials\n          solver_material_library_revision: rev_a\n          solver_material_library_artifact_uri: artifacts/solver/material_library_rev_a.json\n          solver_material_library_artifact_sha256: abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd\n          input_material_library: reviewed_stackup_materials\n          input_material_library_revision: rev_a\n",
    );
    project = project.replace(
        "              solved_impedance_ohm: 49.5\n        - name: dp_dm_solver_result\n",
        "              solved_impedance_ohm: 49.5\n          material_corners:\n            - name: rf_solver_nominal_material\n              source: solver_material_library_rev_a\n              corner: nominal\n              dielectric_layer: prepreg_1\n              material: PTFE laminate\n              dielectric_constant: 4.1\n              nominal_dielectric_constant: 4.1\n              material_library: reviewed_stackup_materials\n              material_library_revision: rev_a\n            - name: rf_solver_high_dk_material\n              source: solver_material_library_rev_a\n              corner: high_dk\n              dielectric_layer: prepreg_1\n              material: FR-4 prepreg\n              dielectric_constant: 4.4\n              nominal_dielectric_constant: 4.1\n              material_library: reviewed_stackup_materials\n              material_library_revision: rev_a\n        - name: dp_dm_solver_result\n",
    );
    project = project.replace(
        "          materials: [FR-4 prepreg]\n",
        "          materials: [FR-4 prepreg, PTFE laminate]\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("does not match stackup layer")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_with_reviewed_solver_qualification() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "      solver_results:\n",
        "      solver_qualifications:\n        - name: reviewed_2d_field_solver_2026_06\n          source: si_tool_qualification_rev_a\n          solver: reviewed_2d_field_solver\n          solver_version: \"2026.06\"\n          qualification_artifact_uri: artifacts/solver/reviewed_2d_field_solver_2026_06_qualification.pdf\n          qualification_artifact_sha256: 00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff\n      solver_results:\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_for_unqualified_solver_version() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "      solver_results:\n",
        "      solver_qualifications:\n        - name: reviewed_2d_field_solver_2026_05\n          source: si_tool_qualification_rev_a\n          solver: reviewed_2d_field_solver\n          solver_version: \"2026.05\"\n          qualification_artifact_uri: artifacts/solver/reviewed_2d_field_solver_2026_05_qualification.pdf\n          qualification_artifact_sha256: 00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff\n      solver_results:\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("requires exactly one reviewed solver qualification")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_out_of_tolerance_solver_result() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          solved_impedance_ohm: 50.8",
        "          solved_impedance_ohm: 56.0",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID");
    assert_eq!(failure["measured"]["result"], "rf_solver_result");
    assert_eq!(failure["measured"]["solver"], "reviewed_2d_field_solver");
    assert_eq!(
        failure["measured"]["solver_artifact_uri"],
        "artifacts/solver/rf_solver_result.json"
    );
    assert_eq!(
        failure["measured"]["solver_artifact_sha256"],
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_eq!(failure["measured"]["solved_impedance_ohm"], 56.0);
    assert_eq!(failure["limit"]["max_impedance_error_ohm"], 2.0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_out_of_tolerance_solver_sample() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "              solved_impedance_ohm: 49.5",
        "              solved_impedance_ohm: 46.5",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["measured"].get("worst_sample").is_some())
        .unwrap();
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID");
    assert_eq!(failure["measured"]["result"], "rf_solver_result");
    assert_eq!(
        failure["measured"]["worst_sample"],
        "rf_solver_high_dk_2900"
    );
    assert_eq!(failure["measured"]["max_sample_impedance_error_ohm"], 3.5);
    assert_eq!(failure["limit"]["max_impedance_error_ohm"], 2.0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_solver_sweep_frequency_gap() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "              frequency_mhz: 2900.0",
        "              frequency_mhz: 3200.0",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| {
            failure["measured"]
                .get("max_solver_frequency_gap_mhz")
                .is_some()
        })
        .unwrap();
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID");
    assert_eq!(failure["measured"]["result"], "rf_solver_result");
    assert_eq!(failure["measured"]["max_solver_frequency_gap_mhz"], 800.0);
    assert_eq!(failure["limit"]["max_solver_frequency_step_mhz"], 500.0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_input_deck_mismatch() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          input_width_mm: 0.20",
        "          input_width_mm: 0.24",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["measured"].get("input_deck_mismatches").is_some())
        .unwrap();
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID");
    assert_eq!(failure["measured"]["result"], "rf_solver_result");
    assert_eq!(
        failure["measured"]["solver_input_deck_uri"],
        "artifacts/solver/rf_solver_input_deck.json"
    );
    assert_eq!(
        failure["measured"]["input_deck_mismatches"][0],
        "solved_width_mm"
    );
    assert_eq!(failure["measured"]["input_width_mm"], 0.24);
    assert_eq!(failure["limit"]["solved_width_mm"], 0.20);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_without_input_deck_digest() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          solver_input_deck_sha256: fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210\n",
        "",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("solver_input_deck_sha256")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_copper_roughness_mismatch() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          input_copper_roughness_um: 1.5",
        "          input_copper_roughness_um: 2.0",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["measured"].get("input_deck_mismatches").is_some())
        .unwrap();
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID");
    assert_eq!(failure["measured"]["result"], "rf_solver_result");
    assert_eq!(
        failure["measured"]["input_deck_mismatches"][0],
        "copper_roughness_um"
    );
    assert_eq!(failure["measured"]["input_copper_roughness_um"], 2.0);
    assert_eq!(failure["limit"]["copper_roughness_um"], 1.5);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_for_partial_copper_roughness_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace("          input_copper_roughness_um: 1.5\n", "");
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("copper roughness metadata")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_etch_compensation_mismatch() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          input_etch_compensation_um: 8.0",
        "          input_etch_compensation_um: 12.0",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["measured"].get("input_deck_mismatches").is_some())
        .unwrap();
    assert_eq!(failure["id"], "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID");
    assert_eq!(failure["measured"]["result"], "rf_solver_result");
    assert_eq!(
        failure["measured"]["input_deck_mismatches"][0],
        "etch_compensation_um"
    );
    assert_eq!(failure["measured"]["input_etch_compensation_um"], 12.0);
    assert_eq!(failure["limit"]["etch_compensation_um"], 8.0);
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_for_partial_etch_compensation_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace("          input_etch_compensation_um: 8.0\n", "");
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("etch compensation metadata")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_without_artifact_digest() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("solver_artifact_sha256")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_without_stackup_layer_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          reference_layer: In1.GND",
        "          reference_layer: L2.GND",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("references stackup layer L2.GND absent")
    );
    assert_report_schema_valid(&report);
}

fn write_impedance_project(parameters: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    write_impedance_project_with_check(parameters, "CONTROLLED_IMPEDANCE_GEOMETRY_VALID")
}

fn write_impedance_project_with_check(
    parameters: &str,
    check: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let project_path = dir.path().join("project.yaml");
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  name: controlled_impedance_fixture
  version: 1
libraries: []
board:
  components: {{}}
  nets:
    RF:
      kind: digital_or_analog
    DP:
      kind: digital_or_analog
    DM:
      kind: digital_or_analog
    GND:
      kind: ground
  manufacturing:
    controlled_impedance:
      nets:
        - net: RF
          source: fab_stackup_table_rev_a
          target_impedance_ohm: 50
          expected_width_mm: 0.20
          max_width_error_mm: 0.03
      differential_pairs:
        - first_net: DP
          second_net: DM
          source: fab_stackup_table_rev_a
          target_differential_impedance_ohm: 90
          expected_width_mm: 0.15
          expected_gap_mm: 0.20
          max_width_error_mm: 0.02
          max_gap_error_mm: 0.03
      coupons:
        - name: rf_coupon
          source: fab_coupon_report_rev_b
          coupon_type: single_ended
          net: RF
          target_impedance_ohm: 50.0
          measured_impedance_ohm: 51.2
          max_impedance_error_ohm: 3.0
          process_lot: lot_2026_06_a
          panel_id: panel_7
          stackup_revision: stackup_rev_a
          coupon_trace_layer: F.Cu
          coupon_trace_width_mm: 0.20
          max_trace_width_delta_mm: 0.03
          min_batch_sample_count: 3
          max_batch_mean_impedance_error_ohm: 1.5
          max_batch_sample_impedance_error_ohm: 2.0
          max_batch_stddev_ohm: 0.5
          samples:
            - name: rf_coupon_s1
              source: fab_coupon_report_rev_b
              measured_impedance_ohm: 50.8
            - name: rf_coupon_s2
              source: fab_coupon_report_rev_b
              measured_impedance_ohm: 51.0
            - name: rf_coupon_s3
              source: fab_coupon_report_rev_b
              measured_impedance_ohm: 51.2
        - name: dp_dm_coupon
          source: fab_coupon_report_rev_b
          coupon_type: differential
          first_net: DP
          second_net: DM
          target_impedance_ohm: 90.0
          measured_impedance_ohm: 96.0
          max_impedance_error_ohm: 5.0
          process_lot: lot_2026_06_a
          panel_id: panel_7
          stackup_revision: stackup_rev_a
          coupon_trace_layer: F.Cu
          coupon_trace_width_mm: 0.15
          max_trace_width_delta_mm: 0.02
          coupon_trace_gap_mm: 0.20
          max_trace_gap_delta_mm: 0.03
          min_batch_sample_count: 3
          max_batch_mean_impedance_error_ohm: 1.5
          max_batch_sample_impedance_error_ohm: 2.0
          max_batch_stddev_ohm: 0.5
          samples:
            - name: dp_dm_coupon_s1
              source: fab_coupon_report_rev_b
              measured_impedance_ohm: 90.6
            - name: dp_dm_coupon_s2
              source: fab_coupon_report_rev_b
              measured_impedance_ohm: 91.0
            - name: dp_dm_coupon_s3
              source: fab_coupon_report_rev_b
              measured_impedance_ohm: 91.4
      solver_material_libraries:
        - name: reviewed_stackup_materials_rev_a
          source: solver_material_library_rev_a
          material_library: reviewed_stackup_materials
          material_library_revision: rev_a
          artifact_uri: artifacts/solver/material_library_rev_a.json
          artifact_sha256: abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd
          corners: [nominal, high_dk]
          dielectric_layers: [prepreg_1]
          materials: [FR-4 prepreg]
      solver_results:
        - name: rf_solver_result
          source: solver_report_rev_c
          solver: reviewed_2d_field_solver
          solver_version: "2026.06"
          solver_artifact_uri: artifacts/solver/rf_solver_result.json
          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
          solver_input_deck_uri: artifacts/solver/rf_solver_input_deck.json
          solver_input_deck_sha256: fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210
          result_type: single_ended
          net: RF
          target_impedance_ohm: 50.0
          solved_impedance_ohm: 50.8
          max_impedance_error_ohm: 2.0
          stackup_revision: stackup_rev_a
          route_layer: F.Cu
          reference_layer: In1.GND
          dielectric_layer: prepreg_1
          solved_width_mm: 0.20
          max_route_width_delta_mm: 0.03
          input_stackup_revision: stackup_rev_a
          input_route_layer: F.Cu
          input_reference_layer: In1.GND
          input_dielectric_layer: prepreg_1
          input_width_mm: 0.20
          frequency_mhz: 2400.0
          input_frequency_mhz: 2400.0
          copper_roughness_model: huray
          copper_roughness_um: 1.5
          input_copper_roughness_model: huray
          input_copper_roughness_um: 1.5
          etch_compensation_model: fabricator_finished_width_bias
          etch_compensation_um: 8.0
          input_etch_compensation_model: fabricator_finished_width_bias
          input_etch_compensation_um: 8.0
          min_solver_sample_count: 4
          max_solver_frequency_step_mhz: 500.0
          required_solver_corners: [nominal, high_dk]
          samples:
            - name: rf_solver_nominal_2400
              source: solver_report_rev_c
              corner: nominal
              frequency_mhz: 2400.0
              solved_impedance_ohm: 50.8
            - name: rf_solver_nominal_2900
              source: solver_report_rev_c
              corner: nominal
              frequency_mhz: 2900.0
              solved_impedance_ohm: 50.9
            - name: rf_solver_high_dk_2400
              source: solver_report_rev_c
              corner: high_dk
              frequency_mhz: 2400.0
              solved_impedance_ohm: 49.4
            - name: rf_solver_high_dk_2900
              source: solver_report_rev_c
              corner: high_dk
              frequency_mhz: 2900.0
              solved_impedance_ohm: 49.5
        - name: dp_dm_solver_result
          source: solver_report_rev_c
          solver: reviewed_2d_field_solver
          solver_version: "2026.06"
          solver_artifact_uri: artifacts/solver/dp_dm_solver_result.json
          solver_artifact_sha256: abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789
          result_type: differential
          first_net: DP
          second_net: DM
          target_impedance_ohm: 90.0
          solved_impedance_ohm: 91.0
          max_impedance_error_ohm: 3.0
          stackup_revision: stackup_rev_a
          route_layer: F.Cu
          reference_layer: In1.GND
          dielectric_layer: prepreg_1
          solved_width_mm: 0.15
          max_route_width_delta_mm: 0.02
          solved_gap_mm: 0.20
          max_route_gap_delta_mm: 0.03
          frequency_mhz: 2400.0
  layout:
    stackup:
      layers:
        - name: F.Cu
          kind: signal
          copper_thickness_um: 35.0
          source: fab_stackup_table_rev_a
        - name: prepreg_1
          kind: dielectric
          thickness_mm: 0.18
          dielectric_constant: 4.1
          material: FR-4 prepreg
          source: fab_stackup_table_rev_a
        - name: In1.GND
          kind: plane
          reference_net: GND
          copper_thickness_um: 17.5
          source: fab_stackup_table_rev_a
        - name: core_1
          kind: dielectric
          thickness_mm: 0.60
          dielectric_constant: 4.2
          material: FR-4 core
          source: fab_stackup_table_rev_a
        - name: B.Cu
          kind: signal
          copper_thickness_um: 35.0
          source: fab_stackup_table_rev_a
    routes:
      RF:
        segments:
          - start: {{ x_mm: 0.0, y_mm: 2.0 }}
            end: {{ x_mm: 10.0, y_mm: 2.0 }}
            width_mm: 0.18
            layer: F.Cu
      DP:
        segments:
          - start: {{ x_mm: 0.0, y_mm: 0.0 }}
            end: {{ x_mm: 10.0, y_mm: 0.0 }}
            width_mm: 0.15
            layer: F.Cu
      DM:
        segments:
          - start: {{ x_mm: 0.0, y_mm: 0.35 }}
            end: {{ x_mm: 10.0, y_mm: 0.35 }}
            width_mm: 0.15
            layer: F.Cu
    solder_mask:
      features:
        - at: {{ x_mm: 20.0, y_mm: 20.0 }}
          layer: F.Mask
          polarity: dark
          net: RF
          source_primitive: gerber_flash
          source_primitive_index: 0
          aperture: D10
          shape: rect
          size: {{ x_mm: 1.0, y_mm: 1.0 }}
scenarios:
  - name: controlled_impedance_geometry
    type: manufacturing
    checks:
      - {check}
    parameters:
{parameters}"#
        ),
    )
    .unwrap();
    (dir, project_path)
}
