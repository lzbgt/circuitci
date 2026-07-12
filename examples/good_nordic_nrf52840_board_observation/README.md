# nRF52840 Board Observation

This direct-open fixture exercises the reduced generated-SPICE face for the
Nordic nRF52840 MCU/wireless SoC.

The fixture models a normal-voltage board boundary with 3.3 V `VDD`, 3.3 V
`VDDH`, 5 V USB `VBUS`, active-low reset released by a pull-up, SWD idle
states, UART-capable GPIO idle states, USB D+/D- weak idle bias, and a 50 ohm
antenna-feed boundary.

The reduced model is intentionally high impedance. It validates observable
board-level rail and line-state assumptions only; it does not emulate firmware,
GPIO drive, SWD or USB transactions, RF behavior, DCDC networks, reset UICR
configuration, thermal behavior, or transient current waveforms.
