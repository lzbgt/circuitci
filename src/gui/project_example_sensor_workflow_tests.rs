use super::CircuitCiApp;
use super::project::{GuiProjectExample, gui_project_examples};
use super::sketch::SketchSelection;

fn gui_project_example_by_id(id: &str) -> GuiProjectExample {
    gui_project_examples()
        .iter()
        .copied()
        .find(|example| example.id == id)
        .unwrap()
}

fn assert_observation_workflow(
    example_id: &str,
    component_id: &str,
    scenario_name: &str,
    expected_probes: &[&str],
    expected_assertions: &[&str],
) {
    let mut app = CircuitCiApp::default();

    app.request_project_example_load(gui_project_example_by_id(example_id), None);

    assert!(
        app.create_scope_example_observation_preset(),
        "{}",
        app.diagnostics.last().unwrap_or(&app.status)
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component(component_id.to_string()))
    );
    assert_eq!(app.analog_generated_scenario, scenario_name);
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(&app.project_yaml).unwrap();
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .unwrap();
    let analog = scenario.analog.as_ref().unwrap();
    for expected_probe in expected_probes {
        assert!(
            analog
                .probes
                .iter()
                .any(|probe| probe.name == *expected_probe),
            "expected generated probe {expected_probe}"
        );
    }
    for expected_assertion in expected_assertions {
        assert!(
            analog
                .assertions
                .iter()
                .any(|assertion| assertion.name == *expected_assertion),
            "expected generated assertion {expected_assertion}"
        );
    }
}

#[test]
fn icm42688p_scope_example_workflow_creates_model_aware_observation_checks() {
    assert_observation_workflow(
        "icm42688p_imu_scope",
        "UIMU",
        "uimu_observation",
        &[
            "v_uimu_vdd",
            "v_uimu_vddio",
            "v_uimu_sclk",
            "v_uimu_cs",
            "v_uimu_sdo",
            "v_uimu_int1",
        ],
        &[
            "v_uimu_vdd_min_voltage",
            "v_uimu_vddio_max_voltage",
            "v_uimu_sclk_spi_low",
            "v_uimu_cs_spi_high",
            "v_uimu_sdo_output_low",
            "v_uimu_int1_output_high",
        ],
    );
}

#[test]
fn bme280_i2c_scope_example_workflow_creates_model_aware_observation_checks() {
    assert_observation_workflow(
        "bme280_i2c_scope",
        "USENS",
        "usens_observation",
        &["v_usens_vdd", "v_usens_sdi", "v_usens_sdo", "v_usens_sck"],
        &[
            "v_usens_vdd_min_voltage",
            "v_usens_vdd_vddio_min_voltage",
            "v_usens_vdd_csb_i2c_select_high",
            "v_usens_sdi_i2c_idle_high",
            "v_usens_sck_i2c_idle_high",
            "v_usens_sdo_address_select_low",
        ],
    );
}

#[test]
fn sht31_i2c_scope_example_workflow_creates_model_aware_observation_checks() {
    assert_observation_workflow(
        "sht31_i2c_scope",
        "USENS",
        "usens_observation",
        &[
            "v_usens_vdd",
            "v_usens_sda",
            "v_usens_scl",
            "v_usens_addr",
            "v_usens_nreset",
            "v_usens_alert",
        ],
        &[
            "v_usens_vdd_min_voltage",
            "v_usens_sda_i2c_idle_high",
            "v_usens_scl_i2c_idle_high",
            "v_usens_addr_address_select_low",
            "v_usens_nreset_reset_released_high",
            "v_usens_alert_alert_idle_low",
        ],
    );
}

#[test]
fn w25q64jv_spi_flash_scope_example_workflow_creates_model_aware_observation_checks() {
    assert_observation_workflow(
        "w25q64jv_spi_flash_scope",
        "UFLASH",
        "uflash_observation",
        &[
            "v_uflash_vcc",
            "v_uflash_cs_n",
            "v_uflash_do_io1",
            "v_uflash_wp_n_io2",
            "v_uflash_di_io0",
            "v_uflash_clk",
            "v_uflash_hold_n_reset_n_io3",
        ],
        &[
            "v_uflash_vcc_min_voltage",
            "v_uflash_cs_n_standby_high",
            "v_uflash_wp_n_io2_write_protect_released_high",
            "v_uflash_hold_n_reset_n_io3_hold_reset_released_high",
            "v_uflash_clk_clk_idle_low",
            "v_uflash_di_io0_mosi_idle_low",
            "v_uflash_do_io1_miso_reference_low",
        ],
    );
}

#[test]
fn at24c02c_i2c_eeprom_scope_example_workflow_creates_model_aware_observation_checks() {
    assert_observation_workflow(
        "at24c02c_i2c_eeprom_scope",
        "UEEPROM",
        "ueeprom_observation",
        &[
            "v_ueeprom_vcc",
            "v_ueeprom_sda",
            "v_ueeprom_scl",
            "v_ueeprom_a0",
            "v_ueeprom_a1",
            "v_ueeprom_a2",
            "v_ueeprom_wp",
        ],
        &[
            "v_ueeprom_vcc_min_voltage",
            "v_ueeprom_sda_i2c_idle_high",
            "v_ueeprom_scl_i2c_idle_high",
            "v_ueeprom_a0_address_select_low",
            "v_ueeprom_a1_address_select_low",
            "v_ueeprom_a2_address_select_low",
            "v_ueeprom_wp_write_enable_low",
        ],
    );
}

#[test]
fn mcp23017_i2c_gpio_expander_scope_example_workflow_creates_model_aware_observation_checks() {
    assert_observation_workflow(
        "mcp23017_i2c_gpio_expander_scope",
        "UIOX",
        "uiox_observation",
        &[
            "v_uiox_vdd",
            "v_uiox_scl",
            "v_uiox_sda",
            "v_uiox_a0",
            "v_uiox_a1",
            "v_uiox_a2",
            "v_uiox_reset",
            "v_uiox_inta",
            "v_uiox_intb",
            "v_uiox_gpa0",
            "v_uiox_gpb0",
        ],
        &[
            "v_uiox_vdd_min_voltage",
            "v_uiox_sda_i2c_idle_high",
            "v_uiox_scl_i2c_idle_high",
            "v_uiox_a0_address_select_low",
            "v_uiox_a1_address_select_low",
            "v_uiox_a2_address_select_low",
            "v_uiox_reset_reset_released_high",
            "v_uiox_inta_interrupt_idle_high",
            "v_uiox_intb_interrupt_idle_high",
            "v_uiox_gpa0_gpio_reference_high",
            "v_uiox_gpb0_gpio_reference_low",
        ],
    );
}
