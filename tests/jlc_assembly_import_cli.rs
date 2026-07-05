mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid, run_validation};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_jlc_assembly_generates_schema_valid_board_ir() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("imported_jlc_assembly.project.yaml");
    let manifest_output = output.with_extension("json");
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
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
        .output()
        .unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("manifest"));

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
        imported["board"]["components"]["C1"]["source"]["placement_layer"],
        "T"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["source"]["placement_side"],
        "top"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["source"]["placement_side_confidence"],
        "source_explicit"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["source"]["placement_rotation_raw"],
        "90"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["source"]["placement_rotation_deg"],
        90.0
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["source"]["placement_orientation_confidence"],
        "source_explicit"
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

    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    let manifest_schema: Value =
        serde_json::from_str(include_str!("../schemas/jlc_assembly_import.schema.json")).unwrap();
    let manifest_validator = jsonschema::validator_for(&manifest_schema).unwrap();
    if let Err(error) = manifest_validator.validate(&manifest) {
        panic!("JLC/EasyEDA assembly manifest failed schema validation: {error}");
    }
    assert_eq!(manifest["schema_version"], "0.3.0");
    assert_eq!(
        manifest["sources"]["bom"]["sha256"].as_str().unwrap().len(),
        64
    );
    assert_eq!(
        manifest["sources"]["bom"]["columns"],
        serde_json::json!([
            "No.",
            "Quantity",
            "Comment",
            "Designator",
            "Footprint",
            "Value",
            "Manufacturer Part",
            "Manufacturer",
            "Supplier Part",
            "Supplier",
            "LCSC Price",
            "JLCPCB Price"
        ])
    );
    assert_eq!(
        manifest["sources"]["placement"]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(manifest["import"]["components"], 4);
    assert_eq!(manifest["import"]["bom_rows"], 3);
    assert_eq!(manifest["import"]["placements"], 4);
    assert_eq!(manifest["bom_rows"][0]["row_number"], 2);
    assert_eq!(
        manifest["bom_rows"][0]["row_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        manifest["bom_rows"][0]["designators"],
        serde_json::json!(["C1", "C3"])
    );
    assert_eq!(manifest["bom_rows"][0]["fields"]["supplier_part"], "C1713");
    assert_eq!(manifest["placement_rows"][0]["row_number"], 3);
    assert_eq!(
        manifest["placement_rows"][0]["row_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(manifest["placement_rows"][0]["designator"], "C1");
    assert_eq!(manifest["placement_rows"][0]["fields"]["layer"], "T");
    assert_eq!(manifest["placement_rows"][0]["fields"]["side"], "top");
    assert_eq!(
        manifest["placement_rows"][0]["fields"]["side_confidence"],
        "source_explicit"
    );
    assert_eq!(
        manifest["placement_rows"][0]["fields"]["raw_rotation"],
        "90"
    );
    assert_eq!(
        manifest["placement_rows"][0]["fields"]["orientation_confidence"],
        "source_explicit"
    );
    let components = manifest["components"].as_array().unwrap();
    let u1 = components
        .iter()
        .find(|component| component["designator"] == "U1")
        .unwrap();
    assert_eq!(u1["has_bom"], true);
    assert_eq!(u1["has_placement"], true);
    assert_eq!(u1["part_number"], "TPS63802DLAR");
    assert_eq!(u1["placement_row"], 5);
    assert_eq!(u1["layer"], "T");
    assert_eq!(u1["side"], "top");
    assert_eq!(u1["side_confidence"], "source_explicit");
    assert_eq!(u1["raw_rotation"], "180");
    assert_eq!(u1["orientation_confidence"], "source_explicit");
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

#[test]
fn import_jlc_assembly_marks_uncertain_side_and_rotation_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let bom = dir.path().join("bom.csv");
    let placement = dir.path().join("placement.csv");
    let output = dir.path().join("imported_uncertain.project.yaml");
    let manifest_output = output.with_extension("json");
    std::fs::write(
        &bom,
        "Designator,Quantity,Comment,Manufacturer Part\nJ1,1,USB,USB-C-CONN\n",
    )
    .unwrap();
    std::fs::write(
        &placement,
        "Designator,Mid X,Mid Y,Layer,Rotation\nJ1,1mm,2mm,Inner,\n",
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
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
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    let fields = &manifest["placement_rows"][0]["fields"];
    assert_eq!(fields["layer"], "Inner");
    assert!(fields["side"].is_null());
    assert_eq!(fields["side_confidence"], "unrecognized_source_value");
    assert!(fields["raw_rotation"].is_null());
    assert!(fields["rotation_deg"].is_null());
    assert_eq!(fields["orientation_confidence"], "missing_source_value");
    assert_eq!(
        manifest["components"][0]["side_confidence"],
        "unrecognized_source_value"
    );
    assert_eq!(
        manifest["components"][0]["orientation_confidence"],
        "missing_source_value"
    );
}
