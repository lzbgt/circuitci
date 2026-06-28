# TI ESD2CAN24-Q1 Model

## Source

- Datasheet: `docs/research/smart_robot/sources/ti_esd2can24_q1_datasheet.pdf`
- Source URL: <https://www.ti.com/lit/ds/symlink/esd2can24-q1.pdf>
- SHA-256:
  `305b0aafdec918e96476fdf7a385cd2143e8b7664383842160c3ef1522d2bc5e`
- Retrieved: 2026-06-17
- CAN ESD application note:
  `docs/research/smart_robot/sources/ti_can_esd_overvoltage_app_note.pdf`

## Modeled Facts

The `vendor.ti.esd2can24_q1` model captures static board-level facts for the
CAN ESD diode:

- pins: `CANH`, `CANL`, and `GND`,
- two-channel ESD protection for CANH/CANL in-vehicle network lines,
- reverse standoff voltage: `+/-24 V`,
- typical I/O capacitance: `3 pF`.

The model encodes two `signal_conditioning.protection_clamps`: `canh` and
`canl`. They use `GND` as the reference so `INTERFACE_PROTECTION_REVIEW` can
catch missing or incorrectly referenced CAN ESD clamps.

## Generated SPICE Observation

The model also declares a reduced generated-SPICE subcircuit in
`models/spice/generic/analog_behavioral.lib`:

```text
CIRCUITCI_ESD2CAN24_Q1_CAN_ESD CANH CANL GND
```

That face is intentionally limited to normal-operation CAN line standoff and
capacitance loading. It leaves CANH/CANL high impedance, applies the
datasheet `3 pF` typical capacitance to ground on each line, and lets generated
transient observations verify that normal CANH/CANL voltages remain below the
`24 V` standoff limit.

Executable example:

```text
examples/good_ti_esd2can24_q1_can_esd_observation/project.yaml
```

This is a preliminary screening model. It does not prove ISO 7637, ISO 10605,
IEC ESD pulse clamp waveforms, surge energy, CAN signal integrity, cable
harness behavior, route placement, stub length, or final PCB layout sign-off.
