use crate::analog_model_resolver::{
    InferredAnalogModelFile, declared_model_file_path_for_project,
    inferred_model_files_for_components,
};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn model_file_values_for_generated_components(
    project_path: &Path,
    project: &crate::board_ir::BoardProject,
    component_ids: &[String],
) -> Result<Vec<serde_yaml_ng::Value>> {
    Ok(
        inferred_model_files_for_components(project_path, project, component_ids)
            .map_err(anyhow::Error::msg)?
            .into_iter()
            .map(|entry| model_file_value(&entry))
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
        inferred_model_files_for_components(project_path, &project, &generated.components)
            .map_err(anyhow::Error::msg)?;
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
        if let Ok(path) = declared_model_file_path_for_project(project_path, &model_file.path) {
            existing_canonical.insert(path);
        }
    }

    let missing = inferred
        .into_iter()
        .filter(|entry| {
            !existing_paths.contains(&entry.model_file.path)
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
        existing_paths.insert(entry.model_file.path.clone());
        model_files.push(model_file_value(&entry));
    }
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited generated model-file YAML is not valid Board IR.")?;
    Ok(updated)
}

fn model_file_value(entry: &InferredAnalogModelFile) -> serde_yaml_ng::Value {
    let model_file = &entry.model_file;
    let mut mapping = serde_yaml_ng::Mapping::new();
    mapping.insert(
        serde_yaml_ng::Value::String("path".to_string()),
        serde_yaml_ng::Value::String(model_file.path.clone()),
    );
    insert_optional_string(&mut mapping, "sha256", model_file.sha256.as_deref());
    insert_optional_string(
        &mut mapping,
        "model_package_name",
        model_file.model_package_name.as_deref(),
    );
    insert_optional_string(
        &mut mapping,
        "model_package_version",
        model_file.model_package_version.as_deref(),
    );
    insert_optional_string(
        &mut mapping,
        "model_package_artifact_id",
        model_file.model_package_artifact_id.as_deref(),
    );
    insert_optional_string(
        &mut mapping,
        "model_package_lock_path",
        model_file.model_package_lock_path.as_deref(),
    );
    insert_optional_string(
        &mut mapping,
        "model_package_lock_sha256",
        model_file.model_package_lock_sha256.as_deref(),
    );
    insert_optional_string(
        &mut mapping,
        "model_package_registry_path",
        model_file.model_package_registry_path.as_deref(),
    );
    insert_optional_string(
        &mut mapping,
        "model_package_registry_sha256",
        model_file.model_package_registry_sha256.as_deref(),
    );
    insert_optional_string(
        &mut mapping,
        "model_package_registry_entry",
        model_file.model_package_registry_entry.as_deref(),
    );
    serde_yaml_ng::Value::Mapping(mapping)
}

fn insert_optional_string(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        mapping.insert(
            serde_yaml_ng::Value::String(name.to_string()),
            serde_yaml_ng::Value::String(value.to_string()),
        );
    }
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
            entries[0].model_file.path,
            "../../models/spice/generic/analog_behavioral.lib"
        );
        assert_eq!(
            entries[0].model_file.sha256.as_deref(),
            Some("ad5aec2585e6d9803b3b6f7930c19148e252c1cf4362b550893e85afdd025e59")
        );
    }

    #[test]
    fn inferred_model_file_resolves_vendor_sensor_pack_from_example_tree() {
        let project_path = Path::new("examples/good_aosong_aht20_i2c_observation/project.yaml");
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(include_str!(
            "../../examples/good_aosong_aht20_i2c_observation/project.yaml"
        ))
        .unwrap();

        let values = model_file_values_for_generated_components(
            project_path,
            &project,
            &["UAHT".to_string()],
        )
        .unwrap();

        assert_eq!(values.len(), 1);
        let entry = values[0].as_mapping().unwrap();
        assert_eq!(
            entry
                .get(key("path"))
                .and_then(serde_yaml_ng::Value::as_str),
            Some("../../models/spice/aosong/aht20_i2c_observation.lib")
        );
        assert_eq!(
            entry
                .get(key("sha256"))
                .and_then(serde_yaml_ng::Value::as_str),
            Some("cbb7ebb94896b20e0e835e70d6e5dac1edc31ffae6bbe8200666c352da567a39")
        );
    }
}
