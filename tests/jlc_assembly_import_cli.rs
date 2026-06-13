mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_jlc_assembly_generates_schema_valid_board_ir() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("imported_jlc_assembly.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-jlc-assembly",
            "--bom",
            "examples/import_jlc_assembly_peer_extract/bom.csv",
            "--placement",
            "examples/import_jlc_assembly_peer_extract/placement.csv",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_jlc_assembly_peer_extract",
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
    assert_eq!(imported["project"]["import_source"], "jlc_assembly");
    assert_eq!(
        imported["board"]["components"]["C1"]["source"]["bom_designator_group"],
        "C1,C3"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["source"]["bom_quantity"],
        2
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["source"]["supplier_part"],
        "C1713"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["part_number"],
        "TPS63802DLAR"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["source"]["placement_pins"],
        10
    );
    assert_eq!(
        imported["board"]["layout"]["placements"]["C1"]["side"],
        "top"
    );
    assert_eq!(
        imported["board"]["layout"]["placements"]["C1"]["rotation_deg"],
        90.0
    );
    assert_eq!(
        imported["board"]["layout"]["placements"]["C3"]["x_mm"],
        20.7
    );
    assert_eq!(
        imported["board"]["layout"]["placements"]["U1"]["y_mm"],
        -8.4
    );

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
}

#[test]
fn import_jlc_assembly_rejects_quantity_designator_mismatch() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let bom = dir.path().join("bad_bom.csv");
    let placement = dir.path().join("placement.csv");
    std::fs::write(
        &bom,
        "Quantity,Designator,Manufacturer Part\n2,\"R1,R2,R3\",RC0402\n",
    )
    .unwrap();
    std::fs::write(
        &placement,
        "Designator,Mid X,Mid Y,Layer,Rotation\nR1,1mm,2mm,T,0\n",
    )
    .unwrap();
    let output = dir.path().join("bad.project.yaml");
    let output_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-jlc-assembly",
            "--bom",
            bom.to_str().unwrap(),
            "--placement",
            placement.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output_status.status.success());
    let stderr = String::from_utf8_lossy(&output_status.stderr);
    assert!(stderr.contains("quantity 2 does not match 3 designators"));
}
