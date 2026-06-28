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

`circuitci suggest-scenarios` emits one `interface_protection` template per
modeled clamp when a board connects this part and does not already declare a
matching clamp review scenario. The suggestion includes `parameters.clamp` and
`scenario.protection_clamps[]` evidence so agents can see the exact USB line,
reference net, standoff limit, and capacitance before deciding the interface
budget.

The model also declares a reduced generated-SPICE observation face:

```text
CIRCUITCI_TPD2EUSB30_USB_ESD DP DM GND
```

This face keeps each line high impedance during normal operation and adds the
datasheet typical `0.7 pF` IO-to-ground capacitance on D+ and D-. It is useful
for generated observations that verify normal D+/D- voltages stay below the
`5.5 V` reverse standoff limit while preserving a first-order line capacitance
load in the deck.

The direct-open GUI fixture lives at:

```text
examples/good_tpd2eusb30_usb_esd_observation/project.yaml
```

This remains intentionally limited. It is not valid for ESD pulse waveform
proof, dynamic clamp behavior, leakage over temperature, USB eye margin,
differential impedance, return-path quality, or final layout sign-off.
