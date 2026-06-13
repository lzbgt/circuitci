use crate::board_ir::{ComponentSpec, NetKind, NetSpec, Scenario, SpicePrimitive};
use crate::library::{BoundBoard, ComponentModel, PortKind};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeMap;

use super::{PowerLoad, resolve_power_net};
use crate::validation::POWER_TREE_VALID;

pub(super) fn validate_power_conversion(
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

    if let Some(min_output_current_a) = conversion.min_output_current_a {
        let loads = loads_by_net
            .get(output_net_name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let total_a = loads
            .iter()
            .filter_map(|load| load.min_current_a)
            .filter(|current| current.is_finite() && *current >= 0.0)
            .sum::<f64>();
        if total_a < min_output_current_a {
            let mut finding = Finding::critical(
                POWER_TREE_VALID,
                &scenario.name,
                format!(
                    "Regulator {component_id} proven minimum output load {:.6} A is below required minimum load {:.6} A.",
                    total_a, min_output_current_a
                ),
            );
            finding.component = Some(component_id.to_string());
            finding.net = Some(output_net_name.to_string());
            finding.measured.insert(
                "declared_minimum_output_load_current_A".to_string(),
                json!(total_a),
            );
            finding.limit.insert(
                "regulator_min_output_current_A".to_string(),
                json!(min_output_current_a),
            );
            finding.suggested_fixes = vec![
                "Add a bleeder or always-on load so the regulator meets its datasheet minimum load requirement.".to_string(),
                "Add min_supply_current_A metadata for always-on loads when the schematic already provides enough minimum load.".to_string(),
                "Select a regulator that remains in regulation at the board's actual no-load condition.".to_string(),
            ];
            findings.push(finding);
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

    if let Some(min_input_capacitance_f) = conversion.input_capacitance_min_f {
        validate_regulator_support_capacitance(
            RegulatorCapacitanceContext {
                component_id,
                pin: &conversion.input_pin,
                role: "input",
                net_name: input_net_name,
                min_capacitance_f: min_input_capacitance_f,
            },
            bound,
            scenario,
            findings,
        );
    }
    if let Some(min_output_capacitance_f) = conversion.output_capacitance_min_f {
        validate_regulator_support_capacitance(
            RegulatorCapacitanceContext {
                component_id,
                pin: &conversion.output_pin,
                role: "output",
                net_name: output_net_name,
                min_capacitance_f: min_output_capacitance_f,
            },
            bound,
            scenario,
            findings,
        );
    }
    if conversion.input_inductance_min_h.is_some() || conversion.input_inductance_max_h.is_some() {
        let Some(switch_pin) = conversion.switch_pin.as_deref() else {
            power_conversion_metadata_finding(
                component_id,
                "switch_pin",
                "power_conversion switch_pin is required when input inductance limits are declared.",
                scenario,
                findings,
            );
            return;
        };
        let Some(switch_net_name) = component.pins.get(switch_pin).map(String::as_str) else {
            power_conversion_pin_finding(component_id, switch_pin, "switch", scenario, findings);
            return;
        };
        validate_regulator_input_inductance(
            RegulatorInductanceContext {
                component_id,
                switch_pin,
                switch_net_name,
                input_pin: &conversion.input_pin,
                input_net_name,
                output_pin: &conversion.output_pin,
                output_net_name,
                min_inductance_h: conversion.input_inductance_min_h,
                max_inductance_h: conversion.input_inductance_max_h,
            },
            bound,
            scenario,
            findings,
        );
    }
    if conversion.output_inductance_min_h.is_some() || conversion.output_inductance_max_h.is_some()
    {
        let Some(switch_pin) = conversion.switch_pin.as_deref() else {
            power_conversion_metadata_finding(
                component_id,
                "switch_pin",
                "power_conversion switch_pin is required when output inductance limits are declared.",
                scenario,
                findings,
            );
            return;
        };
        let Some(switch_net_name) = component.pins.get(switch_pin).map(String::as_str) else {
            power_conversion_pin_finding(component_id, switch_pin, "switch", scenario, findings);
            return;
        };
        validate_regulator_output_inductance(
            RegulatorInductanceContext {
                component_id,
                switch_pin,
                switch_net_name,
                input_pin: &conversion.input_pin,
                input_net_name,
                output_pin: &conversion.output_pin,
                output_net_name,
                min_inductance_h: conversion.output_inductance_min_h,
                max_inductance_h: conversion.output_inductance_max_h,
            },
            bound,
            scenario,
            findings,
        );
    }
    if conversion.switch_inductance_min_h.is_some() || conversion.switch_inductance_max_h.is_some()
    {
        let Some(pin_a) = conversion.switch_inductor_pin_a.as_deref() else {
            power_conversion_metadata_finding(
                component_id,
                "switch_inductor_pin_a",
                "power_conversion switch_inductor_pin_a is required when switch inductance limits are declared.",
                scenario,
                findings,
            );
            return;
        };
        let Some(pin_b) = conversion.switch_inductor_pin_b.as_deref() else {
            power_conversion_metadata_finding(
                component_id,
                "switch_inductor_pin_b",
                "power_conversion switch_inductor_pin_b is required when switch inductance limits are declared.",
                scenario,
                findings,
            );
            return;
        };
        let Some(net_a) = component.pins.get(pin_a).map(String::as_str) else {
            power_conversion_pin_finding(
                component_id,
                pin_a,
                "switch_inductor_a",
                scenario,
                findings,
            );
            return;
        };
        let Some(net_b) = component.pins.get(pin_b).map(String::as_str) else {
            power_conversion_pin_finding(
                component_id,
                pin_b,
                "switch_inductor_b",
                scenario,
                findings,
            );
            return;
        };
        validate_regulator_switch_inductance(
            SwitchInductanceContext {
                component_id,
                pin_a,
                net_a,
                pin_b,
                net_b,
                min_inductance_h: conversion.switch_inductance_min_h,
                max_inductance_h: conversion.switch_inductance_max_h,
            },
            bound,
            scenario,
            findings,
        );
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
    if let Some(switch_pin) = conversion.switch_pin.as_deref() {
        if switch_pin == conversion.input_pin || switch_pin == conversion.output_pin {
            power_conversion_metadata_finding(
                component_id,
                "switch_pin",
                "power_conversion switch_pin must be distinct from input_pin and output_pin.",
                scenario,
                findings,
            );
            valid = false;
        }
        if !model.ports.contains_key(switch_pin) {
            power_conversion_metadata_finding(
                component_id,
                "switch_pin",
                &format!(
                    "power_conversion switch_pin {switch_pin} is not declared in model ports."
                ),
                scenario,
                findings,
            );
            valid = false;
        }
    } else if conversion.input_inductance_min_h.is_some()
        || conversion.input_inductance_max_h.is_some()
        || conversion.output_inductance_min_h.is_some()
        || conversion.output_inductance_max_h.is_some()
    {
        power_conversion_metadata_finding(
            component_id,
            "switch_pin",
            "power_conversion switch_pin is required when inductance limits are declared.",
            scenario,
            findings,
        );
        valid = false;
    }
    match (
        conversion.switch_inductor_pin_a.as_deref(),
        conversion.switch_inductor_pin_b.as_deref(),
    ) {
        (Some(pin_a), Some(pin_b)) => {
            if pin_a == pin_b {
                power_conversion_metadata_finding(
                    component_id,
                    "switch_inductor_pin_a",
                    "power_conversion switch_inductor_pin_a and switch_inductor_pin_b must be distinct.",
                    scenario,
                    findings,
                );
                valid = false;
            }
            for (role, pin) in [
                ("switch_inductor_pin_a", pin_a),
                ("switch_inductor_pin_b", pin_b),
            ] {
                if !model.ports.contains_key(pin) {
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
        (None, None) => {
            if conversion.switch_inductance_min_h.is_some()
                || conversion.switch_inductance_max_h.is_some()
            {
                power_conversion_metadata_finding(
                    component_id,
                    "switch_inductor_pin_a",
                    "power_conversion switch_inductor_pin_a and switch_inductor_pin_b are required when switch inductance limits are declared.",
                    scenario,
                    findings,
                );
                valid = false;
            }
        }
        _ => {
            power_conversion_metadata_finding(
                component_id,
                "switch_inductor_pin_a",
                "power_conversion switch_inductor_pin_a and switch_inductor_pin_b must be declared together.",
                scenario,
                findings,
            );
            valid = false;
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
    if let Some(min_output_current_a) = conversion.min_output_current_a
        && (!min_output_current_a.is_finite() || min_output_current_a < 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "min_output_current_A",
            "power_conversion min_output_current_A must be finite and non-negative.",
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
    if let Some(input_capacitance_min_f) = conversion.input_capacitance_min_f
        && (!input_capacitance_min_f.is_finite() || input_capacitance_min_f <= 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "input_capacitance_min_F",
            "power_conversion input_capacitance_min_F must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(output_capacitance_min_f) = conversion.output_capacitance_min_f
        && (!output_capacitance_min_f.is_finite() || output_capacitance_min_f <= 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "output_capacitance_min_F",
            "power_conversion output_capacitance_min_F must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(input_inductance_min_h) = conversion.input_inductance_min_h
        && (!input_inductance_min_h.is_finite() || input_inductance_min_h <= 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "input_inductance_min_H",
            "power_conversion input_inductance_min_H must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(input_inductance_max_h) = conversion.input_inductance_max_h
        && (!input_inductance_max_h.is_finite() || input_inductance_max_h <= 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "input_inductance_max_H",
            "power_conversion input_inductance_max_H must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let (Some(min_h), Some(max_h)) = (
        conversion.input_inductance_min_h,
        conversion.input_inductance_max_h,
    ) && min_h > max_h
    {
        power_conversion_metadata_finding(
            component_id,
            "input_inductance_min_H",
            "power_conversion input_inductance_min_H must not exceed input_inductance_max_H.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(output_inductance_min_h) = conversion.output_inductance_min_h
        && (!output_inductance_min_h.is_finite() || output_inductance_min_h <= 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "output_inductance_min_H",
            "power_conversion output_inductance_min_H must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(output_inductance_max_h) = conversion.output_inductance_max_h
        && (!output_inductance_max_h.is_finite() || output_inductance_max_h <= 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "output_inductance_max_H",
            "power_conversion output_inductance_max_H must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let (Some(min_h), Some(max_h)) = (
        conversion.output_inductance_min_h,
        conversion.output_inductance_max_h,
    ) && min_h > max_h
    {
        power_conversion_metadata_finding(
            component_id,
            "output_inductance_min_H",
            "power_conversion output_inductance_min_H must not exceed output_inductance_max_H.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(switch_inductance_min_h) = conversion.switch_inductance_min_h
        && (!switch_inductance_min_h.is_finite() || switch_inductance_min_h <= 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "switch_inductance_min_H",
            "power_conversion switch_inductance_min_H must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(switch_inductance_max_h) = conversion.switch_inductance_max_h
        && (!switch_inductance_max_h.is_finite() || switch_inductance_max_h <= 0.0)
    {
        power_conversion_metadata_finding(
            component_id,
            "switch_inductance_max_H",
            "power_conversion switch_inductance_max_H must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let (Some(min_h), Some(max_h)) = (
        conversion.switch_inductance_min_h,
        conversion.switch_inductance_max_h,
    ) && min_h > max_h
    {
        power_conversion_metadata_finding(
            component_id,
            "switch_inductance_min_H",
            "power_conversion switch_inductance_min_H must not exceed switch_inductance_max_H.",
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

fn validate_regulator_support_capacitance(
    context: RegulatorCapacitanceContext<'_>,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let (support_capacitance_f, support_capacitors) =
        support_capacitance_to_ground(bound, context.net_name);
    if support_capacitance_f >= context.min_capacitance_f {
        return;
    }

    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!(
            "Regulator {component_id} {role} rail {net_name} has {:.6e} F support capacitance to ground, below required {:.6e} F.",
            support_capacitance_f,
            context.min_capacitance_f,
            component_id = context.component_id,
            role = context.role,
            net_name = context.net_name,
        ),
    );
    finding.component = Some(context.component_id.to_string());
    finding.net = Some(context.net_name.to_string());
    finding.measured.insert(
        "support_capacitance_F".to_string(),
        json!(support_capacitance_f),
    );
    finding
        .measured
        .insert("support_capacitors".to_string(), json!(support_capacitors));
    finding
        .limit
        .insert("power_conversion_pin".to_string(), json!(context.pin));
    finding.limit.insert(
        format!("regulator_{}_capacitance_min_F", context.role),
        json!(context.min_capacitance_f),
    );
    finding.suggested_fixes = vec![
        format!(
            "Add at least {:.6e} F effective capacitance from {} rail {} to ground near regulator {}.{}.",
            context.min_capacitance_f, context.role, context.net_name, context.component_id, context.pin
        ),
        "Map the schematic capacitor value into Board IR when the capacitor is present but not modeled.".to_string(),
        "Use analog_transient or a regulator-specific stability model for ESR, ESL, DC bias, temperature, and layout-dependent stability sign-off.".to_string(),
    ];
    findings.push(finding);
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

fn validate_regulator_output_inductance(
    context: RegulatorInductanceContext<'_>,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let (output_inductance_h, output_inductors) =
        direct_inductance_between_nets(bound, context.switch_net_name, context.output_net_name);
    let below_min = context
        .min_inductance_h
        .is_some_and(|min_h| output_inductance_h < min_h);
    let above_max = context
        .max_inductance_h
        .is_some_and(|max_h| output_inductance_h > max_h);
    if !below_min && !above_max {
        return;
    }

    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!(
            "Regulator {component_id} output inductor path {switch_net_name}->{output_net_name} has {:.6e} H direct inductance, outside the modeled support range.",
            output_inductance_h,
            component_id = context.component_id,
            switch_net_name = context.switch_net_name,
            output_net_name = context.output_net_name,
        ),
    );
    finding.component = Some(context.component_id.to_string());
    finding.net = Some(context.output_net_name.to_string());
    finding.measured.insert(
        "output_inductance_H".to_string(),
        json!(output_inductance_h),
    );
    finding
        .measured
        .insert("output_inductors".to_string(), json!(output_inductors));
    finding
        .measured
        .insert("switch_net".to_string(), json!(context.switch_net_name));
    finding
        .measured
        .insert("output_net".to_string(), json!(context.output_net_name));
    finding
        .limit
        .insert("switch_pin".to_string(), json!(context.switch_pin));
    finding
        .limit
        .insert("output_pin".to_string(), json!(context.output_pin));
    if let Some(min_h) = context.min_inductance_h {
        finding.limit.insert(
            "regulator_output_inductance_min_H".to_string(),
            json!(min_h),
        );
    }
    if let Some(max_h) = context.max_inductance_h {
        finding.limit.insert(
            "regulator_output_inductance_max_H".to_string(),
            json!(max_h),
        );
    }
    finding.suggested_fixes = vec![
        format!(
            "Add a modeled output inductor directly between regulator switch net {} and output rail {} with inductance inside the datasheet-backed range.",
            context.switch_net_name, context.output_net_name
        ),
        "Map the schematic inductor value into Board IR when the inductor is present but not modeled.".to_string(),
        "Use regulator-specific design tools or analog simulation for saturation current, ripple current, DCR, loop stability, and layout sign-off.".to_string(),
    ];
    findings.push(finding);
}

fn validate_regulator_input_inductance(
    context: RegulatorInductanceContext<'_>,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let (input_inductance_h, input_inductors) =
        direct_inductance_between_nets(bound, context.input_net_name, context.switch_net_name);
    let below_min = context
        .min_inductance_h
        .is_some_and(|min_h| input_inductance_h < min_h);
    let above_max = context
        .max_inductance_h
        .is_some_and(|max_h| input_inductance_h > max_h);
    if !below_min && !above_max {
        return;
    }

    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!(
            "Regulator {component_id} input inductor path {input_net_name}->{switch_net_name} has {:.6e} H direct inductance, outside the modeled support range.",
            input_inductance_h,
            component_id = context.component_id,
            input_net_name = context.input_net_name,
            switch_net_name = context.switch_net_name,
        ),
    );
    finding.component = Some(context.component_id.to_string());
    finding.net = Some(context.input_net_name.to_string());
    finding
        .measured
        .insert("input_inductance_H".to_string(), json!(input_inductance_h));
    finding
        .measured
        .insert("input_inductors".to_string(), json!(input_inductors));
    finding
        .measured
        .insert("input_net".to_string(), json!(context.input_net_name));
    finding
        .measured
        .insert("switch_net".to_string(), json!(context.switch_net_name));
    finding
        .limit
        .insert("input_pin".to_string(), json!(context.input_pin));
    finding
        .limit
        .insert("switch_pin".to_string(), json!(context.switch_pin));
    if let Some(min_h) = context.min_inductance_h {
        finding
            .limit
            .insert("regulator_input_inductance_min_H".to_string(), json!(min_h));
    }
    if let Some(max_h) = context.max_inductance_h {
        finding
            .limit
            .insert("regulator_input_inductance_max_H".to_string(), json!(max_h));
    }
    finding.suggested_fixes = vec![
        format!(
            "Add a modeled input inductor directly between regulator input rail {} and switch net {} with inductance inside the datasheet-backed range.",
            context.input_net_name, context.switch_net_name
        ),
        "Map the schematic inductor value into Board IR when the inductor is present but not modeled.".to_string(),
        "Use regulator-specific design tools or analog simulation for saturation current, ripple current, DCR, loop stability, and layout sign-off.".to_string(),
    ];
    findings.push(finding);
}

fn validate_regulator_switch_inductance(
    context: SwitchInductanceContext<'_>,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let (switch_inductance_h, switch_inductors) =
        direct_inductance_between_nets(bound, context.net_a, context.net_b);
    let below_min = context
        .min_inductance_h
        .is_some_and(|min_h| switch_inductance_h < min_h);
    let above_max = context
        .max_inductance_h
        .is_some_and(|max_h| switch_inductance_h > max_h);
    if !below_min && !above_max {
        return;
    }

    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!(
            "Regulator {component_id} switch inductor path {net_a}->{net_b} has {:.6e} H direct inductance, outside the modeled support range.",
            switch_inductance_h,
            component_id = context.component_id,
            net_a = context.net_a,
            net_b = context.net_b,
        ),
    );
    finding.component = Some(context.component_id.to_string());
    finding.net = Some(context.net_a.to_string());
    finding.measured.insert(
        "switch_inductance_H".to_string(),
        json!(switch_inductance_h),
    );
    finding
        .measured
        .insert("switch_inductors".to_string(), json!(switch_inductors));
    finding
        .measured
        .insert("switch_inductor_net_a".to_string(), json!(context.net_a));
    finding
        .measured
        .insert("switch_inductor_net_b".to_string(), json!(context.net_b));
    finding
        .limit
        .insert("switch_inductor_pin_a".to_string(), json!(context.pin_a));
    finding
        .limit
        .insert("switch_inductor_pin_b".to_string(), json!(context.pin_b));
    if let Some(min_h) = context.min_inductance_h {
        finding.limit.insert(
            "regulator_switch_inductance_min_H".to_string(),
            json!(min_h),
        );
    }
    if let Some(max_h) = context.max_inductance_h {
        finding.limit.insert(
            "regulator_switch_inductance_max_H".to_string(),
            json!(max_h),
        );
    }
    finding.suggested_fixes = vec![
        format!(
            "Add a modeled inductor directly between regulator switch nets {} and {} with inductance inside the datasheet-backed range.",
            context.net_a, context.net_b
        ),
        "Map the schematic inductor value into Board IR when the inductor is present but not modeled.".to_string(),
        "Use regulator-specific design tools or analog simulation for saturation current, ripple current, DCR, loop stability, and layout sign-off.".to_string(),
    ];
    findings.push(finding);
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

struct RegulatorStartupContext<'a> {
    component_id: &'a str,
    input_net_name: &'a str,
    input_net: &'a NetSpec,
    output_net_name: &'a str,
    output_net: &'a NetSpec,
    startup_delay_us: f64,
}

struct RegulatorCapacitanceContext<'a> {
    component_id: &'a str,
    pin: &'a str,
    role: &'a str,
    net_name: &'a str,
    min_capacitance_f: f64,
}

struct RegulatorInductanceContext<'a> {
    component_id: &'a str,
    switch_pin: &'a str,
    switch_net_name: &'a str,
    input_pin: &'a str,
    input_net_name: &'a str,
    output_pin: &'a str,
    output_net_name: &'a str,
    min_inductance_h: Option<f64>,
    max_inductance_h: Option<f64>,
}

struct SwitchInductanceContext<'a> {
    component_id: &'a str,
    pin_a: &'a str,
    net_a: &'a str,
    pin_b: &'a str,
    net_b: &'a str,
    min_inductance_h: Option<f64>,
    max_inductance_h: Option<f64>,
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
