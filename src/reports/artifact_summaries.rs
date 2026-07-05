use super::{
    ModelFileProvenance, ModelPackageBundleImportSummary, ModelPackageBundleInstallSummary,
    ModelPackageBundleVerificationSummary, ModelPackageConformanceCheck, YamlRepairSummary,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(super) fn collect_model_file_provenance(artifacts: &[String]) -> Vec<ModelFileProvenance> {
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

pub(super) fn collect_model_package_conformance_checks(
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

pub(super) fn collect_model_package_bundle_verifications(
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

pub(super) fn collect_model_package_bundle_installs(
    artifacts: &[String],
    profile: &str,
    validation_command: &str,
    project_path: Option<&Path>,
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
            repair_yaml_command: bundle_install_repair_command(
                validation_command,
                profile,
                artifact,
                project_path,
            ),
        });
    }
    records.into_iter().collect()
}

pub(super) fn collect_model_package_bundle_imports(
    artifacts: &[String],
) -> Vec<ModelPackageBundleImportSummary> {
    let mut records = BTreeSet::new();
    for artifact in artifacts {
        let Some(report) = read_json_artifact(artifact) else {
            continue;
        };
        if string_at(&report, &["schema_version"]) != "circuitci.model_package_bundle_import.v1" {
            continue;
        }
        records.insert(ModelPackageBundleImportSummary {
            report: artifact.clone(),
            result: string_at(&report, &["result"]),
            bundle_path: string_at(&report, &["bundle_path"]),
            project: string_at(&report, &["project"]),
            profile: string_at(&report, &["profile"]),
            install_dir: string_at(&report, &["install_dir"]),
            package_name: optional_string_at(&report, &["package", "name"]),
            package_version: optional_string_at(&report, &["package", "version"]),
            bundle_install_report: optional_string_at(&report, &["bundle_install_report"]),
            package_verification_report: optional_string_at(
                &report,
                &["package_verification_report"],
            ),
            yaml_repair_report: optional_string_at(&report, &["yaml_repair_report"]),
            repaired_project: optional_string_at(&report, &["repaired_project"]),
            repaired_validation_report: optional_string_at(
                &report,
                &["repaired_validation_report"],
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
            bundle_artifacts: usize_at(&report, &["summary", "bundle_artifacts"]),
            conformance_checks: usize_at(&report, &["summary", "conformance_checks"]),
            package_findings: usize_at(&report, &["summary", "package_findings"]),
            repair_applied: usize_at(&report, &["summary", "repair_applied"]),
            repair_blocked: usize_at(&report, &["summary", "repair_blocked"]),
            repair_new_criticals: usize_at(&report, &["summary", "repair_new_criticals"]),
        });
    }
    records.into_iter().collect()
}

pub fn collect_yaml_repair_summaries(artifacts: &[String]) -> Vec<YamlRepairSummary> {
    let mut records = BTreeSet::new();
    for artifact in artifacts {
        let Some(report) = read_json_artifact(artifact) else {
            continue;
        };
        if string_at(&report, &["schema_version"]) != "circuitci.repair.v1" {
            continue;
        }
        records.insert(YamlRepairSummary {
            report: artifact.clone(),
            result: string_at(&report, &["result"]),
            finding: string_at(&report, &["finding"]),
            mode: string_at(&report, &["mode"]),
            original_project: string_at(&report, &["original_project"]),
            repaired_project: optional_string_at(&report, &["repaired_project"]),
            original_report: string_at(&report, &["original_report"]),
            repaired_report: optional_string_at(&report, &["repaired_report"]),
            proposed: usize_at(&report, &["summary", "proposed"]),
            selected: usize_at(&report, &["summary", "selected"]),
            applied: usize_at(&report, &["summary", "applied"]),
            blocked: usize_at(&report, &["summary", "blocked"]),
            skipped: usize_at(&report, &["summary", "skipped"]),
            original_matching_findings: usize_at(
                &report,
                &["summary", "original_matching_findings"],
            ),
            repaired_matching_findings: usize_at(
                &report,
                &["summary", "repaired_matching_findings"],
            ),
            original_matching_criticals: usize_at(
                &report,
                &["summary", "original_matching_criticals"],
            ),
            repaired_matching_criticals: usize_at(
                &report,
                &["summary", "repaired_matching_criticals"],
            ),
            new_criticals: usize_at(&report, &["summary", "new_criticals"]),
            original_finding_removed: bool_at(&report, &["proof", "original_finding_removed"]),
            no_new_criticals: bool_at(&report, &["proof", "no_new_criticals"]),
            reason_codes: string_array_at(&report, &["reason_codes"]),
        });
    }
    records.into_iter().collect()
}

fn bundle_install_repair_command(
    validation_command: &str,
    profile: &str,
    bundle_install_report: &str,
    project_path: Option<&Path>,
) -> Option<String> {
    let project_path = project_path
        .map(|path| path.to_string_lossy().into_owned())
        .or_else(|| validation_command_project_path(validation_command))?;
    Some(format!(
        "circuitci repair-yaml {} --profile {} --output out/repair_bundle_import --finding bundle-install-package-metadata --bundle-install-report {}",
        shell_arg(&project_path),
        shell_arg(profile),
        shell_arg(bundle_install_report)
    ))
}

fn validation_command_project_path(command: &str) -> Option<String> {
    let mut words = command.split_whitespace();
    while let Some(word) = words.next() {
        if word == "validate" {
            return words.next().map(ToOwned::to_owned);
        }
    }
    None
}

fn shell_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':' | '='))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
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

fn usize_at(value: &Value, path: &[&str]) -> usize {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(*key) else {
            return 0;
        };
        current = next;
    }
    current
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or_default()
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
