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
        "field,value,unit,source,notes\n\
         stencil_thickness_mm,0.10,mm,JLC stencil order,foil thickness\n\
         min_drill_edge_clearance_mm,0.50,mm,JLC fab order,hole edge clearance\n\
         min_slot_edge_clearance_mm,0.50,mm,JLC fab order,slot edge clearance\n\
         min_paste_area_ratio,70,%,JLC stencil order,minimum aperture area ratio\n\
         max_paste_area_ratio,100,%,JLC stencil order,maximum aperture area ratio\n\
         min_solder_paste_spacing_mm,0.15,mm,JLC stencil order,min paste spacing\n\
         unrelated_order_option,blue,,JLC order,kept as skipped evidence\n",
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
    assert!(stdout.contains("6 applied fields"));
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
    assert_eq!(manifest["schema_version"], "0.1.0");
    assert_eq!(manifest["sources"]["metadata"]["data_rows"], 7);
    assert_eq!(manifest["import"]["applied_fields"], 6);
    assert_eq!(manifest["import"]["skipped_rows"], 1);
    assert_eq!(manifest["rows"][3]["normalized_value"], 0.70);
    assert_eq!(manifest["rows"][6]["status"], "skipped_unknown_field");
    assert_eq!(
        manifest["rows"][6]["raw_columns"]["notes"],
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
