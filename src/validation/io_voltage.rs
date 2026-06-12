use crate::board_ir::{Endpoint, Scenario};
use crate::library::{BoundBoard, ComponentModel, Port, PortKind};
use crate::reports::{EndpointPair, Finding};
use serde_json::json;

use super::IO_VOLTAGE_COMPATIBLE;
use super::common::{PinDirection, component_power_voltage, kicad_pin_direction_capable};

#[derive(Debug)]
struct DigitalPin<'a> {
    component: &'a str,
    pin: &'a str,
    net: &'a str,
    model: &'a ComponentModel,
    port: &'a Port,
}

pub(super) fn validate_io_voltage_compatible(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let diode_drop_v = scenario
        .parameters
        .get("diode_drop_V")
        .and_then(serde_yaml_ng::Value::as_f64)
        .unwrap_or(0.3);
    let mut pins = Vec::new();

    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        for (pin_name, net_name) in &component.pins {
            let Some(port) = model.ports.get(pin_name) else {
                continue;
            };
            if !is_digital(port) {
                continue;
            }
            pins.push(DigitalPin {
                component: component_id,
                pin: pin_name,
                net: net_name,
                model,
                port,
            });
        }
    }

    for driver in pins.iter().filter(|pin| {
        is_output_capable(pin.port) && kicad_direction_capable(bound, pin, PinDirection::Output)
    }) {
        for receiver in pins.iter().filter(|pin| {
            pin.net == driver.net
                && is_input_capable(pin.port)
                && kicad_direction_capable(bound, pin, PinDirection::Input)
        }) {
            if driver.component == receiver.component && driver.pin == receiver.pin {
                continue;
            }
            validate_logic_high_margin(driver, receiver, scenario, findings);
            validate_input_clamp_current(driver, receiver, diode_drop_v, scenario, bound, findings);
        }
    }
}

fn kicad_direction_capable(
    bound: &BoundBoard<'_>,
    pin: &DigitalPin<'_>,
    direction: PinDirection,
) -> bool {
    let endpoint = Endpoint {
        component: pin.component.to_string(),
        pin: pin.pin.to_string(),
    };
    kicad_pin_direction_capable(bound, &endpoint, direction)
}

fn validate_logic_high_margin(
    driver: &DigitalPin<'_>,
    receiver: &DigitalPin<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(driver_high_v) = driver.port.electrical.drive_high_voltage_v else {
        return;
    };
    let Some(vih_min_v) = receiver.port.electrical.vih_min_v else {
        return;
    };
    if driver_high_v >= vih_min_v {
        return;
    }

    let mut finding = Finding::critical(
        IO_VOLTAGE_COMPATIBLE,
        &scenario.name,
        format!(
            "Driver {}.{} high level {:.6} V on net {} is below receiver {}.{} VIH {:.6} V.",
            driver.component,
            driver.pin,
            driver_high_v,
            driver.net,
            receiver.component,
            receiver.pin,
            vih_min_v
        ),
    );
    finding.component = Some(receiver.component.to_string());
    finding.net = Some(driver.net.to_string());
    finding.endpoints = Some(endpoint_pair(driver, receiver));
    finding
        .measured
        .insert("driver_high_voltage_V".to_string(), json!(driver_high_v));
    finding
        .limit
        .insert("receiver_vih_min_V".to_string(), json!(vih_min_v));
    finding.suggested_fixes = vec![
        "Use compatible I/O voltage domains or add a level shifter.".to_string(),
        "Move the driver or receiver to the correct I/O rail if the schematic pin mapping is wrong.".to_string(),
        "Use a datasheet-backed receiver threshold for final sign-off.".to_string(),
    ];
    findings.push(finding);
}

fn validate_input_clamp_current(
    driver: &DigitalPin<'_>,
    receiver: &DigitalPin<'_>,
    diode_drop_v: f64,
    scenario: &Scenario,
    bound: &BoundBoard<'_>,
    findings: &mut Vec<Finding>,
) {
    let Some(driver_high_v) = driver.port.electrical.drive_high_voltage_v else {
        return;
    };
    let Some(source_ohm) = driver.port.electrical.source_impedance_ohm else {
        return;
    };
    let Some(limit_a) = receiver.port.electrical.injection_current_limit_a else {
        return;
    };
    let Some(receiver_rail_v) = component_power_voltage(bound, receiver.component, receiver.model)
    else {
        return;
    };
    if source_ohm <= 0.0 {
        return;
    }

    let injection_current_a =
        ((driver_high_v - receiver_rail_v - diode_drop_v).max(0.0)) / source_ohm;
    if injection_current_a <= limit_a {
        return;
    }

    let mut finding = Finding::critical(
        IO_VOLTAGE_COMPATIBLE,
        &scenario.name,
        format!(
            "Driver {}.{} can inject {:.6} A into receiver {}.{} clamp on net {}, above limit {:.6} A.",
            driver.component,
            driver.pin,
            injection_current_a,
            receiver.component,
            receiver.pin,
            driver.net,
            limit_a
        ),
    );
    finding.component = Some(receiver.component.to_string());
    finding.net = Some(driver.net.to_string());
    finding.endpoints = Some(endpoint_pair(driver, receiver));
    finding
        .measured
        .insert("driver_high_voltage_V".to_string(), json!(driver_high_v));
    finding.measured.insert(
        "receiver_rail_voltage_V".to_string(),
        json!(receiver_rail_v),
    );
    finding
        .measured
        .insert("source_impedance_ohm".to_string(), json!(source_ohm));
    finding
        .measured
        .insert("diode_drop_V".to_string(), json!(diode_drop_v));
    finding.measured.insert(
        "injection_current_A".to_string(),
        json!(injection_current_a),
    );
    finding
        .limit
        .insert("injection_current_A".to_string(), json!(limit_a));
    finding.suggested_fixes = vec![
        "Add a level shifter, bus switch, or isolation device between I/O voltage domains."
            .to_string(),
        "Add series resistance sized to keep input clamp current below the receiver limit."
            .to_string(),
        "Ensure the receiver rail is powered before the driver asserts a high level.".to_string(),
    ];
    findings.push(finding);
}

fn endpoint_pair(driver: &DigitalPin<'_>, receiver: &DigitalPin<'_>) -> EndpointPair {
    EndpointPair {
        driver: Endpoint {
            component: driver.component.to_string(),
            pin: driver.pin.to_string(),
        },
        victim: Endpoint {
            component: receiver.component.to_string(),
            pin: receiver.pin.to_string(),
        },
    }
}

fn is_digital(port: &Port) -> bool {
    matches!(
        port.kind,
        PortKind::DigitalElectricalInput
            | PortKind::DigitalElectricalOutput
            | PortKind::DigitalElectricalIo
    )
}

fn is_output_capable(port: &Port) -> bool {
    matches!(
        port.kind,
        PortKind::DigitalElectricalOutput | PortKind::DigitalElectricalIo
    )
}

fn is_input_capable(port: &Port) -> bool {
    matches!(
        port.kind,
        PortKind::DigitalElectricalInput | PortKind::DigitalElectricalIo
    )
}
