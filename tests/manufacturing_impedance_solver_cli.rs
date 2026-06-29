mod common;

use common::impedance::write_impedance_project_with_check;
use common::{assert_report_schema_valid, run_validation};

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
fn controlled_impedance_solver_result_passes_with_output_schema_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_output_schema: circuitci_controlled_impedance_solver_result\n          solver_output_schema_version: \"1.0\"\n          solver_output_schema_uri: artifacts/solver/controlled_impedance_solver_result_schema_v1.json\n          solver_output_schema_sha256: 55556666777788889999aaaabbbbccccddddeeeeffff00001111222233334444\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_for_partial_output_schema_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_output_schema: circuitci_controlled_impedance_solver_result\n          solver_output_schema_uri: artifacts/solver/controlled_impedance_solver_result_schema_v1.json\n",
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
            .contains("solver_output_schema_version")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_with_config_lock_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_config_lock_uri: artifacts/solver/reviewed_2d_field_solver_config_lock_rev_c.json\n          solver_config_lock_sha256: 6666777788889999aaaabbbbccccddddeeeeffff000011112222333344445555\n          solver_config_lock_tool: reviewed_2d_field_solver\n          solver_config_lock_revision: config_lock_rev_c\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_closed_for_partial_config_lock_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_config_lock_uri: artifacts/solver/reviewed_2d_field_solver_config_lock_rev_c.json\n          solver_config_lock_tool: reviewed_2d_field_solver\n",
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
            .contains("solver_config_lock_sha256")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_config_lock_tool_mismatch() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_config_lock_uri: artifacts/solver/stale_solver_config_lock_rev_b.json\n          solver_config_lock_sha256: 6666777788889999aaaabbbbccccddddeeeeffff000011112222333344445555\n          solver_config_lock_tool: stale_field_solver\n          solver_config_lock_revision: config_lock_rev_b\n",
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
            .contains("solver_config_lock_tool")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_with_runtime_allowlist_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "      solver_results:\n",
        "      solver_runtime_allowlists:\n        - name: reviewed_2d_field_solver_runtime_lock_c\n          source: si_runtime_review_rev_a\n          solver: reviewed_2d_field_solver\n          solver_config_lock_revision: config_lock_rev_c\n          runtime_profile: production_si\n          allowlist_revision: runtime_allowlist_rev_a\n          artifact_uri: artifacts/solver/runtime_allowlist_rev_a.json\n          artifact_sha256: 777788889999aaaabbbbccccddddeeeeffff0000111122223333444455556666\n          allowed_options: [quasi_static, finite_thickness_copper, huray_roughness, fabricator_etch_bias]\n      solver_results:\n",
    );
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_config_lock_uri: artifacts/solver/reviewed_2d_field_solver_config_lock_rev_c.json\n          solver_config_lock_sha256: 6666777788889999aaaabbbbccccddddeeeeffff000011112222333344445555\n          solver_config_lock_tool: reviewed_2d_field_solver\n          solver_config_lock_revision: config_lock_rev_c\n          solver_runtime_allowlist: reviewed_2d_field_solver_runtime_lock_c\n          solver_runtime_profile: production_si\n          solver_runtime_options: [quasi_static, finite_thickness_copper, huray_roughness]\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_runtime_option_not_allowlisted() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "      solver_results:\n",
        "      solver_runtime_allowlists:\n        - name: reviewed_2d_field_solver_runtime_lock_c\n          source: si_runtime_review_rev_a\n          solver: reviewed_2d_field_solver\n          solver_config_lock_revision: config_lock_rev_c\n          runtime_profile: production_si\n          allowlist_revision: runtime_allowlist_rev_a\n          artifact_uri: artifacts/solver/runtime_allowlist_rev_a.json\n          artifact_sha256: 777788889999aaaabbbbccccddddeeeeffff0000111122223333444455556666\n          allowed_options: [quasi_static, finite_thickness_copper]\n      solver_results:\n",
    );
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_config_lock_uri: artifacts/solver/reviewed_2d_field_solver_config_lock_rev_c.json\n          solver_config_lock_sha256: 6666777788889999aaaabbbbccccddddeeeeffff000011112222333344445555\n          solver_config_lock_tool: reviewed_2d_field_solver\n          solver_config_lock_revision: config_lock_rev_c\n          solver_runtime_allowlist: reviewed_2d_field_solver_runtime_lock_c\n          solver_runtime_profile: production_si\n          solver_runtime_options: [quasi_static, disallowed_fast_mode]\n",
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
            .contains("disallowed_fast_mode")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_with_entitlement_evidence() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "      solver_results:\n",
        "      solver_entitlements:\n        - name: reviewed_2d_field_solver_si_entitlement\n          source: si_license_review_rev_a\n          solver: reviewed_2d_field_solver\n          solver_version: \"2026.06\"\n          entitlement_id: si_solver_floating_pool_a\n          entitlement_revision: entitlement_rev_2026_06\n          artifact_uri: artifacts/solver/license_entitlement_rev_2026_06.json\n          artifact_sha256: 88889999aaaabbbbccccddddeeeeffff00001111222233334444555566667777\n          licensed_features: [2d_field_solver, lossy_copper_roughness]\n      solver_results:\n",
    );
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_entitlement: reviewed_2d_field_solver_si_entitlement\n          solver_entitlement_features: [2d_field_solver, lossy_copper_roughness]\n",
    );
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_unlicensed_entitlement_feature() {
    let (_dir, project_path) = write_impedance_project_with_check(
        r#"      solver_results:
        - name: rf_solver_result
"#,
        "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace(
        "      solver_results:\n",
        "      solver_entitlements:\n        - name: reviewed_2d_field_solver_si_entitlement\n          source: si_license_review_rev_a\n          solver: reviewed_2d_field_solver\n          solver_version: \"2026.06\"\n          entitlement_id: si_solver_floating_pool_a\n          entitlement_revision: entitlement_rev_2026_06\n          artifact_uri: artifacts/solver/license_entitlement_rev_2026_06.json\n          artifact_sha256: 88889999aaaabbbbccccddddeeeeffff00001111222233334444555566667777\n          licensed_features: [2d_field_solver]\n      solver_results:\n",
    );
    project = project.replace(
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
        "          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n          solver_entitlement: reviewed_2d_field_solver_si_entitlement\n          solver_entitlement_features: [2d_field_solver, unlicensed_3d_solver]\n",
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
            .contains("unlicensed_3d_solver")
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
fn controlled_impedance_solver_result_fails_for_material_library_field_gap() {
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
        "          content_fields: [corner, dielectric_layer, material, dielectric_constant, nominal_dielectric_constant]\n",
        "          content_fields: [corner, dielectric_layer, material, nominal_dielectric_constant]\n",
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
            .contains("does not declare required artifact content field dielectric_constant")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_material_acceptance_gap() {
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
        "          accepted_corners: [nominal, high_dk]\n",
        "          accepted_corners: [nominal]\n",
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
            .contains("does not accept required corner high_dk")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_material_process_drift_gap() {
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
        "          measured_dielectric_constant: 4.12\n",
        "          measured_dielectric_constant: 4.20\n",
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
            .contains("exceeds reviewed dielectric-constant drift limit")
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
    project = project.replace(
        "          accepted_materials: [FR-4 prepreg]\n",
        "          accepted_materials: [FR-4 prepreg, PTFE laminate]\n",
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
