use crate::board_ir::{ComponentSpec, NetKind, NetSpec, PinLogicState, Scenario};
use crate::library::{BoundBoard, ComponentModel, PortKind, PowerSwitchState};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeMap;

use super::POWER_TREE_VALID;

pub(super) fn validate_power_tree(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut loads_by_net: BTreeMap<String, Vec<PowerLoad>> = BTreeMap::new();

    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        for (pin_name, port) in &model.ports {
            if port.kind != PortKind::ElectricalPower {
                continue;
            }
            let Some(net_name) = resolve_power_net(component, pin_name) else {
                continue;
            };
            let Some(net) = bound.project.board.nets.get(net_name) else {
                continue;
            };

            if !is_inactive_power_mux_input(component, model, pin_name, net) {
                validate_power_net(
                    component_id,
                    pin_name,
                    model,
                    net_name,
                    net,
                    scenario,
                    findings,
                );
            }

            if !is_supply_source(model) {
                loads_by_net
                    .entry(net_name.to_string())
                    .or_default()
                    .push(PowerLoad {
                        component: component_id.clone(),
                        pin: pin_name.clone(),
                        max_current_a: port.electrical.max_supply_current_a,
                    });
            }
        }
    }

    for (component_id, component) in &bound.project.board.components {
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        validate_power_conversion(
            component_id,
            component,
            model,
            &loads_by_net,
            bound,
            scenario,
            findings,
        );
        validate_power_switch(
            component_id,
            component,
            model,
            &loads_by_net,
            bound,
            scenario,
            findings,
        );
        validate_battery_charger(component_id, component, model, bound, scenario, findings);
        validate_power_mux(component_id, component, model, bound, scenario, findings);
    }

    for (net_name, loads) in &loads_by_net {
        let Some(net) = bound.project.board.nets.get(net_name) else {
            continue;
        };
        let Some(limit_a) = net.supply_current_limit_a else {
            continue;
        };
        let mut total_a = 0.0;
        let mut missing_loads = Vec::new();
        for load in loads {
            match load.max_current_a {
                Some(current) if current.is_finite() && current >= 0.0 => total_a += current,
                _ => missing_loads.push(format!("{}.{}", load.component, load.pin)),
            }
        }
        if !missing_loads.is_empty() {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Power rail {net_name} declares supply_current_limit_A but load current metadata is missing for {}.",
                    missing_loads.join(", ")
                ),
            );
            finding.net = Some(net_name.clone());
            finding.measured.insert(
                "missing_load_current_metadata".to_string(),
                json!(missing_loads),
            );
            finding
                .limit
                .insert("supply_current_limit_A".to_string(), json!(limit_a));
            finding.suggested_fixes = vec![
                "Add max_supply_current_A to every component model power pin on the budgeted rail.".to_string(),
                "Split the rail budget by scenario if loads are mutually exclusive rather than simultaneous.".to_string(),
            ];
            findings.push(finding);
            continue;
        }
        if total_a > limit_a {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Power rail {net_name} worst-case declared load {:.6} A exceeds supply limit {:.6} A.",
                    total_a, limit_a
                ),
            );
            finding.net = Some(net_name.clone());
            finding
                .measured
                .insert("declared_load_current_A".to_string(), json!(total_a));
            finding
                .limit
                .insert("supply_current_limit_A".to_string(), json!(limit_a));
            finding.suggested_fixes = vec![
                "Increase regulator or upstream supply current rating with margin for startup and transients.".to_string(),
                "Reduce loads, sequence high-current consumers, or split the design into separately budgeted rails.".to_string(),
            ];
            findings.push(finding);
        }
    }
}

fn validate_power_mux(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(mux) = &model.power_mux else {
        return;
    };
    if !validate_power_mux_metadata(component_id, model, scenario, findings) {
        return;
    }
    let Some(output_net_name) = resolve_power_net(component, &mux.output_pin) else {
        power_mux_pin_finding(component_id, &mux.output_pin, "output", scenario, findings);
        return;
    };
    let Some(output_net) = bound.project.board.nets.get(output_net_name) else {
        return;
    };

    let selected_input = match mux.selected_input_parameter.as_deref() {
        Some(parameter) => match component.parameters.get(parameter) {
            Some(value) => match value.as_str() {
                Some(selected) => Some(selected),
                None => {
                    let mut finding = Finding::critical(
                        POWER_TREE_VALID,
                        &scenario.name,
                        format!(
                            "Power mux {component_id} selected input parameter {parameter} must be a string."
                        ),
                    );
                    finding.component = Some(component_id.to_string());
                    finding
                        .limit
                        .insert("selected_input_parameter".to_string(), json!(parameter));
                    finding.suggested_fixes = vec![
                        format!(
                            "Set components.{component_id}.parameters.{parameter} to one of the model power_mux input names."
                        ),
                        "Split power-tree scenarios when source selection changes by state."
                            .to_string(),
                    ];
                    findings.push(finding);
                    None
                }
            },
            None => {
                let mut finding = Finding::critical(
                    POWER_TREE_VALID,
                    &scenario.name,
                    format!(
                        "Power mux {component_id} requires component parameter {parameter} for source-selection validation."
                    ),
                );
                finding.component = Some(component_id.to_string());
                finding
                    .limit
                    .insert("required_component_parameter".to_string(), json!(parameter));
                finding.suggested_fixes = vec![
                    format!(
                        "Add components.{component_id}.parameters.{parameter} with the selected source name for this power-tree scenario."
                    ),
                    "Use separate scenarios for USB-powered, battery-powered, and transition states.".to_string(),
                ];
                findings.push(finding);
                None
            }
        },
        None => None,
    };

    let mut selected_found = selected_input.is_none();
    for input in &mux.inputs {
        let Some(input_net_name) = resolve_power_net(component, &input.input_pin) else {
            power_mux_pin_finding(component_id, &input.input_pin, "input", scenario, findings);
            continue;
        };
        let Some(input_net) = bound.project.board.nets.get(input_net_name) else {
            continue;
        };
        let is_selected = selected_input == Some(input.name.as_str());
        if is_selected {
            selected_found = true;
        }

        if output_net.powered == Some(true) && is_selected && input_net.powered != Some(true) {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Power mux {component_id} output rail {output_net_name} is powered by selected input {} but input rail {input_net_name} is not powered.",
                    input.name
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(output_net_name.to_string());
            finding
                .measured
                .insert("selected_input".to_string(), json!(input.name));
            finding.measured.insert(
                "selected_input_powered".to_string(),
                json!(input_net.powered),
            );
            finding
                .measured
                .insert("output_powered".to_string(), json!(true));
            finding
                .limit
                .insert("selected_input_powered".to_string(), json!(true));
            finding.suggested_fixes = vec![
                "Select a powered input source for this scenario or mark the mux output rail unpowered.".to_string(),
                "Split USB-present, battery-present, and transition states into separate power-tree scenarios.".to_string(),
            ];
            findings.push(finding);
        }

        if output_net.powered == Some(true)
            && !is_selected
            && input_net.powered == Some(false)
            && input.reverse_blocking != Some(true)
        {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Power mux {component_id} output rail {output_net_name} is powered while inactive input {} on {input_net_name} is unpowered and lacks reverse-blocking evidence.",
                    input.name
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(input_net_name.to_string());
            finding
                .measured
                .insert("inactive_input".to_string(), json!(input.name));
            finding
                .measured
                .insert("inactive_input_powered".to_string(), json!(false));
            finding
                .measured
                .insert("output_powered".to_string(), json!(true));
            finding
                .limit
                .insert("required_reverse_blocking".to_string(), json!(true));
            finding.suggested_fixes = vec![
                "Use a power mux or ideal-diode path with datasheet-backed reverse blocking for the inactive source.".to_string(),
                "Add an ideal diode, load switch, or explicit disconnect so the powered system rail cannot backfeed the unpowered input.".to_string(),
                "Use analog_transient when reverse-current magnitude or switchover waveform must be quantified.".to_string(),
            ];
            findings.push(finding);
        }
    }

    if let Some(selected) = selected_input
        && !selected_found
    {
        let mut finding = Finding::critical(
            POWER_TREE_VALID,
            &scenario.name,
            format!(
                "Power mux {component_id} selected input {selected} is not declared by the model."
            ),
        );
        finding.component = Some(component_id.to_string());
        finding
            .measured
            .insert("selected_input".to_string(), json!(selected));
        finding.limit.insert(
            "allowed_inputs".to_string(),
            json!(
                mux.inputs
                    .iter()
                    .map(|input| input.name.as_str())
                    .collect::<Vec<_>>()
            ),
        );
        finding.suggested_fixes = vec![
            "Correct the selected input parameter to match a declared power_mux input name."
                .to_string(),
            "Correct the component model if the mux has another valid source path.".to_string(),
        ];
        findings.push(finding);
    }
}

fn validate_battery_charger(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(charger) = &model.battery_charger else {
        return;
    };
    if !validate_battery_charger_metadata(component_id, model, scenario, findings) {
        return;
    }
    let Some(input_net_name) = resolve_power_net(component, &charger.input_pin) else {
        battery_charger_pin_finding(
            component_id,
            &charger.input_pin,
            "input",
            scenario,
            findings,
        );
        return;
    };
    let Some(battery_net_name) = resolve_power_net(component, &charger.battery_pin) else {
        battery_charger_pin_finding(
            component_id,
            &charger.battery_pin,
            "battery",
            scenario,
            findings,
        );
        return;
    };
    let Some(input_net) = bound.project.board.nets.get(input_net_name) else {
        return;
    };
    let Some(battery_net) = bound.project.board.nets.get(battery_net_name) else {
        return;
    };

    let charge_current_a = charger
        .charge_current_parameter
        .as_deref()
        .and_then(|parameter| {
            component
                .parameters
                .get(parameter)
                .and_then(serde_yaml_ng::Value::as_f64)
                .map(|current| (parameter, current))
        });
    if let Some(parameter) = charger.charge_current_parameter.as_deref()
        && charge_current_a.is_none()
    {
        let mut finding = Finding::critical(
            POWER_TREE_VALID,
            &scenario.name,
            format!(
                "Battery charger {component_id} requires component parameter {parameter} for programmed charge-current validation."
            ),
        );
        finding.component = Some(component_id.to_string());
        finding
            .limit
            .insert("required_component_parameter".to_string(), json!(parameter));
        finding.suggested_fixes = vec![
            format!("Add components.{component_id}.parameters.{parameter} from the charger PROG resistor or configured charge-current setting."),
            "Use analog_transient or a charger-specific model when charge current is dynamic or firmware controlled.".to_string(),
        ];
        findings.push(finding);
        return;
    }

    if let Some((parameter, charge_current_a)) = charge_current_a {
        if !charge_current_a.is_finite() || charge_current_a < 0.0 {
            battery_charger_metadata_finding(
                component_id,
                parameter,
                "battery_charger programmed charge current must be finite and non-negative.",
                scenario,
                findings,
            );
            return;
        }
        if let Some(min_charge_current_a) = charger.min_charge_current_a
            && charge_current_a < min_charge_current_a
        {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Battery charger {component_id} programmed charge current {:.6} A is below model minimum {:.6} A.",
                    charge_current_a, min_charge_current_a
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(battery_net_name.to_string());
            finding.measured.insert(
                "programmed_charge_current_A".to_string(),
                json!(charge_current_a),
            );
            finding.limit.insert(
                "battery_charger_min_charge_current_A".to_string(),
                json!(min_charge_current_a),
            );
            finding.suggested_fixes = vec![
                "Use a charge-current programming value inside the charger datasheet range.".to_string(),
                "Select a charger whose programmable-current range covers the intended low-current cell or source.".to_string(),
            ];
            findings.push(finding);
        }
        if let Some(max_charge_current_a) = charger.max_charge_current_a
            && charge_current_a > max_charge_current_a
        {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Battery charger {component_id} programmed charge current {:.6} A exceeds model maximum {:.6} A.",
                    charge_current_a, max_charge_current_a
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(battery_net_name.to_string());
            finding.measured.insert(
                "programmed_charge_current_A".to_string(),
                json!(charge_current_a),
            );
            finding.limit.insert(
                "battery_charger_max_charge_current_A".to_string(),
                json!(max_charge_current_a),
            );
            finding.suggested_fixes = vec![
                "Increase the PROG resistor or charger configuration so programmed current stays inside the datasheet range.".to_string(),
                "Select a charger rated for the intended charge current and thermal dissipation.".to_string(),
            ];
            findings.push(finding);
        }
        if let Some(input_limit_a) = input_net.supply_current_limit_a
            && charge_current_a > input_limit_a
        {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Battery charger {component_id} programmed charge current {:.6} A exceeds input rail {input_net_name} current budget {:.6} A.",
                    charge_current_a, input_limit_a
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(input_net_name.to_string());
            finding.measured.insert(
                "programmed_charge_current_A".to_string(),
                json!(charge_current_a),
            );
            finding.limit.insert(
                "input_supply_current_limit_A".to_string(),
                json!(input_limit_a),
            );
            finding.suggested_fixes = vec![
                "Reduce the charger programmed current to fit the USB/source current budget with margin for system load.".to_string(),
                "Negotiate or provide a higher-current input source before allowing this charge current.".to_string(),
                "Split battery charging and system-load budgets if they are not simultaneous in the validated scenario.".to_string(),
            ];
            findings.push(finding);
        }
    }

    if let Some(regulation_voltage_v) = charger.regulation_voltage_v
        && let Some(battery_voltage_v) = battery_net.nominal_voltage
        && battery_voltage_v.is_finite()
        && battery_voltage_v > regulation_voltage_v
    {
        let mut finding = Finding::critical(
            POWER_TREE_VALID,
            &scenario.name,
            format!(
                "Battery net {battery_net_name} nominal voltage {:.6} V exceeds charger {component_id} regulation voltage {:.6} V.",
                battery_voltage_v, regulation_voltage_v
            ),
        );
        finding.component = Some(component_id.to_string());
        finding.net = Some(battery_net_name.to_string());
        finding.measured.insert(
            "battery_nominal_voltage_V".to_string(),
            json!(battery_voltage_v),
        );
        finding.limit.insert(
            "battery_charger_regulation_voltage_V".to_string(),
            json!(regulation_voltage_v),
        );
        finding.suggested_fixes = vec![
            "Use the charger option that matches the cell regulation voltage.".to_string(),
            "Correct the battery rail nominal_voltage if it represents nominal cell voltage rather than charge regulation voltage.".to_string(),
        ];
        findings.push(finding);
    }
}

fn validate_power_conversion(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    loads_by_net: &BTreeMap<String, Vec<PowerLoad>>,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(conversion) = &model.power_conversion else {
        return;
    };
    if !validate_power_conversion_metadata(component_id, model, scenario, findings) {
        return;
    }
    let Some(input_net_name) = resolve_power_net(component, &conversion.input_pin) else {
        power_conversion_pin_finding(
            component_id,
            &conversion.input_pin,
            "input",
            scenario,
            findings,
        );
        return;
    };
    let Some(output_net_name) = resolve_power_net(component, &conversion.output_pin) else {
        power_conversion_pin_finding(
            component_id,
            &conversion.output_pin,
            "output",
            scenario,
            findings,
        );
        return;
    };
    let Some(input_net) = bound.project.board.nets.get(input_net_name) else {
        return;
    };
    let Some(output_net) = bound.project.board.nets.get(output_net_name) else {
        return;
    };

    if let Some(dropout_v) = conversion.dropout_voltage_v {
        let (Some(input_v), Some(output_v)) =
            (input_net.nominal_voltage, output_net.nominal_voltage)
        else {
            return;
        };
        if input_v.is_finite() && output_v.is_finite() {
            let margin_v = input_v - output_v;
            if margin_v < dropout_v {
                let mut finding = Finding::critical(
                    POWER_TREE_VALID,
                    &scenario.name,
                    format!(
                        "Regulator {component_id} dropout margin {:.6} V is below required dropout {:.6} V.",
                        margin_v, dropout_v
                    ),
                );
                finding.component = Some(component_id.to_string());
                finding.net = Some(output_net_name.to_string());
                finding
                    .measured
                    .insert("input_voltage_V".to_string(), json!(input_v));
                finding
                    .measured
                    .insert("output_voltage_V".to_string(), json!(output_v));
                finding
                    .measured
                    .insert("dropout_margin_V".to_string(), json!(margin_v));
                finding
                    .limit
                    .insert("dropout_voltage_V".to_string(), json!(dropout_v));
                finding.suggested_fixes = vec![
                    "Raise the regulator input rail, lower the output rail, or select a regulator with lower dropout at the required load current.".to_string(),
                    "Use analog_transient or a regulator model with load-dependent dropout when startup/load waveform behavior matters.".to_string(),
                ];
                findings.push(finding);
            }
        }
    }

    if let Some(max_output_current_a) = conversion.max_output_current_a {
        let loads = loads_by_net
            .get(output_net_name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut total_a = 0.0;
        let mut missing_loads = Vec::new();
        for load in loads {
            match load.max_current_a {
                Some(current) if current.is_finite() && current >= 0.0 => total_a += current,
                _ => missing_loads.push(format!("{}.{}", load.component, load.pin)),
            }
        }
        if !missing_loads.is_empty() {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Regulator {component_id} output current limit requires load metadata for {}.",
                    missing_loads.join(", ")
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(output_net_name.to_string());
            finding.measured.insert(
                "missing_load_current_metadata".to_string(),
                json!(missing_loads),
            );
            finding.limit.insert(
                "regulator_max_output_current_A".to_string(),
                json!(max_output_current_a),
            );
            finding.suggested_fixes = vec![
                "Add max_supply_current_A to loads fed by the regulator output rail.".to_string(),
                "Split the scenario if high-current loads are sequenced rather than simultaneous."
                    .to_string(),
            ];
            findings.push(finding);
        } else if total_a > max_output_current_a {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Regulator {component_id} worst-case output load {:.6} A exceeds regulator limit {:.6} A.",
                    total_a, max_output_current_a
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(output_net_name.to_string());
            finding
                .measured
                .insert("declared_output_load_current_A".to_string(), json!(total_a));
            finding.limit.insert(
                "regulator_max_output_current_A".to_string(),
                json!(max_output_current_a),
            );
            finding.suggested_fixes = vec![
                "Select a regulator with sufficient output-current rating and thermal margin.".to_string(),
                "Reduce or sequence loads, or split high-current consumers onto separate regulators.".to_string(),
            ];
            findings.push(finding);
        }
    }

    if let Some(startup_delay_us) = conversion.startup_delay_us {
        validate_regulator_startup_timing(
            RegulatorStartupContext {
                component_id,
                input_net_name,
                input_net,
                output_net_name,
                output_net,
                startup_delay_us,
            },
            scenario,
            findings,
        );
    }
}

fn validate_power_switch(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    loads_by_net: &BTreeMap<String, Vec<PowerLoad>>,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(switch) = &model.power_switch else {
        return;
    };
    if !validate_power_switch_metadata(component_id, model, scenario, findings) {
        return;
    }
    let Some(input_net_name) = resolve_power_net(component, &switch.input_pin) else {
        power_switch_pin_finding(component_id, &switch.input_pin, "input", scenario, findings);
        return;
    };
    let Some(output_net_name) = resolve_power_net(component, &switch.output_pin) else {
        power_switch_pin_finding(
            component_id,
            &switch.output_pin,
            "output",
            scenario,
            findings,
        );
        return;
    };
    let Some(input_net) = bound.project.board.nets.get(input_net_name) else {
        return;
    };
    let Some(output_net) = bound.project.board.nets.get(output_net_name) else {
        return;
    };

    if output_net.powered == Some(true) {
        let observed_state = scenario
            .pin_states
            .iter()
            .find(|state| state.component == component_id && state.pin == switch.control_pin);
        let required_state = power_switch_state_name(&switch.enabled_state);
        let enabled = observed_state
            .and_then(|state| state.state.as_ref())
            .is_some_and(|state| power_switch_pin_state_matches(state, &switch.enabled_state));
        if !enabled {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Load switch {component_id} output rail {output_net_name} is declared powered but {component_id}.{} is not proven {required_state}.",
                    switch.control_pin
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(output_net_name.to_string());
            finding.measured.insert(
                "input_powered".to_string(),
                json!(input_net.powered.unwrap_or(false)),
            );
            finding
                .measured
                .insert("output_powered".to_string(), json!(true));
            finding.measured.insert(
                "control_state".to_string(),
                json!(
                    observed_state
                        .and_then(|state| state.state.as_ref())
                        .map(pin_logic_state_name)
                        .unwrap_or("missing")
                ),
            );
            finding
                .limit
                .insert("control_pin".to_string(), json!(switch.control_pin));
            finding
                .limit
                .insert("required_enabled_state".to_string(), json!(required_state));
            finding.suggested_fixes = vec![
                format!(
                    "Prove {component_id}.{} is driven {required_state} in this power-tree scenario, or mark {output_net_name} unpowered for the disabled case.",
                    switch.control_pin
                ),
                "Connect the enable pin to a deterministic rail, supervisor, MCU GPIO state, or strap that matches the intended power state.".to_string(),
                "Use analog_transient when switch turn-on ramp, inrush, or load sequencing must be validated from waveforms.".to_string(),
            ];
            findings.push(finding);
        }
    }

    if let Some(max_output_current_a) = switch.max_output_current_a {
        let loads = loads_by_net
            .get(output_net_name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut total_a = 0.0;
        let mut missing_loads = Vec::new();
        for load in loads {
            match load.max_current_a {
                Some(current) if current.is_finite() && current >= 0.0 => total_a += current,
                _ => missing_loads.push(format!("{}.{}", load.component, load.pin)),
            }
        }
        if !missing_loads.is_empty() {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Load switch {component_id} output current limit requires load metadata for {}.",
                    missing_loads.join(", ")
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(output_net_name.to_string());
            finding.measured.insert(
                "missing_load_current_metadata".to_string(),
                json!(missing_loads),
            );
            finding.limit.insert(
                "load_switch_max_output_current_A".to_string(),
                json!(max_output_current_a),
            );
            finding.suggested_fixes = vec![
                "Add max_supply_current_A to loads fed by the switched rail.".to_string(),
                "Split the scenario if switched loads are not all enabled simultaneously."
                    .to_string(),
            ];
            findings.push(finding);
        } else if total_a > max_output_current_a {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Load switch {component_id} worst-case output load {:.6} A exceeds switch limit {:.6} A.",
                    total_a, max_output_current_a
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(output_net_name.to_string());
            finding
                .measured
                .insert("declared_output_load_current_A".to_string(), json!(total_a));
            finding.limit.insert(
                "load_switch_max_output_current_A".to_string(),
                json!(max_output_current_a),
            );
            finding.suggested_fixes = vec![
                "Select a load switch with sufficient current and thermal margin.".to_string(),
                "Reduce or sequence loads on the switched rail, or split them across separate switches.".to_string(),
            ];
            findings.push(finding);
        }
    }
}

fn validate_power_conversion_metadata(
    component_id: &str,
    model: &ComponentModel,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> bool {
    let Some(conversion) = &model.power_conversion else {
        return true;
    };
    let mut valid = true;
    if conversion.input_pin == conversion.output_pin {
        power_conversion_metadata_finding(
            component_id,
            "input_pin",
            "power_conversion input_pin and output_pin must be distinct.",
            scenario,
            findings,
        );
        valid = false;
    }
    for (role, pin) in [
        ("input_pin", conversion.input_pin.as_str()),
        ("output_pin", conversion.output_pin.as_str()),
    ] {
        match model.ports.get(pin) {
            Some(port) if port.kind == PortKind::ElectricalPower => {}
            Some(_) => {
                power_conversion_metadata_finding(
                    component_id,
                    role,
                    &format!("power_conversion {role} {pin} is not an electrical_power port."),
                    scenario,
                    findings,
                );
                valid = false;
            }
            None => {
                power_conversion_metadata_finding(
                    component_id,
                    role,
                    &format!("power_conversion {role} {pin} is not declared in model ports."),
                    scenario,
                    findings,
                );
                valid = false;
            }
        }
    }
    if let Some(dropout_v) = conversion.dropout_voltage_v
        && (!dropout_v.is_finite() || dropout_v < 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "dropout_voltage_V",
            "power_conversion dropout_voltage_V must be finite and non-negative.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(max_output_current_a) = conversion.max_output_current_a
        && (!max_output_current_a.is_finite() || max_output_current_a < 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "max_output_current_A",
            "power_conversion max_output_current_A must be finite and non-negative.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(startup_delay_us) = conversion.startup_delay_us
        && (!startup_delay_us.is_finite() || startup_delay_us < 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "startup_delay_us",
            "power_conversion startup_delay_us must be finite and non-negative.",
            scenario,
            findings,
        );
        valid = false;
    }
    valid
}

fn validate_battery_charger_metadata(
    component_id: &str,
    model: &ComponentModel,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> bool {
    let Some(charger) = &model.battery_charger else {
        return true;
    };
    let mut valid = true;
    if charger.input_pin == charger.battery_pin {
        battery_charger_metadata_finding(
            component_id,
            "input_pin",
            "battery_charger input_pin and battery_pin must be distinct.",
            scenario,
            findings,
        );
        valid = false;
    }
    for (role, pin) in [
        ("input_pin", charger.input_pin.as_str()),
        ("battery_pin", charger.battery_pin.as_str()),
    ] {
        match model.ports.get(pin) {
            Some(port) if port.kind == PortKind::ElectricalPower => {}
            Some(_) => {
                battery_charger_metadata_finding(
                    component_id,
                    role,
                    &format!("battery_charger {role} {pin} is not an electrical_power port."),
                    scenario,
                    findings,
                );
                valid = false;
            }
            None => {
                battery_charger_metadata_finding(
                    component_id,
                    role,
                    &format!("battery_charger {role} {pin} is not declared in model ports."),
                    scenario,
                    findings,
                );
                valid = false;
            }
        }
    }
    if let Some(min_charge_current_a) = charger.min_charge_current_a
        && (!min_charge_current_a.is_finite() || min_charge_current_a < 0.0)
    {
        battery_charger_metadata_finding(
            component_id,
            "min_charge_current_A",
            "battery_charger min_charge_current_A must be finite and non-negative.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(max_charge_current_a) = charger.max_charge_current_a
        && (!max_charge_current_a.is_finite() || max_charge_current_a < 0.0)
    {
        battery_charger_metadata_finding(
            component_id,
            "max_charge_current_A",
            "battery_charger max_charge_current_A must be finite and non-negative.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let (Some(min_charge_current_a), Some(max_charge_current_a)) =
        (charger.min_charge_current_a, charger.max_charge_current_a)
        && min_charge_current_a > max_charge_current_a
    {
        battery_charger_metadata_finding(
            component_id,
            "min_charge_current_A",
            "battery_charger min_charge_current_A must not exceed max_charge_current_A.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(regulation_voltage_v) = charger.regulation_voltage_v
        && (!regulation_voltage_v.is_finite() || regulation_voltage_v <= 0.0)
    {
        battery_charger_metadata_finding(
            component_id,
            "regulation_voltage_V",
            "battery_charger regulation_voltage_V must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    valid
}

fn validate_power_mux_metadata(
    component_id: &str,
    model: &ComponentModel,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> bool {
    let Some(mux) = &model.power_mux else {
        return true;
    };
    let mut valid = true;
    match model.ports.get(&mux.output_pin) {
        Some(port) if port.kind == PortKind::ElectricalPower => {}
        Some(_) => {
            power_mux_metadata_finding(
                component_id,
                "output_pin",
                &format!(
                    "power_mux output_pin {} is not an electrical_power port.",
                    mux.output_pin
                ),
                scenario,
                findings,
            );
            valid = false;
        }
        None => {
            power_mux_metadata_finding(
                component_id,
                "output_pin",
                &format!(
                    "power_mux output_pin {} is not declared in model ports.",
                    mux.output_pin
                ),
                scenario,
                findings,
            );
            valid = false;
        }
    }
    if mux.inputs.is_empty() {
        power_mux_metadata_finding(
            component_id,
            "inputs",
            "power_mux inputs must not be empty.",
            scenario,
            findings,
        );
        valid = false;
    }
    let mut seen_names = BTreeMap::<&str, ()>::new();
    for input in &mux.inputs {
        if seen_names.insert(input.name.as_str(), ()).is_some() {
            power_mux_metadata_finding(
                component_id,
                "inputs",
                &format!("power_mux input name {} is duplicated.", input.name),
                scenario,
                findings,
            );
            valid = false;
        }
        if input.input_pin == mux.output_pin {
            power_mux_metadata_finding(
                component_id,
                "input_pin",
                &format!(
                    "power_mux input {} uses the same pin as output_pin {}.",
                    input.name, mux.output_pin
                ),
                scenario,
                findings,
            );
            valid = false;
        }
        match model.ports.get(&input.input_pin) {
            Some(port) if port.kind == PortKind::ElectricalPower => {}
            Some(_) => {
                power_mux_metadata_finding(
                    component_id,
                    "input_pin",
                    &format!(
                        "power_mux input {} pin {} is not an electrical_power port.",
                        input.name, input.input_pin
                    ),
                    scenario,
                    findings,
                );
                valid = false;
            }
            None => {
                power_mux_metadata_finding(
                    component_id,
                    "input_pin",
                    &format!(
                        "power_mux input {} pin {} is not declared in model ports.",
                        input.name, input.input_pin
                    ),
                    scenario,
                    findings,
                );
                valid = false;
            }
        }
    }
    valid
}

fn validate_power_switch_metadata(
    component_id: &str,
    model: &ComponentModel,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> bool {
    let Some(switch) = &model.power_switch else {
        return true;
    };
    let mut valid = true;
    if switch.input_pin == switch.output_pin {
        power_switch_metadata_finding(
            component_id,
            "input_pin",
            "power_switch input_pin and output_pin must be distinct.",
            scenario,
            findings,
        );
        valid = false;
    }
    for (role, pin) in [
        ("input_pin", switch.input_pin.as_str()),
        ("output_pin", switch.output_pin.as_str()),
    ] {
        match model.ports.get(pin) {
            Some(port) if port.kind == PortKind::ElectricalPower => {}
            Some(_) => {
                power_switch_metadata_finding(
                    component_id,
                    role,
                    &format!("power_switch {role} {pin} is not an electrical_power port."),
                    scenario,
                    findings,
                );
                valid = false;
            }
            None => {
                power_switch_metadata_finding(
                    component_id,
                    role,
                    &format!("power_switch {role} {pin} is not declared in model ports."),
                    scenario,
                    findings,
                );
                valid = false;
            }
        }
    }
    match model.ports.get(&switch.control_pin) {
        Some(port)
            if matches!(
                port.kind,
                PortKind::DigitalElectricalInput | PortKind::DigitalElectricalIo
            ) => {}
        Some(_) => {
            power_switch_metadata_finding(
                component_id,
                "control_pin",
                &format!(
                    "power_switch control_pin {} is not a digital input or IO port.",
                    switch.control_pin
                ),
                scenario,
                findings,
            );
            valid = false;
        }
        None => {
            power_switch_metadata_finding(
                component_id,
                "control_pin",
                &format!(
                    "power_switch control_pin {} is not declared in model ports.",
                    switch.control_pin
                ),
                scenario,
                findings,
            );
            valid = false;
        }
    }
    if let Some(max_output_current_a) = switch.max_output_current_a
        && (!max_output_current_a.is_finite() || max_output_current_a < 0.0)
    {
        power_switch_metadata_finding(
            component_id,
            "max_output_current_A",
            "power_switch max_output_current_A must be finite and non-negative.",
            scenario,
            findings,
        );
        valid = false;
    }
    valid
}

fn validate_regulator_startup_timing(
    context: RegulatorStartupContext<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(input_valid_at_us) = context.input_net.power_valid_at_us else {
        regulator_startup_missing_timing_finding(
            context.component_id,
            context.input_net_name,
            "input_power_valid_at_us",
            context.startup_delay_us,
            scenario,
            findings,
        );
        return;
    };
    let Some(output_valid_at_us) = context.output_net.power_valid_at_us else {
        regulator_startup_missing_timing_finding(
            context.component_id,
            context.output_net_name,
            "output_power_valid_at_us",
            context.startup_delay_us,
            scenario,
            findings,
        );
        return;
    };
    if !input_valid_at_us.is_finite() || input_valid_at_us < 0.0 {
        regulator_startup_invalid_timing_finding(
            context.component_id,
            context.input_net_name,
            "input_power_valid_at_us",
            input_valid_at_us,
            scenario,
            findings,
        );
        return;
    }
    if !output_valid_at_us.is_finite() || output_valid_at_us < 0.0 {
        regulator_startup_invalid_timing_finding(
            context.component_id,
            context.output_net_name,
            "output_power_valid_at_us",
            output_valid_at_us,
            scenario,
            findings,
        );
        return;
    }

    let earliest_output_valid_at_us = input_valid_at_us + context.startup_delay_us;
    if output_valid_at_us < earliest_output_valid_at_us {
        let mut finding = Finding::critical(
            POWER_TREE_VALID,
            &scenario.name,
            format!(
                "Regulator {component_id} output rail {output_net_name} is declared valid at {:.6} us before input-valid plus startup delay {:.6} us.",
                output_valid_at_us,
                earliest_output_valid_at_us,
                component_id = context.component_id,
                output_net_name = context.output_net_name,
            ),
        );
        finding.component = Some(context.component_id.to_string());
        finding.net = Some(context.output_net_name.to_string());
        finding.measured.insert(
            "input_power_valid_at_us".to_string(),
            json!(input_valid_at_us),
        );
        finding.measured.insert(
            "output_power_valid_at_us".to_string(),
            json!(output_valid_at_us),
        );
        finding.measured.insert(
            "startup_delay_us".to_string(),
            json!(context.startup_delay_us),
        );
        finding.limit.insert(
            "earliest_output_power_valid_at_us".to_string(),
            json!(earliest_output_valid_at_us),
        );
        finding.suggested_fixes = vec![
            "Delay downstream reset release, enable pins, or boot sampling until the regulator output rail is valid.".to_string(),
            "Correct the rail power_valid_at_us metadata if measured startup timing shows a later valid point.".to_string(),
            "Use analog_transient when startup ramp, soft-start, load current, or power-good waveform shape matters.".to_string(),
        ];
        findings.push(finding);
    }
}

struct RegulatorStartupContext<'a> {
    component_id: &'a str,
    input_net_name: &'a str,
    input_net: &'a NetSpec,
    output_net_name: &'a str,
    output_net: &'a NetSpec,
    startup_delay_us: f64,
}

fn regulator_startup_missing_timing_finding(
    component_id: &str,
    net_name: &str,
    field: &str,
    startup_delay_us: f64,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!(
            "Regulator {component_id} declares startup_delay_us but rail {net_name} has no power_valid_at_us timing."
        ),
    );
    finding.component = Some(component_id.to_string());
    finding.net = Some(net_name.to_string());
    finding
        .measured
        .insert("startup_delay_us".to_string(), json!(startup_delay_us));
    finding
        .limit
        .insert("required_rail_timing_field".to_string(), json!(field));
    finding.suggested_fixes = vec![
        "Declare power_valid_at_us on both input and output rails for regulator startup timing checks.".to_string(),
        "Remove startup_delay_us only when the model is not intended to make startup sequencing claims.".to_string(),
    ];
    findings.push(finding);
}

fn regulator_startup_invalid_timing_finding(
    component_id: &str,
    net_name: &str,
    field: &str,
    value: f64,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!("Rail {net_name} has invalid {field} value {value}."),
    );
    finding.component = Some(component_id.to_string());
    finding.net = Some(net_name.to_string());
    finding.measured.insert(field.to_string(), json!(value));
    finding
        .limit
        .insert("power_valid_at_us_non_negative".to_string(), json!(true));
    finding.suggested_fixes = vec![
        "Use finite non-negative power_valid_at_us rail timing metadata.".to_string(),
        "Use analog_transient if rail validity must be derived from a waveform crossing threshold."
            .to_string(),
    ];
    findings.push(finding);
}

fn power_conversion_metadata_finding(
    component_id: &str,
    field: &str,
    message: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(POWER_TREE_VALID, &scenario.name, message.to_string());
    finding.component = Some(component_id.to_string());
    finding
        .limit
        .insert("power_conversion_field".to_string(), json!(field));
    finding.suggested_fixes = vec![
        "Correct the component model power_conversion metadata before using it for power-tree validation.".to_string(),
        "Use analog_transient with an explicit regulator deck when static conversion metadata is insufficient.".to_string(),
    ];
    findings.push(finding);
}

fn power_conversion_pin_finding(
    component_id: &str,
    pin: &str,
    role: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!("Regulator {component_id} power_conversion {role}_pin {pin} is not connected."),
    );
    finding.component = Some(component_id.to_string());
    finding.limit.insert(format!("{role}_pin"), json!(pin));
    finding.suggested_fixes = vec![
        "Connect every declared power_conversion input and output pin to explicit power rails."
            .to_string(),
        "Correct the component model power_conversion pin names if they do not match the model ports."
            .to_string(),
    ];
    findings.push(finding);
}

fn power_switch_metadata_finding(
    component_id: &str,
    field: &str,
    message: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(POWER_TREE_VALID, &scenario.name, message.to_string());
    finding.component = Some(component_id.to_string());
    finding
        .limit
        .insert("power_switch_field".to_string(), json!(field));
    finding.suggested_fixes = vec![
        "Correct the component model power_switch metadata before using it for power-tree validation.".to_string(),
        "Use analog_transient with an explicit switch model when static switch metadata is insufficient.".to_string(),
    ];
    findings.push(finding);
}

fn power_switch_pin_finding(
    component_id: &str,
    pin: &str,
    role: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!("Load switch {component_id} power_switch {role}_pin {pin} is not connected."),
    );
    finding.component = Some(component_id.to_string());
    finding.limit.insert(format!("{role}_pin"), json!(pin));
    finding.suggested_fixes = vec![
        "Connect every declared power_switch input and output pin to explicit power rails."
            .to_string(),
        "Correct the component model power_switch pin names if they do not match the model ports."
            .to_string(),
    ];
    findings.push(finding);
}

fn battery_charger_metadata_finding(
    component_id: &str,
    field: &str,
    message: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(POWER_TREE_VALID, &scenario.name, message.to_string());
    finding.component = Some(component_id.to_string());
    finding
        .limit
        .insert("battery_charger_field".to_string(), json!(field));
    finding.suggested_fixes = vec![
        "Correct the component model battery_charger metadata before using it for power-tree validation.".to_string(),
        "Use analog_transient or a charger-specific model when static charger metadata is insufficient.".to_string(),
    ];
    findings.push(finding);
}

fn battery_charger_pin_finding(
    component_id: &str,
    pin: &str,
    role: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!(
            "Battery charger {component_id} battery_charger {role}_pin {pin} is not connected."
        ),
    );
    finding.component = Some(component_id.to_string());
    finding.limit.insert(format!("{role}_pin"), json!(pin));
    finding.suggested_fixes = vec![
        "Connect every declared battery_charger input and battery pin to explicit power rails."
            .to_string(),
        "Correct the component model battery_charger pin names if they do not match the model ports."
            .to_string(),
    ];
    findings.push(finding);
}

fn power_mux_metadata_finding(
    component_id: &str,
    field: &str,
    message: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(POWER_TREE_VALID, &scenario.name, message.to_string());
    finding.component = Some(component_id.to_string());
    finding
        .limit
        .insert("power_mux_field".to_string(), json!(field));
    finding.suggested_fixes = vec![
        "Correct the component model power_mux metadata before using it for power-tree validation.".to_string(),
        "Use analog_transient or a power-mux-specific model when static mux metadata is insufficient.".to_string(),
    ];
    findings.push(finding);
}

fn power_mux_pin_finding(
    component_id: &str,
    pin: &str,
    role: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!("Power mux {component_id} power_mux {role}_pin {pin} is not connected."),
    );
    finding.component = Some(component_id.to_string());
    finding.limit.insert(format!("{role}_pin"), json!(pin));
    finding.suggested_fixes = vec![
        "Connect every declared power_mux input and output pin to explicit power rails."
            .to_string(),
        "Correct the component model power_mux pin names if they do not match the model ports."
            .to_string(),
    ];
    findings.push(finding);
}

fn validate_power_net(
    component_id: &str,
    pin_name: &str,
    model: &ComponentModel,
    net_name: &str,
    net: &NetSpec,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    if net.kind != NetKind::Power {
        let mut finding = Finding::critical(
            POWER_TREE_VALID,
            &scenario.name,
            format!(
                "Power pin {component_id}.{pin_name} is connected to non-power net {net_name}."
            ),
        );
        finding.component = Some(component_id.to_string());
        finding.net = Some(net_name.to_string());
        finding.suggested_fixes = vec![
            "Connect power pins only to nets declared as kind: power.".to_string(),
            "If this is a passive or signal pin, correct the component model port kind."
                .to_string(),
        ];
        findings.push(finding);
        return;
    }

    if net.powered != Some(true) {
        let mut finding = Finding::critical(
            POWER_TREE_VALID,
            &scenario.name,
            format!("Power rail {net_name} for {component_id}.{pin_name} is not declared powered."),
        );
        finding.component = Some(component_id.to_string());
        finding.net = Some(net_name.to_string());
        finding
            .measured
            .insert("powered".to_string(), json!(net.powered));
        finding.limit.insert("powered".to_string(), json!(true));
        finding.suggested_fixes = vec![
            "Declare the rail powered only when a real source supplies it in this scenario.".to_string(),
            "Add or fix the upstream regulator, load switch, jumper, or connector source for this rail.".to_string(),
        ];
        findings.push(finding);
    }

    let Some(voltage_v) = net.nominal_voltage else {
        let mut finding = Finding::critical(
            POWER_TREE_VALID,
            &scenario.name,
            format!(
                "Power rail {net_name} for {component_id}.{pin_name} is missing nominal_voltage."
            ),
        );
        finding.component = Some(component_id.to_string());
        finding.net = Some(net_name.to_string());
        finding.suggested_fixes = vec![
            "Declare nominal_voltage for every powered rail that feeds active components.".to_string(),
            "Use analog_transient when nominal voltage depends on startup or load waveform behavior.".to_string(),
        ];
        findings.push(finding);
        return;
    };

    if !voltage_v.is_finite() || voltage_v <= 0.0 {
        let mut finding = Finding::critical(
            POWER_TREE_VALID,
            &scenario.name,
            format!(
                "Power rail {net_name} for {component_id}.{pin_name} has invalid nominal voltage {voltage_v}."
            ),
        );
        finding.component = Some(component_id.to_string());
        finding.net = Some(net_name.to_string());
        finding
            .measured
            .insert("nominal_voltage_V".to_string(), json!(voltage_v));
        finding.suggested_fixes = vec![
            "Use a finite positive nominal_voltage for active power rails.".to_string(),
            "Use kind: ground for the zero-volt reference net instead of a zero-volt power rail."
                .to_string(),
        ];
        findings.push(finding);
        return;
    }

    let Some(port) = model.ports.get(pin_name) else {
        return;
    };
    let min_v = port.electrical.operating_voltage_min_v;
    let max_v = port.electrical.operating_voltage_max_v;
    if let Some(min_v) = min_v
        && voltage_v < min_v
    {
        voltage_range_finding(
            PowerVoltageContext {
                component_id,
                pin_name,
                net_name,
                voltage_v,
                scenario,
            },
            "minimum",
            min_v,
            findings,
        );
    }
    if let Some(max_v) = max_v
        && voltage_v > max_v
    {
        voltage_range_finding(
            PowerVoltageContext {
                component_id,
                pin_name,
                net_name,
                voltage_v,
                scenario,
            },
            "maximum",
            max_v,
            findings,
        );
    }
}

fn voltage_range_finding(
    context: PowerVoltageContext<'_>,
    limit_name: &str,
    limit_v: f64,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &context.scenario.name,
        format!(
            "Power rail {net_name} supplies {component_id}.{pin_name} at {:.6} V, outside the model {limit_name} operating voltage {:.6} V.",
            context.voltage_v,
            limit_v,
            net_name = context.net_name,
            component_id = context.component_id,
            pin_name = context.pin_name,
        ),
    );
    finding.component = Some(context.component_id.to_string());
    finding.net = Some(context.net_name.to_string());
    finding
        .measured
        .insert("nominal_voltage_V".to_string(), json!(context.voltage_v));
    finding
        .limit
        .insert(format!("operating_voltage_{limit_name}_V"), json!(limit_v));
    finding.suggested_fixes = vec![
        "Select a regulator or rail voltage inside the component operating range.".to_string(),
        "Move the component power pin to the correct rail or use a level/power-domain translation part where required.".to_string(),
    ];
    findings.push(finding);
}

fn resolve_power_net<'a>(component: &'a ComponentSpec, pin_name: &str) -> Option<&'a str> {
    component
        .power_domains
        .get(pin_name)
        .or_else(|| component.pins.get(pin_name))
        .or(component.power_domain.as_ref())
        .map(String::as_str)
}

fn is_inactive_power_mux_input(
    component: &ComponentSpec,
    model: &ComponentModel,
    pin_name: &str,
    net: &NetSpec,
) -> bool {
    let Some(mux) = &model.power_mux else {
        return false;
    };
    if net.powered != Some(false) {
        return false;
    }
    let Some(input) = mux.inputs.iter().find(|input| input.input_pin == pin_name) else {
        return false;
    };
    let Some(parameter) = mux.selected_input_parameter.as_deref() else {
        return false;
    };
    let Some(selected) = component
        .parameters
        .get(parameter)
        .and_then(serde_yaml_ng::Value::as_str)
    else {
        return false;
    };
    selected != input.name
}

fn is_supply_source(model: &ComponentModel) -> bool {
    matches!(
        model.category.as_str(),
        "voltage_source"
            | "regulator"
            | "power_source"
            | "load_switch"
            | "battery_charger"
            | "power_mux"
    )
}

fn power_switch_pin_state_matches(state: &PinLogicState, required: &PowerSwitchState) -> bool {
    matches!(
        (state, required),
        (PinLogicState::High, PowerSwitchState::High) | (PinLogicState::Low, PowerSwitchState::Low)
    )
}

fn power_switch_state_name(state: &PowerSwitchState) -> &'static str {
    match state {
        PowerSwitchState::High => "high",
        PowerSwitchState::Low => "low",
    }
}

fn pin_logic_state_name(state: &PinLogicState) -> &'static str {
    match state {
        PinLogicState::High => "high",
        PinLogicState::Low => "low",
        PinLogicState::Z => "z",
    }
}

struct PowerLoad {
    component: String,
    pin: String,
    max_current_a: Option<f64>,
}

struct PowerVoltageContext<'a> {
    component_id: &'a str,
    pin_name: &'a str,
    net_name: &'a str,
    voltage_v: f64,
    scenario: &'a Scenario,
}
