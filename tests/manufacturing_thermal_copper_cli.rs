mod common;

use common::{assert_report_schema_valid, run_validation};

#[test]
fn thermal_copper_area_passes_for_reviewed_copper_area() {
    let (_dir, project_path) = write_thermal_project(30.0, "HOT", "U1");

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_copper_area_fails_below_reviewed_minimum() {
    let (_dir, project_path) = write_thermal_project(12.0, "HOT", "U1");

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_COPPER_AREA_VALID")
        .expect("thermal copper failure");
    assert_eq!(failure["component"], "U1");
    assert_eq!(
        failure["measured"]["thermal_copper_name"],
        "u1_heat_spreader"
    );
    assert_eq!(
        failure["measured"]["thermal_copper_source"],
        "thermal_layout_note_rev_a"
    );
    assert_eq!(failure["measured"]["power_loss_w"], 1.5);
    assert_eq!(failure["measured"]["copper_region_area_mm2"], 12.0);
    assert_eq!(failure["measured"]["copper_area_mm2"], 12.0);
    assert_eq!(failure["limit"]["min_copper_area_mm2"], 20.0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_copper_area_fails_closed_without_comparable_copper() {
    let (_dir, project_path) = write_thermal_project(30.0, "GND", "J1");

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
fn thermal_via_stackup_passes_for_reviewed_via_and_copper_thickness_evidence() {
    let (_dir, project_path) = write_thermal_via_project(2, true, 35.0);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_via_stackup_fails_below_reviewed_via_count() {
    let (_dir, project_path) = write_thermal_via_project(1, true, 35.0);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_VIA_STACKUP_VALID")
        .expect("thermal via failure");
    assert_eq!(failure["component"], "U1");
    assert_eq!(
        failure["measured"]["thermal_copper_name"],
        "u1_heat_spreader"
    );
    assert_eq!(failure["measured"]["thermal_via_count"], 1);
    assert_eq!(failure["measured"]["route_via_count"], 1);
    assert_eq!(
        failure["measured"]["observed_min_copper_thickness_um"],
        35.0
    );
    assert_eq!(failure["limit"]["min_thermal_via_count"], 2);
    assert_eq!(failure["limit"]["min_copper_thickness_um"], 30.0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_via_stackup_fails_closed_without_stackup_copper_thickness() {
    let (_dir, project_path) = write_thermal_via_project(2, false, 35.0);

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
fn thermal_package_temperature_passes_with_reviewed_loss_and_model_metadata() {
    let (_dir, project_path) = write_thermal_package_project(1.0, 40.0, 125.0, true, 45.0, 60.0);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_package_temperature_fails_temperature_rise_limit() {
    let (_dir, project_path) = write_thermal_package_project(2.0, 40.0, 125.0, true, 25.0, 60.0);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_PACKAGE_TEMPERATURE_VALID")
        .expect("thermal package failure");
    assert_eq!(failure["component"], "U1");
    assert_eq!(
        failure["measured"]["thermal_copper_name"],
        "u1_package_loss"
    );
    assert_eq!(failure["measured"]["model"], "test.thermal.package");
    assert_eq!(
        failure["measured"]["thermal_package_source"],
        "datasheet_package_table_rev_a"
    );
    assert_eq!(failure["measured"]["power_loss_w"], 2.0);
    assert_eq!(
        failure["measured"]["thermal_resistance_junction_to_ambient_C_per_W"],
        40.0
    );
    assert_eq!(failure["measured"]["estimated_temperature_rise_C"], 80.0);
    assert_eq!(failure["limit"]["max_temperature_rise_C"], 60.0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_package_temperature_fails_junction_temperature_limit() {
    let (_dir, project_path) = write_thermal_package_project(2.0, 40.0, 125.0, true, 60.0, 100.0);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_PACKAGE_TEMPERATURE_VALID")
        .expect("thermal package failure");
    assert_eq!(failure["measured"]["estimated_temperature_rise_C"], 80.0);
    assert_eq!(
        failure["measured"]["estimated_junction_temperature_C"],
        140.0
    );
    assert_eq!(failure["limit"]["allowed_junction_temperature_C"], 125.0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_package_temperature_fails_closed_without_model_metadata() {
    let (_dir, project_path) = write_thermal_package_project(1.0, 40.0, 125.0, false, 45.0, 60.0);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("must declare thermal_package metadata")
    );
    assert_report_schema_valid(&report);
}

fn write_thermal_project(
    copper_area_mm2: f64,
    copper_net: &str,
    copper_component: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let project_path = dir.path().join("project.yaml");
    let height_mm = copper_area_mm2 / 5.0;
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  version: "1"
  name: thermal_copper_fixture
libraries:
  - {generic_library}
board:
  nets:
    HOT: {{ kind: power }}
    GND: {{ kind: ground }}
  components:
    U1:
      model: generic.analog.resistor
      pins:
        A: HOT
        B: GND
  manufacturing:
    thermal_copper:
      - name: u1_heat_spreader
        component: U1
        power_loss_w: 1.5
        min_copper_area_mm2: 20.0
        layers: [F.Cu]
        nets: [HOT]
        source: thermal_layout_note_rev_a
  layout:
    copper:
      regions:
        - points:
            - {{ x_mm: 0.0, y_mm: 0.0 }}
            - {{ x_mm: 5.0, y_mm: 0.0 }}
            - {{ x_mm: 5.0, y_mm: {height_mm} }}
            - {{ x_mm: 0.0, y_mm: {height_mm} }}
          layer: F.Cu
          polarity: dark
          net: {copper_net}
          component: {copper_component}
          source_primitive: gerber_region
          source_primitive_index: 0
scenarios:
  - name: thermal_copper_area
    type: manufacturing
    checks:
      - THERMAL_COPPER_AREA_VALID
    parameters:
      thermal_copper:
        - name: u1_heat_spreader
"#,
            generic_library = repo.join("libs/generic").display(),
        ),
    )
    .unwrap();
    (dir, project_path)
}

fn write_thermal_package_project(
    power_loss_w: f64,
    rja_c_per_w: f64,
    max_junction_c: f64,
    include_thermal_package: bool,
    ambient_temperature_c: f64,
    max_temperature_rise_c: f64,
) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let model_dir = dir.path().join("models");
    std::fs::create_dir_all(&model_dir).unwrap();
    let thermal_package = if include_thermal_package {
        format!(
            r#"thermal_package:
  thermal_resistance_junction_to_ambient_C_per_W: {rja_c_per_w}
  max_junction_temperature_C: {max_junction_c}
  source: datasheet_package_table_rev_a
"#
        )
    } else {
        String::new()
    };
    std::fs::write(
        model_dir.join("test_thermal_package.model.yaml"),
        format!(
            r#"component_id: test.thermal.package
version: 0.1.0
category: thermal_test_ic
ports:
  IN:
    kind: electrical_power
    required: true
  OUT:
    kind: electrical_power
    required: true
  GND:
    kind: electrical_ground
    required: true
{thermal_package}model_quality:
  source: datasheet
  confidence: medium
  intended_use:
    - thermal_package_static_screening
  not_valid_for:
    - transient_thermal_solver
"#,
        ),
    )
    .unwrap();
    let project_path = dir.path().join("project.yaml");
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  version: "1"
  name: thermal_package_fixture
libraries:
  - {model_library}
board:
  nets:
    VIN: {{ kind: power }}
    VOUT: {{ kind: power }}
    GND: {{ kind: ground }}
  components:
    U1:
      model: test.thermal.package
      pins:
        IN: VIN
        OUT: VOUT
        GND: GND
  manufacturing:
    thermal_copper:
      - name: u1_package_loss
        component: U1
        power_loss_w: {power_loss_w}
        min_copper_area_mm2: 20.0
        source: reviewed_loss_budget_rev_a
scenarios:
  - name: thermal_package_temperature
    type: manufacturing
    checks:
      - THERMAL_PACKAGE_TEMPERATURE_VALID
    parameters:
      ambient_temperature_C: {ambient_temperature_c}
      max_temperature_rise_C: {max_temperature_rise_c}
      thermal_copper:
        - name: u1_package_loss
"#,
            model_library = model_dir.display(),
        ),
    )
    .unwrap();
    (dir, project_path)
}

fn write_thermal_via_project(
    via_count: usize,
    include_f_cu_thickness: bool,
    b_cu_thickness_um: f64,
) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let project_path = dir.path().join("project.yaml");
    let f_cu_thickness = if include_f_cu_thickness {
        "          copper_thickness_um: 35.0\n"
    } else {
        ""
    };
    let vias = (0..via_count)
        .map(|index| {
            format!(
                r#"          - at: {{ x_mm: {}, y_mm: 1.0 }}
            size_mm: 0.6
            drill_mm: 0.3
            layers: [F.Cu, B.Cu]
"#,
                index + 1
            )
        })
        .collect::<String>();
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  version: "1"
  name: thermal_via_stackup_fixture
libraries:
  - {generic_library}
board:
  nets:
    HOT: {{ kind: power }}
    GND: {{ kind: ground }}
  components:
    U1:
      model: generic.analog.resistor
      pins:
        A: HOT
        B: GND
  manufacturing:
    thermal_copper:
      - name: u1_heat_spreader
        component: U1
        power_loss_w: 1.5
        min_copper_area_mm2: 20.0
        min_thermal_via_count: 2
        min_copper_thickness_um: 30.0
        layers: [F.Cu, B.Cu]
        nets: [HOT]
        source: thermal_layout_note_rev_a
  layout:
    stackup:
      layers:
        - name: F.Cu
          kind: signal
{f_cu_thickness}          source: reviewed_stackup_rev_a
        - name: core
          kind: dielectric
          source: reviewed_stackup_rev_a
        - name: B.Cu
          kind: signal
          copper_thickness_um: {b_cu_thickness_um}
          source: reviewed_stackup_rev_a
    routes:
      HOT:
        vias:
{vias}    copper:
      regions:
        - points:
            - {{ x_mm: 0.0, y_mm: 0.0 }}
            - {{ x_mm: 5.0, y_mm: 0.0 }}
            - {{ x_mm: 5.0, y_mm: 5.0 }}
            - {{ x_mm: 0.0, y_mm: 5.0 }}
          layer: F.Cu
          polarity: dark
          net: HOT
          component: U1
          source_primitive: gerber_region
          source_primitive_index: 0
scenarios:
  - name: thermal_via_stackup
    type: manufacturing
    checks:
      - THERMAL_VIA_STACKUP_VALID
    parameters:
      thermal_copper:
        - name: u1_heat_spreader
"#,
            generic_library = repo.join("libs/generic").display(),
        ),
    )
    .unwrap();
    (dir, project_path)
}
