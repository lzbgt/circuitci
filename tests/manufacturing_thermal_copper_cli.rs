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
fn thermal_via_plating_passes_for_reviewed_plated_vias() {
    let (_dir, project_path) = write_thermal_via_plating_project(2, 2, 0.30, true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_via_plating_fails_below_reviewed_plated_via_count() {
    let (_dir, project_path) = write_thermal_via_plating_project(2, 1, 0.30, true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_VIA_PLATING_VALID")
        .expect("thermal via plating failure");
    assert_eq!(failure["component"], "U1");
    assert_eq!(
        failure["measured"]["thermal_copper_name"],
        "u1_heat_spreader"
    );
    assert_eq!(failure["measured"]["route_via_count"], 2);
    assert_eq!(failure["measured"]["thermal_via_count"], 2);
    assert_eq!(failure["measured"]["matched_drill_count"], 2);
    assert_eq!(failure["measured"]["plated_thermal_via_count"], 1);
    assert_eq!(failure["measured"]["non_plated_or_unknown_drill_count"], 1);
    assert_eq!(
        failure["measured"]["observed_min_thermal_via_drill_mm"],
        0.30
    );
    assert_eq!(failure["limit"]["min_plated_thermal_via_count"], 2);
    assert_eq!(failure["limit"]["min_thermal_via_drill_mm"], 0.25);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_via_plating_fails_below_reviewed_drill_diameter() {
    let (_dir, project_path) = write_thermal_via_plating_project(2, 2, 0.20, true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_VIA_PLATING_VALID")
        .expect("thermal via drill failure");
    assert_eq!(failure["measured"]["plated_thermal_via_count"], 2);
    assert_eq!(
        failure["measured"]["observed_min_thermal_via_drill_mm"],
        0.20
    );
    assert_eq!(failure["limit"]["min_thermal_via_drill_mm"], 0.25);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_via_plating_fails_closed_without_drill_evidence() {
    let (_dir, project_path) = write_thermal_via_plating_project(2, 2, 0.30, false);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("requires board.layout.drills evidence")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_derating_environment_passes_for_reviewed_context() {
    let (_dir, project_path) =
        write_thermal_derating_project(Some(55.0), Some(250.0), Some("vented_ip20"), true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_derating_environment_fails_high_ambient() {
    let (_dir, project_path) =
        write_thermal_derating_project(Some(75.0), Some(250.0), Some("vented_ip20"), true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_DERATING_ENVIRONMENT_VALID")
        .expect("thermal derating ambient failure");
    assert_eq!(failure["component"], "U1");
    assert_eq!(
        failure["measured"]["thermal_copper_name"],
        "u1_heat_spreader"
    );
    assert_eq!(failure["measured"]["ambient_temperature_C"], 75.0);
    assert_eq!(failure["limit"]["rated_ambient_temperature_C"], 60.0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_derating_environment_fails_low_airflow_and_enclosure_mismatch() {
    let (_dir, project_path) =
        write_thermal_derating_project(Some(55.0), Some(120.0), Some("sealed_ip54"), true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    assert!(failures.iter().any(|failure| {
        failure["id"] == "THERMAL_DERATING_ENVIRONMENT_VALID"
            && failure["measured"]["airflow_lfm"] == 120.0
            && failure["limit"]["min_airflow_lfm"] == 200.0
    }));
    assert!(failures.iter().any(|failure| {
        failure["id"] == "THERMAL_DERATING_ENVIRONMENT_VALID"
            && failure["measured"]["enclosure_profile"] == "sealed_ip54"
            && failure["limit"]["required_enclosure_profile"] == "vented_ip20"
    }));
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_derating_environment_fails_closed_without_required_airflow() {
    let (_dir, project_path) =
        write_thermal_derating_project(Some(55.0), None, Some("vented_ip20"), true);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "VALIDATION_INPUT_MISSING")
        .expect("missing airflow failure");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("requires parameters.airflow_lfm")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_derating_environment_fails_closed_without_derating_metadata() {
    let (_dir, project_path) =
        write_thermal_derating_project(Some(55.0), Some(250.0), Some("vented_ip20"), false);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "VALIDATION_INPUT_MISSING")
        .expect("missing derating metadata failure");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("must declare rated_ambient_temperature_C")
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

#[test]
fn thermal_measured_temperature_passes_with_reviewed_measurement() {
    let (_dir, project_path) =
        write_thermal_measurement_project(72.0, Some(45.0), 85.0, Some(35.0));

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_measured_temperature_passes_with_reviewed_uncertainty_margin() {
    let (_dir, project_path) = write_thermal_measurement_project_with_uncertainty(
        82.0,
        Some(45.0),
        85.0,
        Some(40.0),
        Some(2.0),
        true,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_measured_temperature_fails_absolute_limit() {
    let (_dir, project_path) = write_thermal_measurement_project(92.0, Some(45.0), 85.0, None);

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_MEASURED_TEMPERATURE_VALID")
        .expect("thermal measured temperature failure");
    assert_eq!(failure["component"], "U1");
    assert_eq!(
        failure["measured"]["thermal_measurement_name"],
        "u1_hotspot_steady_state"
    );
    assert_eq!(
        failure["measured"]["thermal_measurement_source"],
        "ir_camera_steady_state_rev_a"
    );
    assert_eq!(failure["measured"]["measured_temperature_C"], 92.0);
    assert_eq!(failure["limit"]["max_measured_temperature_C"], 85.0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_measured_temperature_fails_absolute_limit_with_uncertainty() {
    let (_dir, project_path) = write_thermal_measurement_project_with_uncertainty(
        84.0,
        Some(45.0),
        85.0,
        None,
        Some(2.0),
        true,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_MEASURED_TEMPERATURE_VALID")
        .expect("thermal measured temperature uncertainty failure");
    assert_eq!(failure["measured"]["measured_temperature_C"], 84.0);
    assert_eq!(failure["measured"]["measurement_uncertainty_C"], 2.0);
    assert_eq!(
        failure["measured"]["worst_case_measured_temperature_C"],
        86.0
    );
    assert_eq!(failure["limit"]["max_measured_temperature_C"], 85.0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_measured_temperature_fails_rise_limit() {
    let (_dir, project_path) =
        write_thermal_measurement_project(82.0, Some(45.0), 90.0, Some(30.0));

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_MEASURED_TEMPERATURE_VALID")
        .expect("thermal measured rise failure");
    assert_eq!(failure["measured"]["ambient_temperature_C"], 45.0);
    assert_eq!(failure["measured"]["measured_temperature_rise_C"], 37.0);
    assert_eq!(failure["limit"]["max_temperature_rise_C"], 30.0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_measured_temperature_fails_rise_limit_with_uncertainty() {
    let (_dir, project_path) = write_thermal_measurement_project_with_uncertainty(
        78.0,
        Some(45.0),
        90.0,
        Some(35.0),
        Some(3.0),
        true,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "THERMAL_MEASURED_TEMPERATURE_VALID")
        .expect("thermal measured rise uncertainty failure");
    assert_eq!(failure["measured"]["measured_temperature_rise_C"], 33.0);
    assert_eq!(failure["measured"]["measurement_uncertainty_C"], 3.0);
    assert_eq!(
        failure["measured"]["worst_case_measured_temperature_rise_C"],
        36.0
    );
    assert_eq!(failure["limit"]["max_temperature_rise_C"], 35.0);
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_measured_temperature_fails_closed_without_ambient_for_rise() {
    let (_dir, project_path) = write_thermal_measurement_project(72.0, None, 90.0, Some(30.0));

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("must declare ambient_temperature_C")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn thermal_measured_temperature_fails_closed_without_requested_uncertainty() {
    let (_dir, project_path) = write_thermal_measurement_project_with_uncertainty(
        72.0,
        Some(45.0),
        90.0,
        None,
        None,
        true,
    );

    let report = run_validation(project_path.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("must declare measurement_uncertainty_C")
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

fn write_thermal_derating_project(
    ambient_temperature_c: Option<f64>,
    airflow_lfm: Option<f64>,
    enclosure_profile: Option<&str>,
    include_derating_metadata: bool,
) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let project_path = dir.path().join("project.yaml");
    let derating_metadata = if include_derating_metadata {
        r#"        rated_ambient_temperature_C: 60.0
        min_airflow_lfm: 200.0
        enclosure_profile: vented_ip20
"#
    } else {
        ""
    };
    let ambient = ambient_temperature_c
        .map(|value| format!("      ambient_temperature_C: {value}\n"))
        .unwrap_or_default();
    let airflow = airflow_lfm
        .map(|value| format!("      airflow_lfm: {value}\n"))
        .unwrap_or_default();
    let enclosure = enclosure_profile
        .map(|value| format!("      enclosure_profile: {value}\n"))
        .unwrap_or_default();
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  version: "1"
  name: thermal_derating_fixture
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
{derating_metadata}        source: thermal_derating_note_rev_a
scenarios:
  - name: thermal_derating_environment
    type: manufacturing
    checks:
      - THERMAL_DERATING_ENVIRONMENT_VALID
    parameters:
{ambient}{airflow}{enclosure}      thermal_copper:
        - name: u1_heat_spreader
"#,
            generic_library = repo.join("libs/generic").display(),
        ),
    )
    .unwrap();
    (dir, project_path)
}

fn write_thermal_measurement_project(
    measured_temperature_c: f64,
    ambient_temperature_c: Option<f64>,
    max_measured_temperature_c: f64,
    max_temperature_rise_c: Option<f64>,
) -> (tempfile::TempDir, std::path::PathBuf) {
    write_thermal_measurement_project_with_uncertainty(
        measured_temperature_c,
        ambient_temperature_c,
        max_measured_temperature_c,
        max_temperature_rise_c,
        None,
        false,
    )
}

fn write_thermal_measurement_project_with_uncertainty(
    measured_temperature_c: f64,
    ambient_temperature_c: Option<f64>,
    max_measured_temperature_c: f64,
    max_temperature_rise_c: Option<f64>,
    measurement_uncertainty_c: Option<f64>,
    include_measurement_uncertainty: bool,
) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let project_path = dir.path().join("project.yaml");
    let ambient = ambient_temperature_c
        .map(|value| format!("        ambient_temperature_C: {value}\n"))
        .unwrap_or_default();
    let max_rise = max_temperature_rise_c
        .map(|value| format!("      max_temperature_rise_C: {value}\n"))
        .unwrap_or_default();
    let uncertainty = measurement_uncertainty_c
        .map(|value| format!("        measurement_uncertainty_C: {value}\n"))
        .unwrap_or_default();
    let include_uncertainty = if include_measurement_uncertainty {
        "      include_measurement_uncertainty: true\n"
    } else {
        ""
    };
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  version: "1"
  name: thermal_measurement_fixture
libraries:
  - {generic_library}
board:
  nets:
    VIN: {{ kind: power }}
    OUT: {{ kind: digital_or_analog }}
    GND: {{ kind: ground }}
  components:
    U1:
      model: generic.analog.resistor
      pins:
        A: VIN
        B: OUT
  manufacturing:
    thermal_measurements:
      - name: u1_hotspot_steady_state
        component: U1
        source: ir_camera_steady_state_rev_a
        measured_temperature_C: {measured_temperature_c}
{ambient}        power_loss_w: 1.2
{uncertainty}        measurement_point: package_top
        notes: steady_state_after_20_min
scenarios:
  - name: thermal_measured_temperature
    type: manufacturing
    checks:
      - THERMAL_MEASURED_TEMPERATURE_VALID
    parameters:
      max_measured_temperature_C: {max_measured_temperature_c}
{max_rise}{include_uncertainty}      thermal_measurements:
        - name: u1_hotspot_steady_state
"#,
            generic_library = repo.join("libs/generic").display(),
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

fn write_thermal_via_plating_project(
    via_count: usize,
    plated_count: usize,
    drill_mm: f64,
    include_drills: bool,
) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let project_path = dir.path().join("project.yaml");
    let vias = (0..via_count)
        .map(|index| {
            format!(
                r#"          - at: {{ x_mm: {}, y_mm: 1.0 }}
            size_mm: 0.6
            drill_mm: {drill_mm}
            layers: [F.Cu, B.Cu]
"#,
                index + 1
            )
        })
        .collect::<String>();
    let drills = if include_drills {
        let rows = (0..via_count)
            .map(|index| {
                let plating = if index < plated_count {
                    "plated"
                } else {
                    "non_plated"
                };
                format!(
                    r#"        - at: {{ x_mm: {}, y_mm: 1.0 }}
          drill_mm: {drill_mm}
          plating: {plating}
          net: HOT
          via_index: {index}
"#,
                    index + 1
                )
            })
            .collect::<String>();
        format!("    drills:\n{rows}")
    } else {
        String::new()
    };
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  version: "1"
  name: thermal_via_plating_fixture
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
        min_plated_thermal_via_count: 2
        min_thermal_via_drill_mm: 0.25
        layers: [F.Cu, B.Cu]
        nets: [HOT]
        source: thermal_layout_note_rev_a
  layout:
    routes:
      HOT:
        vias:
{vias}{drills}scenarios:
  - name: thermal_via_plating
    type: manufacturing
    checks:
      - THERMAL_VIA_PLATING_VALID
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
