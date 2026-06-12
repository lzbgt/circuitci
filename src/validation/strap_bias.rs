use crate::board_ir::{BoardProject, NetKind, Scenario, SpicePrimitive};
use crate::library::{BoundBoard, Port};
use crate::reports::Finding;
use serde_json::json;

use super::BOOT_STRAP_BIAS_VALID;
use super::common::{target_model, validation_input_missing};

pub(super) fn validate_boot_strap_bias(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some((target_component, model)) = target_model(bound, scenario) else {
        validation_input_missing(
            findings,
            scenario,
            "reset_boot target component and model are required for boot strap bias validation.",
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
    let current_limit_a = scenario
        .parameters
        .get("max_strap_bias_current_A")
        .and_then(serde_yaml_ng::Value::as_f64);

    for requirement in &mode.straps {
        let Some(strap_net) = bound
            .project
            .net_for_pin(&target_component, &requirement.pin)
            .map(str::to_string)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Boot strap {}.{} does not resolve to a board net.",
                    target_component, requirement.pin
                ),
            );
            continue;
        };
        let Some(port) = model.ports.get(&requirement.pin) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Component model {} does not declare boot strap port {}.",
                    model.component_id, requirement.pin
                ),
            );
            continue;
        };
        let Some(network) = strap_bias_network(bound.project, &strap_net, findings, scenario)
        else {
            continue;
        };
        if network.conductance_s <= 0.0 {
            floating_strap_finding(
                scenario,
                &target_component,
                &requirement.pin,
                &strap_net,
                required_boot_mode,
                &requirement.required_state,
                findings,
            );
            continue;
        }
        let voltage_v = network.weighted_voltage / network.conductance_s;
        let bias_current_a = network
            .sources
            .iter()
            .map(|source| (source.voltage_v - voltage_v).max(0.0) * source.conductance_s)
            .sum::<f64>();
        let context = StrapCheckContext {
            scenario,
            component: &target_component,
            pin: &requirement.pin,
            net: &strap_net,
        };
        validate_threshold(
            context,
            required_boot_mode,
            &requirement.required_state,
            port,
            voltage_v,
            findings,
        );
        if let Some(limit_a) = current_limit_a {
            validate_current_limit(context, voltage_v, bias_current_a, limit_a, findings);
        }
    }
}

fn strap_bias_network(
    project: &BoardProject,
    strap_net: &str,
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
) -> Option<BiasNetwork> {
    let mut network = BiasNetwork::default();
    for (component_id, component) in &project.board.components {
        let Some(spice) = &component.spice else {
            continue;
        };
        if spice.primitive != SpicePrimitive::Resistor {
            continue;
        }
        if !component.pins.values().any(|net| net == strap_net) {
            continue;
        }
        let Some(value_ohm) = spice.value_ohm else {
            invalid_resistor_finding(
                scenario,
                component_id,
                strap_net,
                "strap bias resistor is missing spice.value_ohm",
                findings,
            );
            return None;
        };
        if !value_ohm.is_finite() || value_ohm <= 0.0 {
            invalid_resistor_finding(
                scenario,
                component_id,
                strap_net,
                "strap bias resistor has invalid spice.value_ohm",
                findings,
            );
            return None;
        }
        let connected: Vec<&str> = component
            .pins
            .values()
            .map(String::as_str)
            .filter(|net| *net != strap_net)
            .collect();
        if connected.len() != 1 {
            continue;
        }
        let Some(source_voltage_v) = bias_source_voltage(project, connected[0], findings, scenario)
        else {
            continue;
        };
        let conductance_s = 1.0 / value_ohm;
        network.weighted_voltage += source_voltage_v * conductance_s;
        network.conductance_s += conductance_s;
        network.sources.push(BiasSource {
            voltage_v: source_voltage_v,
            conductance_s,
        });
    }
    Some(network)
}

fn bias_source_voltage(
    project: &BoardProject,
    net_name: &str,
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
) -> Option<f64> {
    let Some(net) = project.board.nets.get(net_name) else {
        validation_input_missing(
            findings,
            scenario,
            format!("Strap bias source net {net_name} is not declared."),
        );
        return None;
    };
    match net.kind {
        NetKind::Ground => Some(0.0),
        NetKind::Power => match (net.powered, net.nominal_voltage) {
            (Some(true), Some(voltage_v)) if voltage_v.is_finite() => Some(voltage_v),
            (Some(false), _) => Some(0.0),
            _ => {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "Strap bias power net {net_name} must declare powered and nominal_voltage."
                    ),
                );
                None
            }
        },
        NetKind::DigitalOrAnalog => None,
    }
}

fn validate_threshold(
    context: StrapCheckContext<'_>,
    required_boot_mode: &str,
    required_state: &str,
    port: &Port,
    voltage_v: f64,
    findings: &mut Vec<Finding>,
) {
    match required_state.trim().to_ascii_lowercase().as_str() {
        "high" => {
            let Some(vih_min_v) = port.electrical.vih_min_v else {
                validation_input_missing(
                    findings,
                    context.scenario,
                    format!(
                        "Boot strap {}.{} requires high but model lacks vih_min_V.",
                        context.component, context.pin
                    ),
                );
                return;
            };
            if voltage_v < vih_min_v {
                threshold_finding(
                    ThresholdFinding {
                        scenario: context.scenario,
                        component: context.component,
                        pin: context.pin,
                        net: context.net,
                        required_boot_mode,
                        required_state: "high",
                        voltage_v,
                        limit_key: "vih_min_V",
                        limit_v: vih_min_v,
                    },
                    findings,
                );
            }
        }
        "low" => {
            let Some(vil_max_v) = port.electrical.vil_max_v else {
                validation_input_missing(
                    findings,
                    context.scenario,
                    format!(
                        "Boot strap {}.{} requires low but model lacks vil_max_V.",
                        context.component, context.pin
                    ),
                );
                return;
            };
            if voltage_v > vil_max_v {
                threshold_finding(
                    ThresholdFinding {
                        scenario: context.scenario,
                        component: context.component,
                        pin: context.pin,
                        net: context.net,
                        required_boot_mode,
                        required_state: "low",
                        voltage_v,
                        limit_key: "vil_max_V",
                        limit_v: vil_max_v,
                    },
                    findings,
                );
            }
        }
        other => validation_input_missing(
            findings,
            context.scenario,
            format!(
                "Boot strap {}.{} has unsupported required state {other}.",
                context.component, context.pin
            ),
        ),
    }
}

fn validate_current_limit(
    context: StrapCheckContext<'_>,
    voltage_v: f64,
    bias_current_a: f64,
    limit_a: f64,
    findings: &mut Vec<Finding>,
) {
    if !limit_a.is_finite() || limit_a < 0.0 {
        validation_input_missing(
            findings,
            context.scenario,
            "parameters.max_strap_bias_current_A must be finite and non-negative.",
        );
        return;
    }
    if bias_current_a <= limit_a {
        return;
    }
    let mut finding = Finding::critical(
        BOOT_STRAP_BIAS_VALID,
        &context.scenario.name,
        format!(
            "Boot strap {}.{} resistor network draws {:.6} A on net {}, above limit {:.6} A.",
            context.component, context.pin, bias_current_a, context.net, limit_a
        ),
    );
    finding.component = Some(context.component.to_string());
    finding.net = Some(context.net.to_string());
    finding
        .measured
        .insert("strap_voltage_V".to_string(), json!(voltage_v));
    finding
        .measured
        .insert("strap_bias_current_A".to_string(), json!(bias_current_a));
    finding
        .limit
        .insert("max_strap_bias_current_A".to_string(), json!(limit_a));
    finding.suggested_fixes = vec![
        "Increase strap resistor values while preserving VIH/VIL margin.".to_string(),
        "Avoid divider networks that waste excessive rail current.".to_string(),
    ];
    findings.push(finding);
}

fn threshold_finding(context: ThresholdFinding<'_>, findings: &mut Vec<Finding>) {
    let mut finding = Finding::critical(
        BOOT_STRAP_BIAS_VALID,
        &context.scenario.name,
        format!(
            "Boot strap {}.{} resistor network produces {:.6} V on net {}, not valid for required {} state in boot mode {}.",
            context.component,
            context.pin,
            context.voltage_v,
            context.net,
            context.required_state,
            context.required_boot_mode
        ),
    );
    finding.component = Some(context.component.to_string());
    finding.net = Some(context.net.to_string());
    finding
        .measured
        .insert("strap_voltage_V".to_string(), json!(context.voltage_v));
    finding.measured.insert(
        "required_boot_mode".to_string(),
        json!(context.required_boot_mode),
    );
    finding.limit.insert(
        format!("required_{}", context.pin),
        json!(context.required_state),
    );
    finding
        .limit
        .insert(context.limit_key.to_string(), json!(context.limit_v));
    finding.suggested_fixes = vec![
        "Resize the pull-up/pull-down resistor divider so the sampled strap voltage is outside the undefined region.".to_string(),
        "Tie the strap to the intended rail through a single appropriate pull resistor when a divider is not required.".to_string(),
        "Use an analog transient scenario if host control circuitry drives this strap during reset release.".to_string(),
    ];
    findings.push(finding);
}

fn floating_strap_finding(
    scenario: &Scenario,
    component: &str,
    pin: &str,
    net: &str,
    required_boot_mode: &str,
    required_state: &str,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        BOOT_STRAP_BIAS_VALID,
        &scenario.name,
        format!(
            "Boot strap {component}.{pin} net {net} has no resistor bias to a declared power or ground net."
        ),
    );
    finding.component = Some(component.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("required_boot_mode".to_string(), json!(required_boot_mode));
    finding
        .measured
        .insert("strap_bias_sources".to_string(), json!(0));
    finding
        .limit
        .insert(format!("required_{pin}"), json!(required_state));
    finding.suggested_fixes = vec![
        "Add an explicit pull-up or pull-down resistor to the required boot strap net.".to_string(),
        "Declare resistor primitive values so the static strap divider can be checked.".to_string(),
    ];
    findings.push(finding);
}

fn invalid_resistor_finding(
    scenario: &Scenario,
    component: &str,
    net: &str,
    reason: &str,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        BOOT_STRAP_BIAS_VALID,
        &scenario.name,
        format!("Component {component} on strap net {net} cannot be used: {reason}."),
    );
    finding.component = Some(component.to_string());
    finding.net = Some(net.to_string());
    finding.suggested_fixes = vec![
        "Map strap resistors as spice primitive resistor components with positive value_ohm."
            .to_string(),
    ];
    findings.push(finding);
}

#[derive(Default)]
struct BiasNetwork {
    conductance_s: f64,
    weighted_voltage: f64,
    sources: Vec<BiasSource>,
}

struct BiasSource {
    voltage_v: f64,
    conductance_s: f64,
}

#[derive(Clone, Copy)]
struct StrapCheckContext<'a> {
    scenario: &'a Scenario,
    component: &'a str,
    pin: &'a str,
    net: &'a str,
}

struct ThresholdFinding<'a> {
    scenario: &'a Scenario,
    component: &'a str,
    pin: &'a str,
    net: &'a str,
    required_boot_mode: &'a str,
    required_state: &'a str,
    voltage_v: f64,
    limit_key: &'a str,
    limit_v: f64,
}
