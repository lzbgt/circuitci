# STM32L431 Source Capture

Date: 2026-06-28

CircuitCI persisted the official ST source PDFs used for the STM32L431
boot/UART/SWD observation model:

| Source | Local file | SHA-256 |
| --- | --- | --- |
| STM32L431xx datasheet | `docs/research/datasheets/st/stm32l431xx_datasheet.pdf` | `71963de3cb899f8fc731e05a46c8bcb62c8ce893fe99f3877b0027ae9dfc5c6f` |
| AN2606 STM32 microcontroller system memory boot mode | `docs/research/datasheets/st/an2606_stm32_system_memory_boot_mode.pdf` | `25b466e39ecc61671dcb988d8ca538462d6452d85f68140cb981ed728f57e654` |
| AN4555 STM32L4 hardware development guide | `docs/research/datasheets/st/an4555_stm32l4_hardware_getting_started.pdf` | `a7c410da6c6a378a3823340fcc44930f62f0efa0f0654ddc6bd7755367458e7e` |

The current reduced generated-SPICE model uses only board-boundary facts from
those sources and existing CircuitCI acceptance-fixture wiring:

- VDD operating range is modeled as `1.71 V` to `3.6 V`.
- NRST and BOOT0 are observed as external control pins.
- USART1 observation uses PA9/PA10.
- SWD observation uses PA13/SWDIO and PA14/SWCLK.

The model intentionally does not claim firmware execution, boot ROM protocol
timing, SWD transaction timing, oscillator accuracy, flash programming effects,
layout, EMC, thermal, or final signal-integrity evidence.
