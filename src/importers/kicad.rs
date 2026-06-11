use anyhow::{Context, Result, bail};
mod passive_values;
use passive_values::resolve_component_spice;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct KicadImportOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub name: String,
    pub default_model: String,
    pub mapping: Option<PathBuf>,
}

#[derive(Debug)]
pub(super) struct LoadedKicadMapping {
    mapping: KicadMapping,
    base_dir: PathBuf,
}

#[derive(Debug)]
pub(super) struct ParsedKicadNetlist {
    pub(super) components: BTreeMap<String, ParsedComponent>,
    pub(super) nets: Vec<ParsedNet>,
}

#[derive(Debug)]
pub(super) struct ParsedComponent {
    pub(super) refdes: String,
    pub(super) value: Option<String>,
    pub(super) lib: Option<String>,
    pub(super) part: Option<String>,
    pub(super) fields: BTreeMap<String, String>,
}

#[derive(Debug)]
pub(super) struct ParsedNet {
    pub(super) code: String,
    pub(super) name: String,
    pub(super) nodes: Vec<ParsedNode>,
}

#[derive(Debug)]
pub(super) struct ParsedNode {
    pub(super) refdes: String,
    pub(super) pin: String,
}

#[derive(Debug, Deserialize)]
struct ImportedComponentModel {
    component_id: String,
    #[serde(default)]
    ports: BTreeMap<String, serde_yaml_ng::Value>,
    #[serde(default)]
    simulation: ImportedModelSimulation,
}

#[derive(Debug, Default, Deserialize)]
struct ImportedModelSimulation {
    #[serde(default)]
    spice: Option<ImportedSpiceModel>,
}

#[derive(Debug, Deserialize)]
struct ImportedSpiceModel {
    model_path: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct KicadMapping {
    #[serde(default)]
    libraries: Vec<String>,
    #[serde(default)]
    components: BTreeMap<String, ComponentMapping>,
    #[serde(default)]
    libsource_rules: Vec<LibsourceRuleMapping>,
    #[serde(default)]
    nets: BTreeMap<String, NetMapping>,
    #[serde(default)]
    analog_scenarios: Vec<AnalogScenarioMapping>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ComponentMapping {
    model: String,
    #[serde(default)]
    pin_map: BTreeMap<String, String>,
    #[serde(default)]
    part_number: Option<String>,
    #[serde(default)]
    spice: Option<ComponentSpiceYaml>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LibsourceRuleMapping {
    lib: String,
    part: String,
    #[serde(default)]
    value: Option<String>,
    model: String,
    #[serde(default)]
    pin_map: BTreeMap<String, String>,
    #[serde(default)]
    spice: Option<ComponentSpiceYaml>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct NetMapping {
    #[serde(default)]
    kind: Option<MappedNetKind>,
    #[serde(default)]
    nominal_voltage: Option<f64>,
    #[serde(default)]
    powered: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ComponentSpiceYaml {
    primitive: SpicePrimitiveYaml,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_ohm: Option<f64>,
    #[serde(default, skip_serializing)]
    value_ohm_from: Option<SpiceValueSourceYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_f: Option<f64>,
    #[serde(default, skip_serializing)]
    value_f_from: Option<SpiceValueSourceYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dc_v: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pulse: Option<PulseSpecYaml>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SpicePrimitiveYaml {
    Resistor,
    Capacitor,
    DcVoltageSource,
    PulseVoltageSource,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SpiceValueSourceYaml {
    SchematicValue,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PulseSpecYaml {
    initial_v: f64,
    pulsed_v: f64,
    delay_us: f64,
    rise_us: f64,
    fall_us: f64,
    width_us: f64,
    period_us: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalogScenarioMapping {
    name: String,
    #[serde(default)]
    backend: AnalogBackendYaml,
    components: Vec<String>,
    ground_net: String,
    #[serde(default)]
    operating_conditions: OperatingConditionsYaml,
    #[serde(default)]
    model_files: Vec<ModelFileYaml>,
    analysis: AnalysisYaml,
    stimuli: Vec<StimulusYaml>,
    probes: Vec<ProbeYaml>,
    assertions: Vec<AssertionYaml>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OperatingConditionsYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    ambient_temperature_c: Option<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    allow_pulse_ratings: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum AnalogBackendYaml {
    #[default]
    Auto,
    Ngspice,
    Xyce,
    EmbeddedNgspice,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MappedNetKind {
    Power,
    Ground,
    DigitalOrAnalog,
}

impl MappedNetKind {
    fn as_board_ir(&self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Ground => "ground",
            Self::DigitalOrAnalog => "digital_or_analog",
        }
    }
}

#[derive(Debug)]
enum TextTarget {
    Value,
    Field(String),
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
    import_source: String,
}

#[derive(Debug, Serialize)]
struct BoardYaml {
    components: BTreeMap<String, ComponentYaml>,
    nets: BTreeMap<String, NetYaml>,
}

#[derive(Debug, Serialize)]
struct ComponentYaml {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    part_number: Option<String>,
    pins: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spice: Option<ComponentSpiceYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<ComponentSourceYaml>,
}

#[derive(Debug, Serialize)]
struct ComponentSourceYaml {
    format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lib: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    part: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct NetYaml {
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    nominal_voltage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    powered: Option<bool>,
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
    backend: AnalogBackendYaml,
    netlist_source: String,
    generated: GeneratedNetlistYaml,
    #[serde(skip_serializing_if = "OperatingConditionsYaml::is_default")]
    operating_conditions: OperatingConditionsYaml,
    model_files: Vec<ModelFileYaml>,
    node_bindings: Vec<NodeBindingYaml>,
    pin_bindings: Vec<PinBindingYaml>,
    analysis: AnalysisYaml,
    stimuli: Vec<StimulusYaml>,
    probes: Vec<ProbeYaml>,
    assertions: Vec<AssertionYaml>,
}

impl OperatingConditionsYaml {
    fn is_default(value: &Self) -> bool {
        value.ambient_temperature_c.is_none() && !value.allow_pulse_ratings
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Serialize)]
struct GeneratedNetlistYaml {
    components: Vec<String>,
    ground_net: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelFileYaml {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
}

#[derive(Debug, Serialize)]
struct NodeBindingYaml {
    node: String,
    net: String,
}

#[derive(Debug, Serialize)]
struct PinBindingYaml {
    node: String,
    endpoint: EndpointYaml,
}

#[derive(Debug, Serialize)]
struct EndpointYaml {
    component: String,
    pin: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AnalysisYaml {
    #[serde(rename = "type")]
    analysis_type: AnalysisTypeYaml,
    stop_time_us: f64,
    max_step_us: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum AnalysisTypeYaml {
    Tran,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StimulusYaml {
    name: String,
    description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProbeYaml {
    name: String,
    expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    quantity: Option<ProbeQuantityYaml>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProbeQuantityYaml {
    Voltage,
    Current,
    Power,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AssertionYaml {
    name: String,
    probe: String,
    relation: AssertionRelationYaml,
    #[serde(skip_serializing_if = "Option::is_none")]
    at_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aggregation: Option<AssertionAggregationYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    threshold_v: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    threshold_a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    threshold_w: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum AssertionRelationYaml {
    Below,
    Above,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum AssertionAggregationYaml {
    Sample,
    Min,
    Max,
}

pub fn import_kicad_netlist(options: &KicadImportOptions) -> Result<()> {
    let parsed = parse_kicad_netlist(&options.input)?;
    import_parsed_kicad(
        options,
        &parsed,
        "kicad_xml_netlist",
        "# Generated by CircuitCI from a KiCad generic XML netlist. Add scenarios before sign-off.\n",
    )
}

pub(super) fn import_parsed_kicad(
    options: &KicadImportOptions,
    parsed: &ParsedKicadNetlist,
    import_source: &str,
    header: &str,
) -> Result<()> {
    let loaded_mapping = load_mapping(options)?;
    let output_dir = options.output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create import output directory {}",
            output_dir.display()
        )
    })?;
    let project = build_project_yaml(options, parsed, &loaded_mapping, import_source)?;
    let mut yaml = serde_yaml_ng::to_string(&project)?;
    yaml.insert_str(0, header);
    fs::write(&options.output, yaml)
        .with_context(|| format!("Failed to write {}", options.output.display()))?;
    Ok(())
}

pub(super) fn parse_kicad_netlist(path: &Path) -> Result<ParsedKicadNetlist> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("Failed to read KiCad netlist {}", path.display()))?;
    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<String> = Vec::new();
    let mut components = BTreeMap::new();
    let mut nets = Vec::new();
    let mut current_component: Option<ParsedComponent> = None;
    let mut current_net: Option<ParsedNet> = None;
    let mut text_target: Option<TextTarget> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let tag = local_name(event.name().as_ref());
                let parent = stack.last().map(String::as_str);
                match (tag.as_str(), parent) {
                    ("comp", Some("components")) => {
                        let refdes = required_attr(&reader, &event, "ref")?;
                        if components.contains_key(&refdes) {
                            bail!("Duplicate KiCad component reference {refdes}.");
                        }
                        current_component = Some(ParsedComponent {
                            refdes,
                            value: None,
                            lib: None,
                            part: None,
                            fields: BTreeMap::new(),
                        });
                    }
                    ("net", Some("nets")) => {
                        current_net = Some(ParsedNet {
                            code: attr_value(&reader, &event, "code")?.unwrap_or_default(),
                            name: attr_value(&reader, &event, "name")?.unwrap_or_default(),
                            nodes: Vec::new(),
                        });
                    }
                    ("value", Some("comp")) if current_component.is_some() => {
                        text_target = Some(TextTarget::Value);
                    }
                    ("field", Some("fields")) if current_component.is_some() => {
                        text_target =
                            Some(TextTarget::Field(required_attr(&reader, &event, "name")?));
                    }
                    ("libsource", Some("comp")) if current_component.is_some() => {
                        apply_libsource(&reader, &event, current_component.as_mut().unwrap())?;
                    }
                    ("node", Some("net")) if current_net.is_some() => {
                        push_node(&reader, &event, current_net.as_mut().unwrap())?;
                    }
                    _ => {}
                }
                stack.push(tag);
            }
            Ok(Event::Empty(event)) => {
                let tag = local_name(event.name().as_ref());
                let parent = stack.last().map(String::as_str);
                match (tag.as_str(), parent) {
                    ("libsource", Some("comp")) if current_component.is_some() => {
                        apply_libsource(&reader, &event, current_component.as_mut().unwrap())?;
                    }
                    ("node", Some("net")) if current_net.is_some() => {
                        push_node(&reader, &event, current_net.as_mut().unwrap())?;
                    }
                    ("field", Some("fields")) if current_component.is_some() => {
                        let name = required_attr(&reader, &event, "name")?;
                        current_component
                            .as_mut()
                            .unwrap()
                            .fields
                            .insert(name, String::new());
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(event)) => {
                if let (Some(component), Some(target)) =
                    (current_component.as_mut(), text_target.as_ref())
                {
                    let value = event
                        .xml_content()
                        .context("Failed to decode KiCad XML text")?
                        .trim()
                        .to_string();
                    match target {
                        TextTarget::Value => component.value = non_empty(value),
                        TextTarget::Field(name) => {
                            component.fields.insert(name.clone(), value);
                        }
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = local_name(event.name().as_ref());
                match tag.as_str() {
                    "value" | "field" => text_target = None,
                    "comp" => {
                        let Some(component) = current_component.take() else {
                            bail!("KiCad XML closed a component without opening one.");
                        };
                        components.insert(component.refdes.clone(), component);
                    }
                    "net" => {
                        let Some(net) = current_net.take() else {
                            bail!("KiCad XML closed a net without opening one.");
                        };
                        nets.push(net);
                    }
                    _ => {}
                }
                let popped = stack.pop();
                if popped.as_deref() != Some(tag.as_str()) {
                    bail!("Malformed KiCad XML nesting near closing tag {tag}.");
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => bail!("Failed to parse KiCad XML netlist: {error}"),
            _ => {}
        }
    }

    if components.is_empty() {
        bail!("KiCad netlist {} contains no components.", path.display());
    }
    if current_component.is_some() || current_net.is_some() || !stack.is_empty() {
        bail!(
            "KiCad netlist {} ended with unclosed XML tags.",
            path.display()
        );
    }
    Ok(ParsedKicadNetlist { components, nets })
}

fn build_project_yaml(
    options: &KicadImportOptions,
    parsed: &ParsedKicadNetlist,
    loaded_mapping: &LoadedKicadMapping,
    import_source: &str,
) -> Result<ProjectYaml> {
    let mapping = &loaded_mapping.mapping;
    validate_mapping_refs(parsed, mapping)?;
    validate_mapping_models(
        parsed,
        mapping,
        &loaded_mapping.base_dir,
        &options.default_model,
    )?;
    let net_names = unique_net_names(&parsed.nets);
    let mut components = parsed
        .components
        .iter()
        .map(|(refdes, component)| {
            let component_mapping = mapping_for_component(component, mapping)?;
            let spice = resolve_component_spice(component, component_mapping.as_ref())?;
            Ok((
                refdes.clone(),
                ComponentYaml {
                    model: model_for_component(
                        component,
                        component_mapping.as_ref(),
                        &options.default_model,
                    ),
                    part_number: component_mapping
                        .as_ref()
                        .and_then(|item| item.part_number.clone())
                        .or_else(|| component.value.clone()),
                    pins: BTreeMap::new(),
                    spice,
                    source: Some(ComponentSourceYaml {
                        format: import_source.to_string(),
                        value: component.value.clone(),
                        lib: component.lib.clone(),
                        part: component.part.clone(),
                        fields: component.fields.clone(),
                    }),
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut assigned_pins = BTreeSet::new();
    for (net_index, net) in parsed.nets.iter().enumerate() {
        let net_name = net_names[net_index].clone();
        for node in &net.nodes {
            let Some(component) = components.get_mut(&node.refdes) else {
                bail!(
                    "KiCad net {} references unknown component {}.",
                    net.name,
                    node.refdes
                );
            };
            let key = format!("{}.{}", node.refdes, node.pin);
            if !assigned_pins.insert(key.clone()) {
                bail!("KiCad component pin {key} appears on more than one net.");
            }
            let parsed_component = parsed
                .components
                .get(&node.refdes)
                .expect("component existence was checked above");
            let component_mapping = mapping_for_component(parsed_component, mapping)?;
            let target_pin = mapped_pin(
                parsed_component,
                component_mapping.as_ref(),
                &options.default_model,
                &node.pin,
            )?;
            component.pins.insert(target_pin, net_name.clone());
        }
    }
    validate_mapped_component_pins(parsed, mapping, &options.default_model)?;
    let nets = parsed
        .nets
        .iter()
        .enumerate()
        .map(|(index, net)| {
            let net_mapping = mapping.nets.get(&net.name);
            Ok((
                net_names[index].clone(),
                NetYaml {
                    kind: mapped_net_kind(&net.name, net_mapping)?,
                    nominal_voltage: net_mapping.and_then(|item| item.nominal_voltage),
                    powered: net_mapping.and_then(|item| item.powered),
                },
            ))
        })
        .collect::<Result<_>>()?;
    let import_models =
        load_import_models(&libraries_for_project(mapping, &loaded_mapping.base_dir))?;
    let scenarios = build_analog_scenarios(
        parsed,
        mapping,
        &loaded_mapping.base_dir,
        &components,
        &nets,
        &net_names,
        &import_models,
    )?;
    Ok(ProjectYaml {
        project: ProjectMetaYaml {
            name: options.name.clone(),
            version: "0.1.0".to_string(),
            import_source: import_source.to_string(),
        },
        libraries: libraries_for_project(mapping, &loaded_mapping.base_dir),
        board: BoardYaml { components, nets },
        scenarios,
    })
}

fn build_analog_scenarios(
    parsed: &ParsedKicadNetlist,
    mapping: &KicadMapping,
    mapping_base_dir: &Path,
    components: &BTreeMap<String, ComponentYaml>,
    nets: &BTreeMap<String, NetYaml>,
    net_names: &[String],
    models: &BTreeMap<String, ImportedComponentModel>,
) -> Result<Vec<ScenarioYaml>> {
    let raw_net_to_board = raw_net_to_board_map(parsed, net_names)?;
    mapping
        .analog_scenarios
        .iter()
        .map(|scenario| {
            let model_files = scenario_model_files(scenario, mapping_base_dir)?;
            validate_analog_scenario_mapping(
                scenario,
                components,
                nets,
                &raw_net_to_board,
                models,
                &model_files,
            )?;
            let generated_components = scenario.components.clone();
            let ground_net = raw_net_to_board
                .get(&scenario.ground_net)
                .cloned()
                .with_context(|| {
                    format!(
                        "KiCad analog scenario {} references unknown ground_net {}.",
                        scenario.name, scenario.ground_net
                    )
                })?;
            let mut used_nets = BTreeSet::new();
            used_nets.insert(ground_net.clone());
            for component_id in &generated_components {
                let component = components
                    .get(component_id)
                    .expect("scenario components were validated before binding generation");
                used_nets.extend(component.pins.values().cloned());
            }
            let node_bindings = used_nets
                .into_iter()
                .map(|net| NodeBindingYaml {
                    node: if net == ground_net {
                        "0".to_string()
                    } else {
                        net.clone()
                    },
                    net,
                })
                .collect::<Vec<_>>();
            let node_by_net = node_bindings
                .iter()
                .map(|binding| (binding.net.clone(), binding.node.clone()))
                .collect::<BTreeMap<_, _>>();
            let mut pin_bindings = Vec::new();
            for component_id in &generated_components {
                let component = components
                    .get(component_id)
                    .expect("scenario components were validated before pin binding generation");
                for (pin, net) in &component.pins {
                    let node = node_by_net
                        .get(net)
                        .expect("component net was inserted into node bindings")
                        .clone();
                    pin_bindings.push(PinBindingYaml {
                        node,
                        endpoint: EndpointYaml {
                            component: component_id.clone(),
                            pin: pin.clone(),
                        },
                    });
                }
            }
            Ok(ScenarioYaml {
                name: scenario.name.clone(),
                scenario_type: "analog_transient".to_string(),
                checks: vec!["SPICE_TRANSIENT_ANALYSIS".to_string()],
                analog: AnalogYaml {
                    backend: scenario.backend.clone(),
                    netlist_source: "generated_from_board".to_string(),
                    generated: GeneratedNetlistYaml {
                        components: generated_components,
                        ground_net,
                    },
                    operating_conditions: scenario.operating_conditions.clone(),
                    model_files,
                    node_bindings,
                    pin_bindings,
                    analysis: scenario.analysis.clone(),
                    stimuli: scenario.stimuli.clone(),
                    probes: scenario.probes.clone(),
                    assertions: scenario.assertions.clone(),
                },
            })
        })
        .collect()
}

fn validate_analog_scenario_mapping(
    scenario: &AnalogScenarioMapping,
    components: &BTreeMap<String, ComponentYaml>,
    nets: &BTreeMap<String, NetYaml>,
    raw_net_to_board: &BTreeMap<String, String>,
    models: &BTreeMap<String, ImportedComponentModel>,
    model_files: &[ModelFileYaml],
) -> Result<()> {
    if scenario.components.is_empty() {
        bail!(
            "KiCad analog scenario {} must declare generated components.",
            scenario.name
        );
    }
    if scenario.stimuli.is_empty() {
        bail!(
            "KiCad analog scenario {} must declare at least one stimulus description.",
            scenario.name
        );
    }
    if scenario.probes.is_empty() {
        bail!(
            "KiCad analog scenario {} must declare at least one probe.",
            scenario.name
        );
    }
    if scenario.assertions.is_empty() {
        bail!(
            "KiCad analog scenario {} must declare at least one quantitative assertion.",
            scenario.name
        );
    }
    if !scenario.analysis.stop_time_us.is_finite()
        || !scenario.analysis.max_step_us.is_finite()
        || scenario.analysis.stop_time_us <= 0.0
        || scenario.analysis.max_step_us <= 0.0
        || scenario.analysis.max_step_us > scenario.analysis.stop_time_us
    {
        bail!(
            "KiCad analog scenario {} has invalid transient timing.",
            scenario.name
        );
    }
    let ground_net = raw_net_to_board
        .get(&scenario.ground_net)
        .with_context(|| {
            format!(
                "KiCad analog scenario {} references unknown ground_net {}.",
                scenario.name, scenario.ground_net
            )
        })?;
    if nets.get(ground_net).is_none_or(|net| net.kind != "ground") {
        bail!(
            "KiCad analog scenario {} ground_net {} does not map to a ground Board IR net.",
            scenario.name,
            scenario.ground_net
        );
    }
    let mut seen_components = BTreeSet::new();
    for component_id in &scenario.components {
        if !seen_components.insert(component_id) {
            bail!(
                "KiCad analog scenario {} lists component {} more than once.",
                scenario.name,
                component_id
            );
        }
        let component = components.get(component_id).with_context(|| {
            format!(
                "KiCad analog scenario {} references unknown component {}.",
                scenario.name, component_id
            )
        })?;
        if component.spice.is_none() {
            let model = models.get(&component.model).with_context(|| {
                format!(
                    "KiCad analog scenario {} selected unresolved model {} for component {}.",
                    scenario.name, component.model, component_id
                )
            })?;
            let spice = model.simulation.spice.as_ref().with_context(|| {
                format!(
                    "KiCad analog scenario {} includes component {}, but neither mapping-file spice metadata nor selected model simulation.spice metadata is available.",
                    scenario.name,
                    component_id
                )
            })?;
            require_model_file_for_component(
                scenario,
                component_id,
                &spice.model_path,
                model_files,
            )?;
        }
    }
    validate_probe_assertion_contract(scenario)
}

fn require_model_file_for_component(
    scenario: &AnalogScenarioMapping,
    component_id: &str,
    model_path: &str,
    model_files: &[ModelFileYaml],
) -> Result<()> {
    let expected = Path::new(model_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(model_path);
    let Some(model_file) = model_files.iter().find(|file| {
        file.path == model_path
            || Path::new(&file.path)
                .file_name()
                .and_then(|name| name.to_str())
                == Some(expected)
    }) else {
        bail!(
            "KiCad analog scenario {} component {} requires SPICE model file {}, but scenario.model_files does not declare it.",
            scenario.name,
            component_id,
            model_path
        );
    };
    if model_file.sha256.is_none() {
        bail!(
            "KiCad analog scenario {} component {} model file {} must declare a SHA-256 pin.",
            scenario.name,
            component_id,
            model_file.path
        );
    }
    Ok(())
}

fn scenario_model_files(
    scenario: &AnalogScenarioMapping,
    mapping_base_dir: &Path,
) -> Result<Vec<ModelFileYaml>> {
    scenario
        .model_files
        .iter()
        .map(|file| {
            let resolved = resolve_mapping_path(mapping_base_dir, &file.path);
            if !resolved.is_file() {
                bail!(
                    "KiCad analog scenario {} model file {} does not exist.",
                    scenario.name,
                    resolved.display()
                );
            }
            let actual_sha = file_sha256_hex(&resolved)?;
            let Some(expected_sha) = &file.sha256 else {
                bail!(
                    "KiCad analog scenario {} model file {} must declare sha256.",
                    scenario.name,
                    file.path
                );
            };
            if !actual_sha.eq_ignore_ascii_case(expected_sha) {
                bail!(
                    "KiCad analog scenario {} model file {} SHA-256 mismatch.",
                    scenario.name,
                    resolved.display()
                );
            }
            Ok(ModelFileYaml {
                path: fs::canonicalize(&resolved)
                    .unwrap_or(resolved)
                    .to_string_lossy()
                    .to_string(),
                sha256: Some(expected_sha.to_ascii_lowercase()),
            })
        })
        .collect()
}

fn file_sha256_hex(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn resolve_mapping_path(mapping_base_dir: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        mapping_base_dir.join(path)
    }
}

fn validate_probe_assertion_contract(scenario: &AnalogScenarioMapping) -> Result<()> {
    let probes = scenario
        .probes
        .iter()
        .map(|probe| probe.name.as_str())
        .collect::<BTreeSet<_>>();
    for assertion in &scenario.assertions {
        if !probes.contains(assertion.probe.as_str()) {
            bail!(
                "KiCad analog scenario {} assertion {} references unknown probe {}.",
                scenario.name,
                assertion.name,
                assertion.probe
            );
        }
        let threshold_count = [
            assertion.threshold_v,
            assertion.threshold_a,
            assertion.threshold_w,
        ]
        .into_iter()
        .filter(|value| value.is_some_and(f64::is_finite))
        .count();
        if threshold_count != 1 {
            bail!(
                "KiCad analog scenario {} assertion {} must declare exactly one finite threshold.",
                scenario.name,
                assertion.name
            );
        }
        let is_window = matches!(
            assertion.aggregation,
            Some(AssertionAggregationYaml::Min | AssertionAggregationYaml::Max)
        );
        if is_window {
            if assertion.start_us.is_none()
                || assertion.end_us.is_none()
                || assertion.at_us.is_some()
            {
                bail!(
                    "KiCad analog scenario {} assertion {} has an invalid window/sample timing contract.",
                    scenario.name,
                    assertion.name
                );
            }
        } else if assertion.at_us.is_none()
            || assertion.start_us.is_some()
            || assertion.end_us.is_some()
        {
            bail!(
                "KiCad analog scenario {} assertion {} has an invalid sample timing contract.",
                scenario.name,
                assertion.name
            );
        }
    }
    Ok(())
}

fn load_mapping(options: &KicadImportOptions) -> Result<LoadedKicadMapping> {
    let Some(path) = &options.mapping else {
        let base_dir = options.input.parent().unwrap_or_else(|| Path::new("."));
        return Ok(LoadedKicadMapping {
            mapping: KicadMapping::default(),
            base_dir: base_dir.to_path_buf(),
        });
    };
    let text = fs::read_to_string(path)
        .with_context(|| format!("Failed to read KiCad mapping file {}", path.display()))?;
    let mapping = serde_yaml_ng::from_str(&text)
        .with_context(|| format!("Failed to parse KiCad mapping file {}", path.display()))?;
    Ok(LoadedKicadMapping {
        mapping,
        base_dir: path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
    })
}

fn validate_mapping_refs(parsed: &ParsedKicadNetlist, mapping: &KicadMapping) -> Result<()> {
    for refdes in mapping.components.keys() {
        if !parsed.components.contains_key(refdes) {
            bail!("KiCad mapping references unknown component {refdes}.");
        }
    }
    let net_names = parsed
        .nets
        .iter()
        .map(|net| net.name.as_str())
        .collect::<BTreeSet<_>>();
    for net_name in mapping.nets.keys() {
        if !net_names.contains(net_name.as_str()) {
            bail!("KiCad mapping references unknown net {net_name}.");
        }
    }
    Ok(())
}

fn mapping_for_component(
    component: &ParsedComponent,
    mapping: &KicadMapping,
) -> Result<Option<ComponentMapping>> {
    if let Some(item) = mapping.components.get(&component.refdes) {
        return Ok(Some(item.clone()));
    }
    let matches = mapping
        .libsource_rules
        .iter()
        .filter(|rule| {
            component.lib.as_deref() == Some(rule.lib.as_str())
                && component.part.as_deref() == Some(rule.part.as_str())
                && rule
                    .value
                    .as_ref()
                    .is_none_or(|value| component.value.as_deref() == Some(value.as_str()))
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [rule] => Ok(Some(ComponentMapping {
            model: rule.model.clone(),
            pin_map: rule.pin_map.clone(),
            part_number: component.value.clone(),
            spice: rule.spice.clone(),
        })),
        _ => bail!(
            "KiCad component {} matches more than one libsource mapping rule.",
            component.refdes
        ),
    }
}

fn model_for_component(
    component: &ParsedComponent,
    mapping: Option<&ComponentMapping>,
    default_model: &str,
) -> String {
    if let Some(mapping) = mapping {
        return mapping.model.clone();
    }
    component
        .fields
        .iter()
        .find(|(name, _)| {
            name.eq_ignore_ascii_case("CircuitCI_Model")
                || name.eq_ignore_ascii_case("CircuitCIModel")
        })
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(default_model)
        .to_string()
}

fn mapped_pin(
    component: &ParsedComponent,
    mapping: Option<&ComponentMapping>,
    default_model: &str,
    imported_pin: &str,
) -> Result<String> {
    let Some(mapping) = mapping else {
        return Ok(imported_pin.to_string());
    };
    if mapping.model == default_model {
        return Ok(mapping
            .pin_map
            .get(imported_pin)
            .cloned()
            .unwrap_or_else(|| imported_pin.to_string()));
    }
    mapping
        .pin_map
        .get(imported_pin)
        .cloned()
        .with_context(|| {
            format!(
                "KiCad mapping for component {} changes model to {}, but imported pin {} is not in pin_map.",
                component.refdes, mapping.model, imported_pin
            )
        })
}

fn validate_mapped_component_pins(
    parsed: &ParsedKicadNetlist,
    mapping: &KicadMapping,
    default_model: &str,
) -> Result<()> {
    let connected = connected_pins_by_component(parsed);
    for component in parsed.components.values() {
        let Some(component_mapping) = mapping_for_component(component, mapping)? else {
            continue;
        };
        let connected_pins = connected
            .get(&component.refdes)
            .cloned()
            .unwrap_or_else(BTreeSet::new);
        for mapped_pin in component_mapping.pin_map.keys() {
            if !connected_pins.contains(mapped_pin) {
                bail!(
                    "KiCad mapping for component {} references unconnected imported pin {}.",
                    component.refdes,
                    mapped_pin
                );
            }
        }
        let mut target_pins = BTreeSet::new();
        for target_pin in component_mapping.pin_map.values() {
            if !target_pins.insert(target_pin) {
                bail!(
                    "KiCad mapping for component {} maps more than one imported pin to model pin {}.",
                    component.refdes,
                    target_pin
                );
            }
        }
        if component_mapping.model != default_model {
            for imported_pin in &connected_pins {
                if !component_mapping.pin_map.contains_key(imported_pin) {
                    bail!(
                        "KiCad mapping for component {} changes model to {}, but connected imported pin {} is not mapped.",
                        component.refdes,
                        component_mapping.model,
                        imported_pin
                    );
                }
            }
        }
    }
    Ok(())
}

fn validate_mapping_models(
    parsed: &ParsedKicadNetlist,
    mapping: &KicadMapping,
    base_dir: &Path,
    default_model: &str,
) -> Result<()> {
    let models = load_import_models(&libraries_for_project(mapping, base_dir))?;
    let connected = connected_pins_by_component(parsed);
    for component in parsed.components.values() {
        let selected_mapping = mapping_for_component(component, mapping)?;
        let model_id = model_for_component(component, selected_mapping.as_ref(), default_model);
        let model = models
            .get(&model_id)
            .with_context(|| format!("KiCad import selected unresolved model {model_id}."))?;
        let connected_pins = connected
            .get(&component.refdes)
            .cloned()
            .unwrap_or_else(BTreeSet::new);
        for imported_pin in connected_pins {
            let target_pin = mapped_pin(
                component,
                selected_mapping.as_ref(),
                default_model,
                &imported_pin,
            )?;
            if !model.ports.contains_key(&target_pin) {
                bail!(
                    "KiCad import maps component {} imported pin {} to {}.{}, but that port is not declared by the selected model.",
                    component.refdes,
                    imported_pin,
                    model_id,
                    target_pin
                );
            }
        }
    }
    Ok(())
}

fn load_import_models(libraries: &[String]) -> Result<BTreeMap<String, ImportedComponentModel>> {
    let mut models = BTreeMap::new();
    for root in libraries {
        let root = Path::new(root);
        if !root.exists() {
            bail!(
                "KiCad import library path {} does not exist.",
                root.display()
            );
        }
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".model.yaml"))
            {
                continue;
            }
            let text = fs::read_to_string(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let model: ImportedComponentModel = serde_yaml_ng::from_str(&text)
                .with_context(|| format!("Failed to parse {}", path.display()))?;
            models.insert(model.component_id.clone(), model);
        }
    }
    Ok(models)
}

fn connected_pins_by_component(parsed: &ParsedKicadNetlist) -> BTreeMap<String, BTreeSet<String>> {
    let mut connected: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for net in &parsed.nets {
        for node in &net.nodes {
            connected
                .entry(node.refdes.clone())
                .or_default()
                .insert(node.pin.clone());
        }
    }
    connected
}

fn mapped_net_kind(name: &str, mapping: Option<&NetMapping>) -> Result<String> {
    let Some(mapping) = mapping else {
        return Ok(classify_net(name).to_string());
    };
    Ok(mapping
        .kind
        .as_ref()
        .map(MappedNetKind::as_board_ir)
        .unwrap_or_else(|| classify_net(name))
        .to_string())
}

fn libraries_for_project(mapping: &KicadMapping, base_dir: &Path) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut libraries = Vec::new();
    for library in
        std::iter::once(generic_library_path()).chain(mapping.libraries.iter().map(|library| {
            let path = Path::new(library);
            if path.is_absolute() {
                library.clone()
            } else {
                let resolved = base_dir.join(path);
                fs::canonicalize(&resolved)
                    .unwrap_or(resolved)
                    .to_string_lossy()
                    .to_string()
            }
        }))
    {
        if seen.insert(library.clone()) {
            libraries.push(library);
        }
    }
    libraries
}

fn apply_libsource(
    reader: &Reader<&[u8]>,
    event: &BytesStart<'_>,
    component: &mut ParsedComponent,
) -> Result<()> {
    component.lib = attr_value(reader, event, "lib")?;
    component.part = attr_value(reader, event, "part")?;
    Ok(())
}

fn push_node(reader: &Reader<&[u8]>, event: &BytesStart<'_>, net: &mut ParsedNet) -> Result<()> {
    net.nodes.push(ParsedNode {
        refdes: required_attr(reader, event, "ref")?,
        pin: required_attr(reader, event, "pin")?,
    });
    Ok(())
}

fn attr_value(
    reader: &Reader<&[u8]>,
    event: &BytesStart<'_>,
    name: &str,
) -> Result<Option<String>> {
    for attribute in event.attributes().with_checks(true) {
        let attribute = attribute.context("Malformed KiCad XML attribute")?;
        if local_name(attribute.key.as_ref()) == name {
            let value = attribute
                .decode_and_unescape_value(reader.decoder())
                .context("Failed to decode KiCad XML attribute")?
                .to_string();
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn required_attr(reader: &Reader<&[u8]>, event: &BytesStart<'_>, name: &str) -> Result<String> {
    match attr_value(reader, event, name)? {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => bail!(
            "KiCad XML <{}> is missing required attribute {name}.",
            local_name(event.name().as_ref())
        ),
    }
}

fn unique_net_names(nets: &[ParsedNet]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    nets.iter()
        .map(|net| {
            let base = net_name_for(net);
            if seen.insert(base.clone()) {
                return base;
            }
            let mut suffix = 2;
            loop {
                let candidate = format!("{base}_{suffix}");
                if seen.insert(candidate.clone()) {
                    return candidate;
                }
                suffix += 1;
            }
        })
        .collect()
}

fn raw_net_to_board_map(
    parsed: &ParsedKicadNetlist,
    net_names: &[String],
) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for (index, net) in parsed.nets.iter().enumerate() {
        if net.name.trim().is_empty() {
            continue;
        }
        if let Some(previous) = map.insert(net.name.clone(), net_names[index].clone())
            && previous != net_names[index]
        {
            bail!(
                "KiCad net name {} maps to more than one Board IR net; use unique net names before mapping generated scenarios.",
                net.name
            );
        }
    }
    Ok(map)
}

fn net_name_for(net: &ParsedNet) -> String {
    if is_ground_net(&net.name) {
        return "gnd".to_string();
    }
    let raw = if net.name.trim().is_empty() {
        format!("code_{}", net.code)
    } else {
        net.name.clone()
    };
    format!("net_{}", sanitize_identifier(&raw))
}

fn classify_net(name: &str) -> &'static str {
    if is_ground_net(name) {
        "ground"
    } else {
        "digital_or_analog"
    }
}

fn is_ground_net(name: &str) -> bool {
    let normalized = normalize_net_name(name);
    normalized == "0" || normalized == "gnd" || normalized == "ground" || normalized.contains("gnd")
}

fn normalize_net_name(name: &str) -> String {
    name.trim()
        .trim_start_matches('/')
        .replace([' ', '_'], "")
        .to_ascii_lowercase()
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
    let trimmed = output.trim_matches('_');
    if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed.to_string()
    }
}

fn local_name(name: &[u8]) -> String {
    let text = String::from_utf8_lossy(name);
    text.rsplit(':').next().unwrap_or(&text).to_string()
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn generic_library_path() -> String {
    fs::canonicalize("libs/generic")
        .unwrap_or_else(|_| PathBuf::from("libs/generic"))
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{classify_net, parse_kicad_netlist};

    #[test]
    fn classifies_common_power_and_ground_nets() {
        assert_eq!(classify_net("GND"), "ground");
        assert_eq!(classify_net("/+3V3"), "digital_or_analog");
        assert_eq!(classify_net("UART_TX"), "digital_or_analog");
    }

    #[test]
    fn parses_basic_kicad_xml_netlist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("board.net");
        std::fs::write(
            &path,
            r#"
<export>
  <components>
    <comp ref="R1"><value>10k</value><libsource lib="Device" part="R"/></comp>
    <comp ref="C1"><value>100n</value></comp>
  </components>
  <nets>
    <net code="1" name="+3V3"><node ref="R1" pin="1"/></net>
    <net code="2" name="RC"><node ref="R1" pin="2"/><node ref="C1" pin="1"/></net>
    <net code="3" name="GND"><node ref="C1" pin="2"/></net>
  </nets>
</export>
"#,
        )
        .unwrap();
        let parsed = parse_kicad_netlist(&path).unwrap();
        assert_eq!(parsed.components.len(), 2);
        assert_eq!(parsed.nets.len(), 3);
        assert_eq!(parsed.nets[1].nodes.len(), 2);
    }
}
