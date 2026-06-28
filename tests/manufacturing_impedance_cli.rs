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
