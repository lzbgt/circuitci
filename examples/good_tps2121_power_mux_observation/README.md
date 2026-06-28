# TPS2121 Power Mux Observation Example

This direct-open GUI fixture exercises the source-pinned TI TPS2121 power-mux
metadata through generated Board IR SPICE. The component model points to
CircuitCI's reduced `CIRCUITCI_TPS2121_POWER_MUX` macro-model in
`models/spice/generic/analog_behavioral.lib`.

The example is an observation workflow, not power-mux switchover sign-off. It
checks a 5 V USB-selected input, an inactive backup input, a preliminary 5 V
OUT rail, and a 100 ohm load. The component instance keeps the static
`selected_input: in1` metadata for power-tree validation and sets
`observation_selected_input_index: 1.0` for the reduced generated-SPICE face.

The behavioral face does not model priority threshold comparators, switchover
droop, reverse-current magnitude, ILIM resistor-derived current limit,
soft-start timing, thermal behavior, status output, or layout behavior.
