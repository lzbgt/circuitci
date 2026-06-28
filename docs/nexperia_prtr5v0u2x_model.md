# Nexperia PRTR5V0U2X Model

## Source

- Datasheet: `docs/research/datasheets/nexperia/prtr5v0u2x.pdf`
- Source URL:
  <https://assets.nexperia.com/documents/data-sheet/PRTR5V0U2X.pdf>
- SHA-256:
  `70105e57571142f00f389b9bf6fe5145cc4832232ec71730d818a77a655bde11`
- Retrieved: 2026-06-12

## Modeled Facts

The `vendor.nexperia.prtr5v0u2x` model captures static board-level facts for
the SOT143B package:

- pins: `GND`, `IO1`, `IO2`, and `VCC`,
- rail-to-rail ESD protection for two high-speed data lines,
- reverse standoff voltage: `5.5 V`,
- maximum I/O-to-ground capacitance: `1.5 pF`,
- typical I/O-to-I/O capacitance: `0.6 pF`,
- typical VCC-to-ground capacitance: `16 pF`.

The model encodes two `signal_conditioning.protection_clamps`:
`io1_to_vcc` and `io2_to_vcc`. They use `VCC` as the power reference so
`INTERFACE_PROTECTION_REVIEW` can catch a VCC pin tied to a non-power net.

## Validation Use

`INTERFACE_PROTECTION_REVIEW` checks that:

- the protected line and VCC reference pins are connected,
- the reference net is declared as a power rail,
- normal protected-net voltage does not exceed the `5.5 V` standoff limit,
- the `1.5 pF` line capacitance fits the declared interface budget.

## Generated SPICE Observation

The model also declares a reduced generated-SPICE subcircuit in
`models/spice/generic/analog_behavioral.lib`:

```text
CIRCUITCI_PRTR5V0U2X_USB_ESD IO1 IO2 VCC GND
```

That face is intentionally limited to normal-operation standoff and capacitance
loading. It leaves IO pins high impedance, applies `1.5 pF` IO-to-ground,
`0.6 pF` IO-to-IO, and `16 pF` VCC-to-ground capacitance, and lets generated
transient observations verify that USB line and VCC voltages remain below the
`5.5 V` standoff limit.

Executable example:

```text
examples/good_nexperia_prtr5v0u2x_usb_esd_observation/project.yaml
```

This is a preliminary screening model. It does not prove IEC ESD pulse clamp
waveforms, rail-to-rail snapback dynamics, leakage over temperature, USB eye
margin, differential impedance, return path quality, or final PCB layout
sign-off.
