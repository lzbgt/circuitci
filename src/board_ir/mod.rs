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
pub struct BoardManufacturing {
    #[serde(default)]
    pub stencil_thickness_mm: Option<f64>,
    #[serde(default)]
    pub min_drill_edge_clearance_mm: Option<f64>,
    #[serde(default)]
    pub min_slot_edge_clearance_mm: Option<f64>,
    #[serde(default)]
    pub min_paste_area_ratio: Option<f64>,
    #[serde(default)]
    pub max_paste_area_ratio: Option<f64>,
    #[serde(default)]
    pub min_solder_paste_spacing_mm: Option<f64>,
    #[serde(default)]
    pub max_stitch_via_distance_mm: Option<f64>,
    #[serde(default)]
    pub controlled_impedance: ControlledImpedanceTargets,
    #[serde(default)]
    pub thermal_copper: Vec<ThermalCopperRule>,
    #[serde(default)]
    pub thermal_measurements: Vec<ThermalMeasurement>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ControlledImpedanceTargets {
    #[serde(default)]
    pub nets: Vec<ControlledImpedanceNetTarget>,
    #[serde(default)]
    pub differential_pairs: Vec<ControlledImpedanceDifferentialPairTarget>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceNetTarget {
    pub net: String,
    pub source: String,
    pub target_impedance_ohm: f64,
    pub expected_width_mm: f64,
    pub max_width_error_mm: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceDifferentialPairTarget {
    pub first_net: String,
    pub second_net: String,
    pub source: String,
    pub target_differential_impedance_ohm: f64,
    pub expected_width_mm: f64,
    pub expected_gap_mm: f64,
    pub max_width_error_mm: f64,
    pub max_gap_error_mm: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThermalCopperRule {
    pub name: String,
    pub component: String,
    pub source: String,
    pub power_loss_w: f64,
    pub min_copper_area_mm2: f64,
    #[serde(default)]
    pub min_thermal_via_count: Option<usize>,
    #[serde(default)]
    pub min_copper_thickness_um: Option<f64>,
    #[serde(default)]
    pub nets: Vec<String>,
    #[serde(default)]
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThermalMeasurement {
    pub name: String,
    pub component: String,
    pub source: String,
    #[serde(rename = "measured_temperature_C")]
    pub measured_temperature_c: f64,
    #[serde(default, rename = "ambient_temperature_C")]
    pub ambient_temperature_c: Option<f64>,
    #[serde(default)]
    pub power_loss_w: Option<f64>,
    #[serde(default)]
    pub measurement_point: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BoardLayout {
    #[serde(default)]
    pub stackup: BoardStackup,
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
pub struct BoardStackup {
    #[serde(default)]
    pub layers: Vec<StackupLayer>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StackupLayer {
    pub name: String,
    pub kind: StackupLayerKind,
    #[serde(default)]
    pub reference_net: Option<String>,
    #[serde(default)]
    pub thickness_mm: Option<f64>,
    #[serde(default)]
    pub copper_thickness_um: Option<f64>,
    #[serde(default)]
    pub dielectric_constant: Option<f64>,
    #[serde(default)]
    pub material: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StackupLayerKind {
    Signal,
    Plane,
    Dielectric,
    #[default]
    Other,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayoutFootprint {
    #[serde(default)]
    pub properties: Vec<LayoutFootprintProperty>,
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
    pub semantics: Option<LayoutFootprintSemantics>,
    #[serde(default)]
    pub entry_direction: Option<LayoutEntryDirection>,
    #[serde(default)]
    pub entry_clearance: Option<LayoutEntryClearance>,
    #[serde(default)]
    pub entry_aperture: Option<LayoutEntryAperture>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutFootprintProperty {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayoutFootprintSemantics {
    #[serde(default)]
    pub body_bounds: Option<LayoutFootprintBounds>,
    #[serde(default)]
    pub courtyard_bounds: Option<LayoutFootprintBounds>,
    #[serde(default)]
    pub pin_1: Option<LayoutPinMarker>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutFootprintBounds {
    pub min: LayoutPoint,
    pub max: LayoutPoint,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutPinMarker {
    pub at: LayoutPoint,
    #[serde(default)]
    pub source: Option<String>,
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
    #[serde(default)]
    pub rf_antenna: RfAntennaLayoutRules,
    #[serde(default)]
    pub usb_connector: UsbConnectorLayoutRule,
    #[serde(default)]
    pub usb_route: UsbRouteLayoutRule,
    #[serde(default)]
    pub usb_vbus_route: UsbVbusRouteLayoutRule,
    #[serde(default)]
    pub usb_return_path: UsbReturnPathLayoutRule,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RfAntennaLayoutRules {
    #[serde(default)]
    pub keepouts: Vec<RfAntennaKeepoutRule>,
    #[serde(default)]
    pub feed_paths: Vec<RfAntennaFeedPathRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RfAntennaKeepoutRule {
    pub name: String,
    pub layer: String,
    pub polygon: Vec<LayoutPoint>,
    pub min_copper_clearance_mm: f64,
    pub source: String,
    #[serde(default)]
    pub antenna_net: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RfAntennaFeedPathRule {
    pub name: String,
    pub antenna_net: String,
    pub feed_component: String,
    pub feed_pin: String,
    #[serde(default)]
    pub matching_components: Vec<String>,
    pub max_feed_route_length_mm: f64,
    pub max_matching_component_distance_mm: f64,
    pub source: String,
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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsbRouteLayoutRule {
    #[serde(default)]
    pub max_data_line_via_count: Option<usize>,
    #[serde(default)]
    pub max_data_line_width_delta_mm: Option<f64>,
    #[serde(default)]
    pub max_connector_to_protection_route_distance_mm: Option<f64>,
    #[serde(default)]
    pub max_component_to_route_distance_mm: Option<f64>,
    #[serde(default)]
    pub max_data_pair_via_count_delta: Option<usize>,
    #[serde(default)]
    pub max_data_pair_gap_delta_mm: Option<f64>,
    #[serde(default)]
    pub require_route_pad_contact_evidence: Option<bool>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsbVbusRouteLayoutRule {
    #[serde(default)]
    pub max_vbus_via_count: Option<usize>,
    #[serde(default)]
    pub min_vbus_route_width_mm: Option<f64>,
    #[serde(default)]
    pub max_connector_to_vbus_protection_route_distance_mm: Option<f64>,
    #[serde(default)]
    pub max_component_to_route_distance_mm: Option<f64>,
    #[serde(default)]
    pub require_vbus_route_pad_contact_evidence: Option<bool>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsbReturnPathLayoutRule {
    #[serde(default)]
    pub max_data_line_unreferenced_length_mm: Option<f64>,
    #[serde(default)]
    pub max_data_via_to_ground_stitch_distance_mm: Option<f64>,
    #[serde(default)]
    pub require_filled_zone_coverage: Option<bool>,
    #[serde(default)]
    pub min_data_line_filled_zone_edge_clearance_mm: Option<f64>,
    #[serde(default)]
    pub require_ground_zone_contact_evidence: Option<bool>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsbConnectorLayoutRule {
    #[serde(default)]
    pub max_connector_to_protection_distance_mm: Option<f64>,
    #[serde(default)]
    pub max_connector_rotation_error_deg: Option<f64>,
    #[serde(default)]
    pub max_connector_to_board_edge_distance_mm: Option<f64>,
    #[serde(default)]
    pub max_connector_body_overhang_mm: Option<f64>,
    #[serde(default)]
    pub min_connector_to_component_clearance_mm: Option<f64>,
    #[serde(default)]
    pub source: Option<String>,
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
    pub source: Option<LayoutPadSource>,
    #[serde(default)]
    pub layers: Vec<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub shape: Option<String>,
    #[serde(default)]
    pub size: Option<LayoutPadSize>,
    #[serde(default)]
    pub fabrication: Option<LayoutPadFabrication>,
    #[serde(default)]
    pub rotation_deg: Option<f64>,
    #[serde(default)]
    pub drill_mm: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutPadSource {
    pub format: String,
    #[serde(default)]
    pub row_index: Option<usize>,
    #[serde(default)]
    pub pin_name: Option<String>,
    #[serde(default)]
    pub pin_no: Option<String>,
    #[serde(default)]
    pub net_type: Option<String>,
    #[serde(default)]
    pub hole_len_mm: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayoutPadFabrication {
    #[serde(default)]
    pub solder_mask_margin_mm: Option<f64>,
    #[serde(default)]
    pub solder_paste_margin_mm: Option<f64>,
    #[serde(default)]
    pub solder_paste_margin_ratio: Option<f64>,
    #[serde(default)]
    pub clearance_mm: Option<f64>,
    #[serde(default)]
    pub zone_connect: Option<u8>,
    #[serde(default)]
    pub thermal_bridge_width_mm: Option<f64>,
    #[serde(default)]
    pub thermal_gap_mm: Option<f64>,
    #[serde(default)]
    pub source: Option<String>,
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
