use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub(super) struct InferredAnalogModelFile {
    pub(super) path: String,
    pub(super) sha256: String,
    canonical_path: PathBuf,
}

pub(super) fn model_file_values_for_generated_components(
    project_path: &Path,
    project: &crate::board_ir::BoardProject,
    component_ids: &[String],
) -> Result<Vec<serde_yaml_ng::Value>> {
    Ok(
        inferred_model_files_for_components(project_path, project, component_ids)?
            .into_iter()
            .map(|entry| model_file_value(&entry.path, &entry.sha256))
            .collect(),
    )
}

pub(super) fn add_missing_generated_model_files(
    text: &str,
    project_path: &Path,
    scenario_name: &str,
) -> Result<String> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Scenario {scenario_name} was not found."))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {scenario_name} is not an analog scenario."))?;
    let Some(generated) = analog.generated.as_ref() else {
        return Ok(text.to_string());
    };
    let inferred =
        inferred_model_files_for_components(project_path, &project, &generated.components)?;
    if inferred.is_empty() {
        return Ok(text.to_string());
    }

    let mut existing_paths: BTreeSet<String> = analog
        .model_files
        .iter()
        .map(|model_file| model_file.path.clone())
        .collect();
    let mut existing_canonical = BTreeSet::new();
    for model_file in &analog.model_files {
        if let Ok(path) = declared_model_file_path(project_path, &model_file.path) {
            existing_canonical.insert(path);
        }
    }

    let missing = inferred
        .into_iter()
        .filter(|entry| {
            !existing_paths.contains(&entry.path)
                && !existing_canonical.contains(&entry.canonical_path)
        })
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(text.to_string());
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let model_files = ensure_child_sequence_mut(analog_mapping, "model_files", "model files")?;
    for entry in missing {
        existing_paths.insert(entry.path.clone());
        model_files.push(model_file_value(&entry.path, &entry.sha256));
    }
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited generated model-file YAML is not valid Board IR.")?;
    Ok(updated)
}

fn inferred_model_files_for_components(
    project_path: &Path,
    project: &crate::board_ir::BoardProject,
    component_ids: &[String],
) -> Result<Vec<InferredAnalogModelFile>> {
    let (library, _findings) = crate::library::load_library(project_path, project);
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    for component_id in component_ids {
        let Some(component) = project.board.components.get(component_id) else {
            continue;
        };
        let Some(model) = library.get(&component.model) else {
            continue;
        };
        let Some(spice) = model.simulation.spice.as_ref() else {
            continue;
        };
        let canonical_path =
            resolve_model_path(project_path, &spice.model_path).with_context(|| {
                format!(
                    "Failed to resolve SPICE model file {} for component {}.",
                    spice.model_path, component_id
                )
            })?;
        if !seen.insert(canonical_path.clone()) {
            continue;
        }
        let project_dir = canonical_project_dir(project_path)?;
        let path = relative_path(&project_dir, &canonical_path)
            .unwrap_or_else(|| canonical_path.clone())
            .to_string_lossy()
            .replace('\\', "/");
        let sha256 = file_sha256_hex(&canonical_path).with_context(|| {
            format!(
                "Failed to hash SPICE model file {}.",
                canonical_path.display()
            )
        })?;
        entries.push(InferredAnalogModelFile {
            path,
            sha256,
            canonical_path,
        });
    }
    Ok(entries)
}

fn model_file_value(path: &str, sha256: &str) -> serde_yaml_ng::Value {
    let mut mapping = serde_yaml_ng::Mapping::new();
    mapping.insert(
        serde_yaml_ng::Value::String("path".to_string()),
        serde_yaml_ng::Value::String(path.to_string()),
    );
    mapping.insert(
        serde_yaml_ng::Value::String("sha256".to_string()),
        serde_yaml_ng::Value::String(sha256.to_string()),
    );
    serde_yaml_ng::Value::Mapping(mapping)
}

fn resolve_model_path(project_path: &Path, model_path: &str) -> Result<PathBuf> {
    let path = Path::new(model_path);
    if path.is_absolute() {
        return path
            .canonicalize()
            .with_context(|| format!("Could not canonicalize {}.", path.display()));
    }

    let project_dir = canonical_project_dir(project_path)?;
    for base in project_dir.ancestors() {
        let candidate = base.join(path);
        if candidate.exists() {
            return candidate
                .canonicalize()
                .with_context(|| format!("Could not canonicalize {}.", candidate.display()));
        }
    }
    anyhow::bail!(
        "relative model path {model_path} was not found from project directory {} or any ancestor",
        project_dir.display()
    );
}

fn declared_model_file_path(project_path: &Path, model_path: &str) -> Result<PathBuf> {
    let path = Path::new(model_path);
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        canonical_project_dir(project_path)?.join(path)
    };
    candidate
        .canonicalize()
        .with_context(|| format!("Could not canonicalize {}.", candidate.display()))
}

fn canonical_project_dir(project_path: &Path) -> Result<PathBuf> {
    let project_dir = project_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    project_dir.canonicalize().with_context(|| {
        format!(
            "Could not canonicalize project dir {}.",
            project_dir.display()
        )
    })
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

fn file_sha256_hex(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

fn scenario_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    scenario_name: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let scenarios = yaml
        .as_mapping_mut()
        .and_then(|mapping| mapping.get_mut(key("scenarios")))
        .and_then(serde_yaml_ng::Value::as_sequence_mut)
        .context("Project YAML must contain a scenarios sequence.")?;
    for scenario in scenarios {
        let Some(mapping) = scenario.as_mapping_mut() else {
            continue;
        };
        if mapping
            .get(key("name"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(scenario_name)
        {
            return Ok(mapping);
        }
    }
    anyhow::bail!("Scenario {scenario_name} was not found.");
}

fn child_mapping_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    field: &str,
    label: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    mapping
        .get_mut(key(field))
        .with_context(|| format!("{label} must contain {field}."))?
        .as_mapping_mut()
        .with_context(|| format!("{label} {field} must be a YAML object."))
}

fn ensure_child_sequence_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    field: &str,
    label: &str,
) -> Result<&'a mut Vec<serde_yaml_ng::Value>> {
    if !mapping.contains_key(key(field)) {
        mapping.insert(key(field), serde_yaml_ng::Value::Sequence(Vec::new()));
    }
    mapping
        .get_mut(key(field))
        .with_context(|| format!("{label} was not found."))?
        .as_sequence_mut()
        .with_context(|| format!("{label} must be a YAML sequence."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inferred_model_file_uses_portable_relative_path_and_sha() {
        let project_path = Path::new("examples/good_ideal_opamp_buffer/project.yaml");
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(include_str!(
            "../../examples/good_ideal_opamp_buffer/project.yaml"
        ))
        .unwrap();

        let entries = inferred_model_files_for_components(
            project_path,
            &project,
            &["XU1".to_string(), "RLOAD".to_string()],
        )
        .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].path,
            "../../models/spice/generic/analog_behavioral.lib"
        );
        assert_eq!(
            entries[0].sha256,
            "7c1e9149faa6e8acf593a034970485d6d6c09d686862c35eef6de7f72106c993"
        );
    }
}
