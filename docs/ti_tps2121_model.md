# TI TPS2121 Model

## Source

- Datasheet:
  `docs/research/datasheets/ti/tps2121.pdf`
- Peer evidence:
  `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C2156025_TPS2121RUXT.pdf`
- Source note:
  `docs/research/datasheets/ti/tps2121_sources.md`
- Retrieved: 2026-06-13

## Modeled Facts

The `vendor.ti.tps2121` model captures static board-level facts for the
TPS2121 dual-input power mux:

- `IN1` and `IN2` operating range: `2.8 V` to `22 V`.
- `OUT` operating range: `0 V` to `22 V`.
- TPS2121 continuous/input-current class: `4.5 A`.
- Datasheet high current-limit programming point: `4.5 A` typical.
- Both inputs are modeled with reverse-blocking evidence.
- `PR1`/`CP2`, `OV1`/`OV2`, `ILIM`, and `SS` are modeled as passive setup pins.

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
IN1 IN2 GND OUT ILIM
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
observation face. It does not validate switchover droop, reverse current
magnitude, ILIM resistor-derived current limits, priority divider thresholds,
soft-start timing, thermal behavior, status output behavior, or layout.

The direct-open GUI fixture lives at:

```text
examples/good_tps2121_power_mux_observation/project.yaml
```
