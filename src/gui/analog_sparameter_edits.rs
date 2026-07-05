use anyhow::{Context, Result};

use super::analog_yaml_edit::{
    child_mapping_mut, ensure_child_sequence_mut, insert_number, insert_string, key,
    scenario_mapping_mut, validated_id,
};

#[derive(Debug, Clone)]
pub(super) struct AnalogSParameterNetworkAssertionDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
    pub(super) metric: String,
    pub(super) relation: String,
    pub(super) threshold: f64,
    pub(super) source_reflection: Option<AnalogSParameterReflectionDraft>,
    pub(super) load_reflection: Option<AnalogSParameterReflectionDraft>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AnalogSParameterReflectionDraft {
    pub(super) real: f64,
    pub(super) imaginary: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSParameterAssertionDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
    pub(super) parameter: String,
    pub(super) metric: String,
    pub(super) aggregation: String,
    pub(super) relation: String,
    pub(super) threshold: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSParameterNoiseAssertionDraft {
    pub(super) scenario_name: String,
    pub(super) assertion_name: String,
    pub(super) metric: String,
    pub(super) relation: String,
    pub(super) threshold: f64,
}

pub(super) fn append_analog_sparameter_network_assertion(
    text: &str,
    draft: &AnalogSParameterNetworkAssertionDraft,
) -> Result<String> {
    validate_sparameter_network_assertion_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    if scenario.scenario_type != "analog_sparameter" {
        anyhow::bail!("S-parameter network assertions require an analog_sparameter scenario.");
    }
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if analog.analysis.s_parameter_ports.len() != 2 {
        anyhow::bail!(
            "S-parameter network assertions require exactly two declared S-parameter ports."
        );
    }
    if analog
        .assertions
        .iter()
        .any(|assertion| assertion.name == draft.assertion_name)
        || analog
            .analysis
            .s_parameter_network_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
    {
        anyhow::bail!(
            "S-parameter network assertion {} already exists in scenario {}.",
            draft.assertion_name,
            scenario.name
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenario_mapping = scenario_mapping_mut(&mut yaml, &draft.scenario_name)?;
    let analog_mapping = child_mapping_mut(scenario_mapping, "analog", "analog scenario")?;
    let analysis_mapping = child_mapping_mut(analog_mapping, "analysis", "analog analysis")?;
    if let Some(source_reflection) = draft.source_reflection {
        analysis_mapping.insert(
            key("s_parameter_source_reflection"),
            sparameter_reflection_value(source_reflection)?,
        );
    }
    if let Some(load_reflection) = draft.load_reflection {
        analysis_mapping.insert(
            key("s_parameter_load_reflection"),
            sparameter_reflection_value(load_reflection)?,
        );
    }
    let assertions = ensure_child_sequence_mut(
        analysis_mapping,
        "s_parameter_network_assertions",
        "S-parameter network assertions",
    )?;
    assertions.push(sparameter_network_assertion_value(draft)?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited S-parameter network assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn append_analog_sparameter_assertion(
    text: &str,
    draft: &AnalogSParameterAssertionDraft,
) -> Result<String> {
    validate_sparameter_assertion_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    if scenario.scenario_type != "analog_sparameter" {
        anyhow::bail!("S-parameter assertions require an analog_sparameter scenario.");
    }
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    let (output_port, input_port) = parse_sparameter_parameter(&draft.parameter)
        .with_context(|| format!("S-parameter {} must look like s11 or s21.", draft.parameter))?;
    let port_count = analog.analysis.s_parameter_ports.len();
    if output_port == 0 || input_port == 0 || output_port > port_count || input_port > port_count {
        anyhow::bail!(
            "S-parameter assertion parameter {} is outside the declared {port_count}-port matrix.",
            draft.parameter
        );
    }
    if matches!(
        draft.metric.as_str(),
        "return_loss_db"
            | "vswr"
            | "mismatch_loss_db"
            | "impedance_real_ohm"
            | "impedance_imag_ohm"
            | "impedance_magnitude_ohm"
    ) && output_port != input_port
    {
        anyhow::bail!(
            "S-parameter assertion metric {} requires a reflection parameter such as s11.",
            draft.metric
        );
    }
    if draft.metric == "insertion_loss_db" && output_port == input_port {
        anyhow::bail!(
            "S-parameter assertion metric insertion_loss_db requires a transmission parameter such as s21."
        );
    }
    if analog
        .assertions
        .iter()
        .any(|assertion| assertion.name == draft.assertion_name)
        || analog
            .analysis
            .s_parameter_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
        || analog
            .analysis
            .s_parameter_network_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
        || analog
            .analysis
            .s_parameter_noise_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
    {
        anyhow::bail!(
            "S-parameter assertion {} already exists in scenario {}.",
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
        "s_parameter_assertions",
        "S-parameter assertions",
    )?;
    assertions.push(sparameter_assertion_value(draft)?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited S-parameter assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn append_analog_sparameter_noise_assertion(
    text: &str,
    draft: &AnalogSParameterNoiseAssertionDraft,
) -> Result<String> {
    validate_sparameter_noise_assertion_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    if scenario.scenario_type != "analog_sparameter" {
        anyhow::bail!("S-parameter noise assertions require an analog_sparameter scenario.");
    }
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if analog.analysis.s_parameter_ports.len() != 2 {
        anyhow::bail!(
            "S-parameter noise assertions require exactly two declared S-parameter ports."
        );
    }
    if analog
        .assertions
        .iter()
        .any(|assertion| assertion.name == draft.assertion_name)
        || analog
            .analysis
            .s_parameter_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
        || analog
            .analysis
            .s_parameter_network_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
        || analog
            .analysis
            .s_parameter_noise_assertions
            .iter()
            .any(|assertion| assertion.name == draft.assertion_name)
    {
        anyhow::bail!(
            "S-parameter noise assertion {} already exists in scenario {}.",
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
        "s_parameter_noise_assertions",
        "S-parameter noise assertions",
    )?;
    assertions.push(sparameter_noise_assertion_value(draft)?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited S-parameter noise assertion YAML is not valid Board IR.")?;
    Ok(updated)
}

pub(super) fn unique_analog_sparameter_network_assertion_name(
    text: &str,
    scenario_name: &str,
    requested_name: &str,
) -> Result<String> {
    unique_sparameter_assertion_name(text, scenario_name, requested_name)
}

pub(super) fn unique_analog_sparameter_assertion_name(
    text: &str,
    scenario_name: &str,
    requested_name: &str,
) -> Result<String> {
    unique_sparameter_assertion_name(text, scenario_name, requested_name)
}

pub(super) fn unique_analog_sparameter_noise_assertion_name(
    text: &str,
    scenario_name: &str,
    requested_name: &str,
) -> Result<String> {
    unique_sparameter_assertion_name(text, scenario_name, requested_name)
}

fn unique_sparameter_assertion_name(
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
            .s_parameter_assertions
            .iter()
            .map(|assertion| assertion.name.as_str()),
    );
    existing.extend(
        analog
            .analysis
            .s_parameter_network_assertions
            .iter()
            .map(|assertion| assertion.name.as_str()),
    );
    existing.extend(
        analog
            .analysis
            .s_parameter_noise_assertions
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

fn validate_sparameter_assertion_draft(draft: &AnalogSParameterAssertionDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    if parse_sparameter_parameter(&draft.parameter).is_none() {
        anyhow::bail!("S-parameter assertion parameter must look like s11 or s21.");
    }
    if !matches!(
        draft.metric.as_str(),
        "magnitude_db"
            | "magnitude_linear"
            | "return_loss_db"
            | "insertion_loss_db"
            | "vswr"
            | "mismatch_loss_db"
            | "group_delay_s"
            | "impedance_real_ohm"
            | "impedance_imag_ohm"
            | "impedance_magnitude_ohm"
    ) {
        anyhow::bail!("Unsupported S-parameter assertion metric {}.", draft.metric);
    }
    if !matches!(draft.aggregation.as_str(), "min" | "max") {
        anyhow::bail!("S-parameter assertion aggregation must be min or max.");
    }
    if !matches!(draft.relation.as_str(), "above" | "below") {
        anyhow::bail!("S-parameter assertion relation must be above or below.");
    }
    if !draft.threshold.is_finite() {
        anyhow::bail!("S-parameter assertion threshold must be finite.");
    }
    Ok(())
}

fn validate_sparameter_noise_assertion_draft(
    draft: &AnalogSParameterNoiseAssertionDraft,
) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    if !matches!(
        draft.metric.as_str(),
        "noise_figure_db_max"
            | "minimum_noise_figure_db_max"
            | "equivalent_noise_resistance_ohm_max"
            | "optimum_source_reflection_magnitude_max"
    ) {
        anyhow::bail!(
            "Unsupported S-parameter noise assertion metric {}.",
            draft.metric
        );
    }
    if !matches!(draft.relation.as_str(), "above" | "below") {
        anyhow::bail!("S-parameter noise assertion relation must be above or below.");
    }
    if !draft.threshold.is_finite() {
        anyhow::bail!("S-parameter noise assertion threshold must be finite.");
    }
    Ok(())
}

fn validate_sparameter_network_assertion_draft(
    draft: &AnalogSParameterNetworkAssertionDraft,
) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.assertion_name, "assertion name")?;
    if !matches!(
        draft.metric.as_str(),
        "reciprocity_error_linear"
            | "passivity_max_singular_value"
            | "rollet_k_min"
            | "stability_delta_magnitude_max"
            | "maximum_available_gain_db_min"
            | "maximum_stable_gain_db_min"
            | "maximum_unilateral_gain_db_min"
            | "transducer_gain_db_min"
            | "available_gain_db_min"
            | "operating_gain_db_min"
    ) {
        anyhow::bail!(
            "Unsupported S-parameter network assertion metric {}.",
            draft.metric
        );
    }
    if !matches!(draft.relation.as_str(), "above" | "below") {
        anyhow::bail!("S-parameter network assertion relation must be above or below.");
    }
    if !draft.threshold.is_finite() {
        anyhow::bail!("S-parameter network assertion threshold must be finite.");
    }
    validate_sparameter_reflection_draft(draft.source_reflection, "source")?;
    validate_sparameter_reflection_draft(draft.load_reflection, "load")?;
    if matches!(
        draft.metric.as_str(),
        "transducer_gain_db_min" | "available_gain_db_min"
    ) && draft.source_reflection.is_none()
    {
        anyhow::bail!(
            "S-parameter network metric {} requires source reflection coefficients.",
            draft.metric
        );
    }
    if matches!(
        draft.metric.as_str(),
        "transducer_gain_db_min" | "operating_gain_db_min"
    ) && draft.load_reflection.is_none()
    {
        anyhow::bail!(
            "S-parameter network metric {} requires load reflection coefficients.",
            draft.metric
        );
    }
    Ok(())
}

fn validate_sparameter_reflection_draft(
    draft: Option<AnalogSParameterReflectionDraft>,
    label: &str,
) -> Result<()> {
    let Some(draft) = draft else {
        return Ok(());
    };
    if !draft.real.is_finite() || !draft.imaginary.is_finite() {
        anyhow::bail!("S-parameter {label} reflection coefficients must be finite.");
    }
    let magnitude_squared = draft
        .real
        .mul_add(draft.real, draft.imaginary * draft.imaginary);
    if magnitude_squared >= 1.0 {
        anyhow::bail!("S-parameter {label} reflection magnitude must be below 1.");
    }
    Ok(())
}

fn sparameter_network_assertion_value(
    draft: &AnalogSParameterNetworkAssertionDraft,
) -> Result<serde_yaml_ng::Value> {
    let mut assertion = serde_yaml_ng::Mapping::new();
    insert_string(&mut assertion, "name", draft.assertion_name.trim());
    insert_string(&mut assertion, "metric", draft.metric.trim());
    insert_string(&mut assertion, "relation", draft.relation.trim());
    insert_number(&mut assertion, "threshold", draft.threshold)?;
    Ok(serde_yaml_ng::Value::Mapping(assertion))
}

fn sparameter_reflection_value(
    draft: AnalogSParameterReflectionDraft,
) -> Result<serde_yaml_ng::Value> {
    let mut reflection = serde_yaml_ng::Mapping::new();
    insert_number(&mut reflection, "real", draft.real)?;
    insert_number(&mut reflection, "imaginary", draft.imaginary)?;
    Ok(serde_yaml_ng::Value::Mapping(reflection))
}

fn sparameter_assertion_value(
    draft: &AnalogSParameterAssertionDraft,
) -> Result<serde_yaml_ng::Value> {
    let mut assertion = serde_yaml_ng::Mapping::new();
    insert_string(&mut assertion, "name", draft.assertion_name.trim());
    insert_string(
        &mut assertion,
        "parameter",
        &draft.parameter.trim().to_ascii_lowercase(),
    );
    insert_string(&mut assertion, "metric", draft.metric.trim());
    insert_string(&mut assertion, "aggregation", draft.aggregation.trim());
    insert_string(&mut assertion, "relation", draft.relation.trim());
    insert_number(&mut assertion, "threshold", draft.threshold)?;
    Ok(serde_yaml_ng::Value::Mapping(assertion))
}

fn sparameter_noise_assertion_value(
    draft: &AnalogSParameterNoiseAssertionDraft,
) -> Result<serde_yaml_ng::Value> {
    let mut assertion = serde_yaml_ng::Mapping::new();
    insert_string(&mut assertion, "name", draft.assertion_name.trim());
    insert_string(&mut assertion, "metric", draft.metric.trim());
    insert_string(&mut assertion, "relation", draft.relation.trim());
    insert_number(&mut assertion, "threshold", draft.threshold)?;
    Ok(serde_yaml_ng::Value::Mapping(assertion))
}

fn parse_sparameter_parameter(parameter: &str) -> Option<(usize, usize)> {
    let parameter = parameter.trim().to_ascii_lowercase();
    let suffix = parameter.strip_prefix('s')?;
    if suffix.len() != 2 || !suffix.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let mut digits = suffix.chars();
    let output = digits.next()?.to_digit(10)? as usize;
    let input = digits.next()?.to_digit(10)? as usize;
    Some((output, input))
}
