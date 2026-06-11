use crate::reports::{
    SuiteCaseReport, SuiteFindingExpectation, SuiteReport, SuiteSummary, ValidationReport,
    write_reports,
};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct SuiteManifest {
    pub suite: SuiteMetadata,
    pub cases: Vec<SuiteCase>,
}

#[derive(Debug, Deserialize)]
pub struct SuiteMetadata {
    pub name: String,
    pub version: String,
    pub validation_profile: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SuiteCase {
    pub id: String,
    pub project: PathBuf,
    pub expect: ExpectedResult,
    #[serde(default)]
    pub required_findings: Vec<RequiredFinding>,
    #[serde(default)]
    pub allowed_blocking_limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedResult {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequiredFinding {
    pub id: String,
    pub severity: RequiredSeverity,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequiredSeverity {
    Critical,
    Warning,
    Info,
}

impl SuiteManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read suite manifest {}.", path.display()))?;
        let manifest: Self = serde_yaml_ng::from_str(&text)
            .with_context(|| format!("Failed to parse suite manifest {}.", path.display()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<()> {
        if self.suite.name.trim().is_empty() {
            bail!("suite.name is required.");
        }
        if self.suite.version.trim().is_empty() {
            bail!("suite.version is required.");
        }
        if self.suite.validation_profile.trim().is_empty() {
            bail!("suite.validation_profile is required.");
        }
        if self.cases.is_empty() {
            bail!("suite cases are required.");
        }
        let mut ids = BTreeSet::new();
        for case in &self.cases {
            if !safe_case_id(&case.id) {
                bail!(
                    "Suite case id {} is invalid; use ASCII letters, digits, '_' or '-'.",
                    case.id
                );
            }
            if !ids.insert(case.id.as_str()) {
                bail!("Duplicate suite case id {}.", case.id);
            }
            if case.project.as_os_str().is_empty() {
                bail!("Suite case {} project path is required.", case.id);
            }
            if case.expect == ExpectedResult::Fail
                && !case
                    .required_findings
                    .iter()
                    .any(|finding| finding.severity == RequiredSeverity::Critical)
            {
                bail!(
                    "Suite case {} expects fail but has no required critical finding.",
                    case.id
                );
            }
        }
        Ok(())
    }
}

pub fn run_suite<F>(
    manifest_path: &Path,
    output: &Path,
    command: String,
    mut validate_project: F,
) -> Result<SuiteReport>
where
    F: FnMut(&Path, &str, &Path) -> Result<ValidationReport>,
{
    let manifest = SuiteManifest::load(manifest_path)?;
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let cases_dir = output.join("cases");
    let mut case_reports = Vec::new();

    for case in &manifest.cases {
        let project_path = manifest_dir.join(&case.project);
        let case_output = cases_dir.join(&case.id);
        let report = validate_project(
            &project_path,
            &manifest.suite.validation_profile,
            &case_output,
        )
        .with_context(|| format!("Suite case {} failed to validate project.", case.id))?;
        let case_report_path = Path::new("cases").join(&case.id).join("report.json");
        let report_ref = case_report_path.to_string_lossy().replace('\\', "/");
        let case_report = evaluate_case(case, &report, report_ref);
        case_reports.push(case_report);
    }

    let passed = case_reports
        .iter()
        .filter(|case| case.result == "pass")
        .count();
    let failed = case_reports.len() - passed;
    let result = if failed == 0 { "pass" } else { "fail" }.to_string();
    Ok(SuiteReport {
        schema_version: "0.1.0".to_string(),
        suite: manifest.suite.name,
        validation_profile: manifest.suite.validation_profile,
        result,
        summary: SuiteSummary {
            cases: case_reports.len(),
            passed,
            failed,
        },
        cases: case_reports,
        reproduction: crate::reports::Reproduction { command },
    })
}

pub fn validate_and_write_project_report(
    project_path: &Path,
    profile: &str,
    output: &Path,
    command: String,
) -> Result<ValidationReport> {
    let project = crate::board_ir::load_project(project_path)?;
    let (library, library_findings) = crate::library::load_library(project_path, &project);
    let bound = crate::library::bind_project(&project, library, library_findings);
    let (findings, limitations) = crate::validation::validate(&bound);
    let report = ValidationReport::from_parts(
        project.project.name.clone(),
        profile.to_string(),
        findings,
        limitations,
        command,
    );
    write_reports(&report, output)?;
    Ok(report)
}

fn evaluate_case(
    case: &SuiteCase,
    report: &ValidationReport,
    report_ref: String,
) -> SuiteCaseReport {
    let expect = case.expect.as_str().to_string();
    let actual = report.result.clone();
    let mut messages = Vec::new();
    if actual != expect {
        messages.push(format!(
            "Expected project result {expect}, observed {actual}."
        ));
    }

    let required_findings: Vec<_> = case
        .required_findings
        .iter()
        .map(RequiredFinding::as_suite_expectation)
        .collect();
    let mut matched_findings = Vec::new();
    for required in &case.required_findings {
        let expected = required.as_suite_expectation();
        if report_has_finding(report, required) {
            matched_findings.push(expected);
        } else {
            messages.push(format!(
                "Required {} finding {} was not present.",
                required.severity.as_str(),
                required.id
            ));
        }
    }

    let allowed: BTreeSet<&str> = case
        .allowed_blocking_limitations
        .iter()
        .map(String::as_str)
        .collect();
    let blocking_limitations: Vec<_> = report
        .limitations
        .iter()
        .filter(|limitation| limitation.blocking)
        .map(|limitation| limitation.id.clone())
        .collect();
    if case.expect == ExpectedResult::Pass {
        for limitation in &blocking_limitations {
            if !allowed.contains(limitation.as_str()) {
                messages.push(format!("Blocking limitation {limitation} is not allowed."));
            }
        }
    }

    let result = if messages.is_empty() { "pass" } else { "fail" }.to_string();
    SuiteCaseReport {
        id: case.id.clone(),
        project: case.project.to_string_lossy().replace('\\', "/"),
        expect,
        actual,
        result,
        required_findings,
        matched_findings,
        blocking_limitations,
        report: report_ref,
        messages,
    }
}

fn report_has_finding(report: &ValidationReport, required: &RequiredFinding) -> bool {
    let findings = match required.severity {
        RequiredSeverity::Critical => &report.failures,
        RequiredSeverity::Warning => &report.warnings,
        RequiredSeverity::Info => &report.infos,
    };
    findings.iter().any(|finding| finding.id == required.id)
}

fn safe_case_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

impl ExpectedResult {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }
}

impl RequiredFinding {
    fn as_suite_expectation(&self) -> SuiteFindingExpectation {
        SuiteFindingExpectation {
            id: self.id.clone(),
            severity: self.severity.as_str().to_string(),
        }
    }
}

impl RequiredSeverity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}
