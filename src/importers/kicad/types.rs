use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub(super) struct ImportedComponentModel {
    pub(super) component_id: String,
    #[serde(default)]
    pub(super) ports: BTreeMap<String, serde_yaml_ng::Value>,
    #[serde(default)]
    pub(super) simulation: ImportedModelSimulation,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct ImportedModelSimulation {
    #[serde(default)]
    pub(super) spice: Option<ImportedSpiceModel>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ImportedSpiceModel {
    pub(super) model_path: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KicadMapping {
    #[serde(default)]
    pub(super) libraries: Vec<String>,
    #[serde(default)]
    pub(super) pin_aliases: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub(super) components: BTreeMap<String, ComponentMapping>,
    #[serde(default)]
    pub(super) libsource_rules: Vec<LibsourceRuleMapping>,
    #[serde(default)]
    pub(super) nets: BTreeMap<String, NetMapping>,
    #[serde(default)]
    pub(super) scenarios: Vec<serde_yaml_ng::Value>,
    #[serde(default)]
    pub(super) analog_scenarios: Vec<AnalogScenarioMapping>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ComponentMapping {
    pub(super) model: String,
    #[serde(default)]
    pub(super) pin_alias: Option<String>,
    #[serde(default)]
    pub(super) pin_map: BTreeMap<String, String>,
    #[serde(default)]
    pub(super) part_number: Option<String>,
    #[serde(default)]
    pub(super) spice: Option<ComponentSpiceYaml>,
    #[serde(default)]
    pub(super) layout: Option<ComponentLayoutMapping>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LibsourceRuleMapping {
    pub(super) lib: String,
    pub(super) part: String,
    #[serde(default)]
    pub(super) value: Option<String>,
    pub(super) model: String,
    #[serde(default)]
    pub(super) pin_alias: Option<String>,
    #[serde(default)]
    pub(super) pin_map: BTreeMap<String, String>,
    #[serde(default)]
    pub(super) spice: Option<ComponentSpiceYaml>,
    #[serde(default)]
    pub(super) layout: Option<ComponentLayoutMapping>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ComponentLayoutMapping {
    #[serde(default)]
    pub(super) entry_direction_offset_deg: Option<f64>,
    #[serde(default)]
    pub(super) entry_clearance_depth_mm: Option<f64>,
    #[serde(default)]
    pub(super) entry_clearance_width_mm: Option<f64>,
    #[serde(default)]
    pub(super) entry_aperture: Option<ComponentEntryApertureMapping>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ComponentEntryApertureMapping {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) front_offset_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) lateral_offset_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) width_mm: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NetMapping {
    #[serde(default)]
    pub(super) kind: Option<MappedNetKind>,
    #[serde(default)]
    pub(super) nominal_voltage: Option<f64>,
    #[serde(default)]
    pub(super) powered: Option<bool>,
    #[serde(default)]
    #[serde(rename = "supply_current_limit_A")]
    pub(super) supply_current_limit_a: Option<f64>,
    #[serde(default)]
    pub(super) power_valid_at_us: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ComponentSpiceYaml {
    pub(super) primitive: SpicePrimitiveYaml,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) value_ohm: Option<f64>,
    #[serde(default, skip_serializing)]
    pub(super) value_ohm_from: Option<SpiceValueSourceYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) value_f: Option<f64>,
    #[serde(default, skip_serializing)]
    pub(super) value_f_from: Option<SpiceValueSourceYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) value_h: Option<f64>,
    #[serde(default, skip_serializing)]
    pub(super) value_h_from: Option<SpiceValueSourceYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) dc_v: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) dc_a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) pulse: Option<PulseSpecYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) current_pulse: Option<CurrentPulseSpecYaml>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum SpicePrimitiveYaml {
    Resistor,
    Capacitor,
    Inductor,
    DcVoltageSource,
    PulseVoltageSource,
    DcCurrentSource,
    PulseCurrentSource,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum SpiceValueSourceYaml {
    SchematicValue,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PulseSpecYaml {
    pub(super) initial_v: f64,
    pub(super) pulsed_v: f64,
    pub(super) delay_us: f64,
    pub(super) rise_us: f64,
    pub(super) fall_us: f64,
    pub(super) width_us: f64,
    pub(super) period_us: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CurrentPulseSpecYaml {
    pub(super) initial_a: f64,
    pub(super) pulsed_a: f64,
    pub(super) delay_us: f64,
    pub(super) rise_us: f64,
    pub(super) fall_us: f64,
    pub(super) width_us: f64,
    pub(super) period_us: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AnalogScenarioMapping {
    pub(super) name: String,
    #[serde(default)]
    pub(super) backend: AnalogBackendYaml,
    pub(super) components: Vec<String>,
    pub(super) ground_net: String,
    #[serde(default)]
    pub(super) operating_conditions: OperatingConditionsYaml,
    #[serde(default)]
    pub(super) model_files: Vec<ModelFileYaml>,
    pub(super) analysis: AnalysisYaml,
    pub(super) stimuli: Vec<StimulusYaml>,
    pub(super) probes: Vec<ProbeYaml>,
    pub(super) assertions: Vec<AssertionYaml>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OperatingConditionsYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) ambient_temperature_c: Option<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(super) allow_pulse_ratings: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AnalogBackendYaml {
    #[default]
    Auto,
    Ngspice,
    Xyce,
    EmbeddedNgspice,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum MappedNetKind {
    Power,
    Ground,
    DigitalOrAnalog,
}

impl MappedNetKind {
    pub(super) fn as_board_ir(&self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Ground => "ground",
            Self::DigitalOrAnalog => "digital_or_analog",
        }
    }
}

#[derive(Debug, Serialize)]
pub(super) struct ProjectYaml {
    pub(super) project: ProjectMetaYaml,
    pub(super) libraries: Vec<String>,
    pub(super) board: BoardYaml,
    pub(super) scenarios: Vec<serde_yaml_ng::Value>,
}

#[derive(Debug, Serialize)]
pub(super) struct ProjectMetaYaml {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) import_source: String,
}

#[derive(Debug, Serialize)]
pub(super) struct BoardYaml {
    pub(super) components: BTreeMap<String, ComponentYaml>,
    pub(super) nets: BTreeMap<String, NetYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) layout: Option<BoardLayoutYaml>,
}

#[derive(Debug, Serialize)]
pub(super) struct BoardLayoutYaml {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) footprints: BTreeMap<String, LayoutFootprintYaml>,
}

#[derive(Debug, Serialize)]
pub(super) struct LayoutFootprintYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) entry_direction: Option<LayoutEntryDirectionYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) entry_clearance: Option<LayoutEntryClearanceYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) entry_aperture: Option<LayoutEntryApertureYaml>,
}

#[derive(Debug, Serialize)]
pub(super) struct LayoutEntryDirectionYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) offset_deg: Option<f64>,
    pub(super) source: String,
}

#[derive(Debug, Serialize)]
pub(super) struct LayoutEntryClearanceYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) depth_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) width_mm: Option<f64>,
    pub(super) source: String,
}

#[derive(Debug, Serialize)]
pub(super) struct LayoutEntryApertureYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) front_offset_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) lateral_offset_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) width_mm: Option<f64>,
    pub(super) source: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ComponentYaml {
    pub(super) model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) part_number: Option<String>,
    pub(super) pins: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) spice: Option<ComponentSpiceYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) source: Option<ComponentSourceYaml>,
}

#[derive(Debug, Serialize)]
pub(super) struct ComponentSourceYaml {
    pub(super) format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) lib: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) part: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) kicad_pin_electrical_types: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) board_pin_electrical_types: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) in_bom: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) unit: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) units: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) instances: Vec<ComponentSourceInstanceYaml>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) fields: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ComponentSourceInstanceYaml {
    pub(super) project: String,
    pub(super) path: String,
    pub(super) reference: String,
    pub(super) unit: u32,
}

#[derive(Debug, Serialize)]
pub(super) struct NetYaml {
    pub(super) kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) nominal_voltage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) powered: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "supply_current_limit_A")]
    pub(super) supply_current_limit_a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) power_valid_at_us: Option<f64>,
}

#[derive(Debug, Serialize)]
pub(super) struct ScenarioYaml {
    pub(super) name: String,
    #[serde(rename = "type")]
    pub(super) scenario_type: String,
    pub(super) checks: Vec<String>,
    pub(super) analog: AnalogYaml,
}

#[derive(Debug, Serialize)]
pub(super) struct AnalogYaml {
    pub(super) backend: AnalogBackendYaml,
    pub(super) netlist_source: String,
    pub(super) generated: GeneratedNetlistYaml,
    #[serde(skip_serializing_if = "OperatingConditionsYaml::is_default")]
    pub(super) operating_conditions: OperatingConditionsYaml,
    pub(super) model_files: Vec<ModelFileYaml>,
    pub(super) node_bindings: Vec<NodeBindingYaml>,
    pub(super) pin_bindings: Vec<PinBindingYaml>,
    pub(super) analysis: AnalysisYaml,
    pub(super) stimuli: Vec<StimulusYaml>,
    pub(super) probes: Vec<ProbeYaml>,
    pub(super) assertions: Vec<AssertionYaml>,
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
pub(super) struct GeneratedNetlistYaml {
    pub(super) components: Vec<String>,
    pub(super) ground_net: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ModelFileYaml {
    pub(super) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) sha256: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct NodeBindingYaml {
    pub(super) node: String,
    pub(super) net: String,
}

#[derive(Debug, Serialize)]
pub(super) struct PinBindingYaml {
    pub(super) node: String,
    pub(super) endpoint: EndpointYaml,
}

#[derive(Debug, Serialize)]
pub(super) struct EndpointYaml {
    pub(super) component: String,
    pub(super) pin: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AnalysisYaml {
    #[serde(rename = "type")]
    pub(super) analysis_type: AnalysisTypeYaml,
    pub(super) stop_time_us: f64,
    pub(super) max_step_us: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AnalysisTypeYaml {
    Tran,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StimulusYaml {
    pub(super) name: String,
    pub(super) description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProbeYaml {
    pub(super) name: String,
    pub(super) expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) quantity: Option<ProbeQuantityYaml>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ProbeQuantityYaml {
    Voltage,
    Current,
    Power,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AssertionYaml {
    pub(super) name: String,
    pub(super) probe: String,
    pub(super) relation: AssertionRelationYaml,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) at_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) start_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) end_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) time_limit_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) aggregation: Option<AssertionAggregationYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) threshold_v: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) threshold_a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) threshold_w: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AssertionRelationYaml {
    Below,
    Above,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AssertionAggregationYaml {
    Sample,
    Min,
    Max,
    Mean,
    Rms,
    RisingCrossingTime,
    FallingCrossingTime,
}
