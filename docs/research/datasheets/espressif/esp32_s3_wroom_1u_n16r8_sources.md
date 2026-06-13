# ESP32-S3-WROOM-1U-N16R8 Source Notes

## Original Documents

| Source | URL or path | Local copy | SHA-256 |
| --- | --- | --- | --- |
| ESP32-S3-WROOM-1/WROOM-1U datasheet | <https://documentation.espressif.com/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf> | `docs/research/datasheets/espressif/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf` | `27d71971da07c280c6068d08c74720d1a25b8f20cf8494dc1765bdd28d40d435` |
| Peer `urine_monitor` LCSC cache for `C3013946` | `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C3013946_ESP32-S3-WROOM-1U-N16R8.pdf` | peer project cache | `d053da7fbeb6896bff4bd973239d1512924867d58e6ef6317fd678f7bf881ebe` |
| Peer extracted text for `C3013946` | `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C3013946_ESP32-S3-WROOM-1U-N16R8.extracted.txt` | peer project cache | not copied |

## Modeled Facts

- Recommended VDD33 operating range is `3.0 V` to `3.6 V`.
- Recommended IVDD supply current is at least `0.5 A`.
- GPIO input high threshold is `0.75 x VDD`; at `3.3 V`, CircuitCI models
  this as `2.475 V`.
- GPIO input low threshold is `0.25 x VDD`; at `3.3 V`, CircuitCI models
  this as `0.825 V`.
- EN uses the same `0.75 x VDD` high and `0.25 x VDD` low thresholds in the
  module datasheet.
- GPIO0 has default strap value `1`; GPIO46 has default strap value `0`.
- SPI boot requires GPIO0 high. Joint download boot requires GPIO0 low and
  GPIO46 low.
- The strapping hold time after CHIP_PU rises is `3 ms`.

## CircuitCI Use

`vendor.espressif.esp32_s3_wroom_1u_n16r8` is intentionally a first static
board-boundary model. It supports:

- `POWER_TREE_VALID` for the module's `3V3` voltage range and `0.5 A` source
  current budget.
- `BOOT_STRAP_BIAS_VALID` and `BOOT_STRAP_DEFINED` for GPIO0/GPIO46 boot mode
  selection.

It does not sign off RF matching, antenna placement, USB signal integrity,
flash/PSRAM pin reuse, ESP ROM protocol behavior, firmware, thermal behavior,
or transient Wi-Fi current waveform shape.
