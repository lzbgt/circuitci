use crate::reports::{
    Finding, Severity, SuiteCaseReport, SuiteFindingEvidence, SuiteFindingExpectation,
    SuiteRepairReport, SuiteReport, SuiteSummary, ValidationReport, write_reports,
};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteManifest {
    pub suite: SuiteMetadata,
    pub cases: Vec<SuiteCase>,
    #[serde(default)]
    pub repairs: Vec<SuiteRepair>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteMetadata {
    pub name: String,
    pub version: String,
    pub validation_profile: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteCase {
    pub id: String,
    pub project: PathBuf,
    pub expect: ExpectedResult,
    #[serde(default)]
    pub required_findings: Vec<RequiredFinding>,
    #[serde(default)]
    pub required_artifacts: Vec<String>,
    #[serde(default)]
    pub required_waveforms: Vec<String>,
    #[serde(default)]
    pub allowed_blocking_limitations: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteRepair {
    pub id: String,
    pub detects_case: String,
    pub fixed_case: String,
    pub fixes_findings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedResult {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
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
        let mut case_ids = BTreeSet::new();
        for case in &self.cases {
            if !safe_case_id(&case.id) {
                bail!(
                    "Suite case id {} is invalid; use ASCII letters, digits, '_' or '-'.",
                    case.id
                );
            }
            if !case_ids.insert(case.id.as_str()) {
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
        let mut repair_ids = BTreeSet::new();
        for repair in &self.repairs {
            if !safe_case_id(&repair.id) {
                bail!(
                    "Suite repair id {} is invalid; use ASCII letters, digits, '_' or '-'.",
                    repair.id
                );
            }
            if !repair_ids.insert(repair.id.as_str()) {
                bail!("Duplicate suite repair id {}.", repair.id);
            }
            if !case_ids.contains(repair.detects_case.as_str()) {
                bail!(
                    "Suite repair {} references unknown detects_case {}.",
                    repair.id,
                    repair.detects_case
                );
            }
            if !case_ids.contains(repair.fixed_case.as_str()) {
                bail!(
                    "Suite repair {} references unknown fixed_case {}.",
                    repair.id,
                    repair.fixed_case
                );
            }
            if repair.fixes_findings.is_empty() {
                bail!(
                    "Suite repair {} fixes_findings must not be empty.",
                    repair.id
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
    let mut validation_reports = BTreeMap::new();

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
        validation_reports.insert(case.id.clone(), report);
    }

    let case_report_by_id: BTreeMap<_, _> = case_reports
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect();
    let case_manifest_by_id: BTreeMap<_, _> = manifest
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect();
    let repair_reports: Vec<_> = manifest
        .repairs
        .iter()
        .map(|repair| {
            evaluate_repair(
                repair,
                &case_manifest_by_id,
                &case_report_by_id,
                &validation_reports,
            )
        })
        .collect();

    let passed = case_reports
        .iter()
        .filter(|case| case.result == "pass")
        .count();
    let failed = case_reports.len() - passed;
    let repairs_passed = repair_reports
        .iter()
        .filter(|repair| repair.result == "pass")
        .count();
    let repairs_failed = repair_reports.len() - repairs_passed;
    let result = if failed == 0 && repairs_failed == 0 {
        "pass"
    } else {
        "fail"
    }
    .to_string();
    Ok(SuiteReport {
        schema_version: "0.1.0".to_string(),
        suite: manifest.suite.name,
        validation_profile: manifest.suite.validation_profile,
        result,
        summary: SuiteSummary {
            cases: case_reports.len(),
            passed,
            failed,
            repairs: repair_reports.len(),
            repairs_passed,
            repairs_failed,
        },
        cases: case_reports,
        repairs: repair_reports,
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
    let outcome = crate::validation::validate(&bound);
    let report = ValidationReport::from_parts(
        project.project.name.clone(),
        profile.to_string(),
        outcome.findings,
        outcome.limitations,
        outcome.artifacts,
        outcome.waveforms,
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
    let required_artifacts = case.required_artifacts.clone();
    let required_waveforms = case.required_waveforms.clone();
    let mut matched_artifacts = Vec::new();
    let mut matched_waveforms = Vec::new();
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
    for artifact in &case.required_artifacts {
        if report.artifacts.iter().any(|actual| actual == artifact) {
            matched_artifacts.push(artifact.clone());
        } else {
            messages.push(format!("Required artifact {artifact} was not present."));
        }
    }
    for waveform in &case.required_waveforms {
        if report.waveforms.iter().any(|actual| actual == waveform) {
            matched_waveforms.push(waveform.clone());
        } else {
            messages.push(format!("Required waveform {waveform} was not present."));
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
        required_artifacts,
        matched_artifacts,
        required_waveforms,
        matched_waveforms,
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

fn evaluate_repair(
    repair: &SuiteRepair,
    case_manifest_by_id: &BTreeMap<&str, &SuiteCase>,
    case_report_by_id: &BTreeMap<&str, &SuiteCaseReport>,
    validation_reports: &BTreeMap<String, ValidationReport>,
) -> SuiteRepairReport {
    let detect_case = case_report_by_id
        .get(repair.detects_case.as_str())
        .expect("repair detects_case was prevalidated");
    let fixed_case = case_report_by_id
        .get(repair.fixed_case.as_str())
        .expect("repair fixed_case was prevalidated");
    let detect_manifest = case_manifest_by_id
        .get(repair.detects_case.as_str())
        .expect("repair detects_case manifest was prevalidated");
    let fixed_manifest = case_manifest_by_id
        .get(repair.fixed_case.as_str())
        .expect("repair fixed_case manifest was prevalidated");
    let detect_report = validation_reports
        .get(&repair.detects_case)
        .expect("repair detects_case report was generated");
    let fixed_report = validation_reports
        .get(&repair.fixed_case)
        .expect("repair fixed_case report was generated");

    let mut messages = Vec::new();
    if detect_manifest.expect != ExpectedResult::Fail {
        messages.push(format!(
            "detects_case {} must be declared expect: fail.",
            repair.detects_case
        ));
    }
    if fixed_manifest.expect != ExpectedResult::Pass {
        messages.push(format!(
            "fixed_case {} must be declared expect: pass.",
            repair.fixed_case
        ));
    }
    if detect_case.result != "pass" || detect_case.actual != "fail" {
        messages.push(format!(
            "detects_case {} must pass suite expectations with actual project result fail.",
            repair.detects_case
        ));
    }
    if fixed_case.result != "pass" || fixed_case.actual != "pass" {
        messages.push(format!(
            "fixed_case {} must pass suite expectations with actual project result pass.",
            repair.fixed_case
        ));
    }

    let mut matched_findings = Vec::new();
    let mut suggested_fixes = BTreeSet::new();
    for finding_id in &repair.fixes_findings {
        let matches: Vec<_> = detect_report
            .failures
            .iter()
            .filter(|finding| finding.id == *finding_id)
            .collect();
        if matches.is_empty() {
            messages.push(format!(
                "detects_case {} does not contain critical finding {}.",
                repair.detects_case, finding_id
            ));
        }
        for finding in matches {
            matched_findings.push(finding_evidence(finding));
            for fix in &finding.suggested_fixes {
                suggested_fixes.insert(fix.clone());
            }
        }
    }

    if !fixed_report.failures.is_empty() {
        messages.push(format!(
            "fixed_case {} still has critical findings.",
            repair.fixed_case
        ));
    }
    if !fixed_case.blocking_limitations.is_empty() {
        messages.push(format!(
            "fixed_case {} has blocking limitations.",
            repair.fixed_case
        ));
    }

    let result = if messages.is_empty() { "pass" } else { "fail" }.to_string();
    SuiteRepairReport {
        id: repair.id.clone(),
        detects_case: repair.detects_case.clone(),
        fixed_case: repair.fixed_case.clone(),
        fixes_findings: repair.fixes_findings.clone(),
        detect_project: detect_case.project.clone(),
        fixed_project: fixed_case.project.clone(),
        detect_report: detect_case.report.clone(),
        fixed_report: fixed_case.report.clone(),
        matched_findings,
        suggested_fixes: suggested_fixes.into_iter().collect(),
        result,
        messages,
    }
}

fn finding_evidence(finding: &Finding) -> SuiteFindingEvidence {
    SuiteFindingEvidence {
        id: finding.id.clone(),
        severity: severity_name(&finding.severity).to_string(),
        scenario: finding.scenario.clone(),
        component: finding.component.clone(),
        net: finding.net.clone(),
        message: finding.message.clone(),
        suggested_fixes: finding.suggested_fixes.clone(),
    }
}

fn severity_name(severity: &Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
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
