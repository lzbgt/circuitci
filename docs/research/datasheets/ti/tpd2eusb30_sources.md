# TI TPD2EUSB30 Datasheet Source

- Datasheet: `tpd2eusb30.pdf`
- Source URL: <https://www.ti.com/lit/ds/symlink/tpd2eusb30.pdf>
- SHA-256:
  `a2c0dd845043a5bbfe610f673879c29e38649544385dea51dbe0a4c49df39136`
- Retrieved: 2026-06-12

## Extracted Board-Level Facts

The saved TI datasheet identifies the DRT package pins as `D1+`, `D1-`, and
`GND`. It describes the signal pins as high-speed ESD clamp ports for
differential data lines.

The modeled static validation facts are:

- `TPD2EUSB30` reverse standoff voltage on D+/D- pins: `5.5 V`.
- IO-to-GND capacitance on DRT signal pins: `0.7 pF` typical.
- The design procedure says the two TPD2EUSB30 pins support `0 V` to `5.5 V`.
- The family is passive ESD protection and does not require a supply rail.

This research note supports only static board-level validation. It does not
claim a SPICE clamp waveform, ESD pulse model, USB eye margin, or layout
sign-off.
