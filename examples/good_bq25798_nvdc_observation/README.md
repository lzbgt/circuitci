# BQ25798 NVDC Observation Example

This direct-open GUI fixture exercises the source-pinned TI BQ25798 metadata
through generated Board IR SPICE. The component model points to CircuitCI's
reduced `CIRCUITCI_BQ25798_NVDC_CHARGER` macro-model in
`models/spice/generic/analog_behavioral.lib`.

The example is intentionally an observation workflow, not charger sign-off. It
checks a 20 V adapter input, a host-declared 2 A charge-current setting, a
12 V preliminary SYS rail target, a 24 ohm system load, and the early rise of a
1 mF battery-node capacitor from an 11 V initial condition.

The behavioral face does not model buck-boost switching, DPM or MPPT control,
BATFET supplement mode, I2C register sequencing, thermal regulation, charge
termination, battery chemistry, safety timers, or cell-pack protection.
