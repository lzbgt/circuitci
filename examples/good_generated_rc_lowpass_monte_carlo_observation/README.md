# Generated RC Low-Pass Monte Carlo Observation

This fixture demonstrates deterministic Monte Carlo run inputs for generated
SPICE. The `rc_monte_carlo` sweep samples `R1.value_ohm` and `C1.value_f`
within their declared tolerances, then runs the normal AC/Bode observation once
per sample.

The generated report should include one Bode artifact per sample plus
`ANALOG_SWEEP_MARGIN_SUMMARY` rows that identify the limiting sampled corner for
each executable gain/cutoff check. It should also include
`ANALOG_MONTE_CARLO_YIELD_SUMMARY` rows with pass/fail sample counts, yield
percent, mean margin, margin standard deviation, min/max margin, and
P1/P5/P50/P95 margin percentiles for each check. The declared Monte Carlo
criteria require 100% sampled yield and at least 0.1 unit of P5 margin, so the
fixture fails if either sampled design check loses yield or tail margin.
