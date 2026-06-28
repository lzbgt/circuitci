# onsemi NL27WZ17 Model

`vendor.onsemi.nl27wz17` is a source-backed onsemi NL27WZ17 dual
non-inverting Schmitt-trigger buffer model for preliminary board-facing
line-state observation checks.

Saved source material:

- Datasheet: `docs/research/datasheets/onsemi/nl27wz17.pdf`
- Datasheet URL: <https://www.onsemi.com/download/data-sheet/pdf/nl27wz17-d.pdf>
- Datasheet SHA-256:
  `64e38515090ece6b098f6e7f8ec9790e8cc447a4d165a9c2732312f43e736dd5`
- Retrieved: `2026-06-14`

The static metadata captures the 1.65 V to 5.5 V `VCC` operating range,
SC-88/SOT-363 pinout, and dual non-inverting Schmitt-trigger buffer role.

The generated-SPICE face is `CIRCUITCI_NL27WZ17_DUAL_BUFFER` in
`models/spice/generic/analog_behavioral.lib`. It maps optional Board IR
component parameters into explicit instance parameters:

- `observation_1a_state` to `A1_STATE`
- `observation_2a_state` to `A2_STATE`

`examples/good_onsemi_nl27wz17_logic_buffer_observation` checks `VCC`, a high
`1A` mirrored to `1Y`, and a low `2A` mirrored to `2Y` in a direct-open GUI
example. Model-aware `Create Checks` presets also add VCC-window and
input/output line-state assertions for connected NL27WZ17 channels.

The generated-SPICE face deliberately does not model Schmitt thresholds,
hysteresis, propagation delay, rise/fall time, capacitive loading, output drive
current, package parasitics, signal integrity, or final switching sign-off.

## TOF-R5001 Validation Context

This model remains useful for exact component binding and static connectivity
checks where a board uses `NL27WZ17` as a logic buffer or gate-drive evidence
point. It is not a MOSFET model and does not by itself prove output-drive
waveform, switching edge rate, or laser pulse behavior.

For TOF-R5001, the supplied schematic text contains `U5 = NL27WZ17DFT2G`, which
identifies a dual Schmitt-trigger buffer in the available design files. It is
not the replacement `Q2` MOSFET. Validating the original pre-fix drive failure
still requires the older schematic, the working replacement MOSFET identity, or
measured dynamic evidence.
