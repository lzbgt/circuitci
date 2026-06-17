mod common;

use common::{assert_report_schema_valid, run_validation};
use serde_json::Value;
use std::process::Command;

#[test]
fn smart_robot_wheel_bridge_budget_passes() {
    let report = run_validation("demos/smart_robot/circuitci/wheel_actuator/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
    assert!(
        !findings_with_id(&report, "MOTOR_BRIDGE_BUDGET_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "wheel actuator bridge budget should pass: {report:#?}"
    );
}

#[test]
fn smart_robot_servo_payload_passes() {
    let report = run_validation("demos/smart_robot/circuitci/servo_payload/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
    assert!(
        report["failures"].as_array().unwrap().is_empty(),
        "servo payload board should not emit critical findings: {report:#?}"
    );
}

#[test]
fn smart_robot_servo_payload_fails_undersized_connector() {
    let (dir, project) = mutated_servo_payload_project(
        "connector_component: JSV0\n      min_connector_current_margin_ratio: 1.5",
        "connector_component: JSV0\n      connector_current_rating_A: 0.5\n      min_connector_current_margin_ratio: 1.5",
    );
    let output = dir.path().join("report");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            project.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_report_schema_valid(&report);
    let connector_findings = findings_with_id(&report, "LOAD_CONNECTOR_CURRENT_VALID");
    assert!(
        connector_findings
            .iter()
            .any(|finding| finding["component"] == "SV0"
                && finding["limit"]["connector_current_rating_A"] == 0.5),
        "expected SV0 connector current failure, got {connector_findings:#?}"
    );
}

#[test]
fn smart_robot_servo_payload_fails_low_connector_voltage_rating() {
    let (dir, project) = mutated_servo_payload_project(
        "connector_component: JSV0\n      min_connector_current_margin_ratio: 1.5",
        "connector_component: JSV0\n      connector_voltage_rating_V: 1.0\n      min_connector_current_margin_ratio: 1.5",
    );
    let output = dir.path().join("report");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            project.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_report_schema_valid(&report);
    let connector_findings = findings_with_id(&report, "LOAD_CONNECTOR_CURRENT_VALID");
    assert!(
        connector_findings
            .iter()
            .any(|finding| finding["component"] == "SV0"
                && finding["measured"]["load_voltage_V"] == 7.4
                && finding["limit"]["connector_voltage_rating_V"] == 1.0),
        "expected SV0 connector voltage failure, got {connector_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_actuator_fails_undersized_actuator_bus_connector() {
    let (dir, project) = mutated_wheel_actuator_project(
        "connector_component: JACT1\n      min_connector_current_margin_ratio: 1.5",
        "connector_component: JACT1\n      connector_current_rating_A: 5.0\n      min_connector_current_margin_ratio: 1.5",
    );
    let output = dir.path().join("report");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            project.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_report_schema_valid(&report);
    let connector_findings = findings_with_id(&report, "LOAD_CONNECTOR_CURRENT_VALID");
    assert!(
        connector_findings
            .iter()
            .any(|finding| finding["component"] == "PWR_STAGE"
                && finding["measured"]["connector_component"] == "JACT1"
                && finding["limit"]["connector_current_rating_A"] == 5.0
                && finding["limit"]["required_connector_current_A"] == 9.0),
        "expected actuator bus connector current failure, got {connector_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_actuator_fails_can_esd_not_ground_referenced() {
    let (dir, project) = mutated_wheel_actuator_project(
        "    UCAN_ESD1:\n      model: vendor.ti.esd2can24_q1\n      part_number: ESD2CAN24-Q1\n      pins:\n        CANH: robot_canh\n        CANL: robot_canl\n        GND: gnd",
        "    UCAN_ESD1:\n      model: vendor.ti.esd2can24_q1\n      part_number: ESD2CAN24-Q1\n      pins:\n        CANH: robot_canh\n        CANL: robot_canl\n        GND: robot_canh",
    );
    let output = dir.path().join("report");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            project.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_report_schema_valid(&report);
    let protection_findings = findings_with_id(&report, "INTERFACE_PROTECTION_REVIEW");
    assert!(
        protection_findings.iter().any(|finding| {
            finding["component"] == "UCAN_ESD1" && finding["severity"] == "critical"
        }),
        "expected CAN ESD ground-reference failure, got {protection_findings:#?}"
    );
}

fn mutated_servo_payload_project(from: &str, to: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let source = std::fs::read_to_string("demos/smart_robot/circuitci/servo_payload/project.yaml")
        .unwrap()
        .replace(
            "../../../../libs/vendor/artery/mcus",
            &repo.join("libs/vendor/artery/mcus").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/jst/connectors",
            &repo.join("libs/vendor/jst/connectors").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/nxp/pwm_drivers",
            &repo.join("libs/vendor/nxp/pwm_drivers").to_string_lossy(),
        )
        .replace(
            "../models",
            &repo
                .join("demos/smart_robot/circuitci/models")
                .to_string_lossy(),
        )
        .replace(from, to);
    let project = dir.path().join("project.yaml");
    std::fs::write(&project, source).unwrap();
    (dir, project)
}

#[test]
fn smart_robot_wheel_bridge_budget_fails_undersized_shunt() {
    let (dir, project) = mutated_wheel_actuator_project(
        "phase_shunt_power_rating_W: 1.0",
        "phase_shunt_power_rating_W: 0.1",
    );
    let output = dir.path().join("report");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            project.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_report_schema_valid(&report);
    let bridge_findings = findings_with_id(&report, "MOTOR_BRIDGE_BUDGET_VALID");
    assert!(
        bridge_findings
            .iter()
            .any(|finding| finding["measured"]["phase_shunt_power_W"] == 0.18),
        "expected phase shunt power failure, got {bridge_findings:#?}"
    );
    assert!(
        bridge_findings
            .iter()
            .any(|finding| finding["measured"]["motor_component"] == "M1"),
        "expected motor-load evidence to identify M1, got {bridge_findings:#?}"
    );
}

fn mutated_wheel_actuator_project(from: &str, to: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let source = std::fs::read_to_string("demos/smart_robot/circuitci/wheel_actuator/project.yaml")
        .unwrap()
        .replace(
            "../../../../libs/generic",
            &repo.join("libs/generic").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/artery/mcus",
            &repo.join("libs/vendor/artery/mcus").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/jst/connectors",
            &repo.join("libs/vendor/jst/connectors").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/ti/can_transceivers",
            &repo
                .join("libs/vendor/ti/can_transceivers")
                .to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/ti/esd_protection",
            &repo.join("libs/vendor/ti/esd_protection").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/ti/motor_drivers",
            &repo.join("libs/vendor/ti/motor_drivers").to_string_lossy(),
        )
        .replace(
            "../models",
            &repo
                .join("demos/smart_robot/circuitci/models")
                .to_string_lossy(),
        )
        .replace(from, to);
    let project = dir.path().join("project.yaml");
    std::fs::write(&project, source).unwrap();
    (dir, project)
}

fn findings_with_id<'a>(report: &'a Value, id: &str) -> Vec<&'a Value> {
    ["failures", "warnings", "infos"]
        .into_iter()
        .flat_map(|section| report[section].as_array().unwrap())
        .filter(|finding| finding["id"] == id)
        .collect()
}
