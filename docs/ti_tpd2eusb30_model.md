# TI TPD2EUSB30 Model

## Source

- Datasheet: `docs/research/datasheets/ti/tpd2eusb30.pdf`
- Source URL: <https://www.ti.com/lit/ds/symlink/tpd2eusb30.pdf>
- SHA-256:
  `a2c0dd845043a5bbfe610f673879c29e38649544385dea51dbe0a4c49df39136`
- Retrieved: 2026-06-12

## Modeled Facts

The `vendor.ti.tpd2eusb30` model captures static board-level facts for the DRT
3-pin package:

- Signal pins: `D1+` and `D1-`.
- Reference pin: `GND`.
- Passive ESD protection; no supply rail is required.
- Reverse standoff voltage for TPD2EUSB30 signal pins: `5.5 V`.
- Typical IO-to-GND capacitance for DRT signal pins: `0.7 pF`.

These facts are encoded as two `signal_conditioning.protection_clamps`:
`d1_plus` and `d1_minus`.

## Validation Use

`INTERFACE_PROTECTION_REVIEW` uses this model with `parameters.clamp` to check:

- each protected line is referenced to a declared ground net,
- normal protected-net voltage does not exceed the `5.5 V` standoff limit,
- the `0.7 pF` line capacitance fits the scenario interface budget.

This model is not a SPICE clamp model. It is not valid for ESD pulse waveform
proof, USB eye margin, differential impedance, return-path quality, or final
layout sign-off.
