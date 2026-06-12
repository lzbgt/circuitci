use crate::board_ir::{ComponentSpec, NetKind, NetSpec, Scenario};
use crate::library::{BoundBoard, ComponentModel, PortKind};
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

            validate_power_net(
                component_id,
                pin_name,
                model,
                net_name,
                net,
                scenario,
                findings,
            );

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

    for (net_name, loads) in loads_by_net {
        let Some(net) = bound.project.board.nets.get(&net_name) else {
            continue;
        };
        let Some(limit_a) = net.supply_current_limit_a else {
            continue;
        };
        let mut total_a = 0.0;
        let mut missing_loads = Vec::new();
        for load in &loads {
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
            finding.net = Some(net_name);
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
            finding.net = Some(net_name);
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

fn is_supply_source(model: &ComponentModel) -> bool {
    matches!(
        model.category.as_str(),
        "voltage_source" | "regulator" | "power_source"
    )
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
