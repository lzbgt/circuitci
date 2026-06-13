# Public Typical Circuit Verification

This note records the public-reference circuits used to assess CircuitCI's
current static validation correctness and runtime behavior.

## Source Set

The original public documents are kept in the repository so later agents can
audit the modeled facts without relying on chat history.

| Circuit family | Public source | Local original |
| --- | --- | --- |
| Diodes AP2112K 3.3 V LDO typical application | <https://www.diodes.com/datasheet/download/AP2112.pdf> | `docs/research/datasheets/diodes/ap2112.pdf` |
| Microchip MCP73831 USB Li-Ion charger typical application | <https://ww1.microchip.com/downloads/en/DeviceDoc/MCP73831-Family-Data-Sheet-DS20001984H.pdf> | `docs/research/datasheets/microchip/mcp73831-family-datasheet.pdf` |
| TI TPS2115A autoswitching power mux typical application | <https://www.ti.com/lit/ds/symlink/tps2115a.pdf> | `docs/research/datasheets/ti/tps2115a.pdf` |
| TI TPD2EUSB30 USB ESD protection typical use | <https://www.ti.com/lit/ds/symlink/tpd2eusb30.pdf> | `docs/research/datasheets/ti/tpd2eusb30.pdf` |
| TI TPS62162 3.3 V synchronous buck typical use | <https://www.ti.com/lit/ds/symlink/tps62160.pdf> | `docs/research/datasheets/ti/tps62160.pdf` |
| Espressif ESP32-WROOM-32E application boot module use | <https://www.espressif.com/sites/default/files/documentation/esp32-wroom-32e_esp32-wroom-32ue_datasheet_en.pdf> and <https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32/esp-hardware-design-guidelines-en-master-esp32.pdf> | `docs/research/datasheets/espressif/esp32-wroom-32e_esp32-wroom-32ue_datasheet_en.pdf` and `docs/research/datasheets/espressif/esp32_hardware_design_guidelines_en.pdf` |
| Espressif ESP32-S3-WROOM-1U-N16R8 application boot module use | <https://documentation.espressif.com/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf> and peer `../urine_monitor` LCSC cache | `docs/research/datasheets/espressif/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf` |

The source URLs were re-checked with web search on 2026-06-13. The local PDF
copies and SHA-256 hashes are also listed in the part-specific research notes
under `docs/research/datasheets/`.

## Executed Suite

`suites/public_typical_circuits.yaml` combines seven public-reference passing
cases and ten paired injected-error cases:

| Case | Fixture | Expected result | Purpose |
| --- | --- | --- | --- |
| `diodes_ap2112k_typical_ldo_passes` | `examples/good_diodes_ap2112k_3v3_regulator/project.yaml` | pass | AP2112K 3.3 V regulator with 5 V input and 1 uF input/output capacitors. |
| `diodes_ap2112k_dropout_detected` | `examples/bad_diodes_ap2112k_3v3_dropout/project.yaml` | fail | Detects insufficient nominal dropout margin. |
| `microchip_mcp73831_typical_usb_charger_passes` | `examples/good_microchip_mcp73831_usb_charger/project.yaml` | pass | MCP73831 USB-powered 4.2 V Li-Ion charger with 100 mA programmed current. |
| `microchip_mcp73831_usb_budget_detected` | `examples/bad_microchip_mcp73831_usb_budget/project.yaml` | fail | Detects charge current above declared USB input budget. |
| `ti_tps2115a_typical_power_mux_passes` | `examples/good_ti_tps2115a_power_mux/project.yaml` | pass | TPS2115A USB-selected mux with inactive unpowered battery input. |
| `ti_tps2115a_output_overcurrent_detected` | `examples/bad_ti_tps2115a_output_current/project.yaml` | fail | Detects output load above modeled mux current limit. |
| `ti_tpd2eusb30_typical_usb_esd_passes` | `examples/good_ti_tpd2eusb30_usb_esd/project.yaml` | pass | TPD2EUSB30 D+/D- clamps with 5.5 V standoff and 0.7 pF line capacitance evidence. |
| `ti_tpd2eusb30_capacitance_budget_detected` | `examples/bad_ti_tpd2eusb30_usb_esd_capacitance/project.yaml` | fail | Detects clamp capacitance above a stricter interface budget. |
| `ti_tps62162_typical_buck_passes` | `examples/good_ti_tps62162_3v3_buck/project.yaml` | pass | TPS62162 fixed 3.3 V synchronous buck with 12 V input, 10 uF input capacitance, 22 uF output capacitance, and 2.2 uH direct output inductance. |
| `ti_tps62162_output_overcurrent_detected` | `examples/bad_ti_tps62162_3v3_output_current/project.yaml` | fail | Detects output load above modeled buck current limit. |
| `ti_tps62162_output_inductance_detected` | `examples/bad_ti_tps62162_3v3_output_inductance/project.yaml` | fail | Detects direct SW-to-output inductance below the datasheet-backed minimum. |
| `espressif_esp32_wroom_32e_application_passes` | `examples/good_espressif_esp32_wroom_32e_application/project.yaml` | pass | ESP32-WROOM-32E on a 3.3 V rail with enough source-current budget and GPIO0 biased high for SPI flash boot. |
| `espressif_esp32_wroom_32e_supply_current_detected` | `examples/bad_espressif_esp32_wroom_32e_supply_current/project.yaml` | fail | Detects a 3.3 V source-current budget below the datasheet-backed 0.5 A external-supply requirement. |
| `espressif_esp32_wroom_32e_gpio0_bootstrap_detected` | `examples/bad_espressif_esp32_wroom_32e_bootstrap/project.yaml` | fail | Detects GPIO0 biased below the high threshold required for SPI flash boot. |
| `espressif_esp32_s3_wroom_1u_application_passes` | `examples/good_espressif_esp32_s3_wroom_1u_application/project.yaml` | pass | ESP32-S3-WROOM-1U-N16R8 on a 3.3 V rail with enough source-current budget and GPIO0 biased high for SPI flash boot. |
| `espressif_esp32_s3_wroom_1u_supply_current_detected` | `examples/bad_espressif_esp32_s3_wroom_1u_supply_current/project.yaml` | fail | Detects a 3.3 V source-current budget below the datasheet-backed 0.5 A IVDD requirement. |
| `espressif_esp32_s3_wroom_1u_gpio46_bootstrap_detected` | `examples/bad_espressif_esp32_s3_wroom_1u_download_bootstrap/project.yaml` | fail | Detects GPIO46 biased high when joint download boot requires GPIO0 low and GPIO46 low. |

Run command:

```bash
circuitci validate-suite suites/public_typical_circuits.yaml --output out/public-typical-circuits
```

## 2026-06-13 Result

Observed command output:

```text
CircuitCI suite public_typical_circuits: pass (cases=17, passed=17, failed=0)
```

The generated suite and case reports are written under
`out/public-typical-circuits/`.

Observed detection details:

| Detection case | Finding | Observed message |
| --- | --- | --- |
| `diodes_ap2112k_dropout_detected` | `POWER_TREE_VALID` | Regulator `UREG` dropout margin `0.300000 V` is below required dropout `0.400000 V`. |
| `microchip_mcp73831_usb_budget_detected` | `POWER_TREE_VALID` | Battery charger `UCHG` programmed charge current `0.500000 A` exceeds input rail `usb_5v` current budget `0.100000 A`. |
| `ti_tps2115a_output_overcurrent_detected` | `POWER_TREE_VALID` | Power mux `UMUX` worst-case output load `1.200000 A` exceeds mux limit `1.000000 A`. |
| `ti_tpd2eusb30_capacitance_budget_detected` | `INTERFACE_PROTECTION_REVIEW` | Protection clamp `d1_plus` has `7.000e-13 F` line capacitance, above the `5.000e-13 F` interface limit. |
| `ti_tps62162_output_overcurrent_detected` | `POWER_TREE_VALID` | Regulator `UBUCK` worst-case output load `1.200000 A` exceeds regulator limit `1.000000 A`. |
| `ti_tps62162_output_inductance_detected` | `POWER_TREE_VALID` | Regulator `UBUCK` output inductor path `buck_sw->rail_3v3` has `1.000000e-6 H` direct inductance, outside the modeled support range. |
| `espressif_esp32_wroom_32e_supply_current_detected` | `POWER_TREE_VALID` | The ESP32-WROOM-32E declared load current `0.500000 A` exceeds the 3.3 V rail current budget `0.300000 A`. |
| `espressif_esp32_wroom_32e_gpio0_bootstrap_detected` | `BOOT_STRAP_BIAS_VALID` | GPIO0 is biased to `1.650000 V`, below the `2.475000 V` high threshold required for SPI flash boot. |
| `espressif_esp32_s3_wroom_1u_supply_current_detected` | `POWER_TREE_VALID` | Power rail `rail_3v3` worst-case declared load `0.500000 A` exceeds supply limit `0.300000 A`. |
| `espressif_esp32_s3_wroom_1u_gpio46_bootstrap_detected` | `BOOT_STRAP_BIAS_VALID` | Boot strap `UESP.IO46` resistor network produces `3.300000 V` on net `esp_io46`, not valid for required low state in boot mode `joint_download`. |

All seven public-reference pass cases produced zero critical findings. All ten
paired injected-error cases failed with the expected critical finding ID, and
all ten repair-pair checks passed.

## Interpretation Limits

This suite assesses the validator slices that are currently modeled:

- static power-tree range, dropout, current-budget, support capacitance,
  support inductance, and reference checks,
- expected-failure detection through suite `required_findings`,
- repair-pair accounting from bad variants to public-reference passing cases.

It does not sign off analog transient behavior, thermal behavior, charger
termination, USB eye margin, ESD pulse waveforms, power-mux switchover droop,
or final PCB layout quality unless a separate executable layout scenario is
declared.
