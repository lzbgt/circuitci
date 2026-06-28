# TPS54331 5 V Buck Observation Example

This direct-open GUI fixture exercises the source-pinned TI TPS54331 5 V
regulator metadata through generated Board IR SPICE. The component model points
to CircuitCI's reduced `CIRCUITCI_TPS54331_5V_BUCK` macro-model in
`models/spice/generic/analog_behavioral.lib`.

The example is intentionally an observation workflow, not switch-mode regulator
sign-off. It checks a 12 V input, a 3.3 V enable drive, a preliminary 5 V
VSENSE/output rail, and a 50 ohm load. The schematic includes PH, BOOT, an
output inductor, and a bootstrap capacitor for readable topology context, but
the generated SPICE observation model does not simulate switching action.

The behavioral face does not model PH/BOOT switching, compensation, inductor
ripple/current, output ripple, current limit, Eco-mode, startup timing,
thermal behavior, layout, EMI, or loop-stability behavior.
