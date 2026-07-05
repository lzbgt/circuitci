mod common;

use common::assert_report_schema_valid;
use serde_json::Value;
use std::process::Command;

#[test]
fn repair_yaml_canonicalizes_unique_model_id_on_project_copy() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: model_id_case_mismatch
  version: 0.1.0

libraries:
  - {}

board:
  components:
    V1:
      model: " Generic.Analog.DC_Voltage_Source "
      pins:
        P: vin
        N: gnd
  nets:
    vin:
      kind: power
    gnd:
      kind: ground
"#,
            repo.join("libs/generic").display()
        ),
    )
    .unwrap();
    let output = temp.path().join("repair");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            project.to_str().unwrap(),
            "--finding",
            "model-not-found",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let repair_report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("repair_report.json")).unwrap())
            .unwrap();
    assert_repair_report_schema_valid(&repair_report);
    assert_eq!(repair_report["result"], "pass");
    assert_eq!(repair_report["finding"], "MODEL_NOT_FOUND");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["applied"], 1);
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 0);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert_eq!(
        repair_report["proposals"][0]["yaml_path"],
        "/board/components/V1/model"
    );
    assert_eq!(repair_report["proposals"][0]["affected_pins"][0], "V1.N");
    assert_eq!(
        repair_report["proposals"][0]["edits"][0]["from"],
        " Generic.Analog.DC_Voltage_Source "
    );
    assert_eq!(
        repair_report["proposals"][0]["edits"][0]["to"],
        "generic.analog.dc_voltage_source"
    );

    let original_report: Value = serde_json::from_str(
        &std::fs::read_to_string(output.join("original/report.json")).unwrap(),
    )
    .unwrap();
    assert_report_schema_valid(&original_report);
    assert_eq!(original_report["result"], "fail");
    assert!(
        original_report["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure["id"] == "MODEL_NOT_FOUND")
    );

    let repaired_report: Value = serde_json::from_str(
        &std::fs::read_to_string(output.join("repaired/report.json")).unwrap(),
    )
    .unwrap();
    assert_report_schema_valid(&repaired_report);
    assert_eq!(repaired_report["result"], "pass");
    assert_eq!(repaired_report["summary"]["critical"], 0);

    let repaired_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string(output.join("repaired/project.yaml")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        repaired_yaml["board"]["components"]["V1"]["model"],
        "generic.analog.dc_voltage_source"
    );
}

#[test]
fn repair_yaml_model_not_found_does_not_invent_unknown_model() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: unknown_model_id
  version: 0.1.0

libraries:
  - {}

board:
  components:
    U1:
      model: vendor.unknown.no_such_part
      pins:
        VDD: vcc
        GND: gnd
  nets:
    vcc:
      kind: power
    gnd:
      kind: ground
"#,
            repo.join("libs/generic").display()
        ),
    )
    .unwrap();
    let output = temp.path().join("repair");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            project.to_str().unwrap(),
            "--finding",
            "model-not-found",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let repair_report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("repair_report.json")).unwrap())
            .unwrap();
    assert_repair_report_schema_valid(&repair_report);
    assert_eq!(repair_report["result"], "fail");
    assert_eq!(repair_report["finding"], "MODEL_NOT_FOUND");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["applied"], 0);
    assert_eq!(repair_report["summary"]["blocked"], 1);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 1);
    assert_eq!(
        repair_report["proposals"][0]["reason_code"],
        "unresolved_model_id"
    );
    assert_eq!(repair_report["proposals"][0]["edits"][0]["to"], Value::Null);

    let reason_codes = repair_report["reason_codes"].as_array().unwrap();
    assert!(
        reason_codes
            .iter()
            .any(|reason| reason == "proposal_blocked")
    );
}

fn assert_repair_report_schema_valid(report: &Value) {
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/repair_report.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(report)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "repair report schema errors: {errors:#?}"
    );
}
