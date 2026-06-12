# KiCad AMS1117 Regulator Suggestion Fixture

`examples/import_kicad_ams1117_regulator_suggestions/` proves that a native
KiCad schematic can bind an AMS1117-3.3 LDO symbol to the datasheet-backed
`vendor.ams.ams1117_3v3` model and carry regulator evidence into automatic
scenario suggestions.

The fixture contains:

- `UREG`, an AMS1117-3.3 with `VIN` on `USB_5V`, `VOUT` on `RAIL_3V3`, and
  `GND` on ground.
- `U1`, a generic MCU powered from `RAIL_3V3`.
- `COUT`, a mapped `22 uF` output support capacitor.
- KiCad net metadata mapping `USB_5V` to a powered `5.0 V` rail and
  `RAIL_3V3` to a powered `3.3 V` rail with `power_valid_at_us = 2000`.

After import, `circuitci suggest-scenarios` emits a runnable
`POWER_TREE_VALID` suggestion with `scenario.regulators[]` evidence:

- regulator component `UREG`,
- input pin/net `VIN` / `net_usb_5v`,
- output pin/net `VOUT` / `net_rail_3v3`,
- AMS1117 dropout limit `1.3 V`,
- AMS1117 minimum load requirement `10 mA`,
- AMS1117 output current screening limit `0.8 A`,
- AMS1117 output capacitance requirement of `22 uF`,
- measured support-capacitor evidence: `COUT = 22 uF` on `net_rail_3v3`.

This fixture is intentionally static. It proves dropout/current/capacitance
validation evidence is available to agents from schematic import, not regulator
transient stability, minimum-load regulation, output capacitor ESR/material, or
thermal sign-off.
