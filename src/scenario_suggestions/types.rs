use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct ScenarioSuggestionReport {
    pub schema_version: String,
    pub project: String,
    pub suggestions: Vec<ScenarioSuggestion>,
}

#[derive(Debug, Serialize)]
pub struct ScenarioSuggestion {
    pub id: String,
    pub kind: String,
    pub confidence: String,
    pub runnable: bool,
    pub reason: String,
    pub scenario: SuggestedScenario,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_inputs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedScenario {
    pub name: String,
    #[serde(rename = "type")]
    pub scenario_type: String,
    pub checks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SuggestedTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<SuggestedTiming>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_boot_mode: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub straps: Vec<SuggestedStrap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootloader: Option<SuggestedBootloader>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<SuggestedEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditioning: Option<SuggestedConditioning>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub protection_clamps: Vec<SuggestedProtectionClamp>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub usb_connectors: Vec<SuggestedUsbConnector>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub usb_routes: Vec<SuggestedUsbRoute>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub usb_route_pairs: Vec<SuggestedUsbRoutePair>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub clocks: Vec<SuggestedClockSource>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reset_supervisors: Vec<SuggestedResetSupervisor>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub regulators: Vec<SuggestedRegulator>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pin_states: Vec<SuggestedPinState>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<SuggestedBackdrivePath>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedTarget {
    pub component: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_pin: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedTiming {
    pub power_valid_at_us: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_release_delay_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_release_at_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_sample_at_us: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedStrap {
    pub component: String,
    pub pin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedBootloader {
    pub component: String,
    pub interface: String,
    pub sync_byte: u8,
    pub expected_response: u8,
}

#[derive(Debug, Serialize)]
pub struct SuggestedEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_us: Option<f64>,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<SuggestedEndpoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<SuggestedEndpoint>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedEndpoint {
    pub component: String,
    pub pin: String,
}

#[derive(Debug, Serialize)]
pub struct SuggestedConditioning {
    pub component: String,
    pub channel: String,
    pub kind: String,
    pub side_a: SuggestedConditioningSide,
    pub side_b: SuggestedConditioningSide,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unpowered_isolation: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedConditioningSide {
    pub pin: String,
    pub net: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supply_pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supply_net: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedProtectionClamp {
    pub component: String,
    pub clamp: String,
    pub protected_pin: String,
    pub protected_net: String,
    pub reference_pin: String,
    pub reference_net: String,
    pub reference: String,
    #[serde(
        rename = "working_voltage_max_V",
        skip_serializing_if = "Option::is_none"
    )]
    pub working_voltage_max_v: Option<f64>,
    #[serde(rename = "line_capacitance_F", skip_serializing_if = "Option::is_none")]
    pub line_capacitance_f: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<SuggestedPlacement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_to_target_mm: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedUsbConnector {
    pub component: String,
    pub standard: String,
    pub vbus_pin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vbus_net: Option<String>,
    pub dp_pin: String,
    pub dp_net: String,
    pub dm_pin: String,
    pub dm_net: String,
    pub gnd_pin: String,
    pub gnd_net: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shield_pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shield_net: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<SuggestedPlacement>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedUsbRoute {
    pub signal: String,
    pub net: String,
    pub route_length_mm: f64,
    pub via_count: usize,
    #[serde(
        rename = "expected_data_line_width_mm",
        skip_serializing_if = "Option::is_none"
    )]
    pub expected_data_line_width_mm: Option<f64>,
    #[serde(
        rename = "measured_data_line_width_mm",
        skip_serializing_if = "Option::is_none"
    )]
    pub measured_data_line_width_mm: Option<f64>,
    #[serde(
        rename = "data_line_width_delta_mm",
        skip_serializing_if = "Option::is_none"
    )]
    pub data_line_width_delta_mm: Option<f64>,
    #[serde(
        rename = "expected_vbus_route_width_mm",
        skip_serializing_if = "Option::is_none"
    )]
    pub expected_vbus_route_width_mm: Option<f64>,
    #[serde(
        rename = "measured_vbus_route_width_min_mm",
        skip_serializing_if = "Option::is_none"
    )]
    pub measured_vbus_route_width_min_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protection_component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unreferenced_route_length_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unreferenced_segments: Option<Vec<SuggestedUsbUnreferencedSegment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_unreferenced_route_length_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_unreferenced_segments: Option<Vec<SuggestedUsbUnreferencedSegment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_zone_edge_clearance_min_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_zone_edge_clearance_segments: Option<Vec<SuggestedUsbFilledZoneClearanceSegment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ground_zone_contacts: Option<Vec<SuggestedUsbGroundZoneContact>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_ground_zone_contacts: Option<Vec<SuggestedUsbGroundZoneContact>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedUsbUnreferencedSegment {
    pub segment_index: usize,
    pub segment_length_mm: f64,
    pub midpoint_x_mm: f64,
    pub midpoint_y_mm: f64,
    pub layer: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedUsbFilledZoneClearanceSegment {
    pub segment_index: usize,
    pub segment_length_mm: f64,
    pub midpoint_x_mm: f64,
    pub midpoint_y_mm: f64,
    pub layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_zone_edge_clearance_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedUsbGroundZoneContact {
    pub net: String,
    pub layer: String,
    pub contact_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pad: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via_index: Option<usize>,
    pub x_mm: f64,
    pub y_mm: f64,
}

#[derive(Debug, Serialize)]
pub struct SuggestedUsbRoutePair {
    pub dp_net: String,
    pub dm_net: String,
    pub dp_route_length_mm: f64,
    pub dm_route_length_mm: f64,
    pub data_pair_length_mismatch_mm: f64,
    pub dp_via_count: usize,
    pub dm_via_count: usize,
    pub data_pair_via_count_delta: usize,
    #[serde(
        rename = "expected_data_pair_gap_mm",
        skip_serializing_if = "Option::is_none"
    )]
    pub expected_data_pair_gap_mm: Option<f64>,
    #[serde(
        rename = "measured_data_pair_gap_mm",
        skip_serializing_if = "Option::is_none"
    )]
    pub measured_data_pair_gap_mm: Option<f64>,
    #[serde(
        rename = "data_pair_gap_delta_mm",
        skip_serializing_if = "Option::is_none"
    )]
    pub data_pair_gap_delta_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedPlacement {
    pub x_mm: f64,
    pub y_mm: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedClockSource {
    pub component: String,
    pub name: String,
    pub input_pin: String,
    pub input_net: String,
    pub output_pin: String,
    pub output_net: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crystal_component: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedResetSupervisor {
    pub component: String,
    pub monitored_pin: String,
    pub monitored_net: String,
    pub reset_output_pin: String,
    pub reset_net: String,
    #[serde(rename = "threshold_min_V")]
    pub threshold_min_v: f64,
    #[serde(rename = "threshold_max_V")]
    pub threshold_max_v: f64,
}

#[derive(Debug, Serialize)]
pub struct SuggestedRegulator {
    pub component: String,
    pub input_pin: String,
    pub input_net: String,
    pub output_pin: String,
    pub output_net: String,
    #[serde(rename = "dropout_voltage_V", skip_serializing_if = "Option::is_none")]
    pub dropout_voltage_v: Option<f64>,
    #[serde(
        rename = "min_output_current_A",
        skip_serializing_if = "Option::is_none"
    )]
    pub min_output_current_a: Option<f64>,
    #[serde(
        rename = "max_output_current_A",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_output_current_a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_delay_us: Option<f64>,
    #[serde(
        rename = "input_capacitance_min_F",
        skip_serializing_if = "Option::is_none"
    )]
    pub input_capacitance_min_f: Option<f64>,
    #[serde(
        rename = "output_capacitance_min_F",
        skip_serializing_if = "Option::is_none"
    )]
    pub output_capacitance_min_f: Option<f64>,
    #[serde(
        rename = "input_support_capacitance_F",
        skip_serializing_if = "Option::is_none"
    )]
    pub input_support_capacitance_f: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_support_capacitors: Option<Vec<String>>,
    #[serde(
        rename = "output_support_capacitance_F",
        skip_serializing_if = "Option::is_none"
    )]
    pub output_support_capacitance_f: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_support_capacitors: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedPinState {
    pub component: String,
    pub pin: String,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestedBackdrivePath {
    pub driver: SuggestedEndpoint,
    pub victim: SuggestedEndpoint,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net: Option<String>,
    pub series_resistance_ohm: f64,
}
