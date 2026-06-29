mod common;

use serde_yaml_ng::Value;
use std::process::Command;

#[test]
fn import_manufacturing_metadata_applies_solver_convergence_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_solver_convergence.project.yaml");
    let output = dir.path().join("with_solver_convergence.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let metadata = dir.path().join("solver_convergence.csv");
    let mut project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    let run_logs =
        project_yaml["board"]["manufacturing"]["controlled_impedance"]["solver_run_logs"]
            .as_sequence_mut()
            .unwrap();
    run_logs.clear();
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    let rows = [
        [
            "field",
            "value",
            "source",
            "name",
            "solver",
            "solver_version",
            "run_id",
            "artifact_uri",
            "artifact_sha256",
            "random_seed",
            "numeric_tolerance_policy",
            "max_residual_error",
            "max_iterations",
            "min_convergence_sample_count",
            "max_convergence_impedance_delta_ohm",
            "required_stopping_criteria",
            "solver_run_log",
            "iteration",
            "solved_impedance_ohm",
            "residual_error",
            "stopping_criteria",
        ],
        [
            "controlled_impedance_solver_run_log",
            "",
            "si_solver_run_review_rev_a",
            "reviewed_2d_field_solver_run_log_rf",
            "reviewed_2d_field_solver",
            "2026.07",
            "rf_solver_run_2026_07_a",
            "artifacts/solver/rf_solver_run_2026_07_a.log",
            "aaaabbbbccccddddeeeeffff0000111122223333444455556666777788889999",
            "seed_2026_07_rf",
            "si_solver_tolerance_rev_a",
            "0.000001",
            "120",
            "2",
            "0.04",
            "residual_and_delta",
            "",
            "",
            "",
            "",
            "",
        ],
        [
            "controlled_impedance_solver_convergence_sample",
            "",
            "si_solver_convergence_review_rev_a",
            "rf_solver_converged_92",
            "",
            "",
            "",
            "artifacts/solver/rf_solver_converged_92.json",
            "77776666555544443333222211110000ffffeeeeddddccccbbbbaaaa99998888",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "reviewed_2d_field_solver_run_log_rf",
            "92",
            "50.81",
            "0.0000004",
            "residual_and_delta",
        ],
        [
            "controlled_impedance_solver_convergence_sample",
            "",
            "si_solver_convergence_review_rev_a",
            "rf_solver_converged_96",
            "",
            "",
            "",
            "artifacts/solver/rf_solver_converged_96.json",
            "6666555544443333222211110000ffffeeeeddddccccbbbbaaaa999988887777",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "reviewed_2d_field_solver_run_log_rf",
            "96",
            "50.80",
            "0.0000003",
            "residual_and_delta",
        ],
    ];
    std::fs::write(
        &metadata,
        rows.iter()
            .map(|row| row.join(","))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            input.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let run_log = &enriched["board"]["manufacturing"]["controlled_impedance"]["solver_run_logs"][0];
    assert_eq!(run_log["name"], "reviewed_2d_field_solver_run_log_rf");
    assert_eq!(run_log["min_convergence_sample_count"], 2);
    assert_eq!(run_log["max_convergence_impedance_delta_ohm"], 0.04);
    assert_eq!(run_log["required_stopping_criteria"], "residual_and_delta");
    assert_eq!(
        run_log["convergence_samples"].as_sequence().unwrap().len(),
        2
    );
    assert_eq!(
        run_log["convergence_samples"][0]["name"],
        "rf_solver_converged_92"
    );
    assert_eq!(
        run_log["convergence_samples"][1]["stopping_criteria"],
        "residual_and_delta"
    );

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], "0.39.0");
    assert_eq!(
        manifest["rows"][2]["board_field"],
        "controlled_impedance.solver_run_logs[].convergence_samples[]"
    );
    assert_eq!(
        manifest["rows"][2]["raw_columns"]["solver_run_log"],
        "reviewed_2d_field_solver_run_log_rf"
    );
}
