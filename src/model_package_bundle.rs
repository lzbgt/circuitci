use crate::model_package::{
    ModelPackageFinding, ModelPackageVerificationReport, ModelPackageVerifyOptions,
    VerifiedModelConformanceCheck, verify_model_package,
};
use crate::repair_yaml::{
    BoardYamlRepairFindingKind, BoardYamlRepairOptions, BoardYamlRepairReport,
};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const MODEL_PACKAGE_BUNDLE_VERIFICATION_SCHEMA: &str =
    "circuitci.model_package_bundle_verification.v1";
pub const MODEL_PACKAGE_BUNDLE_INSTALL_SCHEMA: &str = "circuitci.model_package_bundle_install.v1";
pub const MODEL_PACKAGE_BUNDLE_IMPORT_SCHEMA: &str = "circuitci.model_package_bundle_import.v1";
const MODEL_PACKAGE_BUNDLE_SCHEMA: &str = crate::model_package::MODEL_PACKAGE_BUNDLE_SCHEMA;

#[derive(Debug, Clone)]
pub struct ModelPackageBundleVerifyOptions {
    pub bundle: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModelPackageBundleInstallOptions {
    pub bundle: PathBuf,
    pub install_dir: PathBuf,
    pub registry_output: Option<PathBuf>,
    pub registry_entry: Option<String>,
    pub registry_artifact_id: Option<String>,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModelPackageBundleImportOptions {
    pub bundle: PathBuf,
    pub project: PathBuf,
    pub profile: String,
    pub install_dir: PathBuf,
    pub registry_output: Option<PathBuf>,
    pub registry_entry: Option<String>,
    pub registry_artifact_id: Option<String>,
    pub output: PathBuf,
    pub max_runtime_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPackageBundleVerificationReport {
    pub schema_version: String,
    pub result: String,
    pub bundle_path: String,
    pub manifest: VerifiedBundleFile,
    pub package: VerifiedBundlePackage,
    pub lock: Option<VerifiedBundleFile>,
    pub registry: Option<VerifiedBundleFile>,
    pub verification_report: Option<VerifiedBundleFile>,
    pub verification_markdown: Option<VerifiedBundleFile>,
    pub readme: Option<VerifiedBundleFile>,
    pub artifacts: Vec<VerifiedBundleArtifact>,
    pub conformance_checks: Vec<VerifiedModelConformanceCheck>,
    pub findings: Vec<ModelPackageFinding>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct VerifiedBundlePackage {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifiedBundleFile {
    pub path: String,
    pub sha256_declared: Option<String>,
    pub sha256_actual: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifiedBundleArtifact {
    pub id: String,
    pub path: Option<String>,
    pub artifact_format: Option<String>,
    pub compiler: Option<String>,
    pub sha256_declared: Option<String>,
    pub sha256_actual: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPackageBundleInstallReport {
    pub schema_version: String,
    pub result: String,
    pub source_bundle: String,
    pub install_dir: String,
    pub package: VerifiedBundlePackage,
    pub manifest: VerifiedBundleFile,
    pub lock: Option<VerifiedBundleFile>,
    pub registry: Option<VerifiedBundleFile>,
    pub installed_registry: Option<VerifiedBundleFile>,
    pub scenario_import: Option<BundleScenarioImport>,
    pub artifacts: Vec<VerifiedBundleArtifact>,
    pub conformance_checks: Vec<VerifiedModelConformanceCheck>,
    pub findings: Vec<ModelPackageFinding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPackageBundleImportReport {
    pub schema_version: String,
    pub result: String,
    pub bundle_path: String,
    pub project: String,
    pub profile: String,
    pub install_dir: String,
    pub runtime_budget_ms: Option<u64>,
    pub elapsed_ms: u128,
    pub package: VerifiedBundlePackage,
    pub source_bundle_verification_report: String,
    pub bundle_install_report: Option<String>,
    pub package_verification_report: Option<String>,
    pub yaml_repair_report: Option<String>,
    pub repaired_project: Option<String>,
    pub repaired_validation_report: Option<String>,
    pub scenario_import: Option<BundleScenarioImport>,
    pub summary: ModelPackageBundleImportSummary,
    pub findings: Vec<ModelPackageFinding>,
    pub repair_reason_codes: Vec<String>,
    pub repair_messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPackageBundleImportSummary {
    pub bundle_artifacts: usize,
    pub conformance_checks: usize,
    pub package_findings: usize,
    pub repair_proposed: usize,
    pub repair_selected: usize,
    pub repair_applied: usize,
    pub repair_blocked: usize,
    pub repair_skipped: usize,
    pub repair_new_criticals: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleScenarioImport {
    pub model_package_registry_path: String,
    pub model_package_registry_sha256: String,
    pub model_package_registry_entry: String,
    pub model_package_lock_path: String,
    pub model_package_lock_sha256: String,
    pub model_package_artifact_id: String,
}

pub fn verify_model_package_bundle(
    options: &ModelPackageBundleVerifyOptions,
) -> Result<ModelPackageBundleVerificationReport> {
    let mut findings = Vec::new();
    let manifest_path = options.bundle.join("model_package_bundle_manifest.json");
    let manifest = verified_file(
        &manifest_path,
        None,
        "MODEL_PACKAGE_BUNDLE_MANIFEST_UNAVAILABLE",
        "MODEL_PACKAGE_BUNDLE_MANIFEST_HASH_MISMATCH",
        &mut findings,
    );
    let manifest_value = read_manifest(&manifest_path, &mut findings);
    let package = VerifiedBundlePackage {
        name: manifest_value
            .as_ref()
            .and_then(|value| string_field(value, &["package", "name"])),
        version: manifest_value
            .as_ref()
            .and_then(|value| string_field(value, &["package", "version"])),
    };
    if let Some(value) = &manifest_value {
        validate_manifest_header(value, &manifest_path, &package, &mut findings);
    }
    let lock = manifest_value.as_ref().and_then(|manifest| {
        verify_manifest_file(
            manifest,
            &options.bundle,
            ManifestFileSpec {
                path_field: &["lock_path"],
                sha_field: &["lock_sha256"],
                missing_id: "MODEL_PACKAGE_BUNDLE_LOCK_PATH_MISSING",
                mismatch_id: "MODEL_PACKAGE_BUNDLE_LOCK_HASH_MISMATCH",
            },
            &package,
            &mut findings,
        )
    });
    let registry = manifest_value.as_ref().and_then(|manifest| {
        verify_optional_registry(manifest, &options.bundle, &package, &mut findings)
    });
    let verification_report = manifest_value.as_ref().and_then(|manifest| {
        verify_manifest_file_presence(
            manifest,
            &options.bundle,
            &["verification_report"],
            "MODEL_PACKAGE_BUNDLE_VERIFICATION_REPORT_MISSING",
            &package,
            &mut findings,
        )
    });
    let verification_markdown = manifest_value.as_ref().and_then(|manifest| {
        verify_manifest_file_presence(
            manifest,
            &options.bundle,
            &["verification_markdown"],
            "MODEL_PACKAGE_BUNDLE_VERIFICATION_MARKDOWN_MISSING",
            &package,
            &mut findings,
        )
    });
    let readme = manifest_value.as_ref().and_then(|manifest| {
        verify_manifest_file_presence(
            manifest,
            &options.bundle,
            &["readme"],
            "MODEL_PACKAGE_BUNDLE_README_MISSING",
            &package,
            &mut findings,
        )
    });
    let artifacts = manifest_value
        .as_ref()
        .map(|manifest| {
            verify_manifest_artifacts(manifest, &options.bundle, &package, &mut findings)
        })
        .unwrap_or_default();
    let mut conformance_checks = Vec::new();
    if let Some(lock_file) = &lock
        && lock_file.status == "verified"
    {
        let lock_path = PathBuf::from(&lock_file.path);
        let registry_path = registry.as_ref().and_then(|file| {
            if file.status == "verified" {
                Some(PathBuf::from(&file.path))
            } else {
                None
            }
        });
        let registry_entry = registry_path
            .as_ref()
            .and_then(|path| first_registry_entry(path, &package, &mut findings));
        let package_report = verify_model_package(&ModelPackageVerifyOptions {
            lock: lock_path,
            registry: registry_path,
            registry_entry,
            output: options.output.clone(),
        })?;
        compare_package_report_to_manifest(
            &package_report,
            &artifacts,
            manifest_value.as_ref(),
            &package,
            &mut findings,
        );
        conformance_checks = package_report.conformance_checks;
        findings.extend(package_report.findings);
    }
    let result = if findings
        .iter()
        .any(|finding| finding.severity == "critical")
    {
        "fail"
    } else {
        "pass"
    };
    Ok(ModelPackageBundleVerificationReport {
        schema_version: MODEL_PACKAGE_BUNDLE_VERIFICATION_SCHEMA.to_string(),
        result: result.to_string(),
        bundle_path: options.bundle.to_string_lossy().to_string(),
        manifest,
        package,
        lock,
        registry,
        verification_report,
        verification_markdown,
        readme,
        artifacts,
        conformance_checks,
        findings,
    })
}

pub fn install_model_package_bundle(
    options: &ModelPackageBundleInstallOptions,
) -> Result<ModelPackageBundleInstallReport> {
    validate_install_options(options)?;
    let source_report = verify_model_package_bundle(&ModelPackageBundleVerifyOptions {
        bundle: options.bundle.clone(),
        output: options.output.clone(),
    })?;
    if source_report.result != "pass" {
        anyhow::bail!(
            "Source model package bundle verification failed for {}; install only verified bundles.",
            options.bundle.display()
        );
    }
    copy_bundle_directory(&options.bundle, &options.install_dir)?;
    let mut installed_report = verify_model_package_bundle(&ModelPackageBundleVerifyOptions {
        bundle: options.install_dir.clone(),
        output: options.output.clone(),
    })?;
    let mut installed_registry = None;
    if let Some(registry_output) = &options.registry_output {
        installed_registry = Some(write_install_registry(
            options,
            registry_output,
            &installed_report,
        )?);
        installed_report = verify_model_package_bundle(&ModelPackageBundleVerifyOptions {
            bundle: options.install_dir.clone(),
            output: options.output.clone(),
        })?;
    }
    let scenario_import =
        scenario_import_for_install(options, &installed_report, installed_registry.as_ref())?;
    let result = if installed_report
        .findings
        .iter()
        .any(|finding| finding.severity == "critical")
    {
        "fail"
    } else {
        "pass"
    };
    Ok(ModelPackageBundleInstallReport {
        schema_version: MODEL_PACKAGE_BUNDLE_INSTALL_SCHEMA.to_string(),
        result: result.to_string(),
        source_bundle: options.bundle.to_string_lossy().to_string(),
        install_dir: options.install_dir.to_string_lossy().to_string(),
        package: installed_report.package,
        manifest: installed_report.manifest,
        lock: installed_report.lock,
        registry: installed_report.registry,
        installed_registry,
        scenario_import,
        artifacts: installed_report.artifacts,
        conformance_checks: installed_report.conformance_checks,
        findings: installed_report.findings,
    })
}

pub fn import_model_package_bundle(
    options: &ModelPackageBundleImportOptions,
) -> Result<ModelPackageBundleImportReport> {
    import_model_package_bundle_with_cancel(options, &|| false)
}

pub fn import_model_package_bundle_with_cancel<C>(
    options: &ModelPackageBundleImportOptions,
    should_cancel: &C,
) -> Result<ModelPackageBundleImportReport>
where
    C: Fn() -> bool,
{
    let started = Instant::now();
    std::fs::create_dir_all(&options.output).with_context(|| {
        format!(
            "Failed to create bundle import output directory {}",
            options.output.display()
        )
    })?;
    let source_verification_path = options.output.join("bundle_verification.json");
    let install_report_path = options.output.join("bundle_install.json");
    let package_report_path = options.output.join("package_verification.json");
    let repair_output = options.output.join("repair_yaml");
    if let Some(report) = bundle_import_control_report(
        options,
        started,
        should_cancel,
        "source_bundle_verification",
        VerifiedBundlePackage::default(),
        source_verification_path.clone(),
        Vec::new(),
    ) {
        return Ok(report);
    }
    let source_report = verify_model_package_bundle(&ModelPackageBundleVerifyOptions {
        bundle: options.bundle.clone(),
        output: source_verification_path.clone(),
    })?;
    write_model_package_bundle_verification_report(&source_report, &source_verification_path)?;
    if source_report.result != "pass" {
        return Ok(import_report_from_parts(ImportReportParts {
            options,
            result: "fail",
            package: source_report.package,
            source_verification_path,
            install_report_path: None,
            install_report: None,
            package_report_path: None,
            package_report: None,
            repair_report_path: None,
            repair_report: None,
            started,
            source_findings: source_report.findings,
        }));
    }

    if let Some(report) = bundle_import_control_report(
        options,
        started,
        should_cancel,
        "bundle_install",
        source_report.package.clone(),
        source_verification_path.clone(),
        Vec::new(),
    ) {
        return Ok(report);
    }
    let install_report = install_model_package_bundle(&ModelPackageBundleInstallOptions {
        bundle: options.bundle.clone(),
        install_dir: options.install_dir.clone(),
        registry_output: options.registry_output.clone(),
        registry_entry: options.registry_entry.clone(),
        registry_artifact_id: options.registry_artifact_id.clone(),
        output: install_report_path.clone(),
    })?;
    write_model_package_bundle_install_report(&install_report, &install_report_path)?;
    if let Some(report) = bundle_import_control_report(
        options,
        started,
        should_cancel,
        "installed_package_verification",
        install_report.package.clone(),
        source_verification_path.clone(),
        install_report.findings.clone(),
    ) {
        return Ok(report);
    }
    let package_report = verify_installed_package(&install_report, &package_report_path)?;
    crate::model_package::write_model_package_verification_report(
        &package_report,
        &package_report_path,
    )?;
    if let Some(report) = bundle_import_control_report(
        options,
        started,
        should_cancel,
        "yaml_repair",
        install_report.package.clone(),
        source_verification_path.clone(),
        package_report.findings.clone(),
    ) {
        return Ok(report);
    }
    let repair_report = crate::repair_yaml::run_board_yaml_repair(BoardYamlRepairOptions {
        project: options.project.clone(),
        profile: options.profile.clone(),
        output: repair_output.clone(),
        finding: BoardYamlRepairFindingKind::BundleInstallPackageMetadata,
        dry_run: false,
        apply_report: None,
        proposal_ids: Vec::new(),
        bundle_install_report: Some(install_report_path.clone()),
    })?;
    let repair_report_path = repair_output.join("repair_report.json");
    let result = if install_report.result == "pass"
        && package_report.result == "pass"
        && repair_report.result == "pass"
    {
        "pass"
    } else {
        "fail"
    };
    Ok(import_report_from_parts(ImportReportParts {
        options,
        result,
        package: install_report.package.clone(),
        source_verification_path,
        install_report_path: Some(install_report_path),
        install_report: Some(install_report),
        package_report_path: Some(package_report_path),
        package_report: Some(package_report),
        repair_report_path: Some(repair_report_path),
        repair_report: Some(repair_report),
        started,
        source_findings: Vec::new(),
    }))
}

pub fn write_model_package_bundle_verification_report(
    report: &ModelPackageBundleVerificationReport,
    output: &Path,
) -> Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    let mut text = serde_json::to_string_pretty(report)?;
    text.push('\n');
    std::fs::write(output, text)
        .with_context(|| format!("Failed to write bundle report {}", output.display()))?;
    Ok(())
}

pub fn write_model_package_bundle_install_report(
    report: &ModelPackageBundleInstallReport,
    output: &Path,
) -> Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    let mut text = serde_json::to_string_pretty(report)?;
    text.push('\n');
    std::fs::write(output, text)
        .with_context(|| format!("Failed to write bundle install report {}", output.display()))?;
    Ok(())
}

pub fn write_model_package_bundle_import_report(
    report: &ModelPackageBundleImportReport,
    output_dir: &Path,
) -> Result<()> {
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory {}", output_dir.display()))?;
    let output = output_dir.join("model_package_bundle_import.json");
    let mut text = serde_json::to_string_pretty(report)?;
    text.push('\n');
    std::fs::write(&output, text)
        .with_context(|| format!("Failed to write bundle import report {}", output.display()))?;
    std::fs::write(
        output_dir.join("model_package_bundle_import.md"),
        model_package_bundle_import_markdown(report),
    )
    .with_context(|| {
        format!(
            "Failed to write bundle import Markdown report {}",
            output_dir.join("model_package_bundle_import.md").display()
        )
    })?;
    Ok(())
}

struct ImportReportParts<'a> {
    options: &'a ModelPackageBundleImportOptions,
    result: &'a str,
    package: VerifiedBundlePackage,
    source_verification_path: PathBuf,
    install_report_path: Option<PathBuf>,
    install_report: Option<ModelPackageBundleInstallReport>,
    package_report_path: Option<PathBuf>,
    package_report: Option<ModelPackageVerificationReport>,
    repair_report_path: Option<PathBuf>,
    repair_report: Option<BoardYamlRepairReport>,
    started: Instant,
    source_findings: Vec<ModelPackageFinding>,
}

fn import_report_from_parts(parts: ImportReportParts<'_>) -> ModelPackageBundleImportReport {
    let install = parts.install_report.as_ref();
    let package_report = parts.package_report.as_ref();
    let repair = parts.repair_report.as_ref();
    let mut findings = parts.source_findings;
    if let Some(install) = install {
        findings.extend(install.findings.clone());
    }
    if let Some(package_report) = package_report {
        findings.extend(package_report.findings.clone());
    }
    ModelPackageBundleImportReport {
        schema_version: MODEL_PACKAGE_BUNDLE_IMPORT_SCHEMA.to_string(),
        result: parts.result.to_string(),
        bundle_path: parts.options.bundle.to_string_lossy().into_owned(),
        project: parts.options.project.to_string_lossy().into_owned(),
        profile: parts.options.profile.clone(),
        install_dir: parts.options.install_dir.to_string_lossy().into_owned(),
        runtime_budget_ms: parts.options.max_runtime_ms,
        elapsed_ms: parts.started.elapsed().as_millis(),
        package: parts.package,
        source_bundle_verification_report: parts
            .source_verification_path
            .to_string_lossy()
            .into_owned(),
        bundle_install_report: parts
            .install_report_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        package_verification_report: parts
            .package_report_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        yaml_repair_report: parts
            .repair_report_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        repaired_project: repair.and_then(|report| report.repaired_project.clone()),
        repaired_validation_report: repair.and_then(|report| report.repaired_report.clone()),
        scenario_import: install.and_then(|report| report.scenario_import.clone()),
        summary: ModelPackageBundleImportSummary {
            bundle_artifacts: install.map_or(0, |report| report.artifacts.len()),
            conformance_checks: package_report.map_or(0, |report| report.conformance_checks.len()),
            package_findings: package_report.map_or(0, |report| report.findings.len()),
            repair_proposed: repair.map_or(0, |report| report.summary.proposed),
            repair_selected: repair.map_or(0, |report| report.summary.selected),
            repair_applied: repair.map_or(0, |report| report.summary.applied),
            repair_blocked: repair.map_or(0, |report| report.summary.blocked),
            repair_skipped: repair.map_or(0, |report| report.summary.skipped),
            repair_new_criticals: repair.map_or(0, |report| report.summary.new_criticals),
        },
        findings,
        repair_reason_codes: repair
            .map(|report| report.reason_codes.clone())
            .unwrap_or_default(),
        repair_messages: repair
            .map(|report| report.messages.clone())
            .unwrap_or_default(),
    }
}

fn bundle_import_control_report<C>(
    options: &ModelPackageBundleImportOptions,
    started: Instant,
    should_cancel: &C,
    stage: &str,
    package: VerifiedBundlePackage,
    source_verification_path: PathBuf,
    mut findings: Vec<ModelPackageFinding>,
) -> Option<ModelPackageBundleImportReport>
where
    C: Fn() -> bool,
{
    let elapsed = started.elapsed();
    let exceeded_budget = options
        .max_runtime_ms
        .map(|budget| elapsed >= Duration::from_millis(budget))
        .unwrap_or(false);
    let canceled = should_cancel();
    if !canceled && !exceeded_budget {
        return None;
    }
    let (id, message) = if canceled {
        (
            "MODEL_PACKAGE_BUNDLE_IMPORT_CANCELED",
            format!("Model package bundle import was canceled before stage {stage}."),
        )
    } else {
        (
            "MODEL_PACKAGE_BUNDLE_IMPORT_RUNTIME_BUDGET_EXCEEDED",
            format!(
                "Model package bundle import exceeded runtime budget {:?} before stage {stage} after {} ms.",
                options.max_runtime_ms,
                elapsed.as_millis()
            ),
        )
    };
    let expected_budget = options.max_runtime_ms.map(|value| value.to_string());
    let actual_elapsed = elapsed.as_millis().to_string();
    findings.push(finding(
        id,
        &message,
        package.name.as_deref(),
        None,
        Some(&options.bundle),
        expected_budget.as_deref(),
        Some(&actual_elapsed),
    ));
    Some(import_report_from_parts(ImportReportParts {
        options,
        result: "fail",
        package,
        source_verification_path,
        install_report_path: None,
        install_report: None,
        package_report_path: None,
        package_report: None,
        repair_report_path: None,
        repair_report: None,
        started,
        source_findings: findings,
    }))
}

fn verify_installed_package(
    install_report: &ModelPackageBundleInstallReport,
    output: &Path,
) -> Result<ModelPackageVerificationReport> {
    let import = install_report
        .scenario_import
        .as_ref()
        .context("Bundle install report did not produce scenario_import pins.")?;
    let lock = install_report
        .lock
        .as_ref()
        .filter(|lock| lock.status == "verified")
        .context("Bundle install report did not retain a verified lock.")?;
    let registry = install_report
        .installed_registry
        .as_ref()
        .or(install_report.registry.as_ref())
        .filter(|registry| registry.status == "verified")
        .context("Bundle install report did not retain a verified registry.")?;
    verify_model_package(&ModelPackageVerifyOptions {
        lock: PathBuf::from(&lock.path),
        registry: Some(PathBuf::from(&registry.path)),
        registry_entry: Some(import.model_package_registry_entry.clone()),
        output: output.to_path_buf(),
    })
}

fn model_package_bundle_import_markdown(report: &ModelPackageBundleImportReport) -> String {
    let mut text = String::new();
    text.push_str("# CircuitCI Model Package Bundle Import\n\n");
    text.push_str(&format!(
        "- Result: `{}`\n- Bundle: `{}`\n- Project: `{}`\n- Profile: `{}`\n- Install dir: `{}`\n- Runtime budget ms: `{}`\n- Elapsed ms: `{}`\n\n",
        report.result,
        report.bundle_path,
        report.project,
        report.profile,
        report.install_dir,
        report
            .runtime_budget_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string()),
        report.elapsed_ms
    ));
    text.push_str("## Reports\n\n");
    text.push_str(&format!(
        "- Bundle verification: `{}`\n",
        report.source_bundle_verification_report
    ));
    if let Some(path) = &report.bundle_install_report {
        text.push_str(&format!("- Bundle install: `{path}`\n"));
    }
    if let Some(path) = &report.package_verification_report {
        text.push_str(&format!("- Package verification: `{path}`\n"));
    }
    if let Some(path) = &report.yaml_repair_report {
        text.push_str(&format!("- YAML repair: `{path}`\n"));
    }
    text.push_str("\n## Summary\n\n");
    text.push_str(&format!(
        "- Artifacts: {}\n- Conformance checks: {}\n- Package findings: {}\n- Repair applied/blocked/skipped: {}/{}/{}\n- Repair new criticals: {}\n\n",
        report.summary.bundle_artifacts,
        report.summary.conformance_checks,
        report.summary.package_findings,
        report.summary.repair_applied,
        report.summary.repair_blocked,
        report.summary.repair_skipped,
        report.summary.repair_new_criticals
    ));
    if let Some(import) = &report.scenario_import {
        text.push_str("## Scenario Import\n\n");
        text.push_str(&format!(
            "- Registry: `{}` `{}`\n- Entry: `{}`\n- Lock: `{}` `{}`\n- Artifact: `{}`\n\n",
            import.model_package_registry_path,
            import.model_package_registry_sha256,
            import.model_package_registry_entry,
            import.model_package_lock_path,
            import.model_package_lock_sha256,
            import.model_package_artifact_id
        ));
    }
    if let Some(project) = &report.repaired_project {
        text.push_str(&format!("## Repaired Project\n\n- `{project}`\n"));
        if let Some(repaired_report) = &report.repaired_validation_report {
            text.push_str(&format!("- Validation report: `{repaired_report}`\n"));
        }
    }
    text
}

fn validate_install_options(options: &ModelPackageBundleInstallOptions) -> Result<()> {
    if !options.bundle.is_dir() {
        anyhow::bail!(
            "Model package bundle {} is missing.",
            options.bundle.display()
        );
    }
    if options.registry_output.is_none() {
        if options.registry_entry.is_some() {
            anyhow::bail!("--registry-entry requires --registry-output.");
        }
        if options.registry_artifact_id.is_some() {
            anyhow::bail!("--registry-artifact-id requires --registry-output.");
        }
    }
    if options.install_dir.exists()
        && std::fs::read_dir(&options.install_dir)
            .with_context(|| format!("Unable to read {}", options.install_dir.display()))?
            .next()
            .is_some()
    {
        anyhow::bail!(
            "Install directory {} already exists and is not empty.",
            options.install_dir.display()
        );
    }
    let source = std::fs::canonicalize(&options.bundle)
        .with_context(|| format!("Unable to resolve {}", options.bundle.display()))?;
    let install_parent = options
        .install_dir
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let resolved_parent = std::fs::canonicalize(install_parent)
        .with_context(|| format!("Unable to resolve {}", install_parent.display()))?;
    let install = resolved_parent.join(
        options
            .install_dir
            .file_name()
            .context("Install directory must have a final path component.")?,
    );
    if install == source || install.starts_with(&source) {
        anyhow::bail!("Install directory must not be the source bundle or inside it.");
    }
    Ok(())
}

fn copy_bundle_directory(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination).with_context(|| {
        format!(
            "Failed to create install directory {}",
            destination.display()
        )
    })?;
    for entry in std::fs::read_dir(source)
        .with_context(|| format!("Unable to read bundle directory {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_bundle_directory(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "Failed to copy bundle file {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn write_install_registry(
    options: &ModelPackageBundleInstallOptions,
    registry_output: &Path,
    report: &ModelPackageBundleVerificationReport,
) -> Result<VerifiedBundleFile> {
    let registry_parent = registry_output.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(registry_parent).with_context(|| {
        format!(
            "Failed to create model package registry directory {}",
            registry_parent.display()
        )
    })?;
    let lock = report
        .lock
        .as_ref()
        .filter(|lock| lock.status == "verified")
        .context("Installed bundle must contain a verified package.lock.json.")?;
    let lock_sha = lock
        .sha256_actual
        .as_deref()
        .context("Installed bundle lock SHA-256 is unavailable.")?;
    let registry_seed = report
        .registry
        .as_ref()
        .filter(|registry| registry.status == "verified")
        .and_then(|registry| {
            registry_entry_from_registry(Path::new(&registry.path))
                .ok()
                .flatten()
        });
    let entry_id = options
        .registry_entry
        .clone()
        .or_else(|| registry_seed.as_ref().map(|entry| entry.id.clone()))
        .or_else(|| options.registry_artifact_id.clone())
        .or_else(|| report.artifacts.first().map(|artifact| artifact.id.clone()))
        .context("Unable to choose registry entry id for installed bundle.")?;
    let artifact_id = options
        .registry_artifact_id
        .clone()
        .or_else(|| registry_seed.map(|entry| entry.artifact_id))
        .or_else(|| report.artifacts.first().map(|artifact| artifact.id.clone()))
        .context("Unable to choose registry artifact id for installed bundle.")?;
    let lock_path = relative_path(registry_parent, Path::new(&lock.path))?;
    let registry = serde_json::json!({
        "schema_version": crate::model_package::MODEL_PACKAGE_REGISTRY_SCHEMA,
        "packages": [{
            "id": entry_id,
            "package": {
                "name": report.package.name.clone(),
                "version": report.package.version.clone(),
            },
            "artifact_id": artifact_id,
            "lock_path": lock_path,
            "lock_sha256": lock_sha,
        }],
    });
    write_json_value(registry_output, &registry)?;
    Ok(verified_file(
        registry_output,
        None,
        "MODEL_PACKAGE_BUNDLE_INSTALL_REGISTRY_UNAVAILABLE",
        "MODEL_PACKAGE_BUNDLE_INSTALL_REGISTRY_HASH_MISMATCH",
        &mut Vec::new(),
    ))
}

fn scenario_import_for_install(
    options: &ModelPackageBundleInstallOptions,
    report: &ModelPackageBundleVerificationReport,
    installed_registry: Option<&VerifiedBundleFile>,
) -> Result<Option<BundleScenarioImport>> {
    let registry = installed_registry.or(report.registry.as_ref());
    let Some(registry) = registry.filter(|registry| registry.status == "verified") else {
        return Ok(None);
    };
    let entry = registry_entry_from_registry(Path::new(&registry.path))?
        .context("Installed bundle registry must contain a package entry.")?;
    let lock = report
        .lock
        .as_ref()
        .filter(|lock| lock.status == "verified")
        .context("Installed bundle must contain a verified package.lock.json.")?;
    let registry_path = relative_path(
        options.output.parent().unwrap_or_else(|| Path::new(".")),
        Path::new(&registry.path),
    )?;
    Ok(Some(BundleScenarioImport {
        model_package_registry_path: registry_path,
        model_package_registry_sha256: registry
            .sha256_actual
            .clone()
            .context("Installed registry SHA-256 is unavailable.")?,
        model_package_registry_entry: entry.id,
        model_package_lock_path: lock.path.clone(),
        model_package_lock_sha256: lock
            .sha256_actual
            .clone()
            .context("Installed lock SHA-256 is unavailable.")?,
        model_package_artifact_id: entry.artifact_id,
    }))
}

#[derive(Debug, Clone)]
struct RegistryEntrySummary {
    id: String,
    artifact_id: String,
}

fn registry_entry_from_registry(registry_path: &Path) -> Result<Option<RegistryEntrySummary>> {
    let text = std::fs::read_to_string(registry_path)
        .with_context(|| format!("Unable to read {}", registry_path.display()))?;
    let registry: Value =
        serde_json::from_str(&text).or_else(|_| serde_yaml_ng::from_str(&text))?;
    let entries = registry
        .get("packages")
        .or_else(|| registry.get("model_packages"))
        .or_else(|| registry.get("entries"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(entry) = entries.first() else {
        return Ok(None);
    };
    Ok(Some(RegistryEntrySummary {
        id: string_field(entry, &["id"])
            .or_else(|| string_field(entry, &["name"]))
            .context("Registry entry must declare id.")?,
        artifact_id: string_field(entry, &["artifact_id"])
            .or_else(|| string_field(entry, &["model_package_artifact_id"]))
            .context("Registry entry must declare artifact_id.")?,
    }))
}

fn read_manifest(path: &Path, findings: &mut Vec<ModelPackageFinding>) -> Option<Value> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return None,
    };
    match serde_json::from_str::<Value>(&text).or_else(|_| serde_yaml_ng::from_str::<Value>(&text))
    {
        Ok(value) => Some(value),
        Err(error) => {
            findings.push(finding(
                "MODEL_PACKAGE_BUNDLE_MANIFEST_INVALID",
                &format!("Bundle manifest is not valid JSON or YAML: {error}"),
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

fn validate_manifest_header(
    manifest: &Value,
    manifest_path: &Path,
    package: &VerifiedBundlePackage,
    findings: &mut Vec<ModelPackageFinding>,
) {
    compare_field(
        "MODEL_PACKAGE_BUNDLE_SCHEMA_MISMATCH",
        "Bundle manifest schema_version is not supported.",
        Some(MODEL_PACKAGE_BUNDLE_SCHEMA),
        string_field(manifest, &["schema_version"]).as_deref(),
        package,
        Some(manifest_path),
        findings,
    );
    if package.name.is_none() {
        findings.push(finding(
            "MODEL_PACKAGE_BUNDLE_PACKAGE_MISSING",
            "Bundle manifest must declare package.name.",
            None,
            None,
            Some(manifest_path),
            None,
            None,
        ));
    }
    if package.version.is_none() {
        findings.push(finding(
            "MODEL_PACKAGE_BUNDLE_PACKAGE_MISSING",
            "Bundle manifest must declare package.version.",
            package.name.as_deref(),
            None,
            Some(manifest_path),
            None,
            None,
        ));
    }
}

fn verify_manifest_file(
    manifest: &Value,
    bundle: &Path,
    spec: ManifestFileSpec<'_>,
    package: &VerifiedBundlePackage,
    findings: &mut Vec<ModelPackageFinding>,
) -> Option<VerifiedBundleFile> {
    let relative = required_manifest_string(
        manifest,
        spec.path_field,
        spec.missing_id,
        package,
        findings,
    )?;
    let expected =
        required_manifest_string(manifest, spec.sha_field, spec.missing_id, package, findings);
    let path = bundle_child_path(bundle, &relative, package, findings)?;
    Some(verified_file(
        &path,
        expected.as_deref(),
        spec.missing_id,
        spec.mismatch_id,
        findings,
    ))
}

struct ManifestFileSpec<'a> {
    path_field: &'a [&'a str],
    sha_field: &'a [&'a str],
    missing_id: &'a str,
    mismatch_id: &'a str,
}

fn verify_manifest_file_presence(
    manifest: &Value,
    bundle: &Path,
    path_field: &[&str],
    missing_id: &str,
    package: &VerifiedBundlePackage,
    findings: &mut Vec<ModelPackageFinding>,
) -> Option<VerifiedBundleFile> {
    let relative = required_manifest_string(manifest, path_field, missing_id, package, findings)?;
    let path = bundle_child_path(bundle, &relative, package, findings)?;
    Some(verified_file(&path, None, missing_id, missing_id, findings))
}

fn verify_optional_registry(
    manifest: &Value,
    bundle: &Path,
    package: &VerifiedBundlePackage,
    findings: &mut Vec<ModelPackageFinding>,
) -> Option<VerifiedBundleFile> {
    let registry_path = manifest.get("registry_path").and_then(Value::as_str);
    let registry_sha = manifest.get("registry_sha256").and_then(Value::as_str);
    match (registry_path, registry_sha) {
        (Some(path), sha) if !path.trim().is_empty() => {
            let absolute = bundle_child_path(bundle, path, package, findings)?;
            Some(verified_file(
                &absolute,
                sha,
                "MODEL_PACKAGE_BUNDLE_REGISTRY_UNAVAILABLE",
                "MODEL_PACKAGE_BUNDLE_REGISTRY_HASH_MISMATCH",
                findings,
            ))
        }
        (None, Some(_)) => {
            findings.push(finding(
                "MODEL_PACKAGE_BUNDLE_REGISTRY_MISMATCH",
                "Bundle manifest declares registry_sha256 without registry_path.",
                package.name.as_deref(),
                None,
                Some(&bundle.join("model_package_bundle_manifest.json")),
                None,
                registry_sha,
            ));
            None
        }
        _ => None,
    }
}

fn verify_manifest_artifacts(
    manifest: &Value,
    bundle: &Path,
    package: &VerifiedBundlePackage,
    findings: &mut Vec<ModelPackageFinding>,
) -> Vec<VerifiedBundleArtifact> {
    let Some(artifacts) = manifest.get("artifacts").and_then(Value::as_array) else {
        findings.push(finding(
            "MODEL_PACKAGE_BUNDLE_ARTIFACTS_MISSING",
            "Bundle manifest must declare artifacts[].",
            package.name.as_deref(),
            None,
            Some(&bundle.join("model_package_bundle_manifest.json")),
            None,
            None,
        ));
        return Vec::new();
    };
    artifacts
        .iter()
        .map(|artifact| verify_manifest_artifact(artifact, bundle, package, findings))
        .collect()
}

fn verify_manifest_artifact(
    artifact: &Value,
    bundle: &Path,
    package: &VerifiedBundlePackage,
    findings: &mut Vec<ModelPackageFinding>,
) -> VerifiedBundleArtifact {
    let id = string_field(artifact, &["id"]).unwrap_or_else(|| "<missing>".to_string());
    let path = string_field(artifact, &["artifact_path"]);
    let declared_sha = string_field(artifact, &["artifact_sha256"]);
    let artifact_format = string_field(artifact, &["artifact_format"]);
    let compiler = string_field(artifact, &["compiler"]);
    for (field, value) in [
        (
            "id",
            Some(id.as_str()).filter(|value| *value != "<missing>"),
        ),
        ("artifact_path", path.as_deref()),
        ("artifact_sha256", declared_sha.as_deref()),
        ("artifact_format", artifact_format.as_deref()),
    ] {
        if value.is_none() {
            findings.push(finding(
                "MODEL_PACKAGE_BUNDLE_ARTIFACT_FIELD_MISSING",
                &format!("Bundle artifact {id} must declare {field}."),
                package.name.as_deref(),
                Some(&id),
                Some(&bundle.join("model_package_bundle_manifest.json")),
                None,
                None,
            ));
        }
    }
    let absolute_path = path
        .as_deref()
        .and_then(|path| bundle_child_path(bundle, path, package, findings));
    let actual_sha = absolute_path
        .as_ref()
        .and_then(|path| file_sha256_hex(path).ok());
    if let (Some(expected), Some(actual)) = (declared_sha.as_deref(), actual_sha.as_deref())
        && !expected.eq_ignore_ascii_case(actual)
    {
        findings.push(finding(
            "MODEL_PACKAGE_BUNDLE_ARTIFACT_HASH_MISMATCH",
            "Bundle artifact SHA-256 does not match the manifest.",
            package.name.as_deref(),
            Some(&id),
            absolute_path.as_deref(),
            Some(expected),
            Some(actual),
        ));
    }
    if actual_sha.is_none() {
        findings.push(finding(
            "MODEL_PACKAGE_BUNDLE_ARTIFACT_UNAVAILABLE",
            "Bundle artifact is missing or unreadable.",
            package.name.as_deref(),
            Some(&id),
            absolute_path.as_deref(),
            declared_sha.as_deref(),
            None,
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
    VerifiedBundleArtifact {
        id,
        path,
        artifact_format,
        compiler,
        sha256_declared: declared_sha,
        sha256_actual: actual_sha,
        status: status.to_string(),
    }
}

fn compare_package_report_to_manifest(
    report: &crate::model_package::ModelPackageVerificationReport,
    manifest_artifacts: &[VerifiedBundleArtifact],
    manifest: Option<&Value>,
    package: &VerifiedBundlePackage,
    findings: &mut Vec<ModelPackageFinding>,
) {
    compare_field(
        "MODEL_PACKAGE_BUNDLE_PACKAGE_MISMATCH",
        "Bundled package verification name does not match the manifest.",
        package.name.as_deref(),
        report.lock.package_name.as_deref(),
        package,
        None,
        findings,
    );
    compare_field(
        "MODEL_PACKAGE_BUNDLE_PACKAGE_MISMATCH",
        "Bundled package verification version does not match the manifest.",
        package.version.as_deref(),
        report.lock.package_version.as_deref(),
        package,
        None,
        findings,
    );
    let mut manifest_by_id = BTreeMap::new();
    for artifact in manifest_artifacts {
        manifest_by_id.insert(artifact.id.as_str(), artifact);
    }
    for artifact in &report.artifacts {
        let Some(manifest_artifact) = manifest_by_id.get(artifact.id.as_str()) else {
            findings.push(finding(
                "MODEL_PACKAGE_BUNDLE_ARTIFACT_MISMATCH",
                "Bundled lock artifact is missing from the bundle manifest.",
                package.name.as_deref(),
                Some(&artifact.id),
                artifact.path.as_deref().map(Path::new),
                None,
                artifact.sha256_actual.as_deref(),
            ));
            continue;
        };
        compare_field(
            "MODEL_PACKAGE_BUNDLE_ARTIFACT_MISMATCH",
            "Bundle manifest artifact SHA-256 does not match bundled package verification.",
            manifest_artifact.sha256_declared.as_deref(),
            artifact.sha256_declared.as_deref(),
            package,
            artifact.path.as_deref().map(Path::new),
            findings,
        );
    }
    if let Some(manifest) = manifest {
        let manifest_checks = manifest
            .get("conformance_checks")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        if manifest_checks != report.conformance_checks.len() {
            findings.push(finding(
                "MODEL_PACKAGE_BUNDLE_CONFORMANCE_MISMATCH",
                "Bundle manifest conformance check count does not match package verification.",
                package.name.as_deref(),
                None,
                None,
                Some(&manifest_checks.to_string()),
                Some(&report.conformance_checks.len().to_string()),
            ));
        }
    }
}

fn first_registry_entry(
    registry_path: &Path,
    package: &VerifiedBundlePackage,
    findings: &mut Vec<ModelPackageFinding>,
) -> Option<String> {
    let text = match std::fs::read_to_string(registry_path) {
        Ok(text) => text,
        Err(error) => {
            findings.push(finding(
                "MODEL_PACKAGE_BUNDLE_REGISTRY_UNAVAILABLE",
                &format!("Unable to read bundled registry: {error}"),
                package.name.as_deref(),
                None,
                Some(registry_path),
                None,
                None,
            ));
            return None;
        }
    };
    let registry = match serde_json::from_str::<Value>(&text)
        .or_else(|_| serde_yaml_ng::from_str::<Value>(&text))
    {
        Ok(value) => value,
        Err(error) => {
            findings.push(finding(
                "MODEL_PACKAGE_BUNDLE_REGISTRY_INVALID",
                &format!("Bundled registry is not valid JSON or YAML: {error}"),
                package.name.as_deref(),
                None,
                Some(registry_path),
                None,
                None,
            ));
            return None;
        }
    };
    let entries = registry
        .get("packages")
        .or_else(|| registry.get("model_packages"))
        .or_else(|| registry.get("entries"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if entries.len() != 1 {
        findings.push(finding(
            "MODEL_PACKAGE_BUNDLE_REGISTRY_ENTRY_MISMATCH",
            "Bundled registry must contain exactly one package entry.",
            package.name.as_deref(),
            None,
            Some(registry_path),
            Some("1"),
            Some(&entries.len().to_string()),
        ));
        return None;
    }
    string_field(&entries[0], &["id"]).or_else(|| string_field(&entries[0], &["name"]))
}

fn verified_file(
    path: &Path,
    expected_sha: Option<&str>,
    unavailable_id: &str,
    mismatch_id: &str,
    findings: &mut Vec<ModelPackageFinding>,
) -> VerifiedBundleFile {
    let actual_sha = match file_sha256_hex(path) {
        Ok(sha) => Some(sha),
        Err(error) => {
            findings.push(finding(
                unavailable_id,
                &error.to_string(),
                None,
                None,
                Some(path),
                expected_sha,
                None,
            ));
            None
        }
    };
    if let (Some(expected), Some(actual)) = (expected_sha, actual_sha.as_deref())
        && !expected.eq_ignore_ascii_case(actual)
    {
        findings.push(finding(
            mismatch_id,
            "Bundle file SHA-256 does not match the manifest.",
            None,
            None,
            Some(path),
            Some(expected),
            Some(actual),
        ));
    }
    let status = if actual_sha.is_some()
        && expected_sha.is_none_or(|expected| {
            actual_sha
                .as_deref()
                .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
        }) {
        "verified"
    } else {
        "failed"
    };
    VerifiedBundleFile {
        path: path.to_string_lossy().to_string(),
        sha256_declared: expected_sha.map(ToOwned::to_owned),
        sha256_actual: actual_sha,
        status: status.to_string(),
    }
}

fn required_manifest_string(
    value: &Value,
    path: &[&str],
    id: &str,
    package: &VerifiedBundlePackage,
    findings: &mut Vec<ModelPackageFinding>,
) -> Option<String> {
    let field = string_field(value, path);
    if field.as_deref().is_none_or(str::is_empty) {
        findings.push(finding(
            id,
            &format!("Bundle manifest must declare {}.", path.join(".")),
            package.name.as_deref(),
            None,
            None,
            None,
            None,
        ));
        None
    } else {
        field
    }
}

fn bundle_child_path(
    bundle: &Path,
    relative: &str,
    package: &VerifiedBundlePackage,
    findings: &mut Vec<ModelPackageFinding>,
) -> Option<PathBuf> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        findings.push(finding(
            "MODEL_PACKAGE_BUNDLE_PATH_ESCAPE",
            "Bundle manifest paths must be relative and stay inside the bundle directory.",
            package.name.as_deref(),
            None,
            Some(relative_path),
            None,
            None,
        ));
        None
    } else {
        Some(bundle.join(relative_path))
    }
}

fn compare_field(
    id: &str,
    message: &str,
    expected: Option<&str>,
    actual: Option<&str>,
    package: &VerifiedBundlePackage,
    path: Option<&Path>,
    findings: &mut Vec<ModelPackageFinding>,
) {
    match (expected, actual) {
        (Some(expected), Some(actual)) if expected == actual => {}
        (Some(expected), Some(actual)) => findings.push(finding(
            id,
            message,
            package.name.as_deref(),
            None,
            path,
            Some(expected),
            Some(actual),
        )),
        (Some(expected), None) => findings.push(finding(
            id,
            message,
            package.name.as_deref(),
            None,
            path,
            Some(expected),
            None,
        )),
        (None, _) => {}
    }
}

fn string_field(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToOwned::to_owned)
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

fn file_sha256_hex(path: &Path) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Unable to read {}", path.display()))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn relative_path(base_dir: &Path, target: &Path) -> Result<String> {
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
