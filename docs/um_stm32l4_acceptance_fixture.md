# UM STM32L4 Acceptance Fixture Design

This fixture is the next step toward the local `demo_project_circuit_for_acceptance.md`: validate the USB downloader path for the peer `../urine_monitor` STM32L4 board using CircuitCI's generic board IR, component model library, scenario engine, and JSON reports.

## Product Boundary

The fixture must not make the engine STM32-specific. STM32L431VCT6 and CH340C are library models. The engine only interprets generic contracts:

- power/reset/boot mode metadata
- UART bootloader sync metadata
- board nets and component pins
- scenario timing and observed strap states

## Peer-Project Evidence

Persistent source notes are in [um_stm32l4_acceptance_sources.md](research/um_stm32l4_acceptance_sources.md).

The key acceptance facts are:

- CH340C `TXD` connects to MCU `USART1_RX`, and CH340C `RXD` connects to MCU `USART1_TX`.
- MCU `USART1_TX/RX` are `PA9/PA10` in the firmware pinmux.
- The resident bootloader uses the populated `CH340C -> USART1` path.
- Runtime-safe idle on the fabricated board is `DTR=0` and `RTS=1`, corresponding to BOOT0 disabled and NRST released.
- The USB downloader reset/BOOT0 network is transistor/diode based and timing-sensitive.
- Host-tool control-line evidence is sufficient for ROM entry/readback and safe app-UART idle. It is not a complete transistor truth table and does not by itself prove reliable host-controlled flash-boot reset.

## Model Packs

Add:

- `vendor.st.stm32l431vct6`
  - power: `VDD`, `GND`
  - reset: `NRST`, active low
  - boot strap: `BOOT0`
  - boot modes:
    - `rom_bootloader`: `BOOT0 = high`
    - `application`: `BOOT0 = low`
  - bootloader interface:
    - `usart1_rom`
    - RX pin: `PA10`
    - TX pin: `PA9`
    - sync byte: `0x7F`
    - ACK byte: `0x79`

- `vendor.wch.ch340c`
  - power: `VCC`, `GND`
  - USB: `UD+`, `UD-`
  - UART: `TXD`, `RXD`
  - modem outputs: `DTR_N`, `RTS_N`
  - board-level electrical metadata is documented in
    [wch_ch340c_model.md](wch_ch340c_model.md)

The STM32 model quality is `estimated` until the bootloader metadata is cross-checked against a local ST bootloader application note. The pin mapping itself is sourced from the peer repository's design and firmware files.

## Fixtures

### `examples/um_stm32l4_rom_download_entry`

Purpose: prove the CH340C USART path and ROM-entry mode are modeled.

Expected result: pass.

Scenarios:

- `reset_boot` targeting MCU `U1`
  - required boot mode: `rom_bootloader`
  - observed `BOOT0 = high`
  - reset releases after power valid
- `serial_programming`
  - target `U1`
  - interface `usart1_rom`
  - event sends `[0x7F]` from `CH340C.TXD` to `STM32.PA10`
  - the runtime must prove the sender endpoint and target RX pin share the board net

### `examples/um_stm32l4_app_boot_bad_release`

Purpose: represent the fabricated-board downloader release failure class.

Expected result: fail with `BOOT_STRAP_DEFINED`.

Scenario:

- `reset_boot` targeting MCU `U1`
  - required boot mode: `application`
  - observed `BOOT0 = high`

This encodes the release-edge bug as the MCU still seeing bootloader strap state at sample time.

### `examples/um_stm32l4_app_boot_fixed_release`

Purpose: represent a corrected reset/application-boot release path.

Expected result: pass.

Scenario:

- `reset_boot` targeting MCU `U1`
  - required boot mode: `application`
  - observed `BOOT0 = low`

This fixture represents a verified reset/manual/electrical release where BOOT0 is low at reset sampling. It must not be modeled as a ROM `GO` substitute; ROM `GO` diagnostic execution is useful but is not the same proof as a real reset boot into user flash.

### `examples/um_stm32l4_rom_download_wrong_uart`

Purpose: prove the generic `UART_BOOTLOADER_SYNC` rule catches a wrong sender net, not just a plausible event aimed at the MCU RX pin.

Expected result: fail with `UART_BOOTLOADER_SYNC`.

Scenario:

- `serial_programming`
  - target `U1`
  - interface `usart1_rom`
  - event sends `[0x7F]` from `CH340C.TXD` to `STM32.PA10`
  - board netlist incorrectly places `CH340C.TXD` on the MCU TX net, so the sender and target RX endpoint do not share a net

## Repair Guidance

The failing app-boot fixture should suggest:

- keep BOOT0 low for application boot,
- add or tune the BOOT0 release path so BOOT0 is low at reset sampling,
- use adequate reset hold and post-release settle time,
- consider the peer research recommendation of a Q3 base-emitter bleed for existing boards,
- prefer a verified manual/electrical reset path over pure host line choreography when proving true app boot.

## Definition Of Done

- Docs and schemas remain consistent.
- Model files validate against `component_model.schema.json`.
- Fixtures validate against `board_ir.schema.json`.
- CLI tests cover one passing ROM-entry fixture, one failing app-boot fixture, and one fixed app-boot fixture.
- CLI tests cover one wrong-UART-net negative fixture so `UART_BOOTLOADER_SYNC` proves endpoint connectivity.
- Generated reports validate against `report.schema.json`.
