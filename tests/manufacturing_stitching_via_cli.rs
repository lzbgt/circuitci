mod common;

use common::{assert_report_schema_valid, run_validation};

#[test]
fn return_path_stitching_via_passes_with_nearby_reference_via() {
    let (_dir, project_path) = write_stitching_project((5.4, 5.0), true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn return_path_stitching_via_fails_when_reference_via_is_too_far() {
    let (_dir, project_path) = write_stitching_project((8.0, 5.0), true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "RETURN_PATH_STITCHING_VIA_VALID");
    assert_eq!(failure["measured"]["net"], "SIG");
    assert_eq!(failure["measured"]["reference_net"], "GND");
    assert_eq!(failure["measured"]["signal_via_count"], 1);
    assert_eq!(failure["measured"]["reference_via_count"], 1);
    assert_eq!(failure["measured"]["signal_via_index"], 0);
    assert_eq!(failure["measured"]["nearest_reference_via_index"], 0);
    assert_eq!(failure["limit"]["max_stitch_via_distance_mm"], 1.0);
    let distance = failure["measured"]["nearest_reference_via_distance_mm"]
        .as_f64()
        .unwrap();
    assert!((distance - 3.0).abs() < 1.0e-12);
    assert_report_schema_valid(&report);
}

#[test]
fn return_path_stitching_via_fails_closed_without_explicit_via_layers() {
    let (_dir, project_path) = write_stitching_project((5.4, 5.0), false);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("must declare at least two explicit layers")
    );
    assert_report_schema_valid(&report);
}

fn write_stitching_project(
    reference_via_at: (f64, f64),
    include_layers: bool,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.yaml");
    let signal_layers = if include_layers {
        "            layers: [F.Cu, B.Cu]\n"
    } else {
        ""
    };
    let reference_layers = if include_layers {
        "            layers: [F.Cu, B.Cu]\n"
    } else {
        ""
    };
    std::fs::write(
        &path,
        format!(
            r#"project:
  name: return_path_stitching_via_fixture
  version: 0.1.0
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
        - name: core_1
          kind: dielectric
          source: reviewed_stackup
        - name: B.Cu
          kind: signal
          source: reviewed_stackup
    routes:
      SIG:
        vias:
          - at: {{ x_mm: 5.0, y_mm: 5.0 }}
            size_mm: 0.6
            drill_mm: 0.3
{signal_layers}      GND:
        vias:
          - at: {{ x_mm: {reference_x}, y_mm: {reference_y} }}
            size_mm: 0.6
            drill_mm: 0.3
{reference_layers}scenarios:
  - name: stitching_via
    type: manufacturing
    checks:
      - RETURN_PATH_STITCHING_VIA_VALID
    parameters:
      routes:
        - net: SIG
          reference_net: GND
          max_stitch_via_distance_mm: 1.0
"#,
            reference_x = reference_via_at.0,
            reference_y = reference_via_at.1,
        ),
    )
    .unwrap();
    (dir, path)
}
