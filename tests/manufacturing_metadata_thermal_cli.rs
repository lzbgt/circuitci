mod common;

use common::read_suggestion_report;
use serde_yaml_ng::Value;
use std::process::Command;

#[test]
fn import_manufacturing_metadata_applies_thermal_copper_policy_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_thermal_policy.project.yaml");
    let metadata = dir.path().join("thermal_policy.csv");
    let output = dir.path().join("with_thermal_policy.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let mut project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_thermal_via_plating/project.yaml")
            .unwrap(),
    )
    .unwrap();
    remove_board_manufacturing(&mut project_yaml);
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,component,power_loss_w,min_plated_thermal_via_count,min_thermal_via_drill_mm,min_thermal_via_plating_thickness_um,min_total_thermal_via_barrel_cross_section_mm2,nets,layers\n\
         thermal_copper,20.0,mm2,thermal layout review,reviewed via barrel policy,u1_heat_spreader,U1,1.5,2,0.25,20.0,0.04,HOT,F.Cu|B.Cu\n",
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            input.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );
    assert!(String::from_utf8_lossy(&command_output.stdout).contains("1 applied fields"));

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let thermal_copper = enriched["board"]["manufacturing"]["thermal_copper"]
        .as_sequence()
        .unwrap();
    assert_eq!(thermal_copper.len(), 1);
    assert_eq!(
        thermal_copper[0]["name"],
        Value::String("u1_heat_spreader".to_string())
    );
    assert_eq!(thermal_copper[0]["min_copper_area_mm2"], 20.0);
    assert_eq!(thermal_copper[0]["min_plated_thermal_via_count"], 2);
    assert_eq!(
        thermal_copper[0]["min_total_thermal_via_barrel_cross_section_mm2"],
        0.04
    );
    assert_eq!(
        thermal_copper[0]["nets"][0],
        Value::String("HOT".to_string())
    );
    assert_eq!(
        thermal_copper[0]["layers"][1],
        Value::String("B.Cu".to_string())
    );

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    let manifest_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schemas/manufacturing_metadata_import.schema.json"
    ))
    .unwrap();
    let manifest_validator = jsonschema::validator_for(&manifest_schema).unwrap();
    if let Err(error) = manifest_validator.validate(&manifest) {
        panic!("Manufacturing metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(manifest["schema_version"], "0.31.0");
    assert_eq!(manifest["rows"][0]["board_field"], "thermal_copper[]");
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["min_thermal_via_plating_thickness_um"],
        20.0
    );

    let suggest_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            output.to_str().unwrap(),
            "--output",
            suggestions_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(suggest_status.success());
    let suggestions = read_suggestion_report(&suggestions_output);
    assert_runnable(&suggestions, "thermal_copper_area_u1_heat_spreader");
    assert_runnable(&suggestions, "thermal_via_plating_u1_heat_spreader");
    assert_runnable(
        &suggestions,
        "thermal_via_barrel_cross_section_u1_heat_spreader",
    );
}

#[test]
fn import_manufacturing_metadata_applies_thermal_package_rows() {
    let dir = tempfile::tempdir().unwrap();
    let model_dir = dir.path().join("models");
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::write(
        model_dir.join("thermal_package_import.model.yaml"),
        r#"component_id: test.thermal.package.import
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
model_quality:
  source: datasheet
  confidence: medium
  intended_use:
    - thermal_package_static_screening
"#,
    )
    .unwrap();
    let input = dir.path().join("without_package_metadata.project.yaml");
    let metadata = dir.path().join("thermal_package_metadata.csv");
    let output = dir.path().join("with_package_metadata.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    std::fs::write(
        &input,
        format!(
            r#"project:
  version: "1"
  name: thermal_package_metadata_import
libraries:
  - {model_dir}
board:
  nets:
    VIN: {{ kind: power }}
    VOUT: {{ kind: power }}
    GND: {{ kind: ground }}
  components:
    U1:
      model: test.thermal.package.import
      pins:
        IN: VIN
        OUT: VOUT
        GND: GND
  manufacturing: {{}}
"#,
            model_dir = model_dir.display()
        ),
    )
    .unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,component,power_loss_w,max_junction_temperature_C\n\
         thermal_copper,20.0,mm2,reviewed_loss_budget_rev_a,reviewed power loss,u1_regulator_loss,U1,1.4,\n\
         thermal_package,38.0,C/W,datasheet_package_table_rev_b,reviewed package thermal row,,U1,,125.0\n",
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            input.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--source",
            "thermal_review",
        ])
        .output()
        .unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let packages = enriched["board"]["manufacturing"]["thermal_packages"]
        .as_sequence()
        .unwrap();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["component"], "U1");
    assert_eq!(
        packages[0]["thermal_resistance_junction_to_ambient_C_per_W"],
        38.0
    );
    assert_eq!(packages[0]["max_junction_temperature_C"], 125.0);
    assert_eq!(packages[0]["source"], "datasheet_package_table_rev_b");

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    let manifest_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schemas/manufacturing_metadata_import.schema.json"
    ))
    .unwrap();
    let manifest_validator = jsonschema::validator_for(&manifest_schema).unwrap();
    if let Err(error) = manifest_validator.validate(&manifest) {
        panic!("Manufacturing metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(manifest["schema_version"], "0.31.0");
    assert_eq!(manifest["rows"][1]["board_field"], "thermal_packages[]");
    assert_eq!(
        manifest["rows"][1]["normalized_value"]["thermal_resistance_junction_to_ambient_C_per_W"],
        38.0
    );

    let suggest_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            output.to_str().unwrap(),
            "--output",
            suggestions_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(suggest_status.success());
    let suggestions = read_suggestion_report(&suggestions_output);
    assert_non_runnable(
        &suggestions,
        "thermal_package_temperature_u1_regulator_loss",
    );
}

#[test]
fn import_manufacturing_metadata_applies_thermal_environment_rows() {
    let dir = tempfile::tempdir().unwrap();
    let model_dir = dir.path().join("models");
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::write(
        model_dir.join("thermal_environment_import.model.yaml"),
        r#"component_id: test.thermal.environment.import
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
thermal_package:
  thermal_resistance_junction_to_ambient_C_per_W: 38.0
  max_junction_temperature_C: 125.0
  source: datasheet_package_table_rev_a
model_quality:
  source: datasheet
  confidence: medium
  intended_use:
    - thermal_package_static_screening
"#,
    )
    .unwrap();
    let input = dir.path().join("without_environment_metadata.project.yaml");
    let metadata = dir.path().join("thermal_environment_metadata.csv");
    let output = dir.path().join("with_environment_metadata.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    std::fs::write(
        &input,
        format!(
            r#"project:
  version: "1"
  name: thermal_environment_metadata_import
libraries:
  - {model_dir}
board:
  nets:
    VIN: {{ kind: power }}
    VOUT: {{ kind: power }}
    GND: {{ kind: ground }}
  components:
    U1:
      model: test.thermal.environment.import
      pins:
        IN: VIN
        OUT: VOUT
        GND: GND
  manufacturing: {{}}
"#,
            model_dir = model_dir.display()
        ),
    )
    .unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,component,power_loss_w,rated_ambient_temperature_C,min_airflow_lfm,enclosure_profile,airflow_lfm\n\
         thermal_copper,20.0,mm2,reviewed_loss_budget_rev_a,reviewed thermal policy,u1_regulator_loss,U1,1.4,60.0,200.0,vented_ip20,\n\
         thermal_environment,45.0,C,thermal_chamber_log_rev_b,reviewed operating environment,lab_ambient,,,,,vented_ip20,250.0\n",
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            input.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--source",
            "thermal_review",
        ])
        .output()
        .unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let environments = enriched["board"]["manufacturing"]["thermal_environments"]
        .as_sequence()
        .unwrap();
    assert_eq!(environments.len(), 1);
    assert_eq!(environments[0]["name"], "lab_ambient");
    assert_eq!(environments[0]["ambient_temperature_C"], 45.0);
    assert_eq!(environments[0]["airflow_lfm"], 250.0);
    assert_eq!(environments[0]["enclosure_profile"], "vented_ip20");

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    let manifest_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schemas/manufacturing_metadata_import.schema.json"
    ))
    .unwrap();
    let manifest_validator = jsonschema::validator_for(&manifest_schema).unwrap();
    if let Err(error) = manifest_validator.validate(&manifest) {
        panic!("Manufacturing metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(manifest["schema_version"], "0.31.0");
    assert_eq!(manifest["rows"][1]["board_field"], "thermal_environments[]");
    assert_eq!(
        manifest["rows"][1]["normalized_value"]["ambient_temperature_C"],
        45.0
    );

    let suggest_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            output.to_str().unwrap(),
            "--output",
            suggestions_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(suggest_status.success());
    let suggestions = read_suggestion_report(&suggestions_output);
    let derating = assert_runnable(
        &suggestions,
        "thermal_derating_environment_u1_regulator_loss_lab_ambient",
    );
    assert_eq!(
        derating["scenario"]["parameters"]["ambient_temperature_C"],
        45.0
    );
    assert_eq!(derating["scenario"]["parameters"]["airflow_lfm"], 250.0);
    assert_eq!(
        derating["scenario"]["parameters"]["enclosure_profile"],
        "vented_ip20"
    );
    let package = assert_non_runnable(
        &suggestions,
        "thermal_package_temperature_u1_regulator_loss_lab_ambient",
    );
    assert_eq!(
        package["scenario"]["parameters"]["ambient_temperature_C"],
        45.0
    );
    assert!(
        package["required_inputs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| {
                !item
                    .as_str()
                    .unwrap()
                    .contains("parameters.ambient_temperature_C")
            })
    );
}

#[test]
fn import_manufacturing_metadata_applies_thermal_limit_rows() {
    let dir = tempfile::tempdir().unwrap();
    let model_dir = dir.path().join("models");
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::write(
        model_dir.join("thermal_limit_import.model.yaml"),
        r#"component_id: test.thermal.limit.import
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
thermal_package:
  thermal_resistance_junction_to_ambient_C_per_W: 38.0
  max_junction_temperature_C: 125.0
  source: datasheet_package_table_rev_a
model_quality:
  source: datasheet
  confidence: medium
  intended_use:
    - thermal_package_static_screening
"#,
    )
    .unwrap();
    let input = dir.path().join("without_limit_metadata.project.yaml");
    let metadata = dir.path().join("thermal_limit_metadata.csv");
    let output = dir.path().join("with_limit_metadata.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    std::fs::write(
        &input,
        format!(
            r#"project:
  version: "1"
  name: thermal_limit_metadata_import
libraries:
  - {model_dir}
board:
  nets:
    VIN: {{ kind: power }}
    VOUT: {{ kind: power }}
    GND: {{ kind: ground }}
  components:
    U1:
      model: test.thermal.limit.import
      pins:
        IN: VIN
        OUT: VOUT
        GND: GND
  manufacturing: {{}}
"#,
            model_dir = model_dir.display()
        ),
    )
    .unwrap();
    let csv_rows = [
        [
            "field",
            "value",
            "unit",
            "source",
            "notes",
            "name",
            "component",
            "power_loss_w",
            "ambient_temperature_C",
            "airflow_lfm",
            "enclosure_profile",
            "max_temperature_rise_C",
            "max_junction_temperature_margin_C",
        ],
        [
            "thermal_copper",
            "20.0",
            "mm2",
            "reviewed_loss_budget_rev_a",
            "reviewed thermal policy",
            "u1_regulator_loss",
            "U1",
            "1.4",
            "",
            "",
            "",
            "",
            "",
        ],
        [
            "thermal_environment",
            "45.0",
            "C",
            "thermal_chamber_log_rev_b",
            "reviewed operating environment",
            "lab_ambient",
            "",
            "",
            "",
            "250.0",
            "vented_ip20",
            "",
            "",
        ],
        [
            "thermal_measurement",
            "72.0",
            "C",
            "ir_camera_review_rev_c",
            "steady-state hotspot",
            "u1_hotspot",
            "U1",
            "",
            "45.0",
            "",
            "",
            "",
            "",
        ],
        [
            "thermal_limit",
            "85.0",
            "C",
            "thermal_requirement_rev_d",
            "reviewed operating limit",
            "u1_lab_limit",
            "U1",
            "",
            "",
            "",
            "",
            "50.0",
            "5.0",
        ],
    ];
    std::fs::write(
        &metadata,
        csv_rows
            .iter()
            .map(|row| row.join(","))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            input.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--source",
            "thermal_review",
        ])
        .output()
        .unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let limits = enriched["board"]["manufacturing"]["thermal_limits"]
        .as_sequence()
        .unwrap();
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0]["name"], "u1_lab_limit");
    assert_eq!(limits[0]["max_measured_temperature_C"], 85.0);
    assert_eq!(limits[0]["max_temperature_rise_C"], 50.0);
    assert_eq!(limits[0]["max_junction_temperature_margin_C"], 5.0);

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    let manifest_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schemas/manufacturing_metadata_import.schema.json"
    ))
    .unwrap();
    let manifest_validator = jsonschema::validator_for(&manifest_schema).unwrap();
    if let Err(error) = manifest_validator.validate(&manifest) {
        panic!("Manufacturing metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(manifest["schema_version"], "0.31.0");
    assert_eq!(manifest["rows"][3]["board_field"], "thermal_limits[]");
    assert_eq!(
        manifest["rows"][3]["normalized_value"]["max_measured_temperature_C"],
        85.0
    );

    let suggest_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            output.to_str().unwrap(),
            "--output",
            suggestions_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(suggest_status.success());
    let suggestions = read_suggestion_report(&suggestions_output);
    let measured = assert_runnable(
        &suggestions,
        "thermal_measured_temperature_u1_hotspot_u1_lab_limit",
    );
    assert_eq!(
        measured["scenario"]["parameters"]["max_measured_temperature_C"],
        85.0
    );
    assert_eq!(
        measured["scenario"]["parameters"]["max_temperature_rise_C"],
        50.0
    );
    let package = assert_runnable(
        &suggestions,
        "thermal_package_temperature_u1_regulator_loss_lab_ambient_u1_lab_limit",
    );
    assert_eq!(
        package["scenario"]["parameters"]["ambient_temperature_C"],
        45.0
    );
    assert_eq!(
        package["scenario"]["parameters"]["max_temperature_rise_C"],
        50.0
    );
    assert_eq!(
        package["scenario"]["parameters"]["max_junction_temperature_margin_C"],
        5.0
    );
}

fn remove_board_manufacturing(project_yaml: &mut Value) {
    let board = project_yaml["board"].as_mapping_mut().unwrap();
    board.remove(Value::String("manufacturing".to_string()));
}

fn assert_runnable<'a>(suggestions: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    let suggestion = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == id)
        .unwrap_or_else(|| panic!("missing suggestion {id}"));
    assert_eq!(suggestion["runnable"], true, "{id}");
    assert!(suggestion.get("required_inputs").is_none(), "{id}");
    suggestion
}

fn assert_non_runnable<'a>(suggestions: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    let suggestion = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == id)
        .unwrap_or_else(|| panic!("missing suggestion {id}"));
    assert_eq!(suggestion["runnable"], false, "{id}");
    assert!(suggestion.get("required_inputs").is_some(), "{id}");
    suggestion
}
