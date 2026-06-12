# Diodes AP2112K-3.3 Regulator Model

## Sources

- Official Diodes Incorporated data sheet:
  `docs/research/datasheets/diodes/ap2112.pdf`
- Research note:
  `docs/research/datasheets/diodes/ap2112_sources.md`

## Modeled Facts

The `vendor.diodes.ap2112k_3v3` model captures static board-level facts for the
common fixed 3.3 V SOT25 LDO:

- `VIN` operating range: `2.5 V` to `6.0 V`.
- `VOUT` fixed range from `3.3 V +/-1.5%`: `3.2505 V` to `3.3495 V`.
- Maximum static dropout margin uses the datasheet `400 mV` maximum at
  `600 mA`.
- `max_output_current_A` is `0.6 A`.
- `EN` is active high, with `VIH >= 1.5 V` and `VIL <= 0.4 V`.

## Validation Use

`POWER_TREE_VALID` uses this model through `power_conversion` metadata:

- `VIN` and `VOUT` must both connect to explicit power rails.
- `VIN - VOUT` nominal margin must be at least `0.4 V`.
- Every modeled load on the output rail must declare `max_supply_current_A`,
  and the summed load must not exceed `0.6 A`.
- `VIN` and `VOUT` must each have at least `1 uF` explicit capacitance to
  ground in Board IR.
- `VOUT` rail nominal voltage must remain inside the modeled fixed-output
  tolerance range.

This model is not a SPICE regulator macromodel. It is not valid for output
capacitor ESR/ESL/DC-bias stability, thermal derating, load-transient behavior,
short-circuit foldback, startup waveform, or detailed load-dependent dropout
sign-off.
