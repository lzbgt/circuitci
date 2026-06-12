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
fn import_kicad_schematic_suggests_tlv803_reset_supervisor_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let imported_path = dir.path().join("tlv803_reset_supervisor.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_tlv803_reset_supervisor_suggestions/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_tlv803_reset_supervisor_suggestions/circuitci.kicad-map.yaml",
            "--output",
            imported_path.to_str().unwrap(),
            "--name",
            "kicad_tlv803_reset_supervisor_suggestions",
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
        imported["board"]["components"]["USUP"]["model"],
        "vendor.ti.tlv803ea29"
    );
    assert_eq!(
        imported["board"]["components"]["USUP"]["pins"]["VDD"],
        "net_rail_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["USUP"]["pins"]["RESET"],
        "net_nrst"
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
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power-tree suggestion");
    assert_eq!(power_tree["runnable"], true);
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
    let supervisor = &power_tree["scenario"]["reset_supervisors"][0];
    assert_eq!(supervisor["component"], "USUP");
    assert_eq!(supervisor["monitored_pin"], "VDD");
    assert_eq!(supervisor["monitored_net"], "net_rail_3v3");
    assert_eq!(supervisor["reset_output_pin"], "RESET");
    assert_eq!(supervisor["reset_net"], "net_nrst");
    assert_eq!(supervisor["threshold_min_V"], 2.8714);
    assert_eq!(supervisor["threshold_max_V"], 2.9886);
}

#[test]
fn import_kicad_schematic_suggests_ap2112_regulator_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let imported_path = dir.path().join("ap2112_regulator.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_ap2112_regulator_suggestions/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_ap2112_regulator_suggestions/circuitci.kicad-map.yaml",
            "--output",
            imported_path.to_str().unwrap(),
            "--name",
            "kicad_ap2112_regulator_suggestions",
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
        imported["board"]["components"]["UREG"]["model"],
        "vendor.diodes.ap2112k_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["UREG"]["pins"]["VIN"],
        "net_usb_5v"
    );
    assert_eq!(
        imported["board"]["components"]["UREG"]["pins"]["VOUT"],
        "net_rail_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["UREG"]["pins"]["EN"],
        "net_usb_5v"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["pins"]["VDD"],
        "net_rail_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["spice"]["value_f"],
        0.000001
    );
    assert_eq!(
        imported["board"]["components"]["C2"]["spice"]["value_f"],
        0.000001
    );
    assert_eq!(
        imported["board"]["nets"]["net_usb_5v"]["nominal_voltage"],
        5.0
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
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power-tree suggestion");
    assert_eq!(power_tree["runnable"], true);
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
    let regulator = &power_tree["scenario"]["regulators"][0];
    assert_eq!(regulator["component"], "UREG");
    assert_eq!(regulator["input_pin"], "VIN");
    assert_eq!(regulator["input_net"], "net_usb_5v");
    assert_eq!(regulator["output_pin"], "VOUT");
    assert_eq!(regulator["output_net"], "net_rail_3v3");
    assert_eq!(regulator["dropout_voltage_V"], 0.4);
    assert_eq!(regulator["max_output_current_A"], 0.6);
    assert_eq!(regulator["input_capacitance_min_F"], 0.000001);
    assert_eq!(regulator["output_capacitance_min_F"], 0.000001);
    assert_eq!(regulator["input_support_capacitance_F"], 0.000001);
    assert_eq!(regulator["input_support_capacitors"][0], "C1");
    assert_eq!(regulator["output_support_capacitance_F"], 0.000001);
    assert_eq!(regulator["output_support_capacitors"][0], "C2");
}

#[test]
fn import_kicad_schematic_suggests_ams1117_regulator_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let imported_path = dir.path().join("ams1117_regulator.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_ams1117_regulator_suggestions/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_ams1117_regulator_suggestions/circuitci.kicad-map.yaml",
            "--output",
            imported_path.to_str().unwrap(),
            "--name",
            "kicad_ams1117_regulator_suggestions",
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
        imported["board"]["components"]["UREG"]["model"],
        "vendor.ams.ams1117_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["UREG"]["pins"]["VIN"],
        "net_usb_5v"
    );
    assert_eq!(
        imported["board"]["components"]["UREG"]["pins"]["VOUT"],
        "net_rail_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["pins"]["VDD"],
        "net_rail_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["COUT"]["spice"]["value_f"],
        0.000022
    );
    assert_eq!(
        imported["board"]["nets"]["net_usb_5v"]["nominal_voltage"],
        5.0
    );
    assert_eq!(
        imported["board"]["nets"]["net_rail_3v3"]["power_valid_at_us"],
        2000.0
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
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power-tree suggestion");
    assert_eq!(power_tree["runnable"], true);
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
    let regulator = &power_tree["scenario"]["regulators"][0];
    assert_eq!(regulator["component"], "UREG");
    assert_eq!(regulator["input_pin"], "VIN");
    assert_eq!(regulator["input_net"], "net_usb_5v");
    assert_eq!(regulator["output_pin"], "VOUT");
    assert_eq!(regulator["output_net"], "net_rail_3v3");
    assert_eq!(regulator["dropout_voltage_V"], 1.3);
    assert_eq!(regulator["min_output_current_A"], 0.01);
    assert_eq!(regulator["max_output_current_A"], 0.8);
    assert_eq!(regulator["output_capacitance_min_F"], 0.000022);
    assert_eq!(regulator["output_support_capacitance_F"], 0.000022);
    assert_eq!(regulator["output_support_capacitors"][0], "COUT");
}

#[test]
fn import_kicad_schematic_suggests_tpd2eusb30_usb_esd_clamps() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let imported_path = dir.path().join("tpd2eusb30_usb_esd.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_tpd2eusb30_usb_esd_suggestions/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_tpd2eusb30_usb_esd_suggestions/circuitci.kicad-map.yaml",
            "--output",
            imported_path.to_str().unwrap(),
            "--name",
            "kicad_tpd2eusb30_usb_esd_suggestions",
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
        imported["board"]["components"]["UESD"]["model"],
        "vendor.ti.tpd2eusb30"
    );
    assert_eq!(
        imported["board"]["components"]["UESD"]["pins"]["D1+"],
        "net_usb_dp"
    );
    assert_eq!(
        imported["board"]["components"]["UESD"]["pins"]["D1-"],
        "net_usb_dm"
    );
    assert_eq!(
        imported["board"]["components"]["UESD"]["pins"]["GND"],
        "gnd"
    );
    assert_eq!(
        imported["board"]["nets"]["net_usb_dp"]["nominal_voltage"],
        3.3
    );
    assert_eq!(imported["board"]["nets"]["gnd"]["kind"], "ground");

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

    let dp = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "interface_protection_uesd_d1_plus")
        .expect("D+ clamp suggestion");
    assert_eq!(dp["runnable"], true);
    assert_eq!(dp["scenario"]["checks"][0], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(dp["scenario"]["target"]["component"], "UESD");
    assert_eq!(dp["scenario"]["parameters"]["clamp"], "d1_plus");
    let dp_clamp = &dp["scenario"]["protection_clamps"][0];
    assert_eq!(dp_clamp["protected_pin"], "D1+");
    assert_eq!(dp_clamp["protected_net"], "net_usb_dp");
    assert_eq!(dp_clamp["reference_pin"], "GND");
    assert_eq!(dp_clamp["reference_net"], "gnd");
    assert_eq!(dp_clamp["reference"], "ground");
    assert_eq!(dp_clamp["working_voltage_max_V"], 5.5);
    assert_eq!(dp_clamp["line_capacitance_F"], 7.0e-13);

    let dm = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "interface_protection_uesd_d1_minus")
        .expect("D- clamp suggestion");
    assert_eq!(dm["scenario"]["parameters"]["clamp"], "d1_minus");
    assert_eq!(
        dm["scenario"]["protection_clamps"][0]["protected_net"],
        "net_usb_dm"
    );
}

#[test]
fn import_kicad_schematic_suggests_prtr5v0u2x_usb_esd_clamps() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let imported_path = dir.path().join("prtr5v0u2x_usb_esd.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_prtr5v0u2x_usb_esd_suggestions/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_prtr5v0u2x_usb_esd_suggestions/circuitci.kicad-map.yaml",
            "--output",
            imported_path.to_str().unwrap(),
            "--name",
            "kicad_prtr5v0u2x_usb_esd_suggestions",
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
        imported["board"]["components"]["UESD"]["model"],
        "vendor.nexperia.prtr5v0u2x"
    );
    assert_eq!(
        imported["board"]["components"]["UESD"]["pins"]["IO1"],
        "net_usb_dp"
    );
    assert_eq!(
        imported["board"]["components"]["UESD"]["pins"]["IO2"],
        "net_usb_dm"
    );
    assert_eq!(
        imported["board"]["components"]["UESD"]["pins"]["VCC"],
        "net_usb_vbus"
    );
    assert_eq!(
        imported["board"]["components"]["UESD"]["pins"]["GND"],
        "gnd"
    );
    assert_eq!(imported["board"]["nets"]["net_usb_vbus"]["kind"], "power");
    assert_eq!(
        imported["board"]["nets"]["net_usb_vbus"]["nominal_voltage"],
        5.0
    );
    assert_eq!(imported["board"]["nets"]["net_usb_vbus"]["powered"], true);

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

    let dp = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "interface_protection_uesd_io1_to_vcc")
        .expect("IO1 clamp suggestion");
    assert_eq!(dp["runnable"], true);
    assert_eq!(dp["scenario"]["checks"][0], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(dp["scenario"]["target"]["component"], "UESD");
    assert_eq!(dp["scenario"]["parameters"]["clamp"], "io1_to_vcc");
    let dp_clamp = &dp["scenario"]["protection_clamps"][0];
    assert_eq!(dp_clamp["protected_pin"], "IO1");
    assert_eq!(dp_clamp["protected_net"], "net_usb_dp");
    assert_eq!(dp_clamp["reference_pin"], "VCC");
    assert_eq!(dp_clamp["reference_net"], "net_usb_vbus");
    assert_eq!(dp_clamp["reference"], "power");
    assert_eq!(dp_clamp["working_voltage_max_V"], 5.5);
    assert_eq!(dp_clamp["line_capacitance_F"], 1.5e-12);

    let dm = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "interface_protection_uesd_io2_to_vcc")
        .expect("IO2 clamp suggestion");
    assert_eq!(dm["scenario"]["parameters"]["clamp"], "io2_to_vcc");
    assert_eq!(
        dm["scenario"]["protection_clamps"][0]["protected_net"],
        "net_usb_dm"
    );
}

#[test]
fn import_kicad_schematic_suggests_usb_connector_protection() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let imported_path = dir.path().join("usb_connector_protection.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_usb_connector_protection_suggestions/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_usb_connector_protection_suggestions/circuitci.kicad-map.yaml",
            "--output",
            imported_path.to_str().unwrap(),
            "--name",
            "kicad_usb_connector_protection_suggestions",
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
        imported["board"]["components"]["J1"]["model"],
        "generic.connector.usb2"
    );
    assert_eq!(
        imported["board"]["components"]["J1"]["pins"]["VBUS"],
        "net_usb_vbus"
    );
    assert_eq!(
        imported["board"]["components"]["J1"]["pins"]["D+"],
        "net_usb_dp"
    );
    assert_eq!(
        imported["board"]["components"]["J1"]["pins"]["D-"],
        "net_usb_dm"
    );
    assert_eq!(imported["board"]["components"]["J1"]["pins"]["GND"], "gnd");
    assert_eq!(
        imported["board"]["components"]["J1"]["pins"]["SHIELD"],
        "gnd"
    );
    assert_eq!(
        imported["board"]["layout"]["footprints"]["J1"]["entry_aperture"]["source"],
        "kicad_mapping"
    );
    assert_eq!(
        imported["board"]["layout"]["footprints"]["J1"]["entry_aperture"]["width_mm"],
        1.2
    );
    assert_eq!(
        imported["board"]["components"]["UESD"]["model"],
        "vendor.ti.tpd2eusb30"
    );
    assert_eq!(
        imported["board"]["components"]["UVBUS"]["model"],
        "generic.protection.vbus_esd_basic"
    );
    assert_eq!(imported["board"]["nets"]["net_usb_vbus"]["kind"], "power");
    assert_eq!(
        imported["board"]["nets"]["net_usb_vbus"]["nominal_voltage"],
        5.0
    );
    assert_eq!(imported["board"]["nets"]["net_usb_vbus"]["powered"], true);

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

    let connector = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_protection_j1")
        .expect("USB connector protection suggestion");
    assert_eq!(connector["runnable"], true);
    assert_eq!(
        connector["scenario"]["checks"][0],
        "USB_CONNECTOR_PROTECTION_VALID"
    );
    assert_eq!(connector["scenario"]["target"]["component"], "J1");
    assert_eq!(
        connector["scenario"]["parameters"]["require_vbus_protection"],
        true
    );
    assert_eq!(
        connector["scenario"]["parameters"]["require_shield_ground"],
        true
    );
    assert_eq!(
        connector["scenario"]["parameters"]["data_working_voltage_min_V"],
        3.3
    );
    assert_eq!(
        connector["scenario"]["parameters"]["vbus_working_voltage_min_V"],
        5.0
    );
    let usb = &connector["scenario"]["usb_connectors"][0];
    assert_eq!(usb["component"], "J1");
    assert_eq!(usb["vbus_net"], "net_usb_vbus");
    assert_eq!(usb["dp_net"], "net_usb_dp");
    assert_eq!(usb["dm_net"], "net_usb_dm");
    assert_eq!(usb["gnd_net"], "gnd");
    assert_eq!(usb["shield_pin"], "SHIELD");
    assert_eq!(usb["shield_net"], "gnd");
    let clamps = connector["scenario"]["protection_clamps"]
        .as_array()
        .unwrap();
    assert_eq!(clamps.len(), 3);
    assert!(clamps.iter().any(|clamp| {
        clamp["component"] == "UESD"
            && clamp["clamp"] == "d1_plus"
            && clamp["protected_net"] == "net_usb_dp"
    }));
    assert!(clamps.iter().any(|clamp| {
        clamp["component"] == "UESD"
            && clamp["clamp"] == "d1_minus"
            && clamp["protected_net"] == "net_usb_dm"
    }));
    assert!(clamps.iter().any(|clamp| {
        clamp["component"] == "UVBUS"
            && clamp["clamp"] == "vbus"
            && clamp["protected_net"] == "net_usb_vbus"
    }));
}

#[test]
fn import_kicad_schematic_rejects_invalid_layout_aperture_mapping() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("invalid_layout_aperture.project.yaml");
    let mapping_path = dir.path().join("invalid_layout_aperture.kicad-map.yaml");
    let mapping = std::fs::read_to_string(
        "examples/import_kicad_usb_connector_protection_suggestions/circuitci.kicad-map.yaml",
    )
    .unwrap()
    .replace("width_mm: 1.2", "width_mm: 0.0");
    std::fs::write(&mapping_path, mapping).unwrap();
    let output_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_usb_connector_protection_suggestions/root.kicad_sch",
            "--mapping",
            mapping_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "invalid_layout_aperture",
        ])
        .output()
        .unwrap();
    assert!(
        !output_status.status.success(),
        "invalid layout aperture mapping unexpectedly imported"
    );
    let stderr = String::from_utf8(output_status.stderr).unwrap();
    assert!(
        stderr.contains("layout.entry_aperture.width_mm must be greater than zero"),
        "expected invalid layout aperture width error, got:\n{stderr}"
    );
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
