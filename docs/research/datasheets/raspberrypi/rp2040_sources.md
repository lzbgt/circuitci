# RP2040 Source Notes

Retrieved on 2026-07-05 from official Raspberry Pi documentation endpoints.

## Original Documents

| Document | Source URL | Local file | SHA-256 |
| --- | --- | --- | --- |
| RP2040 Datasheet | <https://datasheets.raspberrypi.com/rp2040/rp2040-datasheet.pdf> | `docs/research/datasheets/raspberrypi/rp2040-datasheet.pdf` | `be56fbb75ba0ae9e26558a73c93ac3e75c2ad4e6878d3b6703de2a76d886ea8c` |
| Hardware design with RP2040 | <https://datasheets.raspberrypi.com/rp2040/hardware-design-with-rp2040.pdf> | `docs/research/datasheets/raspberrypi/hardware-design-with-rp2040.pdf` | `6523a2ebf743fcfbcc69bc3901b24ca3a2d23617d8ee50beb5f606b31134f0b5` |

## Modeled Facts

- `IOVDD` and `VREG_VIN` operating ranges are `1.62 V` to `3.63 V`.
- `DVDD` operating range is `1.05 V` to `1.16 V`.
- `USB_VDD` operating range is `3.135 V` to `3.63 V`.
- `ADC_AVDD` operating range is `1.62 V` to `3.63 V`; ADC performance is
  compromised below `2.97 V`, but that precision limit is not yet a board-rule
  contract.
- The internal regulator output `VREG_VOUT` is nominally `1.1 V` with a
  `100 mA` maximum output-current rating and is connected off-chip to `DVDD`
  in the common single-supply design.
- At `IOVDD = 3.3 V`, GPIO input thresholds are modeled as `VIH >= 2.0 V` and
  `VIL <= 0.8 V`; output-high voltage is modeled as `VOH >= 2.62 V`.
- Maximum total IOVDD current sourced by GPIO and QSPI pins is `50 mA`.
- `RUN` is the global asynchronous active-low reset pin.
- External-flash boot keeps `QSPI_SS` high. Pulling `QSPI_SS` low during reset
  selects BOOTSEL mode, where RP2040 presents as a USB mass-storage device for
  UF2 loading.

## Non-Modeled Facts

The first model does not sign off USB differential routing, crystal oscillator
startup/accuracy, QSPI flash timing, BOOTROM USB protocol behavior, firmware
execution, thermal behavior, or transient current waveform shape.
