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
