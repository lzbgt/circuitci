use serde_json::Value as JsonValue;
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
    let suggestions: JsonValue =
        serde_yaml_ng::from_str(&std::fs::read_to_string(suggestions_output).unwrap()).unwrap();
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

fn remove_board_manufacturing(project_yaml: &mut Value) {
    let board = project_yaml["board"].as_mapping_mut().unwrap();
    board.remove(Value::String("manufacturing".to_string()));
}

fn assert_runnable(suggestions: &JsonValue, id: &str) {
    let suggestion = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == id)
        .unwrap_or_else(|| panic!("missing suggestion {id}"));
    assert_eq!(suggestion["runnable"], true, "{id}");
    assert!(suggestion.get("required_inputs").is_none(), "{id}");
}
