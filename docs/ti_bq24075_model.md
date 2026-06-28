# TI BQ24075 Model

## Source

- Datasheet:
  `docs/research/datasheets/ti/bq24074.pdf`
- Peer evidence:
  `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C15464_BQ24075RGTR.pdf`
- Source note:
  `docs/research/datasheets/ti/bq24075_sources.md`
- Retrieved: 2026-06-13

## Modeled Facts

The `vendor.ti.bq24075` model captures static board-level facts for the
BQ24075 single-cell Li-Ion charger with power path:

- `IN` operating range: `4.35 V` to `6.4 V`.
- `BAT` regulation option: `4.20 V` typical, modeled with a `4.23 V` maximum.
- `OUT` regulation range: `5.4 V` to `5.6 V`.
- RISET-programmed fast-charge current range: `150 mA` to `1.5 A`.
- RISET programming equation: `ICHG = KISET / RISET`, using the datasheet
  typical factor `KISET = 890 A*Ohm` for static checks.
- `ISET` and `ILIM` are represented as passive programming pins.
- `CE`, `EN1`, `EN2`, and `SYSOFF` are represented as digital inputs.

The Board IR component instance may declare:

```yaml
parameters:
  programmed_charge_current_A: 0.45
```

When it is omitted, CircuitCI derives the value from exactly one positive
resistor between `ISET` and `VSS`. Ambiguous or missing resistor evidence still
requires the explicit parameter.

## Validation Use

`POWER_TREE_VALID` uses the model's `battery_charger` metadata to check:

- programmed charge current is present, finite, or derivable from the
  source-backed `ISET` resistor equation,
- programmed charge current is inside the modeled charger range,
- programmed charge current does not exceed the input rail
  `supply_current_limit_A`,
- battery-net nominal voltage does not exceed the charger regulation voltage.

This is intentionally static. It does not validate battery chemistry, thermal
foldback, DPPM/supplement-mode transient behavior, charge termination, USB
enumeration, ILIM/EN-derived current-limit state, or thermal dissipation.

The model also declares `simulation.spice` metadata for generated Board IR
SPICE. The generated-SPICE face points at the reduced-fidelity
`CIRCUITCI_BQ24075_POWER_PATH_CHARGER` subcircuit in
`models/spice/generic/analog_behavioral.lib` with pin order:

`IN, BAT, OUT, VSS, ISET`

That subcircuit models the datasheet 4.2 V BAT regulation target, a preliminary
`min(IN, 5.5 V)` OUT rail source, and charge current from the external ISET
resistor equation. It lets users observe power-path charger wiring, OUT rail
voltage, battery-node voltage, and charge-current probes in Sketch/Scopes. It
intentionally omits DPPM, battery supplement mode, ILIM/EN input-current state
derivation, charge termination, CHG/PGOOD behavior, thermal regulation, timer
behavior, battery chemistry, cell safety, package dissipation, and final
charger sign-off.

`examples/good_bq24075_power_path_observation` proves the BQ24075 generated
SPICE workflow with OUT/BAT/current probes and executable checks. The GUI
Examples picker registers the same fixture as `BQ24075 Power Path`.
