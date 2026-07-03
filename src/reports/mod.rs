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
    pub compiler_available_on_path: Option<bool>,
    pub build_env_enabled: Option<bool>,
    pub rebuild_mode: String,
    pub produced_by_circuitci: Option<bool>,
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

fn optional_string_at(value: &Value, path: &[&str]) -> Option<String> {
    let text = string_at(value, path);
    if text.is_empty() { None } else { Some(text) }
}

fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}
