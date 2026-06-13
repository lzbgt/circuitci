mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_gerber_solder_mask_appends_schema_valid_opening_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("with_gerber_solder_mask.project.yaml");
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-solder-mask",
            "examples/import_gerber_solder_mask_openings/front_mask.gts",
            "--project",
            "examples/import_gerber_solder_mask_openings/base.project.yaml",
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
    assert!(stdout.contains("1 owner-associated flash openings"));
    assert!(stdout.contains("0 owner-associated draw openings"));
    assert!(stdout.contains("0 owner-associated region openings"));
    assert!(stdout.contains("1 apertures"));

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let openings = imported["board"]["layout"]["solder_mask"]["features"]
        .as_array()
        .unwrap();
    assert_eq!(openings.len(), 1);
    assert_eq!(openings[0]["layer"], "F.Mask");
    assert_eq!(openings[0]["polarity"], "dark");
    assert_eq!(openings[0]["source_primitive"], "gerber_flash");
    assert_eq!(openings[0]["source_primitive_index"], 0);
    assert_eq!(openings[0]["aperture"], "D11");
    assert_eq!(openings[0]["shape"], "rect");
    assert_eq!(openings[0]["size"]["x_mm"], 1.14);
    assert_eq!(openings[0]["size"]["y_mm"], 0.94);
    assert_eq!(openings[0]["at"]["x_mm"], 10.0);
    assert_eq!(openings[0]["at"]["y_mm"], 10.0);
    assert_eq!(openings[0]["net"], "GND");
    assert_eq!(openings[0]["owner_kind"], "pad");
    assert_eq!(openings[0]["component"], "J1");
    assert_eq!(openings[0]["pin"], "1");

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn import_gerber_solder_mask_accepts_easyeda_round_rect_apertures() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let gerber = dir.path().join("round_rect_mask.gts");
    let output = dir.path().join("with_round_rect_mask.project.yaml");
    std::fs::write(
        &gerber,
        concat!(
            "G04 Layer: F.Mask*\n",
            "%FSLAX45Y45*%\n",
            "%MOMM*%\n",
            "%AMRoundRect*1,1,$1,$2,$3*1,1,$1,$4,$5*%\n",
            "%ADD10RoundRect,0.1016X-0.675X-0.705X-0.675X0.705*%\n",
            "D10*\n",
            "X01000000Y01000000D03*\n",
            "M02*\n",
        ),
    )
    .unwrap();
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-solder-mask",
            gerber.to_str().unwrap(),
            "--project",
            "examples/import_gerber_solder_mask_openings/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(command_output.status.success());
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("1 owner-associated flash openings"));

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let openings = imported["board"]["layout"]["solder_mask"]["features"]
        .as_array()
        .unwrap();
    assert_eq!(openings.len(), 1);
    assert_eq!(openings[0]["shape"], "rect");
    assert_eq!(openings[0]["owner_kind"], "pad");
    assert_eq!(openings[0]["component"], "J1");
    assert_eq!(openings[0]["pin"], "1");
    let size_x = openings[0]["size"]["x_mm"].as_f64().unwrap();
    let size_y = openings[0]["size"]["y_mm"].as_f64().unwrap();
    assert!((size_x - 1.4516).abs() < 1.0e-12);
    assert!((size_y - 1.5116).abs() < 1.0e-12);
}

#[test]
fn import_gerber_solder_mask_associates_easyeda_top_mask_with_front_copper_pads() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let gerber = dir.path().join("top_mask.gts");
    let output = dir.path().join("with_easyeda_top_mask.project.yaml");
    std::fs::write(
        &gerber,
        concat!(
            "G04 Layer: TopSolderMaskLayer*\n",
            "%FSLAX45Y45*%\n",
            "%MOMM*%\n",
            "%ADD10R,1.140X0.940*%\n",
            "D10*\n",
            "X01000000Y01000000D03*\n",
            "M02*\n",
        ),
    )
    .unwrap();
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-solder-mask",
            gerber.to_str().unwrap(),
            "--project",
            "examples/import_gerber_solder_mask_openings/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(command_output.status.success());
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("1 owner-associated flash openings"));

    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let openings = imported["board"]["layout"]["solder_mask"]["features"]
        .as_array()
        .unwrap();
    assert_eq!(openings[0]["layer"], "TopSolderMaskLayer");
    assert_eq!(openings[0]["net"], "GND");
    assert_eq!(openings[0]["owner_kind"], "pad");
    assert_eq!(openings[0]["component"], "J1");
    assert_eq!(openings[0]["pin"], "1");
}

#[test]
fn import_gerber_solder_mask_associates_draw_opening_owner() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let gerber = dir.path().join("draw_mask.gts");
    let output = dir.path().join("with_draw_mask.project.yaml");
    std::fs::write(
        &gerber,
        concat!(
            "G04 Layer: F.Mask*\n",
            "%FSLAX45Y45*%\n",
            "%MOMM*%\n",
            "%ADD10C,0.300*%\n",
            "D10*\n",
            "X01000000Y01000000D02*\n",
            "X01040000Y01000000D01*\n",
            "M02*\n",
        ),
    )
    .unwrap();
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-gerber-solder-mask",
            gerber.to_str().unwrap(),
            "--project",
            "examples/import_gerber_solder_mask_openings/base.project.yaml",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(command_output.status.success());
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("1 draw openings"));
    assert!(stdout.contains("0 owner-associated flash openings"));
    assert!(stdout.contains("1 owner-associated draw openings"));
    assert!(stdout.contains("0 owner-associated region openings"));

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let segments = imported["board"]["layout"]["solder_mask"]["segments"]
        .as_array()
        .unwrap();
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0]["layer"], "F.Mask");
    assert_eq!(segments[0]["net"], "GND");
    assert_eq!(segments[0]["owner_kind"], "pad");
    assert_eq!(segments[0]["component"], "J1");
    assert_eq!(segments[0]["pin"], "1");
}
