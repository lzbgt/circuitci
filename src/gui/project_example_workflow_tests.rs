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
