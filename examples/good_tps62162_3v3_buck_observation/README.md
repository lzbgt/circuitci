# TPS62162 3.3 V Buck Observation Example

This direct-open GUI fixture exercises the source-pinned TI TPS62162 fixed
3.3 V buck-regulator metadata through generated Board IR SPICE. The component
model points to CircuitCI's reduced `CIRCUITCI_TPS62162_3V3_BUCK` macro-model
in `models/spice/generic/analog_behavioral.lib`.

The example is an observation workflow, not switch-mode regulator sign-off. It
checks a 12 V input, a 3.3 V enable drive, a preliminary 3.3 V VOS/output rail,
and a 330 ohm load. The schematic includes SW, PG, an output inductor, and
input/output capacitors for readable topology context, but the generated SPICE
observation model does not simulate switching action.

The behavioral face does not model SW switching, PG behavior, DCS-Control
dynamics, compensation, inductor ripple/current, output ripple, current limit,
startup timing, thermal behavior, layout, EMI, or loop-stability behavior.
