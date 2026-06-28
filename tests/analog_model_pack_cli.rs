mod common;

use common::{assert_report_schema_valid, binary_available, run_validation};

#[test]
fn generated_generic_ideal_opamp_buffer_uses_model_pack() {
    let report = run_validation("examples/good_ideal_opamp_buffer/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_ap2112k_ldo_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_ap2112k_3v3_ldo_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_ch340c_usb_uart_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_wch_ch340c_usb_uart_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_cp2102n_usb_uart_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_silabs_cp2102n_usb_uart_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_ft232r_usb_uart_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_ftdi_ft232r_usb_uart_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_ch347_usb_jtag_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_wch_ch347_usb_jtag_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_cmsis_dap_swd_probe_observation_uses_source_backed_model_pack() {
    let report = run_validation("examples/good_cmsis_dap_swd_probe_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_txs0108e_level_shifter_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_ti_txs0108e_level_shifter_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tpd2eusb30_usb_esd_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tpd2eusb30_usb_esd_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_prtr5v0u2x_usb_esd_observation_uses_datasheet_backed_model_pack() {
    let report =
        run_validation("examples/good_nexperia_prtr5v0u2x_usb_esd_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_esd2can24_q1_can_esd_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_ti_esd2can24_q1_can_esd_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tcan3413_can_transceiver_observation_uses_datasheet_backed_model_pack() {
    let report =
        run_validation("examples/good_ti_tcan3413_can_transceiver_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_drv8323_gate_driver_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_drv8323_gate_driver_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_esds552_rs485_esd_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_ti_esds552_rs485_esd_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_thvd1450_rs485_transceiver_observation_uses_datasheet_backed_model_pack() {
    let report =
        run_validation("examples/good_ti_thvd1450_rs485_transceiver_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tps54331_buck_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tps54331_5v_buck_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tps62162_buck_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tps62162_3v3_buck_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tps63802_buck_boost_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tps63802_3v3_buck_boost_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tps61023_boost_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tps61023_5v_boost_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tps22918_load_switch_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tps22918_load_switch_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tps25948_efuse_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tps25948_efuse_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tps24751_hot_swap_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tps24751_hot_swap_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tps2115a_power_mux_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tps2115a_power_mux_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tps2121_power_mux_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tps2121_power_mux_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mcp73831_charger_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_mcp73831_charger_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_bq24075_power_path_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_bq24075_power_path_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_bq25798_nvdc_observation_uses_parameterized_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_bq25798_nvdc_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_pca9685_pwm_driver_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_pca9685_pwm_driver_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_tlv803ea29_reset_observation_uses_datasheet_backed_model_pack() {
    let report = run_validation("examples/good_tlv803ea29_reset_observation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/analog_behavioral.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}
