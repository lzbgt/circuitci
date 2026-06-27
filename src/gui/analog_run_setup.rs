use anyhow::{Context, Result};
use std::path::Path;

use super::analog_model_files::model_file_values_for_generated_components;

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
pub(super) struct AnalogAcScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) start_frequency_hz: f64,
    pub(super) stop_frequency_hz: f64,
    pub(super) points_per_decade: u32,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogDcScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogNoiseScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) output_probe_name: String,
    pub(super) input_probe_name: String,
    pub(super) input_source: String,
    pub(super) start_frequency_hz: f64,
    pub(super) stop_frequency_hz: f64,
    pub(super) points_per_decade: u32,
}

#[cfg(test)]
pub(super) fn append_analog_transient_scenario(
    text: &str,
    draft: &AnalogScenarioDraft,
) -> Result<String> {
    append_analog_transient_scenario_with_project_path(text, Path::new("project.yaml"), draft)
}

pub(super) fn append_analog_transient_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogScenarioDraft,
) -> Result<String> {
    validate_transient_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Transient {
            stop_time_us: draft.stop_time_us,
            max_step_us: draft.max_step_us,
        },
    )
}

pub(super) fn append_analog_ac_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogAcScenarioDraft,
) -> Result<String> {
    validate_ac_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Ac {
            start_frequency_hz: draft.start_frequency_hz,
            stop_frequency_hz: draft.stop_frequency_hz,
            points_per_decade: draft.points_per_decade,
        },
    )
}

pub(super) fn append_analog_dc_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogDcScenarioDraft,
) -> Result<String> {
    validate_dc_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Dc,
    )
}

pub(super) fn append_analog_noise_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogNoiseScenarioDraft,
) -> Result<String> {
    validate_noise_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.output_probe_name,
        GeneratedAnalogScenarioKind::Noise {
            start_frequency_hz: draft.start_frequency_hz,
            stop_frequency_hz: draft.stop_frequency_hz,
            points_per_decade: draft.points_per_decade,
            input_source: draft.input_source.trim().to_string(),
            input_probe_name: draft.input_probe_name.trim().to_string(),
        },
    )
}

fn append_generated_analog_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    scenario_name: &str,
    ground_net_id: &str,
    probe_net: &str,
    probe_name: &str,
    kind: GeneratedAnalogScenarioKind,
) -> Result<String> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if project
        .scenarios
        .iter()
        .any(|scenario| scenario.name == scenario_name)
    {
        anyhow::bail!("Scenario {} already exists.", scenario_name);
    }
    let ground_net = project
        .board
        .nets
        .get(ground_net_id)
        .with_context(|| format!("Ground net {} was not found.", ground_net_id))?;
    if ground_net.kind != crate::board_ir::NetKind::Ground {
        anyhow::bail!("Ground net {} must have kind ground.", ground_net_id);
    }
    if !project.board.nets.contains_key(probe_net) {
        anyhow::bail!("Probe net {} was not found.", probe_net);
    }
    if let GeneratedAnalogScenarioKind::Noise { input_source, .. } = &kind
        && !noise_input_source_exists(&project, input_source)
    {
        anyhow::bail!(
            "Noise input source {input_source} must be an included voltage or current source component."
        );
    }
    if project.board.components.is_empty() {
        anyhow::bail!("Generated analog scenarios require at least one component.");
    }

    let node_by_net = node_bindings_for_project(&project, ground_net_id);
    let scenario_spec = GeneratedAnalogScenarioSpec {
        name: scenario_name,
        ground_net: ground_net_id,
        probe_net,
        probe_name,
        kind,
    };
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let scenarios = ensure_sequence_field_mut(&mut yaml, "scenarios")?;
    scenarios.push(analog_scenario_value(
        project_path,
        &project,
        &scenario_spec,
        &node_by_net,
    )?);
    let updated =
        serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited Board IR YAML.")?;
    let _: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&updated).context("Edited scenario YAML is not valid Board IR.")?;
    Ok(updated)
}

fn validate_transient_draft(draft: &AnalogScenarioDraft) -> Result<()> {
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
            "Stop time and max step must be finite positive values, with max step no larger than stop time."
        );
    }
    Ok(())
}

fn validate_ac_draft(draft: &AnalogAcScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    if !draft.start_frequency_hz.is_finite()
        || !draft.stop_frequency_hz.is_finite()
        || draft.start_frequency_hz <= 0.0
        || draft.stop_frequency_hz <= draft.start_frequency_hz
        || draft.points_per_decade == 0
        || draft.points_per_decade > 1000
    {
        anyhow::bail!(
            "Analog AC/Bode start and stop frequencies must be finite and positive, stop must exceed start, and points per decade must be in 1..=1000."
        );
    }
    Ok(())
}

fn validate_dc_draft(draft: &AnalogDcScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    Ok(())
}

fn validate_noise_draft(draft: &AnalogNoiseScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.output_probe_name, "output noise probe name")?;
    validated_id(&draft.input_probe_name, "input noise probe name")?;
    validated_id(&draft.input_source, "noise input source")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    validate_frequency_range(
        draft.start_frequency_hz,
        draft.stop_frequency_hz,
        draft.points_per_decade,
        "Analog noise",
    )
}

fn validate_frequency_range(
    start_frequency_hz: f64,
    stop_frequency_hz: f64,
    points_per_decade: u32,
    label: &str,
) -> Result<()> {
    if !start_frequency_hz.is_finite()
        || !stop_frequency_hz.is_finite()
        || start_frequency_hz <= 0.0
        || stop_frequency_hz <= start_frequency_hz
        || points_per_decade == 0
        || points_per_decade > 1000
    {
        anyhow::bail!(
            "{label} start and stop frequencies must be finite and positive, stop must exceed start, and points per decade must be in 1..=1000."
        );
    }
    Ok(())
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
    project_path: &Path,
    project: &crate::board_ir::BoardProject,
    scenario: &GeneratedAnalogScenarioSpec<'_>,
    node_by_net: &std::collections::BTreeMap<String, String>,
) -> Result<serde_yaml_ng::Value> {
    let mut value = serde_yaml_ng::Mapping::new();
    insert_string(&mut value, "name", scenario.name.trim());
    insert_string(&mut value, "type", scenario.kind.scenario_type());
    value.insert(
        key("checks"),
        serde_yaml_ng::Value::Sequence(vec![serde_yaml_ng::Value::String(
            scenario.kind.check_id().to_string(),
        )]),
    );
    value.insert(
        key("analog"),
        serde_yaml_ng::Value::Mapping(analog_block(project_path, project, scenario, node_by_net)?),
    );
    Ok(serde_yaml_ng::Value::Mapping(value))
}

#[derive(Debug, Clone)]
struct GeneratedAnalogScenarioSpec<'a> {
    name: &'a str,
    ground_net: &'a str,
    probe_net: &'a str,
    probe_name: &'a str,
    kind: GeneratedAnalogScenarioKind,
}

#[derive(Debug, Clone)]
enum GeneratedAnalogScenarioKind {
    Transient {
        stop_time_us: f64,
        max_step_us: f64,
    },
    Ac {
        start_frequency_hz: f64,
        stop_frequency_hz: f64,
        points_per_decade: u32,
    },
    Dc,
    Noise {
        start_frequency_hz: f64,
        stop_frequency_hz: f64,
        points_per_decade: u32,
        input_source: String,
        input_probe_name: String,
    },
}

impl GeneratedAnalogScenarioKind {
    fn scenario_type(&self) -> &'static str {
        match self {
            Self::Transient { .. } => "analog_transient",
            Self::Ac { .. } => "analog_ac",
            Self::Dc => "analog_dc",
            Self::Noise { .. } => "analog_noise",
        }
    }

    fn check_id(&self) -> &'static str {
        match self {
            Self::Transient { .. } => "SPICE_TRANSIENT_ANALYSIS",
            Self::Ac { .. } => "SPICE_AC_ANALYSIS",
            Self::Dc => "SPICE_DC_ANALYSIS",
            Self::Noise { .. } => "SPICE_NOISE_ANALYSIS",
        }
    }
}

fn analog_block(
    project_path: &Path,
    project: &crate::board_ir::BoardProject,
    scenario: &GeneratedAnalogScenarioSpec<'_>,
    node_by_net: &std::collections::BTreeMap<String, String>,
) -> Result<serde_yaml_ng::Mapping> {
    let mut analog = serde_yaml_ng::Mapping::new();
    insert_string(&mut analog, "backend", "auto");
    insert_string(&mut analog, "netlist_source", "generated_from_board");

    let mut generated = serde_yaml_ng::Mapping::new();
    insert_string(&mut generated, "ground_net", scenario.ground_net);
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
    let component_ids = project.board.components.keys().cloned().collect::<Vec<_>>();
    analog.insert(
        key("model_files"),
        serde_yaml_ng::Value::Sequence(model_file_values_for_generated_components(
            project_path,
            project,
            &component_ids,
        )?),
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
    match &scenario.kind {
        GeneratedAnalogScenarioKind::Transient {
            stop_time_us,
            max_step_us,
        } => {
            insert_string(&mut analysis, "type", "tran");
            insert_number(&mut analysis, "stop_time_us", *stop_time_us)?;
            insert_number(&mut analysis, "max_step_us", *max_step_us)?;
        }
        GeneratedAnalogScenarioKind::Ac {
            start_frequency_hz,
            stop_frequency_hz,
            points_per_decade,
        } => {
            insert_string(&mut analysis, "type", "ac");
            insert_number(&mut analysis, "start_frequency_hz", *start_frequency_hz)?;
            insert_number(&mut analysis, "stop_frequency_hz", *stop_frequency_hz)?;
            analysis.insert(
                key("points_per_decade"),
                serde_yaml_ng::to_value(points_per_decade)
                    .context("Failed to encode AC points_per_decade.")?,
            );
        }
        GeneratedAnalogScenarioKind::Dc => {
            insert_string(&mut analysis, "type", "op");
        }
        GeneratedAnalogScenarioKind::Noise {
            start_frequency_hz,
            stop_frequency_hz,
            points_per_decade,
            input_source,
            ..
        } => {
            insert_string(&mut analysis, "type", "noise");
            insert_number(&mut analysis, "start_frequency_hz", *start_frequency_hz)?;
            insert_number(&mut analysis, "stop_frequency_hz", *stop_frequency_hz)?;
            analysis.insert(
                key("points_per_decade"),
                serde_yaml_ng::to_value(points_per_decade)
                    .context("Failed to encode noise points_per_decade.")?,
            );
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "Noise output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            insert_string(&mut analysis, "noise_output_node", output_node);
            insert_string(&mut analysis, "noise_input_source", input_source);
        }
    }
    analog.insert(key("analysis"), serde_yaml_ng::Value::Mapping(analysis));
    analog.insert(key("stimuli"), serde_yaml_ng::Value::Sequence(Vec::new()));

    let probe_node = node_by_net.get(scenario.probe_net).with_context(|| {
        format!(
            "Probe net {} has no generated SPICE node.",
            scenario.probe_net
        )
    })?;
    let probes = match &scenario.kind {
        GeneratedAnalogScenarioKind::Noise {
            input_source,
            input_probe_name,
            ..
        } => {
            let input_node = noise_input_source_positive_node(project, input_source, node_by_net)
                .unwrap_or(probe_node);
            vec![
                probe_value(scenario.probe_name.trim(), &format!("V({probe_node})")),
                probe_value(input_probe_name, &format!("V({input_node})")),
            ]
        }
        _ => vec![probe_value(
            scenario.probe_name.trim(),
            &format!("V({probe_node})"),
        )],
    };
    analog.insert(key("probes"), serde_yaml_ng::Value::Sequence(probes));
    analog.insert(
        key("assertions"),
        serde_yaml_ng::Value::Sequence(Vec::new()),
    );
    Ok(analog)
}

fn probe_value(name: &str, expression: &str) -> serde_yaml_ng::Value {
    let mut probe = serde_yaml_ng::Mapping::new();
    insert_string(&mut probe, "name", name);
    insert_string(&mut probe, "expression", expression);
    serde_yaml_ng::Value::Mapping(probe)
}

fn noise_input_source_exists(project: &crate::board_ir::BoardProject, input_source: &str) -> bool {
    project
        .board
        .components
        .get(input_source)
        .and_then(|component| component.spice.as_ref())
        .is_some_and(|spice| {
            matches!(
                spice.primitive,
                crate::board_ir::SpicePrimitive::DcVoltageSource
                    | crate::board_ir::SpicePrimitive::PulseVoltageSource
                    | crate::board_ir::SpicePrimitive::DcCurrentSource
                    | crate::board_ir::SpicePrimitive::PulseCurrentSource
            )
        })
}

fn noise_input_source_positive_node<'a>(
    project: &crate::board_ir::BoardProject,
    input_source: &str,
    node_by_net: &'a std::collections::BTreeMap<String, String>,
) -> Option<&'a str> {
    let component = project.board.components.get(input_source)?;
    let positive_net = component.pins.get("P").or_else(|| {
        component
            .pins
            .iter()
            .find(|(_, net)| *net != "gnd")
            .map(|(_, net)| net)
    })?;
    node_by_net.get(positive_net).map(String::as_str)
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

fn ensure_sequence_field_mut<'a>(
    yaml: &'a mut serde_yaml_ng::Value,
    field: &str,
) -> Result<&'a mut Vec<serde_yaml_ng::Value>> {
    let mapping = yaml
        .as_mapping_mut()
        .context("Project YAML root must be a mapping.")?;
    let value = mapping
        .entry(key(field))
        .or_insert_with(|| serde_yaml_ng::Value::Sequence(Vec::new()));
    value
        .as_sequence_mut()
        .with_context(|| format!("Project field {field} must be a sequence."))
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
        serde_yaml_ng::to_value(value).context("Failed to encode number.")?,
    );
    Ok(())
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}
