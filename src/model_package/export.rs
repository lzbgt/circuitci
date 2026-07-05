use super::*;

pub fn export_model_package_lock(
    options: &ModelPackageExportOptions,
) -> Result<ModelPackageExportSummary> {
    validate_export_options(options)?;
    let lock_parent = options.output.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(lock_parent).with_context(|| {
        format!(
            "Failed to create model package output directory {}",
            lock_parent.display()
        )
    })?;
    let artifacts = options
        .artifacts
        .iter()
        .map(|artifact| {
            Ok(ModelPackageExportArtifactSummary {
                id: artifact.id.clone(),
                artifact_path: lock_relative_path(lock_parent, &artifact.artifact)?,
                artifact_sha256: file_sha256_hex(&artifact.artifact)?,
                artifact_format: artifact.artifact_format.clone(),
                compiler: artifact.compiler.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let primary = primary_registry_artifact(options, &artifacts)?;
    let lock_text = render_lock_document(options, &artifacts);
    std::fs::write(&options.output, lock_text.as_bytes()).with_context(|| {
        format!(
            "Failed to write model package lock {}",
            options.output.display()
        )
    })?;
    let lock_sha = file_sha256_hex(&options.output)?;
    let mut registry_path = None;
    let mut registry_sha = None;
    if let Some(output) = &options.registry_output {
        let registry_parent = output.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(registry_parent).with_context(|| {
            format!(
                "Failed to create model package registry directory {}",
                registry_parent.display()
            )
        })?;
        let entry = options
            .registry_entry
            .as_deref()
            .unwrap_or(primary.id.as_str());
        let lock_path = lock_relative_path(registry_parent, &options.output)?;
        let registry_text =
            render_registry_document(options, entry, &primary.id, &lock_path, &lock_sha);
        std::fs::write(output, registry_text.as_bytes()).with_context(|| {
            format!(
                "Failed to write model package registry {}",
                output.display()
            )
        })?;
        registry_sha = Some(file_sha256_hex(output)?);
        registry_path = Some(output.to_string_lossy().to_string());
    }
    Ok(ModelPackageExportSummary {
        lock_path: options.output.to_string_lossy().to_string(),
        lock_sha256: lock_sha,
        artifact_id: primary.id.clone(),
        artifact_path: primary.artifact_path.clone(),
        artifact_sha256: primary.artifact_sha256.clone(),
        artifacts,
        registry_path,
        registry_sha256: registry_sha,
        registry_entry: options.registry_entry.clone(),
        registry_artifact_id: options.registry_artifact_id.clone(),
    })
}

pub fn merge_model_package_registries(
    options: &ModelPackageRegistryMergeOptions,
) -> Result<ModelPackageRegistryMergeSummary> {
    validate_registry_merge_options(options)?;
    let output_parent = options.output.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(output_parent).with_context(|| {
        format!(
            "Failed to create model package registry output directory {}",
            output_parent.display()
        )
    })?;
    let mut entries = BTreeMap::new();
    let mut deduplicated_entries = 0usize;
    let mut registry_paths = Vec::new();
    if let Some(base) = &options.base {
        registry_paths.push(base.clone());
    }
    registry_paths.extend(options.inputs.iter().cloned());
    for registry in &registry_paths {
        for entry in read_registry_entries_for_merge(registry, output_parent)? {
            match entries.get(&entry.id) {
                Some(existing) if existing == &entry => {
                    deduplicated_entries += 1;
                }
                Some(_) => {
                    anyhow::bail!(
                        "Model package registry entry {} conflicts with an existing entry.",
                        entry.id
                    );
                }
                None => {
                    entries.insert(entry.id.clone(), entry);
                }
            }
        }
    }
    let registry_text = render_registry_entries_document(entries.values());
    std::fs::write(&options.output, registry_text.as_bytes()).with_context(|| {
        format!(
            "Failed to write model package registry {}",
            options.output.display()
        )
    })?;
    Ok(ModelPackageRegistryMergeSummary {
        registry_path: options.output.to_string_lossy().to_string(),
        registry_sha256: file_sha256_hex(&options.output)?,
        entries: entries.len(),
        input_registries: registry_paths.len(),
        deduplicated_entries,
    })
}

pub fn export_model_package_bundle(
    options: &ModelPackageBundleExportOptions,
) -> Result<ModelPackageBundleExportSummary> {
    validate_bundle_export_options(options)?;
    let source_report = verify_model_package(&ModelPackageVerifyOptions {
        lock: options.lock.clone(),
        registry: options.registry.clone(),
        registry_entry: options.registry_entry.clone(),
        output: options
            .output
            .join("source_model_package_verification.json"),
    })?;
    if source_report.result != "pass" {
        anyhow::bail!(
            "Source model package verification failed for {}; export a bundle only from a passing package.",
            options.lock.display()
        );
    }
    let lock_text = std::fs::read_to_string(&options.lock).with_context(|| {
        format!(
            "Unable to read model package lock {}",
            options.lock.display()
        )
    })?;
    let mut parse_findings = Vec::new();
    let mut lock = parse_model_package_document(
        &lock_text,
        &mut parse_findings,
        "MODEL_PACKAGE_LOCK_INVALID",
    )
    .with_context(|| {
        format!(
            "Model package lock {} is not valid JSON or YAML.",
            options.lock.display()
        )
    })?;
    let package_name = string_field(&lock, &["package", "name"])
        .or_else(|| string_field(&lock, &["package_name"]))
        .or_else(|| string_field(&lock, &["name"]))
        .context("Model package lock must declare package.name.")?;
    let package_version = string_field(&lock, &["package", "version"])
        .or_else(|| string_field(&lock, &["package_version"]))
        .or_else(|| string_field(&lock, &["version"]))
        .context("Model package lock must declare package.version.")?;
    std::fs::create_dir_all(options.output.join("artifacts")).with_context(|| {
        format!(
            "Failed to create model package bundle directory {}",
            options.output.display()
        )
    })?;
    let source_parent = options.lock.parent().unwrap_or_else(|| Path::new("."));
    let mut copied_artifacts = Vec::new();
    let mut used_paths = BTreeSet::new();
    let artifacts = lock_artifacts_mut(&mut lock)?;
    for artifact in artifacts {
        let id = required_value_string(artifact, &["id"], "model package artifact id")?;
        let source_relative =
            required_value_string(artifact, &["path"], "model package artifact path")?;
        let source_path = source_parent.join(&source_relative);
        let file_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| {
                format!("Model package artifact {source_relative} has no file name.")
            })?;
        let mut bundled_relative = format!(
            "artifacts/{}__{}",
            sanitize_bundle_name(&id),
            sanitize_bundle_name(file_name)
        );
        let mut duplicate_index = 2usize;
        while !used_paths.insert(bundled_relative.clone()) {
            bundled_relative = format!(
                "artifacts/{}__{}__{}",
                sanitize_bundle_name(&id),
                duplicate_index,
                sanitize_bundle_name(file_name)
            );
            duplicate_index += 1;
        }
        let destination = options.output.join(&bundled_relative);
        std::fs::copy(&source_path, &destination).with_context(|| {
            format!(
                "Failed to copy model package artifact {} to {}",
                source_path.display(),
                destination.display()
            )
        })?;
        set_object_string(artifact, "path", &bundled_relative)?;
        copied_artifacts.push(ModelPackageExportArtifactSummary {
            id,
            artifact_path: bundled_relative,
            artifact_sha256: file_sha256_hex(&destination)?,
            artifact_format: string_field(artifact, &["artifact_format"]).unwrap_or_default(),
            compiler: string_field(artifact, &["compiler"]),
        });
    }
    set_object_string(&mut lock, "schema_version", MODEL_PACKAGE_LOCK_SCHEMA)?;
    let lock_path = options.output.join("package.lock.json");
    write_json_value(&lock_path, &lock)?;
    let lock_sha = file_sha256_hex(&lock_path)?;
    let (registry_path, registry_sha, registry_entry_id) =
        bundled_registry(options, &package_name, &package_version, &lock_sha)?;
    let verification_path = options.output.join("model_package_verification.json");
    let bundled_report = verify_model_package(&ModelPackageVerifyOptions {
        lock: lock_path.clone(),
        registry: registry_path.clone(),
        registry_entry: registry_entry_id,
        output: verification_path.clone(),
    })?;
    write_model_package_verification_report(&bundled_report, &verification_path)?;
    let readme_path = options.output.join("README.md");
    std::fs::write(
        &readme_path,
        model_package_bundle_readme(
            &package_name,
            &package_version,
            &copied_artifacts,
            &bundled_report,
        ),
    )
    .with_context(|| {
        format!(
            "Failed to write model package bundle README {}",
            readme_path.display()
        )
    })?;
    let manifest_path = options.output.join("model_package_bundle_manifest.json");
    let manifest = serde_json::json!({
        "schema_version": MODEL_PACKAGE_BUNDLE_SCHEMA,
        "package": {
            "name": package_name,
            "version": package_version,
        },
        "lock_path": "package.lock.json",
        "lock_sha256": lock_sha,
        "registry_path": registry_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned),
        "registry_sha256": registry_sha,
        "verification_report": "model_package_verification.json",
        "verification_markdown": "model_package_verification.md",
        "readme": "README.md",
        "artifacts": copied_artifacts.clone(),
        "conformance_checks": bundled_report.conformance_checks.clone(),
    });
    write_json_value(&manifest_path, &manifest)?;
    Ok(ModelPackageBundleExportSummary {
        output: options.output.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        manifest_sha256: file_sha256_hex(&manifest_path)?,
        lock_path: lock_path.to_string_lossy().to_string(),
        lock_sha256: lock_sha,
        registry_path: registry_path.map(|path| path.to_string_lossy().to_string()),
        registry_sha256: registry_sha,
        verification_report: verification_path.to_string_lossy().to_string(),
        artifact_count: copied_artifacts.len(),
        conformance_check_count: bundled_report.conformance_checks.len(),
    })
}

pub fn export_model_conformance_report(
    options: &ModelConformanceReportExportOptions,
) -> Result<ModelConformanceReportExportSummary> {
    validate_conformance_export_options(options)?;
    let runtime_sha = file_sha256_hex(&options.runtime_artifact)?;
    let report_text = std::fs::read_to_string(&options.validation_report).with_context(|| {
        format!(
            "Unable to read validation report {}",
            options.validation_report.display()
        )
    })?;
    let report: Value = serde_json::from_str(&report_text).with_context(|| {
        format!(
            "Validation report {} is not valid JSON.",
            options.validation_report.display()
        )
    })?;
    let report_result = string_field(&report, &["result"])
        .with_context(|| "Validation report must declare result.".to_string())?;
    let critical_failures = report
        .get("failures")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let result = if report_result == "pass" && critical_failures == 0 {
        "pass"
    } else {
        "fail"
    };
    let mut check = serde_json::Map::new();
    check.insert(
        "name".to_string(),
        Value::String(options.check_name.clone()),
    );
    check.insert(
        "analysis".to_string(),
        Value::String(options.analysis.clone()),
    );
    if let Some(solver) = options.solver.as_deref() {
        check.insert("solver".to_string(), Value::String(solver.to_string()));
    }
    check.insert("result".to_string(), Value::String(result.to_string()));
    if let Some(artifacts) = report.get("artifacts").and_then(Value::as_array) {
        let artifact_values = artifacts
            .iter()
            .filter_map(Value::as_str)
            .map(|artifact| Value::String(artifact.to_string()))
            .collect::<Vec<_>>();
        if !artifact_values.is_empty() {
            check.insert("artifacts".to_string(), Value::Array(artifact_values));
        }
    }
    let conformance = serde_json::json!({
        "schema_version": MODEL_CONFORMANCE_REPORT_SCHEMA,
        "package": {
            "name": options.package_name,
            "version": options.package_version,
        },
        "artifact_id": options.artifact_id,
        "runtime_artifact_sha256": runtime_sha,
        "result": result,
        "checks": [Value::Object(check)],
        "source": options.validation_report.to_string_lossy(),
    });
    if let Some(parent) = options.output.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create conformance report output directory {}",
                parent.display()
            )
        })?;
    }
    let mut text = serde_json::to_string_pretty(&conformance)?;
    text.push('\n');
    std::fs::write(&options.output, text).with_context(|| {
        format!(
            "Failed to write model conformance report {}",
            options.output.display()
        )
    })?;
    Ok(ModelConformanceReportExportSummary {
        output: options.output.to_string_lossy().to_string(),
        sha256: file_sha256_hex(&options.output)?,
        result: result.to_string(),
        package_name: options.package_name.clone(),
        package_version: options.package_version.clone(),
        artifact_id: options.artifact_id.clone(),
        runtime_artifact_sha256: runtime_sha,
    })
}

fn validate_conformance_export_options(
    options: &ModelConformanceReportExportOptions,
) -> Result<()> {
    for (field, value) in [
        ("package-name", options.package_name.as_str()),
        ("package-version", options.package_version.as_str()),
        ("artifact-id", options.artifact_id.as_str()),
        ("check-name", options.check_name.as_str()),
        ("analysis", options.analysis.as_str()),
    ] {
        if value.trim().is_empty() {
            anyhow::bail!("--{field} must not be empty.");
        }
    }
    if let Some(solver) = options.solver.as_deref()
        && solver.trim().is_empty()
    {
        anyhow::bail!("--solver must not be empty when supplied.");
    }
    if !options.validation_report.is_file() {
        anyhow::bail!(
            "Validation report {} is missing.",
            options.validation_report.display()
        );
    }
    if !options.runtime_artifact.is_file() {
        anyhow::bail!(
            "Runtime artifact {} is missing.",
            options.runtime_artifact.display()
        );
    }
    Ok(())
}

fn validate_registry_merge_options(options: &ModelPackageRegistryMergeOptions) -> Result<()> {
    if options.base.is_none() && options.inputs.is_empty() {
        anyhow::bail!("merge-model-package-registry requires --base or at least one --input.");
    }
    for path in options.base.iter().chain(options.inputs.iter()) {
        if !path.is_file() {
            anyhow::bail!("Model package registry {} is missing.", path.display());
        }
    }
    Ok(())
}

fn validate_bundle_export_options(options: &ModelPackageBundleExportOptions) -> Result<()> {
    if !options.lock.is_file() {
        anyhow::bail!("Model package lock {} is missing.", options.lock.display());
    }
    if options.registry.is_some() && options.registry_entry.is_none() {
        anyhow::bail!("--registry-entry is required when --registry is supplied.");
    }
    if let Some(registry) = &options.registry
        && !registry.is_file()
    {
        anyhow::bail!("Model package registry {} is missing.", registry.display());
    }
    Ok(())
}

fn lock_artifacts_mut(lock: &mut Value) -> Result<&mut Vec<Value>> {
    if lock.get("artifacts").is_some() {
        lock.get_mut("artifacts")
            .and_then(Value::as_array_mut)
            .context("Model package lock artifacts must be an array.")
    } else {
        lock.get_mut("model_artifacts")
            .and_then(Value::as_array_mut)
            .context("Model package lock must contain an artifacts array.")
    }
}

fn required_value_string(value: &Value, path: &[&str], description: &str) -> Result<String> {
    string_field(value, path).with_context(|| format!("Missing {description}."))
}

fn set_object_string(value: &mut Value, key: &str, text: &str) -> Result<()> {
    let object = value
        .as_object_mut()
        .with_context(|| format!("Expected JSON object while setting {key}."))?;
    object.insert(key.to_string(), Value::String(text.to_string()));
    Ok(())
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

fn bundled_registry(
    options: &ModelPackageBundleExportOptions,
    package_name: &str,
    package_version: &str,
    lock_sha: &str,
) -> Result<(Option<PathBuf>, Option<String>, Option<String>)> {
    let Some(registry_path) = &options.registry else {
        return Ok((None, None, None));
    };
    let entry_id = options
        .registry_entry
        .as_deref()
        .context("--registry-entry is required when --registry is supplied.")?;
    let text = std::fs::read_to_string(registry_path).with_context(|| {
        format!(
            "Unable to read model package registry {}",
            registry_path.display()
        )
    })?;
    let mut findings = Vec::new();
    let registry =
        parse_model_package_document(&text, &mut findings, "MODEL_PACKAGE_REGISTRY_INVALID")
            .with_context(|| {
                format!(
                    "Model package registry {} is not valid JSON or YAML.",
                    registry_path.display()
                )
            })?;
    let entry = model_package_registry_entry(&registry, entry_id)
        .with_context(|| format!("Model package registry does not contain entry {entry_id}."))?;
    let artifact_id = string_field(entry, &["artifact_id"])
        .or_else(|| string_field(entry, &["model_package_artifact_id"]))
        .with_context(|| {
            format!("Model package registry entry {entry_id} must declare artifact_id.")
        })?;
    let bundled_entry = ModelPackageRegistryEntry {
        id: entry_id.to_string(),
        package_name: package_name.to_string(),
        package_version: package_version.to_string(),
        artifact_id,
        lock_path: "package.lock.json".to_string(),
        lock_sha256: lock_sha.to_string(),
    };
    let output = options.output.join("compact_model_registry.json");
    std::fs::write(
        &output,
        render_registry_entries_document([&bundled_entry].into_iter()).as_bytes(),
    )
    .with_context(|| {
        format!(
            "Failed to write bundled model package registry {}",
            output.display()
        )
    })?;
    Ok((
        Some(output.clone()),
        Some(file_sha256_hex(&output)?),
        Some(entry_id.to_string()),
    ))
}

fn sanitize_bundle_name(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        sanitized.push_str("artifact");
    }
    sanitized
}

fn model_package_bundle_readme(
    package_name: &str,
    package_version: &str,
    artifacts: &[ModelPackageExportArtifactSummary],
    report: &ModelPackageVerificationReport,
) -> String {
    let mut text = String::new();
    text.push_str(&format!(
        "# CircuitCI Model Package Bundle: {package_name} {package_version}\n\n"
    ));
    text.push_str("- `package.lock.json`: rewritten lock with bundled artifact paths\n");
    text.push_str("- `model_package_verification.json`: machine-readable verification report\n");
    text.push_str("- `model_package_verification.md`: human-readable verification summary\n");
    text.push_str("- `model_package_bundle_manifest.json`: bundle manifest and hashes\n");
    text.push_str("- `artifacts/`: source/runtime/conformance artifacts\n\n");
    text.push_str("## Artifacts\n\n");
    for artifact in artifacts {
        text.push_str(&format!(
            "- `{}` [{}] `{}` `{}`\n",
            artifact.id, artifact.artifact_format, artifact.artifact_path, artifact.artifact_sha256
        ));
    }
    text.push_str("\n## Conformance Checks\n\n");
    if report.conformance_checks.is_empty() {
        text.push_str("None.\n");
    } else {
        for check in &report.conformance_checks {
            text.push_str(&format!(
                "- `{}` `{}` via `{}`: `{}` target `{}`\n",
                check.check_name.as_deref().unwrap_or(""),
                check.analysis.as_deref().unwrap_or(""),
                check.solver.as_deref().unwrap_or(""),
                check.result.as_deref().unwrap_or(""),
                check.target_artifact_id.as_deref().unwrap_or("")
            ));
        }
    }
    text
}

fn read_registry_entries_for_merge(
    registry_path: &Path,
    output_parent: &Path,
) -> Result<Vec<ModelPackageRegistryEntry>> {
    let text = std::fs::read_to_string(registry_path).with_context(|| {
        format!(
            "Unable to read model package registry {}",
            registry_path.display()
        )
    })?;
    let mut findings = Vec::new();
    let value =
        parse_model_package_document(&text, &mut findings, "MODEL_PACKAGE_REGISTRY_INVALID")
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Model package registry {} is not valid JSON or YAML.",
                    registry_path.display()
                )
            })?;
    let package_entries = value
        .get("packages")
        .or_else(|| value.get("model_packages"))
        .or_else(|| value.get("entries"))
        .and_then(Value::as_array)
        .context("Model package registry must contain a packages array.")?;
    let registry_parent = registry_path.parent().unwrap_or_else(|| Path::new("."));
    package_entries
        .iter()
        .map(|entry| {
            parse_registry_entry_for_merge(entry, registry_path, registry_parent, output_parent)
        })
        .collect()
}

fn parse_registry_entry_for_merge(
    entry: &Value,
    registry_path: &Path,
    registry_parent: &Path,
    output_parent: &Path,
) -> Result<ModelPackageRegistryEntry> {
    let id = required_registry_string(entry, &["id"], registry_path)?;
    let package_name = required_registry_string(entry, &["package", "name"], registry_path)
        .or_else(|_| required_registry_string(entry, &["package_name"], registry_path))
        .or_else(|_| required_registry_string(entry, &["name"], registry_path))?;
    let package_version = required_registry_string(entry, &["package", "version"], registry_path)
        .or_else(|_| required_registry_string(entry, &["package_version"], registry_path))
        .or_else(|_| required_registry_string(entry, &["version"], registry_path))?;
    let artifact_id =
        required_registry_string(entry, &["artifact_id"], registry_path).or_else(|_| {
            required_registry_string(entry, &["model_package_artifact_id"], registry_path)
        })?;
    let lock_path =
        required_registry_string(entry, &["lock_path"], registry_path).or_else(|_| {
            required_registry_string(entry, &["model_package_lock_path"], registry_path)
        })?;
    let lock_sha256 =
        required_registry_string(entry, &["lock_sha256"], registry_path).or_else(|_| {
            required_registry_string(entry, &["model_package_lock_sha256"], registry_path)
        })?;
    let absolute_lock_path = registry_parent.join(&lock_path);
    let output_lock_path = lock_relative_path(output_parent, &absolute_lock_path)?;
    Ok(ModelPackageRegistryEntry {
        id,
        package_name,
        package_version,
        artifact_id,
        lock_path: output_lock_path,
        lock_sha256,
    })
}

fn required_registry_string(entry: &Value, path: &[&str], registry_path: &Path) -> Result<String> {
    let value = string_field(entry, path).with_context(|| {
        format!(
            "Model package registry {} entry is missing {}.",
            registry_path.display(),
            path.join(".")
        )
    })?;
    if value.trim().is_empty() {
        anyhow::bail!(
            "Model package registry {} entry has empty {}.",
            registry_path.display(),
            path.join(".")
        );
    }
    Ok(value)
}

fn validate_export_options(options: &ModelPackageExportOptions) -> Result<()> {
    for (field, value) in [
        ("package-name", options.package_name.as_str()),
        ("package-version", options.package_version.as_str()),
    ] {
        if value.trim().is_empty() {
            anyhow::bail!("--{field} must not be empty.");
        }
    }
    if options.registry_output.is_none() && options.registry_entry.is_some() {
        anyhow::bail!("--registry-entry requires --registry-output.");
    }
    if options.registry_output.is_none() && options.registry_artifact_id.is_some() {
        anyhow::bail!("--registry-artifact-id requires --registry-output.");
    }
    if options.artifacts.is_empty() {
        anyhow::bail!("Model package export requires at least one artifact.");
    }
    let mut ids = BTreeSet::new();
    for artifact in &options.artifacts {
        for (field, value) in [
            ("artifact id", artifact.id.as_str()),
            ("artifact format", artifact.artifact_format.as_str()),
        ] {
            if value.trim().is_empty() {
                anyhow::bail!("Model package {field} must not be empty.");
            }
        }
        if let Some(compiler) = artifact.compiler.as_deref()
            && compiler.trim().is_empty()
        {
            anyhow::bail!("Model package artifact compiler must not be empty when supplied.");
        }
        if !ids.insert(artifact.id.as_str()) {
            anyhow::bail!("Duplicate model package artifact id {}.", artifact.id);
        }
        if !artifact.artifact.is_file() {
            anyhow::bail!(
                "Model package artifact {} is missing.",
                artifact.artifact.display()
            );
        }
    }
    Ok(())
}

fn primary_registry_artifact<'a>(
    options: &ModelPackageExportOptions,
    artifacts: &'a [ModelPackageExportArtifactSummary],
) -> Result<&'a ModelPackageExportArtifactSummary> {
    if let Some(artifact_id) = options.registry_artifact_id.as_deref() {
        return artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id)
            .with_context(|| {
                format!("--registry-artifact-id {artifact_id} does not match an exported artifact.")
            });
    }
    artifacts
        .first()
        .context("Model package export requires at least one artifact.")
}

fn render_lock_document(
    options: &ModelPackageExportOptions,
    artifacts: &[ModelPackageExportArtifactSummary],
) -> String {
    let artifact_rows = artifacts
        .iter()
        .map(render_lock_artifact)
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema_version\": {},\n  \"package\": {{\n    \"name\": {},\n    \"version\": {}\n  }},\n  \"artifacts\": [\n{}\n  ]\n}}\n",
        json_string(MODEL_PACKAGE_LOCK_SCHEMA),
        json_string(&options.package_name),
        json_string(&options.package_version),
        artifact_rows,
    )
}

fn render_lock_artifact(artifact: &ModelPackageExportArtifactSummary) -> String {
    let compiler = artifact
        .compiler
        .as_deref()
        .map(|compiler| format!(",\n      \"compiler\": {}", json_string(compiler)))
        .unwrap_or_default();
    format!(
        "    {{\n      \"id\": {},\n      \"path\": {},\n      \"sha256\": {},\n      \"artifact_format\": {}{}\n    }}",
        json_string(&artifact.id),
        json_string(&artifact.artifact_path),
        json_string(&artifact.artifact_sha256),
        json_string(&artifact.artifact_format),
        compiler,
    )
}

fn render_registry_document(
    options: &ModelPackageExportOptions,
    entry: &str,
    artifact_id: &str,
    lock_path: &str,
    lock_sha: &str,
) -> String {
    format!(
        "{{\n  \"schema_version\": {},\n  \"packages\": [\n    {{\n      \"id\": {},\n      \"package\": {{\n        \"name\": {},\n        \"version\": {}\n      }},\n      \"artifact_id\": {},\n      \"lock_path\": {},\n      \"lock_sha256\": {}\n    }}\n  ]\n}}\n",
        json_string(MODEL_PACKAGE_REGISTRY_SCHEMA),
        json_string(entry),
        json_string(&options.package_name),
        json_string(&options.package_version),
        json_string(artifact_id),
        json_string(lock_path),
        json_string(lock_sha),
    )
}

fn render_registry_entries_document<'a>(
    entries: impl IntoIterator<Item = &'a ModelPackageRegistryEntry>,
) -> String {
    let rows = entries
        .into_iter()
        .map(render_registry_entry)
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema_version\": {},\n  \"packages\": [\n{}\n  ]\n}}\n",
        json_string(MODEL_PACKAGE_REGISTRY_SCHEMA),
        rows,
    )
}

fn render_registry_entry(entry: &ModelPackageRegistryEntry) -> String {
    format!(
        "    {{\n      \"id\": {},\n      \"package\": {{\n        \"name\": {},\n        \"version\": {}\n      }},\n      \"artifact_id\": {},\n      \"lock_path\": {},\n      \"lock_sha256\": {}\n    }}",
        json_string(&entry.id),
        json_string(&entry.package_name),
        json_string(&entry.package_version),
        json_string(&entry.artifact_id),
        json_string(&entry.lock_path),
        json_string(&entry.lock_sha256),
    )
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization cannot fail")
}
