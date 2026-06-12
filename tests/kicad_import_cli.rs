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
fn import_kicad_schematic_grouped_bus_alias_runs_generated_spice() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("grouped_bus_spice.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_grouped_bus_spice/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_grouped_bus_spice/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "grouped_bus_spice",
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
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["A"],
        "net_vin"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["B"],
        "net_port_reset_rc"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["pins"]["A"],
        "net_port_reset_rc"
    );
    assert_eq!(imported["board"]["components"]["C1"]["pins"]["B"], "gnd");
    assert_eq!(
        imported["scenarios"][0]["analog"]["generated"]["components"],
        serde_json::json!(["V1", "R1", "C1"])
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["probes"][0]["expression"],
        "V(net_port_reset_rc)"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["assertions"][0]["name"],
        "grouped_bus_rc_node_charges"
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
fn import_kicad_schematic_suggests_bootstrap_bias_from_mapped_resistors() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let imported_path = dir.path().join("bootstrap_bias_imported.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_bootstrap_bias_suggestions/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_bootstrap_bias_suggestions/circuitci.kicad-map.yaml",
            "--output",
            imported_path.to_str().unwrap(),
            "--name",
            "kicad_bootstrap_bias_suggestions",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&imported_path, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&imported_path).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["U1"]["pins"]["BOOT0"],
        "net_boot0"
    );
    assert_eq!(
        imported["board"]["components"]["RUP"]["pins"]["A"],
        "net_rail_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["RUP"]["pins"]["B"],
        "net_boot0"
    );
    assert_eq!(imported["board"]["components"]["RDN"]["pins"]["A"], "gnd");
    assert_eq!(
        imported["board"]["components"]["RDN"]["pins"]["B"],
        "net_boot0"
    );
    assert_eq!(
        imported["board"]["components"]["RUP"]["spice"]["value_ohm"],
        100000.0
    );
    assert_eq!(
        imported["board"]["components"]["RDN"]["spice"]["value_ohm"],
        10000.0
    );

    let suggestions_path = dir.path().join("suggestions.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            imported_path.to_str().unwrap(),
            "--output",
            suggestions_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let suggestions: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&suggestions_path).unwrap()).unwrap();
    let suggestion_schema: Value = serde_json::from_str(include_str!(
        "../schemas/scenario_suggestion_report.schema.json"
    ))
    .unwrap();
    let suggestion_validator = jsonschema::validator_for(&suggestion_schema).unwrap();
    assert_yaml_file_valid(&suggestions_path, &suggestion_validator);
    let bias = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "boot_strap_bias_valid_u1_application")
        .expect("application boot strap bias suggestion");
    assert_eq!(bias["runnable"], true);
    assert_eq!(bias["scenario"]["type"], "reset_boot");
    assert_eq!(bias["scenario"]["checks"][0], "BOOT_STRAP_BIAS_VALID");
    assert_eq!(bias["scenario"]["target"]["component"], "U1");
    assert_eq!(bias["scenario"]["required_boot_mode"], "application");
    assert!(bias.get("required_inputs").is_none());
    assert!(
        bias["reason"]
            .as_str()
            .unwrap()
            .contains("explicit resistor bias evidence")
    );

    let bootloader = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "boot_strap_defined_u1_bootloader")
        .expect("bootloader observed strap suggestion");
    assert_eq!(bootloader["runnable"], false);
    assert_eq!(bootloader["scenario"]["checks"][0], "BOOT_STRAP_DEFINED");
    assert_eq!(bootloader["scenario"]["straps"][0]["net"], "net_boot0");
}

#[test]
fn import_kicad_schematic_suggests_reset_release_from_mapped_rc() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let imported_path = dir.path().join("reset_rc_imported.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_reset_rc_suggestions/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_reset_rc_suggestions/circuitci.kicad-map.yaml",
            "--output",
            imported_path.to_str().unwrap(),
            "--name",
            "kicad_reset_rc_suggestions",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&imported_path, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&imported_path).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["U1"]["pins"]["NRST"],
        "net_nrst"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["A"],
        "net_rail_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["B"],
        "net_nrst"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["pins"]["A"],
        "net_nrst"
    );
    assert_eq!(imported["board"]["components"]["C1"]["pins"]["B"], "gnd");
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
        imported["board"]["nets"]["net_rail_3v3"]["power_valid_at_us"],
        1500.0
    );

    let suggestions_path = dir.path().join("suggestions.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            imported_path.to_str().unwrap(),
            "--output",
            suggestions_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let suggestions: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&suggestions_path).unwrap()).unwrap();
    let suggestion_schema: Value = serde_json::from_str(include_str!(
        "../schemas/scenario_suggestion_report.schema.json"
    ))
    .unwrap();
    let suggestion_validator = jsonschema::validator_for(&suggestion_schema).unwrap();
    assert_yaml_file_valid(&suggestions_path, &suggestion_validator);
    let reset = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "reset_release_after_power_valid_u1")
        .expect("reset release suggestion");
    assert_eq!(reset["runnable"], true);
    assert_eq!(reset["scenario"]["type"], "reset_boot");
    assert_eq!(
        reset["scenario"]["checks"][0],
        "RESET_RELEASE_AFTER_POWER_VALID"
    );
    assert_eq!(reset["scenario"]["target"]["component"], "U1");
    assert_eq!(reset["scenario"]["target"]["power_pin"], "VDD");
    assert_eq!(reset["scenario"]["target"]["reset_pin"], "NRST");
    assert_eq!(reset["scenario"]["timing"]["power_valid_at_us"], 1500.0);
    let delay_us = reset["scenario"]["timing"]["reset_release_delay_us"]
        .as_f64()
        .unwrap();
    assert!((delay_us - 931.558204).abs() < 0.001);
    let release_at_us = reset["scenario"]["timing"]["reset_release_at_us"]
        .as_f64()
        .unwrap();
    assert!((release_at_us - 2431.558204).abs() < 0.001);
    assert!(reset.get("required_inputs").is_none());
}

#[test]
fn import_kicad_schematic_root_hierarchical_label_runs_generated_spice() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("root_hier_label_spice.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_root_hier_label_spice/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_root_hier_label_spice/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "root_hier_label_spice",
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
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["A"],
        "net_vin"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["B"],
        "net_root_rc"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["pins"]["A"],
        "net_root_rc"
    );
    assert_eq!(imported["board"]["components"]["C1"]["pins"]["B"], "gnd");
    assert_eq!(
        imported["scenarios"][0]["analog"]["generated"]["components"],
        serde_json::json!(["V1", "R1", "C1"])
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["probes"][0]["expression"],
        "V(net_root_rc)"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["assertions"][0]["name"],
        "root_hier_label_rc_node_charges"
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
fn import_kicad_schematic_non_cardinal_rotation_runs_generated_spice() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir
        .path()
        .join("non_cardinal_rotated_kicad_schematic.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_non_cardinal_rotation_spice/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_non_cardinal_rotation_spice/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "non_cardinal_rotated_kicad_schematic",
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
        imported["board"]["components"]["C1"]["pins"]["A"],
        "net_reset_rc"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["generated"]["components"],
        serde_json::json!(["V1", "R1", "C1"])
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
fn import_kicad_schematic_hierarchy_alias_runs_generated_spice() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("hierarchy_alias_spice.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_hierarchy_alias_spice/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_hierarchy_alias_spice/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "hierarchy_alias_spice",
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
    assert_eq!(
        imported["board"]["components"]["V1"]["pins"]["P"],
        "net_vin"
    );
    assert_eq!(imported["board"]["components"]["V1"]["pins"]["N"], "gnd");
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["A"],
        "net_vin"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["B"],
        "net_sense_node"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["pins"]["A"],
        "net_sense_node"
    );
    assert_eq!(imported["board"]["components"]["C1"]["pins"]["B"], "gnd");
    assert_eq!(
        imported["scenarios"][0]["analog"]["generated"]["components"],
        serde_json::json!(["V1", "R1", "C1"])
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["probes"][0]["expression"],
        "V(net_sense_node)"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["assertions"][0]["name"],
        "alias_rc_node_charges"
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
fn import_kicad_schematic_repeated_hierarchy_runs_generated_spice() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("repeated_hierarchy_spice.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_repeated_hierarchy_spice/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_repeated_hierarchy_spice/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "repeated_hierarchy_spice",
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
    assert_eq!(
        imported["board"]["components"]["V1"]["pins"]["P"],
        "net_vin_shared"
    );
    assert_eq!(imported["board"]["components"]["V1"]["pins"]["N"], "gnd");
    assert_eq!(
        imported["board"]["components"]["left_filter__R1"]["pins"]["A"],
        "net_vin_shared"
    );
    assert_eq!(
        imported["board"]["components"]["left_filter__R1"]["pins"]["B"],
        "net_left_out"
    );
    assert_eq!(
        imported["board"]["components"]["left_filter__C1"]["pins"]["A"],
        "net_left_out"
    );
    assert_eq!(
        imported["board"]["components"]["right_filter__R1"]["pins"]["A"],
        "net_vin_shared"
    );
    assert_eq!(
        imported["board"]["components"]["right_filter__R1"]["pins"]["B"],
        "net_right_out"
    );
    assert_eq!(
        imported["board"]["components"]["right_filter__C1"]["pins"]["A"],
        "net_right_out"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["generated"]["components"],
        serde_json::json!([
            "V1",
            "left_filter__R1",
            "left_filter__C1",
            "right_filter__R1",
            "right_filter__C1"
        ])
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["probes"][0]["expression"],
        "V(net_left_out)"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["probes"][1]["expression"],
        "V(net_right_out)"
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
fn import_kicad_schematic_nested_hierarchy_runs_generated_spice() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("nested_hierarchy_spice.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_nested_hierarchy_spice/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_nested_hierarchy_spice/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "nested_hierarchy_spice",
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
    assert_eq!(
        imported["board"]["components"]["V1"]["pins"]["P"],
        "net_vin"
    );
    assert_eq!(imported["board"]["components"]["V1"]["pins"]["N"], "gnd");
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["A"],
        "net_vin"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["B"],
        "net_filter_out"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["pins"]["A"],
        "net_filter_out"
    );
    assert_eq!(imported["board"]["components"]["C1"]["pins"]["B"], "gnd");
    assert_eq!(
        imported["scenarios"][0]["analog"]["generated"]["components"],
        serde_json::json!(["V1", "R1", "C1"])
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["probes"][0]["expression"],
        "V(net_filter_out)"
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
  (wire (pts (xy 20 0) (xy 25 0)))
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
fn import_kicad_schematic_flattens_nested_sheet() {
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
  (wire (pts (xy 20 0) (xy 25 0)))
  (label "ROOT_RC" (at 25 0 0))
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
  (wire (pts (xy 0 0) (xy 20 0)))
  (hierarchical_label "RC" (at 0 0 0))
  (sheet (at 0 0 0) (size 20 20)
    (property "Sheetname" "Nested")
    (property "Sheetfile" "grandchild.kicad_sch")
    (pin "RC" input (at 20 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(
        &grandchild_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:C"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:C") (at 0 0 0)
    (property "Reference" "C1") (property "Value" "100n") (pin "1"))
  (hierarchical_label "RC" (at 0 0 0)))
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
        imported["board"]["components"]["C1"]["pins"]["1"],
        "net_root_rc"
    );
}

#[test]
fn import_kicad_schematic_rejects_sheet_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let child_path = dir.path().join("child.kicad_sch");
    let output = dir.path().join("hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols)
  (wire (pts (xy 20 0) (xy 25 0)))
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
  (wire (pts (xy 0 0) (xy 20 0)))
  (hierarchical_label "RC" (at 0 0 0))
  (sheet (at 0 0 0) (size 20 20)
    (property "Sheetname" "Cycle")
    (property "Sheetfile" "root.kicad_sch")
    (pin "RC" input (at 20 0 0))))
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
    assert!(stderr.contains("hierarchy contains a cycle"));
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
fn import_kicad_schematic_allows_duplicate_sheet_pin_names_when_root_labels_disambiguate() {
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
  (wire (pts (xy 20 0) (xy 25 0)))
  (label "LEFT_IN" (at 25 0 0))
  (wire (pts (xy 60 0) (xy 65 0)))
  (label "RIGHT_IN" (at 65 0 0))
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
    let child = r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (hierarchical_label "IN" (at 0 0 0)))
"#;
    std::fs::write(&first_path, child).unwrap();
    std::fs::write(&second_path, child).unwrap();
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
        imported["board"]["components"]["first__R1"]["pins"]["1"],
        "net_left_in"
    );
    assert_eq!(
        imported["board"]["components"]["second__R1"]["pins"]["1"],
        "net_right_in"
    );
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
    assert!(stderr.contains("without an explicit root label"));
    assert!(!output.exists());
}

#[test]
fn import_kicad_schematic_allows_distinct_sheet_pins_on_labelled_root_net() {
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
  (wire (pts (xy 20 0) (xy 60 0)))
  (label "SENSE_NODE" (at 40 0 0))
  (wire (pts (xy 20 10) (xy 60 10)))
  (label "GND" (at 40 10 0))
  (sheet (at 20 -10 0) (size 20 30)
    (property "Sheetname" "Filter")
    (property "Sheetfile" "first.kicad_sch")
    (pin "FILTER_OUT" output (at 20 0 0))
    (pin "GND" input (at 20 10 0)))
  (sheet (at 60 -10 0) (size 20 30)
    (property "Sheetname" "Adc")
    (property "Sheetfile" "second.kicad_sch")
    (pin "ADC_IN" input (at 60 0 0))
    (pin "GND" input (at 60 10 0))))
"#,
    )
    .unwrap();
    std::fs::write(
        &first_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (hierarchical_label "FILTER_OUT" (at 0 0 0))
  (hierarchical_label "GND" (at 0 10 0)))
"#,
    )
    .unwrap();
    std::fs::write(
        &second_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R2") (property "Value" "47k") (pin "1") (pin "2"))
  (hierarchical_label "ADC_IN" (at 0 0 0))
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
        imported["board"]["components"]["R1"]["pins"]["1"],
        "net_sense_node"
    );
    assert_eq!(
        imported["board"]["components"]["R2"]["pins"]["1"],
        "net_sense_node"
    );
    assert_eq!(imported["board"]["components"]["R1"]["pins"]["2"], "gnd");
    assert_eq!(imported["board"]["components"]["R2"]["pins"]["2"], "gnd");
}

#[test]
fn import_kicad_schematic_rejects_disconnected_alias_name_collision() {
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
  (wire (pts (xy 20 0) (xy 25 0)))
  (label "B" (at 25 0 0))
  (wire (pts (xy 60 0) (xy 65 0)))
  (sheet (at 20 -10 0) (size 20 20)
    (property "Sheetname" "First")
    (property "Sheetfile" "first.kicad_sch")
    (pin "A" output (at 20 0 0)))
  (sheet (at 60 -10 0) (size 20 20)
    (property "Sheetname" "Second")
    (property "Sheetfile" "second.kicad_sch")
    (pin "B" input (at 60 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(
        &first_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (hierarchical_label "A" (at 0 0 0)))
"#,
    )
    .unwrap();
    std::fs::write(
        &second_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R2") (property "Value" "47k") (pin "1"))
  (hierarchical_label "B" (at 0 0 0)))
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
    assert!(stderr.contains("both resolve to sheet alias B"));
    assert!(!output.exists());
}

#[test]
fn import_kicad_schematic_keeps_duplicate_ground_aliases_per_sheet() {
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
  (wire (pts (xy 20 0) (xy 25 0)))
  (label "VREF" (at 25 0 0))
  (wire (pts (xy 60 0) (xy 65 0)))
  (label "GND" (at 65 0 0))
  (sheet (at 20 -10 0) (size 20 20)
    (property "Sheetname" "First")
    (property "Sheetfile" "first.kicad_sch")
    (pin "GND" input (at 20 0 0)))
  (sheet (at 60 -10 0) (size 20 20)
    (property "Sheetname" "Second")
    (property "Sheetfile" "second.kicad_sch")
    (pin "GND" input (at 60 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(
        &first_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1"))
  (hierarchical_label "GND" (at 0 0 0)))
"#,
    )
    .unwrap();
    std::fs::write(
        &second_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R2") (property "Value" "47k") (pin "1"))
  (hierarchical_label "GND" (at 0 0 0)))
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
        imported["board"]["components"]["R1"]["pins"]["1"],
        "net_vref"
    );
    assert_eq!(imported["board"]["components"]["R2"]["pins"]["1"], "gnd");
}

#[test]
fn import_kicad_schematic_namespaces_duplicate_refs_across_sheet() {
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
    assert_eq!(imported["board"]["components"]["R1"]["pins"]["1"], "net_rc");
    assert_eq!(
        imported["board"]["components"]["filter__R1"]["pins"]["1"],
        "net_rc"
    );
    assert_eq!(
        imported["board"]["components"]["filter__R1"]["source"]["format"],
        "kicad_schematic"
    );
}

#[test]
fn import_kicad_schematic_namespaces_repeated_child_sheet_instances() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("root.kicad_sch");
    let child_path = dir.path().join("filter.kicad_sch");
    let output = dir.path().join("hierarchy.project.yaml");
    std::fs::write(
        &root_path,
        r#"
(kicad_sch
  (lib_symbols)
  (wire (pts (xy 20 0) (xy 25 0)))
  (label "LEFT_IN" (at 25 0 0))
  (wire (pts (xy 60 0) (xy 65 0)))
  (label "RIGHT_IN" (at 65 0 0))
  (sheet (at 0 0 0) (size 20 20)
    (property "Sheetname" "Left Filter")
    (property "Sheetfile" "filter.kicad_sch")
    (pin "IN" input (at 20 0 0)))
  (sheet (at 40 0 0) (size 20 20)
    (property "Sheetname" "Right Filter")
    (property "Sheetfile" "filter.kicad_sch")
    (pin "IN" input (at 60 0 0))))
"#,
    )
    .unwrap();
    std::fs::write(
        &child_path,
        r#"
(kicad_sch
  (lib_symbols
    (symbol "Device:R"
      (pin passive line (at 0 0 0) (length 2.54) (number "1"))
      (pin passive line (at 0 10 180) (length 2.54) (number "2")))
    (symbol "Device:C"
      (pin passive line (at 20 10 0) (length 2.54) (number "1"))
      (pin passive line (at 20 20 180) (length 2.54) (number "2"))))
  (symbol (lib_id "Device:R") (at 0 0 0)
    (property "Reference" "R1") (property "Value" "10k") (pin "1") (pin "2"))
  (symbol (lib_id "Device:C") (at 0 0 0)
    (property "Reference" "C1") (property "Value" "100n") (pin "1") (pin "2"))
  (wire (pts (xy 0 10) (xy 20 10)))
  (hierarchical_label "IN" (at 0 0 0))
  (no_connect (at 20 20)))
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
        imported["board"]["components"]["left_filter__R1"]["pins"]["1"],
        "net_left_in"
    );
    assert_eq!(
        imported["board"]["components"]["right_filter__R1"]["pins"]["1"],
        "net_right_in"
    );
    assert_eq!(
        imported["board"]["components"]["left_filter__R1"]["pins"]["2"],
        "net_left_filter_net_2"
    );
    assert_eq!(
        imported["board"]["components"]["left_filter__C1"]["pins"]["1"],
        "net_left_filter_net_2"
    );
    assert_eq!(
        imported["board"]["components"]["right_filter__R1"]["pins"]["2"],
        "net_right_filter_net_2"
    );
    assert_eq!(
        imported["board"]["components"]["right_filter__C1"]["pins"]["1"],
        "net_right_filter_net_2"
    );
}
