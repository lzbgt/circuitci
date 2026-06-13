mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_easyeda_flying_probe_adds_schema_valid_pad_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("with_flying_probe.project.yaml");
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-easyeda-flying-probe",
            "examples/import_easyeda_flying_probe_pads/FlyingProbeTesting.json",
            "--project",
            "examples/import_easyeda_flying_probe_pads/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(command_output.status.success());
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("6 pin rows"));
    assert!(stdout.contains("4 connected pin rows"));
    assert!(stdout.contains("4 pads imported"));
    assert!(stdout.contains("1 duplicate pin rows"));
    assert!(stdout.contains("0 multipart pin rows"));
    assert!(stdout.contains("1 unconnected pins skipped"));
    assert!(stdout.contains("1 components created"));
    assert!(stdout.contains("4 nets imported"));

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let j1_pad_1 = &imported["board"]["layout"]["pads"]["J1"]["1"];
    assert_eq!(j1_pad_1["net"], "GND");
    assert_eq!(j1_pad_1["layers"], serde_json::json!(["F.Cu"]));
    assert_eq!(j1_pad_1["kind"], "smd");
    assert_eq!(j1_pad_1["shape"], "rect");
    assert!((j1_pad_1["at"]["x_mm"].as_f64().unwrap() - 25.4).abs() < 1.0e-12);
    assert!((j1_pad_1["size"]["y_mm"].as_f64().unwrap() - 0.508).abs() < 1.0e-12);
    let j1_pad_2 = &imported["board"]["layout"]["pads"]["J1"]["2"];
    assert_eq!(j1_pad_2["layers"], serde_json::json!(["F.Cu", "B.Cu"]));
    assert_eq!(j1_pad_2["kind"], "through_hole");
    assert_eq!(j1_pad_2["shape"], "circle");
    assert!((j1_pad_2["drill_mm"].as_f64().unwrap() - 0.508).abs() < 1.0e-12);
    let u2_pad_1 = &imported["board"]["layout"]["pads"]["U2"]["1"];
    assert_eq!(u2_pad_1["net"], "BOTTOM_NET");
    assert_eq!(u2_pad_1["layers"], serde_json::json!(["B.Cu"]));
    assert_eq!(
        imported["board"]["components"]["TP1"]["source"]["format"],
        "easyeda_flying_probe"
    );
    assert_eq!(
        imported["board"]["nets"]["GND"],
        serde_json::json!({ "kind": "ground" })
    );
    assert_eq!(
        imported["board"]["nets"]["TEST_NET"],
        serde_json::json!({ "kind": "digital_or_analog" })
    );

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn import_easyeda_flying_probe_rejects_conflicting_duplicate_pin_rows() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let input = dir.path().join("bad_flying_probe.json");
    let output = dir.path().join("bad.project.yaml");
    std::fs::write(
        &input,
        r#"{
  "lengthUnit": "mil",
  "pins": {
    "fields": ["PIN_NAME", "PIN_X", "PIN_Y", "LAYER", "PIN_TYPE", "NET_NAME", "PAD_SHAPE", "PAD_SIZEX", "PAD_SIZEY", "HOLE_SIZE", "PAD_ANGLE"],
    "rows": [
      ["J1_1", "1000", "-500", "T", "SMD", "GND", "R", "40", "20", "0", "0"],
      ["J1_1", "1000", "-500", "T", "SMD", "VBUS", "R", "40", "20", "0", "0"]
    ]
  }
}"#,
    )
    .unwrap();
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-easyeda-flying-probe",
            input.to_str().unwrap(),
            "--project",
            "examples/import_easyeda_flying_probe_pads/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!command_output.status.success());
    let stderr = String::from_utf8_lossy(&command_output.stderr);
    assert!(stderr.contains("conflicts"));
}
