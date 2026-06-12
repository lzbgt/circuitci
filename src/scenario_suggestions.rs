use crate::board_ir::{BoardProject, NetKind};
use crate::library::{BoundBoard, PortKind};
use serde::Serialize;
use std::collections::BTreeMap;

const POWER_TREE_VALID: &str = "POWER_TREE_VALID";
const RESET_RELEASE_AFTER_POWER_VALID: &str = "RESET_RELEASE_AFTER_POWER_VALID";
const BOOT_STRAP_DEFINED: &str = "BOOT_STRAP_DEFINED";
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

pub fn suggest_scenarios(bound: &BoundBoard<'_>) -> ScenarioSuggestionReport {
    let mut suggestions = Vec::new();
    if should_suggest_power_tree(bound.project) {
        suggestions.push(power_tree_suggestion(bound.project));
    }
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

fn power_tree_suggestion(project: &BoardProject) -> ScenarioSuggestion {
    ScenarioSuggestion {
        id: "power_tree_valid".to_string(),
        kind: "power_tree".to_string(),
        confidence: "high".to_string(),
        runnable: true,
        reason: "Project declares power nets but no POWER_TREE_VALID scenario.".to_string(),
        scenario: SuggestedScenario {
            name: format!("{}_power_tree", sanitized_name(&project.project.name)),
            scenario_type: "power_tree".to_string(),
            checks: vec![POWER_TREE_VALID.to_string()],
            target: None,
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
        },
        required_inputs: Vec::new(),
    }
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
        let Some((power_pin, power_valid_at_us)) =
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
                    Some((pin_name.clone(), power_valid_at_us))
                } else {
                    None
                }
            })
        else {
            continue;
        };
        let boot_sample_at_us = model
            .behavior
            .boot
            .as_ref()
            .and_then(|boot| boot.sample_time_after_reset_release_us)
            .map(|delay_us| power_valid_at_us + delay_us);
        suggestions.push(ScenarioSuggestion {
            id: format!("reset_release_after_power_valid_{}", sanitized_name(component_id)),
            kind: "reset_boot".to_string(),
            confidence: "medium".to_string(),
            runnable: false,
            reason: format!(
                "Component {component_id} has reset behavior and target rail power_valid_at_us, but no RESET_RELEASE_AFTER_POWER_VALID scenario."
            ),
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
                    reset_release_delay_us: Some(0.0),
                    reset_release_at_us: None,
                    boot_sample_at_us,
                }),
                required_boot_mode: None,
                straps: Vec::new(),
                bootloader: None,
                events: Vec::new(),
            },
            required_inputs: vec![
                "Fill timing.reset_release_at_us from reset supervisor, RC, control-line, or analog waveform evidence before validation.".to_string(),
                "Keep timing.power_valid_at_us equal to the target rail power_valid_at_us or remove duplicated stale timing.".to_string(),
            ],
        });
    }
    suggestions
}

fn boot_strap_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let existing = existing_boot_strap_checks(bound.project);
    let mut suggestions = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let Some(boot) = &model.behavior.boot else {
            continue;
        };
        for (mode_name, mode) in &boot.modes {
            if existing.contains_key(&(component_id.clone(), mode_name.clone())) {
                continue;
            }
            let mut straps = Vec::new();
            let mut missing_pins = Vec::new();
            for requirement in &mode.straps {
                match component.pins.get(&requirement.pin) {
                    Some(net) => straps.push(SuggestedStrap {
                        component: component_id.clone(),
                        pin: requirement.pin.clone(),
                        net: Some(net.clone()),
                        actual: None,
                    }),
                    None => missing_pins.push(requirement.pin.clone()),
                }
            }
            if straps.is_empty() {
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
                },
                required_inputs,
            });
        }
    }
    suggestions
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
