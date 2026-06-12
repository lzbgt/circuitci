use crate::board_ir::{ComponentSpec, NetSpec, Scenario};
use crate::library::{BoundBoard, ComponentModel, PortKind};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeMap;

use super::{PowerLoad, resolve_power_net};
use crate::validation::POWER_TREE_VALID;

pub(super) fn validate_power_mux(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    loads_by_net: &BTreeMap<String, Vec<PowerLoad>>,
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

    if let Some(max_output_current_a) = mux.max_output_current_a {
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
                    "Power mux {component_id} output current limit requires load metadata for {}.",
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
                "power_mux_max_output_current_A".to_string(),
                json!(max_output_current_a),
            );
            finding.suggested_fixes = vec![
                "Add max_supply_current_A to loads fed by the mux output rail.".to_string(),
                "Split the scenario if mux-fed loads are not all enabled simultaneously."
                    .to_string(),
            ];
            findings.push(finding);
        } else if total_a > max_output_current_a {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Power mux {component_id} worst-case output load {:.6} A exceeds mux limit {:.6} A.",
                    total_a, max_output_current_a
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(output_net_name.to_string());
            finding
                .measured
                .insert("declared_output_load_current_A".to_string(), json!(total_a));
            finding.limit.insert(
                "power_mux_max_output_current_A".to_string(),
                json!(max_output_current_a),
            );
            finding.suggested_fixes = vec![
                "Select a power mux with sufficient current and thermal margin.".to_string(),
                "Reduce or sequence loads on the mux output rail, or split high-current loads across separate power paths.".to_string(),
            ];
            findings.push(finding);
        }
    }
}

pub(super) fn is_inactive_power_mux_input(
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
    if let Some(max_output_current_a) = mux.max_output_current_a
        && (!max_output_current_a.is_finite() || max_output_current_a < 0.0)
    {
        power_mux_metadata_finding(
            component_id,
            "max_output_current_A",
            "power_mux max_output_current_A must be finite and non-negative.",
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
