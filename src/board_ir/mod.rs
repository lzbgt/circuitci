use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct BoardProject {
    pub project: ProjectMetadata,
    #[serde(default)]
    pub libraries: Vec<String>,
    pub board: Board,
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
    #[serde(skip)]
    pub source_dir: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub import_source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Board {
    #[serde(default)]
    pub components: BTreeMap<String, ComponentSpec>,
    #[serde(default)]
    pub nets: BTreeMap<String, NetSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentSpec {
    pub model: String,
    #[serde(default)]
    pub part_number: Option<String>,
    #[serde(default)]
    pub power_domain: Option<String>,
    #[serde(default)]
    pub power_domains: BTreeMap<String, String>,
    #[serde(default)]
    pub pins: BTreeMap<String, String>,
    #[serde(default)]
    pub spice: Option<ComponentSpiceSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentSpiceSpec {
    pub primitive: SpicePrimitive,
    #[serde(default)]
    pub value_ohm: Option<f64>,
    #[serde(default)]
    pub value_f: Option<f64>,
    #[serde(default)]
    pub dc_v: Option<f64>,
    #[serde(default)]
    pub pulse: Option<SpicePulseSpec>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpicePrimitive {
    Resistor,
    Capacitor,
    DcVoltageSource,
    PulseVoltageSource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpicePulseSpec {
    pub initial_v: f64,
    pub pulsed_v: f64,
    pub delay_us: f64,
    pub rise_us: f64,
    pub fall_us: f64,
    pub width_us: f64,
    pub period_us: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetSpec {
    pub kind: NetKind,
    #[serde(default)]
    pub nominal_voltage: Option<f64>,
    #[serde(default)]
    pub powered: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NetKind {
    Power,
    Ground,
    DigitalOrAnalog,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Scenario {
    pub name: String,
    #[serde(rename = "type")]
    pub scenario_type: String,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, serde_yaml_ng::Value>,
    #[serde(default)]
    pub target: Option<ScenarioTarget>,
    #[serde(default)]
    pub pin_states: Vec<PinState>,
    #[serde(default)]
    pub paths: Vec<BackdrivePath>,
    #[serde(default)]
    pub control_effects: Vec<ControlEffect>,
    #[serde(default)]
    pub timing: Option<ScenarioTiming>,
    #[serde(default)]
    pub straps: Vec<BootStrapObservation>,
    #[serde(default)]
    pub required_boot_mode: Option<String>,
    #[serde(default)]
    pub bootloader: Option<BootloaderScenario>,
    #[serde(default)]
    pub protocol: Option<ProtocolScenario>,
    #[serde(default)]
    pub analog: Option<AnalogScenario>,
    #[serde(default)]
    pub events: Vec<ScenarioEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioTarget {
    pub component: String,
    #[serde(default)]
    pub power_pin: Option<String>,
    #[serde(default)]
    pub reset_pin: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PinState {
    pub component: String,
    pub pin: String,
    pub mode: PinMode,
    #[serde(default)]
    pub state: Option<PinLogicState>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PinMode {
    Input,
    Output,
    HighZ,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PinLogicState {
    High,
    Low,
    Z,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackdrivePath {
    pub driver: Endpoint,
    pub victim: Endpoint,
    #[serde(default)]
    pub series_resistance_ohm: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlEffect {
    pub name: String,
    pub source: Endpoint,
    pub target: Endpoint,
    pub asserted_state: String,
    pub released_state: String,
    pub release_delay_us: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioTiming {
    #[serde(rename = "power_valid_at_us")]
    pub power_valid_at_us: f64,
    #[serde(rename = "reset_release_at_us")]
    pub reset_release_at_us: f64,
    #[serde(default, rename = "boot_sample_at_us")]
    pub boot_sample_at_us: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootStrapObservation {
    pub component: String,
    pub pin: String,
    #[serde(default)]
    pub net: Option<String>,
    pub actual: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootloaderScenario {
    #[serde(default)]
    pub component: Option<String>,
    pub interface: String,
    #[serde(default)]
    pub sync_byte: Option<u8>,
    #[serde(default)]
    pub expected_response: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProtocolScenario {
    #[serde(default)]
    pub component: Option<String>,
    pub name: String,
    pub flow: String,
    #[serde(default)]
    pub sender: Option<Endpoint>,
    #[serde(default)]
    pub package_size_bytes: Option<u64>,
    #[serde(default)]
    pub package_sha256: Option<String>,
    #[serde(default)]
    pub chunk_size_bytes: Option<u64>,
    #[serde(default)]
    pub expected_final_state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogScenario {
    pub backend: AnalogBackend,
    #[serde(default)]
    pub netlist_source: AnalogNetlistSource,
    #[serde(default)]
    pub netlist: Option<String>,
    #[serde(default)]
    pub generated: Option<AnalogGeneratedNetlist>,
    #[serde(default)]
    pub operating_conditions: AnalogOperatingConditions,
    pub model_files: Vec<AnalogModelFile>,
    pub node_bindings: Vec<AnalogNodeBinding>,
    pub pin_bindings: Vec<AnalogPinBinding>,
    pub analysis: AnalogTransientAnalysis,
    pub stimuli: Vec<AnalogStimulus>,
    pub probes: Vec<AnalogProbe>,
    pub assertions: Vec<AnalogAssertion>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogNetlistSource {
    #[default]
    File,
    GeneratedFromBoard,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogGeneratedNetlist {
    pub components: Vec<String>,
    pub ground_net: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AnalogOperatingConditions {
    #[serde(default)]
    pub ambient_temperature_c: Option<f64>,
    #[serde(default)]
    pub allow_pulse_ratings: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogBackend {
    Auto,
    Ngspice,
    Xyce,
    EmbeddedNgspice,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogTransientAnalysis {
    #[serde(rename = "type")]
    pub analysis_type: String,
    #[serde(rename = "stop_time_us")]
    pub stop_time_us: f64,
    #[serde(rename = "max_step_us")]
    pub max_step_us: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogModelFile {
    pub path: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogNodeBinding {
    pub node: String,
    pub net: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogPinBinding {
    pub node: String,
    pub endpoint: Endpoint,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogStimulus {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogProbe {
    pub name: String,
    pub expression: String,
    #[serde(default)]
    pub quantity: AnalogQuantity,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogQuantity {
    #[default]
    Voltage,
    Current,
    Power,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogAssertion {
    pub name: String,
    pub probe: String,
    #[serde(default, rename = "at_us")]
    pub at_us: Option<f64>,
    #[serde(default, rename = "start_us")]
    pub start_us: Option<f64>,
    #[serde(default, rename = "end_us")]
    pub end_us: Option<f64>,
    #[serde(default)]
    pub aggregation: AnalogAggregation,
    pub relation: AnalogRelation,
    #[serde(default)]
    pub threshold_v: Option<f64>,
    #[serde(default)]
    pub threshold_a: Option<f64>,
    #[serde(default)]
    pub threshold_w: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogAggregation {
    #[default]
    Sample,
    Min,
    Max,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogRelation {
    Below,
    Above,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioEvent {
    #[serde(rename = "at_us")]
    pub at_us: f64,
    pub action: String,
    #[serde(default)]
    pub from: Option<Endpoint>,
    #[serde(default)]
    pub to: Option<Endpoint>,
    #[serde(default)]
    pub bytes: Vec<u8>,
    #[serde(default)]
    pub operation: Option<String>,
    #[serde(default)]
    pub payload_len: Option<u64>,
    #[serde(default)]
    pub result_code: Option<u64>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub chunk_len: Option<u64>,
    #[serde(default)]
    pub activate_mode: Option<String>,
    #[serde(default)]
    pub line: Option<String>,
    #[serde(default)]
    pub asserted: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct Endpoint {
    pub component: String,
    pub pin: String,
}

pub fn load_project(path: &Path) -> anyhow::Result<BoardProject> {
    let text = fs::read_to_string(path)?;
    let mut project: BoardProject = serde_yaml_ng::from_str(&text)?;
    project.source_dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    Ok(project)
}

impl BoardProject {
    pub fn net_for_pin(&self, component: &str, pin: &str) -> Option<&str> {
        self.board
            .components
            .get(component)?
            .pins
            .get(pin)
            .map(String::as_str)
    }
}
