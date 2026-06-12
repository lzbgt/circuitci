mod analog_assertions;
mod analog_operating_limits;
mod analog_runner;
mod analog_soa;
mod analog_spice;
mod analog_util;
mod backdrive;
mod clock_source;
mod common;
mod control_line;
mod firmware_functional;
mod interface_protection;
mod io_voltage;
mod power_tree;
mod resident_protocol;
mod spice_netlist;
mod strap_bias;
mod target_contract;
mod uart_bootloader;

use crate::library::BoundBoard;
use crate::reports::{Finding, Limitation};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) const GPIO_BACKDRIVE: &str = "GPIO_BACKDRIVE";
pub(super) const INTERFACE_PROTECTION_REVIEW: &str = "INTERFACE_PROTECTION_REVIEW";
pub(super) const RESET_RELEASE_AFTER_POWER_VALID: &str = "RESET_RELEASE_AFTER_POWER_VALID";
pub(super) const BOOT_STRAP_DEFINED: &str = "BOOT_STRAP_DEFINED";
pub(super) const BOOT_STRAP_BIAS_VALID: &str = "BOOT_STRAP_BIAS_VALID";
pub(super) const UART_BOOTLOADER_SYNC: &str = "UART_BOOTLOADER_SYNC";
pub(super) const RESIDENT_BOOTLOADER_UPDATE_SEQUENCE: &str = "RESIDENT_BOOTLOADER_UPDATE_SEQUENCE";
pub(super) const CONTROL_LINE_RELEASE_SEQUENCE: &str = "CONTROL_LINE_RELEASE_SEQUENCE";
pub(super) const CLOCK_SOURCE_VALID: &str = "CLOCK_SOURCE_VALID";
pub(super) const FUNCTIONAL_MCU_FIRMWARE: &str = "FUNCTIONAL_MCU_FIRMWARE";
pub(super) const POWER_TREE_VALID: &str = "POWER_TREE_VALID";
pub(super) const IO_VOLTAGE_COMPATIBLE: &str = "IO_VOLTAGE_COMPATIBLE";
pub(super) const USB_CONNECTOR_PROTECTION_VALID: &str = "USB_CONNECTOR_PROTECTION_VALID";
pub(super) const USB_PROTECTION_PLACEMENT_VALID: &str = "USB_PROTECTION_PLACEMENT_VALID";
pub(super) const USB_ROUTE_GEOMETRY_VALID: &str = "USB_ROUTE_GEOMETRY_VALID";
pub(super) const SPICE_TRANSIENT_ANALYSIS: &str = "SPICE_TRANSIENT_ANALYSIS";
pub(super) const SPICE_OPERATING_LIMIT: &str = "SPICE_OPERATING_LIMIT";
const SUPPORTED_SCENARIO_TYPES: &[&str] = &[
    "gpio_backdrive",
    "reset_boot",
    "serial_programming",
    "firmware_update",
    "firmware_in_loop",
    "interface_protection",
    "power_tree",
    "control_line_sequence",
    "clock",
    "analog_transient",
];

#[derive(Debug, Default)]
pub struct ValidationOutcome {
    pub findings: Vec<Finding>,
    pub limitations: Vec<Limitation>,
    pub artifacts: Vec<String>,
    pub waveforms: Vec<String>,
}

pub fn validate(bound: &BoundBoard<'_>, output: &Path) -> ValidationOutcome {
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
                USB_ROUTE_GEOMETRY_VALID if scenario.scenario_type == "interface_protection" => {
                    interface_protection::validate_usb_route_geometry(
                        bound,
                        scenario,
                        &mut findings,
                    )
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
                IO_VOLTAGE_COMPATIBLE if scenario.scenario_type == "power_tree" => {
                    io_voltage::validate_io_voltage_compatible(bound, scenario, &mut findings)
                }
                SPICE_TRANSIENT_ANALYSIS if scenario.scenario_type == "analog_transient" => {
                    analog_spice::validate_spice_transient(
                        bound,
                        scenario,
                        &mut findings,
                        &mut artifacts,
                        &mut waveforms,
                        output,
                    )
                }
                GPIO_BACKDRIVE
                | INTERFACE_PROTECTION_REVIEW
                | RESET_RELEASE_AFTER_POWER_VALID
                | BOOT_STRAP_DEFINED
                | BOOT_STRAP_BIAS_VALID
                | UART_BOOTLOADER_SYNC
                | RESIDENT_BOOTLOADER_UPDATE_SEQUENCE
                | CONTROL_LINE_RELEASE_SEQUENCE
                | CLOCK_SOURCE_VALID
                | FUNCTIONAL_MCU_FIRMWARE
                | POWER_TREE_VALID
                | IO_VOLTAGE_COMPATIBLE
                | USB_CONNECTOR_PROTECTION_VALID
                | USB_PROTECTION_PLACEMENT_VALID
                | USB_ROUTE_GEOMETRY_VALID
                | SPICE_TRANSIENT_ANALYSIS => findings.push(Finding::critical(
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
