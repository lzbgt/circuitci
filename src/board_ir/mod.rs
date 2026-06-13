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
    #[serde(default)]
    pub manufacturing: BoardManufacturing,
    #[serde(default)]
    pub layout: BoardLayout,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BoardManufacturing {
    #[serde(default)]
    pub stencil_thickness_mm: Option<f64>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BoardLayout {
    #[serde(default)]
    pub placements: BTreeMap<String, ComponentPlacement>,
    #[serde(default)]
    pub footprints: BTreeMap<String, LayoutFootprint>,
    #[serde(default)]
    pub outline: BoardOutline,
    #[serde(default)]
    pub drills: Vec<LayoutDrill>,
    #[serde(default)]
    pub slots: Vec<LayoutSlot>,
    #[serde(default)]
    pub copper: LayoutCopper,
    #[serde(default)]
    pub solder_mask: LayoutCopper,
    #[serde(default)]
    pub solder_paste: LayoutCopper,
    #[serde(default)]
    pub pads: BTreeMap<String, BTreeMap<String, LayoutPad>>,
    #[serde(default)]
    pub routes: BTreeMap<String, NetRoute>,
    #[serde(default)]
    pub zones: BTreeMap<String, Vec<CopperZone>>,
    #[serde(default)]
    pub constraints: LayoutConstraints,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayoutFootprint {
    #[serde(default)]
    pub segments: Vec<LayoutFootprintSegment>,
    #[serde(default)]
    pub rectangles: Vec<LayoutFootprintRectangle>,
    #[serde(default)]
    pub polygons: Vec<LayoutFootprintPolygon>,
    #[serde(default)]
    pub circles: Vec<LayoutFootprintCircle>,
    #[serde(default)]
    pub arcs: Vec<LayoutFootprintArc>,
    #[serde(default)]
    pub entry_direction: Option<LayoutEntryDirection>,
    #[serde(default)]
    pub entry_clearance: Option<LayoutEntryClearance>,
    #[serde(default)]
    pub entry_aperture: Option<LayoutEntryAperture>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayoutEntryDirection {
    #[serde(default)]
    pub offset_deg: Option<f64>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayoutEntryClearance {
    #[serde(default)]
    pub depth_mm: Option<f64>,
    #[serde(default)]
    pub width_mm: Option<f64>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayoutEntryAperture {
    #[serde(default)]
    pub front_offset_mm: Option<f64>,
    #[serde(default)]
    pub lateral_offset_mm: Option<f64>,
    #[serde(default)]
    pub width_mm: Option<f64>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutFootprintSegment {
    pub start: LayoutPoint,
    pub end: LayoutPoint,
    pub layer: String,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutFootprintRectangle {
    pub start: LayoutPoint,
    pub end: LayoutPoint,
    pub layer: String,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutFootprintPolygon {
    pub points: Vec<LayoutPoint>,
    pub layer: String,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutFootprintCircle {
    pub center: LayoutPoint,
    pub end: LayoutPoint,
    pub layer: String,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutFootprintArc {
    pub start: LayoutPoint,
    pub mid: LayoutPoint,
    pub end: LayoutPoint,
    pub layer: String,
    pub kind: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BoardOutline {
    #[serde(default)]
    pub segments: Vec<LayoutSegment>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayoutConstraints {
    #[serde(default)]
    pub net_rules: BTreeMap<String, NetLayoutRule>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NetLayoutRule {
    #[serde(default)]
    pub net_class: Option<String>,
    #[serde(default)]
    pub track_width_mm: Option<f64>,
    #[serde(default)]
    pub diff_pair_width_mm: Option<f64>,
    #[serde(default)]
    pub diff_pair_gap_mm: Option<f64>,
    #[serde(default)]
    pub length_max_mm: Option<f64>,
    #[serde(default)]
    pub skew_max_mm: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentPlacement {
    pub x_mm: f64,
    pub y_mm: f64,
    #[serde(default)]
    pub side: Option<PlacementSide>,
    #[serde(default)]
    pub rotation_deg: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlacementSide {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutPad {
    pub at: LayoutPoint,
    pub net: String,
    #[serde(default)]
    pub layers: Vec<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub shape: Option<String>,
    #[serde(default)]
    pub size: Option<LayoutPadSize>,
    #[serde(default)]
    pub rotation_deg: Option<f64>,
    #[serde(default)]
    pub drill_mm: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutDrill {
    pub at: LayoutPoint,
    pub drill_mm: f64,
    pub plating: String,
    #[serde(default)]
    pub castellated: bool,
    #[serde(default)]
    pub owner_kind: Option<String>,
    #[serde(default)]
    pub net: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub pin: Option<String>,
    #[serde(default)]
    pub via_index: Option<usize>,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub source_hit_index: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutSlot {
    pub start: LayoutPoint,
    pub end: LayoutPoint,
    pub width_mm: f64,
    pub plating: String,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub source_slot_index: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayoutCopper {
    #[serde(default)]
    pub features: Vec<LayoutCopperFeature>,
    #[serde(default)]
    pub segments: Vec<LayoutCopperSegment>,
    #[serde(default)]
    pub regions: Vec<LayoutCopperRegion>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutCopperFeature {
    pub at: LayoutPoint,
    pub layer: String,
    pub polarity: String,
    pub net: Option<String>,
    pub island_id: Option<String>,
    #[serde(default)]
    pub owner_kind: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub pin: Option<String>,
    #[serde(default)]
    pub via_index: Option<usize>,
    pub source_primitive: String,
    pub source_primitive_index: usize,
    pub aperture: String,
    pub shape: String,
    pub size: LayoutPadSize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutCopperSegment {
    pub start: LayoutPoint,
    pub end: LayoutPoint,
    pub layer: String,
    pub polarity: String,
    pub net: Option<String>,
    pub island_id: Option<String>,
    #[serde(default)]
    pub owner_kind: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub pin: Option<String>,
    #[serde(default)]
    pub via_index: Option<usize>,
    pub source_primitive: String,
    pub source_primitive_index: usize,
    pub aperture: String,
    pub width_mm: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutCopperRegion {
    pub points: Vec<LayoutPoint>,
    pub layer: String,
    pub polarity: String,
    pub net: Option<String>,
    pub island_id: Option<String>,
    #[serde(default)]
    pub owner_kind: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub pin: Option<String>,
    #[serde(default)]
    pub via_index: Option<usize>,
    pub source_primitive: String,
    pub source_primitive_index: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutPadSize {
    pub x_mm: f64,
    pub y_mm: f64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NetRoute {
    #[serde(default)]
    pub segments: Vec<RouteSegment>,
    #[serde(default)]
    pub vias: Vec<RouteVia>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteSegment {
    pub start: LayoutPoint,
    pub end: LayoutPoint,
    pub width_mm: f64,
    pub layer: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteVia {
    pub at: LayoutPoint,
    pub size_mm: f64,
    pub drill_mm: f64,
    #[serde(default)]
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CopperZone {
    pub layer: String,
    pub polygon: Vec<LayoutPoint>,
    pub island_id: Option<String>,
    #[serde(default)]
    pub filled_polygons: Vec<Vec<LayoutPoint>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutSegment {
    pub start: LayoutPoint,
    pub end: LayoutPoint,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub source_primitive: Option<String>,
    #[serde(default)]
    pub source_primitive_index: Option<usize>,
    #[serde(default)]
    pub sample_index: Option<usize>,
    #[serde(default)]
    pub sample_count: Option<usize>,
    #[serde(default)]
    pub contour_index: Option<usize>,
    #[serde(default)]
    pub boundary_role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutPoint {
    pub x_mm: f64,
    pub y_mm: f64,
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
    pub board_pin_electrical_types: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentSpiceSpec {
    pub primitive: SpicePrimitive,
    #[serde(default)]
    pub value_ohm: Option<f64>,
    #[serde(default)]
    pub value_f: Option<f64>,
    #[serde(default)]
    pub value_h: Option<f64>,
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
    Inductor,
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
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
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
