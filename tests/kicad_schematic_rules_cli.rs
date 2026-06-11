mod common;

use serde_json::Value;
use std::process::Command;

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
fn import_kicad_schematic_accepts_labelled_bus_graphics() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("bus_graphics.kicad_sch");
    let output = dir.path().join("bus_graphics.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy 0 0) (xy 10 0)))
  (label "DATA7" (at 5 0 0))
  (bus_entry (at 10 0) (size 2.54 -2.54))
  (bus (pts (xy 12.54 -2.54) (xy 20 -2.54)))
  (bus_alias "DATA" (members "DATA[0..7]"))
  (no_connect (at 0 10)))
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
        "net_data7"
    );
}

#[test]
fn import_kicad_schematic_resolves_scalar_label_on_bus_graphic() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("bus_label_graphic.kicad_sch");
    let output = dir.path().join("bus_label_graphic.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy 0 0) (xy 10 0)))
  (bus_entry (at 10 0) (size 2.54 -2.54))
  (bus (pts (xy 12.54 -2.54) (xy 20 -2.54)))
  (label "DATA3" (at 15 -2.54 0))
  (bus_alias "DATA" (members "DATA[0..7]"))
  (no_connect (at 0 10)))
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
        "net_data3"
    );
}

#[test]
fn import_kicad_schematic_rejects_unlabelled_bus_entry_stub() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy 0 0) (xy 10 0)))
  (bus_entry (at 10 0) (size 2.54 -2.54))
  (bus (pts (xy 12.54 -2.54) (xy 20 -2.54)))
  (no_connect (at 0 10)))
"#,
        "requires an explicit scalar label",
    );
}

#[test]
fn import_kicad_schematic_rejects_ambiguous_bus_graphic_label() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy 0 0) (xy 10 0)))
  (bus_entry (at 10 0) (size 2.54 -2.54))
  (bus (pts (xy 12.54 -2.54) (xy 20 -2.54)))
  (label "DATA" (at 15 -2.54 0))
  (bus_alias "DATA" (members "DATA[0..7]"))
  (no_connect (at 0 10)))
"#,
        "ambiguous across bus members",
    );
}

#[test]
fn import_kicad_schematic_rejects_bus_entry_label_outside_alias() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (wire (pts (xy 0 0) (xy 10 0)))
  (label "ADDR0" (at 5 0 0))
  (bus_entry (at 10 0) (size 2.54 -2.54))
  (bus (pts (xy 12.54 -2.54) (xy 20 -2.54)))
  (bus_alias "DATA" (members "DATA[0..7]"))
  (no_connect (at 0 10)))
"#,
        "is not declared by any bus_alias member",
    );
}

#[test]
fn import_kicad_schematic_rejects_malformed_bus_alias_range() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 0 0 0))
  (bus_alias "DATA" (members "DATA[0..]")))
"#,
        "range bounds must be decimal",
    );
}

#[test]
fn import_kicad_schematic_rejects_descending_bus_alias_range() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 0 0 0))
  (bus_alias "DATA" (members "DATA[7..0]")))
"#,
        "descending ranges are unsupported",
    );
}

#[test]
fn import_kicad_schematic_rejects_duplicate_bus_alias_member() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 0 0 0))
  (bus_alias "DATA" (members "DATA0" "DATA0")))
"#,
        "duplicate member DATA0",
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
fn import_kicad_schematic_transforms_mirrored_symbol_pin() {
    let dir = tempfile::tempdir().unwrap();
    let schematic_path = dir.path().join("mirrored_symbol.kicad_sch");
    let output = dir.path().join("mirrored_symbol.project.yaml");
    std::fs::write(
        &schematic_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 -2.54 0) (length 2.54) (number "1"))
      (pin passive line (at 2.54 2.54 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 10 10 0)
    (mirror x)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (label "NET_A" (at 7.46 12.54 0))
  (label "NET_B" (at 12.54 7.46 0)))
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
fn import_kicad_schematic_rejects_unsupported_mirror_axis() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 0)
    (mirror z)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 7.46 10 0)))
"#,
        "unsupported mirror axis",
    );
}

#[test]
fn import_kicad_schematic_rejects_malformed_mirror_token() {
    assert_bad_kicad_schematic_contains(
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at -2.54 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 10 10 0)
    (mirror x y)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (label "NET_A" (at 7.46 10 0)))
"#,
        "malformed mirror token",
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
