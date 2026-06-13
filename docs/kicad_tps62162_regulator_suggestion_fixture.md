# KiCad TPS62162 Regulator Suggestion Fixture

`examples/import_kicad_tps62162_regulator_suggestions/` proves that a native
KiCad schematic can bind a TPS62162 fixed 3.3 V buck symbol to the
datasheet-backed `vendor.ti.tps62162_3v3` model and carry static regulator
evidence into automatic scenario suggestions.

The fixture contains:

- `UBUCK`, a TPS62162 with `VIN` on `RAIL_12V`, `VOS` on `RAIL_3V3`, `SW`
  on `BUCK_SW`, `EN` tied to `RAIL_12V`, `PG` on `BUCK_PG`, and `GND` on
  ground.
- `L1`, a mapped `2.2 uH` output support inductor from `BUCK_SW` to
  `RAIL_3V3`.
- `U1`, a generic MCU powered from `RAIL_3V3`.
- `CIN`, a mapped `10 uF` input support capacitor.
- `COUT`, a mapped `22 uF` output support capacitor.
- KiCad net metadata mapping `RAIL_12V` to a powered `12.0 V` rail and
  `RAIL_3V3` to a powered `3.3 V` rail with `power_valid_at_us = 1000`.

After import, `circuitci suggest-scenarios` emits a runnable
`POWER_TREE_VALID` suggestion with `scenario.regulators[]` evidence:

- regulator component `UBUCK`,
- input pin/net `VIN` / `net_rail_12v`,
- output pin/net `VOS` / `net_rail_3v3`,
- switch pin/net `SW` / `net_buck_sw`,
- TPS62162 output current limit `1.0 A`,
- TPS62162 input/output capacitance requirements of `10 uF` and `22 uF`,
- TPS62162 output inductance minimum requirement of `2.2 uH`,
- measured support-capacitor evidence:
  - `CIN = 10 uF` on `net_rail_12v`,
  - `COUT = 22 uF` on `net_rail_3v3`.
- measured support-inductor evidence:
  - `L1 = 2.2 uH` between `net_buck_sw` and `net_rail_3v3`.

This fixture is intentionally static. It proves power-tree and support-network
evidence is available to agents from schematic import, not inductor saturation
current, DCR, ripple, control-loop stability, thermal behavior, EMI, or layout
sign-off.
