# TI TXS0108E Model

## Source

- Datasheet: `docs/research/datasheets/ti/txs0108e.pdf`
- Source URL: <https://www.ti.com/lit/ds/symlink/txs0108e.pdf>
- SHA-256:
  `90d330e0340a76d71856b2dc2b85a7b824b37adb5419c77f0533a20d1fc9ceb8`
- Retrieved: 2026-06-12

## Modeled Facts

The `vendor.ti.txs0108e` model captures datasheet-backed board-level metadata
needed for static validation:

- A-port supply range: `1.4 V` to `3.6 V`.
- B-port supply range: `1.65 V` to `5.5 V`.
- Operating constraint: `VCCA <= VCCB`, encoded as model supply constraint
  `vcca_lte_vccb`.
- Eight bidirectional level-shifter channels: `A1/B1` through `A8/B8`.
- The model records `unpowered_isolation: false` for each channel because this
  pack does not treat TXS0108E as an unconditional powered-to-unpowered
  isolator.
- Each channel records `enable_pin: OE` and `disabled_state: low`, so a
  powered-to-unpowered review can pass only when the scenario proves the shared
  OE control is low.
- Datasheet guidance says OE low places outputs in high impedance, and OE should
  be held low through power-up or power-down and not enabled until both supplies
  are ramped and stable.

## Validation Use

`INTERFACE_PROTECTION_REVIEW` can use this model to catch designs where one side
of the TXS0108E is powered and the other is unpowered without explicit isolation
or OE-low evidence. This is intentionally conservative. A passing board should
model the rail states, OE/reset behavior, and any timing scenario needed to prove
that the device is high impedance before relying on the level shifter for
protection.

The same check enforces `vcca_lte_vccb` whenever both VCCA and VCCB rails are
powered. For example, a board that connects VCCA to a `5.0 V` rail and VCCB to a
`3.3 V` rail fails with measured `lower_nominal_voltage_V > upper_nominal_voltage_V`.

`suggest-scenarios` can emit runnable `INTERFACE_PROTECTION_REVIEW` templates
for TXS0108E channels because this model is datasheet-backed and carries the
required direction, supply, isolation, OE, and supply-order metadata. When OE is
directly tied to ground, the suggestion includes an `OE` low pin-state entry.

The model is not a SPICE model and is not valid for high-speed signal-integrity
or analog edge-rate sign-off.
