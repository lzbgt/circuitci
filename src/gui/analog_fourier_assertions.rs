use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub(super) struct AnalogFourierAssertionDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
    pub(super) harmonic: Option<u32>,
    pub(super) metric: String,
    pub(super) relation: String,
    pub(super) threshold: f64,
}

pub(super) fn append_analog_fourier_assertion(
    text: &str,
    draft: &AnalogFourierAssertionDraft,
) -> Result<String> {
    validate_fourier_assertion_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    if scenario.scenario_type != "analog_fourier" {
        anyhow::bail!("Fourier assertions require an analog_fourier scenario.");
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
            .fourier_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
    {
        anyhow::bail!(
            "Fourier assertion {} already exists in scenario {}.",
            draft.assertion_name,
            scenario.name
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let analysis_mapping = child_mapping_mut(analog_mapping, "analysis", "analog analysis")?;
    let assertions =
        ensure_child_sequence_mut(analysis_mapping, "fourier_assertions", "Fourier assertions")?;
    assertions.push(fourier_assertion_value(draft)?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited Fourier assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn unique_analog_fourier_assertion_name(
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
            .fourier_assertions
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

fn validate_fourier_assertion_draft(draft: &AnalogFourierAssertionDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    if !matches!(
        draft.metric.as_str(),
        "magnitude" | "normalized_magnitude" | "phase_deg" | "normalized_phase_deg" | "thd_percent"
    ) {
        anyhow::bail!("Unsupported Fourier assertion metric {}.", draft.metric);
    }
    if draft.metric == "thd_percent" {
        if draft.harmonic.is_some() {
            anyhow::bail!("Fourier THD assertions must omit harmonic.");
        }
    } else if !matches!(draft.harmonic, Some(0..=1024)) {
        anyhow::bail!("Fourier harmonic assertions require harmonic in 0..=1024.");
    }
    if !matches!(draft.relation.as_str(), "above" | "below") {
        anyhow::bail!("Fourier assertion relation must be above or below.");
    }
    if !draft.threshold.is_finite() {
        anyhow::bail!("Fourier assertion threshold must be finite.");
    }
    Ok(())
}

fn fourier_assertion_value(draft: &AnalogFourierAssertionDraft) -> Result<serde_yaml_ng::Value> {
    let mut assertion = serde_yaml_ng::Mapping::new();
    insert_string(&mut assertion, "name", draft.assertion_name.trim());
    if let Some(harmonic) = draft.harmonic {
        assertion.insert(
            key("harmonic"),
            serde_yaml_ng::to_value(harmonic).context("Failed to encode Fourier harmonic.")?,
        );
    }
    insert_string(&mut assertion, "metric", draft.metric.trim());
    insert_string(&mut assertion, "relation", draft.relation.trim());
    assertion.insert(
        key("threshold"),
        serde_yaml_ng::to_value(draft.threshold).context("Failed to encode Fourier threshold.")?,
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
    fn append_fourier_assertion_emits_valid_analysis_yaml() {
        let edited =
            append_analog_fourier_assertion(fourier_project_yaml(), &fourier_assertion_draft())
                .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = &project.scenarios[0]
            .analog
            .as_ref()
            .unwrap()
            .analysis
            .fourier_assertions;

        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].name, "h2_ratio_ceiling");
        assert_eq!(assertions[0].harmonic, Some(2));
        assert_eq!(
            assertions[0].metric,
            crate::board_ir::AnalogFourierMetric::NormalizedMagnitude
        );
        assert_eq!(
            assertions[0].relation,
            crate::board_ir::AnalogRelation::Below
        );
        assert_eq!(assertions[0].threshold, 0.03);
    }

    #[test]
    fn append_fourier_thd_assertion_omits_harmonic() {
        let mut draft = fourier_assertion_draft();
        draft.assertion_name = "thd_ceiling".to_string();
        draft.metric = "thd_percent".to_string();
        draft.harmonic = None;

        let edited = append_analog_fourier_assertion(fourier_project_yaml(), &draft).unwrap();
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = yaml["scenarios"][0]["analog"]["analysis"]["fourier_assertions"]
            .as_sequence()
            .unwrap();

        assert!(assertions[0]["harmonic"].is_null());
    }

    #[test]
    fn unique_fourier_assertion_name_suffixes_collisions() {
        let edited =
            append_analog_fourier_assertion(fourier_project_yaml(), &fourier_assertion_draft())
                .unwrap();
        let name = unique_analog_fourier_assertion_name(&edited, "fourier_out", "h2_ratio_ceiling")
            .unwrap();

        assert_eq!(name, "h2_ratio_ceiling_2");
    }

    fn fourier_assertion_draft() -> AnalogFourierAssertionDraft {
        AnalogFourierAssertionDraft {
            scenario_name: "fourier_out".to_string(),
            assertion_name: "h2_ratio_ceiling".to_string(),
            harmonic: Some(2),
            metric: "normalized_magnitude".to_string(),
            relation: "below".to_string(),
            threshold: 0.03,
        }
    }

    fn fourier_project_yaml() -> &'static str {
        r#"
project:
  name: fourier_gui_test
  version: 0.1.0
board:
  components: {}
  nets: {}
models: {}
scenarios:
  - name: fourier_out
    type: analog_fourier
    checks:
      - SPICE_FOURIER_ANALYSIS
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      stimuli: []
      probes: []
      assertions: []
      analysis:
        type: fourier
        stop_time_us: 100
        max_step_us: 1
        fourier_fundamental_frequency_hz: 100000
        fourier_output_expression: V(out)
        fourier_harmonics: 10
"#
    }
}
