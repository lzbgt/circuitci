const NE555_SCOPE_EXAMPLE_PROJECT: &str = "examples/ne555_astable_scope_smoke/project.yaml";
const NE555_SCOPE_EXAMPLE_NAME: &str = "ne555_astable_scope";
const NE555_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v(out)", "v(timing)", "v(vcc)", "i(VCC)", "i(VOUT)"];
const NE555_SCOPE_EXPECTED_FREQUENCY: &str = "about 1.46 kHz";
const RC_LOWPASS_SCOPE_EXAMPLE_PROJECT: &str = "examples/rc_lowpass_scope/project.yaml";
const RC_LOWPASS_SCOPE_EXAMPLE_NAME: &str = "rc_lowpass_scope";
const RC_LOWPASS_SCOPE_EXPECTED_TRACES: &[&str] = &["v(input)", "v(filtered)", "i(VSIN)"];
const RC_LOWPASS_SCOPE_EXPECTED_FREQUENCY: &str = "1.00 kHz sine, fc about 1.59 kHz";
const RC_MONTE_CARLO_EXAMPLE_PROJECT: &str =
    "examples/good_generated_rc_lowpass_monte_carlo_observation/project.yaml";
const RC_MONTE_CARLO_EXAMPLE_NAME: &str = "good_generated_rc_lowpass_monte_carlo_observation";
const RC_MONTE_CARLO_EXPECTED_TRACES: &[&str] = &[
    "input_gain_db",
    "filtered_gain_db",
    "filtered_phase_deg",
    "filtered_mag",
];
const RC_MONTE_CARLO_EXPECTED_FREQUENCY: &str =
    "5 sampled R/C tolerance Bode runs with yield and P5 margin criteria";
const COMPARATOR_THRESHOLD_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/comparator_threshold_scope/project.yaml";
const COMPARATOR_THRESHOLD_SCOPE_EXAMPLE_NAME: &str = "comparator_threshold_scope";
const COMPARATOR_THRESHOLD_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v(input)", "v(reference)", "v(output)", "v(vcc)"];
const COMPARATOR_THRESHOLD_SCOPE_EXPECTED_FREQUENCY: &str =
    "80 us input pulse crossing a 1.2 V reference";
const OPAMP_BUFFER_SCOPE_EXAMPLE_PROJECT: &str = "examples/good_ideal_opamp_buffer/project.yaml";
const OPAMP_BUFFER_SCOPE_EXAMPLE_NAME: &str = "good_ideal_opamp_buffer";
const OPAMP_BUFFER_SCOPE_EXPECTED_TRACES: &[&str] = &["v(input)", "v(output)", "v(vcc)"];
const OPAMP_BUFFER_SCOPE_EXPECTED_FREQUENCY: &str = "80 us input pulse through unity feedback";
const CH340C_USB_UART_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_wch_ch340c_usb_uart_observation/project.yaml";
const CH340C_USB_UART_SCOPE_EXAMPLE_NAME: &str = "good_wch_ch340c_usb_uart_observation";
const CH340C_USB_UART_SCOPE_EXPECTED_TRACES: &[&str] = &["v_vcc", "v_txd", "v_dtr_n", "v_rts_n"];
const CH340C_USB_UART_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V CH340C observation with TXD high, DTR# low, and RTS# high";
const CH340N_USB_UART_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_wch_ch340n_usb_uart_observation/project.yaml";
const CH340N_USB_UART_SCOPE_EXAMPLE_NAME: &str = "good_wch_ch340n_usb_uart_observation";
const CH340N_USB_UART_SCOPE_EXPECTED_TRACES: &[&str] = &["v_vcc", "v_txd", "v_rts_n"];
const CH340N_USB_UART_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V CH340N SOP-8 observation with TXD high and RTS# high";
const CP2102N_USB_UART_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_silabs_cp2102n_usb_uart_observation/project.yaml";
const CP2102N_USB_UART_SCOPE_EXAMPLE_NAME: &str = "good_silabs_cp2102n_usb_uart_observation";
const CP2102N_USB_UART_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_vregin", "v_vdd", "v_txd", "v_rts", "v_dtr"];
const CP2102N_USB_UART_SCOPE_EXPECTED_FREQUENCY: &str =
    "5 V VREGIN, regulated VDD/VIO, TXD high, RTS high, and DTR low";
const FT232R_USB_UART_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_ftdi_ft232r_usb_uart_observation/project.yaml";
const FT232R_USB_UART_SCOPE_EXAMPLE_NAME: &str = "good_ftdi_ft232r_usb_uart_observation";
const FT232R_USB_UART_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_vcc", "v_3v3out", "v_txd", "v_rts_n", "v_dtr_n"];
const FT232R_USB_UART_SCOPE_EXPECTED_FREQUENCY: &str =
    "5 V VCC, generated 3V3OUT/VCCIO, TXD high, RTS# high, and DTR# low";
const CH347_USB_JTAG_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_wch_ch347_usb_jtag_observation/project.yaml";
const CH347_USB_JTAG_SCOPE_EXAMPLE_NAME: &str = "good_wch_ch347_usb_jtag_observation";
const CH347_USB_JTAG_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vcc", "v_txd1", "v_tms", "v_tck", "v_tdi", "v_tdo", "v_trst",
];
const CH347_USB_JTAG_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V CH347 debug bridge with TXD1/TMS/TDI/TRST high and TCK low";
const CMSIS_DAP_SWD_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_cmsis_dap_swd_probe_observation/project.yaml";
const CMSIS_DAP_SWD_SCOPE_EXAMPLE_NAME: &str = "good_cmsis_dap_swd_probe_observation";
const CMSIS_DAP_SWD_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_vtref", "v_swclk", "v_swdio", "v_nreset", "v_swo"];
const CMSIS_DAP_SWD_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V target reference with CMSIS-DAP SWD idle high lines and released reset";
const STM32L431_BOOT_UART_SWD_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_stm32l431_boot_uart_swd_observation/project.yaml";
const STM32L431_BOOT_UART_SWD_SCOPE_EXAMPLE_NAME: &str = "good_stm32l431_boot_uart_swd_observation";
const STM32L431_BOOT_UART_SWD_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vdd",
    "v_nrst",
    "v_boot0",
    "v_pa9_usart1_tx",
    "v_pa10_usart1_rx",
    "v_pa13_swdio",
    "v_pa14_swclk",
];
const STM32L431_BOOT_UART_SWD_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V STM32L431 with NRST released, BOOT0 low, USART1 idle high, and SWD idle lines";
const ESP32_S3_WROOM_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_esp32_s3_wroom_boot_usb_observation/project.yaml";
const ESP32_S3_WROOM_SCOPE_EXAMPLE_NAME: &str = "good_esp32_s3_wroom_boot_usb_observation";
const ESP32_S3_WROOM_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_3v3", "v_en", "v_io0", "v_io46", "v_usb_dm", "v_usb_dp", "v_txd0",
];
const ESP32_S3_WROOM_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V ESP32-S3 module with EN high, GPIO0 high, GPIO46 low, USB D+ high, and D- low";
const ESP32_WROOM_32E_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_esp32_wroom_32e_boot_uart_observation/project.yaml";
const ESP32_WROOM_32E_SCOPE_EXAMPLE_NAME: &str = "good_esp32_wroom_32e_boot_uart_observation";
const ESP32_WROOM_32E_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_3v3", "v_en", "v_io0", "v_io2", "v_txd0"];
const ESP32_WROOM_32E_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V ESP32-WROOM-32E module with EN high, GPIO0 high, GPIO2 low, and TXD0 idle high";
const LICHEERV_NANO_W_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_sipeed_licheerv_nano_w_observation/project.yaml";
const LICHEERV_NANO_W_SCOPE_EXAMPLE_NAME: &str = "good_sipeed_licheerv_nano_w_observation";
const LICHEERV_NANO_W_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_5v",
    "v_uart0_tx",
    "v_uart0_rx",
    "v_motion_en",
    "v_fault_irq",
];
const LICHEERV_NANO_W_SCOPE_EXPECTED_FREQUENCY: &str =
    "5 V LicheeRV-Nano-W module with UART0 TX/RX high, motion enable high, and fault IRQ low";
const AT32F435_MOTION_CORE_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_artery_at32f435_motion_core_observation/project.yaml";
const AT32F435_MOTION_CORE_SCOPE_EXAMPLE_NAME: &str =
    "good_artery_at32f435_motion_core_observation";
const AT32F435_MOTION_CORE_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vdd",
    "v_lrv_uart_rx",
    "v_lrv_uart_tx",
    "v_lrv_motion_en",
    "v_motion_fault_irq",
    "v_can_tx",
    "v_can_rx",
    "v_rs485_tx",
    "v_rs485_rx",
    "v_rs485_de",
    "v_servo_pwm_oe",
];
const AT32F435_MOTION_CORE_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V AT32F435 motion core with Linux UART and field-bus control lines in idle/ready states";
const AT32M416_MOTOR_CONTROL_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_artery_at32m416_motor_control_observation/project.yaml";
const AT32M416_MOTOR_CONTROL_SCOPE_EXAMPLE_NAME: &str =
    "good_artery_at32m416_motor_control_observation";
const AT32M416_MOTOR_CONTROL_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vdd",
    "v_can_tx",
    "v_can_rx",
    "v_pwm_uh",
    "v_pwm_ul",
    "v_pwm_vl",
    "v_drv_en",
    "v_drv_nfault",
    "v_drv_spi_cs",
    "v_cur_u",
    "v_enc_a",
    "v_fault_out",
];
const AT32M416_MOTOR_CONTROL_SCOPE_EXPECTED_FREQUENCY: &str = "3.3 V AT32M416 motor-control MCU with driver, CAN, encoder, and control lines in idle/ready states";
const TXS0108E_LEVEL_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_ti_txs0108e_level_shifter_observation/project.yaml";
const TXS0108E_LEVEL_SCOPE_EXAMPLE_NAME: &str = "good_ti_txs0108e_level_shifter_observation";
const TXS0108E_LEVEL_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_vcca", "v_vccb", "v_oe", "v_a1", "v_b1", "i_b1_load"];
const TXS0108E_LEVEL_SCOPE_EXPECTED_FREQUENCY: &str =
    "1.8 V A-side input translated to a 3.3 V B-side high level";
const NL27WZ17_LOGIC_BUFFER_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_onsemi_nl27wz17_logic_buffer_observation/project.yaml";
const NL27WZ17_LOGIC_BUFFER_SCOPE_EXAMPLE_NAME: &str =
    "good_onsemi_nl27wz17_logic_buffer_observation";
const NL27WZ17_LOGIC_BUFFER_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_vcc", "v_1a", "v_1y", "v_2a", "v_2y"];
const NL27WZ17_LOGIC_BUFFER_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V NL27WZ17 buffer with 1A high mirrored to 1Y and 2A low mirrored to 2Y";
const TPD2EUSB30_ESD_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tpd2eusb30_usb_esd_observation/project.yaml";
const TPD2EUSB30_ESD_SCOPE_EXAMPLE_NAME: &str = "good_tpd2eusb30_usb_esd_observation";
const TPD2EUSB30_ESD_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_dp", "v_dm", "i_dp_source", "i_dm_source"];
const TPD2EUSB30_ESD_SCOPE_EXPECTED_FREQUENCY: &str =
    "normal USB data-line voltages below the 5.5 V TPD2EUSB30 standoff limit";
const PRTR5V0U2X_ESD_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_nexperia_prtr5v0u2x_usb_esd_observation/project.yaml";
const PRTR5V0U2X_ESD_SCOPE_EXAMPLE_NAME: &str = "good_nexperia_prtr5v0u2x_usb_esd_observation";
const PRTR5V0U2X_ESD_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vbus",
    "v_dp",
    "v_dm",
    "i_vbus_source",
    "i_dp_source",
    "i_dm_source",
];
const PRTR5V0U2X_ESD_SCOPE_EXPECTED_FREQUENCY: &str =
    "normal USB VBUS and data-line voltages below the 5.5 V PRTR5V0U2X standoff limit";
const ESD2CAN24_Q1_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_ti_esd2can24_q1_can_esd_observation/project.yaml";
const ESD2CAN24_Q1_SCOPE_EXAMPLE_NAME: &str = "good_ti_esd2can24_q1_can_esd_observation";
const ESD2CAN24_Q1_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_canh", "v_canl", "i_canh_source", "i_canl_source"];
const ESD2CAN24_Q1_SCOPE_EXPECTED_FREQUENCY: &str =
    "normal CANH/CANL voltages below the 24 V ESD2CAN24-Q1 standoff limit";
const TCAN3413_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_ti_tcan3413_can_transceiver_observation/project.yaml";
const TCAN3413_SCOPE_EXAMPLE_NAME: &str = "good_ti_tcan3413_can_transceiver_observation";
const TCAN3413_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vcc", "v_vio", "v_txd", "v_stb", "v_rxd", "v_canh", "v_canl",
];
const TCAN3413_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V normal-mode TCAN3413 CAN dominant line-state observation";
const DRV8323_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_drv8323_gate_driver_observation/project.yaml";
const DRV8323_SCOPE_EXAMPLE_NAME: &str = "good_drv8323_gate_driver_observation";
const DRV8323_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vm", "v_dvdd", "v_enable", "v_nfault", "v_sdo", "v_soa", "v_sob", "v_soc",
];
const DRV8323_SCOPE_EXPECTED_FREQUENCY: &str =
    "24 V VM and 3.3 V DVDD/ENABLE DRV8323 gate-driver observation";
const PCA9685_PWM_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_pca9685_pwm_driver_observation/project.yaml";
const PCA9685_PWM_SCOPE_EXAMPLE_NAME: &str = "good_pca9685_pwm_driver_observation";
const PCA9685_PWM_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vdd", "v_oe", "v_scl", "v_sda", "v_pwm0", "v_pwm1", "v_pwm2", "v_pwm3",
];
const PCA9685_PWM_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V enabled PCA9685 with 50 Hz low-load PWM output observation";
const ICM42688P_IMU_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tdk_icm42688p_imu_observation/project.yaml";
const ICM42688P_IMU_SCOPE_EXAMPLE_NAME: &str = "good_tdk_icm42688p_imu_observation";
const ICM42688P_IMU_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vdd", "v_vddio", "v_sclk", "v_sdi", "v_cs", "v_sdo", "v_int1",
];
const ICM42688P_IMU_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V ICM-42688-P IMU rails, SPI idle line states, SDO low, and INT1 high";
const ESDS552_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_ti_esds552_rs485_esd_observation/project.yaml";
const ESDS552_SCOPE_EXAMPLE_NAME: &str = "good_ti_esds552_rs485_esd_observation";
const ESDS552_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_rs485_a", "v_rs485_b", "i_a_source", "i_b_source"];
const ESDS552_SCOPE_EXPECTED_FREQUENCY: &str =
    "normal RS-485 A/B voltages below the 12 V ESDS552 standoff limit";
const THVD1450_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_ti_thvd1450_rs485_transceiver_observation/project.yaml";
const THVD1450_SCOPE_EXAMPLE_NAME: &str = "good_ti_thvd1450_rs485_transceiver_observation";
const THVD1450_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vcc",
    "v_di",
    "v_de",
    "v_re_n",
    "v_ro",
    "v_rs485_a",
    "v_rs485_b",
];
const THVD1450_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.3 V enabled THVD1450 RS-485 driver/receiver line-state observation";
const JST_XH_CONNECTOR_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_jst_xh_servo_connector_observation/project.yaml";
const JST_XH_CONNECTOR_SCOPE_EXAMPLE_NAME: &str = "good_jst_xh_servo_connector_observation";
const JST_XH_CONNECTOR_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_vcc_board", "v_vcc_load", "v_sig_load", "v_gnd_load"];
const JST_XH_CONNECTOR_SCOPE_EXPECTED_FREQUENCY: &str =
    "JST XH 3-pin servo connector contact-drop observation at 1 A equivalent load";
const JST_VH_CONNECTOR_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_jst_vh_actuator_bus_connector_observation/project.yaml";
const JST_VH_CONNECTOR_SCOPE_EXAMPLE_NAME: &str = "good_jst_vh_actuator_bus_connector_observation";
const JST_VH_CONNECTOR_SCOPE_EXPECTED_TRACES: &[&str] = &[
    "v_vbat_load",
    "v_aux5_load",
    "v_canh_load",
    "v_canl_load",
    "v_enable_load",
    "v_fault_load",
    "v_sync_load",
    "v_gnd_load",
];
const JST_VH_CONNECTOR_SCOPE_EXPECTED_FREQUENCY: &str =
    "JST VH 8-pin actuator-bus connector contact-drop observation at 5 A equivalent load";
const AP2112K_LDO_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_ap2112k_3v3_ldo_observation/project.yaml";
const AP2112K_LDO_SCOPE_EXAMPLE_NAME: &str = "good_ap2112k_3v3_ldo_observation";
const AP2112K_LDO_SCOPE_EXPECTED_TRACES: &[&str] = &["v_usb", "v_en", "v_rail3v3", "i_load"];
const AP2112K_LDO_SCOPE_EXPECTED_FREQUENCY: &str = "5 V enabled input, 3.3 V regulated load rail";
const AMS1117_LDO_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_ams1117_3v3_ldo_observation/project.yaml";
const AMS1117_LDO_SCOPE_EXAMPLE_NAME: &str = "good_ams1117_3v3_ldo_observation";
const AMS1117_LDO_SCOPE_EXPECTED_TRACES: &[&str] = &["v_usb", "v_rail3v3", "i_load"];
const AMS1117_LDO_SCOPE_EXPECTED_FREQUENCY: &str =
    "5 V input, 3.3 V AMS1117 rail, 22 uF output capacitor, and 10 mA load";
const TPS54331_BUCK_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tps54331_5v_buck_observation/project.yaml";
const TPS54331_BUCK_SCOPE_EXAMPLE_NAME: &str = "good_tps54331_5v_buck_observation";
const TPS54331_BUCK_SCOPE_EXPECTED_TRACES: &[&str] = &["v_input", "v_enable", "v_rail5v", "i_load"];
const TPS54331_BUCK_SCOPE_EXPECTED_FREQUENCY: &str =
    "12 V input, 3.3 V enable, and 5 V buck-regulator rail observation";
const TPS62162_BUCK_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tps62162_3v3_buck_observation/project.yaml";
const TPS62162_BUCK_SCOPE_EXAMPLE_NAME: &str = "good_tps62162_3v3_buck_observation";
const TPS62162_BUCK_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_input", "v_enable", "v_rail3v3", "i_load"];
const TPS62162_BUCK_SCOPE_EXPECTED_FREQUENCY: &str =
    "12 V input, 3.3 V enable, and 3.3 V buck-regulator rail observation";
const TPS63802_BUCK_BOOST_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tps63802_3v3_buck_boost_observation/project.yaml";
const TPS63802_BUCK_BOOST_SCOPE_EXAMPLE_NAME: &str = "good_tps63802_3v3_buck_boost_observation";
const TPS63802_BUCK_BOOST_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_battery", "v_enable", "v_rail3v3", "i_load"];
const TPS63802_BUCK_BOOST_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.7 V input, 3.3 V enable, and 3.3 V buck-boost rail observation";
const TPS61023_BOOST_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tps61023_5v_boost_observation/project.yaml";
const TPS61023_BOOST_SCOPE_EXAMPLE_NAME: &str = "good_tps61023_5v_boost_observation";
const TPS61023_BOOST_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_battery", "v_enable", "v_rail5v", "i_load"];
const TPS61023_BOOST_SCOPE_EXPECTED_FREQUENCY: &str =
    "3.7 V input, 3.3 V enable, and 5 V boost-regulator rail observation";
const TPS22918_LOAD_SWITCH_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tps22918_load_switch_observation/project.yaml";
const TPS22918_LOAD_SWITCH_SCOPE_EXAMPLE_NAME: &str = "good_tps22918_load_switch_observation";
const TPS22918_LOAD_SWITCH_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_usb", "v_on", "v_switched5v", "i_load"];
const TPS22918_LOAD_SWITCH_SCOPE_EXPECTED_FREQUENCY: &str =
    "5 V enabled load switch into a 1 kOhm load";
const TPS25948_EFUSE_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tps25948_efuse_observation/project.yaml";
const TPS25948_EFUSE_SCOPE_EXAMPLE_NAME: &str = "good_tps25948_efuse_observation";
const TPS25948_EFUSE_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_input", "v_enable", "v_protected12v", "i_load"];
const TPS25948_EFUSE_SCOPE_EXPECTED_FREQUENCY: &str =
    "12 V enabled eFuse/load-switch path into a 120 ohm load";
const TPS24751_HOT_SWAP_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tps24751_hot_swap_observation/project.yaml";
const TPS24751_HOT_SWAP_SCOPE_EXAMPLE_NAME: &str = "good_tps24751_hot_swap_observation";
const TPS24751_HOT_SWAP_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_input", "v_enable", "v_protected12v", "i_load"];
const TPS24751_HOT_SWAP_SCOPE_EXPECTED_FREQUENCY: &str =
    "12 V enabled hot-swap/reverse-blocking path into a 120 ohm load";
const TPS2121_POWER_MUX_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tps2121_power_mux_observation/project.yaml";
const TPS2121_POWER_MUX_SCOPE_EXAMPLE_NAME: &str = "good_tps2121_power_mux_observation";
const TPS2121_POWER_MUX_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_usb", "v_backup", "v_sys5v", "i_load"];
const TPS2121_POWER_MUX_SCOPE_EXPECTED_FREQUENCY: &str =
    "5 V USB-selected input through a TPS2121 power-mux rail observation";
const TPS2115A_POWER_MUX_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tps2115a_power_mux_observation/project.yaml";
const TPS2115A_POWER_MUX_SCOPE_EXAMPLE_NAME: &str = "good_tps2115a_power_mux_observation";
const TPS2115A_POWER_MUX_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_usb", "v_backup", "v_mode", "v_sys5v", "i_load"];
const TPS2115A_POWER_MUX_SCOPE_EXPECTED_FREQUENCY: &str =
    "5 V USB-selected input through a TPS2115A power-mux rail observation";
const MCP73831_CHARGER_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_mcp73831_charger_observation/project.yaml";
const MCP73831_CHARGER_SCOPE_EXAMPLE_NAME: &str = "good_mcp73831_charger_observation";
const MCP73831_CHARGER_SCOPE_EXPECTED_TRACES: &[&str] = &["v_usb", "v_bat", "i_charge"];
const MCP73831_CHARGER_SCOPE_EXPECTED_FREQUENCY: &str =
    "5 V USB input, 10 kOhm PROG resistor, and 100 mA charge observation";
const BQ24075_POWER_PATH_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_bq24075_power_path_observation/project.yaml";
const BQ24075_POWER_PATH_SCOPE_EXAMPLE_NAME: &str = "good_bq24075_power_path_observation";
const BQ24075_POWER_PATH_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_adapter", "v_sysout", "v_bat", "i_charge", "i_sys_load"];
const BQ24075_POWER_PATH_SCOPE_EXPECTED_FREQUENCY: &str =
    "6 V adapter input, 5.5 V OUT path, and 450 mA ISET charge observation";
const BQ25798_NVDC_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_bq25798_nvdc_observation/project.yaml";
const BQ25798_NVDC_SCOPE_EXAMPLE_NAME: &str = "good_bq25798_nvdc_observation";
const BQ25798_NVDC_SCOPE_EXPECTED_TRACES: &[&str] =
    &["v_adapter", "v_sysout", "v_bat", "i_charge", "i_sys_load"];
const BQ25798_NVDC_SCOPE_EXPECTED_FREQUENCY: &str =
    "20 V adapter input, 12 V SYS rail, and 2 A programmed charge observation";
const TLV803_RESET_SCOPE_EXAMPLE_PROJECT: &str =
    "examples/good_tlv803ea29_reset_observation/project.yaml";
const TLV803_RESET_SCOPE_EXAMPLE_NAME: &str = "good_tlv803ea29_reset_observation";
const TLV803_RESET_SCOPE_EXPECTED_TRACES: &[&str] = &["v_rail", "reset_n"];
const TLV803_RESET_SCOPE_EXPECTED_FREQUENCY: &str = "3.3 V rail ramp with reset release";
const LOOP_STABILITY_BODE_EXAMPLE_PROJECT: &str = "examples/loop_stability_bode_scope/project.yaml";
const LOOP_STABILITY_BODE_EXAMPLE_NAME: &str = "loop_stability_bode_scope";
const LOOP_STABILITY_BODE_EXPECTED_TRACES: &[&str] = &["loop_mag_db", "loop_phase_deg", "loop_mag"];
const LOOP_STABILITY_BODE_EXPECTED_FREQUENCY: &str =
    "Bode loop gain with phase margin >45 deg and gain margin >6 dB";
const DC_BIAS_EXAMPLE_PROJECT: &str = "examples/good_dc_bias_observation/project.yaml";
const DC_BIAS_EXAMPLE_NAME: &str = "good_dc_bias_observation";
const DC_BIAS_EXPECTED_TRACES: &[&str] = &["vin", "midpoint"];
const DC_BIAS_EXPECTED_FREQUENCY: &str = "DC operating point with 9 divider-tolerance corners";
const NOISE_OBSERVATION_EXAMPLE_PROJECT: &str = "examples/good_noise_observation/project.yaml";
const NOISE_OBSERVATION_EXAMPLE_NAME: &str = "good_noise_observation";
const NOISE_OBSERVATION_EXPECTED_TRACES: &[&str] = &[
    "onoise_density",
    "inoise_density",
    "onoise_total",
    "inoise_total",
];
const NOISE_OBSERVATION_EXPECTED_FREQUENCY: &str =
    "10 Hz to 100 kHz divider output and input-referred RMS noise";
const GUI_PROJECT_EXAMPLES: &[GuiProjectExample] = &[
    GuiProjectExample {
        id: "ne555_astable_scope",
        category: "Timer",
        open_label: "Open NE555 Scope Example",
        run_label: "Open NE555 + Run Scopes",
        workflow_title: "NE555 Scope Workflow",
        summary: "Astable-style timer output with timing-node and source-current traces.",
        project_path: NE555_SCOPE_EXAMPLE_PROJECT,
        project_name: NE555_SCOPE_EXAMPLE_NAME,
        expected_traces: NE555_SCOPE_EXPECTED_TRACES,
        expected_frequency: NE555_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
    GuiProjectExample {
        id: "rc_lowpass_scope",
        category: "Filter",
        open_label: "Open RC Low-Pass Scope Example",
        run_label: "Open RC Low-Pass + Run Scopes",
        workflow_title: "RC Low-Pass Scope Workflow",
        summary: "1 kHz sine into a first-order low-pass for input/output comparison.",
        project_path: RC_LOWPASS_SCOPE_EXAMPLE_PROJECT,
        project_name: RC_LOWPASS_SCOPE_EXAMPLE_NAME,
        expected_traces: RC_LOWPASS_SCOPE_EXPECTED_TRACES,
        expected_frequency: RC_LOWPASS_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
    GuiProjectExample {
        id: "rc_monte_carlo_bode",
        category: "Yield",
        open_label: "Open RC Monte Carlo Example",
        run_label: "Open RC Monte Carlo + Run Observations",
        workflow_title: "RC Monte Carlo Yield Workflow",
        summary: "Generated RC low-pass Bode run with sampled R/C tolerances and yield checks.",
        project_path: RC_MONTE_CARLO_EXAMPLE_PROJECT,
        project_name: RC_MONTE_CARLO_EXAMPLE_NAME,
        expected_traces: RC_MONTE_CARLO_EXPECTED_TRACES,
        expected_frequency: RC_MONTE_CARLO_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
    GuiProjectExample {
        id: "comparator_threshold_scope",
        category: "Comparator",
        open_label: "Open Comparator Threshold Example",
        run_label: "Open Comparator + Run Scopes",
        workflow_title: "Comparator Threshold Workflow",
        summary: "Pulse input against a DC reference for output-state threshold checks.",
        project_path: COMPARATOR_THRESHOLD_SCOPE_EXAMPLE_PROJECT,
        project_name: COMPARATOR_THRESHOLD_SCOPE_EXAMPLE_NAME,
        expected_traces: COMPARATOR_THRESHOLD_SCOPE_EXPECTED_TRACES,
        expected_frequency: COMPARATOR_THRESHOLD_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("XU1"),
    },
    GuiProjectExample {
        id: "opamp_buffer_scope",
        category: "Op-Amp",
        open_label: "Open Op-Amp Buffer Example",
        run_label: "Open Op-Amp Buffer + Run Scopes",
        workflow_title: "Op-Amp Buffer Workflow",
        summary: "Unity-gain buffer tracking a pulse input with output settling checks.",
        project_path: OPAMP_BUFFER_SCOPE_EXAMPLE_PROJECT,
        project_name: OPAMP_BUFFER_SCOPE_EXAMPLE_NAME,
        expected_traces: OPAMP_BUFFER_SCOPE_EXPECTED_TRACES,
        expected_frequency: OPAMP_BUFFER_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("XU1"),
    },
    GuiProjectExample {
        id: "ch340c_usb_uart_scope",
        category: "USB-UART",
        open_label: "Open CH340C USB-UART Example",
        run_label: "Open CH340C + Run Scopes",
        workflow_title: "CH340C USB-UART Workflow",
        summary: "Source-backed CH340C bridge output-state observation for boot/control lines.",
        project_path: CH340C_USB_UART_SCOPE_EXAMPLE_PROJECT,
        project_name: CH340C_USB_UART_SCOPE_EXAMPLE_NAME,
        expected_traces: CH340C_USB_UART_SCOPE_EXPECTED_TRACES,
        expected_frequency: CH340C_USB_UART_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UUSB"),
    },
    GuiProjectExample {
        id: "ch340n_usb_uart_scope",
        category: "USB-UART",
        open_label: "Open CH340N USB-UART Example",
        run_label: "Open CH340N + Run Scopes",
        workflow_title: "CH340N USB-UART Workflow",
        summary: "Source-backed SOP-8 CH340N bridge output-state observation.",
        project_path: CH340N_USB_UART_SCOPE_EXAMPLE_PROJECT,
        project_name: CH340N_USB_UART_SCOPE_EXAMPLE_NAME,
        expected_traces: CH340N_USB_UART_SCOPE_EXPECTED_TRACES,
        expected_frequency: CH340N_USB_UART_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UUSB"),
    },
    GuiProjectExample {
        id: "cp2102n_usb_uart_scope",
        category: "USB-UART",
        open_label: "Open CP2102N USB-UART Example",
        run_label: "Open CP2102N + Run Scopes",
        workflow_title: "CP2102N USB-UART Workflow",
        summary: "Source-backed CP2102N regulator and UART output-state observation.",
        project_path: CP2102N_USB_UART_SCOPE_EXAMPLE_PROJECT,
        project_name: CP2102N_USB_UART_SCOPE_EXAMPLE_NAME,
        expected_traces: CP2102N_USB_UART_SCOPE_EXPECTED_TRACES,
        expected_frequency: CP2102N_USB_UART_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UUSB"),
    },
    GuiProjectExample {
        id: "ft232r_usb_uart_scope",
        category: "USB-UART",
        open_label: "Open FT232R USB-UART Example",
        run_label: "Open FT232R + Run Scopes",
        workflow_title: "FT232R USB-UART Workflow",
        summary: "Source-backed FT232R bridge 3V3OUT and UART output-state observation.",
        project_path: FT232R_USB_UART_SCOPE_EXAMPLE_PROJECT,
        project_name: FT232R_USB_UART_SCOPE_EXAMPLE_NAME,
        expected_traces: FT232R_USB_UART_SCOPE_EXPECTED_TRACES,
        expected_frequency: FT232R_USB_UART_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UUSB"),
    },
    GuiProjectExample {
        id: "ch347_usb_jtag_scope",
        category: "Debug",
        open_label: "Open CH347 USB-JTAG Example",
        run_label: "Open CH347 + Run Scopes",
        workflow_title: "CH347 USB-JTAG Workflow",
        summary: "Source-backed CH347 USB-JTAG bridge line-state observation.",
        project_path: CH347_USB_JTAG_SCOPE_EXAMPLE_PROJECT,
        project_name: CH347_USB_JTAG_SCOPE_EXAMPLE_NAME,
        expected_traces: CH347_USB_JTAG_SCOPE_EXPECTED_TRACES,
        expected_frequency: CH347_USB_JTAG_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UDBG"),
    },
    GuiProjectExample {
        id: "cmsis_dap_swd_scope",
        category: "Debug",
        open_label: "Open CMSIS-DAP SWD Example",
        run_label: "Open CMSIS-DAP + Run Scopes",
        workflow_title: "CMSIS-DAP SWD Workflow",
        summary: "Source-backed generic CMSIS-DAP SWD probe line-state observation.",
        project_path: CMSIS_DAP_SWD_SCOPE_EXAMPLE_PROJECT,
        project_name: CMSIS_DAP_SWD_SCOPE_EXAMPLE_NAME,
        expected_traces: CMSIS_DAP_SWD_SCOPE_EXPECTED_TRACES,
        expected_frequency: CMSIS_DAP_SWD_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UPROBE"),
    },
    GuiProjectExample {
        id: "stm32l431_boot_uart_swd_scope",
        category: "MCU",
        open_label: "Open STM32L431 Boot/UART/SWD Example",
        run_label: "Open STM32L431 + Run Scopes",
        workflow_title: "STM32L431 Boot/UART/SWD Workflow",
        summary: "Source-backed STM32L431 rail, reset, BOOT0, USART1, and SWD line-state observation.",
        project_path: STM32L431_BOOT_UART_SWD_SCOPE_EXAMPLE_PROJECT,
        project_name: STM32L431_BOOT_UART_SWD_SCOPE_EXAMPLE_NAME,
        expected_traces: STM32L431_BOOT_UART_SWD_SCOPE_EXPECTED_TRACES,
        expected_frequency: STM32L431_BOOT_UART_SWD_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UMCU"),
    },
    GuiProjectExample {
        id: "esp32_s3_wroom_boot_usb_scope",
        category: "MCU",
        open_label: "Open ESP32-S3 Boot/USB Example",
        run_label: "Open ESP32-S3 + Run Scopes",
        workflow_title: "ESP32-S3 Boot/USB Workflow",
        summary: "Source-backed ESP32-S3 module supply, EN, boot-strap, and USB line-state observation.",
        project_path: ESP32_S3_WROOM_SCOPE_EXAMPLE_PROJECT,
        project_name: ESP32_S3_WROOM_SCOPE_EXAMPLE_NAME,
        expected_traces: ESP32_S3_WROOM_SCOPE_EXPECTED_TRACES,
        expected_frequency: ESP32_S3_WROOM_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UMCU"),
    },
    GuiProjectExample {
        id: "esp32_wroom_32e_boot_uart_scope",
        category: "MCU",
        open_label: "Open ESP32-WROOM-32E Boot/UART Example",
        run_label: "Open ESP32-WROOM-32E + Run Scopes",
        workflow_title: "ESP32-WROOM-32E Boot/UART Workflow",
        summary: "Source-backed ESP32-WROOM-32E module supply, EN, boot-strap, and UART line-state observation.",
        project_path: ESP32_WROOM_32E_SCOPE_EXAMPLE_PROJECT,
        project_name: ESP32_WROOM_32E_SCOPE_EXAMPLE_NAME,
        expected_traces: ESP32_WROOM_32E_SCOPE_EXPECTED_TRACES,
        expected_frequency: ESP32_WROOM_32E_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UESP"),
    },
    GuiProjectExample {
        id: "licheerv_nano_w_scope",
        category: "Module",
        open_label: "Open LicheeRV-Nano-W Example",
        run_label: "Open LicheeRV-Nano-W + Run Scopes",
        workflow_title: "LicheeRV-Nano-W Workflow",
        summary: "Source-backed LicheeRV-Nano-W module power, UART, motion-enable, and fault-IRQ observation.",
        project_path: LICHEERV_NANO_W_SCOPE_EXAMPLE_PROJECT,
        project_name: LICHEERV_NANO_W_SCOPE_EXAMPLE_NAME,
        expected_traces: LICHEERV_NANO_W_SCOPE_EXPECTED_TRACES,
        expected_frequency: LICHEERV_NANO_W_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("USOM"),
    },
    GuiProjectExample {
        id: "at32f435_motion_core_scope",
        category: "MCU",
        open_label: "Open AT32F435 Motion-Core Example",
        run_label: "Open AT32F435 + Run Scopes",
        workflow_title: "AT32F435 Motion-Core Workflow",
        summary: "Source-backed AT32F435 motion-core rail, UART, CAN, RS-485, and control-line observation.",
        project_path: AT32F435_MOTION_CORE_SCOPE_EXAMPLE_PROJECT,
        project_name: AT32F435_MOTION_CORE_SCOPE_EXAMPLE_NAME,
        expected_traces: AT32F435_MOTION_CORE_SCOPE_EXPECTED_TRACES,
        expected_frequency: AT32F435_MOTION_CORE_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UMCU"),
    },
    GuiProjectExample {
        id: "at32m416_motor_control_scope",
        category: "MCU",
        open_label: "Open AT32M416 Motor-Control Example",
        run_label: "Open AT32M416 + Run Scopes",
        workflow_title: "AT32M416 Motor-Control Workflow",
        summary: "Source-backed AT32M416 motor-control rail, CAN, PWM, driver, encoder, and fault-line observation.",
        project_path: AT32M416_MOTOR_CONTROL_SCOPE_EXAMPLE_PROJECT,
        project_name: AT32M416_MOTOR_CONTROL_SCOPE_EXAMPLE_NAME,
        expected_traces: AT32M416_MOTOR_CONTROL_SCOPE_EXPECTED_TRACES,
        expected_frequency: AT32M416_MOTOR_CONTROL_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UMCU"),
    },
    GuiProjectExample {
        id: "txs0108e_level_scope",
        category: "Level Shifter",
        open_label: "Open TXS0108E Level-Shifter Example",
        run_label: "Open TXS0108E + Run Scopes",
        workflow_title: "TXS0108E Level-Shifter Workflow",
        summary: "Source-backed TXS0108E enabled A-to-B mixed-voltage observation.",
        project_path: TXS0108E_LEVEL_SCOPE_EXAMPLE_PROJECT,
        project_name: TXS0108E_LEVEL_SCOPE_EXAMPLE_NAME,
        expected_traces: TXS0108E_LEVEL_SCOPE_EXPECTED_TRACES,
        expected_frequency: TXS0108E_LEVEL_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("ULS"),
    },
    GuiProjectExample {
        id: "nl27wz17_logic_buffer_scope",
        category: "Logic",
        open_label: "Open NL27WZ17 Logic Buffer Example",
        run_label: "Open NL27WZ17 + Run Scopes",
        workflow_title: "NL27WZ17 Logic Buffer Workflow",
        summary: "Source-backed dual Schmitt-buffer input/output line-state observation.",
        project_path: NL27WZ17_LOGIC_BUFFER_SCOPE_EXAMPLE_PROJECT,
        project_name: NL27WZ17_LOGIC_BUFFER_SCOPE_EXAMPLE_NAME,
        expected_traces: NL27WZ17_LOGIC_BUFFER_SCOPE_EXPECTED_TRACES,
        expected_frequency: NL27WZ17_LOGIC_BUFFER_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UBUF"),
    },
    GuiProjectExample {
        id: "jst_xh_servo_connector_scope",
        category: "Connector",
        open_label: "Open JST XH Connector Example",
        run_label: "Open JST XH + Run Scopes",
        workflow_title: "JST XH Connector Workflow",
        summary: "Source-backed JST XH mated-contact voltage-drop observation.",
        project_path: JST_XH_CONNECTOR_SCOPE_EXAMPLE_PROJECT,
        project_name: JST_XH_CONNECTOR_SCOPE_EXAMPLE_NAME,
        expected_traces: JST_XH_CONNECTOR_SCOPE_EXPECTED_TRACES,
        expected_frequency: JST_XH_CONNECTOR_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
    GuiProjectExample {
        id: "jst_vh_actuator_connector_scope",
        category: "Connector",
        open_label: "Open JST VH Connector Example",
        run_label: "Open JST VH + Run Scopes",
        workflow_title: "JST VH Connector Workflow",
        summary: "Source-backed JST VH actuator-bus mated-contact voltage-drop observation.",
        project_path: JST_VH_CONNECTOR_SCOPE_EXAMPLE_PROJECT,
        project_name: JST_VH_CONNECTOR_SCOPE_EXAMPLE_NAME,
        expected_traces: JST_VH_CONNECTOR_SCOPE_EXPECTED_TRACES,
        expected_frequency: JST_VH_CONNECTOR_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
    GuiProjectExample {
        id: "tpd2eusb30_esd_scope",
        category: "Protection",
        open_label: "Open TPD2EUSB30 ESD Example",
        run_label: "Open TPD2EUSB30 + Run Scopes",
        workflow_title: "TPD2EUSB30 USB ESD Workflow",
        summary: "Source-backed USB ESD standoff and line-capacitance observation.",
        project_path: TPD2EUSB30_ESD_SCOPE_EXAMPLE_PROJECT,
        project_name: TPD2EUSB30_ESD_SCOPE_EXAMPLE_NAME,
        expected_traces: TPD2EUSB30_ESD_SCOPE_EXPECTED_TRACES,
        expected_frequency: TPD2EUSB30_ESD_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UESD"),
    },
    GuiProjectExample {
        id: "prtr5v0u2x_esd_scope",
        category: "Protection",
        open_label: "Open PRTR5V0U2X ESD Example",
        run_label: "Open PRTR5V0U2X + Run Scopes",
        workflow_title: "PRTR5V0U2X USB ESD Workflow",
        summary: "Source-backed rail-to-rail USB ESD standoff and capacitance observation.",
        project_path: PRTR5V0U2X_ESD_SCOPE_EXAMPLE_PROJECT,
        project_name: PRTR5V0U2X_ESD_SCOPE_EXAMPLE_NAME,
        expected_traces: PRTR5V0U2X_ESD_SCOPE_EXPECTED_TRACES,
        expected_frequency: PRTR5V0U2X_ESD_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UESD"),
    },
    GuiProjectExample {
        id: "esd2can24_q1_scope",
        category: "Protection",
        open_label: "Open ESD2CAN24-Q1 Example",
        run_label: "Open ESD2CAN24-Q1 + Run Scopes",
        workflow_title: "ESD2CAN24-Q1 CAN ESD Workflow",
        summary: "Source-backed CAN ESD standoff and line-capacitance observation.",
        project_path: ESD2CAN24_Q1_SCOPE_EXAMPLE_PROJECT,
        project_name: ESD2CAN24_Q1_SCOPE_EXAMPLE_NAME,
        expected_traces: ESD2CAN24_Q1_SCOPE_EXPECTED_TRACES,
        expected_frequency: ESD2CAN24_Q1_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UESD"),
    },
    GuiProjectExample {
        id: "tcan3413_can_scope",
        category: "Interface",
        open_label: "Open TCAN3413 CAN Example",
        run_label: "Open TCAN3413 + Run Scopes",
        workflow_title: "TCAN3413 CAN Workflow",
        summary: "Source-backed CAN transceiver dominant line-state observation.",
        project_path: TCAN3413_SCOPE_EXAMPLE_PROJECT,
        project_name: TCAN3413_SCOPE_EXAMPLE_NAME,
        expected_traces: TCAN3413_SCOPE_EXPECTED_TRACES,
        expected_frequency: TCAN3413_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UCAN"),
    },
    GuiProjectExample {
        id: "drv8323_gate_driver_scope",
        category: "Motor Driver",
        open_label: "Open DRV8323 Gate-Driver Example",
        run_label: "Open DRV8323 + Run Scopes",
        workflow_title: "DRV8323 Gate-Driver Workflow",
        summary: "Source-backed DRV8323 supply, fault, SPI-output, and current-sense observation.",
        project_path: DRV8323_SCOPE_EXAMPLE_PROJECT,
        project_name: DRV8323_SCOPE_EXAMPLE_NAME,
        expected_traces: DRV8323_SCOPE_EXPECTED_TRACES,
        expected_frequency: DRV8323_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UDRV"),
    },
    GuiProjectExample {
        id: "pca9685_pwm_scope",
        category: "PWM Driver",
        open_label: "Open PCA9685 PWM Example",
        run_label: "Open PCA9685 + Run Scopes",
        workflow_title: "PCA9685 PWM Workflow",
        summary: "Source-backed PCA9685 VDD, OE, I2C idle, and low-load PWM output observation.",
        project_path: PCA9685_PWM_SCOPE_EXAMPLE_PROJECT,
        project_name: PCA9685_PWM_SCOPE_EXAMPLE_NAME,
        expected_traces: PCA9685_PWM_SCOPE_EXPECTED_TRACES,
        expected_frequency: PCA9685_PWM_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UPWM"),
    },
    GuiProjectExample {
        id: "icm42688p_imu_scope",
        category: "Sensor",
        open_label: "Open ICM-42688-P IMU Example",
        run_label: "Open ICM-42688-P + Run Scopes",
        workflow_title: "ICM-42688-P IMU Workflow",
        summary: "Source-backed ICM-42688-P rail, SPI line-state, and interrupt observation.",
        project_path: ICM42688P_IMU_SCOPE_EXAMPLE_PROJECT,
        project_name: ICM42688P_IMU_SCOPE_EXAMPLE_NAME,
        expected_traces: ICM42688P_IMU_SCOPE_EXPECTED_TRACES,
        expected_frequency: ICM42688P_IMU_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UIMU"),
    },
    GuiProjectExample {
        id: "esds552_scope",
        category: "Protection",
        open_label: "Open ESDS552 Example",
        run_label: "Open ESDS552 + Run Scopes",
        workflow_title: "ESDS552 RS-485 ESD Workflow",
        summary: "Source-backed RS-485/RS-422 ESD standoff and line-capacitance observation.",
        project_path: ESDS552_SCOPE_EXAMPLE_PROJECT,
        project_name: ESDS552_SCOPE_EXAMPLE_NAME,
        expected_traces: ESDS552_SCOPE_EXPECTED_TRACES,
        expected_frequency: ESDS552_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UESD"),
    },
    GuiProjectExample {
        id: "thvd1450_rs485_scope",
        category: "Interface",
        open_label: "Open THVD1450 RS-485 Example",
        run_label: "Open THVD1450 + Run Scopes",
        workflow_title: "THVD1450 RS-485 Workflow",
        summary: "Source-backed RS-485 transceiver line-state observation.",
        project_path: THVD1450_SCOPE_EXAMPLE_PROJECT,
        project_name: THVD1450_SCOPE_EXAMPLE_NAME,
        expected_traces: THVD1450_SCOPE_EXPECTED_TRACES,
        expected_frequency: THVD1450_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UTRX"),
    },
    GuiProjectExample {
        id: "ap2112k_ldo_scope",
        category: "Regulator",
        open_label: "Open AP2112K LDO Example",
        run_label: "Open AP2112K + Run Scopes",
        workflow_title: "AP2112K LDO Workflow",
        summary: "Enabled 3.3 V LDO rail with load-current and output-window checks.",
        project_path: AP2112K_LDO_SCOPE_EXAMPLE_PROJECT,
        project_name: AP2112K_LDO_SCOPE_EXAMPLE_NAME,
        expected_traces: AP2112K_LDO_SCOPE_EXPECTED_TRACES,
        expected_frequency: AP2112K_LDO_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UREG"),
    },
    GuiProjectExample {
        id: "ams1117_ldo_scope",
        category: "Regulator",
        open_label: "Open AMS1117 LDO Example",
        run_label: "Open AMS1117 + Run Scopes",
        workflow_title: "AMS1117 LDO Workflow",
        summary: "Fixed 3.3 V AMS1117 rail with output capacitor and minimum-load checks.",
        project_path: AMS1117_LDO_SCOPE_EXAMPLE_PROJECT,
        project_name: AMS1117_LDO_SCOPE_EXAMPLE_NAME,
        expected_traces: AMS1117_LDO_SCOPE_EXPECTED_TRACES,
        expected_frequency: AMS1117_LDO_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UREG"),
    },
    GuiProjectExample {
        id: "tps54331_buck_scope",
        category: "Regulator",
        open_label: "Open TPS54331 Buck Example",
        run_label: "Open TPS54331 + Run Scopes",
        workflow_title: "TPS54331 Buck Workflow",
        summary: "Enabled 12 V to 5 V buck-regulator rail observation with load-current checks.",
        project_path: TPS54331_BUCK_SCOPE_EXAMPLE_PROJECT,
        project_name: TPS54331_BUCK_SCOPE_EXAMPLE_NAME,
        expected_traces: TPS54331_BUCK_SCOPE_EXPECTED_TRACES,
        expected_frequency: TPS54331_BUCK_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UBUCK"),
    },
    GuiProjectExample {
        id: "tps62162_buck_scope",
        category: "Regulator",
        open_label: "Open TPS62162 Buck Example",
        run_label: "Open TPS62162 + Run Scopes",
        workflow_title: "TPS62162 Buck Workflow",
        summary: "Enabled 12 V to 3.3 V buck-regulator rail observation with load-current checks.",
        project_path: TPS62162_BUCK_SCOPE_EXAMPLE_PROJECT,
        project_name: TPS62162_BUCK_SCOPE_EXAMPLE_NAME,
        expected_traces: TPS62162_BUCK_SCOPE_EXPECTED_TRACES,
        expected_frequency: TPS62162_BUCK_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UBUCK"),
    },
    GuiProjectExample {
        id: "tps63802_buck_boost_scope",
        category: "Regulator",
        open_label: "Open TPS63802 Buck-Boost Example",
        run_label: "Open TPS63802 + Run Scopes",
        workflow_title: "TPS63802 Buck-Boost Workflow",
        summary: "Enabled Li-Ion input to 3.3 V buck-boost rail observation with load-current checks.",
        project_path: TPS63802_BUCK_BOOST_SCOPE_EXAMPLE_PROJECT,
        project_name: TPS63802_BUCK_BOOST_SCOPE_EXAMPLE_NAME,
        expected_traces: TPS63802_BUCK_BOOST_SCOPE_EXPECTED_TRACES,
        expected_frequency: TPS63802_BUCK_BOOST_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UREG"),
    },
    GuiProjectExample {
        id: "tps61023_boost_scope",
        category: "Regulator",
        open_label: "Open TPS61023 Boost Example",
        run_label: "Open TPS61023 + Run Scopes",
        workflow_title: "TPS61023 Boost Workflow",
        summary: "Enabled Li-Ion input to 5 V boost-regulator rail observation with load-current checks.",
        project_path: TPS61023_BOOST_SCOPE_EXAMPLE_PROJECT,
        project_name: TPS61023_BOOST_SCOPE_EXAMPLE_NAME,
        expected_traces: TPS61023_BOOST_SCOPE_EXPECTED_TRACES,
        expected_frequency: TPS61023_BOOST_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UBOOST"),
    },
    GuiProjectExample {
        id: "tps22918_load_switch_scope",
        category: "Load Switch",
        open_label: "Open TPS22918 Load Switch Example",
        run_label: "Open TPS22918 + Run Scopes",
        workflow_title: "TPS22918 Load Switch Workflow",
        summary: "Enabled 5 V load switch path with switched-rail and load-current checks.",
        project_path: TPS22918_LOAD_SWITCH_SCOPE_EXAMPLE_PROJECT,
        project_name: TPS22918_LOAD_SWITCH_SCOPE_EXAMPLE_NAME,
        expected_traces: TPS22918_LOAD_SWITCH_SCOPE_EXPECTED_TRACES,
        expected_frequency: TPS22918_LOAD_SWITCH_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("USW"),
    },
    GuiProjectExample {
        id: "tps25948_efuse_scope",
        category: "eFuse",
        open_label: "Open TPS25948 eFuse Example",
        run_label: "Open TPS25948 + Run Scopes",
        workflow_title: "TPS25948 eFuse Workflow",
        summary: "Enabled 12 V eFuse/load-switch path with protected-rail and load-current checks.",
        project_path: TPS25948_EFUSE_SCOPE_EXAMPLE_PROJECT,
        project_name: TPS25948_EFUSE_SCOPE_EXAMPLE_NAME,
        expected_traces: TPS25948_EFUSE_SCOPE_EXPECTED_TRACES,
        expected_frequency: TPS25948_EFUSE_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UEFUSE"),
    },
    GuiProjectExample {
        id: "tps24751_hot_swap_scope",
        category: "eFuse",
        open_label: "Open TPS24751 Hot-Swap Example",
        run_label: "Open TPS24751 + Run Scopes",
        workflow_title: "TPS24751 Hot-Swap Workflow",
        summary: "Enabled 12 V hot-swap/reverse-blocking path with protected-rail and load-current checks.",
        project_path: TPS24751_HOT_SWAP_SCOPE_EXAMPLE_PROJECT,
        project_name: TPS24751_HOT_SWAP_SCOPE_EXAMPLE_NAME,
        expected_traces: TPS24751_HOT_SWAP_SCOPE_EXPECTED_TRACES,
        expected_frequency: TPS24751_HOT_SWAP_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UHOTSWAP"),
    },
    GuiProjectExample {
        id: "tps2121_power_mux_scope",
        category: "Power Mux",
        open_label: "Open TPS2121 Power Mux Example",
        run_label: "Open TPS2121 + Run Scopes",
        workflow_title: "TPS2121 Power Mux Workflow",
        summary: "USB-selected 5 V power-mux path with output rail and load-current checks.",
        project_path: TPS2121_POWER_MUX_SCOPE_EXAMPLE_PROJECT,
        project_name: TPS2121_POWER_MUX_SCOPE_EXAMPLE_NAME,
        expected_traces: TPS2121_POWER_MUX_SCOPE_EXPECTED_TRACES,
        expected_frequency: TPS2121_POWER_MUX_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UMUX"),
    },
    GuiProjectExample {
        id: "tps2115a_power_mux_scope",
        category: "Power Mux",
        open_label: "Open TPS2115A Power Mux Example",
        run_label: "Open TPS2115A + Run Scopes",
        workflow_title: "TPS2115A Power Mux Workflow",
        summary: "USB-selected 5 V autoswitching power-mux path with output rail and load-current checks.",
        project_path: TPS2115A_POWER_MUX_SCOPE_EXAMPLE_PROJECT,
        project_name: TPS2115A_POWER_MUX_SCOPE_EXAMPLE_NAME,
        expected_traces: TPS2115A_POWER_MUX_SCOPE_EXPECTED_TRACES,
        expected_frequency: TPS2115A_POWER_MUX_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UMUX"),
    },
    GuiProjectExample {
        id: "mcp73831_charger_scope",
        category: "Charger",
        open_label: "Open MCP73831 Charger Example",
        run_label: "Open MCP73831 + Run Scopes",
        workflow_title: "MCP73831 Charger Workflow",
        summary: "USB-powered Li-Ion charger with PROG-current and VBAT checks.",
        project_path: MCP73831_CHARGER_SCOPE_EXAMPLE_PROJECT,
        project_name: MCP73831_CHARGER_SCOPE_EXAMPLE_NAME,
        expected_traces: MCP73831_CHARGER_SCOPE_EXPECTED_TRACES,
        expected_frequency: MCP73831_CHARGER_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UCHG"),
    },
    GuiProjectExample {
        id: "bq24075_power_path_scope",
        category: "Power Path",
        open_label: "Open BQ24075 Power Path Example",
        run_label: "Open BQ24075 + Run Scopes",
        workflow_title: "BQ24075 Power Path Workflow",
        summary: "Adapter-powered charger with OUT rail and BAT charge-current checks.",
        project_path: BQ24075_POWER_PATH_SCOPE_EXAMPLE_PROJECT,
        project_name: BQ24075_POWER_PATH_SCOPE_EXAMPLE_NAME,
        expected_traces: BQ24075_POWER_PATH_SCOPE_EXPECTED_TRACES,
        expected_frequency: BQ24075_POWER_PATH_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("UCHG"),
    },
    GuiProjectExample {
        id: "bq25798_nvdc_scope",
        category: "Power Path",
        open_label: "Open BQ25798 NVDC Example",
        run_label: "Open BQ25798 + Run Scopes",
        workflow_title: "BQ25798 NVDC Workflow",
        summary: "20 V adapter buck-boost/NVDC charger observation with SYS and BAT checks.",
        project_path: BQ25798_NVDC_SCOPE_EXAMPLE_PROJECT,
        project_name: BQ25798_NVDC_SCOPE_EXAMPLE_NAME,
        expected_traces: BQ25798_NVDC_SCOPE_EXPECTED_TRACES,
        expected_frequency: BQ25798_NVDC_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
    GuiProjectExample {
        id: "tlv803_reset_scope",
        category: "Reset",
        open_label: "Open TLV803 Reset Example",
        run_label: "Open TLV803 + Run Scopes",
        workflow_title: "TLV803 Reset Workflow",
        summary: "Reset-supervisor threshold release from a pulsed 3.3 V rail.",
        project_path: TLV803_RESET_SCOPE_EXAMPLE_PROJECT,
        project_name: TLV803_RESET_SCOPE_EXAMPLE_NAME,
        expected_traces: TLV803_RESET_SCOPE_EXPECTED_TRACES,
        expected_frequency: TLV803_RESET_SCOPE_EXPECTED_FREQUENCY,
        observation_preset_component: Some("URESET"),
    },
    GuiProjectExample {
        id: "loop_stability_bode_scope",
        category: "Stability",
        open_label: "Open Loop Stability Bode Example",
        run_label: "Open Loop Stability + Run Scopes",
        workflow_title: "Loop Stability Bode Workflow",
        summary: "Open-loop Bode response with executable phase and gain margin checks.",
        project_path: LOOP_STABILITY_BODE_EXAMPLE_PROJECT,
        project_name: LOOP_STABILITY_BODE_EXAMPLE_NAME,
        expected_traces: LOOP_STABILITY_BODE_EXPECTED_TRACES,
        expected_frequency: LOOP_STABILITY_BODE_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
    GuiProjectExample {
        id: "dc_bias_observation",
        category: "Bias",
        open_label: "Open DC Bias Example",
        run_label: "Open DC Bias + Run Observations",
        workflow_title: "DC Bias Observation Workflow",
        summary: "Generated operating-point divider bias with resistor-tolerance margin checks.",
        project_path: DC_BIAS_EXAMPLE_PROJECT,
        project_name: DC_BIAS_EXAMPLE_NAME,
        expected_traces: DC_BIAS_EXPECTED_TRACES,
        expected_frequency: DC_BIAS_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
    GuiProjectExample {
        id: "noise_observation",
        category: "Noise",
        open_label: "Open Noise Observation Example",
        run_label: "Open Noise + Run Observations",
        workflow_title: "Noise Observation Workflow",
        summary: "Generated divider noise density and integrated RMS noise checks.",
        project_path: NOISE_OBSERVATION_EXAMPLE_PROJECT,
        project_name: NOISE_OBSERVATION_EXAMPLE_NAME,
        expected_traces: NOISE_OBSERVATION_EXPECTED_TRACES,
        expected_frequency: NOISE_OBSERVATION_EXPECTED_FREQUENCY,
        observation_preset_component: None,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GuiProjectExample {
    pub(super) id: &'static str,
    pub(super) category: &'static str,
    pub(super) open_label: &'static str,
    pub(super) run_label: &'static str,
    pub(super) workflow_title: &'static str,
    pub(super) summary: &'static str,
    pub(super) project_path: &'static str,
    pub(super) project_name: &'static str,
    pub(super) expected_traces: &'static [&'static str],
    pub(super) expected_frequency: &'static str,
    pub(super) observation_preset_component: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScopeExampleWorkflowStatus {
    pub(super) title: &'static str,
    pub(super) state: &'static str,
    pub(super) action: &'static str,
    pub(super) expected_traces: &'static [&'static str],
    pub(super) expected_frequency: &'static str,
}

pub(super) fn gui_project_examples() -> &'static [GuiProjectExample] {
    GUI_PROJECT_EXAMPLES
}
