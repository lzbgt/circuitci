use crate::board_ir::Scenario;
use crate::library::{BoundBoard, MotorLoad};
use crate::reports::Finding;
use serde_json::json;

use super::MOTOR_BRIDGE_BUDGET_VALID;

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
