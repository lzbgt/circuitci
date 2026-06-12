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
    assert_eq!(suggestions["suggestions"].as_array().unwrap().len(), 5);
    let power_tree = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["kind"] == "power_tree")
        .expect("power_tree suggestion");
    assert_eq!(power_tree["runnable"], true);
    assert_eq!(power_tree["scenario"]["type"], "power_tree");
    assert_eq!(power_tree["scenario"]["checks"][0], "POWER_TREE_VALID");
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
