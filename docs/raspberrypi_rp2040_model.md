# Raspberry Pi RP2040 Model

## Source

- Datasheet:
  `docs/research/datasheets/raspberrypi/rp2040-datasheet.pdf`
- Hardware design guide:
  `docs/research/datasheets/raspberrypi/hardware-design-with-rp2040.pdf`
- Source note:
  `docs/research/datasheets/raspberrypi/rp2040_sources.md`
- Retrieved: 2026-07-05

## Modeled Facts

The `vendor.raspberrypi.rp2040` model captures the first static board-boundary
checks for RP2040 designs:

- `IOVDD` and `VREG_VIN`: `1.62 V` to `3.63 V`.
- `DVDD`: `1.05 V` to `1.16 V`.
- `USB_VDD`: `3.135 V` to `3.63 V`.
- `ADC_AVDD`: `1.62 V` to `3.63 V`.
- `VREG_VOUT` is modeled as the internal `1.1 V`, `100 mA` regulator output
  that commonly feeds `DVDD` off-chip.
- 3.3 V GPIO thresholds: `2.0 V` high and `0.8 V` low.
- 3.3 V GPIO output-high floor: `2.62 V`.
- `RUN` is active-low reset.
- `external_flash` boot requires `QSPI_SS` high.
- `bootsel_usb` boot requires `QSPI_SS` low.

## Validation Use

`POWER_TREE_VALID` screens RP2040 supply voltages and the internal regulator's
`100 mA` output-current budget when `DVDD` is declared as a load on the
`VREG_VOUT` rail. `BOOT_STRAP_BIAS_VALID` screens resistor-biased `QSPI_SS`
BOOTSEL straps against the modeled 3.3 V GPIO thresholds. `BOOT_STRAP_DEFINED`
can also be used when a scenario supplies explicit observed strap states.

The passing public fixture is:

- `examples/good_raspberrypi_rp2040_bootsel_power/project.yaml`

The paired injected-error fixture is:

- `examples/bad_raspberrypi_rp2040_iovdd_overvoltage/project.yaml`

## Limits

This model is not valid for USB signal integrity, crystal oscillator
startup/accuracy, QSPI flash protocol timing, BOOTROM USB protocol behavior,
firmware execution, thermal sign-off, transient-current waveform shape, or
layout/EMC sign-off. Those require separate rules or simulation evidence.
