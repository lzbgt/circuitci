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

pub(super) fn add_imu_observation_assertions(
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    if model.category != "imu" {
        return;
    }
    for pin in ["VDD", "VDDIO"] {
        add_port_voltage_window_assertions(
            component,
            model,
            probes,
            scenario_name,
            pin,
            stop_time_us,
            assertions,
        );
    }
    for (pin, default_state) in [("SCLK", 0.0), ("SDI", 0.0), ("CS", 1.0)] {
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
        .unwrap_or(default_state);
        let (suffix, relation, threshold) = if state >= 0.5 {
            (
                "spi_high",
                "above",
                port.electrical.vih_min_v.unwrap_or(2.0),
            )
        } else {
            ("spi_low", "below", port.electrical.vil_max_v.unwrap_or(0.8))
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
    for (pin, default_state) in [("SDO", 0.0), ("INT1", 1.0)] {
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
        .unwrap_or(default_state);
        let (suffix, relation, threshold) = if state >= 0.5 {
            (
                "output_high",
                "above",
                port.electrical
                    .drive_high_voltage_v
                    .map(|level| level * 0.7)
                    .unwrap_or(2.0),
            )
        } else {
            ("output_low", "below", 0.5)
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
}

pub(super) fn add_linux_som_observation_assertions(
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    if model.category != "linux_som" {
        return;
    }
    add_port_voltage_window_assertions(
        component,
        model,
        probes,
        scenario_name,
        "5V",
        stop_time_us,
        assertions,
    );
    for (pin, default_state) in [("UART0_RX_A17", 1.0), ("GPIOA15_FAULT_IRQ", 0.0)] {
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
        .unwrap_or(default_state);
        let (suffix, relation, threshold) = if state >= 0.5 {
            (
                "input_high",
                "above",
                port.electrical.vih_min_v.unwrap_or(2.0),
            )
        } else {
            (
                "input_low",
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
    for (pin, default_state) in [("UART0_TX_A16", 1.0), ("GPIOA14_MOTION_EN", 1.0)] {
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
        .unwrap_or(default_state);
        let (suffix, relation, threshold) = if state >= 0.5 {
            (
                "output_high",
                "above",
                port.electrical
                    .drive_high_voltage_v
                    .map(|level| level * 0.7)
                    .unwrap_or(2.0),
            )
        } else {
            ("output_low", "below", 0.5)
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
    add_port_voltage_window_assertions(
        component,
        model,
        probes,
        scenario_name,
        "VDD",
        stop_time_us,
        assertions,
    );
    for pin in ["VDDH", "VBUS"] {
        add_port_voltage_window_assertions(
            component,
            model,
            probes,
            scenario_name,
            pin,
            stop_time_us,
            assertions,
        );
    }
    let logic_high_v = voltage_for_component_pin(project, component, "3V3")
        .or_else(|| voltage_for_component_pin(project, component, "VDD"))
        .unwrap_or(3.3);
    for (pin, parameter, default_state, high_suffix, low_suffix) in [
        (
            "EN",
            "observation_en_state",
            1.0,
            "released_high",
            "held_low",
        ),
        (
            "nRESET",
            "observation_nreset_state",
            1.0,
            "released_high",
            "held_low",
        ),
        (
            "SWDIO",
            "observation_swdio_state",
            1.0,
            "swdio_idle_high",
            "swdio_low",
        ),
        (
            "SWDCLK",
            "observation_swdclk_state",
            0.0,
            "swdclk_high",
            "swdclk_idle_low",
        ),
        (
            "P0_06",
            "observation_p0_06_state",
            1.0,
            "uart_tx_idle_high",
            "uart_tx_low",
        ),
        (
            "P0_08",
            "observation_p0_08_state",
            1.0,
            "uart_rx_idle_high",
            "uart_rx_low",
        ),
        (
            "USB_DP",
            "observation_usb_dp_state",
            0.0,
            "usb_dp_high",
            "usb_dp_idle_low",
        ),
        (
            "USB_DM",
            "observation_usb_dm_state",
            0.0,
            "usb_dm_high",
            "usb_dm_idle_low",
        ),
        (
            "ANT",
            "observation_ant_state",
            0.0,
            "antenna_feed_high",
            "antenna_feed_low",
        ),
        ("IO0", "observation_io0_state", 1.0, "boot_high", "boot_low"),
        (
            "IO2",
            "observation_io2_state",
            1.0,
            "strap_high",
            "strap_low",
        ),
        (
            "IO46",
            "observation_io46_state",
            1.0,
            "strap_high",
            "strap_low",
        ),
        (
            "IO19",
            "observation_usb_dm_state",
            1.0,
            "usb_dm_high",
            "usb_dm_low",
        ),
        (
            "IO20",
            "observation_usb_dp_state",
            1.0,
            "usb_dp_high",
            "usb_dp_low",
        ),
        (
            "TXD0",
            "observation_txd0_state",
            1.0,
            "idle_high",
            "idle_low",
        ),
        (
            "LRV_UART_RX",
            "observation_lrv_uart_rx_state",
            1.0,
            "input_high",
            "input_low",
        ),
        (
            "LRV_UART_TX",
            "observation_lrv_uart_tx_state",
            1.0,
            "idle_high",
            "idle_low",
        ),
        (
            "LRV_MOTION_EN",
            "observation_lrv_motion_en_state",
            1.0,
            "input_high",
            "input_low",
        ),
        (
            "MOTION_FAULT_IRQ",
            "observation_motion_fault_irq_state",
            0.0,
            "output_high",
            "output_low",
        ),
        (
            "CAN_TX",
            "observation_can_tx_state",
            1.0,
            "idle_high",
            "idle_low",
        ),
        (
            "CAN_RX",
            "observation_can_rx_state",
            1.0,
            "input_high",
            "input_low",
        ),
        (
            "RS485_TX",
            "observation_rs485_tx_state",
            1.0,
            "idle_high",
            "idle_low",
        ),
        (
            "RS485_RX",
            "observation_rs485_rx_state",
            1.0,
            "input_high",
            "input_low",
        ),
        (
            "RS485_DE",
            "observation_rs485_de_state",
            0.0,
            "driver_enabled_high",
            "driver_disabled_low",
        ),
        (
            "SERVO_PWM_OE",
            "observation_servo_pwm_oe_state",
            1.0,
            "output_high",
            "output_low",
        ),
        (
            "PWM_UH",
            "observation_pwm_uh_state",
            1.0,
            "drive_high",
            "drive_low",
        ),
        (
            "PWM_UL",
            "observation_pwm_ul_state",
            0.0,
            "drive_high",
            "drive_low",
        ),
        (
            "PWM_VH",
            "observation_pwm_vh_state",
            0.0,
            "drive_high",
            "drive_low",
        ),
        (
            "PWM_VL",
            "observation_pwm_vl_state",
            1.0,
            "drive_high",
            "drive_low",
        ),
        (
            "PWM_WH",
            "observation_pwm_wh_state",
            0.0,
            "drive_high",
            "drive_low",
        ),
        (
            "PWM_WL",
            "observation_pwm_wl_state",
            0.0,
            "drive_high",
            "drive_low",
        ),
        (
            "DRV_EN",
            "observation_drv_en_state",
            1.0,
            "driver_enabled_high",
            "driver_disabled_low",
        ),
        (
            "DRV_NFAULT",
            "observation_drv_nfault_state",
            1.0,
            "fault_clear_high",
            "fault_asserted_low",
        ),
        (
            "DRV_SPI_SCK",
            "observation_drv_spi_sck_state",
            0.0,
            "clock_high",
            "clock_idle_low",
        ),
        (
            "DRV_SPI_MOSI",
            "observation_drv_spi_mosi_state",
            0.0,
            "data_high",
            "data_idle_low",
        ),
        (
            "DRV_SPI_MISO",
            "observation_drv_spi_miso_state",
            1.0,
            "input_high",
            "input_low",
        ),
        (
            "DRV_SPI_CS",
            "observation_drv_spi_cs_state",
            1.0,
            "chip_select_idle_high",
            "chip_select_asserted_low",
        ),
        (
            "ENC_A",
            "observation_enc_a_state",
            1.0,
            "input_high",
            "input_low",
        ),
        (
            "ENC_B",
            "observation_enc_b_state",
            0.0,
            "input_high",
            "input_low",
        ),
        (
            "ENC_Z",
            "observation_enc_z_state",
            0.0,
            "input_high",
            "input_low",
        ),
        (
            "ENABLE_IN",
            "observation_enable_in_state",
            1.0,
            "input_high",
            "input_low",
        ),
        (
            "FAULT_OUT",
            "observation_fault_out_state",
            0.0,
            "output_high",
            "output_low",
        ),
        (
            "NRST",
            "observation_nrst_state",
            1.0,
            "released_high",
            "held_low",
        ),
        (
            "BOOT0",
            "observation_boot0_state",
            0.0,
            "rom_boot_high",
            "app_boot_low",
        ),
        (
            "PA9",
            "observation_pa9_state",
            1.0,
            "usart1_tx_idle_high",
            "usart1_tx_low",
        ),
        (
            "PA10",
            "observation_pa10_state",
            1.0,
            "usart1_rx_idle_high",
            "usart1_rx_low",
        ),
        (
            "PA13",
            "observation_pa13_state",
            1.0,
            "swdio_idle_high",
            "swdio_low",
        ),
        (
            "PA14",
            "observation_pa14_state",
            0.0,
            "swclk_high",
            "swclk_idle_low",
        ),
    ] {
        let state = component_parameter_f64(component, parameter).unwrap_or(default_state);
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
    let assertion_stem = voltage_window_assertion_stem(&probe.probe_name, pin);
    if let Some(min_v) = port.electrical.operating_voltage_min_v {
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{assertion_stem}_min_voltage"),
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
            &format!("{assertion_stem}_max_voltage"),
            &probe.probe_name,
            "mean",
            "below",
            max_v,
            (0.0, stop_time_us),
        ));
    }
}

fn voltage_window_assertion_stem(probe_name: &str, pin: &str) -> String {
    let pin_stem = normalized_pin_stem(pin);
    if probe_name.ends_with(&format!("_{pin_stem}")) {
        probe_name.to_string()
    } else {
        format!("{probe_name}_{pin_stem}")
    }
}

fn normalized_pin_stem(pin: &str) -> String {
    let mut stem = String::new();
    for character in pin.chars() {
        if character.is_ascii_alphanumeric() {
            stem.push(character.to_ascii_lowercase());
        } else if !stem.ends_with('_') {
            stem.push('_');
        }
    }
    let stem = stem.trim_matches('_');
    if stem.is_empty() {
        "pin".to_string()
    } else {
        stem.to_string()
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
