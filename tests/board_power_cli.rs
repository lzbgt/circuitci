mod common;

use common::{assert_report_schema_valid, run_validation};

#[test]
fn good_interface_protection_powered_passes() {
    let report = run_validation("examples/good_interface_protection_powered/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn interface_protection_unisolated_power_domains_fail() {
    let report = run_validation("examples/bad_interface_protection_unisolated/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "U3");
    assert_eq!(failure["measured"]["side_a_powered"], false);
    assert_eq!(failure["measured"]["side_b_powered"], true);
    assert_eq!(failure["limit"]["required_unpowered_isolation"], true);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_txs0108e_unpowered_side_requires_isolation_or_oe_evidence() {
    let report = run_validation("examples/bad_ti_txs0108e_unpowered_side/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "U3");
    assert_eq!(failure["measured"]["side_a_supply_net"], "rail_a");
    assert_eq!(failure["measured"]["side_a_powered"], false);
    assert_eq!(failure["measured"]["side_b_supply_net"], "rail_b");
    assert_eq!(failure["measured"]["side_b_powered"], true);
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("does not prove the channel is disabled")
    );
    assert_eq!(failure["limit"]["enable_pin"], "OE");
    assert_eq!(failure["limit"]["required_disabled_state"], "low");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_txs0108e_oe_low_allows_one_sided_power_review() {
    let report = run_validation("examples/good_ti_txs0108e_oe_low_unpowered_side/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_txs0108e_oe_low_requires_connected_enable_pin() {
    let report = run_validation("examples/bad_ti_txs0108e_oe_low_unconnected/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == "INTERFACE_PROTECTION_REVIEW")
        .expect("interface protection finding");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("does not prove the channel is disabled")
    );
    assert_eq!(failure["limit"]["enable_pin"], "OE");
    assert_eq!(failure["limit"]["required_disabled_state"], "low");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_txs0108e_supply_order_requires_vcca_not_above_vccb() {
    let report = run_validation("examples/bad_ti_txs0108e_supply_order/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["supply_constraint"] == "vcca_lte_vccb")
        .expect("supply-order finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "U3");
    assert_eq!(failure["measured"]["lower_supply_pin"], "VCCA");
    assert_eq!(failure["measured"]["upper_supply_pin"], "VCCB");
    assert_eq!(failure["measured"]["lower_nominal_voltage_V"], 5.0);
    assert_eq!(failure["measured"]["upper_nominal_voltage_V"], 3.3);
    assert_eq!(failure["limit"]["relation"], "less_than_or_equal");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_esd_clamp_protection_passes_static_review() {
    let report = run_validation("examples/good_usb_esd_protection/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn usb_esd_clamp_requires_declared_reference_net_kind() {
    let report = run_validation("examples/bad_usb_esd_reference/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["required_reference"] == "ground")
        .expect("ESD reference finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_shield");
    assert_eq!(
        failure["measured"]["reference_net_kind"],
        "digital_or_analog"
    );
    assert_eq!(failure["limit"]["protection_clamp"], "dp");
    assert_eq!(failure["limit"]["reference_pin"], "GND");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_esd_clamp_requires_standoff_above_protected_net_voltage() {
    let report = run_validation("examples/bad_usb_esd_standoff/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["working_voltage_max_V"] == 5.5)
        .expect("ESD standoff finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["protected_net_nominal_voltage_V"], 6.0);
    assert_eq!(failure["limit"]["protection_clamp"], "dp");
    assert_report_schema_valid(&report);
}

#[test]
fn usb_esd_clamp_capacitance_must_fit_interface_budget() {
    let report = run_validation("examples/bad_usb_esd_line_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["max_line_capacitance_F"] == 2.0e-12)
        .expect("ESD capacitance finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["line_capacitance_F"], 1.0e-11);
    assert_eq!(failure["limit"]["protection_clamp"], "dp");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tpd2eusb30_usb_esd_passes_static_review() {
    let report = run_validation("examples/good_ti_tpd2eusb30_usb_esd/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tpd2eusb30_usb_esd_requires_standoff_above_line_voltage() {
    let report = run_validation("examples/bad_ti_tpd2eusb30_usb_esd_standoff/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["working_voltage_max_V"] == 5.5)
        .expect("TPD2EUSB30 standoff finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["protected_net_nominal_voltage_V"], 6.0);
    assert_eq!(failure["limit"]["protection_clamp"], "d1_plus");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tpd2eusb30_usb_esd_line_capacitance_must_fit_budget() {
    let report = run_validation("examples/bad_ti_tpd2eusb30_usb_esd_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"]["max_line_capacitance_F"] == 5.0e-13)
        .expect("TPD2EUSB30 capacitance finding");
    assert_eq!(failure["id"], "INTERFACE_PROTECTION_REVIEW");
    assert_eq!(failure["component"], "UESD");
    assert_eq!(failure["net"], "usb_dp");
    assert_eq!(failure["measured"]["line_capacitance_F"], 7.0e-13);
    assert_eq!(failure["limit"]["protection_clamp"], "d1_plus");
    assert_report_schema_valid(&report);
}

#[test]
fn good_power_tree_board_passes() {
    let report = run_validation("examples/good_power_tree_board/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn good_load_switch_power_tree_passes_with_enable_evidence() {
    let report = run_validation("examples/good_load_switch_power_tree/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn load_switch_powered_output_requires_enable_evidence() {
    let report = run_validation("examples/bad_load_switch_missing_enable/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("required_enabled_state").is_some())
        .expect("expected load switch enable finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USW");
    assert_eq!(failure["net"], "sensor_3v3");
    assert_eq!(failure["measured"]["control_state"], "missing");
    assert_eq!(failure["limit"]["control_pin"], "EN");
    assert_eq!(failure["limit"]["required_enabled_state"], "high");
    assert_report_schema_valid(&report);
}

#[test]
fn load_switch_disabled_control_cannot_power_output_rail() {
    let report = run_validation("examples/bad_load_switch_disabled_output_powered/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("required_enabled_state").is_some())
        .expect("expected load switch enable finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USW");
    assert_eq!(failure["net"], "sensor_3v3");
    assert_eq!(failure["measured"]["control_state"], "low");
    assert_eq!(failure["limit"]["required_enabled_state"], "high");
    assert_report_schema_valid(&report);
}

#[test]
fn load_switch_output_current_limit_fails() {
    let report = run_validation("examples/bad_load_switch_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("load_switch_max_output_current_A")
                .is_some()
        })
        .expect("expected load switch output current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USW");
    assert_eq!(failure["net"], "sensor_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 0.07);
    assert_eq!(failure["limit"]["load_switch_max_output_current_A"], 0.05);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tps22918_load_switch_power_tree_passes_with_on_evidence() {
    let report = run_validation("examples/good_ti_tps22918_load_switch/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tps22918_powered_output_requires_on_evidence() {
    let report = run_validation("examples/bad_ti_tps22918_missing_on/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("required_enabled_state").is_some())
        .expect("expected TPS22918 ON evidence finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USW");
    assert_eq!(failure["net"], "sensor_3v3");
    assert_eq!(failure["measured"]["control_state"], "missing");
    assert_eq!(failure["limit"]["control_pin"], "ON");
    assert_eq!(failure["limit"]["required_enabled_state"], "high");
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tps22918_output_current_uses_datasheet_limit() {
    let report = run_validation("examples/bad_ti_tps22918_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("load_switch_max_output_current_A")
                .is_some()
        })
        .expect("expected TPS22918 output current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USW");
    assert_eq!(failure["net"], "sensor_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 2.1);
    assert_eq!(failure["limit"]["load_switch_max_output_current_A"], 2.0);
    assert_report_schema_valid(&report);
}

#[test]
fn microchip_mcp73831_usb_charger_passes_current_budget() {
    let report = run_validation("examples/good_microchip_mcp73831_usb_charger/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn microchip_mcp73831_charge_current_must_fit_usb_budget() {
    let report = run_validation("examples/bad_microchip_mcp73831_usb_budget/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("input_supply_current_limit_A")
                .is_some()
        })
        .expect("expected charger input budget finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UCHG");
    assert_eq!(failure["net"], "usb_5v");
    assert_eq!(failure["measured"]["programmed_charge_current_A"], 0.5);
    assert_eq!(failure["limit"]["input_supply_current_limit_A"], 0.1);
    assert_report_schema_valid(&report);
}

#[test]
fn microchip_mcp73831_charge_current_uses_datasheet_limit() {
    let report = run_validation("examples/bad_microchip_mcp73831_charge_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("battery_charger_max_charge_current_A")
                .is_some()
        })
        .expect("expected charger current limit finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UCHG");
    assert_eq!(failure["net"], "battery");
    assert_eq!(failure["measured"]["programmed_charge_current_A"], 0.6);
    assert_eq!(
        failure["limit"]["battery_charger_max_charge_current_A"],
        0.5
    );
    assert_report_schema_valid(&report);
}

#[test]
fn microchip_mcp73831_requires_programmed_charge_current() {
    let report = run_validation("examples/bad_microchip_mcp73831_missing_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("required_component_parameter")
                .is_some()
        })
        .expect("expected missing charger current parameter finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UCHG");
    assert_eq!(
        failure["limit"]["required_component_parameter"],
        "programmed_charge_current_A"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn good_power_mux_usb_selected_passes_with_reverse_blocking() {
    let report = run_validation("examples/good_power_mux_usb_selected/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn power_mux_requires_selected_input_parameter() {
    let report = run_validation("examples/bad_power_mux_missing_selection/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("required_component_parameter")
                .is_some()
        })
        .expect("expected selected input parameter finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UMUX");
    assert_eq!(
        failure["limit"]["required_component_parameter"],
        "selected_input"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn power_mux_selected_input_must_be_powered() {
    let report = run_validation("examples/bad_power_mux_selected_unpowered/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("selected_input_powered").is_some())
        .expect("expected selected unpowered input finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UMUX");
    assert_eq!(failure["net"], "sys");
    assert_eq!(failure["measured"]["selected_input"], "battery");
    assert_eq!(failure["measured"]["selected_input_powered"], false);
    assert_eq!(failure["limit"]["selected_input_powered"], true);
    assert_report_schema_valid(&report);
}

#[test]
fn power_mux_inactive_unpowered_input_requires_reverse_blocking() {
    let report = run_validation("examples/bad_power_mux_backfeed/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("required_reverse_blocking").is_some())
        .expect("expected reverse-blocking finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UMUX");
    assert_eq!(failure["net"], "battery");
    assert_eq!(failure["measured"]["inactive_input"], "battery");
    assert_eq!(failure["measured"]["inactive_input_powered"], false);
    assert_eq!(failure["measured"]["output_powered"], true);
    assert_eq!(failure["limit"]["required_reverse_blocking"], true);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tps2115a_power_mux_passes_with_selected_source_and_reverse_blocking() {
    let report = run_validation("examples/good_ti_tps2115a_power_mux/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tps2115a_output_current_uses_datasheet_limit() {
    let report = run_validation("examples/bad_ti_tps2115a_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("power_mux_max_output_current_A")
                .is_some()
        })
        .expect("expected power mux output current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UMUX");
    assert_eq!(failure["net"], "sys_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 1.2);
    assert_eq!(failure["limit"]["power_mux_max_output_current_A"], 1.0);
    assert_report_schema_valid(&report);
}

#[test]
fn good_clock_source_crystal_passes_load_capacitance_check() {
    let report = run_validation("examples/good_clock_source_crystal/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn clock_source_load_capacitance_fails_outside_crystal_range() {
    let report = run_validation("examples/bad_clock_source_load_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == "CLOCK_SOURCE_VALID")
        .expect("expected clock source finding");
    assert_eq!(failure["component"], "U1");
    assert_eq!(failure["measured"]["crystal_component"], "Y1");
    assert_eq!(failure["measured"]["input_load_capacitance_F"], 8.0e-12);
    assert_eq!(failure["measured"]["output_load_capacitance_F"], 8.0e-12);
    assert_eq!(failure["measured"]["stray_capacitance_F"], 2.0e-12);
    assert_eq!(failure["measured"]["effective_load_capacitance_F"], 6.0e-12);
    assert_eq!(failure["limit"]["crystal_load_capacitance_min_F"], 10.0e-12);
    assert_eq!(failure["limit"]["crystal_load_capacitance_max_F"], 15.0e-12);
    assert_report_schema_valid(&report);
}

#[test]
fn power_tree_overvoltage_fails() {
    let report = run_validation("examples/bad_power_tree_overvoltage/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "POWER_TREE_VALID");
    assert_eq!(report["failures"][0]["component"], "U1");
    assert_eq!(report["failures"][0]["net"], "rail_5v");
    assert_eq!(report["failures"][0]["measured"]["nominal_voltage_V"], 5.0);
    assert_eq!(
        report["failures"][0]["limit"]["operating_voltage_maximum_V"],
        3.6
    );
    assert_report_schema_valid(&report);
}

#[test]
fn wch_ch340c_power_overvoltage_uses_datasheet_limit() {
    let report = run_validation("examples/bad_wch_ch340c_power_overvoltage/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["component"] == "U5" && finding["limit"]["operating_voltage_maximum_V"] == 5.3
        })
        .expect("CH340C VCC finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["measured"]["nominal_voltage_V"], 6.0);
    assert_eq!(failure["limit"]["operating_voltage_maximum_V"], 5.3);
    assert_report_schema_valid(&report);
}

#[test]
fn silabs_cp2102n_vdd_overvoltage_uses_datasheet_limit() {
    let report = run_validation("examples/bad_silabs_cp2102n_vdd_overvoltage/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["component"] == "U6" && finding["limit"]["operating_voltage_maximum_V"] == 3.6
        })
        .expect("CP2102N VDD finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["measured"]["nominal_voltage_V"], 5.0);
    assert_eq!(failure["limit"]["operating_voltage_maximum_V"], 3.6);
    assert_report_schema_valid(&report);
}

#[test]
fn io_voltage_vih_mismatch_fails() {
    let report = run_validation("examples/bad_io_voltage_vih_mismatch/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("receiver_vih_min_V").is_some())
        .expect("VIH mismatch finding");
    assert_eq!(failure["id"], "IO_VOLTAGE_COMPATIBLE");
    assert_eq!(failure["component"], "UMCU");
    assert_eq!(failure["net"], "sensor_irq");
    assert_eq!(failure["measured"]["driver_high_voltage_V"], 1.8);
    assert_eq!(failure["limit"]["receiver_vih_min_V"], 2.4);
    assert_report_schema_valid(&report);
}

#[test]
fn io_voltage_respects_imported_kicad_pin_direction_metadata() {
    let report = run_validation("examples/good_io_voltage_kicad_direction_metadata/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn io_voltage_backfeed_clamp_current_fails() {
    let report = run_validation("examples/bad_io_voltage_backfeed_clamp/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["measured"].get("injection_current_A").is_some())
        .expect("clamp-current finding");
    assert_eq!(failure["id"], "IO_VOLTAGE_COMPATIBLE");
    assert_eq!(failure["component"], "UMCU");
    assert_eq!(failure["net"], "wake_line");
    assert_eq!(failure["measured"]["driver_high_voltage_V"], 5.0);
    assert_eq!(failure["measured"]["receiver_rail_voltage_V"], 3.3);
    let injection_current_a = failure["measured"]["injection_current_A"].as_f64().unwrap();
    assert!((injection_current_a - 0.014).abs() < 1e-12);
    assert_eq!(failure["limit"]["injection_current_A"], 0.001);
    assert_report_schema_valid(&report);
}

#[test]
fn boot_strap_bias_divider_passes() {
    let report = run_validation("examples/good_bootstrap_bias_divider/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn boot_strap_bias_threshold_fails() {
    let report = run_validation("examples/bad_bootstrap_bias_threshold/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "BOOT_STRAP_BIAS_VALID");
    assert_eq!(failure["component"], "U1");
    assert_eq!(failure["net"], "boot0");
    assert_eq!(failure["measured"]["required_boot_mode"], "bootloader");
    assert_eq!(failure["measured"]["strap_voltage_V"], 1.65);
    assert_eq!(failure["limit"]["required_BOOT0"], "high");
    assert_eq!(failure["limit"]["vih_min_V"], 2.0);
    assert_report_schema_valid(&report);
}

#[test]
fn boot_strap_bias_current_limit_fails() {
    let report = run_validation("examples/bad_bootstrap_bias_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("max_strap_bias_current_A").is_some())
        .expect("strap current finding");
    assert_eq!(failure["id"], "BOOT_STRAP_BIAS_VALID");
    assert_eq!(failure["component"], "U1");
    assert_eq!(failure["net"], "boot0");
    assert_eq!(failure["measured"]["strap_voltage_V"], 3.0);
    let current_a = failure["measured"]["strap_bias_current_A"]
        .as_f64()
        .unwrap();
    assert!((current_a - 0.0003).abs() < 1e-12);
    assert_eq!(failure["limit"]["max_strap_bias_current_A"], 0.0001);
    assert_report_schema_valid(&report);
}

#[test]
fn power_tree_current_budget_fails() {
    let report = run_validation("examples/bad_power_tree_current_budget/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "POWER_TREE_VALID");
    assert_eq!(report["failures"][0]["net"], "rail_3v3");
    assert_eq!(
        report["failures"][0]["measured"]["declared_load_current_A"],
        0.05
    );
    assert_eq!(
        report["failures"][0]["limit"]["supply_current_limit_A"],
        0.04
    );
    assert_report_schema_valid(&report);
}

#[test]
fn good_regulator_power_tree_passes() {
    let report = run_validation("examples/good_regulator_power_tree/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn good_reset_supervisor_threshold_passes() {
    let report = run_validation("examples/good_reset_supervisor_threshold/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tlv803ea29_reset_supervisor_threshold_passes() {
    let report = run_validation("examples/good_ti_tlv803ea29_reset_supervisor/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ti_tlv803ea29_threshold_above_nominal_rail_fails() {
    let report = run_validation("examples/bad_ti_tlv803ea29_nominal_rail/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("reset_supervisor_threshold_max_V")
                .is_some()
        })
        .expect("expected TLV803EA29 threshold finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "USUP");
    assert_eq!(failure["net"], "rail_2v9");
    assert_eq!(failure["measured"]["monitored_nominal_voltage_V"], 2.9);
    assert_eq!(failure["limit"]["reset_supervisor_threshold_max_V"], 2.9886);
    assert_report_schema_valid(&report);
}

#[test]
fn reset_supervisor_threshold_below_load_minimum_fails() {
    let report = run_validation("examples/bad_reset_supervisor_threshold_too_low/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["id"] == "POWER_TREE_VALID"
                && finding["measured"]
                    .get("reset_supervisor_threshold_min_V")
                    .is_some()
        })
        .expect("expected reset-supervisor threshold finding");
    assert_eq!(failure["component"], "USUP_LOW_THRESHOLD");
    assert_eq!(failure["net"], "nrst");
    assert_eq!(failure["measured"]["reset_supervisor_threshold_min_V"], 2.4);
    assert_eq!(failure["measured"]["monitored_load_component"], "U1");
    assert_eq!(failure["measured"]["monitored_load_pin"], "VDD");
    assert_eq!(failure["limit"]["load_operating_voltage_min_V"], 2.7);
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_dropout_fails() {
    let report = run_validation("examples/bad_regulator_dropout/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("dropout_voltage_V").is_some())
        .expect("expected regulator dropout finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["input_voltage_V"], 3.4);
    assert_eq!(failure["measured"]["output_voltage_V"], 3.3);
    assert_eq!(failure["limit"]["dropout_voltage_V"], 0.3);
    assert_report_schema_valid(&report);
}

#[test]
fn diodes_ap2112k_3v3_regulator_passes_static_power_tree() {
    let report = run_validation("examples/good_diodes_ap2112k_3v3_regulator/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn diodes_ap2112k_3v3_dropout_uses_datasheet_limit() {
    let report = run_validation("examples/bad_diodes_ap2112k_3v3_dropout/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("dropout_voltage_V").is_some())
        .expect("expected AP2112 dropout finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["input_voltage_V"], 3.6);
    assert_eq!(failure["measured"]["output_voltage_V"], 3.3);
    let margin = failure["measured"]["dropout_margin_V"].as_f64().unwrap();
    assert!((margin - 0.3).abs() < 1e-12);
    assert_eq!(failure["limit"]["dropout_voltage_V"], 0.4);
    assert_report_schema_valid(&report);
}

#[test]
fn diodes_ap2112k_3v3_output_current_uses_datasheet_limit() {
    let report = run_validation("examples/bad_diodes_ap2112k_3v3_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_max_output_current_A")
                .is_some()
        })
        .expect("expected AP2112 output-current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 0.65);
    assert_eq!(failure["limit"]["regulator_max_output_current_A"], 0.6);
    assert_report_schema_valid(&report);
}

#[test]
fn diodes_ap2112k_3v3_output_capacitance_uses_datasheet_requirement() {
    let report = run_validation("examples/bad_diodes_ap2112k_3v3_output_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_output_capacitance_min_F")
                .is_some()
        })
        .expect("expected AP2112 output capacitance finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["support_capacitance_F"], 0.00000047);
    assert_eq!(failure["measured"]["support_capacitors"][0], "COUT");
    assert_eq!(
        failure["limit"]["regulator_output_capacitance_min_F"],
        0.000001
    );
    assert_eq!(failure["limit"]["power_conversion_pin"], "VOUT");
    assert_report_schema_valid(&report);
}

#[test]
fn ams1117_3v3_regulator_passes_static_power_tree() {
    let report = run_validation("examples/good_ams1117_3v3_regulator/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
    assert_report_schema_valid(&report);
}

#[test]
fn ams1117_3v3_dropout_uses_datasheet_limit() {
    let report = run_validation("examples/bad_ams1117_3v3_dropout/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("dropout_voltage_V").is_some())
        .expect("expected AMS1117 dropout finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["input_voltage_V"], 4.2);
    assert_eq!(failure["measured"]["output_voltage_V"], 3.3);
    let margin = failure["measured"]["dropout_margin_V"].as_f64().unwrap();
    assert!((margin - 0.9).abs() < 1e-12);
    assert_eq!(failure["limit"]["dropout_voltage_V"], 1.3);
    assert_report_schema_valid(&report);
}

#[test]
fn ams1117_3v3_minimum_load_uses_datasheet_requirement() {
    let report = run_validation("examples/bad_ams1117_3v3_minimum_load/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_min_output_current_A")
                .is_some()
        })
        .expect("expected AMS1117 minimum-load finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(
        failure["measured"]["declared_minimum_output_load_current_A"],
        0.002
    );
    assert_eq!(failure["limit"]["regulator_min_output_current_A"], 0.01);
    assert_report_schema_valid(&report);
}

#[test]
fn ams1117_3v3_output_current_uses_datasheet_regulation_limit() {
    let report = run_validation("examples/bad_ams1117_3v3_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_max_output_current_A")
                .is_some()
        })
        .expect("expected AMS1117 output-current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 0.85);
    assert_eq!(failure["limit"]["regulator_max_output_current_A"], 0.8);
    assert_report_schema_valid(&report);
}

#[test]
fn ams1117_3v3_output_capacitance_uses_datasheet_requirement() {
    let report = run_validation("examples/bad_ams1117_3v3_output_capacitance/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_output_capacitance_min_F")
                .is_some()
        })
        .expect("expected AMS1117 output capacitance finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["support_capacitance_F"], 0.000010);
    assert_eq!(failure["measured"]["support_capacitors"][0], "COUT");
    assert_eq!(
        failure["limit"]["regulator_output_capacitance_min_F"],
        0.000022
    );
    assert_eq!(failure["limit"]["power_conversion_pin"], "VOUT");
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_output_current_fails() {
    let report = run_validation("examples/bad_regulator_output_current/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("regulator_max_output_current_A")
                .is_some()
        })
        .expect("expected regulator output current finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["declared_output_load_current_A"], 0.05);
    assert_eq!(failure["limit"]["regulator_max_output_current_A"], 0.04);
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_conversion_metadata_fails_closed() {
    let report = run_validation("examples/bad_regulator_conversion_pin/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("power_conversion_field").is_some())
        .expect("expected power_conversion metadata finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["limit"]["power_conversion_field"], "output_pin");
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_startup_sequence_fails() {
    let report = run_validation("examples/bad_regulator_startup_sequence/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["limit"]
                .get("earliest_output_power_valid_at_us")
                .is_some()
        })
        .expect("expected regulator startup timing finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(failure["measured"]["input_power_valid_at_us"], 800.0);
    assert_eq!(failure["measured"]["output_power_valid_at_us"], 1200.0);
    assert_eq!(failure["measured"]["startup_delay_us"], 1000.0);
    assert_eq!(
        failure["limit"]["earliest_output_power_valid_at_us"],
        1800.0
    );
    assert_report_schema_valid(&report);
}

#[test]
fn regulator_startup_missing_timing_fails_closed() {
    let report = run_validation("examples/bad_regulator_startup_missing_timing/project.yaml");
    assert_eq!(report["result"], "fail");
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["limit"].get("required_rail_timing_field").is_some())
        .expect("expected missing rail timing finding");
    assert_eq!(failure["id"], "POWER_TREE_VALID");
    assert_eq!(failure["component"], "UREG");
    assert_eq!(failure["net"], "rail_3v3");
    assert_eq!(
        failure["limit"]["required_rail_timing_field"],
        "output_power_valid_at_us"
    );
    assert_report_schema_valid(&report);
}
