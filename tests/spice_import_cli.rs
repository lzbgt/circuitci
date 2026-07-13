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

#[test]
fn import_spice_creates_dc_sweep_scenario_for_dc_deck() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("dc_sweep_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 0\nR1 in out 1k\nR2 out 0 1k\n.dc V1 0 1 0.5\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_dc_sweep_deck",
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
    assert_eq!(scenarios[0]["type"], "analog_dc_sweep");
    assert_eq!(
        scenarios[0]["checks"],
        Value::Array(vec![Value::String("SPICE_DC_SWEEP_ANALYSIS".to_string())])
    );
    let analysis = &scenarios[0]["analog"]["analysis"];
    assert_eq!(analysis["type"], "dc_sweep");
    assert_eq!(analysis["dc_sweep_source"], "V1");
    assert_eq!(analysis["dc_sweep_start"], 0.0);
    assert_eq!(analysis["dc_sweep_stop"], 1.0);
    assert_eq!(analysis["dc_sweep_step"], 0.5);
    assert!(analysis.get("stop_time_us").is_none());

    let report = run_validation(output.to_str().unwrap());
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert!(
            report["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact.as_str().unwrap().ends_with("dc_sweep.csv"))
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_spice_creates_op_and_dc_sweep_scenarios_when_both_requested() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("op_dc_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 0\nR1 in out 1k\nR2 out 0 1k\n.control\nop\ndc V1 0 1 0.5\n.endc\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_op_dc_deck",
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
    assert_eq!(scenarios[1]["type"], "analog_dc_sweep");
    assert_eq!(scenarios[1]["analog"]["analysis"]["type"], "dc_sweep");
    assert_eq!(scenarios[1]["analog"]["analysis"]["dc_sweep_source"], "V1");
}

#[test]
fn import_spice_creates_ac_scenario_for_ac_deck() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("ac_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 AC 1\nR1 in out 1k\nC1 out 0 100n\n.ac dec 10 10 100k\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_ac_deck",
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
    assert_eq!(scenarios[0]["type"], "analog_ac");
    assert_eq!(
        scenarios[0]["checks"],
        Value::Array(vec![Value::String("SPICE_AC_ANALYSIS".to_string())])
    );
    let analysis = &scenarios[0]["analog"]["analysis"];
    assert_eq!(analysis["type"], "ac");
    assert_eq!(analysis["start_frequency_hz"], 10.0);
    assert_eq!(analysis["stop_frequency_hz"], 100000.0);
    assert_eq!(analysis["points_per_decade"], 10);
    assert!(analysis.get("stop_time_us").is_none());

    let report = run_validation(output.to_str().unwrap());
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert!(
            report["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact.as_str().unwrap().ends_with("bode.csv"))
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_spice_creates_op_and_ac_scenarios_when_both_requested() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("op_ac_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 AC 1\nR1 in out 1k\nC1 out 0 100n\n.control\nop\nac dec 10 10 100k\n.endc\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_op_ac_deck",
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
    assert_eq!(scenarios[1]["type"], "analog_ac");
    assert_eq!(scenarios[1]["analog"]["analysis"]["type"], "ac");
    assert_eq!(
        scenarios[1]["analog"]["analysis"]["start_frequency_hz"],
        10.0
    );
}

#[test]
fn import_spice_creates_noise_scenario_for_noise_deck() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("noise_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 0 AC 1\nR1 in out 1k\nR2 out 0 1k\n.noise V(out) V1 dec 10 10 100k\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_noise_deck",
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
    assert_eq!(scenarios[0]["type"], "analog_noise");
    assert_eq!(
        scenarios[0]["checks"],
        Value::Array(vec![Value::String("SPICE_NOISE_ANALYSIS".to_string())])
    );
    let analysis = &scenarios[0]["analog"]["analysis"];
    assert_eq!(analysis["type"], "noise");
    assert_eq!(analysis["noise_output_node"], "out");
    assert_eq!(analysis["noise_input_source"], "V1");
    assert_eq!(analysis["start_frequency_hz"], 10.0);
    assert_eq!(analysis["stop_frequency_hz"], 100000.0);
    assert_eq!(analysis["points_per_decade"], 10);
    assert!(analysis.get("stop_time_us").is_none());

    let report = run_validation(output.to_str().unwrap());
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert!(
            report["waveforms"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact.as_str().unwrap().ends_with("noise_spectrum.csv"))
        );
        assert!(
            report["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact.as_str().unwrap().ends_with("noise_total.csv"))
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_spice_creates_op_and_noise_scenarios_when_both_requested() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("op_noise_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 0 AC 1\nR1 in out 1k\nR2 out 0 1k\n.control\nop\nnoise V(out) V1 dec 10 10 100k\n.endc\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_op_noise_deck",
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
    assert_eq!(scenarios[1]["type"], "analog_noise");
    assert_eq!(scenarios[1]["analog"]["analysis"]["type"], "noise");
    assert_eq!(
        scenarios[1]["analog"]["analysis"]["noise_output_node"],
        "out"
    );
}

#[test]
fn import_spice_creates_transfer_function_scenario_for_tf_deck() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("tf_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 1\nR1 in out 1k\nR2 out 0 1k\n.tf V(out) V1\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_tf_deck",
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
    assert_eq!(scenarios[0]["type"], "analog_transfer_function");
    assert_eq!(
        scenarios[0]["checks"],
        Value::Array(vec![Value::String(
            "SPICE_TRANSFER_FUNCTION_ANALYSIS".to_string()
        )])
    );
    let analysis = &scenarios[0]["analog"]["analysis"];
    assert_eq!(analysis["type"], "tf");
    assert_eq!(analysis["transfer_output_expression"], "V(out)");
    assert_eq!(analysis["transfer_input_source"], "V1");
    assert!(analysis.get("stop_time_us").is_none());

    let report = run_validation(output.to_str().unwrap());
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert!(
            report["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact
                    .as_str()
                    .unwrap()
                    .ends_with("transfer_function_summary.csv"))
        );
        assert_eq!(
            report["transfer_function_summaries"][0]["input_source"],
            "V1"
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_spice_creates_op_and_transfer_function_scenarios_when_both_requested() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("op_tf_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 1\nR1 in out 1k\nR2 out 0 1k\n.control\nop\ntf V(out) V1\n.endc\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_op_tf_deck",
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
    assert_eq!(scenarios[1]["type"], "analog_transfer_function");
    assert_eq!(scenarios[1]["analog"]["analysis"]["type"], "tf");
    assert_eq!(
        scenarios[1]["analog"]["analysis"]["transfer_output_expression"],
        "V(out)"
    );
}

#[test]
fn import_spice_creates_pole_zero_scenario_for_pz_deck() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("pz_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 1\nR1 in out 1k\nC1 out 0 100n\n.pz in 0 out 0 vol pz\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_pz_deck",
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
    assert_eq!(scenarios[0]["type"], "analog_pole_zero");
    assert_eq!(
        scenarios[0]["checks"],
        Value::Array(vec![Value::String("SPICE_POLE_ZERO_ANALYSIS".to_string())])
    );
    let analysis = &scenarios[0]["analog"]["analysis"];
    assert_eq!(analysis["type"], "pz");
    assert_eq!(analysis["pole_zero_output_node"], "out");
    assert_eq!(analysis["pole_zero_reference_node"], "0");
    assert_eq!(analysis["pole_zero_input_source"], "V1");
    assert_eq!(analysis["pole_zero_mode"], "poles_and_zeros");
    assert!(analysis.get("stop_time_us").is_none());

    let report = run_validation(output.to_str().unwrap());
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert!(
            report["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact
                    .as_str()
                    .unwrap()
                    .ends_with("pole_zero_summary.csv"))
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_spice_creates_op_and_control_pole_zero_scenarios_when_both_requested() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("op_pz_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "I1 in 0 DC 1m\nR1 in out 1k\nC1 out 0 100n\n.control\nop\npz in 0 out 0 cur pol\n.endc\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_op_pz_deck",
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
    assert_eq!(scenarios[1]["type"], "analog_pole_zero");
    assert_eq!(scenarios[1]["analog"]["analysis"]["type"], "pz");
    assert_eq!(
        scenarios[1]["analog"]["analysis"]["pole_zero_input_source"],
        "I1"
    );
    assert_eq!(
        scenarios[1]["analog"]["analysis"]["pole_zero_mode"],
        "poles"
    );
}

#[test]
fn import_spice_rejects_pole_zero_without_matching_input_source() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("bad_pz_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 source 0 DC 1\nR1 source out 1k\nC1 out 0 100n\n.pz in 0 out 0 vol pz\n.end\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_bad_pz_deck",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr
            .contains("SPICE .pz input nodes in 0 must match exactly one imported voltage source")
    );
}

#[test]
fn import_spice_creates_sensitivity_scenario_for_sens_deck() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("sensitivity_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 1\nR1 in out 1k\nR2 out 0 1k\n.sens V(out)\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_sensitivity_deck",
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
    assert_eq!(scenarios[0]["type"], "analog_sensitivity");
    assert_eq!(
        scenarios[0]["checks"],
        Value::Array(vec![Value::String(
            "SPICE_SENSITIVITY_ANALYSIS".to_string()
        )])
    );
    let analysis = &scenarios[0]["analog"]["analysis"];
    assert_eq!(analysis["type"], "sens");
    assert_eq!(analysis["sensitivity_output_expression"], "V(out)");
    assert_eq!(analysis["sensitivity_mode"], "dc");
    assert_eq!(
        analysis["sensitivity_filters"],
        Value::Array(vec![
            Value::String("R1".to_string()),
            Value::String("R2".to_string())
        ])
    );
    assert!(analysis.get("stop_time_us").is_none());

    let report = run_validation(output.to_str().unwrap());
    if binary_available("ngspice") || binary_available("Xyce") || binary_available("xyce") {
        assert_eq!(report["result"], "pass");
        assert!(
            report["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact
                    .as_str()
                    .unwrap()
                    .ends_with("sensitivity_summary.csv"))
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_spice_creates_control_ac_sensitivity_scenario() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("ac_sensitivity_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 AC 1\nR1 in out 1k\nC1 out 0 100n\n.control\nsens V(out) ac dec 5 10 1k\n.endc\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_ac_sensitivity_deck",
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
    assert_eq!(scenarios[0]["type"], "analog_sensitivity");
    let analysis = &scenarios[0]["analog"]["analysis"];
    assert_eq!(analysis["type"], "sens");
    assert_eq!(analysis["sensitivity_mode"], "ac");
    assert_eq!(analysis["sensitivity_output_expression"], "V(out)");
    assert_eq!(analysis["points_per_decade"], 5);
    assert_eq!(analysis["start_frequency_hz"], 10.0);
    assert_eq!(analysis["stop_frequency_hz"], 1000.0);
}

#[test]
fn import_spice_rejects_unsupported_sensitivity_card() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("bad_sensitivity_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 1\nR1 in 0 1k\n.sens par('v(in)*v(in)')\n.end\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_bad_sensitivity_deck",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SPICE .sens output must be a voltage or current expression"));
}

#[test]
fn import_spice_creates_distortion_scenario_for_disto_deck() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("distortion_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "VIN out 0 DC 0.2 DISTOF1 1.0 0.0\nD1 out 0 DMOD\n.model DMOD D(Is=1e-14 N=1)\n.disto dec 3 1k 10k\n.print disto I(VIN)\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_distortion_deck",
            "--backend",
            "ngspice",
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
    assert_eq!(scenarios[0]["type"], "analog_distortion");
    assert_eq!(
        scenarios[0]["checks"],
        Value::Array(vec![Value::String("SPICE_DISTORTION_ANALYSIS".to_string())])
    );
    let analysis = &scenarios[0]["analog"]["analysis"];
    assert_eq!(analysis["type"], "disto");
    assert_eq!(analysis["distortion_mode"], "harmonic");
    assert_eq!(analysis["distortion_start_frequency_hz"], 1000.0);
    assert_eq!(analysis["distortion_stop_frequency_hz"], 10000.0);
    assert_eq!(analysis["distortion_points_per_decade"], 3);
    assert_eq!(analysis["distortion_output_expression"], "I(VIN)");
    assert_eq!(
        analysis["distortion_f1_sources"],
        Value::Array(vec![Value::String("VIN".to_string())])
    );
    assert!(analysis.get("distortion_f2_sources").is_none());
    assert!(analysis.get("distortion_f2_over_f1").is_none());
    assert!(analysis.get("stop_time_us").is_none());
}

#[test]
fn import_spice_creates_control_intermodulation_distortion_scenario() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("intermod_disto_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in1 0 DC 0.01 DISTOF1 1.0 0.0\nV2 in2 0 DC 0.002 DISTOF2 1.0 0.0\nR1 in1 out 1k\nR2 in2 out 2k\nC1 out 0 1u\n.control\ndisto dec 20 10 1Meg 0.9\nprint V(out)\n.endc\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_intermod_disto_deck",
            "--backend",
            "ngspice",
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
    assert_eq!(scenarios[0]["type"], "analog_distortion");
    let analysis = &scenarios[0]["analog"]["analysis"];
    assert_eq!(analysis["type"], "disto");
    assert_eq!(analysis["distortion_mode"], "intermodulation");
    assert_eq!(analysis["distortion_points_per_decade"], 20);
    assert_eq!(analysis["distortion_start_frequency_hz"], 10.0);
    assert_eq!(analysis["distortion_stop_frequency_hz"], 1_000_000.0);
    assert_eq!(analysis["distortion_output_expression"], "V(out)");
    assert_eq!(
        analysis["distortion_f1_sources"],
        Value::Array(vec![Value::String("V1".to_string())])
    );
    assert_eq!(
        analysis["distortion_f2_sources"],
        Value::Array(vec![Value::String("V2".to_string())])
    );
    assert!((analysis["distortion_f2_over_f1"].as_f64().unwrap() - 0.9).abs() < 1.0e-12);
}

#[test]
fn import_spice_rejects_distortion_without_output_expression() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("bad_disto_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 out 0 DC 0.2 DISTOF1 1.0 0.0\nD1 out 0 DMOD\n.model DMOD D(Is=1e-14 N=1)\n.disto dec 3 1k 10k\n.end\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_bad_disto_deck",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SPICE .disto import requires a supported output expression"));
}

#[test]
fn import_spice_projects_save_and_probe_outputs_to_scenario_probes() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("save_probe_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 1\nR1 in out 1k\nR2 out 0 1k\n.save all V(in,out)\n.probe I(V1)\n.tran 1u 10u\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_save_probe_deck",
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
    let probes = imported["scenarios"][0]["analog"]["probes"]
        .as_array()
        .unwrap();
    assert!(
        probes
            .iter()
            .any(|probe| probe["expression"] == Value::String("V(in,out)".to_string()))
    );
    assert_eq!(
        probes
            .iter()
            .filter(|probe| probe["expression"] == Value::String("I(V1)".to_string()))
            .count(),
        1
    );
}

#[test]
fn import_spice_projects_print_and_plot_outputs_to_scenario_probes() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("print_plot_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 AC 1\nR1 in out 1k\nC1 out 0 100n\n.print ac V(in,out)\n.plot ac I(V1)\n.ac dec 5 10 1k\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_print_plot_deck",
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
    let probes = imported["scenarios"][0]["analog"]["probes"]
        .as_array()
        .unwrap();
    assert!(
        probes
            .iter()
            .any(|probe| probe["expression"] == Value::String("V(in,out)".to_string()))
    );
    assert!(probes.iter().any(|probe| {
        probe["expression"] == Value::String("I(V1)".to_string())
            && probe["quantity"] == Value::String("current".to_string())
    }));
}

#[test]
fn import_spice_rejects_unsupported_output_probe_expression() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("bad_probe_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 1\nR1 in 0 1k\n.save par('v(in)*v(in)')\n.end\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_bad_probe_deck",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SPICE output directive expression"));
}

#[test]
fn import_spice_projects_global_nodes_to_nets_and_bindings() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("global_deck.cir");
    let model = dir.path().join("amp.lib");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(&model, ".subckt AMP IN OUT\nRLOAD OUT VDD 1k\n.ends AMP\n").unwrap();
    std::fs::write(
        &deck,
        format!(
            ".include {}\n.global VDD\nV1 in 0 DC 1\nXU1 in out AMP\n.tran 1u 10u\n.end\n",
            model.display()
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_global_deck",
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
    assert_eq!(
        imported["board"]["nets"]["net_vdd"]["kind"],
        "digital_or_analog"
    );
    let bindings = imported["scenarios"][0]["analog"]["node_bindings"]
        .as_array()
        .unwrap();
    assert!(bindings.iter().any(|binding| {
        binding["node"] == Value::String("VDD".to_string())
            && binding["net"] == Value::String("net_vdd".to_string())
    }));
}

#[test]
fn import_spice_skips_inline_subckt_definition_internals() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("inline_subckt_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        ".global VDD\n.subckt BUF IN OUT\nRINT OUT VDD 1k\nCINT OUT 0 1n\n.ends BUF\nV1 in 0 DC 1\nXU1 in out BUF\n.tran 1u 10u\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_inline_subckt_deck",
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
    let components = imported["board"]["components"].as_object().unwrap();
    assert!(components.contains_key("V1"));
    assert!(components.contains_key("XU1"));
    assert!(!components.contains_key("RINT"));
    assert!(!components.contains_key("CINT"));
    assert_eq!(
        components["XU1"]["pins"]["P1"],
        Value::String("net_in".to_string())
    );
    assert_eq!(
        imported["board"]["nets"]["net_vdd"]["kind"],
        "digital_or_analog"
    );
}

#[test]
fn import_spice_rejects_unmatched_subckt_end() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("bad_ends_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(&deck, "V1 in 0 DC 1\n.ends BUF\nR1 in 0 1k\n.end\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_bad_ends_deck",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SPICE .ends appears without a preceding .subckt block"));
}

#[test]
fn import_spice_rejects_unclosed_subckt_definition() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("bad_subckt_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        ".subckt BUF IN OUT\nRINT OUT IN 1k\nV1 in 0 DC 1\nXU1 in out BUF\n.end\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_bad_subckt_deck",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SPICE .subckt block is missing a closing .ends"));
}

#[test]
fn import_spice_records_deck_directives_as_review_metadata() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("directive_metadata_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 1\nR1 in out 1k\nC1 out 0 10n\n.temp 85\n.options reltol=1e-4 method=gear\n.ic V(out)=0\n.nodeset V(in)=1\n.tran 1u 10u\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_directive_metadata_deck",
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
    let scenario = &imported["scenarios"][0];
    assert_eq!(
        scenario["analog"]["operating_conditions"]["ambient_temperature_c"],
        85.0
    );
    let directives = &scenario["parameters"]["imported_spice_directives"];
    assert_eq!(directives["ambient_temperature_c"], 85.0);
    assert_eq!(
        directives["temp_cards"],
        Value::Array(vec![Value::String(".temp 85".to_string())])
    );
    assert_eq!(
        directives["option_cards"],
        Value::Array(vec![Value::String(
            ".options reltol=1e-4 method=gear".to_string()
        )])
    );
    assert_eq!(
        directives["initial_condition_cards"],
        Value::Array(vec![Value::String(".ic V(out)=0".to_string())])
    );
    assert_eq!(
        directives["nodeset_cards"],
        Value::Array(vec![Value::String(".nodeset V(in)=1".to_string())])
    );
}

#[test]
fn import_spice_keeps_multi_temp_directive_as_solver_truth_only() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("multi_temp_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 DC 1\nR1 in 0 1k\n.temp -40 25 85\n.tran 1u 10u\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_multi_temp_deck",
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
    let scenario = &imported["scenarios"][0];
    assert!(
        scenario["analog"]
            .as_object()
            .unwrap()
            .get("operating_conditions")
            .is_none()
    );
    let directives = &scenario["parameters"]["imported_spice_directives"];
    assert_eq!(directives["ambiguous_temperature"], true);
    assert_eq!(
        directives["temp_cards"],
        Value::Array(vec![Value::String(".temp -40 25 85".to_string())])
    );
    assert!(directives.get("ambient_temperature_c").is_none());
}

#[test]
fn import_spice_creates_fourier_scenario_for_four_deck() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("fourier_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 SIN(0 1 1k)\nR1 in out 1k\nC1 out 0 100n\n.four 1k V(out)\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_fourier_deck",
            "--stop-time-us",
            "5000",
            "--max-step-us",
            "10",
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
    assert_eq!(scenarios[0]["type"], "analog_fourier");
    assert_eq!(
        scenarios[0]["checks"],
        Value::Array(vec![Value::String("SPICE_FOURIER_ANALYSIS".to_string())])
    );
    let analysis = &scenarios[0]["analog"]["analysis"];
    assert_eq!(analysis["type"], "fourier");
    assert_eq!(analysis["fourier_fundamental_frequency_hz"], 1000.0);
    assert_eq!(analysis["fourier_output_expression"], "V(out)");
    assert_eq!(analysis["stop_time_us"], 5000.0);
    assert_eq!(analysis["max_step_us"], 10.0);

    let report = run_validation(output.to_str().unwrap());
    if binary_available("ngspice") || binary_available("Xyce") || binary_available("xyce") {
        assert_eq!(report["result"], "pass");
        assert!(
            report["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact.as_str().unwrap().ends_with("fourier_summary.csv"))
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_spice_creates_control_fourier_scenarios_for_multiple_outputs() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("control_fourier_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 SIN(0 1 1k)\nR1 in out 1k\nC1 out 0 100n\n.control\ntran 10u 5m\nfourier 1k V(in) V(out)\n.endc\n.end\n",
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_control_fourier_deck",
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
    assert_eq!(scenarios.len(), 3);
    assert_eq!(scenarios[0]["name"], "imported_spice_fourier_1");
    assert_eq!(scenarios[0]["type"], "analog_fourier");
    assert_eq!(
        scenarios[0]["analog"]["analysis"]["fourier_output_expression"],
        "V(in)"
    );
    assert_eq!(scenarios[1]["name"], "imported_spice_fourier_2");
    assert_eq!(scenarios[1]["type"], "analog_fourier");
    assert_eq!(
        scenarios[1]["analog"]["analysis"]["fourier_output_expression"],
        "V(out)"
    );
    assert_eq!(scenarios[2]["type"], "analog_transient");
    assert_eq!(scenarios[2]["analog"]["analysis"]["type"], "tran");
}

#[test]
fn import_spice_rejects_unsupported_fourier_expression() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let deck = dir.path().join("bad_fourier_deck.cir");
    let output = dir.path().join("imported.project.yaml");
    std::fs::write(
        &deck,
        "V1 in 0 SIN(0 1 1k)\nR1 in 0 1k\n.four 1k par('v(in)*v(in)')\n.end\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_bad_fourier_deck",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SPICE .four output must be a voltage or current expression"));
}
