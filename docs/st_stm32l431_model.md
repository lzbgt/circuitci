# ST STM32L431 Boot/UART/SWD Model

`vendor.st.stm32l431vct6` and `vendor.um.um_stm32l4_resident` now use saved
ST source documents for preliminary STM32L431 rail, boot-strap, USART1, and
SWD board-boundary checks.

## Sources

- STM32L431xx datasheet:
  `docs/research/datasheets/st/stm32l431xx_datasheet.pdf`
  - Source URL:
    `https://www.st.com/resource/en/datasheet/stm32l431rb.pdf`
  - SHA-256:
    `71963de3cb899f8fc731e05a46c8bcb62c8ce893fe99f3877b0027ae9dfc5c6f`
  - Retrieved: `2026-06-28`
- AN2606 STM32 microcontroller system memory boot mode:
  `docs/research/datasheets/st/an2606_stm32_system_memory_boot_mode.pdf`
  - Source URL:
    `https://www.st.com/resource/en/application_note/cd00167594-stm32-microcontroller-system-memory-boot-mode-stmicroelectronics.pdf`
  - SHA-256:
    `25b466e39ecc61671dcb988d8ca538462d6452d85f68140cb981ed728f57e654`
  - Retrieved: `2026-06-28`
- AN4555 STM32L4 hardware development guide:
  `docs/research/datasheets/st/an4555_stm32l4_hardware_getting_started.pdf`
  - Source URL:
    `https://www.st.com/resource/en/application_note/an4555-getting-started-with-stm32l4-series-and-stm32l4-series-hardware-development-stmicroelectronics.pdf`
  - SHA-256:
    `a7c410da6c6a378a3823340fcc44930f62f0efa0f0654ddc6bd7755367458e7e`
  - Retrieved: `2026-06-28`

## Modeled Facts

- STM32L4 VDD operating range is represented as `1.71 V` to `3.6 V`.
- Existing boot metadata keeps BOOT0 low for application boot and high for ROM
  bootloader entry.
- USART1 uses PA9 as TX and PA10 as RX in the current bootloader acceptance
  fixture.
- SWD board-boundary observation uses PA13/SWDIO and PA14/SWCLK, with SWDIO
  modeled as an explicit high line state and SWCLK as an externally driven idle
  low input.

## Generated-SPICE Face

`CIRCUITCI_STM32L431_BOOT_UART_SWD_MCU` is a reduced observation model for:

- VDD rail checks.
- NRST released/held checks.
- BOOT0 application/ROM strap checks.
- USART1 PA9 TX and PA10 RX line-state checks.
- SWD PA13/SWDIO and PA14/SWCLK line-state checks.

The output states are explicit Board IR component parameters:

- `observation_pa9_state`
- `observation_pa13_state`

The direct-open GUI fixture is:

- `examples/good_stm32l431_boot_uart_swd_observation/project.yaml`

Its `Create Checks` action regenerates VDD, reset, boot, USART1, and SWD
checks for the placed MCU without editing YAML.

## Limits

This model is not valid for firmware execution, oscillator accuracy, reset
timing, UART protocol timing, SWD protocol transactions, flash programming side
effects, exhaustive package-pin mapping, layout, EMC, thermal behavior, or
final signal-integrity sign-off. Those require firmware, timing, programming,
package-symbol, layout, thermal, and SI evidence.
