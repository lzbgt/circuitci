# TI TLV803EA29 Reset Supervisor Model

## Sources

- Official Texas Instruments data sheet:
  `docs/research/datasheets/ti/tlv803e-tlv809e-tlv810e.pdf`
- Research note:
  `docs/research/datasheets/ti/tlv803e_sources.md`

## Modeled Facts

The `vendor.ti.tlv803ea29` model captures static board-level facts for the
active-low open-drain `2.93 V` threshold variant:

- `VDD` operating range is `1.7 V` to `6.0 V`.
- `VDD` supply current is modeled as `1 uA` maximum for static rail-budget
  screening at the datasheet `3.3 V` condition.
- Falling reset threshold is modeled from the `2.93 V` option and `+/-2%`
  accuracy:
  - `threshold_min_V = 2.8714`
  - `threshold_max_V = 2.9886`
- Delay option `A` is modeled as `reset_release_delay_us = 270000`, the
  datasheet maximum for the `130/200/270 ms` release-time range.
- `RESET` is active-low and open-drain. The model records this in datasheet
  metadata, but the current port-kind abstraction represents the output as a
  digital output.

## Validation Use

`POWER_TREE_VALID` can now catch two common reset-supervisor mistakes:

- using the `A29` threshold on a rail whose nominal voltage does not exceed the
  worst-case high threshold, and
- using a supervisor whose worst-case low threshold can release reset below a
  powered load's minimum operating voltage.

The model can also provide conservative delay metadata to generated
`RESET_RELEASE_AFTER_POWER_VALID` and UART bootloader timing suggestions when
the supervisor uniquely monitors the target rail and drives the target reset
net. See `examples/scenario_suggestions_tlv803_reset_release/`.

This model is not a reset waveform model. It is not valid for open-drain pull-up
timing, glitch immunity, transient noise immunity, propagation-delay sign-off,
or final low-voltage reset-output behavior.
