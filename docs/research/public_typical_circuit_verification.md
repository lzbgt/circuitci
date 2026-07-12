# Public Typical Circuit Verification

This note records the public-reference circuits used to assess CircuitCI's
current static validation correctness and runtime behavior.

## Source Set

The original public documents are kept in the repository so later agents can
audit the modeled facts without relying on chat history.

| Circuit family | Public source | Local original |
| --- | --- | --- |
| Diodes AP2112K 3.3 V LDO typical application | <https://www.diodes.com/datasheet/download/AP2112.pdf> | `docs/research/datasheets/diodes/ap2112.pdf` |
| AMS1117-3.3 LDO use | <http://www.advanced-monolithic.com/pdf/ds1117.pdf> | `docs/research/datasheets/ams/ams1117.pdf` |
| Microchip MCP73831 USB Li-Ion charger typical application | <https://ww1.microchip.com/downloads/en/DeviceDoc/MCP73831-Family-Data-Sheet-DS20001984H.pdf> | `docs/research/datasheets/microchip/mcp73831-family-datasheet.pdf` |
| TI BQ24075 USB Li-Ion charger with power path typical use | <https://www.ti.com/lit/ds/symlink/bq24074.pdf> and peer `../urine_monitor` LCSC cache | `docs/research/datasheets/ti/bq24074.pdf` |
| TI BQ25798 NVDC buck-boost charger use | <https://www.ti.com/lit/ds/symlink/bq25798.pdf> | `docs/research/smart_robot/sources/bq25798_datasheet.pdf` |
| TI TPS2115A autoswitching power mux typical application | <https://www.ti.com/lit/ds/symlink/tps2115a.pdf> | `docs/research/datasheets/ti/tps2115a.pdf` |
| TI TPS2121 priority power mux typical use | <https://www.ti.com/lit/ds/symlink/tps2121.pdf> and peer `../urine_monitor` LCSC cache | `docs/research/datasheets/ti/tps2121.pdf` |
| TI TPS22918 load switch use | <https://www.ti.com/lit/gpn/TPS22918> | `docs/research/datasheets/ti/tps22918.pdf` |
| TI TPS25948 eFuse use | <https://www.ti.com/lit/ds/symlink/tps25948.pdf> | `docs/research/smart_robot/sources/tps25948_datasheet.pdf` |
| TI TPS24751 hot-swap/eFuse use | <https://www.ti.com/lit/ds/symlink/tps24751.pdf> and <https://www.ti.com/lit/gpn/CSD17501Q5A> | `docs/research/smart_robot/sources/tps24751_datasheet.pdf` and `docs/research/smart_robot/sources/csd17501q5a_datasheet.pdf` |
| TI TPD2EUSB30 USB ESD protection typical use | <https://www.ti.com/lit/ds/symlink/tpd2eusb30.pdf> | `docs/research/datasheets/ti/tpd2eusb30.pdf` |
| Nexperia PRTR5V0U2X rail-to-rail USB ESD protection typical use | <https://assets.nexperia.com/documents/data-sheet/PRTR5V0U2X.pdf> | `docs/research/datasheets/nexperia/prtr5v0u2x.pdf` |
| Nexperia PESD5V0S1UL VBUS ESD protection typical use | <https://assets.nexperia.com/documents/data-sheet/PESD5V0S1UL.pdf> | `docs/research/datasheets/nexperia/pesd5v0s1ul.pdf` |
| TI ESD2CAN24-Q1 CAN ESD protection use | <https://www.ti.com/lit/ds/symlink/esd2can24-q1.pdf> | `docs/research/smart_robot/sources/ti_esd2can24_q1_datasheet.pdf` |
| TI ESDS552 RS-485 ESD protection use | <https://www.ti.com/lit/gpn/ESDS552> | `docs/research/smart_robot/sources/esds552_datasheet.pdf` |
| TI TPS62162 3.3 V synchronous buck typical use | <https://www.ti.com/lit/ds/symlink/tps62160.pdf> | `docs/research/datasheets/ti/tps62160.pdf` |
| TI TPS54331 5 V buck use | <https://www.ti.com/lit/ds/symlink/tps54331.pdf> | `docs/research/smart_robot/sources/tps54331_datasheet.pdf` |
| TI TPS61023 5 V synchronous boost typical use | <https://www.ti.com/lit/ds/symlink/tps61023.pdf> and peer `../urine_monitor` LCSC cache | `docs/research/datasheets/ti/tps61023.pdf` |
| TI TPS63802 3.3 V synchronous buck-boost typical use | <https://www.ti.com/lit/ds/symlink/tps63802.pdf> and peer `../urine_monitor` fresh-design evidence | `docs/research/datasheets/ti/tps63802.pdf` |
| Espressif ESP32-WROOM-32E application boot module use | <https://www.espressif.com/sites/default/files/documentation/esp32-wroom-32e_esp32-wroom-32ue_datasheet_en.pdf> and <https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32/esp-hardware-design-guidelines-en-master-esp32.pdf> | `docs/research/datasheets/espressif/esp32-wroom-32e_esp32-wroom-32ue_datasheet_en.pdf` and `docs/research/datasheets/espressif/esp32_hardware_design_guidelines_en.pdf` |
| Raspberry Pi RP2040 MCU boot and power use | <https://datasheets.raspberrypi.com/rp2040/rp2040-datasheet.pdf> and <https://datasheets.raspberrypi.com/rp2040/hardware-design-with-rp2040.pdf> | `docs/research/datasheets/raspberrypi/rp2040-datasheet.pdf` and `docs/research/datasheets/raspberrypi/hardware-design-with-rp2040.pdf` |
| Nordic nRF52840 normal-voltage MCU and board-observation use | <https://docs.nordicsemi.com/bundle/ps_nrf52840/page/keyfeatures_html5.html> plus retained PDF mirror | `docs/research/datasheets/nordic/nrf52840-product-spec-farnell.pdf` |
| Silicon Labs CP2102N USB-UART bridge use | <https://www.silabs.com/documents/public/data-sheets/cp2102n-datasheet.pdf> | `docs/research/datasheets/silabs/cp2102n-datasheet.pdf` |
| WCH CH340C/CH340N USB-UART bridge use | official WCH metadata endpoints plus public PDF mirror | `docs/research/datasheets/wch/ch340ds1_sparkfun_mirror.pdf` |
| FTDI FT232R USB-UART bridge use | <https://ftdichip.com/wp-content/uploads/2020/08/DS_FT232R.pdf> plus public PDF mirror | `docs/research/datasheets/ftdi/DS_FT232R_sparkfun_mirror.pdf` |
| WCH CH347 USB-JTAG/debug bridge use | official WCH metadata endpoints plus public PDF mirror | `docs/research/datasheets/wch/ch347ds1_bitsavers_mirror.pdf` |
| CMSIS-DAP SWD probe use | <https://github.com/ARM-software/CMSIS-DAP> and retained upstream documentation sources | `docs/research/debug/cmsis_dap/` |
| TI TXS0108E bidirectional level-shifter use | <https://www.ti.com/lit/ds/symlink/txs0108e.pdf> | `docs/research/datasheets/ti/txs0108e.pdf` |
| TI TCAN3413 CAN FD transceiver use | <https://www.ti.com/lit/ds/symlink/tcan3413.pdf> | `docs/research/smart_robot/sources/tcan3413_datasheet.pdf` |
| TI THVD1450 RS-485 transceiver use | <https://www.ti.com/lit/ds/symlink/thvd1450.pdf> | `docs/research/smart_robot/sources/thvd1450_datasheet.pdf` |
| TI DRV8323 gate-driver use | <https://www.ti.com/lit/ds/symlink/drv8323.pdf> | `docs/research/smart_robot/sources/drv8323_datasheet.pdf` |
| NXP PCA9685 PWM-driver use | <https://www.nxp.com/docs/en/data-sheet/PCA9685.pdf> and NXP product page | `docs/research/smart_robot/sources/pca9685_datasheet.pdf` and `docs/research/smart_robot/sources/pca9685_product.html` |
| TDK InvenSense ICM-42688-P IMU use | <https://product.tdk.com/system/files/dam/doc/product/sensor/mortion-inertial/imu/data_sheet/ds-000347-icm-42688-p-v1.6.pdf> | `docs/research/smart_robot/sources/icm42688p_datasheet.pdf` |
| Sensirion SHT31-DIS humidity/temperature sensor use | <https://sensirion.com/media/documents/213E6A3B/63A5A569/Datasheet_SHT3x_DIS.pdf> | `docs/research/datasheets/sensirion/sht3x_dis_datasheet.pdf` |
| JST XH/VH connector contact use | JST connector datasheets plus retained handling precautions | `docs/research/smart_robot/sources/jst_xh_connector_datasheet.pdf`, `docs/research/smart_robot/sources/jst_vh_connector_datasheet.pdf`, and `docs/research/smart_robot/sources/jst_handling_precautions_terminals_connectors.pdf` |
| JLCPCB circular drill diameter process use | <https://jlcpcb.com/capabilities/pcb-capabilities> | `docs/research/fabrication/jlcpcb/pcb_capabilities.html` and retained Nuxt page assets |
| ST STM8S003F3P6 MCU power use | <https://www.st.com/resource/en/datasheet/stm8s003f3.pdf> | `docs/research/datasheets/st/stm8s003f3_datasheet.pdf` |
| ST STM32L431 boot and UART/SWD use | <https://www.st.com/resource/en/datasheet/stm32l431rb.pdf> | `docs/research/datasheets/st/stm32l431xx_datasheet.pdf` |
| Sipeed LicheeRV-Nano-W module use | <https://wiki.sipeed.com/hardware/en/lichee/RV_Nano/1_intro.html> and Sipeed schematic PDF | `docs/research/smart_robot/sources/licheerv_nano_intro.html` and `docs/research/smart_robot/sources/licheerv_nano_70405_schematic.pdf` |
| Artery AT32F435 motion-core MCU use | <https://www.arterytek.com/cn/product/AT32F435.jsp> | `docs/research/smart_robot/sources/at32f435_product.html` |
| Artery AT32M416 motor-control MCU use | <https://www.arterytek.com/cn/product/AT32M416.jsp> | `docs/research/smart_robot/sources/at32m416_product.html` |
| STC15W408AS 1T 8051-family MCU power use | <https://www.stcmicro.com/datasheet/STC15W408AS_Features.pdf> and <https://www.stcmicro.com/datasheet/STC15F2K60S2-en.pdf> | `docs/research/datasheets/stc/stc15w408as_features.pdf` and `docs/research/datasheets/stc/stc15f2k60s2_en.pdf` |
| TI NE555 astable timer power use | <https://www.ti.com/lit/ds/symlink/ne555.pdf> | `docs/research/datasheets/ti/ne555.pdf` |
| TI TLV803EA29 reset-supervisor use | <https://www.ti.com/lit/ds/symlink/tlv803e.pdf> | `docs/research/datasheets/ti/tlv803e-tlv809e-tlv810e.pdf` |
| Microchip MCP1316T-29LE/OT reset-supervisor use | <https://ww1.microchip.com/downloads/aemDocuments/documents/APID/ProductDocuments/DataSheets/MCP131X-2X-Voltage-Supervisor-DS20001985.pdf> | `docs/research/datasheets/microchip/mcp131x_2x_voltage_supervisor.pdf` |
| Abracon ABM3 8 MHz crystal support network | <https://abracon.com/Resonators/ABM3.pdf> | `docs/research/datasheets/abracon/abm3.pdf` |
| Winbond W25Q64JV SPI/QSPI NOR flash power and observation use | <https://www.winbond.com/resource-files/W25Q64JV_DTR%20RevL%2004272026%20Plus.pdf> | `docs/research/datasheets/winbond/w25q64jv_dtr_rev_l_2026.pdf` |
| Microchip AT24C02C I2C EEPROM power and observation use | <https://ww1.microchip.com/downloads/en/DeviceDoc/AT24C01C-AT24C02C-I2C-Compatible-Two-Wire-Serial-EEPROM-1Kbit-2Kbit-20006111A.pdf> | `docs/research/datasheets/microchip/at24c01c_at24c02c_datasheet_ds20006111a.pdf` |
| Microchip MCP23017 I2C GPIO expander power and observation use | <https://ww1.microchip.com/downloads/aemDocuments/documents/APID/ProductDocuments/DataSheets/MCP23017-Data-Sheet-DS20001952.pdf> | `docs/research/datasheets/microchip/mcp23017_mcp23s17_datasheet_ds20001952d.pdf` |
| Bosch BME280 environmental sensor I2C power and observation use | <https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf> | `docs/research/datasheets/bosch/bme280_datasheet.pdf` |
| TI CSD17484F4 N-channel MOSFET switching/discharge use | <https://www.ti.com/lit/gpn/CSD17484F4> | `docs/research/datasheets/ti/csd17484f4.pdf` |
| Kingbright APT1608SURCK red 0603 LED indicator use | <https://www.kingbrightusa.com/images/catalog/SPEC/APT1608SURCK.pdf> | `docs/research/datasheets/kingbright/apt1608surck.pdf` |
| onsemi 1N4148WS switching diode use | <https://www.onsemi.com/download/data-sheet/pdf/1n4148ws-d.pdf> | `docs/research/datasheets/onsemi/1n4148ws.pdf` |
| onsemi NDS7002A low-side MOSFET switch use | <https://www.onsemi.com/download/data-sheet/pdf/nds7002a-d.pdf> | `docs/research/datasheets/onsemi/nds7002a.pdf` |
| onsemi NL27WZ17 dual non-inverting Schmitt-trigger buffer use | <https://www.onsemi.com/download/data-sheet/pdf/nl27wz17-d.pdf> | `docs/research/datasheets/onsemi/nl27wz17.pdf` |
| JLC/EasyEDA BOM/CPL and KiCad footprint-property consistency | committed Board IR fixtures with `source.format: jlc_assembly` plus imported KiCad footprint properties | `examples/good_assembly_footprint_alignment/project.yaml` and `examples/bad_assembly_footprint_alignment/project.yaml` |
| KiCad footprint pin-1 orientation evidence | committed Board IR fixtures with imported `semantics.body_bounds` and `semantics.pin_1` plus explicit reviewed expected direction | `examples/good_pin_1_orientation/project.yaml` and `examples/bad_pin_1_orientation/project.yaml` |
| Reviewed manufacturing metadata limits | committed Board IR fixtures with explicit `board.manufacturing` drill/slot edge-clearance, solder-paste opening ratio, and solder-paste spacing metadata | `examples/good_drill_to_board_edge_clearance/project.yaml`, `examples/bad_drill_to_board_edge_clearance_board_metadata/project.yaml`, `examples/good_slot_to_board_edge_clearance/project.yaml`, `examples/bad_slot_to_board_edge_clearance_board_metadata/project.yaml`, `examples/good_solder_paste_opening/project.yaml`, `examples/bad_solder_paste_opening_undersized_board_metadata/project.yaml`, `examples/good_solder_paste_spacing/project.yaml`, and `examples/bad_solder_paste_spacing_board_metadata/project.yaml` |
| Manufacturing geometry ownership and segment/region variants | committed Board IR fixtures with drill/copper ownership, required copper-layer, solder-mask segment/region, and solder-paste segment/region/window metadata | `examples/good_drill_annular_ring_required_layers/project.yaml`, `examples/bad_drill_annular_ring_missing_copper/project.yaml`, `examples/bad_drill_annular_ring_missing_required_layer/project.yaml`, `examples/bad_drill_annular_ring_owner_mismatch/project.yaml`, `examples/bad_drill_annular_ring_same_net_owner_mismatch/project.yaml`, `examples/good_solder_mask_opening_segment/project.yaml`, `examples/bad_solder_mask_opening_region_undersized/project.yaml`, `examples/bad_solder_mask_dam_segment_region/project.yaml`, `examples/good_solder_paste_opening_segment/project.yaml`, `examples/good_solder_paste_opening_windowed/project.yaml`, `examples/bad_solder_paste_opening_region_oversized/project.yaml`, `examples/bad_solder_paste_opening_windowed_oversized/project.yaml`, and `examples/bad_solder_paste_spacing_segment_region/project.yaml` |
| Manufacturing copper geometry and IC/BGA paste aperture variants | committed Board IR fixtures with copper feature/segment/region edge and spacing metadata plus JLCPCB IC/BGA paste-aperture metadata | `examples/good_copper_same_net_touching/project.yaml`, `examples/bad_copper_feature_to_board_edge_clearance/project.yaml`, `examples/bad_copper_segment_to_board_edge_clearance/project.yaml`, `examples/bad_copper_region_to_board_edge_clearance/project.yaml`, `examples/bad_copper_feature_spacing/project.yaml`, `examples/bad_copper_segment_spacing/project.yaml`, `examples/bad_copper_region_spacing/project.yaml`, `examples/bad_copper_feature_segment_spacing/project.yaml`, `examples/bad_copper_different_net_overlap/project.yaml`, `examples/bad_copper_different_island_overlap/project.yaml`, `examples/good_solder_paste_ic_pin_aperture_jlc/project.yaml`, `examples/bad_solder_paste_ic_pin_aperture_jlc/project.yaml`, `examples/good_solder_paste_ic_pin_aperture_length_jlc/project.yaml`, `examples/bad_solder_paste_ic_pin_aperture_length_jlc/project.yaml`, `examples/good_solder_paste_ic_pin_aperture_extension_jlc/project.yaml`, `examples/bad_solder_paste_ic_pin_aperture_extension_jlc/project.yaml`, `examples/good_solder_paste_bga_aperture_jlc/project.yaml`, `examples/bad_solder_paste_bga_aperture_jlc/project.yaml`, and `examples/bad_solder_paste_bga_pitch_grid_jlc/project.yaml` |
| Manufacturing edge/mask/paste failure remainders | committed Board IR fixtures with cutout-edge drill clearance, missing or undersized mask/paste openings, oversized paste openings, and unsupported fabrication-process metadata | `examples/bad_drill_to_cutout_edge_clearance/project.yaml`, `examples/bad_solder_mask_opening_missing/project.yaml`, `examples/bad_solder_mask_opening_undersized/project.yaml`, `examples/bad_unknown_fabrication_process/project.yaml`, `examples/bad_solder_paste_opening_missing/project.yaml`, and `examples/bad_solder_paste_opening_oversized/project.yaml` |
| USB connector mechanical placement and footprint evidence | committed Board IR fixtures with USB connector placement, board-edge, body/courtyard, nearby-component, cable-entry, footprint polygon, circle, aperture, and model-offset metadata | `examples/good_usb_connector_orientation/project.yaml`, `examples/bad_usb_connector_orientation/project.yaml`, `examples/good_usb_connector_edge_proximity/project.yaml`, `examples/bad_usb_connector_edge_proximity/project.yaml`, `examples/good_usb_connector_edge_proximity_footprint/project.yaml`, `examples/bad_usb_connector_edge_proximity_footprint/project.yaml`, `examples/good_usb_connector_body_overhang/project.yaml`, `examples/bad_usb_connector_body_overhang/project.yaml`, `examples/bad_usb_connector_body_overhang_circle/project.yaml`, `examples/good_usb_connector_component_clearance/project.yaml`, `examples/bad_usb_connector_component_clearance/project.yaml`, `examples/good_usb_connector_entry_clearance/project.yaml`, `examples/bad_usb_connector_entry_clearance/project.yaml`, `examples/bad_usb_connector_entry_clearance_aperture/project.yaml`, and `examples/bad_usb_connector_entry_clearance_model_offset/project.yaml` |
| USB connector protection and protection-placement evidence | committed Board IR fixtures with USB connector pin metadata, connected clamp model metadata, shield-grounding metadata, and connector-to-protection placement metadata | `examples/good_usb_connector_protection/project.yaml`, `examples/bad_usb_connector_missing_data_protection/project.yaml`, `examples/bad_usb_connector_missing_vbus_protection/project.yaml`, `examples/good_usb_connector_shield_ground/project.yaml`, `examples/bad_usb_connector_shield_not_ground/project.yaml`, `examples/good_usb_connector_protection_placement/project.yaml`, and `examples/bad_usb_connector_protection_placement_distance/project.yaml` |
| USB route, VBUS route, and return-path layout evidence | committed Board IR fixtures with USB connector pin metadata, imported route/via/zone geometry, VBUS protection routing, filled-zone contact, and stitching-via metadata | `examples/good_usb_connector_route_geometry/project.yaml`, `examples/bad_usb_connector_route_geometry/project.yaml`, `examples/good_usb_vbus_route_geometry/project.yaml`, `examples/bad_usb_vbus_route_geometry/project.yaml`, `examples/good_usb_return_path/project.yaml`, `examples/good_usb_return_path_pad_contact/project.yaml`, `examples/bad_usb_return_path/project.yaml`, `examples/bad_usb_return_path_filled_zone_gap/project.yaml`, `examples/bad_usb_return_path_floating_zone/project.yaml`, `examples/bad_usb_return_path_split_filled_zone_contact/project.yaml`, `examples/bad_usb_return_path_filled_zone_edge_clearance/project.yaml`, and `examples/bad_usb_return_path_stitching_via/project.yaml` |
| Generic power, reset, and boot validation fixtures | committed Board IR fixtures with generic MCU, regulator, load-switch, power-mux, reset-supervisor, strap, UART bootloader, and UM STM32L4 boot metadata | `examples/good_power_tree_board/project.yaml`, `examples/bad_power_tree_overvoltage/project.yaml`, `examples/bad_power_tree_current_budget/project.yaml`, `examples/good_load_switch_power_tree/project.yaml`, `examples/bad_load_switch_missing_enable/project.yaml`, `examples/bad_load_switch_disabled_output_powered/project.yaml`, `examples/bad_load_switch_output_current/project.yaml`, `examples/good_power_mux_usb_selected/project.yaml`, `examples/good_power_mux_derived_selection/project.yaml`, `examples/bad_power_mux_missing_selection/project.yaml`, `examples/bad_power_mux_selected_unpowered/project.yaml`, `examples/bad_power_mux_backfeed/project.yaml`, `examples/good_regulator_power_tree/project.yaml`, `examples/bad_regulator_dropout/project.yaml`, `examples/bad_regulator_output_current/project.yaml`, `examples/bad_regulator_conversion_pin/project.yaml`, `examples/bad_regulator_startup_sequence/project.yaml`, `examples/bad_regulator_startup_missing_timing/project.yaml`, `examples/good_reset_supervisor_threshold/project.yaml`, `examples/bad_reset_supervisor_threshold_too_low/project.yaml`, `examples/good_bootstrap_bias_divider/project.yaml`, `examples/bad_bootstrap_bias_threshold/project.yaml`, `examples/bad_bootstrap_bias_current/project.yaml`, `examples/good_bootloader_board/project.yaml`, `examples/good_c51_isp_board/project.yaml`, `examples/bad_bootstrap_board/project.yaml`, `examples/bad_uart_bootloader_sync_board/project.yaml`, `examples/bad_reset_release_board/project.yaml`, `examples/bad_reset_power_timing_mismatch/project.yaml`, `examples/bad_reset_supervisor_delay/project.yaml`, `examples/bad_reset_pin_board/project.yaml`, `examples/um_stm32l4_rom_download_entry/project.yaml`, `examples/um_stm32l4_rom_download_wrong_uart/project.yaml`, `examples/um_stm32l4_app_boot_fixed_release/project.yaml`, and `examples/um_stm32l4_app_boot_bad_release/project.yaml` |
| Espressif ESP32-S3-WROOM-1U-N16R8 application boot module use | <https://documentation.espressif.com/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf> and peer `../urine_monitor` LCSC cache | `docs/research/datasheets/espressif/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf` |

The earlier source URLs through the ESP32-WROOM-32E row and the ESP32-S3 row
were re-checked with web search on 2026-06-13; the RP2040, nRF52840,
STM8S003F3P6, STC15W408AS, NE555, and MCP1316 URLs were checked on 2026-07-05.
The Abracon ABM3, Winbond W25Q64JV, Bosch BME280, Nexperia PRTR5V0U2X,
Nexperia PESD5V0S1UL, TI ESD2CAN24-Q1, TI ESDS552, TI BQ25798, TI TPS22918,
TI TPS25948, TI TPS24751, TI TPS54331, TI DRV8323, TI TLV803E, TI CSD17484F4,
TI TXS0108E, TI TCAN3413, TI THVD1450, ST STM32L431, Silicon Labs CP2102N,
AMS1117, NXP PCA9685, TDK ICM-42688-P, Sipeed LicheeRV-Nano-W, Artery
AT32F435, Artery AT32M416, JST XH/VH, JLCPCB PCB capabilities, onsemi 1N4148WS,
onsemi NDS7002A, onsemi NL27WZ17, Kingbright APT1608SURCK, and Microchip
MCP131X/2X source artifacts were downloaded or retained from their official
vendor URLs.
CMSIS-DAP retains Arm's upstream repository documentation sources. WCH CH340C/N,
WCH CH347, and FTDI FT232R retain official vendor metadata or URL evidence plus
public PDF mirrors because direct automated binary retrieval was blocked by the
vendor endpoints. The local PDF copies and SHA-256 hashes are listed in the
part-specific research notes under `docs/research/datasheets/` and
`docs/research/smart_robot/source_review.md`.

## Executed Suite

`suites/public_typical_circuits.yaml` combines one hundred thirty-six
public-reference passing cases and one hundred sixty-nine paired injected-error
cases:

| Case | Fixture | Expected result | Purpose |
| --- | --- | --- | --- |
| `jlcpcb_drill_diameter_process_passes` | `examples/good_drill_diameter_jlc_process/project.yaml` | pass | JLCPCB circular drill examples at the source-backed 0.15 mm minimum, 6.3 mm maximum, and a nominal interior size. |
| `jlcpcb_drill_diameter_process_detected` | `examples/bad_drill_diameter_jlc_process/project.yaml` | fail | Detects circular drill diameters below the JLCPCB 0.15 mm minimum and above the 6.3 mm maximum. |
| `manufacturing_drill_to_board_edge_passes` | `examples/good_drill_to_board_edge_clearance/project.yaml` | pass | Drill-to-board-edge clearance satisfies explicit reviewed board metadata. |
| `manufacturing_drill_to_board_edge_metadata_detected` | `examples/bad_drill_to_board_edge_clearance_board_metadata/project.yaml` | fail | Detects drill-to-board-edge clearance below explicit reviewed board metadata. |
| `manufacturing_drill_to_cutout_edge_detected` | `examples/bad_drill_to_cutout_edge_clearance/project.yaml` | fail | Detects drill clearance below the reviewed limit to a board cutout edge. |
| `jlcpcb_slot_width_process_passes` | `examples/good_slot_width_jlc_process/project.yaml` | pass | JLCPCB slot-width examples at the source-backed plated and non-plated slot limits. |
| `jlcpcb_slot_width_process_detected` | `examples/bad_slot_width_jlc_process/project.yaml` | fail | Detects plated, non-plated, and unknown-plating slots below the JLCPCB process limits. |
| `manufacturing_slot_to_board_edge_passes` | `examples/good_slot_to_board_edge_clearance/project.yaml` | pass | Slot-to-board-edge clearance satisfies explicit reviewed board metadata. |
| `manufacturing_slot_to_board_edge_metadata_detected` | `examples/bad_slot_to_board_edge_clearance_board_metadata/project.yaml` | fail | Detects slot-to-board-edge clearance below explicit reviewed board metadata. |
| `jlcpcb_slot_aspect_ratio_process_passes` | `examples/good_slot_aspect_ratio_jlc_process/project.yaml` | pass | JLCPCB slot aspect-ratio example above the source-backed minimum ratio. |
| `jlcpcb_slot_aspect_ratio_process_detected` | `examples/bad_slot_aspect_ratio_jlc_process/project.yaml` | fail | Detects a JLCPCB slot whose length-to-width ratio is below the source-backed minimum. |
| `jlcpcb_castellated_hole_process_passes` | `examples/good_castellated_hole_jlc_process/project.yaml` | pass | JLCPCB castellated-hole examples satisfying diameter, edge-clearance, and hole-spacing process limits. |
| `jlcpcb_castellated_hole_process_detected` | `examples/bad_castellated_hole_jlc_process/project.yaml` | fail | Detects JLCPCB castellated-hole diameter, edge-clearance, and spacing violations. |
| `manufacturing_drill_annular_ring_passes` | `examples/good_drill_annular_ring/project.yaml` | pass | Drill annular-ring example with copper flash large enough for the declared drill and offset limits. |
| `manufacturing_drill_annular_required_layers_passes` | `examples/good_drill_annular_ring_required_layers/project.yaml` | pass | Drill annular-ring example with required copper flash evidence on every declared layer. |
| `jlcpcb_drill_annular_ring_process_detected` | `examples/bad_drill_annular_ring_jlc_via_process/project.yaml` | fail | Detects a JLCPCB via annular ring below the process-derived minimum. |
| `manufacturing_drill_annular_missing_copper_detected` | `examples/bad_drill_annular_ring_missing_copper/project.yaml` | fail | Detects a plated drill without matching copper flash evidence. |
| `manufacturing_drill_annular_missing_required_layer_detected` | `examples/bad_drill_annular_ring_missing_required_layer/project.yaml` | fail | Detects missing copper flash evidence on a required annular-ring layer. |
| `manufacturing_drill_annular_owner_mismatch_detected` | `examples/bad_drill_annular_ring_owner_mismatch/project.yaml` | fail | Detects drill/copper ownership mismatch for annular-ring evidence. |
| `manufacturing_drill_annular_same_net_owner_mismatch_detected` | `examples/bad_drill_annular_ring_same_net_owner_mismatch/project.yaml` | fail | Detects same-net pad ownership mismatch for annular-ring evidence. |
| `manufacturing_copper_to_board_edge_passes` | `examples/good_copper_to_board_edge_clearance/project.yaml` | pass | Copper-to-board-edge fixture with flash and trace features beyond the declared clearance limit. |
| `manufacturing_copper_same_net_touching_passes` | `examples/good_copper_same_net_touching/project.yaml` | pass | Same-net copper touching is allowed while still exercising spacing evidence. |
| `jlcpcb_copper_to_board_edge_process_detected` | `examples/bad_copper_to_board_edge_jlc_routed_process/project.yaml` | fail | Detects copper too close to a routed JLCPCB board edge. |
| `manufacturing_copper_feature_to_board_edge_detected` | `examples/bad_copper_feature_to_board_edge_clearance/project.yaml` | fail | Detects a copper flash too close to board-edge evidence. |
| `manufacturing_copper_segment_to_board_edge_detected` | `examples/bad_copper_segment_to_board_edge_clearance/project.yaml` | fail | Detects a copper draw segment too close to board-edge evidence. |
| `manufacturing_copper_region_to_board_edge_detected` | `examples/bad_copper_region_to_board_edge_clearance/project.yaml` | fail | Detects a copper region too close to board-edge evidence. |
| `manufacturing_copper_spacing_passes` | `examples/good_copper_spacing/project.yaml` | pass | Copper-spacing fixture with same-net touching allowed and different-layer copper separated from the checked same-layer feature. |
| `jlcpcb_copper_spacing_process_detected` | `examples/bad_copper_spacing_jlc_1oz_process/project.yaml` | fail | Detects same-layer different-net copper spacing below the JLCPCB 1 oz copper process limit. |
| `manufacturing_copper_feature_spacing_detected` | `examples/bad_copper_feature_spacing/project.yaml` | fail | Detects spacing below the reviewed limit between copper flashes. |
| `manufacturing_copper_segment_spacing_detected` | `examples/bad_copper_segment_spacing/project.yaml` | fail | Detects spacing below the reviewed limit between copper draw segments. |
| `manufacturing_copper_region_spacing_detected` | `examples/bad_copper_region_spacing/project.yaml` | fail | Detects spacing below the reviewed limit between a copper flash and copper region. |
| `manufacturing_copper_feature_segment_spacing_detected` | `examples/bad_copper_feature_segment_spacing/project.yaml` | fail | Detects spacing below the reviewed limit between a copper flash and copper draw segment. |
| `manufacturing_copper_different_net_overlap_detected` | `examples/bad_copper_different_net_overlap/project.yaml` | fail | Detects overlapping same-layer copper on different nets. |
| `manufacturing_copper_different_island_overlap_detected` | `examples/bad_copper_different_island_overlap/project.yaml` | fail | Detects overlapping same-layer copper on different copper islands. |
| `jlcpcb_solder_mask_opening_process_passes` | `examples/good_solder_mask_opening_explicit_override/project.yaml` | pass | JLCPCB solder-mask opening example with explicit override limits and adequate mask expansion. |
| `manufacturing_solder_mask_opening_segment_passes` | `examples/good_solder_mask_opening_segment/project.yaml` | pass | Solder-mask segment opening expands matching copper evidence enough for the declared limit. |
| `jlcpcb_solder_mask_opening_process_detected` | `examples/bad_solder_mask_opening_jlc_process/project.yaml` | fail | Detects a JLCPCB solder-mask opening below the process-derived expansion/offset requirements. |
| `manufacturing_solder_mask_opening_region_detected` | `examples/bad_solder_mask_opening_region_undersized/project.yaml` | fail | Detects an undersized solder-mask region opening around copper evidence. |
| `manufacturing_solder_mask_opening_missing_detected` | `examples/bad_solder_mask_opening_missing/project.yaml` | fail | Detects missing solder-mask opening evidence for copper that requires an opening. |
| `manufacturing_solder_mask_opening_undersized_detected` | `examples/bad_solder_mask_opening_undersized/project.yaml` | fail | Detects an undersized solder-mask feature opening around copper evidence. |
| `manufacturing_unknown_fabrication_process_detected` | `examples/bad_unknown_fabrication_process/project.yaml` | fail | Detects unsupported fabrication process preset metadata before applying process-derived defaults. |
| `manufacturing_solder_mask_dam_passes` | `examples/good_solder_mask_dam/project.yaml` | pass | Solder-mask dam fixture whose adjacent openings leave enough web for the declared dam limit. |
| `jlcpcb_solder_mask_dam_process_detected` | `examples/bad_solder_mask_dam_jlc_process/project.yaml` | fail | Detects a JLCPCB solder-mask dam narrower than the source-backed process floor. |
| `manufacturing_solder_mask_dam_segment_region_detected` | `examples/bad_solder_mask_dam_segment_region/project.yaml` | fail | Detects a solder-mask dam violation between non-flash segment and region openings. |
| `jlcpcb_solder_paste_aperture_size_process_passes` | `examples/good_solder_paste_aperture_size_jlc_process/project.yaml` | pass | JLCPCB stencil aperture-size fixture above the source-backed minimum aperture dimension. |
| `jlcpcb_solder_paste_aperture_size_process_detected` | `examples/bad_solder_paste_aperture_size_jlc_process/project.yaml` | fail | Detects JLCPCB stencil feature and segment apertures below the process minimum size. |
| `jlcpcb_solder_paste_area_ratio_process_passes` | `examples/good_solder_paste_aperture_area_ratio_jlc/project.yaml` | pass | JLCPCB stencil aperture area-ratio fixture satisfying the source-backed minimum area ratio. |
| `jlcpcb_solder_paste_area_ratio_process_detected` | `examples/bad_solder_paste_aperture_area_ratio_jlc/project.yaml` | fail | Detects a JLCPCB stencil aperture area ratio below the source-backed process minimum. |
| `manufacturing_solder_paste_opening_passes` | `examples/good_solder_paste_opening/project.yaml` | pass | Solder-paste opening area ratio satisfies explicit reviewed board metadata. |
| `manufacturing_solder_paste_opening_segment_passes` | `examples/good_solder_paste_opening_segment/project.yaml` | pass | Solder-paste segment opening area ratio satisfies explicit reviewed board metadata. |
| `manufacturing_solder_paste_opening_windowed_passes` | `examples/good_solder_paste_opening_windowed/project.yaml` | pass | Multiple windowed solder-paste openings aggregate to the reviewed area-ratio limit. |
| `manufacturing_solder_paste_opening_metadata_detected` | `examples/bad_solder_paste_opening_undersized_board_metadata/project.yaml` | fail | Detects solder-paste opening area ratio below explicit reviewed board metadata. |
| `manufacturing_solder_paste_opening_region_detected` | `examples/bad_solder_paste_opening_region_oversized/project.yaml` | fail | Detects solder-paste region opening area ratio outside the reviewed range. |
| `manufacturing_solder_paste_opening_windowed_detected` | `examples/bad_solder_paste_opening_windowed_oversized/project.yaml` | fail | Detects aggregate windowed solder-paste opening area ratio outside the reviewed range. |
| `manufacturing_solder_paste_opening_missing_detected` | `examples/bad_solder_paste_opening_missing/project.yaml` | fail | Detects missing solder-paste opening evidence for a pad-owned copper flash. |
| `manufacturing_solder_paste_opening_oversized_detected` | `examples/bad_solder_paste_opening_oversized/project.yaml` | fail | Detects solder-paste opening area ratio above the reviewed maximum. |
| `manufacturing_solder_paste_spacing_passes` | `examples/good_solder_paste_spacing/project.yaml` | pass | Solder-paste opening spacing satisfies explicit reviewed board metadata. |
| `manufacturing_solder_paste_spacing_metadata_detected` | `examples/bad_solder_paste_spacing_board_metadata/project.yaml` | fail | Detects solder-paste opening spacing below explicit reviewed board metadata. |
| `manufacturing_solder_paste_spacing_segment_region_detected` | `examples/bad_solder_paste_spacing_segment_region/project.yaml` | fail | Detects solder-paste spacing violation between non-flash segment and region openings. |
| `jlcpcb_solder_paste_ic_pin_aperture_passes` | `examples/good_solder_paste_ic_pin_aperture_jlc/project.yaml` | pass | JLCPCB IC pin paste aperture width satisfies pitch-conditioned source guidance. |
| `jlcpcb_solder_paste_ic_pin_aperture_detected` | `examples/bad_solder_paste_ic_pin_aperture_jlc/project.yaml` | fail | Detects IC pin paste aperture width outside pitch-conditioned source guidance. |
| `jlcpcb_solder_paste_ic_pin_aperture_length_passes` | `examples/good_solder_paste_ic_pin_aperture_length_jlc/project.yaml` | pass | JLCPCB IC pin paste aperture length satisfies pitch-conditioned source guidance. |
| `jlcpcb_solder_paste_ic_pin_aperture_length_detected` | `examples/bad_solder_paste_ic_pin_aperture_length_jlc/project.yaml` | fail | Detects IC pin paste aperture length below pitch-conditioned source guidance. |
| `jlcpcb_solder_paste_ic_pin_aperture_extension_passes` | `examples/good_solder_paste_ic_pin_aperture_extension_jlc/project.yaml` | pass | JLCPCB IC pin paste aperture extension satisfies owner-matched copper-pad guidance. |
| `jlcpcb_solder_paste_ic_pin_aperture_extension_detected` | `examples/bad_solder_paste_ic_pin_aperture_extension_jlc/project.yaml` | fail | Detects IC pin paste aperture extension below owner-matched copper-pad guidance. |
| `jlcpcb_solder_paste_bga_aperture_passes` | `examples/good_solder_paste_bga_aperture_jlc/project.yaml` | pass | JLCPCB BGA paste aperture size and pitch-grid evidence satisfy source guidance. |
| `jlcpcb_solder_paste_bga_aperture_detected` | `examples/bad_solder_paste_bga_aperture_jlc/project.yaml` | fail | Detects BGA paste aperture size outside source guidance. |
| `jlcpcb_solder_paste_bga_pitch_grid_detected` | `examples/bad_solder_paste_bga_pitch_grid_jlc/project.yaml` | fail | Detects BGA paste evidence that does not prove the expected pitch grid. |
| `assembly_footprint_alignment_passes` | `examples/good_assembly_footprint_alignment/project.yaml` | pass | JLC/EasyEDA BOM/CPL footprint, part-number, side, and rotation evidence matches imported KiCad footprint properties and placement evidence. |
| `assembly_footprint_alignment_detected` | `examples/bad_assembly_footprint_alignment/project.yaml` | fail | Detects direct contradictions between JLC/EasyEDA BOM/CPL evidence and imported KiCad footprint properties, placement side, and rotation. |
| `pin_1_orientation_passes` | `examples/good_pin_1_orientation/project.yaml` | pass | Imported KiCad footprint body bounds and pad-1 marker agree with explicit reviewed expected pin-1 direction evidence. |
| `pin_1_orientation_detected` | `examples/bad_pin_1_orientation/project.yaml` | fail | Detects a pin-1 marker direction that contradicts explicit reviewed expected pin-1 direction evidence. |
| `usb_connector_orientation_passes` | `examples/good_usb_connector_orientation/project.yaml` | pass | USB connector placement rotation is within the explicit board-edge/enclosure orientation tolerance. |
| `usb_connector_orientation_detected` | `examples/bad_usb_connector_orientation/project.yaml` | fail | Detects USB connector placement rotation outside the explicit orientation tolerance. |
| `usb_connector_edge_proximity_passes` | `examples/good_usb_connector_edge_proximity/project.yaml` | pass | USB connector placement is close enough to imported board-edge evidence. |
| `usb_connector_edge_proximity_detected` | `examples/bad_usb_connector_edge_proximity/project.yaml` | fail | Detects a USB connector too far from imported board-edge evidence. |
| `usb_connector_edge_proximity_footprint_passes` | `examples/good_usb_connector_edge_proximity_footprint/project.yaml` | pass | USB connector footprint polygon evidence is close enough to imported board-edge evidence. |
| `usb_connector_edge_proximity_footprint_detected` | `examples/bad_usb_connector_edge_proximity_footprint/project.yaml` | fail | Detects a USB connector footprint polygon too far from imported board-edge evidence. |
| `usb_connector_body_overhang_passes` | `examples/good_usb_connector_body_overhang/project.yaml` | pass | USB connector body/courtyard evidence stays within the declared allowed board-edge overhang. |
| `usb_connector_body_overhang_detected` | `examples/bad_usb_connector_body_overhang/project.yaml` | fail | Detects USB connector body/courtyard evidence extending too far beyond the board edge. |
| `usb_connector_body_overhang_circle_detected` | `examples/bad_usb_connector_body_overhang_circle/project.yaml` | fail | Detects USB connector circle body evidence extending beyond the declared allowed board-edge overhang. |
| `usb_connector_component_clearance_passes` | `examples/good_usb_connector_component_clearance/project.yaml` | pass | USB connector courtyard/body evidence has enough clearance to nearby component footprint evidence. |
| `usb_connector_component_clearance_detected` | `examples/bad_usb_connector_component_clearance/project.yaml` | fail | Detects nearby component footprint evidence inside the declared USB connector clearance. |
| `usb_connector_entry_clearance_passes` | `examples/good_usb_connector_entry_clearance/project.yaml` | pass | USB connector cable-entry corridor evidence is clear for the declared depth and width. |
| `usb_connector_entry_clearance_detected` | `examples/bad_usb_connector_entry_clearance/project.yaml` | fail | Detects footprint evidence obstructing the declared USB connector cable-entry corridor. |
| `usb_connector_entry_clearance_aperture_detected` | `examples/bad_usb_connector_entry_clearance_aperture/project.yaml` | fail | Detects model-aperture geometry that narrows the effective USB connector cable-entry corridor. |
| `usb_connector_entry_clearance_model_offset_detected` | `examples/bad_usb_connector_entry_clearance_model_offset/project.yaml` | fail | Detects a model-provided cable-entry direction offset with obstructing footprint evidence. |
| `usb_connector_protection_passes` | `examples/good_usb_connector_protection/project.yaml` | pass | USB D+/D-/VBUS connector pins have connected clamp metadata satisfying connector-level protection requirements. |
| `usb_connector_missing_data_protection_detected` | `examples/bad_usb_connector_missing_data_protection/project.yaml` | fail | Detects missing required D+/D- connector clamp coverage. |
| `usb_connector_missing_vbus_protection_detected` | `examples/bad_usb_connector_missing_vbus_protection/project.yaml` | fail | Detects missing required VBUS connector clamp coverage. |
| `usb_connector_shield_ground_passes` | `examples/good_usb_connector_shield_ground/project.yaml` | pass | USB connector shield metadata is tied to a ground net when shield grounding is required. |
| `usb_connector_shield_ground_detected` | `examples/bad_usb_connector_shield_not_ground/project.yaml` | fail | Detects USB connector shield metadata tied to a non-ground net when grounding is required. |
| `usb_protection_placement_passes` | `examples/good_usb_connector_protection_placement/project.yaml` | pass | USB connector data-line clamp placement evidence satisfies the connector-to-protection distance rule. |
| `usb_protection_placement_detected` | `examples/bad_usb_connector_protection_placement_distance/project.yaml` | fail | Detects USB connector data-line clamp placement beyond the reviewed distance limit. |
| `usb_route_geometry_passes` | `examples/good_usb_connector_route_geometry/project.yaml` | pass | USB D+/D- route length, via-count, width, pair mismatch, pair via symmetry, gap, and protection-order evidence satisfy reviewed limits. |
| `usb_route_geometry_detected` | `examples/bad_usb_connector_route_geometry/project.yaml` | fail | Detects USB data route length, protection-order, via-count, width, pair mismatch, via symmetry, and pair-gap violations. |
| `usb_vbus_route_geometry_passes` | `examples/good_usb_vbus_route_geometry/project.yaml` | pass | USB VBUS route length, via-count, width, and protection-order evidence satisfy reviewed limits. |
| `usb_vbus_route_geometry_detected` | `examples/bad_usb_vbus_route_geometry/project.yaml` | fail | Detects USB VBUS route length, via-count, width, and protection-order violations. |
| `usb_return_path_passes` | `examples/good_usb_return_path/project.yaml` | pass | USB data routes have source-backed same-layer ground-zone return-path coverage. |
| `usb_return_path_pad_contact_passes` | `examples/good_usb_return_path_pad_contact/project.yaml` | pass | Imported filled-zone return-path evidence includes required same-net pad or via contact. |
| `usb_return_path_unreferenced_route_detected` | `examples/bad_usb_return_path/project.yaml` | fail | Detects USB data route length without the required ground reference coverage. |
| `usb_return_path_filled_zone_gap_detected` | `examples/bad_usb_return_path_filled_zone_gap/project.yaml` | fail | Detects a same-layer filled-zone gap under the USB data route. |
| `usb_return_path_floating_zone_detected` | `examples/bad_usb_return_path_floating_zone/project.yaml` | fail | Detects filled-zone return-path evidence that lacks the required same-net pad or via contact. |
| `usb_return_path_split_zone_detected` | `examples/bad_usb_return_path_split_filled_zone_contact/project.yaml` | fail | Detects a filled-zone return path where the available contact is on a different zone island. |
| `usb_return_path_zone_edge_clearance_detected` | `examples/bad_usb_return_path_filled_zone_edge_clearance/project.yaml` | fail | Detects USB data route clearance to filled-zone edge below the reviewed limit. |
| `usb_return_path_stitching_via_detected` | `examples/bad_usb_return_path_stitching_via/project.yaml` | fail | Detects a USB data via too far from the nearest acceptable ground stitching via. |
| `generic_power_tree_passes` | `examples/good_power_tree_board/project.yaml` | pass | Generic MCU and USB-UART rail powered within operating limits and current budget. |
| `generic_power_tree_overvoltage_detected` | `examples/bad_power_tree_overvoltage/project.yaml` | fail | Detects a generic MCU powered above its model maximum voltage. |
| `generic_power_tree_current_budget_detected` | `examples/bad_power_tree_current_budget/project.yaml` | fail | Detects generic 3.3 V rail declared load above its supply-current budget. |
| `generic_load_switch_passes` | `examples/good_load_switch_power_tree/project.yaml` | pass | Generic load switch output rail is powered with explicit enabled-control evidence. |
| `generic_load_switch_missing_enable_detected` | `examples/bad_load_switch_missing_enable/project.yaml` | fail | Detects a powered switched output without enable-state evidence. |
| `generic_load_switch_disabled_output_detected` | `examples/bad_load_switch_disabled_output_powered/project.yaml` | fail | Detects a powered switched output while the enable pin is explicitly low. |
| `generic_load_switch_output_current_detected` | `examples/bad_load_switch_output_current/project.yaml` | fail | Detects switched output load above the generic load-switch current limit. |
| `generic_power_mux_usb_selected_passes` | `examples/good_power_mux_usb_selected/project.yaml` | pass | Generic power mux uses a selected powered USB input with reverse-blocked inactive battery input. |
| `generic_power_mux_derived_selection_passes` | `examples/good_power_mux_derived_selection/project.yaml` | pass | Generic power mux selected input is derived from the only powered source. |
| `generic_power_mux_missing_selection_detected` | `examples/bad_power_mux_missing_selection/project.yaml` | fail | Detects missing selected-input evidence when multiple mux inputs are powered. |
| `generic_power_mux_selected_unpowered_detected` | `examples/bad_power_mux_selected_unpowered/project.yaml` | fail | Detects selected mux input whose rail is not powered. |
| `generic_power_mux_backfeed_detected` | `examples/bad_power_mux_backfeed/project.yaml` | fail | Detects powered mux output backfeeding an inactive unpowered input without reverse blocking. |
| `generic_regulator_power_tree_passes` | `examples/good_regulator_power_tree/project.yaml` | pass | Generic regulator has adequate input, output, dropout, and load evidence. |
| `generic_regulator_dropout_detected` | `examples/bad_regulator_dropout/project.yaml` | fail | Detects regulator dropout margin below the model requirement. |
| `generic_regulator_output_current_detected` | `examples/bad_regulator_output_current/project.yaml` | fail | Detects generic regulator output load above its current limit. |
| `generic_regulator_conversion_pin_detected` | `examples/bad_regulator_conversion_pin/project.yaml` | fail | Detects invalid regulator power-conversion model metadata. |
| `generic_regulator_startup_sequence_detected` | `examples/bad_regulator_startup_sequence/project.yaml` | fail | Detects regulator output rail valid before input-valid plus startup delay. |
| `generic_regulator_startup_missing_timing_detected` | `examples/bad_regulator_startup_missing_timing/project.yaml` | fail | Detects missing output power-valid timing required by regulator startup-delay metadata. |
| `generic_reset_supervisor_threshold_passes` | `examples/good_reset_supervisor_threshold/project.yaml` | pass | Generic reset supervisor threshold releases after monitored rail is safely valid. |
| `generic_reset_supervisor_threshold_too_low_detected` | `examples/bad_reset_supervisor_threshold_too_low/project.yaml` | fail | Detects reset-supervisor threshold below the monitored load minimum operating voltage. |
| `generic_bootstrap_bias_divider_passes` | `examples/good_bootstrap_bias_divider/project.yaml` | pass | Generic boot-strap resistor divider satisfies required boot state and bias-current limit. |
| `generic_bootstrap_bias_threshold_detected` | `examples/bad_bootstrap_bias_threshold/project.yaml` | fail | Detects boot-strap voltage below the required high threshold. |
| `generic_bootstrap_bias_current_detected` | `examples/bad_bootstrap_bias_current/project.yaml` | fail | Detects boot-strap resistor-network current above the reviewed limit. |
| `generic_bootloader_board_passes` | `examples/good_bootloader_board/project.yaml` | pass | Generic MCU reset, boot-strap, and UART bootloader sync checks all pass. |
| `generic_c51_isp_board_passes` | `examples/good_c51_isp_board/project.yaml` | pass | C51-style ISP bootloader metadata passes reset, boot-strap, and UART sync checks. |
| `generic_bootstrap_defined_detected` | `examples/bad_bootstrap_board/project.yaml` | fail | Detects boot-strap state that does not satisfy the required bootloader mode. |
| `generic_uart_bootloader_sync_detected` | `examples/bad_uart_bootloader_sync_board/project.yaml` | fail | Detects missing UART bootloader sync-byte evidence. |
| `generic_reset_release_before_power_detected` | `examples/bad_reset_release_board/project.yaml` | fail | Detects reset release before power-valid timing. |
| `generic_reset_power_timing_mismatch_detected` | `examples/bad_reset_power_timing_mismatch/project.yaml` | fail | Detects reset scenario power-valid timing that contradicts the target rail timing. |
| `generic_reset_supervisor_delay_detected` | `examples/bad_reset_supervisor_delay/project.yaml` | fail | Detects reset-supervisor release delay that is too short for the declared power-valid timing. |
| `generic_reset_pin_mismatch_detected` | `examples/bad_reset_pin_board/project.yaml` | fail | Detects a reset scenario targeting a pin that is not the model reset pin. |
| `um_stm32l4_rom_download_entry_passes` | `examples/um_stm32l4_rom_download_entry/project.yaml` | pass | UM STM32L4 ROM download entry passes reset, BOOT0, and UART sync evidence checks. |
| `um_stm32l4_wrong_uart_detected` | `examples/um_stm32l4_rom_download_wrong_uart/project.yaml` | fail | Detects UART bootloader sync sent from a transmitter not connected to the target RX pin. |
| `um_stm32l4_app_boot_fixed_release_passes` | `examples/um_stm32l4_app_boot_fixed_release/project.yaml` | pass | UM STM32L4 application boot release fixture proves BOOT0 low at application boot. |
| `um_stm32l4_app_boot_bad_release_detected` | `examples/um_stm32l4_app_boot_bad_release/project.yaml` | fail | Detects BOOT0 state invalid for application boot mode. |
| `diodes_ap2112k_typical_ldo_passes` | `examples/good_diodes_ap2112k_3v3_regulator/project.yaml` | pass | AP2112K 3.3 V regulator with 5 V input and 1 uF input/output capacitors. |
| `diodes_ap2112k_ldo_observation_passes` | `examples/good_ap2112k_3v3_ldo_observation/project.yaml` | pass | AP2112K generated-SPICE LDO observation with 5 V input, 3.3 V regulated output, and light output load. |
| `ams_ams1117_3v3_ldo_observation_passes` | `examples/good_ams1117_3v3_ldo_observation/project.yaml` | pass | AMS1117-3.3 generated-SPICE LDO observation with 5 V input, 22 uF output support capacitance, and a minimum-load resistor. |
| `ams_ams1117_3v3_static_regulator_passes` | `examples/good_ams1117_3v3_regulator/project.yaml` | pass | AMS1117-3.3 static regulator fixture with 5 V input, 22 uF output support capacitance, and enough minimum load. |
| `diodes_ap2112k_dropout_detected` | `examples/bad_diodes_ap2112k_3v3_dropout/project.yaml` | fail | Detects insufficient nominal dropout margin. |
| `diodes_ap2112k_output_current_detected` | `examples/bad_diodes_ap2112k_3v3_output_current/project.yaml` | fail | Detects AP2112K output load above the datasheet-backed 600 mA output-current class. |
| `diodes_ap2112k_output_capacitance_detected` | `examples/bad_diodes_ap2112k_3v3_output_capacitance/project.yaml` | fail | Detects AP2112K output support capacitance below the datasheet-backed 1 uF requirement. |
| `ams_ams1117_3v3_dropout_detected` | `examples/bad_ams1117_3v3_dropout/project.yaml` | fail | Detects AMS1117 nominal dropout margin below the datasheet-backed 1.3 V limit. |
| `ams_ams1117_3v3_minimum_load_detected` | `examples/bad_ams1117_3v3_minimum_load/project.yaml` | fail | Detects AMS1117 proven minimum load below the datasheet-backed 10 mA requirement. |
| `ams_ams1117_3v3_output_current_detected` | `examples/bad_ams1117_3v3_output_current/project.yaml` | fail | Detects AMS1117 output load above the source-backed 800 mA regulation-screen limit. |
| `ams_ams1117_3v3_output_capacitance_detected` | `examples/bad_ams1117_3v3_output_capacitance/project.yaml` | fail | Detects AMS1117 output support capacitance below the datasheet-backed 22 uF requirement. |
| `microchip_mcp73831_typical_usb_charger_passes` | `examples/good_microchip_mcp73831_usb_charger/project.yaml` | pass | MCP73831 USB-powered 4.2 V Li-Ion charger with 100 mA programmed current. |
| `microchip_mcp73831_charger_observation_passes` | `examples/good_mcp73831_charger_observation/project.yaml` | pass | MCP73831 generated-SPICE charger observation with USB input, battery node, and source-backed programmed current. |
| `microchip_mcp73831_prog_resistor_passes` | `examples/good_microchip_mcp73831_prog_resistor/project.yaml` | pass | MCP73831 static PROG-resistor fixture deriving charge current from a 10 kOhm programming resistor. |
| `microchip_mcp73831_usb_budget_detected` | `examples/bad_microchip_mcp73831_usb_budget/project.yaml` | fail | Detects charge current above declared USB input budget. |
| `microchip_mcp73831_charge_current_detected` | `examples/bad_microchip_mcp73831_charge_current/project.yaml` | fail | Detects programmed charge current above the datasheet-backed 500 mA charger maximum. |
| `microchip_mcp73831_prog_resistor_charge_current_detected` | `examples/bad_microchip_mcp73831_prog_resistor_charge_current/project.yaml` | fail | Detects PROG-resistor-derived charge current above the datasheet-backed 500 mA charger maximum. |
| `microchip_mcp73831_missing_current_detected` | `examples/bad_microchip_mcp73831_missing_current/project.yaml` | fail | Detects missing explicit or PROG-resistor-derived programmed charge current evidence. |
| `ti_bq24075_typical_usb_charger_passes` | `examples/good_ti_bq24075_usb_charger/project.yaml` | pass | BQ24075 USB-powered 4.2 V Li-Ion charger with 450 mA programmed current and 5.5 V system output evidence. |
| `ti_bq24075_power_path_observation_passes` | `examples/good_bq24075_power_path_observation/project.yaml` | pass | BQ24075 generated-SPICE power-path charger observation with USB input, SYS output, battery node, and programmed charge current. |
| `ti_bq24075_usb_budget_detected` | `examples/bad_ti_bq24075_usb_budget/project.yaml` | fail | Detects programmed charge current above declared USB input budget. |
| `ti_bq24075_charge_current_detected` | `examples/bad_ti_bq24075_charge_current/project.yaml` | fail | Detects programmed charge current above the datasheet-backed 1.5 A charger maximum. |
| `ti_bq25798_nvdc_observation_passes` | `examples/good_bq25798_nvdc_observation/project.yaml` | pass | BQ25798 generated-SPICE NVDC buck-boost charger observation with 20 V adapter input, 12 V SYS target, battery node, and 2 A programmed charge current. |
| `ti_bq25798_charge_current_detected` | `examples/bad_bq25798_charge_current/project.yaml` | fail | Detects programmed charge current above the datasheet-backed 5 A BQ25798 charger maximum. |
| `ti_tps2115a_typical_power_mux_passes` | `examples/good_ti_tps2115a_power_mux/project.yaml` | pass | TPS2115A USB-selected mux with inactive unpowered battery input. |
| `ti_tps2115a_power_mux_observation_passes` | `examples/good_tps2115a_power_mux_observation/project.yaml` | pass | TPS2115A generated-SPICE power-mux observation with selected USB input and protected output rail. |
| `ti_tps2115a_output_overcurrent_detected` | `examples/bad_ti_tps2115a_output_current/project.yaml` | fail | Detects output load above modeled mux current limit. |
| `ti_tps2121_typical_power_mux_passes` | `examples/good_ti_tps2121_power_mux/project.yaml` | pass | TPS2121 USB-selected priority mux with inactive unpowered backup input. |
| `ti_tps2121_power_mux_observation_passes` | `examples/good_tps2121_power_mux_observation/project.yaml` | pass | TPS2121 generated-SPICE priority-mux observation with selected adapter input and protected output rail. |
| `ti_tps2121_output_overcurrent_detected` | `examples/bad_ti_tps2121_output_current/project.yaml` | fail | Detects output load above the datasheet-backed 4.5 A mux current class. |
| `ti_tps2121_input_overvoltage_detected` | `examples/bad_ti_tps2121_input_overvoltage/project.yaml` | fail | Detects selected input voltage above the datasheet-backed 22 V operating maximum. |
| `ti_tps22918_load_switch_passes` | `examples/good_ti_tps22918_load_switch/project.yaml` | pass | TPS22918 active-high load switch with 5 V input, powered 3.3 V output, and explicit `ON` high evidence. |
| `ti_tps22918_load_switch_observation_passes` | `examples/good_tps22918_load_switch_observation/project.yaml` | pass | TPS22918 generated-SPICE observation with a 5 V input source, enabled `ON` drive, and light switched load. |
| `ti_tps22918_missing_on_detected` | `examples/bad_ti_tps22918_missing_on/project.yaml` | fail | Detects a powered TPS22918 output rail without source-backed evidence that `ON` is high. |
| `ti_tps22918_output_overcurrent_detected` | `examples/bad_ti_tps22918_output_current/project.yaml` | fail | Detects TPS22918 switched output load above the source-backed 2 A current class. |
| `ti_tps25948_efuse_observation_passes` | `examples/good_tps25948_efuse_observation/project.yaml` | pass | TPS25948 eFuse/generated-SPICE observation with 12 V input, protected output rail, explicit enabled `EN` evidence, and a light switched load. |
| `ti_tps25948_missing_enable_detected` | `examples/bad_tps25948_missing_enable/project.yaml` | fail | Detects a powered TPS25948 protected output without source-backed evidence that `EN` is high. |
| `ti_tps24751_hot_swap_observation_passes` | `examples/good_tps24751_hot_swap_observation/project.yaml` | pass | TPS24751/CSD17501Q5A hot-swap/generated-SPICE observation with 12 V input, protected output rail, explicit enabled `EN` evidence, and a light load. |
| `ti_tps24751_missing_enable_detected` | `examples/bad_tps24751_missing_enable/project.yaml` | fail | Detects a powered TPS24751 protected output without source-backed evidence that `EN` is high. |
| `ti_tpd2eusb30_typical_usb_esd_passes` | `examples/good_ti_tpd2eusb30_usb_esd/project.yaml` | pass | TPD2EUSB30 D+/D- clamps with 5.5 V standoff and 0.7 pF line capacitance evidence. |
| `ti_tpd2eusb30_usb_esd_observation_passes` | `examples/good_tpd2eusb30_usb_esd_observation/project.yaml` | pass | TPD2EUSB30 generated-SPICE USB D+/D- clamp observation with normal data-line voltages below source-backed standoff. |
| `ti_tpd2eusb30_capacitance_budget_detected` | `examples/bad_ti_tpd2eusb30_usb_esd_capacitance/project.yaml` | fail | Detects clamp capacitance above a stricter interface budget. |
| `ti_tpd2eusb30_standoff_detected` | `examples/bad_ti_tpd2eusb30_usb_esd_standoff/project.yaml` | fail | Detects a USB data-line voltage above the TPD2EUSB30 source-backed 5.5 V standoff rating. |
| `nexperia_prtr5v0u2x_typical_usb_esd_passes` | `examples/good_nexperia_prtr5v0u2x_usb_esd/project.yaml` | pass | PRTR5V0U2X rail-to-rail D+/D- clamp with 5.5 V standoff and 1.5 pF maximum I/O line capacitance evidence. |
| `nexperia_prtr5v0u2x_usb_esd_observation_passes` | `examples/good_nexperia_prtr5v0u2x_usb_esd_observation/project.yaml` | pass | PRTR5V0U2X generated-SPICE USB D+/D- clamp observation with rail-to-rail reference pins and normal line voltages. |
| `nexperia_prtr5v0u2x_capacitance_budget_detected` | `examples/bad_nexperia_prtr5v0u2x_usb_esd_capacitance/project.yaml` | fail | Detects PRTR5V0U2X capacitance above a stricter USB data-line budget. |
| `nexperia_prtr5v0u2x_reference_detected` | `examples/bad_nexperia_prtr5v0u2x_usb_esd_reference/project.yaml` | fail | Detects PRTR5V0U2X `VCC` tied to a non-power signal net instead of a power reference. |
| `nexperia_pesd5v0s1ul_vbus_esd_passes` | `examples/good_nexperia_pesd5v0s1ul_vbus_esd/project.yaml` | pass | PESD5V0S1UL VBUS-to-ground clamp with source-backed 5.0 V standoff and 200 pF maximum capacitance evidence. |
| `nexperia_pesd5v0s1ul_capacitance_budget_detected` | `examples/bad_nexperia_pesd5v0s1ul_vbus_capacitance/project.yaml` | fail | Detects PESD5V0S1UL capacitance above a stricter VBUS-line budget. |
| `ti_esd2can24_q1_can_esd_observation_passes` | `examples/good_ti_esd2can24_q1_can_esd_observation/project.yaml` | pass | ESD2CAN24-Q1 CANH/CANL generated-SPICE observation with normal dominant-state line voltages below the 24 V standoff limit. |
| `ti_esd2can24_q1_reference_detected` | `examples/bad_ti_esd2can24_q1_can_esd_reference/project.yaml` | fail | Detects ESD2CAN24-Q1 `GND` tied to the CANH signal net instead of a ground reference. |
| `ti_esds552_rs485_esd_observation_passes` | `examples/good_ti_esds552_rs485_esd_observation/project.yaml` | pass | ESDS552 RS-485 A/B generated-SPICE observation with normal line voltages below the 12 V standoff limit. |
| `ti_esds552_reference_detected` | `examples/bad_ti_esds552_rs485_esd_reference/project.yaml` | fail | Detects ESDS552 `GND` tied to the RS-485 A signal net instead of a ground reference. |
| `ti_tps62162_typical_buck_passes` | `examples/good_ti_tps62162_3v3_buck/project.yaml` | pass | TPS62162 fixed 3.3 V synchronous buck with 12 V input, 10 uF input capacitance, 22 uF output capacitance, and 2.2 uH direct output inductance. |
| `ti_tps62162_buck_observation_passes` | `examples/good_tps62162_3v3_buck_observation/project.yaml` | pass | TPS62162 generated-SPICE buck observation with 12 V input, source-backed 3.3 V output target, and light load. |
| `ti_tps62162_output_overcurrent_detected` | `examples/bad_ti_tps62162_3v3_output_current/project.yaml` | fail | Detects output load above modeled buck current limit. |
| `ti_tps62162_output_inductance_detected` | `examples/bad_ti_tps62162_3v3_output_inductance/project.yaml` | fail | Detects direct SW-to-output inductance below the datasheet-backed minimum. |
| `ti_tps54331_5v_buck_observation_passes` | `examples/good_tps54331_5v_buck_observation/project.yaml` | pass | TPS54331 5 V buck/generated-SPICE observation with source-backed 12 V input, 5 V output feedback, and light load. |
| `ti_tps54331_5v_output_overcurrent_detected` | `examples/bad_tps54331_5v_output_current/project.yaml` | fail | Detects TPS54331 5 V buck output load above the source-backed 3 A regulator output-current class. |
| `ti_tps61023_typical_5v_boost_passes` | `examples/good_ti_tps61023_5v_boost/project.yaml` | pass | TPS61023 5 V boost with Li-ion input, 10 uF input capacitance, 2 x 22 uF output capacitance, and 1 uH direct input inductance. |
| `ti_tps61023_boost_observation_passes` | `examples/good_tps61023_5v_boost_observation/project.yaml` | pass | TPS61023 generated-SPICE boost observation with Li-ion input, 5 V output target, and source-backed support network. |
| `ti_tps61023_input_inductance_detected` | `examples/bad_ti_tps61023_5v_input_inductance/project.yaml` | fail | Detects direct VIN-to-SW boost inductance below the datasheet-backed minimum. |
| `ti_tps63802_typical_3v3_buck_boost_passes` | `examples/good_ti_tps63802_3v3_buck_boost/project.yaml` | pass | TPS63802 3.3 V buck-boost with Li-ion input, 10 uF input capacitance, 22 uF output capacitance, and 0.47 uH direct L1-L2 switch inductance. |
| `ti_tps63802_buck_boost_observation_passes` | `examples/good_tps63802_3v3_buck_boost_observation/project.yaml` | pass | TPS63802 generated-SPICE buck-boost observation with Li-ion input, 3.3 V output target, and source-backed switch inductance. |
| `ti_tps63802_switch_inductance_detected` | `examples/bad_ti_tps63802_3v3_switch_inductance/project.yaml` | fail | Detects direct L1-to-L2 buck-boost switch inductance below the datasheet-backed minimum. |
| `ti_tps63802_output_overcurrent_detected` | `examples/bad_ti_tps63802_3v3_output_current/project.yaml` | fail | Detects output load above the datasheet-backed 2 A output-current condition. |
| `espressif_esp32_wroom_32e_application_passes` | `examples/good_espressif_esp32_wroom_32e_application/project.yaml` | pass | ESP32-WROOM-32E on a 3.3 V rail with enough source-current budget and GPIO0 biased high for SPI flash boot. |
| `espressif_esp32_wroom_32e_boot_uart_observation_passes` | `examples/good_esp32_wroom_32e_boot_uart_observation/project.yaml` | pass | ESP32-WROOM-32E generated-SPICE boot/UART observation with source-backed boot strap and UART line-state metadata. |
| `espressif_esp32_wroom_32e_supply_current_detected` | `examples/bad_espressif_esp32_wroom_32e_supply_current/project.yaml` | fail | Detects a 3.3 V source-current budget below the datasheet-backed 0.5 A external-supply requirement. |
| `espressif_esp32_wroom_32e_gpio0_bootstrap_detected` | `examples/bad_espressif_esp32_wroom_32e_bootstrap/project.yaml` | fail | Detects GPIO0 biased below the high threshold required for SPI flash boot. |
| `raspberrypi_rp2040_bootsel_power_board_passes` | `examples/good_raspberrypi_rp2040_bootsel_power/project.yaml` | pass | RP2040 with 3.3 V IOVDD/VREG_VIN/USB_VDD/ADC_AVDD, internal 1.1 V VREG_VOUT feeding DVDD, RUN pulled high, and QSPI_SS pulled high for external-flash boot. |
| `raspberrypi_rp2040_iovdd_overvoltage_detected` | `examples/bad_raspberrypi_rp2040_iovdd_overvoltage/project.yaml` | fail | Detects IOVDD connected to a 5 V rail above the source-backed 3.63 V maximum. |
| `nordic_nrf52840_normal_voltage_power_passes` | `examples/good_nordic_nrf52840_normal_voltage_power/project.yaml` | pass | nRF52840 with source-backed 3.3 V `VDD`, 5 V `VBUS`, active-low reset, SWD, USB, and RF antenna pin boundaries. |
| `nordic_nrf52840_board_observation_passes` | `examples/good_nordic_nrf52840_board_observation/project.yaml` | pass | nRF52840 generated-SPICE board observation with source-backed 3.3 V `VDD`/`VDDH`, 5 V `VBUS`, released reset, SWD/UART idle states, USB boundary, and antenna feed bias. |
| `nordic_nrf52840_vdd_overvoltage_detected` | `examples/bad_nordic_nrf52840_vdd_overvoltage/project.yaml` | fail | Detects `VDD` connected to a 5 V rail above the source-backed 3.6 V maximum. |
| `silabs_cp2102n_usb_uart_observation_passes` | `examples/good_silabs_cp2102n_usb_uart_observation/project.yaml` | pass | CP2102N USB-UART bridge with 5 V `VREGIN`, generated 3.3 V `VDD`/`VIO`, reset pull-up, and generated-SPICE UART/modem output-state observation. |
| `silabs_cp2102n_vdd_overvoltage_detected` | `examples/bad_silabs_cp2102n_vdd_overvoltage/project.yaml` | fail | Detects CP2102N `VDD` connected to a 5 V rail above the source-backed 3.6 V maximum. |
| `wch_ch340c_usb_uart_observation_passes` | `examples/good_wch_ch340c_usb_uart_observation/project.yaml` | pass | CH340C USB-UART bridge with source-backed 3.3 V supply mode and generated-SPICE idle/control-line output observation. |
| `wch_ch340c_power_overvoltage_detected` | `examples/bad_wch_ch340c_power_overvoltage/project.yaml` | fail | Detects CH340C `VCC` connected to a 6 V rail above the source-backed 5.3 V maximum. |
| `wch_ch340n_usb_uart_observation_passes` | `examples/good_wch_ch340n_usb_uart_observation/project.yaml` | pass | CH340N SOP-8 USB-UART bridge with source-backed 3.3 V supply mode and generated-SPICE TXD/RTS# output-state observation. |
| `wch_ch340n_power_overvoltage_detected` | `examples/bad_wch_ch340n_power_overvoltage/project.yaml` | fail | Detects CH340N `VCC` connected to a 6 V rail above the source-backed 5.3 V maximum. |
| `ftdi_ft232r_usb_uart_observation_passes` | `examples/good_ftdi_ft232r_usb_uart_observation/project.yaml` | pass | FT232R USB-UART bridge with 5 V `VCC`, generated 3.3 V `3V3OUT`/`VCCIO`, reset pull-up, and generated-SPICE UART/modem output-state observation. |
| `ftdi_ft232r_vcc_overvoltage_detected` | `examples/bad_ftdi_ft232r_vcc_overvoltage/project.yaml` | fail | Detects FT232R `VCC` connected to a 6 V rail above the source-backed 5.25 V operating maximum. |
| `wch_ch347_usb_jtag_observation_passes` | `examples/good_wch_ch347_usb_jtag_observation/project.yaml` | pass | CH347T USB-JTAG/debug bridge with source-backed 3.3 V supply mode and generated-SPICE UART/JTAG line-state observation. |
| `cmsis_dap_swd_probe_observation_passes` | `examples/good_cmsis_dap_swd_probe_observation/project.yaml` | pass | CMSIS-DAP SWD generated-SPICE probe observation with VTREF, SWCLK/SWDIO high, nRESET released, and SWO pulled low. |
| `wch_ch347_vcc_overvoltage_detected` | `examples/bad_wch_ch347_vcc_overvoltage/project.yaml` | fail | Detects CH347T `VCC` connected to a 5 V rail above the source-backed 3.6 V maximum. |
| `ti_txs0108e_level_shifter_observation_passes` | `examples/good_ti_txs0108e_level_shifter_observation/project.yaml` | pass | TXS0108E generated-SPICE observation with source-backed 1.8 V to 3.3 V enabled A1-to-B1 level-shift behavior. |
| `ti_txs0108e_oe_low_unpowered_side_passes` | `examples/good_ti_txs0108e_oe_low_unpowered_side/project.yaml` | pass | TXS0108E static review where one side is unpowered and explicit `OE` low evidence disables the channel. |
| `ti_txs0108e_unpowered_side_detected` | `examples/bad_ti_txs0108e_unpowered_side/project.yaml` | fail | Detects powered-to-unpowered TXS0108E channel use without disabled-channel evidence. |
| `ti_txs0108e_oe_low_unconnected_detected` | `examples/bad_ti_txs0108e_oe_low_unconnected/project.yaml` | fail | Detects that asserted `OE` low evidence cannot disable a TXS0108E channel when the required `OE` pin is unconnected. |
| `ti_txs0108e_supply_order_detected` | `examples/bad_ti_txs0108e_supply_order/project.yaml` | fail | Detects TXS0108E `VCCA` above `VCCB`, violating the source-backed supply-order constraint. |
| `ti_tcan3413_can_transceiver_observation_passes` | `examples/good_ti_tcan3413_can_transceiver_observation/project.yaml` | pass | TCAN3413 CAN FD transceiver generated-SPICE observation with source-backed 3.3 V `VCC`, 3.3 V `VIO`, and normal reduced bus-line snapshot. |
| `ti_tcan3413_vcc_overvoltage_detected` | `examples/bad_ti_tcan3413_vcc_overvoltage/project.yaml` | fail | Detects TCAN3413 `VCC` connected to a 5 V rail above the source-backed 3.6 V maximum. |
| `ti_thvd1450_rs485_transceiver_observation_passes` | `examples/good_ti_thvd1450_rs485_transceiver_observation/project.yaml` | pass | THVD1450 RS-485 transceiver generated-SPICE observation with source-backed 3.3 V `VCC` and enabled driver/receiver line-state snapshot. |
| `ti_thvd1450_vcc_overvoltage_detected` | `examples/bad_ti_thvd1450_vcc_overvoltage/project.yaml` | fail | Detects THVD1450 `VCC` connected to a 6 V rail above the source-backed 5.5 V maximum. |
| `ti_drv8323_gate_driver_observation_passes` | `examples/good_drv8323_gate_driver_observation/project.yaml` | pass | DRV8323 gate-driver/generated-SPICE observation with source-backed motor-bus `VM`, `DVDD`, SPI/control pins, and enabled line-state snapshot. |
| `ti_drv8323_vm_overvoltage_detected` | `examples/bad_drv8323_vm_overvoltage/project.yaml` | fail | Detects DRV8323 `VM` connected to a 70 V motor rail above the source-backed 60 V maximum. |
| `nxp_pca9685_pwm_driver_observation_passes` | `examples/good_pca9685_pwm_driver_observation/project.yaml` | pass | PCA9685 generated-SPICE PWM-driver observation with source-backed VDD, OE low, I2C idle states, and four representative PWM output samples. |
| `tdk_icm42688p_imu_observation_passes` | `examples/good_tdk_icm42688p_imu_observation/project.yaml` | pass | ICM-42688-P generated-SPICE IMU observation with source-backed VDD/VDDIO, SPI idle inputs, SDO, and INT1 line states. |
| `sensirion_sht31_i2c_observation_passes` | `examples/good_sensirion_sht31_i2c_observation/project.yaml` | pass | SHT31-DIS generated-SPICE I2C observation with source-backed VDD, SDA/SCL pull-ups, ADDR low, nRESET high, and ALERT idle low. |
| `jst_xh_servo_connector_observation_passes` | `examples/good_jst_xh_servo_connector_observation/project.yaml` | pass | JST XH generated-SPICE servo connector observation with source-backed contact-resistance pass-through on power, ground, and PWM signal pins. |
| `jst_vh_actuator_bus_connector_observation_passes` | `examples/good_jst_vh_actuator_bus_connector_observation/project.yaml` | pass | JST VH generated-SPICE actuator-bus connector observation with source-backed contact-resistance pass-through on power, CAN, enable, fault, and sync pins. |
| `st_stm8s003f3p6_power_passes` | `examples/good_st_stm8s003f3p6_power/project.yaml` | pass | STM8S003F3P6 with 5 V `VDD`, `VCAP` pin support capacitor, active-low reset, SWIM, and UART1 TX/RX boundaries. |
| `st_stm8s003f3p6_vdd_overvoltage_detected` | `examples/bad_st_stm8s003f3p6_vdd_overvoltage/project.yaml` | fail | Detects `VDD` connected to a 6 V rail above the source-backed 5.5 V maximum. |
| `st_stm32l431_boot_uart_swd_observation_passes` | `examples/good_stm32l431_boot_uart_swd_observation/project.yaml` | pass | STM32L431 generated-SPICE boot UART/SWD observation with source-backed power, reset, BOOT0, UART, and SWD pin metadata. |
| `sipeed_licheerv_nano_w_observation_passes` | `examples/good_sipeed_licheerv_nano_w_observation/project.yaml` | pass | LicheeRV-Nano-W generated-SPICE module observation with source-backed 5 V rail, UART0 TX/RX, motion-enable, and fault-IRQ line-state metadata. |
| `artery_at32f435_motion_core_observation_passes` | `examples/good_artery_at32f435_motion_core_observation/project.yaml` | pass | AT32F435 generated-SPICE motion-core observation with source-backed VDD and UART, motion-control, CAN, RS-485, and servo-enable line states. |
| `artery_at32m416_motor_control_observation_passes` | `examples/good_artery_at32m416_motor_control_observation/project.yaml` | pass | AT32M416 generated-SPICE motor-control observation with source-backed VDD, CAN, PWM, gate-driver, SPI, current-sense, encoder, enable, and fault line states. |
| `stc_stc15w408as_power_passes` | `examples/good_stc_stc15w408as_power/project.yaml` | pass | STC15W408AS with 5 V `VCC`, active-high reset boundary, and primary UART/ISP RX/TX pin boundaries. |
| `stc_stc15w408as_vcc_overvoltage_detected` | `examples/bad_stc_stc15w408as_vcc_overvoltage/project.yaml` | fail | Detects `VCC` connected to a 6 V rail above the source-backed 5.5 V maximum. |
| `ti_ne555_astable_power_passes` | `examples/good_ti_ne555_astable_power/project.yaml` | pass | NE555 in a source-backed astable-style board connection on a 5 V `VCC` rail with VCC and CONT bypass capacitors. |
| `ti_ne555_vcc_overvoltage_detected` | `examples/bad_ti_ne555_vcc_overvoltage/project.yaml` | fail | Detects NE555 `VCC` connected to an 18 V rail above the source-backed 16 V maximum for NA555/NE555/SA555. |
| `microchip_mcp1316_reset_supervisor_passes` | `examples/good_microchip_mcp1316_reset_supervisor/project.yaml` | pass | MCP1316T-29LE/OT monitors a 3.3 V rail and drives an MCU reset net with source-backed 2.90 V threshold and 280 ms reset-timeout metadata. |
| `microchip_mcp1316_nominal_rail_detected` | `examples/bad_microchip_mcp1316_nominal_rail/project.yaml` | fail | Detects a 2.9 V monitored rail below the MCP1316 worst-case 2.973 V threshold maximum. |
| `ti_tlv803ea29_reset_supervisor_passes` | `examples/good_ti_tlv803ea29_reset_supervisor/project.yaml` | pass | TLV803EA29 active-low open-drain reset supervisor monitors a 3.3 V rail and drives an MCU reset net. |
| `ti_tlv803ea29_reset_observation_passes` | `examples/good_tlv803ea29_reset_observation/project.yaml` | pass | TLV803EA29 generated-SPICE reset-threshold observation with a pulsed 3.3 V rail and reset pull-up. |
| `ti_tlv803ea29_nominal_rail_detected` | `examples/bad_ti_tlv803ea29_nominal_rail/project.yaml` | fail | Detects a 2.9 V monitored rail below the TLV803EA29 worst-case 2.9886 V release-threshold maximum. |
| `abracon_abm3_8mhz_clock_passes` | `examples/good_clock_source_crystal/project.yaml` | pass | ABM3 8 MHz crystal between MCU oscillator pins with 32 pF leg capacitors and modeled 2 pF stray capacitance, producing the source-backed 18 pF load target. |
| `abracon_abm3_8mhz_load_capacitance_detected` | `examples/bad_clock_source_load_capacitance/project.yaml` | fail | Detects 8 pF leg capacitors that produce only 6 pF effective load against the ABM3 18 pF target. |
| `winbond_w25q64jv_spi_flash_power_passes` | `examples/good_winbond_w25q64jv_spi_flash_power/project.yaml` | pass | W25Q64JV SPI/QSPI NOR flash on a 3.3 V VCC rail with every 8-pin SPI/QSPI board-boundary pin bound. |
| `winbond_w25q64jv_spi_flash_observation_passes` | `examples/good_winbond_w25q64jv_spi_flash_observation/project.yaml` | pass | W25Q64JV generated-SPICE standby observation with source-backed 3.3 V `VCC`, `/CS` high, `/WP` high, `/HOLD or /RESET` high, and low idle bias on `CLK`, `DI/IO0`, and high-impedance `DO/IO1`. |
| `winbond_w25q64jv_vcc_overvoltage_detected` | `examples/bad_winbond_w25q64jv_vcc_overvoltage/project.yaml` | fail | Detects W25Q64JV `VCC` connected to a 5 V rail above the source-backed 3.6 V maximum. |
| `microchip_at24c02c_i2c_eeprom_power_passes` | `examples/good_microchip_at24c02c_i2c_eeprom_power/project.yaml` | pass | AT24C02C I2C EEPROM on a 3.3 V VCC rail with all SOIC-8 board-boundary pins bound. |
| `microchip_at24c02c_i2c_eeprom_observation_passes` | `examples/good_microchip_at24c02c_i2c_eeprom_observation/project.yaml` | pass | AT24C02C generated-SPICE I2C observation with source-backed 3.3 V `VCC`, idle-high `SDA`/`SCL`, `A0`/`A1`/`A2` low address select, and `WP` low for normal writes. |
| `microchip_at24c02c_vcc_overvoltage_detected` | `examples/bad_microchip_at24c02c_vcc_overvoltage/project.yaml` | fail | Detects AT24C02C `VCC` connected to a 6 V rail above the source-backed 5.5 V maximum. |
| `microchip_mcp23017_i2c_gpio_expander_power_passes` | `examples/good_microchip_mcp23017_i2c_gpio_expander_power/project.yaml` | pass | MCP23017 I2C GPIO expander on a 3.3 V VDD rail with every modeled 28-pin SSOP board-boundary pin bound. |
| `microchip_mcp23017_i2c_gpio_expander_observation_passes` | `examples/good_microchip_mcp23017_i2c_gpio_expander_observation/project.yaml` | pass | MCP23017 generated-SPICE I2C GPIO observation with source-backed 3.3 V `VDD`, idle-high `SDA`/`SCL`, low `A0`/`A1`/`A2`, released `RESET`, idle `INTA`/`INTB`, `GPA0` high, and `GPB0` low. |
| `microchip_mcp23017_vdd_overvoltage_detected` | `examples/bad_microchip_mcp23017_vdd_overvoltage/project.yaml` | fail | Detects MCP23017 `VDD` connected to a 6 V rail above the source-backed 5.5 V maximum. |
| `bosch_bme280_i2c_power_passes` | `examples/good_bosch_bme280_i2c_power/project.yaml` | pass | BME280 with source-backed 3.3 V `VDD`/`VDDIO`, `CSB` tied high for I2C, and `SDO` tied low for address `0x76`. |
| `bosch_bme280_i2c_observation_passes` | `examples/good_bosch_bme280_i2c_observation/project.yaml` | pass | BME280 generated-SPICE I2C observation with source-backed 3.3 V `VDD`/`VDDIO`, idle SDA/SCL pull-ups, `CSB` high for I2C, and `SDO` low for address `0x76`. |
| `bosch_bme280_vddio_overvoltage_detected` | `examples/bad_bosch_bme280_vddio_overvoltage/project.yaml` | fail | Detects BME280 `VDDIO` connected to a 5 V rail above the source-backed 3.6 V maximum. |
| `onsemi_1n4148ws_switching_diode_passes` | `examples/good_diode_switching/project.yaml` | pass | 1N4148WS switching diode feeds a 1 k load while retaining the source-backed generated-SPICE model and diode operating-limit probes. |
| `onsemi_1n4148ws_overcurrent_detected` | `examples/bad_diode_overcurrent/project.yaml` | fail | Detects 1N4148WS forward current above the source-backed 150 mA average rectified-current rating. |
| `onsemi_1n4148ws_reverse_voltage_detected` | `examples/bad_diode_reverse_voltage/project.yaml` | fail | Detects 1N4148WS reverse voltage above the source-backed 100 V repetitive peak reverse-voltage rating. |
| `onsemi_1n4148ws_offset_reverse_voltage_detected` | `examples/bad_diode_reverse_voltage_offset/project.yaml` | fail | Detects reverse-voltage stress using the diode terminal voltage difference rather than node-to-ground voltage. |
| `onsemi_1n4148ws_missing_derating_detected` | `examples/bad_diode_missing_derating/project.yaml` | fail | Detects temperature-derating requests when the diode model lacks power-derating metadata. |
| `onsemi_nds7002a_low_side_switch_passes` | `examples/good_mosfet_low_side_switch/project.yaml` | pass | NDS7002A low-side MOSFET switch pulls a 100 ohm load while retaining the source-backed generated-SPICE model and MOSFET operating-limit probes. |
| `onsemi_nds7002a_overcurrent_detected` | `examples/bad_mosfet_overcurrent/project.yaml` | fail | Detects NDS7002A drain current above the source-backed 280 mA continuous rating and power above the 300 mW limit. |
| `onsemi_nds7002a_high_ambient_derating_detected` | `examples/bad_mosfet_high_ambient_derating/project.yaml` | fail | Detects NDS7002A power dissipation above the source-backed temperature-derated limit at high ambient temperature. |
| `onsemi_nds7002a_unqualified_pulse_detected` | `examples/bad_mosfet_unqualified_pulse_rating/project.yaml` | fail | Detects pulsed-current use when the MOSFET model lacks pulse width and duty-cycle qualification metadata. |
| `onsemi_nds7002a_model_missing_sha_detected` | `examples/bad_mosfet_model_missing_sha/project.yaml` | fail | Detects an external MOSFET model file without a pinned SHA-256 digest. |
| `onsemi_bss84_high_side_switch_passes` | `examples/good_pmos_high_side_switch/project.yaml` | pass | BSS84 high-side PMOS switch pulls a 200 ohm load from a 5 V rail while retaining the source-backed generated-SPICE model and MOSFET operating-limit probes. |
| `onsemi_bss84_overcurrent_detected` | `examples/bad_pmos_overcurrent/project.yaml` | fail | Detects BSS84 drain current above the source-backed 130 mA continuous magnitude. |
| `onsemi_fdmc86184_qualified_pulse_passes` | `examples/good_mosfet_qualified_pulse_current/project.yaml` | pass | FDMC86184 low-side pulse switch exercises source-backed pulsed-current width and duty metadata without exceeding the qualified pulse envelope. |
| `onsemi_fdmc86184_pulse_duty_detected` | `examples/bad_mosfet_pulse_duty/project.yaml` | fail | Detects an FDMC86184 pulse that remains below scalar pulsed current but exceeds the encoded pulse width and duty-cycle constraints. |
| `onsemi_fdmc86184_soa_violation_detected` | `examples/bad_mosfet_soa_violation/project.yaml` | fail | Detects an FDMC86184 simultaneous VDS/ID stress point outside the hand-digitized Figure 11 SOA envelope. |
| `onsemi_ss8050_ss8550_switches_pass` | `examples/good_onsemi_ss8050_ss8550_switches/project.yaml` | pass | SS8050 low-side and SS8550 high-side BJT switches drive light 1k loads while retaining the source-backed shared generated-SPICE model. |
| `onsemi_ss8050_collector_overcurrent_detected` | `examples/bad_bjt_overcurrent/project.yaml` | fail | Detects SS8050 collector current above the source-backed 1.5 A continuous rating. |
| `onsemi_2n3904_low_side_switch_passes` | `examples/good_onsemi_2n3904_low_side_switch/project.yaml` | pass | 2N3904 low-side switch driven from 5 V through a 47k base resistor into a 1k collector load, retaining the source-backed generated-SPICE model and operating-limit probes. |
| `onsemi_2n3904_collector_overcurrent_detected` | `examples/bad_onsemi_2n3904_collector_overcurrent/project.yaml` | fail | Detects a 2N3904 collector-current violation above the source-backed 200 mA continuous rating. |
| `onsemi_2n3906_high_side_switch_passes` | `examples/good_onsemi_2n3906_high_side_switch/project.yaml` | pass | 2N3906 high-side switch driven from 5 V through a 47k base resistor into a 1k collector load, retaining signed PNP rating provenance. |
| `onsemi_2n3906_collector_overcurrent_detected` | `examples/bad_onsemi_2n3906_collector_overcurrent/project.yaml` | fail | Detects a 2N3906 collector-current violation above the source-backed 200 mA continuous magnitude while preserving the signed rating value. |
| `onsemi_1n5819_schottky_rectifier_passes` | `examples/good_onsemi_1n5819_schottky_rectifier/project.yaml` | pass | 1N5819 Schottky rectifier feeds a light 5 V load while retaining the source-backed generated-SPICE model and diode operating-limit probes. |
| `onsemi_1n5819_overcurrent_detected` | `examples/bad_onsemi_1n5819_overcurrent/project.yaml` | fail | Detects 1N5819 forward current above the source-backed 1 A average rectified-current rating. |
| `kingbright_apt1608surck_led_indicator_passes` | `examples/good_kingbright_apt1608surck_led_indicator/project.yaml` | pass | APT1608SURCK red LED indicator driven from 3.3 V through 1 kOhm while retaining the source-backed generated-SPICE card and LED operating-limit probes. |
| `kingbright_apt1608surck_overcurrent_detected` | `examples/bad_kingbright_apt1608surck_led_overcurrent/project.yaml` | fail | Detects APT1608SURCK LED forward current and power above the source-backed 30 mA / 75 mW ratings. |
| `ti_csd17484f4_low_side_switch_passes` | `examples/good_csd17484f4_low_side_switch/project.yaml` | pass | CSD17484F4 generated-SPICE low-side MOSFET switch observation with the retained TI SPICE card. |
| `ti_csd17484f4_vcsel_capacitor_discharge_passes` | `examples/good_csd17484f4_vcsel_capacitor_discharge/project.yaml` | pass | CSD17484F4 generated-SPICE capacitor-discharge observation with an initial-condition VCSEL-style load path. |
| `onsemi_nl27wz17_logic_buffer_observation_passes` | `examples/good_onsemi_nl27wz17_logic_buffer_observation/project.yaml` | pass | NL27WZ17 generated-SPICE logic-buffer observation with 3.3 V VCC, one high input mirrored high, and one low input mirrored low. |
| `onsemi_nl27wz17_vcc_overvoltage_detected` | `examples/bad_onsemi_nl27wz17_vcc_overvoltage/project.yaml` | fail | Detects NL27WZ17 `VCC` connected to a 6 V rail above the source-backed 5.5 V maximum. |
| `espressif_esp32_s3_wroom_1u_application_passes` | `examples/good_espressif_esp32_s3_wroom_1u_application/project.yaml` | pass | ESP32-S3-WROOM-1U-N16R8 on a 3.3 V rail with enough source-current budget and GPIO0 biased high for SPI flash boot. |
| `espressif_esp32_s3_wroom_1u_boot_usb_observation_passes` | `examples/good_esp32_s3_wroom_boot_usb_observation/project.yaml` | pass | ESP32-S3-WROOM-1U generated-SPICE boot/USB observation with source-backed boot strap and USB line-state metadata. |
| `espressif_esp32_s3_wroom_1u_supply_current_detected` | `examples/bad_espressif_esp32_s3_wroom_1u_supply_current/project.yaml` | fail | Detects a 3.3 V source-current budget below the datasheet-backed 0.5 A IVDD requirement. |
| `espressif_esp32_s3_wroom_1u_gpio46_bootstrap_detected` | `examples/bad_espressif_esp32_s3_wroom_1u_download_bootstrap/project.yaml` | fail | Detects GPIO46 biased high when joint download boot requires GPIO0 low and GPIO46 low. |

Run command:

```bash
circuitci validate-suite suites/public_typical_circuits.yaml --output out/public-typical-circuits
```

## 2026-07-13 Result

Observed command output:

```text
CircuitCI suite public_typical_circuits: pass (cases=305, passed=305, failed=0)
```

The generated suite and case reports are written under
`out/public-typical-circuits/`.

Observed detection details:

| Detection case | Finding | Observed message |
| --- | --- | --- |
| `jlcpcb_drill_diameter_process_detected` | `DRILL_DIAMETER_VALID` | Drill hit `0` is `0.100 mm`; selected fabrication process supports `0.150 mm` to `6.300 mm` circular drills. Drill hit `1` is `6.400 mm`; selected fabrication process supports `0.150 mm` to `6.300 mm` circular drills. |
| `manufacturing_drill_to_board_edge_metadata_detected` | `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID` | Drill hit 0 has 0.300 mm edge-to-board clearance, below 0.500 mm minimum. |
| `manufacturing_drill_to_cutout_edge_detected` | `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID` | Drill hit 0 has 0.200 mm edge-to-board clearance, below 0.400 mm minimum. |
| `jlcpcb_slot_width_process_detected` | `SLOT_WIDTH_VALID` | Routed slot `0` is `0.600 mm` wide for plated process evidence; required at least `0.650 mm`. Routed slot `1` is `0.900 mm` wide for non-plated process evidence; required at least `1.000 mm`. Routed slot `2` is `0.900 mm` wide for unknown-plating process evidence; required at least `1.000 mm`. |
| `manufacturing_slot_to_board_edge_metadata_detected` | `SLOT_TO_BOARD_EDGE_CLEARANCE_VALID` | Routed slot 0 has 0.300 mm edge-to-board clearance, below 0.500 mm minimum. |
| `jlcpcb_slot_aspect_ratio_process_detected` | `SLOT_ASPECT_RATIO_VALID` | Routed slot `0` has length-to-width ratio `2.000`; selected process requires at least `2.500`. |
| `jlcpcb_castellated_hole_process_detected` | `CASTELLATED_HOLE_VALID` | Castellated drill hit `0` is `0.250 mm`; selected castellated-hole process requires at least `0.300 mm`. Castellated drill hit `1` has `0.750 mm` hole-edge-to-board-edge clearance, below `1.000 mm` minimum. Castellated drill hits `2` and `3` have `0.200 mm` hole-to-hole spacing, below `0.400 mm` minimum. |
| `jlcpcb_drill_annular_ring_process_detected` | `DRILL_ANNULAR_RING_VALID` | Drill hit `0` has `0.045 mm` annular ring, below `0.050 mm` minimum. |
| `manufacturing_drill_annular_missing_copper_detected` | `DRILL_ANNULAR_RING_VALID` | Plated/unknown drill hit 0 has no co-located Gerber copper flash evidence within 0.050 mm. |
| `manufacturing_drill_annular_missing_required_layer_detected` | `DRILL_ANNULAR_RING_VALID` | Plated/unknown drill hit 0 has no co-located Gerber copper flash evidence on required layer B.Cu within 0.050 mm. |
| `manufacturing_drill_annular_owner_mismatch_detected` | `DRILL_ANNULAR_RING_VALID` | Drill hit 0 owner net GND is co-located with Gerber copper flash owner net VBUS, so annular-ring evidence is on the wrong owner. |
| `manufacturing_drill_annular_same_net_owner_mismatch_detected` | `DRILL_ANNULAR_RING_VALID` | Drill hit 0 owner pad J1/1 on net GND is co-located with Gerber copper flash owner pad J1/2 on net GND, so annular-ring evidence is on a different pad/via owner. |
| `jlcpcb_copper_to_board_edge_process_detected` | `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID` | Gerber copper segment `0` has `0.150 mm` board-edge clearance, below `0.200 mm` minimum. |
| `manufacturing_copper_feature_to_board_edge_detected` | `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID` | Gerber copper feature 0 has 0.100 mm board-edge clearance, below 0.250 mm minimum. |
| `manufacturing_copper_segment_to_board_edge_detected` | `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID` | Gerber copper segment 0 has 0.200 mm board-edge clearance, below 0.250 mm minimum. |
| `manufacturing_copper_region_to_board_edge_detected` | `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID` | Gerber copper region 0 has 0.100 mm board-edge clearance, below 0.250 mm minimum. |
| `jlcpcb_copper_spacing_process_detected` | `COPPER_SPACING_VALID` | Gerber copper feature and feature have `0.050 mm` same-layer spacing, below `0.100 mm` minimum. |
| `manufacturing_copper_feature_spacing_detected` | `COPPER_SPACING_VALID` | Gerber copper feature and feature have 0.200 mm same-layer spacing, below 0.250 mm minimum. |
| `manufacturing_copper_segment_spacing_detected` | `COPPER_SPACING_VALID` | Gerber copper segment and segment have 0.200 mm same-layer spacing, below 0.250 mm minimum. |
| `manufacturing_copper_region_spacing_detected` | `COPPER_SPACING_VALID` | Gerber copper feature and region have 0.150 mm same-layer spacing, below 0.250 mm minimum. |
| `manufacturing_copper_feature_segment_spacing_detected` | `COPPER_SPACING_VALID` | Gerber copper feature and segment have 0.350 mm same-layer spacing, below 0.400 mm minimum. |
| `manufacturing_copper_different_net_overlap_detected` | `COPPER_SPACING_VALID` | Gerber copper feature and feature have 0.000 mm same-layer spacing, below 0.250 mm minimum. |
| `manufacturing_copper_different_island_overlap_detected` | `COPPER_SPACING_VALID` | Gerber copper feature and feature have 0.000 mm same-layer spacing, below 0.250 mm minimum. |
| `jlcpcb_solder_mask_opening_process_detected` | `SOLDER_MASK_OPENING_VALID` | Solder-mask opening feature on `B.Mask` expands copper flash `0` by only `0.030000 mm`; required at least `0.050000 mm`. |
| `manufacturing_solder_mask_opening_region_detected` | `SOLDER_MASK_OPENING_VALID` | Solder-mask opening region on F.Mask expands copper flash 0 by only -0.056569 mm; required at least 0.050000 mm. |
| `manufacturing_solder_mask_opening_missing_detected` | `SOLDER_MASK_OPENING_VALID` | Copper flash 0 on F.Cu has no co-located solder-mask opening on F.Mask. |
| `manufacturing_solder_mask_opening_undersized_detected` | `SOLDER_MASK_OPENING_VALID` | Solder-mask opening feature on B.Mask expands copper flash 0 by only 0.030000 mm; required at least 0.050000 mm. |
| `manufacturing_unknown_fabrication_process_detected` | `VALIDATION_INPUT_MISSING` | manufacturing parameters.fabrication_process references unsupported process preset 'unknown_fab_process'. |
| `jlcpcb_solder_mask_dam_process_detected` | `SOLDER_MASK_DAM_VALID` | Solder-mask feature and feature openings on `F.Mask` leave only `0.080000 mm` mask dam; required at least `0.100000 mm`. |
| `manufacturing_solder_mask_dam_segment_region_detected` | `SOLDER_MASK_DAM_VALID` | Solder-mask segment and region openings on F.Mask leave only 0.120000 mm mask dam; required at least 0.150000 mm. |
| `jlcpcb_solder_paste_aperture_size_process_detected` | `SOLDER_PASTE_APERTURE_SIZE_VALID` | Solder-paste feature opening on `F.Paste` has minimum aperture size `0.080000 mm`; JLCPCB stencil process requires greater than `0.080000 mm`. Solder-paste segment opening on `F.Paste` has minimum aperture size `0.070000 mm`; JLCPCB stencil process requires greater than `0.080000 mm`. |
| `jlcpcb_solder_paste_area_ratio_process_detected` | `SOLDER_PASTE_APERTURE_AREA_RATIO_VALID` | Solder-paste feature opening on `F.Paste` has stencil aperture area ratio `0.300000`; JLCPCB/IPC guidance requires at least `0.660000`. |
| `manufacturing_solder_paste_opening_metadata_detected` | `SOLDER_PASTE_OPENING_VALID` | Solder-paste feature opening on F.Paste has area ratio 0.437500 against copper flash 0; allowed range is 0.700000..=1.000000. |
| `manufacturing_solder_paste_opening_region_detected` | `SOLDER_PASTE_OPENING_VALID` | Solder-paste region opening on F.Paste has area ratio 1.500000 against copper flash 0; allowed range is 0.700000..=1.000000. |
| `manufacturing_solder_paste_opening_windowed_detected` | `SOLDER_PASTE_OPENING_VALID` | Solder-paste feature opening on F.Paste has area ratio 0.250000 against copper flash 0; allowed range is 0.200000..=0.240000. |
| `manufacturing_solder_paste_opening_missing_detected` | `SOLDER_PASTE_OPENING_VALID` | Copper flash 0 on F.Cu has no co-located solder-paste opening on F.Paste. |
| `manufacturing_solder_paste_opening_oversized_detected` | `SOLDER_PASTE_OPENING_VALID` | Solder-paste feature opening on F.Paste has area ratio 1.237500 against copper flash 0; allowed range is 0.700000..=1.000000. |
| `manufacturing_solder_paste_spacing_metadata_detected` | `SOLDER_PASTE_SPACING_VALID` | Solder-paste feature and feature openings on F.Paste leave only 0.080000 mm spacing; required at least 0.150000 mm. |
| `manufacturing_solder_paste_spacing_segment_region_detected` | `SOLDER_PASTE_SPACING_VALID` | Solder-paste segment and region openings on F.Paste leave only 0.120000 mm spacing; required at least 0.150000 mm. |
| `jlcpcb_solder_paste_ic_pin_aperture_detected` | `SOLDER_PASTE_IC_PIN_APERTURE_VALID` | Solder-paste feature opening on F.Paste has IC pin aperture width 0.300000 mm; JLCPCB pitch-conditioned range for 0.500 mm pitch is 0.240000..=0.240000 mm. |
| `jlcpcb_solder_paste_ic_pin_aperture_length_detected` | `SOLDER_PASTE_IC_PIN_APERTURE_VALID` | Solder-paste feature opening on F.Paste has IC pin aperture length 0.800000 mm; JLCPCB pitch-conditioned length for 0.650 mm pitch is 1.000000 mm. |
| `jlcpcb_solder_paste_ic_pin_aperture_extension_detected` | `SOLDER_PASTE_IC_PIN_APERTURE_VALID` | Solder-paste feature opening on F.Paste has IC pin aperture length 1.300000 mm; JLCPCB pitch-conditioned length extension for 0.500 mm pitch requires at least 1.400000 mm from owner-matched copper pad length 1.200000 mm. |
| `jlcpcb_solder_paste_bga_aperture_detected` | `SOLDER_PASTE_BGA_APERTURE_VALID` | Solder-paste feature opening on F.Paste has BGA aperture size 0.400000 mm; JLCPCB source size for 0.500 mm BGA pitch is 0.300000 mm. |
| `jlcpcb_solder_paste_bga_pitch_grid_detected` | `SOLDER_PASTE_BGA_APERTURE_VALID` | Pad-owned solder-paste flashes do not prove a two-axis BGA grid at 0.650 mm pitch; found 0 horizontal and 0 vertical matching gaps. |
| `assembly_footprint_alignment_detected` | `ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID` | Component U1 assembly source.footprint 'Package_SO:SOIC-8_3.9x4.9mm_P1.27mm' does not match any imported KiCad footprint property value. |
| `pin_1_orientation_detected` | `PIN_1_ORIENTATION_VALID` | Component U1 imported pin-1 marker direction 180.000 deg differs from expected 0.000 deg by 180.000 deg. |
| `usb_connector_orientation_detected` | `USB_CONNECTOR_ORIENTATION_VALID` | USB connector J1 placement rotation 180.000 deg differs from expected 0.000 deg by 180.000 deg, exceeding limit 5.000 deg. |
| `usb_connector_edge_proximity_detected` | `USB_CONNECTOR_EDGE_PROXIMITY_VALID` | USB connector J1 is 1.400 mm from the nearest board edge, exceeding limit 0.500 mm. |
| `usb_connector_edge_proximity_footprint_detected` | `USB_CONNECTOR_EDGE_PROXIMITY_VALID` | USB connector J1 is 1.200 mm from the nearest board edge, exceeding limit 0.500 mm. |
| `usb_connector_body_overhang_detected` | `USB_CONNECTOR_BODY_OVERHANG_VALID` | USB connector J1 body overhang 0.050 mm past the nearest board edge exceeds limit 0.020 mm. |
| `usb_connector_body_overhang_circle_detected` | `USB_CONNECTOR_BODY_OVERHANG_VALID` | USB connector J1 body overhang 0.150 mm past the nearest board edge exceeds limit 0.100 mm. |
| `usb_connector_component_clearance_detected` | `USB_CONNECTOR_COMPONENT_CLEARANCE_VALID` | USB connector J1 clearance to component R1 is 0.300 mm, below required 0.500 mm. |
| `usb_connector_entry_clearance_detected` | `USB_CONNECTOR_ENTRY_CLEARANCE_VALID` | USB connector J1 cable-entry corridor is obstructed by component R1 at 0.750 mm into the required 2.000 mm depth. |
| `usb_connector_entry_clearance_aperture_detected` | `USB_CONNECTOR_ENTRY_CLEARANCE_VALID` | USB connector J1 cable-entry corridor is obstructed by component R1 at 0.150 mm into the required 2.000 mm depth. |
| `usb_connector_entry_clearance_model_offset_detected` | `USB_CONNECTOR_ENTRY_CLEARANCE_VALID` | USB connector J1 cable-entry corridor is obstructed by component R1 at 0.750 mm into the required 2.000 mm depth. |
| `usb_connector_missing_data_protection_detected` | `USB_CONNECTOR_PROTECTION_VALID` | USB connector J1 D+ pin D+ on net usb_dp has no valid protection clamp coverage. |
| `usb_connector_missing_vbus_protection_detected` | `USB_CONNECTOR_PROTECTION_VALID` | USB connector J1 VBUS pin VBUS on net usb_vbus has no valid protection clamp coverage. |
| `usb_connector_shield_ground_detected` | `USB_CONNECTOR_PROTECTION_VALID` | USB connector J1 shield pin SHIELD is connected to digital_or_analog net usb_shield, expected ground because require_shield_ground is true. |
| `usb_protection_placement_detected` | `USB_PROTECTION_PLACEMENT_VALID` | USB connector J1 D+ net usb_dp is protected by UESD.dp, but placement distance 6.000 mm exceeds limit 2.000 mm. |
| `usb_route_geometry_detected` | `USB_ROUTE_GEOMETRY_VALID` | USB connector J1 D+ net usb_dp route length 6.000 mm exceeds limit 5.000 mm. |
| `usb_vbus_route_geometry_detected` | `USB_VBUS_ROUTE_VALID` | USB connector J1 VBUS net usb_vbus route length 6.000 mm exceeds limit 5.000 mm. |
| `usb_return_path_unreferenced_route_detected` | `USB_RETURN_PATH_VALID` | USB connector J1 D+ net usb_dp has 1.000 mm of routed data-line length without same-layer ground-zone outline coverage, above limit 0.000 mm. |
| `usb_return_path_filled_zone_gap_detected` | `USB_RETURN_PATH_VALID` | USB connector J1 D+ net usb_dp has 1.000 mm of routed data-line length without same-layer ground-zone filled-polygon coverage, above limit 0.000 mm. |
| `usb_return_path_floating_zone_detected` | `USB_RETURN_PATH_VALID` | USB connector J1 D+ net usb_dp has 1.000 mm of routed data-line length without same-layer ground-zone filled-polygon coverage with same-net pad/via contact evidence, above limit 0.000 mm. |
| `usb_return_path_split_zone_detected` | `USB_RETURN_PATH_VALID` | USB connector J1 D+ net usb_dp has 1.000 mm of routed data-line length without same-layer ground-zone filled-polygon coverage with same-net pad/via contact evidence, above limit 0.000 mm. |
| `usb_return_path_zone_edge_clearance_detected` | `USB_RETURN_PATH_VALID` | USB connector J1 D+ net usb_dp segment 0 has 0.020 mm filled ground-zone edge clearance, below required 0.100 mm. |
| `usb_return_path_stitching_via_detected` | `USB_RETURN_PATH_VALID` | USB connector J1 D+ net usb_dp data via 0 is 1.500 mm from the nearest matching ground stitch via, above limit 0.200 mm. |
| `generic_power_tree_overvoltage_detected` | `POWER_TREE_VALID` | Power rail rail_5v supplies U1.VDD at 5.000000 V, outside the model maximum operating voltage 3.600000 V. |
| `generic_power_tree_current_budget_detected` | `POWER_TREE_VALID` | Power rail rail_3v3 worst-case declared load 0.050000 A exceeds supply limit 0.040000 A. |
| `generic_load_switch_missing_enable_detected` | `POWER_TREE_VALID` | Load switch USW output rail sensor_3v3 is declared powered but USW.EN is not proven high. |
| `generic_load_switch_disabled_output_detected` | `POWER_TREE_VALID` | Load switch USW output rail sensor_3v3 is declared powered but USW.EN is not proven high. |
| `generic_load_switch_output_current_detected` | `POWER_TREE_VALID` | Load switch USW worst-case output load 0.070000 A exceeds switch limit 0.050000 A. |
| `generic_power_mux_missing_selection_detected` | `POWER_TREE_VALID` | Power mux UMUX requires component parameter selected_input for source-selection validation. |
| `generic_power_mux_selected_unpowered_detected` | `POWER_TREE_VALID` | Power rail battery for UMUX.BAT_IN is not declared powered. |
| `generic_power_mux_backfeed_detected` | `POWER_TREE_VALID` | Power mux UMUX output rail sys is powered while inactive input battery on battery is unpowered and lacks reverse-blocking evidence. |
| `generic_regulator_dropout_detected` | `POWER_TREE_VALID` | Regulator UREG dropout margin 0.100000 V is below required dropout 0.300000 V. |
| `generic_regulator_output_current_detected` | `POWER_TREE_VALID` | Regulator UREG worst-case output load 0.050000 A exceeds regulator limit 0.040000 A. |
| `generic_regulator_conversion_pin_detected` | `POWER_TREE_VALID` | power_conversion output_pin OUT is not declared in model ports. |
| `generic_regulator_startup_sequence_detected` | `POWER_TREE_VALID` | Regulator UREG output rail rail_3v3 is declared valid at 1200.000000 us before input-valid plus startup delay 1800.000000 us. |
| `generic_regulator_startup_missing_timing_detected` | `POWER_TREE_VALID` | Regulator UREG declares startup_delay_us but rail rail_3v3 has no power_valid_at_us timing. |
| `generic_reset_supervisor_threshold_too_low_detected` | `POWER_TREE_VALID` | Reset supervisor USUP_LOW_THRESHOLD can release nrst at 2.400000 V, below U1.VDD minimum operating voltage 2.700000 V. |
| `generic_bootstrap_bias_threshold_detected` | `BOOT_STRAP_BIAS_VALID` | Boot strap U1.BOOT0 resistor network produces 1.650000 V on net boot0, not valid for required high state in boot mode bootloader. |
| `generic_bootstrap_bias_current_detected` | `BOOT_STRAP_BIAS_VALID` | Boot strap U1.BOOT0 resistor network draws 0.000300 A on net boot0, above limit 0.000100 A. |
| `generic_bootstrap_defined_detected` | `BOOT_STRAP_DEFINED` | Boot strap U1.BOOT0 is not valid for boot mode bootloader. |
| `generic_uart_bootloader_sync_detected` | `UART_BOOTLOADER_SYNC` | No UART bootloader sync byte was sent to U1.RX. |
| `generic_reset_release_before_power_detected` | `RESET_RELEASE_AFTER_POWER_VALID` | Reset releases before power is valid for component U1. |
| `generic_reset_power_timing_mismatch_detected` | `RESET_RELEASE_AFTER_POWER_VALID` | Scenario power_valid_at_us does not match target power rail mcu_3v3 timing. |
| `generic_reset_supervisor_delay_detected` | `RESET_RELEASE_AFTER_POWER_VALID` | Reset releases before power is valid for component U1. |
| `generic_reset_pin_mismatch_detected` | `TARGET_RESET_PIN_MISMATCH` | Scenario reset pin U1.BOOT0 does not match model reset pin NRST. |
| `um_stm32l4_wrong_uart_detected` | `UART_BOOTLOADER_SYNC` | Sender U5.TXD is not connected to target RX U1.PA10. |
| `um_stm32l4_app_boot_bad_release_detected` | `BOOT_STRAP_DEFINED` | Boot strap U1.BOOT0 is not valid for boot mode application. |
| `diodes_ap2112k_dropout_detected` | `POWER_TREE_VALID` | Regulator `UREG` dropout margin `0.300000 V` is below required dropout `0.400000 V`. |
| `diodes_ap2112k_output_current_detected` | `POWER_TREE_VALID` | Regulator `UREG` worst-case output load `0.650000 A` exceeds regulator limit `0.600000 A`. |
| `diodes_ap2112k_output_capacitance_detected` | `POWER_TREE_VALID` | Regulator `UREG` output rail `rail_3v3` has `4.700000e-7 F` support capacitance to ground, below required `1.000000e-6 F`. |
| `ams_ams1117_3v3_dropout_detected` | `POWER_TREE_VALID` | Regulator `UREG` dropout margin `0.900000 V` is below required dropout `1.300000 V`. |
| `ams_ams1117_3v3_minimum_load_detected` | `POWER_TREE_VALID` | Regulator `UREG` proven minimum output load `0.002000 A` is below required minimum load `0.010000 A`. |
| `ams_ams1117_3v3_output_current_detected` | `POWER_TREE_VALID` | Regulator `UREG` worst-case output load `0.850000 A` exceeds regulator limit `0.800000 A`. |
| `ams_ams1117_3v3_output_capacitance_detected` | `POWER_TREE_VALID` | Regulator `UREG` output rail `rail_3v3` has `1.000000e-5 F` support capacitance to ground, below required `2.200000e-5 F`. |
| `microchip_mcp73831_usb_budget_detected` | `POWER_TREE_VALID` | Battery charger `UCHG` programmed charge current `0.500000 A` exceeds input rail `usb_5v` current budget `0.100000 A`. |
| `microchip_mcp73831_charge_current_detected` | `POWER_TREE_VALID` | Battery charger `UCHG` programmed charge current `0.600000 A` exceeds model maximum `0.500000 A`. |
| `microchip_mcp73831_prog_resistor_charge_current_detected` | `POWER_TREE_VALID` | Battery charger `UCHG` programmed charge current `1.000000 A` exceeds model maximum `0.500000 A`. |
| `microchip_mcp73831_missing_current_detected` | `POWER_TREE_VALID` | Battery charger `UCHG` requires component parameter `programmed_charge_current_A` for programmed charge-current validation. |
| `ti_bq24075_usb_budget_detected` | `POWER_TREE_VALID` | Battery charger `UCHG` programmed charge current `1.000000 A` exceeds input rail `usb_5v` current budget `0.500000 A`. |
| `ti_bq24075_charge_current_detected` | `POWER_TREE_VALID` | Battery charger `UCHG` programmed charge current `1.800000 A` exceeds model maximum `1.500000 A`. |
| `ti_bq25798_charge_current_detected` | `POWER_TREE_VALID` | Battery charger `UCHG` programmed charge current `6.000000 A` exceeds model maximum `5.000000 A`. |
| `ti_tps2115a_output_overcurrent_detected` | `POWER_TREE_VALID` | Power mux `UMUX` worst-case output load `1.200000 A` exceeds mux limit `1.000000 A`. |
| `ti_tps2121_output_overcurrent_detected` | `POWER_TREE_VALID` | Power mux `UMUX` worst-case output load `5.000000 A` exceeds mux limit `4.500000 A`. |
| `ti_tps2121_input_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `adapter_24v` supplies `UMUX.IN1` at `24.000000 V`, outside the model maximum operating voltage `22.000000 V`. |
| `ti_tps22918_missing_on_detected` | `POWER_TREE_VALID` | Load switch `USW` output rail `sensor_3v3` is declared powered but `USW.ON` is not proven high. |
| `ti_tps22918_output_overcurrent_detected` | `POWER_TREE_VALID` | Load switch `USW` worst-case output load `2.100000 A` exceeds switch limit `2.000000 A`. |
| `ti_tps25948_missing_enable_detected` | `POWER_TREE_VALID` | Load switch `UEFUSE` output rail `protected_12v` is declared powered but `UEFUSE.EN` is not proven high. |
| `ti_tps24751_missing_enable_detected` | `POWER_TREE_VALID` | Load switch `UHOTSWAP` output rail `protected_12v` is declared powered but `UHOTSWAP.EN` is not proven high. |
| `ti_tpd2eusb30_capacitance_budget_detected` | `INTERFACE_PROTECTION_REVIEW` | Protection clamp `d1_plus` has `7.000e-13 F` line capacitance, above the `5.000e-13 F` interface limit. |
| `ti_tpd2eusb30_standoff_detected` | `INTERFACE_PROTECTION_REVIEW` | Protection clamp `d1_plus` on component `UESD` sees protected net `usb_dp` at `6.000000 V`, above standoff limit `5.500000 V`. |
| `nexperia_prtr5v0u2x_capacitance_budget_detected` | `INTERFACE_PROTECTION_REVIEW` | Protection clamp `io1_to_vcc` on component `UESD` has `1.500e-12 F` line capacitance on `usb_dp`, above interface limit `1.000e-12 F`. |
| `nexperia_prtr5v0u2x_reference_detected` | `INTERFACE_PROTECTION_REVIEW` | Protection clamp `io1_to_vcc` on component `UESD` reference pin `VCC` is connected to digital-or-analog net `usb_dp`, expected power. |
| `nexperia_pesd5v0s1ul_capacitance_budget_detected` | `INTERFACE_PROTECTION_REVIEW` | Protection clamp `vbus_to_ground` has `2.000e-10 F` line capacitance, above the `1.000e-10 F` interface limit. |
| `ti_esd2can24_q1_reference_detected` | `INTERFACE_PROTECTION_REVIEW` | Protection clamp `canh` on component `UESD` reference pin `GND` is connected to digital-or-analog net `canh`, expected ground. |
| `ti_esds552_reference_detected` | `INTERFACE_PROTECTION_REVIEW` | Protection clamp `a` on component `UESD` reference pin `GND` is connected to digital-or-analog net `rs485_a`, expected ground. |
| `ti_tps62162_output_overcurrent_detected` | `POWER_TREE_VALID` | Regulator `UBUCK` worst-case output load `1.200000 A` exceeds regulator limit `1.000000 A`. |
| `ti_tps62162_output_inductance_detected` | `POWER_TREE_VALID` | Regulator `UBUCK` output inductor path `buck_sw->rail_3v3` has `1.000000e-6 H` direct inductance, outside the modeled support range. |
| `ti_tps54331_5v_output_overcurrent_detected` | `POWER_TREE_VALID` | Regulator `UBUCK` worst-case output load `3.500000 A` exceeds regulator limit `3.000000 A`. |
| `ti_tps61023_input_inductance_detected` | `POWER_TREE_VALID` | Regulator `UBOOST` input inductor path `battery->boost_sw` has `2.200000e-7 H` direct inductance, outside the modeled support range. |
| `ti_tps63802_switch_inductance_detected` | `POWER_TREE_VALID` | Regulator `UBUCKBOOST` switch inductor path `bb_l1->bb_l2` has `2.200000e-7 H` direct inductance, outside the modeled support range. |
| `ti_tps63802_output_overcurrent_detected` | `POWER_TREE_VALID` | Regulator `UBUCKBOOST` worst-case output load `2.500000 A` exceeds regulator limit `2.000000 A`. |
| `espressif_esp32_wroom_32e_supply_current_detected` | `POWER_TREE_VALID` | The ESP32-WROOM-32E declared load current `0.500000 A` exceeds the 3.3 V rail current budget `0.300000 A`. |
| `espressif_esp32_wroom_32e_gpio0_bootstrap_detected` | `BOOT_STRAP_BIAS_VALID` | GPIO0 is biased to `1.650000 V`, below the `2.475000 V` high threshold required for SPI flash boot. |
| `raspberrypi_rp2040_iovdd_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_5v` supplies `URP.IOVDD` at `5.000000 V`, outside the model maximum operating voltage `3.630000 V`. |
| `nordic_nrf52840_vdd_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_5v` supplies `UNRF.VDD` at `5.000000 V`, outside the model maximum operating voltage `3.600000 V`. |
| `silabs_cp2102n_vdd_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `cp2102n_vdd_bad_5v` supplies `U6.VDD` at `5.000000 V`, outside the model maximum operating voltage `3.600000 V`. |
| `wch_ch340c_power_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_6v` supplies `U5.VCC` at `6.000000 V`, outside the model maximum operating voltage `5.300000 V`. |
| `wch_ch340n_power_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_6v` supplies `U5.VCC` at `6.000000 V`, outside the model maximum operating voltage `5.300000 V`. |
| `ftdi_ft232r_vcc_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `usb_6v_bad` supplies `U7.VCC` at `6.000000 V`, outside the model maximum operating voltage `5.250000 V`. |
| `wch_ch347_vcc_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `debug_5v_bad` supplies `U8.VCC` at `5.000000 V`, outside the model maximum operating voltage `3.600000 V`. |
| `ti_txs0108e_unpowered_side_detected` | `INTERFACE_PROTECTION_REVIEW` | Interface protection channel `ch1` on component `U3` connects powered/unpowered domains, but datasheet metadata says `unpowered_isolation` is false and the scenario does not prove the channel is disabled. |
| `ti_txs0108e_oe_low_unconnected_detected` | `INTERFACE_PROTECTION_REVIEW` | Interface protection channel `ch1` on component `U3` connects powered/unpowered domains, but datasheet metadata says `unpowered_isolation` is false and the scenario does not prove the channel is disabled. |
| `ti_txs0108e_supply_order_detected` | `INTERFACE_PROTECTION_REVIEW` | Signal-conditioning supply constraint `vcca_lte_vccb` on component `U3` requires `VCCA <= VCCB`, but `5.000000 V > 3.300000 V`. |
| `ti_tcan3413_vcc_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `can_5v_bad` supplies `UCAN.VCC` at `5.000000 V`, outside the model maximum operating voltage `3.600000 V`. |
| `ti_thvd1450_vcc_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rs485_6v_bad` supplies `UTRX.VCC` at `6.000000 V`, outside the model maximum operating voltage `5.500000 V`. |
| `ti_drv8323_vm_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `motor_70v` supplies `UDRV.VM` at `70.000000 V`, outside the model maximum operating voltage `60.000000 V`. |
| `st_stm8s003f3p6_vdd_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_6v` supplies `USTM8.VDD` at `6.000000 V`, outside the model maximum operating voltage `5.500000 V`. |
| `stc_stc15w408as_vcc_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_6v` supplies `USTC.VCC` at `6.000000 V`, outside the model maximum operating voltage `5.500000 V`. |
| `ti_ne555_vcc_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_18v` supplies `U555.VCC` at `18.000000 V`, outside the model maximum operating voltage `16.000000 V`. |
| `microchip_mcp1316_nominal_rail_detected` | `POWER_TREE_VALID` | Reset supervisor `USUP` monitored rail `rail_2v9` nominal voltage `2.900000 V` is not above worst-case release threshold `2.973000 V`. |
| `ti_tlv803ea29_nominal_rail_detected` | `POWER_TREE_VALID` | Reset supervisor `USUP` monitored rail `rail_2v9` nominal voltage `2.900000 V` is not above worst-case release threshold `2.988600 V`. |
| `abracon_abm3_8mhz_load_capacitance_detected` | `CLOCK_SOURCE_VALID` | Clock source `U1` uses crystal `Y1` with effective load capacitance `6.000000e-12 F`, outside the modeled crystal load range. |
| `winbond_w25q64jv_vcc_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_5v` supplies `UFLASH.VCC` at `5.000000 V`, outside the model maximum operating voltage `3.600000 V`. |
| `microchip_at24c02c_vcc_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_6v` supplies `UEEPROM.VCC` at `6.000000 V`, outside the model maximum operating voltage `5.500000 V`. |
| `microchip_mcp23017_vdd_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_6v` supplies `UIOX.VDD` at `6.000000 V`, outside the model maximum operating voltage `5.500000 V`. |
| `bosch_bme280_vddio_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `rail_5v` supplies `UBME.VDDIO` at `5.000000 V`, outside the model maximum operating voltage `3.600000 V`. |
| `onsemi_1n4148ws_overcurrent_detected` | `SPICE_OPERATING_LIMIT` | Component `D1` exceeded datasheet `IF_AV`: maximum simulated current was `0.384935 A`, limit is `0.150000 A`; it also exceeded `PD` at `0.443047 W` against `0.200000 W`. |
| `onsemi_1n4148ws_reverse_voltage_detected` | `SPICE_OPERATING_LIMIT` | Component `D1` exceeded datasheet `VRRM`: maximum simulated voltage was `119.995528 V`, limit is `100.000000 V`. |
| `onsemi_1n4148ws_offset_reverse_voltage_detected` | `SPICE_OPERATING_LIMIT` | Component `D1` exceeded datasheet `VRRM`: maximum simulated voltage was `119.995528 V`, limit is `100.000000 V`. |
| `onsemi_1n4148ws_missing_derating_detected` | `SPICE_OPERATING_LIMIT` | Component `D1` model `vendor.onsemi.1n4148ws` has `PD` or `Ptot` but lacks linear temperature derating metadata required by scenario ambient temperature. |
| `onsemi_nds7002a_overcurrent_detected` | `SPICE_OPERATING_LIMIT` | Component `M1` exceeded datasheet `ID_continuous`: maximum simulated current was `0.703265 A`, limit is `0.280000 A`; it also exceeded `PD` at `1.248291 W` against `0.300000 W`. |
| `onsemi_nds7002a_high_ambient_derating_detected` | `SPICE_OPERATING_LIMIT` | Component `M1` exceeded datasheet `PD`: maximum simulated power was `0.139956 W`, limit is `0.120000 W`. |
| `onsemi_nds7002a_unqualified_pulse_detected` | `SPICE_OPERATING_LIMIT` | Component `M1` model `vendor.onsemi.nds7002a` enables pulse current checks but lacks qualified pulse rating metadata for `ID_pulsed`. |
| `onsemi_nds7002a_model_missing_sha_detected` | `SPICE_TRANSIENT_ANALYSIS` | Generated SPICE component `M1` requires model file `models/spice/onsemi/nds7002a.lib`, but the matching `analog.model_files` entry has no SHA-256 pin. |
| `onsemi_bss84_overcurrent_detected` | `SPICE_OPERATING_LIMIT` | Component `M1` exceeded datasheet `ID_continuous`: maximum simulated current was `0.210814 A`, limit is `0.130000 A`. |
| `onsemi_fdmc86184_pulse_duty_detected` | `SPICE_OPERATING_LIMIT` | Component `M1` exceeded datasheet `ID_continuous`: maximum simulated current was `16.607301 A`, limit is `12.000000 A`; measured pulse duration was about `500.17 us` and duty cycle about `0.250085`, above the encoded `300 us` / `0.02` pulse-rating envelope. |
| `onsemi_fdmc86184_soa_violation_detected` | `SPICE_OPERATING_LIMIT` | Component `M1` exceeded datasheet `PD`: maximum simulated power was `650.235557 W`, limit is `2.300000 W`; it also exceeded digitized SOA curve `forward_bias_100us` with `ID 15.040721 A` at `VDS 43.231676 V` against `14.801354 A` allowed. |
| `onsemi_ss8050_collector_overcurrent_detected` | `SPICE_OPERATING_LIMIT` | Component `Q1` exceeded datasheet `IC`: maximum simulated current was `2.224090 A`, limit is `1.500000 A`; it also exceeded `PD` at `3.124999 W` against `1.000000 W`. |
| `kingbright_apt1608surck_overcurrent_detected` | `SPICE_OPERATING_LIMIT` | Component `DLED` exceeded datasheet `IF`: maximum simulated current was `0.071788 A`, limit is `0.030000 A`; it also exceeded `PD` at `0.188874 W` against `0.075000 W`. |
| `onsemi_nl27wz17_vcc_overvoltage_detected` | `POWER_TREE_VALID` | Power rail `logic_6v` supplies `UBUF.VCC` at `6.000000 V`, outside the model maximum operating voltage `5.500000 V`. |
| `espressif_esp32_s3_wroom_1u_supply_current_detected` | `POWER_TREE_VALID` | Power rail `rail_3v3` worst-case declared load `0.500000 A` exceeds supply limit `0.300000 A`. |
| `espressif_esp32_s3_wroom_1u_gpio46_bootstrap_detected` | `BOOT_STRAP_BIAS_VALID` | Boot strap `UESP.IO46` resistor network produces `3.300000 V` on net `esp_io46`, not valid for required low state in boot mode `joint_download`. |

All one hundred thirty-six public-reference pass cases produced zero critical
findings.
All one hundred sixty-nine paired injected-error cases failed with the expected
critical finding ID, and all one hundred sixty-nine repair-pair checks passed.

## Interpretation Limits

This suite assesses the validator slices that are currently modeled:

- static power-tree range, dropout, current-budget, support capacitance,
  support inductance, and reference checks,
- source-backed manufacturing process geometry checks,
- expected-failure detection through suite `required_findings`,
- repair-pair accounting from bad variants to public-reference passing cases.

It does not sign off analog transient behavior, thermal behavior, charger
termination, USB eye margin, ESD pulse waveforms, power-mux switchover droop,
or final PCB layout quality unless a separate executable layout scenario is
declared.
