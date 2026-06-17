use crate::board_ir::{NetRoute, Scenario};
use crate::library::{BoundBoard, MotorLoad};
use crate::reports::Finding;
use serde_json::json;

use super::{MOTOR_BRIDGE_BUDGET_VALID, MOTOR_ROUTE_CURRENT_VALID};

const VALIDATION_INPUT_MISSING: &str = "VALIDATION_INPUT_MISSING";

pub(super) fn validate_motor_bridge_budget(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to the motor bridge/power-stage component.",
            findings,
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to an existing motor bridge/power-stage component.",
            findings,
        );
        return;
    };
    if bound.library.get(&component.model).is_none() {
        missing_input(
            scenario,
            "component.model",
            "Bind the motor bridge component to a source-backed component model.",
            findings,
        );
        return;
    }

    let motor_load = motor_load_evidence(bound, scenario, findings);

    let Some(peak_current_a) = required_positive_with_fallback(
        scenario,
        "motor_phase_peak_current_A",
        motor_load
            .as_ref()
            .and_then(|evidence| evidence.load.phase_peak_current_a),
        "motor_load.phase_peak_current_A",
        findings,
    ) else {
        return;
    };
    let Some(rms_current_a) = required_positive_with_fallback(
        scenario,
        "motor_phase_rms_current_A",
        motor_load
            .as_ref()
            .and_then(|evidence| evidence.load.phase_rms_current_a),
        "motor_load.phase_rms_current_A",
        findings,
    ) else {
        return;
    };
    let Some(regen_current_a) = required_non_negative_with_fallback(
        scenario,
        "max_regen_current_A",
        motor_load
            .as_ref()
            .and_then(|evidence| evidence.load.max_regen_current_a),
        "motor_load.max_regen_current_A",
        findings,
    ) else {
        return;
    };
    let Some(bridge_reference_current_a) =
        required_positive(scenario, "bridge_reference_current_A", findings)
    else {
        return;
    };
    let Some(bridge_device_current_class_a) =
        required_positive(scenario, "bridge_device_current_class_A", findings)
    else {
        return;
    };
    let Some(shunt_resistance_ohm) =
        required_positive(scenario, "phase_shunt_resistance_ohm", findings)
    else {
        return;
    };
    let Some(shunt_power_rating_w) =
        required_positive(scenario, "phase_shunt_power_rating_W", findings)
    else {
        return;
    };
    let Some(shunt_power_margin) =
        required_at_least(scenario, "min_shunt_power_margin_ratio", 1.0, findings)
    else {
        return;
    };
    let Some(connector_current_a) =
        required_positive(scenario, "motor_connector_current_rating_A", findings)
    else {
        return;
    };
    let Some(gate_resistor_ohm) = required_positive(scenario, "gate_resistor_ohm", findings) else {
        return;
    };
    let Some(dead_time_ns) = required_positive(scenario, "dead_time_ns", findings) else {
        return;
    };
    let Some(pwm_frequency_hz) = required_positive(scenario, "pwm_frequency_Hz", findings) else {
        return;
    };

    let shunt_power_w = rms_current_a * rms_current_a * shunt_resistance_ohm;
    if rms_current_a > peak_current_a {
        budget_finding(
            scenario,
            &target.component,
            BudgetComparison {
                measured_name: "motor_phase_rms_current_A",
                limit_name: "motor_phase_peak_current_A",
                measured: rms_current_a,
                limit: peak_current_a,
                message: "Motor phase RMS current cannot exceed declared phase peak current.",
                fix: "Correct the motor current profile or use a peak current that covers the expected control envelope.",
            },
            findings,
        );
    }
    if rms_current_a > bridge_reference_current_a {
        budget_finding(
            scenario,
            &target.component,
            BudgetComparison {
                measured_name: "motor_phase_rms_current_A",
                limit_name: "bridge_reference_current_A",
                measured: rms_current_a,
                limit: bridge_reference_current_a,
                message: "Motor phase RMS current exceeds the bridge reference current.",
                fix: "Select a stronger bridge, lower the current target, or add sourced thermal/current evidence for the intended operating point.",
            },
            findings,
        );
    }
    if peak_current_a > bridge_device_current_class_a {
        budget_finding(
            scenario,
            &target.component,
            BudgetComparison {
                measured_name: "motor_phase_peak_current_A",
                limit_name: "bridge_device_current_class_A",
                measured: peak_current_a,
                limit: bridge_device_current_class_a,
                message: "Motor phase peak current exceeds the bridge device current class.",
                fix: "Select a higher-current bridge or reduce the peak current limit.",
            },
            findings,
        );
    }
    if rms_current_a > connector_current_a {
        budget_finding(
            scenario,
            &target.component,
            BudgetComparison {
                measured_name: "motor_phase_rms_current_A",
                limit_name: "motor_connector_current_rating_A",
                measured: rms_current_a,
                limit: connector_current_a,
                message: "Motor phase RMS current exceeds the declared motor connector current rating.",
                fix: "Select a higher-current motor connector or reduce the current target.",
            },
            findings,
        );
    }
    if regen_current_a > connector_current_a {
        budget_finding(
            scenario,
            &target.component,
            BudgetComparison {
                measured_name: "max_regen_current_A",
                limit_name: "motor_connector_current_rating_A",
                measured: regen_current_a,
                limit: connector_current_a,
                message: "Maximum regeneration current exceeds the declared motor connector current rating.",
                fix: "Rate the connector and upstream path for regeneration current or clamp/limit regen in firmware and hardware.",
            },
            findings,
        );
    }
    if shunt_power_w * shunt_power_margin > shunt_power_rating_w {
        let mut finding = Finding::critical(
            MOTOR_BRIDGE_BUDGET_VALID,
            &scenario.name,
            format!(
                "Phase shunt dissipates {:.6} W at {:.6} A RMS; with {:.3}x margin it exceeds {:.6} W rating.",
                shunt_power_w, rms_current_a, shunt_power_margin, shunt_power_rating_w
            ),
        );
        finding.component = Some(target.component.clone());
        if let Some(evidence) = &motor_load {
            finding
                .measured
                .insert("motor_component".to_string(), json!(evidence.component_id));
        }
        finding
            .measured
            .insert("phase_shunt_power_W".to_string(), json!(shunt_power_w));
        finding.measured.insert(
            "motor_phase_rms_current_A".to_string(),
            json!(rms_current_a),
        );
        finding.limit.insert(
            "phase_shunt_power_rating_W".to_string(),
            json!(shunt_power_rating_w),
        );
        finding.limit.insert(
            "min_shunt_power_margin_ratio".to_string(),
            json!(shunt_power_margin),
        );
        finding.suggested_fixes = vec![
            "Use a lower shunt resistance if current-sense resolution still meets ADC/noise requirements.".to_string(),
            "Select a higher-power four-terminal shunt and validate PCB copper temperature rise.".to_string(),
            "Reduce the wheel phase-current limit in the motor controller.".to_string(),
        ];
        findings.push(finding);
    }

    if let Some(max_sense_v) = optional_positive(scenario, "max_shunt_sense_voltage_V", findings) {
        let peak_sense_v = peak_current_a * shunt_resistance_ohm;
        if peak_sense_v > max_sense_v {
            budget_finding(
                scenario,
                &target.component,
                BudgetComparison {
                    measured_name: "phase_shunt_sense_voltage_V",
                    limit_name: "max_shunt_sense_voltage_V",
                    measured: peak_sense_v,
                    limit: max_sense_v,
                    message: "Peak shunt sense voltage exceeds the declared current-sense input range.",
                    fix: "Lower the shunt value, adjust current-sense gain, or use a larger ADC/current-sense input range.",
                },
                findings,
            );
        }
    }

    if !gate_resistor_ohm.is_finite() || !dead_time_ns.is_finite() || !pwm_frequency_hz.is_finite()
    {
        missing_input(
            scenario,
            "gate_resistor_ohm/dead_time_ns/pwm_frequency_Hz",
            "Use finite gate resistor, dead-time, and PWM frequency values.",
            findings,
        );
    }
}

pub(super) fn validate_motor_route_current(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let route_nets = required_route_nets(scenario, findings);
    if route_nets.is_empty() {
        return;
    }
    let Some(max_current_density_a_per_mm) =
        required_positive(scenario, "max_current_density_A_per_mm", findings)
    else {
        return;
    };
    let Some(route_current) = route_current_evidence(bound, scenario, findings) else {
        return;
    };
    let required_route_width_mm = route_current.current_a / max_current_density_a_per_mm;

    for net in route_nets {
        let Some(route) = route_evidence(bound, scenario, &net, findings) else {
            continue;
        };
        let Some(min_width_mm) = min_route_width_mm(route) else {
            missing_input(
                scenario,
                "board.layout.routes",
                "Use imported or explicit routed copper evidence with positive finite segment widths.",
                findings,
            );
            continue;
        };
        if min_width_mm + f64::EPSILON < required_route_width_mm {
            let mut finding = Finding::critical(
                MOTOR_ROUTE_CURRENT_VALID,
                &scenario.name,
                format!(
                    "Motor route {net} has minimum width {min_width_mm:.6} mm, but {:.6} A with {:.6} A/mm policy requires at least {required_route_width_mm:.6} mm.",
                    route_current.current_a, max_current_density_a_per_mm
                ),
            );
            finding.net = Some(net);
            finding.measured.insert(
                "route_current_A".to_string(),
                json!(route_current.current_a),
            );
            finding.measured.insert(
                "route_current_source".to_string(),
                json!(route_current.source),
            );
            finding
                .measured
                .insert("min_route_width_mm".to_string(), json!(min_width_mm));
            if let Some(component_id) = &route_current.motor_component {
                finding
                    .measured
                    .insert("motor_component".to_string(), json!(component_id));
            }
            finding.limit.insert(
                "max_current_density_A_per_mm".to_string(),
                json!(max_current_density_a_per_mm),
            );
            finding.limit.insert(
                "required_route_width_mm".to_string(),
                json!(required_route_width_mm),
            );
            finding.suggested_fixes = vec![
                "Increase the routed copper width, add parallel copper with explicit route evidence, or lower the declared motor current limit.".to_string(),
                "Keep max_current_density_A_per_mm tied to board stackup, copper weight, temperature-rise, and layout policy evidence.".to_string(),
            ];
            findings.push(finding);
        }
    }
}

fn required_route_nets(scenario: &Scenario, findings: &mut Vec<Finding>) -> Vec<String> {
    let Some(raw) = scenario.parameters.get("route_nets") else {
        missing_input(
            scenario,
            "route_nets",
            "Add motor_drive parameters.route_nets as a non-empty list of routed motor/power nets.",
            findings,
        );
        return Vec::new();
    };
    let Some(values) = raw.as_sequence() else {
        missing_input(
            scenario,
            "route_nets",
            "Set motor_drive parameters.route_nets to a list of net names.",
            findings,
        );
        return Vec::new();
    };
    let mut nets = Vec::new();
    for value in values {
        let Some(net) = value.as_str() else {
            missing_input(
                scenario,
                "route_nets",
                "Each motor_drive parameters.route_nets entry must be a string.",
                findings,
            );
            return Vec::new();
        };
        let trimmed = net.trim();
        if trimmed.is_empty() {
            missing_input(
                scenario,
                "route_nets",
                "Motor-drive route net names must not be blank.",
                findings,
            );
            return Vec::new();
        }
        if !nets.iter().any(|existing| existing == trimmed) {
            nets.push(trimmed.to_string());
        }
    }
    if nets.is_empty() {
        missing_input(
            scenario,
            "route_nets",
            "Add at least one motor_drive parameters.route_nets entry.",
            findings,
        );
    }
    nets
}

struct RouteCurrentEvidence {
    current_a: f64,
    source: &'static str,
    motor_component: Option<String>,
}

fn route_current_evidence(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<RouteCurrentEvidence> {
    if scenario.parameters.contains_key("route_current_A") {
        return required_positive(scenario, "route_current_A", findings).map(|current_a| {
            RouteCurrentEvidence {
                current_a,
                source: "route_current_A",
                motor_component: None,
            }
        });
    }
    let source = required_current_source(scenario, findings)?;
    let motor_load = motor_load_evidence(bound, scenario, findings)?;
    let current = match source {
        "phase_rms" => motor_load.load.phase_rms_current_a,
        "phase_peak" => motor_load.load.phase_peak_current_a,
        "max_regen" => motor_load.load.max_regen_current_a,
        _ => None,
    };
    let Some(current_a) = current else {
        missing_input(
            scenario,
            "current_source",
            &format!("Motor load component does not declare current evidence for {source}."),
            findings,
        );
        return None;
    };
    if !current_a.is_finite() || current_a <= 0.0 {
        missing_input(
            scenario,
            "current_source",
            &format!("Motor load {source} current must be finite and greater than zero."),
            findings,
        );
        return None;
    }
    Some(RouteCurrentEvidence {
        current_a,
        source,
        motor_component: Some(motor_load.component_id),
    })
}

fn required_current_source(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<&'static str> {
    let Some(raw) = scenario.parameters.get("current_source") else {
        missing_input(
            scenario,
            "current_source",
            "Add motor_drive parameters.current_source as phase_rms, phase_peak, or max_regen when route_current_A is omitted.",
            findings,
        );
        return None;
    };
    let Some(value) = raw.as_str() else {
        missing_input(
            scenario,
            "current_source",
            "Set motor_drive parameters.current_source to phase_rms, phase_peak, or max_regen.",
            findings,
        );
        return None;
    };
    match value.trim() {
        "phase_rms" => Some("phase_rms"),
        "phase_peak" => Some("phase_peak"),
        "max_regen" => Some("max_regen"),
        _ => {
            missing_input(
                scenario,
                "current_source",
                "Use current_source: phase_rms, phase_peak, or max_regen.",
                findings,
            );
            None
        }
    }
}

fn route_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    net: &str,
    findings: &mut Vec<Finding>,
) -> Option<&'a NetRoute> {
    if !bound.project.board.nets.contains_key(net) {
        missing_input(
            scenario,
            "route_nets",
            &format!("Declare motor-drive route net {net} in board.nets."),
            findings,
        );
        return None;
    }
    let Some(route) = bound.project.board.layout.routes.get(net) else {
        missing_input(
            scenario,
            "board.layout.routes",
            &format!(
                "Add imported or explicit board.layout.routes evidence for motor-drive net {net}."
            ),
            findings,
        );
        return None;
    };
    Some(route)
}

fn min_route_width_mm(route: &NetRoute) -> Option<f64> {
    route
        .segments
        .iter()
        .filter_map(|segment| {
            if segment.width_mm.is_finite() && segment.width_mm > 0.0 {
                Some(segment.width_mm)
            } else {
                None
            }
        })
        .min_by(|left, right| left.total_cmp(right))
}

fn required_positive(scenario: &Scenario, name: &str, findings: &mut Vec<Finding>) -> Option<f64> {
    let value = required_number(scenario, name, findings)?;
    if value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

fn required_positive_with_fallback(
    scenario: &Scenario,
    name: &str,
    fallback: Option<f64>,
    fallback_name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if scenario.parameters.contains_key(name) {
        return required_positive(scenario, name, findings);
    }
    let Some(value) = fallback else {
        missing_input(
            scenario,
            name,
            &format!(
                "Add motor_drive parameters.{name}, or set parameters.motor_component to a component model with {fallback_name}."
            ),
            findings,
        );
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            fallback_name,
            &format!("Set {fallback_name} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

fn required_non_negative(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let value = required_number(scenario, name, findings)?;
    if value >= 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a finite non-negative value."),
            findings,
        );
        None
    }
}

fn required_non_negative_with_fallback(
    scenario: &Scenario,
    name: &str,
    fallback: Option<f64>,
    fallback_name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if scenario.parameters.contains_key(name) {
        return required_non_negative(scenario, name, findings);
    }
    let Some(value) = fallback else {
        missing_input(
            scenario,
            name,
            &format!(
                "Add motor_drive parameters.{name}, or set parameters.motor_component to a component model with {fallback_name}."
            ),
            findings,
        );
        return None;
    };
    if value.is_finite() && value >= 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            fallback_name,
            &format!("Set {fallback_name} to a finite non-negative value."),
            findings,
        );
        None
    }
}

fn required_at_least(
    scenario: &Scenario,
    name: &str,
    min: f64,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let value = required_number(scenario, name, findings)?;
    if value >= min {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to at least {min}."),
            findings,
        );
        None
    }
}

fn optional_positive(scenario: &Scenario, name: &str, findings: &mut Vec<Finding>) -> Option<f64> {
    let raw = scenario.parameters.get(name)?;
    let Some(value) = raw.as_f64() else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a number."),
            findings,
        );
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

fn required_number(scenario: &Scenario, name: &str, findings: &mut Vec<Finding>) -> Option<f64> {
    let Some(raw) = scenario.parameters.get(name) else {
        missing_input(
            scenario,
            name,
            &format!("Add motor_drive parameters.{name}."),
            findings,
        );
        return None;
    };
    let Some(value) = raw.as_f64() else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a number."),
            findings,
        );
        return None;
    };
    if value.is_finite() {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a finite number."),
            findings,
        );
        None
    }
}

struct MotorLoadEvidence<'a> {
    component_id: String,
    load: &'a MotorLoad,
}

fn motor_load_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<MotorLoadEvidence<'a>> {
    let raw = scenario.parameters.get("motor_component")?;
    let Some(component_id) = raw.as_str() else {
        missing_input(
            scenario,
            "motor_component",
            "Set motor_drive parameters.motor_component to a component id string.",
            findings,
        );
        return None;
    };
    let Some(component) = bound.project.board.components.get(component_id) else {
        missing_input(
            scenario,
            "motor_component",
            "Set motor_drive parameters.motor_component to an existing motor/load component.",
            findings,
        );
        return None;
    };
    let Some(model) = bound.library.get(&component.model) else {
        missing_input(
            scenario,
            "motor_component.model",
            "Bind the motor/load component to a component model.",
            findings,
        );
        return None;
    };
    let Some(load) = model.motor_load.as_ref() else {
        missing_input(
            scenario,
            "motor_component.motor_load",
            "Use a motor/load component model that declares motor_load current evidence.",
            findings,
        );
        return None;
    };
    Some(MotorLoadEvidence {
        component_id: component_id.to_string(),
        load,
    })
}

fn missing_input(scenario: &Scenario, input: &str, fix: &str, findings: &mut Vec<Finding>) {
    let mut finding = Finding::critical(
        VALIDATION_INPUT_MISSING,
        &scenario.name,
        format!("Motor-drive bridge-budget validation requires {input}."),
    );
    finding
        .limit
        .insert("required_input".to_string(), json!(input));
    finding.suggested_fixes = vec![fix.to_string()];
    findings.push(finding);
}

struct BudgetComparison<'a> {
    measured_name: &'a str,
    limit_name: &'a str,
    measured: f64,
    limit: f64,
    message: &'a str,
    fix: &'a str,
}

fn budget_finding(
    scenario: &Scenario,
    component: &str,
    comparison: BudgetComparison<'_>,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        MOTOR_BRIDGE_BUDGET_VALID,
        &scenario.name,
        comparison.message,
    );
    finding.component = Some(component.to_string());
    finding.measured.insert(
        comparison.measured_name.to_string(),
        json!(comparison.measured),
    );
    finding
        .limit
        .insert(comparison.limit_name.to_string(), json!(comparison.limit));
    finding.suggested_fixes = vec![comparison.fix.to_string()];
    findings.push(finding);
}
