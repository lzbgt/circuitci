use crate::board_ir::{BoardProject, ComponentSpec, NetKind, SpicePrimitive};
use crate::charger_programming::derive_charge_current_from_programming_resistor;
use crate::library::{BoundBoard, ComponentModel, PortKind, PowerSwitchState};
use crate::power_mux_selection::derive_selected_power_mux_input_from_powered_nets;
use std::collections::BTreeMap;

mod backdrive;
mod control_line;
mod interface_protection;
mod manufacturing;
mod types;

pub use types::*;

const POWER_TREE_VALID: &str = "POWER_TREE_VALID";
pub(super) const GPIO_BACKDRIVE: &str = "GPIO_BACKDRIVE";
const INTERFACE_PROTECTION_REVIEW: &str = "INTERFACE_PROTECTION_REVIEW";
const USB_CONNECTOR_PROTECTION_VALID: &str = "USB_CONNECTOR_PROTECTION_VALID";
const USB_PROTECTION_PLACEMENT_VALID: &str = "USB_PROTECTION_PLACEMENT_VALID";
const USB_CONNECTOR_ORIENTATION_VALID: &str = "USB_CONNECTOR_ORIENTATION_VALID";
const USB_CONNECTOR_EDGE_PROXIMITY_VALID: &str = "USB_CONNECTOR_EDGE_PROXIMITY_VALID";
const USB_CONNECTOR_BODY_OVERHANG_VALID: &str = "USB_CONNECTOR_BODY_OVERHANG_VALID";
const USB_CONNECTOR_COMPONENT_CLEARANCE_VALID: &str = "USB_CONNECTOR_COMPONENT_CLEARANCE_VALID";
const USB_CONNECTOR_ENTRY_CLEARANCE_VALID: &str = "USB_CONNECTOR_ENTRY_CLEARANCE_VALID";
const USB_ROUTE_GEOMETRY_VALID: &str = "USB_ROUTE_GEOMETRY_VALID";
const USB_VBUS_ROUTE_VALID: &str = "USB_VBUS_ROUTE_VALID";
const USB_RETURN_PATH_VALID: &str = "USB_RETURN_PATH_VALID";
const IO_VOLTAGE_COMPATIBLE: &str = "IO_VOLTAGE_COMPATIBLE";
const CLOCK_SOURCE_VALID: &str = "CLOCK_SOURCE_VALID";
const RESET_RELEASE_AFTER_POWER_VALID: &str = "RESET_RELEASE_AFTER_POWER_VALID";
const BOOT_STRAP_DEFINED: &str = "BOOT_STRAP_DEFINED";
const BOOT_STRAP_BIAS_VALID: &str = "BOOT_STRAP_BIAS_VALID";
pub(super) const CONTROL_LINE_RELEASE_SEQUENCE: &str = "CONTROL_LINE_RELEASE_SEQUENCE";
const UART_BOOTLOADER_SYNC: &str = "UART_BOOTLOADER_SYNC";

#[derive(Debug)]
struct ResetRcEvidence {
    pullup_component: String,
    capacitor_component: String,
    reset_release_delay_us: f64,
    reset_release_at_us: f64,
}

#[derive(Debug)]
struct ResetRuntimeEvidence {
    reset_release_delay_us: f64,
    reset_release_at_us: f64,
    source: Option<String>,
}

#[derive(Debug)]
struct BootloaderTimingEvidence {
    power_pin: String,
    power_valid_at_us: f64,
    reset_pin: String,
    reset_release_delay_us: f64,
    reset_release_at_us: f64,
    boot_sample_at_us: f64,
}

#[derive(Debug)]
struct BootModeSuggestion {
    mode_name: String,
    straps: Vec<SuggestedStrap>,
}

pub fn suggest_scenarios(bound: &BoundBoard<'_>) -> ScenarioSuggestionReport {
    let mut suggestions = Vec::new();
    if should_suggest_power_tree(bound.project) {
        suggestions.push(power_tree_suggestion(bound));
    }
    if let Some(suggestion) = io_voltage_suggestion(bound) {
        suggestions.push(suggestion);
    }
    suggestions.extend(backdrive::gpio_backdrive_suggestions(bound));
    suggestions.extend(interface_protection::interface_protection_suggestions(
        bound,
    ));
    suggestions.extend(clock_source_suggestions(bound));
    suggestions.extend(reset_release_suggestions(bound));
    suggestions.extend(boot_strap_suggestions(bound));
    suggestions.extend(control_line::control_line_sequence_suggestions(bound));
    suggestions.extend(uart_bootloader_suggestions(bound));
    suggestions.extend(manufacturing::manufacturing_suggestions(bound));
    ScenarioSuggestionReport {
        schema_version: "0.1.0".to_string(),
        project: bound.project.project.name.clone(),
        suggestions,
    }
}

fn should_suggest_power_tree(project: &BoardProject) -> bool {
    let has_power_net = project
        .board
        .nets
        .values()
        .any(|net| net.kind == NetKind::Power);
    let already_declared = project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "power_tree"
            && scenario
                .checks
                .iter()
                .any(|check| check == POWER_TREE_VALID)
    });
    has_power_net && !already_declared
}

fn power_tree_suggestion(bound: &BoundBoard<'_>) -> ScenarioSuggestion {
    let (pin_states, required_inputs) = load_switch_power_tree_inputs(bound);
    let mut required_inputs = required_inputs;
    required_inputs.extend(battery_charger_power_tree_inputs(bound));
    required_inputs.extend(power_mux_power_tree_inputs(bound));
    let reset_supervisors = reset_supervisor_power_tree_evidence(bound);
    let regulators = regulator_power_tree_evidence(bound);
    let runnable = required_inputs.is_empty();
    ScenarioSuggestion {
        id: "power_tree_valid".to_string(),
        kind: "power_tree".to_string(),
        confidence: "high".to_string(),
        runnable,
        reason: if runnable {
            "Project declares power nets but no POWER_TREE_VALID scenario.".to_string()
        } else {
            "Project declares power nets with switch, charger, or power-mux evidence gaps but no complete POWER_TREE_VALID scenario.".to_string()
        },
        scenario: SuggestedScenario {
            name: format!("{}_power_tree", sanitized_name(&bound.project.project.name)),
            scenario_type: "power_tree".to_string(),
            checks: vec![POWER_TREE_VALID.to_string()],
            parameters: None,
            target: None,
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            control_effects: Vec::new(),
            events: Vec::new(),
            conditioning: None,
            protection_clamps: Vec::new(),
            usb_connectors: Vec::new(),
            usb_routes: Vec::new(),
            usb_route_pairs: Vec::new(),
            clocks: Vec::new(),
            reset_supervisors,
            regulators,
            pin_states,
            paths: Vec::new(),
        },
        required_inputs,
    }
}

fn load_switch_power_tree_inputs(bound: &BoundBoard<'_>) -> (Vec<SuggestedPinState>, Vec<String>) {
    let mut pin_states = Vec::new();
    let mut required_inputs = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let Some(power_switch) = &model.power_switch else {
            continue;
        };
        let Some(output_net_name) = resolve_power_pin_net(component, &power_switch.output_pin)
        else {
            continue;
        };
        let Some(output_net) = bound.project.board.nets.get(output_net_name) else {
            continue;
        };
        if output_net.powered != Some(true) {
            continue;
        }
        let enabled_state = power_switch_state_name(&power_switch.enabled_state);
        let control_state = component
            .pins
            .get(&power_switch.control_pin)
            .and_then(|control_net| {
                inferred_load_switch_control_state(bound.project, control_net, enabled_state)
            });
        pin_states.push(SuggestedPinState {
            component: component_id.clone(),
            pin: power_switch.control_pin.clone(),
            mode: "input".to_string(),
            state: Some(control_state.unwrap_or(enabled_state).to_string()),
        });
        if control_state.is_none() {
            required_inputs.push(format!(
                "Prove {component_id}.{} is {enabled_state} whenever switched rail {output_net_name} is declared powered.",
                power_switch.control_pin
            ));
        }
    }
    (pin_states, required_inputs)
}

pub(super) fn inferred_logic_state_from_net(
    project: &BoardProject,
    net_name: &str,
) -> Option<&'static str> {
    let net = project.board.nets.get(net_name)?;
    match net.kind {
        NetKind::Ground => Some("low"),
        NetKind::Power if net.powered == Some(true) => Some("high"),
        _ => None,
    }
}

fn inferred_load_switch_control_state(
    project: &BoardProject,
    control_net: &str,
    enabled_state: &str,
) -> Option<&'static str> {
    let state = inferred_logic_state_from_net(project, control_net)
        .or_else(|| unique_resistor_bias_logic_state(project, control_net))?;
    if state == enabled_state {
        Some(state)
    } else {
        None
    }
}

fn unique_resistor_bias_logic_state(
    project: &BoardProject,
    net_name: &str,
) -> Option<&'static str> {
    let mut states = Vec::new();
    for component in project.board.components.values() {
        let Some(spice) = &component.spice else {
            continue;
        };
        if spice.primitive != SpicePrimitive::Resistor || finite_positive(spice.value_ohm).is_none()
        {
            continue;
        }
        if !component.pins.values().any(|net| net == net_name) {
            continue;
        }
        let mut component_states = component
            .pins
            .values()
            .filter(|net| *net != net_name)
            .filter_map(|net| inferred_logic_state_from_net(project, net))
            .collect::<Vec<_>>();
        component_states.sort_unstable();
        component_states.dedup();
        if component_states.len() == 1 {
            states.push(component_states[0]);
        }
    }
    if states.len() == 1 {
        Some(states[0])
    } else {
        None
    }
}

fn battery_charger_power_tree_inputs(bound: &BoundBoard<'_>) -> Vec<String> {
    let mut required_inputs = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let Some(charger) = &model.battery_charger else {
            continue;
        };
        let Some(parameter) = charger.charge_current_parameter.as_deref() else {
            continue;
        };
        if component.parameters.contains_key(parameter)
            || derive_charge_current_from_programming_resistor(bound.project, component, charger)
                .is_some()
        {
            continue;
        }
        required_inputs.push(format!(
            "Add components.{component_id}.parameters.{parameter} from the charger PROG resistor or board charge-current configuration."
        ));
    }
    required_inputs
}

fn power_mux_power_tree_inputs(bound: &BoundBoard<'_>) -> Vec<String> {
    let mut required_inputs = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let Some(mux) = &model.power_mux else {
            continue;
        };
        let Some(parameter) = mux.selected_input_parameter.as_deref() else {
            continue;
        };
        if component.parameters.contains_key(parameter)
            || derive_selected_power_mux_input_from_powered_nets(bound.project, component, mux)
                .is_some()
        {
            continue;
        }
        let Some(output_net_name) = resolve_power_pin_net(component, &mux.output_pin) else {
            continue;
        };
        let Some(output_net) = bound.project.board.nets.get(output_net_name) else {
            continue;
        };
        if output_net.powered != Some(true) {
            continue;
        }
        let allowed_inputs = mux
            .inputs
            .iter()
            .map(|input| input.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        required_inputs.push(format!(
            "Add components.{component_id}.parameters.{parameter} with the selected power-mux input for powered output rail {output_net_name}; allowed inputs: {allowed_inputs}."
        ));
    }
    required_inputs
}

fn reset_supervisor_power_tree_evidence(bound: &BoundBoard<'_>) -> Vec<SuggestedResetSupervisor> {
    bound
        .project
        .board
        .components
        .iter()
        .filter_map(|(component_id, component)| {
            let model = bound.library.get(&component.model)?;
            let supervisor = model.reset_supervisor.as_ref()?;
            let monitored_net = resolve_power_pin_net(component, &supervisor.monitored_pin)?;
            let reset_net = component.pins.get(&supervisor.reset_output_pin)?;
            Some(SuggestedResetSupervisor {
                component: component_id.clone(),
                monitored_pin: supervisor.monitored_pin.clone(),
                monitored_net: monitored_net.to_string(),
                reset_output_pin: supervisor.reset_output_pin.clone(),
                reset_net: reset_net.clone(),
                threshold_min_v: supervisor.threshold_min_v,
                threshold_max_v: supervisor.threshold_max_v,
            })
        })
        .collect()
}

fn regulator_power_tree_evidence(bound: &BoundBoard<'_>) -> Vec<SuggestedRegulator> {
    bound
        .project
        .board
        .components
        .iter()
        .filter_map(|(component_id, component)| {
            let model = bound.library.get(&component.model)?;
            let conversion = model.power_conversion.as_ref()?;
            let input_net = resolve_power_pin_net(component, &conversion.input_pin)?;
            let output_net = resolve_power_pin_net(component, &conversion.output_pin)?;
            let input_support = conversion
                .input_capacitance_min_f
                .map(|_| support_capacitance_to_ground(bound, input_net));
            let output_support = conversion
                .output_capacitance_min_f
                .map(|_| support_capacitance_to_ground(bound, output_net));
            let switch_net = conversion
                .switch_pin
                .as_deref()
                .and_then(|pin| component.pins.get(pin).map(String::as_str));
            let output_inductor_support = switch_net.and_then(|net| {
                if conversion.output_inductance_min_h.is_some()
                    || conversion.output_inductance_max_h.is_some()
                {
                    Some(direct_inductance_between_nets(bound, net, output_net))
                } else {
                    None
                }
            });
            let input_inductor_support = switch_net.and_then(|net| {
                if conversion.input_inductance_min_h.is_some()
                    || conversion.input_inductance_max_h.is_some()
                {
                    Some(direct_inductance_between_nets(bound, input_net, net))
                } else {
                    None
                }
            });
            let switch_inductor_net_a = conversion
                .switch_inductor_pin_a
                .as_deref()
                .and_then(|pin| component.pins.get(pin).map(String::as_str));
            let switch_inductor_net_b = conversion
                .switch_inductor_pin_b
                .as_deref()
                .and_then(|pin| component.pins.get(pin).map(String::as_str));
            let switch_inductor_support = match (switch_inductor_net_a, switch_inductor_net_b) {
                (Some(net_a), Some(net_b))
                    if conversion.switch_inductance_min_h.is_some()
                        || conversion.switch_inductance_max_h.is_some() =>
                {
                    Some(direct_inductance_between_nets(bound, net_a, net_b))
                }
                _ => None,
            };
            Some(SuggestedRegulator {
                component: component_id.clone(),
                input_pin: conversion.input_pin.clone(),
                input_net: input_net.to_string(),
                output_pin: conversion.output_pin.clone(),
                output_net: output_net.to_string(),
                dropout_voltage_v: conversion.dropout_voltage_v,
                min_output_current_a: conversion.min_output_current_a,
                max_output_current_a: conversion.max_output_current_a,
                startup_delay_us: conversion.startup_delay_us,
                input_capacitance_min_f: conversion.input_capacitance_min_f,
                output_capacitance_min_f: conversion.output_capacitance_min_f,
                input_support_capacitance_f: input_support
                    .as_ref()
                    .map(|(capacitance_f, _)| *capacitance_f),
                input_support_capacitors: input_support.map(|(_, capacitors)| capacitors),
                output_support_capacitance_f: output_support
                    .as_ref()
                    .map(|(capacitance_f, _)| *capacitance_f),
                output_support_capacitors: output_support.map(|(_, capacitors)| capacitors),
                switch_pin: conversion.switch_pin.clone(),
                switch_net: switch_net.map(str::to_string),
                switch_inductor_pin_a: conversion.switch_inductor_pin_a.clone(),
                switch_inductor_net_a: switch_inductor_net_a.map(str::to_string),
                switch_inductor_pin_b: conversion.switch_inductor_pin_b.clone(),
                switch_inductor_net_b: switch_inductor_net_b.map(str::to_string),
                input_inductance_min_h: conversion.input_inductance_min_h,
                input_inductance_max_h: conversion.input_inductance_max_h,
                input_support_inductance_h: input_inductor_support
                    .as_ref()
                    .map(|(inductance_h, _)| *inductance_h),
                input_support_inductors: input_inductor_support.map(|(_, inductors)| inductors),
                output_inductance_min_h: conversion.output_inductance_min_h,
                output_inductance_max_h: conversion.output_inductance_max_h,
                output_support_inductance_h: output_inductor_support
                    .as_ref()
                    .map(|(inductance_h, _)| *inductance_h),
                output_support_inductors: output_inductor_support.map(|(_, inductors)| inductors),
                switch_inductance_min_h: conversion.switch_inductance_min_h,
                switch_inductance_max_h: conversion.switch_inductance_max_h,
                switch_support_inductance_h: switch_inductor_support
                    .as_ref()
                    .map(|(inductance_h, _)| *inductance_h),
                switch_support_inductors: switch_inductor_support.map(|(_, inductors)| inductors),
            })
        })
        .collect()
}

fn support_capacitance_to_ground(bound: &BoundBoard<'_>, net_name: &str) -> (f64, Vec<String>) {
    let mut total_f = 0.0;
    let mut capacitors = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(spice) = &component.spice else {
            continue;
        };
        if spice.primitive != SpicePrimitive::Capacitor {
            continue;
        }
        let Some(value_f) = spice.value_f else {
            continue;
        };
        if !value_f.is_finite() || value_f <= 0.0 {
            continue;
        }
        if component_connects_net_to_ground(bound, component, net_name) {
            total_f += value_f;
            capacitors.push(component_id.clone());
        }
    }
    (total_f, capacitors)
}

fn direct_inductance_between_nets(
    bound: &BoundBoard<'_>,
    first_net_name: &str,
    second_net_name: &str,
) -> (f64, Vec<String>) {
    let mut total_h = 0.0;
    let mut inductors = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(spice) = &component.spice else {
            continue;
        };
        if spice.primitive != SpicePrimitive::Inductor {
            continue;
        }
        let Some(value_h) = spice.value_h else {
            continue;
        };
        if !value_h.is_finite() || value_h <= 0.0 {
            continue;
        }
        if component_connects_two_nets(component, first_net_name, second_net_name) {
            total_h += value_h;
            inductors.push(component_id.clone());
        }
    }
    (total_h, inductors)
}

fn component_connects_net_to_ground(
    bound: &BoundBoard<'_>,
    component: &ComponentSpec,
    net_name: &str,
) -> bool {
    component.pins.values().any(|net| net == net_name)
        && component.pins.values().any(|net| {
            net != net_name
                && bound
                    .project
                    .board
                    .nets
                    .get(net)
                    .is_some_and(|spec| spec.kind == NetKind::Ground)
        })
}

fn component_connects_two_nets(
    component: &ComponentSpec,
    first_net_name: &str,
    second_net_name: &str,
) -> bool {
    component.pins.values().any(|net| net == first_net_name)
        && component.pins.values().any(|net| net == second_net_name)
}

fn io_voltage_suggestion(bound: &BoundBoard<'_>) -> Option<ScenarioSuggestion> {
    let already_declared = bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "power_tree"
            && scenario
                .checks
                .iter()
                .any(|check| check == IO_VOLTAGE_COMPATIBLE)
    });
    if already_declared {
        return None;
    }
    let paths = io_voltage_paths(bound);
    if paths.is_empty() {
        return None;
    }
    Some(ScenarioSuggestion {
        id: "io_voltage_compatible".to_string(),
        kind: "power_tree".to_string(),
        confidence: "medium".to_string(),
        runnable: true,
        reason: "Project has same-net digital output/input pairs with modeled I/O voltage metadata but no IO_VOLTAGE_COMPATIBLE check.".to_string(),
        scenario: SuggestedScenario {
            name: format!("{}_io_voltage", sanitized_name(&bound.project.project.name)),
            scenario_type: "power_tree".to_string(),
            checks: vec![IO_VOLTAGE_COMPATIBLE.to_string()],
            parameters: None,
            target: None,
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
                control_effects: Vec::new(),
            events: Vec::new(),
            conditioning: None,
            protection_clamps: Vec::new(),
            usb_connectors: Vec::new(),
            usb_routes: Vec::new(),
            usb_route_pairs: Vec::new(),
            clocks: Vec::new(),
            reset_supervisors: Vec::new(),
            regulators: Vec::new(),
            pin_states: Vec::new(),
            paths,
        },
        required_inputs: Vec::new(),
    })
}

fn io_voltage_paths(bound: &BoundBoard<'_>) -> Vec<SuggestedBackdrivePath> {
    let mut paths = Vec::new();
    for (driver_component_id, driver_component) in &bound.project.board.components {
        let Some(driver_model) = bound.library.get(&driver_component.model) else {
            continue;
        };
        for (driver_pin, driver_net) in &driver_component.pins {
            let Some(driver_port) = driver_model.ports.get(driver_pin) else {
                continue;
            };
            if !matches!(
                driver_port.kind,
                PortKind::DigitalElectricalOutput | PortKind::DigitalElectricalIo
            ) || !kicad_pin_type_output_capable(driver_component, driver_pin)
            {
                continue;
            }
            for (victim_component_id, victim_component) in &bound.project.board.components {
                let Some(victim_model) = bound.library.get(&victim_component.model) else {
                    continue;
                };
                for (victim_pin, victim_net) in &victim_component.pins {
                    if victim_net != driver_net
                        || (driver_component_id == victim_component_id && driver_pin == victim_pin)
                    {
                        continue;
                    }
                    let Some(victim_port) = victim_model.ports.get(victim_pin) else {
                        continue;
                    };
                    if !matches!(
                        victim_port.kind,
                        PortKind::DigitalElectricalInput | PortKind::DigitalElectricalIo
                    ) || !kicad_pin_type_input_capable(victim_component, victim_pin)
                    {
                        continue;
                    }
                    if !io_voltage_pair_has_check(driver_port, victim_port) {
                        continue;
                    }
                    paths.push(SuggestedBackdrivePath {
                        driver: SuggestedEndpoint {
                            component: driver_component_id.clone(),
                            pin: driver_pin.clone(),
                        },
                        victim: SuggestedEndpoint {
                            component: victim_component_id.clone(),
                            pin: victim_pin.clone(),
                        },
                        net: Some(driver_net.clone()),
                        series_resistance_ohm: 0.0,
                    });
                }
            }
        }
    }
    paths
}

fn io_voltage_pair_has_check(
    driver_port: &crate::library::Port,
    victim_port: &crate::library::Port,
) -> bool {
    if driver_port.electrical.drive_high_voltage_v.is_none() {
        return false;
    }
    victim_port.electrical.vih_min_v.is_some()
        || (driver_port.electrical.source_impedance_ohm.is_some()
            && victim_port.electrical.injection_current_limit_a.is_some())
}

fn clock_source_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_clock_checks(bound.project);
    let mut suggestions = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        if existing.contains_key(component_id) {
            continue;
        }
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        if model.clock_sources.is_empty() {
            continue;
        }
        let clocks = model
            .clock_sources
            .iter()
            .filter_map(|clock| {
                suggested_clock_source(bound, component_id, component, model, clock)
            })
            .collect::<Vec<_>>();
        if clocks.is_empty() {
            continue;
        }
        let required_inputs = clocks
            .iter()
            .filter(|clock| clock.crystal_component.is_none())
            .map(|clock| {
                format!(
                    "Connect or model a crystal/resonator between {}.{} net {} and {}.{} net {} before relying on this clock-source check.",
                    clock.component,
                    clock.input_pin,
                    clock.input_net,
                    clock.component,
                    clock.output_pin,
                    clock.output_net
                )
            })
            .collect::<Vec<_>>();
        let runnable = true;
        suggestions.push(ScenarioSuggestion {
            id: format!("clock_source_valid_{}", sanitized_name(component_id)),
            kind: "clock".to_string(),
            confidence: if required_inputs.is_empty() {
                "medium"
            } else {
                "low"
            }
            .to_string(),
            runnable,
            reason: format!(
                "Component {component_id} model declares external clock source metadata, but no CLOCK_SOURCE_VALID scenario covers it."
            ),
            scenario: SuggestedScenario {
                name: format!("{}_clock_source", sanitized_name(component_id)),
                scenario_type: "clock".to_string(),
                checks: vec![CLOCK_SOURCE_VALID.to_string()],
                parameters: None,
                target: Some(SuggestedTarget {
                    component: component_id.clone(),
                    power_pin: None,
                    reset_pin: None,
                }),
                timing: None,
                required_boot_mode: None,
                straps: Vec::new(),
                bootloader: None,
                control_effects: Vec::new(),
                events: Vec::new(),
                conditioning: None,
                protection_clamps: Vec::new(),
                usb_connectors: Vec::new(),
                usb_routes: Vec::new(),
                usb_route_pairs: Vec::new(),
                clocks,
                reset_supervisors: Vec::new(),
                regulators: Vec::new(),
                pin_states: Vec::new(),
                paths: Vec::new(),
            },
            required_inputs,
        });
    }
    suggestions
}

fn suggested_clock_source(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    clock: &crate::library::ClockSource,
) -> Option<SuggestedClockSource> {
    if !model.ports.contains_key(&clock.input_pin) || !model.ports.contains_key(&clock.output_pin) {
        return None;
    }
    let input_net = component.pins.get(&clock.input_pin)?;
    let output_net = component.pins.get(&clock.output_pin)?;
    if input_net == output_net {
        return None;
    }
    Some(SuggestedClockSource {
        component: component_id.to_string(),
        name: clock.name.clone(),
        input_pin: clock.input_pin.clone(),
        input_net: input_net.clone(),
        output_pin: clock.output_pin.clone(),
        output_net: output_net.clone(),
        crystal_component: find_crystal_between_nets(bound, input_net, output_net),
    })
}

fn find_crystal_between_nets(bound: &BoundBoard<'_>, net_a: &str, net_b: &str) -> Option<String> {
    bound
        .project
        .board
        .components
        .iter()
        .find_map(|(component_id, component)| {
            let model = bound.library.get(&component.model)?;
            model.crystal.as_ref()?;
            if component_connects_nets(component, net_a, net_b) {
                Some(component_id.clone())
            } else {
                None
            }
        })
}

fn reset_release_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_reset_checks(bound.project);
    let mut suggestions = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        if existing.contains_key(component_id) {
            continue;
        }
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let Some(reset) = &model.behavior.reset else {
            continue;
        };
        if !component.pins.contains_key(&reset.pin) {
            continue;
        }
        let Some((power_pin, power_net, power_valid_at_us)) =
            model.ports.iter().find_map(|(pin_name, port)| {
                if port.kind != PortKind::ElectricalPower {
                    return None;
                }
                let net_name = component
                    .power_domains
                    .get(pin_name)
                    .or_else(|| component.pins.get(pin_name))
                    .or(component.power_domain.as_ref())?;
                let net = bound.project.board.nets.get(net_name)?;
                let power_valid_at_us = net.power_valid_at_us?;
                if power_valid_at_us.is_finite() && power_valid_at_us >= 0.0 {
                    Some((pin_name.clone(), net_name.clone(), power_valid_at_us))
                } else {
                    None
                }
            })
        else {
            continue;
        };
        let reset_net = component.pins.get(&reset.pin);
        let rc_evidence = reset_net.and_then(|net| {
            reset_rc_evidence(
                bound,
                component,
                model,
                &reset.pin,
                net,
                &power_net,
                power_valid_at_us,
            )
        });
        let runtime_evidence =
            runtime_reset_release_evidence(bound.project, component_id, &reset.pin, &power_pin);
        let reset_release_at_us = rc_evidence
            .as_ref()
            .map(|evidence| evidence.reset_release_at_us)
            .or_else(|| {
                runtime_evidence
                    .as_ref()
                    .map(|evidence| evidence.reset_release_at_us)
            });
        let reset_release_delay_us = rc_evidence
            .as_ref()
            .map(|evidence| evidence.reset_release_delay_us)
            .or_else(|| {
                runtime_evidence
                    .as_ref()
                    .map(|evidence| evidence.reset_release_delay_us)
            })
            .unwrap_or(0.0);
        let boot_sample_at_us = model
            .behavior
            .boot
            .as_ref()
            .and_then(|boot| boot.sample_time_after_reset_release_us)
            .map(|delay_us| reset_release_at_us.unwrap_or(power_valid_at_us) + delay_us);
        let (runnable, reason, required_inputs) = if let Some(evidence) = &rc_evidence {
            (
                true,
                format!(
                    "Component {component_id} has active-low reset behavior, target rail power_valid_at_us, and explicit RC reset evidence from {} and {}.",
                    evidence.pullup_component, evidence.capacitor_component
                ),
                Vec::new(),
            )
        } else if let Some(evidence) = &runtime_evidence {
            let source = evidence
                .source
                .as_deref()
                .map(|source| format!(" from {source}"))
                .unwrap_or_default();
            (
                true,
                format!(
                    "Component {component_id} has reset behavior, target rail power_valid_at_us, and explicit runtime reset-release timing{source}."
                ),
                Vec::new(),
            )
        } else {
            (
                false,
                format!(
                    "Component {component_id} has reset behavior and target rail power_valid_at_us, but no RESET_RELEASE_AFTER_POWER_VALID scenario."
                ),
                vec![
                    "Fill timing.reset_release_at_us from reset supervisor, RC, control-line, or analog waveform evidence before validation.".to_string(),
                    "Keep timing.power_valid_at_us equal to the target rail power_valid_at_us or remove duplicated stale timing.".to_string(),
                ],
            )
        };
        suggestions.push(ScenarioSuggestion {
            id: format!(
                "reset_release_after_power_valid_{}",
                sanitized_name(component_id)
            ),
            kind: "reset_boot".to_string(),
            confidence: "medium".to_string(),
            runnable,
            reason,
            scenario: SuggestedScenario {
                name: format!("{}_reset_release_after_power", sanitized_name(component_id)),
                scenario_type: "reset_boot".to_string(),
                checks: vec![RESET_RELEASE_AFTER_POWER_VALID.to_string()],
                parameters: None,
                target: Some(SuggestedTarget {
                    component: component_id.clone(),
                    power_pin: Some(power_pin),
                    reset_pin: Some(reset.pin.clone()),
                }),
                timing: Some(SuggestedTiming {
                    power_valid_at_us,
                    reset_release_delay_us: Some(reset_release_delay_us),
                    reset_release_at_us,
                    boot_sample_at_us,
                }),
                required_boot_mode: None,
                straps: Vec::new(),
                bootloader: None,
                control_effects: Vec::new(),
                events: Vec::new(),
                conditioning: None,
                protection_clamps: Vec::new(),
                usb_connectors: Vec::new(),
                usb_routes: Vec::new(),
                usb_route_pairs: Vec::new(),
                clocks: Vec::new(),
                reset_supervisors: Vec::new(),
                regulators: Vec::new(),
                pin_states: Vec::new(),
                paths: Vec::new(),
            },
            required_inputs,
        });
    }
    suggestions
}

fn reset_rc_evidence(
    bound: &BoundBoard<'_>,
    target_component: &ComponentSpec,
    target_model: &ComponentModel,
    reset_pin: &str,
    reset_net: &str,
    power_net: &str,
    power_valid_at_us: f64,
) -> Option<ResetRcEvidence> {
    let reset = target_model.behavior.reset.as_ref()?;
    if !reset.active.trim().eq_ignore_ascii_case("low") {
        return None;
    }
    let reset_port = target_model.ports.get(reset_pin)?;
    let vih_min_v = finite_positive(reset_port.electrical.vih_min_v)?;
    let rail_voltage_v = finite_positive(bound.project.board.nets.get(power_net)?.nominal_voltage)?;
    if vih_min_v >= rail_voltage_v {
        return None;
    }
    if !target_component.pins.values().any(|net| net == reset_net) {
        return None;
    }

    let pullups: Vec<(String, f64)> = bound
        .project
        .board
        .components
        .iter()
        .filter_map(|(component_id, component)| {
            let spice = component.spice.as_ref()?;
            if spice.primitive != SpicePrimitive::Resistor {
                return None;
            }
            let value_ohm = finite_positive(spice.value_ohm)?;
            if component_connects_nets(component, reset_net, power_net) {
                Some((component_id.clone(), value_ohm))
            } else {
                None
            }
        })
        .collect();
    if pullups.len() != 1 {
        return None;
    }

    let capacitors: Vec<(String, f64)> = bound
        .project
        .board
        .components
        .iter()
        .filter_map(|(component_id, component)| {
            let spice = component.spice.as_ref()?;
            if spice.primitive != SpicePrimitive::Capacitor {
                return None;
            }
            let value_f = finite_positive(spice.value_f)?;
            if component_connects_reset_to_ground(bound.project, component, reset_net) {
                Some((component_id.clone(), value_f))
            } else {
                None
            }
        })
        .collect();
    if capacitors.len() != 1 {
        return None;
    }

    let (pullup_component, resistance_ohm) = &pullups[0];
    let (capacitor_component, capacitance_f) = &capacitors[0];
    let release_ratio = 1.0 - (vih_min_v / rail_voltage_v);
    if !(0.0..1.0).contains(&release_ratio) {
        return None;
    }
    let reset_release_delay_us = -resistance_ohm * capacitance_f * release_ratio.ln() * 1_000_000.0;
    if !reset_release_delay_us.is_finite() || reset_release_delay_us < 0.0 {
        return None;
    }
    let reset_release_at_us = power_valid_at_us + reset_release_delay_us;
    if !reset_release_at_us.is_finite() {
        return None;
    }

    Some(ResetRcEvidence {
        pullup_component: pullup_component.clone(),
        capacitor_component: capacitor_component.clone(),
        reset_release_delay_us,
        reset_release_at_us,
    })
}

fn runtime_reset_release_evidence(
    project: &BoardProject,
    component_id: &str,
    reset_pin: &str,
    power_pin: &str,
) -> Option<ResetRuntimeEvidence> {
    let matches: Vec<ResetRuntimeEvidence> = project
        .board
        .runtime
        .reset_release
        .iter()
        .filter(|evidence| evidence.component == component_id && evidence.reset_pin == reset_pin)
        .filter(|evidence| {
            evidence
                .power_pin
                .as_deref()
                .is_none_or(|evidence_power_pin| evidence_power_pin == power_pin)
        })
        .filter_map(|evidence| {
            if !evidence.reset_release_at_us.is_finite() || evidence.reset_release_at_us < 0.0 {
                return None;
            }
            let reset_release_delay_us = evidence.reset_release_delay_us.unwrap_or(0.0);
            if !reset_release_delay_us.is_finite() || reset_release_delay_us < 0.0 {
                return None;
            }
            Some(ResetRuntimeEvidence {
                reset_release_delay_us,
                reset_release_at_us: evidence.reset_release_at_us,
                source: evidence.source.clone(),
            })
        })
        .collect();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn finite_positive(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn component_connects_nets(component: &ComponentSpec, net_a: &str, net_b: &str) -> bool {
    component.pins.values().any(|net| net == net_a)
        && component.pins.values().any(|net| net == net_b)
}

fn component_connects_reset_to_ground(
    project: &BoardProject,
    component: &ComponentSpec,
    reset_net: &str,
) -> bool {
    component.pins.values().any(|net| net == reset_net)
        && component.pins.values().any(|net| {
            net != reset_net
                && project
                    .board
                    .nets
                    .get(net)
                    .is_some_and(|spec| spec.kind == NetKind::Ground)
        })
}

fn boot_strap_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_boot_strap_checks(bound.project);
    let existing_bias = existing_boot_strap_bias_checks(bound.project);
    let mut suggestions = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let Some(boot) = &model.behavior.boot else {
            continue;
        };
        for (mode_name, mode) in &boot.modes {
            let mut straps = Vec::new();
            let mut missing_pins = Vec::new();
            let mut all_straps_have_bias = true;
            let mut all_defined_straps_match_direct_state = true;
            for requirement in &mode.straps {
                match component.pins.get(&requirement.pin) {
                    Some(net) => {
                        if !strap_net_has_bias(bound.project, net) {
                            all_straps_have_bias = false;
                        }
                        let required_state = requirement.required_state.trim().to_ascii_lowercase();
                        let actual = direct_net_logic_state(bound.project, net);
                        if actual.as_deref() != Some(required_state.as_str()) {
                            all_defined_straps_match_direct_state = false;
                        }
                        straps.push(SuggestedStrap {
                            component: component_id.clone(),
                            pin: requirement.pin.clone(),
                            net: Some(net.clone()),
                            actual,
                        });
                    }
                    None => {
                        missing_pins.push(requirement.pin.clone());
                        all_defined_straps_match_direct_state = false;
                    }
                }
            }
            if straps.is_empty() {
                continue;
            }
            if missing_pins.is_empty()
                && all_straps_have_bias
                && !existing_bias.contains_key(&(component_id.clone(), mode_name.clone()))
            {
                suggestions.push(ScenarioSuggestion {
                    id: format!(
                        "boot_strap_bias_valid_{}_{}",
                        sanitized_name(component_id),
                        sanitized_name(mode_name)
                    ),
                    kind: "reset_boot".to_string(),
                    confidence: "medium".to_string(),
                    runnable: true,
                    reason: format!(
                        "Component {component_id} boot mode {mode_name} has explicit resistor bias evidence but no BOOT_STRAP_BIAS_VALID scenario covers it."
                    ),
                    scenario: SuggestedScenario {
                        name: format!(
                            "{}_boot_strap_bias_{}",
                            sanitized_name(component_id),
                            sanitized_name(mode_name)
                        ),
                        scenario_type: "reset_boot".to_string(),
                        checks: vec![BOOT_STRAP_BIAS_VALID.to_string()],
                        parameters: None,
                        target: Some(SuggestedTarget {
                            component: component_id.clone(),
                            power_pin: None,
                            reset_pin: None,
                        }),
                        timing: None,
                        required_boot_mode: Some(mode_name.clone()),
                        straps: Vec::new(),
                        bootloader: None,
                control_effects: Vec::new(),
                        events: Vec::new(),
                        conditioning: None,
                        protection_clamps: Vec::new(),
                        usb_connectors: Vec::new(),
                        usb_routes: Vec::new(),
                        usb_route_pairs: Vec::new(),
                        clocks: Vec::new(),
                        reset_supervisors: Vec::new(),
                        regulators: Vec::new(),
                        pin_states: Vec::new(),
                        paths: Vec::new(),
                    },
                    required_inputs: Vec::new(),
                });
            }
            if existing.contains_key(&(component_id.clone(), mode_name.clone())) {
                continue;
            }
            let runnable = missing_pins.is_empty() && all_defined_straps_match_direct_state;
            let mut required_inputs = if runnable {
                Vec::new()
            } else {
                vec![format!(
                    "Fill strap actual states for boot mode {mode_name}: {}.",
                    mode.straps
                        .iter()
                        .map(|strap| format!(
                            "{}.{}={}",
                            component_id, strap.pin, strap.required_state
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                )]
            };
            if !missing_pins.is_empty() {
                required_inputs.push(format!(
                    "Connect missing boot strap pins before this template can validate: {}.",
                    missing_pins.join(", ")
                ));
            }
            suggestions.push(ScenarioSuggestion {
                id: format!(
                    "boot_strap_defined_{}_{}",
                    sanitized_name(component_id),
                    sanitized_name(mode_name)
                ),
                kind: "reset_boot".to_string(),
                confidence: if runnable { "high" } else { "medium" }.to_string(),
                runnable,
                reason: format!(
                    "Component {component_id} model declares boot mode {mode_name}, but no BOOT_STRAP_DEFINED scenario covers it."
                ),
                scenario: SuggestedScenario {
                    name: format!(
                        "{}_boot_straps_{}",
                        sanitized_name(component_id),
                        sanitized_name(mode_name)
                    ),
                    scenario_type: "reset_boot".to_string(),
                    checks: vec![BOOT_STRAP_DEFINED.to_string()],
                    parameters: None,
                    target: Some(SuggestedTarget {
                        component: component_id.clone(),
                        power_pin: None,
                        reset_pin: None,
                    }),
                    timing: None,
                    required_boot_mode: Some(mode_name.clone()),
                    straps,
                    bootloader: None,
                control_effects: Vec::new(),
                    events: Vec::new(),
                    conditioning: None,
                    protection_clamps: Vec::new(),
                    usb_connectors: Vec::new(),
                    usb_routes: Vec::new(),
                    usb_route_pairs: Vec::new(),
                    clocks: Vec::new(),
                    reset_supervisors: Vec::new(),
                    regulators: Vec::new(),
                    pin_states: Vec::new(),
                    paths: Vec::new(),
                },
                required_inputs,
            });
        }
    }
    suggestions
}

fn uart_bootloader_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_uart_checks(bound.project);
    let mut suggestions = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        if existing.contains_key(component_id) {
            continue;
        }
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let Some(bootloader) = &model.behavior.bootloader else {
            continue;
        };
        for (interface_name, interface) in &bootloader.interfaces {
            let Some(rx_net) = component.pins.get(&interface.rx_pin) else {
                continue;
            };
            let sender = find_output_sender(bound, component_id, rx_net);
            let timing = uart_bootloader_timing_evidence(bound, component_id, component, model);
            let boot_mode = direct_boot_mode_suggestion(bound, component_id, component, model);
            let boot_modes_required = model
                .behavior
                .boot
                .as_ref()
                .is_some_and(|boot| !boot.modes.is_empty());
            let mut required_inputs = Vec::new();
            if sender.is_none() {
                required_inputs.push(format!(
                    "Connect an output-capable sender pin to {}.{} for interface {interface_name}.",
                    component_id, interface.rx_pin
                ));
            }
            if timing.is_none() {
                required_inputs.push(
                    "Provide reset release and boot strap sampling timing from explicit RC, supervisor, control-line, or waveform evidence.".to_string(),
                );
            }
            if boot_modes_required && boot_mode.is_none() {
                required_inputs.push(
                    "Provide observed boot strap states or direct rail/ground strap evidence proving exactly one boot mode.".to_string(),
                );
            }
            let runnable = sender.is_some()
                && timing.is_some()
                && (!boot_modes_required || boot_mode.is_some());
            suggestions.push(ScenarioSuggestion {
                id: format!(
                    "uart_bootloader_sync_{}_{}",
                    sanitized_name(component_id),
                    sanitized_name(interface_name)
                ),
                kind: "serial_programming".to_string(),
                confidence: if runnable {
                    "high"
                } else if sender.is_some() {
                    "medium"
                } else {
                    "low"
                }
                .to_string(),
                runnable,
                reason: format!(
                    "Component {component_id} model declares bootloader interface {interface_name}, but no UART_BOOTLOADER_SYNC scenario covers it."
                ),
                scenario: SuggestedScenario {
                    name: format!(
                        "{}_{}_bootloader_sync",
                        sanitized_name(component_id),
                        sanitized_name(interface_name)
                    ),
                    scenario_type: "serial_programming".to_string(),
                    checks: vec![UART_BOOTLOADER_SYNC.to_string()],
                    parameters: None,
                    target: Some(SuggestedTarget {
                        component: component_id.clone(),
                        power_pin: timing.as_ref().map(|evidence| evidence.power_pin.clone()),
                        reset_pin: timing.as_ref().map(|evidence| evidence.reset_pin.clone()),
                    }),
                    timing: timing.as_ref().map(|evidence| SuggestedTiming {
                        power_valid_at_us: evidence.power_valid_at_us,
                        reset_release_delay_us: Some(evidence.reset_release_delay_us),
                        reset_release_at_us: Some(evidence.reset_release_at_us),
                        boot_sample_at_us: Some(evidence.boot_sample_at_us),
                    }),
                    required_boot_mode: boot_mode
                        .as_ref()
                        .map(|evidence| evidence.mode_name.clone()),
                    straps: boot_mode
                        .as_ref()
                        .map(|evidence| evidence.straps.clone())
                        .unwrap_or_default(),
                    bootloader: Some(SuggestedBootloader {
                        component: component_id.clone(),
                        interface: interface_name.to_string(),
                        sync_byte: interface.sync_byte,
                        expected_response: interface.ack_byte,
                    }),
                    control_effects: Vec::new(),
                    events: vec![SuggestedEvent {
                        at_us: timing.as_ref().map(|evidence| evidence.boot_sample_at_us),
                        action: "uart_send".to_string(),
                        from: sender,
                        to: Some(SuggestedEndpoint {
                            component: component_id.clone(),
                            pin: interface.rx_pin.clone(),
                        }),
                        bytes: vec![interface.sync_byte],
                        line: None,
                        asserted: None,
                    }],
                    conditioning: None,
                    protection_clamps: Vec::new(),
                    usb_connectors: Vec::new(),
                    usb_routes: Vec::new(),
                    usb_route_pairs: Vec::new(),
                    clocks: Vec::new(),
                    reset_supervisors: Vec::new(),
                    regulators: Vec::new(),
                    pin_states: Vec::new(),
                    paths: Vec::new(),
                },
                required_inputs,
            });
        }
    }
    suggestions
}

fn existing_clock_checks(project: &BoardProject) -> BTreeMap<String, ()> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "clock"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == CLOCK_SOURCE_VALID)
        })
        .filter_map(|scenario| {
            scenario
                .target
                .as_ref()
                .map(|target| (target.component.clone(), ()))
        })
        .collect()
}

fn existing_reset_checks(project: &BoardProject) -> BTreeMap<String, ()> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "reset_boot"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == RESET_RELEASE_AFTER_POWER_VALID)
        })
        .filter_map(|scenario| {
            scenario
                .target
                .as_ref()
                .map(|target| (target.component.clone(), ()))
        })
        .collect()
}

fn existing_boot_strap_checks(project: &BoardProject) -> BTreeMap<(String, String), ()> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "reset_boot"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == BOOT_STRAP_DEFINED)
        })
        .filter_map(|scenario| {
            Some((
                (
                    scenario.target.as_ref()?.component.clone(),
                    scenario.required_boot_mode.clone()?,
                ),
                (),
            ))
        })
        .collect()
}

fn existing_boot_strap_bias_checks(project: &BoardProject) -> BTreeMap<(String, String), ()> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "reset_boot"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == BOOT_STRAP_BIAS_VALID)
        })
        .filter_map(|scenario| {
            Some((
                (
                    scenario.target.as_ref()?.component.clone(),
                    scenario.required_boot_mode.clone()?,
                ),
                (),
            ))
        })
        .collect()
}

fn strap_net_has_bias(project: &BoardProject, strap_net: &str) -> bool {
    project.board.components.values().any(|component| {
        let Some(spice) = &component.spice else {
            return false;
        };
        if spice.primitive != crate::board_ir::SpicePrimitive::Resistor
            || !spice
                .value_ohm
                .is_some_and(|value| value.is_finite() && value > 0.0)
            || !component.pins.values().any(|net| net == strap_net)
        {
            return false;
        }
        component.pins.values().any(|net| {
            net != strap_net
                && project
                    .board
                    .nets
                    .get(net)
                    .is_some_and(|spec| matches!(spec.kind, NetKind::Power | NetKind::Ground))
        })
    })
}

fn existing_uart_checks(project: &BoardProject) -> BTreeMap<String, ()> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "serial_programming"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == UART_BOOTLOADER_SYNC)
        })
        .filter_map(|scenario| {
            scenario
                .target
                .as_ref()
                .map(|target| (target.component.clone(), ()))
        })
        .collect()
}

fn find_output_sender(
    bound: &BoundBoard<'_>,
    target_component: &str,
    target_rx_net: &str,
) -> Option<SuggestedEndpoint> {
    for (component_id, component) in &bound.project.board.components {
        if component_id == target_component {
            continue;
        }
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        if !model.signal_conditioning.channels.is_empty() {
            continue;
        }
        for (pin_name, net_name) in &component.pins {
            if net_name != target_rx_net {
                continue;
            }
            let Some(port) = model.ports.get(pin_name) else {
                continue;
            };
            if !matches!(
                port.kind,
                PortKind::DigitalElectricalOutput | PortKind::DigitalElectricalIo
            ) {
                continue;
            }
            if !kicad_pin_type_output_capable(component, pin_name) {
                continue;
            }
            return Some(SuggestedEndpoint {
                component: component_id.clone(),
                pin: pin_name.clone(),
            });
        }
    }
    None
}

fn uart_bootloader_timing_evidence(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<BootloaderTimingEvidence> {
    let reset = model.behavior.reset.as_ref()?;
    let (power_pin, power_net, power_valid_at_us) =
        model.ports.iter().find_map(|(pin_name, port)| {
            if port.kind != PortKind::ElectricalPower {
                return None;
            }
            let net_name = resolve_power_pin_net(component, pin_name)?;
            let net = bound.project.board.nets.get(net_name)?;
            let power_valid_at_us = net.power_valid_at_us?;
            if power_valid_at_us.is_finite() && power_valid_at_us >= 0.0 {
                Some((pin_name.clone(), net_name.to_string(), power_valid_at_us))
            } else {
                None
            }
        })?;
    let reset_net = component.pins.get(&reset.pin)?;
    let rc_evidence = reset_rc_evidence(
        bound,
        component,
        model,
        &reset.pin,
        reset_net,
        &power_net,
        power_valid_at_us,
    );
    let runtime_evidence =
        runtime_reset_release_evidence(bound.project, component_id, &reset.pin, &power_pin);
    let reset_release_at_us = rc_evidence
        .as_ref()
        .map(|evidence| evidence.reset_release_at_us)
        .or_else(|| {
            runtime_evidence
                .as_ref()
                .map(|evidence| evidence.reset_release_at_us)
        })?;
    let reset_release_delay_us = rc_evidence
        .as_ref()
        .map(|evidence| evidence.reset_release_delay_us)
        .or_else(|| {
            runtime_evidence
                .as_ref()
                .map(|evidence| evidence.reset_release_delay_us)
        })
        .unwrap_or(0.0);
    let boot_sample_delay_us = model
        .behavior
        .boot
        .as_ref()
        .and_then(|boot| boot.sample_time_after_reset_release_us)?;
    if !boot_sample_delay_us.is_finite() || boot_sample_delay_us < 0.0 {
        return None;
    }
    let boot_sample_at_us = reset_release_at_us + boot_sample_delay_us;
    if !boot_sample_at_us.is_finite() {
        return None;
    }
    Some(BootloaderTimingEvidence {
        power_pin,
        power_valid_at_us,
        reset_pin: reset.pin.clone(),
        reset_release_delay_us,
        reset_release_at_us,
        boot_sample_at_us,
    })
}

fn direct_boot_mode_suggestion(
    bound: &BoundBoard<'_>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Option<BootModeSuggestion> {
    let boot = model.behavior.boot.as_ref()?;
    if boot.modes.is_empty() {
        return None;
    }
    let mut matches = Vec::new();
    for (mode_name, mode) in &boot.modes {
        let mut straps = Vec::new();
        let mut mode_matches = true;
        for requirement in &mode.straps {
            let Some(net_name) = component.pins.get(&requirement.pin) else {
                mode_matches = false;
                break;
            };
            let Some(actual) = direct_net_logic_state(bound.project, net_name) else {
                mode_matches = false;
                break;
            };
            if actual != requirement.required_state.trim().to_ascii_lowercase() {
                mode_matches = false;
                break;
            }
            straps.push(SuggestedStrap {
                component: component_id.to_string(),
                pin: requirement.pin.clone(),
                net: Some(net_name.clone()),
                actual: Some(actual),
            });
        }
        if mode_matches && !straps.is_empty() {
            matches.push(BootModeSuggestion {
                mode_name: mode_name.clone(),
                straps,
            });
        }
    }
    if matches.len() == 1 {
        matches.pop()
    } else {
        None
    }
}

fn direct_net_logic_state(project: &BoardProject, net_name: &str) -> Option<String> {
    let net = project.board.nets.get(net_name)?;
    match net.kind {
        NetKind::Power if net.powered == Some(true) => Some("high".to_string()),
        NetKind::Ground => Some("low".to_string()),
        _ => None,
    }
}

pub(super) fn component_power_state(
    bound: &BoundBoard<'_>,
    component_id: &str,
    model: &crate::library::ComponentModel,
) -> Option<bool> {
    let component = bound.project.board.components.get(component_id)?;
    let power_port = model
        .ports
        .iter()
        .find(|(_, port)| port.kind == PortKind::ElectricalPower)
        .map(|(name, _)| name)?;
    let net_name = component
        .power_domains
        .get(power_port)
        .or_else(|| component.pins.get(power_port))
        .or(component.power_domain.as_ref())?;
    bound.project.board.nets.get(net_name)?.powered
}

fn resolve_power_pin_net<'a>(component: &'a ComponentSpec, pin_name: &str) -> Option<&'a str> {
    component
        .power_domains
        .get(pin_name)
        .or_else(|| component.pins.get(pin_name))
        .or(component.power_domain.as_ref())
        .map(String::as_str)
}

fn power_switch_state_name(state: &PowerSwitchState) -> &'static str {
    match state {
        PowerSwitchState::High => "high",
        PowerSwitchState::Low => "low",
    }
}

pub(super) fn kicad_pin_type_output_capable(
    component: &crate::board_ir::ComponentSpec,
    pin_name: &str,
) -> bool {
    let Some(electrical_type) = component
        .source
        .as_ref()
        .and_then(|source| source.board_pin_electrical_types.get(pin_name))
    else {
        return true;
    };
    matches!(
        electrical_type
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '-'], "_")
            .as_str(),
        "output" | "bidirectional" | "tri_state" | "power_out" | "open_collector" | "open_emitter"
    )
}

pub(super) fn kicad_pin_type_input_capable(
    component: &crate::board_ir::ComponentSpec,
    pin_name: &str,
) -> bool {
    let Some(electrical_type) = component
        .source
        .as_ref()
        .and_then(|source| source.board_pin_electrical_types.get(pin_name))
    else {
        return true;
    };
    matches!(
        electrical_type
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '-'], "_")
            .as_str(),
        "input" | "bidirectional" | "tri_state"
    )
}

pub(super) fn sanitized_name(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            out.push('_');
            last_was_separator = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "scenario".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::sanitized_name;

    #[test]
    fn sanitizes_scenario_names() {
        assert_eq!(sanitized_name("UM STM32L4"), "um_stm32l4");
        assert_eq!(sanitized_name("U1"), "u1");
        assert_eq!(sanitized_name("!!!"), "scenario");
    }
}
