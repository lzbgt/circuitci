use crate::board_ir::Endpoint;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

mod analog_summaries;
mod artifact_summaries;
mod findings_markdown;
pub use analog_summaries::{
    DistortionSummary, FourierSummary, HarmonicBalanceSummary, PoleZeroSummary,
    SParameterNetworkSummary, SParameterNoiseSummary, SParameterSummary, SensitivitySummary,
    TransferFunctionSummary,
};
use analog_summaries::{
    collect_distortion_summaries, collect_fourier_summaries, collect_harmonic_balance_summaries,
    collect_pole_zero_summaries, collect_s_parameter_network_summaries,
    collect_s_parameter_noise_summaries, collect_s_parameter_summaries,
    collect_sensitivity_summaries, collect_transfer_function_summaries,
    render_harmonic_balance_summary_markdown, render_s_parameter_network_summary_markdown,
};
pub use artifact_summaries::collect_yaml_repair_summaries;
use artifact_summaries::{
    collect_model_file_provenance, collect_model_package_bundle_imports,
    collect_model_package_bundle_installs, collect_model_package_bundle_verifications,
    collect_model_package_conformance_checks,
};
use findings_markdown::push_findings;

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
    pub distortion_summaries: Vec<DistortionSummary>,
    pub fourier_summaries: Vec<FourierSummary>,
    pub hb_summaries: Vec<HarmonicBalanceSummary>,
    pub pole_zero_summaries: Vec<PoleZeroSummary>,
    pub sensitivity_summaries: Vec<SensitivitySummary>,
    pub transfer_function_summaries: Vec<TransferFunctionSummary>,
    pub s_parameter_summaries: Vec<SParameterSummary>,
    pub s_parameter_network_summaries: Vec<SParameterNetworkSummary>,
    pub s_parameter_noise_summaries: Vec<SParameterNoiseSummary>,
    pub model_file_provenance: Vec<ModelFileProvenance>,
    pub model_package_conformance_checks: Vec<ModelPackageConformanceCheck>,
    pub model_package_bundle_verifications: Vec<ModelPackageBundleVerificationSummary>,
    pub model_package_bundle_installs: Vec<ModelPackageBundleInstallSummary>,
    pub model_package_bundle_imports: Vec<ModelPackageBundleImportSummary>,
    pub yaml_repairs: Vec<YamlRepairSummary>,
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
    pub repair_yaml_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelPackageBundleImportSummary {
    pub report: String,
    pub result: String,
    pub bundle_path: String,
    pub project: String,
    pub profile: String,
    pub install_dir: String,
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub bundle_install_report: Option<String>,
    pub package_verification_report: Option<String>,
    pub yaml_repair_report: Option<String>,
    pub repaired_project: Option<String>,
    pub repaired_validation_report: Option<String>,
    pub model_package_registry_path: Option<String>,
    pub model_package_registry_sha256: Option<String>,
    pub model_package_registry_entry: Option<String>,
    pub model_package_lock_path: Option<String>,
    pub model_package_lock_sha256: Option<String>,
    pub model_package_artifact_id: Option<String>,
    pub bundle_artifacts: usize,
    pub conformance_checks: usize,
    pub package_findings: usize,
    pub repair_applied: usize,
    pub repair_blocked: usize,
    pub repair_new_criticals: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct YamlRepairSummary {
    pub report: String,
    pub result: String,
    pub finding: String,
    pub mode: String,
    pub original_project: String,
    pub repaired_project: Option<String>,
    pub original_report: String,
    pub repaired_report: Option<String>,
    pub proposed: usize,
    pub selected: usize,
    pub applied: usize,
    pub blocked: usize,
    pub skipped: usize,
    pub original_matching_findings: usize,
    pub repaired_matching_findings: usize,
    pub original_matching_criticals: usize,
    pub repaired_matching_criticals: usize,
    pub new_criticals: usize,
    pub original_finding_removed: Option<bool>,
    pub no_new_criticals: Option<bool>,
    pub reason_codes: Vec<String>,
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

pub struct ValidationReportReproductionInput<'a> {
    pub command: String,
    pub project_path: Option<&'a Path>,
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
        Self::from_parts_with_reproduction(
            project,
            profile,
            findings,
            limitations,
            artifacts,
            waveforms,
            ValidationReportReproductionInput {
                command,
                project_path: None,
            },
        )
    }

    pub fn from_parts_with_reproduction(
        project: String,
        profile: String,
        findings: Vec<Finding>,
        limitations: Vec<Limitation>,
        artifacts: Vec<String>,
        waveforms: Vec<String>,
        reproduction: ValidationReportReproductionInput<'_>,
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
        let distortion_summaries = collect_distortion_summaries(&artifacts);
        let fourier_summaries = collect_fourier_summaries(&artifacts);
        let hb_summaries = collect_harmonic_balance_summaries(&artifacts);
        let pole_zero_summaries = collect_pole_zero_summaries(&artifacts);
        let sensitivity_summaries = collect_sensitivity_summaries(&artifacts);
        let transfer_function_summaries = collect_transfer_function_summaries(&artifacts);
        let s_parameter_summaries = collect_s_parameter_summaries(&artifacts);
        let s_parameter_network_summaries = collect_s_parameter_network_summaries(&artifacts);
        let s_parameter_noise_summaries = collect_s_parameter_noise_summaries(&artifacts);
        let model_file_provenance = collect_model_file_provenance(&artifacts);
        let model_package_conformance_checks = collect_model_package_conformance_checks(&artifacts);
        let model_package_bundle_verifications =
            collect_model_package_bundle_verifications(&artifacts);
        let model_package_bundle_installs = collect_model_package_bundle_installs(
            &artifacts,
            &profile,
            &reproduction.command,
            reproduction.project_path,
        );
        let model_package_bundle_imports = collect_model_package_bundle_imports(&artifacts);
        let yaml_repairs = collect_yaml_repair_summaries(&artifacts);
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
            distortion_summaries,
            fourier_summaries,
            hb_summaries,
            pole_zero_summaries,
            sensitivity_summaries,
            transfer_function_summaries,
            s_parameter_summaries,
            s_parameter_network_summaries,
            s_parameter_noise_summaries,
            model_file_provenance,
            model_package_conformance_checks,
            model_package_bundle_verifications,
            model_package_bundle_installs,
            model_package_bundle_imports,
            yaml_repairs,
            limitations,
            suggested_next_actions,
            reproduction: Reproduction {
                command: reproduction.command,
            },
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

pub fn markdown_report(report: &ValidationReport) -> String {
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
    text.push_str("## Distortion Summary\n\n");
    if report.distortion_summaries.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for row in &report.distortion_summaries {
            text.push_str(&format!(
                "- `{}` `{}`: rows={} max_magnitude={:.6e} at {:.6e} Hz\n",
                row.component,
                row.output_expression,
                row.row_count,
                row.max_magnitude,
                row.frequency_hz_at_max
            ));
            text.push_str(&format!("  - Artifact: `{}`\n", row.artifact));
        }
        text.push('\n');
    }
    text.push_str("## Fourier Summary\n\n");
    if report.fourier_summaries.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for row in &report.fourier_summaries {
            let thd = row
                .thd_percent
                .map(|value| format!("{value:.6e}%"))
                .unwrap_or_else(|| "n/a".to_string());
            text.push_str(&format!(
                "- `{}` h{}: frequency={:.6e} Hz magnitude={:.6e} phase={:.6e} deg normalized_magnitude={:.6e} normalized_phase={:.6e} deg THD={}\n",
                row.output_expression,
                row.harmonic,
                row.frequency_hz,
                row.magnitude,
                row.phase_deg,
                row.normalized_magnitude,
                row.normalized_phase_deg,
                thd
            ));
            text.push_str(&format!("  - Artifact: `{}`\n", row.artifact));
        }
        text.push('\n');
    }
    text.push_str(&render_harmonic_balance_summary_markdown(
        &report.hb_summaries,
    ));
    text.push_str("## Pole-Zero Summary\n\n");
    if report.pole_zero_summaries.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for row in &report.pole_zero_summaries {
            text.push_str(&format!(
                "- `{}` {}: real={:.6e} rad/s imaginary={:.6e} rad/s frequency={:.6e} Hz output=`{}` reference=`{}` input=`{}` mode=`{}`\n",
                row.root_kind,
                row.root_index,
                row.real_rad_per_s,
                row.imaginary_rad_per_s,
                row.frequency_hz,
                row.output_node,
                row.reference_node,
                row.input_source,
                row.mode
            ));
            text.push_str(&format!("  - Artifact: `{}`\n", row.artifact));
        }
        text.push('\n');
    }
    text.push_str("## Sensitivity Summary\n\n");
    if report.sensitivity_summaries.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for row in &report.sensitivity_summaries {
            let frequency = row
                .frequency_hz
                .map(|value| format!("{value:.6e} Hz"))
                .unwrap_or_else(|| "dc".to_string());
            text.push_str(&format!(
                "- `{}` `{}` `{}`: frequency={} real={:.6e} imaginary={:.6e} magnitude={:.6e}\n",
                row.output_expression,
                row.mode,
                row.parameter,
                frequency,
                row.sensitivity_real,
                row.sensitivity_imaginary,
                row.sensitivity_magnitude
            ));
            text.push_str(&format!("  - Artifact: `{}`\n", row.artifact));
        }
        text.push('\n');
    }
    text.push_str("## Transfer Function Summary\n\n");
    if report.transfer_function_summaries.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for row in &report.transfer_function_summaries {
            text.push_str(&format!(
                "- `{}` from `{}`: gain={:.6e} input_resistance={:.6e} ohm output_resistance={:.6e} ohm\n",
                row.output_expression,
                row.input_source,
                row.transfer_function_gain,
                row.input_resistance_ohm,
                row.output_resistance_ohm
            ));
            text.push_str(&format!("  - Artifact: `{}`\n", row.artifact));
        }
        text.push('\n');
    }
    text.push_str("## S-Parameter Summary\n\n");
    if report.s_parameter_summaries.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for row in &report.s_parameter_summaries {
            let return_loss = format_optional_range(row.min_return_loss_db, row.max_return_loss_db);
            let insertion_loss =
                format_optional_range(row.min_insertion_loss_db, row.max_insertion_loss_db);
            let vswr = format_optional_range(row.min_vswr, row.max_vswr);
            let mismatch_loss =
                format_optional_range(row.min_mismatch_loss_db, row.max_mismatch_loss_db);
            let group_delay = format_optional_range(row.min_group_delay_s, row.max_group_delay_s);
            let impedance_magnitude = format_optional_range(
                row.min_impedance_magnitude_ohm,
                row.max_impedance_magnitude_ohm,
            );
            text.push_str(&format!(
                "- `{}`: rows={} frequency={:.6e}..{:.6e} Hz magnitude_db={:.6e}..{:.6e} return_loss_db={} insertion_loss_db={} vswr={} mismatch_loss_db={} group_delay_s={} impedance_magnitude_ohm={}\n",
                row.parameter,
                row.row_count,
                row.min_frequency_hz,
                row.max_frequency_hz,
                row.min_mag_db,
                row.max_mag_db,
                return_loss,
                insertion_loss,
                vswr,
                mismatch_loss,
                group_delay,
                impedance_magnitude
            ));
            text.push_str(&format!("  - Artifact: `{}`\n", row.artifact));
        }
        text.push('\n');
    }
    text.push_str("## S-Parameter Network Summary\n\n");
    if report.s_parameter_network_summaries.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for row in &report.s_parameter_network_summaries {
            text.push_str(&render_s_parameter_network_summary_markdown(row));
        }
        text.push('\n');
    }
    text.push_str("## S-Parameter Noise Summary\n\n");
    if report.s_parameter_noise_summaries.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for row in &report.s_parameter_noise_summaries {
            text.push_str(&format!(
                "- rows={} frequency={:.6e}..{:.6e} Hz noise_figure_db_max={:.6e} at {:.6e} Hz minimum_noise_figure_db_max={:.6e} at {:.6e} Hz equivalent_noise_resistance_ohm_max={:.6e} at {:.6e} Hz optimum_source_reflection_magnitude_max={:.6e} at {:.6e} Hz\n",
                row.row_count,
                row.min_frequency_hz,
                row.max_frequency_hz,
                row.max_noise_figure_db,
                row.frequency_hz_at_max_noise_figure,
                row.max_minimum_noise_figure_db,
                row.frequency_hz_at_max_minimum_noise_figure,
                row.max_equivalent_noise_resistance_ohm,
                row.frequency_hz_at_max_equivalent_noise_resistance,
                row.max_optimum_source_reflection_magnitude,
                row.frequency_hz_at_max_optimum_source_reflection_magnitude
            ));
            text.push_str(&format!("  - Artifact: `{}`\n", row.artifact));
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
            if let Some(command) = &install.repair_yaml_command {
                text.push_str(&format!("  - Repair command: `{command}`\n"));
            }
        }
        text.push('\n');
    }
    text.push_str("## Model Package Bundle Import\n\n");
    if report.model_package_bundle_imports.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for import in &report.model_package_bundle_imports {
            text.push_str(&format!(
                "- `{}` `{}`: `{}` artifacts={} conformance_checks={} repair_applied={} repair_blocked={}\n",
                import
                    .package_name
                    .as_deref()
                    .unwrap_or("<unknown-package>"),
                import.package_version.as_deref().unwrap_or(""),
                import.result,
                import.bundle_artifacts,
                import.conformance_checks,
                import.repair_applied,
                import.repair_blocked
            ));
            text.push_str(&format!(
                "  - Bundle: `{}` project `{}` install `{}` report `{}`\n",
                import.bundle_path, import.project, import.install_dir, import.report
            ));
            if let Some(entry) = &import.model_package_registry_entry {
                text.push_str(&format!(
                    "  - Scenario import: registry `{}` sha `{}` entry `{}` lock `{}` sha `{}` artifact `{}`\n",
                    import.model_package_registry_path.as_deref().unwrap_or(""),
                    import.model_package_registry_sha256.as_deref().unwrap_or(""),
                    entry,
                    import.model_package_lock_path.as_deref().unwrap_or(""),
                    import.model_package_lock_sha256.as_deref().unwrap_or(""),
                    import.model_package_artifact_id.as_deref().unwrap_or("")
                ));
            }
            if let Some(repaired_project) = &import.repaired_project {
                text.push_str(&format!(
                    "  - Repaired project: `{}` report `{}`\n",
                    repaired_project,
                    import.repaired_validation_report.as_deref().unwrap_or("")
                ));
            }
        }
        text.push('\n');
    }
    text.push_str("## YAML Repairs\n\n");
    if report.yaml_repairs.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for repair in &report.yaml_repairs {
            text.push_str(&format!(
                "- `{}` via `{}`: `{}` proposed={} selected={} applied={} blocked={} skipped={}\n",
                repair.finding,
                repair.mode,
                repair.result,
                repair.proposed,
                repair.selected,
                repair.applied,
                repair.blocked,
                repair.skipped
            ));
            text.push_str(&format!(
                "  - Project: `{}` -> `{}`\n",
                repair.original_project,
                repair.repaired_project.as_deref().unwrap_or("")
            ));
            text.push_str(&format!(
                "  - Reports: `{}` -> `{}` repair `{}`\n",
                repair.original_report,
                repair.repaired_report.as_deref().unwrap_or(""),
                repair.report
            ));
            text.push_str(&format!(
                "  - Proof: original_finding_removed={} no_new_criticals={} new_criticals={}\n",
                repair
                    .original_finding_removed
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                repair
                    .no_new_criticals
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                repair.new_criticals
            ));
            if !repair.reason_codes.is_empty() {
                text.push_str(&format!(
                    "  - Reason codes: `{}`\n",
                    repair.reason_codes.join("`, `")
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

fn format_optional_range(min: Option<f64>, max: Option<f64>) -> String {
    match (min, max) {
        (Some(min), Some(max)) => format!("{min:.6e}..{max:.6e}"),
        _ => "n/a".to_string(),
    }
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
        let bundle_import_report = dir.path().join("bundle_import.json");
        let repair_report = dir.path().join("repair_report.json");
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
        fs::write(
            &repair_report,
            r#"{
  "schema_version": "circuitci.repair.v1",
  "project": "project",
  "profile": "profile",
  "finding": "BUNDLE_INSTALL_PACKAGE_METADATA",
  "mode": "apply",
  "result": "pass",
  "messages": [],
  "reason_codes": [],
  "summary": {
    "proposed": 1,
    "selected": 1,
    "applied": 1,
    "blocked": 0,
    "skipped": 0,
    "original_matching_findings": 0,
    "repaired_matching_findings": 0,
    "original_matching_criticals": 0,
    "repaired_matching_criticals": 0,
    "new_criticals": 0
  },
  "original_project": "project.yaml",
  "repaired_project": "repair/repaired.project.yaml",
  "original_report": "repair/original/report.json",
  "repaired_report": "repair/repaired/report.json",
  "proposals": [],
  "proof": {
    "original_finding_removed": true,
    "no_new_criticals": true,
    "original_matching_findings": [],
    "repaired_matching_findings": [],
    "new_critical_findings": []
  },
  "reproduction": {
    "command": "circuitci repair-yaml project.yaml --finding bundle-install-package-metadata"
  }
}
"#,
        )
        .unwrap();
        fs::write(
            &bundle_import_report,
            r#"{
  "schema_version": "circuitci.model_package_bundle_import.v1",
  "result": "pass",
  "bundle_path": "bundle",
  "project": "project.yaml",
  "profile": "profile",
  "install_dir": "installed_bundle",
  "runtime_budget_ms": null,
  "elapsed_ms": 12,
  "package": {
    "name": "org.circuitci.test.model",
    "version": "1.0.0"
  },
  "source_bundle_verification_report": "bundle_verification.json",
  "bundle_install_report": "bundle_install.json",
  "package_verification_report": "package_verification.json",
  "yaml_repair_report": "repair_yaml/repair_report.json",
  "repaired_project": "repair_yaml/repaired.project.yaml",
  "repaired_validation_report": "repair_yaml/repaired/report.json",
  "scenario_import": {
    "model_package_registry_path": "shared/compact_model_registry.json",
    "model_package_registry_sha256": "registry-sha",
    "model_package_registry_entry": "bundle_fixture_runtime",
    "model_package_lock_path": "installed_bundle/package.lock.json",
    "model_package_lock_sha256": "lock-sha",
    "model_package_artifact_id": "runtime_osdi"
  },
  "summary": {
    "bundle_artifacts": 1,
    "conformance_checks": 1,
    "package_findings": 0,
    "repair_proposed": 1,
    "repair_selected": 1,
    "repair_applied": 1,
    "repair_blocked": 0,
    "repair_skipped": 0,
    "repair_new_criticals": 0
  },
  "findings": [],
  "repair_reason_codes": [],
  "repair_messages": []
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
                bundle_import_report.to_string_lossy().into_owned(),
                repair_report.to_string_lossy().into_owned(),
            ],
            Vec::new(),
            "circuitci validate project.yaml --profile profile --output out".to_string(),
        );

        assert!(report.distortion_summaries.is_empty());
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
        let expected_repair_command = format!(
            "circuitci repair-yaml project.yaml --profile profile --output out/repair_bundle_import --finding bundle-install-package-metadata --bundle-install-report {}",
            bundle_install_report.to_string_lossy()
        );
        assert_eq!(
            install.repair_yaml_command.as_deref(),
            Some(expected_repair_command.as_str())
        );
        assert_eq!(report.model_package_bundle_imports.len(), 1);
        let import = &report.model_package_bundle_imports[0];
        assert_eq!(import.result, "pass");
        assert_eq!(import.bundle_path, "bundle");
        assert_eq!(import.repair_applied, 1);
        assert_eq!(import.repair_blocked, 0);
        assert_eq!(
            import.model_package_registry_entry.as_deref(),
            Some("bundle_fixture_runtime")
        );
        assert_eq!(report.yaml_repairs.len(), 1);
        let repair = &report.yaml_repairs[0];
        assert_eq!(repair.finding, "BUNDLE_INSTALL_PACKAGE_METADATA");
        assert_eq!(repair.result, "pass");
        assert_eq!(repair.applied, 1);
        assert_eq!(repair.blocked, 0);
        assert_eq!(
            repair.repaired_project.as_deref(),
            Some("repair/repaired.project.yaml")
        );
        assert_eq!(repair.original_finding_removed, Some(true));
        assert_eq!(repair.no_new_criticals, Some(true));
        let markdown = markdown_report(&report);
        assert!(markdown.contains("## Model Package Conformance"));
        assert!(markdown.contains("`transient_smoke`"));
        assert!(markdown.contains("`runtime_osdi`"));
        assert!(markdown.contains("## Model Package Bundle Verification"));
        assert!(markdown.contains("## Model Package Bundle Install"));
        assert!(markdown.contains("## Model Package Bundle Import"));
        assert!(markdown.contains("`bundle_fixture_runtime`"));
        assert!(markdown.contains("## YAML Repairs"));
        assert!(markdown.contains("`BUNDLE_INSTALL_PACKAGE_METADATA`"));
    }

    #[test]
    fn validation_report_projects_distortion_summary_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let summary = dir.path().join("distortion_summary.csv");
        fs::write(
            &summary,
            "component,output_expression,row_count,max_magnitude,frequency_hz_at_max\nh2,\"V(out,0)\",2,5.000000000000e-3,1.000000000000e+3\nh3,\"V(out,0)\",2,1.000000000000e-3,2.000000000000e+3\n",
        )
        .unwrap();

        let report = ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            Vec::new(),
            Vec::new(),
            vec![summary.to_string_lossy().into_owned()],
            Vec::new(),
            "circuitci validate project.yaml".to_string(),
        );

        assert_eq!(report.distortion_summaries.len(), 2);
        assert_eq!(report.distortion_summaries[0].component, "h2");
        assert_eq!(report.distortion_summaries[0].output_expression, "V(out,0)");
        assert_eq!(report.distortion_summaries[0].row_count, 2);
        assert_eq!(report.distortion_summaries[0].max_magnitude, 5.0e-3);
        let markdown = markdown_report(&report);
        assert!(markdown.contains("## Distortion Summary"));
        assert!(markdown.contains("`h2` `V(out,0)`"));
        assert!(markdown.contains("max_magnitude=5.000000e-3"));
    }

    #[test]
    fn validation_report_projects_fourier_summary_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let summary = dir.path().join("fourier_summary.csv");
        fs::write(
            &summary,
            "output_expression,fundamental_frequency_hz,reported_harmonics,harmonic,frequency_hz,magnitude,phase_deg,normalized_magnitude,normalized_phase_deg,thd_percent,grid_size,interpolation_degree,periods\n\"V(out,0)\",1.000000000000e5,5,0,0.000000000000e0,5.099860000000e-1,0.000000000000e0,0.000000000000e0,0.000000000000e0,1.854350000000e1,200,1,1\n\"V(out,0)\",1.000000000000e5,5,2,2.000000000000e5,1.242320000000e-2,3.132120000000e1,2.305810000000e-2,6.705410000000e1,1.854350000000e1,200,1,1\n",
        )
        .unwrap();

        let report = ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            Vec::new(),
            Vec::new(),
            vec![summary.to_string_lossy().into_owned()],
            Vec::new(),
            "circuitci validate project.yaml".to_string(),
        );

        assert_eq!(report.fourier_summaries.len(), 2);
        assert_eq!(report.fourier_summaries[0].output_expression, "V(out,0)");
        assert_eq!(report.fourier_summaries[0].harmonic, 0);
        assert_eq!(report.fourier_summaries[1].harmonic, 2);
        assert_eq!(report.fourier_summaries[1].normalized_magnitude, 2.30581e-2);
        assert_eq!(report.fourier_summaries[1].thd_percent, Some(18.5435));
        let markdown = markdown_report(&report);
        assert!(markdown.contains("## Fourier Summary"));
        assert!(markdown.contains("`V(out,0)` h2"));
        assert!(markdown.contains("normalized_magnitude=2.305810e-2"));
    }

    #[test]
    fn validation_report_projects_pole_zero_summary_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let summary = dir.path().join("pole_zero_summary.csv");
        fs::write(
            &summary,
            "output_node,reference_node,input_source,mode,root_kind,root_index,real_rad_per_s,imaginary_rad_per_s,frequency_hz\nout,0,V1,poles_and_zeros,pole,1,-1.000000000000e3,0.000000000000e0,1.591549430919e2\nout,0,V1,poles_and_zeros,zero,1,-2.000000000000e3,5.000000000000e2,3.281149852721e2\n",
        )
        .unwrap();

        let report = ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            Vec::new(),
            Vec::new(),
            vec![summary.to_string_lossy().into_owned()],
            Vec::new(),
            "circuitci validate project.yaml".to_string(),
        );

        assert_eq!(report.pole_zero_summaries.len(), 2);
        assert_eq!(report.pole_zero_summaries[0].root_kind, "pole");
        assert_eq!(report.pole_zero_summaries[0].root_index, 1);
        assert_eq!(report.pole_zero_summaries[0].real_rad_per_s, -1.0e3);
        assert_eq!(report.pole_zero_summaries[1].root_kind, "zero");
        assert_eq!(report.pole_zero_summaries[1].imaginary_rad_per_s, 5.0e2);
        let markdown = markdown_report(&report);
        assert!(markdown.contains("## Pole-Zero Summary"));
        assert!(markdown.contains("`pole` 1"));
        assert!(markdown.contains("frequency=1.591549e2 Hz"));
    }

    #[test]
    fn validation_report_projects_sensitivity_summary_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let summary = dir.path().join("sensitivity_summary.csv");
        fs::write(
            &summary,
            "output_expression,mode,parameter,frequency_hz,sensitivity_real,sensitivity_imaginary,sensitivity_magnitude\nV(out),dc,r1,,-2.500000000000e-4,0.000000000000e0,2.500000000000e-4\nV(out),ac,r1,1.000000000000e2,-2.500000000000e-4,1.000000000000e-6,2.500019999920e-4\n",
        )
        .unwrap();

        let report = ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            Vec::new(),
            Vec::new(),
            vec![summary.to_string_lossy().into_owned()],
            Vec::new(),
            "circuitci validate project.yaml".to_string(),
        );

        assert_eq!(report.sensitivity_summaries.len(), 2);
        assert_eq!(report.sensitivity_summaries[0].output_expression, "V(out)");
        assert_eq!(report.sensitivity_summaries[0].mode, "ac");
        assert_eq!(report.sensitivity_summaries[0].frequency_hz, Some(100.0));
        assert_eq!(report.sensitivity_summaries[1].mode, "dc");
        assert_eq!(report.sensitivity_summaries[1].frequency_hz, None);
        assert_eq!(
            report.sensitivity_summaries[1].sensitivity_magnitude,
            2.5e-4
        );
        let markdown = markdown_report(&report);
        assert!(markdown.contains("## Sensitivity Summary"));
        assert!(markdown.contains("`V(out)` `dc` `r1`"));
        assert!(markdown.contains("magnitude=2.500000e-4"));
    }

    #[test]
    fn validation_report_projects_transfer_function_summary_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let summary = dir.path().join("transfer_function_summary.csv");
        fs::write(
            &summary,
            "output_expression,input_source,transfer_function_gain,input_resistance_ohm,output_resistance_ohm\nV(out),V1,5.000000000000e-1,2.000000000000e3,5.000000000000e2\n",
        )
        .unwrap();

        let report = ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            Vec::new(),
            Vec::new(),
            vec![summary.to_string_lossy().into_owned()],
            Vec::new(),
            "circuitci validate project.yaml".to_string(),
        );

        assert_eq!(report.transfer_function_summaries.len(), 1);
        assert_eq!(
            report.transfer_function_summaries[0].output_expression,
            "V(out)"
        );
        assert_eq!(report.transfer_function_summaries[0].input_source, "V1");
        assert_eq!(
            report.transfer_function_summaries[0].transfer_function_gain,
            0.5
        );
        assert_eq!(
            report.transfer_function_summaries[0].input_resistance_ohm,
            2000.0
        );
        assert_eq!(
            report.transfer_function_summaries[0].output_resistance_ohm,
            500.0
        );
        let markdown = markdown_report(&report);
        assert!(markdown.contains("## Transfer Function Summary"));
        assert!(markdown.contains("`V(out)` from `V1`"));
        assert!(markdown.contains("gain=5.000000e-1"));
    }

    #[test]
    fn validation_report_projects_s_parameter_summary_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let summary = dir.path().join("s_parameter_summary.csv");
        fs::write(
            &summary,
            "parameter,row_count,min_frequency_hz,max_frequency_hz,min_mag_db,max_mag_db,min_mag_linear,max_mag_linear,min_return_loss_db,max_return_loss_db,min_insertion_loss_db,max_insertion_loss_db,min_vswr,max_vswr,min_mismatch_loss_db,max_mismatch_loss_db,min_group_delay_s,max_group_delay_s,min_impedance_real_ohm,max_impedance_real_ohm,min_impedance_imag_ohm,max_impedance_imag_ohm,min_impedance_magnitude_ohm,max_impedance_magnitude_ohm\ns11,2,1.000000000000e6,1.000000000000e9,-1.397940008672e1,-6.020599913280e0,2.000000000000e-1,5.000000000000e-1,6.020599913280e0,1.397940008672e1,,,1.500000000000e0,3.000000000000e0,1.774277346343e-1,1.249387366083e0,0.000000000000e0,0.000000000000e0,7.500000000000e1,1.500000000000e2,0.000000000000e0,0.000000000000e0,7.500000000000e1,1.500000000000e2\ns21,2,1.000000000000e6,1.000000000000e9,3.521825181114e0,6.020599913280e0,1.500000000000e0,2.000000000000e0,,,-6.020599913280e0,-3.521825181114e0,,,,,1.000000000000e-9,2.000000000000e-9,,,,,,\n",
        )
        .unwrap();

        let report = ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            Vec::new(),
            Vec::new(),
            vec![summary.to_string_lossy().into_owned()],
            Vec::new(),
            "circuitci validate project.yaml".to_string(),
        );

        assert_eq!(report.s_parameter_summaries.len(), 2);
        assert_eq!(report.s_parameter_summaries[0].parameter, "s11");
        assert_eq!(
            report.s_parameter_summaries[0].min_return_loss_db,
            Some(6.02059991328)
        );
        assert_eq!(report.s_parameter_summaries[0].max_vswr, Some(3.0));
        assert_eq!(
            report.s_parameter_summaries[0].max_mismatch_loss_db,
            Some(1.249387366083)
        );
        assert_eq!(report.s_parameter_summaries[0].min_group_delay_s, Some(0.0));
        assert_eq!(
            report.s_parameter_summaries[0].max_impedance_magnitude_ohm,
            Some(150.0)
        );
        assert_eq!(report.s_parameter_summaries[1].parameter, "s21");
        assert_eq!(report.s_parameter_summaries[1].min_return_loss_db, None);
        assert_eq!(
            report.s_parameter_summaries[1].max_insertion_loss_db,
            Some(-3.521825181114)
        );
        let markdown = markdown_report(&report);
        assert!(markdown.contains("## S-Parameter Summary"));
        assert!(markdown.contains("`s11`"));
        assert!(markdown.contains("vswr=1.500000e0..3.000000e0"));
        assert!(markdown.contains("mismatch_loss_db=1.774277e-1..1.249387e0"));
        assert!(markdown.contains("group_delay_s=1.000000e-9..2.000000e-9"));
        assert!(markdown.contains("impedance_magnitude_ohm=7.500000e1..1.500000e2"));
    }
}
