use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub(super) struct AnalogMonteCarloComponentValueDraft {
    pub(super) scenario_name: String,
    pub(super) sweep_name: String,
    pub(super) samples: String,
    pub(super) seed: String,
    pub(super) component: String,
    pub(super) field: String,
    pub(super) nominal: String,
    pub(super) tolerance_percent: String,
    pub(super) distribution: String,
}

struct ParsedMonteCarloComponentValue<'a> {
    component: &'a str,
    field: &'a str,
    nominal: f64,
    tolerance_percent: f64,
    distribution: &'a str,
}

pub(super) fn append_analog_sweep_with_monte_carlo(
    text: &str,
    draft: &AnalogMonteCarloComponentValueDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let samples = parse_monte_carlo_samples(&draft.samples)?;
    let seed = parse_monte_carlo_seed(&draft.seed)?;
    let component_value = parsed_component_value(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Run setup {scenario_name} was not found."))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Run setup {scenario_name} is not analog."))?;
    if analog.sweeps.iter().any(|sweep| sweep.name == sweep_name) {
        anyhow::bail!("Sweep {sweep_name} already exists in run setup {scenario_name}.");
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let analog_mapping = analog_mapping_mut(&mut yaml, scenario_name)?;
    let sweeps = ensure_child_sequence_mut(analog_mapping, "sweeps", "analog sweeps")?;
    let mut sweep = serde_yaml_ng::Mapping::new();
    insert_string(&mut sweep, "name", sweep_name);
    sweep.insert(
        key("monte_carlo"),
        monte_carlo_value(samples, seed, component_value),
    );
    sweeps.push(serde_yaml_ng::Value::Mapping(sweep));
    validate_updated_yaml(yaml)
}

pub(super) fn append_analog_monte_carlo_component_value(
    text: &str,
    draft: &AnalogMonteCarloComponentValueDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let component_value = parsed_component_value(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Run setup {scenario_name} was not found."))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Run setup {scenario_name} is not analog."))?;
    let sweep = analog
        .sweeps
        .iter()
        .find(|sweep| sweep.name == sweep_name)
        .with_context(|| format!("Sweep {sweep_name} was not found."))?;
    let monte_carlo = sweep
        .monte_carlo
        .as_ref()
        .with_context(|| format!("Sweep {sweep_name} has no Monte Carlo block."))?;
    if sweep.component_values.iter().any(|entry| {
        entry.component == component_value.component
            && entry.field.as_str() == component_value.field
    }) || monte_carlo.component_values.iter().any(|entry| {
        entry.component == component_value.component
            && entry.field.as_str() == component_value.field
    }) {
        anyhow::bail!(
            "Monte Carlo component value {}.{} already exists in sweep {sweep_name}.",
            component_value.component,
            component_value.field
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let monte_carlo_mapping = monte_carlo_mapping_mut(&mut yaml, scenario_name, sweep_name)?;
    let component_values = ensure_child_sequence_mut(
        monte_carlo_mapping,
        "component_values",
        "Monte Carlo component values",
    )?;
    component_values.push(monte_carlo_component_value_entry(component_value));
    validate_updated_yaml(yaml)
}

pub(super) fn remove_analog_monte_carlo_component_value(
    text: &str,
    draft: &AnalogMonteCarloComponentValueDraft,
) -> Result<String> {
    let scenario_name = validated_id(&draft.scenario_name, "run setup")?;
    let sweep_name = validated_id(&draft.sweep_name, "sweep name")?;
    let component = validated_component_id(&draft.component)?;
    let field = validated_component_value_field(&draft.field)?;
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let monte_carlo_mapping = monte_carlo_mapping_mut(&mut yaml, scenario_name, sweep_name)?;
    let component_values = ensure_child_sequence_mut(
        monte_carlo_mapping,
        "component_values",
        "Monte Carlo component values",
    )?;
    if component_values.len() <= 1 {
        anyhow::bail!(
            "Sweep {sweep_name} must keep at least one Monte Carlo component value. Remove the sweep instead."
        );
    }
    let before = component_values.len();
    component_values.retain(|component_value| {
        let Some(mapping) = component_value.as_mapping() else {
            return true;
        };
        let same_component = mapping
            .get(key("component"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(component);
        let same_field = mapping
            .get(key("field"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(field);
        !(same_component && same_field)
    });
    if component_values.len() == before {
        anyhow::bail!(
            "Monte Carlo component value {component}.{field} was not found in sweep {sweep_name}."
        );
    }
    validate_updated_yaml(yaml)
}

fn monte_carlo_value(
    samples: usize,
    seed: u64,
    component_value: ParsedMonteCarloComponentValue<'_>,
) -> serde_yaml_ng::Value {
    let mut mapping = serde_yaml_ng::Mapping::new();
    mapping.insert(key("samples"), serde_yaml_ng::Value::Number(samples.into()));
    mapping.insert(key("seed"), serde_yaml_ng::Value::Number(seed.into()));
    mapping.insert(
        key("component_values"),
        serde_yaml_ng::Value::Sequence(vec![monte_carlo_component_value_entry(component_value)]),
    );
    serde_yaml_ng::Value::Mapping(mapping)
}

fn monte_carlo_component_value_entry(
    component_value: ParsedMonteCarloComponentValue<'_>,
) -> serde_yaml_ng::Value {
    let mut mapping = serde_yaml_ng::Mapping::new();
    insert_string(&mut mapping, "component", component_value.component);
    insert_string(&mut mapping, "field", component_value.field);
    mapping.insert(
        key("nominal"),
        serde_yaml_ng::Value::Number(component_value.nominal.into()),
    );
    mapping.insert(
        key("tolerance_percent"),
        serde_yaml_ng::Value::Number(component_value.tolerance_percent.into()),
    );
    insert_string(&mut mapping, "distribution", component_value.distribution);
    serde_yaml_ng::Value::Mapping(mapping)
}

fn parsed_component_value(
    draft: &AnalogMonteCarloComponentValueDraft,
) -> Result<ParsedMonteCarloComponentValue<'_>> {
    let component = validated_component_id(&draft.component)?;
    let field = validated_component_value_field(&draft.field)?;
    let nominal = parse_required_number(&draft.nominal, "Monte Carlo nominal value")?;
    let tolerance_percent =
        parse_required_number(&draft.tolerance_percent, "Monte Carlo tolerance percent")?;
    if tolerance_percent < 0.0 {
        anyhow::bail!("Monte Carlo tolerance percent must be zero or greater.");
    }
    let distribution = validated_distribution(&draft.distribution)?;
    validate_component_value_range(field, nominal)?;
    Ok(ParsedMonteCarloComponentValue {
        component,
        field,
        nominal,
        tolerance_percent,
        distribution,
    })
}

fn parse_monte_carlo_samples(text: &str) -> Result<usize> {
    let text = text.trim();
    let samples: usize = text
        .parse()
        .with_context(|| format!("Monte Carlo samples {text} is not an integer."))?;
    if !(1..=64).contains(&samples) {
        anyhow::bail!("Monte Carlo samples must be in 1..=64.");
    }
    Ok(samples)
}

fn parse_monte_carlo_seed(text: &str) -> Result<u64> {
    let text = text.trim();
    let seed: u64 = text
        .parse()
        .with_context(|| format!("Monte Carlo seed {text} is not an integer."))?;
    Ok(seed)
}

fn parse_required_number(text: &str, label: &str) -> Result<f64> {
    let text = text.trim();
    let value: f64 = text
        .parse()
        .with_context(|| format!("{label} {text} is not a number."))?;
    if !value.is_finite() {
        anyhow::bail!("{label} must be finite.");
    }
    Ok(value)
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

fn validated_component_id(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("Monte Carlo component value target must not be blank.");
    }
    Ok(value)
}

fn validated_component_value_field(value: &str) -> Result<&str> {
    let value = value.trim();
    match value {
        "value_ohm" | "value_f" | "value_h" | "dc_v" | "dc_a" => Ok(value),
        _ => anyhow::bail!(
            "Component value field {value} must be one of value_ohm, value_f, value_h, dc_v, or dc_a."
        ),
    }
}

fn validate_component_value_range(field: &str, nominal: f64) -> Result<()> {
    if matches!(field, "value_ohm" | "value_f" | "value_h") && nominal <= 0.0 {
        anyhow::bail!("Passive Monte Carlo component values must use positive nominals.");
    }
    Ok(())
}

fn validated_distribution(value: &str) -> Result<&str> {
    let value = value.trim();
    match value {
        "uniform" | "normal" => Ok(value),
        _ => anyhow::bail!("Monte Carlo distribution must be uniform or normal."),
    }
}

fn validate_updated_yaml(yaml: serde_yaml_ng::Value) -> Result<String> {
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited analog Monte Carlo sweep YAML is not valid Board IR.")?;
    Ok(updated)
}

fn analog_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    scenario_name: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let scenarios = yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?
        .get_mut(key("scenarios"))
        .context("Board IR project must declare scenarios.")?
        .as_sequence_mut()
        .context("Board IR scenarios must be a YAML list.")?;
    for scenario in scenarios {
        let Some(mapping) = scenario.as_mapping_mut() else {
            continue;
        };
        if mapping
            .get(key("name"))
            .and_then(serde_yaml_ng::Value::as_str)
            != Some(scenario_name)
        {
            continue;
        }
        return mapping
            .get_mut(key("analog"))
            .context("Run setup must declare analog settings.")?
            .as_mapping_mut()
            .context("Run setup analog settings must be a YAML object.");
    }
    anyhow::bail!("Run setup {scenario_name} was not found.")
}

fn monte_carlo_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    scenario_name: &str,
    sweep_name: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let sweep_mapping = sweep_mapping_mut(yaml, scenario_name, sweep_name)?;
    sweep_mapping
        .get_mut(key("monte_carlo"))
        .with_context(|| format!("Sweep {sweep_name} has no Monte Carlo block."))?
        .as_mapping_mut()
        .context("Sweep Monte Carlo block must be a YAML object.")
}

fn sweep_mapping_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    scenario_name: &str,
    sweep_name: &str,
) -> Result<&'a mut serde_yaml_ng::Mapping> {
    let analog_mapping = analog_mapping_mut(yaml, scenario_name)?;
    let sweeps = ensure_child_sequence_mut(analog_mapping, "sweeps", "analog sweeps")?;
    for sweep in sweeps {
        let Some(mapping) = sweep.as_mapping_mut() else {
            continue;
        };
        if mapping
            .get(key("name"))
            .and_then(serde_yaml_ng::Value::as_str)
            == Some(sweep_name)
        {
            return Ok(mapping);
        }
    }
    anyhow::bail!("Sweep {sweep_name} was not found.")
}

fn ensure_child_sequence_mut<'a>(
    mapping: &'a mut serde_yaml_ng::Mapping,
    child: &str,
    label: &str,
) -> Result<&'a mut Vec<serde_yaml_ng::Value>> {
    if !mapping.contains_key(key(child)) {
        mapping.insert(
            key(child),
            serde_yaml_ng::Value::Sequence(Vec::<serde_yaml_ng::Value>::new()),
        );
    }
    mapping
        .get_mut(key(child))
        .with_context(|| format!("{label} was not found."))?
        .as_sequence_mut()
        .with_context(|| format!("{label} must be a YAML list."))
}

fn insert_string(mapping: &mut serde_yaml_ng::Mapping, key_name: &str, value: &str) {
    mapping.insert(
        key(key_name),
        serde_yaml_ng::Value::String(value.to_string()),
    );
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        AnalogMonteCarloComponentValueDraft, append_analog_monte_carlo_component_value,
        append_analog_sweep_with_monte_carlo, remove_analog_monte_carlo_component_value,
    };

    fn generated_load_project_yaml() -> &'static str {
        "project:
  name: gui_monte_carlo_sweep_test
  version: 0.1.0
board:
  components:
    RLOAD:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: out
        B: gnd
    ILOAD:
      model: generic.analog.dc_current_source
      spice:
        primitive: dc_current_source
        dc_a: 0.01
      pins:
        P: out
        N: gnd
  nets:
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
scenarios:
  - name: load_run
    type: analog_transient
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [RLOAD, ILOAD]
      model_files: []
      node_bindings:
        - { node: out, net: out }
        - { node: '0', net: gnd }
      pin_bindings: []
      analysis:
        type: tran
        stop_time_us: 1000
        max_step_us: 1
      stimuli: []
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"
    }

    fn rload_draft() -> AnalogMonteCarloComponentValueDraft {
        AnalogMonteCarloComponentValueDraft {
            scenario_name: "load_run".to_string(),
            sweep_name: "load_monte_carlo".to_string(),
            samples: "16".to_string(),
            seed: "99".to_string(),
            component: "RLOAD".to_string(),
            field: "value_ohm".to_string(),
            nominal: "1000".to_string(),
            tolerance_percent: "5".to_string(),
            distribution: "uniform".to_string(),
        }
    }

    fn iload_draft() -> AnalogMonteCarloComponentValueDraft {
        AnalogMonteCarloComponentValueDraft {
            component: "ILOAD".to_string(),
            field: "dc_a".to_string(),
            nominal: "0.01".to_string(),
            tolerance_percent: "10".to_string(),
            ..rload_draft()
        }
    }

    #[test]
    fn append_monte_carlo_sweep_emits_valid_yaml() {
        let edited =
            append_analog_sweep_with_monte_carlo(generated_load_project_yaml(), &rload_draft())
                .unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
        let monte_carlo = sweep.monte_carlo.as_ref().unwrap();
        assert_eq!(sweep.name, "load_monte_carlo");
        assert_eq!(monte_carlo.samples, 16);
        assert_eq!(monte_carlo.seed, 99);
        assert_eq!(monte_carlo.component_values[0].component, "RLOAD");
        assert_eq!(monte_carlo.component_values[0].field.as_str(), "value_ohm");
        assert_eq!(monte_carlo.component_values[0].nominal, 1000.0);
        assert_eq!(monte_carlo.component_values[0].tolerance_percent, 5.0);
        assert_eq!(
            monte_carlo.component_values[0].distribution.as_str(),
            "uniform"
        );
    }

    #[test]
    fn append_monte_carlo_sweep_accepts_normal_distribution() {
        let draft = AnalogMonteCarloComponentValueDraft {
            distribution: "normal".to_string(),
            ..rload_draft()
        };
        let edited =
            append_analog_sweep_with_monte_carlo(generated_load_project_yaml(), &draft).unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let distribution = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0]
            .monte_carlo
            .as_ref()
            .unwrap()
            .component_values[0]
            .distribution;
        assert_eq!(distribution.as_str(), "normal");
    }

    #[test]
    fn add_and_remove_monte_carlo_component_value_emit_valid_yaml() {
        let edited =
            append_analog_sweep_with_monte_carlo(generated_load_project_yaml(), &rload_draft())
                .unwrap();
        let edited = append_analog_monte_carlo_component_value(&edited, &iload_draft()).unwrap();

        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let monte_carlo = project.scenarios[0].analog.as_ref().unwrap().sweeps[0]
            .monte_carlo
            .as_ref()
            .unwrap();
        assert_eq!(monte_carlo.component_values.len(), 2);
        assert_eq!(monte_carlo.component_values[1].component, "ILOAD");

        let edited = remove_analog_monte_carlo_component_value(&edited, &iload_draft()).unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let monte_carlo = project.scenarios[0].analog.as_ref().unwrap().sweeps[0]
            .monte_carlo
            .as_ref()
            .unwrap();
        assert_eq!(monte_carlo.component_values.len(), 1);
        assert_eq!(monte_carlo.component_values[0].component, "RLOAD");
    }

    #[test]
    fn monte_carlo_component_value_rejects_duplicate_target() {
        let edited =
            append_analog_sweep_with_monte_carlo(generated_load_project_yaml(), &rload_draft())
                .unwrap();
        let error = append_analog_monte_carlo_component_value(&edited, &rload_draft()).unwrap_err();

        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    fn monte_carlo_component_value_remove_rejects_last_target() {
        let edited =
            append_analog_sweep_with_monte_carlo(generated_load_project_yaml(), &rload_draft())
                .unwrap();
        let error = remove_analog_monte_carlo_component_value(&edited, &rload_draft()).unwrap_err();

        assert!(error.to_string().contains("Remove the sweep instead"));
    }
}
