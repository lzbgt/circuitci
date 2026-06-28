# TPS63802 3.3 V Buck-Boost Observation Example

This direct-open GUI fixture exercises the source-pinned TI TPS63802 3.3 V
buck-boost metadata through generated Board IR SPICE. The component model
points to CircuitCI's reduced `CIRCUITCI_TPS63802_3V3_BUCK_BOOST` macro-model
in `models/spice/generic/analog_behavioral.lib`.

The example is an observation workflow, not buck-boost regulator sign-off. It
checks a 3.7 V Li-Ion-style input, a 3.3 V enable drive, a preliminary 3.3 V
VOUT rail, and a 220 ohm load. The schematic includes L1/L2, FB, MODE, PG, and
support capacitors for readable topology context, but the generated SPICE
observation model does not simulate buck-boost switching action.

The behavioral face does not model L1/L2 switching, FB-loop dynamics, MODE/PG
behavior, inductor ripple/current, output ripple, startup from depleted cells,
current limit, thermal behavior, layout, EMI, or loop-stability behavior.
