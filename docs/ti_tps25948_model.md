# TI TPS25948 Model

## Source

- Product page: `docs/research/smart_robot/sources/tps25948_product.html`
- Datasheet: `docs/research/smart_robot/sources/tps25948_datasheet.pdf`
- Product URL: <https://www.ti.com/product/TPS25948>
- Datasheet URL: <https://www.ti.com/lit/ds/symlink/tps25948.pdf>
- Product-page SHA-256:
  `b4b10fa77d3e8963eb4b24edb2f8ba3561de2201f41beef6e382e34ae6478bc4`
- Datasheet SHA-256:
  `7259b8c694b939b0cdc05821eb4aa8772ca69213635bc2f139ac5d037c648584`
- Retrieved: 2026-06-17

## Modeled Facts

The `vendor.ti.tps25948_8a_rcb_dvdt` model captures board-level facts needed
for static power-path validation:

- eFuse/load-switch path with integrated back-to-back FETs.
- `VIN` recommended operating range: `3.5 V` to `23.0 V`.
- Active-high `EN/UVLO` control represented as model pin `EN`.
- `EN/UVLO` rising threshold represented from the datasheet `1.17 V` to
  `1.23 V` range.
- Continuous switch current: `8.0 A`.
- Current-limit setting represented as `8.0 A` for the modeled RILM case.
- Maximum on-resistance represented as `20 mOhm`.
- Reverse-current-blocking mode represented as `always`.
- Pins represented: `VIN`, `VOUT`, `EN`, `GND`, `ILM`, `DVDT`, `ITIMER`,
  `FLT`, `SPLYGD`, `OVLO`, and `RCBCTRL`.

`ILM`, `DVDT`, `ITIMER`, `FLT`, `SPLYGD`, `OVLO`, and `RCBCTRL` are retained
for source-pinned component metadata and schematic/import preservation. The
static validators use `power_switch` metadata for output current, inrush, and
reverse-current-blocking checks when scenarios declare those requirements.

## Generated SPICE Use

The model declares `simulation.spice` metadata for generated Board IR SPICE.
The generated-SPICE face points at the reduced-fidelity
`CIRCUITCI_TPS25948_EFUSE_RCB` subcircuit in
`models/spice/generic/analog_behavioral.lib` with pin order:

`VIN, VOUT, GND, EN`

That subcircuit models an active-high smooth conductance near the datasheet
maximum `20 mOhm` on-resistance so users can observe enabled eFuse/load-switch
wiring, protected-rail voltage, and load current. It intentionally omits dVdt
slew shaping, ILM/ITIMER current-limit behavior, fault timers, FLT/SPLYGD
status outputs, OVLO behavior, RCBCTRL control, reverse-current dynamics,
thermal shutdown, inrush accuracy, and final protection sign-off.

`examples/good_tps25948_efuse_observation` proves the generated-SPICE workflow
with voltage/current probes and executable checks. The GUI Examples picker
registers the same fixture as `TPS25948 eFuse`.
