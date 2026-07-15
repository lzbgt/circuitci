use crate::analog_model_resolver::declared_model_file_path_for_source_dir;
use anyhow::{Context, Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[path = "spice_directives.rs"]
mod spice_directives;
#[path = "spice_elements.rs"]
mod spice_elements;
#[path = "spice_scenarios.rs"]
mod spice_scenarios;
#[cfg(test)]
#[path = "spice_tests.rs"]
mod spice_tests;

use spice_directives::{
    ImportedDirectiveMetadata, operating_conditions_for_directives, record_imported_directive,
    scenario_parameters_for_directives,
};
use spice_elements::{parse_element, parse_spice_number};
use spice_scenarios::{
    ac_scenario_for_yaml, dc_sweep_scenario_for_yaml, distortion_scenario_for_yaml,
    fourier_scenario_for_yaml, measure_scenario_for_yaml, noise_scenario_for_yaml,
    operating_point_scenario_for_yaml, pole_zero_scenario_for_yaml, sensitivity_scenario_for_yaml,
    transfer_function_scenario_for_yaml, transient_scenario_for_yaml,
};

#[derive(Debug, Clone)]
pub struct SpiceImportOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub name: String,
    pub backend: String,
    pub stop_time_us: f64,
    pub max_step_us: f64,
}

#[derive(Debug)]
struct ParsedDeck {
    elements: Vec<ParsedElement>,
    includes: Vec<IncludeFile>,
    nodes: BTreeSet<String>,
    tran: Option<TranSpec>,
    op: bool,
    dc: Option<DcSweepSpec>,
    ac: Option<AcSpec>,
    noise: Option<NoiseSpec>,
    transfer_function: Option<TransferFunctionSpec>,
    pole_zero: Option<PoleZeroSpec>,
    sensitivity: Option<SensitivitySpec>,
    distortion: Option<DistortionSpec>,
    fourier: Vec<FourierSpec>,
    measures: Vec<MeasureStatementSpec>,
    explicit_probes: Vec<OutputProbeSpec>,
    directive_metadata: ImportedDirectiveMetadata,
}

#[derive(Debug)]
struct ParsedElement {
    name: String,
    model: String,
    pins: Vec<(String, String)>,
    spice: Option<SpicePrimitiveSpec>,
    source_kind: Option<ImportedSourceKind>,
    distortion_role: DistortionSourceRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportedSourceKind {
    Voltage,
    Current,
}

#[derive(Debug)]
struct IncludeFile {
    resolved: PathBuf,
}

#[derive(Debug)]
struct TranSpec {
    stop_time_us: f64,
    max_step_us: f64,
}

#[derive(Debug)]
struct AcSpec {
    sweep_type: String,
    start_frequency_hz: f64,
    stop_frequency_hz: f64,
    points_per_decade: u32,
}

#[derive(Debug)]
struct DcSweepSpec {
    source: String,
    start: f64,
    stop: f64,
    step: f64,
}

#[derive(Debug)]
struct NoiseSpec {
    output_node: String,
    reference_node: Option<String>,
    input_source: String,
    sweep_type: String,
    start_frequency_hz: f64,
    stop_frequency_hz: f64,
    points_per_decade: u32,
}

#[derive(Debug)]
struct TransferFunctionSpec {
    output_expression: String,
    input_source: String,
}

#[derive(Debug)]
struct PoleZeroSpec {
    output_node: String,
    reference_node: String,
    input_source: String,
    mode: String,
}

#[derive(Debug)]
struct PoleZeroDirective {
    input_positive_node: String,
    input_negative_node: String,
    output_node: String,
    reference_node: String,
    source_kind: ImportedSourceKind,
    mode: String,
}

#[derive(Debug)]
struct SensitivitySpec {
    output_expression: String,
    mode: String,
    ac: Option<AcSpec>,
    filters: Vec<String>,
}

#[derive(Debug)]
struct SensitivityDirective {
    output_expression: String,
    mode: String,
    ac: Option<AcSpec>,
}

#[derive(Debug)]
struct DistortionSpec {
    mode: String,
    sweep_type: String,
    start_frequency_hz: f64,
    stop_frequency_hz: f64,
    points_per_decade: u32,
    output_expression: String,
    f1_sources: Vec<String>,
    f2_sources: Vec<String>,
    f2_over_f1: Option<f64>,
}

#[derive(Debug)]
struct DistortionDirective {
    sweep_type: String,
    start_frequency_hz: f64,
    stop_frequency_hz: f64,
    points_per_decade: u32,
    f2_over_f1: Option<f64>,
}

#[derive(Debug, Default, Clone, Copy)]
struct DistortionSourceRole {
    f1: bool,
    f2: bool,
}

#[derive(Debug)]
struct FourierSpec {
    fundamental_frequency_hz: f64,
    output_expression: String,
}

#[derive(Debug)]
struct MeasureStatementSpec {
    mode: String,
    name: String,
    statement: String,
}

#[derive(Debug, Clone)]
struct OutputProbeSpec {
    expression: String,
    quantity: Option<&'static str>,
}

#[derive(Debug)]
enum SpicePrimitiveSpec {
    Resistor { value_ohm: f64 },
    Capacitor { value_f: f64 },
    Inductor { value_h: f64 },
    DcVoltageSource { dc_v: f64 },
    PulseVoltageSource { pulse: PulseSpec },
    DcCurrentSource { dc_a: f64 },
    PulseCurrentSource { pulse: CurrentPulseSpec },
}

#[derive(Debug, Serialize)]
struct ProjectYaml {
    project: ProjectMetaYaml,
    libraries: Vec<String>,
    board: BoardYaml,
    scenarios: Vec<ScenarioYaml>,
}

#[derive(Debug, Serialize)]
struct ProjectMetaYaml {
    name: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct BoardYaml {
    components: BTreeMap<String, ComponentYaml>,
    nets: BTreeMap<String, NetYaml>,
}

#[derive(Debug, Serialize)]
struct ComponentYaml {
    model: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pins: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spice: Option<ComponentSpiceYaml>,
}

#[derive(Debug, Serialize)]
struct ComponentSpiceYaml {
    primitive: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_ohm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_f: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initial_v: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_h: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dc_v: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dc_a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pulse: Option<PulseSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_pulse: Option<CurrentPulseSpec>,
}

#[derive(Debug, Serialize)]
struct PulseSpec {
    initial_v: f64,
    pulsed_v: f64,
    delay_us: f64,
    rise_us: f64,
    fall_us: f64,
    width_us: f64,
    period_us: f64,
}

#[derive(Debug, Serialize)]
struct CurrentPulseSpec {
    initial_a: f64,
    pulsed_a: f64,
    delay_us: f64,
    rise_us: f64,
    fall_us: f64,
    width_us: f64,
    period_us: f64,
}

#[derive(Debug, Serialize)]
struct NetYaml {
    kind: String,
}

#[derive(Debug, Serialize)]
struct ScenarioYaml {
    name: String,
    #[serde(rename = "type")]
    scenario_type: String,
    checks: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    parameters: BTreeMap<String, serde_yaml_ng::Value>,
    analog: AnalogYaml,
}

#[derive(Debug, Serialize)]
struct AnalogYaml {
    backend: String,
    netlist_source: String,
    netlist: String,
    #[serde(skip_serializing_if = "OperatingConditionsYaml::is_empty")]
    operating_conditions: OperatingConditionsYaml,
    model_files: Vec<ModelFileYaml>,
    node_bindings: Vec<NodeBindingYaml>,
    pin_bindings: Vec<PinBindingYaml>,
    analysis: AnalysisYaml,
    stimuli: Vec<StimulusYaml>,
    probes: Vec<ProbeYaml>,
    assertions: Vec<AssertionYaml>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct OperatingConditionsYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    ambient_temperature_c: Option<f64>,
}

impl OperatingConditionsYaml {
    fn is_empty(&self) -> bool {
        self.ambient_temperature_c.is_none()
    }
}

#[derive(Debug, Clone, Serialize)]
struct ModelFileYaml {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct NodeBindingYaml {
    node: String,
    net: String,
}

#[derive(Debug, Clone, Serialize)]
struct PinBindingYaml {
    node: String,
    endpoint: EndpointYaml,
}

#[derive(Debug, Clone, Serialize)]
struct EndpointYaml {
    component: String,
    pin: String,
}

#[derive(Debug, Serialize)]
struct AnalysisYaml {
    #[serde(rename = "type")]
    analysis_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_time_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_step_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_frequency_hz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_frequency_hz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    points_per_decade: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sweep_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    noise_output_node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    noise_reference_node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    noise_input_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transfer_output_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transfer_input_source: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    transfer_function_assertions: Vec<AssertionYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pole_zero_output_node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pole_zero_reference_node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pole_zero_input_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pole_zero_mode: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pole_zero_assertions: Vec<AssertionYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sensitivity_output_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sensitivity_mode: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    sensitivity_filters: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    sensitivity_assertions: Vec<AssertionYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distortion_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distortion_start_frequency_hz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distortion_stop_frequency_hz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distortion_points_per_decade: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distortion_sweep_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distortion_output_expression: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    distortion_f1_sources: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    distortion_f2_sources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distortion_f2_over_f1: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    distortion_assertions: Vec<AssertionYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fourier_fundamental_frequency_hz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fourier_output_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fourier_harmonics: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fourier_assertions: Vec<AssertionYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dc_sweep_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dc_sweep_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dc_sweep_stop: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dc_sweep_step: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dc_sweep_assertions: Vec<AssertionYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    measure_mode: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    measure_statements: Vec<MeasureStatementYaml>,
}

#[derive(Debug, Clone, Serialize)]
struct MeasureStatementYaml {
    name: String,
    statement: String,
}

#[derive(Debug, Clone, Serialize)]
struct StimulusYaml {
    name: String,
    description: String,
}

#[derive(Debug, Clone, Serialize)]
struct ProbeYaml {
    name: String,
    expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    quantity: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct AssertionYaml {}

#[derive(Debug, Clone)]
struct AnalogScenarioParts {
    netlist: String,
    parameters: BTreeMap<String, serde_yaml_ng::Value>,
    operating_conditions: OperatingConditionsYaml,
    model_files: Vec<ModelFileYaml>,
    node_bindings: Vec<NodeBindingYaml>,
    pin_bindings: Vec<PinBindingYaml>,
    stimuli: Vec<StimulusYaml>,
    probes: Vec<ProbeYaml>,
}

pub fn import_spice(options: &SpiceImportOptions) -> Result<()> {
    import_spice_with_progress(options, |_, _| {})
}

pub fn import_spice_with_progress<F>(options: &SpiceImportOptions, mut on_progress: F) -> Result<()>
where
    F: FnMut(&'static str, String),
{
    import_spice_with_progress_and_cancel(options, &mut on_progress, || false)
}

pub fn import_spice_with_progress_and_cancel<F, C>(
    options: &SpiceImportOptions,
    mut on_progress: F,
    should_cancel: C,
) -> Result<()>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    on_progress(
        "Parsing SPICE deck",
        format!("Reading {}.", options.input.display()),
    );
    let deck = parse_spice_deck(&options.input)?;
    ensure_not_canceled(&should_cancel)?;
    on_progress(
        "Preparing output",
        format!(
            "Creating output directory for {}.",
            options.output.display()
        ),
    );
    let output_dir = options.output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create import output directory {}",
            output_dir.display()
        )
    })?;
    ensure_not_canceled(&should_cancel)?;
    on_progress(
        "Building Board IR",
        format!(
            "{} element(s), {} include(s), {} node(s).",
            deck.elements.len(),
            deck.includes.len(),
            deck.nodes.len()
        ),
    );
    let project = build_project_yaml(options, &deck, output_dir)?;
    ensure_not_canceled(&should_cancel)?;
    on_progress(
        "Serializing Board IR",
        format!("Writing {}.", options.output.display()),
    );
    let mut yaml = serde_yaml_ng::to_string(&project)?;
    yaml.insert_str(
        0,
        "# Generated by CircuitCI from a SPICE deck. Review probes/assertions before sign-off.\n",
    );
    fs::write(&options.output, yaml)
        .with_context(|| format!("Failed to write {}", options.output.display()))?;
    Ok(())
}

fn ensure_not_canceled(should_cancel: &impl Fn() -> bool) -> Result<()> {
    if should_cancel() {
        return Err(crate::cancellation::canceled(
            "SPICE import canceled before completion.",
        ));
    }
    Ok(())
}

fn parse_spice_deck(path: &Path) -> Result<ParsedDeck> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("Failed to read SPICE deck {}", path.display()))?;
    let logical_lines = logical_lines(&text)?;
    let source_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut elements = Vec::new();
    let mut includes = Vec::new();
    let mut nodes = BTreeSet::new();
    let mut tran = None;
    let mut op = false;
    let mut dc = None;
    let mut ac = None;
    let mut noise = None;
    let mut transfer_function = None;
    let mut pole_zero_directive = None;
    let mut sensitivity_directive = None;
    let mut distortion_directive = None;
    let mut distortion_output_expression = None;
    let mut fourier = Vec::new();
    let mut measures = Vec::new();
    let mut measure_names = BTreeSet::new();
    let mut explicit_probes = Vec::new();
    let mut directive_metadata = ImportedDirectiveMetadata::default();
    let mut in_control = false;
    let mut control_distortion_active = false;
    let mut subckt_depth = 0usize;
    let mut inline_lib_depth = 0usize;
    for line in logical_lines {
        let tokens = tokenize(&line);
        if tokens.is_empty() {
            continue;
        }
        let first = tokens[0].as_str();
        let command = first.to_ascii_lowercase();
        if inline_lib_depth > 0 {
            match command.as_str() {
                ".lib" => {
                    handle_lib_directive(&tokens, source_dir, &mut includes, &mut inline_lib_depth)?
                }
                ".endl" | "endl" => inline_lib_depth -= 1,
                ".include" => includes.push(parse_include(&tokens, source_dir)?),
                ".global" => {
                    nodes.extend(parse_global_nodes(&tokens)?);
                }
                ".temp" | ".option" | ".options" | ".ic" | ".nodeset" | ".model" => {
                    record_imported_directive(&mut directive_metadata, &tokens, &line)?;
                }
                _ => {}
            }
            continue;
        }
        if subckt_depth > 0 {
            match command.as_str() {
                ".subckt" => subckt_depth += 1,
                ".ends" => subckt_depth -= 1,
                ".include" => includes.push(parse_include(&tokens, source_dir)?),
                ".lib" => {
                    handle_lib_directive(&tokens, source_dir, &mut includes, &mut inline_lib_depth)?
                }
                ".global" => {
                    nodes.extend(parse_global_nodes(&tokens)?);
                }
                ".temp" | ".option" | ".options" | ".ic" | ".nodeset" | ".model" => {
                    record_imported_directive(&mut directive_metadata, &tokens, &line)?;
                }
                _ => {}
            }
            continue;
        }
        if in_control {
            match command.as_str() {
                ".endc" | "endc" => {
                    in_control = false;
                    control_distortion_active = false;
                }
                ".tran" | "tran" => {
                    control_distortion_active = false;
                    tran = parse_tran(&tokens).or(tran);
                }
                ".op" | "op" => {
                    control_distortion_active = false;
                    op = true;
                }
                ".dc" | "dc" => {
                    control_distortion_active = false;
                    dc = Some(parse_dc_sweep(&tokens)?);
                }
                ".ac" | "ac" => {
                    control_distortion_active = false;
                    ac = Some(parse_ac(&tokens)?);
                }
                ".noise" | "noise" => {
                    control_distortion_active = false;
                    noise = Some(parse_noise(&tokens)?);
                }
                ".tf" | "tf" => {
                    control_distortion_active = false;
                    transfer_function = Some(parse_transfer_function(&tokens)?);
                }
                ".pz" | "pz" => {
                    control_distortion_active = false;
                    pole_zero_directive = Some(parse_pole_zero_directive(&tokens)?);
                }
                ".sens" | "sens" => {
                    control_distortion_active = false;
                    sensitivity_directive = Some(parse_sensitivity(&tokens)?);
                }
                ".disto" | "disto" => {
                    control_distortion_active = true;
                    distortion_directive = Some(parse_distortion(&tokens)?);
                }
                ".print" | ".plot" | "print" | "plot" => {
                    if control_distortion_active && distortion_output_expression.is_none() {
                        distortion_output_expression = parse_control_distortion_output(&tokens)?;
                    } else {
                        explicit_probes.extend(parse_control_probe_outputs(&tokens)?);
                    }
                }
                ".save" | "save" | ".probe" | "probe" => {
                    explicit_probes.extend(parse_save_probe_outputs(&tokens)?);
                }
                ".temp" | "temp" | ".option" | "option" | ".options" | "options" | ".ic" | "ic"
                | ".nodeset" | "nodeset" | ".model" | "model" => {
                    record_imported_directive(&mut directive_metadata, &tokens, &line)?;
                }
                ".four" | "four" | "fourier" => {
                    control_distortion_active = false;
                    fourier.extend(parse_fourier(&tokens)?);
                }
                ".meas" | ".measure" | "meas" | "measure" => {
                    control_distortion_active = false;
                    push_measure_statement(
                        &mut measures,
                        &mut measure_names,
                        parse_measure_statement(&tokens, &line)?,
                    )?;
                }
                _ => {}
            }
            continue;
        }
        if first.starts_with('.') {
            match command.as_str() {
                ".include" => includes.push(parse_include(&tokens, source_dir)?),
                ".lib" => {
                    handle_lib_directive(&tokens, source_dir, &mut includes, &mut inline_lib_depth)?
                }
                ".global" => {
                    nodes.extend(parse_global_nodes(&tokens)?);
                }
                ".subckt" => subckt_depth = 1,
                ".ends" => bail!("SPICE .ends appears without a preceding .subckt block."),
                ".endl" => bail!("SPICE .endl appears without a preceding inline .lib block."),
                ".tran" => tran = parse_tran(&tokens).or(tran),
                ".op" => op = true,
                ".dc" => dc = Some(parse_dc_sweep(&tokens)?),
                ".ac" => ac = Some(parse_ac(&tokens)?),
                ".noise" => noise = Some(parse_noise(&tokens)?),
                ".tf" => transfer_function = Some(parse_transfer_function(&tokens)?),
                ".pz" => pole_zero_directive = Some(parse_pole_zero_directive(&tokens)?),
                ".sens" => sensitivity_directive = Some(parse_sensitivity(&tokens)?),
                ".disto" => distortion_directive = Some(parse_distortion(&tokens)?),
                ".print" | ".plot" => {
                    distortion_output_expression = parse_print_or_plot_distortion_output(&tokens)?
                        .or(distortion_output_expression);
                    explicit_probes.extend(parse_print_or_plot_probe_outputs(&tokens)?);
                }
                ".save" | ".probe" => {
                    explicit_probes.extend(parse_save_probe_outputs(&tokens)?);
                }
                ".temp" | ".option" | ".options" | ".ic" | ".nodeset" | ".model" => {
                    record_imported_directive(&mut directive_metadata, &tokens, &line)?;
                }
                ".four" => fourier.extend(parse_fourier(&tokens)?),
                ".meas" | ".measure" => {
                    push_measure_statement(
                        &mut measures,
                        &mut measure_names,
                        parse_measure_statement(&tokens, &line)?,
                    )?;
                }
                ".control" => in_control = true,
                ".endc" => bail!("SPICE .endc appears without a preceding .control block."),
                _ => {}
            }
            continue;
        }
        let element = parse_element(&tokens, &line)?;
        for (_, node) in &element.pins {
            nodes.insert(node.clone());
        }
        elements.push(element);
    }
    if in_control {
        bail!("SPICE .control block is missing a closing .endc.");
    }
    if subckt_depth > 0 {
        bail!("SPICE .subckt block is missing a closing .ends.");
    }
    if inline_lib_depth > 0 {
        bail!("SPICE inline .lib block is missing a closing .endl.");
    }
    if elements.is_empty() {
        bail!(
            "SPICE deck {} contains no importable elements.",
            path.display()
        );
    }
    let pole_zero = pole_zero_directive
        .map(|directive| pole_zero_spec_from_directive(directive, &elements))
        .transpose()?;
    let sensitivity = sensitivity_directive
        .map(|directive| sensitivity_spec_from_directive(directive, &elements));
    let distortion = distortion_directive
        .map(|directive| {
            distortion_spec_from_directive(directive, distortion_output_expression, &elements)
        })
        .transpose()?;
    Ok(ParsedDeck {
        elements,
        includes,
        nodes,
        tran,
        op,
        dc,
        ac,
        noise,
        transfer_function,
        pole_zero,
        sensitivity,
        distortion,
        fourier,
        measures,
        explicit_probes,
        directive_metadata,
    })
}

fn logical_lines(text: &str) -> Result<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let trimmed = strip_inline_comment(raw).trim();
        if trimmed.is_empty() || trimmed.starts_with('*') || trimmed.starts_with(';') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('+') {
            let Some(previous) = lines.last_mut() else {
                bail!(
                    "Continuation line {} has no preceding SPICE card.",
                    index + 1
                );
            };
            previous.push(' ');
            previous.push_str(rest.trim());
        } else {
            lines.push(trimmed.to_string());
        }
    }
    Ok(lines)
}

fn strip_inline_comment(line: &str) -> &str {
    line.split_once('$').map_or(line, |(before, _)| before)
}

fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut paren_depth = 0usize;
    for character in line.chars() {
        if let Some(mark) = quote {
            if character == mark {
                quote = None;
            } else {
                current.push(character);
            }
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = Some(character);
            continue;
        }
        match character {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            _ => {}
        }
        if character.is_whitespace() && paren_depth == 0 {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_include(tokens: &[String], source_dir: &Path) -> Result<IncludeFile> {
    if tokens.len() < 2 {
        bail!("{} directive requires a path.", tokens[0]);
    }
    let path = tokens[1].clone();
    let resolved = declared_model_file_path_for_source_dir(source_dir, &path)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("Failed to resolve SPICE include {path}."))?;
    if !resolved.is_file() {
        bail!(
            "SPICE include {} does not resolve to a file.",
            resolved.display()
        );
    }
    Ok(IncludeFile { resolved })
}

fn handle_lib_directive(
    tokens: &[String],
    source_dir: &Path,
    includes: &mut Vec<IncludeFile>,
    inline_lib_depth: &mut usize,
) -> Result<()> {
    if tokens.len() < 2 {
        bail!("SPICE .lib directive requires a file path or section name.");
    }
    if lib_directive_references_external_file(tokens, source_dir) {
        includes.push(parse_include(tokens, source_dir)?);
    } else {
        *inline_lib_depth += 1;
    }
    Ok(())
}

fn lib_directive_references_external_file(tokens: &[String], source_dir: &Path) -> bool {
    if tokens.len() >= 3 {
        return true;
    }
    let candidate = tokens[1].as_str();
    if declared_model_file_path_for_source_dir(source_dir, candidate)
        .map(|path| path.is_file())
        .unwrap_or(false)
    {
        return true;
    }
    candidate.contains('/')
        || candidate.contains('\\')
        || Path::new(candidate).extension().is_some()
}

fn parse_global_nodes(tokens: &[String]) -> Result<Vec<String>> {
    if tokens.len() < 2 {
        bail!("SPICE .global directive requires at least one node.");
    }
    tokens[1..]
        .iter()
        .map(|token| {
            let node = token.trim();
            if node.is_empty() {
                bail!("SPICE .global directive contains an empty node name.");
            }
            Ok(node.to_string())
        })
        .collect()
}

fn parse_tran(tokens: &[String]) -> Option<TranSpec> {
    if tokens.len() < 3 {
        return None;
    }
    let max_step_us = parse_spice_number(&tokens[1]).map(|value| value * 1_000_000.0)?;
    let stop_time_us = parse_spice_number(&tokens[2]).map(|value| value * 1_000_000.0)?;
    Some(TranSpec {
        stop_time_us,
        max_step_us,
    })
}

fn parse_ac(tokens: &[String]) -> Result<AcSpec> {
    if tokens.len() < 5 {
        bail!(".ac directive requires sweep type, point count, start, and stop.");
    }
    parse_frequency_sweep(".ac", &tokens[1], &tokens[2], &tokens[3], &tokens[4])
}

fn parse_dc_sweep(tokens: &[String]) -> Result<DcSweepSpec> {
    if tokens.len() < 5 {
        bail!(".dc directive requires source, start, stop, and step.");
    }
    let start = parse_spice_number(&tokens[2])
        .with_context(|| format!("Could not parse .dc start {}", tokens[2]))?;
    let stop = parse_spice_number(&tokens[3])
        .with_context(|| format!("Could not parse .dc stop {}", tokens[3]))?;
    let step = parse_spice_number(&tokens[4])
        .with_context(|| format!("Could not parse .dc step {}", tokens[4]))?;
    if !start.is_finite()
        || !stop.is_finite()
        || !step.is_finite()
        || step <= 0.0
        || (stop - start).abs() < f64::EPSILON
        || step > (stop - start).abs()
    {
        bail!(
            ".dc sweep must use finite distinct start/stop values and a positive step no larger than the sweep span."
        );
    }
    Ok(DcSweepSpec {
        source: tokens[1].clone(),
        start,
        stop,
        step,
    })
}

fn parse_noise(tokens: &[String]) -> Result<NoiseSpec> {
    if tokens.len() < 7 {
        bail!(
            ".noise directive requires output, input source, sweep type, point count, start, and stop."
        );
    }
    let (output_node, reference_node) = parse_noise_output_expression(&tokens[1])?;
    let sweep = parse_frequency_sweep(".noise", &tokens[3], &tokens[4], &tokens[5], &tokens[6])?;
    Ok(NoiseSpec {
        output_node,
        reference_node,
        input_source: tokens[2].clone(),
        sweep_type: sweep.sweep_type,
        start_frequency_hz: sweep.start_frequency_hz,
        stop_frequency_hz: sweep.stop_frequency_hz,
        points_per_decade: sweep.points_per_decade,
    })
}

fn parse_frequency_sweep(
    directive: &str,
    sweep_type_token: &str,
    points_token: &str,
    start_token: &str,
    stop_token: &str,
) -> Result<AcSpec> {
    let sweep_type = match sweep_type_token.to_ascii_lowercase().as_str() {
        "dec" | "oct" | "lin" => sweep_type_token.to_ascii_lowercase(),
        _ => bail!("{directive} sweep type must be dec, oct, or lin."),
    };
    let points_per_decade = points_token
        .parse::<u32>()
        .with_context(|| format!("Could not parse {directive} point count {points_token}"))?;
    let start_frequency_hz = parse_spice_number(start_token)
        .with_context(|| format!("Could not parse {directive} start frequency {start_token}"))?;
    let stop_frequency_hz = parse_spice_number(stop_token)
        .with_context(|| format!("Could not parse {directive} stop frequency {stop_token}"))?;
    if points_per_decade == 0
        || points_per_decade > 1000
        || !start_frequency_hz.is_finite()
        || !stop_frequency_hz.is_finite()
        || start_frequency_hz <= 0.0
        || stop_frequency_hz <= start_frequency_hz
    {
        bail!(
            "{directive} sweep must use positive finite start/stop frequencies and point count in 1..=1000."
        );
    }
    Ok(AcSpec {
        sweep_type,
        start_frequency_hz,
        stop_frequency_hz,
        points_per_decade,
    })
}

fn parse_noise_output_expression(expression: &str) -> Result<(String, Option<String>)> {
    let trimmed = expression.trim();
    if trimmed.len() < 4 || !trimmed[..2].eq_ignore_ascii_case("v(") || !trimmed.ends_with(')') {
        bail!("SPICE .noise output must be a voltage expression like V(out) or V(out,ref).");
    }
    let inner = &trimmed[2..trimmed.len() - 1];
    let mut nodes = inner.split(',').map(str::trim);
    let Some(output_node) = nodes.next().filter(|value| !value.is_empty()) else {
        bail!("SPICE .noise output expression is missing an output node.");
    };
    if is_ground_node(output_node) {
        bail!("SPICE .noise output node must not be ground.");
    }
    let reference_node = nodes
        .next()
        .filter(|value| !value.is_empty() && !is_ground_node(value))
        .map(str::to_string);
    if nodes.next().is_some() {
        bail!("SPICE .noise output expression accepts at most output and reference nodes.");
    }
    Ok((output_node.to_string(), reference_node))
}

fn parse_transfer_function(tokens: &[String]) -> Result<TransferFunctionSpec> {
    if tokens.len() < 3 {
        bail!(".tf directive requires output expression and input source.");
    }
    let output_expression = tokens[1].trim();
    if !is_supported_output_expression(output_expression) {
        bail!("SPICE .tf output must be a voltage or current expression like V(out) or I(V1).");
    }
    let input_source = tokens[2].trim();
    if input_source.is_empty() {
        bail!("SPICE .tf input source must be non-empty.");
    }
    Ok(TransferFunctionSpec {
        output_expression: output_expression.to_string(),
        input_source: input_source.to_string(),
    })
}

fn parse_pole_zero_directive(tokens: &[String]) -> Result<PoleZeroDirective> {
    if tokens.len() < 7 {
        bail!(".pz directive requires input nodes, output nodes, transfer kind, and mode.");
    }
    let source_kind = match tokens[5].to_ascii_lowercase().as_str() {
        "vol" => ImportedSourceKind::Voltage,
        "cur" => ImportedSourceKind::Current,
        _ => bail!("SPICE .pz transfer kind must be vol or cur."),
    };
    let mode = match tokens[6].to_ascii_lowercase().as_str() {
        "pol" => "poles",
        "zer" => "zeros",
        "pz" => "poles_and_zeros",
        _ => bail!("SPICE .pz mode must be pol, zer, or pz."),
    };
    Ok(PoleZeroDirective {
        input_positive_node: tokens[1].clone(),
        input_negative_node: tokens[2].clone(),
        output_node: tokens[3].clone(),
        reference_node: tokens[4].clone(),
        source_kind,
        mode: mode.to_string(),
    })
}

fn parse_sensitivity(tokens: &[String]) -> Result<SensitivityDirective> {
    if tokens.len() < 2 {
        bail!(".sens directive requires an output expression.");
    }
    let output_expression = tokens[1].trim();
    if !is_supported_output_expression(output_expression) {
        bail!("SPICE .sens output must be a voltage or current expression like V(out) or I(V1).");
    }
    if tokens.len() == 2 || (tokens.len() == 3 && tokens[2].eq_ignore_ascii_case("dc")) {
        return Ok(SensitivityDirective {
            output_expression: output_expression.to_string(),
            mode: "dc".to_string(),
            ac: None,
        });
    }
    if tokens.len() == 7 && tokens[2].eq_ignore_ascii_case("ac") {
        let ac_tokens = vec![
            ".ac".to_string(),
            tokens[3].clone(),
            tokens[4].clone(),
            tokens[5].clone(),
            tokens[6].clone(),
        ];
        return Ok(SensitivityDirective {
            output_expression: output_expression.to_string(),
            mode: "ac".to_string(),
            ac: Some(parse_ac(&ac_tokens)?),
        });
    }
    bail!(
        "import-spice sensitivity import supports .sens OUTPUT_EXPR or .sens OUTPUT_EXPR ac dec|oct|lin POINTS START STOP."
    );
}

fn parse_distortion(tokens: &[String]) -> Result<DistortionDirective> {
    if tokens.len() < 5 {
        bail!(".disto directive requires sweep type, point count, start, and stop.");
    }
    let sweep = parse_frequency_sweep(".disto", &tokens[1], &tokens[2], &tokens[3], &tokens[4])?;
    let f2_over_f1 = if tokens.len() >= 6 {
        let ratio = parse_spice_number(&tokens[5])
            .with_context(|| format!("Could not parse .disto f2overf1 {}", tokens[5]))?;
        if !ratio.is_finite() || ratio <= 0.0 || ratio >= 1.0 {
            bail!(".disto intermodulation f2overf1 must be finite and in 0..1.");
        }
        Some(ratio)
    } else {
        None
    };
    Ok(DistortionDirective {
        sweep_type: sweep.sweep_type,
        start_frequency_hz: sweep.start_frequency_hz,
        stop_frequency_hz: sweep.stop_frequency_hz,
        points_per_decade: sweep.points_per_decade,
        f2_over_f1,
    })
}

fn parse_print_or_plot_distortion_output(tokens: &[String]) -> Result<Option<String>> {
    if tokens.len() < 3 || !tokens[1].eq_ignore_ascii_case("disto") {
        return Ok(None);
    }
    if tokens.len() != 3 {
        bail!(
            "import-spice distortion import requires exactly one .print/.plot disto output expression."
        );
    }
    let output_expression = tokens[2].trim();
    if !is_supported_output_expression(output_expression) {
        bail!(
            "SPICE .print/.plot disto output must be a voltage or current expression like V(out) or I(V1)."
        );
    }
    Ok(Some(output_expression.to_string()))
}

fn parse_control_distortion_output(tokens: &[String]) -> Result<Option<String>> {
    if tokens.len() != 2 {
        return Ok(None);
    }
    let output_expression = tokens[1].trim();
    if is_supported_output_expression(output_expression) {
        Ok(Some(output_expression.to_string()))
    } else {
        Ok(None)
    }
}

fn parse_print_or_plot_probe_outputs(tokens: &[String]) -> Result<Vec<OutputProbeSpec>> {
    if tokens.len() < 3 {
        return Ok(Vec::new());
    }
    if tokens[1].eq_ignore_ascii_case("disto") {
        return Ok(Vec::new());
    }
    if is_supported_analysis_print_mode(&tokens[1]) {
        return output_probes_from_tokens(&tokens[2..]);
    }
    Ok(Vec::new())
}

fn parse_control_probe_outputs(tokens: &[String]) -> Result<Vec<OutputProbeSpec>> {
    if tokens.len() < 2 {
        return Ok(Vec::new());
    }
    output_probes_from_tokens(&tokens[1..])
}

fn parse_save_probe_outputs(tokens: &[String]) -> Result<Vec<OutputProbeSpec>> {
    if tokens.len() < 2 {
        return Ok(Vec::new());
    }
    output_probes_from_tokens(&tokens[1..])
}

fn output_probes_from_tokens(tokens: &[String]) -> Result<Vec<OutputProbeSpec>> {
    let mut probes = Vec::new();
    for token in tokens {
        let expression = token.trim();
        if expression.eq_ignore_ascii_case("all") {
            continue;
        }
        let Some(expression) = normalize_probe_output_expression(expression) else {
            bail!(
                "SPICE output directive expression {expression} is not supported; import-spice currently accepts V(...), I(...), voltage aliases like VM(...)/VDB(...), and simple vector wrappers like MAG(V(...)) or DB(I(...))."
            )
        };
        probes.push(OutputProbeSpec {
            expression: expression.clone(),
            quantity: output_expression_quantity(&expression),
        });
    }
    Ok(probes)
}

fn is_supported_analysis_print_mode(mode: &str) -> bool {
    matches!(
        mode.to_ascii_lowercase().as_str(),
        "tran" | "ac" | "dc" | "op" | "noise" | "tf" | "pz" | "sens"
    )
}

fn output_expression_quantity(expression: &str) -> Option<&'static str> {
    if expression
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("i("))
    {
        Some("current")
    } else {
        None
    }
}

fn normalize_probe_output_expression(expression: &str) -> Option<String> {
    let trimmed = expression.trim();
    if let Some(vector) = normalize_vector_output_expression(trimmed) {
        return Some(vector);
    }
    let (function, inner) = split_function_call(trimmed)?;
    if matches!(
        function.to_ascii_lowercase().as_str(),
        "db" | "mag" | "phase" | "cph" | "real" | "imag"
    ) {
        normalize_vector_output_expression(inner)
    } else {
        None
    }
}

fn normalize_vector_output_expression(expression: &str) -> Option<String> {
    let (function, inner) = split_function_call(expression.trim())?;
    let function = function.to_ascii_lowercase();
    match function.as_str() {
        "v" => normalize_voltage_output_expression(inner),
        "i" => normalize_current_output_expression(inner),
        "vdb" | "vm" | "vp" | "vr" | "vi" => normalize_voltage_output_expression(inner),
        _ => None,
    }
}

fn split_function_call(expression: &str) -> Option<(&str, &str)> {
    let trimmed = expression.trim();
    let open = trimmed.find('(')?;
    if !trimmed.ends_with(')') {
        return None;
    }
    let function = trimmed[..open].trim();
    if function.is_empty()
        || !function
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphabetic())
    {
        return None;
    }
    Some((function, trimmed[open + 1..trimmed.len() - 1].trim()))
}

fn normalize_voltage_output_expression(inner: &str) -> Option<String> {
    let nodes = inner
        .split(',')
        .map(str::trim)
        .filter(|node| !node.is_empty())
        .collect::<Vec<_>>();
    if !(1..=2).contains(&nodes.len()) {
        return None;
    }
    Some(format!("V({})", nodes.join(",")))
}

fn normalize_current_output_expression(inner: &str) -> Option<String> {
    let source = inner.trim();
    if source.is_empty() || source.contains(',') {
        return None;
    }
    Some(format!("I({source})"))
}

fn pole_zero_spec_from_directive(
    directive: PoleZeroDirective,
    elements: &[ParsedElement],
) -> Result<PoleZeroSpec> {
    let matching_sources = elements
        .iter()
        .filter(|element| {
            matches!(
                (directive.source_kind, element.spice.as_ref()),
                (
                    ImportedSourceKind::Voltage,
                    Some(
                        SpicePrimitiveSpec::DcVoltageSource { .. }
                            | SpicePrimitiveSpec::PulseVoltageSource { .. }
                    )
                ) | (
                    ImportedSourceKind::Current,
                    Some(
                        SpicePrimitiveSpec::DcCurrentSource { .. }
                            | SpicePrimitiveSpec::PulseCurrentSource { .. }
                    )
                )
            )
        })
        .filter(|element| {
            element
                .pins
                .iter()
                .any(|(pin, node)| pin == "P" && node == &directive.input_positive_node)
                && element
                    .pins
                    .iter()
                    .any(|(pin, node)| pin == "N" && node == &directive.input_negative_node)
        })
        .collect::<Vec<_>>();
    let [input_source] = matching_sources.as_slice() else {
        bail!(
            "SPICE .pz input nodes {} {} must match exactly one imported {} source.",
            directive.input_positive_node,
            directive.input_negative_node,
            match directive.source_kind {
                ImportedSourceKind::Voltage => "voltage",
                ImportedSourceKind::Current => "current",
            }
        );
    };
    Ok(PoleZeroSpec {
        output_node: directive.output_node,
        reference_node: directive.reference_node,
        input_source: input_source.name.clone(),
        mode: directive.mode,
    })
}

fn sensitivity_spec_from_directive(
    directive: SensitivityDirective,
    elements: &[ParsedElement],
) -> SensitivitySpec {
    SensitivitySpec {
        output_expression: directive.output_expression,
        mode: directive.mode,
        ac: directive.ac,
        filters: sensitivity_filters_from_elements(elements),
    }
}

fn distortion_spec_from_directive(
    directive: DistortionDirective,
    output_expression: Option<String>,
    elements: &[ParsedElement],
) -> Result<DistortionSpec> {
    let Some(output_expression) = output_expression else {
        bail!(
            "SPICE .disto import requires a supported output expression from .print disto, .plot disto, or a control-block print command."
        );
    };
    let f1_sources = elements
        .iter()
        .filter(|element| element.distortion_role.f1)
        .map(|element| element.name.clone())
        .collect::<Vec<_>>();
    let f2_sources = elements
        .iter()
        .filter(|element| element.distortion_role.f2)
        .map(|element| element.name.clone())
        .collect::<Vec<_>>();
    if f1_sources.is_empty() {
        bail!("SPICE .disto import requires at least one independent source marked DISTOF1.");
    }
    let mode = if f2_sources.is_empty() {
        if directive.f2_over_f1.is_some() {
            bail!("SPICE .disto f2overf1 requires at least one independent source marked DISTOF2.");
        }
        "harmonic"
    } else {
        if directive.f2_over_f1.is_none() {
            bail!("SPICE .disto DISTOF2 sources require an explicit f2overf1 ratio.");
        }
        "intermodulation"
    };
    Ok(DistortionSpec {
        mode: mode.to_string(),
        sweep_type: directive.sweep_type,
        start_frequency_hz: directive.start_frequency_hz,
        stop_frequency_hz: directive.stop_frequency_hz,
        points_per_decade: directive.points_per_decade,
        output_expression,
        f1_sources,
        f2_sources,
        f2_over_f1: directive.f2_over_f1,
    })
}

fn sensitivity_filters_from_elements(elements: &[ParsedElement]) -> Vec<String> {
    elements
        .iter()
        .filter(|element| {
            matches!(
                element.spice,
                Some(
                    SpicePrimitiveSpec::Resistor { .. }
                        | SpicePrimitiveSpec::Capacitor { .. }
                        | SpicePrimitiveSpec::Inductor { .. }
                )
            )
        })
        .map(|element| element.name.clone())
        .collect()
}

fn distortion_role_from_tokens(tokens: &[String]) -> DistortionSourceRole {
    let mut role = DistortionSourceRole::default();
    for token in tokens {
        if token.eq_ignore_ascii_case("distof1") {
            role.f1 = true;
        }
        if token.eq_ignore_ascii_case("distof2") {
            role.f2 = true;
        }
    }
    role
}

fn parse_fourier(tokens: &[String]) -> Result<Vec<FourierSpec>> {
    if tokens.len() < 3 {
        bail!(".four directive requires fundamental frequency and at least one output expression.");
    }
    let fundamental_frequency_hz = parse_spice_number(&tokens[1])
        .with_context(|| format!("Could not parse .four fundamental frequency {}", tokens[1]))?;
    if !fundamental_frequency_hz.is_finite() || fundamental_frequency_hz <= 0.0 {
        bail!(".four fundamental frequency must be positive and finite.");
    }
    tokens[2..]
        .iter()
        .map(|token| {
            let output_expression = token.trim();
            if !is_supported_output_expression(output_expression) {
                bail!(
                    "SPICE .four output must be a voltage or current expression like V(out) or I(V1)."
                );
            }
            Ok(FourierSpec {
                fundamental_frequency_hz,
                output_expression: output_expression.to_string(),
            })
        })
        .collect()
}

fn is_supported_output_expression(expression: &str) -> bool {
    let trimmed = expression.trim();
    trimmed.ends_with(')')
        && trimmed.get(..2).is_some_and(|prefix| {
            prefix.eq_ignore_ascii_case("v(") || prefix.eq_ignore_ascii_case("i(")
        })
}

fn parse_measure_statement(tokens: &[String], line: &str) -> Result<MeasureStatementSpec> {
    if tokens.len() < 4 {
        bail!("SPICE .meas statement must include mode, name, operation, and expression.");
    }
    let command = tokens[0].trim_start_matches('.');
    if !command.eq_ignore_ascii_case("meas") && !command.eq_ignore_ascii_case("measure") {
        bail!("SPICE measure statement must start with meas, .meas, measure, or .measure.");
    }
    let mode = tokens[1].to_ascii_lowercase();
    if !matches!(mode.as_str(), "tran" | "ac") {
        bail!(
            "SPICE .meas mode {} is not supported by import-spice.",
            tokens[1]
        );
    }
    Ok(MeasureStatementSpec {
        mode,
        name: tokens[2].clone(),
        statement: line.to_string(),
    })
}

fn push_measure_statement(
    measures: &mut Vec<MeasureStatementSpec>,
    names: &mut BTreeSet<String>,
    measure: MeasureStatementSpec,
) -> Result<()> {
    if !names.insert(measure.name.to_ascii_lowercase()) {
        bail!("Duplicate SPICE .meas result name {}.", measure.name);
    }
    measures.push(measure);
    Ok(())
}

fn build_project_yaml(
    options: &SpiceImportOptions,
    deck: &ParsedDeck,
    output_dir: &Path,
) -> Result<ProjectYaml> {
    let mut components = BTreeMap::new();
    let mut pin_bindings = Vec::new();
    for element in &deck.elements {
        let mut pins = BTreeMap::new();
        for (pin, node) in &element.pins {
            let net = net_name_for_node(node);
            pins.insert(pin.clone(), net.clone());
            pin_bindings.push(PinBindingYaml {
                node: node.clone(),
                endpoint: EndpointYaml {
                    component: element.name.clone(),
                    pin: pin.clone(),
                },
            });
        }
        components.insert(
            element.name.clone(),
            ComponentYaml {
                model: element.model.clone(),
                pins,
                spice: element.spice.as_ref().map(ComponentSpiceYaml::from),
            },
        );
    }
    let mut nets = BTreeMap::new();
    for node in &deck.nodes {
        let kind = if is_ground_node(node) {
            "ground"
        } else {
            "digital_or_analog"
        };
        nets.insert(
            net_name_for_node(node),
            NetYaml {
                kind: kind.to_string(),
            },
        );
    }
    let node_bindings: Vec<NodeBindingYaml> = deck
        .nodes
        .iter()
        .map(|node| NodeBindingYaml {
            node: node.clone(),
            net: net_name_for_node(node),
        })
        .collect();
    let mut probes = deck
        .nodes
        .iter()
        .filter(|node| !is_ground_node(node))
        .map(|node| ProbeYaml {
            name: format!("v_{}", sanitize_identifier(node)),
            expression: format!("V({node})"),
            quantity: None,
        })
        .collect::<Vec<_>>();
    probes.extend(
        deck.elements
            .iter()
            .filter(|element| matches!(element.source_kind, Some(ImportedSourceKind::Voltage)))
            .map(|element| ProbeYaml {
                name: format!("i_{}", sanitize_identifier(&element.name)),
                expression: format!("I({})", element.name),
                quantity: Some("current"),
            }),
    );
    append_explicit_probes(&mut probes, &deck.explicit_probes);
    let tran = deck.tran.as_ref();
    let stop_time_us = tran.map_or(options.stop_time_us, |value| value.stop_time_us);
    let max_step_us = tran.map_or(options.max_step_us, |value| value.max_step_us);
    let parts = AnalogScenarioParts {
        netlist: path_for_yaml(&options.input, output_dir),
        parameters: scenario_parameters_for_directives(&deck.directive_metadata),
        operating_conditions: operating_conditions_for_directives(&deck.directive_metadata),
        model_files: model_files_for_yaml(&deck.includes, output_dir)?,
        node_bindings,
        pin_bindings,
        stimuli: vec![StimulusYaml {
            name: "imported_deck_sources".to_string(),
            description: "Stimuli are defined by independent sources in the imported SPICE deck."
                .to_string(),
        }],
        probes,
    };
    let mut scenarios = Vec::new();
    if deck.op {
        scenarios.push(operating_point_scenario_for_yaml(options, &parts));
    }
    if let Some(dc) = &deck.dc {
        scenarios.push(dc_sweep_scenario_for_yaml(options, &parts, dc));
    }
    if let Some(ac) = &deck.ac {
        scenarios.push(ac_scenario_for_yaml(options, &parts, ac));
    }
    if let Some(noise) = &deck.noise {
        scenarios.push(noise_scenario_for_yaml(options, &parts, noise));
    }
    if let Some(transfer_function) = &deck.transfer_function {
        scenarios.push(transfer_function_scenario_for_yaml(
            options,
            &parts,
            transfer_function,
        ));
    }
    if let Some(pole_zero) = &deck.pole_zero {
        scenarios.push(pole_zero_scenario_for_yaml(options, &parts, pole_zero));
    }
    if let Some(sensitivity) = &deck.sensitivity {
        scenarios.push(sensitivity_scenario_for_yaml(options, &parts, sensitivity));
    }
    if let Some(distortion) = &deck.distortion {
        scenarios.push(distortion_scenario_for_yaml(options, &parts, distortion));
    }
    let multiple_fourier_outputs = deck.fourier.len() > 1;
    for (index, fourier) in deck.fourier.iter().enumerate() {
        scenarios.push(fourier_scenario_for_yaml(
            options,
            &parts,
            fourier,
            index,
            multiple_fourier_outputs,
            stop_time_us,
            max_step_us,
        ));
    }
    if deck.tran.is_some()
        || (!deck.op
            && deck.dc.is_none()
            && deck.ac.is_none()
            && deck.noise.is_none()
            && deck.transfer_function.is_none()
            && deck.pole_zero.is_none()
            && deck.sensitivity.is_none()
            && deck.distortion.is_none()
            && deck.fourier.is_empty())
    {
        scenarios.push(transient_scenario_for_yaml(
            options,
            &parts,
            stop_time_us,
            max_step_us,
        ));
    }
    if !deck.measures.is_empty() {
        scenarios.push(measure_scenario_for_yaml(
            options,
            deck,
            parts,
            stop_time_us,
            max_step_us,
        )?);
    }
    Ok(ProjectYaml {
        project: ProjectMetaYaml {
            name: options.name.clone(),
            version: "0.1.0".to_string(),
        },
        libraries: vec![generic_analog_library_path()],
        board: BoardYaml { components, nets },
        scenarios,
    })
}

fn append_explicit_probes(probes: &mut Vec<ProbeYaml>, explicit: &[OutputProbeSpec]) {
    let mut seen = probes
        .iter()
        .map(|probe| probe.expression.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    for probe in explicit {
        if !seen.insert(probe.expression.to_ascii_lowercase()) {
            continue;
        }
        probes.push(ProbeYaml {
            name: format!("probe_{}", sanitize_identifier(&probe.expression)),
            expression: probe.expression.clone(),
            quantity: probe.quantity,
        });
    }
}

impl From<&SpicePrimitiveSpec> for ComponentSpiceYaml {
    fn from(value: &SpicePrimitiveSpec) -> Self {
        match value {
            SpicePrimitiveSpec::Resistor { value_ohm } => Self {
                primitive: "resistor".to_string(),
                value_ohm: Some(*value_ohm),
                value_f: None,
                initial_v: None,
                value_h: None,
                dc_v: None,
                dc_a: None,
                pulse: None,
                current_pulse: None,
            },
            SpicePrimitiveSpec::Capacitor { value_f } => Self {
                primitive: "capacitor".to_string(),
                value_ohm: None,
                value_f: Some(*value_f),
                initial_v: None,
                value_h: None,
                dc_v: None,
                dc_a: None,
                pulse: None,
                current_pulse: None,
            },
            SpicePrimitiveSpec::Inductor { value_h } => Self {
                primitive: "inductor".to_string(),
                value_ohm: None,
                value_f: None,
                initial_v: None,
                value_h: Some(*value_h),
                dc_v: None,
                dc_a: None,
                pulse: None,
                current_pulse: None,
            },
            SpicePrimitiveSpec::DcVoltageSource { dc_v } => Self {
                primitive: "dc_voltage_source".to_string(),
                value_ohm: None,
                value_f: None,
                initial_v: None,
                value_h: None,
                dc_v: Some(*dc_v),
                dc_a: None,
                pulse: None,
                current_pulse: None,
            },
            SpicePrimitiveSpec::PulseVoltageSource { pulse } => Self {
                primitive: "pulse_voltage_source".to_string(),
                value_ohm: None,
                value_f: None,
                initial_v: None,
                value_h: None,
                dc_v: None,
                dc_a: None,
                pulse: Some(PulseSpec {
                    initial_v: pulse.initial_v,
                    pulsed_v: pulse.pulsed_v,
                    delay_us: pulse.delay_us,
                    rise_us: pulse.rise_us,
                    fall_us: pulse.fall_us,
                    width_us: pulse.width_us,
                    period_us: pulse.period_us,
                }),
                current_pulse: None,
            },
            SpicePrimitiveSpec::DcCurrentSource { dc_a } => Self {
                primitive: "dc_current_source".to_string(),
                value_ohm: None,
                value_f: None,
                initial_v: None,
                value_h: None,
                dc_v: None,
                dc_a: Some(*dc_a),
                pulse: None,
                current_pulse: None,
            },
            SpicePrimitiveSpec::PulseCurrentSource { pulse } => Self {
                primitive: "pulse_current_source".to_string(),
                value_ohm: None,
                value_f: None,
                initial_v: None,
                value_h: None,
                dc_v: None,
                dc_a: None,
                pulse: None,
                current_pulse: Some(CurrentPulseSpec {
                    initial_a: pulse.initial_a,
                    pulsed_a: pulse.pulsed_a,
                    delay_us: pulse.delay_us,
                    rise_us: pulse.rise_us,
                    fall_us: pulse.fall_us,
                    width_us: pulse.width_us,
                    period_us: pulse.period_us,
                }),
            },
        }
    }
}

fn model_files_for_yaml(includes: &[IncludeFile], output_dir: &Path) -> Result<Vec<ModelFileYaml>> {
    let mut seen = BTreeSet::new();
    let mut files = Vec::new();
    for include in includes {
        let path = path_for_yaml(&include.resolved, output_dir);
        if !seen.insert(path.clone()) {
            continue;
        }
        files.push(ModelFileYaml {
            path,
            sha256: Some(file_sha256_hex(&include.resolved)?),
        });
    }
    Ok(files)
}

fn file_sha256_hex(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn path_for_yaml(path: &Path, output_dir: &Path) -> String {
    if path.is_absolute() {
        return path.to_string_lossy().to_string();
    }
    let candidate = output_dir.join(path);
    if candidate.exists() {
        path.to_string_lossy().to_string()
    } else {
        fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_string()
    }
}

fn generic_analog_library_path() -> String {
    fs::canonicalize("libs/generic/analog")
        .unwrap_or_else(|_| PathBuf::from("libs/generic/analog"))
        .to_string_lossy()
        .to_string()
}

fn net_name_for_node(node: &str) -> String {
    if is_ground_node(node) {
        "gnd".to_string()
    } else {
        format!("net_{}", sanitize_identifier(node))
    }
}

fn is_ground_node(node: &str) -> bool {
    matches!(node.to_ascii_lowercase().as_str(), "0" | "gnd" | "ground")
}

fn sanitize_identifier(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
        } else {
            output.push('_');
        }
    }
    output.trim_matches('_').to_string()
}
