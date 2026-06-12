# KiCad TLV803 Reset Supervisor Suggestion Fixture

`examples/import_kicad_tlv803_reset_supervisor_suggestions/` proves that a
native KiCad schematic can bind a reset supervisor symbol to the datasheet-backed
`vendor.ti.tlv803ea29` model and carry that evidence into automatic scenario
suggestions.

The fixture contains:

- `U1`, a generic MCU with `VDD`, `GND`, and `NRST`.
- `USUP`, a TLV803EA29 reset supervisor monitoring `RAIL_3V3` and driving
  `NRST`.
- KiCad net metadata mapping `RAIL_3V3` to a powered `3.3 V` rail with
  `power_valid_at_us = 1500`.

After import, `circuitci suggest-scenarios` emits a runnable
`POWER_TREE_VALID` suggestion with `scenario.reset_supervisors[]` evidence:

- supervisor component `USUP`,
- monitored pin/net `VDD` / `net_rail_3v3`,
- reset output pin/net `RESET` / `net_nrst`,
- TLV803EA29 threshold window `2.8714 V` to `2.9886 V`.

This fixture is intentionally static. It proves threshold evidence is available
to agents from schematic import, not open-drain pull-up timing or reset waveform
shape.
