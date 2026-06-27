# Generated DC Bias Observation Example

This example proves first-class `analog_dc` operating-point simulation from
Board IR. It models a 5 V source feeding a 10 kOhm / 10 kOhm divider, runs
ngspice `.op`, exports `operating_point.csv`, and checks that the divider bias
stays inside the declared 2.35 V to 2.65 V window.

The `divider_tolerance` run-input sweep varies `R1.value_ohm` and
`R2.value_ohm` across 9 corners. Validation reports worst-corner margin summary
rows so the limiting resistor combination is visible without manually comparing
all operating-point CSV files.
