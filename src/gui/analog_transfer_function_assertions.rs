use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub(super) struct AnalogTransferFunctionAssertionDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
    pub(super) metric: String,
    pub(super) relation: String,
    pub(super) threshold: f64,
}

pub(super) fn append_analog_transfer_function_assertion(
    text: &str,
    draft: &AnalogTransferFunctionAssertionDraft,
) -> Result<String> {
    validate_transfer_function_assertion_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    if scenario.scenario_type != "analog_transfer_function" {
        anyhow::bail!("Transfer-function assertions require an analog_transfer_function scenario.");
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
            .transfer_function_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
    {
        anyhow::bail!(
            "Transfer-function assertion {} already exists in scenario {}.",
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
        "transfer_function_assertions",
        "transfer-function assertions",
    )?;
    assertions.push(transfer_function_assertion_value(draft)?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited transfer-function assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn unique_analog_transfer_function_assertion_name(
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
            .transfer_function_assertions
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

fn validate_transfer_function_assertion_draft(
    draft: &AnalogTransferFunctionAssertionDraft,
) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    if !matches!(
        draft.metric.as_str(),
        "transfer_function_gain" | "input_resistance_ohm" | "output_resistance_ohm"
    ) {
        anyhow::bail!(
            "Unsupported transfer-function assertion metric {}.",
            draft.metric
        );
    }
    if !matches!(draft.relation.as_str(), "above" | "below") {
        anyhow::bail!("Transfer-function assertion relation must be above or below.");
    }
    if !draft.threshold.is_finite() {
        anyhow::bail!("Transfer-function assertion threshold must be finite.");
    }
    Ok(())
}

fn transfer_function_assertion_value(
    draft: &AnalogTransferFunctionAssertionDraft,
) -> Result<serde_yaml_ng::Value> {
    let mut assertion = serde_yaml_ng::Mapping::new();
    insert_string(&mut assertion, "name", draft.assertion_name.trim());
    insert_string(&mut assertion, "metric", draft.metric.trim());
    insert_string(&mut assertion, "relation", draft.relation.trim());
    assertion.insert(
        key("threshold"),
        serde_yaml_ng::to_value(draft.threshold)
            .context("Failed to encode transfer-function threshold.")?,
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
    fn append_transfer_function_assertion_emits_valid_analysis_yaml() {
        let edited = append_analog_transfer_function_assertion(
            transfer_function_project_yaml(),
            &transfer_function_assertion_draft(),
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = &project.scenarios[0]
            .analog
            .as_ref()
            .unwrap()
            .analysis
            .transfer_function_assertions;

        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].name, "gain_floor");
        assert_eq!(
            assertions[0].metric,
            crate::board_ir::AnalogTransferFunctionMetric::TransferFunctionGain
        );
        assert_eq!(
            assertions[0].relation,
            crate::board_ir::AnalogRelation::Above
        );
        assert_eq!(assertions[0].threshold, 0.75);
    }

    #[test]
    fn append_transfer_function_assertion_rejects_wrong_scenario_type() {
        let error = append_analog_transfer_function_assertion(
            non_tf_project_yaml(),
            &transfer_function_assertion_draft(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("analog_transfer_function"));
    }

    #[test]
    fn unique_transfer_function_assertion_name_suffixes_collisions() {
        let edited = append_analog_transfer_function_assertion(
            transfer_function_project_yaml(),
            &transfer_function_assertion_draft(),
        )
        .unwrap();
        let name = unique_analog_transfer_function_assertion_name(&edited, "tf_out", "gain_floor")
            .unwrap();

        assert_eq!(name, "gain_floor_2");
    }

    fn transfer_function_assertion_draft() -> AnalogTransferFunctionAssertionDraft {
        AnalogTransferFunctionAssertionDraft {
            scenario_name: "tf_out".to_string(),
            assertion_name: "gain_floor".to_string(),
            metric: "transfer_function_gain".to_string(),
            relation: "above".to_string(),
            threshold: 0.75,
        }
    }

    fn transfer_function_project_yaml() -> &'static str {
        r#"
project:
  name: transfer_function_gui_test
  version: 0.1.0
board:
  components: {}
  nets: {}
models: {}
scenarios:
  - name: tf_out
    type: analog_transfer_function
    checks:
      - SPICE_TRANSFER_FUNCTION_ANALYSIS
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      stimuli: []
      probes: []
      assertions: []
      analysis:
        type: tf
        transfer_output_expression: V(out)
        transfer_input_source: V1
"#
    }

    fn non_tf_project_yaml() -> &'static str {
        r#"
project:
  name: transfer_function_gui_test
  version: 0.1.0
board:
  components: {}
  nets: {}
models: {}
scenarios:
  - name: tf_out
    type: analog_ac
    checks:
      - SPICE_AC_ANALYSIS
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      stimuli: []
      probes: []
      assertions: []
      analysis:
        type: ac
        start_frequency_hz: 1
        stop_frequency_hz: 10
        points_per_decade: 2
"#
    }
}
