mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_gerber_outline_generates_schema_valid_board_ir() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("with_gerber_outline.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-outline",
            "examples/import_jlc_gerber_outline_peer_extract/board_outline.gko",
            "--project",
            "examples/import_jlc_gerber_outline_peer_extract/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let segments = imported["board"]["layout"]["outline"]["segments"]
        .as_array()
        .unwrap();
    assert_eq!(segments.len(), 16);
    assert_eq!(segments[0]["layer"], "BoardOutlineLayer");
    assert_eq!(segments[0]["source_primitive"], "gerber_linear");
    assert_eq!(segments[0]["source_primitive_index"], 0);
    assert_eq!(segments[0]["boundary_role"], "external");
    assert_eq!(segments[0]["start"]["x_mm"], 0.0);
    assert_eq!(segments[0]["start"]["y_mm"], -142.0);
    assert_eq!(segments[1]["end"]["x_mm"], 120.0);
    assert_eq!(segments[4]["boundary_role"], "cutout");
    assert_eq!(segments[4]["start"]["x_mm"], 34.0);
    assert_eq!(segments[4]["start"]["y_mm"], -72.0);
    assert_eq!(
        segments
            .iter()
            .filter(|segment| segment["boundary_role"] == "external")
            .count(),
        4
    );
    assert_eq!(
        segments
            .iter()
            .filter(|segment| segment["boundary_role"] == "cutout")
            .count(),
        12
    );

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn import_gerber_outline_rejects_inch_units() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let gerber = dir.path().join("bad_inches.gko");
    let output = dir.path().join("bad.project.yaml");
    std::fs::write(
        &gerber,
        "%FSLAX45Y45*%\n%MOIN*%\nG01X0Y0D02*\nG01X100000Y0D01*\nM02*\n",
    )
    .unwrap();
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-outline",
            gerber.to_str().unwrap(),
            "--project",
            "examples/import_jlc_gerber_outline_peer_extract/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!command_output.status.success());
    let stderr = String::from_utf8_lossy(&command_output.stderr);
    assert!(stderr.contains("uses inches"));
}
