mod common;

use common::assert_report_schema_valid;
use serde_json::Value;
use std::process::Command;

#[test]
fn repair_yaml_fixes_invalid_power_domain_on_project_copy() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: bad_power_net_kind
  version: 0.1.0

libraries:
  - {}

board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins:
        P: vin
        N: gnd
  nets:
    vin:
      kind: digital_or_analog
      nominal_voltage: 5.0
      powered: true
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
    assert_eq!(repair_report["finding"], "INVALID_POWER_DOMAIN");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["applied"], 1);
    assert_eq!(repair_report["summary"]["original_matching_criticals"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_criticals"], 0);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert_eq!(repair_report["proof"]["original_finding_removed"], true);
    assert_eq!(repair_report["proof"]["no_new_criticals"], true);
    assert_eq!(
        repair_report["proposals"][0]["edits"][0]["path"],
        "/board/nets/vin/kind"
    );
    assert_eq!(repair_report["proposals"][0]["affected_pins"][0], "V1.P");
    assert_eq!(
        repair_report["proposals"][0]["edits"][0]["from"],
        "digital_or_analog"
    );
    assert_eq!(repair_report["proposals"][0]["edits"][0]["to"], "power");

    let original_report: Value = serde_json::from_str(
        &std::fs::read_to_string(output.join("original/report.json")).unwrap(),
    )
    .unwrap();
    assert_report_schema_valid(&original_report);
    assert_eq!(original_report["result"], "fail");
    assert_eq!(original_report["failures"][0]["id"], "INVALID_POWER_DOMAIN");

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
    assert_eq!(repaired_yaml["board"]["nets"]["vin"]["kind"], "power");
}

#[test]
fn repair_yaml_adds_missing_net_with_model_inferred_kind() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: missing_power_net
  version: 0.1.0

libraries:
  - {}

board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins:
        P: vin
        N: gnd
  nets:
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
            "net-not-found",
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
    assert_eq!(repair_report["finding"], "NET_NOT_FOUND");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["applied"], 1);
    assert_eq!(repair_report["summary"]["original_matching_criticals"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_criticals"], 0);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert_eq!(repair_report["proof"]["original_finding_removed"], true);
    assert_eq!(repair_report["proof"]["no_new_criticals"], true);
    assert_eq!(
        repair_report["proposals"][0]["yaml_path"],
        "/board/nets/vin"
    );
    assert_eq!(repair_report["proposals"][0]["affected_pins"][0], "V1.P");
    assert_eq!(repair_report["proposals"][0]["edits"][0]["op"], "add");
    assert!(repair_report["proposals"][0]["edits"][0]["from"].is_null());
    assert_eq!(
        repair_report["proposals"][0]["edits"][0]["to"]["kind"],
        "power"
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
            .any(|failure| failure["id"] == "NET_NOT_FOUND")
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
    assert_eq!(repaired_yaml["board"]["nets"]["vin"]["kind"], "power");
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
