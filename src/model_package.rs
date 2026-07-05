use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const MODEL_PACKAGE_VERIFICATION_SCHEMA: &str = "circuitci.model_package_verification.v1";
pub const MODEL_PACKAGE_LOCK_SCHEMA: &str = "circuitci.model_package_lock.v1";
pub const MODEL_PACKAGE_REGISTRY_SCHEMA: &str = "circuitci.model_package_registry.v1";
pub const MODEL_CONFORMANCE_REPORT_SCHEMA: &str = "circuitci.model_conformance_report.v1";
pub const MODEL_PACKAGE_BUNDLE_SCHEMA: &str = "circuitci.model_package_bundle.v1";

mod export;

pub use export::{
    export_model_conformance_report, export_model_package_bundle, export_model_package_lock,
    merge_model_package_registries,
};

#[derive(Debug, Clone)]
pub struct ModelPackageVerifyOptions {
    pub lock: PathBuf,
    pub registry: Option<PathBuf>,
    pub registry_entry: Option<String>,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModelPackageExportOptions {
    pub package_name: String,
    pub package_version: String,
    pub artifacts: Vec<ModelPackageExportArtifactInput>,
    pub output: PathBuf,
    pub registry_output: Option<PathBuf>,
    pub registry_entry: Option<String>,
    pub registry_artifact_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModelPackageRegistryMergeOptions {
    pub base: Option<PathBuf>,
    pub inputs: Vec<PathBuf>,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModelPackageBundleExportOptions {
    pub lock: PathBuf,
    pub registry: Option<PathBuf>,
    pub registry_entry: Option<String>,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModelConformanceReportExportOptions {
    pub validation_report: PathBuf,
    pub package_name: String,
    pub package_version: String,
    pub artifact_id: String,
    pub runtime_artifact: PathBuf,
    pub check_name: String,
    pub analysis: String,
    pub solver: Option<String>,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModelPackageExportArtifactInput {
    pub id: String,
    pub artifact: PathBuf,
    pub artifact_format: String,
    pub compiler: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPackageExportSummary {
    pub lock_path: String,
    pub lock_sha256: String,
    pub artifact_id: String,
    pub artifact_path: String,
    pub artifact_sha256: String,
    pub artifacts: Vec<ModelPackageExportArtifactSummary>,
    pub registry_path: Option<String>,
    pub registry_sha256: Option<String>,
    pub registry_entry: Option<String>,
    pub registry_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPackageExportArtifactSummary {
    pub id: String,
    pub artifact_path: String,
    pub artifact_sha256: String,
    pub artifact_format: String,
    pub compiler: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPackageRegistryMergeSummary {
    pub registry_path: String,
    pub registry_sha256: String,
    pub entries: usize,
    pub input_registries: usize,
    pub deduplicated_entries: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelConformanceReportExportSummary {
    pub output: String,
    pub sha256: String,
    pub result: String,
    pub package_name: String,
    pub package_version: String,
    pub artifact_id: String,
    pub runtime_artifact_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPackageBundleExportSummary {
    pub output: String,
    pub manifest_path: String,
    pub manifest_sha256: String,
    pub lock_path: String,
    pub lock_sha256: String,
    pub registry_path: Option<String>,
    pub registry_sha256: Option<String>,
    pub verification_report: String,
    pub artifact_count: usize,
    pub conformance_check_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ModelPackageRegistryEntry {
    id: String,
    package_name: String,
    package_version: String,
    artifact_id: String,
    lock_path: String,
    lock_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPackageVerificationReport {
    pub schema_version: String,
    pub result: String,
    pub lock: VerifiedLock,
    pub registry: Option<VerifiedRegistry>,
    pub artifacts: Vec<VerifiedModelArtifact>,
    pub conformance_checks: Vec<VerifiedModelConformanceCheck>,
    pub findings: Vec<ModelPackageFinding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifiedLock {
    pub path: String,
    pub sha256: Option<String>,
    pub package_name: Option<String>,
    pub package_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifiedRegistry {
    pub path: String,
    pub sha256: Option<String>,
    pub entry: String,
    pub lock_path: Option<String>,
    pub lock_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifiedModelArtifact {
    pub id: String,
    pub path: Option<String>,
    pub artifact_format: Option<String>,
    pub compiler: Option<String>,
    pub sha256_declared: Option<String>,
    pub sha256_actual: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifiedModelConformanceCheck {
    pub report_artifact_id: String,
    pub report_path: String,
    pub target_artifact_id: Option<String>,
    pub target_artifact_sha256: Option<String>,
    pub check_name: Option<String>,
    pub analysis: Option<String>,
    pub solver: Option<String>,
    pub result: Option<String>,
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPackageFinding {
    pub id: String,
    pub severity: String,
    pub message: String,
    pub package: Option<String>,
    pub artifact_id: Option<String>,
    pub path: Option<String>,
    pub expected_sha256: Option<String>,
    pub actual_sha256: Option<String>,
}

pub fn verify_model_package(
    options: &ModelPackageVerifyOptions,
) -> Result<ModelPackageVerificationReport> {
    let mut findings = Vec::new();
    let mut conformance_checks = Vec::new();
    let lock_text = read_text(
        &options.lock,
        &mut findings,
        "MODEL_PACKAGE_LOCK_UNAVAILABLE",
    );
    let lock_sha = file_sha256_hex(&options.lock).ok();
    let lock_value = lock_text.as_deref().and_then(|text| {
        parse_model_package_document(text, &mut findings, "MODEL_PACKAGE_LOCK_INVALID")
    });
    let package_name = lock_value.as_ref().and_then(|value| {
        string_field(value, &["package", "name"])
            .or_else(|| string_field(value, &["package_name"]))
            .or_else(|| string_field(value, &["name"]))
    });
    let package_version = lock_value.as_ref().and_then(|value| {
        string_field(value, &["package", "version"])
            .or_else(|| string_field(value, &["package_version"]))
            .or_else(|| string_field(value, &["version"]))
    });
    if package_name.is_none() {
        findings.push(finding(
            "MODEL_PACKAGE_LOCK_PACKAGE_MISSING",
            "Model package lock must declare package.name.",
            None,
            None,
            Some(&options.lock),
            None,
            None,
        ));
    }
    if package_version.is_none() {
        findings.push(finding(
            "MODEL_PACKAGE_LOCK_PACKAGE_MISSING",
            "Model package lock must declare package.version.",
            package_name.as_deref(),
            None,
            Some(&options.lock),
            None,
            None,
        ));
    }
    let artifacts = lock_value
        .as_ref()
        .map(|value| {
            verify_lock_artifacts(
                value,
                &options.lock,
                package_name.as_deref(),
                package_version.as_deref(),
                &mut findings,
                &mut conformance_checks,
            )
        })
        .unwrap_or_default();
    let registry = verify_registry(
        options,
        &lock_sha,
        package_name.as_deref(),
        package_version.as_deref(),
        &mut findings,
    );
    let result = if findings
        .iter()
        .any(|finding| finding.severity == "critical")
    {
        "fail"
    } else {
        "pass"
    };
    Ok(ModelPackageVerificationReport {
        schema_version: MODEL_PACKAGE_VERIFICATION_SCHEMA.to_string(),
        result: result.to_string(),
        lock: VerifiedLock {
            path: options.lock.to_string_lossy().to_string(),
            sha256: lock_sha,
            package_name,
            package_version,
        },
        registry,
        artifacts,
        conformance_checks,
        findings,
    })
}

pub fn write_model_package_verification_report(
    report: &ModelPackageVerificationReport,
    output: &Path,
) -> Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(report)?;
    std::fs::write(output, text)
        .with_context(|| format!("Failed to write model package report {}", output.display()))?;
    let markdown = model_package_verification_markdown(report);
    std::fs::write(output.with_extension("md"), markdown).with_context(|| {
        format!(
            "Failed to write model package markdown report {}",
            output.with_extension("md").display()
        )
    })?;
    Ok(())
}

fn model_package_verification_markdown(report: &ModelPackageVerificationReport) -> String {
    let mut text = String::new();
    text.push_str("# CircuitCI Model Package Verification\n\n");
    text.push_str("## Summary\n\n");
    text.push_str(&format!(
        "- Result: `{}`\n- Package: `{}` `{}`\n- Artifacts: {}\n- Conformance checks: {}\n- Findings: {}\n\n",
        report.result,
        report.lock.package_name.as_deref().unwrap_or(""),
        report.lock.package_version.as_deref().unwrap_or(""),
        report.artifacts.len(),
        report.conformance_checks.len(),
        report.findings.len()
    ));
    text.push_str("## Artifacts\n\n");
    if report.artifacts.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for artifact in &report.artifacts {
            text.push_str(&format!(
                "- `{}` [{}] status `{}` sha `{}`\n",
                artifact.id,
                artifact.artifact_format.as_deref().unwrap_or(""),
                artifact.status,
                artifact.sha256_actual.as_deref().unwrap_or("")
            ));
        }
        text.push('\n');
    }
    text.push_str("## Conformance Checks\n\n");
    if report.conformance_checks.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for check in &report.conformance_checks {
            text.push_str(&format!(
                "- `{}` `{}` via `{}`: `{}` target `{}` `{}`\n",
                check.check_name.as_deref().unwrap_or(""),
                check.analysis.as_deref().unwrap_or(""),
                check.solver.as_deref().unwrap_or(""),
                check.result.as_deref().unwrap_or(""),
                check.target_artifact_id.as_deref().unwrap_or(""),
                check.target_artifact_sha256.as_deref().unwrap_or("")
            ));
            text.push_str(&format!(
                "  - Report: `{}` `{}`\n",
                check.report_artifact_id, check.report_path
            ));
            if !check.artifacts.is_empty() {
                text.push_str(&format!(
                    "  - Evidence: `{}`\n",
                    check.artifacts.join("`, `")
                ));
            }
        }
        text.push('\n');
    }
    text.push_str("## Findings\n\n");
    if report.findings.is_empty() {
        text.push_str("None.\n");
    } else {
        for finding in &report.findings {
            text.push_str(&format!(
                "- `{}` [{}] artifact `{}`: {}\n",
                finding.id,
                finding.severity,
                finding.artifact_id.as_deref().unwrap_or(""),
                finding.message
            ));
        }
    }
    text
}

fn verify_lock_artifacts(
    lock: &Value,
    lock_path: &Path,
    package: Option<&str>,
    package_version: Option<&str>,
    findings: &mut Vec<ModelPackageFinding>,
    conformance_checks: &mut Vec<VerifiedModelConformanceCheck>,
) -> Vec<VerifiedModelArtifact> {
    let Some(artifacts) = lock
        .get("artifacts")
        .or_else(|| lock.get("model_artifacts"))
        .and_then(Value::as_array)
    else {
        findings.push(finding(
            "MODEL_PACKAGE_LOCK_ARTIFACTS_MISSING",
            "Model package lock must contain a non-empty artifacts array.",
            package,
            None,
            Some(lock_path),
            None,
            None,
        ));
        return Vec::new();
    };
    if artifacts.is_empty() {
        findings.push(finding(
            "MODEL_PACKAGE_LOCK_ARTIFACTS_MISSING",
            "Model package lock artifacts array must not be empty.",
            package,
            None,
            Some(lock_path),
            None,
            None,
        ));
    }
    let verified = artifacts
        .iter()
        .map(|artifact| verify_artifact(artifact, lock_path, package, findings))
        .collect::<Vec<_>>();
    verify_conformance_reports(
        lock,
        lock_path,
        package,
        package_version,
        findings,
        conformance_checks,
    );
    verified
}

fn verify_artifact(
    artifact: &Value,
    lock_path: &Path,
    package: Option<&str>,
    findings: &mut Vec<ModelPackageFinding>,
) -> VerifiedModelArtifact {
    let id = string_field(artifact, &["id"])
        .or_else(|| string_field(artifact, &["name"]))
        .unwrap_or_else(|| "<missing>".to_string());
    let path = string_field(artifact, &["path"]);
    let declared_sha = string_field(artifact, &["sha256"]);
    let artifact_format = string_field(artifact, &["artifact_format"]);
    let compiler = string_field(artifact, &["compiler"]);
    for (field, value) in [
        (
            "id",
            Some(id.as_str()).filter(|value| *value != "<missing>"),
        ),
        ("path", path.as_deref()),
        ("sha256", declared_sha.as_deref()),
        ("artifact_format", artifact_format.as_deref()),
    ] {
        if value.is_none() {
            findings.push(finding(
                "MODEL_PACKAGE_LOCK_ARTIFACT_FIELD_MISSING",
                &format!("Model package artifact {id} must declare {field}."),
                package,
                Some(&id),
                path.as_deref().map(Path::new),
                None,
                None,
            ));
        }
    }
    let actual_sha = path.as_deref().and_then(|relative| {
        let artifact_path = lock_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(relative);
        match file_sha256_hex(&artifact_path) {
            Ok(actual) => Some(actual),
            Err(message) => {
                findings.push(finding(
                    "MODEL_PACKAGE_ARTIFACT_UNAVAILABLE",
                    &message.to_string(),
                    package,
                    Some(&id),
                    Some(&artifact_path),
                    declared_sha.as_deref(),
                    None,
                ));
                None
            }
        }
    });
    if let (Some(expected), Some(actual)) = (declared_sha.as_deref(), actual_sha.as_deref())
        && !expected.eq_ignore_ascii_case(actual)
    {
        findings.push(finding(
            "MODEL_PACKAGE_ARTIFACT_HASH_MISMATCH",
            "Model package artifact SHA-256 does not match the lock.",
            package,
            Some(&id),
            path.as_deref().map(Path::new),
            Some(expected),
            Some(actual),
        ));
    }
    let status = if actual_sha
        .as_deref()
        .zip(declared_sha.as_deref())
        .is_some_and(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
    {
        "verified"
    } else {
        "failed"
    };
    VerifiedModelArtifact {
        id,
        path,
        artifact_format,
        compiler,
        sha256_declared: declared_sha,
        sha256_actual: actual_sha,
        status: status.to_string(),
    }
}

fn verify_conformance_reports(
    lock: &Value,
    lock_path: &Path,
    package: Option<&str>,
    package_version: Option<&str>,
    findings: &mut Vec<ModelPackageFinding>,
    conformance_checks: &mut Vec<VerifiedModelConformanceCheck>,
) {
    let Some(artifacts) = lock
        .get("artifacts")
        .or_else(|| lock.get("model_artifacts"))
        .and_then(Value::as_array)
    else {
        return;
    };
    for artifact in artifacts {
        if string_field(artifact, &["artifact_format"]).as_deref()
            != Some("model_conformance_report")
        {
            continue;
        }
        let id = string_field(artifact, &["id"])
            .or_else(|| string_field(artifact, &["name"]))
            .unwrap_or_else(|| "<missing>".to_string());
        let Some(path) = string_field(artifact, &["path"]) else {
            continue;
        };
        let report_path = lock_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&path);
        let Ok(text) = std::fs::read_to_string(&report_path) else {
            continue;
        };
        let Some(report) = parse_model_package_document(
            &text,
            findings,
            "MODEL_PACKAGE_CONFORMANCE_REPORT_INVALID",
        ) else {
            continue;
        };
        validate_conformance_report(
            &report,
            artifacts,
            &ConformanceReportContext {
                report_id: &id,
                report_path: &report_path,
                package,
                package_version,
            },
            findings,
            conformance_checks,
        );
    }
}

fn validate_conformance_report(
    report: &Value,
    artifacts: &[Value],
    context: &ConformanceReportContext<'_>,
    findings: &mut Vec<ModelPackageFinding>,
    conformance_checks: &mut Vec<VerifiedModelConformanceCheck>,
) {
    compare_conformance_field(
        "schema_version",
        Some(MODEL_CONFORMANCE_REPORT_SCHEMA),
        string_field(report, &["schema_version"]).as_deref(),
        context.report_id,
        context.report_path,
        context.package,
        findings,
    );
    compare_conformance_field(
        "package.name",
        context.package,
        string_field(report, &["package", "name"]).as_deref(),
        context.report_id,
        context.report_path,
        context.package,
        findings,
    );
    compare_conformance_field(
        "package.version",
        context.package_version,
        string_field(report, &["package", "version"]).as_deref(),
        context.report_id,
        context.report_path,
        context.package,
        findings,
    );
    let target_artifact_id = string_field(report, &["artifact_id"]);
    let target_artifact = target_artifact_id.as_deref().and_then(|target| {
        artifacts
            .iter()
            .find(|artifact| artifact_id(artifact) == Some(target))
    });
    match (target_artifact_id.as_deref(), target_artifact) {
        (Some(_), Some(target)) => {
            if string_field(target, &["artifact_format"]).as_deref()
                == Some("model_conformance_report")
            {
                findings.push(finding(
                    "MODEL_PACKAGE_CONFORMANCE_REPORT_MISMATCH",
                    "Model conformance report must target a runtime/source artifact, not another conformance report.",
                    context.package,
                    Some(context.report_id),
                    Some(context.report_path),
                    None,
                    None,
                ));
            }
            let expected_sha = string_field(target, &["sha256"]);
            compare_conformance_field(
                "runtime_artifact_sha256",
                expected_sha.as_deref(),
                string_field(report, &["runtime_artifact_sha256"]).as_deref(),
                context.report_id,
                context.report_path,
                context.package,
                findings,
            );
        }
        (Some(target), None) => findings.push(finding(
            "MODEL_PACKAGE_CONFORMANCE_REPORT_MISMATCH",
            "Model conformance report artifact_id does not match any package artifact.",
            context.package,
            Some(context.report_id),
            Some(context.report_path),
            Some(target),
            None,
        )),
        (None, _) => findings.push(finding(
            "MODEL_PACKAGE_CONFORMANCE_REPORT_INVALID",
            "Model conformance report must declare artifact_id.",
            context.package,
            Some(context.report_id),
            Some(context.report_path),
            None,
            None,
        )),
    }
    if string_field(report, &["result"]).as_deref() != Some("pass") {
        findings.push(finding(
            "MODEL_PACKAGE_CONFORMANCE_FAILED",
            "Model conformance report result is not pass.",
            context.package,
            Some(context.report_id),
            Some(context.report_path),
            Some("pass"),
            string_field(report, &["result"]).as_deref(),
        ));
    }
    let Some(checks) = report.get("checks").and_then(Value::as_array) else {
        findings.push(finding(
            "MODEL_PACKAGE_CONFORMANCE_REPORT_INVALID",
            "Model conformance report must contain a non-empty checks array.",
            context.package,
            Some(context.report_id),
            Some(context.report_path),
            None,
            None,
        ));
        return;
    };
    if checks.is_empty() {
        findings.push(finding(
            "MODEL_PACKAGE_CONFORMANCE_REPORT_INVALID",
            "Model conformance report checks array must not be empty.",
            context.package,
            Some(context.report_id),
            Some(context.report_path),
            None,
            None,
        ));
    }
    for check in checks {
        let check_name = string_field(check, &["name"]).unwrap_or_else(|| "<missing>".to_string());
        if check_name == "<missing>" || string_field(check, &["analysis"]).is_none() {
            findings.push(finding(
                "MODEL_PACKAGE_CONFORMANCE_REPORT_INVALID",
                "Every model conformance check must declare name and analysis.",
                context.package,
                Some(context.report_id),
                Some(context.report_path),
                None,
                None,
            ));
        }
        if string_field(check, &["result"]).as_deref() != Some("pass") {
            findings.push(finding(
                "MODEL_PACKAGE_CONFORMANCE_FAILED",
                &format!("Model conformance check {check_name} is not pass."),
                context.package,
                Some(context.report_id),
                Some(context.report_path),
                Some("pass"),
                string_field(check, &["result"]).as_deref(),
            ));
        }
        conformance_checks.push(VerifiedModelConformanceCheck {
            report_artifact_id: context.report_id.to_string(),
            report_path: context.report_path.to_string_lossy().to_string(),
            target_artifact_id: target_artifact_id.clone(),
            target_artifact_sha256: string_field(report, &["runtime_artifact_sha256"]),
            check_name: string_field(check, &["name"]),
            analysis: string_field(check, &["analysis"]),
            solver: string_field(check, &["solver"]),
            result: string_field(check, &["result"]),
            artifacts: check
                .get("artifacts")
                .and_then(Value::as_array)
                .map(|artifacts| {
                    artifacts
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
}

struct ConformanceReportContext<'a> {
    report_id: &'a str,
    report_path: &'a Path,
    package: Option<&'a str>,
    package_version: Option<&'a str>,
}

fn compare_conformance_field(
    field: &str,
    expected: Option<&str>,
    actual: Option<&str>,
    report_id: &str,
    report_path: &Path,
    package: Option<&str>,
    findings: &mut Vec<ModelPackageFinding>,
) {
    match (expected, actual) {
        (Some(expected), Some(actual)) if expected == actual => {}
        (Some(expected), Some(actual)) => findings.push(finding(
            "MODEL_PACKAGE_CONFORMANCE_REPORT_MISMATCH",
            &format!("Model conformance report {field} does not match package lock."),
            package,
            Some(report_id),
            Some(report_path),
            Some(expected),
            Some(actual),
        )),
        (Some(expected), None) => findings.push(finding(
            "MODEL_PACKAGE_CONFORMANCE_REPORT_INVALID",
            &format!("Model conformance report must declare {field}."),
            package,
            Some(report_id),
            Some(report_path),
            Some(expected),
            None,
        )),
        (None, Some(_)) | (None, None) => {}
    }
}

fn artifact_id(artifact: &Value) -> Option<&str> {
    artifact
        .get("id")
        .or_else(|| artifact.get("name"))
        .and_then(Value::as_str)
}

fn verify_registry(
    options: &ModelPackageVerifyOptions,
    lock_sha: &Option<String>,
    package_name: Option<&str>,
    package_version: Option<&str>,
    findings: &mut Vec<ModelPackageFinding>,
) -> Option<VerifiedRegistry> {
    let registry_path = options.registry.as_ref()?;
    let Some(entry_id) = options
        .registry_entry
        .as_deref()
        .filter(|entry| !entry.trim().is_empty())
    else {
        findings.push(finding(
            "MODEL_PACKAGE_REGISTRY_ENTRY_MISSING",
            "--registry-entry is required when --registry is supplied.",
            package_name,
            None,
            Some(registry_path),
            None,
            None,
        ));
        return Some(VerifiedRegistry {
            path: registry_path.to_string_lossy().to_string(),
            sha256: file_sha256_hex(registry_path).ok(),
            entry: String::new(),
            lock_path: None,
            lock_sha256: None,
        });
    };
    let registry_sha = file_sha256_hex(registry_path).ok();
    let registry_text = read_text(
        registry_path,
        findings,
        "MODEL_PACKAGE_REGISTRY_UNAVAILABLE",
    );
    let registry_value = registry_text.as_deref().and_then(|text| {
        parse_model_package_document(text, findings, "MODEL_PACKAGE_REGISTRY_INVALID")
    });
    let entry = registry_value
        .as_ref()
        .and_then(|value| model_package_registry_entry(value, entry_id));
    let Some(entry) = entry else {
        findings.push(finding(
            "MODEL_PACKAGE_REGISTRY_ENTRY_MISSING",
            "Model package registry does not contain the requested entry.",
            package_name,
            Some(entry_id),
            Some(registry_path),
            None,
            None,
        ));
        return Some(VerifiedRegistry {
            path: registry_path.to_string_lossy().to_string(),
            sha256: registry_sha,
            entry: entry_id.to_string(),
            lock_path: None,
            lock_sha256: None,
        });
    };
    let entry_package_name = string_field(entry, &["package", "name"])
        .or_else(|| string_field(entry, &["package_name"]))
        .or_else(|| string_field(entry, &["name"]));
    let entry_package_version = string_field(entry, &["package", "version"])
        .or_else(|| string_field(entry, &["package_version"]))
        .or_else(|| string_field(entry, &["version"]));
    let entry_lock_path = string_field(entry, &["lock_path"])
        .or_else(|| string_field(entry, &["model_package_lock_path"]));
    let entry_lock_sha = string_field(entry, &["lock_sha256"])
        .or_else(|| string_field(entry, &["model_package_lock_sha256"]));
    let compare_context = RegistryCompareContext {
        package: package_name,
        entry: entry_id,
        registry_path,
    };
    compare_registry_field(
        "MODEL_PACKAGE_REGISTRY_PACKAGE_MISMATCH",
        "Registry package name does not match lock package name.",
        package_name,
        entry_package_name.as_deref(),
        &compare_context,
        findings,
    );
    compare_registry_field(
        "MODEL_PACKAGE_REGISTRY_PACKAGE_MISMATCH",
        "Registry package version does not match lock package version.",
        package_version,
        entry_package_version.as_deref(),
        &compare_context,
        findings,
    );
    compare_registry_field(
        "MODEL_PACKAGE_REGISTRY_LOCK_HASH_MISMATCH",
        "Registry lock SHA-256 does not match the supplied lock file.",
        lock_sha.as_deref(),
        entry_lock_sha.as_deref(),
        &compare_context,
        findings,
    );
    if let Some(entry_lock_path) = entry_lock_path.as_deref() {
        let expected = normalize_path(
            &registry_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(entry_lock_path),
        );
        let actual = normalize_path(&options.lock);
        if expected != actual {
            findings.push(finding(
                "MODEL_PACKAGE_REGISTRY_LOCK_PATH_MISMATCH",
                "Registry lock path does not resolve to the supplied lock file.",
                package_name,
                Some(entry_id),
                Some(registry_path),
                Some(&expected.to_string_lossy()),
                Some(&actual.to_string_lossy()),
            ));
        }
    } else {
        findings.push(finding(
            "MODEL_PACKAGE_REGISTRY_FIELD_MISSING",
            "Registry entry must declare lock_path.",
            package_name,
            Some(entry_id),
            Some(registry_path),
            None,
            None,
        ));
    }
    if entry_lock_sha.is_none() {
        findings.push(finding(
            "MODEL_PACKAGE_REGISTRY_FIELD_MISSING",
            "Registry entry must declare lock_sha256.",
            package_name,
            Some(entry_id),
            Some(registry_path),
            None,
            None,
        ));
    }
    Some(VerifiedRegistry {
        path: registry_path.to_string_lossy().to_string(),
        sha256: registry_sha,
        entry: entry_id.to_string(),
        lock_path: entry_lock_path,
        lock_sha256: entry_lock_sha,
    })
}

fn compare_registry_field(
    id: &str,
    message: &str,
    expected: Option<&str>,
    actual: Option<&str>,
    context: &RegistryCompareContext<'_>,
    findings: &mut Vec<ModelPackageFinding>,
) {
    match (expected, actual) {
        (Some(expected), Some(actual)) if expected == actual => {}
        (Some(expected), Some(actual)) => findings.push(finding(
            id,
            message,
            context.package,
            Some(context.entry),
            Some(context.registry_path),
            Some(expected),
            Some(actual),
        )),
        (Some(expected), None) => findings.push(finding(
            "MODEL_PACKAGE_REGISTRY_FIELD_MISSING",
            message,
            context.package,
            Some(context.entry),
            Some(context.registry_path),
            Some(expected),
            None,
        )),
        (None, _) => {}
    }
}

struct RegistryCompareContext<'a> {
    package: Option<&'a str>,
    entry: &'a str,
    registry_path: &'a Path,
}

fn read_text(path: &Path, findings: &mut Vec<ModelPackageFinding>, id: &str) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) => {
            findings.push(finding(
                id,
                &format!("Unable to read {}: {error}", path.display()),
                None,
                None,
                Some(path),
                None,
                None,
            ));
            None
        }
    }
}

fn parse_model_package_document(
    text: &str,
    findings: &mut Vec<ModelPackageFinding>,
    id: &str,
) -> Option<Value> {
    match serde_json::from_str::<Value>(text).or_else(|_| serde_yaml_ng::from_str::<Value>(text)) {
        Ok(value) => Some(value),
        Err(error) => {
            findings.push(finding(
                id,
                &format!("Model package document is not valid JSON or YAML: {error}"),
                None,
                None,
                None,
                None,
                None,
            ));
            None
        }
    }
}

fn model_package_registry_entry<'a>(registry: &'a Value, entry_id: &str) -> Option<&'a Value> {
    registry
        .get("packages")
        .or_else(|| registry.get("model_packages"))
        .or_else(|| registry.get("entries"))
        .and_then(Value::as_array)?
        .iter()
        .find(|entry| {
            string_field(entry, &["id"])
                .or_else(|| string_field(entry, &["name"]))
                .as_deref()
                == Some(entry_id)
        })
}

fn string_field(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToOwned::to_owned)
}

fn file_sha256_hex(path: &Path) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Unable to read {}", path.display()))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn lock_relative_path(base_dir: &Path, target: &Path) -> Result<String> {
    let base = std::fs::canonicalize(base_dir)
        .with_context(|| format!("Unable to resolve {}", base_dir.display()))?;
    let target = std::fs::canonicalize(target)
        .with_context(|| format!("Unable to resolve {}", target.display()))?;
    let base_components = base.components().collect::<Vec<_>>();
    let target_components = target.components().collect::<Vec<_>>();
    let common = base_components
        .iter()
        .zip(target_components.iter())
        .take_while(|(base, target)| base == target)
        .count();
    let mut path = PathBuf::new();
    for _ in common..base_components.len() {
        path.push("..");
    }
    for component in target_components.iter().skip(common) {
        path.push(component.as_os_str());
    }
    if path.as_os_str().is_empty() {
        path.push(".");
    }
    Ok(path.to_string_lossy().to_string())
}

fn finding(
    id: &str,
    message: &str,
    package: Option<&str>,
    artifact_id: Option<&str>,
    path: Option<&Path>,
    expected_sha256: Option<&str>,
    actual_sha256: Option<&str>,
) -> ModelPackageFinding {
    ModelPackageFinding {
        id: id.to_string(),
        severity: "critical".to_string(),
        message: message.to_string(),
        package: package.map(ToOwned::to_owned),
        artifact_id: artifact_id.map(ToOwned::to_owned),
        path: path.map(|path| path.to_string_lossy().to_string()),
        expected_sha256: expected_sha256.map(ToOwned::to_owned),
        actual_sha256: actual_sha256.map(ToOwned::to_owned),
    }
}
