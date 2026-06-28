mod common;

use common::{assert_report_schema_valid, run_validation};

#[test]
fn rf_antenna_keepout_passes_when_non_antenna_copper_is_clear() {
    let (_dir, project_path) = write_rf_keepout_project(12.0, true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn rf_antenna_keepout_fails_for_non_antenna_copper_intrusion() {
    let (_dir, project_path) = write_rf_keepout_project(10.4, true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "RF_ANTENNA_KEEPOUT_VALID");
    assert_eq!(
        failure["measured"]["keepout_name"],
        "chip_antenna_clearance"
    );
    assert_eq!(
        failure["measured"]["keepout_source"],
        "antenna_layout_guide_rev_a"
    );
    assert_eq!(failure["measured"]["keepout_layer"], "F.Cu");
    assert_eq!(failure["measured"]["antenna_net"], "ANT");
    assert_eq!(failure["measured"]["copper_kind"], "feature");
    assert_eq!(failure["measured"]["copper_index"], 1);
    assert_eq!(failure["measured"]["copper_net"], "GND");
    assert!(failure["measured"]["clearance_mm"].as_f64().unwrap() < 1.0);
    assert_eq!(failure["limit"]["min_copper_clearance_mm"], 1.0);
    assert_report_schema_valid(&report);
}

#[test]
fn rf_antenna_keepout_fails_closed_without_comparable_copper() {
    let (_dir, project_path) = write_rf_keepout_project(12.0, false);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("has no comparable board.layout.copper evidence")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn rf_antenna_feed_path_passes_with_short_route_and_near_matching_component() {
    let (_dir, project_path) = write_rf_feed_path_project(8.0, 1.2, true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn rf_antenna_feed_path_fails_when_feed_route_is_too_long() {
    let (_dir, project_path) = write_rf_feed_path_project(12.0, 1.2, true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "RF_ANTENNA_FEED_PATH_VALID")
        .expect("feed path failure");
    assert_eq!(failure["measured"]["feed_path_name"], "chip_antenna_feed");
    assert_eq!(
        failure["measured"]["feed_path_source"],
        "antenna_layout_guide_rev_a"
    );
    assert_eq!(failure["measured"]["antenna_net"], "ANT");
    assert_eq!(failure["measured"]["feed_component"], "ANT1");
    assert_eq!(failure["measured"]["feed_pin"], "A");
    assert_eq!(failure["measured"]["feed_route_length_mm"], 12.0);
    assert_eq!(failure["limit"]["max_feed_route_length_mm"], 10.0);
    assert_report_schema_valid(&report);
}

#[test]
fn rf_antenna_feed_path_fails_when_matching_component_is_too_far() {
    let (_dir, project_path) = write_rf_feed_path_project(8.0, 4.0, true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "RF_ANTENNA_FEED_PATH_VALID")
        .expect("feed path failure");
    assert_eq!(failure["measured"]["matching_component"], "C1");
    assert_eq!(failure["measured"]["matching_component_distance_mm"], 4.0);
    assert_eq!(failure["limit"]["max_matching_component_distance_mm"], 2.0);
    assert_report_schema_valid(&report);
}

#[test]
fn rf_antenna_feed_path_fails_closed_without_matching_pad_evidence() {
    let (_dir, project_path) = write_rf_feed_path_project(8.0, 1.2, false);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("matching component C1 lacks placement and antenna-net pad evidence")
            || failure["message"]
                .as_str()
                .unwrap()
                .contains("matching component C1 has no explicit pin")
    );
    assert_report_schema_valid(&report);
}

fn write_rf_keepout_project(
    non_antenna_x_mm: f64,
    include_non_antenna_copper: bool,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let project_path = dir.path().join("project.yaml");
    let non_antenna_feature = if include_non_antenna_copper {
        format!(
            r#"        - at: {{ x_mm: {non_antenna_x_mm}, y_mm: 5.0 }}
          layer: F.Cu
          polarity: dark
          net: GND
          source_primitive: gerber_flash
          source_primitive_index: 1
          aperture: D11
          shape: circle
          size: {{ x_mm: 0.8, y_mm: 0.8 }}
"#
        )
    } else {
        String::new()
    };
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  name: rf_antenna_keepout_fixture
  version: 1
libraries: []
board:
  components: {{}}
  nets:
    ANT:
      kind: digital_or_analog
    GND:
      kind: ground
  layout:
    constraints:
      rf_antenna:
        keepouts:
          - name: chip_antenna_clearance
            antenna_net: ANT
            layer: F.Cu
            source: antenna_layout_guide_rev_a
            min_copper_clearance_mm: 1.0
            polygon:
              - {{ x_mm: 0.0, y_mm: 0.0 }}
              - {{ x_mm: 10.0, y_mm: 0.0 }}
              - {{ x_mm: 10.0, y_mm: 10.0 }}
              - {{ x_mm: 0.0, y_mm: 10.0 }}
    copper:
      features:
        - at: {{ x_mm: 5.0, y_mm: 5.0 }}
          layer: F.Cu
          polarity: dark
          net: ANT
          source_primitive: gerber_flash
          source_primitive_index: 0
          aperture: D10
          shape: circle
          size: {{ x_mm: 1.0, y_mm: 1.0 }}
{non_antenna_feature}scenarios:
  - name: rf_antenna_keepout
    type: manufacturing
    checks:
      - RF_ANTENNA_KEEPOUT_VALID
    parameters:
      keepouts:
        - name: chip_antenna_clearance
"#
        ),
    )
    .unwrap();
    (dir, project_path)
}

fn write_rf_feed_path_project(
    route_end_x_mm: f64,
    matching_component_x_mm: f64,
    include_matching_antenna_pad: bool,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let project_path = dir.path().join("project.yaml");
    let repo = std::env::current_dir().unwrap();
    let matching_pin_net = if include_matching_antenna_pad {
        "ANT"
    } else {
        "GND"
    };
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  name: rf_antenna_feed_path_fixture
  version: 1
libraries:
  - {generic_library}
board:
  components:
    ANT1:
      model: generic.analog.resistor
      pins:
        A: ANT
        B: GND
    C1:
      model: generic.analog.capacitor
      pins:
        A: {matching_pin_net}
        B: GND
  nets:
    ANT:
      kind: digital_or_analog
    GND:
      kind: ground
  layout:
    constraints:
      rf_antenna:
        feed_paths:
          - name: chip_antenna_feed
            antenna_net: ANT
            feed_component: ANT1
            feed_pin: A
            matching_components: [C1]
            max_feed_route_length_mm: 10.0
            max_matching_component_distance_mm: 2.0
            source: antenna_layout_guide_rev_a
    placements:
      ANT1: {{ x_mm: 0.0, y_mm: 0.0, side: top, rotation_deg: 0.0 }}
      C1: {{ x_mm: {matching_component_x_mm}, y_mm: 0.0, side: top, rotation_deg: 0.0 }}
    pads:
      ANT1:
        A:
          at: {{ x_mm: 0.0, y_mm: 0.0 }}
          net: ANT
          layers: [F.Cu]
          kind: smd
          shape: rect
          size: {{ x_mm: 1.0, y_mm: 0.5 }}
        B:
          at: {{ x_mm: -1.0, y_mm: 0.0 }}
          net: GND
          layers: [F.Cu]
          kind: smd
          shape: rect
          size: {{ x_mm: 1.0, y_mm: 0.5 }}
      C1:
        A:
          at: {{ x_mm: {matching_component_x_mm}, y_mm: 0.0 }}
          net: {matching_pin_net}
          layers: [F.Cu]
          kind: smd
          shape: rect
          size: {{ x_mm: 0.6, y_mm: 0.4 }}
        B:
          at: {{ x_mm: {matching_component_x_mm}, y_mm: 0.8 }}
          net: GND
          layers: [F.Cu]
          kind: smd
          shape: rect
          size: {{ x_mm: 0.6, y_mm: 0.4 }}
    routes:
      ANT:
        segments:
          - start: {{ x_mm: 0.0, y_mm: 0.0 }}
            end: {{ x_mm: {route_end_x_mm}, y_mm: 0.0 }}
            width_mm: 0.25
            layer: F.Cu
scenarios:
  - name: rf_antenna_feed_path
    type: manufacturing
    checks:
      - RF_ANTENNA_FEED_PATH_VALID
    parameters:
      feed_paths:
        - name: chip_antenna_feed
"#,
            generic_library = repo.join("libs/generic").display()
        ),
    )
    .unwrap();
    (dir, project_path)
}
