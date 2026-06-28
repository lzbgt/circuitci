# Espressif ESP32-WROOM-32E Model

## Source

- Datasheet:
  `docs/research/datasheets/espressif/esp32-wroom-32e_esp32-wroom-32ue_datasheet_en.pdf`
- Hardware design guidelines:
  `docs/research/datasheets/espressif/esp32_hardware_design_guidelines_en.pdf`
- Source note:
  `docs/research/datasheets/espressif/esp32_wroom_32e_sources.md`
- Retrieved: 2026-06-13

## Modeled Facts

The `vendor.espressif.esp32_wroom_32e` model captures the first static
board-boundary checks for the common ESP32-WROOM-32E Wi-Fi/Bluetooth module:

- `3V3` operating-voltage range: `3.0 V` to `3.6 V`.
- `3V3` source-current budget requirement: `0.5 A`, matching the datasheet
  minimum current delivered by the external supply.
- GPIO input thresholds at a 3.3 V rail: `2.475 V` high and `0.825 V` low.
- `EN` is active-low reset/shutdown.
- `spi_flash` boot requires `IO0` high.
- `uart_download` boot requires `IO0` low and `IO2` low.
- Strap sampling is modeled as reset release plus `3 ms`, matching the hardware
  design guideline hold-time value. This is a conservative static timing point,
  not a boot-ROM timing simulator.

## Validation Use

`POWER_TREE_VALID` screens the module rail voltage and source-current budget.
`BOOT_STRAP_BIAS_VALID` screens resistor-biased GPIO0/GPIO2 boot straps against
datasheet GPIO thresholds. `BOOT_STRAP_DEFINED` can also be used when an
external scenario supplies explicit observed strap states.

The generated-SPICE observation face
`CIRCUITCI_ESP32_WROOM_32E_BOOT_UART` adds a preliminary board-boundary
simulation for 3.3 V supply, EN release, GPIO0/GPIO2 strap state, TXD0 idle
state, and RXD0 high-impedance connectivity. Its state parameters are explicit
Board IR component parameters:

- `observation_en_state`
- `observation_io0_state`
- `observation_io2_state`
- `observation_txd0_state`

The direct-open GUI fixture is:

- `examples/good_esp32_wroom_32e_boot_uart_observation/project.yaml`

Its `Create Checks` action regenerates rail, EN, strap, and TXD0 idle checks
for the placed module without editing YAML.

The passing public fixture is:

- `examples/good_espressif_esp32_wroom_32e_application/project.yaml`

The paired injected-error fixtures are:

- `examples/bad_espressif_esp32_wroom_32e_supply_current/project.yaml`
- `examples/bad_espressif_esp32_wroom_32e_bootstrap/project.yaml`

## Limits

This model is not valid for RF matching, antenna layout, EMC, ESP ROM serial
packet timing, firmware behavior, flash/PSRAM pin-mux safety, thermal sign-off,
peak-current waveform shape, regulator/transient supply droop, or UART
protocol timing. Those require separate rules or simulation evidence.
