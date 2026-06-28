# TPS61023 5 V Boost Observation Example

This direct-open GUI fixture exercises the source-pinned TI TPS61023 5 V boost
metadata through generated Board IR SPICE. The component model points to
CircuitCI's reduced `CIRCUITCI_TPS61023_5V_BOOST` macro-model in
`models/spice/generic/analog_behavioral.lib`.

The example is an observation workflow, not boost regulator sign-off. It checks
a 3.7 V Li-Ion-style input, a 3.3 V enable drive, a preliminary 5 V VOUT rail,
and a 100 ohm load. The schematic includes SW, FB, input/output capacitors, and
the input boost inductor for readable topology context, but the generated SPICE
observation model does not simulate switching action.

The behavioral face does not model SW switching, FB-loop dynamics, inductor
ripple/current, output ripple, valley current-limit behavior, startup from
deeply depleted cells, thermal behavior, layout, EMI, or loop-stability
behavior.
