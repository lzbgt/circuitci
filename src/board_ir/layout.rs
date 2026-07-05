use serde::Deserialize;
use std::collections::BTreeMap;

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
    #[serde(default)]
    pub matching_networks: Vec<RfAntennaMatchingNetworkRule>,
    #[serde(default)]
    pub measurements: Vec<RfAntennaMeasurement>,
    #[serde(default)]
    pub performance_limits: Vec<RfAntennaPerformanceLimit>,
    #[serde(default)]
    pub measurement_conditions: Vec<RfAntennaMeasurementCondition>,
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

#[derive(Debug, Clone, Deserialize)]
pub struct RfAntennaMeasurement {
    pub name: String,
    pub antenna_net: String,
    pub frequency_mhz: f64,
    pub return_loss_db: f64,
    pub source: String,
    #[serde(default)]
    pub measurement_method: Option<String>,
    #[serde(default)]
    pub measurement_condition: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RfAntennaPerformanceLimit {
    pub name: String,
    pub antenna_net: String,
    pub min_return_loss_db: f64,
    pub source: String,
    #[serde(default)]
    pub frequency_min_mhz: Option<f64>,
    #[serde(default)]
    pub frequency_max_mhz: Option<f64>,
    #[serde(default)]
    pub min_measurement_count: Option<usize>,
    #[serde(default)]
    pub max_frequency_step_mhz: Option<f64>,
    #[serde(default)]
    pub required_measurement_condition: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RfAntennaMeasurementCondition {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub fixture: Option<String>,
    #[serde(default)]
    pub cable_setup: Option<String>,
    #[serde(default)]
    pub enclosure_profile: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RfAntennaMatchingNetworkRule {
    pub name: String,
    pub antenna_net: String,
    pub topology: String,
    pub source: String,
    #[serde(default)]
    pub reference_net: Option<String>,
    #[serde(default)]
    pub elements: Vec<RfAntennaMatchingElement>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RfAntennaMatchingElement {
    pub component: String,
    pub role: String,
    #[serde(default)]
    pub input_net: Option<String>,
    #[serde(default)]
    pub output_net: Option<String>,
    #[serde(default)]
    pub signal_net: Option<String>,
    #[serde(default)]
    pub reference_net: Option<String>,
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
    pub plating_thickness_um: Option<f64>,
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
