# TI TPS2115A Model

## Source

- Datasheet: `docs/research/datasheets/ti/tps2115a.pdf`
- Source URL: <https://www.ti.com/lit/ds/symlink/tps2115a.pdf>
- SHA-256:
  `479e2901a83785b0ee1ebf925423c47fffcea0975a46e35172cc160f02d3a231`
- Retrieved: 2026-06-12

## Modeled Facts

The `vendor.ti.tps2115a` model captures static board-level facts for the
TPS2115A autoswitching power multiplexer:

- Two input power pins, `IN1` and `IN2`, and one output power pin, `OUT`.
- Input supply operating range: `2.8 V` to `5.5 V`.
- Output rail modeled up to `5.5 V`.
- Reverse-conduction and cross-conduction blocking are represented as
  `reverse_blocking: true` on both input paths.
- Static output-current screen uses `max_output_current_A: 1.0`.
- `EN`, `D0`, and `D1` are represented as digital input pins for connectivity.
- `VSNS` and `ILIM` are preserved for connectivity/import; this first model
  does not derive the current limit from the ILIM resistor.

The Board IR component instance must declare which input is selected in a given
power-tree scenario:

```yaml
parameters:
  selected_input: in1
```

Use separate scenarios for USB-selected, battery-selected, and switchover
states.

## Validation Use

`POWER_TREE_VALID` uses this model through `power_mux` metadata:

- the selected input must be declared and connected to a powered rail when
  `OUT` is powered,
- inactive unpowered inputs pass because the datasheet-backed model declares
  reverse-blocking behavior,
- loads on `OUT` are summed from component-model `max_supply_current_A`
  metadata and checked against the `1.0 A` static mux limit,
- `IN1`, `IN2`, and `OUT` are still checked against modeled operating voltage
  ranges.

This is not a SPICE or transient switchover model. It is not valid for
ILIM-resistor current-limit sign-off, reverse-current magnitude, switchover
droop, inrush, thermal sign-off, or source-priority timing.
