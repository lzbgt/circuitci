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

The model also declares `simulation.spice` metadata for generated Board IR
transient workflows. That simulation face uses the shared
`CIRCUITCI_IDEAL_LDO_3V3` subcircuit from
`models/spice/generic/analog_behavioral.lib` with AP2112K pin order:

```text
VIN, EN, GND, VOUT
```

`examples/good_ap2112k_3v3_ldo_observation` proves the AP2112K model can
participate in generated-SPICE observation, with SHA-pinned model-file evidence,
voltage probes, current probes, and executable rail/load checks.

This simulation face is still a conservative workflow/topology macro-model. It
is not valid for output capacitor ESR/ESL/DC-bias stability, thermal derating,
load-transient behavior, current limit, short-circuit foldback, PSRR, noise,
startup waveform, or detailed load-dependent dropout sign-off.
