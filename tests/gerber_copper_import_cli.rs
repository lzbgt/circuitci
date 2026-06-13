mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_gerber_copper_appends_schema_valid_flash_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("with_gerber_copper.project.yaml");
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-copper",
            "examples/import_jlc_gerber_copper_peer_extract/front_copper.gtl",
            "--project",
            "examples/import_jlc_gerber_copper_peer_extract/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(command_output.status.success());
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("3 flash features"));
    assert!(stdout.contains("3 apertures"));
    assert!(stdout.contains("1 ignored draw records"));
    assert!(stdout.contains("1 skipped clear flashes"));

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let features = imported["board"]["layout"]["copper"]["features"]
        .as_array()
        .unwrap();
    assert_eq!(features.len(), 3);
    assert_eq!(features[0]["layer"], "F.Cu");
    assert_eq!(features[0]["polarity"], "dark");
    assert_eq!(features[0]["source_primitive"], "gerber_flash");
    assert_eq!(features[0]["source_primitive_index"], 0);
    assert_eq!(features[0]["aperture"], "D10");
    assert_eq!(features[0]["shape"], "circle");
    assert_eq!(features[0]["size"]["x_mm"], 0.6);
    assert_eq!(features[0]["size"]["y_mm"], 0.6);
    assert_eq!(features[0]["at"]["x_mm"], 29.3);
    assert_eq!(features[0]["at"]["y_mm"], -8.64001);
    assert_eq!(features[1]["shape"], "rect");
    assert_eq!(features[1]["size"]["x_mm"], 1.2);
    assert_eq!(features[1]["size"]["y_mm"], 0.8);
    assert_eq!(features[2]["shape"], "oval");
    assert_eq!(features[2]["size"]["x_mm"], 1.5);
    assert_eq!(features[2]["size"]["y_mm"], 0.7);

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn import_gerber_copper_rejects_undefined_aperture_selection() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let gerber = dir.path().join("undefined_aperture.gtl");
    let output = dir.path().join("bad.project.yaml");
    std::fs::write(
        &gerber,
        "%FSLAX45Y45*%\n%MOMM*%\nD10*\nX00000000Y00000000D03*\nM02*\n",
    )
    .unwrap();
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-copper",
            gerber.to_str().unwrap(),
            "--project",
            "examples/import_jlc_gerber_copper_peer_extract/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!command_output.status.success());
    let stderr = String::from_utf8_lossy(&command_output.stderr);
    assert!(stderr.contains("selects undefined aperture D10"));
}

#[test]
fn import_gerber_copper_rejects_unsupported_macro_apertures() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let gerber = dir.path().join("unsupported_macro.gtl");
    let output = dir.path().join("bad.project.yaml");
    std::fs::write(
        &gerber,
        "%FSLAX45Y45*%\n%MOMM*%\n%ADD10P,1.0X5*%\nD10*\nX00000000Y00000000D03*\nM02*\n",
    )
    .unwrap();
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-copper",
            gerber.to_str().unwrap(),
            "--project",
            "examples/import_jlc_gerber_copper_peer_extract/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!command_output.status.success());
    let stderr = String::from_utf8_lossy(&command_output.stderr);
    assert!(stderr.contains("unsupported aperture shape P"));
}
