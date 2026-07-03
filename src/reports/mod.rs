use crate::board_ir::Endpoint;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub scenario: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<EndpointPair>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub measured: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub limit: BTreeMap<String, serde_json::Value>,
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointPair {
    pub driver: Endpoint,
    pub victim: Endpoint,
}

#[derive(Debug, Clone, Serialize)]
pub struct Limitation {
    pub id: String,
    pub scope: String,
    pub confidence: String,
    pub blocking: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub schema_version: String,
    pub project: String,
    pub profile: String,
    pub result: String,
    pub summary: Summary,
    pub failures: Vec<Finding>,
    pub warnings: Vec<Finding>,
    pub infos: Vec<Finding>,
    pub waveforms: Vec<String>,
    pub artifacts: Vec<String>,
    pub model_file_provenance: Vec<ModelFileProvenance>,
    pub model_package_conformance_checks: Vec<ModelPackageConformanceCheck>,
    pub model_package_bundle_verifications: Vec<ModelPackageBundleVerificationSummary>,
    pub model_package_bundle_installs: Vec<ModelPackageBundleInstallSummary>,
    pub limitations: Vec<Limitation>,
    pub suggested_next_actions: Vec<String>,
    pub reproduction: Reproduction,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelFileProvenance {
    pub scenario: String,
    pub analysis: String,
    pub backend: String,
    pub manifest: String,
    pub model_file: String,
    pub artifact_format: String,
    pub source_path: String,
    pub source_sha256_declared: String,
    pub source_sha256_actual: String,
    pub artifact_sha256_declared: String,
    pub artifact_sha256_actual: String,
    pub compiler: String,
    pub compiler_version: String,
    pub compiler_command: String,
    pub model_package_name: Option<String>,
    pub model_package_version: Option<String>,
    pub model_package_artifact_id: Option<String>,
    pub model_package_lock_path: Option<String>,
    pub model_package_lock_sha256: Option<String>,
    pub model_package_registry_path: Option<String>,
    pub model_package_registry_sha256: Option<String>,
    pub model_package_registry_entry: Option<String>,
    pub compiler_available_on_path: Option<bool>,
    pub build_env_enabled: Option<bool>,
    pub rebuild_mode: String,
    pub produced_by_circuitci: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelPackageConformanceCheck {
    pub report: String,
    pub report_artifact_id: String,
    pub target_artifact_id: String,
    pub target_artifact_sha256: String,
    pub check_name: String,
    pub analysis: String,
    pub solver: String,
    pub result: String,
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelPackageBundleVerificationSummary {
    pub report: String,
    pub result: String,
    pub bundle_path: String,
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub manifest_path: String,
    pub manifest_sha256_actual: Option<String>,
    pub lock_path: Option<String>,
    pub lock_sha256_actual: Option<String>,
    pub registry_path: Option<String>,
    pub registry_sha256_actual: Option<String>,
    pub artifact_count: usize,
    pub conformance_check_count: usize,
    pub finding_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelPackageBundleInstallSummary {
    pub report: String,
    pub result: String,
    pub source_bundle: String,
    pub install_dir: String,
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub installed_registry_path: Option<String>,
    pub installed_registry_sha256_actual: Option<String>,
    pub model_package_registry_path: Option<String>,
    pub model_package_registry_sha256: Option<String>,
    pub model_package_registry_entry: Option<String>,
    pub model_package_lock_path: Option<String>,
    pub model_package_lock_sha256: Option<String>,
    pub model_package_artifact_id: Option<String>,
    pub artifact_count: usize,
    pub conformance_check_count: usize,
    pub finding_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuiteReport {
    pub schema_version: String,
    pub suite: String,
    pub validation_profile: String,
    pub result: String,
    pub summary: SuiteSummary,
    pub cases: Vec<SuiteCaseReport>,
    pub repairs: Vec<SuiteRepairReport>,
    pub reproduction: Reproduction,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuiteSummary {
    pub cases: usize,
    pub passed: usize,
    pub failed: usize,
    pub repairs: usize,
    pub repairs_passed: usize,
    pub repairs_failed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuiteCaseReport {
    pub id: String,
    pub project: String,
    pub expect: String,
    pub actual: String,
    pub result: String,
    pub required_findings: Vec<SuiteFindingExpectation>,
    pub matched_findings: Vec<SuiteFindingExpectation>,
    pub required_artifacts: Vec<String>,
    pub matched_artifacts: Vec<String>,
    pub required_waveforms: Vec<String>,
    pub matched_waveforms: Vec<String>,
    pub blocking_limitations: Vec<String>,
    pub report: String,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SuiteFindingExpectation {
    pub id: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuiteRepairReport {
    pub id: String,
    pub detects_case: String,
    pub fixed_case: String,
    pub fixes_findings: Vec<String>,
    pub detect_project: String,
    pub fixed_project: String,
    pub detect_report: String,
    pub fixed_report: String,
    pub matched_findings: Vec<SuiteFindingEvidence>,
    pub suggested_fixes: Vec<String>,
    pub result: String,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuiteFindingEvidence {
    pub id: String,
    pub severity: String,
    pub scenario: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net: Option<String>,
    pub message: String,
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub critical: usize,
    pub warning: usize,
    pub info: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Reproduction {
    pub command: String,
}

impl Finding {
    pub fn critical(id: &str, scenario: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Severity::Critical, scenario, message)
    }

    pub fn warning(id: &str, scenario: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Severity::Warning, scenario, message)
    }

    pub fn info(id: &str, scenario: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, Severity::Info, scenario, message)
    }

    fn new(
        id: &str,
        severity: Severity,
        scenario: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: id.to_string(),
            severity,
            scenario: scenario.into(),
            message: message.into(),
            component: None,
            net: None,
            endpoints: None,
            measured: BTreeMap::new(),
            limit: BTreeMap::new(),
            suggested_fixes: Vec::new(),
        }
    }
}

impl ValidationReport {
    pub fn from_parts(
        project: String,
        profile: String,
        findings: Vec<Finding>,
        limitations: Vec<Limitation>,
        artifacts: Vec<String>,
        waveforms: Vec<String>,
        command: String,
    ) -> Self {
        let mut failures = Vec::new();
        let mut warnings = Vec::new();
        let mut infos = Vec::new();
        for finding in findings {
            match finding.severity {
                Severity::Critical => failures.push(finding),
                Severity::Warning => warnings.push(finding),
                Severity::Info => infos.push(finding),
            }
        }
        let summary = Summary {
            critical: failures.len(),
            warning: warnings.len(),
            info: infos.len(),
        };
        let suggested_next_actions = failures
            .iter()
            .flat_map(|finding| finding.suggested_fixes.iter().cloned())
            .collect();
        let result = if summary.critical > 0 { "fail" } else { "pass" }.to_string();
        let model_file_provenance = collect_model_file_provenance(&artifacts);
        let model_package_conformance_checks = collect_model_package_conformance_checks(&artifacts);
        let model_package_bundle_verifications =
            collect_model_package_bundle_verifications(&artifacts);
        let model_package_bundle_installs = collect_model_package_bundle_installs(&artifacts);
        Self {
            schema_version: "0.1.0".to_string(),
            project,
            profile,
            result,
            summary,
            failures,
            warnings,
            infos,
            waveforms,
            artifacts,
            model_file_provenance,
            model_package_conformance_checks,
            model_package_bundle_verifications,
            model_package_bundle_installs,
            limitations,
            suggested_next_actions,
            reproduction: Reproduction { command },
        }
    }
}

pub fn write_reports(report: &ValidationReport, output: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(output)?;
    let json = serde_json::to_string_pretty(report)?;
    fs::write(output.join("report.json"), json)?;
    fs::write(output.join("report.md"), markdown_report(report))?;
    Ok(())
}

pub fn write_suite_reports(report: &SuiteReport, output: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(output)?;
    let json = serde_json::to_string_pretty(report)?;
    fs::write(output.join("report.json"), json)?;
    fs::write(output.join("report.md"), suite_markdown_report(report))?;
    Ok(())
}

fn markdown_report(report: &ValidationReport) -> String {
    let mut text = String::new();
    text.push_str(&format!("# CircuitCI Report: {}\n\n", report.project));
    text.push_str("## Executive Summary\n\n");
    text.push_str(&format!(
        "- Result: `{}`\n- Critical: {}\n- Warning: {}\n- Info: {}\n\n",
        report.result, report.summary.critical, report.summary.warning, report.summary.info
    ));
    text.push_str("## Critical Failures\n\n");
    push_findings(&mut text, &report.failures);
    text.push_str("## Warnings\n\n");
    push_findings(&mut text, &report.warnings);
    text.push_str("## Unmodeled Or Low-Confidence Areas\n\n");
    if report.limitations.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for limitation in &report.limitations {
            text.push_str(&format!(
                "- `{}` [{}]: {}\n",
                limitation.id, limitation.confidence, limitation.message
            ));
        }
        text.push('\n');
    }
    text.push_str("## Model File Provenance\n\n");
    if report.model_file_provenance.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for provenance in &report.model_file_provenance {
            text.push_str(&format!(
                "- `{}` in scenario `{}` via `{}`: `{}` ({}, produced_by_circuitci: {})\n",
                provenance.model_file,
                provenance.scenario,
                provenance.backend,
                provenance.rebuild_mode,
                provenance.compiler,
                provenance
                    .produced_by_circuitci
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ));
            text.push_str(&format!(
                "  - Source: `{}` `{}`\n",
                provenance.source_path, provenance.source_sha256_actual
            ));
            text.push_str(&format!(
                "  - Artifact: `{}` `{}`\n",
                provenance.artifact_format, provenance.artifact_sha256_actual
            ));
            if let Some(package_name) = &provenance.model_package_name {
                text.push_str(&format!(
                    "  - Package: `{}` `{}` artifact `{}` lock `{}`\n",
                    package_name,
                    provenance.model_package_version.as_deref().unwrap_or(""),
                    provenance
                        .model_package_artifact_id
                        .as_deref()
                        .unwrap_or(""),
                    provenance.model_package_lock_path.as_deref().unwrap_or("")
                ));
                if let Some(registry_path) = &provenance.model_package_registry_path {
                    text.push_str(&format!(
                        "  - Registry: `{}` entry `{}`\n",
                        registry_path,
                        provenance
                            .model_package_registry_entry
                            .as_deref()
                            .unwrap_or("")
                    ));
                }
            }
        }
        text.push('\n');
    }
    text.push_str("## Model Package Conformance\n\n");
    if report.model_package_conformance_checks.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for check in &report.model_package_conformance_checks {
            text.push_str(&format!(
                "- `{}` `{}` via `{}`: `{}` target `{}` `{}`\n",
                check.check_name,
                check.analysis,
                check.solver,
                check.result,
                check.target_artifact_id,
                check.target_artifact_sha256
            ));
            text.push_str(&format!(
                "  - Report: `{}` artifact `{}`\n",
                check.report, check.report_artifact_id
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
    text.push_str("## Model Package Bundle Verification\n\n");
    if report.model_package_bundle_verifications.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for bundle in &report.model_package_bundle_verifications {
            text.push_str(&format!(
                "- `{}` `{}`: `{}` artifacts={} conformance_checks={} findings={}\n",
                bundle
                    .package_name
                    .as_deref()
                    .unwrap_or("<unknown-package>"),
                bundle.package_version.as_deref().unwrap_or(""),
                bundle.result,
                bundle.artifact_count,
                bundle.conformance_check_count,
                bundle.finding_count
            ));
            text.push_str(&format!(
                "  - Bundle: `{}` report `{}` manifest `{}` `{}`\n",
                bundle.bundle_path,
                bundle.report,
                bundle.manifest_path,
                bundle.manifest_sha256_actual.as_deref().unwrap_or("")
            ));
            if let Some(lock_path) = &bundle.lock_path {
                text.push_str(&format!(
                    "  - Lock: `{}` `{}`\n",
                    lock_path,
                    bundle.lock_sha256_actual.as_deref().unwrap_or("")
                ));
            }
            if let Some(registry_path) = &bundle.registry_path {
                text.push_str(&format!(
                    "  - Registry: `{}` `{}`\n",
                    registry_path,
                    bundle.registry_sha256_actual.as_deref().unwrap_or("")
                ));
            }
        }
        text.push('\n');
    }
    text.push_str("## Model Package Bundle Install\n\n");
    if report.model_package_bundle_installs.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for install in &report.model_package_bundle_installs {
            text.push_str(&format!(
                "- `{}` `{}`: `{}` artifacts={} conformance_checks={} findings={}\n",
                install
                    .package_name
                    .as_deref()
                    .unwrap_or("<unknown-package>"),
                install.package_version.as_deref().unwrap_or(""),
                install.result,
                install.artifact_count,
                install.conformance_check_count,
                install.finding_count
            ));
            text.push_str(&format!(
                "  - Source: `{}` installed to `{}` report `{}`\n",
                install.source_bundle, install.install_dir, install.report
            ));
            if let Some(registry_path) = &install.installed_registry_path {
                text.push_str(&format!(
                    "  - Installed registry: `{}` `{}`\n",
                    registry_path,
                    install
                        .installed_registry_sha256_actual
                        .as_deref()
                        .unwrap_or("")
                ));
            }
            if let Some(entry) = &install.model_package_registry_entry {
                text.push_str(&format!(
                    "  - Scenario import: registry `{}` sha `{}` entry `{}` lock `{}` sha `{}` artifact `{}`\n",
                    install.model_package_registry_path.as_deref().unwrap_or(""),
                    install.model_package_registry_sha256.as_deref().unwrap_or(""),
                    entry,
                    install.model_package_lock_path.as_deref().unwrap_or(""),
                    install.model_package_lock_sha256.as_deref().unwrap_or(""),
                    install.model_package_artifact_id.as_deref().unwrap_or("")
                ));
            }
        }
        text.push('\n');
    }
    text.push_str("## Reproduction\n\n");
    text.push_str(&format!("```bash\n{}\n```\n", report.reproduction.command));
    text
}

fn suite_markdown_report(report: &SuiteReport) -> String {
    let mut text = String::new();
    text.push_str(&format!("# CircuitCI Suite Report: {}\n\n", report.suite));
    text.push_str("## Executive Summary\n\n");
    text.push_str(&format!(
        "- Result: `{}`\n- Cases: {}\n- Passed: {}\n- Failed: {}\n- Repairs: {}\n- Repairs passed: {}\n- Repairs failed: {}\n\n",
        report.result,
        report.summary.cases,
        report.summary.passed,
        report.summary.failed,
        report.summary.repairs,
        report.summary.repairs_passed,
        report.summary.repairs_failed
    ));
    text.push_str("## Cases\n\n");
    for case in &report.cases {
        text.push_str(&format!(
            "- `{}`: `{}` (expected `{}`, actual `{}`)\n",
            case.id, case.result, case.expect, case.actual
        ));
        for message in &case.messages {
            text.push_str(&format!("  - {message}\n"));
        }
    }
    text.push_str("\n## Repairs\n\n");
    if report.repairs.is_empty() {
        text.push_str("None.\n");
    } else {
        for repair in &report.repairs {
            text.push_str(&format!(
                "- `{}`: `{}` ({} -> {})\n",
                repair.id, repair.result, repair.detects_case, repair.fixed_case
            ));
            for message in &repair.messages {
                text.push_str(&format!("  - {message}\n"));
            }
        }
    }
    text.push_str("\n## Reproduction\n\n");
    text.push_str(&format!("```bash\n{}\n```\n", report.reproduction.command));
    text
}

fn push_findings(text: &mut String, findings: &[Finding]) {
    if findings.is_empty() {
        text.push_str("None.\n\n");
        return;
    }
    for finding in findings {
        text.push_str(&format!("- `{}`: {}\n", finding.id, finding.message));
        for fix in &finding.suggested_fixes {
            text.push_str(&format!("  - Fix: {fix}\n"));
        }
    }
    text.push('\n');
}

fn collect_model_file_provenance(artifacts: &[String]) -> Vec<ModelFileProvenance> {
    let mut records = BTreeSet::new();
    for artifact in artifacts {
        if !artifact.ends_with("solver_manifest.json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let scenario = string_at(&manifest, &["scenario"]);
        let analysis = string_at(&manifest, &["analysis", "kind"]);
        let backend = string_at(&manifest, &["backend", "selected"]);
        let Some(entries) = manifest
            .get("inputs")
            .and_then(|inputs| inputs.get("model_file_provenance"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for entry in entries {
            let record = ModelFileProvenance {
                scenario: scenario.clone(),
                analysis: analysis.clone(),
                backend: backend.clone(),
                manifest: artifact.clone(),
                model_file: string_at(entry, &["model_file"]),
                artifact_format: string_at(entry, &["artifact_format"]),
                source_path: string_at(entry, &["source_path"]),
                source_sha256_declared: string_at(entry, &["source_sha256_declared"]),
                source_sha256_actual: string_at(entry, &["source_sha256_actual"]),
                artifact_sha256_declared: string_at(entry, &["artifact_sha256_declared"]),
                artifact_sha256_actual: string_at(entry, &["artifact_sha256_actual"]),
                compiler: string_at(entry, &["compiler"]),
                compiler_version: string_at(entry, &["compiler_version"]),
                compiler_command: string_at(entry, &["compiler_command"]),
                model_package_name: optional_string_at(entry, &["model_package_name"]),
                model_package_version: optional_string_at(entry, &["model_package_version"]),
                model_package_artifact_id: optional_string_at(
                    entry,
                    &["model_package_artifact_id"],
                ),
                model_package_lock_path: optional_string_at(entry, &["model_package_lock_path"]),
                model_package_lock_sha256: optional_string_at(
                    entry,
                    &["model_package_lock_sha256"],
                ),
                model_package_registry_path: optional_string_at(
                    entry,
                    &["model_package_registry_path"],
                ),
                model_package_registry_sha256: optional_string_at(
                    entry,
                    &["model_package_registry_sha256"],
                ),
                model_package_registry_entry: optional_string_at(
                    entry,
                    &["model_package_registry_entry"],
                ),
                compiler_available_on_path: bool_at(entry, &["compiler_available_on_path"]),
                build_env_enabled: bool_at(entry, &["build_env_enabled"]),
                rebuild_mode: string_at(entry, &["rebuild_mode"]),
                produced_by_circuitci: bool_at(entry, &["produced_by_circuitci"]),
            };
            if !record.model_file.is_empty() {
                records.insert(record);
            }
        }
    }
    records.into_iter().collect()
}

fn collect_model_package_conformance_checks(
    artifacts: &[String],
) -> Vec<ModelPackageConformanceCheck> {
    let mut records = BTreeSet::new();
    for artifact in artifacts {
        if !artifact.ends_with(".json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        let Ok(report) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if string_at(&report, &["schema_version"]) != "circuitci.model_package_verification.v1" {
            continue;
        }
        let Some(checks) = report.get("conformance_checks").and_then(Value::as_array) else {
            continue;
        };
        for check in checks {
            let record = ModelPackageConformanceCheck {
                report: artifact.clone(),
                report_artifact_id: string_at(check, &["report_artifact_id"]),
                target_artifact_id: string_at(check, &["target_artifact_id"]),
                target_artifact_sha256: string_at(check, &["target_artifact_sha256"]),
                check_name: string_at(check, &["check_name"]),
                analysis: string_at(check, &["analysis"]),
                solver: string_at(check, &["solver"]),
                result: string_at(check, &["result"]),
                artifacts: string_array_at(check, &["artifacts"]),
            };
            if !record.report_artifact_id.is_empty() || !record.check_name.is_empty() {
                records.insert(record);
            }
        }
    }
    records.into_iter().collect()
}

fn collect_model_package_bundle_verifications(
    artifacts: &[String],
) -> Vec<ModelPackageBundleVerificationSummary> {
    let mut records = BTreeSet::new();
    for artifact in artifacts {
        let Some(report) = read_json_artifact(artifact) else {
            continue;
        };
        if string_at(&report, &["schema_version"])
            != "circuitci.model_package_bundle_verification.v1"
        {
            continue;
        }
        records.insert(ModelPackageBundleVerificationSummary {
            report: artifact.clone(),
            result: string_at(&report, &["result"]),
            bundle_path: string_at(&report, &["bundle_path"]),
            package_name: optional_string_at(&report, &["package", "name"]),
            package_version: optional_string_at(&report, &["package", "version"]),
            manifest_path: string_at(&report, &["manifest", "path"]),
            manifest_sha256_actual: optional_string_at(&report, &["manifest", "sha256_actual"]),
            lock_path: optional_string_at(&report, &["lock", "path"]),
            lock_sha256_actual: optional_string_at(&report, &["lock", "sha256_actual"]),
            registry_path: optional_string_at(&report, &["registry", "path"]),
            registry_sha256_actual: optional_string_at(&report, &["registry", "sha256_actual"]),
            artifact_count: array_len_at(&report, &["artifacts"]),
            conformance_check_count: array_len_at(&report, &["conformance_checks"]),
            finding_count: array_len_at(&report, &["findings"]),
        });
    }
    records.into_iter().collect()
}

fn collect_model_package_bundle_installs(
    artifacts: &[String],
) -> Vec<ModelPackageBundleInstallSummary> {
    let mut records = BTreeSet::new();
    for artifact in artifacts {
        let Some(report) = read_json_artifact(artifact) else {
            continue;
        };
        if string_at(&report, &["schema_version"]) != "circuitci.model_package_bundle_install.v1" {
            continue;
        }
        records.insert(ModelPackageBundleInstallSummary {
            report: artifact.clone(),
            result: string_at(&report, &["result"]),
            source_bundle: string_at(&report, &["source_bundle"]),
            install_dir: string_at(&report, &["install_dir"]),
            package_name: optional_string_at(&report, &["package", "name"]),
            package_version: optional_string_at(&report, &["package", "version"]),
            installed_registry_path: optional_string_at(&report, &["installed_registry", "path"]),
            installed_registry_sha256_actual: optional_string_at(
                &report,
                &["installed_registry", "sha256_actual"],
            ),
            model_package_registry_path: optional_string_at(
                &report,
                &["scenario_import", "model_package_registry_path"],
            ),
            model_package_registry_sha256: optional_string_at(
                &report,
                &["scenario_import", "model_package_registry_sha256"],
            ),
            model_package_registry_entry: optional_string_at(
                &report,
                &["scenario_import", "model_package_registry_entry"],
            ),
            model_package_lock_path: optional_string_at(
                &report,
                &["scenario_import", "model_package_lock_path"],
            ),
            model_package_lock_sha256: optional_string_at(
                &report,
                &["scenario_import", "model_package_lock_sha256"],
            ),
            model_package_artifact_id: optional_string_at(
                &report,
                &["scenario_import", "model_package_artifact_id"],
            ),
            artifact_count: array_len_at(&report, &["artifacts"]),
            conformance_check_count: array_len_at(&report, &["conformance_checks"]),
            finding_count: array_len_at(&report, &["findings"]),
        });
    }
    records.into_iter().collect()
}

fn read_json_artifact(artifact: &str) -> Option<Value> {
    if !artifact.ends_with(".json") {
        return None;
    }
    let text = fs::read_to_string(artifact).ok()?;
    serde_json::from_str::<Value>(&text).ok()
}

fn string_at(value: &Value, path: &[&str]) -> String {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(*key) else {
            return String::new();
        };
        current = next;
    }
    current.as_str().unwrap_or_default().to_string()
}

fn array_len_at(value: &Value, path: &[&str]) -> usize {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(*key) else {
            return 0;
        };
        current = next;
    }
    current.as_array().map_or(0, Vec::len)
}

fn optional_string_at(value: &Value, path: &[&str]) -> Option<String> {
    let text = string_at(value, path);
    if text.is_empty() { None } else { Some(text) }
}

fn string_array_at(value: &Value, path: &[&str]) -> Vec<String> {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(*key) else {
            return Vec::new();
        };
        current = next;
    }
    current
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_report_projects_model_package_conformance_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let package_report = dir.path().join("model_package_verification.json");
        let bundle_verification_report = dir.path().join("bundle_verification.json");
        let bundle_install_report = dir.path().join("bundle_install.json");
        fs::write(
            &package_report,
            r#"{
  "schema_version": "circuitci.model_package_verification.v1",
  "result": "pass",
  "lock": {
    "path": "package.lock.json",
    "sha256": "lock-sha",
    "package_name": "org.circuitci.test.model",
    "package_version": "1.0.0"
  },
  "registry": null,
  "artifacts": [],
  "conformance_checks": [
    {
      "report_artifact_id": "conformance",
      "report_path": "conformance.json",
      "target_artifact_id": "runtime_osdi",
      "target_artifact_sha256": "runtime-sha",
      "check_name": "transient_smoke",
      "analysis": "tran",
      "solver": "ngspice",
      "result": "pass",
      "artifacts": ["solver_manifest.json"]
    }
  ],
  "findings": []
}
"#,
        )
        .unwrap();
        fs::write(
            &bundle_verification_report,
            r#"{
  "schema_version": "circuitci.model_package_bundle_verification.v1",
  "result": "pass",
  "bundle_path": "bundle",
  "manifest": {
    "path": "model_package_bundle_manifest.json",
    "sha256_declared": "manifest-sha",
    "sha256_actual": "manifest-sha",
    "status": "verified"
  },
  "package": {
    "name": "org.circuitci.test.model",
    "version": "1.0.0"
  },
  "lock": {
    "path": "package.lock.json",
    "sha256_declared": "lock-sha",
    "sha256_actual": "lock-sha",
    "status": "verified"
  },
  "registry": {
    "path": "compact_model_registry.json",
    "sha256_declared": "registry-sha",
    "sha256_actual": "registry-sha",
    "status": "verified"
  },
  "verification_report": null,
  "verification_markdown": null,
  "readme": null,
  "artifacts": [
    {
      "id": "runtime_osdi",
      "path": "artifacts/runtime.osdi",
      "artifact_format": "osdi_shared_object",
      "compiler": "openvaf",
      "sha256_declared": "runtime-sha",
      "sha256_actual": "runtime-sha",
      "status": "verified"
    }
  ],
  "conformance_checks": [
    {
      "report_artifact_id": "conformance",
      "report_path": "conformance.json",
      "target_artifact_id": "runtime_osdi",
      "target_artifact_sha256": "runtime-sha",
      "check_name": "transient_smoke",
      "analysis": "tran",
      "solver": "ngspice",
      "result": "pass",
      "artifacts": ["solver_manifest.json"]
    }
  ],
  "findings": []
}
"#,
        )
        .unwrap();
        fs::write(
            &bundle_install_report,
            r#"{
  "schema_version": "circuitci.model_package_bundle_install.v1",
  "result": "pass",
  "source_bundle": "bundle",
  "install_dir": "installed_bundle",
  "package": {
    "name": "org.circuitci.test.model",
    "version": "1.0.0"
  },
  "manifest": {
    "path": "installed_bundle/model_package_bundle_manifest.json",
    "sha256_declared": "manifest-sha",
    "sha256_actual": "manifest-sha",
    "status": "verified"
  },
  "lock": {
    "path": "installed_bundle/package.lock.json",
    "sha256_declared": "lock-sha",
    "sha256_actual": "lock-sha",
    "status": "verified"
  },
  "registry": null,
  "installed_registry": {
    "path": "shared/compact_model_registry.json",
    "sha256_declared": "registry-sha",
    "sha256_actual": "registry-sha",
    "status": "verified"
  },
  "scenario_import": {
    "model_package_registry_path": "shared/compact_model_registry.json",
    "model_package_registry_sha256": "registry-sha",
    "model_package_registry_entry": "bundle_fixture_runtime",
    "model_package_lock_path": "installed_bundle/package.lock.json",
    "model_package_lock_sha256": "lock-sha",
    "model_package_artifact_id": "runtime_osdi"
  },
  "artifacts": [
    {
      "id": "runtime_osdi",
      "path": "installed_bundle/artifacts/runtime.osdi",
      "artifact_format": "osdi_shared_object",
      "compiler": "openvaf",
      "sha256_declared": "runtime-sha",
      "sha256_actual": "runtime-sha",
      "status": "verified"
    }
  ],
  "conformance_checks": [],
  "findings": []
}
"#,
        )
        .unwrap();

        let report = ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            Vec::new(),
            Vec::new(),
            vec![
                package_report.to_string_lossy().into_owned(),
                bundle_verification_report.to_string_lossy().into_owned(),
                bundle_install_report.to_string_lossy().into_owned(),
            ],
            Vec::new(),
            "validate".to_string(),
        );

        assert_eq!(report.model_package_conformance_checks.len(), 1);
        let check = &report.model_package_conformance_checks[0];
        assert_eq!(check.check_name, "transient_smoke");
        assert_eq!(check.target_artifact_id, "runtime_osdi");
        assert_eq!(check.artifacts, vec!["solver_manifest.json"]);
        assert_eq!(report.model_package_bundle_verifications.len(), 1);
        let bundle = &report.model_package_bundle_verifications[0];
        assert_eq!(bundle.result, "pass");
        assert_eq!(bundle.bundle_path, "bundle");
        assert_eq!(
            bundle.package_name.as_deref(),
            Some("org.circuitci.test.model")
        );
        assert_eq!(bundle.lock_path.as_deref(), Some("package.lock.json"));
        assert_eq!(bundle.artifact_count, 1);
        assert_eq!(bundle.conformance_check_count, 1);
        assert_eq!(report.model_package_bundle_installs.len(), 1);
        let install = &report.model_package_bundle_installs[0];
        assert_eq!(install.result, "pass");
        assert_eq!(install.install_dir, "installed_bundle");
        assert_eq!(
            install.model_package_registry_entry.as_deref(),
            Some("bundle_fixture_runtime")
        );
        assert_eq!(
            install.model_package_artifact_id.as_deref(),
            Some("runtime_osdi")
        );
        let markdown = markdown_report(&report);
        assert!(markdown.contains("## Model Package Conformance"));
        assert!(markdown.contains("`transient_smoke`"));
        assert!(markdown.contains("`runtime_osdi`"));
        assert!(markdown.contains("## Model Package Bundle Verification"));
        assert!(markdown.contains("## Model Package Bundle Install"));
        assert!(markdown.contains("`bundle_fixture_runtime`"));
    }
}
