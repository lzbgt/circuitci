# TI TPS2115A Model

## Source

- Datasheet: `docs/research/datasheets/ti/tps2115a.pdf`
- Datasheet URL: <https://www.ti.com/lit/ds/symlink/tps2115a.pdf>
- Datasheet SHA-256:
  `479e2901a83785b0ee1ebf925423c47fffcea0975a46e35172cc160f02d3a231`
- Retrieved: 2026-06-12

## Modeled Facts

The `vendor.ti.tps2115a` model captures static board-level facts for the
TPS2115A dual-input autoswitching power mux:

- `IN1` and `IN2` operating range: `2.8 V` to `5.5 V`.
- `OUT` operating range: `0 V` to `5.5 V`.
- Family-level output-current class: `1.0 A`.
- Both inputs are modeled with reverse-conduction and cross-conduction blocking
  evidence.
- `EN`, `D0`, and `D1` are represented as required digital control pins.
- `ILIM` is represented as a required passive setup pin because current limit
  is resistor programmable, even though this first model does not derive the
  numeric current limit from the resistor.

The Board IR component instance must declare:

```yaml
parameters:
  selected_input: in1
```

That value represents the board's intended static source-selection state for
`POWER_TREE_VALID`.

The model also declares `simulation.spice` metadata for generated Board IR
observation. Its SPICE face points to
`models/spice/generic/analog_behavioral.lib` with pin order:

```text
IN1 IN2 GND OUT EN D0 D1 ILIM
```

The generated subcircuit instance can map a numeric Board IR component
parameter into the selected source:

```yaml
instance_parameters:
  - spice_name: SELECT_INPUT
    component_parameter: observation_selected_input_index
    default_value: 1.0
```

`observation_selected_input_index: 1.0` selects `IN1`; `2.0` selects `IN2`.
This numeric setting is only for generated-SPICE observation. The string
`selected_input` parameter remains the static power-tree contract.

## Validation Use

`POWER_TREE_VALID` uses the model's `power_mux` metadata to check:

- selected input exists and is powered,
- inactive unpowered inputs have reverse-blocking evidence,
- output load current does not exceed the modeled mux current capability,
- input/output rail voltages are within modeled port ranges.

This is a static power-mux screen plus a reduced generated-SPICE selected-source
observation face. It does not validate EN/D0/D1/VSNS autoswitch truth-table
behavior, switchover droop, reverse-current magnitude, ILIM resistor-derived
current limits, thermal behavior, package limits, or layout.

The direct-open GUI fixture lives at:

```text
examples/good_tps2115a_power_mux_observation/project.yaml
```
