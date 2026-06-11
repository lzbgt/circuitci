use serde_json::Value;
use std::process::Command;
use walkdir::WalkDir;

#[test]
fn bad_backdrive_board_fails_with_gpio_backdrive() {
    let out_dir = tempfile::tempdir().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            "examples/bad_backdrive_board/project.yaml",
            "--profile",
            "iot_basic_v0",
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_eq!(report["summary"]["critical"], 1);
    assert_eq!(report["failures"][0]["id"], "GPIO_BACKDRIVE");
    assert_eq!(report["failures"][0]["net"], "uart_rx");
    assert_report_schema_valid(&report);
}

#[test]
fn fixed_backdrive_board_passes() {
    let report = run_validation("examples/good_backdrive_fixed_board/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_eq!(report["failures"].as_array().unwrap().len(), 0);
    assert_report_schema_valid(&report);
}

#[test]
fn good_bootloader_board_passes_reset_boot_and_sync() {
    let report = run_validation("examples/good_bootloader_board/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn c51_style_bootloader_board_uses_model_metadata() {
    let report = run_validation("examples/good_c51_isp_board/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn reset_release_before_power_valid_fails() {
    let report = run_validation("examples/bad_reset_release_board/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "RESET_RELEASE_AFTER_POWER_VALID"
    );
    assert_eq!(report["failures"][0]["component"], "U1");
    assert_report_schema_valid(&report);
}

#[test]
fn wrong_bootstrap_state_fails() {
    let report = run_validation("examples/bad_bootstrap_board/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "BOOT_STRAP_DEFINED");
    assert_eq!(report["failures"][0]["limit"]["required_BOOT0"], "high");
    assert_report_schema_valid(&report);
}

#[test]
fn missing_uart_bootloader_sync_fails() {
    let report = run_validation("examples/bad_uart_bootloader_sync_board/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "UART_BOOTLOADER_SYNC");
    assert_eq!(report["failures"][0]["limit"]["sync_byte"], 127);
    assert_report_schema_valid(&report);
}

#[test]
fn wrong_reset_target_pin_fails() {
    let report = run_validation("examples/bad_reset_pin_board/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "TARGET_RESET_PIN_MISMATCH");
    assert_eq!(report["failures"][0]["limit"]["model_reset_pin"], "NRST");
    assert_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_rom_download_entry_passes() {
    let report = run_validation("examples/um_stm32l4_rom_download_entry/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation["id"] == "LOW_CONFIDENCE_MODEL")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_wrong_uart_wiring_fails() {
    let report = run_validation("examples/um_stm32l4_rom_download_wrong_uart/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "UART_BOOTLOADER_SYNC");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("not connected to target RX")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_bad_app_boot_release_fails() {
    let report = run_validation("examples/um_stm32l4_app_boot_bad_release/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "BOOT_STRAP_DEFINED");
    assert_eq!(report["failures"][0]["limit"]["required_BOOT0"], "low");
    assert_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_fixed_app_boot_release_passes() {
    let report = run_validation("examples/um_stm32l4_app_boot_fixed_release/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_resident_update_activate_passes() {
    let report = run_validation("examples/um_stm32l4_resident_update_activate/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation["id"] == "ABSTRACT_PROTOCOL_TRACE")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_resident_update_missing_finish_fails() {
    let report = run_validation("examples/um_stm32l4_resident_update_missing_finish/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "RESIDENT_BOOTLOADER_UPDATE_SEQUENCE"
    );
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("expected operation finish")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_resident_update_oversize_chunk_fails() {
    let report = run_validation("examples/um_stm32l4_resident_update_oversize_chunk/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "RESIDENT_BOOTLOADER_UPDATE_SEQUENCE"
    );
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("payload length is outside model limits")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_resident_update_wrong_sender_fails() {
    let report = run_validation("examples/um_stm32l4_resident_update_wrong_sender/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "RESIDENT_BOOTLOADER_UPDATE_SEQUENCE"
    );
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("not connected to target RX")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn example_projects_and_component_models_match_schemas() {
    let board_schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let model_schema: Value =
        serde_json::from_str(include_str!("../schemas/component_model.schema.json")).unwrap();
    let board_validator = jsonschema::validator_for(&board_schema).unwrap();
    let model_validator = jsonschema::validator_for(&model_schema).unwrap();

    for entry in WalkDir::new("examples").into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && entry.file_name() == "project.yaml" {
            assert_yaml_file_valid(entry.path(), &board_validator);
        }
    }
    for entry in WalkDir::new("libs").into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(".model.yaml"))
        {
            assert_yaml_file_valid(entry.path(), &model_validator);
        }
    }
}

fn run_validation(project: &str) -> Value {
    let out_dir = tempfile::tempdir().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            project,
            "--profile",
            "iot_basic_v0",
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    serde_json::from_str(&std::fs::read_to_string(out_dir.path().join("report.json")).unwrap())
        .unwrap()
}

fn assert_yaml_file_valid(path: &std::path::Path, validator: &jsonschema::Validator) {
    let yaml = std::fs::read_to_string(path).unwrap();
    let value: Value = serde_yaml_ng::from_str(&yaml).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "{} schema errors: {errors:#?}",
        path.display()
    );
}

fn assert_report_schema_valid(report: &Value) {
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/report.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(report)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "report schema errors: {errors:#?}");
}
