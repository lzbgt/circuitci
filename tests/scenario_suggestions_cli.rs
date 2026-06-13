use serde_json::Value;
use std::process::Command;

fn run_suggest_scenarios(project: &str) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("suggestions.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            project,
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let suggestions: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(output).unwrap()).unwrap();
    assert_suggestion_schema_valid(&suggestions);
    suggestions
}

fn assert_suggestion_schema_valid(suggestions: &Value) {
    let schema: Value = serde_json::from_str(include_str!(
        "../schemas/scenario_suggestion_report.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(suggestions)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "suggestion schema errors: {errors:#?}");
}

#[test]
fn suggest_scenarios_derives_power_boot_reset_and_uart_templates() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_power_reset/project.yaml");
    assert_eq!(suggestions["project"], "scenario_suggestions_power_reset");
    assert_eq!(suggestions["suggestions"].as_array().unwrap().len(), 6);
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["kind"] == "power_tree")
        .expect("power_tree suggestion");
    assert_eq!(power_tree["runnable"], true);
    assert_eq!(power_tree["scenario"]["type"], "power_tree");
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
    let io_voltage = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "io_voltage_compatible")
        .expect("I/O voltage suggestion");
    assert_eq!(io_voltage["kind"], "power_tree");
    assert_eq!(io_voltage["runnable"], true);
    assert_eq!(io_voltage["scenario"]["type"], "power_tree");
    assert_eq!(io_voltage["scenario"]["checks"][0], "IO_VOLTAGE_COMPATIBLE");
    let io_paths = io_voltage["scenario"]["paths"].as_array().unwrap();
    assert_eq!(io_paths.len(), 2);
    let usb_to_mcu = io_paths
        .iter()
        .find(|path| path["driver"]["component"] == "U2" && path["driver"]["pin"] == "TXD")
        .expect("USB-UART TX to MCU RX I/O voltage path");
    assert_eq!(usb_to_mcu["victim"]["component"], "U1");
    assert_eq!(usb_to_mcu["victim"]["pin"], "RX");
    assert_eq!(usb_to_mcu["net"], "uart_mcu_rx");
    let mcu_to_usb = io_paths
        .iter()
        .find(|path| path["driver"]["component"] == "U1" && path["driver"]["pin"] == "TX")
        .expect("MCU TX to USB-UART RX I/O voltage path");
    assert_eq!(mcu_to_usb["victim"]["component"], "U2");
    assert_eq!(mcu_to_usb["victim"]["pin"], "RXD");
    assert_eq!(mcu_to_usb["net"], "uart_mcu_tx");
    let reset = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["kind"] == "reset_boot")
        .expect("reset suggestion");
    assert_eq!(reset["runnable"], false);
    assert_eq!(reset["scenario"]["target"]["component"], "U1");
    assert_eq!(reset["scenario"]["target"]["power_pin"], "VDD");
    assert_eq!(reset["scenario"]["target"]["reset_pin"], "NRST");
    assert_eq!(reset["scenario"]["timing"]["power_valid_at_us"], 1500.0);
    assert!(
        reset["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("reset_release_at_us")
    );
    let bootloader_strap = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "boot_strap_defined_u1_bootloader")
        .expect("bootloader strap suggestion");
    assert_eq!(bootloader_strap["runnable"], false);
    assert_eq!(
        bootloader_strap["scenario"]["required_boot_mode"],
        "bootloader"
    );
    assert_eq!(
        bootloader_strap["scenario"]["checks"][0],
        "BOOT_STRAP_DEFINED"
    );
    assert_eq!(bootloader_strap["scenario"]["straps"][0]["pin"], "BOOT0");
    assert_eq!(bootloader_strap["scenario"]["straps"][0]["net"], "boot0");
    assert!(bootloader_strap["scenario"]["straps"][0]["actual"].is_null());
    let uart = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "uart_bootloader_sync_u1_uart")
        .expect("uart bootloader suggestion");
    assert_eq!(uart["runnable"], false);
    assert_eq!(uart["scenario"]["type"], "serial_programming");
    assert_eq!(uart["scenario"]["bootloader"]["interface"], "uart");
    assert_eq!(uart["scenario"]["bootloader"]["sync_byte"], 127);
    assert_eq!(uart["scenario"]["bootloader"]["expected_response"], 121);
    assert_eq!(uart["scenario"]["events"][0]["from"]["component"], "U2");
    assert_eq!(uart["scenario"]["events"][0]["from"]["pin"], "TXD");
    assert_eq!(uart["scenario"]["events"][0]["to"]["component"], "U1");
    assert_eq!(uart["scenario"]["events"][0]["to"]["pin"], "RX");
    assert_eq!(uart["scenario"]["events"][0]["bytes"][0], 127);
    assert!(uart["scenario"]["events"][0]["at_us"].is_null());
}

#[test]
fn suggest_scenarios_marks_load_switch_power_tree_template_non_runnable() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_load_switch/project.yaml");
    assert_eq!(suggestions["project"], "scenario_suggestions_load_switch");
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power_tree suggestion");
    assert_eq!(power_tree["kind"], "power_tree");
    assert_eq!(power_tree["runnable"], false);
    assert_eq!(power_tree["scenario"]["type"], "power_tree");
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
    assert_eq!(power_tree["scenario"]["pin_states"][0]["component"], "USW");
    assert_eq!(power_tree["scenario"]["pin_states"][0]["pin"], "EN");
    assert_eq!(power_tree["scenario"]["pin_states"][0]["mode"], "input");
    assert_eq!(power_tree["scenario"]["pin_states"][0]["state"], "high");
    assert!(
        power_tree["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("sensor_3v3")
    );
}

#[test]
fn suggest_scenarios_makes_tied_load_switch_power_tree_template_runnable() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_load_switch_tied_enable/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_load_switch_tied_enable"
    );
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power_tree suggestion");
    assert_eq!(power_tree["kind"], "power_tree");
    assert_eq!(power_tree["runnable"], true);
    assert!(power_tree.get("required_inputs").is_none());
    assert_eq!(power_tree["scenario"]["type"], "power_tree");
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
    assert_eq!(power_tree["scenario"]["pin_states"][0]["component"], "USW");
    assert_eq!(power_tree["scenario"]["pin_states"][0]["pin"], "EN");
    assert_eq!(power_tree["scenario"]["pin_states"][0]["mode"], "input");
    assert_eq!(power_tree["scenario"]["pin_states"][0]["state"], "high");
}

#[test]
fn suggest_scenarios_marks_charger_power_tree_template_non_runnable_without_current() {
    let suggestions = run_suggest_scenarios("examples/scenario_suggestions_charger/project.yaml");
    assert_eq!(suggestions["project"], "scenario_suggestions_charger");
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power_tree suggestion");
    assert_eq!(power_tree["kind"], "power_tree");
    assert_eq!(power_tree["runnable"], false);
    assert_eq!(power_tree["scenario"]["type"], "power_tree");
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
    assert!(
        power_tree["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("programmed_charge_current_A")
    );
}

#[test]
fn suggest_scenarios_makes_charger_power_tree_template_runnable_from_prog_resistor() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_charger_prog_resistor/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_charger_prog_resistor"
    );
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power_tree suggestion");
    assert_eq!(power_tree["kind"], "power_tree");
    assert_eq!(power_tree["runnable"], true);
    assert!(power_tree.get("required_inputs").is_none());
    assert_eq!(power_tree["scenario"]["type"], "power_tree");
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
}

#[test]
fn suggest_scenarios_marks_power_mux_template_non_runnable_without_source_selection() {
    let suggestions = run_suggest_scenarios("examples/scenario_suggestions_power_mux/project.yaml");
    assert_eq!(suggestions["project"], "scenario_suggestions_power_mux");
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power_tree suggestion");
    assert_eq!(power_tree["kind"], "power_tree");
    assert_eq!(power_tree["runnable"], false);
    assert_eq!(power_tree["scenario"]["type"], "power_tree");
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
    let required_input = power_tree["required_inputs"][0].as_str().unwrap();
    assert!(required_input.contains("UMUX"));
    assert!(required_input.contains("selected_input"));
    assert!(required_input.contains("sys"));
    assert!(required_input.contains("usb"));
    assert!(required_input.contains("battery"));
}

#[test]
fn suggest_scenarios_reports_boost_input_inductance_evidence() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_tps61023_boost/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_tps61023_boost"
    );
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power_tree suggestion");
    assert_eq!(power_tree["kind"], "power_tree");
    assert_eq!(power_tree["runnable"], true);
    let regulator = power_tree["scenario"]["regulators"]
        .as_array()
        .unwrap()
        .iter()
        .find(|regulator| regulator["component"] == "UBOOST")
        .expect("TPS61023 regulator evidence");
    assert_eq!(regulator["input_pin"], "VIN");
    assert_eq!(regulator["input_net"], "battery");
    assert_eq!(regulator["switch_pin"], "SW");
    assert_eq!(regulator["switch_net"], "boost_sw");
    assert_eq!(regulator["input_inductance_min_H"], 0.00000037);
    assert_eq!(regulator["input_inductance_max_H"], 0.0000029);
    assert_eq!(regulator["input_support_inductance_H"], 0.000001);
    assert_eq!(regulator["input_support_inductors"][0], "L1");
}

#[test]
fn suggest_scenarios_reports_buck_boost_switch_inductance_evidence() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_tps63802_buck_boost/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_tps63802_buck_boost"
    );
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power_tree suggestion");
    assert_eq!(power_tree["kind"], "power_tree");
    assert_eq!(power_tree["runnable"], true);
    let regulator = power_tree["scenario"]["regulators"]
        .as_array()
        .unwrap()
        .iter()
        .find(|regulator| regulator["component"] == "UBUCKBOOST")
        .expect("TPS63802 regulator evidence");
    assert_eq!(regulator["input_pin"], "VIN");
    assert_eq!(regulator["input_net"], "battery");
    assert_eq!(regulator["output_pin"], "VOUT");
    assert_eq!(regulator["output_net"], "rail_3v3");
    assert_eq!(regulator["switch_inductor_pin_a"], "L1");
    assert_eq!(regulator["switch_inductor_net_a"], "bb_l1");
    assert_eq!(regulator["switch_inductor_pin_b"], "L2");
    assert_eq!(regulator["switch_inductor_net_b"], "bb_l2");
    assert_eq!(regulator["switch_inductance_min_H"], 0.00000037);
    assert_eq!(regulator["switch_inductance_max_H"], 0.00000057);
    assert_eq!(regulator["switch_support_inductance_H"], 0.00000047);
    assert_eq!(regulator["switch_support_inductors"][0], "L1");
}

#[test]
fn suggest_scenarios_derives_manufacturing_artifact_templates() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_manufacturing_artifacts/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_manufacturing_artifacts"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 16);

    let drill_diameter = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "drill_diameter_valid")
        .expect("drill diameter suggestion");
    assert_eq!(drill_diameter["kind"], "manufacturing_drill_diameter");
    assert_eq!(drill_diameter["runnable"], true);
    assert_eq!(drill_diameter["scenario"]["type"], "manufacturing");
    assert_eq!(
        drill_diameter["scenario"]["checks"][0],
        "DRILL_DIAMETER_VALID"
    );
    assert_eq!(
        drill_diameter["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_drill_diameter_range_2026_06"
    );

    let drill_edge = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "drill_to_board_edge_clearance")
        .expect("drill edge suggestion");
    assert_eq!(drill_edge["runnable"], true);
    assert_eq!(
        drill_edge["scenario"]["checks"][0],
        "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID"
    );
    assert_eq!(
        drill_edge["scenario"]["parameters"]["min_drill_edge_clearance_mm"],
        0.50
    );
    assert!(drill_edge.get("required_inputs").is_none());

    let slot_width = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "slot_width_valid")
        .expect("slot width suggestion");
    assert_eq!(slot_width["runnable"], true);
    assert_eq!(slot_width["scenario"]["checks"][0], "SLOT_WIDTH_VALID");
    assert_eq!(
        slot_width["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_slot_min_2026_06"
    );

    let slot_aspect_ratio = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "slot_aspect_ratio_valid")
        .expect("slot aspect ratio suggestion");
    assert_eq!(slot_aspect_ratio["runnable"], true);
    assert_eq!(
        slot_aspect_ratio["scenario"]["checks"][0],
        "SLOT_ASPECT_RATIO_VALID"
    );
    assert_eq!(
        slot_aspect_ratio["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_slot_min_2026_06"
    );

    let slot_edge = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "slot_to_board_edge_clearance")
        .expect("slot edge suggestion");
    assert_eq!(slot_edge["runnable"], true);
    assert_eq!(
        slot_edge["scenario"]["checks"][0],
        "SLOT_TO_BOARD_EDGE_CLEARANCE_VALID"
    );
    assert_eq!(
        slot_edge["scenario"]["parameters"]["min_slot_edge_clearance_mm"],
        0.50
    );
    assert!(slot_edge.get("required_inputs").is_none());

    let castellated_hole = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "castellated_hole_valid")
        .expect("castellated hole suggestion");
    assert_eq!(castellated_hole["runnable"], true);
    assert_eq!(
        castellated_hole["scenario"]["checks"][0],
        "CASTELLATED_HOLE_VALID"
    );
    assert_eq!(
        castellated_hole["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_castellated_hole_2026_06"
    );

    let annular_ring = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "drill_annular_ring_valid")
        .expect("annular ring suggestion");
    assert_eq!(annular_ring["runnable"], true);
    assert_eq!(
        annular_ring["scenario"]["checks"][0],
        "DRILL_ANNULAR_RING_VALID"
    );
    assert_eq!(
        annular_ring["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_double_sided_via_min_2026_06"
    );

    let copper_edge = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "copper_to_board_edge_clearance")
        .expect("copper edge suggestion");
    assert_eq!(copper_edge["runnable"], true);
    assert_eq!(
        copper_edge["scenario"]["checks"][0],
        "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID"
    );
    assert_eq!(
        copper_edge["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_routed_edge_copper_clearance_2026_06"
    );

    let mask_opening = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_mask_opening_valid")
        .expect("mask opening suggestion");
    assert_eq!(mask_opening["runnable"], true);
    assert_eq!(
        mask_opening["scenario"]["checks"][0],
        "SOLDER_MASK_OPENING_VALID"
    );
    assert_eq!(
        mask_opening["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_standard_2026_06"
    );

    let mask_dam = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_mask_dam_valid")
        .expect("mask dam suggestion");
    assert_eq!(mask_dam["runnable"], true);
    assert_eq!(mask_dam["scenario"]["checks"][0], "SOLDER_MASK_DAM_VALID");
    assert_eq!(
        mask_dam["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_standard_2026_06"
    );

    let copper_spacing = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "copper_spacing_valid")
        .expect("copper spacing suggestion");
    assert_eq!(copper_spacing["runnable"], true);
    assert_eq!(
        copper_spacing["scenario"]["checks"][0],
        "COPPER_SPACING_VALID"
    );
    assert_eq!(
        copper_spacing["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_1oz_copper_spacing_2026_06"
    );

    let paste_opening = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_opening_valid")
        .expect("paste opening suggestion");
    assert_eq!(paste_opening["runnable"], true);
    assert_eq!(
        paste_opening["scenario"]["checks"][0],
        "SOLDER_PASTE_OPENING_VALID"
    );
    assert_eq!(
        paste_opening["scenario"]["parameters"]["min_paste_area_ratio"],
        0.70
    );
    assert_eq!(
        paste_opening["scenario"]["parameters"]["max_paste_area_ratio"],
        1.00
    );
    assert!(paste_opening.get("required_inputs").is_none());

    let paste_aperture = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_aperture_size_valid")
        .expect("paste aperture suggestion");
    assert_eq!(paste_aperture["runnable"], true);
    assert_eq!(
        paste_aperture["scenario"]["checks"][0],
        "SOLDER_PASTE_APERTURE_SIZE_VALID"
    );
    assert_eq!(
        paste_aperture["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_stencil_aperture_min_2026_06"
    );

    let paste_area_ratio = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_aperture_area_ratio_valid")
        .expect("paste aperture area ratio suggestion");
    assert_eq!(paste_area_ratio["runnable"], true);
    assert_eq!(
        paste_area_ratio["scenario"]["checks"][0],
        "SOLDER_PASTE_APERTURE_AREA_RATIO_VALID"
    );
    assert_eq!(
        paste_area_ratio["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_stencil_area_ratio_2026_06"
    );
    assert_eq!(
        paste_area_ratio["scenario"]["parameters"]["stencil_thickness_mm"],
        0.10
    );
    assert!(paste_area_ratio.get("required_inputs").is_none());

    let paste_ic_pin = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_ic_pin_aperture_valid")
        .expect("IC pin paste aperture suggestion");
    assert_eq!(paste_ic_pin["runnable"], true);
    assert_eq!(
        paste_ic_pin["scenario"]["checks"][0],
        "SOLDER_PASTE_IC_PIN_APERTURE_VALID"
    );
    assert_eq!(paste_ic_pin["scenario"]["target"]["component"], "U1");
    assert_eq!(paste_ic_pin["scenario"]["parameters"]["pin_pitch_mm"], 0.5);
    assert!(
        paste_ic_pin["reason"]
            .as_str()
            .unwrap()
            .contains("U1 on F.Paste")
    );

    let paste_spacing = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_spacing_valid")
        .expect("paste spacing suggestion");
    assert_eq!(paste_spacing["runnable"], true);
    assert_eq!(
        paste_spacing["scenario"]["checks"][0],
        "SOLDER_PASTE_SPACING_VALID"
    );
    assert_eq!(
        paste_spacing["scenario"]["parameters"]["min_solder_paste_spacing_mm"],
        0.15
    );
    assert!(paste_spacing.get("required_inputs").is_none());
}

#[test]
fn suggest_scenarios_derives_broad_ic_stencil_pitch_template() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_ic_stencil_broad_pitch/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_ic_stencil_broad_pitch"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 4);

    let paste_ic_pin = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_ic_pin_aperture_valid")
        .expect("IC pin paste aperture suggestion");
    assert_eq!(paste_ic_pin["runnable"], true);
    assert_eq!(
        paste_ic_pin["scenario"]["checks"][0],
        "SOLDER_PASTE_IC_PIN_APERTURE_VALID"
    );
    assert_eq!(paste_ic_pin["scenario"]["target"]["component"], "U1");
    assert_eq!(paste_ic_pin["scenario"]["parameters"]["pin_pitch_mm"], 1.0);
    assert!(
        paste_ic_pin["reason"]
            .as_str()
            .unwrap()
            .contains("3 repeated 1.000 mm")
    );
}

#[test]
fn suggest_scenarios_derives_bga_stencil_pitch_template() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_bga_stencil_pitch/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_bga_stencil_pitch"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 4);

    let paste_bga = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_bga_aperture_valid")
        .expect("BGA paste aperture suggestion");
    assert_eq!(paste_bga["runnable"], true);
    assert_eq!(
        paste_bga["scenario"]["checks"][0],
        "SOLDER_PASTE_BGA_APERTURE_VALID"
    );
    assert_eq!(paste_bga["scenario"]["target"]["component"], "U1");
    assert_eq!(paste_bga["scenario"]["parameters"]["pin_pitch_mm"], 0.8);
    assert!(
        paste_bga["reason"]
            .as_str()
            .unwrap()
            .contains("2 horizontal and 2 vertical repeated 0.800 mm")
    );
    assert!(
        suggested
            .iter()
            .all(|suggestion| suggestion["id"] != "solder_paste_ic_pin_aperture_valid")
    );
}

#[test]
fn suggest_scenarios_derives_gpio_backdrive_template() {
    let suggestions = run_suggest_scenarios("examples/scenario_suggestions_backdrive/project.yaml");
    assert_eq!(suggestions["project"], "scenario_suggestions_backdrive");
    let backdrive = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "gpio_backdrive_u2_txd_to_u1_rx")
        .expect("gpio backdrive suggestion");
    assert_eq!(backdrive["kind"], "gpio_backdrive");
    assert_eq!(backdrive["runnable"], false);
    assert_eq!(backdrive["scenario"]["type"], "gpio_backdrive");
    assert_eq!(backdrive["scenario"]["checks"][0], "GPIO_BACKDRIVE");
    assert_eq!(backdrive["scenario"]["pin_states"][0]["component"], "U2");
    assert_eq!(backdrive["scenario"]["pin_states"][0]["pin"], "TXD");
    assert_eq!(backdrive["scenario"]["pin_states"][0]["mode"], "output");
    assert_eq!(backdrive["scenario"]["pin_states"][0]["state"], "high");
    assert_eq!(backdrive["scenario"]["pin_states"][1]["component"], "U1");
    assert_eq!(backdrive["scenario"]["pin_states"][1]["pin"], "RX");
    assert_eq!(backdrive["scenario"]["pin_states"][1]["mode"], "input");
    assert_eq!(
        backdrive["scenario"]["paths"][0]["driver"]["component"],
        "U2"
    );
    assert_eq!(backdrive["scenario"]["paths"][0]["driver"]["pin"], "TXD");
    assert_eq!(backdrive["scenario"]["paths"][0]["net"], "uart_rx");
    assert_eq!(
        backdrive["scenario"]["paths"][0]["victim"]["component"],
        "U1"
    );
    assert_eq!(backdrive["scenario"]["paths"][0]["victim"]["pin"], "RX");
    assert_eq!(
        backdrive["scenario"]["paths"][0]["series_resistance_ohm"],
        0.0
    );
    assert!(
        backdrive["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("driver can be high")
    );
}

#[test]
fn suggest_scenarios_derives_interface_protection_template() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_level_shifter/project.yaml");
    assert_eq!(suggestions["project"], "scenario_suggestions_level_shifter");
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|suggestion| suggestion["kind"] != "gpio_backdrive")
    );
    let protection = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "interface_protection_u3_ch1")
        .expect("interface protection suggestion");
    assert_eq!(protection["kind"], "interface_protection");
    assert_eq!(protection["runnable"], false);
    assert_eq!(protection["scenario"]["type"], "interface_protection");
    assert_eq!(
        protection["scenario"]["checks"][0],
        "INTERFACE_PROTECTION_REVIEW"
    );
    assert_eq!(protection["scenario"]["target"]["component"], "U3");
    let conditioning = &protection["scenario"]["conditioning"];
    assert_eq!(conditioning["component"], "U3");
    assert_eq!(conditioning["channel"], "ch1");
    assert_eq!(conditioning["kind"], "level_shifter");
    assert_eq!(conditioning["direction"], "bidirectional");
    assert_eq!(conditioning["unpowered_isolation"], false);
    assert_eq!(conditioning["side_a"]["pin"], "A1");
    assert_eq!(conditioning["side_a"]["net"], "mcu_rx_shifted");
    assert_eq!(conditioning["side_a"]["supply_pin"], "VCCA");
    assert_eq!(conditioning["side_a"]["supply_net"], "mcu_3v3");
    assert_eq!(conditioning["side_b"]["pin"], "B1");
    assert_eq!(conditioning["side_b"]["net"], "usb_uart_tx");
    assert_eq!(conditioning["side_b"]["supply_pin"], "VCCB");
    assert_eq!(conditioning["side_b"]["supply_net"], "usb_uart_3v3");
    assert!(
        protection["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("datasheet supports")
    );
    let uart = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "uart_bootloader_sync_u1_uart")
        .expect("uart bootloader suggestion");
    assert!(uart["scenario"]["events"][0]["from"].is_null());
    assert!(
        uart["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("output-capable sender")
    );
}

#[test]
fn suggest_scenarios_derives_usb_esd_clamp_templates() {
    let suggestions = run_suggest_scenarios("examples/scenario_suggestions_usb_esd/project.yaml");
    assert_eq!(suggestions["project"], "scenario_suggestions_usb_esd");
    let dp = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "interface_protection_uesd_d1_plus")
        .expect("D+ clamp suggestion");
    assert_eq!(dp["kind"], "interface_protection");
    assert_eq!(dp["runnable"], true);
    assert_eq!(dp["scenario"]["type"], "interface_protection");
    assert_eq!(dp["scenario"]["checks"][0], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(dp["scenario"]["target"]["component"], "UESD");
    assert_eq!(dp["scenario"]["parameters"]["clamp"], "d1_plus");
    let clamp = &dp["scenario"]["protection_clamps"][0];
    assert_eq!(clamp["component"], "UESD");
    assert_eq!(clamp["clamp"], "d1_plus");
    assert_eq!(clamp["protected_pin"], "D1+");
    assert_eq!(clamp["protected_net"], "usb_dp");
    assert_eq!(clamp["reference_pin"], "GND");
    assert_eq!(clamp["reference_net"], "gnd");
    assert_eq!(clamp["reference"], "ground");
    assert_eq!(clamp["working_voltage_max_V"], 5.5);
    assert_eq!(clamp["line_capacitance_F"], 7.0e-13);
    assert!(
        dp["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("max_line_capacitance_F")
    );
    let dm = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "interface_protection_uesd_d1_minus")
        .expect("D- clamp suggestion");
    assert_eq!(dm["scenario"]["parameters"]["clamp"], "d1_minus");
    assert_eq!(
        dm["scenario"]["protection_clamps"][0]["protected_net"],
        "usb_dm"
    );
}

#[test]
fn suggest_scenarios_derives_prtr5v0u2x_rail_to_rail_usb_esd_templates() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_prtr5v0u2x_usb_esd/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_prtr5v0u2x_usb_esd"
    );
    let dp = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "interface_protection_uesd_io1_to_vcc")
        .expect("IO1 clamp suggestion");
    assert_eq!(dp["kind"], "interface_protection");
    assert_eq!(dp["runnable"], true);
    assert_eq!(dp["scenario"]["type"], "interface_protection");
    assert_eq!(dp["scenario"]["checks"][0], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(dp["scenario"]["target"]["component"], "UESD");
    assert_eq!(dp["scenario"]["parameters"]["clamp"], "io1_to_vcc");
    let clamp = &dp["scenario"]["protection_clamps"][0];
    assert_eq!(clamp["component"], "UESD");
    assert_eq!(clamp["clamp"], "io1_to_vcc");
    assert_eq!(clamp["protected_pin"], "IO1");
    assert_eq!(clamp["protected_net"], "usb_dp");
    assert_eq!(clamp["reference_pin"], "VCC");
    assert_eq!(clamp["reference_net"], "usb_vbus");
    assert_eq!(clamp["reference"], "power");
    assert_eq!(clamp["working_voltage_max_V"], 5.5);
    assert_eq!(clamp["line_capacitance_F"], 1.5e-12);
    let dm = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "interface_protection_uesd_io2_to_vcc")
        .expect("IO2 clamp suggestion");
    assert_eq!(dm["scenario"]["parameters"]["clamp"], "io2_to_vcc");
    assert_eq!(
        dm["scenario"]["protection_clamps"][0]["protected_net"],
        "usb_dm"
    );
}

#[test]
fn suggest_scenarios_derives_usb_connector_protection_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_usb_connector_protection/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_usb_connector_protection"
    );
    let connector = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_protection_j1")
        .expect("USB connector protection suggestion");
    assert_eq!(connector["kind"], "interface_protection");
    assert_eq!(connector["runnable"], true);
    assert_eq!(connector["scenario"]["type"], "interface_protection");
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
    assert_eq!(usb["standard"], "usb2");
    assert_eq!(usb["placement"]["x_mm"], 0.0);
    assert_eq!(usb["placement"]["y_mm"], 0.0);
    assert_eq!(usb["placement"]["side"], "top");
    assert_eq!(usb["vbus_pin"], "VBUS");
    assert_eq!(usb["vbus_net"], "usb_vbus");
    assert_eq!(usb["dp_pin"], "D+");
    assert_eq!(usb["dp_net"], "usb_dp");
    assert_eq!(usb["dm_pin"], "D-");
    assert_eq!(usb["dm_net"], "usb_dm");
    assert_eq!(usb["gnd_pin"], "GND");
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
            && clamp["protected_net"] == "usb_dp"
            && clamp["placement"]["x_mm"] == 1.0
    }));
    assert!(clamps.iter().any(|clamp| {
        clamp["component"] == "UESD"
            && clamp["clamp"] == "d1_minus"
            && clamp["protected_net"] == "usb_dm"
            && clamp["placement"]["side"] == "top"
    }));
    assert!(clamps.iter().any(|clamp| {
        clamp["component"] == "UVBUS"
            && clamp["clamp"] == "vbus"
            && clamp["protected_net"] == "usb_vbus"
            && clamp["placement"]["x_mm"] == 1.5
    }));
    assert!(
        connector["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("PCB/layout validation")
    );
    assert!(
        connector["required_inputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|input| input.as_str().unwrap().contains("require_shield_ground"))
    );
    let placement = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_protection_placement_j1")
        .expect("USB protection placement suggestion");
    assert_eq!(placement["kind"], "interface_protection");
    assert_eq!(placement["runnable"], true);
    assert!(placement.get("required_inputs").is_none());
    assert_eq!(placement["scenario"]["type"], "interface_protection");
    assert_eq!(
        placement["scenario"]["checks"][0],
        "USB_PROTECTION_PLACEMENT_VALID"
    );
    assert_eq!(placement["scenario"]["target"]["component"], "J1");
    assert_eq!(
        placement["scenario"]["parameters"]["require_vbus_protection"],
        true
    );
    assert_eq!(
        placement["scenario"]["parameters"]["max_connector_to_protection_distance_mm"],
        2.0
    );
    let placement_clamps = placement["scenario"]["protection_clamps"]
        .as_array()
        .unwrap();
    assert_eq!(placement_clamps.len(), 3);
    assert!(placement_clamps.iter().any(|clamp| {
        clamp["component"] == "UESD"
            && clamp["protected_net"] == "usb_dp"
            && clamp["distance_to_target_mm"] == 1.0
    }));
    assert!(placement_clamps.iter().any(|clamp| {
        clamp["component"] == "UESD"
            && clamp["protected_net"] == "usb_dm"
            && clamp["distance_to_target_mm"] == 1.0
    }));
    assert!(placement_clamps.iter().any(|clamp| {
        clamp["component"] == "UVBUS"
            && clamp["protected_net"] == "usb_vbus"
            && clamp["distance_to_target_mm"] == 1.5
    }));
    let orientation = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_orientation_j1")
        .expect("USB connector orientation suggestion");
    assert_eq!(orientation["kind"], "interface_protection");
    assert_eq!(orientation["runnable"], false);
    assert_eq!(orientation["scenario"]["type"], "interface_protection");
    assert_eq!(
        orientation["scenario"]["checks"][0],
        "USB_CONNECTOR_ORIENTATION_VALID"
    );
    assert!(orientation["scenario"]["parameters"]["expected_connector_rotation_deg"].is_null());
    assert_eq!(
        orientation["scenario"]["parameters"]["max_connector_rotation_error_deg"],
        181.0
    );
    assert_eq!(
        orientation["scenario"]["usb_connectors"][0]["placement"]["rotation_deg"],
        0.0
    );
    assert!(
        orientation["required_inputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|input| input
                .as_str()
                .unwrap()
                .contains("expected_connector_rotation_deg"))
    );
    let route = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_route_geometry_j1")
        .expect("USB route geometry suggestion");
    assert_eq!(route["kind"], "interface_protection");
    assert_eq!(route["runnable"], false);
    assert_eq!(route["scenario"]["type"], "interface_protection");
    assert_eq!(route["scenario"]["checks"][0], "USB_ROUTE_GEOMETRY_VALID");
    assert_eq!(route["scenario"]["target"]["component"], "J1");
    assert!(route["scenario"]["parameters"]["max_data_line_route_length_mm"].is_null());
    assert_eq!(
        route["scenario"]["parameters"]["max_data_line_via_count"],
        1
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_data_line_width_delta_mm"],
        0.02
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_connector_to_protection_route_distance_mm"],
        2.0
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_component_to_route_distance_mm"],
        0.05
    );
    assert!(route["scenario"]["parameters"]["max_data_pair_length_mismatch_mm"].is_null());
    assert_eq!(
        route["scenario"]["parameters"]["max_data_pair_via_count_delta"],
        1
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_data_pair_gap_delta_mm"],
        0.12
    );
    assert_eq!(
        route["scenario"]["parameters"]["require_route_pad_contact_evidence"],
        true
    );
    let routes = route["scenario"]["usb_routes"].as_array().unwrap();
    assert!(routes.iter().any(|usb_route| {
        usb_route["signal"] == "D+"
            && usb_route["net"] == "usb_dp"
            && usb_route["route_length_mm"] == 1.0
            && usb_route["via_count"] == 0
            && usb_route["expected_data_line_width_mm"].is_null()
            && usb_route["measured_data_line_width_mm"].is_null()
            && usb_route["data_line_width_delta_mm"].is_null()
            && usb_route["protection_component"] == "UESD"
    }));
    assert!(routes.iter().any(|usb_route| {
        usb_route["signal"] == "D-"
            && usb_route["net"] == "usb_dm"
            && usb_route["route_length_mm"] == 1.0
            && usb_route["via_count"] == 1
            && usb_route["expected_data_line_width_mm"].is_null()
            && usb_route["measured_data_line_width_mm"].is_null()
            && usb_route["data_line_width_delta_mm"].is_null()
            && usb_route["protection_component"] == "UESD"
    }));
    let route_pairs = route["scenario"]["usb_route_pairs"].as_array().unwrap();
    assert_eq!(route_pairs.len(), 1);
    assert_eq!(route_pairs[0]["dp_net"], "usb_dp");
    assert_eq!(route_pairs[0]["dm_net"], "usb_dm");
    assert_eq!(route_pairs[0]["dp_route_length_mm"], 1.0);
    assert_eq!(route_pairs[0]["dm_route_length_mm"], 1.0);
    assert_eq!(route_pairs[0]["data_pair_length_mismatch_mm"], 0.0);
    assert_eq!(route_pairs[0]["dp_via_count"], 0);
    assert_eq!(route_pairs[0]["dm_via_count"], 1);
    assert_eq!(route_pairs[0]["data_pair_via_count_delta"], 1);
    assert!(route_pairs[0]["expected_data_pair_gap_mm"].is_null());
    assert!(route_pairs[0]["measured_data_pair_gap_mm"].is_null());
    assert!(route_pairs[0]["data_pair_gap_delta_mm"].is_null());
    assert!(
        route["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("max_data_line_route_length_mm")
    );
    let vbus_route = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_vbus_route_j1")
        .expect("USB VBUS route suggestion");
    assert_eq!(vbus_route["kind"], "interface_protection");
    assert_eq!(vbus_route["runnable"], true);
    assert!(vbus_route.get("required_inputs").is_none());
    assert_eq!(vbus_route["scenario"]["type"], "interface_protection");
    assert_eq!(vbus_route["scenario"]["checks"][0], "USB_VBUS_ROUTE_VALID");
    assert_eq!(vbus_route["scenario"]["target"]["component"], "J1");
    assert_eq!(
        vbus_route["scenario"]["parameters"]["max_vbus_route_length_mm"],
        20.0
    );
    assert!(vbus_route["scenario"]["parameters"]["max_vbus_via_count"].is_null());
    assert_eq!(
        vbus_route["scenario"]["parameters"]["min_vbus_route_width_mm"],
        0.30
    );
    assert!(
        vbus_route["scenario"]["parameters"]["max_connector_to_vbus_protection_route_distance_mm"]
            .is_null()
    );
    assert!(vbus_route["scenario"]["parameters"]["max_component_to_route_distance_mm"].is_null());
    assert!(
        vbus_route["scenario"]["parameters"]["require_vbus_route_pad_contact_evidence"].is_null()
    );
    let vbus_routes = vbus_route["scenario"]["usb_routes"].as_array().unwrap();
    assert_eq!(vbus_routes.len(), 1);
    assert_eq!(vbus_routes[0]["signal"], "VBUS");
    assert_eq!(vbus_routes[0]["net"], "usb_vbus");
    assert_eq!(vbus_routes[0]["route_length_mm"], 1.5);
    assert_eq!(vbus_routes[0]["via_count"], 0);
    assert_eq!(vbus_routes[0]["expected_vbus_route_width_mm"], 0.30);
    assert_eq!(vbus_routes[0]["measured_vbus_route_width_min_mm"], 0.30);
    assert_eq!(vbus_routes[0]["protection_component"], "UVBUS");
}

#[test]
fn suggest_scenarios_uses_usb_connector_entry_direction_offset() {
    let suggestions = run_suggest_scenarios(
        "examples/bad_usb_connector_entry_clearance_model_offset/project_suggestions.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_usb_connector_entry_offset"
    );
    let entry = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_entry_clearance_j1")
        .expect("USB connector entry-clearance suggestion");
    assert_eq!(entry["runnable"], false);
    assert_eq!(
        entry["scenario"]["checks"][0],
        "USB_CONNECTOR_ENTRY_CLEARANCE_VALID"
    );
    assert_eq!(entry["scenario"]["parameters"]["entry_direction_deg"], 0.0);
    let entry_evidence = &entry["scenario"]["usb_connectors"][0]["entry_clearance"];
    assert_eq!(entry_evidence["entry_direction_deg"], 0.0);
    assert_eq!(
        entry_evidence["entry_direction_source"],
        "component_model_offset"
    );
    assert_eq!(entry_evidence["entry_direction_offset_deg"], 90.0);
    assert_eq!(entry_evidence["nearest_obstruction"]["component"], "R1");
    assert_eq!(
        entry_evidence["nearest_obstruction"]["obstruction_depth_mm"],
        0.75
    );
    assert_eq!(
        entry_evidence["nearest_obstruction"]["obstruction_reference"],
        "footprint_rectangle"
    );
    let orientation = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_orientation_j1")
        .expect("USB connector orientation suggestion");
    assert_eq!(
        orientation["scenario"]["parameters"]["expected_connector_rotation_deg"],
        270.0
    );
    let nearest_edge = &orientation["scenario"]["usb_connectors"][0]["nearest_board_edge"];
    assert_eq!(nearest_edge["outward_normal_deg"], 0.0);
    assert_eq!(nearest_edge["expected_connector_rotation_deg"], 270.0);
    assert_eq!(nearest_edge["connector_entry_direction_offset_deg"], 90.0);
    assert_eq!(nearest_edge["connector_rotation_error_deg"], 0.0);
}

#[test]
fn suggest_scenarios_reports_usb_connector_entry_aperture() {
    let suggestions = run_suggest_scenarios(
        "examples/bad_usb_connector_entry_clearance_aperture/project_suggestions.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_usb_connector_entry_aperture"
    );
    let entry = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_entry_clearance_j1")
        .expect("USB connector entry-clearance suggestion");
    assert_eq!(entry["runnable"], true);
    assert!(entry.get("required_inputs").is_none());
    let entry_evidence = &entry["scenario"]["usb_connectors"][0]["entry_clearance"];
    assert_eq!(
        entry_evidence["entry_aperture_source"],
        "component_model_aperture"
    );
    assert_eq!(entry_evidence["connector_front_projection_mm"], 0.5);
    assert_eq!(entry_evidence["entry_aperture_front_projection_mm"], 0.75);
    assert_eq!(
        entry_evidence["entry_aperture_center_lateral_projection_mm"],
        1.0
    );
    assert_eq!(entry_evidence["entry_aperture_front_offset_mm"], 0.25);
    assert_eq!(entry_evidence["entry_aperture_lateral_offset_mm"], 1.0);
    assert_eq!(entry_evidence["entry_aperture_width_mm"], 0.5);
    assert_eq!(
        entry_evidence["entry_clearance_depth_source"],
        "component_model_depth"
    );
    assert_eq!(
        entry_evidence["suggested_min_cable_entry_clearance_depth_mm"],
        1.5
    );
    assert_eq!(
        entry_evidence["entry_clearance_width_source"],
        "component_model_width"
    );
    assert_eq!(
        entry_evidence["suggested_cable_entry_clearance_width_mm"],
        0.8
    );
    assert_eq!(
        entry["scenario"]["parameters"]["min_cable_entry_clearance_depth_mm"],
        1.5
    );
    assert_eq!(
        entry["scenario"]["parameters"]["cable_entry_clearance_width_mm"],
        0.8
    );
    assert_eq!(
        entry_evidence["aperture_min_effective_clearance_width_mm"],
        0.5
    );
    assert_eq!(entry_evidence["nearest_obstruction"]["component"], "R1");
    assert!(
        (entry_evidence["nearest_obstruction"]["obstruction_depth_mm"]
            .as_f64()
            .unwrap()
            - 0.15)
            .abs()
            < 1.0e-12
    );
}

#[test]
fn suggest_scenarios_derives_boot_strap_bias_template() {
    let suggestions = run_suggest_scenarios("examples/good_bootstrap_bias_divider/project.yaml");
    assert_eq!(suggestions["project"], "good_bootstrap_bias_divider");
    let bias = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "boot_strap_bias_valid_u1_application")
        .expect("boot strap bias suggestion");
    assert_eq!(bias["kind"], "reset_boot");
    assert_eq!(bias["runnable"], true);
    assert_eq!(bias["scenario"]["type"], "reset_boot");
    assert_eq!(bias["scenario"]["checks"][0], "BOOT_STRAP_BIAS_VALID");
    assert_eq!(bias["scenario"]["target"]["component"], "U1");
    assert_eq!(bias["scenario"]["required_boot_mode"], "application");
    assert!(bias.get("required_inputs").is_none());
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|suggestion| suggestion["id"] != "boot_strap_bias_valid_u1_bootloader")
    );
    let observed = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "boot_strap_defined_u1_bootloader")
        .expect("observed boot strap suggestion");
    assert_eq!(observed["runnable"], false);
    assert_eq!(observed["scenario"]["checks"][0], "BOOT_STRAP_DEFINED");
}

#[test]
fn suggest_scenarios_derives_reset_release_from_rc_network() {
    let suggestions = run_suggest_scenarios("examples/scenario_suggestions_reset_rc/project.yaml");
    assert_eq!(suggestions["project"], "scenario_suggestions_reset_rc");
    let reset = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "reset_release_after_power_valid_u1")
        .expect("reset release suggestion");
    assert_eq!(reset["kind"], "reset_boot");
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
    let boot_sample_at_us = reset["scenario"]["timing"]["boot_sample_at_us"]
        .as_f64()
        .unwrap();
    assert!((boot_sample_at_us - 2531.558204).abs() < 0.001);
    assert!(reset.get("required_inputs").is_none());
    assert!(
        reset["reason"]
            .as_str()
            .unwrap()
            .contains("explicit RC reset evidence from R1 and C1")
    );
}

#[test]
fn suggest_scenarios_derives_clock_source_template() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_clock_source/project.yaml");
    assert_eq!(suggestions["project"], "scenario_suggestions_clock_source");
    let clock = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "clock_source_valid_u1")
        .expect("clock source suggestion");
    assert_eq!(clock["kind"], "clock");
    assert_eq!(clock["runnable"], true);
    assert_eq!(clock["confidence"], "medium");
    assert_eq!(clock["scenario"]["type"], "clock");
    assert_eq!(clock["scenario"]["checks"][0], "CLOCK_SOURCE_VALID");
    assert_eq!(clock["scenario"]["target"]["component"], "U1");
    let clock_evidence = &clock["scenario"]["clocks"][0];
    assert_eq!(clock_evidence["component"], "U1");
    assert_eq!(clock_evidence["name"], "hse");
    assert_eq!(clock_evidence["input_pin"], "OSC_IN");
    assert_eq!(clock_evidence["input_net"], "osc_in");
    assert_eq!(clock_evidence["output_pin"], "OSC_OUT");
    assert_eq!(clock_evidence["output_net"], "osc_out");
    assert_eq!(clock_evidence["crystal_component"], "Y1");
    assert!(clock.get("required_inputs").is_none());
}

#[test]
fn suggest_scenarios_reports_reset_supervisor_evidence() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_reset_supervisor/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_reset_supervisor"
    );
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "power_tree_valid")
        .expect("power tree suggestion");
    assert_eq!(power_tree["kind"], "power_tree");
    assert_eq!(power_tree["runnable"], true);
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
    let supervisor = &power_tree["scenario"]["reset_supervisors"][0];
    assert_eq!(supervisor["component"], "USUP");
    assert_eq!(supervisor["monitored_pin"], "VDD");
    assert_eq!(supervisor["monitored_net"], "rail_3v3");
    assert_eq!(supervisor["reset_output_pin"], "RESET");
    assert_eq!(supervisor["reset_net"], "nrst");
    assert_eq!(supervisor["threshold_min_V"], 2.93);
    assert_eq!(supervisor["threshold_max_V"], 3.08);
}
