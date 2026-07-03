use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct BundleInstallPackageMetadata {
    pub(crate) model_package_name: Option<String>,
    pub(crate) model_package_version: Option<String>,
    pub(crate) model_package_artifact_id: String,
    pub(crate) model_package_lock_path: String,
    pub(crate) model_package_lock_sha256: String,
    pub(crate) model_package_registry_path: String,
    pub(crate) model_package_registry_sha256: String,
    pub(crate) model_package_registry_entry: String,
    pub(crate) runtime_artifact_paths: BTreeSet<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct BundleInstallReport {
    schema_version: String,
    result: String,
    package: BundleInstallPackage,
    scenario_import: Option<BundleInstallScenarioImport>,
    artifacts: Vec<BundleInstallArtifact>,
}

#[derive(Debug, Deserialize)]
struct BundleInstallPackage {
    name: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BundleInstallScenarioImport {
    model_package_registry_path: String,
    model_package_registry_sha256: String,
    model_package_registry_entry: String,
    model_package_lock_path: String,
    model_package_lock_sha256: String,
    model_package_artifact_id: String,
}

#[derive(Debug, Deserialize)]
struct BundleInstallArtifact {
    id: String,
    path: Option<String>,
}

pub(crate) fn load_bundle_install_package_metadata(
    report_path: &Path,
) -> Result<BundleInstallPackageMetadata> {
    let report = load_bundle_install_report(report_path)?;
    if report.schema_version != "circuitci.model_package_bundle_install.v1" {
        bail!(
            "Bundle install report {} has unsupported schema_version {}.",
            report_path.display(),
            report.schema_version
        );
    }
    if report.result != "pass" {
        bail!(
            "Bundle install report {} result is {}; refusing to import failed install pins.",
            report_path.display(),
            report.result
        );
    }
    let Some(import) = &report.scenario_import else {
        bail!(
            "Bundle install report {} does not contain scenario_import pins.",
            report_path.display()
        );
    };
    let mut runtime_artifact_paths = BTreeSet::new();
    for artifact in &report.artifacts {
        if artifact.id == import.model_package_artifact_id
            && let Some(path) = artifact.path.as_deref()
        {
            runtime_artifact_paths.insert(resolve_bundle_install_report_path(report_path, path));
        }
    }
    Ok(BundleInstallPackageMetadata {
        model_package_name: report.package.name,
        model_package_version: report.package.version,
        model_package_artifact_id: import.model_package_artifact_id.clone(),
        model_package_lock_path: resolve_bundle_install_report_path(
            report_path,
            &import.model_package_lock_path,
        )
        .to_string_lossy()
        .replace('\\', "/"),
        model_package_lock_sha256: import.model_package_lock_sha256.clone(),
        model_package_registry_path: resolve_bundle_install_report_path(
            report_path,
            &import.model_package_registry_path,
        )
        .to_string_lossy()
        .replace('\\', "/"),
        model_package_registry_sha256: import.model_package_registry_sha256.clone(),
        model_package_registry_entry: import.model_package_registry_entry.clone(),
        runtime_artifact_paths,
    })
}

fn load_bundle_install_report(path: &Path) -> Result<BundleInstallReport> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read bundle install report {}.", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse bundle install report {}.", path.display()))
}

fn resolve_bundle_install_report_path(report_path: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        report_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    };
    let normalized = normalize_path(&resolved);
    std::fs::canonicalize(&normalized).unwrap_or(normalized)
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
