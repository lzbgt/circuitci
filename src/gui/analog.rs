use anyhow::{Context, Result};
use std::path::Path;

use super::analog_branches::{current_probe_expression, power_probe_expression};

#[derive(Debug, Clone)]
pub(super) struct AnalogScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) stop_time_us: f64,
    pub(super) max_step_us: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogAssertionDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
    pub(super) probe_name: String,
    pub(super) aggregation: String,
    pub(super) relation: String,
    pub(super) threshold: f64,
    pub(super) at_us: f64,
    pub(super) start_us: f64,
    pub(super) end_us: f64,
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

pub(super) fn append_analog_transient_scenario(
    text: &str,
    draft: &AnalogScenarioDraft,
) -> Result<String> {
    validate_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if project
        .scenarios
        .iter()
        .any(|scenario| scenario.name == draft.name)
    {
        anyhow::bail!("Scenario {} already exists.", draft.name);
    }
    let ground_net = project
        .board
        .nets
        .get(&draft.ground_net)
        .with_context(|| format!("Ground net {} was not found.", draft.ground_net))?;
    if ground_net.kind != crate::board_ir::NetKind::Ground {
        anyhow::bail!("Ground net {} must have kind ground.", draft.ground_net);
    }
    if !project.board.nets.contains_key(&draft.probe_net) {
        anyhow::bail!("Probe net {} was not found.", draft.probe_net);
    }
    if project.board.components.is_empty() {
        anyhow::bail!("Generated analog scenarios require at least one component.");
    }

    let node_by_net = node_bindings_for_project(&project, &draft.ground_net);
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenarios = ensure_sequence_field_mut(&mut yaml, "scenarios")?;
    scenarios.push(analog_scenario_value(&project, draft, &node_by_net)?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&updated).context("Edited scenario YAML is not valid Board IR.")?;
    Ok(updated)
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
                    aggregation: aggregation_label(&assertion.aggregation).to_string(),
                    relation: relation_label(&assertion.relation).to_string(),
                    threshold: assertion_threshold_value(assertion, &probe.quantity)
                        .unwrap_or_default(),
                    at_us: assertion.at_us.unwrap_or_default(),
                    start_us: assertion.start_us.unwrap_or_default(),
                    end_us: assertion.end_us.unwrap_or_default(),
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
    validate_assertion_timing(draft, analog.analysis.stop_time_us)?;

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let assertions = ensure_child_sequence_mut(analog_mapping, "assertions", "analog assertions")?;
    assertions.push(assertion_value(draft, &probe.quantity)?);
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
            *assertion = assertion_value(&draft.replacement, &probe.quantity)?;
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

fn validate_draft(draft: &AnalogScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    if !draft.stop_time_us.is_finite()
        || !draft.max_step_us.is_finite()
        || draft.stop_time_us <= 0.0
        || draft.max_step_us <= 0.0
        || draft.max_step_us > draft.stop_time_us
    {
        anyhow::bail!(
            "Analog transient stop time and max step must be finite and positive, and max step must not exceed stop time."
        );
    }
    Ok(())
}

fn validate_assertion_draft(draft: &AnalogAssertionDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if !matches!(draft.aggregation.as_str(), "sample" | "min" | "max") {
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
        "min" | "max" => {
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
        _ => unreachable!("aggregation was validated"),
    }
    Ok(())
}

fn aggregation_label(aggregation: &crate::board_ir::AnalogAggregation) -> &'static str {
    match aggregation {
        crate::board_ir::AnalogAggregation::Sample => "sample",
        crate::board_ir::AnalogAggregation::Min => "min",
        crate::board_ir::AnalogAggregation::Max => "max",
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
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => assertion.threshold_v,
        crate::board_ir::AnalogQuantity::Current => assertion.threshold_a,
        crate::board_ir::AnalogQuantity::Power => assertion.threshold_w,
    }
}

fn assertion_timing_label(assertion: &crate::board_ir::AnalogAssertion) -> String {
    match assertion.aggregation {
        crate::board_ir::AnalogAggregation::Sample => {
            format!("at {:.6} us", assertion.at_us.unwrap_or_default())
        }
        crate::board_ir::AnalogAggregation::Min | crate::board_ir::AnalogAggregation::Max => {
            format!(
                "{:.6}..{:.6} us",
                assertion.start_us.unwrap_or_default(),
                assertion.end_us.unwrap_or_default()
            )
        }
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

fn node_bindings_for_project(
    project: &crate::board_ir::BoardProject,
    ground_net: &str,
) -> std::collections::BTreeMap<String, String> {
    let mut used_nodes = std::collections::BTreeSet::new();
    let mut node_by_net = std::collections::BTreeMap::new();
    for net in project.board.nets.keys() {
        let node = if net == ground_net {
            "0".to_string()
        } else {
            unique_node_name(net, &mut used_nodes)
        };
        node_by_net.insert(net.clone(), node);
    }
    node_by_net
}

fn unique_node_name(net: &str, used_nodes: &mut std::collections::BTreeSet<String>) -> String {
    let base = sanitize_spice_node(net);
    let mut candidate = base.clone();
    let mut suffix = 2usize;
    while !used_nodes.insert(candidate.clone()) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    candidate
}

fn sanitize_spice_node(value: &str) -> String {
    let mut node = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            node.push(character);
        } else if !node.ends_with('_') {
            node.push('_');
        }
    }
    let node = node.trim_matches('_');
    if node.is_empty() {
        "n".to_string()
    } else if node
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        format!("n_{node}")
    } else {
        node.to_string()
    }
}

fn analog_scenario_value(
    project: &crate::board_ir::BoardProject,
    draft: &AnalogScenarioDraft,
    node_by_net: &std::collections::BTreeMap<String, String>,
) -> Result<serde_yaml_ng::Value> {
    let mut scenario = serde_yaml_ng::Mapping::new();
    insert_string(&mut scenario, "name", draft.name.trim());
    insert_string(&mut scenario, "type", "analog_transient");
    scenario.insert(
        key("checks"),
        serde_yaml_ng::Value::Sequence(vec![serde_yaml_ng::Value::String(
            "SPICE_TRANSIENT_ANALYSIS".to_string(),
        )]),
    );
    scenario.insert(
        key("analog"),
        serde_yaml_ng::Value::Mapping(analog_block(project, draft, node_by_net)?),
    );
    Ok(serde_yaml_ng::Value::Mapping(scenario))
}

fn analog_block(
    project: &crate::board_ir::BoardProject,
    draft: &AnalogScenarioDraft,
    node_by_net: &std::collections::BTreeMap<String, String>,
) -> Result<serde_yaml_ng::Mapping> {
    let mut analog = serde_yaml_ng::Mapping::new();
    insert_string(&mut analog, "backend", "auto");
    insert_string(&mut analog, "netlist_source", "generated_from_board");

    let mut generated = serde_yaml_ng::Mapping::new();
    insert_string(&mut generated, "ground_net", &draft.ground_net);
    generated.insert(
        key("components"),
        serde_yaml_ng::Value::Sequence(
            project
                .board
                .components
                .keys()
                .map(|component| serde_yaml_ng::Value::String(component.clone()))
                .collect(),
        ),
    );
    analog.insert(key("generated"), serde_yaml_ng::Value::Mapping(generated));
    analog.insert(
        key("model_files"),
        serde_yaml_ng::Value::Sequence(Vec::new()),
    );

    analog.insert(
        key("node_bindings"),
        serde_yaml_ng::Value::Sequence(
            node_by_net
                .iter()
                .map(|(net, node)| {
                    mapping_value([("node", node.as_str()), ("net", net.as_str())].into_iter())
                })
                .collect(),
        ),
    );
    analog.insert(
        key("pin_bindings"),
        serde_yaml_ng::Value::Sequence(pin_bindings(project, node_by_net)?),
    );

    let mut analysis = serde_yaml_ng::Mapping::new();
    insert_string(&mut analysis, "type", "tran");
    insert_number(&mut analysis, "stop_time_us", draft.stop_time_us)?;
    insert_number(&mut analysis, "max_step_us", draft.max_step_us)?;
    analog.insert(key("analysis"), serde_yaml_ng::Value::Mapping(analysis));
    analog.insert(key("stimuli"), serde_yaml_ng::Value::Sequence(Vec::new()));

    let probe_node = node_by_net
        .get(&draft.probe_net)
        .with_context(|| format!("Probe net {} has no generated SPICE node.", draft.probe_net))?;
    let mut probe = serde_yaml_ng::Mapping::new();
    insert_string(&mut probe, "name", draft.probe_name.trim());
    insert_string(&mut probe, "expression", &format!("V({probe_node})"));
    analog.insert(
        key("probes"),
        serde_yaml_ng::Value::Sequence(vec![serde_yaml_ng::Value::Mapping(probe)]),
    );
    analog.insert(
        key("assertions"),
        serde_yaml_ng::Value::Sequence(Vec::new()),
    );
    Ok(analog)
}

fn pin_bindings(
    project: &crate::board_ir::BoardProject,
    node_by_net: &std::collections::BTreeMap<String, String>,
) -> Result<Vec<serde_yaml_ng::Value>> {
    let mut bindings = Vec::new();
    for (component_id, component) in &project.board.components {
        for (pin_id, net) in &component.pins {
            let node = node_by_net.get(net).with_context(|| {
                format!("Component {component_id}.{pin_id} references unknown net {net}.")
            })?;
            let mut endpoint = serde_yaml_ng::Mapping::new();
            insert_string(&mut endpoint, "component", component_id);
            insert_string(&mut endpoint, "pin", pin_id);

            let mut binding = serde_yaml_ng::Mapping::new();
            insert_string(&mut binding, "node", node);
            binding.insert(key("endpoint"), serde_yaml_ng::Value::Mapping(endpoint));
            bindings.push(serde_yaml_ng::Value::Mapping(binding));
        }
    }
    Ok(bindings)
}

fn assertion_value(
    draft: &AnalogAssertionDraft,
    quantity: &crate::board_ir::AnalogQuantity,
) -> Result<serde_yaml_ng::Value> {
    let mut assertion = serde_yaml_ng::Mapping::new();
    insert_string(&mut assertion, "name", draft.assertion_name.trim());
    insert_string(&mut assertion, "probe", draft.probe_name.trim());
    if draft.aggregation != "sample" {
        insert_string(&mut assertion, "aggregation", &draft.aggregation);
    }
    match draft.aggregation.as_str() {
        "sample" => insert_number(&mut assertion, "at_us", draft.at_us)?,
        "min" | "max" => {
            insert_number(&mut assertion, "start_us", draft.start_us)?;
            insert_number(&mut assertion, "end_us", draft.end_us)?;
        }
        _ => unreachable!("aggregation was validated"),
    }
    insert_string(&mut assertion, "relation", &draft.relation);
    insert_number(&mut assertion, threshold_field(quantity), draft.threshold)?;
    Ok(serde_yaml_ng::Value::Mapping(assertion))
}

fn threshold_field(quantity: &crate::board_ir::AnalogQuantity) -> &'static str {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => "threshold_v",
        crate::board_ir::AnalogQuantity::Current => "threshold_a",
        crate::board_ir::AnalogQuantity::Power => "threshold_w",
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

fn mapping_value<'a>(pairs: impl Iterator<Item = (&'a str, &'a str)>) -> serde_yaml_ng::Value {
    let mut mapping = serde_yaml_ng::Mapping::new();
    for (name, value) in pairs {
        insert_string(&mut mapping, name, value);
    }
    serde_yaml_ng::Value::Mapping(mapping)
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
