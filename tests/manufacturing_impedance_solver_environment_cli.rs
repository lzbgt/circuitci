mod common;

use common::{assert_report_schema_valid, run_validation};

#[test]
fn controlled_impedance_solver_result_passes_with_execution_environment_evidence() {
    let (_dir, project_path) = write_solver_environment_project(
        r#"      solver_execution_environments:
        - name: reviewed_2d_field_solver_env_lock
          source: si_environment_review_rev_a
          solver: reviewed_2d_field_solver
          solver_version: "2026.06"
          environment_id: si_solver_env_a
          environment_revision: env_rev_2026_06
          artifact_uri: artifacts/solver/execution_environment_rev_2026_06.json
          artifact_sha256: 9999aaaabbbbccccddddeeeeffff000011112222333344445555666677778888
          reproducibility_fingerprint: env_fp_2026_06_a
          locked_components: [solver_binary, material_library, config_lock]
      solver_results:
"#,
        r#"          solver_execution_environment: reviewed_2d_field_solver_env_lock
          solver_environment_fingerprint: env_fp_2026_06_a
          solver_environment_components: [solver_binary, material_library]
"#,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_unlocked_environment_component() {
    let (_dir, project_path) = write_solver_environment_project(
        r#"      solver_execution_environments:
        - name: reviewed_2d_field_solver_env_lock
          source: si_environment_review_rev_a
          solver: reviewed_2d_field_solver
          solver_version: "2026.06"
          environment_id: si_solver_env_a
          environment_revision: env_rev_2026_06
          artifact_uri: artifacts/solver/execution_environment_rev_2026_06.json
          artifact_sha256: 9999aaaabbbbccccddddeeeeffff000011112222333344445555666677778888
          reproducibility_fingerprint: env_fp_2026_06_a
          locked_components: [solver_binary, material_library]
      solver_results:
"#,
        r#"          solver_execution_environment: reviewed_2d_field_solver_env_lock
          solver_environment_fingerprint: env_fp_2026_06_a
          solver_environment_components: [solver_binary, unreviewed_plugin]
"#,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("unreviewed_plugin")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_with_run_log_evidence() {
    let (_dir, project_path) = write_solver_environment_project(
        r#"      solver_run_logs:
        - name: reviewed_2d_field_solver_run_log_rf
          source: si_solver_run_review_rev_a
          solver: reviewed_2d_field_solver
          solver_version: "2026.06"
          run_id: rf_solver_run_2026_06_a
          artifact_uri: artifacts/solver/rf_solver_run_2026_06_a.log
          artifact_sha256: aaaabbbbccccddddeeeeffff0000111122223333444455556666777788889999
          random_seed: seed_2026_06_rf
          numeric_tolerance_policy: si_solver_tolerance_rev_a
          max_residual_error: 0.000001
          max_iterations: 120
      solver_results:
"#,
        r#"          solver_run_log: reviewed_2d_field_solver_run_log_rf
          solver_run_id: rf_solver_run_2026_06_a
          solver_random_seed: seed_2026_06_rf
          solver_numeric_tolerance_policy: si_solver_tolerance_rev_a
          solver_residual_error: 0.0000004
          solver_iterations: 84
"#,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_run_log_residual_limit() {
    let (_dir, project_path) = write_solver_environment_project(
        r#"      solver_run_logs:
        - name: reviewed_2d_field_solver_run_log_rf
          source: si_solver_run_review_rev_a
          solver: reviewed_2d_field_solver
          solver_version: "2026.06"
          run_id: rf_solver_run_2026_06_a
          artifact_uri: artifacts/solver/rf_solver_run_2026_06_a.log
          artifact_sha256: aaaabbbbccccddddeeeeffff0000111122223333444455556666777788889999
          random_seed: seed_2026_06_rf
          numeric_tolerance_policy: si_solver_tolerance_rev_a
          max_residual_error: 0.000001
          max_iterations: 120
      solver_results:
"#,
        r#"          solver_run_log: reviewed_2d_field_solver_run_log_rf
          solver_run_id: rf_solver_run_2026_06_a
          solver_random_seed: seed_2026_06_rf
          solver_numeric_tolerance_policy: si_solver_tolerance_rev_a
          solver_residual_error: 0.00002
          solver_iterations: 84
"#,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("residual error")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_passes_with_deterministic_rerun_evidence() {
    let (_dir, project_path) = write_solver_environment_project(
        r#"      solver_run_logs:
        - name: reviewed_2d_field_solver_run_log_rf
          source: si_solver_run_review_rev_a
          solver: reviewed_2d_field_solver
          solver_version: "2026.06"
          run_id: rf_solver_run_2026_06_a
          artifact_uri: artifacts/solver/rf_solver_run_2026_06_a.log
          artifact_sha256: aaaabbbbccccddddeeeeffff0000111122223333444455556666777788889999
          random_seed: seed_2026_06_rf
          numeric_tolerance_policy: si_solver_tolerance_rev_a
          max_residual_error: 0.000001
          max_iterations: 120
          min_rerun_count: 2
          max_rerun_impedance_delta_ohm: 0.05
          reruns:
          - name: rf_solver_rerun_a
            source: si_solver_rerun_review_rev_a
            run_id: rf_solver_run_2026_06_a_rerun_1
            artifact_uri: artifacts/solver/rf_solver_run_2026_06_a_rerun_1.log
            artifact_sha256: 9999888877776666555544443333222211110000ffffeeeeddddccccbbbbaaaa
            random_seed: seed_2026_06_rf
            solved_impedance_ohm: 50.82
            residual_error: 0.0000005
            iterations: 86
          - name: rf_solver_rerun_b
            source: si_solver_rerun_review_rev_a
            run_id: rf_solver_run_2026_06_a_rerun_2
            artifact_uri: artifacts/solver/rf_solver_run_2026_06_a_rerun_2.log
            artifact_sha256: 888877776666555544443333222211110000ffffeeeeddddccccbbbbaaaa9999
            random_seed: seed_2026_06_rf
            solved_impedance_ohm: 50.79
            residual_error: 0.0000004
            iterations: 83
      solver_results:
"#,
        r#"          solver_run_log: reviewed_2d_field_solver_run_log_rf
          solver_run_id: rf_solver_run_2026_06_a
          solver_random_seed: seed_2026_06_rf
          solver_numeric_tolerance_policy: si_solver_tolerance_rev_a
          solver_residual_error: 0.0000004
          solver_iterations: 84
"#,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn controlled_impedance_solver_result_fails_for_rerun_impedance_window() {
    let (_dir, project_path) = write_solver_environment_project(
        r#"      solver_run_logs:
        - name: reviewed_2d_field_solver_run_log_rf
          source: si_solver_run_review_rev_a
          solver: reviewed_2d_field_solver
          solver_version: "2026.06"
          run_id: rf_solver_run_2026_06_a
          artifact_uri: artifacts/solver/rf_solver_run_2026_06_a.log
          artifact_sha256: aaaabbbbccccddddeeeeffff0000111122223333444455556666777788889999
          random_seed: seed_2026_06_rf
          numeric_tolerance_policy: si_solver_tolerance_rev_a
          max_residual_error: 0.000001
          max_iterations: 120
          min_rerun_count: 1
          max_rerun_impedance_delta_ohm: 0.05
          reruns:
          - name: rf_solver_rerun_a
            source: si_solver_rerun_review_rev_a
            run_id: rf_solver_run_2026_06_a_rerun_1
            artifact_uri: artifacts/solver/rf_solver_run_2026_06_a_rerun_1.log
            artifact_sha256: 9999888877776666555544443333222211110000ffffeeeeddddccccbbbbaaaa
            random_seed: seed_2026_06_rf
            solved_impedance_ohm: 50.70
            residual_error: 0.0000005
            iterations: 86
      solver_results:
"#,
        r#"          solver_run_log: reviewed_2d_field_solver_run_log_rf
          solver_run_id: rf_solver_run_2026_06_a
          solver_random_seed: seed_2026_06_rf
          solver_numeric_tolerance_policy: si_solver_tolerance_rev_a
          solver_residual_error: 0.0000004
          solver_iterations: 84
"#,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("deterministic rerun impedance delta")
    );
    assert_report_schema_valid(&report);
}

fn write_solver_environment_project(
    solver_metadata: &str,
    result_environment: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let project_path = dir.path().join("project.yaml");
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  name: impedance_solver_environment_fixture
  version: "1"
libraries: []
board:
  components: {{}}
  nets:
    RF:
      kind: digital_or_analog
      pins: []
  manufacturing:
    controlled_impedance:
      nets:
        - net: RF
          source: fab_stackup_table_rev_a
          target_impedance_ohm: 50
          expected_width_mm: 0.20
          max_width_error_mm: 0.03
{solver_metadata}        - name: rf_solver_result
          source: solver_report_rev_a
          solver: reviewed_2d_field_solver
          solver_version: "2026.06"
          solver_artifact_uri: artifacts/solver/rf_solver_result.json
          solver_artifact_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
{result_environment}          result_type: single_ended
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
    routes:
      RF:
        segments:
          - start: {{ x_mm: 0.0, y_mm: 2.0 }}
            end: {{ x_mm: 10.0, y_mm: 2.0 }}
            width_mm: 0.20
            layer: F.Cu
scenarios:
  - name: controlled_impedance_solver_environment
    type: manufacturing
    checks:
      - CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID
    parameters:
      solver_results:
        - name: rf_solver_result
"#
        ),
    )
    .unwrap();
    (dir, project_path)
}
