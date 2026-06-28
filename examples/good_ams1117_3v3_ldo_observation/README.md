# AMS1117-3.3 LDO Observation

This direct-open GUI example proves the source-backed AMS1117-3.3 LDO can be
used in generated SPICE observations.

The fixture uses a 5 V input, the reduced AMS1117 fixed 3.3 V observation
model, a 22 uF output capacitor, and a 330 ohm load near the datasheet minimum
load requirement. It is intended for preliminary rail and load-wiring evidence,
not loop stability, output-capacitor ESR/material sign-off, thermal behavior,
PSRR, noise, current-limit behavior, or startup timing sign-off.
