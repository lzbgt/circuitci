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
    assert_eq!(repair_report["mode"], "apply");
    assert_eq!(repair_report["finding"], "INVALID_POWER_DOMAIN");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["applied"], 1);
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(repair_report["summary"]["skipped"], 0);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 0);
    assert_eq!(repair_report["summary"]["original_matching_criticals"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_criticals"], 0);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert!(repair_report["messages"].as_array().unwrap().is_empty());
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
fn repair_yaml_dry_run_reports_proposals_without_writing_repaired_copy() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: dry_run_power_net_kind
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
            "--dry-run",
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
    assert_eq!(repair_report["result"], "dry_run");
    assert_eq!(repair_report["mode"], "dry_run");
    assert_eq!(repair_report["finding"], "INVALID_POWER_DOMAIN");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["applied"], 0);
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(repair_report["summary"]["skipped"], 0);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 0);
    assert_eq!(repair_report["summary"]["original_matching_criticals"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_criticals"], 0);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert!(repair_report["repaired_project"].is_null());
    assert!(repair_report["repaired_report"].is_null());
    assert!(repair_report["proof"]["original_finding_removed"].is_null());
    assert!(repair_report["proof"]["no_new_criticals"].is_null());
    assert_eq!(repair_report["proposals"][0]["status"], "proposed");
    assert_eq!(
        repair_report["proposals"][0]["edits"][0]["path"],
        "/board/nets/vin/kind"
    );
    assert!(
        repair_report["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message.as_str().unwrap().contains("Dry run skipped"))
    );
    assert!(
        repair_report["reason_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "dry_run_not_validated")
    );
    assert!(
        repair_report["reproduction"]["command"]
            .as_str()
            .unwrap()
            .contains("--dry-run")
    );
    assert!(output.join("original/report.json").exists());
    assert!(!output.join("repaired/project.yaml").exists());
    assert!(!output.join("repaired/report.json").exists());
}

#[test]
fn repair_yaml_apply_report_replays_previous_dry_run_edits() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: apply_report_power_net_kind
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
    let dry_output = temp.path().join("repair_dry");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            project.to_str().unwrap(),
            "--dry-run",
            "--output",
            dry_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let apply_output = temp.path().join("repair_apply");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            project.to_str().unwrap(),
            "--apply-report",
            dry_output.join("repair_report.json").to_str().unwrap(),
            "--output",
            apply_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let repair_report: Value = serde_json::from_str(
        &std::fs::read_to_string(apply_output.join("repair_report.json")).unwrap(),
    )
    .unwrap();
    assert_repair_report_schema_valid(&repair_report);
    assert_eq!(repair_report["result"], "pass");
    assert_eq!(repair_report["mode"], "apply_report");
    assert_eq!(repair_report["finding"], "INVALID_POWER_DOMAIN");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["selected"], 1);
    assert_eq!(repair_report["summary"]["applied"], 1);
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(repair_report["summary"]["skipped"], 0);
    assert_eq!(repair_report["proof"]["original_finding_removed"], true);
    assert_eq!(repair_report["proof"]["no_new_criticals"], true);
    assert_eq!(repair_report["proposals"][0]["status"], "applied");
    assert!(
        repair_report["reproduction"]["command"]
            .as_str()
            .unwrap()
            .contains("--apply-report")
    );

    let repaired_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string(apply_output.join("repaired/project.yaml")).unwrap(),
    )
    .unwrap();
    assert_eq!(repaired_yaml["board"]["nets"]["vin"]["kind"], "power");
}

#[test]
fn repair_yaml_apply_report_applies_selected_proposal_id_only() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: selective_apply_report_power_net_kind
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
    V2:
      model: generic.analog.dc_voltage_source
      pins:
        P: vaux
        N: gnd
  nets:
    vin:
      kind: digital_or_analog
      nominal_voltage: 5.0
      powered: true
    vaux:
      kind: digital_or_analog
      nominal_voltage: 3.3
      powered: true
    gnd:
      kind: ground
"#,
            repo.join("libs/generic").display()
        ),
    )
    .unwrap();
    let dry_output = temp.path().join("repair_dry");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            project.to_str().unwrap(),
            "--dry-run",
            "--output",
            dry_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let dry_report: Value = serde_json::from_str(
        &std::fs::read_to_string(dry_output.join("repair_report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(dry_report["summary"]["proposed"], 2);
    let selected_id = dry_report["proposals"][0]["id"].as_str().unwrap();
    let selected_path = dry_report["proposals"][0]["yaml_path"].as_str().unwrap();
    let selected_net = selected_path
        .strip_prefix("/board/nets/")
        .unwrap()
        .strip_suffix("/kind")
        .unwrap();
    let unselected_net = if selected_net == "vin" { "vaux" } else { "vin" };

    let apply_output = temp.path().join("repair_apply");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            project.to_str().unwrap(),
            "--apply-report",
            dry_output.join("repair_report.json").to_str().unwrap(),
            "--proposal-id",
            selected_id,
            "--output",
            apply_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let repair_report: Value = serde_json::from_str(
        &std::fs::read_to_string(apply_output.join("repair_report.json")).unwrap(),
    )
    .unwrap();
    assert_repair_report_schema_valid(&repair_report);
    assert_eq!(repair_report["result"], "fail");
    assert_eq!(repair_report["mode"], "apply_report");
    assert_eq!(repair_report["summary"]["proposed"], 2);
    assert_eq!(repair_report["summary"]["selected"], 1);
    assert_eq!(repair_report["summary"]["applied"], 1);
    assert_eq!(repair_report["summary"]["skipped"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 1);
    assert_eq!(repair_report["proof"]["original_finding_removed"], false);
    assert_eq!(repair_report["proof"]["no_new_criticals"], true);
    let proposals = repair_report["proposals"].as_array().unwrap();
    assert!(
        proposals
            .iter()
            .any(|proposal| { proposal["id"] == selected_id && proposal["status"] == "applied" })
    );
    assert!(
        proposals
            .iter()
            .any(|proposal| { proposal["id"] != selected_id && proposal["status"] == "skipped" })
    );
    assert!(proposals.iter().any(|proposal| {
        proposal["id"] != selected_id && proposal["reason_code"] == "not_selected"
    }));
    let messages = repair_report["messages"].as_array().unwrap();
    assert!(messages.iter().any(|message| {
        message
            .as_str()
            .unwrap()
            .contains("not selected by --proposal-id")
    }));
    assert!(
        repair_report["reason_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "proposal_skipped_not_selected")
    );
    assert!(messages.iter().any(|message| {
        message
            .as_str()
            .unwrap()
            .contains("selective repaired copy")
    }));
    assert!(
        repair_report["reason_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "target_finding_remains_selective_apply")
    );
    assert!(
        repair_report["reproduction"]["command"]
            .as_str()
            .unwrap()
            .contains(&format!("--proposal-id {selected_id}"))
    );

    let repaired_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string(apply_output.join("repaired/project.yaml")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        repaired_yaml["board"]["nets"][selected_net]["kind"],
        "power"
    );
    assert_eq!(
        repaired_yaml["board"]["nets"][unselected_net]["kind"],
        "digital_or_analog"
    );
}

#[test]
fn repair_yaml_apply_report_rejects_stale_project_findings() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    let bad_project = format!(
        r#"
project:
  name: stale_apply_report_power_net_kind
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
    );
    std::fs::write(&project, &bad_project).unwrap();
    let dry_output = temp.path().join("repair_dry");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            project.to_str().unwrap(),
            "--dry-run",
            "--output",
            dry_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    std::fs::write(
        &project,
        bad_project.replace("kind: digital_or_analog", "kind: power"),
    )
    .unwrap();
    let apply_output = temp.path().join("repair_apply");
    let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            project.to_str().unwrap(),
            "--apply-report",
            dry_output.join("repair_report.json").to_str().unwrap(),
            "--output",
            apply_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("original matching findings no longer match"));
    assert!(!apply_output.join("repair_report.json").exists());
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
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(repair_report["summary"]["skipped"], 0);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 0);
    assert_eq!(repair_report["summary"]["original_matching_criticals"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_criticals"], 0);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert!(repair_report["messages"].as_array().unwrap().is_empty());
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

#[test]
fn repair_yaml_reports_ambiguous_missing_net_without_applying_patch() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: ambiguous_missing_net
  version: 0.1.0

libraries:
  - {}

board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins:
        P: shared
        N: gnd
    R1:
      model: generic.analog.resistor
      pins:
        A: shared
        B: gnd
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
    assert_eq!(repair_report["result"], "fail");
    assert_eq!(repair_report["finding"], "NET_NOT_FOUND");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["applied"], 0);
    assert_eq!(repair_report["summary"]["blocked"], 1);
    assert_eq!(repair_report["summary"]["skipped"], 0);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 2);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 2);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert_eq!(repair_report["proof"]["original_finding_removed"], false);
    assert_eq!(repair_report["proof"]["no_new_criticals"], true);
    assert_eq!(repair_report["proposals"][0]["status"], "blocked");
    assert_eq!(
        repair_report["proposals"][0]["reason_code"],
        "conflicting_inferred_net_kinds"
    );
    assert_eq!(
        repair_report["proposals"][0]["yaml_path"],
        "/board/nets/shared"
    );
    assert!(
        repair_report["proposals"][0]["edits"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let affected = repair_report["proposals"][0]["affected_pins"]
        .as_array()
        .unwrap();
    assert!(affected.iter().any(|pin| pin == "V1.P"));
    assert!(affected.iter().any(|pin| pin == "R1.A"));
    assert!(
        repair_report["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message.as_str().unwrap().contains("blocked"))
    );
    assert!(
        repair_report["reason_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "proposal_blocked")
    );
    assert!(
        repair_report["proposals"][0]["description"]
            .as_str()
            .unwrap()
            .contains("conflicting net kinds")
    );

    let repaired_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string(output.join("repaired/project.yaml")).unwrap(),
    )
    .unwrap();
    assert!(repaired_yaml["board"]["nets"]["shared"].is_null());
}

#[test]
fn repair_yaml_removes_pin_not_declared_warning_on_project_copy() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: stray_pin_binding
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
        EXTRA: spare
  nets:
    vin:
      kind: power
    gnd:
      kind: ground
    spare:
      kind: digital_or_analog
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
            "pin-not-declared",
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
    assert_eq!(repair_report["finding"], "PIN_NOT_DECLARED");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["applied"], 1);
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(repair_report["summary"]["skipped"], 0);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 0);
    assert_eq!(repair_report["summary"]["original_matching_criticals"], 0);
    assert_eq!(repair_report["summary"]["repaired_matching_criticals"], 0);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert!(repair_report["messages"].as_array().unwrap().is_empty());
    assert_eq!(repair_report["proof"]["original_finding_removed"], true);
    assert_eq!(repair_report["proof"]["no_new_criticals"], true);
    assert_eq!(
        repair_report["proof"]["original_matching_findings"][0]["severity"],
        "warning"
    );
    assert_eq!(
        repair_report["proposals"][0]["yaml_path"],
        "/board/components/V1/pins/EXTRA"
    );
    assert_eq!(
        repair_report["proposals"][0]["affected_pins"][0],
        "V1.EXTRA"
    );
    assert_eq!(repair_report["proposals"][0]["edits"][0]["op"], "remove");
    assert_eq!(repair_report["proposals"][0]["edits"][0]["from"], "spare");
    assert!(repair_report["proposals"][0]["edits"][0]["to"].is_null());

    let original_report: Value = serde_json::from_str(
        &std::fs::read_to_string(output.join("original/report.json")).unwrap(),
    )
    .unwrap();
    assert_report_schema_valid(&original_report);
    assert_eq!(original_report["result"], "pass");
    assert!(
        original_report["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["id"] == "PIN_NOT_DECLARED")
    );

    let repaired_report: Value = serde_json::from_str(
        &std::fs::read_to_string(output.join("repaired/report.json")).unwrap(),
    )
    .unwrap();
    assert_report_schema_valid(&repaired_report);
    assert_eq!(repaired_report["result"], "pass");
    assert_eq!(repaired_report["summary"]["critical"], 0);
    assert!(
        repaired_report["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|warning| warning["id"] != "PIN_NOT_DECLARED")
    );

    let repaired_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string(output.join("repaired/project.yaml")).unwrap(),
    )
    .unwrap();
    assert!(repaired_yaml["board"]["components"]["V1"]["pins"]["EXTRA"].is_null());
}

#[test]
fn repair_yaml_connects_required_pin_from_component_power_domain_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: required_pin_from_power_domain
  version: 0.1.0

libraries:
  - {}

board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      power_domains:
        P: vin
      pins:
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
            "required-pin-floating",
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
    assert_eq!(repair_report["finding"], "REQUIRED_PIN_FLOATING");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["selected"], 1);
    assert_eq!(repair_report["summary"]["applied"], 1);
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(repair_report["summary"]["skipped"], 0);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 0);
    assert_eq!(repair_report["summary"]["original_matching_criticals"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_criticals"], 0);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert!(repair_report["messages"].as_array().unwrap().is_empty());
    assert_eq!(repair_report["proof"]["original_finding_removed"], true);
    assert_eq!(repair_report["proof"]["no_new_criticals"], true);
    assert_eq!(
        repair_report["proposals"][0]["yaml_path"],
        "/board/components/V1/pins/P"
    );
    assert_eq!(repair_report["proposals"][0]["affected_pins"][0], "V1.P");
    assert_eq!(repair_report["proposals"][0]["edits"][0]["op"], "add");
    assert!(repair_report["proposals"][0]["edits"][0]["from"].is_null());
    assert_eq!(repair_report["proposals"][0]["edits"][0]["to"], "vin");

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
            .any(|failure| failure["id"] == "REQUIRED_PIN_FLOATING")
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
        repaired_yaml["board"]["components"]["V1"]["pins"]["P"],
        "vin"
    );
    assert_eq!(
        repaired_yaml["board"]["components"]["V1"]["pins"]["N"],
        "gnd"
    );
}

#[test]
fn repair_yaml_required_pin_floating_does_not_invent_unproven_pin_net() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: required_pin_without_safe_target
  version: 0.1.0

libraries:
  - {}

board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      pins:
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
            "required-pin-floating",
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
    assert_eq!(repair_report["finding"], "REQUIRED_PIN_FLOATING");
    assert_eq!(repair_report["summary"]["proposed"], 0);
    assert_eq!(repair_report["summary"]["selected"], 0);
    assert_eq!(repair_report["summary"]["applied"], 0);
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(repair_report["summary"]["skipped"], 0);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 1);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 1);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert_eq!(repair_report["proof"]["original_finding_removed"], false);
    assert_eq!(repair_report["proof"]["no_new_criticals"], true);
    assert!(repair_report["proposals"].as_array().unwrap().is_empty());
    assert!(
        repair_report["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message
                .as_str()
                .unwrap()
                .contains("No supported REQUIRED_PIN_FLOATING repair proposal"))
    );
    let reason_codes = repair_report["reason_codes"].as_array().unwrap();
    assert!(
        reason_codes
            .iter()
            .any(|code| code == "no_supported_proposal")
    );
    assert!(
        reason_codes
            .iter()
            .any(|code| code == "target_finding_remains")
    );

    let repaired_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string(output.join("repaired/project.yaml")).unwrap(),
    )
    .unwrap();
    assert!(repaired_yaml["board"]["components"]["V1"]["pins"]["P"].is_null());
}

#[test]
fn repair_yaml_reports_empty_repair_when_target_finding_is_absent() {
    let temp = tempfile::tempdir().unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: already_valid_power_source
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
            "pin-not-declared",
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
    assert_eq!(repair_report["finding"], "PIN_NOT_DECLARED");
    assert_eq!(repair_report["summary"]["proposed"], 0);
    assert_eq!(repair_report["summary"]["applied"], 0);
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(repair_report["summary"]["skipped"], 0);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 0);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 0);
    assert_eq!(repair_report["summary"]["new_criticals"], 0);
    assert!(repair_report["proposals"].as_array().unwrap().is_empty());
    let messages = repair_report["messages"].as_array().unwrap();
    assert!(messages.iter().any(|message| {
        message
            .as_str()
            .unwrap()
            .contains("no matching finding was available")
    }));
    assert!(messages.iter().any(|message| {
        message
            .as_str()
            .unwrap()
            .contains("No supported PIN_NOT_DECLARED repair proposal")
    }));
    let reason_codes = repair_report["reason_codes"].as_array().unwrap();
    assert!(
        reason_codes
            .iter()
            .any(|code| code == "target_finding_absent")
    );
    assert!(
        reason_codes
            .iter()
            .any(|code| code == "no_supported_proposal")
    );
}

#[test]
fn repair_yaml_adds_generated_analog_model_package_metadata() {
    std::fs::create_dir_all("out").unwrap();
    let temp = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let project = temp.path().join("project.yaml");
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: old_generated_analog_package_metadata
  version: 0.1.0

libraries:
  - {}

board:
  components:
    VCC:
      model: generic.analog.dc_voltage_source
      pins: {{P: vcc_5v, N: gnd}}
      spice: {{primitive: dc_voltage_source, dc_v: 5.0}}
    VIN:
      model: generic.analog.dc_voltage_source
      pins: {{P: input, N: gnd}}
      spice: {{primitive: dc_voltage_source, dc_v: 1.0}}
    XU1:
      model: generic.analog.ideal_opamp
      pins: {{INP: input, INN: output, VCC: vcc_5v, VEE: gnd, OUT: output}}
    RLOAD:
      model: generic.analog.resistor
      pins: {{A: output, B: gnd}}
      spice: {{primitive: resistor, value_ohm: 10000.0}}
  nets:
    vcc_5v: {{kind: power, nominal_voltage: 5.0, powered: true}}
    input: {{kind: digital_or_analog}}
    output: {{kind: digital_or_analog}}
    gnd: {{kind: ground}}

scenarios:
  - name: old_generated_step
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [VCC, VIN, XU1, RLOAD]
      model_files:
        - path: ../../models/spice/generic/analog_behavioral.lib
          sha256: ad5aec2585e6d9803b3b6f7930c19148e252c1cf4362b550893e85afdd025e59
      node_bindings:
        - {{node: vcc, net: vcc_5v}}
        - {{node: in, net: input}}
        - {{node: out, net: output}}
        - {{node: "0", net: gnd}}
      pin_bindings:
        - {{node: vcc, endpoint: {{component: VCC, pin: P}}}}
        - {{node: in, endpoint: {{component: VIN, pin: P}}}}
        - {{node: out, endpoint: {{component: XU1, pin: OUT}}}}
      analysis:
        type: transient
        stop_time_us: 10.0
        max_step_us: 0.1
      stimuli: []
      probes:
        - {{name: output_voltage, expression: V(out)}}
      assertions: []
"#,
            repo.join("libs/generic/analog").display()
        ),
    )
    .unwrap();

    let output = temp.path().join("repair");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            project.to_str().unwrap(),
            "--finding",
            "analog-model-package-metadata",
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
    assert_eq!(repair_report["finding"], "ANALOG_MODEL_PACKAGE_METADATA");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["applied"], 1);
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(repair_report["summary"]["original_matching_findings"], 0);
    assert_eq!(repair_report["summary"]["repaired_matching_findings"], 0);
    assert_eq!(repair_report["proof"]["original_finding_removed"], true);
    assert_eq!(repair_report["proof"]["no_new_criticals"], true);
    assert!(repair_report["messages"].as_array().unwrap().is_empty());
    assert_eq!(
        repair_report["proposals"][0]["edits"][0]["path"],
        "/scenarios/0/analog/model_files/0/model_package_name"
    );

    let repaired_project = output.join("repaired/project.yaml");
    let repaired_yaml = std::fs::read_to_string(repaired_project).unwrap();
    let repaired: circuitci::board_ir::BoardProject =
        serde_yaml_ng::from_str(&repaired_yaml).unwrap();
    let model_file = &repaired.scenarios[0].analog.as_ref().unwrap().model_files[0];
    assert_eq!(
        model_file.model_package_name.as_deref(),
        Some("org.circuitci.models.generic.analog_behavioral")
    );
    assert_eq!(
        model_file.model_package_artifact_id.as_deref(),
        Some("generic_analog_behavioral_spice")
    );
    assert_eq!(
        model_file.model_package_lock_path.as_deref(),
        Some(
            repo.join("models/packages/generic/analog_behavioral.lock.json")
                .to_str()
                .unwrap()
        )
    );
    assert_eq!(
        model_file.model_package_registry_path.as_deref(),
        Some(
            repo.join("models/packages/compact_model_registry.json")
                .to_str()
                .unwrap()
        )
    );
    assert_eq!(
        model_file.model_package_registry_entry.as_deref(),
        Some("generic_analog_behavioral_spice")
    );
}

#[test]
fn repair_yaml_imports_bundle_install_package_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project.yaml");
    let installed = temp.path().join("installed_bundle");
    let artifacts = installed.join("artifacts");
    let shared = temp.path().join("shared");
    std::fs::create_dir_all(&artifacts).unwrap();
    std::fs::create_dir_all(&shared).unwrap();
    std::fs::write(artifacts.join("runtime.osdi"), "runtime artifact\n").unwrap();
    std::fs::write(installed.join("package.lock.json"), "{}\n").unwrap();
    std::fs::write(shared.join("compact_model_registry.json"), "{}\n").unwrap();
    let lock_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let registry_sha = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    std::fs::write(
        &project,
        r#"
project:
  name: bundle_install_metadata_import
  version: 0.1.0

board:
  components: {}
  nets: {}

scenarios:
  - name: package_import
    type: analog_transient
    checks: []
    analog:
      backend: ngspice
      netlist_source: file
      model_files:
        - path: installed_bundle/artifacts/runtime.osdi
          artifact_format: osdi_shared_object
          model_package_artifact_id: runtime_osdi
      node_bindings: []
      pin_bindings: []
      analysis:
        type: transient
        stop_time_us: 1.0
        max_step_us: 0.1
      stimuli: []
      probes: []
      assertions: []
"#,
    )
    .unwrap();
    let install_report = temp.path().join("bundle_install_report.json");
    std::fs::write(
        &install_report,
        format!(
            r#"{{
  "schema_version": "circuitci.model_package_bundle_install.v1",
  "result": "pass",
  "source_bundle": "bundle",
  "install_dir": "installed_bundle",
  "package": {{
    "name": "org.circuitci.models.bundle.test",
    "version": "1.2.3"
  }},
  "manifest": {{
    "path": "installed_bundle/model_package_bundle_manifest.json",
    "sha256_declared": null,
    "sha256_actual": null,
    "status": "verified"
  }},
  "lock": {{
    "path": "installed_bundle/package.lock.json",
    "sha256_declared": "{lock_sha}",
    "sha256_actual": "{lock_sha}",
    "status": "verified"
  }},
  "registry": null,
  "installed_registry": {{
    "path": "shared/compact_model_registry.json",
    "sha256_declared": "{registry_sha}",
    "sha256_actual": "{registry_sha}",
    "status": "verified"
  }},
  "scenario_import": {{
    "model_package_registry_path": "shared/compact_model_registry.json",
    "model_package_registry_sha256": "{registry_sha}",
    "model_package_registry_entry": "bundle_runtime",
    "model_package_lock_path": "installed_bundle/package.lock.json",
    "model_package_lock_sha256": "{lock_sha}",
    "model_package_artifact_id": "runtime_osdi"
  }},
  "artifacts": [
    {{
      "id": "runtime_osdi",
      "path": "installed_bundle/artifacts/runtime.osdi",
      "artifact_format": "osdi_shared_object",
      "compiler": "openvaf",
      "sha256_declared": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "sha256_actual": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "status": "verified"
    }}
  ],
  "conformance_checks": [],
  "findings": []
}}
"#
        ),
    )
    .unwrap();

    let output = temp.path().join("repair");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            project.to_str().unwrap(),
            "--finding",
            "bundle-install-package-metadata",
            "--bundle-install-report",
            install_report.to_str().unwrap(),
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
    assert_eq!(repair_report["finding"], "BUNDLE_INSTALL_PACKAGE_METADATA");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["applied"], 1);
    assert_eq!(repair_report["summary"]["blocked"], 0);
    assert_eq!(
        repair_report["proposals"][0]["edits"][0]["path"],
        "/scenarios/0/analog/model_files/0/model_package_name"
    );

    let repaired_project = output.join("repaired/project.yaml");
    let repaired_yaml = std::fs::read_to_string(repaired_project).unwrap();
    let repaired: circuitci::board_ir::BoardProject =
        serde_yaml_ng::from_str(&repaired_yaml).unwrap();
    let model_file = &repaired.scenarios[0].analog.as_ref().unwrap().model_files[0];
    let expected_lock = std::fs::canonicalize(installed.join("package.lock.json")).unwrap();
    let expected_registry =
        std::fs::canonicalize(shared.join("compact_model_registry.json")).unwrap();
    assert_eq!(
        model_file.model_package_name.as_deref(),
        Some("org.circuitci.models.bundle.test")
    );
    assert_eq!(model_file.model_package_version.as_deref(), Some("1.2.3"));
    assert_eq!(
        model_file.model_package_artifact_id.as_deref(),
        Some("runtime_osdi")
    );
    assert_eq!(
        model_file.model_package_lock_path.as_deref(),
        Some(expected_lock.to_str().unwrap())
    );
    assert_eq!(
        model_file.model_package_lock_sha256.as_deref(),
        Some(lock_sha)
    );
    assert_eq!(
        model_file.model_package_registry_path.as_deref(),
        Some(expected_registry.to_str().unwrap())
    );
    assert_eq!(
        model_file.model_package_registry_sha256.as_deref(),
        Some(registry_sha)
    );
    assert_eq!(
        model_file.model_package_registry_entry.as_deref(),
        Some("bundle_runtime")
    );
}

#[test]
fn repair_yaml_blocks_bundle_install_package_metadata_conflict() {
    let fixture = bundle_install_repair_fixture(
        "pass",
        "          model_package_registry_entry: different_bundle\n",
    );
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            fixture.project.to_str().unwrap(),
            "--finding",
            "bundle-install-package-metadata",
            "--bundle-install-report",
            fixture.install_report.to_str().unwrap(),
            "--output",
            fixture.output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let repair_report: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture.output.join("repair_report.json")).unwrap(),
    )
    .unwrap();
    assert_repair_report_schema_valid(&repair_report);
    assert_eq!(repair_report["result"], "fail");
    assert_eq!(repair_report["finding"], "BUNDLE_INSTALL_PACKAGE_METADATA");
    assert_eq!(repair_report["summary"]["proposed"], 1);
    assert_eq!(repair_report["summary"]["selected"], 0);
    assert_eq!(repair_report["summary"]["applied"], 0);
    assert_eq!(repair_report["summary"]["blocked"], 1);
    assert_eq!(
        repair_report["proposals"][0]["reason_code"],
        "bundle_install_package_metadata_conflict"
    );
    assert!(
        repair_report["proposals"][0]["description"]
            .as_str()
            .unwrap()
            .contains("model_package_registry_entry=different_bundle")
    );
}

#[test]
fn repair_yaml_rejects_failed_bundle_install_report() {
    let fixture = bundle_install_repair_fixture("fail", "");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "repair-yaml",
            fixture.project.to_str().unwrap(),
            "--finding",
            "bundle-install-package-metadata",
            "--bundle-install-report",
            fixture.install_report.to_str().unwrap(),
            "--output",
            fixture.output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!fixture.output.join("repair_report.json").exists());
}

struct BundleInstallRepairFixture {
    project: std::path::PathBuf,
    install_report: std::path::PathBuf,
    output: std::path::PathBuf,
    _temp: tempfile::TempDir,
}

fn bundle_install_repair_fixture(
    install_result: &str,
    model_file_extra: &str,
) -> BundleInstallRepairFixture {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project.yaml");
    let installed = temp.path().join("installed_bundle");
    let artifacts = installed.join("artifacts");
    let shared = temp.path().join("shared");
    std::fs::create_dir_all(&artifacts).unwrap();
    std::fs::create_dir_all(&shared).unwrap();
    std::fs::write(artifacts.join("runtime.osdi"), "runtime artifact\n").unwrap();
    std::fs::write(installed.join("package.lock.json"), "{}\n").unwrap();
    std::fs::write(shared.join("compact_model_registry.json"), "{}\n").unwrap();
    let lock_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let registry_sha = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    std::fs::write(
        &project,
        format!(
            r#"
project:
  name: bundle_install_negative
  version: 0.1.0

board:
  components: {{}}
  nets: {{}}

scenarios:
  - name: package_import
    type: analog_transient
    checks: []
    analog:
      backend: ngspice
      netlist_source: file
      model_files:
        - path: installed_bundle/artifacts/runtime.osdi
          artifact_format: osdi_shared_object
          model_package_artifact_id: runtime_osdi
{model_file_extra}      node_bindings: []
      pin_bindings: []
      analysis:
        type: transient
        stop_time_us: 1.0
        max_step_us: 0.1
      stimuli: []
      probes: []
      assertions: []
"#
        ),
    )
    .unwrap();
    let install_report = temp.path().join("bundle_install_report.json");
    std::fs::write(
        &install_report,
        format!(
            r#"{{
  "schema_version": "circuitci.model_package_bundle_install.v1",
  "result": "{install_result}",
  "source_bundle": "bundle",
  "install_dir": "installed_bundle",
  "package": {{
    "name": "org.circuitci.models.bundle.test",
    "version": "1.2.3"
  }},
  "manifest": {{
    "path": "installed_bundle/model_package_bundle_manifest.json",
    "sha256_declared": null,
    "sha256_actual": null,
    "status": "verified"
  }},
  "lock": {{
    "path": "installed_bundle/package.lock.json",
    "sha256_declared": "{lock_sha}",
    "sha256_actual": "{lock_sha}",
    "status": "verified"
  }},
  "registry": null,
  "installed_registry": {{
    "path": "shared/compact_model_registry.json",
    "sha256_declared": "{registry_sha}",
    "sha256_actual": "{registry_sha}",
    "status": "verified"
  }},
  "scenario_import": {{
    "model_package_registry_path": "shared/compact_model_registry.json",
    "model_package_registry_sha256": "{registry_sha}",
    "model_package_registry_entry": "bundle_runtime",
    "model_package_lock_path": "installed_bundle/package.lock.json",
    "model_package_lock_sha256": "{lock_sha}",
    "model_package_artifact_id": "runtime_osdi"
  }},
  "artifacts": [
    {{
      "id": "runtime_osdi",
      "path": "installed_bundle/artifacts/runtime.osdi",
      "artifact_format": "osdi_shared_object",
      "compiler": "openvaf",
      "sha256_declared": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "sha256_actual": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "status": "verified"
    }}
  ],
  "conformance_checks": [],
  "findings": []
}}
"#
        ),
    )
    .unwrap();
    let output = temp.path().join("repair");
    BundleInstallRepairFixture {
        project,
        install_report,
        output,
        _temp: temp,
    }
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
