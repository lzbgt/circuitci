use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct BoardProject {
    pub project: ProjectMetadata,
    #[serde(default)]
    pub libraries: Vec<String>,
    pub board: Board,
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
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
    pub timing: Option<ScenarioTiming>,
    #[serde(default)]
    pub straps: Vec<BootStrapObservation>,
    #[serde(default)]
    pub required_boot_mode: Option<String>,
    #[serde(default)]
    pub bootloader: Option<BootloaderScenario>,
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
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct Endpoint {
    pub component: String,
    pub pin: String,
}

pub fn load_project(path: &Path) -> anyhow::Result<BoardProject> {
    let text = fs::read_to_string(path)?;
    let project = serde_yaml_ng::from_str(&text)?;
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
