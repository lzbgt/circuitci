# STM32L431 Boot/UART/SWD Observation

This fixture exercises the source-backed `vendor.st.stm32l431vct6` model
through CircuitCI's generated-SPICE path. Its `simulation.spice` face points to
CircuitCI's reduced STM32L431 boot/UART/SWD macro-model in
`models/spice/generic/analog_behavioral.lib`.

The scenario checks the VDD rail, NRST released high, BOOT0 application-boot
low, USART1 PA9/PA10 idle high states, and SWD PA13/SWDIO high plus PA14/SWCLK
idle low states. It is a board-boundary observation aid, not firmware, ROM
bootloader timing, SWD transaction, oscillator, flash-programming, package,
layout, thermal, EMC, or SI sign-off.
