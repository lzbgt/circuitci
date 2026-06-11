use crate::board_ir::Endpoint;
use serde::Serialize;
use std::collections::BTreeMap;
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
    pub limitations: Vec<Limitation>,
    pub suggested_next_actions: Vec<String>,
    pub reproduction: Reproduction,
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
        Self {
            schema_version: "0.1.0".to_string(),
            project,
            profile,
            result,
            summary,
            failures,
            warnings,
            infos,
            waveforms: Vec::new(),
            artifacts: Vec::new(),
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
