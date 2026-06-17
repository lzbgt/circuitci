mod common;

use common::{assert_report_schema_valid, run_validation};
use serde_json::Value;
use serde_yaml_ng::Value as YamlValue;
use std::process::Command;

#[test]
fn smart_robot_wheel_blocks_placeholder_load_signoff() {
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
    assert!(
        !findings_with_id(&report, "MOTOR_BRIDGE_SOA_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "wheel actuator system SOA budget should pass: {report:#?}"
    );
    let model_quality_findings = findings_with_id(&report, "MODEL_QUALITY_REQUIRED");
    assert!(
        model_quality_findings
            .iter()
            .any(|finding| finding["component"] == "M1"
                && finding["measured"]["model_source"] == "generic"
                && finding["measured"]["model_confidence"] == "low"
                && finding["limit"]["allowed_sources"]
                    .as_array()
                    .unwrap()
                    .contains(&Value::String("datasheet".to_string()))
                && finding["limit"]["min_confidence"] == "medium"),
        "wheel actuator must fail sign-off on placeholder motor evidence: {model_quality_findings:#?}"
    );
    assert!(
        model_quality_findings
            .iter()
            .any(|finding| finding["component"] == "REGEN1"
                && finding["measured"]["model_source"] == "generic"
                && finding["measured"]["model_confidence"] == "low"),
        "wheel actuator must fail sign-off on placeholder regen evidence: {model_quality_findings:#?}"
    );
    let missing_inputs = findings_with_id(&report, "VALIDATION_INPUT_MISSING");
    assert!(
        missing_inputs.iter().any(|finding| {
            finding["scenario"] == "wheel_actuator_bus_cable_budget"
                && finding["limit"]["required_input"] == "cable_current_rating_A"
        }),
        "wheel actuator must fail sign-off until actuator-bus cable current evidence is selected: {missing_inputs:#?}"
    );
    assert!(
        missing_inputs.iter().any(|finding| {
            finding["scenario"] == "wheel_actuator_bus_cable_thermal_derating"
                && finding["limit"]["required_input"] == "cable_temperature_rise_test_current_A"
        }),
        "wheel actuator must fail sign-off until actuator-bus cable thermal evidence is selected: {missing_inputs:#?}"
    );
    assert!(
        missing_inputs.iter().any(|finding| {
            finding["scenario"] == "wheel_actuator_bus_cable_voltage_drop"
                && finding["limit"]["required_input"] == "cable_loop_resistance_ohm"
        }),
        "wheel actuator must fail sign-off until actuator-bus cable loop resistance evidence is selected: {missing_inputs:#?}"
    );
}

#[test]
fn smart_robot_wheel_model_quality_gate_passes_with_source_backed_loads() {
    let (dir, project) = wheel_actuator_project_with_source_backed_load_models();
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
        !findings_with_id(&report, "MODEL_QUALITY_REQUIRED")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "source-backed load evidence should clear the wheel model-quality gate: {report:#?}"
    );
}

#[test]
fn smart_robot_pmu_passes_with_selected_power_switch_paths() {
    let report = run_validation("demos/smart_robot/circuitci/pmu/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
    assert!(
        !findings_with_id(&report, "POWER_TREE_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "PMU power-tree checks should remain clean with selected switch evidence: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "MODEL_QUALITY_REQUIRED")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "PMU selected switch models should clear model-quality sign-off: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "VALIDATION_INPUT_MISSING")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "PMU selected switch models should not leave required switch inputs missing: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "POWER_SWITCH_REVERSE_CURRENT_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "TPS25948 must satisfy the servo always-on reverse mode and TPS24751 path must satisfy wheel off-state isolation: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "POWER_SWITCH_BUDGET_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "selected PMU switch paths should clear static current and thermal budgets: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "POWER_SWITCH_INRUSH_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "selected PMU switch paths should clear first-pass inrush budgets: {report:#?}"
    );
}

#[test]
fn smart_robot_pmu_switch_budget_passes_with_source_backed_switches() {
    let (dir, project) = pmu_project_with_source_backed_switch_model();
    let output = dir.path().join("report");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args(["validate", project.to_str().unwrap(), "--output"])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
    assert!(
        !findings_with_id(&report, "POWER_SWITCH_BUDGET_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "source-backed PMU switch evidence should clear switch budget checks: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "POWER_SWITCH_REVERSE_CURRENT_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "source-backed PMU switch evidence should clear reverse-current checks: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "POWER_SWITCH_INRUSH_VALID")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "source-backed PMU switch evidence should clear inrush checks: {report:#?}"
    );
    assert!(
        !findings_with_id(&report, "MODEL_QUALITY_REQUIRED")
            .into_iter()
            .any(|finding| finding["severity"] == "critical"),
        "source-backed PMU switch evidence should clear model-quality checks: {report:#?}"
    );
}

#[test]
fn smart_robot_pmu_rejects_off_state_only_reverse_blocking_for_servo_switch() {
    let (dir, project) = pmu_project_with_when_disabled_switch_model();
    let output = dir.path().join("report");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args(["validate", project.to_str().unwrap(), "--output"])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(output.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_report_schema_valid(&report);
    let reverse_findings = findings_with_id(&report, "POWER_SWITCH_REVERSE_CURRENT_VALID");
    assert!(
        reverse_findings.iter().any(|finding| {
            finding["component"] == "U_SERVO_SW"
                && finding["scenario"] == "pmu_servo_switch_reverse_current"
                && finding["measured"]["reverse_current_blocking_mode"] == "when_disabled"
                && finding["limit"]["reverse_current_blocking_mode_required"] == "always"
        }),
        "off-state-only reverse blocking must not satisfy an always-on reverse-current requirement: {reverse_findings:#?}"
    );
    assert!(
        !reverse_findings.iter().any(|finding| {
            finding["component"] == "U_WHEEL_SW"
                && finding["scenario"] == "pmu_wheel_switch_reverse_current"
        }),
        "off-state reverse blocking should satisfy the wheel e-stop isolation requirement: {reverse_findings:#?}"
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
fn smart_robot_wheel_actuator_fails_undersized_actuator_bus_cable() {
    let (dir, project) = mutated_wheel_actuator_project(
        "min_cable_current_margin_ratio: 1.5",
        "cable_current_rating_A: 5.0\n      min_cable_current_margin_ratio: 1.5",
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
    let cable_findings = findings_with_id(&report, "LOAD_CABLE_CURRENT_VALID");
    assert!(
        cable_findings.iter().any(|finding| {
            finding["component"] == "PWR_STAGE"
                && finding["measured"]["load_current_A"] == 6.0
                && finding["limit"]["cable_current_rating_A"] == 5.0
                && finding["limit"]["required_cable_current_A"] == 9.0
        }),
        "expected actuator-bus cable current failure, got {cable_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_actuator_fails_hot_actuator_bus_cable() {
    let (dir, project) = mutated_wheel_actuator_project(
        "thermal_current_margin_ratio: 1.5",
        "cable_temperature_rise_test_current_A: 6.0\n      cable_temperature_rise_at_test_current_C: 20.0\n      max_cable_temperature_rise_C: 30.0\n      thermal_current_margin_ratio: 1.5",
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
    let thermal_findings = findings_with_id(&report, "LOAD_CABLE_THERMAL_DERATING_VALID");
    assert!(
        thermal_findings.iter().any(|finding| {
            finding["component"] == "PWR_STAGE"
                && finding["measured"]["load_current_A"] == 6.0
                && finding["measured"]["thermal_current_A"] == 9.0
                && finding["measured"]["estimated_temperature_rise_C"] == 45.0
                && finding["limit"]["max_cable_temperature_rise_C"] == 30.0
        }),
        "expected actuator-bus cable thermal derating failure, got {thermal_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_actuator_fails_lossy_actuator_bus_cable() {
    let (dir, project) = mutated_wheel_actuator_project(
        "max_cable_voltage_drop_V: 0.3\n      max_cable_power_loss_W: 2.0\n      drop_current_margin_ratio: 1.5",
        "cable_loop_resistance_ohm: 0.05\n      max_cable_voltage_drop_V: 0.3\n      max_cable_power_loss_W: 2.0\n      drop_current_margin_ratio: 1.5",
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
    let drop_findings = findings_with_id(&report, "LOAD_CABLE_VOLTAGE_DROP_VALID");
    assert!(
        drop_findings.iter().any(|finding| {
            finding["component"] == "PWR_STAGE"
                && finding["measured"]["drop_current_A"] == 9.0
                && finding["measured"]["cable_loop_resistance_ohm"] == 0.05
                && finding["measured"]["estimated_voltage_drop_V"] == 0.45
                && finding["measured"]["estimated_power_loss_W"] == 4.05
                && finding["limit"]["max_cable_voltage_drop_V"] == 0.3
                && finding["limit"]["max_cable_power_loss_W"] == 2.0
        }),
        "expected actuator-bus cable voltage-drop failure, got {drop_findings:#?}"
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
fn smart_robot_wheel_bridge_soa_fails_without_source_curve() {
    let (dir, project) = wheel_actuator_project_without_system_soa();
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
            finding["component"] == "PWR_STAGE"
                && finding["measured"]["model"]
                    == "demo.smart_robot.csd88599q5dc_3phase_bridge_budget_no_soa"
                && finding["measured"]["soa_metadata_error"]
                    == "missing safe_operating_area.vds_id_curves"
        }),
        "expected missing SOA metadata failure, got {soa_findings:#?}"
    );
}

#[test]
fn smart_robot_wheel_bridge_soa_static_budget_fails_low_system_margin() {
    let (dir, project) = mutated_wheel_actuator_project(
        "min_soa_current_margin_ratio: 2.0",
        "min_soa_current_margin_ratio: 5.0",
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
    let soa_findings = findings_with_id(&report, "MOTOR_BRIDGE_SOA_VALID");
    assert!(
        soa_findings.iter().any(|finding| {
            let margin = finding["measured"]["system_soa_current_margin_ratio"]
                .as_f64()
                .unwrap();
            finding["component"] == "PWR_STAGE"
                && (margin - 4.0).abs() < 1e-9
                && finding["limit"]["min_soa_current_margin_ratio"] == 5.0
                && finding["limit"]["system_soa_curve"] == "figure_4_3_typical_board_temperature"
        }),
        "expected low system SOA margin failure, got {soa_findings:#?}"
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

fn pmu_project_with_source_backed_switch_model() -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let model_dir = dir.path().join("models");
    std::fs::create_dir_all(&model_dir).unwrap();

    let mut switch_model: YamlValue = serde_yaml_ng::from_str(
        &std::fs::read_to_string(
            "demos/smart_robot/circuitci/models/estop_power_switch_policy.model.yaml",
        )
        .unwrap(),
    )
    .unwrap();
    switch_model["component_id"] =
        YamlValue::String("demo.smart_robot.test_source_backed_estop_switch".to_string());
    switch_model["power_switch"]["current_limit_A"] = YamlValue::Number(15.0.into());
    switch_model["power_switch"]["on_resistance_ohm"] = YamlValue::Number(0.010.into());
    switch_model["power_switch"]["thermal_resistance_junction_to_ambient_C_per_W"] =
        YamlValue::Number(40.0.into());
    switch_model["power_switch"]["max_junction_temperature_C"] = YamlValue::Number(150.0.into());
    switch_model["power_switch"]["reverse_current_blocking"] = YamlValue::Bool(true);
    switch_model["power_switch"]["max_inrush_current_A"] = YamlValue::Number(20.0.into());
    switch_model["power_switch"]["soft_start_time_us"] = YamlValue::Number(2000.0.into());
    switch_model["model_quality"]["source"] = YamlValue::String("datasheet".to_string());
    switch_model["model_quality"]["confidence"] = YamlValue::String("medium".to_string());
    std::fs::write(
        model_dir.join("test_source_backed_estop_switch.model.yaml"),
        serde_yaml_ng::to_string(&switch_model).unwrap(),
    )
    .unwrap();

    let source = pmu_project_source()
        .replace(
            "libraries:\n",
            &format!("libraries:\n  - {}\n", model_dir.to_string_lossy()),
        )
        .replace(
            "demo.smart_robot.estop_power_switch_policy",
            "demo.smart_robot.test_source_backed_estop_switch",
        )
        .replace(
            "vendor.ti.tps25948_8a_rcb_dvdt",
            "demo.smart_robot.test_source_backed_estop_switch",
        )
        .replace(
            "switch_component: U_SERVO_SW\n      min_inrush_current_margin_ratio: 1.2",
            "switch_component: U_SERVO_SW\n      switched_capacitance_F: 0.001\n      min_inrush_current_margin_ratio: 1.2",
        )
        .replace(
            "switch_component: U_WHEEL_SW\n      min_inrush_current_margin_ratio: 1.2",
            "switch_component: U_WHEEL_SW\n      switched_capacitance_F: 0.001\n      min_inrush_current_margin_ratio: 1.2",
        );
    let project = dir.path().join("project.yaml");
    std::fs::write(&project, source).unwrap();
    (dir, project)
}

fn pmu_project_with_when_disabled_switch_model() -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let model_dir = dir.path().join("models");
    std::fs::create_dir_all(&model_dir).unwrap();

    let mut switch_model: YamlValue = serde_yaml_ng::from_str(
        &std::fs::read_to_string(
            "demos/smart_robot/circuitci/models/estop_power_switch_policy.model.yaml",
        )
        .unwrap(),
    )
    .unwrap();
    switch_model["component_id"] =
        YamlValue::String("demo.smart_robot.test_when_disabled_estop_switch".to_string());
    switch_model["power_switch"]["current_limit_A"] = YamlValue::Number(15.0.into());
    switch_model["power_switch"]["on_resistance_ohm"] = YamlValue::Number(0.010.into());
    switch_model["power_switch"]["thermal_resistance_junction_to_ambient_C_per_W"] =
        YamlValue::Number(40.0.into());
    switch_model["power_switch"]["max_junction_temperature_C"] = YamlValue::Number(150.0.into());
    switch_model["power_switch"]["reverse_current_blocking_mode"] =
        YamlValue::String("when_disabled".to_string());
    switch_model["power_switch"]["max_inrush_current_A"] = YamlValue::Number(20.0.into());
    switch_model["power_switch"]["soft_start_time_us"] = YamlValue::Number(2000.0.into());
    switch_model["model_quality"]["source"] = YamlValue::String("datasheet".to_string());
    switch_model["model_quality"]["confidence"] = YamlValue::String("medium".to_string());
    std::fs::write(
        model_dir.join("test_when_disabled_estop_switch.model.yaml"),
        serde_yaml_ng::to_string(&switch_model).unwrap(),
    )
    .unwrap();

    let source = pmu_project_source()
        .replace(
            "libraries:\n",
            &format!("libraries:\n  - {}\n", model_dir.to_string_lossy()),
        )
        .replace(
            "demo.smart_robot.estop_power_switch_policy",
            "demo.smart_robot.test_when_disabled_estop_switch",
        )
        .replace(
            "vendor.ti.tps25948_8a_rcb_dvdt",
            "demo.smart_robot.test_when_disabled_estop_switch",
        )
        .replace(
            "switch_component: U_WHEEL_SW\n      min_inrush_current_margin_ratio: 1.2",
            "switch_component: U_WHEEL_SW\n      switched_capacitance_F: 0.001\n      min_inrush_current_margin_ratio: 1.2",
        );
    let project = dir.path().join("project.yaml");
    std::fs::write(&project, source).unwrap();
    (dir, project)
}

fn pmu_project_source() -> String {
    let repo = std::env::current_dir().unwrap();
    std::fs::read_to_string("demos/smart_robot/circuitci/pmu/project.yaml")
        .unwrap()
        .replace(
            "../../../../libs/generic",
            &repo.join("libs/generic").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/ti/chargers",
            &repo.join("libs/vendor/ti/chargers").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/ti/efuses",
            &repo.join("libs/vendor/ti/efuses").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/ti/regulators",
            &repo.join("libs/vendor/ti/regulators").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/sipeed/modules",
            &repo.join("libs/vendor/sipeed/modules").to_string_lossy(),
        )
        .replace(
            "../../../../libs/vendor/artery/mcus",
            &repo.join("libs/vendor/artery/mcus").to_string_lossy(),
        )
        .replace(
            "../models",
            &repo
                .join("demos/smart_robot/circuitci/models")
                .to_string_lossy(),
        )
}

fn wheel_actuator_project_without_system_soa() -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let model_dir = dir.path().join("models");
    std::fs::create_dir_all(&model_dir).unwrap();
    let mut model: YamlValue = serde_yaml_ng::from_str(
        &std::fs::read_to_string(
            "demos/smart_robot/circuitci/models/csd88599q5dc_3phase_bridge_budget.model.yaml",
        )
        .unwrap(),
    )
    .unwrap();
    model["component_id"] =
        YamlValue::String("demo.smart_robot.csd88599q5dc_3phase_bridge_budget_no_soa".to_string());
    model["motor_bridge"]
        .as_mapping_mut()
        .unwrap()
        .remove(YamlValue::String("system_soa".to_string()));
    std::fs::write(
        model_dir.join("csd88599q5dc_3phase_bridge_budget_no_soa.model.yaml"),
        serde_yaml_ng::to_string(&model).unwrap(),
    )
    .unwrap();

    let source = wheel_actuator_project_source()
        .replace(
            "libraries:\n",
            &format!("libraries:\n  - {}\n", model_dir.to_string_lossy()),
        )
        .replace(
            "demo.smart_robot.csd88599q5dc_3phase_bridge_budget",
            "demo.smart_robot.csd88599q5dc_3phase_bridge_budget_no_soa",
        );
    let project = dir.path().join("project.yaml");
    std::fs::write(&project, source).unwrap();
    (dir, project)
}

fn wheel_actuator_project_with_source_backed_load_models() -> (tempfile::TempDir, std::path::PathBuf)
{
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let model_dir = dir.path().join("models");
    std::fs::create_dir_all(&model_dir).unwrap();

    let mut motor_model: YamlValue = serde_yaml_ng::from_str(
        &std::fs::read_to_string(
            "demos/smart_robot/circuitci/models/wheel_motor_design_envelope.model.yaml",
        )
        .unwrap(),
    )
    .unwrap();
    motor_model["component_id"] =
        YamlValue::String("demo.smart_robot.test_source_backed_wheel_motor".to_string());
    motor_model["model_quality"]["source"] = YamlValue::String("datasheet".to_string());
    motor_model["model_quality"]["confidence"] = YamlValue::String("medium".to_string());
    std::fs::write(
        model_dir.join("test_source_backed_wheel_motor.model.yaml"),
        serde_yaml_ng::to_string(&motor_model).unwrap(),
    )
    .unwrap();

    let mut regen_model: YamlValue = serde_yaml_ng::from_str(
        &std::fs::read_to_string(
            "demos/smart_robot/circuitci/models/regen_clamp_design_envelope.model.yaml",
        )
        .unwrap(),
    )
    .unwrap();
    regen_model["component_id"] =
        YamlValue::String("demo.smart_robot.test_source_backed_regen_clamp".to_string());
    regen_model["model_quality"]["source"] = YamlValue::String("measured".to_string());
    regen_model["model_quality"]["confidence"] = YamlValue::String("medium".to_string());
    std::fs::write(
        model_dir.join("test_source_backed_regen_clamp.model.yaml"),
        serde_yaml_ng::to_string(&regen_model).unwrap(),
    )
    .unwrap();

    let source = wheel_actuator_project_source()
        .replace(
            "libraries:\n",
            &format!("libraries:\n  - {}\n", model_dir.to_string_lossy()),
        )
        .replace(
            "demo.smart_robot.wheel_motor_design_envelope",
            "demo.smart_robot.test_source_backed_wheel_motor",
        )
        .replace(
            "demo.smart_robot.regen_clamp_design_envelope",
            "demo.smart_robot.test_source_backed_regen_clamp",
        )
        .replace(
            "min_cable_current_margin_ratio: 1.5",
            "cable_current_rating_A: 12.0\n      cable_voltage_rating_V: 30.0\n      min_cable_current_margin_ratio: 1.5",
        )
        .replace(
            "thermal_current_margin_ratio: 1.5",
            "cable_temperature_rise_test_current_A: 12.0\n      cable_temperature_rise_at_test_current_C: 20.0\n      max_cable_temperature_rise_C: 30.0\n      thermal_current_margin_ratio: 1.5",
        )
        .replace(
            "max_cable_voltage_drop_V: 0.3\n      max_cable_power_loss_W: 2.0\n      drop_current_margin_ratio: 1.5",
            "cable_loop_resistance_ohm: 0.02\n      max_cable_voltage_drop_V: 0.3\n      max_cable_power_loss_W: 2.0\n      drop_current_margin_ratio: 1.5",
        );
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
