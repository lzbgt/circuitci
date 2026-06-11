use crate::board_ir::{BoardProject, Endpoint, PinLogicState, PinMode, Scenario};
use crate::library::{BoundBoard, ComponentModel, Port, PortKind};
use crate::reports::{EndpointPair, Finding, Limitation};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const GPIO_BACKDRIVE: &str = "GPIO_BACKDRIVE";
const RESET_RELEASE_AFTER_POWER_VALID: &str = "RESET_RELEASE_AFTER_POWER_VALID";
const BOOT_STRAP_DEFINED: &str = "BOOT_STRAP_DEFINED";
const UART_BOOTLOADER_SYNC: &str = "UART_BOOTLOADER_SYNC";

pub fn validate(bound: &BoundBoard<'_>) -> (Vec<Finding>, Vec<Limitation>) {
    let mut findings = bound.findings.clone();
    let mut limitations = Vec::new();
    let mut added_backdrive_limitation = false;

    for scenario in &bound.project.scenarios {
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
                    validate_backdrive(bound, scenario, &mut findings)
                }
                RESET_RELEASE_AFTER_POWER_VALID if scenario.scenario_type == "reset_boot" => {
                    validate_reset_release(scenario, &mut findings)
                }
                BOOT_STRAP_DEFINED if scenario.scenario_type == "reset_boot" => {
                    validate_boot_straps(bound, scenario, &mut findings)
                }
                UART_BOOTLOADER_SYNC if scenario.scenario_type == "serial_programming" => {
                    validate_uart_bootloader_sync(bound, scenario, &mut findings)
                }
                GPIO_BACKDRIVE
                | RESET_RELEASE_AFTER_POWER_VALID
                | BOOT_STRAP_DEFINED
                | UART_BOOTLOADER_SYNC => findings.push(Finding::critical(
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

    (findings, limitations)
}

fn validate_backdrive(bound: &BoundBoard<'_>, scenario: &Scenario, findings: &mut Vec<Finding>) {
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

fn validate_reset_release(scenario: &Scenario, findings: &mut Vec<Finding>) {
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "reset_boot target.component is required.",
        );
        return;
    };
    let Some(timing) = &scenario.timing else {
        validation_input_missing(findings, scenario, "reset_boot timing is required.");
        return;
    };

    let margin_us = timing.reset_release_at_us - timing.power_valid_at_us;
    if margin_us < 0.0 {
        let mut finding = Finding::critical(
            RESET_RELEASE_AFTER_POWER_VALID,
            &scenario.name,
            format!(
                "Reset releases before power is valid for component {}.",
                target.component
            ),
        );
        finding.component = Some(target.component.clone());
        finding.measured.insert(
            "power_valid_at_us".to_string(),
            json!(timing.power_valid_at_us),
        );
        finding.measured.insert(
            "reset_release_at_us".to_string(),
            json!(timing.reset_release_at_us),
        );
        finding
            .measured
            .insert("margin_us".to_string(), json!(margin_us));
        finding.limit.insert(
            "reset_release_not_before_power_valid".to_string(),
            json!(true),
        );
        finding.suggested_fixes = vec![
            "Delay reset release until the MCU operating rail is valid.".to_string(),
            "Increase reset RC delay or use a supervisor IC.".to_string(),
            "Tie reset release to regulator power-good when available.".to_string(),
        ];
        findings.push(finding);
    }
}

fn validate_boot_straps(bound: &BoundBoard<'_>, scenario: &Scenario, findings: &mut Vec<Finding>) {
    let Some((target_component, model)) = target_model(bound, scenario) else {
        validation_input_missing(
            findings,
            scenario,
            "reset_boot target component and model are required for boot strap validation.",
        );
        return;
    };
    let Some(required_boot_mode) = &scenario.required_boot_mode else {
        validation_input_missing(findings, scenario, "required_boot_mode is required.");
        return;
    };
    let Some(boot) = &model.behavior.boot else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Component model {} does not declare boot modes.",
                model.component_id
            ),
        );
        return;
    };
    let Some(mode) = boot.modes.get(required_boot_mode) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Component model {} does not declare boot mode {}.",
                model.component_id, required_boot_mode
            ),
        );
        return;
    };

    for requirement in &mode.straps {
        let observed = scenario
            .straps
            .iter()
            .find(|strap| strap.component == target_component && strap.pin == requirement.pin);
        let observed_state = observed.map(|strap| normalize_state(&strap.actual));
        let required_state = normalize_state(&requirement.required_state);
        let failed = match observed_state.as_deref() {
            None | Some("floating" | "undefined") => true,
            Some(actual) => actual != required_state,
        };
        if failed {
            let mut finding = Finding::critical(
                BOOT_STRAP_DEFINED,
                &scenario.name,
                format!(
                    "Boot strap {}.{} is not valid for boot mode {}.",
                    target_component, requirement.pin, required_boot_mode
                ),
            );
            finding.component = Some(target_component.clone());
            if let Some(net) = observed.and_then(|strap| strap.net.clone()) {
                finding.net = Some(net);
            }
            finding
                .measured
                .insert("required_boot_mode".to_string(), json!(required_boot_mode));
            finding.measured.insert(
                format!("observed_{}", requirement.pin),
                json!(observed_state.unwrap_or_else(|| "missing".to_string())),
            );
            finding.limit.insert(
                format!("required_{}", requirement.pin),
                json!(required_state),
            );
            finding.suggested_fixes = vec![
                "Set the boot strap resistor network to the required state during sampling."
                    .to_string(),
                "Avoid leaving boot strap pins floating or in the undefined region.".to_string(),
                "Check reset timing so straps are stable before the boot sample time.".to_string(),
            ];
            findings.push(finding);
        }
    }
}

fn validate_uart_bootloader_sync(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some((target_component, model)) = target_model(bound, scenario) else {
        validation_input_missing(
            findings,
            scenario,
            "serial_programming target component and model are required.",
        );
        return;
    };
    let Some(bootloader) = &scenario.bootloader else {
        validation_input_missing(findings, scenario, "bootloader block is required.");
        return;
    };
    if bootloader
        .component
        .as_ref()
        .is_some_and(|component| component != &target_component)
    {
        validation_input_missing(
            findings,
            scenario,
            "bootloader.component must match target.component.",
        );
        return;
    }
    let Some(behavior) = &model.behavior.bootloader else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "Component model {} does not declare bootloader interfaces.",
                model.component_id
            ),
        );
        return;
    };
    let Some(interface) = behavior.interfaces.get(&bootloader.interface) else {
        let mut finding = Finding::critical(
            UART_BOOTLOADER_SYNC,
            &scenario.name,
            format!(
                "Component model {} does not support bootloader interface {}.",
                model.component_id, bootloader.interface
            ),
        );
        finding.component = Some(target_component.clone());
        finding
            .limit
            .insert("interface".to_string(), json!(bootloader.interface));
        finding.suggested_fixes = vec![
            "Select a bootloader interface supported by the target component model.".to_string(),
            "Add a model-library bootloader interface only when the datasheet supports it."
                .to_string(),
        ];
        findings.push(finding);
        return;
    };
    if bootloader
        .sync_byte
        .is_some_and(|sync_byte| sync_byte != interface.sync_byte)
        || bootloader
            .expected_response
            .is_some_and(|ack_byte| ack_byte != interface.ack_byte)
    {
        let mut finding = Finding::critical(
            UART_BOOTLOADER_SYNC,
            &scenario.name,
            "Scenario bootloader sync/ACK bytes conflict with the target model.",
        );
        finding.component = Some(target_component.clone());
        finding
            .measured
            .insert("interface".to_string(), json!(bootloader.interface));
        finding
            .limit
            .insert("sync_byte".to_string(), json!(interface.sync_byte));
        finding
            .limit
            .insert("expected_response".to_string(), json!(interface.ack_byte));
        finding.suggested_fixes = vec![
            "Use the sync and ACK bytes declared by the component model datasheet metadata."
                .to_string(),
        ];
        findings.push(finding);
        return;
    }

    let min_event_time = scenario
        .timing
        .as_ref()
        .and_then(|timing| timing.boot_sample_at_us)
        .unwrap_or(0.0);
    let sync_event = scenario.events.iter().find(|event| {
        event.action == "uart_send"
            && event.at_us >= min_event_time
            && event
                .to
                .as_ref()
                .is_some_and(|to| to.component == target_component && to.pin == interface.rx_pin)
            && event.bytes == [interface.sync_byte]
    });
    if sync_event.is_none() {
        let mut finding = Finding::critical(
            UART_BOOTLOADER_SYNC,
            &scenario.name,
            format!(
                "No UART bootloader sync byte was sent to {}.{}.",
                target_component, interface.rx_pin
            ),
        );
        finding.component = Some(target_component.clone());
        finding
            .measured
            .insert("interface".to_string(), json!(bootloader.interface));
        finding
            .measured
            .insert("sync_event_found".to_string(), json!(false));
        finding
            .limit
            .insert("sync_byte".to_string(), json!(interface.sync_byte));
        finding
            .limit
            .insert("expected_response".to_string(), json!(interface.ack_byte));
        finding
            .limit
            .insert("rx_pin".to_string(), json!(interface.rx_pin));
        if let Some(required_boot_mode) = &scenario.required_boot_mode {
            finding
                .limit
                .insert("required_boot_mode".to_string(), json!(required_boot_mode));
        }
        finding.suggested_fixes = vec![
            "Send the model-declared bootloader sync byte to the target RX pin after boot sampling."
                .to_string(),
            "Check USB-UART TX/RX crossing and target boot mode straps.".to_string(),
        ];
        findings.push(finding);
        return;
    }

    if let Some(required_boot_mode) = &scenario.required_boot_mode {
        let mut local_findings = Vec::new();
        validate_boot_straps(bound, scenario, &mut local_findings);
        if !local_findings.is_empty() {
            findings.extend(local_findings);
        }
        let Some(boot) = &model.behavior.boot else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Component model {} does not declare boot modes.",
                    model.component_id
                ),
            );
            return;
        };
        if !boot.modes.contains_key(required_boot_mode) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Component model {} does not declare boot mode {}.",
                    model.component_id, required_boot_mode
                ),
            );
        }
    }
}

fn shared_net<'a>(
    project: &'a BoardProject,
    driver: &Endpoint,
    victim: &Endpoint,
) -> Option<&'a str> {
    let driver_net = project.net_for_pin(&driver.component, &driver.pin)?;
    let victim_net = project.net_for_pin(&victim.component, &victim.pin)?;
    (driver_net == victim_net).then_some(driver_net)
}

fn model_port<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &str,
    pin: &str,
) -> Option<(&'a ComponentModel, &'a Port)> {
    let component = bound.project.board.components.get(component_id)?;
    let model = bound.library.get(&component.model)?;
    let port = model.ports.get(pin)?;
    Some((model, port))
}

fn component_power_voltage(
    bound: &BoundBoard<'_>,
    component_id: &str,
    model: &ComponentModel,
) -> Option<f64> {
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
    let net = bound.project.board.nets.get(net_name)?;
    match net.powered {
        Some(true) => net.nominal_voltage,
        Some(false) => Some(0.0),
        None => None,
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

fn target_model<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
) -> Option<(String, &'a ComponentModel)> {
    let target = scenario.target.as_ref()?;
    let component = bound.project.board.components.get(&target.component)?;
    let model = bound.library.get(&component.model)?;
    Some((target.component.clone(), model))
}

fn validation_input_missing(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    message: impl Into<String>,
) {
    let mut finding = Finding::critical("VALIDATION_INPUT_MISSING", &scenario.name, message);
    finding.suggested_fixes = vec![
        "Add the missing scenario or component-model data required by the declared check."
            .to_string(),
        "Do not declare a validation check until its required inputs are modeled.".to_string(),
    ];
    findings.push(finding);
}

fn normalize_state(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
