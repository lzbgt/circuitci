use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::motor_drive_common::*;
use super::{MOTOR_BRIDGE_LOSS_THERMAL_VALID, MOTOR_BRIDGE_SWITCHING_VALID};

pub(super) fn validate_motor_load_supply(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let motor_load = motor_load_evidence(bound, scenario, findings);
    let Some(bus_voltage_min_v) = required_non_negative(scenario, "bus_voltage_min_V", findings)
    else {
        return;
    };
    let Some(bus_voltage_max_v) = required_positive(scenario, "bus_voltage_max_V", findings) else {
        return;
    };
    let Some(motor_supply_min_v) = required_non_negative_with_fallback(
        scenario,
        "motor_supply_voltage_min_V",
        motor_load
            .as_ref()
            .and_then(|evidence| evidence.load.supply_voltage_min_v),
        "motor_load.supply_voltage_min_V",
        findings,
    ) else {
        return;
    };
    let Some(motor_supply_max_v) = required_positive_with_fallback(
        scenario,
        "motor_supply_voltage_max_V",
        motor_load
            .as_ref()
            .and_then(|evidence| evidence.load.supply_voltage_max_v),
        "motor_load.supply_voltage_max_V",
        findings,
    ) else {
        return;
    };

    if bus_voltage_min_v > bus_voltage_max_v {
        missing_input(
            scenario,
            "bus_voltage_min_V",
            "Set bus_voltage_min_V less than or equal to bus_voltage_max_V.",
            findings,
        );
        return;
    }
    if motor_supply_min_v > motor_supply_max_v {
        missing_input(
            scenario,
            "motor_load.supply_voltage_min_V",
            "Set motor supply minimum voltage less than or equal to motor supply maximum voltage.",
            findings,
        );
        return;
    }

    if bus_voltage_min_v < motor_supply_min_v {
        motor_supply_finding(
            scenario,
            motor_load
                .as_ref()
                .map(|evidence| evidence.component_id.as_str())
                .unwrap_or("motor"),
            MotorSupplyComparison {
                measured_name: "bus_voltage_min_V",
                limit_name: "motor_supply_voltage_min_V",
                measured: bus_voltage_min_v,
                limit: motor_supply_min_v,
                message: "Wheel bus minimum voltage is below the selected motor supply range.",
                fix: "Select a motor specified for the battery range, raise the minimum wheel bus voltage, or add measured low-voltage torque/current evidence.",
            },
            findings,
        );
    }
    if bus_voltage_max_v > motor_supply_max_v {
        motor_supply_finding(
            scenario,
            motor_load
                .as_ref()
                .map(|evidence| evidence.component_id.as_str())
                .unwrap_or("motor"),
            MotorSupplyComparison {
                measured_name: "bus_voltage_max_V",
                limit_name: "motor_supply_voltage_max_V",
                measured: bus_voltage_max_v,
                limit: motor_supply_max_v,
                message: "Wheel bus maximum voltage exceeds the selected motor supply range.",
                fix: "Select a motor with adequate voltage rating, lower the wheel bus voltage, or add a separate motor-drive rail within the motor rating.",
            },
            findings,
        );
    }
}

pub(super) fn validate_motor_bridge_loss_thermal(
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
    let Some(model) = bound.library.get(&component.model) else {
        missing_input(
            scenario,
            "component.model",
            "Bind the motor bridge component to a source-backed component model.",
            findings,
        );
        return;
    };
    let Some(bridge) = model.motor_bridge.as_ref() else {
        missing_input(
            scenario,
            "component.model.motor_bridge",
            "Use a motor bridge component model with source-backed motor_bridge loss and rating metadata.",
            findings,
        );
        return;
    };
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
    let Some(bus_voltage_max_v) = required_positive(scenario, "bus_voltage_max_V", findings) else {
        return;
    };
    let Some(max_total_bridge_loss_w) =
        required_positive(scenario, "max_total_bridge_loss_W", findings)
    else {
        return;
    };
    let Some(min_loss_margin_ratio) =
        required_at_least(scenario, "min_loss_margin_ratio", 1.0, findings)
    else {
        return;
    };

    let Some(voltage_rating_v) = positive_bridge_value(
        scenario,
        "motor_bridge.voltage_rating_V",
        bridge.voltage_rating_v,
        findings,
    ) else {
        return;
    };
    let Some(current_rating_a) = positive_bridge_value(
        scenario,
        "motor_bridge.current_rating_A",
        bridge.current_rating_a,
        findings,
    ) else {
        return;
    };
    let Some(reference_loss_w) = positive_bridge_value(
        scenario,
        "motor_bridge.reference_loss_W",
        bridge.reference_loss_w,
        findings,
    ) else {
        return;
    };
    let Some(reference_current_a) = positive_bridge_value(
        scenario,
        "motor_bridge.reference_current_A",
        bridge.reference_current_a,
        findings,
    ) else {
        return;
    };
    let Some(loss_multiplier) = bridge_loss_multiplier(scenario, bridge, findings) else {
        return;
    };

    if bus_voltage_max_v > voltage_rating_v {
        bridge_loss_finding(
            scenario,
            &target.component,
            BridgeLossComparison {
                measured_name: "bus_voltage_max_V",
                limit_name: "motor_bridge_voltage_rating_V",
                measured: bus_voltage_max_v,
                limit: voltage_rating_v,
                message: "Motor bridge maximum bus voltage exceeds the bridge voltage rating.",
                fix: "Select a higher-voltage bridge or lower the declared maximum wheel bus voltage.",
            },
            findings,
        );
    }
    if peak_current_a > current_rating_a {
        bridge_loss_finding(
            scenario,
            &target.component,
            BridgeLossComparison {
                measured_name: "motor_phase_peak_current_A",
                limit_name: "motor_bridge_current_rating_A",
                measured: peak_current_a,
                limit: current_rating_a,
                message: "Motor phase peak current exceeds the bridge current rating.",
                fix: "Select a higher-current bridge, reduce the motor current limit, or add sourced pulse/SOA evidence for the operating point.",
            },
            findings,
        );
    }

    let estimated_total_loss_w =
        reference_loss_w * (rms_current_a / reference_current_a).powi(2) * loss_multiplier;
    if estimated_total_loss_w * min_loss_margin_ratio > max_total_bridge_loss_w {
        let mut finding = Finding::critical(
            MOTOR_BRIDGE_LOSS_THERMAL_VALID,
            &scenario.name,
            format!(
                "Estimated bridge loss {estimated_total_loss_w:.6} W with {min_loss_margin_ratio:.3}x margin exceeds {max_total_bridge_loss_w:.6} W board thermal budget."
            ),
        );
        finding.component = Some(target.component.clone());
        finding.measured.insert(
            "estimated_total_bridge_loss_W".to_string(),
            json!(estimated_total_loss_w),
        );
        finding.measured.insert(
            "motor_phase_rms_current_A".to_string(),
            json!(rms_current_a),
        );
        finding
            .measured
            .insert("reference_loss_W".to_string(), json!(reference_loss_w));
        finding.measured.insert(
            "reference_current_A".to_string(),
            json!(reference_current_a),
        );
        finding
            .measured
            .insert("loss_multiplier".to_string(), json!(loss_multiplier));
        if let Some(source) = bridge.source.as_deref() {
            finding
                .measured
                .insert("motor_bridge_source".to_string(), json!(source));
        }
        if let Some(evidence) = &motor_load {
            finding
                .measured
                .insert("motor_component".to_string(), json!(evidence.component_id));
        }
        finding.limit.insert(
            "max_total_bridge_loss_W".to_string(),
            json!(max_total_bridge_loss_w),
        );
        finding.limit.insert(
            "min_loss_margin_ratio".to_string(),
            json!(min_loss_margin_ratio),
        );
        finding.suggested_fixes = vec![
            "Lower the wheel current target, improve the thermal design, or select a lower-loss bridge.".to_string(),
            "Replace this first-pass scaled-loss screen with sourced SOA/switching-loss/thermal evidence before final fabrication sign-off.".to_string(),
        ];
        findings.push(finding);
    }
}

pub(super) fn validate_motor_bridge_switching(
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
    let Some(model) = bound.library.get(&component.model) else {
        missing_input(
            scenario,
            "component.model",
            "Bind the motor bridge component to a source-backed component model.",
            findings,
        );
        return;
    };
    let Some(bridge) = model.motor_bridge.as_ref() else {
        missing_input(
            scenario,
            "component.model.motor_bridge",
            "Use a motor bridge component model with source-backed motor_bridge switching metadata.",
            findings,
        );
        return;
    };

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
    let Some(bus_voltage_max_v) = required_positive(scenario, "bus_voltage_max_V", findings) else {
        return;
    };
    let Some(pwm_frequency_hz) = required_positive(scenario, "pwm_frequency_Hz", findings) else {
        return;
    };
    let Some(gate_drive_voltage_v) = required_positive(scenario, "gate_drive_voltage_V", findings)
    else {
        return;
    };
    let Some(switching_events_per_cycle) =
        required_at_least(scenario, "switching_events_per_pwm_cycle", 1.0, findings)
    else {
        return;
    };
    let Some(gate_charge_events_per_cycle) =
        required_at_least(scenario, "gate_charge_events_per_pwm_cycle", 1.0, findings)
    else {
        return;
    };
    let Some(max_total_switching_loss_w) =
        required_positive(scenario, "max_total_switching_loss_W", findings)
    else {
        return;
    };
    let Some(min_switching_loss_margin_ratio) =
        required_at_least(scenario, "min_switching_loss_margin_ratio", 1.0, findings)
    else {
        return;
    };
    let Some(max_average_gate_drive_current_a) =
        required_positive(scenario, "max_average_gate_drive_current_A", findings)
    else {
        return;
    };

    let Some(gate_charge_total_c) = positive_bridge_value(
        scenario,
        "motor_bridge.gate_charge_total_C",
        bridge.gate_charge_total_c,
        findings,
    ) else {
        return;
    };
    let Some(rise_time_s) = positive_bridge_value(
        scenario,
        "motor_bridge.rise_time_s",
        bridge.rise_time_s,
        findings,
    ) else {
        return;
    };
    let Some(fall_time_s) = positive_bridge_value(
        scenario,
        "motor_bridge.fall_time_s",
        bridge.fall_time_s,
        findings,
    ) else {
        return;
    };

    let estimated_total_switching_loss_w = 0.5
        * bus_voltage_max_v
        * peak_current_a
        * (rise_time_s + fall_time_s)
        * pwm_frequency_hz
        * switching_events_per_cycle;
    let required_switching_loss_budget_w =
        estimated_total_switching_loss_w * min_switching_loss_margin_ratio;
    if required_switching_loss_budget_w > max_total_switching_loss_w {
        let mut finding = Finding::critical(
            MOTOR_BRIDGE_SWITCHING_VALID,
            &scenario.name,
            format!(
                "Estimated bridge switching loss {estimated_total_switching_loss_w:.6} W with {min_switching_loss_margin_ratio:.3}x margin exceeds {max_total_switching_loss_w:.6} W budget."
            ),
        );
        finding.component = Some(target.component.clone());
        finding.measured.insert(
            "estimated_total_switching_loss_W".to_string(),
            json!(estimated_total_switching_loss_w),
        );
        finding
            .measured
            .insert("bus_voltage_max_V".to_string(), json!(bus_voltage_max_v));
        finding.measured.insert(
            "motor_phase_peak_current_A".to_string(),
            json!(peak_current_a),
        );
        finding
            .measured
            .insert("rise_time_s".to_string(), json!(rise_time_s));
        finding
            .measured
            .insert("fall_time_s".to_string(), json!(fall_time_s));
        finding
            .measured
            .insert("pwm_frequency_Hz".to_string(), json!(pwm_frequency_hz));
        finding.measured.insert(
            "switching_events_per_pwm_cycle".to_string(),
            json!(switching_events_per_cycle),
        );
        if let Some(evidence) = &motor_load {
            finding
                .measured
                .insert("motor_component".to_string(), json!(evidence.component_id));
        }
        finding.limit.insert(
            "max_total_switching_loss_W".to_string(),
            json!(max_total_switching_loss_w),
        );
        finding.limit.insert(
            "min_switching_loss_margin_ratio".to_string(),
            json!(min_switching_loss_margin_ratio),
        );
        finding.suggested_fixes = vec![
            "Reduce PWM frequency, edge count, bus voltage, or peak current; or choose a bridge with faster sourced switching data.".to_string(),
            "Replace this static transition estimate with measured switching waveforms before final fabrication sign-off.".to_string(),
        ];
        findings.push(finding);
    }

    let average_gate_drive_current_a =
        gate_charge_total_c * pwm_frequency_hz * gate_charge_events_per_cycle;
    if average_gate_drive_current_a > max_average_gate_drive_current_a {
        let mut finding = Finding::critical(
            MOTOR_BRIDGE_SWITCHING_VALID,
            &scenario.name,
            format!(
                "Average gate-drive charge current {average_gate_drive_current_a:.6} A exceeds {max_average_gate_drive_current_a:.6} A budget."
            ),
        );
        finding.component = Some(target.component.clone());
        finding.measured.insert(
            "average_gate_drive_current_A".to_string(),
            json!(average_gate_drive_current_a),
        );
        finding.measured.insert(
            "gate_charge_total_C".to_string(),
            json!(gate_charge_total_c),
        );
        finding
            .measured
            .insert("pwm_frequency_Hz".to_string(), json!(pwm_frequency_hz));
        finding.measured.insert(
            "gate_charge_events_per_pwm_cycle".to_string(),
            json!(gate_charge_events_per_cycle),
        );
        finding.measured.insert(
            "gate_drive_power_W".to_string(),
            json!(average_gate_drive_current_a * gate_drive_voltage_v),
        );
        if let Some(gate_charge_voltage_v) = bridge.gate_charge_voltage_v {
            finding.measured.insert(
                "gate_charge_voltage_V".to_string(),
                json!(gate_charge_voltage_v),
            );
        }
        finding.limit.insert(
            "max_average_gate_drive_current_A".to_string(),
            json!(max_average_gate_drive_current_a),
        );
        finding.suggested_fixes = vec![
            "Reduce PWM frequency or switching edge count, select a lower-gate-charge bridge, or verify the gate driver supply/current budget.".to_string(),
            "Treat average gate-drive charge as a static budget only; validate peak source/sink current and waveform ringing separately.".to_string(),
        ];
        findings.push(finding);
    }
}
