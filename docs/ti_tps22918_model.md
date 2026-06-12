# TI TPS22918 Model

## Source

- Datasheet: `docs/research/datasheets/ti/tps22918.pdf`
- Source URL: <https://www.ti.com/lit/gpn/TPS22918>
- SHA-256:
  `802fae6fbe69eded2c6e389178f6fcc1b33cc672464a99935148d9bf77b3f97a`
- Retrieved: 2026-06-12

## Modeled Facts

The `vendor.ti.tps22918` model captures board-level facts needed for static
power-path validation:

- Single-channel active-high load switch.
- `VIN` recommended operating range: `1.0 V` to `5.5 V`.
- `ON` high-level input threshold: at least `1.0 V`.
- `ON` low-level input threshold: at most `0.5 V`.
- Maximum continuous switch current: `2.0 A`.
- Pins represented: `VIN`, `VOUT`, `GND`, `ON`, `CT`, and `QOD`.

`CT` and `QOD` are represented as passive pins because the current static
validator only needs them for connectivity/import preservation. Their analog
effects are intentionally not modeled by `POWER_TREE_VALID`.

## Validation Use

`POWER_TREE_VALID` uses this model through `power_switch` metadata:

- If `VOUT` is connected to a rail declared `powered: true`, the scenario must
  prove `ON` is high.
- Loads on the `VOUT` rail are summed from component-model
  `max_supply_current_A` metadata and checked against the `2.0 A` switch limit.
- `VIN` and `VOUT` rails are still checked against modeled operating voltage
  ranges.

This model is not a SPICE model and is not valid for inrush, CT-controlled
rise time, quick-output-discharge timing, reverse current, thermal sign-off, or
final load-switch transient behavior.
