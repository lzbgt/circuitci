mod common;

use common::{
    assert_report_schema_valid, assert_suite_report_schema_valid, assert_yaml_file_valid,
    binary_available, run_validation,
};
use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
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
fn good_power_tree_board_passes() {
    let report = run_validation("examples/good_power_tree_board/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn power_tree_overvoltage_fails() {
    let report = run_validation("examples/bad_power_tree_overvoltage/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "POWER_TREE_VALID");
    assert_eq!(report["failures"][0]["component"], "U1");
    assert_eq!(report["failures"][0]["net"], "rail_5v");
    assert_eq!(report["failures"][0]["measured"]["nominal_voltage_V"], 5.0);
    assert_eq!(
        report["failures"][0]["limit"]["operating_voltage_maximum_V"],
        3.6
    );
    assert_report_schema_valid(&report);
}

#[test]
fn power_tree_current_budget_fails() {
    let report = run_validation("examples/bad_power_tree_current_budget/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "POWER_TREE_VALID");
    assert_eq!(report["failures"][0]["net"], "rail_3v3");
    assert_eq!(
        report["failures"][0]["measured"]["declared_load_current_A"],
        0.05
    );
    assert_eq!(
        report["failures"][0]["limit"]["supply_current_limit_A"],
        0.04
    );
    assert_report_schema_valid(&report);
}

#[test]
fn good_regulator_power_tree_passes() {
    let report = run_validation("examples/good_regulator_power_tree/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_dropout_fails() {
    let report = run_validation("examples/bad_regulator_dropout/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("dropout_voltage_V").is_some())
        .expect("expected regulator dropout finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["input_voltage_V"], 3.4);
    assert_eq!(failure["measured"]["output_voltage_V"], 3.3);
    assert_eq!(failure["limit"]["dropout_voltage_V"], 0.3);
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_output_current_fails() {
    let report = run_validation("examples/bad_regulator_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_max_output_current_A")
                .is_some()
        })
        .expect("expected regulator output current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 0.05);
    assert_eq!(failure["limit"]["regulator_max_output_current_A"], 0.04);
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_conversion_metadata_fails_closed() {
    let report = run_validation("examples/bad_regulator_conversion_pin/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("power_conversion_field").is_some())
        .expect("expected power_conversion metadata finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["limit"]["power_conversion_field"], "output_pin");
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
fn um_stm32l4_control_line_bad_release_fails() {
    let report = run_validation("examples/um_stm32l4_control_line_app_release_bad/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "CONTROL_LINE_RELEASE_SEQUENCE");
    assert_eq!(report["failures"][0]["measured"]["derived_BOOT0"], "high");
    assert_eq!(report["failures"][0]["limit"]["required_BOOT0"], "low");
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation["id"] == "ABSTRACT_CONTROL_LINE_MODEL")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_control_line_fixed_release_passes() {
    let report = run_validation("examples/um_stm32l4_control_line_app_release_fixed/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_control_line_rom_entry_passes() {
    let report = run_validation("examples/um_stm32l4_control_line_rom_entry/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn control_line_rejects_kicad_input_source_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("bad_kicad_source_direction.project.yaml");
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        control_line_direction_project(
            &repo.join("libs/generic").display().to_string(),
            "TXD: input",
            "NRST: input",
        ),
    )
    .unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "CONTROL_LINE_RELEASE_SEQUENCE");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("source U5.TXD has KiCad electrical type input, which is not output-capable")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn control_line_rejects_kicad_output_target_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("bad_kicad_target_direction.project.yaml");
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        control_line_direction_project(
            &repo.join("libs/generic").display().to_string(),
            "TXD: output",
            "NRST: output",
        ),
    )
    .unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "CONTROL_LINE_RELEASE_SEQUENCE");
    assert!(
        report["failures"][0]["message"].as_str().unwrap().contains(
            "target U1.NRST has KiCad electrical type output, which is not input-capable"
        )
    );
    assert_report_schema_valid(&report);
}

#[test]
fn uart_sync_rejects_kicad_input_sender_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir
        .path()
        .join("bad_uart_kicad_sender_direction.project.yaml");
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        uart_direction_project(
            &repo.join("libs/generic").display().to_string(),
            "TXD: input",
            "RX: input",
        ),
    )
    .unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "UART_BOOTLOADER_SYNC");
    assert!(report["failures"][0]["message"].as_str().unwrap().contains(
        "sender endpoint U2.TXD has KiCad electrical type input, which is not output-capable"
    ));
    assert_report_schema_valid(&report);
}

#[test]
fn uart_sync_rejects_kicad_output_target_rx_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir
        .path()
        .join("bad_uart_kicad_target_direction.project.yaml");
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        uart_direction_project(
            &repo.join("libs/generic").display().to_string(),
            "TXD: output",
            "RX: output",
        ),
    )
    .unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "UART_BOOTLOADER_SYNC");
    assert!(
        report["failures"][0]["message"].as_str().unwrap().contains(
            "target RX U1.RX has KiCad electrical type output, which is not input-capable"
        )
    );
    assert_report_schema_valid(&report);
}

#[test]
fn resident_protocol_rejects_kicad_input_sender_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir
        .path()
        .join("bad_resident_kicad_sender_direction.project.yaml");
    let repo = std::env::current_dir().unwrap();
    let text = std::fs::read_to_string("examples/um_stm32l4_resident_update_activate/project.yaml")
        .unwrap()
        .replace(
            "../../libs/vendor/um",
            &repo.join("libs/vendor/um").display().to_string(),
        )
        .replace(
            "../../libs/vendor/wch",
            &repo.join("libs/vendor/wch").display().to_string(),
        )
        .replace(
            "        RXD: usart1_tx\n",
            "        RXD: usart1_tx\n      source:\n        board_pin_electrical_types:\n          TXD: input\n",
        );
    std::fs::write(&project, text).unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "RESIDENT_BOOTLOADER_UPDATE_SEQUENCE"
    );
    assert!(report["failures"][0]["message"].as_str().unwrap().contains(
        "sender endpoint U5.TXD has KiCad electrical type input, which is not output-capable"
    ));
    assert_report_schema_valid(&report);
}

#[test]
fn backdrive_rejects_kicad_input_driver_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir
        .path()
        .join("bad_backdrive_kicad_driver_direction.project.yaml");
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        backdrive_direction_project(
            &repo.join("libs/generic").display().to_string(),
            "TXD: input",
            "RX: input",
        ),
    )
    .unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "GPIO_BACKDRIVE");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("driver U2.TXD has KiCad electrical type input, which is not output-capable")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn backdrive_rejects_kicad_output_victim_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir
        .path()
        .join("bad_backdrive_kicad_victim_direction.project.yaml");
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        backdrive_direction_project(
            &repo.join("libs/generic").display().to_string(),
            "TXD: output",
            "RX: output",
        ),
    )
    .unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "GPIO_BACKDRIVE");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("victim U1.RX has KiCad electrical type output, which is not input-capable")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn functional_mcu_firmware_check_fails_closed_without_backend() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("firmware_blackbox.project.yaml");
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        firmware_functional_project(&repo.join("libs/generic").display().to_string(), true),
    )
    .unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "FUNCTIONAL_MCU_FIRMWARE");
    assert_eq!(report["failures"][0]["component"], "U1");
    assert_eq!(report["failures"][0]["measured"]["target_component"], "U1");
    assert_eq!(
        report["failures"][0]["measured"]["target_model"],
        "generic.mcu.basic"
    );
    assert_eq!(report["failures"][0]["measured"]["backend"], "auto");
    assert_eq!(
        report["failures"][0]["measured"]["firmware_image"],
        "firmware/app.elf"
    );
    assert_eq!(report["failures"][0]["measured"]["expected_pin_states"], 1);
    assert_eq!(
        report["failures"][0]["limit"]["functional_blackbox_boundary"],
        "firmware-visible peripherals and board-facing pin behavior"
    );
    assert_eq!(
        report["failures"][0]["limit"]["transistor_level_mcu_required"],
        false
    );
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("No functional MCU firmware backend is selectable")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn functional_mcu_firmware_requires_expected_pin_behavior() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir
        .path()
        .join("firmware_missing_pin_behavior.project.yaml");
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        firmware_functional_project(&repo.join("libs/generic").display().to_string(), false),
    )
    .unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "VALIDATION_INPUT_MISSING");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("expected_pin_states")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn functional_mcu_qemu_backend_passes_with_matching_pin_trace() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("firmware_qemu.project.yaml");
    let output = dir.path().join("out");
    let qemu = write_fake_qemu(
        dir.path(),
        "CIRCUITCI_PIN U1.TX mode=output state=high\n",
        0,
    );
    write_dummy_firmware(dir.path());
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        firmware_qemu_project(
            &repo.join("libs/generic").display().to_string(),
            &qemu.display().to_string(),
            "high",
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            project.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    let qemu_log = report["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with("qemu.log"))
        .expect("qemu.log artifact");
    let qemu_log_text = std::fs::read_to_string(qemu_log).unwrap();
    assert!(qemu_log_text.contains("CIRCUITCI_PIN U1.TX mode=output state=high"));
    assert_report_schema_valid(&report);
}

#[test]
fn functional_mcu_qemu_backend_fails_on_pin_trace_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("firmware_qemu_mismatch.project.yaml");
    let qemu = write_fake_qemu(dir.path(), "CIRCUITCI_PIN U1.TX mode=output state=low\n", 0);
    write_dummy_firmware(dir.path());
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        firmware_qemu_project(
            &repo.join("libs/generic").display().to_string(),
            &qemu.display().to_string(),
            "high",
        ),
    )
    .unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "FUNCTIONAL_MCU_FIRMWARE");
    assert_eq!(report["failures"][0]["measured"]["observed_state"], "low");
    assert_eq!(report["failures"][0]["limit"]["expected_state"], "high");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Observed QEMU pin behavior did not match")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn functional_mcu_qemu_backend_runs_declared_firmware_build() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("firmware_qemu_build.project.yaml");
    let output = dir.path().join("out");
    let qemu = write_fake_qemu(
        dir.path(),
        "CIRCUITCI_PIN U1.TX mode=output state=high\n",
        0,
    );
    write_fake_build_script(
        dir.path(),
        "mkdir -p firmware\nprintf built > firmware/app.elf\n",
    );
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        firmware_qemu_build_project(
            &repo.join("libs/generic").display().to_string(),
            &qemu.display().to_string(),
            "firmware/app.elf",
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            project.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "pass");
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("firmware_build.log"))
    );
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("firmware/app.elf"))
    );
    assert_report_schema_valid(&report);
}

#[test]
fn functional_mcu_firmware_build_requires_declared_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir
        .path()
        .join("firmware_qemu_missing_build_output.project.yaml");
    let qemu = write_fake_qemu(
        dir.path(),
        "CIRCUITCI_PIN U1.TX mode=output state=high\n",
        0,
    );
    write_fake_build_script(dir.path(), "mkdir -p firmware\n");
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &project,
        firmware_qemu_build_project(
            &repo.join("libs/generic").display().to_string(),
            &qemu.display().to_string(),
            "firmware/app.elf",
        ),
    )
    .unwrap();

    let report = run_validation(project.to_str().unwrap());
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "FUNCTIONAL_MCU_FIRMWARE");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("firmware.build output")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn c51_control_line_release_uses_generic_reset_polarity() {
    let report = run_validation("examples/good_c51_control_line_app_release/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}
#[test]
fn example_projects_and_component_models_match_schemas() {
    let board_schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let model_schema: Value =
        serde_json::from_str(include_str!("../schemas/component_model.schema.json")).unwrap();
    let suite_schema: Value =
        serde_json::from_str(include_str!("../schemas/suite_manifest.schema.json")).unwrap();
    let board_validator = jsonschema::validator_for(&board_schema).unwrap();
    let model_validator = jsonschema::validator_for(&model_schema).unwrap();
    let suite_validator = jsonschema::validator_for(&suite_schema).unwrap();

    for entry in WalkDir::new("examples").into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && entry.file_name() == "project.yaml" {
            assert_yaml_file_valid(entry.path(), &board_validator);
        }
    }
    for entry in WalkDir::new("libs")
        .into_iter()
        .chain(WalkDir::new("examples").into_iter())
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(".model.yaml"))
        {
            assert_yaml_file_valid(entry.path(), &model_validator);
        }
    }
    for entry in WalkDir::new("suites").into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(".yaml"))
        {
            assert_yaml_file_valid(entry.path(), &suite_validator);
        }
    }
}

#[test]
fn um_stm32l4_acceptance_suite_passes() {
    let out_dir = tempfile::tempdir().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate-suite",
            "suites/um_stm32l4_downloader_acceptance.yaml",
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["suite"], "um_stm32l4_downloader_acceptance");
    assert_eq!(report["validation_profile"], "iot_basic_v0");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["cases"], 12);
    assert_eq!(report["summary"]["failed"], 0);
    assert_eq!(report["summary"]["repairs"], 7);
    assert_eq!(report["summary"]["repairs_failed"], 0);
    assert_eq!(report["repairs"][0]["id"], "fix_backdrive");
    assert_eq!(report["repairs"][0]["result"], "pass");
    assert_eq!(
        report["repairs"][0]["matched_findings"][0]["id"],
        "GPIO_BACKDRIVE"
    );
    assert!(
        !report["repairs"][0]["suggested_fixes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        out_dir
            .path()
            .join("cases/control_line_bad_release_detected/report.json")
            .exists()
    );
    assert_suite_report_schema_valid(&report);
}

#[test]
fn um_stm32l4_physical_acceptance_suite_reports_spice_failure() {
    if !binary_available("ngspice") {
        return;
    }
    let out_dir = tempfile::tempdir().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate-suite",
            "suites/um_stm32l4_downloader_physical_acceptance.yaml",
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["suite"], "um_stm32l4_downloader_physical_acceptance");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["cases"], 1);
    assert_eq!(report["cases"][0]["actual"], "fail");
    assert_eq!(
        report["cases"][0]["matched_findings"][0]["id"],
        "SPICE_TRANSIENT_ANALYSIS"
    );
    assert_suite_report_schema_valid(&report);
}

#[test]
fn suite_expectation_mismatch_exits_nonzero_after_report() {
    let suite_dir = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let project_path = std::env::current_dir()
        .unwrap()
        .join("examples/good_backdrive_fixed_board/project.yaml");
    let manifest = suite_dir.path().join("mismatch.yaml");
    std::fs::write(
        &manifest,
        format!(
            "suite:\n  name: mismatch_suite\n  version: 0.1.0\n  validation_profile: iot_basic_v0\ncases:\n  - id: expected_failure\n    project: {}\n    expect: fail\n    required_findings:\n      - id: GPIO_BACKDRIVE\n        severity: critical\n",
            project_path.display()
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate-suite",
            manifest.to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_eq!(report["summary"]["failed"], 1);
    assert!(
        report["cases"][0]["messages"][0]
            .as_str()
            .unwrap()
            .contains("Expected project result fail")
    );
    assert_suite_report_schema_valid(&report);
}

#[test]
fn suite_repair_missing_finding_exits_nonzero_after_report() {
    let suite_dir = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let bad_project = std::env::current_dir()
        .unwrap()
        .join("examples/bad_backdrive_board/project.yaml");
    let fixed_project = std::env::current_dir()
        .unwrap()
        .join("examples/good_backdrive_fixed_board/project.yaml");
    let manifest = suite_dir.path().join("bad_repair.yaml");
    std::fs::write(
        &manifest,
        format!(
            "suite:\n  name: bad_repair_suite\n  version: 0.1.0\n  validation_profile: iot_basic_v0\ncases:\n  - id: detect\n    project: {}\n    expect: fail\n    required_findings:\n      - id: GPIO_BACKDRIVE\n        severity: critical\n  - id: fixed\n    project: {}\n    expect: pass\nrepairs:\n  - id: wrong_rule\n    detects_case: detect\n    fixed_case: fixed\n    fixes_findings:\n      - BOOT_STRAP_DEFINED\n",
            bad_project.display(),
            fixed_project.display()
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate-suite",
            manifest.to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_eq!(report["summary"]["repairs_failed"], 1);
    assert!(
        report["repairs"][0]["messages"][0]
            .as_str()
            .unwrap()
            .contains("does not contain critical finding BOOT_STRAP_DEFINED")
    );
    assert_suite_report_schema_valid(&report);
}

#[test]
fn suite_rejects_duplicate_case_ids() {
    let suite_dir = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let project_path = std::env::current_dir()
        .unwrap()
        .join("examples/good_backdrive_fixed_board/project.yaml");
    let manifest = suite_dir.path().join("duplicate.yaml");
    std::fs::write(
        &manifest,
        format!(
            "suite:\n  name: duplicate_suite\n  version: 0.1.0\n  validation_profile: iot_basic_v0\ncases:\n  - id: same\n    project: {}\n    expect: pass\n  - id: same\n    project: {}\n    expect: pass\n",
            project_path.display(),
            project_path.display()
        ),
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate-suite",
            manifest.to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!out_dir.path().join("report.json").exists());
}

#[test]
fn suite_rejects_duplicate_repair_ids() {
    let suite_dir = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let bad_project = std::env::current_dir()
        .unwrap()
        .join("examples/bad_backdrive_board/project.yaml");
    let fixed_project = std::env::current_dir()
        .unwrap()
        .join("examples/good_backdrive_fixed_board/project.yaml");
    let manifest = suite_dir.path().join("duplicate_repair.yaml");
    std::fs::write(
        &manifest,
        format!(
            "suite:\n  name: duplicate_repair_suite\n  version: 0.1.0\n  validation_profile: iot_basic_v0\ncases:\n  - id: detect\n    project: {}\n    expect: fail\n    required_findings:\n      - id: GPIO_BACKDRIVE\n        severity: critical\n  - id: fixed\n    project: {}\n    expect: pass\nrepairs:\n  - id: same\n    detects_case: detect\n    fixed_case: fixed\n    fixes_findings:\n      - GPIO_BACKDRIVE\n  - id: same\n    detects_case: detect\n    fixed_case: fixed\n    fixes_findings:\n      - GPIO_BACKDRIVE\n",
            bad_project.display(),
            fixed_project.display()
        ),
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate-suite",
            manifest.to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!out_dir.path().join("report.json").exists());
}

#[test]
fn suite_rejects_unknown_manifest_fields() {
    let suite_dir = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let project_path = std::env::current_dir()
        .unwrap()
        .join("examples/good_backdrive_fixed_board/project.yaml");
    let manifest = suite_dir.path().join("unknown_field.yaml");
    std::fs::write(
        &manifest,
        format!(
            "suite:\n  name: unknown_field_suite\n  version: 0.1.0\n  validation_profile: iot_basic_v0\ncases:\n  - id: fixed\n    project: {}\n    expect: pass\nignored_by_old_runtime: true\n",
            project_path.display()
        ),
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate-suite",
            manifest.to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!out_dir.path().join("report.json").exists());
}

fn control_line_direction_project(
    library_path: &str,
    source_direction: &str,
    target_direction: &str,
) -> String {
    format!(
        r#"project:
  name: bad_kicad_direction_metadata
  version: 0.1.0
libraries:
  - {library_path}
board:
  components:
    U1:
      model: generic.mcu.basic
      power_domains:
        VDD: vdd_3v3
      pins:
        VDD: vdd_3v3
        GND: gnd
        NRST: nrst
        BOOT0: boot0
      source:
        board_pin_electrical_types:
          {target_direction}
    U5:
      model: generic.usb_uart.basic
      power_domains:
        VCC: vdd_3v3
      pins:
        VCC: vdd_3v3
        GND: gnd
        TXD: nrst
      source:
        board_pin_electrical_types:
          {source_direction}
  nets:
    vdd_3v3:
      kind: power
      nominal_voltage: 3.3
      powered: true
    gnd:
      kind: ground
    nrst:
      kind: digital_or_analog
    boot0:
      kind: digital_or_analog
scenarios:
  - name: imported_direction_metadata
    type: control_line_sequence
    target:
      component: U1
      power_pin: VDD
      reset_pin: NRST
    checks:
      - CONTROL_LINE_RELEASE_SEQUENCE
    required_boot_mode: application
    timing:
      power_valid_at_us: 100
      reset_release_at_us: 200
      boot_sample_at_us: 300
    control_effects:
      - name: reset
        source:
          component: U5
          pin: TXD
        target:
          component: U1
          pin: NRST
        asserted_state: low
        released_state: high
        release_delay_us: 0
    events:
      - at_us: 0
        action: control_line
        line: reset
        asserted: false
"#
    )
}

fn uart_direction_project(
    library_path: &str,
    sender_direction: &str,
    target_direction: &str,
) -> String {
    format!(
        r#"project:
  name: bad_uart_kicad_direction_metadata
  version: 0.1.0
libraries:
  - {library_path}
board:
  components:
    U1:
      model: generic.mcu.basic
      power_domains:
        VDD: mcu_3v3
      pins:
        VDD: mcu_3v3
        GND: gnd
        BOOT0: boot0
        RX: uart_mcu_rx
        TX: uart_mcu_tx
      source:
        board_pin_electrical_types:
          {target_direction}
    U2:
      model: generic.usb_uart.basic
      power_domains:
        VCC: usb_uart_3v3
      pins:
        VCC: usb_uart_3v3
        GND: gnd
        TXD: uart_mcu_rx
        RXD: uart_mcu_tx
      source:
        board_pin_electrical_types:
          {sender_direction}
  nets:
    mcu_3v3:
      kind: power
      nominal_voltage: 3.3
      powered: true
    usb_uart_3v3:
      kind: power
      nominal_voltage: 3.3
      powered: true
    gnd:
      kind: ground
    boot0:
      kind: digital_or_analog
    uart_mcu_rx:
      kind: digital_or_analog
    uart_mcu_tx:
      kind: digital_or_analog
scenarios:
  - name: imported_uart_direction_metadata
    type: serial_programming
    target:
      component: U1
    checks:
      - UART_BOOTLOADER_SYNC
    required_boot_mode: bootloader
    timing:
      power_valid_at_us: 100
      reset_release_at_us: 200
      boot_sample_at_us: 300
    bootloader:
      component: U1
      interface: uart
      sync_byte: 127
      expected_response: 121
    straps:
      - component: U1
        pin: BOOT0
        net: boot0
        actual: high
    events:
      - at_us: 1000
        action: uart_send
        from:
          component: U2
          pin: TXD
        to:
          component: U1
          pin: RX
        bytes: [127]
"#
    )
}

fn backdrive_direction_project(
    library_path: &str,
    driver_direction: &str,
    victim_direction: &str,
) -> String {
    format!(
        r#"project:
  name: bad_backdrive_kicad_direction_metadata
  version: 0.1.0
libraries:
  - {library_path}
board:
  components:
    U1:
      model: generic.mcu.basic
      power_domains:
        VDD: mcu_3v3
      pins:
        VDD: mcu_3v3
        GND: gnd
        RX: uart_rx
      source:
        board_pin_electrical_types:
          {victim_direction}
    U2:
      model: generic.usb_uart.basic
      power_domains:
        VCC: usb_uart_3v3
      pins:
        VCC: usb_uart_3v3
        GND: gnd
        TXD: uart_rx
      source:
        board_pin_electrical_types:
          {driver_direction}
  nets:
    mcu_3v3:
      kind: power
      nominal_voltage: 3.3
      powered: false
    usb_uart_3v3:
      kind: power
      nominal_voltage: 3.3
      powered: true
    gnd:
      kind: ground
    uart_rx:
      kind: digital_or_analog
scenarios:
  - name: imported_backdrive_direction_metadata
    type: gpio_backdrive
    checks:
      - GPIO_BACKDRIVE
    parameters:
      diode_drop_V: 0.3
    pin_states:
      - component: U2
        pin: TXD
        mode: output
        state: high
      - component: U1
        pin: RX
        mode: input
    paths:
      - driver:
          component: U2
          pin: TXD
        victim:
          component: U1
          pin: RX
        series_resistance_ohm: 0
"#
    )
}

fn firmware_functional_project(library_path: &str, include_expected_pin_state: bool) -> String {
    let expected_pin_states = if include_expected_pin_state {
        r#"      expected_pin_states:
        - component: U1
          pin: TX
          mode: output
          state: high
"#
    } else {
        "      expected_pin_states: []\n"
    };
    format!(
        r#"project:
  name: firmware_blackbox_contract
  version: 0.1.0
libraries:
  - {library_path}
board:
  components:
    U1:
      model: generic.mcu.basic
      power_domains:
        VDD: mcu_3v3
      pins:
        VDD: mcu_3v3
        GND: gnd
        NRST: nrst
        BOOT0: boot0
        RX: uart_mcu_rx
        TX: uart_mcu_tx
  nets:
    mcu_3v3:
      kind: power
      nominal_voltage: 3.3
      powered: true
    gnd:
      kind: ground
    nrst:
      kind: digital_or_analog
    boot0:
      kind: digital_or_analog
    uart_mcu_rx:
      kind: digital_or_analog
    uart_mcu_tx:
      kind: digital_or_analog
scenarios:
  - name: firmware_blackbox_pin_behavior
    type: firmware_in_loop
    target:
      component: U1
    checks:
      - FUNCTIONAL_MCU_FIRMWARE
    firmware:
      backend: auto
      image: firmware/app.elf
{expected_pin_states}"#
    )
}

fn firmware_qemu_project(
    library_path: &str,
    qemu_executable: &str,
    expected_state: &str,
) -> String {
    format!(
        r#"project:
  name: firmware_qemu_contract
  version: 0.1.0
libraries:
  - {library_path}
board:
  components:
    U1:
      model: generic.mcu.basic
      power_domains:
        VDD: mcu_3v3
      pins:
        VDD: mcu_3v3
        GND: gnd
        TX: uart_mcu_tx
  nets:
    mcu_3v3:
      kind: power
      nominal_voltage: 3.3
      powered: true
    gnd:
      kind: ground
    uart_mcu_tx:
      kind: digital_or_analog
scenarios:
  - name: firmware_qemu_pin_behavior
    type: firmware_in_loop
    target:
      component: U1
    checks:
      - FUNCTIONAL_MCU_FIRMWARE
    firmware:
      backend: qemu
      image: firmware/app.elf
      machine: stm32vldiscovery
      qemu:
        executable: {qemu_executable}
        timeout_ms: 1000
      expected_pin_states:
        - component: U1
          pin: TX
          mode: output
          state: {expected_state}
"#
    )
}

fn firmware_qemu_build_project(
    library_path: &str,
    qemu_executable: &str,
    output_image: &str,
) -> String {
    format!(
        r#"project:
  name: firmware_qemu_build_contract
  version: 0.1.0
libraries:
  - {library_path}
board:
  components:
    U1:
      model: generic.mcu.basic
      power_domains:
        VDD: mcu_3v3
      pins:
        VDD: mcu_3v3
        GND: gnd
        TX: uart_mcu_tx
  nets:
    mcu_3v3:
      kind: power
      nominal_voltage: 3.3
      powered: true
    gnd:
      kind: ground
    uart_mcu_tx:
      kind: digital_or_analog
scenarios:
  - name: firmware_qemu_build_pin_behavior
    type: firmware_in_loop
    target:
      component: U1
    checks:
      - FUNCTIONAL_MCU_FIRMWARE
    firmware:
      backend: qemu
      image: {output_image}
      machine: stm32vldiscovery
      build:
        command: ["./build_firmware.sh"]
        outputs:
          - {output_image}
        timeout_ms: 1000
      qemu:
        executable: {qemu_executable}
        timeout_ms: 1000
      expected_pin_states:
        - component: U1
          pin: TX
          mode: output
          state: high
"#
    )
}

fn write_dummy_firmware(dir: &std::path::Path) {
    let firmware_dir = dir.join("firmware");
    std::fs::create_dir_all(&firmware_dir).unwrap();
    std::fs::write(firmware_dir.join("app.elf"), b"dummy firmware bytes").unwrap();
}

fn write_fake_build_script(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
    let path = dir.join("build_firmware.sh");
    std::fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}")).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn write_fake_qemu(dir: &std::path::Path, output: &str, exit_code: i32) -> std::path::PathBuf {
    let path = dir.join("qemu-system-arm");
    std::fs::write(
        &path,
        format!("#!/bin/sh\ncat <<'EOF'\n{output}EOF\nexit {exit_code}\n"),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}
