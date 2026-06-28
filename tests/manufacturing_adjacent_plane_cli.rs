mod common;

use common::{assert_report_schema_valid, run_validation};

#[test]
fn adjacent_plane_return_path_passes_with_sampled_reference_plane_coverage() {
    let (_dir, project_path) = write_adjacent_plane_project(
        r#"      routes:
        - net: SIG
          reference_net: GND
          max_unreferenced_length_mm: 0.0
"#,
        true,
        "ADJACENT_PLANE_RETURN_PATH_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn adjacent_plane_return_path_fails_when_route_leaves_reference_plane_coverage() {
    let (_dir, project_path) = write_adjacent_plane_project(
        r#"      routes:
        - net: SIG
          reference_net: GND
          max_unreferenced_length_mm: 1.0
"#,
        false,
        "ADJACENT_PLANE_RETURN_PATH_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "ADJACENT_PLANE_RETURN_PATH_VALID");
    assert_eq!(failure["measured"]["net"], "SIG");
    assert_eq!(failure["measured"]["reference_net"], "GND");
    assert_eq!(failure["measured"]["reference_layers"][0], "In1.Cu");
    assert_eq!(
        failure["measured"]["first_unreferenced_route_layer"],
        "F.Cu"
    );
    assert_eq!(
        failure["measured"]["first_unreferenced_reference_layer"],
        "In1.Cu"
    );
    assert_eq!(failure["limit"]["max_unreferenced_length_mm"], 1.0);
    let unreferenced = failure["measured"]["unreferenced_route_length_mm"]
        .as_f64()
        .unwrap();
    assert!((unreferenced - 8.0).abs() < 1.0e-12);
    assert_report_schema_valid(&report);
}

#[test]
fn adjacent_plane_return_path_fails_closed_without_adjacent_plane_stackup() {
    let (_dir, project_path) = write_adjacent_plane_project(
        r#"      routes:
        - net: SIG
          reference_net: GND
          max_unreferenced_length_mm: 0.0
"#,
        true,
        "ADJACENT_PLANE_RETURN_PATH_VALID",
    );
    let mut project = std::fs::read_to_string(&project_path).unwrap();
    project = project.replace("          kind: plane", "          kind: signal");
    std::fs::write(&project_path, project).unwrap();

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("has no adjacent explicit plane")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn reference_plane_slot_crossing_passes_with_continuous_plane_coverage() {
    let (_dir, project_path) = write_adjacent_plane_project(
        r#"      routes:
        - net: SIG
          reference_net: GND
          reference_layer: In1.Cu
          max_slot_crossings: 0
"#,
        true,
        "REFERENCE_PLANE_SLOT_CROSSING_VALID",
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn reference_plane_slot_crossing_fails_when_route_crosses_split_plane_gap() {
    let (_dir, project_path) = write_split_plane_project(
        r#"      routes:
        - net: SIG
          reference_net: GND
          reference_layer: In1.Cu
          max_slot_crossings: 0
"#,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "REFERENCE_PLANE_SLOT_CROSSING_VALID");
    assert_eq!(failure["measured"]["net"], "SIG");
    assert_eq!(failure["measured"]["reference_net"], "GND");
    assert_eq!(failure["measured"]["slot_crossing_count"], 1);
    assert_eq!(failure["measured"]["first_slot_route_layer"], "F.Cu");
    assert_eq!(failure["measured"]["first_slot_reference_layer"], "In1.Cu");
    assert_eq!(failure["limit"]["max_slot_crossings"], 0);
    let start = failure["measured"]["first_slot_start_mm"].as_f64().unwrap();
    let end = failure["measured"]["first_slot_end_mm"].as_f64().unwrap();
    assert!((start - 3.0).abs() < 1.0e-12);
    assert!((end - 5.0).abs() < 1.0e-12);
    assert_report_schema_valid(&report);
}

fn write_adjacent_plane_project(
    parameters: &str,
    covered_zone: bool,
    check: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let project_path = dir.path().join("project.yaml");
    let zone_polygon = if covered_zone {
        r#"
          - layer: In1.Cu
            polygon:
              - { x_mm: 0.0, y_mm: 0.0 }
              - { x_mm: 10.0, y_mm: 0.0 }
              - { x_mm: 10.0, y_mm: 2.0 }
              - { x_mm: 0.0, y_mm: 2.0 }
"#
    } else {
        r#"
          - layer: In1.Cu
            polygon:
              - { x_mm: 0.0, y_mm: 3.0 }
              - { x_mm: 10.0, y_mm: 3.0 }
              - { x_mm: 10.0, y_mm: 5.0 }
              - { x_mm: 0.0, y_mm: 5.0 }
"#
    };
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  name: adjacent_plane_fixture
  version: 1
libraries: []
board:
  components: {{}}
  nets:
    SIG:
      kind: digital_or_analog
    GND:
      kind: ground
  layout:
    stackup:
      layers:
        - name: F.Cu
          kind: signal
          source: reviewed_stackup
        - name: prepreg_1
          kind: dielectric
          source: reviewed_stackup
        - name: In1.Cu
          kind: plane
          reference_net: GND
          source: reviewed_stackup
    routes:
      SIG:
        segments:
          - start: {{ x_mm: 1.0, y_mm: 1.0 }}
            end: {{ x_mm: 9.0, y_mm: 1.0 }}
            width_mm: 0.15
            layer: F.Cu
    zones:
      GND:{zone_polygon}
scenarios:
  - name: adjacent_plane_return_path
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

fn write_split_plane_project(parameters: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let project_path = dir.path().join("project.yaml");
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  name: split_plane_fixture
  version: 1
libraries: []
board:
  components: {{}}
  nets:
    SIG:
      kind: digital_or_analog
    GND:
      kind: ground
  layout:
    stackup:
      layers:
        - name: F.Cu
          kind: signal
          source: reviewed_stackup
        - name: prepreg_1
          kind: dielectric
          source: reviewed_stackup
        - name: In1.Cu
          kind: plane
          reference_net: GND
          source: reviewed_stackup
    routes:
      SIG:
        segments:
          - start: {{ x_mm: 1.0, y_mm: 1.0 }}
            end: {{ x_mm: 9.0, y_mm: 1.0 }}
            width_mm: 0.15
            layer: F.Cu
    zones:
      GND:
        - layer: In1.Cu
          island_id: gnd_left
          polygon:
            - {{ x_mm: 0.0, y_mm: 0.0 }}
            - {{ x_mm: 4.0, y_mm: 0.0 }}
            - {{ x_mm: 4.0, y_mm: 2.0 }}
            - {{ x_mm: 0.0, y_mm: 2.0 }}
        - layer: In1.Cu
          island_id: gnd_right
          polygon:
            - {{ x_mm: 6.0, y_mm: 0.0 }}
            - {{ x_mm: 10.0, y_mm: 0.0 }}
            - {{ x_mm: 10.0, y_mm: 2.0 }}
            - {{ x_mm: 6.0, y_mm: 2.0 }}
scenarios:
  - name: reference_plane_slot_crossing
    type: manufacturing
    checks:
      - REFERENCE_PLANE_SLOT_CROSSING_VALID
    parameters:
{parameters}"#
        ),
    )
    .unwrap();
    (dir, project_path)
}
