mod common;

use common::{
    assert_report_schema_valid, assert_yaml_file_valid, binary_available, run_validation,
};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_spice_creates_operating_point_scenario_for_op_deck() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("op_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 vin 0 DC 5\nR1 vin midpoint 10k\nR2 midpoint 0 10k\n.op\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_op_deck",
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
    let scenarios = imported["scenarios"].as_array().unwrap();
    assert_eq!(scenarios.len(), 1);
    assert_eq!(scenarios[0]["type"], "analog_dc");
    assert_eq!(
        scenarios[0]["checks"],
        Value::Array(vec![Value::String("SPICE_DC_ANALYSIS".to_string())])
    );
    assert_eq!(scenarios[0]["analog"]["analysis"]["type"], "op");
    assert!(
        scenarios[0]["analog"]["analysis"]
            .get("stop_time_us")
            .is_none()
    );

    let report = run_validation(output.to_str().unwrap());
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert!(
            report["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact.as_str().unwrap().ends_with("operating_point.csv"))
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_spice_creates_op_and_transient_scenarios_when_both_requested() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("op_tran_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 PULSE(0 1 1u 0.1u 0.1u 2u 10u)\nR1 in 0 1k\n.control\nop\ntran 0.1u 10u\n.endc\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_op_tran_deck",
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
    let scenarios = imported["scenarios"].as_array().unwrap();
    assert_eq!(scenarios.len(), 2);
    assert_eq!(scenarios[0]["type"], "analog_dc");
    assert_eq!(scenarios[0]["analog"]["analysis"]["type"], "op");
    assert_eq!(scenarios[1]["type"], "analog_transient");
    assert_eq!(scenarios[1]["analog"]["analysis"]["type"], "tran");
    assert!(
        (scenarios[1]["analog"]["analysis"]["stop_time_us"]
            .as_f64()
            .unwrap()
            - 10.0)
            .abs()
            < 1.0e-9
    );
    assert!(
        (scenarios[1]["analog"]["analysis"]["max_step_us"]
            .as_f64()
            .unwrap()
            - 0.1)
            .abs()
            < 1.0e-9
    );
}
