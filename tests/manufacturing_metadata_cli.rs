mod common;

use common::read_suggestion_report;
use serde_yaml_ng::Value;
use std::process::Command;

#[test]
fn set_manufacturing_metadata_makes_artifact_suggestions_runnable() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_metadata.project.yaml");
    let output = dir.path().join("with_metadata.project.yaml");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let mut project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string(
            "examples/scenario_suggestions_manufacturing_artifacts/project.yaml",
        )
        .unwrap(),
    )
    .unwrap();
    remove_board_manufacturing(&mut project_yaml);
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "set-manufacturing-metadata",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--stencil-thickness-mm",
            "0.10",
            "--min-drill-edge-clearance-mm",
            "0.50",
            "--min-slot-edge-clearance-mm",
            "0.50",
            "--min-paste-area-ratio",
            "0.70",
            "--max-paste-area-ratio",
            "1.00",
            "--min-solder-paste-spacing-mm",
            "0.15",
            "--source",
            "jlc_order_metadata",
        ])
        .output()
        .unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&command_output.stdout)
            .contains("CircuitCI applied 7 board manufacturing metadata fields")
    );

    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let manufacturing = enriched["board"]["manufacturing"].as_mapping().unwrap();
    assert_eq!(
        manufacturing[&Value::String("stencil_thickness_mm".to_string())],
        serde_yaml_ng::to_value(0.10).unwrap()
    );
    assert_eq!(
        manufacturing[&Value::String("min_paste_area_ratio".to_string())],
        serde_yaml_ng::to_value(0.70).unwrap()
    );
    assert_eq!(
        manufacturing[&Value::String("source".to_string())],
        Value::String("jlc_order_metadata".to_string())
    );
    assert!(enriched["libraries"][0].as_str().unwrap().starts_with('/'));

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
    assert_runnable(&suggestions, "drill_to_board_edge_clearance");
    assert_runnable(&suggestions, "slot_to_board_edge_clearance");
    assert_runnable(&suggestions, "solder_paste_opening_valid");
    assert_runnable(&suggestions, "solder_paste_aperture_area_ratio_valid");
    assert_runnable(&suggestions, "solder_paste_spacing_valid");
}

#[test]
fn set_manufacturing_metadata_rejects_noop_and_invalid_ratios() {
    let dir = tempfile::tempdir().unwrap();
    let input = "examples/scenario_suggestions_manufacturing_artifacts/project.yaml";
    let noop_output = dir.path().join("noop.project.yaml");
    let noop = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "set-manufacturing-metadata",
            input,
            "--output",
            noop_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!noop.status.success());
    assert!(
        String::from_utf8_lossy(&noop.stderr)
            .contains("At least one manufacturing metadata value must be supplied")
    );

    let invalid_output = dir.path().join("invalid.project.yaml");
    let invalid = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "set-manufacturing-metadata",
            input,
            "--output",
            invalid_output.to_str().unwrap(),
            "--min-paste-area-ratio",
            "0.90",
            "--max-paste-area-ratio",
            "0.80",
        ])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(
        String::from_utf8_lossy(&invalid.stderr)
            .contains("max_paste_area_ratio must be greater than or equal")
    );
}

#[test]
fn import_manufacturing_metadata_applies_csv_with_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_metadata.project.yaml");
    let metadata = dir.path().join("order_metadata.csv");
    let output = dir.path().join("with_imported_metadata.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let mut project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string(
            "examples/scenario_suggestions_manufacturing_artifacts/project.yaml",
        )
        .unwrap(),
    )
    .unwrap();
    remove_board_manufacturing(&mut project_yaml);
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,component,ambient_temperature_C,measurement_uncertainty_C,power_loss_w,measurement_point\n\
         stencil_thickness_mm,0.10,mm,JLC stencil order,foil thickness,,,,,,\n\
         min_drill_edge_clearance_mm,0.50,mm,JLC fab order,hole edge clearance,,,,,,\n\
         min_slot_edge_clearance_mm,0.50,mm,JLC fab order,slot edge clearance,,,,,,\n\
         min_paste_area_ratio,70,%,JLC stencil order,minimum aperture area ratio,,,,,,\n\
         max_paste_area_ratio,100,%,JLC stencil order,maximum aperture area ratio,,,,,,\n\
         min_solder_paste_spacing_mm,0.15,mm,JLC stencil order,min paste spacing,,,,,,\n\
         max_stitch_via_distance_mm,1.00,mm,Layout review,max stitching via distance,,,,,,\n\
         thermal_measurement,72.0,C,IR camera review,steady-state hotspot,u1_hotspot_steady_state,U1,45.0,2.5,1.2,package_top\n\
         unrelated_order_option,blue,,JLC order,kept as skipped evidence,,,,,,\n",
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
            "jlc_order_metadata",
            "--allow-unknown-fields",
        ])
        .output()
        .unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("8 applied fields"));
    assert!(stdout.contains("1 skipped rows"));
    assert!(stdout.contains("manifest"));

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let manufacturing = enriched["board"]["manufacturing"].as_mapping().unwrap();
    assert_eq!(
        manufacturing[&Value::String("stencil_thickness_mm".to_string())],
        serde_yaml_ng::to_value(0.10).unwrap()
    );
    assert_eq!(
        manufacturing[&Value::String("min_paste_area_ratio".to_string())],
        serde_yaml_ng::to_value(0.70).unwrap()
    );
    assert_eq!(
        manufacturing[&Value::String("max_paste_area_ratio".to_string())],
        serde_yaml_ng::to_value(1.0).unwrap()
    );
    assert_eq!(
        manufacturing[&Value::String("max_stitch_via_distance_mm".to_string())],
        serde_yaml_ng::to_value(1.0).unwrap()
    );
    let thermal_measurements = manufacturing[&Value::String("thermal_measurements".to_string())]
        .as_sequence()
        .unwrap();
    assert_eq!(thermal_measurements.len(), 1);
    assert_eq!(
        thermal_measurements[0]["name"],
        Value::String("u1_hotspot_steady_state".to_string())
    );
    assert_eq!(thermal_measurements[0]["measured_temperature_C"], 72.0);
    assert_eq!(thermal_measurements[0]["ambient_temperature_C"], 45.0);
    assert_eq!(thermal_measurements[0]["measurement_uncertainty_C"], 2.5);
    assert_eq!(thermal_measurements[0]["power_loss_w"], 1.2);
    assert_eq!(
        manufacturing[&Value::String("source".to_string())],
        Value::String("jlc_order_metadata".to_string())
    );
    assert!(enriched["libraries"][0].as_str().unwrap().starts_with('/'));

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
    assert_eq!(manifest["schema_version"], "0.6.0");
    assert_eq!(manifest["sources"]["metadata"]["data_rows"], 9);
    assert_eq!(manifest["import"]["applied_fields"], 8);
    assert_eq!(manifest["import"]["skipped_rows"], 1);
    assert_eq!(manifest["rows"][3]["normalized_value"], 0.70);
    assert_eq!(manifest["rows"][7]["board_field"], "thermal_measurements[]");
    assert_eq!(
        manifest["rows"][7]["normalized_value"]["measured_temperature_C"],
        72.0
    );
    assert_eq!(
        manifest["rows"][7]["normalized_value"]["measurement_uncertainty_C"],
        2.5
    );
    assert_eq!(manifest["rows"][8]["status"], "skipped_unknown_field");
    assert_eq!(
        manifest["rows"][8]["raw_columns"]["notes"],
        "kept as skipped evidence"
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
    assert_runnable(&suggestions, "drill_to_board_edge_clearance");
    assert_runnable(&suggestions, "slot_to_board_edge_clearance");
    assert_runnable(&suggestions, "solder_paste_opening_valid");
    assert_runnable(&suggestions, "solder_paste_aperture_area_ratio_valid");
    assert_runnable(&suggestions, "solder_paste_spacing_valid");
}

#[test]
fn import_manufacturing_metadata_applies_controlled_impedance_targets() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_impedance_targets.project.yaml");
    let metadata = dir.path().join("impedance_targets.csv");
    let output = dir.path().join("with_impedance_targets.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let mut project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    remove_board_manufacturing(&mut project_yaml);
    remove_layout_stackup(&mut project_yaml);
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    let csv_rows = [
        [
            "field",
            "value",
            "unit",
            "source",
            "notes",
            "net",
            "first_net",
            "second_net",
            "expected_width_mm",
            "expected_gap_mm",
            "max_width_error_mm",
            "max_gap_error_mm",
            "name",
            "reference_net",
            "thickness_mm",
            "copper_thickness_um",
            "dielectric_constant",
            "material",
        ],
        [
            "controlled_impedance_net",
            "50",
            "ohm",
            "fab stackup table",
            "reviewed RF target",
            "RF",
            "",
            "",
            "0.20",
            "",
            "0.03",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        ],
        [
            "controlled_impedance_pair",
            "90",
            "ohm",
            "fab stackup table",
            "reviewed USB target",
            "",
            "DP",
            "DM",
            "0.15",
            "0.20",
            "0.02",
            "0.03",
            "",
            "",
            "",
            "",
            "",
            "",
        ],
        [
            "stackup_layer",
            "signal",
            "",
            "fab stackup table",
            "top copper",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "F.Cu",
            "",
            "",
            "35.0",
            "",
            "",
        ],
        [
            "stackup_layer",
            "dielectric",
            "",
            "fab stackup table",
            "prepreg",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "prepreg_1",
            "",
            "0.18",
            "",
            "4.1",
            "FR-4 prepreg",
        ],
        [
            "stackup_layer",
            "plane",
            "",
            "fab stackup table",
            "reference plane",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "In1.GND",
            "GND",
            "",
            "17.5",
            "",
            "",
        ],
    ];
    std::fs::write(
        &metadata,
        format!(
            "{}\n",
            csv_rows
                .iter()
                .map(|row| row.join(","))
                .collect::<Vec<_>>()
                .join("\n")
        ),
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
    assert!(String::from_utf8_lossy(&command_output.stdout).contains("5 applied fields"));

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let controlled_impedance = &enriched["board"]["manufacturing"]["controlled_impedance"];
    assert_eq!(controlled_impedance["nets"][0]["net"], "RF");
    assert_eq!(
        controlled_impedance["nets"][0]["target_impedance_ohm"],
        50.0
    );
    assert_eq!(controlled_impedance["nets"][0]["expected_width_mm"], 0.20);
    assert_eq!(
        controlled_impedance["differential_pairs"][0]["first_net"],
        "DP"
    );
    assert_eq!(
        controlled_impedance["differential_pairs"][0]["second_net"],
        "DM"
    );
    assert_eq!(
        controlled_impedance["differential_pairs"][0]["target_differential_impedance_ohm"],
        90.0
    );
    let stackup_layers = enriched["board"]["layout"]["stackup"]["layers"]
        .as_sequence()
        .unwrap();
    assert_eq!(stackup_layers.len(), 3);
    assert_eq!(stackup_layers[0]["name"], "F.Cu");
    assert_eq!(stackup_layers[0]["kind"], "signal");
    assert_eq!(stackup_layers[0]["copper_thickness_um"], 35.0);
    assert_eq!(stackup_layers[1]["name"], "prepreg_1");
    assert_eq!(stackup_layers[1]["dielectric_constant"], 4.1);
    assert_eq!(stackup_layers[1]["material"], "FR-4 prepreg");
    assert_eq!(stackup_layers[2]["name"], "In1.GND");
    assert_eq!(stackup_layers[2]["reference_net"], "GND");

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
    assert_eq!(
        manifest["rows"][0]["board_field"],
        "controlled_impedance.nets[]"
    );
    assert_eq!(
        manifest["rows"][1]["board_field"],
        "controlled_impedance.differential_pairs[]"
    );
    assert_eq!(
        manifest["rows"][1]["normalized_value"]["target_differential_impedance_ohm"],
        90.0
    );
    assert_eq!(
        manifest["rows"][2]["board_field"],
        "layout.stackup.layers[]"
    );
    assert_eq!(manifest["rows"][2]["normalized_value"]["name"], "F.Cu");
    assert_eq!(
        manifest["rows"][4]["normalized_value"]["reference_net"],
        "GND"
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
    assert_runnable(&suggestions, "controlled_impedance_rf");
    assert_runnable(&suggestions, "controlled_impedance_dp_dm");
    assert_runnable(&suggestions, "controlled_impedance_stackup_rf");
    assert_runnable(&suggestions, "controlled_impedance_stackup_dp_dm");
}

#[test]
fn import_manufacturing_metadata_applies_rf_antenna_constraints() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_rf_constraints.project.yaml");
    let metadata = dir.path().join("rf_constraints.csv");
    let output = dir.path().join("with_rf_constraints.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let mut project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_rf_antenna_keepout/project.yaml")
            .unwrap(),
    )
    .unwrap();
    remove_rf_antenna_constraints(&mut project_yaml);
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,antenna_net,layer,polygon,feed_component,feed_pin,matching_components,max_matching_component_distance_mm\n\
         rf_antenna_keepout,1.0,mm,antenna_layout_guide_rev_a,reviewed antenna keepout,chip_antenna_clearance,ANT,F.Cu,0:0;10:0;10:10;0:10,,,,\n\
         rf_antenna_feed_path,10.0,mm,antenna_layout_guide_rev_a,reviewed feed path,chip_antenna_feed,ANT,,,ANT1,A,C1,2.0\n",
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
    assert!(String::from_utf8_lossy(&command_output.stdout).contains("2 applied fields"));

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let rf_antenna = &enriched["board"]["layout"]["constraints"]["rf_antenna"];
    let keepouts = rf_antenna["keepouts"].as_sequence().unwrap();
    assert_eq!(keepouts.len(), 1);
    assert_eq!(keepouts[0]["name"], "chip_antenna_clearance");
    assert_eq!(keepouts[0]["antenna_net"], "ANT");
    assert_eq!(keepouts[0]["layer"], "F.Cu");
    assert_eq!(keepouts[0]["min_copper_clearance_mm"], 1.0);
    assert_eq!(keepouts[0]["polygon"].as_sequence().unwrap().len(), 4);
    let feed_paths = rf_antenna["feed_paths"].as_sequence().unwrap();
    assert_eq!(feed_paths.len(), 1);
    assert_eq!(feed_paths[0]["name"], "chip_antenna_feed");
    assert_eq!(feed_paths[0]["feed_component"], "ANT1");
    assert_eq!(feed_paths[0]["feed_pin"], "A");
    assert_eq!(feed_paths[0]["matching_components"][0], "C1");
    assert_eq!(feed_paths[0]["max_feed_route_length_mm"], 10.0);
    assert_eq!(feed_paths[0]["max_matching_component_distance_mm"], 2.0);

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
    assert_eq!(manifest["schema_version"], "0.6.0");
    assert_eq!(
        manifest["rows"][0]["board_field"],
        "layout.constraints.rf_antenna.keepouts[]"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["polygon"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        manifest["rows"][1]["board_field"],
        "layout.constraints.rf_antenna.feed_paths[]"
    );
    assert_eq!(
        manifest["rows"][1]["normalized_value"]["max_matching_component_distance_mm"],
        2.0
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
    assert_runnable(&suggestions, "rf_antenna_keepout_chip_antenna_clearance");
    assert_runnable(&suggestions, "rf_antenna_feed_path_chip_antenna_feed");
}

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
    assert_eq!(manifest["schema_version"], "0.6.0");
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
fn import_manufacturing_metadata_rejects_unknown_fields_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let metadata = dir.path().join("order_metadata.csv");
    let output = dir.path().join("with_imported_metadata.project.yaml");
    std::fs::write(
        &metadata,
        "field,value,unit\nstencil_thickness_mm,0.10,mm\nunrelated_order_option,blue,\n",
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            "examples/scenario_suggestions_manufacturing_artifacts/project.yaml",
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!command_output.status.success());
    assert!(String::from_utf8_lossy(&command_output.stderr).contains("unsupported field"));
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
    assert_eq!(manifest["schema_version"], "0.6.0");
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

fn remove_board_manufacturing(project_yaml: &mut Value) {
    let board = project_yaml["board"].as_mapping_mut().unwrap();
    board.remove(Value::String("manufacturing".to_string()));
}

fn remove_layout_stackup(project_yaml: &mut Value) {
    let board = project_yaml["board"].as_mapping_mut().unwrap();
    let layout = board
        .get_mut(Value::String("layout".to_string()))
        .unwrap()
        .as_mapping_mut()
        .unwrap();
    layout.remove(Value::String("stackup".to_string()));
}

fn remove_rf_antenna_constraints(project_yaml: &mut Value) {
    let constraints = project_yaml["board"]["layout"]["constraints"]
        .as_mapping_mut()
        .unwrap();
    constraints.remove(Value::String("rf_antenna".to_string()));
}

fn assert_runnable(suggestions: &serde_json::Value, id: &str) {
    let suggestion = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == id)
        .unwrap_or_else(|| panic!("missing suggestion {id}"));
    assert_eq!(suggestion["runnable"], true, "{id}");
    assert!(suggestion.get("required_inputs").is_none(), "{id}");
}

fn assert_non_runnable(suggestions: &serde_json::Value, id: &str) {
    let suggestion = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == id)
        .unwrap_or_else(|| panic!("missing suggestion {id}"));
    assert_eq!(suggestion["runnable"], false, "{id}");
    assert!(suggestion.get("required_inputs").is_some(), "{id}");
}
