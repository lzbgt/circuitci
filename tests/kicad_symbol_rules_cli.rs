mod common;

use common::assert_bad_kicad_schematic_contains;
use serde_json::Value;
use std::process::Command;

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
        "duplicate unit 1",
    );
}

#[test]
fn import_kicad_schematic_skips_on_board_no_symbol() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("on_board_no.kicad_sch");
    let output = dir.path().join("on_board_no.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0) (on_board no)
    (property "Reference" "R_SKIP") (property "Value" "10k") (pin "1") (pin "2"))
  (symbol (lib_id "Device:R") (at 20 0 0)
    (property "Reference" "R_KEEP") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy 0 0) (xy 20 0)))
  (label "KEEP_A" (at 10 0 0))
  (wire (pts (xy 0 10) (xy 20 10)))
  (label "KEEP_B" (at 10 10 0)))
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
    assert!(imported["board"]["components"]["R_SKIP"].is_null());
    assert_eq!(
        imported["board"]["components"]["R_KEEP"]["pins"]["1"],
        "net_keep_a"
    );
    assert_eq!(
        imported["board"]["components"]["R_KEEP"]["pins"]["2"],
        "net_keep_b"
    );
}

#[test]
fn import_kicad_schematic_rejects_malformed_on_board_token() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0) (on_board maybe)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 0 0 0)))
"#,
        "on_board must be yes or no",
    );
}

#[test]
fn import_kicad_schematic_preserves_in_bom_and_instance_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("symbol_metadata.kicad_sch");
    let output = dir.path().join("symbol_metadata.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 10 10 0) (unit 1) (in_bom no)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2")
    (instances
      (project "demo"
        (path "/11111111-1111-1111-1111-111111111111"
          (reference "R1")
          (unit 1)))))
  (label "NET_A" (at 7.46 10 0))
  (label "NET_B" (at 12.54 10 0)))
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
        imported["board"]["components"]["R1"]["source"]["in_bom"],
        false
    );
    assert_eq!(imported["board"]["components"]["R1"]["source"]["unit"], 1);
    assert_eq!(
        imported["board"]["components"]["R1"]["source"]["instances"][0]["project"],
        "demo"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["source"]["instances"][0]["path"],
        "/11111111-1111-1111-1111-111111111111"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["source"]["instances"][0]["reference"],
        "R1"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["source"]["instances"][0]["unit"],
        1
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["source"]["kicad_pin_electrical_types"]["1"],
        "passive"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["source"]["kicad_pin_electrical_types"]["2"],
        "passive"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["source"]["board_pin_electrical_types"]["1"],
        "passive"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["source"]["board_pin_electrical_types"]["2"],
        "passive"
    );
}

#[test]
fn import_kicad_schematic_rejects_malformed_in_bom_token() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0) (in_bom maybe)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 0 0 0)))
"#,
        "in_bom must be yes or no",
    );
}

#[test]
fn import_kicad_schematic_rejects_instance_reference_mismatch() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1")
    (instances
      (project "demo"
        (path "/11111111-1111-1111-1111-111111111111"
          (reference "R2")
          (unit 1)))))
  (label "NET_A" (at 0 0 0)))
"#,
        "references R2",
    );
}

#[test]
fn import_kicad_schematic_rejects_instance_unit_mismatch() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0) (unit 1)
    (property "Reference" "R1") (property "Value" "10k") (pin "1")
    (instances
      (project "demo"
        (path "/11111111-1111-1111-1111-111111111111"
          (reference "R1")
          (unit 2)))))
  (label "NET_A" (at 0 0 0)))
"#,
        "does not match symbol unit",
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
fn import_kicad_schematic_uses_extended_symbol_pin_geometry() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("extended_symbol.kicad_sch");
    let output = dir.path().join("extended_symbol.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:BaseR"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (number "2")))
    (symbol "Device:AliasR"
      (extends "Device:BaseR")
      (property "Value" "alias")))
  (symbol (lib_id "Device:AliasR") (at 10 10 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (label "NET_A" (at 7.46 10 0))
  (label "NET_B" (at 12.54 10 0)))
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
fn import_kicad_schematic_rejects_missing_extended_symbol_base() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:AliasR"
      (extends "Device:MissingR")))
  (symbol (lib_id "Device:AliasR") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2")))
"#,
        "extends missing base",
    );
}

#[test]
fn import_kicad_schematic_rejects_extended_symbol_with_pins() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:BaseR"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (number "2")))
    (symbol "Device:AliasR"
      (extends "Device:BaseR")
      (pin passive line (at 0 0 0) (length 2.54) (number "3"))))
  (symbol (lib_id "Device:AliasR") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2")))
"#,
        "extends another symbol and cannot declare pins",
    );
}

#[test]
fn import_kicad_schematic_rejects_extended_symbol_cycle() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:A" (extends "Device:B"))
    (symbol "Device:B" (extends "Device:A")))
  (symbol (lib_id "Device:A") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2")))
"#,
        "inheritance cycle",
    );
}

#[test]
fn import_kicad_schematic_selects_multi_unit_pin_geometry() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("multi_unit.kicad_sch");
    let output = dir.path().join("multi_unit.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:DualUnit"
      (symbol "Device:DualUnit_0_1"
        (pin power_in line (at 0 -10 90) (length 2.54) (number "8")))
      (symbol "Device:DualUnit_1_1"
        (pin passive line (at -5 0 0) (length 2.54) (number "1"))
        (pin passive line (at 5 0 180) (length 2.54) (number "2")))
      (symbol "Device:DualUnit_2_1"
        (pin passive line (at -5 10 0) (length 2.54) (number "3"))
        (pin passive line (at 5 10 180) (length 2.54) (number "4")))))
  (symbol (lib_id "Device:DualUnit") (at 10 10 0) (unit 2)
    (property "Reference" "U1") (property "Value" "DualUnit")
    (pin "3") (pin "4") (pin "8"))
  (label "IN_B" (at 5 20 0))
  (label "OUT_B" (at 15 20 0))
  (label "VCC" (at 10 0 0)))
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
        imported["board"]["components"]["U1"]["pins"]["3"],
        "net_in_b"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["pins"]["4"],
        "net_out_b"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["pins"]["8"],
        "net_vcc"
    );
}

#[test]
fn import_kicad_schematic_merges_multi_unit_package() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("merged_multi_unit.kicad_sch");
    let output = dir.path().join("merged_multi_unit.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:DualUnit"
      (symbol "Device:DualUnit_0_1"
        (pin power_in line (at 0 -10 90) (length 2.54) hide (name "VCC") (number "8")))
      (symbol "Device:DualUnit_1_1"
        (pin passive line (at -5 0 0) (length 2.54) (number "1"))
        (pin passive line (at 5 0 180) (length 2.54) (number "2")))
      (symbol "Device:DualUnit_2_1"
        (pin passive line (at -5 10 0) (length 2.54) (number "3"))
        (pin passive line (at 5 10 180) (length 2.54) (number "4")))))
  (symbol (lib_id "Device:DualUnit") (at 10 10 0) (unit 1)
    (property "Reference" "U1") (property "Value" "DualUnit") (pin "1") (pin "2"))
  (symbol (lib_id "Device:DualUnit") (at 30 10 0) (unit 2)
    (property "Reference" "U1") (property "Value" "DualUnit") (pin "3") (pin "4"))
  (label "IN_A" (at 5 10 0))
  (label "OUT_A" (at 15 10 0))
  (label "IN_B" (at 25 20 0))
  (label "OUT_B" (at 35 20 0)))
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
    let component = &imported["board"]["components"]["U1"];
    assert_eq!(component["pins"]["1"], "net_in_a");
    assert_eq!(component["pins"]["2"], "net_out_a");
    assert_eq!(component["pins"]["3"], "net_in_b");
    assert_eq!(component["pins"]["4"], "net_out_b");
    assert_eq!(component["pins"]["8"], "net_vcc");
    assert!(component["source"]["unit"].is_null());
    assert_eq!(component["source"]["units"][0], 1);
    assert_eq!(component["source"]["units"][1], 2);
}

#[test]
fn import_kicad_schematic_rejects_multi_unit_package_metadata_conflict() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:DualUnit"
      (symbol "Device:DualUnit_1_1"
        (pin passive line (at -5 0 0) (length 2.54) (number "1")))
      (symbol "Device:DualUnit_2_1"
        (pin passive line (at -5 10 0) (length 2.54) (number "2")))))
  (symbol (lib_id "Device:DualUnit") (at 10 10 0) (unit 1)
    (property "Reference" "U1") (property "Value" "A") (pin "1"))
  (symbol (lib_id "Device:DualUnit") (at 30 10 0) (unit 2)
    (property "Reference" "U1") (property "Value" "B") (pin "2"))
  (label "IN_A" (at 5 10 0))
  (label "IN_B" (at 25 20 0)))
"#,
        "conflicting package metadata",
    );
}

#[test]
fn import_kicad_schematic_rejects_multi_unit_common_pin_on_different_nets() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:DualUnit"
      (symbol "Device:DualUnit_0_1"
        (pin passive line (at 0 -10 90) (length 2.54) (number "8")))
      (symbol "Device:DualUnit_1_1"
        (pin passive line (at -5 0 0) (length 2.54) (number "1")))
      (symbol "Device:DualUnit_2_1"
        (pin passive line (at -5 10 0) (length 2.54) (number "2")))))
  (symbol (lib_id "Device:DualUnit") (at 10 10 0) (unit 1)
    (property "Reference" "U1") (property "Value" "DualUnit") (pin "1") (pin "8"))
  (symbol (lib_id "Device:DualUnit") (at 30 10 0) (unit 2)
    (property "Reference" "U1") (property "Value" "DualUnit") (pin "2") (pin "8"))
  (label "IN_A" (at 5 10 0))
  (label "IN_B" (at 25 20 0))
  (label "VCCA" (at 10 0 0))
  (label "VCCB" (at 30 0 0)))
"#,
        "U1.8 appears on more than one net",
    );
}

#[test]
fn import_kicad_schematic_rejects_missing_multi_unit_geometry() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:DualUnit"
      (symbol "Device:DualUnit_1_1"
        (pin passive line (at -5 0 0) (length 2.54) (number "1")))))
  (symbol (lib_id "Device:DualUnit") (at 0 0 0) (unit 2)
    (property "Reference" "U1") (property "Value" "DualUnit") (pin "1"))
  (label "NET_A" (at -5 0 0)))
"#,
        "selects unit 2",
    );
}

#[test]
fn import_kicad_schematic_rejects_duplicate_multi_unit_pin_geometry() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:DualUnit"
      (symbol "Device:DualUnit_1_1"
        (pin passive line (at -5 0 0) (length 2.54) (number "1"))
        (pin passive line (at 5 0 180) (length 2.54) (number "1")))))
  (symbol (lib_id "Device:DualUnit") (at 0 0 0) (unit 1)
    (property "Reference" "U1") (property "Value" "DualUnit") (pin "1"))
  (label "NET_A" (at -5 0 0)))
"#,
        "duplicate pin geometry",
    );
}

#[test]
fn import_kicad_schematic_imports_hidden_power_pin_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("hidden_power.kicad_sch");
    let output = dir.path().join("hidden_power.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:HiddenPower"
      (pin input line (at -5 0 0) (length 2.54) (name "IN") (number "1"))
      (pin output line (at 5 0 180) (length 2.54) (name "OUT") (number "2"))
      (pin power_in line (at 0 -5 90) (length 2.54) hide (name "VCC") (number "8"))))
  (symbol (lib_id "Device:HiddenPower") (at 10 10 0)
    (property "Reference" "U1") (property "Value" "HiddenPower")
    (pin "1") (pin "2"))
  (label "IN" (at 5 10 0))
  (label "OUT" (at 15 10 0)))
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
    assert_eq!(imported["board"]["components"]["U1"]["pins"]["1"], "net_in");
    assert_eq!(
        imported["board"]["components"]["U1"]["pins"]["2"],
        "net_out"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["pins"]["8"],
        "net_vcc"
    );
}

#[test]
fn import_kicad_schematic_rejects_hidden_non_power_pin() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:HiddenBad"
      (pin input line (at 0 0 0) (length 2.54) hide (name "IN") (number "1"))))
  (symbol (lib_id "Device:HiddenBad") (at 0 0 0)
    (property "Reference" "U1") (property "Value" "HiddenBad") (pin "1"))
  (label "IN" (at 0 0 0)))
"#,
        "hidden but has unsupported electrical type input",
    );
}

#[test]
fn import_kicad_schematic_rejects_hidden_power_pin_without_name() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:HiddenBad"
      (pin power_in line (at 0 0 0) (length 2.54) hide (number "8"))))
  (symbol (lib_id "Device:HiddenBad") (at 0 0 0)
    (property "Reference" "U1") (property "Value" "HiddenBad") (pin "8"))
  (label "VCC" (at 0 0 0)))
"#,
        "hidden power pin 8 is missing a name",
    );
}
