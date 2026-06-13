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

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}
