mod analog_ac_assertions;
mod analog_assertions;
mod analog_backend_plan;
mod analog_dc_assertions;
mod analog_dc_runner;
mod analog_dc_spice;
mod analog_noise_assertions;
mod analog_noise_runner;
mod analog_noise_spice;
mod analog_operating_limits;
mod analog_runner;
mod analog_soa;
mod analog_sparameter_spice;
mod analog_spice;
mod analog_sweep_reports;
mod analog_sweep_sampling;
mod analog_util;
mod analog_waveform_measurements;
mod analog_xyce_runner;
mod backdrive;
mod clock_source;
mod common;
mod control_line;
mod firmware_functional;
mod interface_protection;
mod io_voltage;
mod load_budget;
mod load_budget_power_switch;
mod manufacturing;
mod model_quality;
mod motor_drive;
mod motor_drive_bridge;
mod motor_drive_common;
mod power_tree;
mod resident_protocol;
mod spice_netlist;
mod strap_bias;
mod target_contract;
mod uart_bootloader;

use crate::board_ir::BoardProject;
use crate::library::BoundBoard;
use crate::reports::{Finding, Limitation};
use crate::validation_profiles::{IOT_BASIC_CORE_PROFILE_CHECKS, IOT_BASIC_V0};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) const GPIO_BACKDRIVE: &str = "GPIO_BACKDRIVE";
pub(super) const INTERFACE_PROTECTION_REVIEW: &str = "INTERFACE_PROTECTION_REVIEW";
pub(super) const BUS_TERMINATION_VALID: &str = "BUS_TERMINATION_VALID";
pub(super) const BUS_PROTECTION_PLACEMENT_VALID: &str = "BUS_PROTECTION_PLACEMENT_VALID";
pub(super) const RESET_RELEASE_AFTER_POWER_VALID: &str = "RESET_RELEASE_AFTER_POWER_VALID";
pub(super) const BOOT_STRAP_DEFINED: &str = "BOOT_STRAP_DEFINED";
pub(super) const BOOT_STRAP_BIAS_VALID: &str = "BOOT_STRAP_BIAS_VALID";
pub(super) const UART_BOOTLOADER_SYNC: &str = "UART_BOOTLOADER_SYNC";
pub(super) const RESIDENT_BOOTLOADER_UPDATE_SEQUENCE: &str = "RESIDENT_BOOTLOADER_UPDATE_SEQUENCE";
pub(super) const CONTROL_LINE_RELEASE_SEQUENCE: &str = "CONTROL_LINE_RELEASE_SEQUENCE";
pub(super) const CLOCK_SOURCE_VALID: &str = "CLOCK_SOURCE_VALID";
pub(super) const FUNCTIONAL_MCU_FIRMWARE: &str = "FUNCTIONAL_MCU_FIRMWARE";
pub(super) const POWER_TREE_VALID: &str = "POWER_TREE_VALID";
pub(super) const DRILL_DIAMETER_VALID: &str = "DRILL_DIAMETER_VALID";
pub(super) const DRILL_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID";
pub(super) const SLOT_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "SLOT_TO_BOARD_EDGE_CLEARANCE_VALID";
pub(super) const SLOT_WIDTH_VALID: &str = "SLOT_WIDTH_VALID";
pub(super) const SLOT_ASPECT_RATIO_VALID: &str = "SLOT_ASPECT_RATIO_VALID";
pub(super) const CASTELLATED_HOLE_VALID: &str = "CASTELLATED_HOLE_VALID";
pub(super) const DRILL_ANNULAR_RING_VALID: &str = "DRILL_ANNULAR_RING_VALID";
pub(super) const COPPER_TO_BOARD_EDGE_CLEARANCE_VALID: &str =
    "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID";
pub(super) const COPPER_SPACING_VALID: &str = "COPPER_SPACING_VALID";
pub(super) const CONDUCTOR_CREEPAGE_CLEARANCE_VALID: &str = "CONDUCTOR_CREEPAGE_CLEARANCE_VALID";
pub(super) const RF_ANTENNA_KEEPOUT_VALID: &str = "RF_ANTENNA_KEEPOUT_VALID";
pub(super) const RF_ANTENNA_FEED_PATH_VALID: &str = "RF_ANTENNA_FEED_PATH_VALID";
pub(super) const RF_ANTENNA_MATCHING_TOPOLOGY_VALID: &str = "RF_ANTENNA_MATCHING_TOPOLOGY_VALID";
pub(super) const RF_ANTENNA_MEASURED_PERFORMANCE_VALID: &str =
    "RF_ANTENNA_MEASURED_PERFORMANCE_VALID";
pub(super) const THERMAL_COPPER_AREA_VALID: &str = "THERMAL_COPPER_AREA_VALID";
pub(super) const THERMAL_VIA_STACKUP_VALID: &str = "THERMAL_VIA_STACKUP_VALID";
pub(super) const THERMAL_VIA_PLATING_VALID: &str = "THERMAL_VIA_PLATING_VALID";
pub(super) const THERMAL_VIA_BARREL_CROSS_SECTION_VALID: &str =
    "THERMAL_VIA_BARREL_CROSS_SECTION_VALID";
pub(super) const THERMAL_PACKAGE_TEMPERATURE_VALID: &str = "THERMAL_PACKAGE_TEMPERATURE_VALID";
pub(super) const THERMAL_MEASURED_TEMPERATURE_VALID: &str = "THERMAL_MEASURED_TEMPERATURE_VALID";
pub(super) const THERMAL_DERATING_ENVIRONMENT_VALID: &str = "THERMAL_DERATING_ENVIRONMENT_VALID";
pub(super) const CONTROLLED_IMPEDANCE_GEOMETRY_VALID: &str = "CONTROLLED_IMPEDANCE_GEOMETRY_VALID";
pub(super) const CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID: &str =
    "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID";
pub(super) const CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID: &str =
    "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID";
pub(super) const CONTROLLED_IMPEDANCE_COUPON_VALID: &str = "CONTROLLED_IMPEDANCE_COUPON_VALID";
pub(super) const CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID: &str =
    "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID";
pub(super) const CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID: &str =
    "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID";
pub(super) const CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID: &str =
    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID";
pub(super) const ADJACENT_PLANE_RETURN_PATH_VALID: &str = "ADJACENT_PLANE_RETURN_PATH_VALID";
pub(super) const REFERENCE_PLANE_SLOT_CROSSING_VALID: &str = "REFERENCE_PLANE_SLOT_CROSSING_VALID";
pub(super) const RETURN_PATH_STITCHING_VIA_VALID: &str = "RETURN_PATH_STITCHING_VIA_VALID";
pub(super) const SOLDER_MASK_OPENING_VALID: &str = "SOLDER_MASK_OPENING_VALID";
pub(super) const SOLDER_MASK_DAM_VALID: &str = "SOLDER_MASK_DAM_VALID";
pub(super) const SOLDER_PASTE_OPENING_VALID: &str = "SOLDER_PASTE_OPENING_VALID";
pub(super) const SOLDER_PASTE_APERTURE_SIZE_VALID: &str = "SOLDER_PASTE_APERTURE_SIZE_VALID";
pub(super) const SOLDER_PASTE_APERTURE_AREA_RATIO_VALID: &str =
    "SOLDER_PASTE_APERTURE_AREA_RATIO_VALID";
pub(super) const SOLDER_PASTE_IC_PIN_APERTURE_VALID: &str = "SOLDER_PASTE_IC_PIN_APERTURE_VALID";
pub(super) const SOLDER_PASTE_BGA_APERTURE_VALID: &str = "SOLDER_PASTE_BGA_APERTURE_VALID";
pub(super) const SOLDER_PASTE_SPACING_VALID: &str = "SOLDER_PASTE_SPACING_VALID";
pub(super) const ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID: &str = "ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID";
pub(super) const PIN_1_ORIENTATION_VALID: &str = "PIN_1_ORIENTATION_VALID";
pub(super) const IO_VOLTAGE_COMPATIBLE: &str = "IO_VOLTAGE_COMPATIBLE";
pub(super) const USB_CONNECTOR_PROTECTION_VALID: &str = "USB_CONNECTOR_PROTECTION_VALID";
pub(super) const USB_PROTECTION_PLACEMENT_VALID: &str = "USB_PROTECTION_PLACEMENT_VALID";
pub(super) const USB_CONNECTOR_ORIENTATION_VALID: &str = "USB_CONNECTOR_ORIENTATION_VALID";
pub(super) const USB_CONNECTOR_EDGE_PROXIMITY_VALID: &str = "USB_CONNECTOR_EDGE_PROXIMITY_VALID";
pub(super) const USB_CONNECTOR_BODY_OVERHANG_VALID: &str = "USB_CONNECTOR_BODY_OVERHANG_VALID";
pub(super) const USB_CONNECTOR_COMPONENT_CLEARANCE_VALID: &str =
    "USB_CONNECTOR_COMPONENT_CLEARANCE_VALID";
pub(super) const USB_CONNECTOR_ENTRY_CLEARANCE_VALID: &str = "USB_CONNECTOR_ENTRY_CLEARANCE_VALID";
pub(super) const USB_ROUTE_GEOMETRY_VALID: &str = "USB_ROUTE_GEOMETRY_VALID";
pub(super) const USB_VBUS_ROUTE_VALID: &str = "USB_VBUS_ROUTE_VALID";
pub(super) const USB_RETURN_PATH_VALID: &str = "USB_RETURN_PATH_VALID";
pub(super) const SPICE_TRANSIENT_ANALYSIS: &str = "SPICE_TRANSIENT_ANALYSIS";
pub(super) const SPICE_AC_ANALYSIS: &str = "SPICE_AC_ANALYSIS";
pub(super) const SPICE_DC_ANALYSIS: &str = "SPICE_DC_ANALYSIS";
pub(super) const SPICE_NOISE_ANALYSIS: &str = "SPICE_NOISE_ANALYSIS";
pub(super) const SPICE_S_PARAMETER_ANALYSIS: &str = "SPICE_S_PARAMETER_ANALYSIS";
pub(super) const SPICE_OPERATING_LIMIT: &str = "SPICE_OPERATING_LIMIT";
pub(super) const MOTOR_BRIDGE_BUDGET_VALID: &str = "MOTOR_BRIDGE_BUDGET_VALID";
pub(super) const MOTOR_BRIDGE_LOSS_THERMAL_VALID: &str = "MOTOR_BRIDGE_LOSS_THERMAL_VALID";
pub(super) const MOTOR_BRIDGE_SWITCHING_VALID: &str = "MOTOR_BRIDGE_SWITCHING_VALID";
pub(super) const MOTOR_BRIDGE_SOA_VALID: &str = "MOTOR_BRIDGE_SOA_VALID";
pub(super) const MOTOR_LOAD_SUPPLY_VALID: &str = "MOTOR_LOAD_SUPPLY_VALID";
pub(super) const MOTOR_REGEN_CLAMP_VALID: &str = "MOTOR_REGEN_CLAMP_VALID";
pub(super) const MOTOR_ROUTE_CURRENT_VALID: &str = "MOTOR_ROUTE_CURRENT_VALID";
pub(super) const MOTOR_CURRENT_SENSE_ACCURACY_VALID: &str = "MOTOR_CURRENT_SENSE_ACCURACY_VALID";
pub(super) const MOTOR_CURRENT_SENSE_PLACEMENT_VALID: &str = "MOTOR_CURRENT_SENSE_PLACEMENT_VALID";
pub(super) const LOAD_CONNECTOR_CURRENT_VALID: &str = "LOAD_CONNECTOR_CURRENT_VALID";
pub(super) const POWER_SWITCH_BUDGET_VALID: &str = "POWER_SWITCH_BUDGET_VALID";
pub(super) const POWER_SWITCH_REVERSE_CURRENT_VALID: &str = "POWER_SWITCH_REVERSE_CURRENT_VALID";
pub(super) const POWER_SWITCH_INRUSH_VALID: &str = "POWER_SWITCH_INRUSH_VALID";
pub(super) const LOAD_CABLE_CURRENT_VALID: &str = "LOAD_CABLE_CURRENT_VALID";
pub(super) const LOAD_CABLE_THERMAL_DERATING_VALID: &str = "LOAD_CABLE_THERMAL_DERATING_VALID";
pub(super) const LOAD_CABLE_VOLTAGE_DROP_VALID: &str = "LOAD_CABLE_VOLTAGE_DROP_VALID";
pub(super) const MODEL_QUALITY_REQUIRED: &str = "MODEL_QUALITY_REQUIRED";
const SUPPORTED_SCENARIO_TYPES: &[&str] = &[
    "gpio_backdrive",
    "reset_boot",
    "serial_programming",
    "firmware_update",
    "firmware_in_loop",
    "interface_protection",
    "manufacturing",
    "power_tree",
    "control_line_sequence",
    "clock",
    "analog_transient",
    "analog_ac",
    "analog_dc",
    "analog_noise",
    "analog_sparameter",
    "motor_drive",
    "load_budget",
    "model_quality",
];

#[derive(Debug, Default)]
pub struct ValidationOutcome {
    pub findings: Vec<Finding>,
    pub limitations: Vec<Limitation>,
    pub artifacts: Vec<String>,
    pub waveforms: Vec<String>,
}

pub fn profile_coverage_limitations(profile: &str, project: &BoardProject) -> Vec<Limitation> {
    if profile != IOT_BASIC_V0 {
        return Vec::new();
    }

    let declared_checks: BTreeSet<&str> = project
        .scenarios
        .iter()
        .flat_map(|scenario| scenario.checks.iter().map(String::as_str))
        .collect();
    let missing_checks: Vec<_> = IOT_BASIC_CORE_PROFILE_CHECKS
        .iter()
        .copied()
        .filter(|check| !declared_checks.contains(check))
        .collect();
    if missing_checks.is_empty() {
        return Vec::new();
    }

    let declared_summary = if declared_checks.is_empty() {
        "none".to_string()
    } else {
        declared_checks
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .join(", ")
    };
    vec![Limitation {
        id: "PROFILE_COVERAGE_PARTIAL".to_string(),
        scope: "profile:iot_basic_v0".to_string(),
        confidence: "high".to_string(),
        blocking: false,
        message: format!(
            "iot_basic_v0 validation is scenario-declaration driven. Declared checks: {declared_summary}. Missing core profile coverage: {}. Add scenarios or run suggest-scenarios --profile iot_basic_v0 after importing available evidence before treating this report as full-profile sign-off.",
            missing_checks.join(", ")
        ),
    }]
}

pub fn validate(bound: &BoundBoard<'_>, output: &Path) -> ValidationOutcome {
    validate_with_progress(bound, output, |_, _| {})
}

pub fn validate_with_progress<F>(
    bound: &BoundBoard<'_>,
    output: &Path,
    mut on_progress: F,
) -> ValidationOutcome
where
    F: FnMut(&'static str, String),
{
    validate_with_progress_and_cancel(bound, output, &mut on_progress, || false)
}

pub fn validate_with_progress_and_cancel<F, C>(
    bound: &BoundBoard<'_>,
    output: &Path,
    mut on_progress: F,
    should_cancel: C,
) -> ValidationOutcome
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    let mut findings = bound.findings.clone();
    let mut limitations = model_quality_limitations(bound);
    if matches!(
        bound.project.project.import_source.as_deref(),
        Some("kicad_xml_netlist" | "kicad_schematic")
    ) {
        limitations.push(Limitation {
            id: "SCHEMATIC_IMPORT_ONLY".to_string(),
            scope: "project".to_string(),
            confidence: "high".to_string(),
            blocking: false,
            message: "This project was imported from KiCad schematic connectivity. It is not physical simulation sign-off until explicit component models and validation scenarios are added.".to_string(),
        });
    }
    let mut artifacts = Vec::new();
    let mut waveforms = Vec::new();
    let mut added_backdrive_limitation = false;
    let mut added_protocol_limitation = false;
    let mut added_control_line_limitation = false;

    for scenario in &bound.project.scenarios {
        if should_cancel() {
            findings.push(Finding::critical(
                "VALIDATION_CANCELED",
                "validation",
                "Validation was canceled before all scenarios completed.",
            ));
            return ValidationOutcome {
                findings,
                limitations,
                artifacts,
                waveforms,
            };
        }
        if !SUPPORTED_SCENARIO_TYPES.contains(&scenario.scenario_type.as_str()) {
            limitations.push(Limitation {
                id: "UNSUPPORTED_SCENARIO".to_string(),
                scope: format!("scenario:{}", scenario.name),
                confidence: "low".to_string(),
                blocking: true,
                message: format!(
                    "Scenario type {} is not implemented in this runtime.",
                    scenario.scenario_type
                ),
            });
        }

        let mut seen = BTreeSet::new();
        for check in &scenario.checks {
            if !seen.insert(check) {
                continue;
            }
            match check.as_str() {
                GPIO_BACKDRIVE if scenario.scenario_type == "gpio_backdrive" => {
                    if !added_backdrive_limitation {
                        limitations.push(Limitation {
                            id: "SIMPLE_BACKDRIVE_MODEL".to_string(),
                            scope: "validation:GPIO_BACKDRIVE".to_string(),
                            confidence: "medium".to_string(),
                            blocking: false,
                            message: "GPIO_BACKDRIVE uses a deterministic behavioral approximation, not an analog solver waveform.".to_string(),
                        });
                        added_backdrive_limitation = true;
                    }
                    backdrive::validate_backdrive(bound, scenario, &mut findings)
                }
                INTERFACE_PROTECTION_REVIEW if scenario.scenario_type == "interface_protection" => {
                    interface_protection::validate_interface_protection(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                BUS_TERMINATION_VALID if scenario.scenario_type == "interface_protection" => {
                    interface_protection::validate_bus_termination(bound, scenario, &mut findings)
                }
                BUS_PROTECTION_PLACEMENT_VALID
                    if scenario.scenario_type == "interface_protection" =>
                {
                    interface_protection::validate_bus_protection_placement(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                USB_CONNECTOR_PROTECTION_VALID
                    if scenario.scenario_type == "interface_protection" =>
                {
                    interface_protection::validate_usb_connector_protection(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                USB_PROTECTION_PLACEMENT_VALID
                    if scenario.scenario_type == "interface_protection" =>
                {
                    interface_protection::validate_usb_protection_placement(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                USB_CONNECTOR_ORIENTATION_VALID
                    if scenario.scenario_type == "interface_protection" =>
                {
                    interface_protection::validate_usb_connector_orientation(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                USB_CONNECTOR_EDGE_PROXIMITY_VALID
                    if scenario.scenario_type == "interface_protection" =>
                {
                    interface_protection::validate_usb_connector_edge_proximity(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                USB_CONNECTOR_BODY_OVERHANG_VALID
                    if scenario.scenario_type == "interface_protection" =>
                {
                    interface_protection::validate_usb_connector_body_overhang(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                USB_CONNECTOR_COMPONENT_CLEARANCE_VALID
                    if scenario.scenario_type == "interface_protection" =>
                {
                    interface_protection::validate_usb_connector_component_clearance(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                USB_CONNECTOR_ENTRY_CLEARANCE_VALID
                    if scenario.scenario_type == "interface_protection" =>
                {
                    interface_protection::validate_usb_connector_entry_clearance(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                USB_ROUTE_GEOMETRY_VALID if scenario.scenario_type == "interface_protection" => {
                    interface_protection::validate_usb_route_geometry(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                USB_VBUS_ROUTE_VALID if scenario.scenario_type == "interface_protection" => {
                    interface_protection::validate_usb_vbus_route(bound, scenario, &mut findings)
                }
                USB_RETURN_PATH_VALID if scenario.scenario_type == "interface_protection" => {
                    interface_protection::validate_usb_return_path(bound, scenario, &mut findings)
                }
                RESET_RELEASE_AFTER_POWER_VALID if scenario.scenario_type == "reset_boot" => {
                    target_contract::validate_reset_release(bound, scenario, &mut findings)
                }
                BOOT_STRAP_DEFINED if scenario.scenario_type == "reset_boot" => {
                    target_contract::validate_boot_straps(bound, scenario, &mut findings)
                }
                BOOT_STRAP_BIAS_VALID if scenario.scenario_type == "reset_boot" => {
                    strap_bias::validate_boot_strap_bias(bound, scenario, &mut findings)
                }
                UART_BOOTLOADER_SYNC if scenario.scenario_type == "serial_programming" => {
                    uart_bootloader::validate_uart_bootloader_sync(bound, scenario, &mut findings)
                }
                RESIDENT_BOOTLOADER_UPDATE_SEQUENCE
                    if scenario.scenario_type == "firmware_update" =>
                {
                    if !added_protocol_limitation {
                        limitations.push(Limitation {
                            id: "ABSTRACT_PROTOCOL_TRACE".to_string(),
                            scope: format!("validation:{RESIDENT_BOOTLOADER_UPDATE_SEQUENCE}"),
                            confidence: "medium".to_string(),
                            blocking: false,
                            message: "Resident protocol validation checks declared transaction traces; it does not execute firmware, decode raw frames, recompute CRCs, or prove HIL behavior.".to_string(),
                        });
                        added_protocol_limitation = true;
                    }
                    resident_protocol::validate_resident_bootloader_update(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                CONTROL_LINE_RELEASE_SEQUENCE
                    if scenario.scenario_type == "control_line_sequence" =>
                {
                    if !added_control_line_limitation {
                        limitations.push(Limitation {
                            id: "ABSTRACT_CONTROL_LINE_MODEL".to_string(),
                            scope: format!("validation:{CONTROL_LINE_RELEASE_SEQUENCE}"),
                            confidence: "medium".to_string(),
                            blocking: false,
                            message: "Control-line release validation uses declared line effects and release delays; it is not a transistor-level or RC waveform solver.".to_string(),
                        });
                        added_control_line_limitation = true;
                    }
                    control_line::validate_control_line_release(bound, scenario, &mut findings)
                }
                CLOCK_SOURCE_VALID if scenario.scenario_type == "clock" => {
                    clock_source::validate_clock_sources(bound, scenario, &mut findings)
                }
                FUNCTIONAL_MCU_FIRMWARE if scenario.scenario_type == "firmware_in_loop" => {
                    firmware_functional::validate_functional_mcu_firmware(
                        bound,
                        scenario,
                        &mut findings,
                        &mut artifacts,
                        output,
                    )
                }
                POWER_TREE_VALID if scenario.scenario_type == "power_tree" => {
                    power_tree::validate_power_tree(bound, scenario, &mut findings)
                }
                DRILL_DIAMETER_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_drill_diameter(bound, scenario, &mut findings)
                }
                DRILL_TO_BOARD_EDGE_CLEARANCE_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_drill_to_board_edge_clearance(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                SLOT_TO_BOARD_EDGE_CLEARANCE_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_slot_to_board_edge_clearance(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                SLOT_WIDTH_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_slot_width(bound, scenario, &mut findings)
                }
                SLOT_ASPECT_RATIO_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_slot_aspect_ratio(bound, scenario, &mut findings)
                }
                CASTELLATED_HOLE_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_castellated_hole(bound, scenario, &mut findings)
                }
                DRILL_ANNULAR_RING_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_drill_annular_ring(bound, scenario, &mut findings)
                }
                COPPER_TO_BOARD_EDGE_CLEARANCE_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_copper_to_board_edge_clearance(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                COPPER_SPACING_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_copper_spacing(bound, scenario, &mut findings)
                }
                CONDUCTOR_CREEPAGE_CLEARANCE_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_conductor_creepage_clearance(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                RF_ANTENNA_KEEPOUT_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_rf_antenna_keepout(bound, scenario, &mut findings)
                }
                RF_ANTENNA_FEED_PATH_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_rf_antenna_feed_path(bound, scenario, &mut findings)
                }
                RF_ANTENNA_MATCHING_TOPOLOGY_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_rf_antenna_matching_topology(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                RF_ANTENNA_MEASURED_PERFORMANCE_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_rf_antenna_measured_performance(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                THERMAL_COPPER_AREA_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_thermal_copper_area(bound, scenario, &mut findings)
                }
                THERMAL_VIA_STACKUP_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_thermal_via_stackup(bound, scenario, &mut findings)
                }
                THERMAL_VIA_PLATING_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_thermal_via_plating(bound, scenario, &mut findings)
                }
                THERMAL_VIA_BARREL_CROSS_SECTION_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_thermal_via_barrel_cross_section(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                THERMAL_PACKAGE_TEMPERATURE_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_thermal_package_temperature(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                THERMAL_MEASURED_TEMPERATURE_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_thermal_measured_temperature(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                THERMAL_DERATING_ENVIRONMENT_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_thermal_derating_environment(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                CONTROLLED_IMPEDANCE_GEOMETRY_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_controlled_impedance_geometry(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_controlled_impedance_stackup_evidence(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_controlled_impedance_solder_mask_loading(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                CONTROLLED_IMPEDANCE_COUPON_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_controlled_impedance_coupon(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_controlled_impedance_coupon_batch(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_controlled_impedance_coupon_trace_correlation(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_controlled_impedance_solver_result(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                ADJACENT_PLANE_RETURN_PATH_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_adjacent_plane_return_path(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                REFERENCE_PLANE_SLOT_CROSSING_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_reference_plane_slot_crossing(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                RETURN_PATH_STITCHING_VIA_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_return_path_stitching_via(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                SOLDER_MASK_OPENING_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_solder_mask_opening(bound, scenario, &mut findings)
                }
                SOLDER_MASK_DAM_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_solder_mask_dam(bound, scenario, &mut findings)
                }
                SOLDER_PASTE_OPENING_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_solder_paste_opening(bound, scenario, &mut findings)
                }
                SOLDER_PASTE_APERTURE_SIZE_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_solder_paste_aperture_size(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                SOLDER_PASTE_APERTURE_AREA_RATIO_VALID
                    if scenario.scenario_type == "manufacturing" =>
                {
                    manufacturing::validate_solder_paste_aperture_area_ratio(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                SOLDER_PASTE_IC_PIN_APERTURE_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_solder_paste_ic_pin_aperture(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                SOLDER_PASTE_BGA_APERTURE_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_solder_paste_bga_aperture(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                SOLDER_PASTE_SPACING_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_solder_paste_spacing(bound, scenario, &mut findings)
                }
                ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_assembly_footprint_alignment(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                PIN_1_ORIENTATION_VALID if scenario.scenario_type == "manufacturing" => {
                    manufacturing::validate_pin_1_orientation(bound, scenario, &mut findings)
                }
                IO_VOLTAGE_COMPATIBLE if scenario.scenario_type == "power_tree" => {
                    io_voltage::validate_io_voltage_compatible(bound, scenario, &mut findings)
                }
                SPICE_TRANSIENT_ANALYSIS if scenario.scenario_type == "analog_transient" => {
                    let mut sinks = analog_spice::AnalogTransientSinks {
                        findings: &mut findings,
                        artifacts: &mut artifacts,
                        waveforms: &mut waveforms,
                    };
                    analog_spice::validate_spice_transient_with_progress(
                        bound,
                        scenario,
                        &mut sinks,
                        output,
                        &mut on_progress,
                        &should_cancel,
                    )
                }
                SPICE_AC_ANALYSIS if scenario.scenario_type == "analog_ac" => {
                    let mut sinks = analog_spice::AnalogAcSinks {
                        findings: &mut findings,
                        artifacts: &mut artifacts,
                        waveforms: &mut waveforms,
                    };
                    analog_spice::validate_spice_ac_with_progress(
                        bound,
                        scenario,
                        &mut sinks,
                        output,
                        &mut on_progress,
                        &should_cancel,
                    )
                }
                SPICE_DC_ANALYSIS if scenario.scenario_type == "analog_dc" => {
                    let mut sinks = analog_dc_spice::AnalogDcSinks {
                        findings: &mut findings,
                        artifacts: &mut artifacts,
                    };
                    analog_dc_spice::validate_spice_dc_with_progress(
                        bound,
                        scenario,
                        &mut sinks,
                        output,
                        &mut on_progress,
                        &should_cancel,
                    )
                }
                SPICE_NOISE_ANALYSIS if scenario.scenario_type == "analog_noise" => {
                    let mut sinks = analog_noise_spice::AnalogNoiseSinks {
                        findings: &mut findings,
                        artifacts: &mut artifacts,
                        waveforms: &mut waveforms,
                    };
                    analog_noise_spice::validate_spice_noise_with_progress(
                        bound,
                        scenario,
                        &mut sinks,
                        output,
                        &mut on_progress,
                        &should_cancel,
                    )
                }
                SPICE_S_PARAMETER_ANALYSIS if scenario.scenario_type == "analog_sparameter" => {
                    let mut sinks = analog_sparameter_spice::AnalogSParameterSinks {
                        findings: &mut findings,
                        artifacts: &mut artifacts,
                    };
                    analog_sparameter_spice::validate_spice_sparameter_with_progress(
                        bound,
                        scenario,
                        &mut sinks,
                        &mut on_progress,
                        &should_cancel,
                    )
                }
                MOTOR_BRIDGE_BUDGET_VALID if scenario.scenario_type == "motor_drive" => {
                    motor_drive::validate_motor_bridge_budget(bound, scenario, &mut findings)
                }
                MOTOR_BRIDGE_LOSS_THERMAL_VALID if scenario.scenario_type == "motor_drive" => {
                    motor_drive_bridge::validate_motor_bridge_loss_thermal(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                MOTOR_BRIDGE_SWITCHING_VALID if scenario.scenario_type == "motor_drive" => {
                    motor_drive_bridge::validate_motor_bridge_switching(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                MOTOR_BRIDGE_SOA_VALID if scenario.scenario_type == "motor_drive" => {
                    motor_drive::validate_motor_bridge_soa(bound, scenario, &mut findings)
                }
                MOTOR_LOAD_SUPPLY_VALID if scenario.scenario_type == "motor_drive" => {
                    motor_drive_bridge::validate_motor_load_supply(bound, scenario, &mut findings)
                }
                MOTOR_REGEN_CLAMP_VALID if scenario.scenario_type == "motor_drive" => {
                    motor_drive::validate_motor_regen_clamp(bound, scenario, &mut findings)
                }
                MOTOR_ROUTE_CURRENT_VALID if scenario.scenario_type == "motor_drive" => {
                    motor_drive::validate_motor_route_current(bound, scenario, &mut findings)
                }
                MOTOR_CURRENT_SENSE_ACCURACY_VALID if scenario.scenario_type == "motor_drive" => {
                    motor_drive::validate_motor_current_sense_accuracy(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                MOTOR_CURRENT_SENSE_PLACEMENT_VALID if scenario.scenario_type == "motor_drive" => {
                    motor_drive::validate_motor_current_sense_placement(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                LOAD_CONNECTOR_CURRENT_VALID if scenario.scenario_type == "load_budget" => {
                    load_budget::validate_load_connector_current(bound, scenario, &mut findings)
                }
                POWER_SWITCH_BUDGET_VALID if scenario.scenario_type == "load_budget" => {
                    load_budget_power_switch::validate_power_switch_budget(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                POWER_SWITCH_REVERSE_CURRENT_VALID if scenario.scenario_type == "load_budget" => {
                    load_budget_power_switch::validate_power_switch_reverse_current(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                POWER_SWITCH_INRUSH_VALID if scenario.scenario_type == "load_budget" => {
                    load_budget_power_switch::validate_power_switch_inrush(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                LOAD_CABLE_CURRENT_VALID if scenario.scenario_type == "load_budget" => {
                    load_budget::validate_load_cable_current(bound, scenario, &mut findings)
                }
                LOAD_CABLE_THERMAL_DERATING_VALID if scenario.scenario_type == "load_budget" => {
                    load_budget::validate_load_cable_thermal_derating(
                        bound,
                        scenario,
                        &mut findings,
                    )
                }
                LOAD_CABLE_VOLTAGE_DROP_VALID if scenario.scenario_type == "load_budget" => {
                    load_budget::validate_load_cable_voltage_drop(bound, scenario, &mut findings)
                }
                MODEL_QUALITY_REQUIRED if scenario.scenario_type == "model_quality" => {
                    model_quality::validate_model_quality_required(bound, scenario, &mut findings)
                }
                GPIO_BACKDRIVE
                | INTERFACE_PROTECTION_REVIEW
                | BUS_TERMINATION_VALID
                | BUS_PROTECTION_PLACEMENT_VALID
                | RESET_RELEASE_AFTER_POWER_VALID
                | BOOT_STRAP_DEFINED
                | BOOT_STRAP_BIAS_VALID
                | UART_BOOTLOADER_SYNC
                | RESIDENT_BOOTLOADER_UPDATE_SEQUENCE
                | CONTROL_LINE_RELEASE_SEQUENCE
                | CLOCK_SOURCE_VALID
                | FUNCTIONAL_MCU_FIRMWARE
                | POWER_TREE_VALID
                | DRILL_DIAMETER_VALID
                | DRILL_TO_BOARD_EDGE_CLEARANCE_VALID
                | SLOT_TO_BOARD_EDGE_CLEARANCE_VALID
                | SLOT_WIDTH_VALID
                | SLOT_ASPECT_RATIO_VALID
                | CASTELLATED_HOLE_VALID
                | DRILL_ANNULAR_RING_VALID
                | COPPER_TO_BOARD_EDGE_CLEARANCE_VALID
                | COPPER_SPACING_VALID
                | CONDUCTOR_CREEPAGE_CLEARANCE_VALID
                | RF_ANTENNA_KEEPOUT_VALID
                | RF_ANTENNA_FEED_PATH_VALID
                | RF_ANTENNA_MATCHING_TOPOLOGY_VALID
                | RF_ANTENNA_MEASURED_PERFORMANCE_VALID
                | THERMAL_COPPER_AREA_VALID
                | THERMAL_VIA_STACKUP_VALID
                | THERMAL_VIA_PLATING_VALID
                | THERMAL_VIA_BARREL_CROSS_SECTION_VALID
                | THERMAL_PACKAGE_TEMPERATURE_VALID
                | THERMAL_MEASURED_TEMPERATURE_VALID
                | THERMAL_DERATING_ENVIRONMENT_VALID
                | CONTROLLED_IMPEDANCE_GEOMETRY_VALID
                | CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID
                | CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID
                | CONTROLLED_IMPEDANCE_COUPON_VALID
                | CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID
                | CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID
                | CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID
                | ADJACENT_PLANE_RETURN_PATH_VALID
                | REFERENCE_PLANE_SLOT_CROSSING_VALID
                | RETURN_PATH_STITCHING_VIA_VALID
                | SOLDER_MASK_OPENING_VALID
                | SOLDER_MASK_DAM_VALID
                | SOLDER_PASTE_OPENING_VALID
                | SOLDER_PASTE_APERTURE_SIZE_VALID
                | SOLDER_PASTE_APERTURE_AREA_RATIO_VALID
                | SOLDER_PASTE_IC_PIN_APERTURE_VALID
                | SOLDER_PASTE_BGA_APERTURE_VALID
                | SOLDER_PASTE_SPACING_VALID
                | ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID
                | PIN_1_ORIENTATION_VALID
                | IO_VOLTAGE_COMPATIBLE
                | USB_CONNECTOR_PROTECTION_VALID
                | USB_PROTECTION_PLACEMENT_VALID
                | USB_CONNECTOR_ORIENTATION_VALID
                | USB_CONNECTOR_EDGE_PROXIMITY_VALID
                | USB_CONNECTOR_BODY_OVERHANG_VALID
                | USB_CONNECTOR_COMPONENT_CLEARANCE_VALID
                | USB_CONNECTOR_ENTRY_CLEARANCE_VALID
                | USB_ROUTE_GEOMETRY_VALID
                | USB_VBUS_ROUTE_VALID
                | USB_RETURN_PATH_VALID
                | SPICE_TRANSIENT_ANALYSIS
                | SPICE_AC_ANALYSIS
                | SPICE_DC_ANALYSIS
                | SPICE_NOISE_ANALYSIS
                | SPICE_S_PARAMETER_ANALYSIS
                | MOTOR_BRIDGE_BUDGET_VALID
                | MOTOR_BRIDGE_LOSS_THERMAL_VALID
                | MOTOR_BRIDGE_SWITCHING_VALID
                | MOTOR_BRIDGE_SOA_VALID
                | MOTOR_LOAD_SUPPLY_VALID
                | MOTOR_REGEN_CLAMP_VALID
                | MOTOR_ROUTE_CURRENT_VALID
                | MOTOR_CURRENT_SENSE_ACCURACY_VALID
                | MOTOR_CURRENT_SENSE_PLACEMENT_VALID
                | LOAD_CONNECTOR_CURRENT_VALID
                | LOAD_CABLE_CURRENT_VALID
                | LOAD_CABLE_THERMAL_DERATING_VALID
                | LOAD_CABLE_VOLTAGE_DROP_VALID
                | MODEL_QUALITY_REQUIRED => findings.push(Finding::critical(
                    "CHECK_SCENARIO_TYPE_MISMATCH",
                    &scenario.name,
                    format!(
                        "Check {check} is not valid for scenario type {}.",
                        scenario.scenario_type
                    ),
                )),
                other => limitations.push(Limitation {
                    id: "UNSUPPORTED_CHECK".to_string(),
                    scope: format!("scenario:{}:check:{other}", scenario.name),
                    confidence: "low".to_string(),
                    blocking: true,
                    message: format!("Check {other} is not implemented in this runtime."),
                }),
            }
        }
    }

    ValidationOutcome {
        findings,
        limitations,
        artifacts,
        waveforms,
    }
}

fn model_quality_limitations(bound: &BoundBoard<'_>) -> Vec<Limitation> {
    let mut limitations = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let source = model.model_quality.source.as_str();
        let confidence = model.model_quality.confidence.as_str();
        if matches!(source, "estimated" | "generic") || confidence == "low" {
            limitations.push(Limitation {
                id: "LOW_CONFIDENCE_MODEL".to_string(),
                scope: format!("component:{component_id}:model:{}", model.component_id),
                confidence: model.model_quality.confidence.clone(),
                blocking: false,
                message: format!(
                    "Component {component_id} uses {} model {} with {} confidence.",
                    model.model_quality.source, model.component_id, model.model_quality.confidence
                ),
            });
        }
    }
    limitations
}
