use crate::analog_model_resolver::declared_model_file_path_for_source_dir;
use anyhow::{Context, Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
#[path = "spice_tests.rs"]
mod spice_tests;

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
    fourier: Vec<FourierSpec>,
    measures: Vec<MeasureStatementSpec>,
}

#[derive(Debug)]
struct ParsedElement {
    name: String,
    model: String,
    pins: Vec<(String, String)>,
    spice: Option<SpicePrimitiveSpec>,
    source_kind: Option<ImportedSourceKind>,
}

#[derive(Debug, Clone, Copy)]
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
    analog: AnalogYaml,
}

#[derive(Debug, Serialize)]
struct AnalogYaml {
    backend: String,
    netlist_source: String,
    netlist: String,
    model_files: Vec<ModelFileYaml>,
    node_bindings: Vec<NodeBindingYaml>,
    pin_bindings: Vec<PinBindingYaml>,
    analysis: AnalysisYaml,
    stimuli: Vec<StimulusYaml>,
    probes: Vec<ProbeYaml>,
    assertions: Vec<AssertionYaml>,
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
    let mut fourier = Vec::new();
    let mut measures = Vec::new();
    let mut measure_names = BTreeSet::new();
    let mut in_control = false;
    for line in logical_lines {
        let tokens = tokenize(&line);
        if tokens.is_empty() {
            continue;
        }
        let first = tokens[0].as_str();
        let command = first.to_ascii_lowercase();
        if in_control {
            match command.as_str() {
                ".endc" | "endc" => in_control = false,
                ".tran" | "tran" => tran = parse_tran(&tokens).or(tran),
                ".op" | "op" => op = true,
                ".dc" | "dc" => dc = Some(parse_dc_sweep(&tokens)?),
                ".ac" | "ac" => ac = Some(parse_ac(&tokens)?),
                ".noise" | "noise" => noise = Some(parse_noise(&tokens)?),
                ".tf" | "tf" => transfer_function = Some(parse_transfer_function(&tokens)?),
                ".four" | "four" | "fourier" => fourier.extend(parse_fourier(&tokens)?),
                ".meas" | ".measure" | "meas" | "measure" => {
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
                ".include" | ".lib" => includes.push(parse_include(&tokens, source_dir)?),
                ".tran" => tran = parse_tran(&tokens).or(tran),
                ".op" => op = true,
                ".dc" => dc = Some(parse_dc_sweep(&tokens)?),
                ".ac" => ac = Some(parse_ac(&tokens)?),
                ".noise" => noise = Some(parse_noise(&tokens)?),
                ".tf" => transfer_function = Some(parse_transfer_function(&tokens)?),
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
    if elements.is_empty() {
        bail!(
            "SPICE deck {} contains no importable elements.",
            path.display()
        );
    }
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
        fourier,
        measures,
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
        if character.is_whitespace() {
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
    if !tokens[1].eq_ignore_ascii_case("dec") {
        bail!("import-spice AC import currently requires .ac dec sweeps.");
    }
    let points_per_decade = tokens[2]
        .parse::<u32>()
        .with_context(|| format!("Could not parse .ac points-per-decade {}", tokens[2]))?;
    let start_frequency_hz = parse_spice_number(&tokens[3])
        .with_context(|| format!("Could not parse .ac start frequency {}", tokens[3]))?;
    let stop_frequency_hz = parse_spice_number(&tokens[4])
        .with_context(|| format!("Could not parse .ac stop frequency {}", tokens[4]))?;
    if points_per_decade == 0
        || !start_frequency_hz.is_finite()
        || !stop_frequency_hz.is_finite()
        || start_frequency_hz <= 0.0
        || stop_frequency_hz <= start_frequency_hz
    {
        bail!(".ac dec sweep must use positive finite start/stop frequencies and points.");
    }
    Ok(AcSpec {
        start_frequency_hz,
        stop_frequency_hz,
        points_per_decade,
    })
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
    if !tokens[3].eq_ignore_ascii_case("dec") {
        bail!("import-spice noise import currently requires .noise ... dec POINTS START STOP.");
    }
    let points_per_decade = tokens[4]
        .parse::<u32>()
        .with_context(|| format!("Could not parse .noise points-per-decade {}", tokens[4]))?;
    let start_frequency_hz = parse_spice_number(&tokens[5])
        .with_context(|| format!("Could not parse .noise start frequency {}", tokens[5]))?;
    let stop_frequency_hz = parse_spice_number(&tokens[6])
        .with_context(|| format!("Could not parse .noise stop frequency {}", tokens[6]))?;
    if points_per_decade == 0
        || points_per_decade > 1000
        || !start_frequency_hz.is_finite()
        || !stop_frequency_hz.is_finite()
        || start_frequency_hz <= 0.0
        || stop_frequency_hz <= start_frequency_hz
    {
        bail!(
            ".noise dec sweep must use positive finite start/stop frequencies and points in 1..=1000."
        );
    }
    Ok(NoiseSpec {
        output_node,
        reference_node,
        input_source: tokens[2].clone(),
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

fn parse_element(tokens: &[String], line: &str) -> Result<ParsedElement> {
    let name = tokens[0].clone();
    validate_refdes(&name)?;
    let prefix = name
        .chars()
        .next()
        .expect("tokenized element name is not empty")
        .to_ascii_uppercase();
    match prefix {
        'R' => parse_two_terminal(tokens, "generic.analog.resistor", |value| {
            SpicePrimitiveSpec::Resistor { value_ohm: value }
        }),
        'C' => parse_two_terminal(tokens, "generic.analog.capacitor", |value| {
            SpicePrimitiveSpec::Capacitor { value_f: value }
        }),
        'L' => parse_two_terminal(tokens, "generic.analog.inductor", |value| {
            SpicePrimitiveSpec::Inductor { value_h: value }
        }),
        'V' => parse_voltage_source(tokens),
        'I' => parse_current_source(tokens),
        'E' => parse_voltage_controlled_source(tokens, ImportedSourceKind::Voltage),
        'G' => parse_voltage_controlled_source(tokens, ImportedSourceKind::Current),
        'F' => parse_current_controlled_source(tokens, ImportedSourceKind::Current),
        'H' => parse_current_controlled_source(tokens, ImportedSourceKind::Voltage),
        'B' => parse_behavioral_source(tokens),
        'D' => parse_fixed_pins(
            tokens,
            3,
            &["A", "K"],
            "generic.analog.imported_spice_device",
        ),
        'Q' => {
            if tokens.len() >= 5 {
                let pin_names = if tokens.len() >= 6 {
                    vec!["C", "B", "E", "S"]
                } else {
                    vec!["C", "B", "E"]
                };
                parse_fixed_pins(
                    tokens,
                    pin_names.len() + 2,
                    &pin_names,
                    "generic.analog.imported_spice_device",
                )
            } else {
                bail!("Malformed BJT element line: {line}")
            }
        }
        'M' => parse_fixed_pins(
            tokens,
            6,
            &["D", "G", "S", "B"],
            "generic.analog.imported_spice_device",
        ),
        'X' => parse_subckt(tokens, line),
        _ if tokens.len() >= 4 => parse_fixed_pins(
            tokens,
            4,
            &["A", "B"],
            "generic.analog.imported_spice_device",
        ),
        _ => bail!("Unsupported or malformed SPICE element line: {line}"),
    }
}

fn parse_two_terminal<F>(tokens: &[String], model: &str, primitive: F) -> Result<ParsedElement>
where
    F: FnOnce(f64) -> SpicePrimitiveSpec,
{
    if tokens.len() < 4 {
        bail!(
            "Malformed two-terminal SPICE element line: {}",
            tokens.join(" ")
        );
    }
    let spice = parse_spice_number(&tokens[3]).map(primitive);
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: model.to_string(),
        pins: vec![
            ("A".to_string(), tokens[1].clone()),
            ("B".to_string(), tokens[2].clone()),
        ],
        spice,
        source_kind: None,
    })
}

fn parse_voltage_source(tokens: &[String]) -> Result<ParsedElement> {
    if tokens.len() < 4 {
        bail!(
            "Malformed voltage-source SPICE element line: {}",
            tokens.join(" ")
        );
    }
    let spice = parse_voltage_source_primitive(tokens)?;
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: vec![
            ("P".to_string(), tokens[1].clone()),
            ("N".to_string(), tokens[2].clone()),
        ],
        spice,
        source_kind: Some(ImportedSourceKind::Voltage),
    })
}

fn parse_current_source(tokens: &[String]) -> Result<ParsedElement> {
    if tokens.len() < 4 {
        bail!(
            "Malformed current-source SPICE element line: {}",
            tokens.join(" ")
        );
    }
    let spice = parse_current_source_primitive(tokens)?;
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: vec![
            ("P".to_string(), tokens[1].clone()),
            ("N".to_string(), tokens[2].clone()),
        ],
        spice,
        source_kind: Some(ImportedSourceKind::Current),
    })
}

fn parse_voltage_controlled_source(
    tokens: &[String],
    source_kind: ImportedSourceKind,
) -> Result<ParsedElement> {
    if tokens.len() < 6 {
        bail!(
            "Malformed voltage-controlled SPICE source line: {}",
            tokens.join(" ")
        );
    }
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: vec![
            ("P".to_string(), tokens[1].clone()),
            ("N".to_string(), tokens[2].clone()),
            ("CP".to_string(), tokens[3].clone()),
            ("CN".to_string(), tokens[4].clone()),
        ],
        spice: None,
        source_kind: Some(source_kind),
    })
}

fn parse_current_controlled_source(
    tokens: &[String],
    source_kind: ImportedSourceKind,
) -> Result<ParsedElement> {
    if tokens.len() < 5 {
        bail!(
            "Malformed current-controlled SPICE source line: {}",
            tokens.join(" ")
        );
    }
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: vec![
            ("P".to_string(), tokens[1].clone()),
            ("N".to_string(), tokens[2].clone()),
        ],
        spice: None,
        source_kind: Some(source_kind),
    })
}

fn parse_behavioral_source(tokens: &[String]) -> Result<ParsedElement> {
    if tokens.len() < 4 {
        bail!(
            "Malformed behavioral SPICE source line: {}",
            tokens.join(" ")
        );
    }
    let source_expression = tokens[3..].join("");
    let source_expression = source_expression.to_ascii_uppercase();
    let source_kind = if source_expression.starts_with("V=") {
        Some(ImportedSourceKind::Voltage)
    } else if source_expression.starts_with("I=") {
        Some(ImportedSourceKind::Current)
    } else {
        None
    };
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: vec![
            ("P".to_string(), tokens[1].clone()),
            ("N".to_string(), tokens[2].clone()),
        ],
        spice: None,
        source_kind,
    })
}

fn parse_fixed_pins(
    tokens: &[String],
    min_tokens: usize,
    pin_names: &[&str],
    model: &str,
) -> Result<ParsedElement> {
    if tokens.len() < min_tokens {
        bail!("Malformed SPICE element line: {}", tokens.join(" "));
    }
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: model.to_string(),
        pins: pin_names
            .iter()
            .enumerate()
            .map(|(index, pin)| ((*pin).to_string(), tokens[index + 1].clone()))
            .collect(),
        spice: None,
        source_kind: None,
    })
}

fn parse_subckt(tokens: &[String], line: &str) -> Result<ParsedElement> {
    if tokens.len() < 4 {
        bail!("Malformed subcircuit instance line: {line}");
    }
    let node_count = tokens.len() - 2;
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: (0..node_count)
            .map(|index| (format!("P{}", index + 1), tokens[index + 1].clone()))
            .collect(),
        spice: None,
        source_kind: None,
    })
}

fn parse_voltage_source_primitive(tokens: &[String]) -> Result<Option<SpicePrimitiveSpec>> {
    let spec = tokens[3..].join(" ");
    if spec.trim_start().to_ascii_uppercase().starts_with("PULSE") {
        return Ok(Some(SpicePrimitiveSpec::PulseVoltageSource {
            pulse: parse_pulse(&spec)?,
        }));
    }
    if source_uses_file_backed_waveform(tokens) {
        return Ok(None);
    }
    Ok(Some(SpicePrimitiveSpec::DcVoltageSource {
        dc_v: parse_independent_source_dc_value(tokens, "voltage")?,
    }))
}

fn parse_current_source_primitive(tokens: &[String]) -> Result<Option<SpicePrimitiveSpec>> {
    let spec = tokens[3..].join(" ");
    if spec.trim_start().to_ascii_uppercase().starts_with("PULSE") {
        return Ok(Some(SpicePrimitiveSpec::PulseCurrentSource {
            pulse: parse_current_pulse(&spec)?,
        }));
    }
    if source_uses_file_backed_waveform(tokens) {
        return Ok(None);
    }
    Ok(Some(SpicePrimitiveSpec::DcCurrentSource {
        dc_a: parse_independent_source_dc_value(tokens, "current")?,
    }))
}

fn source_uses_file_backed_waveform(tokens: &[String]) -> bool {
    tokens[3..].iter().any(|token| {
        let upper = token.trim_start().to_ascii_uppercase();
        let function = upper
            .split_once('(')
            .map_or(upper.as_str(), |(name, _)| name);
        matches!(function, "SIN" | "SINE" | "PWL" | "EXP" | "SFFM" | "AM")
    })
}

fn parse_pulse(spec: &str) -> Result<PulseSpec> {
    let start = spec.find('(').context("PULSE source is missing '('")?;
    let end = spec.rfind(')').context("PULSE source is missing ')'")?;
    let values: Vec<_> = spec[start + 1..end]
        .split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .map(parse_spice_number)
        .collect::<Option<Vec<_>>>()
        .context("PULSE source contains an unparseable numeric value")?;
    if values.len() < 7 {
        bail!("PULSE source requires at least seven values.");
    }
    Ok(PulseSpec {
        initial_v: values[0],
        pulsed_v: values[1],
        delay_us: values[2] * 1_000_000.0,
        rise_us: values[3] * 1_000_000.0,
        fall_us: values[4] * 1_000_000.0,
        width_us: values[5] * 1_000_000.0,
        period_us: values[6] * 1_000_000.0,
    })
}

fn parse_independent_source_dc_value(tokens: &[String], kind: &str) -> Result<f64> {
    if tokens[3].eq_ignore_ascii_case("AC") {
        return Ok(0.0);
    }
    let value_token = if tokens[3].eq_ignore_ascii_case("DC") {
        if tokens.len() < 5 {
            bail!("{} source DC keyword requires a value.", kind);
        }
        &tokens[4]
    } else {
        &tokens[3]
    };
    parse_spice_number(value_token)
        .with_context(|| format!("Could not parse {kind} source value {value_token}"))
}

fn parse_current_pulse(spec: &str) -> Result<CurrentPulseSpec> {
    let pulse = parse_pulse(spec)?;
    Ok(CurrentPulseSpec {
        initial_a: pulse.initial_v,
        pulsed_a: pulse.pulsed_v,
        delay_us: pulse.delay_us,
        rise_us: pulse.rise_us,
        fall_us: pulse.fall_us,
        width_us: pulse.width_us,
        period_us: pulse.period_us,
    })
}

fn validate_refdes(name: &str) -> Result<()> {
    if name.is_empty()
        || !name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
    {
        bail!("SPICE element name {name:?} cannot be represented as a Board IR component id.");
    }
    Ok(())
}

fn parse_spice_number(token: &str) -> Option<f64> {
    let upper = token.trim().to_ascii_uppercase();
    for (suffix, scale) in [
        ("MEG", 1e6),
        ("T", 1e12),
        ("G", 1e9),
        ("K", 1e3),
        ("M", 1e-3),
        ("U", 1e-6),
        ("N", 1e-9),
        ("P", 1e-12),
        ("F", 1e-15),
    ] {
        if let Some(number) = upper.strip_suffix(suffix) {
            return number.parse::<f64>().ok().map(|value| value * scale);
        }
    }
    upper.parse::<f64>().ok()
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
    let tran = deck.tran.as_ref();
    let stop_time_us = tran.map_or(options.stop_time_us, |value| value.stop_time_us);
    let max_step_us = tran.map_or(options.max_step_us, |value| value.max_step_us);
    let parts = AnalogScenarioParts {
        netlist: path_for_yaml(&options.input, output_dir),
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

fn transient_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    stop_time_us: f64,
    max_step_us: f64,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_transient".to_string(),
        scenario_type: "analog_transient".to_string(),
        checks: vec!["SPICE_TRANSIENT_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                analysis_type: "tran".to_string(),
                stop_time_us: Some(stop_time_us),
                max_step_us: Some(max_step_us),
                start_frequency_hz: None,
                stop_frequency_hz: None,
                points_per_decade: None,
                noise_output_node: None,
                noise_reference_node: None,
                noise_input_source: None,
                transfer_output_expression: None,
                transfer_input_source: None,
                transfer_function_assertions: Vec::new(),
                fourier_fundamental_frequency_hz: None,
                fourier_output_expression: None,
                fourier_harmonics: None,
                fourier_assertions: Vec::new(),
                dc_sweep_source: None,
                dc_sweep_start: None,
                dc_sweep_stop: None,
                dc_sweep_step: None,
                dc_sweep_assertions: Vec::new(),
                measure_mode: None,
                measure_statements: Vec::new(),
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

fn operating_point_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_operating_point".to_string(),
        scenario_type: "analog_dc".to_string(),
        checks: vec!["SPICE_DC_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                analysis_type: "op".to_string(),
                stop_time_us: None,
                max_step_us: None,
                start_frequency_hz: None,
                stop_frequency_hz: None,
                points_per_decade: None,
                noise_output_node: None,
                noise_reference_node: None,
                noise_input_source: None,
                transfer_output_expression: None,
                transfer_input_source: None,
                transfer_function_assertions: Vec::new(),
                fourier_fundamental_frequency_hz: None,
                fourier_output_expression: None,
                fourier_harmonics: None,
                fourier_assertions: Vec::new(),
                dc_sweep_source: None,
                dc_sweep_start: None,
                dc_sweep_stop: None,
                dc_sweep_step: None,
                dc_sweep_assertions: Vec::new(),
                measure_mode: None,
                measure_statements: Vec::new(),
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

fn dc_sweep_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    dc: &DcSweepSpec,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_dc_sweep".to_string(),
        scenario_type: "analog_dc_sweep".to_string(),
        checks: vec!["SPICE_DC_SWEEP_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                analysis_type: "dc_sweep".to_string(),
                stop_time_us: None,
                max_step_us: None,
                start_frequency_hz: None,
                stop_frequency_hz: None,
                points_per_decade: None,
                noise_output_node: None,
                noise_reference_node: None,
                noise_input_source: None,
                transfer_output_expression: None,
                transfer_input_source: None,
                transfer_function_assertions: Vec::new(),
                fourier_fundamental_frequency_hz: None,
                fourier_output_expression: None,
                fourier_harmonics: None,
                fourier_assertions: Vec::new(),
                dc_sweep_source: Some(dc.source.clone()),
                dc_sweep_start: Some(dc.start),
                dc_sweep_stop: Some(dc.stop),
                dc_sweep_step: Some(dc.step),
                dc_sweep_assertions: Vec::new(),
                measure_mode: None,
                measure_statements: Vec::new(),
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

fn ac_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    ac: &AcSpec,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_ac".to_string(),
        scenario_type: "analog_ac".to_string(),
        checks: vec!["SPICE_AC_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                analysis_type: "ac".to_string(),
                stop_time_us: None,
                max_step_us: None,
                start_frequency_hz: Some(ac.start_frequency_hz),
                stop_frequency_hz: Some(ac.stop_frequency_hz),
                points_per_decade: Some(ac.points_per_decade),
                noise_output_node: None,
                noise_reference_node: None,
                noise_input_source: None,
                transfer_output_expression: None,
                transfer_input_source: None,
                transfer_function_assertions: Vec::new(),
                fourier_fundamental_frequency_hz: None,
                fourier_output_expression: None,
                fourier_harmonics: None,
                fourier_assertions: Vec::new(),
                dc_sweep_source: None,
                dc_sweep_start: None,
                dc_sweep_stop: None,
                dc_sweep_step: None,
                dc_sweep_assertions: Vec::new(),
                measure_mode: None,
                measure_statements: Vec::new(),
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

fn noise_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    noise: &NoiseSpec,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_noise".to_string(),
        scenario_type: "analog_noise".to_string(),
        checks: vec!["SPICE_NOISE_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                analysis_type: "noise".to_string(),
                stop_time_us: None,
                max_step_us: None,
                start_frequency_hz: Some(noise.start_frequency_hz),
                stop_frequency_hz: Some(noise.stop_frequency_hz),
                points_per_decade: Some(noise.points_per_decade),
                noise_output_node: Some(noise.output_node.clone()),
                noise_reference_node: noise.reference_node.clone(),
                noise_input_source: Some(noise.input_source.clone()),
                transfer_output_expression: None,
                transfer_input_source: None,
                transfer_function_assertions: Vec::new(),
                fourier_fundamental_frequency_hz: None,
                fourier_output_expression: None,
                fourier_harmonics: None,
                fourier_assertions: Vec::new(),
                dc_sweep_source: None,
                dc_sweep_start: None,
                dc_sweep_stop: None,
                dc_sweep_step: None,
                dc_sweep_assertions: Vec::new(),
                measure_mode: None,
                measure_statements: Vec::new(),
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

fn transfer_function_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    transfer_function: &TransferFunctionSpec,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_transfer_function".to_string(),
        scenario_type: "analog_transfer_function".to_string(),
        checks: vec!["SPICE_TRANSFER_FUNCTION_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                analysis_type: "tf".to_string(),
                stop_time_us: None,
                max_step_us: None,
                start_frequency_hz: None,
                stop_frequency_hz: None,
                points_per_decade: None,
                noise_output_node: None,
                noise_reference_node: None,
                noise_input_source: None,
                transfer_output_expression: Some(transfer_function.output_expression.clone()),
                transfer_input_source: Some(transfer_function.input_source.clone()),
                transfer_function_assertions: Vec::new(),
                fourier_fundamental_frequency_hz: None,
                fourier_output_expression: None,
                fourier_harmonics: None,
                fourier_assertions: Vec::new(),
                dc_sweep_source: None,
                dc_sweep_start: None,
                dc_sweep_stop: None,
                dc_sweep_step: None,
                dc_sweep_assertions: Vec::new(),
                measure_mode: None,
                measure_statements: Vec::new(),
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

fn fourier_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    fourier: &FourierSpec,
    index: usize,
    multiple_outputs: bool,
    stop_time_us: f64,
    max_step_us: f64,
) -> ScenarioYaml {
    let name = if multiple_outputs {
        format!("imported_spice_fourier_{}", index + 1)
    } else {
        "imported_spice_fourier".to_string()
    };
    ScenarioYaml {
        name,
        scenario_type: "analog_fourier".to_string(),
        checks: vec!["SPICE_FOURIER_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                analysis_type: "fourier".to_string(),
                stop_time_us: Some(stop_time_us),
                max_step_us: Some(max_step_us),
                start_frequency_hz: None,
                stop_frequency_hz: None,
                points_per_decade: None,
                noise_output_node: None,
                noise_reference_node: None,
                noise_input_source: None,
                transfer_output_expression: None,
                transfer_input_source: None,
                transfer_function_assertions: Vec::new(),
                fourier_fundamental_frequency_hz: Some(fourier.fundamental_frequency_hz),
                fourier_output_expression: Some(fourier.output_expression.clone()),
                fourier_harmonics: None,
                fourier_assertions: Vec::new(),
                dc_sweep_source: None,
                dc_sweep_start: None,
                dc_sweep_stop: None,
                dc_sweep_step: None,
                dc_sweep_assertions: Vec::new(),
                measure_mode: None,
                measure_statements: Vec::new(),
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

fn measure_scenario_for_yaml(
    options: &SpiceImportOptions,
    deck: &ParsedDeck,
    parts: AnalogScenarioParts,
    stop_time_us: f64,
    max_step_us: f64,
) -> Result<ScenarioYaml> {
    let mode = common_measure_mode(&deck.measures)?;
    let (
        tran_stop_time_us,
        tran_max_step_us,
        start_frequency_hz,
        stop_frequency_hz,
        points_per_decade,
    ) = if mode == "ac" {
        let ac = deck
            .ac
            .as_ref()
            .context("SPICE .meas ac import requires a valid .ac dec sweep.")?;
        (
            None,
            None,
            Some(ac.start_frequency_hz),
            Some(ac.stop_frequency_hz),
            Some(ac.points_per_decade),
        )
    } else {
        (Some(stop_time_us), Some(max_step_us), None, None, None)
    };
    Ok(ScenarioYaml {
        name: format!("{}_measure", mode),
        scenario_type: "analog_measure".to_string(),
        checks: vec!["SPICE_MEASURE_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist,
            model_files: parts.model_files,
            node_bindings: parts.node_bindings,
            pin_bindings: parts.pin_bindings,
            analysis: AnalysisYaml {
                analysis_type: "measure".to_string(),
                stop_time_us: tran_stop_time_us,
                max_step_us: tran_max_step_us,
                start_frequency_hz,
                stop_frequency_hz,
                points_per_decade,
                noise_output_node: None,
                noise_reference_node: None,
                noise_input_source: None,
                transfer_output_expression: None,
                transfer_input_source: None,
                transfer_function_assertions: Vec::new(),
                fourier_fundamental_frequency_hz: None,
                fourier_output_expression: None,
                fourier_harmonics: None,
                fourier_assertions: Vec::new(),
                dc_sweep_source: None,
                dc_sweep_start: None,
                dc_sweep_stop: None,
                dc_sweep_step: None,
                dc_sweep_assertions: Vec::new(),
                measure_mode: Some(mode.to_string()),
                measure_statements: deck
                    .measures
                    .iter()
                    .map(|measure| MeasureStatementYaml {
                        name: measure.name.clone(),
                        statement: measure.statement.clone(),
                    })
                    .collect(),
            },
            stimuli: parts.stimuli,
            probes: parts.probes,
            assertions: Vec::new(),
        },
    })
}

fn common_measure_mode(measures: &[MeasureStatementSpec]) -> Result<&str> {
    let first = measures
        .first()
        .context("measure scenario requires at least one measure statement")?;
    if let Some(other) = measures.iter().find(|measure| measure.mode != first.mode) {
        bail!(
            "SPICE .meas statements mix modes {} and {}; import-spice emits one measure scenario per deck.",
            first.mode,
            other.mode
        );
    }
    Ok(&first.mode)
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
