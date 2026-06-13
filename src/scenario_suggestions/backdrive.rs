use crate::board_ir::{BoardProject, GpioBackdriveRuntimeEvidence, PinLogicState, PinMode};
use crate::library::{BoundBoard, PortKind};
use std::collections::BTreeMap;

use super::{
    GPIO_BACKDRIVE, ScenarioSuggestion, SuggestedBackdrivePath, SuggestedEndpoint,
    SuggestedPinState, SuggestedScenario, component_power_state, kicad_pin_type_input_capable,
    kicad_pin_type_output_capable, sanitized_name,
};

pub(super) fn gpio_backdrive_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
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
                        bound,
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
    bound: &BoundBoard<'_>,
    driver_component: &str,
    driver_pin: &str,
    victim_component: &str,
    victim_pin: &str,
    net: &str,
) -> ScenarioSuggestion {
    let runtime_evidence = gpio_backdrive_runtime_evidence(
        bound,
        driver_component,
        driver_pin,
        victim_component,
        victim_pin,
    );
    let series_resistance_ohm = runtime_evidence
        .as_ref()
        .and_then(|evidence| evidence.series_resistance_ohm)
        .unwrap_or(0.0);
    let runnable = runtime_evidence.is_some();
    ScenarioSuggestion {
        id: format!(
            "gpio_backdrive_{}_{}_to_{}_{}",
            sanitized_name(driver_component),
            sanitized_name(driver_pin),
            sanitized_name(victim_component),
            sanitized_name(victim_pin)
        ),
        kind: "gpio_backdrive".to_string(),
        confidence: if runnable { "high" } else { "medium" }.to_string(),
        runnable,
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
            parameters: None,
            target: None,
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
            conditioning: None,
            protection_clamps: Vec::new(),
            usb_connectors: Vec::new(),
            usb_routes: Vec::new(),
            usb_route_pairs: Vec::new(),
            clocks: Vec::new(),
            reset_supervisors: Vec::new(),
            regulators: Vec::new(),
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
                series_resistance_ohm,
            }],
        },
        required_inputs: if runnable {
            Vec::new()
        } else {
            vec![
                "Add board.runtime.gpio_backdrive evidence confirming the driver can be high while the victim rail is unpowered.".to_string(),
                "Fill board.runtime.gpio_backdrive[].series_resistance_ohm from the schematic protection path; use 0 only when there is no series resistor, switch, or protection element.".to_string(),
            ]
        },
    }
}

fn gpio_backdrive_runtime_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    driver_component: &str,
    driver_pin: &str,
    victim_component: &str,
    victim_pin: &str,
) -> Option<&'a GpioBackdriveRuntimeEvidence> {
    bound
        .project
        .board
        .runtime
        .gpio_backdrive
        .iter()
        .find(|evidence| {
            evidence.driver.component == driver_component
                && evidence.driver.pin == driver_pin
                && evidence.victim.component == victim_component
                && evidence.victim.pin == victim_pin
                && evidence.driver_state == Some(PinLogicState::High)
                && evidence.victim_mode == Some(PinMode::Input)
                && evidence
                    .series_resistance_ohm
                    .is_some_and(|value| value.is_finite() && value >= 0.0)
        })
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
