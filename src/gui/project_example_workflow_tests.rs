use super::project::{GuiProjectExample, gui_project_examples};
use super::sketch::SketchSelection;
use super::{CircuitCiApp, Stage};

fn gui_project_example_by_id(id: &str) -> GuiProjectExample {
    gui_project_examples()
        .iter()
        .copied()
        .find(|example| example.id == id)
        .unwrap()
}

#[test]
fn opamp_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("opamp_buffer_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.diagnostics.last().unwrap_or(&app.status)
    );
    assert!(app.project_yaml_dirty);
    assert_eq!(app.stage, Stage::Sketch);
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("XU1".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "xu1_observation");
    assert!(app.status.contains("Generated observation preset"));
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "xu1_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_xu1_inn"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_xu1_inn_tracks_input_high_below")
    );
}

#[test]
fn comparator_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(
        gui_project_example_by_id("comparator_threshold_scope"),
        None,
    );

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.diagnostics.last().unwrap_or(&app.status)
    );
    assert_eq!(app.analog_generated_scenario, "xu1_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "xu1_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_xu1_out"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_xu1_out_positive_input_low_state")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_xu1_out_positive_input_high_state")
    );
}

#[test]
fn ap2112k_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("ap2112k_ldo_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UREG".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "ureg_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "ureg_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_ureg_vout")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureg_vout_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureg_vout_max_voltage")
    );
}

#[test]
fn ams1117_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("ams1117_ldo_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.diagnostics.last().unwrap_or(&app.status)
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UREG".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "ureg_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "ureg_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_ureg_vout")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureg_vout_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureg_vout_max_voltage")
    );
}

#[test]
fn ch340c_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("ch340c_usb_uart_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UUSB".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uusb_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uusb_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uusb_txd"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uusb_txd_output_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uusb_dtr_n_output_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uusb_rts_n_output_high")
    );
}

#[test]
fn cp2102n_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("cp2102n_usb_uart_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UUSB".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uusb_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uusb_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uusb_vdd"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uusb_txd"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uusb_vdd_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uusb_txd_output_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uusb_dtr_output_low")
    );
}

#[test]
fn ft232r_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("ft232r_usb_uart_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UUSB".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uusb_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uusb_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uusb_3v3out")
    );
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uusb_txd"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uusb_3v3out_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uusb_txd_output_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uusb_dtr_n_output_low")
    );
}

#[test]
fn ch347_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("ch347_usb_jtag_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UDBG".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "udbg_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "udbg_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_udbg_tms"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_udbg_tck"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_udbg_txd1_output_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_udbg_tms_output_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_udbg_tck_output_low")
    );
}

#[test]
fn cmsis_dap_swd_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("cmsis_dap_swd_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UPROBE".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uprobe_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uprobe_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uprobe_swclk")
    );
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uprobe_swdio")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uprobe_swclk_output_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uprobe_swdio_output_high")
    );
}

#[test]
fn stm32l431_boot_uart_swd_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(
        gui_project_example_by_id("stm32l431_boot_uart_swd_scope"),
        None,
    );

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.diagnostics.last().unwrap_or(&app.status)
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UMCU".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "umcu_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "umcu_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_umcu_vdd"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_umcu_pa9"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_vdd_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_nrst_released_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_boot0_app_boot_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_pa9_usart1_tx_idle_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_pa13_swdio_idle_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_pa14_swclk_idle_low")
    );
}

#[test]
fn nrf52840_board_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("nrf52840_board_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.diagnostics.last().unwrap_or(&app.status)
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UNRF".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "unrf_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "unrf_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_unrf_vdd"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_unrf_vbus")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_unrf_vdd_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_unrf_vdd_vddh_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_unrf_vbus_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_unrf_nreset_released_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_unrf_swdio_swdio_idle_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_unrf_swdclk_swdclk_idle_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_unrf_p0_06_uart_tx_idle_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_unrf_usb_dp_usb_dp_idle_low")
    );
}

#[test]
fn esp32_s3_wroom_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(
        gui_project_example_by_id("esp32_s3_wroom_boot_usb_scope"),
        None,
    );

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UMCU".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "umcu_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "umcu_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_umcu_3v3"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_umcu_io20")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_3v3_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_en_released_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_io0_boot_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_io46_strap_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_io19_usb_dm_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_io20_usb_dp_high")
    );
}

#[test]
fn esp32_wroom_32e_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(
        gui_project_example_by_id("esp32_wroom_32e_boot_uart_scope"),
        None,
    );

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UESP".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uesp_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uesp_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uesp_3v3"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uesp_txd0")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesp_3v3_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesp_en_released_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesp_io0_boot_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesp_io2_strap_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesp_txd0_idle_high")
    );
}

#[test]
fn licheerv_nano_w_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("licheerv_nano_w_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("USOM".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "usom_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "usom_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_usom_5v"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_usom_uart0_tx_a16")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_usom_5v_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_usom_uart0_tx_a16_output_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_usom_uart0_rx_a17_input_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_usom_gpioa14_motion_en_output_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_usom_gpioa15_fault_irq_input_low")
    );
}

#[test]
fn at32f435_motion_core_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(
        gui_project_example_by_id("at32f435_motion_core_scope"),
        None,
    );

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UMCU".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "umcu_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "umcu_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_umcu_vdd"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_umcu_lrv_uart_tx")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_vdd_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_lrv_uart_rx_input_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_lrv_uart_tx_idle_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_motion_fault_irq_output_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_rs485_de_driver_disabled_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_servo_pwm_oe_output_high")
    );
}

#[test]
fn at32m416_motor_control_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(
        gui_project_example_by_id("at32m416_motor_control_scope"),
        None,
    );

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UMCU".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "umcu_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "umcu_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_umcu_vdd"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_umcu_pwm_uh")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_vdd_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_pwm_uh_drive_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_drv_nfault_fault_clear_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_drv_spi_cs_chip_select_idle_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_enc_b_input_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_umcu_fault_out_output_low")
    );
}

#[test]
fn txs0108e_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("txs0108e_level_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("ULS".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uls_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uls_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uls_a1"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uls_b1"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uls_vcca_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uls_oe_enable_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uls_b1_translated_high")
    );
}

#[test]
fn nl27wz17_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(
        gui_project_example_by_id("nl27wz17_logic_buffer_scope"),
        None,
    );

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UBUF".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "ubuf_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "ubuf_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_ubuf_vcc"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_ubuf_1a"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_ubuf_1y"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_ubuf_2y"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ubuf_vcc_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ubuf_1a_input_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ubuf_1y_output_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ubuf_2y_output_low")
    );
}

#[test]
fn tpd2eusb30_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tpd2eusb30_esd_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UESD".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uesd_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uesd_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uesd_d1"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uesd_d1_2")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesd_d1_d1_plus_standoff")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesd_d1_2_d1_minus_standoff")
    );
}

#[test]
fn prtr5v0u2x_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("prtr5v0u2x_esd_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UESD".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uesd_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uesd_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uesd_io1"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uesd_io2"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uesd_vcc"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesd_io1_io1_to_vcc_standoff")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesd_io2_io2_to_vcc_standoff")
    );
}

#[test]
fn esd2can24_q1_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("esd2can24_q1_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UESD".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uesd_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uesd_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uesd_canh")
    );
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uesd_canl")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesd_canh_canh_standoff")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesd_canl_canl_standoff")
    );
}

#[test]
fn tcan3413_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tcan3413_can_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UCAN".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "ucan_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "ucan_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_ucan_vcc"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_ucan_vio"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_ucan_txd"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_ucan_rxd"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_ucan_canh")
    );
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_ucan_canl")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ucan_rxd_output_low")
    );
}

#[test]
fn drv8323_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("drv8323_gate_driver_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UDRV".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "udrv_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "udrv_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_udrv_vm"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_udrv_dvdd")
    );
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_udrv_nfault")
    );
    assert!(analog.probes.iter().any(|probe| probe.name == "v_udrv_soa"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_udrv_nfault_output_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_udrv_sdo_output_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_udrv_soa_current_sense_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_udrv_soc_current_sense_max_voltage")
    );
}

#[test]
fn pca9685_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("pca9685_pwm_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UPWM".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "upwm_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "upwm_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.analysis.stop_time_us, 40000.0);
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_upwm_pwm0")
    );
    assert!(analog.probes.iter().any(|probe| probe.name == "v_upwm_scl"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_upwm_pwm0_pwm_high_sample")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_upwm_pwm0_pwm_low_sample")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_upwm_scl_idle_high")
    );
}

#[test]
fn icm42688p_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("icm42688p_imu_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UIMU".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uimu_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uimu_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uimu_vdd"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uimu_vddio")
    );
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uimu_sclk")
    );
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uimu_cs"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uimu_sdo"));
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uimu_int1")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uimu_vdd_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uimu_vddio_max_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uimu_sclk_spi_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uimu_cs_spi_high")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uimu_sdo_output_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uimu_int1_output_high")
    );
}

#[test]
fn esds552_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("esds552_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UESD".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uesd_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uesd_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uesd_a"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uesd_b"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesd_a_a_standoff")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uesd_b_b_standoff")
    );
}

#[test]
fn thvd1450_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("thvd1450_rs485_scope"), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.status
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UTRX".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "utrx_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "utrx_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_utrx_vcc"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_utrx_di"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_utrx_ro"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_utrx_a"));
    assert!(analog.probes.iter().any(|probe| probe.name == "v_utrx_b"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_utrx_ro_output_high")
    );
}

#[test]
fn tps54331_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tps54331_buck_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UBUCK".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "ubuck_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "ubuck_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_ubuck_vsense")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ubuck_vsense_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ubuck_vsense_max_voltage")
    );
}

#[test]
fn tps62162_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tps62162_buck_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UBUCK".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "ubuck_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "ubuck_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_ubuck_vos")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ubuck_vos_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ubuck_vos_max_voltage")
    );
}

#[test]
fn tps63802_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tps63802_buck_boost_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UREG".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "ureg_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "ureg_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_ureg_vout")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureg_vout_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureg_vout_max_voltage")
    );
}

#[test]
fn tps61023_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tps61023_boost_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UBOOST".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uboost_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uboost_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uboost_vout")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uboost_vout_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uboost_vout_max_voltage")
    );
}

#[test]
fn tps22918_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(
        gui_project_example_by_id("tps22918_load_switch_scope"),
        None,
    );

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("USW".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "usw_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "usw_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_usw_vout"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_usw_vout_enabled_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_usw_vout_enabled_max_voltage")
    );
}

#[test]
fn tps25948_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tps25948_efuse_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UEFUSE".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uefuse_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uefuse_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uefuse_vout")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uefuse_vout_enabled_min_voltage")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uefuse_vout_enabled_max_voltage")
    );
}

#[test]
fn tps24751_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tps24751_hot_swap_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UHOTSWAP".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uhotswap_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uhotswap_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uhotswap_vout")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| { assertion.name == "v_uhotswap_vout_enabled_min_voltage" })
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| { assertion.name == "v_uhotswap_vout_enabled_max_voltage" })
    );
}

#[test]
fn tps2115a_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tps2115a_power_mux_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UMUX".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "umux_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "umux_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_umux_out"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| { assertion.name == "v_umux_out_selected_source_min_voltage" })
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| { assertion.name == "v_umux_out_selected_source_max_voltage" })
    );
}

#[test]
fn mcp73831_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("mcp73831_charger_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UCHG".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uchg_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uchg_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_uchg_vbat")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uchg_vbat_regulation_ceiling")
    );
}

#[test]
fn bq24075_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("bq24075_power_path_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("UCHG".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "uchg_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "uchg_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(analog.probes.iter().any(|probe| probe.name == "v_uchg_out"));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uchg_bat_regulation_ceiling")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_uchg_out_power_path_ceiling")
    );
}

#[test]
fn tlv803_scope_example_workflow_creates_model_aware_observation_checks() {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id("tlv803_reset_scope"), None);

    assert!(app.create_scope_example_observation_preset());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("URESET".to_string()))
    );
    assert_eq!(app.analog_generated_scenario, "ureset_observation");
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "ureset_observation")
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "v_ureset_reset")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureset_reset_asserted_low")
    );
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "v_ureset_reset_released_high")
    );
}
