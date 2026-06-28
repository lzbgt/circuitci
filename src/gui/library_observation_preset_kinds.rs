use super::analog::AnalogAssertionDraft;
use super::library_observation_presets::{
    ObservationProbeSpec, component_parameter_f64, default_sample_assertion,
    default_voltage_assertion, probe_for_component_pin, voltage_for_component_pin,
};

pub(super) fn supports_comms_output_observation(model: &crate::library::ComponentModel) -> bool {
    matches!(
        model.category.as_str(),
        "comms" | "rs485_transceiver" | "can_transceiver" | "gate_driver"
    )
}

pub(super) fn add_comms_output_observation_assertions(
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    for (pin, port) in &model.ports {
        if port.kind != crate::library::PortKind::DigitalElectricalOutput {
            continue;
        }
        let Some(output_probe) = probe_for_component_pin(probes, component, pin) else {
            continue;
        };
        let Some(high_threshold) = port.electrical.drive_high_voltage_v else {
            continue;
        };
        let state = component_parameter_f64(
            component,
            &format!("observation_{}_state", pin.to_ascii_lowercase()),
        )
        .unwrap_or(1.0);
        let (suffix, relation, threshold) = if state >= 0.5 {
            ("output_high", "above", high_threshold)
        } else {
            ("output_low", "below", 0.5)
        };
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_{}", output_probe.probe_name, suffix),
            &output_probe.probe_name,
            "mean",
            relation,
            threshold,
            (0.0, stop_time_us),
        ));
    }
}

pub(super) fn add_gate_driver_observation_assertions(
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    if model.category != "gate_driver" {
        return;
    }
    for pin in ["SOA", "SOB", "SOC"] {
        let Some(probe) = probe_for_component_pin(probes, component, pin) else {
            continue;
        };
        let target = component_parameter_f64(
            component,
            &format!("observation_{}_v", pin.to_ascii_lowercase()),
        )
        .unwrap_or(1.65);
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_current_sense_min_voltage", probe.probe_name),
            &probe.probe_name,
            "mean",
            "above",
            target - 0.05,
            (0.0, stop_time_us),
        ));
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_current_sense_max_voltage", probe.probe_name),
            &probe.probe_name,
            "mean",
            "below",
            target + 0.05,
            (0.0, stop_time_us),
        ));
    }
}

pub(super) fn add_pwm_driver_observation_assertions(
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    if model.category != "pwm_driver" {
        return;
    }
    add_port_voltage_window_assertions(
        component,
        model,
        probes,
        scenario_name,
        "VDD",
        stop_time_us,
        assertions,
    );
    if let Some(oe_probe) = probe_for_component_pin(probes, component, "OE") {
        let threshold = model
            .ports
            .get("OE")
            .and_then(|port| port.electrical.vil_max_v)
            .unwrap_or(0.8);
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_enabled_low", oe_probe.probe_name),
            &oe_probe.probe_name,
            "mean",
            "below",
            threshold,
            (0.0, stop_time_us),
        ));
    }
    for pin in ["SCL", "SDA"] {
        let Some(probe) = probe_for_component_pin(probes, component, pin) else {
            continue;
        };
        let Some(port) = model.ports.get(pin) else {
            continue;
        };
        let state = component_parameter_f64(
            component,
            &format!("observation_{}_state", pin.to_ascii_lowercase()),
        )
        .unwrap_or(1.0);
        let (suffix, relation, threshold) = if state >= 0.5 {
            (
                "idle_high",
                "above",
                port.electrical
                    .vih_min_v
                    .or(port
                        .electrical
                        .drive_high_voltage_v
                        .map(|level| level * 0.7))
                    .unwrap_or(2.31),
            )
        } else {
            (
                "idle_low",
                "below",
                port.electrical.vil_max_v.unwrap_or(0.8),
            )
        };
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_{}", probe.probe_name, suffix),
            &probe.probe_name,
            "mean",
            relation,
            threshold,
            (0.0, stop_time_us),
        ));
    }
    let frequency_hz =
        component_parameter_f64(component, "observation_pwm_frequency_hz").unwrap_or(50.0);
    for pin in ["PWM0", "PWM1", "PWM2", "PWM3"] {
        let Some(probe) = probe_for_component_pin(probes, component, pin) else {
            continue;
        };
        let Some(port) = model.ports.get(pin) else {
            continue;
        };
        let high_level = component_parameter_f64(component, "observation_pwm_high_v")
            .or(port.electrical.drive_high_voltage_v)
            .unwrap_or(3.3);
        let duty_percent = component_parameter_f64(
            component,
            &format!("observation_{}_duty_percent", pin.to_ascii_lowercase()),
        )
        .unwrap_or(7.5);
        let (high_at_us, low_at_us) = pwm_sample_times_us(frequency_hz, duty_percent, stop_time_us);
        assertions.push(default_sample_assertion(
            scenario_name,
            &format!("{}_pwm_high_sample", probe.probe_name),
            &probe.probe_name,
            "above",
            high_level * 0.7,
            high_at_us,
        ));
        assertions.push(default_sample_assertion(
            scenario_name,
            &format!("{}_pwm_low_sample", probe.probe_name),
            &probe.probe_name,
            "below",
            high_level * 0.3,
            low_at_us,
        ));
    }
}

pub(super) fn add_level_shifter_observation_assertions(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    for channel in &model.signal_conditioning.channels {
        if channel.kind != crate::library::SignalConditioningKind::LevelShifter {
            continue;
        }
        let Some(side_a_probe) = probe_for_component_pin(probes, component, &channel.side_a_pin)
        else {
            continue;
        };
        let Some(side_b_probe) = probe_for_component_pin(probes, component, &channel.side_b_pin)
        else {
            continue;
        };
        let Some(side_a_supply_pin) = channel.side_a_supply_pin.as_deref() else {
            continue;
        };
        let Some(side_b_supply_pin) = channel.side_b_supply_pin.as_deref() else {
            continue;
        };
        let Some(side_a_v) = voltage_for_component_pin(project, component, side_a_supply_pin)
        else {
            continue;
        };
        let Some(side_b_v) = voltage_for_component_pin(project, component, side_b_supply_pin)
        else {
            continue;
        };
        add_port_voltage_window_assertions(
            component,
            model,
            probes,
            scenario_name,
            side_a_supply_pin,
            stop_time_us,
            assertions,
        );
        add_port_voltage_window_assertions(
            component,
            model,
            probes,
            scenario_name,
            side_b_supply_pin,
            stop_time_us,
            assertions,
        );
        if let Some(enable_pin) = channel.enable_pin.as_deref()
            && let Some(enable_probe) = probe_for_component_pin(probes, component, enable_pin)
        {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_enable_high", enable_probe.probe_name),
                &enable_probe.probe_name,
                "mean",
                "above",
                side_a_v * 0.7,
                (0.0, stop_time_us),
            ));
        }
        let state = component_parameter_f64(
            component,
            &format!("observation_{}_state", channel.name.to_ascii_lowercase()),
        )
        .unwrap_or(1.0);
        if state >= 0.5 {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_input_high", side_a_probe.probe_name),
                &side_a_probe.probe_name,
                "mean",
                "above",
                side_a_v * 0.7,
                (0.0, stop_time_us),
            ));
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_translated_high", side_b_probe.probe_name),
                &side_b_probe.probe_name,
                "mean",
                "above",
                side_b_v * 0.7,
                (0.0, stop_time_us),
            ));
        } else {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_input_low", side_a_probe.probe_name),
                &side_a_probe.probe_name,
                "mean",
                "below",
                side_a_v * 0.3,
                (0.0, stop_time_us),
            ));
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_translated_low", side_b_probe.probe_name),
                &side_b_probe.probe_name,
                "mean",
                "below",
                side_b_v * 0.3,
                (0.0, stop_time_us),
            ));
        }
    }
}

pub(super) fn add_logic_buffer_observation_assertions(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    if model.category != "logic_buffer" {
        return;
    }
    add_port_voltage_window_assertions(
        component,
        model,
        probes,
        scenario_name,
        "VCC",
        stop_time_us,
        assertions,
    );
    let logic_high_v = voltage_for_component_pin(project, component, "VCC").unwrap_or(3.3);
    for (input_pin, output_pin) in [("1A", "1Y"), ("2A", "2Y")] {
        let state = component_parameter_f64(
            component,
            &format!("observation_{}_state", input_pin.to_ascii_lowercase()),
        )
        .unwrap_or(1.0);
        let state_suffix = logic_state_suffix(state);
        let (relation, threshold) = logic_state_relation_threshold(state, logic_high_v);
        if let Some(input_probe) = probe_for_component_pin(probes, component, input_pin) {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_input_{}", input_probe.probe_name, state_suffix),
                &input_probe.probe_name,
                "mean",
                relation,
                threshold,
                (0.0, stop_time_us),
            ));
        }
        if let Some(output_probe) = probe_for_component_pin(probes, component, output_pin) {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_output_{}", output_probe.probe_name, state_suffix),
                &output_probe.probe_name,
                "mean",
                relation,
                threshold,
                (0.0, stop_time_us),
            ));
        }
    }
}

pub(super) fn add_mcu_observation_assertions(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    if model.category != "mcu" {
        return;
    }
    add_port_voltage_window_assertions(
        component,
        model,
        probes,
        scenario_name,
        "3V3",
        stop_time_us,
        assertions,
    );
    let logic_high_v = voltage_for_component_pin(project, component, "3V3").unwrap_or(3.3);
    for (pin, parameter, high_suffix, low_suffix) in [
        ("EN", "observation_en_state", "released_high", "held_low"),
        ("IO0", "observation_io0_state", "boot_high", "boot_low"),
        ("IO2", "observation_io2_state", "strap_high", "strap_low"),
        ("IO46", "observation_io46_state", "strap_high", "strap_low"),
        (
            "IO19",
            "observation_usb_dm_state",
            "usb_dm_high",
            "usb_dm_low",
        ),
        (
            "IO20",
            "observation_usb_dp_state",
            "usb_dp_high",
            "usb_dp_low",
        ),
        ("TXD0", "observation_txd0_state", "idle_high", "idle_low"),
    ] {
        let state = component_parameter_f64(component, parameter).unwrap_or(1.0);
        let suffix = if state >= 0.5 {
            high_suffix
        } else {
            low_suffix
        };
        if let Some(probe) = probe_for_component_pin(probes, component, pin) {
            let Some(port) = model.ports.get(pin) else {
                continue;
            };
            let (relation, threshold) = port_state_relation_threshold(port, state, logic_high_v);
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_{}", probe.probe_name, suffix),
                &probe.probe_name,
                "mean",
                relation,
                threshold,
                (0.0, stop_time_us),
            ));
        }
    }
}

pub(super) fn add_protection_clamp_observation_assertions(
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    for clamp in &model.signal_conditioning.protection_clamps {
        let Some(max_v) = clamp.working_voltage_max_v else {
            continue;
        };
        let Some(probe) = probe_for_component_pin(probes, component, &clamp.protected_pin) else {
            continue;
        };
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_{}_standoff", probe.probe_name, clamp.name),
            &probe.probe_name,
            "max",
            "below",
            max_v,
            (0.0, stop_time_us),
        ));
    }
}

fn add_port_voltage_window_assertions(
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    pin: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    let Some(probe) = probe_for_component_pin(probes, component, pin) else {
        return;
    };
    let Some(port) = model.ports.get(pin) else {
        return;
    };
    if let Some(min_v) = port.electrical.operating_voltage_min_v {
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_min_voltage", probe.probe_name),
            &probe.probe_name,
            "mean",
            "above",
            min_v,
            (0.0, stop_time_us),
        ));
    }
    if let Some(max_v) = port.electrical.operating_voltage_max_v {
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_max_voltage", probe.probe_name),
            &probe.probe_name,
            "mean",
            "below",
            max_v,
            (0.0, stop_time_us),
        ));
    }
}

fn logic_state_suffix(state: f64) -> &'static str {
    if state >= 0.5 { "high" } else { "low" }
}

fn logic_state_relation_threshold(state: f64, logic_high_v: f64) -> (&'static str, f64) {
    if state >= 0.5 {
        ("above", logic_high_v * 0.7)
    } else {
        ("below", logic_high_v * 0.3)
    }
}

fn port_state_relation_threshold(
    port: &crate::library::Port,
    state: f64,
    logic_high_v: f64,
) -> (&'static str, f64) {
    if state >= 0.5 {
        (
            "above",
            port.electrical
                .vih_min_v
                .or(port
                    .electrical
                    .drive_high_voltage_v
                    .map(|level| level * 0.7))
                .unwrap_or(logic_high_v * 0.7),
        )
    } else {
        (
            "below",
            port.electrical
                .vil_max_v
                .unwrap_or((logic_high_v * 0.3).min(0.99)),
        )
    }
}

fn pwm_sample_times_us(frequency_hz: f64, duty_percent: f64, stop_time_us: f64) -> (f64, f64) {
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return (0.0, stop_time_us);
    }
    let period_us = 1_000_000.0 / frequency_hz;
    let high_width_us = (period_us * (duty_percent.clamp(0.1, 99.9) / 100.0))
        .min(period_us)
        .max(0.0);
    let high_at_us = (high_width_us * 0.5).clamp(0.0, stop_time_us);
    let low_at_us = (high_width_us + (period_us - high_width_us) * 0.5).clamp(0.0, stop_time_us);
    (high_at_us, low_at_us)
}
