use crate::board_ir::{BoardProject, NetKind};
use crate::reports::Finding;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentModel {
    pub component_id: String,
    pub version: String,
    pub category: String,
    #[serde(default)]
    pub ports: BTreeMap<String, Port>,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub behavior: Behavior,
    pub model_quality: ModelQuality,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Port {
    pub kind: PortKind,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub electrical: Electrical,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortKind {
    ElectricalPower,
    ElectricalGround,
    DigitalElectricalInput,
    DigitalElectricalOutput,
    DigitalElectricalIo,
    Passive,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Electrical {
    #[serde(default, rename = "vih_min_V")]
    pub vih_min_v: Option<f64>,
    #[serde(default, rename = "vil_max_V")]
    pub vil_max_v: Option<f64>,
    #[serde(default, rename = "injection_current_limit_A")]
    pub injection_current_limit_a: Option<f64>,
    #[serde(default, rename = "drive_high_voltage_V")]
    pub drive_high_voltage_v: Option<f64>,
    #[serde(default)]
    pub source_impedance_ohm: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelQuality {
    pub source: String,
    pub confidence: String,
    #[serde(default)]
    pub intended_use: Vec<String>,
    #[serde(default)]
    pub not_valid_for: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Behavior {
    #[serde(default)]
    pub reset: Option<ResetBehavior>,
    #[serde(default)]
    pub boot: Option<BootBehavior>,
    #[serde(default)]
    pub bootloader: Option<BootloaderBehavior>,
    #[serde(default)]
    pub protocols: BTreeMap<String, ProtocolBehavior>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResetBehavior {
    pub pin: String,
    pub active: String,
    #[serde(default)]
    pub min_assert_us: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootBehavior {
    #[serde(default, rename = "sample_time_after_reset_release_us")]
    pub sample_time_after_reset_release_us: Option<f64>,
    #[serde(default)]
    pub modes: BTreeMap<String, BootMode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootMode {
    #[serde(default)]
    pub straps: Vec<BootStrapRequirement>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootStrapRequirement {
    pub pin: String,
    pub required_state: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootloaderBehavior {
    #[serde(default)]
    pub interfaces: BTreeMap<String, BootloaderInterface>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootloaderInterface {
    pub rx_pin: String,
    #[serde(default)]
    pub tx_pin: Option<String>,
    pub sync_byte: u8,
    pub ack_byte: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProtocolBehavior {
    #[serde(default)]
    pub transport_interface: Option<String>,
    #[serde(default)]
    pub frame: ProtocolFrame,
    #[serde(default)]
    pub operations: BTreeMap<String, ProtocolOperation>,
    #[serde(default)]
    pub flows: BTreeMap<String, ProtocolFlow>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProtocolFrame {
    #[serde(default)]
    pub magic: Vec<u8>,
    #[serde(default)]
    pub version: Option<u64>,
    #[serde(default)]
    pub request_type: Option<u64>,
    #[serde(default)]
    pub response_type: Option<u64>,
    #[serde(default)]
    pub crc: Option<String>,
    #[serde(default)]
    pub max_payload_len: Option<u64>,
    #[serde(default)]
    pub ok_result: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProtocolOperation {
    pub opcode: u64,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub payload: Option<ProtocolPayload>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProtocolPayload {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub min_len: Option<u64>,
    #[serde(default)]
    pub max_len: Option<u64>,
    #[serde(default)]
    pub len: Option<u64>,
    #[serde(default)]
    pub overhead_len: Option<u64>,
    #[serde(default)]
    pub values: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProtocolFlow {
    #[serde(default)]
    pub phases: Vec<ProtocolFlowPhase>,
    #[serde(default)]
    pub final_state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProtocolFlowPhase {
    pub operation: String,
    #[serde(default)]
    pub repeat: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ComponentLibrary {
    models: BTreeMap<String, ComponentModel>,
}

#[derive(Debug)]
pub struct BoundBoard<'a> {
    pub project: &'a BoardProject,
    pub library: ComponentLibrary,
    pub findings: Vec<Finding>,
}

pub fn load_library(
    project_path: &Path,
    project: &BoardProject,
) -> (ComponentLibrary, Vec<Finding>) {
    let mut library = ComponentLibrary {
        models: BTreeMap::new(),
    };
    let mut findings = Vec::new();
    let base_dir = project_path.parent().unwrap_or_else(|| Path::new("."));
    let roots = if project.libraries.is_empty() {
        vec![PathBuf::from("libs/generic")]
    } else {
        project.libraries.iter().map(PathBuf::from).collect()
    };

    for root in roots {
        let root = if root.is_absolute() {
            root
        } else {
            base_dir.join(root)
        };
        if !root.exists() {
            findings.push(Finding::warning(
                "LIBRARY_NOT_FOUND",
                "binding",
                format!("Library path {} does not exist.", root.display()),
            ));
            continue;
        }
        for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
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
            match load_model(path) {
                Ok(model) => {
                    if library.models.contains_key(&model.component_id) {
                        findings.push(Finding::warning(
                            "DUPLICATE_MODEL_ID",
                            "binding",
                            format!("Duplicate component model {}.", model.component_id),
                        ));
                    } else {
                        library.models.insert(model.component_id.clone(), model);
                    }
                }
                Err(error) => findings.push(Finding::warning(
                    "MODEL_LOAD_FAILED",
                    "binding",
                    format!("Could not load {}: {error}", path.display()),
                )),
            }
        }
    }

    (library, findings)
}

pub fn bind_project<'a>(
    project: &'a BoardProject,
    library: ComponentLibrary,
    mut findings: Vec<Finding>,
) -> BoundBoard<'a> {
    for (component_id, component) in &project.board.components {
        let Some(model) = library.get(&component.model) else {
            findings.push(Finding::critical(
                "MODEL_NOT_FOUND",
                "binding",
                format!(
                    "Component {component_id} references unresolved model {}.",
                    component.model
                ),
            ));
            continue;
        };

        for (pin, net) in &component.pins {
            if !model.ports.contains_key(pin) {
                findings.push(Finding::warning(
                    "PIN_NOT_DECLARED",
                    "binding",
                    format!(
                        "Component {component_id}.{pin} is not declared by model {}.",
                        model.component_id
                    ),
                ));
            }
            if !project.board.nets.contains_key(net) {
                findings.push(Finding::critical(
                    "NET_NOT_FOUND",
                    "binding",
                    format!("Component {component_id}.{pin} references missing net {net}."),
                ));
            }
        }

        for (pin_name, port) in &model.ports {
            if port.required && !component.pins.contains_key(pin_name) {
                findings.push(Finding::critical(
                    "REQUIRED_PIN_FLOATING",
                    "binding",
                    format!("Required pin {component_id}.{pin_name} is not connected."),
                ));
            }
            if port.kind == PortKind::ElectricalPower {
                let rail = component
                    .power_domains
                    .get(pin_name)
                    .or_else(|| component.pins.get(pin_name))
                    .or(component.power_domain.as_ref());
                match rail.and_then(|net| project.board.nets.get(net).map(|spec| (net, spec))) {
                    Some((_, net)) if net.kind == NetKind::Power => {}
                    Some((net_name, _)) => findings.push(Finding::critical(
                        "INVALID_POWER_DOMAIN",
                        "binding",
                        format!("Power pin {component_id}.{pin_name} is connected to non-power net {net_name}."),
                    )),
                    None => findings.push(Finding::critical(
                        "POWER_DOMAIN_NOT_FOUND",
                        "binding",
                        format!("Power pin {component_id}.{pin_name} does not resolve to a declared power net."),
                    )),
                }
            }
        }
    }

    BoundBoard {
        project,
        library,
        findings,
    }
}

fn load_model(path: &Path) -> anyhow::Result<ComponentModel> {
    let text = fs::read_to_string(path)?;
    let model = serde_yaml_ng::from_str(&text)?;
    Ok(model)
}

impl ComponentLibrary {
    pub fn get(&self, component_id: &str) -> Option<&ComponentModel> {
        self.models.get(component_id)
    }
}
