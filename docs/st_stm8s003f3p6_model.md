# ST STM8S003F3P6 Model

## Source

- Datasheet:
  `docs/research/datasheets/st/stm8s003f3_datasheet.pdf`
- Source note:
  `docs/research/datasheets/st/stm8s003f3_sources.md`
- Source URL:
  `https://www.st.com/resource/en/datasheet/stm8s003f3.pdf`
- Retrieved: 2026-07-05

## Modeled Facts

The `vendor.st.stm8s003f3p6` model captures the first source-backed static
board-boundary checks for STM8S003F3P6 designs:

- `VDD`: `2.95 V` to `5.5 V`.
- `VSS`: ground.
- `VCAP`: required internal regulator capacitor pin boundary.
- `NRST`: active-low reset boundary.
- `SWIM`: single-wire programming/debug boundary on `PD1`.
- `UART1_TX` and `UART1_RX`: board-facing UART1 pins on `PD5` and `PD6`.

## Validation Use

`POWER_TREE_VALID` screens STM8S003F3P6 supply voltage against the
source-backed recommended operating range. The passing public fixture is:

- `examples/good_st_stm8s003f3p6_power/project.yaml`

The paired injected-error fixture is:

- `examples/bad_st_stm8s003f3p6_vdd_overvoltage/project.yaml`

## Limits

This model does not validate `VCAP` capacitance/ESR/ESL, formula-based GPIO or
NRST thresholds, oscillator startup/accuracy, SWIM protocol timing, UART
bootloader/protocol behavior, flash/EEPROM programming behavior, firmware
execution, thermal sign-off, or transient current waveforms. Those require
separate source evidence and rules.
