use serde_json::Value;
use std::process::Command;

#[test]
fn suggest_scenarios_derives_adjacent_plane_return_path_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_adjacent_plane_return_path/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_adjacent_plane_return_path"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 1);

    let return_path = &suggested[0];
    assert_eq!(return_path["id"], "adjacent_plane_return_path_sig");
    assert_eq!(
        return_path["kind"],
        "manufacturing_adjacent_plane_return_path_sig"
    );
    assert_eq!(return_path["runnable"], true);
    assert_eq!(return_path["scenario"]["type"], "manufacturing");
    assert_eq!(
        return_path["scenario"]["checks"][0],
        "ADJACENT_PLANE_RETURN_PATH_VALID"
    );
    let route = &return_path["scenario"]["parameters"]["routes"][0];
    assert_eq!(route["net"], "SIG");
    assert_eq!(route["reference_net"], "GND");
    assert_eq!(route["reference_layer"], "In1.Cu");
    assert_eq!(route["max_unreferenced_length_mm"], 0.0);
    assert!(return_path.get("required_inputs").is_none());
    assert!(
        return_path["reason"]
            .as_str()
            .unwrap()
            .contains("sampled adjacent GND plane-zone coverage")
    );
}

#[test]
fn suggest_scenarios_derives_reference_plane_slot_crossing_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_reference_plane_slot_crossing/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_reference_plane_slot_crossing"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 1);

    let slot_crossing = &suggested[0];
    assert_eq!(slot_crossing["id"], "reference_plane_slot_crossing_sig");
    assert_eq!(
        slot_crossing["kind"],
        "manufacturing_reference_plane_slot_crossing_sig"
    );
    assert_eq!(slot_crossing["runnable"], true);
    assert_eq!(slot_crossing["scenario"]["type"], "manufacturing");
    assert_eq!(
        slot_crossing["scenario"]["checks"][0],
        "REFERENCE_PLANE_SLOT_CROSSING_VALID"
    );
    let route = &slot_crossing["scenario"]["parameters"]["routes"][0];
    assert_eq!(route["net"], "SIG");
    assert_eq!(route["reference_net"], "GND");
    assert_eq!(route["reference_layer"], "In1.Cu");
    assert_eq!(route["max_slot_crossings"], 0);
    assert!(slot_crossing.get("required_inputs").is_none());
    assert!(
        slot_crossing["reason"]
            .as_str()
            .unwrap()
            .contains("1 internal reference-plane gap")
    );
}

#[test]
fn suggest_scenarios_derives_return_path_stitching_via_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_return_path_stitching_via/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_return_path_stitching_via"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 1);

    let stitching = &suggested[0];
    assert_eq!(stitching["id"], "return_path_stitching_via_sig");
    assert_eq!(
        stitching["kind"],
        "manufacturing_return_path_stitching_via_sig"
    );
    assert_eq!(stitching["runnable"], true);
    assert_eq!(stitching["scenario"]["type"], "manufacturing");
    assert_eq!(
        stitching["scenario"]["checks"][0],
        "RETURN_PATH_STITCHING_VIA_VALID"
    );
    let route = &stitching["scenario"]["parameters"]["routes"][0];
    assert_eq!(route["net"], "SIG");
    assert_eq!(route["reference_net"], "GND");
    assert_eq!(route["max_stitch_via_distance_mm"], 1.0);
    assert!(stitching.get("required_inputs").is_none());
    assert!(
        stitching["reason"]
            .as_str()
            .unwrap()
            .contains("reviewed board.manufacturing.max_stitch_via_distance_mm")
    );
}

fn run_suggest_scenarios(project: &str) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("suggestions.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            project,
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let suggestions: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(output).unwrap()).unwrap();
    assert_suggestion_schema_valid(&suggestions);
    assert_runnable_suggestions_have_no_required_inputs(&suggestions);
    suggestions
}

fn assert_suggestion_schema_valid(suggestions: &Value) {
    let schema: Value = serde_json::from_str(include_str!(
        "../schemas/scenario_suggestion_report.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(suggestions)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "suggestion schema errors: {errors:#?}");
}

fn assert_runnable_suggestions_have_no_required_inputs(suggestions: &Value) {
    for suggestion in suggestions["suggestions"].as_array().unwrap() {
        if suggestion["runnable"].as_bool().unwrap() {
            assert!(
                suggestion.get("required_inputs").is_none(),
                "runnable suggestion {} has required_inputs",
                suggestion["id"]
            );
        }
    }
}
