# BQ24075 Power-Path Charger Observation Example

This fixture exercises a datasheet-backed TI BQ24075 single-cell charger with
power path in generated Board IR SPICE. The vendor model keeps BQ24075
datasheet pinout, ISET resistor charge-current equation, 4.2 V BAT regulation
target, 5.5 V OUT target, and static `battery_charger` limits, while its
`simulation.spice` face points to CircuitCI's reduced power-path charger
macro-model.

The circuit drives `IN` from a 6 V adapter source, programs fast-charge current
with a 1.98 kOhm `RISET`, loads `OUT` with 5.5 kOhm, inserts a zero-volt
`VSENSE` source for charge-current measurement, and charges a small 1 uF
battery-node capacitor. The run setup checks that `OUT` stays in the 5.4 V to
5.6 V datasheet window, charge current is near 450 mA, `BAT` rises past 4 V,
and the battery node remains below the regulation ceiling.

This is useful for generated-SPICE power-path charger wiring, ISET resistor,
OUT/BAT/current probe, assertion, and GUI model-pack workflows. It is not a
BQ24075 DPPM, supplement-mode, ILIM/EN current-limit, charge-termination,
CHG/PGOOD, timer, thermal, battery-chemistry, cell-safety,
package-dissipation, or final charger sign-off model.

The fixture is registered in the GUI Examples picker as `BQ24075 Power Path`.
Opening it lands in Sketch with the routed schematic, and `Create Checks`
regenerates a model-aware observation preset for `UCHG`.

Sources:

- `docs/research/datasheets/ti/bq24074.pdf`
- `docs/research/datasheets/ti/bq24075_sources.md`
- `docs/ti_bq24075_model.md`
