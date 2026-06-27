use anyhow::{Context, Result};
use std::path::Path;

use super::analog_assertion_kinds::validate_assertion_scenario_type;
use super::analog_branches::{current_probe_expression, power_probe_expression};
#[cfg(test)]
pub(super) use super::analog_run_setup::append_analog_transient_scenario;
pub(super) use super::analog_run_setup::{
    AnalogAcScenarioDraft, AnalogScenarioDraft, append_analog_ac_scenario_with_project_path,
    append_analog_transient_scenario_with_project_path,
};

#[derive(Debug, Clone)]
pub(super) struct AnalogAssertionDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
    pub(super) probe_name: String,
    pub(super) reference_probe: String,
    pub(super) aggregation: String,
    pub(super) relation: String,
    pub(super) threshold: f64,
    pub(super) reference_threshold: f64,
    pub(super) target: f64,
    pub(super) tolerance: f64,
    pub(super) at_us: f64,
    pub(super) at_hz: f64,
    pub(super) start_us: f64,
    pub(super) end_us: f64,
    pub(super) time_limit_us: f64,
    pub(super) frequency_limit_hz: f64,
    pub(super) duty_limit_percent: f64,
    pub(super) count_limit: f64,
    pub(super) overshoot_limit_percent: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogProbeDraft {
    pub(super) scenario_name: String,
    pub(super) net_id: String,
    pub(super) probe_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogCurrentProbeDraft {
    pub(super) scenario_name: String,
    pub(super) component_id: String,
    pub(super) probe_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogPowerProbeDraft {
    pub(super) scenario_name: String,
    pub(super) component_id: String,
    pub(super) probe_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogExpressionProbeDraft {
    pub(super) scenario_name: String,
    pub(super) probe_name: String,
    pub(super) expression: String,
    pub(super) quantity: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogProbeRemoveDraft {
    pub(super) scenario_name: String,
    pub(super) probe_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogProbeAssertionsRemoveDraft {
    pub(super) scenario_name: String,
    pub(super) probe_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogAssertionRemoveDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogAssertionReplaceDraft {
    pub(super) scenario_name: String,
    pub(super) original_assertion_name: String,
    pub(super) replacement: AnalogAssertionDraft,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogScenarioChoice {
    pub(super) name: String,
    pub(super) scenario_type: String,
    pub(super) stop_time_us: f64,
    pub(super) probes: Vec<AnalogProbeChoice>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogProbeChoice {
    pub(super) name: String,
    pub(super) quantity: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AnalogAssertionUiStatus {
    Unknown,
    Pass,
    Fail,
}

impl AnalogAssertionUiStatus {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct AnalogProbeAssertionSummary {
    pub(super) name: String,
    pub(super) aggregation: String,
    pub(super) relation: String,
    pub(super) threshold: String,
    pub(super) timing: String,
    pub(super) draft: AnalogAssertionDraft,
    pub(super) status: AnalogAssertionUiStatus,
    pub(super) failure_message: Option<String>,
}

pub(super) fn analog_scenario_choices(text: &str) -> Result<Vec<AnalogScenarioChoice>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    Ok(project
        .scenarios
        .iter()
        .filter_map(|scenario| {
            let analog = scenario.analog.as_ref()?;
            Some(AnalogScenarioChoice {
                name: scenario.name.clone(),
                scenario_type: scenario.scenario_type.clone(),
                stop_time_us: analog.analysis.stop_time_us,
                probes: analog
                    .probes
                    .iter()
                    .map(|probe| AnalogProbeChoice {
                        name: probe.name.clone(),
                        quantity: quantity_label(&probe.quantity).to_string(),
                    })
                    .collect(),
            })
        })
        .collect())
}

pub(super) fn analog_probe_assertion_summaries(
    text: &str,
    report: Option<&crate::reports::ValidationReport>,
    scenario_name: &str,
    probe_name: &str,
) -> Result<Vec<AnalogProbeAssertionSummary>> {
    let scenario_name = validated_id(scenario_name, "scenario name")?;
    let probe_name = validated_id(probe_name, "probe name")?;
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
    let probe = analog
        .probes
        .iter()
        .find(|probe| probe.name == probe_name)
        .with_context(|| {
            format!("Probe {probe_name} was not found in scenario {scenario_name}.")
        })?;
    Ok(analog
        .assertions
        .iter()
        .filter(|assertion| assertion.probe == probe.name)
        .map(|assertion| {
            let failure = report.and_then(|report| {
                report.failures.iter().find(|finding| {
                    finding.scenario == scenario_name
                        && finding_mentions_assertion(finding, &assertion.name)
                })
            });
            let non_assertion_failure = report.is_some_and(|report| {
                report.failures.iter().any(|finding| {
                    finding.scenario == scenario_name && !finding.message.contains("assertion ")
                })
            });
            let status = if failure.is_some() {
                AnalogAssertionUiStatus::Fail
            } else if report.is_none() || non_assertion_failure {
                AnalogAssertionUiStatus::Unknown
            } else {
                AnalogAssertionUiStatus::Pass
            };
            AnalogProbeAssertionSummary {
                name: assertion.name.clone(),
                aggregation: aggregation_label(&assertion.aggregation).to_string(),
                relation: relation_label(&assertion.relation).to_string(),
                threshold: assertion_threshold_label(assertion, &probe.quantity),
                timing: assertion_timing_label(assertion),
                draft: AnalogAssertionDraft {
                    scenario_name: scenario_name.to_string(),
                    assertion_name: assertion.name.clone(),
                    probe_name: assertion.probe.clone(),
                    reference_probe: assertion.reference_probe.clone().unwrap_or_default(),
                    aggregation: aggregation_label(&assertion.aggregation).to_string(),
                    relation: relation_label(&assertion.relation).to_string(),
                    threshold: assertion_threshold_value(assertion, &probe.quantity)
                        .unwrap_or_default(),
                    reference_threshold: assertion_reference_threshold_value(assertion)
                        .unwrap_or_default(),
                    target: assertion_target_value(assertion, &probe.quantity).unwrap_or_default(),
                    tolerance: assertion_tolerance_value(assertion, &probe.quantity)
                        .unwrap_or_default(),
                    at_us: assertion.at_us.unwrap_or_default(),
                    at_hz: assertion.at_hz.unwrap_or_default(),
                    start_us: assertion.start_us.unwrap_or_default(),
                    end_us: assertion.end_us.unwrap_or_default(),
                    time_limit_us: assertion.time_limit_us.unwrap_or_default(),
                    frequency_limit_hz: assertion.frequency_limit_hz.unwrap_or_default(),
                    duty_limit_percent: assertion.duty_limit_percent.unwrap_or_default(),
                    count_limit: assertion.count_limit.unwrap_or_default(),
                    overshoot_limit_percent: assertion.overshoot_limit_percent.unwrap_or_default(),
                },
                status,
                failure_message: failure.map(|finding| finding.message.clone()),
            }
        })
        .collect())
}

pub(super) fn append_analog_assertion(text: &str, draft: &AnalogAssertionDraft) -> Result<String> {
    validate_assertion_draft(draft)?;
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
    validate_assertion_scenario_type(draft, &scenario.scenario_type)?;
    if analog
        .assertions
        .iter()
        .any(|assertion| assertion.name == draft.assertion_name)
    {
        anyhow::bail!(
            "Analog assertion {} already exists in scenario {}.",
            draft.assertion_name,
            scenario.name
        );
    }
    let probe = analog
        .probes
        .iter()
        .find(|probe| probe.name == draft.probe_name)
        .with_context(|| {
            format!(
                "Probe {} was not found in scenario {}.",
                draft.probe_name, scenario.name
            )
        })?;
    let reference_quantity = reference_timing_quantity(draft, analog)?;
    validate_assertion_probe_quantity(draft, &probe.quantity)?;
    validate_assertion_timing(draft, analog.analysis.stop_time_us)?;

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let assertions = ensure_child_sequence_mut(analog_mapping, "assertions", "analog assertions")?;
    assertions.push(assertion_value(
        draft,
        &probe.quantity,
        reference_quantity.as_ref(),
    )?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn append_analog_voltage_probe(text: &str, draft: &AnalogProbeDraft) -> Result<String> {
    validate_probe_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.nets.contains_key(&draft.net_id) {
        anyhow::bail!("Probe net {} was not found.", draft.net_id);
    }
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if analog
        .probes
        .iter()
        .any(|probe| probe.name == draft.probe_name)
    {
        anyhow::bail!(
            "Analog probe {} already exists in scenario {}.",
            draft.probe_name,
            scenario.name
        );
    }
    let node = analog
        .node_bindings
        .iter()
        .find(|binding| binding.net == draft.net_id)
        .map(|binding| binding.node.clone())
        .with_context(|| {
            format!(
                "Scenario {} has no node binding for net {}.",
                scenario.name, draft.net_id
            )
        })?;

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let probes = ensure_child_sequence_mut(analog_mapping, "probes", "analog probes")?;
    let mut probe = serde_yaml_ng::Mapping::new();
    insert_string(&mut probe, "name", draft.probe_name.trim());
    insert_string(&mut probe, "expression", &format!("V({node})"));
    insert_string(&mut probe, "quantity", "voltage");
    probes.push(serde_yaml_ng::Value::Mapping(probe));
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog probe YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn append_analog_current_probe(
    text: &str,
    project_path: &Path,
    draft: &AnalogCurrentProbeDraft,
) -> Result<String> {
    validate_current_probe_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.components.contains_key(&draft.component_id) {
        anyhow::bail!("Probe component {} was not found.", draft.component_id);
    }
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if analog.netlist_source != crate::board_ir::AnalogNetlistSource::GeneratedFromBoard {
        anyhow::bail!(
            "Canvas current probes require a generated_from_board analog scenario; scenario {} uses a file-backed deck.",
            scenario.name
        );
    }
    let generated = analog.generated.as_ref().with_context(|| {
        format!(
            "Scenario {} must declare analog.generated before component current probes can be added.",
            scenario.name
        )
    })?;
    if !generated
        .components
        .iter()
        .any(|component_id| component_id == &draft.component_id)
    {
        anyhow::bail!(
            "Scenario {} does not include generated component {}.",
            scenario.name,
            draft.component_id
        );
    }
    if analog
        .probes
        .iter()
        .any(|probe| probe.name == draft.probe_name)
    {
        anyhow::bail!(
            "Analog probe {} already exists in scenario {}.",
            draft.probe_name,
            scenario.name
        );
    }
    let expression = current_probe_expression(&project, project_path, &draft.component_id)?;

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let probes = ensure_child_sequence_mut(analog_mapping, "probes", "analog probes")?;
    let mut probe = serde_yaml_ng::Mapping::new();
    insert_string(&mut probe, "name", draft.probe_name.trim());
    insert_string(&mut probe, "expression", &expression);
    insert_string(&mut probe, "quantity", "current");
    probes.push(serde_yaml_ng::Value::Mapping(probe));
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog current probe YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn append_analog_power_probe(
    text: &str,
    project_path: &Path,
    draft: &AnalogPowerProbeDraft,
) -> Result<String> {
    validate_power_probe_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = generated_component_probe_scenario(
        &project,
        &draft.scenario_name,
        &draft.component_id,
        &draft.probe_name,
    )?;
    let analog = scenario
        .analog
        .as_ref()
        .expect("generated_component_probe_scenario validates analog presence");
    let expression = power_probe_expression(&project, project_path, analog, &draft.component_id)?;

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let probes = ensure_child_sequence_mut(analog_mapping, "probes", "analog probes")?;
    let mut probe = serde_yaml_ng::Mapping::new();
    insert_string(&mut probe, "name", draft.probe_name.trim());
    insert_string(&mut probe, "expression", &expression);
    insert_string(&mut probe, "quantity", "power");
    probes.push(serde_yaml_ng::Value::Mapping(probe));
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog power probe YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn append_analog_expression_probe(
    text: &str,
    draft: &AnalogExpressionProbeDraft,
) -> Result<String> {
    validate_expression_probe_draft(draft)?;
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
    if analog
        .probes
        .iter()
        .any(|probe| probe.name == draft.probe_name)
    {
        anyhow::bail!(
            "Analog probe {} already exists in scenario {}.",
            draft.probe_name,
            scenario.name
        );
    }
    validate_probe_contract(draft.expression.trim(), draft.quantity.trim())?;

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let probes = ensure_child_sequence_mut(analog_mapping, "probes", "analog probes")?;
    let mut probe = serde_yaml_ng::Mapping::new();
    insert_string(&mut probe, "name", draft.probe_name.trim());
    insert_string(&mut probe, "expression", draft.expression.trim());
    insert_string(&mut probe, "quantity", draft.quantity.trim());
    probes.push(serde_yaml_ng::Value::Mapping(probe));
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog expression probe YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn remove_analog_probe(text: &str, draft: &AnalogProbeRemoveDraft) -> Result<String> {
    validate_probe_remove_draft(draft)?;
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
        .probes
        .iter()
        .any(|probe| probe.name == draft.probe_name)
    {
        anyhow::bail!(
            "Analog probe {} was not found in scenario {}.",
            draft.probe_name,
            scenario.name
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let probes = ensure_child_sequence_mut(analog_mapping, "probes", "analog probes")?;
    let before_probe_count = probes.len();
    probes.retain(|probe| {
        probe
            .as_mapping()
            .and_then(|mapping| mapping.get(key("name")))
            .and_then(serde_yaml_ng::Value::as_str)
            != Some(draft.probe_name.as_str())
    });
    if probes.len() == before_probe_count {
        anyhow::bail!(
            "Analog probe {} was not found in scenario {}.",
            draft.probe_name,
            draft.scenario_name
        );
    }
    if let Some(assertions) = analog_mapping
        .get_mut(key("assertions"))
        .and_then(serde_yaml_ng::Value::as_sequence_mut)
    {
        assertions.retain(|assertion| {
            assertion
                .as_mapping()
                .and_then(|mapping| mapping.get(key("probe")))
                .and_then(serde_yaml_ng::Value::as_str)
                != Some(draft.probe_name.as_str())
        });
    }
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog probe YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn remove_analog_assertions_for_probe(
    text: &str,
    draft: &AnalogProbeAssertionsRemoveDraft,
) -> Result<String> {
    validate_probe_assertions_remove_draft(draft)?;
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
        .probes
        .iter()
        .any(|probe| probe.name == draft.probe_name)
    {
        anyhow::bail!(
            "Analog probe {} was not found in scenario {}.",
            draft.probe_name,
            scenario.name
        );
    }
    if !analog
        .assertions
        .iter()
        .any(|assertion| assertion.probe == draft.probe_name)
    {
        anyhow::bail!(
            "No analog assertions reference probe {} in scenario {}.",
            draft.probe_name,
            scenario.name
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let assertions = ensure_child_sequence_mut(analog_mapping, "assertions", "analog assertions")?;
    assertions.retain(|assertion| {
        assertion
            .as_mapping()
            .and_then(|mapping| mapping.get(key("probe")))
            .and_then(serde_yaml_ng::Value::as_str)
            != Some(draft.probe_name.as_str())
    });
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn remove_analog_assertion(
    text: &str,
    draft: &AnalogAssertionRemoveDraft,
) -> Result<String> {
    validate_assertion_remove_draft(draft)?;
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
        .assertions
        .iter()
        .any(|assertion| assertion.name == draft.assertion_name)
    {
        anyhow::bail!(
            "Analog assertion {} was not found in scenario {}.",
            draft.assertion_name,
            scenario.name
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let assertions = ensure_child_sequence_mut(analog_mapping, "assertions", "analog assertions")?;
    let before_assertion_count = assertions.len();
    assertions.retain(|assertion| {
        assertion
            .as_mapping()
            .and_then(|mapping| mapping.get(key("name")))
            .and_then(serde_yaml_ng::Value::as_str)
            != Some(draft.assertion_name.as_str())
    });
    if assertions.len() == before_assertion_count {
        anyhow::bail!(
            "Analog assertion {} was not found in scenario {}.",
            draft.assertion_name,
            draft.scenario_name
        );
    }
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn replace_analog_assertion(
    text: &str,
    draft: &AnalogAssertionReplaceDraft,
) -> Result<String> {
    validate_assertion_replace_draft(draft)?;
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
    validate_assertion_scenario_type(&draft.replacement, &scenario.scenario_type)?;
    if !analog
        .assertions
        .iter()
        .any(|assertion| assertion.name == draft.original_assertion_name)
    {
        anyhow::bail!(
            "Analog assertion {} was not found in scenario {}.",
            draft.original_assertion_name,
            scenario.name
        );
    }
    if draft.original_assertion_name != draft.replacement.assertion_name
        && analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == draft.replacement.assertion_name)
    {
        anyhow::bail!(
            "Analog assertion {} already exists in scenario {}.",
            draft.replacement.assertion_name,
            scenario.name
        );
    }
    let probe = analog
        .probes
        .iter()
        .find(|probe| probe.name == draft.replacement.probe_name)
        .with_context(|| {
            format!(
                "Probe {} was not found in scenario {}.",
                draft.replacement.probe_name, scenario.name
            )
        })?;
    let reference_quantity = reference_timing_quantity(&draft.replacement, analog)?;
    validate_assertion_probe_quantity(&draft.replacement, &probe.quantity)?;
    validate_assertion_timing(&draft.replacement, analog.analysis.stop_time_us)?;

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let assertions = ensure_child_sequence_mut(analog_mapping, "assertions", "analog assertions")?;
    let mut replaced = false;
    for assertion in assertions.iter_mut() {
        let is_target = assertion
            .as_mapping()
            .and_then(|mapping| mapping.get(key("name")))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(draft.original_assertion_name.as_str());
        if is_target {
            *assertion = assertion_value(
                &draft.replacement,
                &probe.quantity,
                reference_quantity.as_ref(),
            )?;
            replaced = true;
            break;
        }
    }
    if !replaced {
        anyhow::bail!(
            "Analog assertion {} was not found in scenario {}.",
            draft.original_assertion_name,
            draft.scenario_name
        );
    }
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn unique_analog_assertion_name(
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
    let existing: std::collections::BTreeSet<&str> = analog
        .assertions
        .iter()
        .map(|assertion| assertion.name.as_str())
        .collect();
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

fn validate_assertion_draft(draft: &AnalogAssertionDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if !matches!(
        draft.aggregation.as_str(),
        "sample"
            | "min"
            | "max"
            | "mean"
            | "rms"
            | "integral"
            | "energy"
            | "settling_time"
            | "overshoot_percent"
            | "rising_phase_delay"
            | "falling_phase_delay"
            | "rising_setup_time"
            | "rising_hold_time"
            | "falling_setup_time"
            | "falling_hold_time"
            | "rising_crossing_time"
            | "falling_crossing_time"
            | "min_high_pulse_width"
            | "min_low_pulse_width"
            | "duty_cycle"
            | "crossing_count"
            | "rising_crossing_count"
            | "falling_crossing_count"
            | "gain_db_at_frequency"
            | "phase_deg_at_frequency"
            | "rising_gain_crossing_frequency"
            | "falling_gain_crossing_frequency"
            | "phase_margin_deg"
            | "gain_margin_db"
    ) {
        anyhow::bail!(
            "Analog assertion aggregation {} is not supported.",
            draft.aggregation
        );
    }
    if !matches!(draft.relation.as_str(), "above" | "below") {
        anyhow::bail!(
            "Analog assertion relation {} is not supported.",
            draft.relation
        );
    }
    if !draft.threshold.is_finite() {
        anyhow::bail!("Analog assertion threshold must be finite.");
    }
    if matches!(
        draft.aggregation.as_str(),
        "rising_phase_delay"
            | "falling_phase_delay"
            | "rising_setup_time"
            | "rising_hold_time"
            | "falling_setup_time"
            | "falling_hold_time"
    ) {
        validated_id(&draft.reference_probe, "reference probe")?;
        if !draft.reference_threshold.is_finite() {
            anyhow::bail!("Analog assertion reference threshold must be finite.");
        }
    }
    Ok(())
}

fn validate_probe_draft(draft: &AnalogProbeDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.net_id.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    Ok(())
}

fn validate_current_probe_draft(draft: &AnalogCurrentProbeDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.component_id, "component id")?;
    validated_id(&draft.probe_name, "probe name")?;
    Ok(())
}

fn validate_power_probe_draft(draft: &AnalogPowerProbeDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.component_id, "component id")?;
    validated_id(&draft.probe_name, "probe name")?;
    Ok(())
}

fn validate_expression_probe_draft(draft: &AnalogExpressionProbeDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.expression.trim().is_empty() {
        anyhow::bail!("Analog probe expression must not be blank.");
    }
    if !matches!(draft.quantity.trim(), "voltage" | "current" | "power") {
        anyhow::bail!("Analog probe quantity {} is not supported.", draft.quantity);
    }
    Ok(())
}

fn validate_probe_contract(expression: &str, quantity: &str) -> Result<()> {
    let normalized = expression.trim().to_ascii_lowercase().replace(' ', "");
    let valid = match quantity {
        "voltage" => normalized.starts_with("v("),
        "current" => {
            normalized.starts_with("i(")
                || normalized.starts_with("-i(")
                || normalized.starts_with("abs(i(")
        }
        "power" => {
            normalized.contains("v(") && normalized.contains("i(") && normalized.contains('*')
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        anyhow::bail!(
            "Expression {expression} is not consistent with declared {quantity} quantity."
        );
    }
}

fn validate_probe_remove_draft(draft: &AnalogProbeRemoveDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    Ok(())
}

fn validate_probe_assertions_remove_draft(draft: &AnalogProbeAssertionsRemoveDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    Ok(())
}

fn validate_assertion_remove_draft(draft: &AnalogAssertionRemoveDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    Ok(())
}

fn validate_assertion_replace_draft(draft: &AnalogAssertionReplaceDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.original_assertion_name, "assertion name")?;
    if draft.replacement.scenario_name != draft.scenario_name {
        anyhow::bail!("Replacement assertion scenario must match the edited scenario.");
    }
    validate_assertion_draft(&draft.replacement)?;
    Ok(())
}

fn generated_component_probe_scenario<'a>(
    project: &'a crate::board_ir::BoardProject,
    scenario_name: &str,
    component_id: &str,
    probe_name: &str,
) -> Result<&'a crate::board_ir::Scenario> {
    if !project.board.components.contains_key(component_id) {
        anyhow::bail!("Probe component {component_id} was not found.");
    }
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Scenario {scenario_name} was not found."))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if analog.netlist_source != crate::board_ir::AnalogNetlistSource::GeneratedFromBoard {
        anyhow::bail!(
            "Canvas component probes require a generated_from_board analog scenario; scenario {} uses a file-backed deck.",
            scenario.name
        );
    }
    let generated = analog.generated.as_ref().with_context(|| {
        format!(
            "Scenario {} must declare analog.generated before component probes can be added.",
            scenario.name
        )
    })?;
    if !generated
        .components
        .iter()
        .any(|generated_component_id| generated_component_id == component_id)
    {
        anyhow::bail!(
            "Scenario {} does not include generated component {}.",
            scenario.name,
            component_id
        );
    }
    if analog.probes.iter().any(|probe| probe.name == probe_name) {
        anyhow::bail!(
            "Analog probe {} already exists in scenario {}.",
            probe_name,
            scenario.name
        );
    }
    Ok(scenario)
}

fn validate_assertion_timing(draft: &AnalogAssertionDraft, stop_time_us: f64) -> Result<()> {
    match draft.aggregation.as_str() {
        "sample" => {
            if !draft.at_us.is_finite() || draft.at_us < 0.0 || draft.at_us > stop_time_us {
                anyhow::bail!(
                    "Sample assertion time must be finite and within the scenario stop time."
                );
            }
        }
        "min" | "max" | "mean" | "rms" | "integral" | "energy" => {
            if !draft.start_us.is_finite()
                || !draft.end_us.is_finite()
                || draft.start_us < 0.0
                || draft.end_us < draft.start_us
                || draft.end_us > stop_time_us
            {
                anyhow::bail!(
                    "Window assertion bounds must be finite, ordered, and within the scenario stop time."
                );
            }
        }
        "rising_crossing_time"
        | "falling_crossing_time"
        | "min_high_pulse_width"
        | "min_low_pulse_width"
        | "settling_time"
        | "rising_phase_delay"
        | "falling_phase_delay"
        | "rising_setup_time"
        | "rising_hold_time"
        | "falling_setup_time"
        | "falling_hold_time" => {
            if !draft.start_us.is_finite()
                || !draft.end_us.is_finite()
                || draft.start_us < 0.0
                || draft.end_us < draft.start_us
                || draft.end_us > stop_time_us
                || !draft.time_limit_us.is_finite()
                || draft.time_limit_us < 0.0
                || draft.time_limit_us > stop_time_us
            {
                anyhow::bail!(
                    "Timing assertion bounds and time limit must be finite, ordered, and within the scenario stop time."
                );
            }
        }
        "duty_cycle" => {
            if !draft.start_us.is_finite()
                || !draft.end_us.is_finite()
                || draft.start_us < 0.0
                || draft.end_us < draft.start_us
                || draft.end_us > stop_time_us
                || !draft.duty_limit_percent.is_finite()
                || !(0.0..=100.0).contains(&draft.duty_limit_percent)
            {
                anyhow::bail!(
                    "Duty-cycle assertion bounds must be finite and ordered, and duty limit must be 0..100%."
                );
            }
        }
        "crossing_count" | "rising_crossing_count" | "falling_crossing_count" => {
            if !draft.start_us.is_finite()
                || !draft.end_us.is_finite()
                || draft.start_us < 0.0
                || draft.end_us < draft.start_us
                || draft.end_us > stop_time_us
                || !draft.count_limit.is_finite()
                || draft.count_limit < 0.0
            {
                anyhow::bail!(
                    "Crossing-count assertion bounds must be finite and ordered, and count limit must be nonnegative."
                );
            }
        }
        "overshoot_percent" => {
            if !draft.start_us.is_finite()
                || !draft.end_us.is_finite()
                || draft.start_us < 0.0
                || draft.end_us < draft.start_us
                || draft.end_us > stop_time_us
                || !draft.overshoot_limit_percent.is_finite()
                || draft.overshoot_limit_percent < 0.0
            {
                anyhow::bail!(
                    "Overshoot assertion bounds must be finite and ordered, and overshoot limit must be nonnegative."
                );
            }
        }
        "gain_db_at_frequency" | "phase_deg_at_frequency" => {
            if !draft.at_hz.is_finite() || draft.at_hz <= 0.0 {
                anyhow::bail!("AC sample frequency must be finite and positive.");
            }
        }
        "rising_gain_crossing_frequency" | "falling_gain_crossing_frequency" => {
            if !draft.frequency_limit_hz.is_finite() || draft.frequency_limit_hz <= 0.0 {
                anyhow::bail!("AC crossing frequency limit must be finite and positive.");
            }
        }
        "phase_margin_deg" | "gain_margin_db" => {}
        _ => unreachable!("aggregation was validated"),
    }
    Ok(())
}

fn validate_assertion_probe_quantity(
    draft: &AnalogAssertionDraft,
    quantity: &crate::board_ir::AnalogQuantity,
) -> Result<()> {
    if draft.aggregation == "energy" && !matches!(quantity, crate::board_ir::AnalogQuantity::Power)
    {
        anyhow::bail!("Energy assertions require a power probe.");
    }
    Ok(())
}

fn reference_timing_quantity(
    draft: &AnalogAssertionDraft,
    analog: &crate::board_ir::AnalogScenario,
) -> Result<Option<crate::board_ir::AnalogQuantity>> {
    if !matches!(
        draft.aggregation.as_str(),
        "rising_phase_delay"
            | "falling_phase_delay"
            | "rising_setup_time"
            | "rising_hold_time"
            | "falling_setup_time"
            | "falling_hold_time"
    ) {
        return Ok(None);
    }
    let reference_probe = analog
        .probes
        .iter()
        .find(|probe| probe.name == draft.reference_probe)
        .with_context(|| {
            format!(
                "Reference probe {} was not found in scenario {}.",
                draft.reference_probe, draft.scenario_name
            )
        })?;
    Ok(Some(reference_probe.quantity.clone()))
}

fn aggregation_label(aggregation: &crate::board_ir::AnalogAggregation) -> &'static str {
    match aggregation {
        crate::board_ir::AnalogAggregation::Sample => "sample",
        crate::board_ir::AnalogAggregation::OperatingPoint => "operating_point",
        crate::board_ir::AnalogAggregation::Min => "min",
        crate::board_ir::AnalogAggregation::Max => "max",
        crate::board_ir::AnalogAggregation::Mean => "mean",
        crate::board_ir::AnalogAggregation::Rms => "rms",
        crate::board_ir::AnalogAggregation::Integral => "integral",
        crate::board_ir::AnalogAggregation::Energy => "energy",
        crate::board_ir::AnalogAggregation::SettlingTime => "settling_time",
        crate::board_ir::AnalogAggregation::OvershootPercent => "overshoot_percent",
        crate::board_ir::AnalogAggregation::RisingPhaseDelay => "rising_phase_delay",
        crate::board_ir::AnalogAggregation::FallingPhaseDelay => "falling_phase_delay",
        crate::board_ir::AnalogAggregation::RisingSetupTime => "rising_setup_time",
        crate::board_ir::AnalogAggregation::RisingHoldTime => "rising_hold_time",
        crate::board_ir::AnalogAggregation::FallingSetupTime => "falling_setup_time",
        crate::board_ir::AnalogAggregation::FallingHoldTime => "falling_hold_time",
        crate::board_ir::AnalogAggregation::RisingCrossingTime => "rising_crossing_time",
        crate::board_ir::AnalogAggregation::FallingCrossingTime => "falling_crossing_time",
        crate::board_ir::AnalogAggregation::MinHighPulseWidth => "min_high_pulse_width",
        crate::board_ir::AnalogAggregation::MinLowPulseWidth => "min_low_pulse_width",
        crate::board_ir::AnalogAggregation::DutyCycle => "duty_cycle",
        crate::board_ir::AnalogAggregation::CrossingCount => "crossing_count",
        crate::board_ir::AnalogAggregation::RisingCrossingCount => "rising_crossing_count",
        crate::board_ir::AnalogAggregation::FallingCrossingCount => "falling_crossing_count",
        crate::board_ir::AnalogAggregation::GainDbAtFrequency => "gain_db_at_frequency",
        crate::board_ir::AnalogAggregation::PhaseDegAtFrequency => "phase_deg_at_frequency",
        crate::board_ir::AnalogAggregation::RisingGainCrossingFrequency => {
            "rising_gain_crossing_frequency"
        }
        crate::board_ir::AnalogAggregation::FallingGainCrossingFrequency => {
            "falling_gain_crossing_frequency"
        }
        crate::board_ir::AnalogAggregation::PhaseMarginDeg => "phase_margin_deg",
        crate::board_ir::AnalogAggregation::GainMarginDb => "gain_margin_db",
    }
}

fn relation_label(relation: &crate::board_ir::AnalogRelation) -> &'static str {
    match relation {
        crate::board_ir::AnalogRelation::Above => "above",
        crate::board_ir::AnalogRelation::Below => "below",
    }
}

fn assertion_threshold_label(
    assertion: &crate::board_ir::AnalogAssertion,
    quantity: &crate::board_ir::AnalogQuantity,
) -> String {
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::GainDbAtFrequency
            | crate::board_ir::AnalogAggregation::RisingGainCrossingFrequency
            | crate::board_ir::AnalogAggregation::FallingGainCrossingFrequency
            | crate::board_ir::AnalogAggregation::GainMarginDb
    ) {
        return assertion
            .threshold_db
            .map(|value| format!("{value:.6} dB"))
            .unwrap_or_else(|| "missing gain threshold".to_string());
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::PhaseDegAtFrequency
            | crate::board_ir::AnalogAggregation::PhaseMarginDeg
    ) {
        return assertion
            .threshold_deg
            .map(|value| format!("{value:.6} deg"))
            .unwrap_or_else(|| "missing phase threshold".to_string());
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::Energy
    ) {
        return assertion
            .threshold_j
            .map(|value| format!("{value:.6} J"))
            .unwrap_or_else(|| "missing energy threshold".to_string());
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::Integral
    ) {
        return match quantity {
            crate::board_ir::AnalogQuantity::Voltage => assertion
                .threshold_vs
                .map(|value| format!("{value:.6} V*s"))
                .unwrap_or_else(|| "missing voltage integral threshold".to_string()),
            crate::board_ir::AnalogQuantity::Current => assertion
                .threshold_c
                .map(|value| format!("{value:.6} C"))
                .unwrap_or_else(|| "missing charge threshold".to_string()),
            crate::board_ir::AnalogQuantity::Power => assertion
                .threshold_j
                .map(|value| format!("{value:.6} J"))
                .unwrap_or_else(|| "missing energy threshold".to_string()),
        };
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::SettlingTime
            | crate::board_ir::AnalogAggregation::OvershootPercent
    ) {
        return match quantity {
            crate::board_ir::AnalogQuantity::Voltage => assertion
                .target_v
                .map(|value| format!("target {value:.6} V"))
                .unwrap_or_else(|| "missing voltage target".to_string()),
            crate::board_ir::AnalogQuantity::Current => assertion
                .target_a
                .map(|value| format!("target {value:.6} A"))
                .unwrap_or_else(|| "missing current target".to_string()),
            crate::board_ir::AnalogQuantity::Power => assertion
                .target_w
                .map(|value| format!("target {value:.6} W"))
                .unwrap_or_else(|| "missing power target".to_string()),
        };
    }
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => assertion
            .threshold_v
            .map(|value| format!("{value:.6} V"))
            .unwrap_or_else(|| "missing voltage threshold".to_string()),
        crate::board_ir::AnalogQuantity::Current => assertion
            .threshold_a
            .map(|value| format!("{value:.6} A"))
            .unwrap_or_else(|| "missing current threshold".to_string()),
        crate::board_ir::AnalogQuantity::Power => assertion
            .threshold_w
            .map(|value| format!("{value:.6} W"))
            .unwrap_or_else(|| "missing power threshold".to_string()),
    }
}

fn assertion_threshold_value(
    assertion: &crate::board_ir::AnalogAssertion,
    quantity: &crate::board_ir::AnalogQuantity,
) -> Option<f64> {
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::GainDbAtFrequency
            | crate::board_ir::AnalogAggregation::RisingGainCrossingFrequency
            | crate::board_ir::AnalogAggregation::FallingGainCrossingFrequency
            | crate::board_ir::AnalogAggregation::GainMarginDb
    ) {
        return assertion.threshold_db;
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::PhaseDegAtFrequency
            | crate::board_ir::AnalogAggregation::PhaseMarginDeg
    ) {
        return assertion.threshold_deg;
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::Energy
    ) {
        return assertion.threshold_j;
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::Integral
    ) {
        return match quantity {
            crate::board_ir::AnalogQuantity::Voltage => assertion.threshold_vs,
            crate::board_ir::AnalogQuantity::Current => assertion.threshold_c,
            crate::board_ir::AnalogQuantity::Power => assertion.threshold_j,
        };
    }
    if matches!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::SettlingTime
            | crate::board_ir::AnalogAggregation::OvershootPercent
    ) {
        return match quantity {
            crate::board_ir::AnalogQuantity::Voltage => assertion.target_v,
            crate::board_ir::AnalogQuantity::Current => assertion.target_a,
            crate::board_ir::AnalogQuantity::Power => assertion.target_w,
        };
    }
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => assertion.threshold_v,
        crate::board_ir::AnalogQuantity::Current => assertion.threshold_a,
        crate::board_ir::AnalogQuantity::Power => assertion.threshold_w,
    }
}

fn assertion_target_value(
    assertion: &crate::board_ir::AnalogAssertion,
    quantity: &crate::board_ir::AnalogQuantity,
) -> Option<f64> {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => assertion.target_v,
        crate::board_ir::AnalogQuantity::Current => assertion.target_a,
        crate::board_ir::AnalogQuantity::Power => assertion.target_w,
    }
}

fn assertion_reference_threshold_value(
    assertion: &crate::board_ir::AnalogAssertion,
) -> Option<f64> {
    assertion
        .reference_threshold_v
        .or(assertion.reference_threshold_a)
        .or(assertion.reference_threshold_w)
}

fn assertion_tolerance_value(
    assertion: &crate::board_ir::AnalogAssertion,
    quantity: &crate::board_ir::AnalogQuantity,
) -> Option<f64> {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => assertion.tolerance_v,
        crate::board_ir::AnalogQuantity::Current => assertion.tolerance_a,
        crate::board_ir::AnalogQuantity::Power => assertion.tolerance_w,
    }
}

fn assertion_timing_label(assertion: &crate::board_ir::AnalogAssertion) -> String {
    match assertion.aggregation {
        crate::board_ir::AnalogAggregation::Sample => {
            format!("at {:.6} us", assertion.at_us.unwrap_or_default())
        }
        crate::board_ir::AnalogAggregation::OperatingPoint => "DC operating point".to_string(),
        crate::board_ir::AnalogAggregation::Min
        | crate::board_ir::AnalogAggregation::Max
        | crate::board_ir::AnalogAggregation::Mean
        | crate::board_ir::AnalogAggregation::Rms
        | crate::board_ir::AnalogAggregation::Integral
        | crate::board_ir::AnalogAggregation::Energy => {
            format!(
                "{:.6}..{:.6} us",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::RisingCrossingTime
        | crate::board_ir::AnalogAggregation::FallingCrossingTime
        | crate::board_ir::AnalogAggregation::MinHighPulseWidth
        | crate::board_ir::AnalogAggregation::MinLowPulseWidth
        | crate::board_ir::AnalogAggregation::SettlingTime
        | crate::board_ir::AnalogAggregation::RisingPhaseDelay
        | crate::board_ir::AnalogAggregation::FallingPhaseDelay
        | crate::board_ir::AnalogAggregation::RisingSetupTime
        | crate::board_ir::AnalogAggregation::RisingHoldTime
        | crate::board_ir::AnalogAggregation::FallingSetupTime
        | crate::board_ir::AnalogAggregation::FallingHoldTime => {
            format!(
                "{:.6}..{:.6} us, limit {:.6} us",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default(),
                assertion.time_limit_us.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::OvershootPercent => {
            format!(
                "{:.6}..{:.6} us, limit {:.6}%",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default(),
                assertion.overshoot_limit_percent.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::DutyCycle => {
            format!(
                "{:.6}..{:.6} us, limit {:.6}%",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default(),
                assertion.duty_limit_percent.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::CrossingCount
        | crate::board_ir::AnalogAggregation::RisingCrossingCount
        | crate::board_ir::AnalogAggregation::FallingCrossingCount => {
            format!(
                "{:.6}..{:.6} us, limit {:.6} crossings",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default(),
                assertion.count_limit.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::GainDbAtFrequency
        | crate::board_ir::AnalogAggregation::PhaseDegAtFrequency => {
            format!("{:.6} Hz", assertion.at_hz.unwrap_or_default())
        }
        crate::board_ir::AnalogAggregation::RisingGainCrossingFrequency
        | crate::board_ir::AnalogAggregation::FallingGainCrossingFrequency => {
            format!(
                "threshold {:.6} dB, limit {:.6} Hz",
                assertion.threshold_db.unwrap_or_default(),
                assertion.frequency_limit_hz.unwrap_or_default()
            )
        }
        crate::board_ir::AnalogAggregation::PhaseMarginDeg => "unity-gain crossing".to_string(),
        crate::board_ir::AnalogAggregation::GainMarginDb => "phase -180 deg crossing".to_string(),
    }
}

fn finding_mentions_assertion(finding: &crate::reports::Finding, assertion_name: &str) -> bool {
    finding
        .message
        .contains(&format!("assertion {assertion_name} "))
        || finding
            .message
            .contains(&format!("assertion {assertion_name}."))
        || finding
            .message
            .contains(&format!("assertion {assertion_name}:"))
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

fn assertion_value(
    draft: &AnalogAssertionDraft,
    quantity: &crate::board_ir::AnalogQuantity,
    reference_quantity: Option<&crate::board_ir::AnalogQuantity>,
) -> Result<serde_yaml_ng::Value> {
    let mut assertion = serde_yaml_ng::Mapping::new();
    insert_string(&mut assertion, "name", draft.assertion_name.trim());
    insert_string(&mut assertion, "probe", draft.probe_name.trim());
    if draft.aggregation != "sample" {
        insert_string(&mut assertion, "aggregation", &draft.aggregation);
    }
    match draft.aggregation.as_str() {
        "sample" => insert_number(&mut assertion, "at_us", draft.at_us)?,
        "min" | "max" | "mean" | "rms" | "integral" | "energy" => {
            insert_number(&mut assertion, "start_us", draft.start_us)?;
            insert_number(&mut assertion, "end_us", draft.end_us)?;
        }
        "rising_crossing_time"
        | "falling_crossing_time"
        | "min_high_pulse_width"
        | "min_low_pulse_width"
        | "settling_time"
        | "rising_phase_delay"
        | "falling_phase_delay"
        | "rising_setup_time"
        | "rising_hold_time"
        | "falling_setup_time"
        | "falling_hold_time" => {
            insert_number(&mut assertion, "start_us", draft.start_us)?;
            insert_number(&mut assertion, "end_us", draft.end_us)?;
            insert_number(&mut assertion, "time_limit_us", draft.time_limit_us)?;
        }
        "duty_cycle" => {
            insert_number(&mut assertion, "start_us", draft.start_us)?;
            insert_number(&mut assertion, "end_us", draft.end_us)?;
            insert_number(
                &mut assertion,
                "duty_limit_percent",
                draft.duty_limit_percent,
            )?;
        }
        "crossing_count" | "rising_crossing_count" | "falling_crossing_count" => {
            insert_number(&mut assertion, "start_us", draft.start_us)?;
            insert_number(&mut assertion, "end_us", draft.end_us)?;
            insert_number(&mut assertion, "count_limit", draft.count_limit)?;
        }
        "overshoot_percent" => {
            insert_number(&mut assertion, "start_us", draft.start_us)?;
            insert_number(&mut assertion, "end_us", draft.end_us)?;
            insert_number(
                &mut assertion,
                "overshoot_limit_percent",
                draft.overshoot_limit_percent,
            )?;
        }
        "gain_db_at_frequency" | "phase_deg_at_frequency" => {
            insert_number(&mut assertion, "at_hz", draft.at_hz)?;
        }
        "rising_gain_crossing_frequency" | "falling_gain_crossing_frequency" => {
            insert_number(
                &mut assertion,
                "frequency_limit_hz",
                draft.frequency_limit_hz,
            )?;
        }
        "phase_margin_deg" | "gain_margin_db" => {}
        _ => unreachable!("aggregation was validated"),
    }
    insert_string(&mut assertion, "relation", &draft.relation);
    if draft.aggregation == "settling_time" {
        insert_number(&mut assertion, target_field(quantity), draft.target)?;
        insert_number(&mut assertion, tolerance_field(quantity), draft.tolerance)?;
    } else if draft.aggregation == "overshoot_percent" {
        insert_number(&mut assertion, target_field(quantity), draft.target)?;
    } else if matches!(
        draft.aggregation.as_str(),
        "rising_phase_delay"
            | "falling_phase_delay"
            | "rising_setup_time"
            | "rising_hold_time"
            | "falling_setup_time"
            | "falling_hold_time"
    ) {
        let reference_quantity = reference_quantity
            .context("Reference-timing assertions require a resolved reference probe quantity.")?;
        insert_string(
            &mut assertion,
            "reference_probe",
            draft.reference_probe.trim(),
        );
        insert_number(
            &mut assertion,
            reference_threshold_field(reference_quantity),
            draft.reference_threshold,
        )?;
        insert_number(
            &mut assertion,
            threshold_field(&draft.aggregation, quantity),
            draft.threshold,
        )?;
    } else {
        insert_number(
            &mut assertion,
            threshold_field(&draft.aggregation, quantity),
            draft.threshold,
        )?;
    }
    Ok(serde_yaml_ng::Value::Mapping(assertion))
}

fn threshold_field(aggregation: &str, quantity: &crate::board_ir::AnalogQuantity) -> &'static str {
    if matches!(
        aggregation,
        "gain_db_at_frequency"
            | "rising_gain_crossing_frequency"
            | "falling_gain_crossing_frequency"
            | "gain_margin_db"
    ) {
        return "threshold_db";
    }
    if matches!(aggregation, "phase_deg_at_frequency" | "phase_margin_deg") {
        return "threshold_deg";
    }
    if aggregation == "energy" {
        return "threshold_j";
    }
    if aggregation == "integral" {
        return match quantity {
            crate::board_ir::AnalogQuantity::Voltage => "threshold_vs",
            crate::board_ir::AnalogQuantity::Current => "threshold_c",
            crate::board_ir::AnalogQuantity::Power => "threshold_j",
        };
    }
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "threshold_v",
        crate::board_ir::AnalogQuantity::Current => "threshold_a",
        crate::board_ir::AnalogQuantity::Power => "threshold_w",
    }
}

fn target_field(quantity: &crate::board_ir::AnalogQuantity) -> &'static str {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "target_v",
        crate::board_ir::AnalogQuantity::Current => "target_a",
        crate::board_ir::AnalogQuantity::Power => "target_w",
    }
}

fn tolerance_field(quantity: &crate::board_ir::AnalogQuantity) -> &'static str {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "tolerance_v",
        crate::board_ir::AnalogQuantity::Current => "tolerance_a",
        crate::board_ir::AnalogQuantity::Power => "tolerance_w",
    }
}

fn reference_threshold_field(quantity: &crate::board_ir::AnalogQuantity) -> &'static str {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "reference_threshold_v",
        crate::board_ir::AnalogQuantity::Current => "reference_threshold_a",
        crate::board_ir::AnalogQuantity::Power => "reference_threshold_w",
    }
}

fn quantity_label(quantity: &crate::board_ir::AnalogQuantity) -> &'static str {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "voltage",
        crate::board_ir::AnalogQuantity::Current => "current",
        crate::board_ir::AnalogQuantity::Power => "power",
    }
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

fn insert_number(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: f64) -> Result<()> {
    mapping.insert(
        key(name),
        serde_yaml_ng::to_value(value).context("Failed to encode analog scenario number.")?,
    );
    Ok(())
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}
