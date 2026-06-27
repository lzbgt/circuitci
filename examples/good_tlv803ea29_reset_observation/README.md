# TLV803EA29 Reset Supervisor Observation Example

This fixture exercises a datasheet-backed reset-supervisor component in
generated Board IR SPICE. `vendor.ti.tlv803ea29` keeps TI TLV803EA29 threshold,
active-low open-drain topology, delay, and operating-range metadata, while its
`simulation.spice` face points to a reduced behavioral threshold model in
`models/spice/generic/analog_behavioral.lib`.

The circuit ramps a 3.3 V rail from 0 V to 3.3 V, pulls `RESET` up through
10 kOhm, and lets the TLV803EA29 behavioral open-drain output hold reset low
until `VDD` crosses the nominal 2.93 V threshold. The analog run setup checks
that reset is low before the rail rises, high after the rail is valid, and has
crossed mid-rail by `20 us`.

This example is useful for generated-SPICE model-pack, probe, assertion, and
schematic observation workflows. It is not a reset-release delay, hysteresis,
glitch-immunity, low-VDD output-validity, leakage, or pull-up RC timing
sign-off model.

Sources:

- `docs/research/datasheets/ti/tlv803e-tlv809e-tlv810e.pdf`
- `docs/research/datasheets/ti/tlv803e_sources.md`
- `docs/ti_tlv803ea29_model.md`
