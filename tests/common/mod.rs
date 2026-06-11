#![allow(dead_code)]

use serde_json::Value;
use std::process::Command;

pub fn run_validation(project: &str) -> Value {
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

pub fn run_validation_with_path(project: &str, path: &std::path::Path) -> Value {
    run_validation_with_path_and_env(project, path, &[])
}

pub fn run_validation_with_path_and_env(
    project: &str,
    path: &std::path::Path,
    envs: &[(&str, &str)],
) -> Value {
    let out_dir = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_circuitci"));
    command
        .args([
            "validate",
            project,
            "--profile",
            "iot_basic_v0",
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .env("PATH", path);
    for (key, value) in envs {
        command.env(key, value);
    }
    let status = command.status().unwrap();
    assert!(status.success());
    serde_json::from_str(&std::fs::read_to_string(out_dir.path().join("report.json")).unwrap())
        .unwrap()
}

pub fn run_validation_with_env(project: &str, envs: &[(&str, &str)]) -> Value {
    let out_dir = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_circuitci"));
    command.args([
        "validate",
        project,
        "--profile",
        "iot_basic_v0",
        "--output",
        out_dir.path().to_str().unwrap(),
    ]);
    for (key, value) in envs {
        command.env(key, value);
    }
    let status = command.status().unwrap();
    assert!(status.success());
    serde_json::from_str(&std::fs::read_to_string(out_dir.path().join("report.json")).unwrap())
        .unwrap()
}

pub fn embedded_backend_project(project: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let repo = std::env::current_dir().unwrap();
    let text = std::fs::read_to_string(project)
        .unwrap()
        .replace("backend: auto", "backend: embedded_ngspice")
        .replace("../../libs", &repo.join("libs").to_string_lossy())
        .replace("../../models", &repo.join("models").to_string_lossy());
    let path = dir.path().join("project.yaml");
    std::fs::write(&path, text).unwrap();
    (dir, path)
}

pub fn binary_available(binary: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| dir.join(binary).is_file())
}

pub fn assert_yaml_file_valid(path: &std::path::Path, validator: &jsonschema::Validator) {
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

pub fn assert_report_schema_valid(report: &Value) {
    let schema: Value =
        serde_json::from_str(include_str!("../../schemas/report.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(report)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "report schema errors: {errors:#?}");
}

pub fn assert_suite_report_schema_valid(report: &Value) {
    let schema: Value =
        serde_json::from_str(include_str!("../../schemas/suite_report.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(report)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "suite report schema errors: {errors:#?}");
}

pub fn assert_no_generated_solver_artifacts(report: &Value) {
    let artifacts = report["artifacts"].as_array().unwrap();
    for suffix in [
        "generated_board.cir",
        "circuitci_ngspice.cir",
        "ngspice.log",
        "waveform.csv",
    ] {
        assert!(
            !artifacts
                .iter()
                .any(|artifact| artifact.as_str().unwrap().ends_with(suffix)),
            "unexpected generated solver artifact {suffix} in {artifacts:#?}"
        );
    }
}
