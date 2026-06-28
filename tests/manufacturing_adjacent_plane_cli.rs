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

fn write_adjacent_plane_project(
    parameters: &str,
    covered_zone: bool,
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
      - ADJACENT_PLANE_RETURN_PATH_VALID
    parameters:
{parameters}"#
        ),
    )
    .unwrap();
    (dir, project_path)
}
