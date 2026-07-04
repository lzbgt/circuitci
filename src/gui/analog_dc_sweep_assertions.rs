use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub(super) struct AnalogDcSweepAssertionDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
    pub(super) probe: String,
    pub(super) aggregation: String,
    pub(super) relation: String,
    pub(super) threshold: f64,
    pub(super) at_sweep_value: Option<f64>,
}

pub(super) fn append_analog_dc_sweep_assertion(
    text: &str,
    draft: &AnalogDcSweepAssertionDraft,
) -> Result<String> {
    validate_dc_sweep_assertion_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    if scenario.scenario_type != "analog_dc_sweep" {
        anyhow::bail!("DC sweep assertions require an analog_dc_sweep scenario.");
    }
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if !analog
        .probes
        .iter()
        .any(|probe| probe.name == draft.probe.trim())
    {
        anyhow::bail!(
            "DC sweep assertion {} references unknown probe {}.",
            draft.assertion_name,
            draft.probe
        );
    }
    if analog
        .assertions
        .iter()
        .any(|assertion| assertion.name == draft.assertion_name)
        || analog
            .analysis
            .dc_sweep_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
    {
        anyhow::bail!(
            "DC sweep assertion {} already exists in scenario {}.",
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
        "dc_sweep_assertions",
        "DC sweep assertions",
    )?;
    assertions.push(dc_sweep_assertion_value(draft)?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited DC sweep assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn unique_analog_dc_sweep_assertion_name(
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
            .dc_sweep_assertions
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

fn validate_dc_sweep_assertion_draft(draft: &AnalogDcSweepAssertionDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    validated_id(&draft.probe, "probe name")?;
    if !matches!(
        draft.aggregation.as_str(),
        "min" | "max" | "mean" | "sample"
    ) {
        anyhow::bail!("DC sweep assertion aggregation must be min, max, mean, or sample.");
    }
    if !matches!(draft.relation.as_str(), "above" | "below") {
        anyhow::bail!("DC sweep assertion relation must be above or below.");
    }
    if !draft.threshold.is_finite() {
        anyhow::bail!("DC sweep assertion threshold must be finite.");
    }
    match draft.aggregation.as_str() {
        "sample" => {
            if !draft.at_sweep_value.is_some_and(f64::is_finite) {
                anyhow::bail!("DC sweep sample assertion requires finite at_sweep_value.");
            }
        }
        _ => {
            if draft.at_sweep_value.is_some() {
                anyhow::bail!("DC sweep min/max/mean assertions must not declare at_sweep_value.");
            }
        }
    }
    Ok(())
}

fn dc_sweep_assertion_value(draft: &AnalogDcSweepAssertionDraft) -> Result<serde_yaml_ng::Value> {
    let mut assertion = serde_yaml_ng::Mapping::new();
    insert_string(&mut assertion, "name", draft.assertion_name.trim());
    insert_string(&mut assertion, "probe", draft.probe.trim());
    insert_string(&mut assertion, "aggregation", draft.aggregation.trim());
    insert_string(&mut assertion, "relation", draft.relation.trim());
    assertion.insert(
        key("threshold"),
        serde_yaml_ng::to_value(draft.threshold).context("Failed to encode DC sweep threshold.")?,
    );
    if let Some(at_sweep_value) = draft.at_sweep_value {
        assertion.insert(
            key("at_sweep_value"),
            serde_yaml_ng::to_value(at_sweep_value)
                .context("Failed to encode DC sweep sample point.")?,
        );
    }
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
    fn append_dc_sweep_assertion_emits_valid_analysis_yaml() {
        let edited =
            append_analog_dc_sweep_assertion(dc_sweep_project_yaml(), &sample_draft()).unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let assertions = &project.scenarios[0]
            .analog
            .as_ref()
            .unwrap()
            .analysis
            .dc_sweep_assertions;

        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].name, "out_sample_below_limit");
        assert_eq!(assertions[0].probe, "out_voltage");
        assert_eq!(
            assertions[0].aggregation,
            crate::board_ir::AnalogDcSweepAggregation::Sample
        );
        assert_eq!(
            assertions[0].relation,
            crate::board_ir::AnalogRelation::Below
        );
        assert_eq!(assertions[0].threshold, 0.4);
        assert_eq!(assertions[0].at_sweep_value, Some(1.0));
    }

    #[test]
    fn append_dc_sweep_assertion_rejects_sample_without_sweep_value() {
        let mut draft = sample_draft();
        draft.at_sweep_value = None;

        let error = append_analog_dc_sweep_assertion(dc_sweep_project_yaml(), &draft).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("sample assertion requires finite at_sweep_value")
        );
    }

    #[test]
    fn append_dc_sweep_assertion_rejects_unknown_probe() {
        let mut draft = sample_draft();
        draft.probe = "missing_probe".to_string();

        let error = append_analog_dc_sweep_assertion(dc_sweep_project_yaml(), &draft).unwrap_err();

        assert!(error.to_string().contains("references unknown probe"));
    }

    #[test]
    fn unique_dc_sweep_assertion_name_suffixes_existing_names() {
        let edited =
            append_analog_dc_sweep_assertion(dc_sweep_project_yaml(), &sample_draft()).unwrap();

        let name =
            unique_analog_dc_sweep_assertion_name(&edited, "sweep_out", "out_sample_below_limit")
                .unwrap();

        assert_eq!(name, "out_sample_below_limit_2");
    }

    fn sample_draft() -> AnalogDcSweepAssertionDraft {
        AnalogDcSweepAssertionDraft {
            scenario_name: "sweep_out".to_string(),
            assertion_name: "out_sample_below_limit".to_string(),
            probe: "out_voltage".to_string(),
            aggregation: "sample".to_string(),
            relation: "below".to_string(),
            threshold: 0.4,
            at_sweep_value: Some(1.0),
        }
    }

    fn dc_sweep_project_yaml() -> &'static str {
        r#"project: { name: gui_dc_sweep_assertion, version: 0.1.0 }
board:
  parts:
    R1:
      model: generic.analog.resistor
      pins: { A: out, B: gnd }
      spice: { primitive: resistor, value_ohm: 1000 }
    V1:
      model: generic.analog.vsource
      pins: { P: out, N: gnd }
      spice: { primitive: vsource, dc_voltage: 0.0 }
  nets:
    out: { kind: digital_or_analog }
    gnd: { kind: ground }
scenarios:
  - name: sweep_out
    type: analog_dc_sweep
    analog:
      backend: ngspice
      model_files: []
      generated:
        ground_net: gnd
        components: []
      node_bindings:
        - { node: "0", net: gnd }
        - { node: out, net: OUT }
      pin_bindings:
        - { node: out, endpoint: { component: R1, pin: A } }
      analysis:
        type: dc_sweep
        dc_sweep_source: V1
        dc_sweep_start: 0.0
        dc_sweep_stop: 1.0
        dc_sweep_step: 0.5
      stimuli: []
      probes:
        - { name: out_voltage, expression: V(out), quantity: voltage }
      assertions: []
"#
    }
}
