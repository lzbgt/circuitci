# onsemi NL27WZ17 Model

## Source

- Datasheet: `docs/research/datasheets/onsemi/nl27wz17.pdf`
- Source URL: <https://www.onsemi.com/download/data-sheet/pdf/nl27wz17-d.pdf>
- SHA-256:
  `64e38515090ece6b098f6e7f8ec9790e8cc447a4d165a9c2732312f43e736dd5`
- Retrieved: 2026-06-14

## Modeled Facts

The `vendor.onsemi.nl27wz17` model captures source-backed static metadata for a
dual non-inverting Schmitt-trigger buffer:

- Logic package family: `SC-88 / SOT-363`.
- Pinout: `1=1A`, `2=GND`, `3=2A`, `4=2Y`, `5=VCC`, `6=1Y`.
- Recommended supply range: `1.65 V` to `5.5 V`.
- Absolute maximum supply/input/output voltage entries are preserved as
  datasheet metadata.

## Validation Use

This model is useful for exact component binding and static connectivity checks
where a board uses `NL27WZ17` as a logic buffer or gate-drive evidence point.
It is not a MOSFET model and does not by itself prove output-drive waveform,
switching edge rate, or laser pulse behavior.

For TOF-R5001, the supplied schematic text contains `U5 = NL27WZ17DFT2G`, which
identifies a dual Schmitt-trigger buffer in the available design files. It is
not the replacement `Q2` MOSFET. Validating the original pre-fix drive failure
still requires the older schematic, the working replacement MOSFET identity, or
measured dynamic evidence.
