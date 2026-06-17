use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::motor_drive_common::*;
use super::{
    MOTOR_BRIDGE_BUDGET_VALID, MOTOR_BRIDGE_LOSS_THERMAL_VALID, MOTOR_BRIDGE_SWITCHING_VALID,
    MOTOR_CURRENT_SENSE_ACCURACY_VALID, MOTOR_REGEN_CLAMP_VALID, MOTOR_ROUTE_CURRENT_VALID,
};

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

pub(super) fn validate_motor_regen_clamp(
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
    if !bound
        .project
        .board
        .components
        .contains_key(&target.component)
    {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to an existing motor bridge/power-stage component.",
            findings,
        );
        return;
    };
    let Some(clamp_component) = required_string(scenario, "clamp_component", findings) else {
        return;
    };
    let Some(clamp) = bound.project.board.components.get(&clamp_component) else {
        missing_input(
            scenario,
            "clamp_component",
            "Set parameters.clamp_component to an existing regeneration clamp or absorber component.",
            findings,
        );
        return;
    };
    if bound.library.get(&clamp.model).is_none() {
        missing_input(
            scenario,
            "clamp_component.model",
            "Bind the regeneration clamp or absorber component to a component model.",
            findings,
        );
        return;
    }

    let motor_load = motor_load_evidence(bound, scenario, findings);
    let Some(max_regen_current_a) = required_non_negative_with_fallback(
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
    let Some(regen_energy_j) = required_positive(scenario, "regen_energy_J", findings) else {
        return;
    };
    let Some(bus_capacitance_f) = required_positive(scenario, "bus_capacitance_F", findings) else {
        return;
    };
    let Some(bus_voltage_nominal_v) =
        required_positive(scenario, "bus_voltage_nominal_V", findings)
    else {
        return;
    };
    let Some(clamp_voltage_v) = required_positive(scenario, "clamp_voltage_V", findings) else {
        return;
    };
    let Some(max_bus_voltage_v) = required_positive(scenario, "max_bus_voltage_V", findings) else {
        return;
    };
    let Some(clamp_current_rating_a) =
        required_positive(scenario, "clamp_current_rating_A", findings)
    else {
        return;
    };
    let Some(clamp_energy_rating_j) =
        required_positive(scenario, "clamp_energy_rating_J", findings)
    else {
        return;
    };
    let Some(min_current_margin_ratio) =
        required_at_least(scenario, "min_clamp_current_margin_ratio", 1.0, findings)
    else {
        return;
    };
    let Some(min_energy_margin_ratio) =
        required_at_least(scenario, "min_regen_energy_margin_ratio", 1.0, findings)
    else {
        return;
    };

    if clamp_voltage_v <= bus_voltage_nominal_v {
        missing_input(
            scenario,
            "clamp_voltage_V",
            "Set clamp_voltage_V greater than bus_voltage_nominal_V so bus capacitance has a positive absorption window.",
            findings,
        );
        return;
    }

    if clamp_voltage_v > max_bus_voltage_v {
        regen_clamp_finding(
            scenario,
            &clamp_component,
            RegenClampComparison {
                measured_name: "clamp_voltage_V",
                limit_name: "max_bus_voltage_V",
                measured: clamp_voltage_v,
                limit: max_bus_voltage_v,
                message: "Motor regeneration clamp voltage exceeds the declared maximum wheel-bus voltage.",
                fix: "Select a lower clamp threshold, raise the source-backed bus/bridge voltage limit, or add a staged brake path that limits bus voltage.",
            },
            findings,
        );
    }

    let required_clamp_current_a = max_regen_current_a * min_current_margin_ratio;
    if required_clamp_current_a > clamp_current_rating_a {
        regen_clamp_finding(
            scenario,
            &clamp_component,
            RegenClampComparison {
                measured_name: "required_clamp_current_A",
                limit_name: "clamp_current_rating_A",
                measured: required_clamp_current_a,
                limit: clamp_current_rating_a,
                message: "Motor regeneration clamp current rating is below the declared regeneration current with margin.",
                fix: "Select a stronger brake/clamp path, reduce allowed regeneration current, or add sourced current-sharing evidence.",
            },
            findings,
        );
    }

    let bus_absorption_energy_j =
        0.5 * bus_capacitance_f * (clamp_voltage_v.powi(2) - bus_voltage_nominal_v.powi(2));
    let total_absorption_energy_j = bus_absorption_energy_j + clamp_energy_rating_j;
    let required_absorption_energy_j = regen_energy_j * min_energy_margin_ratio;
    if required_absorption_energy_j > total_absorption_energy_j {
        let mut finding = Finding::critical(
            MOTOR_REGEN_CLAMP_VALID,
            &scenario.name,
            format!(
                "Motor regeneration envelope requires {required_absorption_energy_j:.6} J with margin, but bus capacitance plus clamp rating absorbs {total_absorption_energy_j:.6} J."
            ),
        );
        finding.component = Some(clamp_component);
        finding
            .measured
            .insert("regen_energy_J".to_string(), json!(regen_energy_j));
        finding.measured.insert(
            "bus_absorption_energy_J".to_string(),
            json!(bus_absorption_energy_j),
        );
        finding.measured.insert(
            "total_absorption_energy_J".to_string(),
            json!(total_absorption_energy_j),
        );
        if let Some(evidence) = &motor_load {
            finding
                .measured
                .insert("motor_component".to_string(), json!(evidence.component_id));
        }
        finding.limit.insert(
            "required_absorption_energy_J".to_string(),
            json!(required_absorption_energy_j),
        );
        finding.limit.insert(
            "min_regen_energy_margin_ratio".to_string(),
            json!(min_energy_margin_ratio),
        );
        finding.suggested_fixes = vec![
            "Select a brake resistor, active clamp, or upstream energy sink with sourced pulse-energy evidence.".to_string(),
            "Increase source-backed bus capacitance or lower the declared regeneration energy envelope.".to_string(),
            "Do not treat this static screen as repeated-pulse thermal or firmware regeneration-control sign-off.".to_string(),
        ];
        findings.push(finding);
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

pub(super) fn validate_motor_current_sense_placement(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(reference_component) = required_string(scenario, "reference_component", findings)
    else {
        return;
    };
    let Some(reference_placement) =
        placement_evidence(bound, scenario, &reference_component, findings)
    else {
        return;
    };
    let shunt_components = required_string_list(scenario, "shunt_components", findings);
    let phase_route_nets = required_string_list(scenario, "phase_route_nets", findings);
    let sense_route_nets = required_string_list(scenario, "sense_route_nets", findings);
    if shunt_components.is_empty() || phase_route_nets.is_empty() || sense_route_nets.is_empty() {
        return;
    }
    if shunt_components.len() != phase_route_nets.len()
        || shunt_components.len() != sense_route_nets.len()
    {
        missing_input(
            scenario,
            "shunt_components/phase_route_nets/sense_route_nets",
            "Use equal-length shunt_components, phase_route_nets, and sense_route_nets lists.",
            findings,
        );
        return;
    }
    let Some(max_shunt_to_reference_distance_mm) =
        required_positive(scenario, "max_shunt_to_reference_distance_mm", findings)
    else {
        return;
    };
    let Some(max_shunt_to_phase_route_distance_mm) =
        required_positive(scenario, "max_shunt_to_phase_route_distance_mm", findings)
    else {
        return;
    };
    let Some(max_shunt_to_sense_route_distance_mm) =
        required_positive(scenario, "max_shunt_to_sense_route_distance_mm", findings)
    else {
        return;
    };
    let Some(max_sense_route_length_mm) =
        required_positive(scenario, "max_sense_route_length_mm", findings)
    else {
        return;
    };

    for ((shunt_component, phase_net), sense_net) in shunt_components
        .iter()
        .zip(phase_route_nets.iter())
        .zip(sense_route_nets.iter())
    {
        let Some(shunt_placement) = placement_evidence(bound, scenario, shunt_component, findings)
        else {
            continue;
        };
        let Some(phase_route) = route_evidence(bound, scenario, phase_net, findings) else {
            continue;
        };
        let Some(sense_route) = route_evidence(bound, scenario, sense_net, findings) else {
            continue;
        };
        let shunt_point = PlacementPoint::from(shunt_placement);
        let reference_distance_mm =
            point_distance_mm(shunt_point, PlacementPoint::from(reference_placement));
        if reference_distance_mm > max_shunt_to_reference_distance_mm {
            current_sense_distance_finding(
                scenario,
                shunt_component,
                "shunt_to_reference_distance_mm",
                reference_distance_mm,
                "max_shunt_to_reference_distance_mm",
                max_shunt_to_reference_distance_mm,
                findings,
            );
        }
        let Some(phase_distance_mm) = distance_to_route_mm(phase_route, shunt_point) else {
            missing_input(
                scenario,
                "phase_route_nets",
                &format!(
                    "Add non-empty positive-width route evidence for phase route {phase_net}."
                ),
                findings,
            );
            continue;
        };
        if phase_distance_mm > max_shunt_to_phase_route_distance_mm {
            current_sense_distance_finding(
                scenario,
                shunt_component,
                "shunt_to_phase_route_distance_mm",
                phase_distance_mm,
                "max_shunt_to_phase_route_distance_mm",
                max_shunt_to_phase_route_distance_mm,
                findings,
            );
        }
        let Some(sense_distance_mm) = distance_to_route_mm(sense_route, shunt_point) else {
            missing_input(
                scenario,
                "sense_route_nets",
                &format!(
                    "Add non-empty positive-width route evidence for sense route {sense_net}."
                ),
                findings,
            );
            continue;
        };
        if sense_distance_mm > max_shunt_to_sense_route_distance_mm {
            current_sense_distance_finding(
                scenario,
                shunt_component,
                "shunt_to_sense_route_distance_mm",
                sense_distance_mm,
                "max_shunt_to_sense_route_distance_mm",
                max_shunt_to_sense_route_distance_mm,
                findings,
            );
        }
        let sense_route_length_mm = route_length_mm(sense_route);
        if sense_route_length_mm > max_sense_route_length_mm {
            current_sense_distance_finding(
                scenario,
                shunt_component,
                "sense_route_length_mm",
                sense_route_length_mm,
                "max_sense_route_length_mm",
                max_sense_route_length_mm,
                findings,
            );
        }
    }
}

pub(super) fn validate_motor_current_sense_accuracy(
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
    if !bound
        .project
        .board
        .components
        .contains_key(&target.component)
    {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to an existing motor bridge/power-stage component.",
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
    let Some(shunt_resistance_ohm) =
        required_positive(scenario, "phase_shunt_resistance_ohm", findings)
    else {
        return;
    };
    let Some(shunt_tolerance_ratio) =
        required_non_negative(scenario, "shunt_tolerance_ratio", findings)
    else {
        return;
    };
    let Some(sense_gain) = required_positive(scenario, "sense_gain_V_per_V", findings) else {
        return;
    };
    let Some(gain_error_ratio) = required_non_negative(scenario, "gain_error_ratio", findings)
    else {
        return;
    };
    let Some(input_offset_voltage_v) =
        required_non_negative(scenario, "input_offset_voltage_V", findings)
    else {
        return;
    };
    let Some(adc_reference_voltage_v) =
        required_positive(scenario, "adc_reference_voltage_V", findings)
    else {
        return;
    };
    let Some(adc_input_max_voltage_v) =
        required_positive(scenario, "adc_input_max_voltage_V", findings)
    else {
        return;
    };
    let Some(adc_resolution_bits) = required_resolution_bits(scenario, findings) else {
        return;
    };
    let Some(min_current_measurement_a) =
        required_positive(scenario, "min_current_measurement_A", findings)
    else {
        return;
    };
    let Some(min_adc_counts_at_min_current) =
        required_positive(scenario, "min_adc_counts_at_min_current", findings)
    else {
        return;
    };
    let Some(max_total_current_error_a) =
        required_positive(scenario, "max_total_current_error_A", findings)
    else {
        return;
    };

    let peak_output_voltage_v = peak_current_a * shunt_resistance_ohm * sense_gain;
    let effective_adc_input_max_v = adc_input_max_voltage_v.min(adc_reference_voltage_v);
    if peak_output_voltage_v > effective_adc_input_max_v {
        current_sense_accuracy_finding(
            scenario,
            &target.component,
            CurrentSenseAccuracyComparison {
                measured_name: "peak_sense_output_voltage_V",
                limit_name: "effective_adc_input_max_voltage_V",
                measured: peak_output_voltage_v,
                limit: effective_adc_input_max_v,
                message: "Peak motor current drives the current-sense output beyond the usable ADC input range.",
                fix: "Reduce shunt resistance or current-sense gain, increase the ADC input range, or lower the motor peak-current limit.",
            },
            findings,
        );
    }

    let adc_full_scale_codes = 2_f64.powi(adc_resolution_bits as i32) - 1.0;
    let adc_lsb_voltage_v = adc_reference_voltage_v / adc_full_scale_codes;
    let min_current_output_voltage_v =
        min_current_measurement_a * shunt_resistance_ohm * sense_gain;
    let adc_counts_at_min_current = min_current_output_voltage_v / adc_lsb_voltage_v;
    if adc_counts_at_min_current < min_adc_counts_at_min_current {
        current_sense_accuracy_finding(
            scenario,
            &target.component,
            CurrentSenseAccuracyComparison {
                measured_name: "adc_counts_at_min_current",
                limit_name: "min_adc_counts_at_min_current",
                measured: adc_counts_at_min_current,
                limit: min_adc_counts_at_min_current,
                message: "Declared minimum measurable phase current produces too few ADC counts.",
                fix: "Increase current-sense gain, use a larger shunt if thermal budget allows, improve ADC resolution/reference range, or raise the minimum trustworthy current threshold.",
            },
            findings,
        );
    }

    let quantization_error_a = 0.5 * adc_lsb_voltage_v / (shunt_resistance_ohm * sense_gain);
    let offset_error_a = input_offset_voltage_v / shunt_resistance_ohm;
    let shunt_tolerance_error_a = rms_current_a * shunt_tolerance_ratio;
    let gain_error_a = rms_current_a * gain_error_ratio;
    let total_current_error_a =
        quantization_error_a + offset_error_a + shunt_tolerance_error_a + gain_error_a;
    if total_current_error_a > max_total_current_error_a {
        let mut finding = Finding::critical(
            MOTOR_CURRENT_SENSE_ACCURACY_VALID,
            &scenario.name,
            format!(
                "Worst-case current-sense error {total_current_error_a:.6} A exceeds {max_total_current_error_a:.6} A."
            ),
        );
        finding.component = Some(target.component.clone());
        finding.measured.insert(
            "total_current_error_A".to_string(),
            json!(total_current_error_a),
        );
        finding.measured.insert(
            "quantization_error_A".to_string(),
            json!(quantization_error_a),
        );
        finding
            .measured
            .insert("offset_error_A".to_string(), json!(offset_error_a));
        finding.measured.insert(
            "shunt_tolerance_error_A".to_string(),
            json!(shunt_tolerance_error_a),
        );
        finding
            .measured
            .insert("gain_error_A".to_string(), json!(gain_error_a));
        finding
            .measured
            .insert("adc_lsb_voltage_V".to_string(), json!(adc_lsb_voltage_v));
        if let Some(evidence) = &motor_load {
            finding
                .measured
                .insert("motor_component".to_string(), json!(evidence.component_id));
        }
        finding.limit.insert(
            "max_total_current_error_A".to_string(),
            json!(max_total_current_error_a),
        );
        finding.suggested_fixes = vec![
            "Use a tighter shunt, lower-offset amplifier, calibrated gain path, or higher-resolution ADC.".to_string(),
            "Rebalance shunt value and gain while keeping peak ADC range and shunt thermal limits valid.".to_string(),
            "Treat this as a static worst-case screen; validate PWM common-mode rejection and sampled waveform behavior separately.".to_string(),
        ];
        findings.push(finding);
    }
}
