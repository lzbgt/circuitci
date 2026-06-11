mod common;

use common::{
    assert_report_schema_valid, assert_yaml_file_valid, binary_available, run_validation,
};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_kicad_schematic_generates_schema_valid_connectivity_project() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("imported_kicad_schematic.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_schematic/basic_rc.kicad_sch",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_kicad_schematic",
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
    assert_eq!(imported["project"]["import_source"], "kicad_schematic");
    assert_eq!(
        imported["board"]["components"]["R1"]["source"]["format"],
        "kicad_schematic"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["1"],
        "net_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["2"],
        "net_reset_rc"
    );
    assert_eq!(imported["board"]["components"]["C1"]["pins"]["2"], "gnd");
    assert_eq!(imported["board"]["nets"]["gnd"]["kind"], "ground");

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation["id"] == "SCHEMATIC_IMPORT_ONLY")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn import_kicad_schematic_applies_mapping_and_runs_generated_spice() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("mapped_kicad_schematic.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_schematic/basic_rc.kicad_sch",
            "--mapping",
            "examples/import_kicad_schematic/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "mapped_kicad_schematic",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["scenarios"][0]["analog"]["generated"]["components"],
        serde_json::json!(["V1", "R1", "D1", "C1"])
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["spice"]["value_ohm"],
        10000.0
    );
    assert!(
        (imported["board"]["components"]["C1"]["spice"]["value_f"]
            .as_f64()
            .unwrap()
            - 100e-9)
            .abs()
            < 1e-18
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["model_files"][0]["sha256"],
        "dee84e9189e05a9af600a0224a63cb6d01ebec4df27ff4ed12baeddd34869504"
    );

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert!(!report["waveforms"].as_array().unwrap().is_empty());
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation["id"] == "SCHEMATIC_IMPORT_ONLY")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn import_kicad_schematic_maps_mosfet_soa_scenario() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir
        .path()
        .join("mapped_kicad_schematic_mosfet.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_schematic/mosfet_soa.kicad_sch",
            "--mapping",
            "examples/import_kicad_schematic/mosfet.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "mapped_kicad_schematic_mosfet",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let analog = &imported["scenarios"][0]["analog"];
    assert_eq!(imported["project"]["import_source"], "kicad_schematic");
    assert_eq!(
        imported["board"]["components"]["M1"]["model"],
        "vendor.onsemi.fdmc86184"
    );
    assert_eq!(
        imported["board"]["components"]["M1"]["pins"]["D"],
        "net_switched"
    );
    assert_eq!(analog["operating_conditions"]["allow_pulse_ratings"], true);
    assert_eq!(
        analog["model_files"][0]["sha256"],
        "c22b2f13d52a4545933f3d97588e0d626562e4813bda3ead62f103bd64e19c01"
    );

    let validate_out = dir.path().join("validate");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            output.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            validate_out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(validate_out.join("report.json")).unwrap())
            .unwrap();
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation["id"] == "SCHEMATIC_IMPORT_ONLY")
    );
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        assert!(
            report["failures"]
                .as_array()
                .unwrap()
                .iter()
                .any(|failure| {
                    failure["id"] == "SPICE_OPERATING_LIMIT"
                        && failure["measured"]["component"] == "M1"
                        && failure["measured"]["rating"] == "SOA"
                        && failure["measured"]["soa_margin_ratio"].as_f64().unwrap() > 1.0
                        && failure["limit"]["soa_curve"] == "forward_bias_100us"
                })
        );
    } else {
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_kicad_schematic_applies_rotated_symbol_pin_coordinates() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("rotated_kicad_schematic.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_schematic/rotated_rc.kicad_sch",
            "--mapping",
            "examples/import_kicad_schematic/rotated_rc.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "rotated_kicad_schematic",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(imported["project"]["import_source"], "kicad_schematic");
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["A"],
        "net_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["B"],
        "net_reset_rc"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["generated"]["components"],
        serde_json::json!(["V1", "R1", "C1"])
    );

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert!(!report["waveforms"].as_array().unwrap().is_empty());
    assert_report_schema_valid(&report);
}

#[test]
fn import_kicad_schematic_flattens_one_child_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let child_path = dir.path().join("child.kicad_sch");
    let output = dir.path().join("hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Simulation_SPICE:VSOURCE"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Simulation_SPICE:VSOURCE") (at 0 0 0)
    (property "Reference" "V1") (property "Value" "3.3V") (pin "1") (pin "2"))
  (wire (pts (xy 0 0) (xy 20 0)))
  (label "GND" (at 0 10 0))
  (sheet (at 10 -10 0) (size 20 20)
    (property "Sheetname" "Filter")
    (property "Sheetfile" "child.kicad_sch")
    (pin "RC" input (at 20 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(
        &child_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:C"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:C") (at 0 0 0)
    (property "Reference" "C1") (property "Value" "100n") (pin "1") (pin "2"))
  (hierarchical_label "RC" (at 0 0 0))
  (label "GND" (at 0 10 0)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            root_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(imported["project"]["import_source"], "kicad_schematic");
    assert_eq!(imported["board"]["components"]["V1"]["pins"]["1"], "net_rc");
    assert_eq!(imported["board"]["components"]["C1"]["pins"]["1"], "net_rc");
    assert_eq!(imported["board"]["components"]["V1"]["pins"]["2"], "gnd");
    assert_eq!(imported["board"]["components"]["C1"]["pins"]["2"], "gnd");
}

#[test]
fn import_kicad_schematic_flattens_multiple_child_sheets() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let filter_path = dir.path().join("filter.kicad_sch");
    let sense_path = dir.path().join("sense.kicad_sch");
    let output = dir.path().join("multi_hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Simulation_SPICE:VSOURCE"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Simulation_SPICE:VSOURCE") (at 0 0 0)
    (property "Reference" "V1") (property "Value" "3.3V") (pin "1") (pin "2"))
  (symbol (lib_id "Simulation_SPICE:VSOURCE") (at 40 0 0)
    (property "Reference" "V2") (property "Value" "1.2V") (pin "1") (pin "2"))
  (wire (pts (xy 0 0) (xy 20 0)))
  (wire (pts (xy 0 10) (xy 20 10) (xy 60 10)))
  (wire (pts (xy 40 0) (xy 60 0)))
  (sheet (at 20 -10 0) (size 20 30)
    (property "Sheetname" "Filter")
    (property "Sheetfile" "filter.kicad_sch")
    (pin "FILTER_IN" input (at 20 0 0))
    (pin "GND" input (at 20 10 0)))
  (sheet (at 60 -10 0) (size 20 30)
    (property "Sheetname" "Sense")
    (property "Sheetfile" "sense.kicad_sch")
    (pin "SENSE_IN" input (at 60 0 0))
    (pin "GND" input (at 60 10 0))))
"#,
    )
    .unwrap();
    std::fs::write(
        &filter_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (hierarchical_label "FILTER_IN" (at 0 0 0))
  (hierarchical_label "GND" (at 0 10 0)))
"#,
    )
    .unwrap();
    std::fs::write(
        &sense_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R2") (property "Value" "47k") (pin "1") (pin "2"))
  (hierarchical_label "SENSE_IN" (at 0 0 0))
  (hierarchical_label "GND" (at 0 10 0)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            root_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["V1"]["pins"]["1"],
        "net_filter_in"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["1"],
        "net_filter_in"
    );
    assert_eq!(
        imported["board"]["components"]["V2"]["pins"]["1"],
        "net_sense_in"
    );
    assert_eq!(
        imported["board"]["components"]["R2"]["pins"]["1"],
        "net_sense_in"
    );
    assert_eq!(imported["board"]["components"]["R1"]["pins"]["2"], "gnd");
    assert_eq!(imported["board"]["components"]["R2"]["pins"]["2"], "gnd");
}

#[test]
fn import_kicad_schematic_hierarchy_runs_generated_spice() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("hierarchy_spice.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_hierarchy_spice/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_hierarchy_spice/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "hierarchy_spice",
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
    assert_eq!(imported["project"]["import_source"], "kicad_schematic");
    assert_eq!(
        imported["board"]["components"]["V1"]["pins"]["P"],
        "net_vin"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["A"],
        "net_vin"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["B"],
        "net_out"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["pins"]["A"],
        "net_out"
    );
    assert_eq!(imported["board"]["components"]["C1"]["pins"]["B"], "gnd");
    assert_eq!(
        imported["scenarios"][0]["analog"]["generated"]["components"],
        serde_json::json!(["V1", "R1", "C1"])
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["probes"][0]["expression"],
        "V(net_out)"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["assertions"][0]["name"],
        "child_rc_node_charges"
    );

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert!(!report["waveforms"].as_array().unwrap().is_empty());
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation["id"] == "SCHEMATIC_IMPORT_ONLY")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn import_kicad_schematic_rejects_sheet_pin_label_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let child_path = dir.path().join("child.kicad_sch");
    let output = dir.path().join("hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols)
  (sheet (at 0 0 0) (size 20 20)
    (property "Sheetname" "Filter")
    (property "Sheetfile" "child.kicad_sch")
    (pin "RC" input (at 20 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(
        &child_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:C"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:C") (at 0 0 0)
    (property "Reference" "C1") (property "Value" "100n") (pin "1"))
  (hierarchical_label "OTHER" (at 0 0 0)))
"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            root_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("do not exactly match child hierarchical labels"));
    assert!(!output.exists());
}

#[test]
fn import_kicad_schematic_rejects_nested_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let child_path = dir.path().join("child.kicad_sch");
    let grandchild_path = dir.path().join("grandchild.kicad_sch");
    let output = dir.path().join("hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols)
  (sheet (at 0 0 0) (size 20 20)
    (property "Sheetname" "Filter")
    (property "Sheetfile" "child.kicad_sch")
    (pin "RC" input (at 20 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(
        &child_path,
        r#"
(kicad_sch
  (lib_symbols)
  (sheet (at 0 0 0) (size 20 20)
    (property "Sheetname" "Nested")
    (property "Sheetfile" "grandchild.kicad_sch")
    (pin "RC" input (at 20 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(&grandchild_path, "(kicad_sch (lib_symbols))").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            root_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("does not support nested sheets yet"));
    assert!(!output.exists());
}

#[test]
fn import_kicad_schematic_rejects_duplicate_root_sheet_name() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let child_path = dir.path().join("child.kicad_sch");
    let output = dir.path().join("hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols)
  (sheet (at 0 0 0) (size 20 20)
    (property "Sheetname" "Filter")
    (property "Sheetfile" "child.kicad_sch")
    (pin "A" input (at 20 0 0)))
  (sheet (at 40 0 0) (size 20 20)
    (property "Sheetname" "Filter")
    (property "Sheetfile" "child.kicad_sch")
    (pin "B" input (at 60 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(&child_path, "(kicad_sch (lib_symbols))").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            root_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("duplicate sheet name Filter"));
    assert!(!output.exists());
}

#[test]
fn import_kicad_schematic_rejects_duplicate_sanitized_sheet_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let first_path = dir.path().join("first.kicad_sch");
    let second_path = dir.path().join("second.kicad_sch");
    let output = dir.path().join("hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols)
  (sheet (at 0 0 0) (size 20 20)
    (property "Sheetname" "A-B")
    (property "Sheetfile" "first.kicad_sch")
    (pin "A_IN" input (at 20 0 0)))
  (sheet (at 40 0 0) (size 20 20)
    (property "Sheetname" "A_B")
    (property "Sheetfile" "second.kicad_sch")
    (pin "B_IN" input (at 60 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(&first_path, "(kicad_sch (lib_symbols))").unwrap();
    std::fs::write(&second_path, "(kicad_sch (lib_symbols))").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            root_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("sanitize to the same local-net prefix a_b"));
    assert!(!output.exists());
}

#[test]
fn import_kicad_schematic_rejects_duplicate_non_ground_sheet_pin_name() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let first_path = dir.path().join("first.kicad_sch");
    let second_path = dir.path().join("second.kicad_sch");
    let output = dir.path().join("hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols)
  (sheet (at 0 0 0) (size 20 20)
    (property "Sheetname" "First")
    (property "Sheetfile" "first.kicad_sch")
    (pin "IN" input (at 20 0 0)))
  (sheet (at 40 0 0) (size 20 20)
    (property "Sheetname" "Second")
    (property "Sheetfile" "second.kicad_sch")
    (pin "IN" input (at 60 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(&first_path, "(kicad_sch (lib_symbols))").unwrap();
    std::fs::write(&second_path, "(kicad_sch (lib_symbols))").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            root_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("sheet pin IN appears on multiple root sheets"));
    assert!(!output.exists());
}

#[test]
fn import_kicad_schematic_rejects_distinct_sheet_pins_on_one_root_net() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let first_path = dir.path().join("first.kicad_sch");
    let second_path = dir.path().join("second.kicad_sch");
    let output = dir.path().join("hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "RROOT") (property "Value" "10k") (pin "1"))
  (wire (pts (xy 0 0) (xy 20 0) (xy 60 0)))
  (sheet (at 20 -10 0) (size 20 20)
    (property "Sheetname" "First")
    (property "Sheetfile" "first.kicad_sch")
    (pin "FIRST_IN" input (at 20 0 0)))
  (sheet (at 60 -10 0) (size 20 20)
    (property "Sheetname" "Second")
    (property "Sheetfile" "second.kicad_sch")
    (pin "SECOND_IN" input (at 60 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(&first_path, "(kicad_sch (lib_symbols))").unwrap();
    std::fs::write(&second_path, "(kicad_sch (lib_symbols))").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            root_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("net has conflicting labels"));
    assert!(!output.exists());
}

#[test]
fn import_kicad_schematic_rejects_duplicate_refs_across_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let child_path = dir.path().join("child.kicad_sch");
    let output = dir.path().join("hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (wire (pts (xy 0 0) (xy 20 0)))
  (sheet (at 0 10 0) (size 20 20)
    (property "Sheetname" "Filter")
    (property "Sheetfile" "child.kicad_sch")
    (pin "RC" input (at 20 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(
        &child_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:C"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:C") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "100n") (pin "1"))
  (hierarchical_label "RC" (at 0 0 0)))
"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            root_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("duplicate component reference R1"));
    assert!(!output.exists());
}

#[test]
fn import_kicad_schematic_accepts_wrapped_cardinal_rotation() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("wrapped_rotation.kicad_sch");
    let output = dir.path().join("wrapped_rotation.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 10 10 450)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (label "NET_A" (at 10 7.46 0))
  (label "NET_B" (at 10 12.54 0)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["1"],
        "net_net_a"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["2"],
        "net_net_b"
    );
}

#[test]
fn import_kicad_schematic_connects_wire_to_transformed_pin() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("rotated_wire.kicad_sch");
    let output = dir.path().join("rotated_wire.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 10 10 90)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy 10 7.46) (xy 20 7.46)))
  (label "NET_A" (at 20 7.46 0))
  (wire (pts (xy 10 12.54) (xy 20 12.54)))
  (label "NET_B" (at 20 12.54 0)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["1"],
        "net_net_a"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["2"],
        "net_net_b"
    );
}

#[test]
fn import_kicad_schematic_transforms_rotated_power_symbol_pin() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("rotated_power.kicad_sch");
    let output = dir.path().join("rotated_power.project.yaml");
    std::fs::write(
        &schematic_path,
        r##"
(kicad_sch
  (lib_symbols
    (symbol "power:+3V3"
      (pin power_in line (at -2.54 0 0) (length 2.54) (number "1")))
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "power:+3V3") (at 10 10 90)
    (property "Reference" "#PWR01") (property "Value" "+3V3") (pin "1"))
  (symbol (lib_id "Device:R") (at 12.54 7.46 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (label "LOAD" (at 15.08 7.46 0)))
"##,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["1"],
        "net_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["2"],
        "net_load"
    );
}

#[test]
fn import_kicad_schematic_rejects_unsupported_sheet() {
    assert_bad_kicad_schematic(
        r#"
(kicad_sch
  (lib_symbols)
  (sheet (at 0 0) (size 10 10) (property "Sheetname" "child")))
"#,
    );
}

#[test]
fn import_kicad_schematic_rejects_bus_wire() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols)
  (bus (pts (xy 0 0) (xy 10 0))))
"#,
        "does not support buses yet",
    );
}

#[test]
fn import_kicad_schematic_rejects_bus_entry() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols)
  (bus_entry (at 10 10) (size 2.54 2.54)))
"#,
        "does not support buses yet",
    );
}

#[test]
fn import_kicad_schematic_rejects_bus_alias() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols)
  (bus_alias "DATA" (members "D0" "D1")))
"#,
        "does not support buses yet",
    );
}

#[test]
fn import_kicad_schematic_rejects_non_cardinal_symbol_rotation() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 45)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 8.203051 8.203051 0)))
"#,
        "supports only cardinal symbol rotations",
    );
}

#[test]
fn import_kicad_schematic_rejects_wrapped_non_cardinal_symbol_rotation() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 450.1)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 10 7.46 0)))
"#,
        "supports only cardinal symbol rotations",
    );
}

#[test]
fn import_kicad_schematic_rejects_malformed_symbol_rotation() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 bad)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 7.46 10 0)))
"#,
        "malformed rotation angle",
    );
}

#[test]
fn import_kicad_schematic_rejects_non_finite_symbol_rotation() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 NaN)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 7.46 10 0)))
"#,
        "non-finite rotation angle",
    );
}

#[test]
fn import_kicad_schematic_rejects_mirrored_symbol() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 0)
    (mirror x)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 7.46 10 0)))
"#,
        "does not support mirrored symbol",
    );
}

#[test]
fn import_kicad_schematic_rejects_duplicate_refs() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (symbol (lib_id "Device:R") (at 10 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1")))
"#,
        "Duplicate KiCad schematic component reference",
    );
}

#[test]
fn import_kicad_schematic_rejects_missing_pin_geometry() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "2")))
"#,
        "has no matching lib_symbols pin geometry",
    );
}

#[test]
fn import_kicad_schematic_rejects_floating_label() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "FLOATING" (at 20 20 0)))
"#,
        "is not attached to a wire or pin",
    );
}

#[test]
fn import_kicad_schematic_rejects_label_without_name() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label (at 0 0 0)))
"#,
        "label is missing a label name",
    );
}

#[test]
fn import_kicad_schematic_rejects_label_without_coordinates() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A"))
"#,
        "label NET_A is missing valid coordinates",
    );
}

#[test]
fn import_kicad_schematic_rejects_global_label_without_name() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (global_label (at 0 0 0)))
"#,
        "global_label is missing a label name",
    );
}

#[test]
fn import_kicad_schematic_rejects_global_label_without_coordinates() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (global_label "NET_A"))
"#,
        "global_label NET_A is missing valid coordinates",
    );
}

#[test]
fn import_kicad_schematic_rejects_duplicate_label_at_same_coordinate() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 0 0 0))
  (label "NET_A" (at 0 0 0)))
"#,
        "duplicate label NET_A",
    );
}

#[test]
fn import_kicad_schematic_rejects_conflicting_labels_at_same_coordinate() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 0 0 0))
  (global_label "NET_B" (at 0 0 0)))
"#,
        "conflicting labels",
    );
}

#[test]
fn import_kicad_schematic_rejects_conflicting_labels_on_one_net_group() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 10 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy 0 0) (xy 10 0)))
  (label "NET_A" (at 0 0 0))
  (label "NET_B" (at 10 0 0)))
"#,
        "net has conflicting labels",
    );
}

#[test]
fn import_kicad_schematic_rejects_power_symbol_label_conflict() {
    assert_bad_kicad_schematic_contains(
        r##"
(kicad_sch
  (lib_symbols
    (symbol "power:+3V3"
      (pin power_in line (at 0 0 0) (length 2.54) (number "1")))
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "power:+3V3") (at 0 0 0)
    (property "Reference" "#PWR01") (property "Value" "+3V3") (pin "1"))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "OTHER" (at 0 0 0)))
"##,
        "conflicting labels",
    );
}

#[test]
fn import_kicad_schematic_rejects_duplicate_power_symbols_same_coordinate() {
    assert_bad_kicad_schematic_contains(
        r##"
(kicad_sch
  (lib_symbols
    (symbol "power:+3V3"
      (pin power_in line (at 0 0 0) (length 2.54) (number "1")))
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "power:+3V3") (at 0 0 0)
    (property "Reference" "#PWR01") (property "Value" "+3V3") (pin "1"))
  (symbol (lib_id "power:+3V3") (at 0 0 0)
    (property "Reference" "#PWR02") (property "Value" "+3V3") (pin "1"))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1")))
"##,
        "duplicate label +3V3",
    );
}

#[test]
fn import_kicad_schematic_rejects_conflicting_power_symbols_same_coordinate() {
    assert_bad_kicad_schematic_contains(
        r##"
(kicad_sch
  (lib_symbols
    (symbol "power:+3V3"
      (pin power_in line (at 0 0 0) (length 2.54) (number "1")))
    (symbol "power:+5V"
      (pin power_in line (at 0 0 0) (length 2.54) (number "1")))
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "power:+3V3") (at 0 0 0)
    (property "Reference" "#PWR01") (property "Value" "+3V3") (pin "1"))
  (symbol (lib_id "power:+5V") (at 0 0 0)
    (property "Reference" "#PWR02") (property "Value" "+5V") (pin "1"))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1")))
"##,
        "conflicting labels",
    );
}

#[test]
fn import_kicad_schematic_rejects_duplicate_explicit_power_label() {
    assert_bad_kicad_schematic_contains(
        r##"
(kicad_sch
  (lib_symbols
    (symbol "power:+3V3"
      (pin power_in line (at 0 0 0) (length 2.54) (number "1")))
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "power:+3V3") (at 0 0 0)
    (property "Reference" "#PWR01") (property "Value" "+3V3") (pin "1"))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "+3V3" (at 0 0 0)))
"##,
        "duplicate label +3V3",
    );
}

#[test]
fn import_kicad_schematic_rejects_empty_power_symbol_value() {
    assert_bad_kicad_schematic_contains(
        r##"
(kicad_sch
  (lib_symbols
    (symbol "power:+3V3"
      (pin power_in line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "power:+3V3") (at 0 0 0)
    (property "Reference" "#PWR01") (property "Value" "   ") (pin "1")))
"##,
        "power symbol #PWR01 is missing a non-empty Value label",
    );
}

#[test]
fn import_kicad_schematic_accepts_explicit_no_connect_pin() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("no_connect.kicad_sch");
    let output = dir.path().join("no_connect.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 10 10 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (label "NET_A" (at 7.46 10 0))
  (no_connect (at 12.54 10)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["1"],
        "net_net_a"
    );
    assert!(imported["board"]["components"]["R1"]["pins"]["2"].is_null());
    assert_eq!(imported["board"]["nets"].as_object().unwrap().len(), 1);
}

#[test]
fn import_kicad_schematic_rejects_unconnected_pin_without_no_connect() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 10 10 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (label "NET_A" (at 7.46 10 0)))
"#,
        "pin R1.2 is unconnected",
    );
}

#[test]
fn import_kicad_schematic_rejects_floating_no_connect_marker() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 7.46 10 0))
  (no_connect (at 20 20)))
"#,
        "no_connect marker is not attached",
    );
}

#[test]
fn import_kicad_schematic_rejects_malformed_no_connect_marker() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 7.46 10 0))
  (no_connect))
"#,
        "no_connect marker is missing valid coordinates",
    );
}

#[test]
fn import_kicad_schematic_rejects_library_no_connect_pin_without_marker() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin no_connect line (at 2.54 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 10 10 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (label "NET_A" (at 7.46 10 0)))
"#,
        "pin R1.2 is unconnected",
    );
}

#[test]
fn import_kicad_schematic_rejects_no_connect_on_connected_pin() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 7.46 10 0))
  (no_connect (at 7.46 10)))
"#,
        "no_connect marker is attached to connected pin R1.1",
    );
}

#[test]
fn import_kicad_schematic_rejects_ambiguous_no_connect_marker() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:TestPoint"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:TestPoint") (at 10 10 0)
    (property "Reference" "TP1") (property "Value" "TP") (pin "1"))
  (symbol (lib_id "Device:TestPoint") (at 10 10 0)
    (property "Reference" "TP2") (property "Value" "TP") (pin "1"))
  (no_connect (at 10 10)))
"#,
        "no_connect marker matches multiple symbol pins",
    );
}

#[test]
fn import_kicad_schematic_rejects_all_no_connect_component() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:TestPoint"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:TestPoint") (at 10 10 0)
    (property "Reference" "TP1") (property "Value" "TP") (pin "1"))
  (no_connect (at 10 10)))
"#,
        "component TP1 has no connected pins",
    );
}

#[test]
fn import_kicad_schematic_accepts_no_connect_at_transformed_open_pin() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("rotated_no_connect.kicad_sch");
    let output = dir.path().join("rotated_no_connect.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 10 10 90)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (label "NET_A" (at 10 7.46 0))
  (no_connect (at 10 12.54)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["1"],
        "net_net_a"
    );
    assert!(imported["board"]["components"]["R1"]["pins"]["2"].is_null());
}

#[test]
fn import_kicad_schematic_rejects_no_connect_at_transformed_connected_pin() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 90)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 10 7.46 0))
  (no_connect (at 10 7.46)))
"#,
        "no_connect marker is attached to connected pin R1.1",
    );
}

#[test]
fn import_kicad_schematic_rejects_no_connect_at_unrotated_old_coordinate() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 10 10 90)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (label "NET_A" (at 10 7.46 0))
  (no_connect (at 12.54 10)))
"#,
        "no_connect marker is not attached",
    );
}

#[test]
fn import_kicad_schematic_rejects_ambiguous_no_connect_after_rotation() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:TestPoint"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:TestPoint") (at 10 10 90)
    (property "Reference" "TP1") (property "Value" "TP") (pin "1"))
  (symbol (lib_id "Device:TestPoint") (at 10 4.92 270)
    (property "Reference" "TP2") (property "Value" "TP") (pin "1"))
  (no_connect (at 10 7.46)))
"#,
        "no_connect marker matches multiple symbol pins",
    );
}

#[test]
fn import_kicad_schematic_rejects_wire_crossing_without_junction() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 10 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy 0 -10) (xy 0 10)))
  (wire (pts (xy -10 0) (xy 10 0))))
"#,
        "crossing wires without an explicit junction",
    );
}

#[test]
fn import_kicad_schematic_accepts_wire_crossing_with_junction() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("junction_crossing.kicad_sch");
    let output = dir.path().join("junction_crossing.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -10 0 0) (length 2.54) (number "1"))
      (pin passive line (at 10 0 180) (length 2.54) (number "2")))
    (symbol "Device:TestPoint"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (symbol (lib_id "Device:TestPoint") (at 0 10 0)
    (property "Reference" "TP1") (property "Value" "TP") (pin "1"))
  (wire (pts (xy -10 0) (xy 10 0)))
  (wire (pts (xy 0 -10) (xy 0 10)))
  (junction (at 0 0))
  (label "NET_A" (at -10 0 0)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["1"],
        "net_net_a"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["2"],
        "net_net_a"
    );
    assert_eq!(
        imported["board"]["components"]["TP1"]["pins"]["1"],
        "net_net_a"
    );
}

#[test]
fn import_kicad_schematic_accepts_endpoint_touch_without_junction() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("endpoint_touch.kicad_sch");
    let output = dir.path().join("endpoint_touch.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -10 0 0) (length 2.54) (number "1"))
      (pin passive line (at 10 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy -10 0) (xy 0 0)))
  (wire (pts (xy 0 0) (xy 10 0)))
  (label "NET_A" (at -10 0 0)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["2"],
        "net_net_a"
    );
}

#[test]
fn import_kicad_schematic_accepts_endpoint_to_midspan_t_touch_without_junction() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("endpoint_midspan_touch.kicad_sch");
    let output = dir.path().join("endpoint_midspan_touch.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -10 0 0) (length 2.54) (number "1"))
      (pin passive line (at 10 0 180) (length 2.54) (number "2")))
    (symbol "Device:TestPoint"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (symbol (lib_id "Device:TestPoint") (at 0 10 0)
    (property "Reference" "TP1") (property "Value" "TP") (pin "1"))
  (wire (pts (xy -10 0) (xy 10 0)))
  (wire (pts (xy 0 0) (xy 0 10)))
  (label "NET_A" (at -10 0 0)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["TP1"]["pins"]["1"],
        "net_net_a"
    );
}

#[test]
fn import_kicad_schematic_accepts_corner_junction() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("corner_junction.kicad_sch");
    let output = dir.path().join("corner_junction.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -10 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 270) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy -10 0) (xy 0 0)))
  (wire (pts (xy 0 0) (xy 0 10)))
  (junction (at 0 0))
  (label "NET_A" (at -10 0 0)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["2"],
        "net_net_a"
    );
}

#[test]
fn import_kicad_schematic_accepts_collinear_overlap_junction() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("collinear_junction.kicad_sch");
    let output = dir.path().join("collinear_junction.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -10 0 0) (length 2.54) (number "1"))
      (pin passive line (at 20 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy -10 0) (xy 10 0)))
  (wire (pts (xy 0 0) (xy 20 0)))
  (junction (at 0 0))
  (label "NET_A" (at -10 0 0)))
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["2"],
        "net_net_a"
    );
}

#[test]
fn import_kicad_schematic_rejects_malformed_junction() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -10 0 0) (length 2.54) (number "1"))
      (pin passive line (at 10 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy -10 0) (xy 10 0)))
  (label "NET_A" (at -10 0 0))
  (junction))
"#,
        "junction is missing valid coordinates",
    );
}

#[test]
fn import_kicad_schematic_rejects_duplicate_junction() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -10 0 0) (length 2.54) (number "1"))
      (pin passive line (at 10 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy -10 0) (xy 10 0)))
  (wire (pts (xy 0 -10) (xy 0 10)))
  (junction (at 0 0))
  (junction (at 0 0))
  (label "NET_A" (at -10 0 0)))
"#,
        "duplicate junction",
    );
}

#[test]
fn import_kicad_schematic_rejects_floating_junction() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -10 0 0) (length 2.54) (number "1"))
      (pin passive line (at 10 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy -10 0) (xy 10 0)))
  (label "NET_A" (at -10 0 0))
  (junction (at 20 20)))
"#,
        "junction is not attached to any wire",
    );
}

#[test]
fn import_kicad_schematic_rejects_one_segment_junction() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -10 0 0) (length 2.54) (number "1"))
      (pin passive line (at 10 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy -10 0) (xy 10 0)))
  (label "NET_A" (at -10 0 0))
  (junction (at 0 0)))
"#,
        "junction touches only one wire segment",
    );
}

fn assert_bad_kicad_schematic(schematic: &str) {
    let output = bad_kicad_schematic_output(schematic);
    assert!(!output.status.success());
}

fn assert_bad_kicad_schematic_contains(schematic: &str, expected: &str) {
    let output = bad_kicad_schematic_output(schematic);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected stderr to contain {expected:?}, got:\n{stderr}"
    );
}

fn bad_kicad_schematic_output(schematic: &str) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("bad.kicad_sch");
    let output = dir.path().join("bad.project.yaml");
    std::fs::write(&schematic_path, schematic).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            schematic_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.exists());
    result
}
