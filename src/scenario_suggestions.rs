use crate::board_ir::{BoardProject, ComponentSpec, NetKind, SpicePrimitive};
use crate::library::{
    BoundBoard, ComponentModel, PortKind, PowerSwitchState, SignalConditioningChannel,
    SignalConditioningKind,
};
use serde::Serialize;
use std::collections::BTreeMap;

const POWER_TREE_VALID: &str = "POWER_TREE_VALID";
const GPIO_BACKDRIVE: &str = "GPIO_BACKDRIVE";
const INTERFACE_PROTECTION_REVIEW: &str = "INTERFACE_PROTECTION_REVIEW";
const IO_VOLTAGE_COMPATIBLE: &str = "IO_VOLTAGE_COMPATIBLE";
const RESET_RELEASE_AFTER_POWER_VALID: &str = "RESET_RELEASE_AFTER_POWER_VALID";
const BOOT_STRAP_DEFINED: &str = "BOOT_STRAP_DEFINED";
const BOOT_STRAP_BIAS_VALID: &str = "BOOT_STRAP_BIAS_VALID";
const UART_BOOTLOADER_SYNC: &str = "UART_BOOTLOADER_SYNC";

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

#[derive(Debug)]
struct ResetRcEvidence {
    pullup_component: String,
    capacitor_component: String,
    reset_release_delay_us: f64,
    reset_release_at_us: f64,
}

pub fn suggest_scenarios(bound: &BoundBoard<'_>) -> ScenarioSuggestionReport {
    let mut suggestions = Vec::new();
    if should_suggest_power_tree(bound.project) {
        suggestions.push(power_tree_suggestion(bound));
    }
    if let Some(suggestion) = io_voltage_suggestion(bound) {
        suggestions.push(suggestion);
    }
    suggestions.extend(gpio_backdrive_suggestions(bound));
    suggestions.extend(interface_protection_suggestions(bound));
    suggestions.extend(reset_release_suggestions(bound));
    suggestions.extend(boot_strap_suggestions(bound));
    suggestions.extend(uart_bootloader_suggestions(bound));
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
            target: None,
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
            conditioning: None,
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
        pin_states.push(SuggestedPinState {
            component: component_id.clone(),
            pin: power_switch.control_pin.clone(),
            mode: "input".to_string(),
            state: Some(enabled_state.to_string()),
        });
        required_inputs.push(format!(
            "Prove {component_id}.{} is {enabled_state} whenever switched rail {output_net_name} is declared powered.",
            power_switch.control_pin
        ));
    }
    (pin_states, required_inputs)
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
        if component.parameters.contains_key(parameter) {
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
        if component.parameters.contains_key(parameter) {
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
            target: None,
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
            conditioning: None,
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

fn gpio_backdrive_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_backdrive_paths(bound.project);
    let mut suggestions = Vec::new();
    for (driver_component_id, driver_component) in &bound.project.board.components {
        let Some(driver_model) = bound.library.get(&driver_component.model) else {
            continue;
        };
        if component_power_state(bound, driver_component_id, driver_model) != Some(true) {
            continue;
        }
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
            if driver_port.electrical.drive_high_voltage_v.is_none()
                || driver_port.electrical.source_impedance_ohm.is_none()
            {
                continue;
            }
            for (victim_component_id, victim_component) in &bound.project.board.components {
                if victim_component_id == driver_component_id {
                    continue;
                }
                let Some(victim_model) = bound.library.get(&victim_component.model) else {
                    continue;
                };
                if component_power_state(bound, victim_component_id, victim_model) != Some(false) {
                    continue;
                }
                for (victim_pin, victim_net) in &victim_component.pins {
                    if victim_net != driver_net {
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
                    if victim_port.electrical.injection_current_limit_a.is_none() {
                        continue;
                    }
                    let key = (
                        driver_component_id.clone(),
                        driver_pin.clone(),
                        victim_component_id.clone(),
                        victim_pin.clone(),
                    );
                    if existing.contains_key(&key) {
                        continue;
                    }
                    suggestions.push(backdrive_suggestion(
                        driver_component_id,
                        driver_pin,
                        victim_component_id,
                        victim_pin,
                        driver_net,
                    ));
                }
            }
        }
    }
    suggestions
}

fn backdrive_suggestion(
    driver_component: &str,
    driver_pin: &str,
    victim_component: &str,
    victim_pin: &str,
    net: &str,
) -> ScenarioSuggestion {
    ScenarioSuggestion {
        id: format!(
            "gpio_backdrive_{}_{}_to_{}_{}",
            sanitized_name(driver_component),
            sanitized_name(driver_pin),
            sanitized_name(victim_component),
            sanitized_name(victim_pin)
        ),
        kind: "gpio_backdrive".to_string(),
        confidence: "medium".to_string(),
        runnable: false,
        reason: format!(
            "Powered output {driver_component}.{driver_pin} shares net {net} with unpowered input {victim_component}.{victim_pin}, but no GPIO_BACKDRIVE scenario covers that path."
        ),
        scenario: SuggestedScenario {
            name: format!(
                "{}_to_{}_backdrive",
                sanitized_name(driver_component),
                sanitized_name(victim_component)
            ),
            scenario_type: "gpio_backdrive".to_string(),
            checks: vec![GPIO_BACKDRIVE.to_string()],
            target: None,
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
            conditioning: None,
            pin_states: vec![
                SuggestedPinState {
                    component: driver_component.to_string(),
                    pin: driver_pin.to_string(),
                    mode: "output".to_string(),
                    state: Some("high".to_string()),
                },
                SuggestedPinState {
                    component: victim_component.to_string(),
                    pin: victim_pin.to_string(),
                    mode: "input".to_string(),
                    state: None,
                },
            ],
            paths: vec![SuggestedBackdrivePath {
                driver: SuggestedEndpoint {
                    component: driver_component.to_string(),
                    pin: driver_pin.to_string(),
                },
                victim: SuggestedEndpoint {
                    component: victim_component.to_string(),
                    pin: victim_pin.to_string(),
                },
                net: Some(net.to_string()),
                series_resistance_ohm: 0.0,
            }],
        },
        required_inputs: vec![
            "Confirm the driver can be high while the victim rail is unpowered, using firmware, host, reset-state, or hot-plug evidence.".to_string(),
            "Fill paths[].series_resistance_ohm from the schematic protection path; keep 0 only when there is no series resistor, switch, or protection element.".to_string(),
        ],
    }
}

fn interface_protection_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_interface_protection_checks(bound.project);
    let mut suggestions = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        for channel in &model.signal_conditioning.channels {
            if existing.contains_key(&(component_id.clone(), channel.name.clone())) {
                continue;
            }
            let Some(conditioning) = suggested_conditioning(bound, component_id, model, channel)
            else {
                continue;
            };
            suggestions.push(ScenarioSuggestion {
                id: format!(
                    "interface_protection_{}_{}",
                    sanitized_name(component_id),
                    sanitized_name(&channel.name)
                ),
                kind: "interface_protection".to_string(),
                confidence: "medium".to_string(),
                runnable: false,
                reason: format!(
                    "Component {component_id} model declares signal-conditioning channel {}, but no interface protection review scenario covers it.",
                    channel.name
                ),
                scenario: SuggestedScenario {
                    name: format!(
                        "{}_{}_interface_protection",
                        sanitized_name(component_id),
                        sanitized_name(&channel.name)
                    ),
                    scenario_type: "interface_protection".to_string(),
                    checks: vec![INTERFACE_PROTECTION_REVIEW.to_string()],
                    target: Some(SuggestedTarget {
                        component: component_id.clone(),
                        power_pin: None,
                        reset_pin: None,
                    }),
                    timing: None,
                    required_boot_mode: None,
                    straps: Vec::new(),
                    bootloader: None,
                    events: Vec::new(),
                    conditioning: Some(conditioning),
                    pin_states: Vec::new(),
                    paths: Vec::new(),
                },
                required_inputs: vec![
                    "Confirm the signal-conditioning part datasheet supports this direction, voltage range, and unpowered-side behavior.".to_string(),
                    "Fill enable/OE/reset-state evidence when the part can disconnect or leave either side high impedance.".to_string(),
                    "Add analog_transient or GPIO_BACKDRIVE scenarios for any datasheet condition that does not guarantee isolation.".to_string(),
                ],
            });
        }
    }
    suggestions
}

fn suggested_conditioning(
    bound: &BoundBoard<'_>,
    component_id: &str,
    model: &ComponentModel,
    channel: &SignalConditioningChannel,
) -> Option<SuggestedConditioning> {
    let side_a = suggested_conditioning_side(
        bound,
        component_id,
        &channel.side_a_pin,
        channel.side_a_supply_pin.as_deref(),
    )?;
    let side_b = suggested_conditioning_side(
        bound,
        component_id,
        &channel.side_b_pin,
        channel.side_b_supply_pin.as_deref(),
    )?;
    if !model.ports.contains_key(&channel.side_a_pin)
        || !model.ports.contains_key(&channel.side_b_pin)
    {
        return None;
    }
    Some(SuggestedConditioning {
        component: component_id.to_string(),
        channel: channel.name.clone(),
        kind: signal_conditioning_kind_name(&channel.kind).to_string(),
        side_a,
        side_b,
        direction: channel.direction.clone(),
        unpowered_isolation: channel.unpowered_isolation,
    })
}

fn signal_conditioning_kind_name(kind: &SignalConditioningKind) -> &'static str {
    match kind {
        SignalConditioningKind::LevelShifter => "level_shifter",
        SignalConditioningKind::Protection => "protection",
        SignalConditioningKind::SeriesResistor => "series_resistor",
        SignalConditioningKind::BusSwitch => "bus_switch",
    }
}

fn suggested_conditioning_side(
    bound: &BoundBoard<'_>,
    component_id: &str,
    pin: &str,
    supply_pin: Option<&str>,
) -> Option<SuggestedConditioningSide> {
    let component = bound.project.board.components.get(component_id)?;
    let net = component.pins.get(pin)?.clone();
    let supply_net = supply_pin
        .and_then(|pin_name| {
            component
                .power_domains
                .get(pin_name)
                .or_else(|| component.pins.get(pin_name))
        })
        .cloned();
    Some(SuggestedConditioningSide {
        pin: pin.to_string(),
        net,
        supply_pin: supply_pin.map(str::to_string),
        supply_net,
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
        let reset_release_at_us = rc_evidence
            .as_ref()
            .map(|evidence| evidence.reset_release_at_us);
        let reset_release_delay_us = rc_evidence
            .as_ref()
            .map(|evidence| evidence.reset_release_delay_us)
            .unwrap_or(0.0);
        let boot_sample_at_us = model
            .behavior
            .boot
            .as_ref()
            .and_then(|boot| boot.sample_time_after_reset_release_us)
            .map(|delay_us| reset_release_at_us.unwrap_or(power_valid_at_us) + delay_us);
        let (runnable, reason, required_inputs) = match &rc_evidence {
            Some(evidence) => (
                true,
                format!(
                    "Component {component_id} has active-low reset behavior, target rail power_valid_at_us, and explicit RC reset evidence from {} and {}.",
                    evidence.pullup_component, evidence.capacitor_component
                ),
                Vec::new(),
            ),
            None => (
                false,
                format!(
                    "Component {component_id} has reset behavior and target rail power_valid_at_us, but no RESET_RELEASE_AFTER_POWER_VALID scenario."
                ),
                vec![
                    "Fill timing.reset_release_at_us from reset supervisor, RC, control-line, or analog waveform evidence before validation.".to_string(),
                    "Keep timing.power_valid_at_us equal to the target rail power_valid_at_us or remove duplicated stale timing.".to_string(),
                ],
            ),
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
                events: Vec::new(),
                conditioning: None,
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
            for requirement in &mode.straps {
                match component.pins.get(&requirement.pin) {
                    Some(net) => {
                        if !strap_net_has_bias(bound.project, net) {
                            all_straps_have_bias = false;
                        }
                        straps.push(SuggestedStrap {
                            component: component_id.clone(),
                            pin: requirement.pin.clone(),
                            net: Some(net.clone()),
                            actual: None,
                        });
                    }
                    None => missing_pins.push(requirement.pin.clone()),
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
                        target: Some(SuggestedTarget {
                            component: component_id.clone(),
                            power_pin: None,
                            reset_pin: None,
                        }),
                        timing: None,
                        required_boot_mode: Some(mode_name.clone()),
                        straps: Vec::new(),
                        bootloader: None,
                        events: Vec::new(),
                        conditioning: None,
                        pin_states: Vec::new(),
                        paths: Vec::new(),
                    },
                    required_inputs: Vec::new(),
                });
            }
            if existing.contains_key(&(component_id.clone(), mode_name.clone())) {
                continue;
            }
            let mut required_inputs = vec![format!(
                "Fill strap actual states for boot mode {mode_name}: {}.",
                mode.straps
                    .iter()
                    .map(|strap| format!("{}.{}={}", component_id, strap.pin, strap.required_state))
                    .collect::<Vec<_>>()
                    .join(", ")
            )];
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
                confidence: "medium".to_string(),
                runnable: false,
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
                    target: Some(SuggestedTarget {
                        component: component_id.clone(),
                        power_pin: None,
                        reset_pin: None,
                    }),
                    timing: None,
                    required_boot_mode: Some(mode_name.clone()),
                    straps,
                    bootloader: None,
                    events: Vec::new(),
                    conditioning: None,
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
            let mut required_inputs = Vec::new();
            if sender.is_none() {
                required_inputs.push(format!(
                    "Connect an output-capable sender pin to {}.{} for interface {interface_name}.",
                    component_id, interface.rx_pin
                ));
            }
            required_inputs.push(
                "Fill event at_us after reset release and boot strap sampling evidence."
                    .to_string(),
            );
            suggestions.push(ScenarioSuggestion {
                id: format!(
                    "uart_bootloader_sync_{}_{}",
                    sanitized_name(component_id),
                    sanitized_name(interface_name)
                ),
                kind: "serial_programming".to_string(),
                confidence: if sender.is_some() { "medium" } else { "low" }.to_string(),
                runnable: false,
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
                    target: Some(SuggestedTarget {
                        component: component_id.clone(),
                        power_pin: None,
                        reset_pin: None,
                    }),
                    timing: None,
                    required_boot_mode: None,
                    straps: Vec::new(),
                    bootloader: Some(SuggestedBootloader {
                        component: component_id.clone(),
                        interface: interface_name.to_string(),
                        sync_byte: interface.sync_byte,
                        expected_response: interface.ack_byte,
                    }),
                    events: vec![SuggestedEvent {
                        at_us: None,
                        action: "uart_send".to_string(),
                        from: sender,
                        to: Some(SuggestedEndpoint {
                            component: component_id.clone(),
                            pin: interface.rx_pin.clone(),
                        }),
                        bytes: vec![interface.sync_byte],
                    }],
                    conditioning: None,
                    pin_states: Vec::new(),
                    paths: Vec::new(),
                },
                required_inputs,
            });
        }
    }
    suggestions
}

fn existing_backdrive_paths(
    project: &BoardProject,
) -> BTreeMap<(String, String, String, String), ()> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "gpio_backdrive"
                && scenario.checks.iter().any(|check| check == GPIO_BACKDRIVE)
        })
        .flat_map(|scenario| {
            scenario.paths.iter().map(|path| {
                (
                    (
                        path.driver.component.clone(),
                        path.driver.pin.clone(),
                        path.victim.component.clone(),
                        path.victim.pin.clone(),
                    ),
                    (),
                )
            })
        })
        .collect()
}

fn existing_interface_protection_checks(project: &BoardProject) -> BTreeMap<(String, String), ()> {
    project
        .scenarios
        .iter()
        .filter(|scenario| {
            scenario.scenario_type == "interface_protection"
                && scenario
                    .checks
                    .iter()
                    .any(|check| check == INTERFACE_PROTECTION_REVIEW)
        })
        .filter_map(|scenario| {
            let target = scenario.target.as_ref()?;
            let channel = scenario
                .parameters
                .get("channel")
                .and_then(serde_yaml_ng::Value::as_str)?;
            Some(((target.component.clone(), channel.to_string()), ()))
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

fn component_power_state(
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

fn kicad_pin_type_output_capable(
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

fn kicad_pin_type_input_capable(
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

fn sanitized_name(value: &str) -> String {
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
