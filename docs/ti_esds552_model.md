# TI ESDS552 Model

## Source

- Product page: `docs/research/smart_robot/sources/esds552_product.html`
- Datasheet: `docs/research/smart_robot/sources/esds552_datasheet.pdf`
- Datasheet source URL: <https://www.ti.com/lit/gpn/ESDS552>
- Datasheet SHA-256:
  `3ce62d5dbb2b1637cb8591437db132fd4d1771059051ee0ab58aa2d0332fc936`
- Retrieved: 2026-06-17

## Modeled Facts

The `vendor.ti.esds552` model captures static board-level facts for the
RS-485/RS-422 protection diode:

- pins: `A`, `B`, and `GND`,
- two-channel bidirectional ESD and surge protection for RS-485/RS-422 lines,
- reverse standoff voltage: `+/-12 V`,
- I/O-to-ground line capacitance: `9.5 pF` typical, `11 pF` maximum.

The model encodes two `signal_conditioning.protection_clamps`: `a` and `b`.
They use `GND` as the reference so `INTERFACE_PROTECTION_REVIEW` can catch
missing or incorrectly referenced RS-485/RS-422 protection clamps.

## Generated SPICE Observation

The model also declares a reduced generated-SPICE subcircuit in
`models/spice/generic/analog_behavioral.lib`:

```text
CIRCUITCI_ESDS552_RS485_ESD A B GND
```

That face is intentionally limited to normal-operation line standoff and
capacitance loading. It leaves A/B high impedance, applies the datasheet
`11 pF` maximum capacitance to ground on each line, and lets generated
transient observations verify that normal RS-485/RS-422 A/B voltages remain
below the `12 V` standoff limit.

Executable example:

```text
examples/good_ti_esds552_rs485_esd_observation/project.yaml
```

This is a preliminary screening model. It does not prove IEC 61000-4-2,
IEC 61000-4-5, ESD/surge pulse clamp waveforms, surge response, RS-485
common-mode stress, bus termination, cable-harness behavior, signal integrity,
route placement, stub length, or final PCB layout sign-off.
