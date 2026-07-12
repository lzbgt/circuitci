use crate::board_ir::{AnalogModelFile, AnalogScenario};
use crate::library::{BoundBoard, SpiceModel};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct InferredAnalogModelFile {
    pub(crate) model_file: AnalogModelFile,
    pub(crate) canonical_path: PathBuf,
}

pub(crate) fn effective_model_files(
    bound: &BoundBoard<'_>,
    analog: &AnalogScenario,
) -> Result<Vec<AnalogModelFile>, String> {
    let mut model_files = analog.model_files.clone();
    let mut canonical_paths = model_files
        .iter()
        .map(|model_file| declared_model_file_path(bound, &model_file.path).ok())
        .collect::<Vec<_>>();
    let Some(generated) = analog.generated.as_ref() else {
        return Ok(model_files);
    };

    for inferred in infer_generated_component_model_files(bound, &generated.components)? {
        if let Some(index) = canonical_paths
            .iter()
            .position(|path| path.as_ref() == Some(&inferred.canonical_path))
        {
            merge_inferred_model_file(&mut model_files[index], inferred.model_file);
            continue;
        }
        canonical_paths.push(Some(inferred.canonical_path));
        model_files.push(inferred.model_file);
    }
    Ok(model_files)
}

#[cfg(any(feature = "gui", test))]
pub(crate) fn inferred_model_files_for_components(
    project_path: &Path,
    project: &crate::board_ir::BoardProject,
    component_ids: &[String],
) -> Result<Vec<InferredAnalogModelFile>, String> {
    let mut project = project.clone();
    project.source_dir = project_source_dir(project_path);
    let (library, findings) = crate::library::load_library(project_path, &project);
    let bound = crate::library::bind_project(&project, library, findings);
    inferred_model_files_for_bound_components(&bound, component_ids)
}

pub(crate) fn inferred_model_files_for_bound_components(
    bound: &BoundBoard<'_>,
    component_ids: &[String],
) -> Result<Vec<InferredAnalogModelFile>, String> {
    infer_generated_component_model_files(bound, component_ids)
}

#[cfg(feature = "gui")]
pub(crate) fn declared_model_file_path_for_project(
    project_path: &Path,
    path: &str,
) -> Result<PathBuf, String> {
    declared_model_file_path_for_source_dir(&project_source_dir(project_path), path)
}

pub(crate) fn declared_model_file_path_for_source_dir(
    source_dir: &Path,
    path: &str,
) -> Result<PathBuf, String> {
    let path = Path::new(path);
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        source_dir.join(path)
    };
    absolute_path(&candidate).map_err(|error| error.to_string())
}

pub(crate) fn inferred_model_file_for_model_path(
    source_dir: &Path,
    model_path: &str,
) -> Result<InferredAnalogModelFile, String> {
    let canonical_path = resolve_model_path_from_dir(source_dir, model_path)?;
    Ok(InferredAnalogModelFile {
        model_file: analog_model_file_for_path(
            source_dir,
            &canonical_path,
            ModelFileMetadata::default(),
        )?,
        canonical_path,
    })
}

fn infer_generated_component_model_files(
    bound: &BoundBoard<'_>,
    component_ids: &[String],
) -> Result<Vec<InferredAnalogModelFile>, String> {
    let mut entries = Vec::new();
    let mut seen_inferred = BTreeSet::new();
    for component_id in component_ids {
        let Some(component) = bound.project.board.components.get(component_id) else {
            continue;
        };
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let Some(spice) = model.simulation.spice.as_ref() else {
            continue;
        };
        let canonical_path =
            resolve_model_path_from_dir(&bound.project.source_dir, &spice.model_path).map_err(
                |error| {
                    format!(
                        "Failed to resolve SPICE model file {} for generated component {}: {error}",
                        spice.model_path, component_id
                    )
                },
            )?;
        if !seen_inferred.insert(canonical_path.clone()) {
            continue;
        }
        entries.push(InferredAnalogModelFile {
            model_file: inferred_model_file(bound, spice, &canonical_path)?,
            canonical_path,
        });
    }
    Ok(entries)
}

fn inferred_model_file(
    bound: &BoundBoard<'_>,
    spice: &SpiceModel,
    canonical_path: &Path,
) -> Result<AnalogModelFile, String> {
    analog_model_file_for_path(
        &bound.project.source_dir,
        canonical_path,
        ModelFileMetadata {
            model_package_name: spice.model_package_name.clone(),
            model_package_version: spice.model_package_version.clone(),
            model_package_artifact_id: spice.model_package_artifact_id.clone(),
            model_package_lock_path: optional_project_relative_existing_path(
                bound,
                spice.model_package_lock_path.as_deref(),
            )?,
            model_package_lock_sha256: spice.model_package_lock_sha256.clone(),
            model_package_registry_path: optional_project_relative_existing_path(
                bound,
                spice.model_package_registry_path.as_deref(),
            )?,
            model_package_registry_sha256: spice.model_package_registry_sha256.clone(),
            model_package_registry_entry: spice.model_package_registry_entry.clone(),
        },
    )
}

#[derive(Default)]
struct ModelFileMetadata {
    model_package_name: Option<String>,
    model_package_version: Option<String>,
    model_package_artifact_id: Option<String>,
    model_package_lock_path: Option<String>,
    model_package_lock_sha256: Option<String>,
    model_package_registry_path: Option<String>,
    model_package_registry_sha256: Option<String>,
    model_package_registry_entry: Option<String>,
}

fn analog_model_file_for_path(
    source_dir: &Path,
    canonical_path: &Path,
    metadata: ModelFileMetadata,
) -> Result<AnalogModelFile, String> {
    Ok(AnalogModelFile {
        path: relative_path(&canonical_source_dir(source_dir), canonical_path)
            .unwrap_or_else(|| canonical_path.to_path_buf())
            .to_string_lossy()
            .replace('\\', "/"),
        sha256: Some(file_sha256_hex(canonical_path)?),
        artifact_format: None,
        source_path: None,
        source_sha256: None,
        compiler: None,
        compiler_version: None,
        compiler_command: None,
        plugin_load_command: None,
        xyce_version: None,
        xyce_adms_template_revision: None,
        xyce_configure_options: Vec::new(),
        conformance_artifact: None,
        conformance_sha256: None,
        model_package_name: metadata.model_package_name,
        model_package_version: metadata.model_package_version,
        model_package_artifact_id: metadata.model_package_artifact_id,
        model_package_lock_path: metadata.model_package_lock_path,
        model_package_lock_sha256: metadata.model_package_lock_sha256,
        model_package_registry_path: metadata.model_package_registry_path,
        model_package_registry_sha256: metadata.model_package_registry_sha256,
        model_package_registry_entry: metadata.model_package_registry_entry,
    })
}

fn merge_inferred_model_file(existing: &mut AnalogModelFile, inferred: AnalogModelFile) {
    fill_optional(
        &mut existing.model_package_name,
        inferred.model_package_name,
    );
    fill_optional(
        &mut existing.model_package_version,
        inferred.model_package_version,
    );
    fill_optional(
        &mut existing.model_package_artifact_id,
        inferred.model_package_artifact_id,
    );
    fill_optional(
        &mut existing.model_package_lock_path,
        inferred.model_package_lock_path,
    );
    fill_optional(
        &mut existing.model_package_lock_sha256,
        inferred.model_package_lock_sha256,
    );
    fill_optional(
        &mut existing.model_package_registry_path,
        inferred.model_package_registry_path,
    );
    fill_optional(
        &mut existing.model_package_registry_sha256,
        inferred.model_package_registry_sha256,
    );
    fill_optional(
        &mut existing.model_package_registry_entry,
        inferred.model_package_registry_entry,
    );
}

fn fill_optional(target: &mut Option<String>, inferred: Option<String>) {
    if target.is_none() {
        *target = inferred;
    }
}

fn declared_model_file_path(bound: &BoundBoard<'_>, path: &str) -> Result<PathBuf, String> {
    declared_model_file_path_for_source_dir(&bound.project.source_dir, path)
}

fn resolve_model_path(bound: &BoundBoard<'_>, model_path: &str) -> Result<PathBuf, String> {
    resolve_model_path_from_dir(&bound.project.source_dir, model_path)
}

fn resolve_model_path_from_dir(source_dir: &Path, model_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(model_path);
    if path.is_absolute() {
        return absolute_path(path).map_err(|error| error.to_string());
    }

    for base in source_dir.ancestors() {
        let candidate = base.join(path);
        if candidate.exists() {
            return absolute_path(&candidate).map_err(|error| error.to_string());
        }
    }

    Err(format!(
        "relative model path {model_path} was not found from project directory {} or any ancestor",
        source_dir.display()
    ))
}

fn optional_project_relative_existing_path(
    bound: &BoundBoard<'_>,
    path: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let canonical_path = resolve_model_path(bound, path)?;
    Ok(Some(
        relative_path(
            &canonical_source_dir(&bound.project.source_dir),
            &canonical_path,
        )
        .unwrap_or(canonical_path)
        .to_string_lossy()
        .replace('\\', "/"),
    ))
}

fn canonical_source_dir(source_dir: &Path) -> PathBuf {
    absolute_path(source_dir).unwrap_or_else(|_| source_dir.to_path_buf())
}

#[cfg(any(feature = "gui", test))]
fn project_source_dir(project_path: &Path) -> PathBuf {
    project_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn file_sha256_hex(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("Failed to read model file {}: {error}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn absolute_path(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(normalize_path(path));
    }
    Ok(normalize_path(&env::current_dir()?.join(path)))
}

fn relative_path(from_dir: &Path, to: &Path) -> Option<PathBuf> {
    let from_components = from_dir.components().collect::<Vec<_>>();
    let to_components = to.components().collect::<Vec<_>>();
    let common = from_components
        .iter()
        .zip(&to_components)
        .take_while(|(left, right)| left == right)
        .count();
    if common == 0 {
        return None;
    }

    let mut path = PathBuf::new();
    for component in &from_components[common..] {
        match component {
            Component::Normal(_) => path.push(".."),
            Component::CurDir => {}
            Component::ParentDir => path.push(".."),
            Component::Prefix(_) | Component::RootDir => return None,
        }
    }
    for component in &to_components[common..] {
        match component {
            Component::Normal(value) => path.push(value),
            Component::CurDir => {}
            Component::ParentDir => path.push(".."),
            Component::Prefix(_) | Component::RootDir => return None,
        }
    }
    Some(if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effective_model_files_infer_generated_component_pack_artifacts() {
        let project_path = Path::new("examples/good_aosong_aht20_i2c_observation/project.yaml");
        let mut project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(include_str!(
            "../examples/good_aosong_aht20_i2c_observation/project.yaml"
        ))
        .unwrap();
        project.source_dir = project_path.parent().unwrap().to_path_buf();
        let analog = project.scenarios[0].analog.as_mut().unwrap();
        analog.model_files.clear();

        let (library, findings) = crate::library::load_library(project_path, &project);
        let bound = crate::library::bind_project(&project, library, findings);
        let model_files =
            effective_model_files(&bound, bound.project.scenarios[0].analog.as_ref().unwrap())
                .unwrap();

        assert_eq!(model_files.len(), 1);
        assert_eq!(
            model_files[0].path,
            "../../models/spice/aosong/aht20_i2c_observation.lib"
        );
        assert_eq!(
            model_files[0].sha256.as_deref(),
            Some("cbb7ebb94896b20e0e835e70d6e5dac1edc31ffae6bbe8200666c352da567a39")
        );
    }

    #[test]
    fn inferred_model_files_for_components_matches_effective_resolver() {
        let project_path = Path::new("examples/good_aosong_aht20_i2c_observation/project.yaml");
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(include_str!(
            "../examples/good_aosong_aht20_i2c_observation/project.yaml"
        ))
        .unwrap();

        let entries =
            inferred_model_files_for_components(project_path, &project, &["UAHT".to_string()])
                .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].model_file.path,
            "../../models/spice/aosong/aht20_i2c_observation.lib"
        );
        assert_eq!(
            entries[0].model_file.sha256.as_deref(),
            Some("cbb7ebb94896b20e0e835e70d6e5dac1edc31ffae6bbe8200666c352da567a39")
        );
        assert!(
            entries[0]
                .canonical_path
                .ends_with("aht20_i2c_observation.lib")
        );
    }
}
