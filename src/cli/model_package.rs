use crate::suite::ValidationBundleImportRequest;
use anyhow::{Context, Result, bail};
use std::path::PathBuf;

pub(super) fn run_verify_model_package(
    lock: PathBuf,
    registry: Option<PathBuf>,
    registry_entry: Option<String>,
    output: PathBuf,
) -> Result<()> {
    let report = crate::model_package::verify_model_package(
        &crate::model_package::ModelPackageVerifyOptions {
            lock: lock.clone(),
            registry,
            registry_entry,
            output: output.clone(),
        },
    )?;
    crate::model_package::write_model_package_verification_report(&report, &output)?;
    println!(
        "CircuitCI model package verification {}: artifacts={} findings={} -> {}",
        report.result,
        report.artifacts.len(),
        report.findings.len(),
        output.display()
    );
    if report.result != "pass" {
        bail!(
            "Model package verification failed for {}; see {}",
            lock.display(),
            output.display()
        );
    }
    Ok(())
}

pub(super) fn run_verify_model_package_bundle(bundle: PathBuf, output: PathBuf) -> Result<()> {
    let report = crate::model_package_bundle::verify_model_package_bundle(
        &crate::model_package_bundle::ModelPackageBundleVerifyOptions {
            bundle: bundle.clone(),
            output: output.clone(),
        },
    )?;
    crate::model_package_bundle::write_model_package_bundle_verification_report(&report, &output)?;
    println!(
        "CircuitCI model package bundle verification {}: artifacts={} conformance_checks={} findings={} -> {}",
        report.result,
        report.artifacts.len(),
        report.conformance_checks.len(),
        report.findings.len(),
        output.display()
    );
    if report.result != "pass" {
        bail!(
            "Model package bundle verification failed for {}; see {}",
            bundle.display(),
            output.display()
        );
    }
    Ok(())
}

pub(super) fn run_install_model_package_bundle(
    bundle: PathBuf,
    install_dir: PathBuf,
    registry_output: Option<PathBuf>,
    registry_entry: Option<String>,
    registry_artifact_id: Option<String>,
    output: PathBuf,
) -> Result<()> {
    let report = crate::model_package_bundle::install_model_package_bundle(
        &crate::model_package_bundle::ModelPackageBundleInstallOptions {
            bundle: bundle.clone(),
            install_dir,
            registry_output,
            registry_entry,
            registry_artifact_id,
            output: output.clone(),
        },
    )?;
    crate::model_package_bundle::write_model_package_bundle_install_report(&report, &output)?;
    println!(
        "CircuitCI installed model package bundle {}: artifacts={} conformance_checks={} findings={} -> {}",
        report.install_dir,
        report.artifacts.len(),
        report.conformance_checks.len(),
        report.findings.len(),
        output.display()
    );
    if let Some(import) = &report.scenario_import {
        println!(
            "CircuitCI model package registry pin path={} sha256={} entry={} artifact={}",
            import.model_package_registry_path,
            import.model_package_registry_sha256,
            import.model_package_registry_entry,
            import.model_package_artifact_id
        );
    }
    if report.result != "pass" {
        bail!(
            "Model package bundle install failed for {}; see {}",
            bundle.display(),
            output.display()
        );
    }
    Ok(())
}

pub(super) fn run_import_model_package_bundle(
    options: crate::model_package_bundle::ModelPackageBundleImportOptions,
) -> Result<()> {
    let bundle = options.bundle.clone();
    let output = options.output.clone();
    let report = crate::model_package_bundle::import_model_package_bundle(&options)?;
    crate::model_package_bundle::write_model_package_bundle_import_report(&report, &output)?;
    println!(
        "CircuitCI imported model package bundle {}: result={} artifacts={} conformance_checks={} repair_applied={} repair_blocked={} -> {}",
        report.bundle_path,
        report.result,
        report.summary.bundle_artifacts,
        report.summary.conformance_checks,
        report.summary.repair_applied,
        report.summary.repair_blocked,
        output.join("model_package_bundle_import.json").display()
    );
    if report.result != "pass" {
        bail!(
            "Model package bundle import failed for {}; see {}",
            bundle.display(),
            output.join("model_package_bundle_import.json").display()
        );
    }
    Ok(())
}

pub(super) fn run_merge_model_package_registry(
    base: Option<PathBuf>,
    inputs: Vec<PathBuf>,
    output: PathBuf,
) -> Result<()> {
    let summary = crate::model_package::merge_model_package_registries(
        &crate::model_package::ModelPackageRegistryMergeOptions {
            base,
            inputs,
            output,
        },
    )?;
    println!(
        "CircuitCI merged model package registry {} sha256={} entries={} input_registries={} deduplicated_entries={}",
        summary.registry_path,
        summary.registry_sha256,
        summary.entries,
        summary.input_registries,
        summary.deduplicated_entries
    );
    Ok(())
}

#[derive(Debug)]
pub(super) struct CliModelPackageExportArgs {
    pub(super) package_name: String,
    pub(super) package_version: String,
    pub(super) package_artifacts: Vec<String>,
    pub(super) artifact_id: Option<String>,
    pub(super) artifact: Option<PathBuf>,
    pub(super) artifact_format: Option<String>,
    pub(super) compiler: Option<String>,
    pub(super) output: PathBuf,
    pub(super) registry_output: Option<PathBuf>,
    pub(super) registry_entry: Option<String>,
    pub(super) registry_artifact_id: Option<String>,
}

pub(super) fn run_export_model_package(args: CliModelPackageExportArgs) -> Result<()> {
    let artifacts = export_model_package_artifacts(&args)?;
    let summary = crate::model_package::export_model_package_lock(
        &crate::model_package::ModelPackageExportOptions {
            package_name: args.package_name,
            package_version: args.package_version,
            artifacts,
            output: args.output,
            registry_output: args.registry_output,
            registry_entry: args.registry_entry,
            registry_artifact_id: args.registry_artifact_id,
        },
    )?;
    println!(
        "CircuitCI exported model package lock {} sha256={} artifact={} artifact_sha256={}",
        summary.lock_path, summary.lock_sha256, summary.artifact_path, summary.artifact_sha256
    );
    if let (Some(registry_path), Some(registry_sha256)) = (
        summary.registry_path.as_deref(),
        summary.registry_sha256.as_deref(),
    ) {
        println!(
            "CircuitCI exported model package registry {registry_path} sha256={registry_sha256}"
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_export_model_conformance_report(
    report: PathBuf,
    package_name: String,
    package_version: String,
    artifact_id: String,
    runtime_artifact: PathBuf,
    check_name: String,
    analysis: String,
    solver: Option<String>,
    output: PathBuf,
) -> Result<()> {
    let summary = crate::model_package::export_model_conformance_report(
        &crate::model_package::ModelConformanceReportExportOptions {
            validation_report: report,
            package_name,
            package_version,
            artifact_id,
            runtime_artifact,
            check_name,
            analysis,
            solver,
            output,
        },
    )?;
    println!(
        "CircuitCI exported model conformance report {} sha256={} result={} artifact={} artifact_sha256={}",
        summary.output,
        summary.sha256,
        summary.result,
        summary.artifact_id,
        summary.runtime_artifact_sha256
    );
    Ok(())
}

pub(super) fn run_export_model_package_bundle(
    lock: PathBuf,
    registry: Option<PathBuf>,
    registry_entry: Option<String>,
    output: PathBuf,
) -> Result<()> {
    let summary = crate::model_package::export_model_package_bundle(
        &crate::model_package::ModelPackageBundleExportOptions {
            lock,
            registry,
            registry_entry,
            output,
        },
    )?;
    println!(
        "CircuitCI exported model package bundle {} manifest={} manifest_sha256={} artifacts={} conformance_checks={}",
        summary.output,
        summary.manifest_path,
        summary.manifest_sha256,
        summary.artifact_count,
        summary.conformance_check_count
    );
    Ok(())
}

fn export_model_package_artifacts(
    args: &CliModelPackageExportArgs,
) -> Result<Vec<crate::model_package::ModelPackageExportArtifactInput>> {
    if !args.package_artifacts.is_empty() {
        if args.artifact_id.is_some() || args.artifact.is_some() || args.artifact_format.is_some() {
            bail!(
                "--package-artifact cannot be combined with --artifact-id, --artifact, or --artifact-format."
            );
        }
        if args.compiler.is_some() {
            bail!(
                "--compiler cannot be combined with --package-artifact; include compiler= in each artifact spec."
            );
        }
        return args
            .package_artifacts
            .iter()
            .map(|spec| parse_model_package_artifact_spec(spec))
            .collect();
    }
    Ok(vec![
        crate::model_package::ModelPackageExportArtifactInput {
            id: args
                .artifact_id
                .clone()
                .context("--artifact-id is required when --package-artifact is not supplied.")?,
            artifact: args
                .artifact
                .clone()
                .context("--artifact is required when --package-artifact is not supplied.")?,
            artifact_format: args.artifact_format.clone().context(
                "--artifact-format is required when --package-artifact is not supplied.",
            )?,
            compiler: args.compiler.clone(),
        },
    ])
}

fn parse_model_package_artifact_spec(
    spec: &str,
) -> Result<crate::model_package::ModelPackageExportArtifactInput> {
    let mut id = None;
    let mut artifact = None;
    let mut artifact_format = None;
    let mut compiler = None;
    let mut seen = std::collections::BTreeSet::new();
    for piece in spec.split(',') {
        let (raw_key, raw_value) = piece.split_once('=').with_context(|| {
            format!("Invalid --package-artifact segment {piece:?}; expected key=value.")
        })?;
        let key = raw_key.trim();
        let value = raw_value.trim();
        if key.is_empty() || value.is_empty() {
            bail!(
                "Invalid --package-artifact segment {piece:?}; keys and values must be non-empty."
            );
        }
        let canonical_key = match key {
            "id" => "id",
            "path" | "artifact" => "path",
            "artifact_format" | "format" => "artifact_format",
            "compiler" => "compiler",
            _ => bail!("Unknown --package-artifact key {key:?}."),
        };
        if !seen.insert(canonical_key) {
            bail!("Duplicate --package-artifact key {canonical_key:?}.");
        }
        match canonical_key {
            "id" => id = Some(value.to_string()),
            "path" => artifact = Some(PathBuf::from(value)),
            "artifact_format" => artifact_format = Some(value.to_string()),
            "compiler" => compiler = Some(value.to_string()),
            _ => unreachable!(),
        }
    }
    Ok(crate::model_package::ModelPackageExportArtifactInput {
        id: id.context("--package-artifact requires id=<artifact-id>.")?,
        artifact: artifact.context("--package-artifact requires path=<artifact-path>.")?,
        artifact_format: artifact_format
            .context("--package-artifact requires artifact_format=<format>.")?,
        compiler,
    })
}

pub(super) fn parse_validation_bundle_import_spec(
    spec: &str,
) -> Result<ValidationBundleImportRequest> {
    let mut id = None;
    let mut bundle = None;
    let mut install_dir = None;
    let mut registry_output = None;
    let mut registry_entry = None;
    let mut registry_artifact_id = None;
    let mut max_runtime_ms = None;
    let mut seen = std::collections::BTreeSet::new();
    for piece in spec.split(',') {
        let Some((raw_key, raw_value)) = piece.split_once('=') else {
            bail!("Invalid --model-package-bundle-import segment {piece:?}; expected key=value.");
        };
        let key = raw_key.trim();
        let value = raw_value.trim();
        if key.is_empty() || value.is_empty() {
            bail!(
                "Invalid --model-package-bundle-import segment {piece:?}; keys and values must be non-empty."
            );
        }
        let canonical_key = match key {
            "id" => "id",
            "bundle" | "bundle_path" | "path" => "bundle",
            "install_dir" | "install" => "install_dir",
            "registry_output" | "registry" => "registry_output",
            "registry_entry" | "entry" => "registry_entry",
            "registry_artifact_id" | "artifact_id" => "registry_artifact_id",
            "max_runtime_ms" | "runtime_ms" => "max_runtime_ms",
            _ => bail!("Unknown --model-package-bundle-import key {key:?}."),
        };
        if !seen.insert(canonical_key) {
            bail!("Duplicate --model-package-bundle-import key {canonical_key:?}.");
        }
        match canonical_key {
            "id" => id = Some(value.to_string()),
            "bundle" => bundle = Some(PathBuf::from(value)),
            "install_dir" => install_dir = Some(PathBuf::from(value)),
            "registry_output" => registry_output = Some(PathBuf::from(value)),
            "registry_entry" => registry_entry = Some(value.to_string()),
            "registry_artifact_id" => registry_artifact_id = Some(value.to_string()),
            "max_runtime_ms" => {
                max_runtime_ms = Some(value.parse::<u64>().with_context(|| {
                    format!("Invalid --model-package-bundle-import max_runtime_ms={value:?}.")
                })?)
            }
            _ => unreachable!(),
        }
    }
    Ok(ValidationBundleImportRequest {
        id,
        bundle: bundle.context("--model-package-bundle-import requires bundle=<path>.")?,
        install_dir: install_dir
            .context("--model-package-bundle-import requires install_dir=<path>.")?,
        registry_output,
        registry_entry,
        registry_artifact_id,
        max_runtime_ms,
    })
}
