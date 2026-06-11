# UM STM32L4 Acceptance Source Notes

This note records the local evidence used to build the CircuitCI acceptance fixture for the peer `../urine_monitor` project. The acceptance target is the local repo file `demo_project_circuit_for_acceptance.md`; the peer repository contributes board and firmware evidence.

## Peer Source Files

- `../urine_monitor/firmware_stm32l431_bootloader/README.md`
- `../urine_monitor/firmware_stm32l431_bootloader/app/um_stm32_boot_proto.h`
- `../urine_monitor/firmware_stm32l431_node/cubemx/Core/Src/usart.c`
- `../urine_monitor/tools/flash_um_stm32l4_usb.sh`
- `../urine_monitor/tools/um_stm32_cli_probe.py`
- `../urine_monitor/docs/research/usb_downloader_circuit_2026-06-09/usb_downloader_circuit_2026-06-09.md`
- `../urine_monitor/docs/fresh_design/projects/um_stm32l4_v1/spec/connectivity.toml`
- `../urine_monitor/docs/fresh_design/projects/um_stm32l4_v1/spec/pinmap.md`
- `../urine_monitor/docs/fresh_design/projects/um_stm32l4_v1/spec/schematic_spec.md`

## Extracted Facts

- The acceptance board is `UM-STM32L4 v1` using an STM32L431-family MCU.
- The fresh-design connectivity file maps MCU `PA9` to `USART1_TX` and `PA10` to `USART1_RX`.
- The same connectivity file maps CH340C `TXD` to `USART1_RX` and `RXD` to `USART1_TX`.
- The resident bootloader README says the board uses a populated `CH340C -> USART1` path.
- The resident bootloader README says the fabricated-board failure was narrowed to CH340 auto-reset / boot-to-flash sequencing, not resident-image validity.
- The firmware USART1 MSP init configures `PA9` and `PA10` as `USART1` alternate-function pins.
- The USB flashing wrapper documents a host-tool mapping that enters ROM and reads back over the populated CH340 path: `--swap-rts-dtr --reset-active-high --boot0-active-low`.
- The CLI probe documents runtime-safe idle as `DTR=0 => BOOT0 disabled` and `RTS=1 => NRST released`.
- The downloader research identifies a transistor/diode reset/BOOT0 network with `Q2 = S8050`, `Q3 = SS8550`, `D13 = 1N4148WS`, `R1/R26/R27 = 1k`, `R8 = 10k`.
- The downloader research concludes the network is analog and edge-sensitive: entering ROM is reasonable, but leaving ROM by pure line choreography is less deterministic than issuing a ROM `GO`.
- The downloader research recommends a `68k` base-emitter bleed on Q3 as the first single hardware rework for release-edge determinism on existing boards.

## Modeling Limits

- The fixture does not claim full transistor-level simulation.
- The fixture does not execute STM32 firmware.
- The fixture encodes the ROM-entry and app-boot mode checks using generic reset/boot/UART validation rules.
- The STM32L431 and CH340C behavior are component-library metadata, not engine branches.
- The host-tool control-line mapping is proven here only for ROM entry/readback and safe app-UART idle behavior. This note does not prove the full transistor truth table or reliable host-controlled flash-boot reset.
