use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub(super) struct AnalogSensitivityAssertionDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
    pub(super) parameter: String,
    pub(super) frequency_hz: Option<f64>,
    pub(super) metric: String,
    pub(super) relation: String,
    pub(super) threshold: f64,
}

pub(super) fn append_analog_sensitivity_assertion(
    text: &str,
    draft: &AnalogSensitivityAssertionDraft,
) -> Result<String> {
    validate_sensitivity_assertion_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    if scenario.scenario_type != "analog_sensitivity" {
        anyhow::bail!("Sensitivity assertions require an analog_sensitivity scenario.");
    }
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if analog.analysis.sensitivity_mode.as_deref() == Some("dc") && draft.frequency_hz.is_some() {
        anyhow::bail!("DC sensitivity assertions must omit frequency_hz.");
    }
    if analog
        .assertions
        .iter()
        .any(|assertion| assertion.name == draft.assertion_name)
        || analog
            .analysis
            .sensitivity_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
    {
        anyhow::bail!(
            "Sensitivity assertion {} already exists in scenario {}.",
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
        "sensitivity_assertions",
        "sensitivity assertions",
    )?;
    assertions.push(sensitivity_assertion_value(draft)?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited sensitivity assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn unique_analog_sensitivity_assertion_name(
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
            .sensitivity_assertions
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

fn validate_sensitivity_assertion_draft(draft: &AnalogSensitivityAssertionDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    validated_id(&draft.parameter, "sensitivity parameter")?;
    if draft
        .frequency_hz
        .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        anyhow::bail!("Sensitivity assertion frequency must be finite and positive.");
    }
    if !matches!(
        draft.metric.as_str(),
        "sensitivity_real" | "sensitivity_imaginary" | "sensitivity_magnitude"
    ) {
        anyhow::bail!("Unsupported sensitivity assertion metric {}.", draft.metric);
    }
    if !matches!(draft.relation.as_str(), "above" | "below") {
        anyhow::bail!("Sensitivity assertion relation must be above or below.");
    }
    if !draft.threshold.is_finite() {
        anyhow::bail!("Sensitivity assertion threshold must be finite.");
    }
    Ok(())
}

fn sensitivity_assertion_value(
    draft: &AnalogSensitivityAssertionDraft,
) -> Result<serde_yaml_ng::Value> {
    let mut assertion = serde_yaml_ng::Mapping::new();
    insert_string(&mut assertion, "name", draft.assertion_name.trim());
    insert_string(&mut assertion, "parameter", draft.parameter.trim());
    if let Some(frequency_hz) = draft.frequency_hz {
        assertion.insert(
            key("frequency_hz"),
            serde_yaml_ng::to_value(frequency_hz)
                .context("Failed to encode sensitivity frequency.")?,
        );
    }
    insert_string(&mut assertion, "metric", draft.metric.trim());
    insert_string(&mut assertion, "relation", draft.relation.trim());
    assertion.insert(
        key("threshold"),
        serde_yaml_ng::to_value(draft.threshold)
            .context("Failed to encode sensitivity threshold.")?,
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
    fn append_sensitivity_assertion_emits_valid_analysis_yaml() {
        let edited = append_analog_sensitivity_assertion(
            &sensitivity_project_yaml("ac"),
            &sensitivity_assertion_draft(),
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = &project.scenarios[0]
            .analog
            .as_ref()
            .unwrap()
            .analysis
            .sensitivity_assertions;

        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].name, "r1_magnitude_ceiling");
        assert_eq!(assertions[0].parameter, "R1");
        assert_eq!(assertions[0].frequency_hz, Some(100.0));
        assert_eq!(
            assertions[0].metric,
            crate::board_ir::AnalogSensitivityMetric::SensitivityMagnitude
        );
        assert_eq!(
            assertions[0].relation,
            crate::board_ir::AnalogRelation::Below
        );
        assert_eq!(assertions[0].threshold, 0.001);
    }

    #[test]
    fn append_sensitivity_assertion_rejects_dc_frequency() {
        let error = append_analog_sensitivity_assertion(
            &sensitivity_project_yaml("dc"),
            &sensitivity_assertion_draft(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("DC sensitivity"));
    }

    #[test]
    fn unique_sensitivity_assertion_name_suffixes_collisions() {
        let edited = append_analog_sensitivity_assertion(
            &sensitivity_project_yaml("ac"),
            &sensitivity_assertion_draft(),
        )
        .unwrap();
        let name =
            unique_analog_sensitivity_assertion_name(&edited, "sens_out", "r1_magnitude_ceiling")
                .unwrap();

        assert_eq!(name, "r1_magnitude_ceiling_2");
    }

    fn sensitivity_assertion_draft() -> AnalogSensitivityAssertionDraft {
        AnalogSensitivityAssertionDraft {
            scenario_name: "sens_out".to_string(),
            assertion_name: "r1_magnitude_ceiling".to_string(),
            parameter: "R1".to_string(),
            frequency_hz: Some(100.0),
            metric: "sensitivity_magnitude".to_string(),
            relation: "below".to_string(),
            threshold: 0.001,
        }
    }

    fn sensitivity_project_yaml(mode: &str) -> String {
        format!(
            r#"
project:
  name: sensitivity_gui_test
  version: 0.1.0
board:
  components: {{}}
  nets: {{}}
models: {{}}
scenarios:
  - name: sens_out
    type: analog_sensitivity
    checks:
      - SPICE_SENSITIVITY_ANALYSIS
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      stimuli: []
      probes: []
      assertions: []
      analysis:
        type: sens
        sensitivity_mode: {mode}
        sensitivity_output_expression: V(out)
        sensitivity_filters: [R1]
"#
        )
    }
}
