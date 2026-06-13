mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_excellon_drill_appends_schema_valid_drill_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let pth_output = dir.path().join("with_pth.project.yaml");
    let combined_output = dir.path().join("with_pth_npth.project.yaml");

    let pth_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-excellon-drill",
            "examples/import_jlc_excellon_drill_peer_extract/pth.drl",
            "--project",
            "examples/import_jlc_excellon_drill_peer_extract/base.project.yaml",
            "--output",
            pth_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(pth_status.success());

    let npth_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-excellon-drill",
            "examples/import_jlc_excellon_drill_peer_extract/npth.drl",
            "--project",
            pth_output.to_str().unwrap(),
            "--output",
            combined_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(npth_status.success());

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&combined_output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&combined_output).unwrap()).unwrap();
    let drills = imported["board"]["layout"]["drills"].as_array().unwrap();
    assert_eq!(drills.len(), 6);
    assert_eq!(drills[0]["plating"], "plated");
    assert_eq!(drills[0]["layer"], "PTH_Through");
    assert_eq!(drills[0]["tool"], "T01");
    assert_eq!(drills[0]["source_hit_index"], 0);
    assert_eq!(drills[0]["drill_mm"], 0.305);
    assert_eq!(drills[0]["at"]["x_mm"], 29.3);
    assert_eq!(drills[0]["at"]["y_mm"], -8.64001);
    assert_eq!(drills[2]["tool"], "T02");
    assert_eq!(drills[2]["drill_mm"], 0.6);
    assert_eq!(drills[3]["plating"], "non_plated");
    assert_eq!(drills[3]["layer"], "NPTH_Through");
    assert_eq!(drills[5]["drill_mm"], 3.30002);

    let report = run_validation(combined_output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn import_excellon_drill_associates_pad_and_via_owners() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("with_owned_drills.project.yaml");

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-excellon-drill",
            "examples/import_excellon_drill_ownership/pth.drl",
            "--project",
            "examples/import_excellon_drill_ownership/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(command_output.status.success());
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("1 pad-associated"));
    assert!(stdout.contains("1 via-associated"));

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let drills = imported["board"]["layout"]["drills"].as_array().unwrap();
    assert_eq!(drills.len(), 2);
    assert_eq!(drills[0]["owner_kind"], "pad");
    assert_eq!(drills[0]["net"], "GND");
    assert_eq!(drills[0]["component"], "J1");
    assert_eq!(drills[0]["pin"], "1");
    assert_eq!(drills[1]["owner_kind"], "via");
    assert_eq!(drills[1]["net"], "USB_DP");
    assert_eq!(drills[1]["via_index"], 0);

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == "DRILL_ANNULAR_RING_VALID")
        .unwrap();
    assert_eq!(failure["id"], "DRILL_ANNULAR_RING_VALID");
    assert_eq!(failure["measured"]["drill_owner_kind"], "pad");
    assert_eq!(failure["measured"]["drill_net"], "GND");
    assert_eq!(failure["measured"]["drill_component"], "J1");
    assert_eq!(failure["measured"]["drill_pin"], "1");
    assert_report_schema_valid(&report);
}

#[test]
fn import_excellon_drill_associates_via_layer_hits_from_copper_net_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let drill = dir.path().join("vias.drl");
    let output = dir.path().join("with_copper_owned_vias.project.yaml");
    std::fs::write(
        &drill,
        "M48\n;TYPE=PLATED\n;Layer: PTH_Through_Via\nMETRIC,LZ,3.3\nT01C0.30000\n%\nG90\nT01\nX3.0Y3.0\nM30\n",
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-excellon-drill",
            drill.to_str().unwrap(),
            "--project",
            "examples/import_excellon_drill_ownership/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(command_output.status.success());
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("0 pad-associated"));
    assert!(stdout.contains("1 via-associated"));

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let drills = imported["board"]["layout"]["drills"].as_array().unwrap();
    assert_eq!(drills.len(), 1);
    assert_eq!(drills[0]["owner_kind"], "via");
    assert_eq!(drills[0]["net"], "USB_DP");
    assert_eq!(drills[0]["layer"], "PTH_Through_Via");
    assert!(drills[0].get("via_index").is_none());
}

#[test]
fn import_excellon_drill_rejects_inch_units() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let drill = dir.path().join("bad_inches.drl");
    let output = dir.path().join("bad.project.yaml");
    std::fs::write(
        &drill,
        "M48\nINCH,LZ\nT01C0.02000\n%\nG90\nT01\nX1.0Y2.0\nM30\n",
    )
    .unwrap();
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-excellon-drill",
            drill.to_str().unwrap(),
            "--project",
            "examples/import_jlc_excellon_drill_peer_extract/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!command_output.status.success());
    let stderr = String::from_utf8_lossy(&command_output.stderr);
    assert!(stderr.contains("uses inches"));
}
