mod common;

use common::{assert_report_schema_valid, run_validation};
use serde_json::Value;
use std::process::Command;

#[test]
fn smart_robot_wheel_bridge_budgets_fail_closed_without_soa_curve() {
    let report = run_validation("demos/smart_robot/circuitci/wheel_actuator/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_report_schema_valid(&report);
    assert!(
        !findings_with_id(&report, "MOTOR_BRIDGE_BUDGET_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "wheel actuator bridge budget should pass: {report:#?}"
    );
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| {
                limitation["id"] == "LOW_CONFIDENCE_MODEL"
                    && limitation["scope"]
                        == "component:M1:model:demo.smart_robot.wheel_motor_design_envelope"
                    && limitation["blocking"] == false
            }),
        "wheel actuator must expose the demo motor-envelope limitation: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "MOTOR_REGEN_CLAMP_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "wheel actuator regen clamp budget should pass: {report:#?}"
    );
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| {
                limitation["id"] == "LOW_CONFIDENCE_MODEL"
                    && limitation["scope"]
                        == "component:REGEN1:model:demo.smart_robot.regen_clamp_design_envelope"
                    && limitation["blocking"] == false
            }),
        "wheel actuator must expose the demo regen-clamp limitation: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "MOTOR_CURRENT_SENSE_ACCURACY_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "wheel actuator current-sense accuracy budget should pass: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "MOTOR_BRIDGE_SWITCHING_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "wheel actuator switching budget should pass: {report:#?}"
    );
    let soa_findings = findings_with_id(&report, "MOTOR_BRIDGE_SOA_VALID");
    assert!(
        soa_findings.iter().any(|finding| {
            finding["component"] == "PWR_STAGE"
                && finding["measured"]["model"]
                    == "demo.smart_robot.csd88599q5dc_3phase_bridge_budget"
                && finding["measured"]["soa_metadata_error"]
                    == "missing safe_operating_area.vds_id_curves"
        }),
        "wheel actuator must fail closed until the bridge model has sourced SOA curves: {report:#?}"
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

#[test]
fn smart_robot_motion_core_fails_rs485_esd_not_ground_referenced() {
    let (dir, project) = mutated_motion_core_project(
        "    U485_ESD1:\n      model: vendor.ti.esds552\n      part_number: ESDS552\n      pins:\n        A: rs485_servo_a\n        B: rs485_servo_b\n        GND: gnd",
        "    U485_ESD1:\n      model: vendor.ti.esds552\n      part_number: ESDS552\n      pins:\n        A: rs485_servo_a\n        B: rs485_servo_b\n        GND: rs485_servo_a",
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
            finding["component"] == "U485_ESD1" && finding["severity"] == "critical"
        }),
        "expected RS485 ESD ground-reference failure, got {protection_findings:#?}"
    );
}

#[test]
fn smart_robot_motion_core_fails_wrong_can_termination_value() {
    let (dir, project) = mutated_motion_core_project("value_ohm: 120.0", "value_ohm: 100.0");
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
    let termination_findings = findings_with_id(&report, "BUS_TERMINATION_VALID");
    assert!(
        termination_findings.iter().any(|finding| {
            finding["component"] == "RT_CAN1"
                && finding["measured"]["termination_ohm"] == 100.0
                && finding["limit"]["expected_termination_ohm"] == 120.0
        }),
        "expected CAN termination value failure, got {termination_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_actuator_fails_wrong_can_termination_value() {
    let (dir, project) = mutated_wheel_actuator_project("value_ohm: 120.0", "value_ohm: 100.0");
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
    let termination_findings = findings_with_id(&report, "BUS_TERMINATION_VALID");
    assert!(
        termination_findings.iter().any(|finding| {
            finding["component"] == "RT_CAN1"
                && finding["measured"]["termination_ohm"] == 100.0
                && finding["limit"]["expected_termination_ohm"] == 120.0
        }),
        "expected wheel CAN termination value failure, got {termination_findings:#?}"
    );
}

#[test]
fn smart_robot_motion_core_fails_can_esd_route_placement_limit() {
    let (dir, project) = mutated_motion_core_project(
        "checked_component: UCAN_ESD1\n      max_reference_to_checked_route_distance_mm: 5.0",
        "checked_component: UCAN_ESD1\n      max_reference_to_checked_route_distance_mm: 1.0",
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
    let placement_findings = findings_with_id(&report, "BUS_PROTECTION_PLACEMENT_VALID");
    assert!(
        placement_findings.iter().any(|finding| {
            finding["component"] == "UCAN_ESD1"
                && finding["measured"]["reference_component"] == "U3"
                && finding["measured"]["line_a_route_distance_mm"] == 3.0
                && finding["limit"]["max_reference_to_checked_route_distance_mm"] == 1.0
        }),
        "expected CAN ESD route-placement failure, got {placement_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_actuator_fails_can_termination_route_placement_limit() {
    let (dir, project) = mutated_wheel_actuator_project(
        "checked_component: RT_CAN1\n      max_reference_to_checked_route_distance_mm: 8.0",
        "checked_component: RT_CAN1\n      max_reference_to_checked_route_distance_mm: 3.0",
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
    let placement_findings = findings_with_id(&report, "BUS_PROTECTION_PLACEMENT_VALID");
    assert!(
        placement_findings.iter().any(|finding| {
            finding["component"] == "RT_CAN1"
                && finding["measured"]["reference_component"] == "JACT1"
                && finding["measured"]["line_b_route_distance_mm"] == 7.0
                && finding["limit"]["max_reference_to_checked_route_distance_mm"] == 3.0
        }),
        "expected wheel CAN termination route-placement failure, got {placement_findings:#?}"
    );
}

fn mutated_motion_core_project(from: &str, to: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let source = std::fs::read_to_string("demos/smart_robot/circuitci/motion_core/project.yaml")
        .unwrap()
        .replace(
            "../../../../libs/vendor/sipeed/modules",
            &repo.join("libs/vendor/sipeed/modules").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/artery/mcus",
            &repo.join("libs/vendor/artery/mcus").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/tdk/imu",
            &repo.join("libs/vendor/tdk/imu").to_string_lossy(),
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
            "../../../../libs/vendor/ti/rs485_transceivers",
            &repo
                .join("libs/vendor/ti/rs485_transceivers")
                .to_string_lossy(),
        )
        .replace(
            "../../../../libs/generic/analog",
            &repo.join("libs/generic/analog").to_string_lossy(),
        )
        .replace(
            "../../../../libs/generic/digital",
            &repo.join("libs/generic/digital").to_string_lossy(),
        )
        .replace(from, to);
    let project = dir.path().join("project.yaml");
    std::fs::write(&project, source).unwrap();
    (dir, project)
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

#[test]
fn smart_robot_wheel_bridge_loss_thermal_fails_low_board_budget() {
    let (dir, project) = mutated_wheel_actuator_project(
        "max_total_bridge_loss_W: 2.0",
        "max_total_bridge_loss_W: 0.2",
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
    let thermal_findings = findings_with_id(&report, "MOTOR_BRIDGE_LOSS_THERMAL_VALID");
    assert!(
        thermal_findings.iter().any(|finding| {
            let estimated_loss = finding["measured"]["estimated_total_bridge_loss_W"]
                .as_f64()
                .unwrap();
            finding["component"] == "PWR_STAGE"
                && (estimated_loss - 0.36).abs() < 1e-9
                && finding["limit"]["max_total_bridge_loss_W"] == 0.2
                && finding["limit"]["min_loss_margin_ratio"] == 2.0
        }),
        "expected motor bridge thermal budget failure, got {thermal_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_bridge_switching_fails_low_loss_budget() {
    let (dir, project) = mutated_wheel_actuator_project(
        "max_total_switching_loss_W: 0.5",
        "max_total_switching_loss_W: 0.05",
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
    let switching_findings = findings_with_id(&report, "MOTOR_BRIDGE_SWITCHING_VALID");
    assert!(
        switching_findings.iter().any(|finding| {
            let estimated_loss = finding["measured"]["estimated_total_switching_loss_W"]
                .as_f64()
                .unwrap();
            finding["component"] == "PWR_STAGE"
                && (estimated_loss - 0.17387999999999998).abs() < 1e-9
                && finding["limit"]["max_total_switching_loss_W"] == 0.05
                && finding["limit"]["min_switching_loss_margin_ratio"] == 2.0
        }),
        "expected motor bridge switching-loss budget failure, got {switching_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_bridge_soa_static_budget_passes_with_source_curve() {
    let (dir, project) = wheel_actuator_project_with_static_soa_curve(None);
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
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
    assert!(
        !findings_with_id(&report, "MOTOR_BRIDGE_SOA_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "SOA-backed wheel bridge fixture should pass: {report:#?}"
    );
}

#[test]
fn smart_robot_wheel_bridge_soa_static_budget_fails_low_curve_margin() {
    let (dir, project) = wheel_actuator_project_with_static_soa_curve(Some((
        "min_soa_current_margin_ratio: 2.0",
        "min_soa_current_margin_ratio: 6.0",
    )));
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
    let soa_findings = findings_with_id(&report, "MOTOR_BRIDGE_SOA_VALID");
    assert!(
        soa_findings.iter().any(|finding| {
            let margin = finding["measured"]["soa_current_margin_ratio"]
                .as_f64()
                .unwrap();
            finding["component"] == "PWR_STAGE"
                && (margin - 5.564142314645005).abs() < 1e-9
                && finding["limit"]["min_soa_current_margin_ratio"] == 6.0
        }),
        "expected low SOA margin failure, got {soa_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_regen_clamp_fails_low_energy_rating() {
    let (dir, project) =
        mutated_wheel_actuator_project("clamp_energy_rating_J: 1.5", "clamp_energy_rating_J: 0.2");
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
    let regen_findings = findings_with_id(&report, "MOTOR_REGEN_CLAMP_VALID");
    assert!(
        regen_findings.iter().any(|finding| {
            let total_energy = finding["measured"]["total_absorption_energy_J"]
                .as_f64()
                .unwrap();
            finding["component"] == "REGEN1"
                && (total_energy - 0.24862).abs() < 1e-9
                && finding["limit"]["required_absorption_energy_J"] == 1.5
                && finding["limit"]["min_regen_energy_margin_ratio"] == 1.5
        }),
        "expected regen clamp energy failure, got {regen_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_route_current_fails_undersized_phase_width() {
    let (dir, project) = mutated_wheel_actuator_project("width_mm: 1.20", "width_mm: 0.80");
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
    let route_findings = findings_with_id(&report, "MOTOR_ROUTE_CURRENT_VALID");
    assert!(
        route_findings.iter().any(|finding| {
            finding["net"] == "phase_u"
                && finding["measured"]["route_current_A"] == 6.0
                && finding["measured"]["min_route_width_mm"] == 0.8
                && finding["limit"]["required_route_width_mm"] == 1.2
        }),
        "expected undersized phase route failure, got {route_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_current_sense_placement_fails_remote_shunt() {
    let (dir, project) = mutated_wheel_actuator_project(
        "max_shunt_to_reference_distance_mm: 6.0",
        "max_shunt_to_reference_distance_mm: 1.0",
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
    let placement_findings = findings_with_id(&report, "MOTOR_CURRENT_SENSE_PLACEMENT_VALID");
    assert!(
        placement_findings.iter().any(|finding| {
            finding["component"] == "RSHUNT_V"
                && finding["measured"]["shunt_to_reference_distance_mm"] == 5.0
                && finding["limit"]["max_shunt_to_reference_distance_mm"] == 1.0
        }),
        "expected remote phase-shunt placement failure, got {placement_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_current_sense_accuracy_fails_low_gain() {
    let (dir, project) =
        mutated_wheel_actuator_project("sense_gain_V_per_V: 20.0", "sense_gain_V_per_V: 2.0");
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
    let accuracy_findings = findings_with_id(&report, "MOTOR_CURRENT_SENSE_ACCURACY_VALID");
    assert!(
        accuracy_findings.iter().any(|finding| {
            let counts = finding["measured"]["adc_counts_at_min_current"]
                .as_f64()
                .unwrap();
            finding["component"] == "PWR_STAGE"
                && (counts - 6.204545454545455).abs() < 1e-9
                && finding["limit"]["min_adc_counts_at_min_current"] == 20.0
        }),
        "expected low-gain current-sense resolution failure, got {accuracy_findings:#?}"
    );
}

fn mutated_wheel_actuator_project(from: &str, to: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let source = wheel_actuator_project_source().replace(from, to);
    let project = dir.path().join("project.yaml");
    std::fs::write(&project, source).unwrap();
    (dir, project)
}

fn wheel_actuator_project_with_static_soa_curve(
    project_replace: Option<(&str, &str)>,
) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let model_dir = dir.path().join("models");
    std::fs::create_dir_all(&model_dir).unwrap();
    let model = std::fs::read_to_string(
        "demos/smart_robot/circuitci/models/csd88599q5dc_3phase_bridge_budget.model.yaml",
    )
    .unwrap()
    .replace(
        "component_id: demo.smart_robot.csd88599q5dc_3phase_bridge_budget",
        "component_id: demo.smart_robot.csd88599q5dc_3phase_bridge_budget_with_soa",
    )
    .replace(
        "  electrical_characteristics:",
        "  safe_operating_area:\n    vds_id_curves:\n      - name: static_100us\n        pulse_width_us: 100.0\n        duty_cycle_max: 0.02\n        temperature_c: 25.0\n        source_document: tests/smart_robot_cli.rs\n        source_figure: synthetic static SOA fixture\n        digitization:\n          method: regression_fixture\n          confidence: high\n          note: Test-only curve shape used to prove MOTOR_BRIDGE_SOA_VALID behavior.\n        points:\n          - vds_v: 1.0\n            id_a: 100.0\n          - vds_v: 20.0\n            id_a: 50.0\n          - vds_v: 60.0\n            id_a: 20.0\n  electrical_characteristics:",
    );
    std::fs::write(
        model_dir.join("csd88599q5dc_3phase_bridge_budget_with_soa.model.yaml"),
        model,
    )
    .unwrap();

    let mut source = wheel_actuator_project_source()
        .replace(
            "libraries:\n",
            &format!("libraries:\n  - {}\n", model_dir.to_string_lossy()),
        )
        .replace(
            "demo.smart_robot.csd88599q5dc_3phase_bridge_budget",
            "demo.smart_robot.csd88599q5dc_3phase_bridge_budget_with_soa",
        );
    if let Some((from, to)) = project_replace {
        source = source.replace(from, to);
    }
    let project = dir.path().join("project.yaml");
    std::fs::write(&project, source).unwrap();
    (dir, project)
}

fn wheel_actuator_project_source() -> String {
    let repo = std::env::current_dir().unwrap();
    std::fs::read_to_string("demos/smart_robot/circuitci/wheel_actuator/project.yaml")
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
}

fn findings_with_id<'a>(report: &'a Value, id: &str) -> Vec<&'a Value> {
    ["failures", "warnings", "infos"]
        .into_iter()
        .flat_map(|section| report[section].as_array().unwrap())
        .filter(|finding| finding["id"] == id)
        .collect()
}
