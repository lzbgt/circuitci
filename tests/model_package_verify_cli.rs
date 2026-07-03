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

fn assert_model_package_registry_schema_valid(registry: &Value) {
    let schema: Value = serde_json::from_str(include_str!(
        "../schemas/model_package_registry.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(registry)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "model package registry schema errors: {errors:#?}"
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
fn export_model_package_supports_multiple_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    let source = b"`include \"disciplines.vams\"\nmodule tiny(p,n); inout p,n; endmodule\n";
    let osdi = b"compiled ngspice osdi fixture\n";
    let xyce_plugin = b"compiled xyce plugin fixture\n";
    let conformance = b"{\"result\":\"pass\",\"solver\":\"fixture\"}\n";
    fs::write(dir.path().join("tiny.va"), source).unwrap();
    fs::write(dir.path().join("tiny.osdi"), osdi).unwrap();
    fs::write(dir.path().join("tiny_xyce_plugin.so"), xyce_plugin).unwrap();
    fs::write(dir.path().join("tiny_conformance.json"), conformance).unwrap();
    let lock = dir.path().join("multi_artifact_model.lock.json");
    let registry = dir.path().join("multi_artifact_registry.json");

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .arg("export-model-package")
        .args([
            "--package-name",
            "org.circuitci.test.multi_artifact_model",
            "--package-version",
            "2.0.0",
            "--package-artifact",
            &format!(
                "id=tiny_source,path={},artifact_format=verilog_a_source",
                dir.path().join("tiny.va").display()
            ),
            "--package-artifact",
            &format!(
                "id=tiny_osdi,path={},artifact_format=osdi_shared_object,compiler=openvaf",
                dir.path().join("tiny.osdi").display()
            ),
            "--package-artifact",
            &format!(
                "id=tiny_xyce_plugin,path={},artifact_format=xyce_adms_plugin,compiler=xyce_adms",
                dir.path().join("tiny_xyce_plugin.so").display()
            ),
            "--package-artifact",
            &format!(
                "id=tiny_conformance,path={},artifact_format=model_conformance_report",
                dir.path().join("tiny_conformance.json").display()
            ),
            "--output",
            lock.to_str().unwrap(),
            "--registry-output",
            registry.to_str().unwrap(),
            "--registry-entry",
            "tiny_multi_artifact_qualified",
            "--registry-artifact-id",
            "tiny_osdi",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    let lock_value: Value = serde_json::from_str(&fs::read_to_string(&lock).unwrap()).unwrap();
    assert_model_package_lock_schema_valid(&lock_value);
    let artifacts = lock_value["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 4);
    assert_eq!(artifacts[0]["id"], "tiny_source");
    assert_eq!(artifacts[0]["path"], "tiny.va");
    assert_eq!(artifacts[0]["sha256"], sha256_hex(source));
    assert_eq!(artifacts[1]["id"], "tiny_osdi");
    assert_eq!(artifacts[1]["compiler"], "openvaf");
    assert_eq!(artifacts[2]["id"], "tiny_xyce_plugin");
    assert_eq!(artifacts[2]["compiler"], "xyce_adms");
    assert_eq!(artifacts[3]["id"], "tiny_conformance");
    let registry_value: Value =
        serde_json::from_str(&fs::read_to_string(&registry).unwrap()).unwrap();
    assert_eq!(registry_value["packages"][0]["artifact_id"], "tiny_osdi");

    let report_path = dir.path().join("multi_artifact_verification.json");
    let verify_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "verify-model-package",
            lock.to_str().unwrap(),
            "--registry",
            registry.to_str().unwrap(),
            "--registry-entry",
            "tiny_multi_artifact_qualified",
            "--output",
            report_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(verify_status.success());
    let report: Value = serde_json::from_str(&fs::read_to_string(report_path).unwrap()).unwrap();
    assert_eq!(report["result"], "pass");
    assert_eq!(report["artifacts"].as_array().unwrap().len(), 4);
    assert_model_package_report_schema_valid(&report);
}

#[test]
fn export_model_package_rejects_unknown_registry_artifact() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tiny.va"), b"module tiny; endmodule\n").unwrap();
    let lock = dir.path().join("bad_registry_artifact.lock.json");
    let registry = dir.path().join("bad_registry_artifact_registry.json");

    let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .arg("export-model-package")
        .args([
            "--package-name",
            "org.circuitci.test.bad_registry_artifact",
            "--package-version",
            "1.0.0",
            "--package-artifact",
            &format!(
                "id=tiny_source,path={},artifact_format=verilog_a_source",
                dir.path().join("tiny.va").display()
            ),
            "--output",
            lock.to_str().unwrap(),
            "--registry-output",
            registry.to_str().unwrap(),
            "--registry-entry",
            "bad_registry_artifact",
            "--registry-artifact-id",
            "missing_runtime",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--registry-artifact-id missing_runtime"));
}

#[test]
fn merge_model_package_registry_imports_exported_entries() {
    let dir = tempfile::tempdir().unwrap();
    let pkg_a = dir.path().join("pkg_a");
    let pkg_b = dir.path().join("pkg_b");
    let shared = dir.path().join("shared");
    fs::create_dir_all(&pkg_a).unwrap();
    fs::create_dir_all(&pkg_b).unwrap();
    fs::create_dir_all(&shared).unwrap();
    fs::write(pkg_a.join("a.osdi"), b"package a osdi\n").unwrap();
    fs::write(pkg_b.join("b.osdi"), b"package b osdi\n").unwrap();

    for (dir, package, artifact, entry) in [
        (
            &pkg_a,
            "org.circuitci.test.pkg_a",
            "a.osdi",
            "pkg_a_runtime",
        ),
        (
            &pkg_b,
            "org.circuitci.test.pkg_b",
            "b.osdi",
            "pkg_b_runtime",
        ),
    ] {
        let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
            .args([
                "export-model-package",
                "--package-name",
                package,
                "--package-version",
                "1.0.0",
                "--artifact-id",
                entry,
                "--artifact",
                dir.join(artifact).to_str().unwrap(),
                "--artifact-format",
                "osdi_shared_object",
                "--compiler",
                "openvaf",
                "--output",
                dir.join("package.lock.json").to_str().unwrap(),
                "--registry-output",
                dir.join("package_registry.json").to_str().unwrap(),
                "--registry-entry",
                entry,
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }

    let merged_registry = shared.join("compact_model_registry.json");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "merge-model-package-registry",
            "--input",
            pkg_b.join("package_registry.json").to_str().unwrap(),
            "--input",
            pkg_a.join("package_registry.json").to_str().unwrap(),
            "--output",
            merged_registry.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let registry: Value =
        serde_json::from_str(&fs::read_to_string(&merged_registry).unwrap()).unwrap();
    assert_model_package_registry_schema_valid(&registry);
    let packages = registry["packages"].as_array().unwrap();
    assert_eq!(packages.len(), 2);
    assert_eq!(packages[0]["id"], "pkg_a_runtime");
    assert_eq!(packages[0]["lock_path"], "../pkg_a/package.lock.json");
    assert_eq!(packages[1]["id"], "pkg_b_runtime");
    assert_eq!(packages[1]["lock_path"], "../pkg_b/package.lock.json");

    let report_path = dir.path().join("shared_registry_verification.json");
    let verify_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "verify-model-package",
            pkg_a.join("package.lock.json").to_str().unwrap(),
            "--registry",
            merged_registry.to_str().unwrap(),
            "--registry-entry",
            "pkg_a_runtime",
            "--output",
            report_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(verify_status.success());
}

#[test]
fn merge_model_package_registry_rejects_conflicting_duplicate_entries() {
    let dir = tempfile::tempdir().unwrap();
    let left = dir.path().join("left");
    let right = dir.path().join("right");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    fs::write(left.join("left.lock.json"), "{}").unwrap();
    fs::write(right.join("right.lock.json"), "{}").unwrap();
    let left_sha = sha256_hex(b"{}");
    let right_sha = sha256_hex(b"{}");
    fs::write(
        left.join("registry.json"),
        format!(
            r#"{{
  "schema_version": "circuitci.model_package_registry.v1",
  "packages": [
    {{
      "id": "duplicate",
      "package": {{ "name": "org.circuitci.left", "version": "1.0.0" }},
      "artifact_id": "left_osdi",
      "lock_path": "left.lock.json",
      "lock_sha256": "{left_sha}"
    }}
  ]
}}
"#
        ),
    )
    .unwrap();
    fs::write(
        right.join("registry.json"),
        format!(
            r#"{{
  "schema_version": "circuitci.model_package_registry.v1",
  "packages": [
    {{
      "id": "duplicate",
      "package": {{ "name": "org.circuitci.right", "version": "1.0.0" }},
      "artifact_id": "right_osdi",
      "lock_path": "right.lock.json",
      "lock_sha256": "{right_sha}"
    }}
  ]
}}
"#
        ),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "merge-model-package-registry",
            "--input",
            left.join("registry.json").to_str().unwrap(),
            "--input",
            right.join("registry.json").to_str().unwrap(),
            "--output",
            dir.path().join("merged.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate"));
    assert!(stderr.contains("conflicts"));
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
