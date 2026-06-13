# Espressif ESP32-S3-WROOM-1U-N16R8 Model

## Source

- Datasheet:
  `docs/research/datasheets/espressif/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf`
- Peer evidence:
  `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C3013946_ESP32-S3-WROOM-1U-N16R8.pdf`
- Source note:
  `docs/research/datasheets/espressif/esp32_s3_wroom_1u_n16r8_sources.md`
- Retrieved: 2026-06-13

## Modeled Facts

The `vendor.espressif.esp32_s3_wroom_1u_n16r8` model captures the first static
board-boundary checks for the peer project's ESP32-S3-WROOM-1U module:

- `3V3` operating-voltage range: `3.0 V` to `3.6 V`.
- `3V3` source-current budget requirement: `0.5 A`.
- GPIO input thresholds at a 3.3 V rail: `2.475 V` high and `0.825 V` low.
- `EN` is the active-low reset/chip-enable input.
- `spi_flash` boot requires `IO0` high.
- `joint_download` boot requires `IO0` low and `IO46` low.
- Strap sampling is modeled as reset release plus `3 ms`.

## Validation Use

`POWER_TREE_VALID` screens rail voltage and source-current budget.
`BOOT_STRAP_BIAS_VALID` screens resistor-biased GPIO0/GPIO46 boot straps
against the modeled thresholds. `BOOT_STRAP_DEFINED` can be used when a
scenario supplies explicit observed strap states.

The passing public fixture is:

- `examples/good_espressif_esp32_s3_wroom_1u_application/project.yaml`

The paired injected-error fixtures are:

- `examples/bad_espressif_esp32_s3_wroom_1u_supply_current/project.yaml`
- `examples/bad_espressif_esp32_s3_wroom_1u_download_bootstrap/project.yaml`

## Limits

This model is not valid for RF matching, antenna layout, EMC, USB eye margin,
ESP ROM serial/USB download protocol timing, firmware behavior,
flash/PSRAM pin-mux safety, thermal sign-off, or transient current waveform
shape. Those require separate rules or simulation evidence.
