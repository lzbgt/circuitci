# Abracon ABM3 Source Notes

## Retained Source

| Document | Source URL | Local copy | SHA-256 |
| --- | --- | --- | --- |
| ABM3 Ceramic SMD Crystal datasheet | <https://abracon.com/Resonators/ABM3.pdf> | `docs/research/datasheets/abracon/abm3.pdf` | `74a42fd33b5a0226229acd1657bdeb9c391711058c9dcdd391021d49506e9f23` |

Retrieved on 2026-07-05.

## Modeled Facts

- The ABM3 family covers 8.000 MHz to 54.000 MHz fundamental-mode crystals.
- The `vendor.abracon.abm3_8mhz_18pf` catalog model represents the 8.000 MHz
  option with the datasheet's standard 18 pF load capacitance.
- For the 8.000 MHz to below-9.000 MHz fundamental range, the datasheet lists
  140 ohm maximum equivalent series resistance.
- Retained electrical metadata also records 7 pF shunt capacitance, 10 uW to
  100 uW drive level, +/-50 ppm standard frequency tolerance at 25 C, +/-50 ppm
  standard frequency stability, and +/-5 ppm first-year aging.

## Model Boundary

`vendor.abracon.abm3_8mhz_18pf` is a static board-boundary model for
`CLOCK_SOURCE_VALID`. It lets the validator check that a declared MCU
oscillator has a crystal between its oscillator pins and that the two load
capacitors produce an effective load near the 18 pF target after modeled stray
capacitance.

The model does not sign off oscillator startup, negative resistance, gain
margin, motional parameters, drive-level stress, ppm clock accuracy, layout
parasitics, or phase-noise behavior.
