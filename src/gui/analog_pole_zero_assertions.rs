use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub(super) struct AnalogPoleZeroAssertionDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
    pub(super) root_kind: String,
    pub(super) root_index: Option<u32>,
    pub(super) metric: String,
    pub(super) relation: String,
    pub(super) threshold: f64,
}

pub(super) fn append_analog_pole_zero_assertion(
    text: &str,
    draft: &AnalogPoleZeroAssertionDraft,
) -> Result<String> {
    validate_pole_zero_assertion_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    if scenario.scenario_type != "analog_pole_zero" {
        anyhow::bail!("Pole-zero assertions require an analog_pole_zero scenario.");
    }
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if analog
        .assertions
        .iter()
        .any(|assertion| assertion.name == draft.assertion_name)
        || analog
            .analysis
            .pole_zero_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
    {
        anyhow::bail!(
            "Pole-zero assertion {} already exists in scenario {}.",
            draft.assertion_name,
            scenario.name
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let analysis_mapping = child_mapping_mut(analog_mapping, "analysis", "analog analysis")?;
    let assertions = ensure_child_sequence_mut(
        analysis_mapping,
        "pole_zero_assertions",
        "pole-zero assertions",
    )?;
    assertions.push(pole_zero_assertion_value(draft)?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited pole-zero assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn unique_analog_pole_zero_assertion_name(
    text: &str,
    scenario_name: &str,
    requested_name: &str,
) -> Result<String> {
    let scenario_name = validated_id(scenario_name, "scenario name")?;
    let requested_name = validated_id(requested_name, "assertion name")?;
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
    let mut existing: std::collections::BTreeSet<&str> = analog
        .assertions
        .iter()
        .map(|assertion| assertion.name.as_str())
        .collect();
    existing.extend(
        analog
            .analysis
            .pole_zero_assertions
            .iter()
            .map(|assertion| assertion.name.as_str()),
    );
    if !existing.contains(requested_name) {
        return Ok(requested_name.to_string());
    }
    for suffix in 2.. {
        let candidate = format!("{requested_name}_{suffix}");
        if !existing.contains(candidate.as_str()) {
            return Ok(candidate);
        }
    }
    unreachable!("unbounded assertion suffix search must return")
}

fn validate_pole_zero_assertion_draft(draft: &AnalogPoleZeroAssertionDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    if !matches!(draft.root_kind.as_str(), "pole" | "zero") {
        anyhow::bail!("Pole-zero assertion root_kind must be pole or zero.");
    }
    if draft.root_index == Some(0) {
        anyhow::bail!("Pole-zero assertion root_index must be >= 1.");
    }
    if !matches!(
        draft.metric.as_str(),
        "real_rad_per_s" | "imaginary_rad_per_s" | "frequency_hz"
    ) {
        anyhow::bail!("Unsupported pole-zero assertion metric {}.", draft.metric);
    }
    if !matches!(draft.relation.as_str(), "above" | "below") {
        anyhow::bail!("Pole-zero assertion relation must be above or below.");
    }
    if !draft.threshold.is_finite() {
        anyhow::bail!("Pole-zero assertion threshold must be finite.");
    }
    Ok(())
}

fn pole_zero_assertion_value(draft: &AnalogPoleZeroAssertionDraft) -> Result<serde_yaml_ng::Value> {
    let mut assertion = serde_yaml_ng::Mapping::new();
    insert_string(&mut assertion, "name", draft.assertion_name.trim());
    insert_string(&mut assertion, "root_kind", draft.root_kind.trim());
    if let Some(root_index) = draft.root_index {
        assertion.insert(
            key("root_index"),
            serde_yaml_ng::to_value(root_index).context("Failed to encode root_index.")?,
        );
    }
    insert_string(&mut assertion, "metric", draft.metric.trim());
    insert_string(&mut assertion, "relation", draft.relation.trim());
    assertion.insert(
        key("threshold"),
        serde_yaml_ng::to_value(draft.threshold)
            .context("Failed to encode pole-zero threshold.")?,
    );
    Ok(serde_yaml_ng::Value::Mapping(assertion))
}

fn scenario_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    scenario_name: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let scenarios = ensure_sequence_field_mut(yaml, "scenarios")?;
    for scenario in scenarios {
        let Some(mapping) = scenario.as_mapping_mut() else {
            continue;
        };
        let is_target = mapping
            .get(key("name"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(scenario_name);
        if is_target {
            return Ok(mapping);
        }
    }
    anyhow::bail!("Scenario {scenario_name} was not found.");
}

fn ensure_sequence_field_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    field: &str,
) -> Result<&'a mut Vec<serde_yaml_ng::Value>> {
    let mapping = yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let key = key(field);
    if !mapping.contains_key(&key) {
        mapping.insert(key.clone(), serde_yaml_ng::Value::Sequence(Vec::new()));
    }
    mapping
        .get_mut(&key)
        .expect("field was inserted when absent")
        .as_sequence_mut()
        .with_context(|| format!("Board IR field {field} must be a list."))
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

fn validated_id<'a>(value: &'a str, label: &str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{label} must not be blank.");
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        anyhow::bail!("{label} {value} contains unsupported characters.");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_pole_zero_assertion_emits_valid_analysis_yaml() {
        let edited =
            append_analog_pole_zero_assertion(pole_zero_project_yaml(), &pole_zero_draft())
                .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = &project.scenarios[0]
            .analog
            .as_ref()
            .unwrap()
            .analysis
            .pole_zero_assertions;

        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].name, "stable_pole");
        assert_eq!(
            assertions[0].root_kind,
            crate::board_ir::AnalogPoleZeroRootKind::Pole
        );
        assert_eq!(assertions[0].root_index, Some(1));
        assert_eq!(
            assertions[0].metric,
            crate::board_ir::AnalogPoleZeroMetric::RealRadPerS
        );
        assert_eq!(
            assertions[0].relation,
            crate::board_ir::AnalogRelation::Below
        );
        assert_eq!(assertions[0].threshold, -500.0);
    }

    #[test]
    fn append_pole_zero_assertion_allows_omitted_root_index() {
        let mut draft = pole_zero_draft();
        draft.root_index = None;
        let edited = append_analog_pole_zero_assertion(pole_zero_project_yaml(), &draft).unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();

        assert_eq!(
            project.scenarios[0]
                .analog
                .as_ref()
                .unwrap()
                .analysis
                .pole_zero_assertions[0]
                .root_index,
            None
        );
    }

    #[test]
    fn unique_pole_zero_assertion_name_suffixes_collisions() {
        let edited =
            append_analog_pole_zero_assertion(pole_zero_project_yaml(), &pole_zero_draft())
                .unwrap();
        let name =
            unique_analog_pole_zero_assertion_name(&edited, "pz_out", "stable_pole").unwrap();

        assert_eq!(name, "stable_pole_2");
    }

    fn pole_zero_draft() -> AnalogPoleZeroAssertionDraft {
        AnalogPoleZeroAssertionDraft {
            scenario_name: "pz_out".to_string(),
            assertion_name: "stable_pole".to_string(),
            root_kind: "pole".to_string(),
            root_index: Some(1),
            metric: "real_rad_per_s".to_string(),
            relation: "below".to_string(),
            threshold: -500.0,
        }
    }

    fn pole_zero_project_yaml() -> &'static str {
        r#"
project:
  name: pole_zero_gui_test
  version: 0.1.0
board:
  components: {}
  nets: {}
models: {}
scenarios:
  - name: pz_out
    type: analog_pole_zero
    checks:
      - SPICE_POLE_ZERO_ANALYSIS
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      stimuli: []
      probes: []
      assertions: []
      analysis:
        type: pz
        pole_zero_output_node: out
        pole_zero_reference_node: "0"
        pole_zero_input_source: V1
        pole_zero_mode: poles_and_zeros
"#
    }
}
