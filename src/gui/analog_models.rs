use crate::analog_model_resolver::declared_model_file_path_for_project;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub(super) struct AnalogModelFileDraft {
    pub(super) scenario_name: String,
    pub(super) path: String,
    pub(super) sha256: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogModelFileRemoveDraft {
    pub(super) scenario_name: String,
    pub(super) path: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogModelFileScenario {
    pub(super) name: String,
    pub(super) model_files: Vec<AnalogModelFileEntry>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogModelFileEntry {
    pub(super) path: String,
    pub(super) sha256: Option<String>,
}

pub(super) fn analog_model_file_scenarios(text: &str) -> Result<Vec<AnalogModelFileScenario>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    Ok(project
        .scenarios
        .iter()
        .filter_map(|scenario| {
            let analog = scenario.analog.as_ref()?;
            Some(AnalogModelFileScenario {
                name: scenario.name.clone(),
                model_files: analog
                    .model_files
                    .iter()
                    .map(|model_file| AnalogModelFileEntry {
                        path: model_file.path.clone(),
                        sha256: model_file.sha256.clone(),
                    })
                    .collect(),
            })
        })
        .collect())
}

pub(super) fn append_analog_model_file(
    text: &str,
    project_path: &Path,
    draft: &AnalogModelFileDraft,
) -> Result<String> {
    validate_model_file_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    let path = normalized_model_path(&draft.path)?;
    if analog
        .model_files
        .iter()
        .any(|model_file| model_file.path == path)
    {
        anyhow::bail!(
            "Analog model file {} already exists in scenario {}.",
            path,
            scenario.name
        );
    }
    let resolved_path = declared_model_file_path_for_project(project_path, &path)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("Failed to resolve SPICE model file {path}."))?;
    let actual_sha = file_sha256_hex(&resolved_path).with_context(|| {
        format!(
            "Failed to hash SPICE model file {}.",
            resolved_path.display()
        )
    })?;
    if !actual_sha.eq_ignore_ascii_case(draft.sha256.trim()) {
        anyhow::bail!(
            "SPICE model file {} SHA-256 does not match the provided hash.",
            resolved_path.display()
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let model_files = ensure_child_sequence_mut(analog_mapping, "model_files", "model files")?;
    let mut entry = serde_yaml_ng::Mapping::new();
    insert_string(&mut entry, "path", &path);
    insert_string(&mut entry, "sha256", &actual_sha);
    model_files.push(serde_yaml_ng::Value::Mapping(entry));
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog model-file YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn remove_analog_model_file(
    text: &str,
    draft: &AnalogModelFileRemoveDraft,
) -> Result<String> {
    validate_model_file_remove_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if !analog
        .model_files
        .iter()
        .any(|model_file| model_file.path == draft.path)
    {
        anyhow::bail!(
            "Analog model file {} was not found in scenario {}.",
            draft.path,
            scenario.name
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let model_files = ensure_child_sequence_mut(analog_mapping, "model_files", "model files")?;
    model_files.retain(|model_file| {
        model_file
            .as_mapping()
            .and_then(|mapping| mapping.get(key("path")))
            .and_then(serde_yaml_ng::Value::as_str)
            != Some(draft.path.as_str())
    });
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog model-file YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn model_file_sha256(project_path: &Path, model_path: &str) -> Result<String> {
    let path = normalized_model_path(model_path)?;
    let resolved_path = declared_model_file_path_for_project(project_path, &path)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("Failed to resolve SPICE model file {path}."))?;
    file_sha256_hex(&resolved_path).with_context(|| {
        format!(
            "Failed to hash SPICE model file {}.",
            resolved_path.display()
        )
    })
}

fn validate_model_file_draft(draft: &AnalogModelFileDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    normalized_model_path(&draft.path)?;
    if !is_sha256_hex(draft.sha256.trim()) {
        anyhow::bail!("SPICE model file SHA-256 must be a 64-character hex string.");
    }
    Ok(())
}

fn validate_model_file_remove_draft(draft: &AnalogModelFileRemoveDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    normalized_model_path(&draft.path)?;
    Ok(())
}

fn normalized_model_path(path: &str) -> Result<String> {
    let path = path.trim();
    if path.is_empty() {
        anyhow::bail!("SPICE model file path must not be blank.");
    }
    if path.contains('\n') || path.contains('\r') {
        anyhow::bail!("SPICE model file path must fit on one line.");
    }
    Ok(path.to_string())
}

fn file_sha256_hex(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validated_id(value: &str, label: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{label} must not be blank.");
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        anyhow::bail!("{label} {value:?} contains unsupported characters.");
    }
    Ok(value.to_string())
}

fn scenario_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    scenario_name: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let scenarios = yaml
        .as_mapping_mut()
        .and_then(|mapping| mapping.get_mut(key("scenarios")))
        .and_then(serde_yaml_ng::Value::as_sequence_mut)
        .context("Project YAML must declare scenarios as a list.")?;
    scenarios
        .iter_mut()
        .filter_map(serde_yaml_ng::Value::as_mapping_mut)
        .find(|mapping| {
            mapping
                .get(key("name"))
                .and_then(serde_yaml_ng::Value::as_str)
                == Some(scenario_name)
        })
        .with_context(|| format!("Scenario {scenario_name} was not found."))
}

fn child_mapping_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    field: &str,
    label: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    mapping
        .get_mut(key(field))
        .with_context(|| format!("{label} must declare {field}."))?
        .as_mapping_mut()
        .with_context(|| format!("{label} {field} must be a YAML object."))
}

fn ensure_child_sequence_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    field: &str,
    label: &str,
) -> Result<&'a mut Vec<serde_yaml_ng::Value>> {
    let key = key(field);
    if !mapping.contains_key(&key) {
        mapping.insert(key.clone(), serde_yaml_ng::Value::Sequence(Vec::new()));
    }
    mapping
        .get_mut(&key)
        .expect("field was inserted when absent")
        .as_sequence_mut()
        .with_context(|| format!("{label} must be a YAML list."))
}

fn insert_string(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: &str) {
    mapping.insert(key(name), serde_yaml_ng::Value::String(value.to_string()));
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        AnalogModelFileDraft, AnalogModelFileRemoveDraft, analog_model_file_scenarios,
        append_analog_model_file, model_file_sha256, remove_analog_model_file,
    };
    use std::fs;

    fn project_yaml() -> &'static str {
        "project:
  name: gui_model_file_test
  version: 0.1.0
board:
  components: {}
  nets:
    gnd:
      kind: ground
scenarios:
  - name: transient
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: file
      netlist: deck.cir
      model_files: []
      node_bindings:
        - node: '0'
          net: gnd
      pin_bindings:
        - node: '0'
          endpoint:
            component: U1
            pin: GND
      analysis:
        type: tran
        stop_time_us: 10
        max_step_us: 1
      stimuli: []
      probes:
        - name: ground
          expression: V(0)
          quantity: voltage
      assertions: []
"
    }

    #[test]
    fn append_analog_model_file_emits_valid_sha_backed_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let project_path = dir.path().join("project.yaml");
        let model_path = dir.path().join("vendor.lib");
        fs::write(&project_path, project_yaml()).unwrap();
        fs::write(&model_path, ".model DTEST D\n").unwrap();
        let sha256 = model_file_sha256(&project_path, "vendor.lib").unwrap();

        let edited = append_analog_model_file(
            project_yaml(),
            &project_path,
            &AnalogModelFileDraft {
                scenario_name: "transient".to_string(),
                path: "vendor.lib".to_string(),
                sha256: sha256.clone(),
            },
        )
        .unwrap();

        let scenarios = analog_model_file_scenarios(&edited).unwrap();
        assert_eq!(scenarios[0].model_files[0].path, "vendor.lib");
        assert_eq!(
            scenarios[0].model_files[0].sha256.as_deref(),
            Some(sha256.as_str())
        );
    }

    #[test]
    fn append_analog_model_file_rejects_sha_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let project_path = dir.path().join("project.yaml");
        let model_path = dir.path().join("vendor.lib");
        fs::write(&project_path, project_yaml()).unwrap();
        fs::write(&model_path, ".model DTEST D\n").unwrap();

        let error = append_analog_model_file(
            project_yaml(),
            &project_path,
            &AnalogModelFileDraft {
                scenario_name: "transient".to_string(),
                path: "vendor.lib".to_string(),
                sha256: "0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("does not match"));
    }

    #[test]
    fn remove_analog_model_file_preserves_scenario() {
        let dir = tempfile::tempdir().unwrap();
        let project_path = dir.path().join("project.yaml");
        let model_path = dir.path().join("vendor.lib");
        fs::write(&project_path, project_yaml()).unwrap();
        fs::write(&model_path, ".model DTEST D\n").unwrap();
        let sha256 = model_file_sha256(&project_path, "vendor.lib").unwrap();
        let edited = append_analog_model_file(
            project_yaml(),
            &project_path,
            &AnalogModelFileDraft {
                scenario_name: "transient".to_string(),
                path: "vendor.lib".to_string(),
                sha256,
            },
        )
        .unwrap();

        let edited = remove_analog_model_file(
            &edited,
            &AnalogModelFileRemoveDraft {
                scenario_name: "transient".to_string(),
                path: "vendor.lib".to_string(),
            },
        )
        .unwrap();

        let scenarios = analog_model_file_scenarios(&edited).unwrap();
        assert!(scenarios[0].model_files.is_empty());
    }
}
