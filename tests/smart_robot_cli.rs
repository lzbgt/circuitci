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
fn smart_robot_wheel_bridge_budget_fails_undersized_shunt() {
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
            "../../../../libs/vendor/ti/motor_drivers",
            &repo.join("libs/vendor/ti/motor_drivers").to_string_lossy(),
        )
        .replace(
            "../models",
            &repo
                .join("demos/smart_robot/circuitci/models")
                .to_string_lossy(),
        )
        .replace(
            "phase_shunt_power_rating_W: 1.0",
            "phase_shunt_power_rating_W: 0.1",
        );
    let project = dir.path().join("project.yaml");
    std::fs::write(&project, source).unwrap();
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

fn findings_with_id<'a>(report: &'a Value, id: &str) -> Vec<&'a Value> {
    ["failures", "warnings", "infos"]
        .into_iter()
        .flat_map(|section| report[section].as_array().unwrap())
        .filter(|finding| finding["id"] == id)
        .collect()
}
