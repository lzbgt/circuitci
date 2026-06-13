# KiCad ESP32-WROOM-32E Suggestion Fixture

`examples/import_kicad_esp32_wroom_32e_suggestions/` proves that a native
KiCad schematic can bind an ESP32-WROOM-32E module symbol to the
datasheet-backed `vendor.espressif.esp32_wroom_32e` model and carry power plus
boot-strap evidence into automatic scenario suggestions.

The fixture contains:

- `UESP`, an ESP32-WROOM-32E module with `3V3`, `GND`, `EN`, `IO0`, `IO2`,
  `TXD0`, and `RXD0` mapped from KiCad symbol pin numbers.
- `RIO0_UP = 10k` from `RAIL_3V3` to `ESP_IO0`.
- `RIO0_DN = 100k` from `ESP_IO0` to `GND`.
- `RIO2_DN = 10k` from `ESP_IO2` to `GND`.
- KiCad net metadata mapping `RAIL_3V3` to a powered `3.3 V` rail with
  `0.6 A` source-current budget.

After import, `circuitci suggest-scenarios` emits:

- a runnable `POWER_TREE_VALID` template for the ESP32 3.3 V rail,
- a runnable `BOOT_STRAP_BIAS_VALID` template for `spi_flash` boot using the
  imported GPIO0 resistor divider,
- a runnable `BOOT_STRAP_BIAS_VALID` template for `uart_download` boot using
  the same imported GPIO0/GPIO2 resistor evidence, which will fail if copied
  into this application-biased board without changing GPIO0 bias,
- non-runnable `BOOT_STRAP_DEFINED` templates for explicit observed strap-state
  checks.

This fixture is schematic-only. It proves import and suggestion automation for
ESP32 power and boot straps, not RF layout, antenna keepout, flash/PSRAM pin
reuse, boot-ROM protocol timing, firmware behavior, or transient current shape.
