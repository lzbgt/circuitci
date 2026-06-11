use crate::board_ir::{Endpoint, PinLogicState, PinMode, Scenario};
use crate::library::{BoundBoard, PortKind};
use crate::reports::{EndpointPair, Finding};
use serde_json::json;
use std::collections::BTreeMap;

use super::GPIO_BACKDRIVE;
use super::common::{component_power_voltage, model_port, shared_net};

pub(super) fn validate_backdrive(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let diode_drop_v = scenario
        .parameters
        .get("diode_drop_V")
        .and_then(serde_yaml_ng::Value::as_f64)
        .unwrap_or(0.3);

    for path in &scenario.paths {
        let Some(driver_state) = scenario
            .pin_states
            .iter()
            .find(|state| state.component == path.driver.component && state.pin == path.driver.pin)
        else {
            findings.push(Finding::warning(
                "PIN_STATE_MISSING",
                &scenario.name,
                format!(
                    "Missing pin state for driver {}.{}.",
                    path.driver.component, path.driver.pin
                ),
            ));
            continue;
        };
        if driver_state.mode != PinMode::Output || driver_state.state != Some(PinLogicState::High) {
            continue;
        }

        let Some(victim_state) = scenario
            .pin_states
            .iter()
            .find(|state| state.component == path.victim.component && state.pin == path.victim.pin)
        else {
            findings.push(Finding::warning(
                "PIN_STATE_MISSING",
                &scenario.name,
                format!(
                    "Missing pin state for victim {}.{}.",
                    path.victim.component, path.victim.pin
                ),
            ));
            continue;
        };
        if victim_state.mode != PinMode::Input {
            continue;
        }

        let Some(net) = shared_net(bound.project, &path.driver, &path.victim) else {
            findings.push(Finding::warning(
                "BACKDRIVE_PATH_NET_MISMATCH",
                &scenario.name,
                format!(
                    "Backdrive path {}.{} -> {}.{} is not on one shared net.",
                    path.driver.component, path.driver.pin, path.victim.component, path.victim.pin
                ),
            ));
            continue;
        };

        let Some((driver_model, driver_port)) =
            model_port(bound, &path.driver.component, &path.driver.pin)
        else {
            findings.push(Finding::warning(
                "DRIVER_PORT_NOT_FOUND",
                &scenario.name,
                format!(
                    "Driver port {}.{} is unresolved.",
                    path.driver.component, path.driver.pin
                ),
            ));
            continue;
        };
        let Some((victim_model, victim_port)) =
            model_port(bound, &path.victim.component, &path.victim.pin)
        else {
            findings.push(Finding::warning(
                "VICTIM_PORT_NOT_FOUND",
                &scenario.name,
                format!(
                    "Victim port {}.{} is unresolved.",
                    path.victim.component, path.victim.pin
                ),
            ));
            continue;
        };

        if !matches!(
            driver_port.kind,
            PortKind::DigitalElectricalOutput | PortKind::DigitalElectricalIo
        ) {
            findings.push(Finding::warning(
                "DRIVER_KIND_INVALID",
                &scenario.name,
                format!(
                    "Driver {}.{} is not an output-capable port.",
                    path.driver.component, path.driver.pin
                ),
            ));
            continue;
        }
        if !matches!(
            victim_port.kind,
            PortKind::DigitalElectricalInput | PortKind::DigitalElectricalIo
        ) {
            findings.push(Finding::warning(
                "VICTIM_KIND_INVALID",
                &scenario.name,
                format!(
                    "Victim {}.{} is not an input-capable port.",
                    path.victim.component, path.victim.pin
                ),
            ));
            continue;
        }

        let Some(driver_high_v) = driver_port.electrical.drive_high_voltage_v else {
            missing_electrical(
                findings,
                &scenario.name,
                "drive_high_voltage_V",
                &path.driver,
            );
            continue;
        };
        let Some(source_ohm) = driver_port.electrical.source_impedance_ohm else {
            missing_electrical(
                findings,
                &scenario.name,
                "source_impedance_ohm",
                &path.driver,
            );
            continue;
        };
        let Some(limit_a) = victim_port.electrical.injection_current_limit_a else {
            missing_electrical(
                findings,
                &scenario.name,
                "injection_current_limit_A",
                &path.victim,
            );
            continue;
        };
        let Some(victim_rail_v) =
            component_power_voltage(bound, &path.victim.component, victim_model)
        else {
            findings.push(Finding::warning(
                "VICTIM_POWER_UNKNOWN",
                &scenario.name,
                format!(
                    "Victim component {} power voltage is unknown.",
                    path.victim.component
                ),
            ));
            continue;
        };
        let Some(driver_rail_v) =
            component_power_voltage(bound, &path.driver.component, driver_model)
        else {
            findings.push(Finding::warning(
                "DRIVER_POWER_UNKNOWN",
                &scenario.name,
                format!(
                    "Driver component {} power voltage is unknown.",
                    path.driver.component
                ),
            ));
            continue;
        };
        if driver_rail_v <= 0.0 {
            continue;
        }

        let effective_ohm = source_ohm + path.series_resistance_ohm;
        if effective_ohm <= 0.0 {
            findings.push(Finding::warning(
                "INVALID_BACKDRIVE_RESISTANCE",
                &scenario.name,
                "Backdrive effective resistance must be greater than zero.",
            ));
            continue;
        }
        let injection_current_a =
            ((driver_high_v - victim_rail_v - diode_drop_v).max(0.0)) / effective_ohm;
        if injection_current_a > limit_a {
            let mut measured = BTreeMap::new();
            measured.insert(
                "injection_current_A".to_string(),
                json!(injection_current_a),
            );
            measured.insert("driver_high_voltage_V".to_string(), json!(driver_high_v));
            measured.insert("victim_rail_voltage_V".to_string(), json!(victim_rail_v));
            measured.insert("effective_resistance_ohm".to_string(), json!(effective_ohm));
            let mut limit = BTreeMap::new();
            limit.insert("injection_current_A".to_string(), json!(limit_a));

            let mut finding = Finding::critical(
                GPIO_BACKDRIVE,
                &scenario.name,
                format!(
                    "Powered component {}.{} drives unpowered component {}.{} on net {net}.",
                    path.driver.component, path.driver.pin, path.victim.component, path.victim.pin
                ),
            );
            finding.component = Some(path.victim.component.clone());
            finding.net = Some(net.to_string());
            finding.endpoints = Some(EndpointPair {
                driver: path.driver.clone(),
                victim: path.victim.clone(),
            });
            finding.measured = measured;
            finding.limit = limit;
            finding.suggested_fixes = vec![
                "Add a series resistor sized to keep injection current below the receiving pin limit.".to_string(),
                "Add a bus switch or isolation device.".to_string(),
                "Ensure both components are in the same powered domain before driving the net.".to_string(),
                "Configure the driving pin as high impedance while the receiving component is unpowered.".to_string(),
            ];
            findings.push(finding);
        }
    }
}

fn missing_electrical(
    findings: &mut Vec<Finding>,
    scenario: &str,
    field: &str,
    endpoint: &Endpoint,
) {
    findings.push(Finding::warning(
        "ELECTRICAL_METADATA_MISSING",
        scenario,
        format!(
            "Missing {field} for {}.{}.",
            endpoint.component, endpoint.pin
        ),
    ));
}
