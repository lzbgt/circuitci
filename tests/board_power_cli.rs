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
fn good_power_tree_board_passes() {
    let report = run_validation("examples/good_power_tree_board/project.yaml");
    assert_eq!(report["result"], "pass");
    assert_eq!(report["summary"]["critical"], 0);
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
