# Advanced Monolithic Systems AMS1117 Sources

Retrieved on 2026-06-12.

## Primary Source

- Official Advanced Monolithic Systems data sheet:
  <http://www.advanced-monolithic.com/pdf/ds1117.pdf>
- Local copy:
  `docs/research/datasheets/ams/ams1117.pdf`
- SHA-256:
  `189a2651878a87d590b768eaa9b44217a3fdf460352ce6ecaff127221282a3f0`

## Extracted Facts Used In `vendor.ams.ams1117_3v3`

- AMS1117 is a 1 A low-dropout regulator family with fixed `3.3 V` ordering
  options.
- Absolute maximum input voltage is `15 V`.
- The 3.3 V fixed output table gives:
  - `3.201 V` minimum over the full listed range,
  - `3.300 V` typical,
  - `3.399 V` maximum over the full listed range.
- Dropout voltage is `1.1 V` typical and `1.3 V` maximum at `0.8 A`.
- The data sheet notes dropout is specified up to `0.8 A`; for currents over
  `0.8 A`, dropout is higher.
- Current limit for fixed variants is listed as `900 mA` minimum,
  `1100 mA` typical, and `1500 mA` maximum at `(VIN - VOUT) = 1.5 V`.
- The model uses `0.8 A` as `max_output_current_A` because line/load/dropout
  regulation conditions are explicitly bounded to `0.8 A`.
- The stability section states that `22 uF` solid tantalum on the output
  ensures stability for all operating conditions.
- The data sheet defines `10 mA` minimum load current for guaranteed
  regulation, but CircuitCI does not yet have minimum-load evidence in Board IR,
  so this is recorded as datasheet metadata only.

## Modeling Notes

The model is a static board-level regulator screen. `POWER_TREE_VALID` can use
it to catch wrong rail voltage, insufficient nominal dropout margin, excessive
declared output load current, and missing/undersized explicit output support
capacitance. It does not sign off minimum-load regulation, thermal derating,
output capacitor ESR/material choice, load-transient behavior, startup waveform,
or short-circuit behavior.
