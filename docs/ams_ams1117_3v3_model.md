# AMS AMS1117-3.3 Regulator Model

## Sources

- Official Advanced Monolithic Systems data sheet:
  `docs/research/datasheets/ams/ams1117.pdf`
- Research note:
  `docs/research/datasheets/ams/ams1117_sources.md`

## Modeled Facts

The `vendor.ams.ams1117_3v3` model captures static board-level facts for the
common fixed 3.3 V 1117-style LDO:

- `VIN` is bounded by the `15 V` absolute maximum, with static dropout checked
  separately.
- `VOUT` fixed range from the data sheet table: `3.201 V` to `3.399 V`.
- Static dropout margin uses the data sheet `1.3 V` maximum at `0.8 A`.
- `min_output_current_A` is `0.01 A`, from the data sheet minimum load current
  for guaranteed regulation.
- `max_output_current_A` is `0.8 A`, matching the listed line/load/dropout
  regulation range rather than the separate current-limit value.
- `VOUT` must have at least `22 uF` explicit capacitance to ground.

## Validation Use

`POWER_TREE_VALID` uses this model through `power_conversion` metadata:

- `VIN` and `VOUT` must both connect to explicit power rails.
- `VIN - VOUT` nominal margin must be at least `1.3 V`.
- Always-on loads on `VOUT` must prove at least `10 mA` minimum load with
  `min_supply_current_A` metadata.
- Every modeled load on the output rail must declare `max_supply_current_A`,
  and the summed load must not exceed `0.8 A`.
- `VOUT` must have at least `22 uF` explicit capacitance to ground in Board IR.
- `VOUT` rail nominal voltage must remain inside the modeled fixed-output
  tolerance range.

This model is not a SPICE regulator macromodel. It checks declared minimum-load
evidence, but it is not valid for output capacitor ESR/material stability
sign-off, thermal derating, load-transient behavior, short-circuit foldback,
startup waveform, or detailed load-dependent dropout sign-off.
