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
    assert_eq!(manifest["schema_version"], "0.32.0");
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
            "solder_mask_state",
            "solder_mask_layer",
            "solder_mask_source",
            "coupon_type",
            "target_impedance_ohm",
            "max_impedance_error_ohm",
            "min_batch_sample_count",
            "max_batch_mean_impedance_error_ohm",
            "max_batch_sample_impedance_error_ohm",
            "max_batch_stddev_ohm",
            "coupon_name",
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
            "covered",
            "F.Mask",
            "fab solder mask review",
            "",
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
            "covered",
            "F.Mask",
            "fab solder mask review",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        ],
        [
            "controlled_impedance_coupon",
            "51.2",
            "ohm",
            "fab coupon report",
            "reviewed RF coupon",
            "RF",
            "",
            "",
            "",
            "",
            "",
            "",
            "rf_coupon",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "single_ended",
            "50.0",
            "3.0",
            "3",
            "1.5",
            "2.0",
            "0.5",
            "",
        ],
        [
            "controlled_impedance_coupon_sample",
            "50.8",
            "ohm",
            "fab coupon report",
            "RF coupon sample 1",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "rf_coupon_s1",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "rf_coupon",
        ],
        [
            "controlled_impedance_coupon_sample",
            "51.0",
            "ohm",
            "fab coupon report",
            "RF coupon sample 2",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "rf_coupon_s2",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "rf_coupon",
        ],
        [
            "controlled_impedance_coupon_sample",
            "51.2",
            "ohm",
            "fab coupon report",
            "RF coupon sample 3",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "rf_coupon_s3",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "rf_coupon",
        ],
        [
            "controlled_impedance_coupon",
            "96.0",
            "ohm",
            "fab coupon report",
            "reviewed USB coupon",
            "",
            "DP",
            "DM",
            "",
            "",
            "",
            "",
            "dp_dm_coupon",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "differential",
            "90.0",
            "5.0",
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
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
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
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
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
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
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
    assert!(String::from_utf8_lossy(&command_output.stdout).contains("10 applied fields"));

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
        controlled_impedance["nets"][0]["solder_mask_state"],
        "covered"
    );
    assert_eq!(
        controlled_impedance["nets"][0]["solder_mask_layer"],
        "F.Mask"
    );
    assert_eq!(
        controlled_impedance["nets"][0]["solder_mask_source"],
        "fab solder mask review"
    );
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
    assert_eq!(
        controlled_impedance["differential_pairs"][0]["solder_mask_state"],
        "covered"
    );
    assert_eq!(
        controlled_impedance["differential_pairs"][0]["solder_mask_layer"],
        "F.Mask"
    );
    assert_eq!(controlled_impedance["coupons"][0]["name"], "rf_coupon");
    assert_eq!(
        controlled_impedance["coupons"][0]["coupon_type"],
        "single_ended"
    );
    assert_eq!(controlled_impedance["coupons"][0]["net"], "RF");
    assert_eq!(
        controlled_impedance["coupons"][0]["measured_impedance_ohm"],
        51.2
    );
    assert_eq!(
        controlled_impedance["coupons"][0]["min_batch_sample_count"],
        3
    );
    assert_eq!(
        controlled_impedance["coupons"][0]["max_batch_mean_impedance_error_ohm"],
        1.5
    );
    let samples = controlled_impedance["coupons"][0]["samples"]
        .as_sequence()
        .unwrap();
    assert_eq!(samples.len(), 3);
    assert_eq!(samples[0]["name"], "rf_coupon_s1");
    assert_eq!(samples[0]["measured_impedance_ohm"], 50.8);
    assert_eq!(controlled_impedance["coupons"][1]["name"], "dp_dm_coupon");
    assert_eq!(
        controlled_impedance["coupons"][1]["coupon_type"],
        "differential"
    );
    assert_eq!(controlled_impedance["coupons"][1]["first_net"], "DP");
    assert_eq!(controlled_impedance["coupons"][1]["second_net"], "DM");
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
        manifest["rows"][1]["normalized_value"]["solder_mask_state"],
        "covered"
    );
    assert_eq!(
        manifest["rows"][2]["board_field"],
        "controlled_impedance.coupons[]"
    );
    assert_eq!(
        manifest["rows"][2]["normalized_value"]["measured_impedance_ohm"],
        51.2
    );
    assert_eq!(
        manifest["rows"][3]["board_field"],
        "controlled_impedance.coupons[].samples[]"
    );
    assert_eq!(
        manifest["rows"][3]["normalized_value"]["measured_impedance_ohm"],
        50.8
    );
    assert_eq!(
        manifest["rows"][5]["board_field"],
        "controlled_impedance.coupons[].samples[]"
    );
    assert_eq!(
        manifest["rows"][6]["board_field"],
        "controlled_impedance.coupons[]"
    );
    assert_eq!(
        manifest["rows"][6]["normalized_value"]["coupon_type"],
        "differential"
    );
    assert_eq!(
        manifest["rows"][7]["board_field"],
        "layout.stackup.layers[]"
    );
    assert_eq!(manifest["rows"][7]["normalized_value"]["name"], "F.Cu");
    assert_eq!(
        manifest["rows"][9]["normalized_value"]["reference_net"],
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
    assert_runnable(&suggestions, "controlled_impedance_solder_mask_rf");
    assert_runnable(&suggestions, "controlled_impedance_solder_mask_dp_dm");
    assert_runnable(&suggestions, "controlled_impedance_coupon_rf_coupon");
    assert_runnable(&suggestions, "controlled_impedance_coupon_dp_dm_coupon");
    assert_runnable(&suggestions, "controlled_impedance_coupon_batch_rf_coupon");
}

#[test]
fn import_manufacturing_metadata_applies_coupon_trace_correlation_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_coupon_trace.project.yaml");
    let metadata = dir.path().join("coupon_trace.csv");
    let output = dir.path().join("with_coupon_trace.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,coupon_type,net,target_impedance_ohm,max_impedance_error_ohm,process_lot,panel_id,stackup_revision,coupon_trace_layer,coupon_trace_width_mm,max_trace_width_delta_mm\n\
         controlled_impedance_coupon,51.2,ohm,fab coupon report,reviewed trace correlation,rf_coupon,single_ended,RF,50.0,3.0,lot_2026_06_b,panel_9,stackup_rev_b,F.Cu,0.20,0.03\n",
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

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let coupon = &enriched["board"]["manufacturing"]["controlled_impedance"]["coupons"][0];
    assert_eq!(coupon["name"], "rf_coupon");
    assert_eq!(coupon["process_lot"], "lot_2026_06_b");
    assert_eq!(coupon["panel_id"], "panel_9");
    assert_eq!(coupon["stackup_revision"], "stackup_rev_b");
    assert_eq!(coupon["coupon_trace_layer"], "F.Cu");
    assert_eq!(coupon["coupon_trace_width_mm"], 0.20);
    assert_eq!(coupon["max_trace_width_delta_mm"], 0.03);

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
    assert_eq!(manifest["schema_version"], "0.32.0");
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["process_lot"],
        "lot_2026_06_b"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["coupon_trace_width_mm"],
        0.20
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
    assert_runnable(
        &suggestions,
        "controlled_impedance_coupon_trace_correlation_rf_coupon",
    );
}

#[test]
fn import_manufacturing_metadata_applies_solver_result_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_solver_result.project.yaml");
    let metadata = dir.path().join("solver_result.csv");
    let output = dir.path().join("with_solver_result.project.yaml");
    let library_output = dir
        .path()
        .join("with_solver_result_and_material_library.project.yaml");
    let process_output = dir
        .path()
        .join("with_solver_result_and_material_process.project.yaml");
    let qualified_output = dir
        .path()
        .join("with_solver_result_and_qualification.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let library_manifest_output = library_output.with_extension("manufacturing.json");
    let process_manifest_output = process_output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,result_type,net,target_impedance_ohm,max_impedance_error_ohm,solver,solver_version,solver_artifact_uri,solver_artifact_sha256,solver_input_deck_uri,solver_input_deck_sha256,stackup_revision,route_layer,reference_layer,dielectric_layer,solved_width_mm,max_route_width_delta_mm,input_stackup_revision,input_route_layer,input_reference_layer,input_dielectric_layer,input_width_mm,frequency_mhz,input_frequency_mhz,min_solver_sample_count,max_solver_frequency_step_mhz,required_solver_corners,solver_result_name,corner,copper_roughness_model,copper_roughness_um,input_copper_roughness_model,input_copper_roughness_um,etch_compensation_model,etch_compensation_um,input_etch_compensation_model,input_etch_compensation_um,solver_artifact_signature_uri,solver_artifact_signature_sha256,solver_artifact_signer,solver_output_schema,solver_output_schema_version,solver_output_schema_uri,solver_output_schema_sha256,solver_material_library,solver_material_library_revision,solver_material_library_artifact_uri,solver_material_library_artifact_sha256,input_material_library,input_material_library_revision,stackup_signoff_source,fabricator_stackup_revision,stackup_signoff_artifact_uri,stackup_signoff_artifact_sha256\n\
         controlled_impedance_solver_result,50.6,ohm,solver report,reviewed solver evidence,rf_solver_result,single_ended,RF,50.0,2.0,reviewed_2d_field_solver,2026.07,artifacts/solver/rf_solver_result.json,0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef,artifacts/solver/rf_solver_input_deck.json,fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210,stackup_rev_b,F.Cu,In1.GND,prepreg_1,0.20,0.03,stackup_rev_b,F.Cu,In1.GND,prepreg_1,0.20,2400,2400,4,500,nominal;high_dk,,,huray,1.5,huray,1.5,fabricator_finished_width_bias,8.0,fabricator_finished_width_bias,8.0,artifacts/solver/rf_solver_result.sig,1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef,si_review_key_2026,circuitci_controlled_impedance_solver_result,1.0,artifacts/solver/controlled_impedance_solver_result_schema_v1.json,55556666777788889999aaaabbbbccccddddeeeeffff00001111222233334444,reviewed_stackup_materials,rev_b,artifacts/solver/material_library_rev_b.json,abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd,reviewed_stackup_materials,rev_b,fabricator_stackup_review_rev_b,stackup_rev_b,artifacts/fabricator/stackup_signoff_rev_b.pdf,111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000\n\
         controlled_impedance_solver_sample,50.6,ohm,solver report,nominal sample,rf_solver_nominal_2400,,,,,,,,,,,,,,,,,,,,,,2400,,,,,rf_solver_result,nominal,,,,,,,,,,,,,,,,,,,,,\n\
         controlled_impedance_solver_sample,50.7,ohm,solver report,nominal sample,rf_solver_nominal_2900,,,,,,,,,,,,,,,,,,,,,,2900,,,,,rf_solver_result,nominal,,,,,,,,,,,,,,,,,,,,,\n\
         controlled_impedance_solver_sample,49.5,ohm,solver report,high dk sample,rf_solver_high_dk_2400,,,,,,,,,,,,,,,,,,,,,,2400,,,,,rf_solver_result,high_dk,,,,,,,,,,,,,,,,,,,,,\n\
         controlled_impedance_solver_sample,49.6,ohm,solver report,high dk sample,rf_solver_high_dk_2900,,,,,,,,,,,,,,,,,,,,,,2900,,,,,rf_solver_result,high_dk,,,,,,,,,,,,,,,,,,,,,\n",
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

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let result = &enriched["board"]["manufacturing"]["controlled_impedance"]["solver_results"][0];
    assert_eq!(result["name"], "rf_solver_result");
    assert_eq!(result["source"], "solver report");
    assert_eq!(result["solver_version"], "2026.07");
    assert_eq!(
        result["solver_artifact_uri"],
        "artifacts/solver/rf_solver_result.json"
    );
    assert_eq!(
        result["solver_artifact_sha256"],
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_eq!(
        result["solver_artifact_signature_uri"],
        "artifacts/solver/rf_solver_result.sig"
    );
    assert_eq!(
        result["solver_artifact_signature_sha256"],
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
    );
    assert_eq!(result["solver_artifact_signer"], "si_review_key_2026");
    assert_eq!(
        result["solver_output_schema"],
        "circuitci_controlled_impedance_solver_result"
    );
    assert_eq!(result["solver_output_schema_version"], "1.0");
    assert_eq!(
        result["solver_output_schema_uri"],
        "artifacts/solver/controlled_impedance_solver_result_schema_v1.json"
    );
    assert_eq!(
        result["solver_output_schema_sha256"],
        "55556666777788889999aaaabbbbccccddddeeeeffff00001111222233334444"
    );
    assert_eq!(
        result["solver_input_deck_uri"],
        "artifacts/solver/rf_solver_input_deck.json"
    );
    assert_eq!(
        result["solver_input_deck_sha256"],
        "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
    );
    assert_eq!(result["solved_impedance_ohm"], 50.6);
    assert_eq!(result["stackup_revision"], "stackup_rev_b");
    assert_eq!(result["input_width_mm"], 0.20);
    assert_eq!(result["copper_roughness_model"], "huray");
    assert_eq!(result["copper_roughness_um"], 1.5);
    assert_eq!(result["input_copper_roughness_model"], "huray");
    assert_eq!(result["input_copper_roughness_um"], 1.5);
    assert_eq!(
        result["etch_compensation_model"],
        "fabricator_finished_width_bias"
    );
    assert_eq!(result["etch_compensation_um"], 8.0);
    assert_eq!(
        result["input_etch_compensation_model"],
        "fabricator_finished_width_bias"
    );
    assert_eq!(result["input_etch_compensation_um"], 8.0);
    assert_eq!(
        result["solver_material_library"],
        "reviewed_stackup_materials"
    );
    assert_eq!(result["solver_material_library_revision"], "rev_b");
    assert_eq!(
        result["solver_material_library_artifact_uri"],
        "artifacts/solver/material_library_rev_b.json"
    );
    assert_eq!(
        result["solver_material_library_artifact_sha256"],
        "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
    );
    assert_eq!(
        result["input_material_library"],
        "reviewed_stackup_materials"
    );
    assert_eq!(result["input_material_library_revision"], "rev_b");
    assert_eq!(
        result["stackup_signoff_source"],
        "fabricator_stackup_review_rev_b"
    );
    assert_eq!(result["fabricator_stackup_revision"], "stackup_rev_b");
    assert_eq!(
        result["stackup_signoff_artifact_uri"],
        "artifacts/fabricator/stackup_signoff_rev_b.pdf"
    );
    assert_eq!(
        result["stackup_signoff_artifact_sha256"],
        "111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000"
    );
    assert_eq!(result["min_solver_sample_count"], 4);
    assert_eq!(result["max_solver_frequency_step_mhz"], 500.0);
    assert_eq!(result["required_solver_corners"][0], "nominal");
    assert_eq!(result["samples"][0]["name"], "rf_solver_nominal_2400");
    assert_eq!(result["samples"][0]["corner"], "nominal");

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
    assert_eq!(manifest["schema_version"], "0.32.0");
    assert_eq!(
        manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_results[]"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver"],
        "reviewed_2d_field_solver"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_artifact_sha256"],
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_artifact_signature_sha256"],
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_artifact_signer"],
        "si_review_key_2026"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_output_schema_version"],
        "1.0"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_output_schema_sha256"],
        "55556666777788889999aaaabbbbccccddddeeeeffff00001111222233334444"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_input_deck_uri"],
        "artifacts/solver/rf_solver_input_deck.json"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_material_library_revision"],
        "rev_b"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["input_material_library"],
        "reviewed_stackup_materials"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["stackup_signoff_artifact_sha256"],
        "111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000"
    );
    assert_eq!(
        manifest["rows"][1]["board_field"],
        "controlled_impedance.solver_results[].samples[]"
    );
    assert_eq!(
        manifest["rows"][1]["normalized_value"]["solved_impedance_ohm"],
        50.6
    );

    std::fs::write(
        &metadata,
        "field,value,source,name,material_library,material_library_revision,artifact_uri,artifact_sha256,corners,dielectric_layers,materials,content_fields,fabricator_stackup_revision,acceptance_artifact_uri,acceptance_artifact_sha256,accepted_by,accepted_corners,accepted_dielectric_layers,accepted_materials\n\
         controlled_impedance_solver_material_library,reviewed,solver material library,reviewed_stackup_materials_rev_b,reviewed_stackup_materials,rev_b,artifacts/solver/material_library_rev_b.json,abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd,nominal;high_dk,prepreg_1,FR-4 prepreg,corner;dielectric_layer;material;dielectric_constant;nominal_dielectric_constant,,,,,,,\n\
         controlled_impedance_solver_material_acceptance,reviewed,fabricator material acceptance,reviewed_stackup_materials_rev_b_acceptance,reviewed_stackup_materials,rev_b,,,,,,,stackup_rev_b,artifacts/fabricator/material_acceptance_rev_b.pdf,22223333444455556666777788889999aaaabbbbccccddddeeeeffff00001111,fabricator_si_review,nominal;high_dk,prepreg_1,FR-4 prepreg\n",
    )
    .unwrap();
    let library_import = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            output.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            library_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        library_import.status.success(),
        "{}",
        String::from_utf8_lossy(&library_import.stderr)
    );
    common::assert_yaml_file_valid(&library_output, &validator);
    let enriched_with_library: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&library_output).unwrap()).unwrap();
    let library = enriched_with_library["board"]["manufacturing"]["controlled_impedance"]
        ["solver_material_libraries"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|library| library["name"] == "reviewed_stackup_materials_rev_b")
        .unwrap();
    assert_eq!(library["name"], "reviewed_stackup_materials_rev_b");
    assert_eq!(library["material_library"], "reviewed_stackup_materials");
    assert_eq!(library["material_library_revision"], "rev_b");
    assert_eq!(library["corners"][1], "high_dk");
    assert_eq!(library["content_fields"][3], "dielectric_constant");
    let acceptance = enriched_with_library["board"]["manufacturing"]["controlled_impedance"]
        ["solver_material_acceptances"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|acceptance| acceptance["name"] == "reviewed_stackup_materials_rev_b_acceptance")
        .unwrap();
    assert_eq!(
        acceptance["acceptance_artifact_uri"],
        "artifacts/fabricator/material_acceptance_rev_b.pdf"
    );
    assert_eq!(acceptance["accepted_corners"][1], "high_dk");
    let library_manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(library_manifest_output).unwrap()).unwrap();
    if let Err(error) = manifest_validator.validate(&library_manifest) {
        panic!("Material-library metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(
        library_manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_material_libraries[]"
    );
    assert_eq!(
        library_manifest["rows"][0]["normalized_value"]["artifact_sha256"],
        "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
    );
    assert_eq!(
        library_manifest["rows"][0]["normalized_value"]["content_fields"][3],
        "dielectric_constant"
    );
    assert_eq!(
        library_manifest["rows"][1]["board_field"],
        "controlled_impedance.solver_material_acceptances[]"
    );
    assert_eq!(
        library_manifest["rows"][1]["normalized_value"]["acceptance_artifact_sha256"],
        "22223333444455556666777788889999aaaabbbbccccddddeeeeffff00001111"
    );

    std::fs::write(
        &metadata,
        "field,value,source,name,material_library,material_library_revision,fabricator_stackup_revision,dielectric_layer,material,process_lot,material_lot,process_revision,drift_artifact_uri,drift_artifact_sha256,accepted_dielectric_constant,measured_dielectric_constant,max_dielectric_constant_delta,accepted_thickness_mm,measured_thickness_mm,max_thickness_delta_mm\n\
         controlled_impedance_solver_material_process,reviewed,fabricator material lot drift,reviewed_stackup_materials_rev_b_lot_a,reviewed_stackup_materials,rev_b,stackup_rev_b,prepreg_1,FR-4 prepreg,lot_2026_06_b,fr4_prepreg_lot_8,lamination_rev_d,artifacts/fabricator/material_lot_drift_rev_b.pdf,3333444455556666777788889999aaaabbbbccccddddeeeeffff000011112222,4.1,4.12,0.05,0.18,0.181,0.005\n",
    )
    .unwrap();
    let process_import = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            library_output.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            process_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        process_import.status.success(),
        "{}",
        String::from_utf8_lossy(&process_import.stderr)
    );
    common::assert_yaml_file_valid(&process_output, &validator);
    let enriched_with_process: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&process_output).unwrap()).unwrap();
    let process = enriched_with_process["board"]["manufacturing"]["controlled_impedance"]
        ["solver_material_processes"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|process| process["name"] == "reviewed_stackup_materials_rev_b_lot_a")
        .unwrap();
    assert_eq!(process["process_lot"], "lot_2026_06_b");
    assert_eq!(process["measured_dielectric_constant"], 4.12);
    let process_manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(process_manifest_output).unwrap()).unwrap();
    if let Err(error) = manifest_validator.validate(&process_manifest) {
        panic!("Material-process metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(
        process_manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_material_processes[]"
    );
    assert_eq!(
        process_manifest["rows"][0]["normalized_value"]["drift_artifact_sha256"],
        "3333444455556666777788889999aaaabbbbccccddddeeeeffff000011112222"
    );

    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,solver,solver_version,qualification_artifact_uri,qualification_artifact_sha256\n\
         controlled_impedance_solver_qualification,qualified,,si tool qualification,reviewed tool/version qualification,reviewed_2d_field_solver_2026_07,reviewed_2d_field_solver,2026.07,artifacts/solver/reviewed_2d_field_solver_2026_07_qualification.pdf,11223344556677889900aabbccddeeff11223344556677889900aabbccddeeff\n",
    )
    .unwrap();
    let qualification_import = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            process_output.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            qualified_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        qualification_import.status.success(),
        "{}",
        String::from_utf8_lossy(&qualification_import.stderr)
    );

    let suggest_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            qualified_output.to_str().unwrap(),
            "--output",
            suggestions_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(suggest_status.success());
    let suggestions = read_suggestion_report(&suggestions_output);
    assert_runnable(
        &suggestions,
        "controlled_impedance_solver_result_rf_solver_result",
    );
}

#[test]
fn import_manufacturing_metadata_applies_solver_material_corner_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir
        .path()
        .join("without_solver_material_corners.project.yaml");
    let metadata = dir.path().join("solver_material_corners.csv");
    let output = dir.path().join("with_solver_material_corners.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,solver_result_name,name,corner,dielectric_layer,material,nominal_dielectric_constant,material_library,material_library_revision\n\
         controlled_impedance_solver_material_corner,4.1,,solver material library,nominal material corner,rf_solver_result,rf_solver_nominal_material,nominal,prepreg_1,FR-4 prepreg,4.1,reviewed_stackup_materials,rev_a\n\
         controlled_impedance_solver_material_corner,4.4,,solver material library,high-dk material corner,rf_solver_result,rf_solver_high_dk_material,high_dk,prepreg_1,FR-4 prepreg,4.1,reviewed_stackup_materials,rev_a\n",
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

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let result = &enriched["board"]["manufacturing"]["controlled_impedance"]["solver_results"][0];
    assert_eq!(
        result["material_corners"][0]["name"],
        "rf_solver_nominal_material"
    );
    assert_eq!(result["material_corners"][0]["corner"], "nominal");
    assert_eq!(result["material_corners"][0]["dielectric_constant"], 4.1);
    assert_eq!(
        result["material_corners"][0]["nominal_dielectric_constant"],
        4.1
    );
    assert_eq!(result["material_corners"][1]["corner"], "high_dk");
    assert_eq!(result["material_corners"][1]["dielectric_constant"], 4.4);

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
    assert_eq!(manifest["schema_version"], "0.32.0");
    assert_eq!(
        manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_results[].material_corners[]"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["material_library_revision"],
        "rev_a"
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
    assert_runnable(
        &suggestions,
        "controlled_impedance_solver_result_rf_solver_result",
    );
}

#[test]
fn import_manufacturing_metadata_applies_solver_qualification_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_solver_qualification.project.yaml");
    let metadata = dir.path().join("solver_qualification.csv");
    let output = dir.path().join("with_solver_qualification.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,solver,solver_version,qualification_artifact_uri,qualification_artifact_sha256\n\
         controlled_impedance_solver_qualification,qualified,,si tool qualification,reviewed tool/version qualification,reviewed_2d_field_solver_2026_06,reviewed_2d_field_solver,2026.06,artifacts/solver/reviewed_2d_field_solver_2026_06_qualification.pdf,00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff\n",
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

    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let qualification =
        &enriched["board"]["manufacturing"]["controlled_impedance"]["solver_qualifications"][0];
    assert_eq!(qualification["name"], "reviewed_2d_field_solver_2026_06");
    assert_eq!(qualification["source"], "si tool qualification");
    assert_eq!(qualification["solver"], "reviewed_2d_field_solver");
    assert_eq!(qualification["solver_version"], "2026.06");
    assert_eq!(
        qualification["qualification_artifact_sha256"],
        "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
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
    assert_eq!(manifest["schema_version"], "0.32.0");
    assert_eq!(
        manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_qualifications[]"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["qualification_artifact_uri"],
        "artifacts/solver/reviewed_2d_field_solver_2026_06_qualification.pdf"
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
    assert_runnable(
        &suggestions,
        "controlled_impedance_solver_result_rf_solver_result",
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
