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

pub fn export_model_package_lock(
    options: &ModelPackageExportOptions,
) -> Result<ModelPackageExportSummary> {
    validate_export_options(options)?;
    let lock_parent = options.output.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(lock_parent).with_context(|| {
        format!(
            "Failed to create model package output directory {}",
            lock_parent.display()
        )
    })?;
    let artifacts = options
        .artifacts
        .iter()
        .map(|artifact| {
            Ok(ModelPackageExportArtifactSummary {
                id: artifact.id.clone(),
                artifact_path: lock_relative_path(lock_parent, &artifact.artifact)?,
                artifact_sha256: file_sha256_hex(&artifact.artifact)?,
                artifact_format: artifact.artifact_format.clone(),
                compiler: artifact.compiler.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let primary = primary_registry_artifact(options, &artifacts)?;
    let lock_text = render_lock_document(options, &artifacts);
    std::fs::write(&options.output, lock_text.as_bytes()).with_context(|| {
        format!(
            "Failed to write model package lock {}",
            options.output.display()
        )
    })?;
    let lock_sha = file_sha256_hex(&options.output)?;
    let mut registry_path = None;
    let mut registry_sha = None;
    if let Some(output) = &options.registry_output {
        let registry_parent = output.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(registry_parent).with_context(|| {
            format!(
                "Failed to create model package registry directory {}",
                registry_parent.display()
            )
        })?;
        let entry = options
            .registry_entry
            .as_deref()
            .unwrap_or(primary.id.as_str());
        let lock_path = lock_relative_path(registry_parent, &options.output)?;
        let registry_text =
            render_registry_document(options, entry, &primary.id, &lock_path, &lock_sha);
        std::fs::write(output, registry_text.as_bytes()).with_context(|| {
            format!(
                "Failed to write model package registry {}",
                output.display()
            )
        })?;
        registry_sha = Some(file_sha256_hex(output)?);
        registry_path = Some(output.to_string_lossy().to_string());
    }
    Ok(ModelPackageExportSummary {
        lock_path: options.output.to_string_lossy().to_string(),
        lock_sha256: lock_sha,
        artifact_id: primary.id.clone(),
        artifact_path: primary.artifact_path.clone(),
        artifact_sha256: primary.artifact_sha256.clone(),
        artifacts,
        registry_path,
        registry_sha256: registry_sha,
        registry_entry: options.registry_entry.clone(),
        registry_artifact_id: options.registry_artifact_id.clone(),
    })
}

pub fn merge_model_package_registries(
    options: &ModelPackageRegistryMergeOptions,
) -> Result<ModelPackageRegistryMergeSummary> {
    validate_registry_merge_options(options)?;
    let output_parent = options.output.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(output_parent).with_context(|| {
        format!(
            "Failed to create model package registry output directory {}",
            output_parent.display()
        )
    })?;
    let mut entries = BTreeMap::new();
    let mut deduplicated_entries = 0usize;
    let mut registry_paths = Vec::new();
    if let Some(base) = &options.base {
        registry_paths.push(base.clone());
    }
    registry_paths.extend(options.inputs.iter().cloned());
    for registry in &registry_paths {
        for entry in read_registry_entries_for_merge(registry, output_parent)? {
            match entries.get(&entry.id) {
                Some(existing) if existing == &entry => {
                    deduplicated_entries += 1;
                }
                Some(_) => {
                    anyhow::bail!(
                        "Model package registry entry {} conflicts with an existing entry.",
                        entry.id
                    );
                }
                None => {
                    entries.insert(entry.id.clone(), entry);
                }
            }
        }
    }
    let registry_text = render_registry_entries_document(entries.values());
    std::fs::write(&options.output, registry_text.as_bytes()).with_context(|| {
        format!(
            "Failed to write model package registry {}",
            options.output.display()
        )
    })?;
    Ok(ModelPackageRegistryMergeSummary {
        registry_path: options.output.to_string_lossy().to_string(),
        registry_sha256: file_sha256_hex(&options.output)?,
        entries: entries.len(),
        input_registries: registry_paths.len(),
        deduplicated_entries,
    })
}

pub fn export_model_package_bundle(
    options: &ModelPackageBundleExportOptions,
) -> Result<ModelPackageBundleExportSummary> {
    validate_bundle_export_options(options)?;
    let source_report = verify_model_package(&ModelPackageVerifyOptions {
        lock: options.lock.clone(),
        registry: options.registry.clone(),
        registry_entry: options.registry_entry.clone(),
        output: options
            .output
            .join("source_model_package_verification.json"),
    })?;
    if source_report.result != "pass" {
        anyhow::bail!(
            "Source model package verification failed for {}; export a bundle only from a passing package.",
            options.lock.display()
        );
    }
    let lock_text = std::fs::read_to_string(&options.lock).with_context(|| {
        format!(
            "Unable to read model package lock {}",
            options.lock.display()
        )
    })?;
    let mut parse_findings = Vec::new();
    let mut lock = parse_model_package_document(
        &lock_text,
        &mut parse_findings,
        "MODEL_PACKAGE_LOCK_INVALID",
    )
    .with_context(|| {
        format!(
            "Model package lock {} is not valid JSON or YAML.",
            options.lock.display()
        )
    })?;
    let package_name = string_field(&lock, &["package", "name"])
        .or_else(|| string_field(&lock, &["package_name"]))
        .or_else(|| string_field(&lock, &["name"]))
        .context("Model package lock must declare package.name.")?;
    let package_version = string_field(&lock, &["package", "version"])
        .or_else(|| string_field(&lock, &["package_version"]))
        .or_else(|| string_field(&lock, &["version"]))
        .context("Model package lock must declare package.version.")?;
    std::fs::create_dir_all(options.output.join("artifacts")).with_context(|| {
        format!(
            "Failed to create model package bundle directory {}",
            options.output.display()
        )
    })?;
    let source_parent = options.lock.parent().unwrap_or_else(|| Path::new("."));
    let mut copied_artifacts = Vec::new();
    let mut used_paths = BTreeSet::new();
    let artifacts = lock_artifacts_mut(&mut lock)?;
    for artifact in artifacts {
        let id = required_value_string(artifact, &["id"], "model package artifact id")?;
        let source_relative =
            required_value_string(artifact, &["path"], "model package artifact path")?;
        let source_path = source_parent.join(&source_relative);
        let file_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| {
                format!("Model package artifact {source_relative} has no file name.")
            })?;
        let mut bundled_relative = format!(
            "artifacts/{}__{}",
            sanitize_bundle_name(&id),
            sanitize_bundle_name(file_name)
        );
        let mut duplicate_index = 2usize;
        while !used_paths.insert(bundled_relative.clone()) {
            bundled_relative = format!(
                "artifacts/{}__{}__{}",
                sanitize_bundle_name(&id),
                duplicate_index,
                sanitize_bundle_name(file_name)
            );
            duplicate_index += 1;
        }
        let destination = options.output.join(&bundled_relative);
        std::fs::copy(&source_path, &destination).with_context(|| {
            format!(
                "Failed to copy model package artifact {} to {}",
                source_path.display(),
                destination.display()
            )
        })?;
        set_object_string(artifact, "path", &bundled_relative)?;
        copied_artifacts.push(ModelPackageExportArtifactSummary {
            id,
            artifact_path: bundled_relative,
            artifact_sha256: file_sha256_hex(&destination)?,
            artifact_format: string_field(artifact, &["artifact_format"]).unwrap_or_default(),
            compiler: string_field(artifact, &["compiler"]),
        });
    }
    set_object_string(&mut lock, "schema_version", MODEL_PACKAGE_LOCK_SCHEMA)?;
    let lock_path = options.output.join("package.lock.json");
    write_json_value(&lock_path, &lock)?;
    let lock_sha = file_sha256_hex(&lock_path)?;
    let (registry_path, registry_sha, registry_entry_id) =
        bundled_registry(options, &package_name, &package_version, &lock_sha)?;
    let verification_path = options.output.join("model_package_verification.json");
    let bundled_report = verify_model_package(&ModelPackageVerifyOptions {
        lock: lock_path.clone(),
        registry: registry_path.clone(),
        registry_entry: registry_entry_id,
        output: verification_path.clone(),
    })?;
    write_model_package_verification_report(&bundled_report, &verification_path)?;
    let readme_path = options.output.join("README.md");
    std::fs::write(
        &readme_path,
        model_package_bundle_readme(
            &package_name,
            &package_version,
            &copied_artifacts,
            &bundled_report,
        ),
    )
    .with_context(|| {
        format!(
            "Failed to write model package bundle README {}",
            readme_path.display()
        )
    })?;
    let manifest_path = options.output.join("model_package_bundle_manifest.json");
    let manifest = serde_json::json!({
        "schema_version": MODEL_PACKAGE_BUNDLE_SCHEMA,
        "package": {
            "name": package_name,
            "version": package_version,
        },
        "lock_path": "package.lock.json",
        "lock_sha256": lock_sha,
        "registry_path": registry_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned),
        "registry_sha256": registry_sha,
        "verification_report": "model_package_verification.json",
        "verification_markdown": "model_package_verification.md",
        "readme": "README.md",
        "artifacts": copied_artifacts.clone(),
        "conformance_checks": bundled_report.conformance_checks.clone(),
    });
    write_json_value(&manifest_path, &manifest)?;
    Ok(ModelPackageBundleExportSummary {
        output: options.output.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        manifest_sha256: file_sha256_hex(&manifest_path)?,
        lock_path: lock_path.to_string_lossy().to_string(),
        lock_sha256: lock_sha,
        registry_path: registry_path.map(|path| path.to_string_lossy().to_string()),
        registry_sha256: registry_sha,
        verification_report: verification_path.to_string_lossy().to_string(),
        artifact_count: copied_artifacts.len(),
        conformance_check_count: bundled_report.conformance_checks.len(),
    })
}

pub fn export_model_conformance_report(
    options: &ModelConformanceReportExportOptions,
) -> Result<ModelConformanceReportExportSummary> {
    validate_conformance_export_options(options)?;
    let runtime_sha = file_sha256_hex(&options.runtime_artifact)?;
    let report_text = std::fs::read_to_string(&options.validation_report).with_context(|| {
        format!(
            "Unable to read validation report {}",
            options.validation_report.display()
        )
    })?;
    let report: Value = serde_json::from_str(&report_text).with_context(|| {
        format!(
            "Validation report {} is not valid JSON.",
            options.validation_report.display()
        )
    })?;
    let report_result = string_field(&report, &["result"])
        .with_context(|| "Validation report must declare result.".to_string())?;
    let critical_failures = report
        .get("failures")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let result = if report_result == "pass" && critical_failures == 0 {
        "pass"
    } else {
        "fail"
    };
    let mut check = serde_json::Map::new();
    check.insert(
        "name".to_string(),
        Value::String(options.check_name.clone()),
    );
    check.insert(
        "analysis".to_string(),
        Value::String(options.analysis.clone()),
    );
    if let Some(solver) = options.solver.as_deref() {
        check.insert("solver".to_string(), Value::String(solver.to_string()));
    }
    check.insert("result".to_string(), Value::String(result.to_string()));
    if let Some(artifacts) = report.get("artifacts").and_then(Value::as_array) {
        let artifact_values = artifacts
            .iter()
            .filter_map(Value::as_str)
            .map(|artifact| Value::String(artifact.to_string()))
            .collect::<Vec<_>>();
        if !artifact_values.is_empty() {
            check.insert("artifacts".to_string(), Value::Array(artifact_values));
        }
    }
    let conformance = serde_json::json!({
        "schema_version": MODEL_CONFORMANCE_REPORT_SCHEMA,
        "package": {
            "name": options.package_name,
            "version": options.package_version,
        },
        "artifact_id": options.artifact_id,
        "runtime_artifact_sha256": runtime_sha,
        "result": result,
        "checks": [Value::Object(check)],
        "source": options.validation_report.to_string_lossy(),
    });
    if let Some(parent) = options.output.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create conformance report output directory {}",
                parent.display()
            )
        })?;
    }
    let mut text = serde_json::to_string_pretty(&conformance)?;
    text.push('\n');
    std::fs::write(&options.output, text).with_context(|| {
        format!(
            "Failed to write model conformance report {}",
            options.output.display()
        )
    })?;
    Ok(ModelConformanceReportExportSummary {
        output: options.output.to_string_lossy().to_string(),
        sha256: file_sha256_hex(&options.output)?,
        result: result.to_string(),
        package_name: options.package_name.clone(),
        package_version: options.package_version.clone(),
        artifact_id: options.artifact_id.clone(),
        runtime_artifact_sha256: runtime_sha,
    })
}

fn validate_conformance_export_options(
    options: &ModelConformanceReportExportOptions,
) -> Result<()> {
    for (field, value) in [
        ("package-name", options.package_name.as_str()),
        ("package-version", options.package_version.as_str()),
        ("artifact-id", options.artifact_id.as_str()),
        ("check-name", options.check_name.as_str()),
        ("analysis", options.analysis.as_str()),
    ] {
        if value.trim().is_empty() {
            anyhow::bail!("--{field} must not be empty.");
        }
    }
    if let Some(solver) = options.solver.as_deref()
        && solver.trim().is_empty()
    {
        anyhow::bail!("--solver must not be empty when supplied.");
    }
    if !options.validation_report.is_file() {
        anyhow::bail!(
            "Validation report {} is missing.",
            options.validation_report.display()
        );
    }
    if !options.runtime_artifact.is_file() {
        anyhow::bail!(
            "Runtime artifact {} is missing.",
            options.runtime_artifact.display()
        );
    }
    Ok(())
}

fn validate_registry_merge_options(options: &ModelPackageRegistryMergeOptions) -> Result<()> {
    if options.base.is_none() && options.inputs.is_empty() {
        anyhow::bail!("merge-model-package-registry requires --base or at least one --input.");
    }
    for path in options.base.iter().chain(options.inputs.iter()) {
        if !path.is_file() {
            anyhow::bail!("Model package registry {} is missing.", path.display());
        }
    }
    Ok(())
}

fn validate_bundle_export_options(options: &ModelPackageBundleExportOptions) -> Result<()> {
    if !options.lock.is_file() {
        anyhow::bail!("Model package lock {} is missing.", options.lock.display());
    }
    if options.registry.is_some() && options.registry_entry.is_none() {
        anyhow::bail!("--registry-entry is required when --registry is supplied.");
    }
    if let Some(registry) = &options.registry
        && !registry.is_file()
    {
        anyhow::bail!("Model package registry {} is missing.", registry.display());
    }
    Ok(())
}

fn lock_artifacts_mut(lock: &mut Value) -> Result<&mut Vec<Value>> {
    if lock.get("artifacts").is_some() {
        lock.get_mut("artifacts")
            .and_then(Value::as_array_mut)
            .context("Model package lock artifacts must be an array.")
    } else {
        lock.get_mut("model_artifacts")
            .and_then(Value::as_array_mut)
            .context("Model package lock must contain an artifacts array.")
    }
}

fn required_value_string(value: &Value, path: &[&str], description: &str) -> Result<String> {
    string_field(value, path).with_context(|| format!("Missing {description}."))
}

fn set_object_string(value: &mut Value, key: &str, text: &str) -> Result<()> {
    let object = value
        .as_object_mut()
        .with_context(|| format!("Expected JSON object while setting {key}."))?;
    object.insert(key.to_string(), Value::String(text.to_string()));
    Ok(())
}

fn write_json_value(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    std::fs::write(path, text).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn bundled_registry(
    options: &ModelPackageBundleExportOptions,
    package_name: &str,
    package_version: &str,
    lock_sha: &str,
) -> Result<(Option<PathBuf>, Option<String>, Option<String>)> {
    let Some(registry_path) = &options.registry else {
        return Ok((None, None, None));
    };
    let entry_id = options
        .registry_entry
        .as_deref()
        .context("--registry-entry is required when --registry is supplied.")?;
    let text = std::fs::read_to_string(registry_path).with_context(|| {
        format!(
            "Unable to read model package registry {}",
            registry_path.display()
        )
    })?;
    let mut findings = Vec::new();
    let registry =
        parse_model_package_document(&text, &mut findings, "MODEL_PACKAGE_REGISTRY_INVALID")
            .with_context(|| {
                format!(
                    "Model package registry {} is not valid JSON or YAML.",
                    registry_path.display()
                )
            })?;
    let entry = model_package_registry_entry(&registry, entry_id)
        .with_context(|| format!("Model package registry does not contain entry {entry_id}."))?;
    let artifact_id = string_field(entry, &["artifact_id"])
        .or_else(|| string_field(entry, &["model_package_artifact_id"]))
        .with_context(|| {
            format!("Model package registry entry {entry_id} must declare artifact_id.")
        })?;
    let bundled_entry = ModelPackageRegistryEntry {
        id: entry_id.to_string(),
        package_name: package_name.to_string(),
        package_version: package_version.to_string(),
        artifact_id,
        lock_path: "package.lock.json".to_string(),
        lock_sha256: lock_sha.to_string(),
    };
    let output = options.output.join("compact_model_registry.json");
    std::fs::write(
        &output,
        render_registry_entries_document([&bundled_entry].into_iter()).as_bytes(),
    )
    .with_context(|| {
        format!(
            "Failed to write bundled model package registry {}",
            output.display()
        )
    })?;
    Ok((
        Some(output.clone()),
        Some(file_sha256_hex(&output)?),
        Some(entry_id.to_string()),
    ))
}

fn sanitize_bundle_name(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        sanitized.push_str("artifact");
    }
    sanitized
}

fn model_package_bundle_readme(
    package_name: &str,
    package_version: &str,
    artifacts: &[ModelPackageExportArtifactSummary],
    report: &ModelPackageVerificationReport,
) -> String {
    let mut text = String::new();
    text.push_str(&format!(
        "# CircuitCI Model Package Bundle: {package_name} {package_version}\n\n"
    ));
    text.push_str("- `package.lock.json`: rewritten lock with bundled artifact paths\n");
    text.push_str("- `model_package_verification.json`: machine-readable verification report\n");
    text.push_str("- `model_package_verification.md`: human-readable verification summary\n");
    text.push_str("- `model_package_bundle_manifest.json`: bundle manifest and hashes\n");
    text.push_str("- `artifacts/`: source/runtime/conformance artifacts\n\n");
    text.push_str("## Artifacts\n\n");
    for artifact in artifacts {
        text.push_str(&format!(
            "- `{}` [{}] `{}` `{}`\n",
            artifact.id, artifact.artifact_format, artifact.artifact_path, artifact.artifact_sha256
        ));
    }
    text.push_str("\n## Conformance Checks\n\n");
    if report.conformance_checks.is_empty() {
        text.push_str("None.\n");
    } else {
        for check in &report.conformance_checks {
            text.push_str(&format!(
                "- `{}` `{}` via `{}`: `{}` target `{}`\n",
                check.check_name.as_deref().unwrap_or(""),
                check.analysis.as_deref().unwrap_or(""),
                check.solver.as_deref().unwrap_or(""),
                check.result.as_deref().unwrap_or(""),
                check.target_artifact_id.as_deref().unwrap_or("")
            ));
        }
    }
    text
}

fn read_registry_entries_for_merge(
    registry_path: &Path,
    output_parent: &Path,
) -> Result<Vec<ModelPackageRegistryEntry>> {
    let text = std::fs::read_to_string(registry_path).with_context(|| {
        format!(
            "Unable to read model package registry {}",
            registry_path.display()
        )
    })?;
    let mut findings = Vec::new();
    let value =
        parse_model_package_document(&text, &mut findings, "MODEL_PACKAGE_REGISTRY_INVALID")
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Model package registry {} is not valid JSON or YAML.",
                    registry_path.display()
                )
            })?;
    let package_entries = value
        .get("packages")
        .or_else(|| value.get("model_packages"))
        .or_else(|| value.get("entries"))
        .and_then(Value::as_array)
        .context("Model package registry must contain a packages array.")?;
    let registry_parent = registry_path.parent().unwrap_or_else(|| Path::new("."));
    package_entries
        .iter()
        .map(|entry| {
            parse_registry_entry_for_merge(entry, registry_path, registry_parent, output_parent)
        })
        .collect()
}

fn parse_registry_entry_for_merge(
    entry: &Value,
    registry_path: &Path,
    registry_parent: &Path,
    output_parent: &Path,
) -> Result<ModelPackageRegistryEntry> {
    let id = required_registry_string(entry, &["id"], registry_path)?;
    let package_name = required_registry_string(entry, &["package", "name"], registry_path)
        .or_else(|_| required_registry_string(entry, &["package_name"], registry_path))
        .or_else(|_| required_registry_string(entry, &["name"], registry_path))?;
    let package_version = required_registry_string(entry, &["package", "version"], registry_path)
        .or_else(|_| required_registry_string(entry, &["package_version"], registry_path))
        .or_else(|_| required_registry_string(entry, &["version"], registry_path))?;
    let artifact_id =
        required_registry_string(entry, &["artifact_id"], registry_path).or_else(|_| {
            required_registry_string(entry, &["model_package_artifact_id"], registry_path)
        })?;
    let lock_path =
        required_registry_string(entry, &["lock_path"], registry_path).or_else(|_| {
            required_registry_string(entry, &["model_package_lock_path"], registry_path)
        })?;
    let lock_sha256 =
        required_registry_string(entry, &["lock_sha256"], registry_path).or_else(|_| {
            required_registry_string(entry, &["model_package_lock_sha256"], registry_path)
        })?;
    let absolute_lock_path = registry_parent.join(&lock_path);
    let output_lock_path = lock_relative_path(output_parent, &absolute_lock_path)?;
    Ok(ModelPackageRegistryEntry {
        id,
        package_name,
        package_version,
        artifact_id,
        lock_path: output_lock_path,
        lock_sha256,
    })
}

fn required_registry_string(entry: &Value, path: &[&str], registry_path: &Path) -> Result<String> {
    let value = string_field(entry, path).with_context(|| {
        format!(
            "Model package registry {} entry is missing {}.",
            registry_path.display(),
            path.join(".")
        )
    })?;
    if value.trim().is_empty() {
        anyhow::bail!(
            "Model package registry {} entry has empty {}.",
            registry_path.display(),
            path.join(".")
        );
    }
    Ok(value)
}

fn validate_export_options(options: &ModelPackageExportOptions) -> Result<()> {
    for (field, value) in [
        ("package-name", options.package_name.as_str()),
        ("package-version", options.package_version.as_str()),
    ] {
        if value.trim().is_empty() {
            anyhow::bail!("--{field} must not be empty.");
        }
    }
    if options.registry_output.is_none() && options.registry_entry.is_some() {
        anyhow::bail!("--registry-entry requires --registry-output.");
    }
    if options.registry_output.is_none() && options.registry_artifact_id.is_some() {
        anyhow::bail!("--registry-artifact-id requires --registry-output.");
    }
    if options.artifacts.is_empty() {
        anyhow::bail!("Model package export requires at least one artifact.");
    }
    let mut ids = BTreeSet::new();
    for artifact in &options.artifacts {
        for (field, value) in [
            ("artifact id", artifact.id.as_str()),
            ("artifact format", artifact.artifact_format.as_str()),
        ] {
            if value.trim().is_empty() {
                anyhow::bail!("Model package {field} must not be empty.");
            }
        }
        if let Some(compiler) = artifact.compiler.as_deref()
            && compiler.trim().is_empty()
        {
            anyhow::bail!("Model package artifact compiler must not be empty when supplied.");
        }
        if !ids.insert(artifact.id.as_str()) {
            anyhow::bail!("Duplicate model package artifact id {}.", artifact.id);
        }
        if !artifact.artifact.is_file() {
            anyhow::bail!(
                "Model package artifact {} is missing.",
                artifact.artifact.display()
            );
        }
    }
    Ok(())
}

fn primary_registry_artifact<'a>(
    options: &ModelPackageExportOptions,
    artifacts: &'a [ModelPackageExportArtifactSummary],
) -> Result<&'a ModelPackageExportArtifactSummary> {
    if let Some(artifact_id) = options.registry_artifact_id.as_deref() {
        return artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id)
            .with_context(|| {
                format!("--registry-artifact-id {artifact_id} does not match an exported artifact.")
            });
    }
    artifacts
        .first()
        .context("Model package export requires at least one artifact.")
}

fn render_lock_document(
    options: &ModelPackageExportOptions,
    artifacts: &[ModelPackageExportArtifactSummary],
) -> String {
    let artifact_rows = artifacts
        .iter()
        .map(render_lock_artifact)
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema_version\": {},\n  \"package\": {{\n    \"name\": {},\n    \"version\": {}\n  }},\n  \"artifacts\": [\n{}\n  ]\n}}\n",
        json_string(MODEL_PACKAGE_LOCK_SCHEMA),
        json_string(&options.package_name),
        json_string(&options.package_version),
        artifact_rows,
    )
}

fn render_lock_artifact(artifact: &ModelPackageExportArtifactSummary) -> String {
    let compiler = artifact
        .compiler
        .as_deref()
        .map(|compiler| format!(",\n      \"compiler\": {}", json_string(compiler)))
        .unwrap_or_default();
    format!(
        "    {{\n      \"id\": {},\n      \"path\": {},\n      \"sha256\": {},\n      \"artifact_format\": {}{}\n    }}",
        json_string(&artifact.id),
        json_string(&artifact.artifact_path),
        json_string(&artifact.artifact_sha256),
        json_string(&artifact.artifact_format),
        compiler,
    )
}

fn render_registry_document(
    options: &ModelPackageExportOptions,
    entry: &str,
    artifact_id: &str,
    lock_path: &str,
    lock_sha: &str,
) -> String {
    format!(
        "{{\n  \"schema_version\": {},\n  \"packages\": [\n    {{\n      \"id\": {},\n      \"package\": {{\n        \"name\": {},\n        \"version\": {}\n      }},\n      \"artifact_id\": {},\n      \"lock_path\": {},\n      \"lock_sha256\": {}\n    }}\n  ]\n}}\n",
        json_string(MODEL_PACKAGE_REGISTRY_SCHEMA),
        json_string(entry),
        json_string(&options.package_name),
        json_string(&options.package_version),
        json_string(artifact_id),
        json_string(lock_path),
        json_string(lock_sha),
    )
}

fn render_registry_entries_document<'a>(
    entries: impl IntoIterator<Item = &'a ModelPackageRegistryEntry>,
) -> String {
    let rows = entries
        .into_iter()
        .map(render_registry_entry)
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema_version\": {},\n  \"packages\": [\n{}\n  ]\n}}\n",
        json_string(MODEL_PACKAGE_REGISTRY_SCHEMA),
        rows,
    )
}

fn render_registry_entry(entry: &ModelPackageRegistryEntry) -> String {
    format!(
        "    {{\n      \"id\": {},\n      \"package\": {{\n        \"name\": {},\n        \"version\": {}\n      }},\n      \"artifact_id\": {},\n      \"lock_path\": {},\n      \"lock_sha256\": {}\n    }}",
        json_string(&entry.id),
        json_string(&entry.package_name),
        json_string(&entry.package_version),
        json_string(&entry.artifact_id),
        json_string(&entry.lock_path),
        json_string(&entry.lock_sha256),
    )
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization cannot fail")
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
