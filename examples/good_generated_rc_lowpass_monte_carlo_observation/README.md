# Generated RC Low-Pass Monte Carlo Observation

This fixture demonstrates deterministic Monte Carlo run inputs for generated
SPICE. The `rc_monte_carlo` sweep samples `R1.value_ohm` and `C1.value_f`
within their declared tolerances, then runs the normal AC/Bode observation once
per sample.

The generated report should include one Bode artifact per sample plus
`ANALOG_SWEEP_MARGIN_SUMMARY` rows that identify the limiting sampled corner for
each executable gain/cutoff check.
