mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_gerber_solder_paste_appends_schema_valid_opening_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("with_gerber_solder_paste.project.yaml");
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-solder-paste",
            "examples/import_gerber_solder_paste_openings/front_paste.gtp",
            "--project",
            "examples/import_gerber_solder_paste_openings/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(command_output.status.success());
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("1 flash openings"));
    assert!(stdout.contains("0 draw openings"));
    assert!(stdout.contains("0 region openings"));
    assert!(stdout.contains("1 apertures"));

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let openings = imported["board"]["layout"]["solder_paste"]["features"]
        .as_array()
        .unwrap();
    assert_eq!(openings.len(), 1);
    assert_eq!(openings[0]["layer"], "F.Paste");
    assert_eq!(openings[0]["polarity"], "dark");
    assert_eq!(openings[0]["source_primitive"], "gerber_flash");
    assert_eq!(openings[0]["source_primitive_index"], 0);
    assert_eq!(openings[0]["aperture"], "D11");
    assert_eq!(openings[0]["shape"], "rect");
    assert_eq!(openings[0]["size"]["x_mm"], 0.9);
    assert_eq!(openings[0]["size"]["y_mm"], 0.72);
    assert_eq!(openings[0]["at"]["x_mm"], 10.0);
    assert_eq!(openings[0]["at"]["y_mm"], 10.0);
    assert_eq!(openings[0]["net"], "GND");
    assert_eq!(openings[0]["owner_kind"], "pad");
    assert_eq!(openings[0]["component"], "U1");
    assert_eq!(openings[0]["pin"], "1");

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn import_gerber_solder_paste_samples_arc_draw_openings() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let gerber = dir.path().join("arc_paste.gtp");
    let output = dir.path().join("with_arc_paste.project.yaml");
    std::fs::write(
        &gerber,
        concat!(
            "G04 Layer: F.Paste*\n",
            "%FSLAX45Y45*%\n",
            "%MOMM*%\n",
            "%ADD10C,0.200*%\n",
            "D10*\n",
            "X01000000Y01000000D02*\n",
            "G03X01100000Y01100000I0000000J0100000D01*\n",
            "M02*\n",
        ),
    )
    .unwrap();
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-solder-paste",
            gerber.to_str().unwrap(),
            "--project",
            "examples/import_gerber_solder_paste_openings/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(command_output.status.success());
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("0 flash openings"));
    assert!(stdout.contains("7 draw openings"));

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let segments = imported["board"]["layout"]["solder_paste"]["segments"]
        .as_array()
        .unwrap();
    assert_eq!(segments.len(), 7);
    assert_eq!(segments[0]["source_primitive"], "gerber_arc_draw");
    assert_eq!(segments[0]["layer"], "F.Paste");
    assert_eq!(segments[0]["width_mm"], 0.2);
    assert_eq!(segments[6]["end"]["x_mm"], 11.0);
    assert_eq!(segments[6]["end"]["y_mm"], 11.0);
}
