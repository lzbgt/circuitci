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
- Operating constraint: `VCCA <= VCCB`.
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

The model is not a SPICE model and is not valid for high-speed signal-integrity
or analog edge-rate sign-off.
