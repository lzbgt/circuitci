use serde::Deserialize;

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
    pub thermal_packages: Vec<ThermalPackageRule>,
    #[serde(default)]
    pub thermal_environments: Vec<ThermalEnvironment>,
    #[serde(default)]
    pub thermal_limits: Vec<ThermalLimit>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ControlledImpedanceTargets {
    #[serde(default)]
    pub nets: Vec<ControlledImpedanceNetTarget>,
    #[serde(default)]
    pub differential_pairs: Vec<ControlledImpedanceDifferentialPairTarget>,
    #[serde(default)]
    pub coupons: Vec<ControlledImpedanceCoupon>,
    #[serde(default)]
    pub solver_qualifications: Vec<ControlledImpedanceSolverQualification>,
    #[serde(default)]
    pub solver_material_libraries: Vec<ControlledImpedanceSolverMaterialLibrary>,
    #[serde(default)]
    pub solver_material_acceptances: Vec<ControlledImpedanceSolverMaterialAcceptance>,
    #[serde(default)]
    pub solver_material_processes: Vec<ControlledImpedanceSolverMaterialProcess>,
    #[serde(default)]
    pub solver_results: Vec<ControlledImpedanceSolverResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceNetTarget {
    pub net: String,
    pub source: String,
    pub target_impedance_ohm: f64,
    pub expected_width_mm: f64,
    pub max_width_error_mm: f64,
    #[serde(default)]
    pub solder_mask_state: Option<String>,
    #[serde(default)]
    pub solder_mask_layer: Option<String>,
    #[serde(default)]
    pub solder_mask_source: Option<String>,
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
    #[serde(default)]
    pub solder_mask_state: Option<String>,
    #[serde(default)]
    pub solder_mask_layer: Option<String>,
    #[serde(default)]
    pub solder_mask_source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceCoupon {
    pub name: String,
    pub source: String,
    pub coupon_type: ControlledImpedanceCouponType,
    #[serde(default)]
    pub net: Option<String>,
    #[serde(default)]
    pub first_net: Option<String>,
    #[serde(default)]
    pub second_net: Option<String>,
    pub target_impedance_ohm: f64,
    pub measured_impedance_ohm: f64,
    pub max_impedance_error_ohm: f64,
    #[serde(default)]
    pub process_lot: Option<String>,
    #[serde(default)]
    pub panel_id: Option<String>,
    #[serde(default)]
    pub stackup_revision: Option<String>,
    #[serde(default)]
    pub coupon_trace_layer: Option<String>,
    #[serde(default)]
    pub coupon_trace_width_mm: Option<f64>,
    #[serde(default)]
    pub max_trace_width_delta_mm: Option<f64>,
    #[serde(default)]
    pub coupon_trace_gap_mm: Option<f64>,
    #[serde(default)]
    pub max_trace_gap_delta_mm: Option<f64>,
    #[serde(default)]
    pub min_batch_sample_count: Option<usize>,
    #[serde(default)]
    pub max_batch_mean_impedance_error_ohm: Option<f64>,
    #[serde(default)]
    pub max_batch_sample_impedance_error_ohm: Option<f64>,
    #[serde(default)]
    pub max_batch_stddev_ohm: Option<f64>,
    #[serde(default)]
    pub samples: Vec<ControlledImpedanceCouponSample>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceCouponSample {
    pub name: String,
    pub source: String,
    pub measured_impedance_ohm: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceSolverQualification {
    pub name: String,
    pub source: String,
    pub solver: String,
    pub solver_version: String,
    pub qualification_artifact_uri: String,
    pub qualification_artifact_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceSolverMaterialLibrary {
    pub name: String,
    pub source: String,
    pub material_library: String,
    pub material_library_revision: String,
    pub artifact_uri: String,
    pub artifact_sha256: String,
    #[serde(default)]
    pub corners: Vec<String>,
    #[serde(default)]
    pub dielectric_layers: Vec<String>,
    #[serde(default)]
    pub materials: Vec<String>,
    #[serde(default)]
    pub content_fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceSolverMaterialAcceptance {
    pub name: String,
    pub source: String,
    pub material_library: String,
    pub material_library_revision: String,
    pub fabricator_stackup_revision: String,
    pub acceptance_artifact_uri: String,
    pub acceptance_artifact_sha256: String,
    #[serde(default)]
    pub accepted_by: Option<String>,
    #[serde(default)]
    pub accepted_corners: Vec<String>,
    #[serde(default)]
    pub accepted_dielectric_layers: Vec<String>,
    #[serde(default)]
    pub accepted_materials: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceSolverMaterialProcess {
    pub name: String,
    pub source: String,
    pub material_library: String,
    pub material_library_revision: String,
    pub fabricator_stackup_revision: String,
    pub dielectric_layer: String,
    pub material: String,
    pub process_lot: String,
    pub material_lot: String,
    pub process_revision: String,
    pub drift_artifact_uri: String,
    pub drift_artifact_sha256: String,
    pub accepted_dielectric_constant: f64,
    pub measured_dielectric_constant: f64,
    pub max_dielectric_constant_delta: f64,
    pub accepted_thickness_mm: f64,
    pub measured_thickness_mm: f64,
    pub max_thickness_delta_mm: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceSolverResult {
    pub name: String,
    pub source: String,
    pub solver: String,
    #[serde(default)]
    pub solver_version: Option<String>,
    #[serde(default)]
    pub solver_artifact_uri: Option<String>,
    #[serde(default)]
    pub solver_artifact_sha256: Option<String>,
    #[serde(default)]
    pub solver_artifact_signature_uri: Option<String>,
    #[serde(default)]
    pub solver_artifact_signature_sha256: Option<String>,
    #[serde(default)]
    pub solver_artifact_signer: Option<String>,
    #[serde(default)]
    pub solver_output_schema: Option<String>,
    #[serde(default)]
    pub solver_output_schema_version: Option<String>,
    #[serde(default)]
    pub solver_output_schema_uri: Option<String>,
    #[serde(default)]
    pub solver_output_schema_sha256: Option<String>,
    #[serde(default)]
    pub solver_config_lock_uri: Option<String>,
    #[serde(default)]
    pub solver_config_lock_sha256: Option<String>,
    #[serde(default)]
    pub solver_config_lock_tool: Option<String>,
    #[serde(default)]
    pub solver_config_lock_revision: Option<String>,
    #[serde(default)]
    pub solver_input_deck_uri: Option<String>,
    #[serde(default)]
    pub solver_input_deck_sha256: Option<String>,
    pub result_type: ControlledImpedanceSolverResultType,
    #[serde(default)]
    pub net: Option<String>,
    #[serde(default)]
    pub first_net: Option<String>,
    #[serde(default)]
    pub second_net: Option<String>,
    pub target_impedance_ohm: f64,
    pub solved_impedance_ohm: f64,
    pub max_impedance_error_ohm: f64,
    pub stackup_revision: String,
    pub route_layer: String,
    pub reference_layer: String,
    pub dielectric_layer: String,
    pub solved_width_mm: f64,
    pub max_route_width_delta_mm: f64,
    #[serde(default)]
    pub input_stackup_revision: Option<String>,
    #[serde(default)]
    pub input_route_layer: Option<String>,
    #[serde(default)]
    pub input_reference_layer: Option<String>,
    #[serde(default)]
    pub input_dielectric_layer: Option<String>,
    #[serde(default)]
    pub input_width_mm: Option<f64>,
    #[serde(default)]
    pub solved_gap_mm: Option<f64>,
    #[serde(default)]
    pub max_route_gap_delta_mm: Option<f64>,
    #[serde(default)]
    pub input_gap_mm: Option<f64>,
    #[serde(default)]
    pub frequency_mhz: Option<f64>,
    #[serde(default)]
    pub input_frequency_mhz: Option<f64>,
    #[serde(default)]
    pub copper_roughness_model: Option<String>,
    #[serde(default)]
    pub copper_roughness_um: Option<f64>,
    #[serde(default)]
    pub input_copper_roughness_model: Option<String>,
    #[serde(default)]
    pub input_copper_roughness_um: Option<f64>,
    #[serde(default)]
    pub etch_compensation_model: Option<String>,
    #[serde(default)]
    pub etch_compensation_um: Option<f64>,
    #[serde(default)]
    pub input_etch_compensation_model: Option<String>,
    #[serde(default)]
    pub input_etch_compensation_um: Option<f64>,
    #[serde(default)]
    pub solver_material_library: Option<String>,
    #[serde(default)]
    pub solver_material_library_revision: Option<String>,
    #[serde(default)]
    pub solver_material_library_artifact_uri: Option<String>,
    #[serde(default)]
    pub solver_material_library_artifact_sha256: Option<String>,
    #[serde(default)]
    pub input_material_library: Option<String>,
    #[serde(default)]
    pub input_material_library_revision: Option<String>,
    #[serde(default)]
    pub stackup_signoff_source: Option<String>,
    #[serde(default)]
    pub fabricator_stackup_revision: Option<String>,
    #[serde(default)]
    pub stackup_signoff_artifact_uri: Option<String>,
    #[serde(default)]
    pub stackup_signoff_artifact_sha256: Option<String>,
    #[serde(default)]
    pub min_solver_sample_count: Option<usize>,
    #[serde(default)]
    pub max_solver_frequency_step_mhz: Option<f64>,
    #[serde(default)]
    pub required_solver_corners: Vec<String>,
    #[serde(default)]
    pub samples: Vec<ControlledImpedanceSolverSample>,
    #[serde(default)]
    pub material_corners: Vec<ControlledImpedanceSolverMaterialCorner>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceSolverSample {
    pub name: String,
    pub source: String,
    pub corner: String,
    pub frequency_mhz: f64,
    pub solved_impedance_ohm: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlledImpedanceSolverMaterialCorner {
    pub name: String,
    pub source: String,
    pub corner: String,
    pub dielectric_layer: String,
    pub material: String,
    pub dielectric_constant: f64,
    pub nominal_dielectric_constant: f64,
    pub material_library: String,
    pub material_library_revision: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ControlledImpedanceSolverResultType {
    SingleEnded,
    Differential,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ControlledImpedanceCouponType {
    SingleEnded,
    Differential,
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
    pub min_plated_thermal_via_count: Option<usize>,
    #[serde(default)]
    pub min_thermal_via_drill_mm: Option<f64>,
    #[serde(default)]
    pub min_thermal_via_plating_thickness_um: Option<f64>,
    #[serde(default)]
    pub min_total_thermal_via_barrel_cross_section_mm2: Option<f64>,
    #[serde(default)]
    pub min_copper_thickness_um: Option<f64>,
    #[serde(default, rename = "rated_ambient_temperature_C")]
    pub rated_ambient_temperature_c: Option<f64>,
    #[serde(default)]
    pub min_airflow_lfm: Option<f64>,
    #[serde(default)]
    pub enclosure_profile: Option<String>,
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
    #[serde(default, rename = "measurement_uncertainty_C")]
    pub measurement_uncertainty_c: Option<f64>,
    #[serde(default)]
    pub power_loss_w: Option<f64>,
    #[serde(default)]
    pub measurement_point: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThermalLimit {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default, rename = "max_measured_temperature_C")]
    pub max_measured_temperature_c: Option<f64>,
    #[serde(default, rename = "max_temperature_rise_C")]
    pub max_temperature_rise_c: Option<f64>,
    #[serde(default, rename = "max_junction_temperature_margin_C")]
    pub max_junction_temperature_margin_c: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThermalPackageRule {
    pub component: String,
    pub source: String,
    #[serde(rename = "thermal_resistance_junction_to_ambient_C_per_W")]
    pub thermal_resistance_junction_to_ambient_c_per_w: f64,
    #[serde(rename = "max_junction_temperature_C")]
    pub max_junction_temperature_c: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThermalEnvironment {
    pub name: String,
    pub source: String,
    #[serde(rename = "ambient_temperature_C")]
    pub ambient_temperature_c: f64,
    #[serde(default)]
    pub airflow_lfm: Option<f64>,
    #[serde(default)]
    pub enclosure_profile: Option<String>,
}
