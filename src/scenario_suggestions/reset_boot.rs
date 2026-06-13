use crate::board_ir::{BoardProject, ComponentSpec, NetKind, SpicePrimitive};
use crate::library::{BoundBoard, ComponentModel, PortKind, ResetSupervisorActive};
use std::collections::BTreeMap;

use super::types::{
    ScenarioSuggestion, SuggestedBootloader, SuggestedEndpoint, SuggestedEvent,
    SuggestedResetSupervisor, SuggestedScenario, SuggestedStrap, SuggestedTarget, SuggestedTiming,
};
use super::{
    BOOT_STRAP_BIAS_VALID, BOOT_STRAP_DEFINED, RESET_RELEASE_AFTER_POWER_VALID,
    UART_BOOTLOADER_SYNC, component_connects_nets, finite_positive, kicad_pin_type_output_capable,
    resolve_power_pin_net, sanitized_name,
};

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
struct ResetSupervisorTimingEvidence {
    reset_release_delay_us: f64,
    reset_release_at_us: f64,
    supervisor: SuggestedResetSupervisor,
}

#[derive(Debug)]
struct BootloaderTimingEvidence {
    power_pin: String,
    power_valid_at_us: f64,
    reset_pin: String,
    reset_release_delay_us: f64,
    reset_release_at_us: f64,
    boot_sample_at_us: f64,
    reset_supervisors: Vec<SuggestedResetSupervisor>,
}

#[derive(Debug)]
struct BootModeSuggestion {
    mode_name: String,
    straps: Vec<SuggestedStrap>,
}

pub(super) fn reset_release_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
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
        let supervisor_evidence = reset_net.and_then(|net| {
            reset_supervisor_timing_evidence(
                bound,
                component_id,
                net,
                &power_net,
                power_valid_at_us,
            )
        });
        let reset_release_at_us = rc_evidence
            .as_ref()
            .map(|evidence| evidence.reset_release_at_us)
            .or_else(|| {
                runtime_evidence
                    .as_ref()
                    .map(|evidence| evidence.reset_release_at_us)
            })
            .or_else(|| {
                supervisor_evidence
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
            .or_else(|| {
                supervisor_evidence
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
        } else if let Some(evidence) = &supervisor_evidence {
            (
                true,
                format!(
                    "Component {component_id} has reset behavior, target rail power_valid_at_us, and source-backed reset-supervisor timing from {}.",
                    evidence.supervisor.component
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
                reset_supervisors: supervisor_evidence
                    .as_ref()
                    .map(|evidence| vec![evidence.supervisor.clone()])
                    .unwrap_or_default(),
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

fn reset_supervisor_timing_evidence(
    bound: &BoundBoard<'_>,
    target_component: &str,
    reset_net: &str,
    power_net: &str,
    power_valid_at_us: f64,
) -> Option<ResetSupervisorTimingEvidence> {
    let matches: Vec<ResetSupervisorTimingEvidence> = bound
        .project
        .board
        .components
        .iter()
        .filter(|(component_id, _)| component_id.as_str() != target_component)
        .filter_map(|(component_id, component)| {
            let model = bound.library.get(&component.model)?;
            let supervisor = model.reset_supervisor.as_ref()?;
            if model.model_quality.source.trim() != "datasheet"
                || model.model_quality.confidence.trim() == "low"
                || supervisor.active != ResetSupervisorActive::Low
            {
                return None;
            }
            let monitored_net = resolve_power_pin_net(component, &supervisor.monitored_pin)?;
            if monitored_net != power_net {
                return None;
            }
            let supervisor_reset_net = component.pins.get(&supervisor.reset_output_pin)?;
            if supervisor_reset_net != reset_net {
                return None;
            }
            let reset_release_delay_us = supervisor.reset_release_delay_us?;
            if !reset_release_delay_us.is_finite() || reset_release_delay_us < 0.0 {
                return None;
            }
            let reset_release_at_us = power_valid_at_us + reset_release_delay_us;
            if !reset_release_at_us.is_finite() {
                return None;
            }
            Some(ResetSupervisorTimingEvidence {
                reset_release_delay_us,
                reset_release_at_us,
                supervisor: SuggestedResetSupervisor {
                    component: component_id.clone(),
                    monitored_pin: supervisor.monitored_pin.clone(),
                    monitored_net: monitored_net.to_string(),
                    reset_output_pin: supervisor.reset_output_pin.clone(),
                    reset_net: supervisor_reset_net.clone(),
                    threshold_min_v: supervisor.threshold_min_v,
                    threshold_max_v: supervisor.threshold_max_v,
                },
            })
        })
        .collect();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
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

pub(super) fn boot_strap_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
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

pub(super) fn uart_bootloader_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
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
                    reset_supervisors: timing
                        .as_ref()
                        .map(|evidence| evidence.reset_supervisors.clone())
                        .unwrap_or_default(),
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
    let supervisor_evidence = reset_supervisor_timing_evidence(
        bound,
        component_id,
        reset_net,
        &power_net,
        power_valid_at_us,
    );
    let reset_release_at_us = rc_evidence
        .as_ref()
        .map(|evidence| evidence.reset_release_at_us)
        .or_else(|| {
            runtime_evidence
                .as_ref()
                .map(|evidence| evidence.reset_release_at_us)
        })
        .or_else(|| {
            supervisor_evidence
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
        .or_else(|| {
            supervisor_evidence
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
        reset_supervisors: supervisor_evidence
            .map(|evidence| vec![evidence.supervisor])
            .unwrap_or_default(),
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
