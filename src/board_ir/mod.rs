use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod analog_assertion_types;
mod layout;
mod manufacturing;

pub use analog_assertion_types::*;
pub use layout::*;
pub use manufacturing::*;

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
    #[serde(default)]
    pub schematic: BoardSchematic,
    #[serde(default)]
    pub manufacturing: BoardManufacturing,
    #[serde(default)]
    pub runtime: BoardRuntime,
    #[serde(default)]
    pub layout: BoardLayout,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BoardSchematic {
    #[serde(default)]
    pub node_positions: BTreeMap<String, SchematicNodePosition>,
    #[serde(default)]
    pub node_styles: BTreeMap<String, SchematicNodeStyle>,
    #[serde(default)]
    pub component_symbols: BTreeMap<String, String>,
    #[serde(default)]
    pub component_labels: BTreeMap<String, SchematicComponentLabels>,
    #[serde(default)]
    pub wire_routes: BTreeMap<String, SchematicWireRoute>,
    #[serde(default)]
    pub net_labels: BTreeMap<String, SchematicNetLabel>,
    #[serde(default)]
    pub probe_elements: BTreeMap<String, SchematicProbeElement>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchematicNodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SchematicWireRoute {
    #[serde(default)]
    pub points: Vec<SchematicWirePoint>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchematicWirePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SchematicComponentLabels {
    #[serde(default)]
    pub reference: Option<SchematicLabelPosition>,
    #[serde(default)]
    pub value: Option<SchematicLabelPosition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchematicLabelPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchematicNetLabel {
    pub net: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub kind: SchematicNetLabelKind,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchematicNetLabelKind {
    #[default]
    Local,
    OffPage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchematicProbeElement {
    pub scenario: String,
    pub probe: String,
    pub target: SchematicProbeElementTarget,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchematicProbeElementTarget {
    pub kind: SchematicProbeElementTargetKind,
    pub id: String,
    #[serde(default)]
    pub attach: Option<SchematicProbeAttachmentKind>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchematicProbeElementTargetKind {
    Net,
    Component,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchematicProbeAttachmentKind {
    Node,
    Pin,
    Wire,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchematicNodeStyle {
    #[serde(default)]
    pub rotation_deg: Option<i32>,
    #[serde(default)]
    pub mirrored: Option<bool>,
    #[serde(default)]
    pub pin_side: Option<SchematicPinSide>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchematicPinSide {
    Auto,
    Left,
    Right,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BoardRuntime {
    #[serde(default)]
    pub gpio_backdrive: Vec<GpioBackdriveRuntimeEvidence>,
    #[serde(default)]
    pub reset_release: Vec<ResetReleaseRuntimeEvidence>,
    #[serde(default)]
    pub control_line_sequences: Vec<ControlLineSequenceRuntimeEvidence>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GpioBackdriveRuntimeEvidence {
    pub driver: Endpoint,
    pub victim: Endpoint,
    #[serde(default)]
    pub driver_state: Option<PinLogicState>,
    #[serde(default)]
    pub victim_mode: Option<PinMode>,
    #[serde(default)]
    pub series_resistance_ohm: Option<f64>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResetReleaseRuntimeEvidence {
    pub component: String,
    pub reset_pin: String,
    #[serde(default)]
    pub power_pin: Option<String>,
    #[serde(rename = "reset_release_at_us")]
    pub reset_release_at_us: f64,
    #[serde(default, rename = "reset_release_delay_us")]
    pub reset_release_delay_us: Option<f64>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlLineSequenceRuntimeEvidence {
    #[serde(default)]
    pub name: Option<String>,
    pub target: ScenarioTarget,
    pub required_boot_mode: String,
    pub timing: ScenarioTiming,
    #[serde(default)]
    pub control_effects: Vec<ControlEffect>,
    #[serde(default)]
    pub events: Vec<ScenarioEvent>,
    #[serde(default)]
    pub source: Option<String>,
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
    pub parameters: BTreeMap<String, serde_yaml_ng::Value>,
    #[serde(default)]
    pub spice: Option<ComponentSpiceSpec>,
    #[serde(default)]
    pub source: Option<ComponentSourceSpec>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ComponentSourceSpec {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub footprint: Option<String>,
    #[serde(default)]
    pub manufacturer_part: Option<String>,
    #[serde(default)]
    pub supplier_part: Option<String>,
    #[serde(default)]
    pub placement_footprint: Option<String>,
    #[serde(default)]
    pub placement_side: Option<PlacementSide>,
    #[serde(default)]
    pub placement_side_confidence: Option<String>,
    #[serde(default)]
    pub placement_rotation_deg: Option<f64>,
    #[serde(default)]
    pub placement_orientation_confidence: Option<String>,
    #[serde(default)]
    pub board_pin_electrical_types: BTreeMap<String, String>,
    #[serde(default)]
    pub instances: Vec<ComponentSourceInstanceSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentSourceInstanceSpec {
    pub project: String,
    pub path: String,
    pub reference: String,
    pub unit: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentSpiceSpec {
    pub primitive: SpicePrimitive,
    #[serde(default)]
    pub value_ohm: Option<f64>,
    #[serde(default)]
    pub value_f: Option<f64>,
    #[serde(default)]
    pub initial_v: Option<f64>,
    #[serde(default)]
    pub value_h: Option<f64>,
    #[serde(default)]
    pub dc_v: Option<f64>,
    #[serde(default)]
    pub dc_a: Option<f64>,
    #[serde(default)]
    pub pulse: Option<SpicePulseSpec>,
    #[serde(default)]
    pub current_pulse: Option<SpiceCurrentPulseSpec>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpicePrimitive {
    Resistor,
    Capacitor,
    Inductor,
    DcVoltageSource,
    PulseVoltageSource,
    DcCurrentSource,
    PulseCurrentSource,
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
pub struct SpiceCurrentPulseSpec {
    pub initial_a: f64,
    pub pulsed_a: f64,
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
    #[serde(default, rename = "supply_current_limit_A")]
    pub supply_current_limit_a: Option<f64>,
    #[serde(default)]
    pub power_valid_at_us: Option<f64>,
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
    pub firmware: Option<FirmwareScenario>,
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
    #[serde(default, rename = "reset_release_delay_us")]
    pub reset_release_delay_us: Option<f64>,
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
pub struct FirmwareScenario {
    pub backend: FirmwareBackend,
    pub image: String,
    #[serde(default)]
    pub machine: Option<String>,
    #[serde(default)]
    pub build: Option<FirmwareBuildSpec>,
    #[serde(default)]
    pub qemu: Option<QemuFirmwareOptions>,
    #[serde(default)]
    pub expected_pin_states: Vec<PinState>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FirmwareBuildSpec {
    pub command: Vec<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct QemuFirmwareOptions {
    #[serde(default)]
    pub executable: Option<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub pin_trace_prefix: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareBackend {
    Auto,
    Renode,
    Qemu,
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
    #[serde(default)]
    pub sweeps: Vec<AnalogParameterSweep>,
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
    #[serde(default)]
    #[serde(rename = "stop_time_us")]
    pub stop_time_us: f64,
    #[serde(default)]
    #[serde(rename = "max_step_us")]
    pub max_step_us: f64,
    #[serde(default)]
    pub start_frequency_hz: Option<f64>,
    #[serde(default)]
    pub stop_frequency_hz: Option<f64>,
    #[serde(default)]
    pub points_per_decade: Option<u32>,
    #[serde(default)]
    pub noise_output_node: Option<String>,
    #[serde(default)]
    pub noise_reference_node: Option<String>,
    #[serde(default)]
    pub noise_input_source: Option<String>,
    #[serde(default)]
    pub dc_sweep_source: Option<String>,
    #[serde(default)]
    pub dc_sweep_start: Option<f64>,
    #[serde(default)]
    pub dc_sweep_stop: Option<f64>,
    #[serde(default)]
    pub dc_sweep_step: Option<f64>,
    #[serde(default)]
    pub dc_sweep_assertions: Vec<AnalogDcSweepAssertion>,
    #[serde(default)]
    pub s_parameter_ports: Vec<AnalogSParameterPort>,
    #[serde(default)]
    pub s_parameter_assertions: Vec<AnalogSParameterAssertion>,
    #[serde(default)]
    pub s_parameter_network_assertions: Vec<AnalogSParameterNetworkAssertion>,
    #[serde(default)]
    pub s_parameter_noise_assertions: Vec<AnalogSParameterNoiseAssertion>,
    #[serde(default)]
    pub s_parameter_source_reflection: Option<AnalogSParameterReflectionCoefficient>,
    #[serde(default)]
    pub s_parameter_load_reflection: Option<AnalogSParameterReflectionCoefficient>,
    #[serde(default)]
    pub transfer_output_expression: Option<String>,
    #[serde(default)]
    pub transfer_input_source: Option<String>,
    #[serde(default)]
    pub transfer_function_assertions: Vec<AnalogTransferFunctionAssertion>,
    #[serde(default)]
    pub pole_zero_output_node: Option<String>,
    #[serde(default)]
    pub pole_zero_reference_node: Option<String>,
    #[serde(default)]
    pub pole_zero_input_source: Option<String>,
    #[serde(default)]
    pub pole_zero_mode: Option<String>,
    #[serde(default)]
    pub pole_zero_assertions: Vec<AnalogPoleZeroAssertion>,
    #[serde(default)]
    pub sensitivity_output_expression: Option<String>,
    #[serde(default)]
    pub sensitivity_mode: Option<String>,
    #[serde(default)]
    pub sensitivity_filters: Vec<String>,
    #[serde(default)]
    pub sensitivity_assertions: Vec<AnalogSensitivityAssertion>,
    #[serde(default)]
    pub fourier_fundamental_frequency_hz: Option<f64>,
    #[serde(default)]
    pub fourier_output_expression: Option<String>,
    #[serde(default)]
    pub fourier_harmonics: Option<u32>,
    #[serde(default)]
    pub fourier_assertions: Vec<AnalogFourierAssertion>,
    #[serde(default)]
    pub hb_fundamental_frequency_hz: Option<f64>,
    #[serde(default)]
    pub hb_output_expression: Option<String>,
    #[serde(default)]
    pub hb_harmonics: Option<u32>,
    #[serde(default)]
    pub hb_drive_sources: Vec<String>,
    #[serde(default)]
    pub hb_assertions: Vec<AnalogHarmonicBalanceAssertion>,
    #[serde(default)]
    pub pss_mode: Option<String>,
    #[serde(default)]
    pub pss_frequency_guess_hz: Option<f64>,
    #[serde(default)]
    pub pss_stabilization_time_us: Option<f64>,
    #[serde(default)]
    pub pss_periods: Option<u32>,
    #[serde(default)]
    pub pss_output_expression: Option<String>,
    #[serde(default)]
    pub pss_drive_sources: Vec<String>,
    #[serde(default)]
    pub pss_residual_tolerance: Option<f64>,
    #[serde(default)]
    pub pss_state_error_tolerance: Option<f64>,
    #[serde(default)]
    pub pss_max_iterations: Option<u32>,
    #[serde(default)]
    pub phase_noise_mode: Option<String>,
    #[serde(default)]
    pub phase_noise_carrier_frequency_hz: Option<f64>,
    #[serde(default)]
    pub phase_noise_offset_start_hz: Option<f64>,
    #[serde(default)]
    pub phase_noise_offset_stop_hz: Option<f64>,
    #[serde(default)]
    pub phase_noise_points_per_decade: Option<u32>,
    #[serde(default)]
    pub phase_noise_output_expression: Option<String>,
    #[serde(default)]
    pub phase_noise_drive_sources: Vec<String>,
    #[serde(default)]
    pub phase_noise_integration_start_hz: Option<f64>,
    #[serde(default)]
    pub phase_noise_integration_stop_hz: Option<f64>,
    #[serde(default)]
    pub pac_mode: Option<String>,
    #[serde(default)]
    pub pac_carrier_frequency_hz: Option<f64>,
    #[serde(default)]
    pub pac_start_frequency_hz: Option<f64>,
    #[serde(default)]
    pub pac_stop_frequency_hz: Option<f64>,
    #[serde(default)]
    pub pac_points_per_decade: Option<u32>,
    #[serde(default)]
    pub pac_output_expression: Option<String>,
    #[serde(default)]
    pub pac_input_source: Option<String>,
    #[serde(default)]
    pub pac_sidebands: Option<u32>,
    #[serde(default)]
    pub pac_drive_sources: Vec<String>,
    #[serde(default)]
    pub distortion_mode: Option<String>,
    #[serde(default)]
    pub distortion_start_frequency_hz: Option<f64>,
    #[serde(default)]
    pub distortion_stop_frequency_hz: Option<f64>,
    #[serde(default)]
    pub distortion_points_per_decade: Option<u32>,
    #[serde(default)]
    pub distortion_output_expression: Option<String>,
    #[serde(default)]
    pub distortion_f1_sources: Vec<String>,
    #[serde(default)]
    pub distortion_f2_sources: Vec<String>,
    #[serde(default)]
    pub distortion_f2_over_f1: Option<f64>,
    #[serde(default)]
    pub distortion_assertions: Vec<AnalogDistortionAssertion>,
    #[serde(default)]
    pub measure_mode: Option<String>,
    #[serde(default)]
    pub measure_statements: Vec<AnalogMeasureStatement>,
    #[serde(default)]
    pub measure_templates: Vec<AnalogMeasureTemplate>,
    #[serde(default)]
    pub measure_assertions: Vec<AnalogMeasureAssertion>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogMeasureStatement {
    pub name: String,
    pub statement: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogMeasureTemplate {
    pub name: String,
    pub operation: String,
    pub expression: String,
    #[serde(default)]
    pub trigger_expression: Option<String>,
    #[serde(default)]
    pub trigger_value: Option<f64>,
    #[serde(default)]
    pub target_value: Option<f64>,
    #[serde(default)]
    pub trigger_edge: Option<String>,
    #[serde(default)]
    pub target_edge: Option<String>,
    #[serde(default)]
    pub trigger_count: Option<u32>,
    #[serde(default)]
    pub target_count: Option<u32>,
    #[serde(default)]
    pub from_us: Option<f64>,
    #[serde(default)]
    pub to_us: Option<f64>,
    #[serde(default)]
    pub at_us: Option<f64>,
    #[serde(default)]
    pub from_hz: Option<f64>,
    #[serde(default)]
    pub to_hz: Option<f64>,
    #[serde(default)]
    pub at_hz: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogModelFile {
    pub path: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub artifact_format: Option<String>,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub source_sha256: Option<String>,
    #[serde(default)]
    pub compiler: Option<String>,
    #[serde(default)]
    pub compiler_version: Option<String>,
    #[serde(default)]
    pub compiler_command: Option<String>,
    #[serde(default)]
    pub plugin_load_command: Option<String>,
    #[serde(default)]
    pub xyce_version: Option<String>,
    #[serde(default)]
    pub xyce_adms_template_revision: Option<String>,
    #[serde(default)]
    pub xyce_configure_options: Vec<String>,
    #[serde(default)]
    pub conformance_artifact: Option<String>,
    #[serde(default)]
    pub conformance_sha256: Option<String>,
    #[serde(default)]
    pub model_package_name: Option<String>,
    #[serde(default)]
    pub model_package_version: Option<String>,
    #[serde(default)]
    pub model_package_artifact_id: Option<String>,
    #[serde(default)]
    pub model_package_lock_path: Option<String>,
    #[serde(default)]
    pub model_package_lock_sha256: Option<String>,
    #[serde(default)]
    pub model_package_registry_path: Option<String>,
    #[serde(default)]
    pub model_package_registry_sha256: Option<String>,
    #[serde(default)]
    pub model_package_registry_entry: Option<String>,
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
pub struct AnalogParameterSweep {
    pub name: String,
    #[serde(default)]
    pub parameters: Vec<AnalogSweepParameter>,
    #[serde(default)]
    pub component_values: Vec<AnalogSweepComponentValue>,
    #[serde(default)]
    pub model_sections: Vec<AnalogModelSectionSweep>,
    #[serde(default)]
    pub monte_carlo: Option<AnalogMonteCarloSweep>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogSweepParameter {
    pub name: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogSweepComponentValue {
    pub component: String,
    pub field: AnalogSweepComponentField,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AnalogSweepComponentField {
    ValueOhm,
    ValueF,
    ValueH,
    DcV,
    DcA,
}

impl AnalogSweepComponentField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ValueOhm => "value_ohm",
            Self::ValueF => "value_f",
            Self::ValueH => "value_h",
            Self::DcV => "dc_v",
            Self::DcA => "dc_a",
        }
    }

    pub fn requires_positive_value(self) -> bool {
        matches!(self, Self::ValueOhm | Self::ValueF | Self::ValueH)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogModelSectionSweep {
    pub path: String,
    pub sections: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogMonteCarloSweep {
    pub samples: usize,
    #[serde(default = "default_monte_carlo_seed")]
    pub seed: u64,
    pub component_values: Vec<AnalogMonteCarloComponentValue>,
    #[serde(default)]
    pub criteria: Option<AnalogMonteCarloCriteria>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AnalogMonteCarloCriteria {
    #[serde(default)]
    pub min_yield_percent: Option<f64>,
    #[serde(default)]
    pub min_p1_margin: Option<f64>,
    #[serde(default)]
    pub min_p5_margin: Option<f64>,
    #[serde(default)]
    pub min_p50_margin: Option<f64>,
    #[serde(default)]
    pub min_p95_margin: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogMonteCarloComponentValue {
    pub component: String,
    pub field: AnalogSweepComponentField,
    pub nominal: f64,
    pub tolerance_percent: f64,
    #[serde(default)]
    pub distribution: AnalogMonteCarloDistribution,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogMonteCarloDistribution {
    #[default]
    Uniform,
    Normal,
}

impl AnalogMonteCarloDistribution {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uniform => "uniform",
            Self::Normal => "normal",
        }
    }
}

fn default_monte_carlo_seed() -> u64 {
    1
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
    #[serde(default)]
    pub reference_probe: Option<String>,
    #[serde(default, rename = "at_us")]
    pub at_us: Option<f64>,
    #[serde(default, rename = "start_us")]
    pub start_us: Option<f64>,
    #[serde(default, rename = "end_us")]
    pub end_us: Option<f64>,
    #[serde(default, rename = "time_limit_us")]
    pub time_limit_us: Option<f64>,
    #[serde(default, rename = "at_hz")]
    pub at_hz: Option<f64>,
    #[serde(default, rename = "frequency_limit_hz")]
    pub frequency_limit_hz: Option<f64>,
    #[serde(default, rename = "duty_limit_percent")]
    pub duty_limit_percent: Option<f64>,
    #[serde(default, rename = "count_limit")]
    pub count_limit: Option<f64>,
    #[serde(default)]
    pub aggregation: AnalogAggregation,
    pub relation: AnalogRelation,
    #[serde(default)]
    pub threshold_v: Option<f64>,
    #[serde(default)]
    pub threshold_a: Option<f64>,
    #[serde(default)]
    pub threshold_w: Option<f64>,
    #[serde(default, rename = "threshold_vs")]
    pub threshold_vs: Option<f64>,
    #[serde(default, rename = "threshold_s")]
    pub threshold_s: Option<f64>,
    #[serde(default, rename = "threshold_c")]
    pub threshold_c: Option<f64>,
    #[serde(default, rename = "threshold_j")]
    pub threshold_j: Option<f64>,
    #[serde(default, rename = "threshold_db")]
    pub threshold_db: Option<f64>,
    #[serde(default, rename = "threshold_deg")]
    pub threshold_deg: Option<f64>,
    #[serde(default, rename = "threshold_v_per_sqrt_hz")]
    pub threshold_v_per_sqrt_hz: Option<f64>,
    #[serde(default)]
    pub reference_threshold_v: Option<f64>,
    #[serde(default)]
    pub reference_threshold_a: Option<f64>,
    #[serde(default)]
    pub reference_threshold_w: Option<f64>,
    #[serde(default)]
    pub target_v: Option<f64>,
    #[serde(default)]
    pub target_a: Option<f64>,
    #[serde(default)]
    pub target_w: Option<f64>,
    #[serde(default)]
    pub tolerance_v: Option<f64>,
    #[serde(default)]
    pub tolerance_a: Option<f64>,
    #[serde(default)]
    pub tolerance_w: Option<f64>,
    #[serde(default, rename = "overshoot_limit_percent")]
    pub overshoot_limit_percent: Option<f64>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogAggregation {
    #[default]
    Sample,
    OperatingPoint,
    Min,
    Max,
    Mean,
    Rms,
    Integral,
    Energy,
    SettlingTime,
    OvershootPercent,
    RisingPhaseDelay,
    FallingPhaseDelay,
    RisingSetupTime,
    RisingHoldTime,
    FallingSetupTime,
    FallingHoldTime,
    RisingCrossingTime,
    FallingCrossingTime,
    MinHighPulseWidth,
    MinLowPulseWidth,
    DutyCycle,
    CrossingCount,
    RisingCrossingCount,
    FallingCrossingCount,
    GainDbAtFrequency,
    PhaseDegAtFrequency,
    RisingGainCrossingFrequency,
    FallingGainCrossingFrequency,
    PhaseMarginDeg,
    GainMarginDb,
    GroupDelaySAtFrequency,
    OutputNoiseDensityAtFrequency,
    InputNoiseDensityAtFrequency,
    IntegratedOutputNoise,
    IntegratedInputNoise,
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
