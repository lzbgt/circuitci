use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn assert_model_package_report_schema_valid(report: &Value) {
    let schema: Value = serde_json::from_str(include_str!(
        "../schemas/model_package_verification_report.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(report)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "model package verification report schema errors: {errors:#?}"
    );
}

fn assert_model_package_lock_schema_valid(lock: &Value) {
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/model_package_lock.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(lock)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "model package lock schema errors: {errors:#?}"
    );
}

fn write_package_files(dir: &std::path::Path) -> (String, String) {
    let artifact = b"stable compact model artifact\n";
    let artifact_sha = sha256_hex(artifact);
    fs::write(dir.join("tiny_resistor.osdi"), artifact).unwrap();
    let lock = format!(
        r#"package:
  name: org.circuitci.test.tiny_resistor
  version: 1.0.0
artifacts:
  - id: tiny_resistor_osdi
    path: tiny_resistor.osdi
    sha256: {artifact_sha}
    artifact_format: osdi_shared_object
    compiler: openvaf
"#
    );
    let lock_sha = sha256_hex(lock.as_bytes());
    fs::write(dir.join("compact_model.lock.yaml"), lock).unwrap();
    let registry = format!(
        r#"packages:
  - id: tiny_resistor_qualified_osdi
    package:
      name: org.circuitci.test.tiny_resistor
      version: 1.0.0
    artifact_id: tiny_resistor_osdi
    lock_path: compact_model.lock.yaml
    lock_sha256: {lock_sha}
"#
    );
    fs::write(dir.join("compact_model_registry.yaml"), registry).unwrap();
    (artifact_sha, lock_sha)
}

#[test]
fn verify_model_package_passes_for_hash_pinned_lock_and_registry() {
    let dir = tempfile::tempdir().unwrap();
    let (_artifact_sha, lock_sha) = write_package_files(dir.path());
    let output = dir.path().join("model_package_verification.json");

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "verify-model-package",
            dir.path().join("compact_model.lock.yaml").to_str().unwrap(),
            "--registry",
            dir.path()
                .join("compact_model_registry.yaml")
                .to_str()
                .unwrap(),
            "--registry-entry",
            "tiny_resistor_qualified_osdi",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    let report: Value = serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert_eq!(report["result"], "pass");
    assert_eq!(report["lock"]["sha256"], lock_sha);
    assert_eq!(report["registry"]["entry"], "tiny_resistor_qualified_osdi");
    assert_eq!(report["artifacts"][0]["status"], "verified");
    assert!(report["findings"].as_array().unwrap().is_empty());
    assert_model_package_report_schema_valid(&report);
}

#[test]
fn export_model_package_generates_verifiable_lock_and_registry() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = b"exported compact model artifact\n";
    let artifact_sha = sha256_hex(artifact);
    fs::write(dir.path().join("exported_resistor.osdi"), artifact).unwrap();
    let lock = dir.path().join("exported_model.lock.json");
    let registry = dir.path().join("exported_model_registry.json");

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "export-model-package",
            "--package-name",
            "org.circuitci.test.exported_resistor",
            "--package-version",
            "1.2.3",
            "--artifact-id",
            "exported_resistor_osdi",
            "--artifact",
            dir.path().join("exported_resistor.osdi").to_str().unwrap(),
            "--artifact-format",
            "osdi_shared_object",
            "--compiler",
            "openvaf",
            "--output",
            lock.to_str().unwrap(),
            "--registry-output",
            registry.to_str().unwrap(),
            "--registry-entry",
            "exported_resistor_qualified",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    let lock_value: Value = serde_json::from_str(&fs::read_to_string(&lock).unwrap()).unwrap();
    assert_model_package_lock_schema_valid(&lock_value);
    assert_eq!(
        lock_value["package"]["name"],
        "org.circuitci.test.exported_resistor"
    );
    assert_eq!(lock_value["artifacts"][0]["path"], "exported_resistor.osdi");
    assert_eq!(lock_value["artifacts"][0]["sha256"], artifact_sha);
    let report_path = dir.path().join("generated_package_verification.json");
    let verify_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "verify-model-package",
            lock.to_str().unwrap(),
            "--registry",
            registry.to_str().unwrap(),
            "--registry-entry",
            "exported_resistor_qualified",
            "--output",
            report_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(verify_status.success());
    let report: Value = serde_json::from_str(&fs::read_to_string(report_path).unwrap()).unwrap();
    assert_eq!(report["result"], "pass");
    assert_eq!(report["artifacts"][0]["sha256_actual"], artifact_sha);
    assert_model_package_report_schema_valid(&report);
}

#[test]
fn verify_model_package_fails_for_artifact_hash_mismatch_but_writes_report() {
    let dir = tempfile::tempdir().unwrap();
    write_package_files(dir.path());
    fs::write(
        dir.path().join("tiny_resistor.osdi"),
        b"tampered artifact\n",
    )
    .unwrap();
    let output = dir.path().join("model_package_verification.json");

    let result = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "verify-model-package",
            dir.path().join("compact_model.lock.yaml").to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!result.status.success());
    let report: Value = serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert_eq!(report["result"], "fail");
    assert_eq!(report["artifacts"][0]["status"], "failed");
    assert!(
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| { finding["id"] == "MODEL_PACKAGE_ARTIFACT_HASH_MISMATCH" })
    );
    assert_model_package_report_schema_valid(&report);
}
