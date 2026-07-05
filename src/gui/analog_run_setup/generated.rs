use anyhow::{Context, Result};
use std::path::Path;

use super::super::analog_model_files::model_file_values_for_generated_components;

pub(in crate::gui) fn append_generated_analog_scenario_with_project_path(
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
    if let GeneratedAnalogScenarioKind::HarmonicBalance { drive_source, .. } = &kind
        && !project.board.components.contains_key(drive_source)
    {
        anyhow::bail!(
            "Harmonic-balance drive source {drive_source} must be an included board component."
        );
    }
    if let GeneratedAnalogScenarioKind::DcSweep { source, .. } = &kind
        && !noise_input_source_exists(&project, source)
    {
        anyhow::bail!(
            "DC sweep source {source} must be an included voltage or current source component."
        );
    }
    if let GeneratedAnalogScenarioKind::TransferFunction { input_source } = &kind
        && !noise_input_source_exists(&project, input_source)
    {
        anyhow::bail!(
            "Transfer-function input source {input_source} must be an included voltage or current source component."
        );
    }
    if let GeneratedAnalogScenarioKind::PoleZero { input_source, .. } = &kind
        && !noise_input_source_exists(&project, input_source)
    {
        anyhow::bail!(
            "Pole-zero input source {input_source} must be an included voltage or current source component."
        );
    }
    if let GeneratedAnalogScenarioKind::Sensitivity { filters, .. } = &kind {
        for filter in filters {
            let declared = filter.trim();
            if !declared.contains(':') && !project.board.components.contains_key(declared) {
                anyhow::bail!(
                    "Sensitivity filter {declared} must be an included board component or explicit backend parameter."
                );
            }
        }
    }
    if let GeneratedAnalogScenarioKind::Distortion {
        f1_sources,
        f2_sources,
        ..
    } = &kind
    {
        for source in f1_sources.iter().chain(f2_sources.iter()) {
            if !noise_input_source_exists(&project, source) {
                anyhow::bail!(
                    "Distortion source {source} must be an included voltage or current source component."
                );
            }
        }
    }
    if let GeneratedAnalogScenarioKind::SParameter { port2_net, .. } = &kind {
        if !project.board.nets.contains_key(port2_net) {
            anyhow::bail!("S-parameter port 2 net {port2_net} was not found.");
        }
        if port2_net == probe_net {
            anyhow::bail!("S-parameter port 2 net must differ from port 1 net.");
        }
    }
    if let GeneratedAnalogScenarioKind::Pss { drive_source, .. }
    | GeneratedAnalogScenarioKind::PhaseNoise { drive_source, .. }
    | GeneratedAnalogScenarioKind::PeriodicAc { drive_source, .. } = &kind
        && let Some(source) = drive_source
        && !project.board.components.contains_key(source)
    {
        anyhow::bail!("Periodic drive source {source} must be an included board component.");
    }
    if let GeneratedAnalogScenarioKind::PeriodicAc { input_source, .. } = &kind
        && !noise_input_source_exists(&project, input_source)
    {
        anyhow::bail!(
            "Periodic AC input source {input_source} must be an included voltage or current source component."
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
pub(in crate::gui) enum GeneratedAnalogScenarioKind {
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
    DcSweep {
        source: String,
        start: f64,
        stop: f64,
        step: f64,
    },
    TransferFunction {
        input_source: String,
    },
    PoleZero {
        input_source: String,
        mode: String,
    },
    Sensitivity {
        mode: String,
        start_frequency_hz: f64,
        stop_frequency_hz: f64,
        points_per_decade: u32,
        filters: Vec<String>,
    },
    Distortion {
        mode: String,
        start_frequency_hz: f64,
        stop_frequency_hz: f64,
        points_per_decade: u32,
        f1_sources: Vec<String>,
        f2_sources: Vec<String>,
        f2_over_f1: Option<f64>,
    },
    Measure {
        mode: String,
        template_name: String,
        operation: String,
        from: f64,
        to: f64,
        stop_time_us: f64,
        max_step_us: f64,
        start_frequency_hz: f64,
        stop_frequency_hz: f64,
        points_per_decade: u32,
    },
    SParameter {
        port2_net: String,
        start_frequency_hz: f64,
        stop_frequency_hz: f64,
        points_per_decade: u32,
        reference_impedance_ohm: f64,
    },
    Pss {
        mode: String,
        frequency_guess_hz: f64,
        stabilization_time_us: f64,
        periods: u32,
        drive_source: Option<String>,
    },
    PhaseNoise {
        mode: String,
        carrier_frequency_hz: f64,
        offset_start_hz: f64,
        offset_stop_hz: f64,
        points_per_decade: u32,
        drive_source: Option<String>,
    },
    PeriodicAc {
        mode: String,
        carrier_frequency_hz: f64,
        start_frequency_hz: f64,
        stop_frequency_hz: f64,
        points_per_decade: u32,
        input_source: String,
        sidebands: u32,
        drive_source: Option<String>,
    },
    Noise {
        start_frequency_hz: f64,
        stop_frequency_hz: f64,
        points_per_decade: u32,
        input_source: String,
        input_probe_name: String,
    },
    Fourier {
        stop_time_us: f64,
        max_step_us: f64,
        fundamental_frequency_hz: f64,
        harmonics: u32,
    },
    HarmonicBalance {
        fundamental_frequency_hz: f64,
        harmonics: u32,
        drive_source: String,
    },
}

impl GeneratedAnalogScenarioKind {
    fn scenario_type(&self) -> &'static str {
        match self {
            Self::Transient { .. } => "analog_transient",
            Self::Ac { .. } => "analog_ac",
            Self::Dc => "analog_dc",
            Self::DcSweep { .. } => "analog_dc_sweep",
            Self::TransferFunction { .. } => "analog_transfer_function",
            Self::PoleZero { .. } => "analog_pole_zero",
            Self::Sensitivity { .. } => "analog_sensitivity",
            Self::Distortion { .. } => "analog_distortion",
            Self::Measure { .. } => "analog_measure",
            Self::SParameter { .. } => "analog_sparameter",
            Self::Pss { .. } => "analog_pss",
            Self::PhaseNoise { .. } => "analog_phase_noise",
            Self::PeriodicAc { .. } => "analog_periodic_ac",
            Self::Noise { .. } => "analog_noise",
            Self::Fourier { .. } => "analog_fourier",
            Self::HarmonicBalance { .. } => "analog_harmonic_balance",
        }
    }

    fn check_id(&self) -> &'static str {
        match self {
            Self::Transient { .. } => "SPICE_TRANSIENT_ANALYSIS",
            Self::Ac { .. } => "SPICE_AC_ANALYSIS",
            Self::Dc => "SPICE_DC_ANALYSIS",
            Self::DcSweep { .. } => "SPICE_DC_SWEEP_ANALYSIS",
            Self::TransferFunction { .. } => "SPICE_TRANSFER_FUNCTION_ANALYSIS",
            Self::PoleZero { .. } => "SPICE_POLE_ZERO_ANALYSIS",
            Self::Sensitivity { .. } => "SPICE_SENSITIVITY_ANALYSIS",
            Self::Distortion { .. } => "SPICE_DISTORTION_ANALYSIS",
            Self::Measure { .. } => "SPICE_MEASURE_ANALYSIS",
            Self::SParameter { .. } => "SPICE_S_PARAMETER_ANALYSIS",
            Self::Pss { .. } => "SPICE_PSS_ANALYSIS",
            Self::PhaseNoise { .. } => "SPICE_PHASE_NOISE_ANALYSIS",
            Self::PeriodicAc { .. } => "SPICE_PERIODIC_AC_ANALYSIS",
            Self::Noise { .. } => "SPICE_NOISE_ANALYSIS",
            Self::Fourier { .. } => "SPICE_FOURIER_ANALYSIS",
            Self::HarmonicBalance { .. } => "SPICE_HARMONIC_BALANCE_ANALYSIS",
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
        GeneratedAnalogScenarioKind::DcSweep {
            source,
            start,
            stop,
            step,
        } => {
            insert_string(&mut analysis, "type", "dc_sweep");
            insert_string(&mut analysis, "dc_sweep_source", source);
            insert_number(&mut analysis, "dc_sweep_start", *start)?;
            insert_number(&mut analysis, "dc_sweep_stop", *stop)?;
            insert_number(&mut analysis, "dc_sweep_step", *step)?;
        }
        GeneratedAnalogScenarioKind::TransferFunction { input_source } => {
            insert_string(&mut analysis, "type", "tf");
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "Transfer-function output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            insert_string(
                &mut analysis,
                "transfer_output_expression",
                &format!("V({output_node})"),
            );
            insert_string(&mut analysis, "transfer_input_source", input_source);
        }
        GeneratedAnalogScenarioKind::PoleZero { input_source, mode } => {
            insert_string(&mut analysis, "type", "pz");
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "Pole-zero output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            let reference_node = node_by_net.get(scenario.ground_net).with_context(|| {
                format!(
                    "Pole-zero reference net {} has no generated SPICE node.",
                    scenario.ground_net
                )
            })?;
            insert_string(&mut analysis, "pole_zero_output_node", output_node);
            insert_string(&mut analysis, "pole_zero_reference_node", reference_node);
            insert_string(&mut analysis, "pole_zero_input_source", input_source);
            insert_string(&mut analysis, "pole_zero_mode", mode);
        }
        GeneratedAnalogScenarioKind::Sensitivity {
            mode,
            start_frequency_hz,
            stop_frequency_hz,
            points_per_decade,
            filters,
        } => {
            insert_string(&mut analysis, "type", "sens");
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "Sensitivity output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            insert_string(
                &mut analysis,
                "sensitivity_output_expression",
                &format!("V({output_node})"),
            );
            insert_string(&mut analysis, "sensitivity_mode", mode);
            if mode == "ac" {
                insert_number(&mut analysis, "start_frequency_hz", *start_frequency_hz)?;
                insert_number(&mut analysis, "stop_frequency_hz", *stop_frequency_hz)?;
                analysis.insert(
                    key("points_per_decade"),
                    serde_yaml_ng::to_value(points_per_decade)
                        .context("Failed to encode sensitivity points_per_decade.")?,
                );
            }
            analysis.insert(
                key("sensitivity_filters"),
                serde_yaml_ng::Value::Sequence(
                    filters
                        .iter()
                        .map(|filter| serde_yaml_ng::Value::String(filter.clone()))
                        .collect(),
                ),
            );
        }
        GeneratedAnalogScenarioKind::Distortion {
            mode,
            start_frequency_hz,
            stop_frequency_hz,
            points_per_decade,
            f1_sources,
            f2_sources,
            f2_over_f1,
        } => {
            insert_string(&mut analysis, "type", "disto");
            insert_string(&mut analysis, "distortion_mode", mode);
            insert_number(
                &mut analysis,
                "distortion_start_frequency_hz",
                *start_frequency_hz,
            )?;
            insert_number(
                &mut analysis,
                "distortion_stop_frequency_hz",
                *stop_frequency_hz,
            )?;
            analysis.insert(
                key("distortion_points_per_decade"),
                serde_yaml_ng::to_value(points_per_decade)
                    .context("Failed to encode distortion_points_per_decade.")?,
            );
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "Distortion output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            insert_string(
                &mut analysis,
                "distortion_output_expression",
                &format!("V({output_node})"),
            );
            analysis.insert(
                key("distortion_f1_sources"),
                serde_yaml_ng::Value::Sequence(
                    f1_sources
                        .iter()
                        .map(|source| serde_yaml_ng::Value::String(source.clone()))
                        .collect(),
                ),
            );
            if !f2_sources.is_empty() {
                analysis.insert(
                    key("distortion_f2_sources"),
                    serde_yaml_ng::Value::Sequence(
                        f2_sources
                            .iter()
                            .map(|source| serde_yaml_ng::Value::String(source.clone()))
                            .collect(),
                    ),
                );
            }
            if let Some(ratio) = f2_over_f1 {
                insert_number(&mut analysis, "distortion_f2_over_f1", *ratio)?;
            }
        }
        GeneratedAnalogScenarioKind::Measure {
            mode,
            template_name,
            operation,
            from,
            to,
            stop_time_us,
            max_step_us,
            start_frequency_hz,
            stop_frequency_hz,
            points_per_decade,
        } => {
            insert_string(&mut analysis, "type", "measure");
            insert_string(&mut analysis, "measure_mode", mode);
            if mode == "tran" {
                insert_number(&mut analysis, "stop_time_us", *stop_time_us)?;
                insert_number(&mut analysis, "max_step_us", *max_step_us)?;
            } else {
                insert_number(&mut analysis, "start_frequency_hz", *start_frequency_hz)?;
                insert_number(&mut analysis, "stop_frequency_hz", *stop_frequency_hz)?;
                analysis.insert(
                    key("points_per_decade"),
                    serde_yaml_ng::to_value(points_per_decade)
                        .context("Failed to encode measure points_per_decade.")?,
                );
            }
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "Measure output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            let mut template = serde_yaml_ng::Mapping::new();
            insert_string(&mut template, "name", template_name);
            insert_string(&mut template, "operation", operation);
            insert_string(&mut template, "expression", &format!("V({output_node})"));
            if mode == "tran" {
                insert_number(&mut template, "from_us", *from)?;
                insert_number(&mut template, "to_us", *to)?;
            } else {
                insert_number(&mut template, "from_hz", *from)?;
                insert_number(&mut template, "to_hz", *to)?;
            }
            analysis.insert(
                key("measure_templates"),
                serde_yaml_ng::Value::Sequence(vec![serde_yaml_ng::Value::Mapping(template)]),
            );
        }
        GeneratedAnalogScenarioKind::SParameter {
            port2_net,
            start_frequency_hz,
            stop_frequency_hz,
            points_per_decade,
            reference_impedance_ohm,
        } => {
            insert_string(&mut analysis, "type", "sparam");
            insert_number(&mut analysis, "start_frequency_hz", *start_frequency_hz)?;
            insert_number(&mut analysis, "stop_frequency_hz", *stop_frequency_hz)?;
            analysis.insert(
                key("points_per_decade"),
                serde_yaml_ng::to_value(points_per_decade)
                    .context("Failed to encode S-parameter points_per_decade.")?,
            );
            let reference_node = node_by_net.get(scenario.ground_net).with_context(|| {
                format!(
                    "S-parameter reference net {} has no generated SPICE node.",
                    scenario.ground_net
                )
            })?;
            let port1_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "S-parameter port 1 net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            let port2_node = node_by_net.get(port2_net).with_context(|| {
                format!("S-parameter port 2 net {port2_net} has no generated SPICE node.")
            })?;
            analysis.insert(
                key("s_parameter_ports"),
                serde_yaml_ng::Value::Sequence(vec![
                    sparameter_port_value(
                        "p1",
                        port1_node,
                        reference_node,
                        *reference_impedance_ohm,
                    )?,
                    sparameter_port_value(
                        "p2",
                        port2_node,
                        reference_node,
                        *reference_impedance_ohm,
                    )?,
                ]),
            );
        }
        GeneratedAnalogScenarioKind::Pss {
            mode,
            frequency_guess_hz,
            stabilization_time_us,
            periods,
            drive_source,
        } => {
            insert_string(&mut analysis, "type", "pss");
            insert_string(&mut analysis, "pss_mode", mode);
            insert_number(&mut analysis, "pss_frequency_guess_hz", *frequency_guess_hz)?;
            insert_number(
                &mut analysis,
                "pss_stabilization_time_us",
                *stabilization_time_us,
            )?;
            analysis.insert(
                key("pss_periods"),
                serde_yaml_ng::to_value(periods).context("Failed to encode pss_periods.")?,
            );
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "PSS output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            insert_string(
                &mut analysis,
                "pss_output_expression",
                &format!("V({output_node})"),
            );
            analysis.insert(
                key("pss_drive_sources"),
                drive_source_sequence(drive_source),
            );
        }
        GeneratedAnalogScenarioKind::PhaseNoise {
            mode,
            carrier_frequency_hz,
            offset_start_hz,
            offset_stop_hz,
            points_per_decade,
            drive_source,
        } => {
            insert_string(&mut analysis, "type", "phase_noise");
            insert_string(&mut analysis, "phase_noise_mode", mode);
            insert_number(
                &mut analysis,
                "phase_noise_carrier_frequency_hz",
                *carrier_frequency_hz,
            )?;
            insert_number(
                &mut analysis,
                "phase_noise_offset_start_hz",
                *offset_start_hz,
            )?;
            insert_number(&mut analysis, "phase_noise_offset_stop_hz", *offset_stop_hz)?;
            analysis.insert(
                key("phase_noise_points_per_decade"),
                serde_yaml_ng::to_value(points_per_decade)
                    .context("Failed to encode phase_noise_points_per_decade.")?,
            );
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "Phase-noise output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            insert_string(
                &mut analysis,
                "phase_noise_output_expression",
                &format!("V({output_node})"),
            );
            analysis.insert(
                key("phase_noise_drive_sources"),
                drive_source_sequence(drive_source),
            );
        }
        GeneratedAnalogScenarioKind::PeriodicAc {
            mode,
            carrier_frequency_hz,
            start_frequency_hz,
            stop_frequency_hz,
            points_per_decade,
            input_source,
            sidebands,
            drive_source,
        } => {
            insert_string(&mut analysis, "type", "pac");
            insert_string(&mut analysis, "pac_mode", mode);
            insert_number(
                &mut analysis,
                "pac_carrier_frequency_hz",
                *carrier_frequency_hz,
            )?;
            insert_number(&mut analysis, "pac_start_frequency_hz", *start_frequency_hz)?;
            insert_number(&mut analysis, "pac_stop_frequency_hz", *stop_frequency_hz)?;
            analysis.insert(
                key("pac_points_per_decade"),
                serde_yaml_ng::to_value(points_per_decade)
                    .context("Failed to encode pac_points_per_decade.")?,
            );
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "Periodic AC output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            insert_string(
                &mut analysis,
                "pac_output_expression",
                &format!("V({output_node})"),
            );
            insert_string(&mut analysis, "pac_input_source", input_source);
            analysis.insert(
                key("pac_sidebands"),
                serde_yaml_ng::to_value(sidebands).context("Failed to encode pac_sidebands.")?,
            );
            analysis.insert(
                key("pac_drive_sources"),
                drive_source_sequence(drive_source),
            );
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
        GeneratedAnalogScenarioKind::Fourier {
            stop_time_us,
            max_step_us,
            fundamental_frequency_hz,
            harmonics,
        } => {
            insert_string(&mut analysis, "type", "fourier");
            insert_number(&mut analysis, "stop_time_us", *stop_time_us)?;
            insert_number(&mut analysis, "max_step_us", *max_step_us)?;
            insert_number(
                &mut analysis,
                "fourier_fundamental_frequency_hz",
                *fundamental_frequency_hz,
            )?;
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "Fourier output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            insert_string(
                &mut analysis,
                "fourier_output_expression",
                &format!("V({output_node})"),
            );
            analysis.insert(
                key("fourier_harmonics"),
                serde_yaml_ng::to_value(harmonics)
                    .context("Failed to encode fourier_harmonics.")?,
            );
        }
        GeneratedAnalogScenarioKind::HarmonicBalance {
            fundamental_frequency_hz,
            harmonics,
            drive_source,
        } => {
            insert_string(&mut analysis, "type", "hb");
            insert_number(
                &mut analysis,
                "hb_fundamental_frequency_hz",
                *fundamental_frequency_hz,
            )?;
            analysis.insert(
                key("hb_harmonics"),
                serde_yaml_ng::to_value(harmonics).context("Failed to encode hb_harmonics.")?,
            );
            let output_node = node_by_net.get(scenario.probe_net).with_context(|| {
                format!(
                    "Harmonic-balance output net {} has no generated SPICE node.",
                    scenario.probe_net
                )
            })?;
            insert_string(
                &mut analysis,
                "hb_output_expression",
                &format!("V({output_node})"),
            );
            analysis.insert(
                key("hb_drive_sources"),
                serde_yaml_ng::Value::Sequence(vec![serde_yaml_ng::Value::String(
                    drive_source.clone(),
                )]),
            );
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
        GeneratedAnalogScenarioKind::SParameter { .. } => {
            vec![probe_value(scenario.probe_name.trim(), "S(p1,p1)")]
        }
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

fn sparameter_port_value(
    name: &str,
    positive_node: &str,
    negative_node: &str,
    reference_impedance_ohm: f64,
) -> Result<serde_yaml_ng::Value> {
    let mut port = serde_yaml_ng::Mapping::new();
    insert_string(&mut port, "name", name);
    insert_string(&mut port, "positive_node", positive_node);
    insert_string(&mut port, "negative_node", negative_node);
    insert_number(
        &mut port,
        "reference_impedance_ohm",
        reference_impedance_ohm,
    )?;
    Ok(serde_yaml_ng::Value::Mapping(port))
}

fn drive_source_sequence(source: &Option<String>) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::Sequence(
        source
            .iter()
            .map(|source| serde_yaml_ng::Value::String(source.clone()))
            .collect(),
    )
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
