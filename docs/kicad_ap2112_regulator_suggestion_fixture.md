# KiCad AP2112 Regulator Suggestion Fixture

`examples/import_kicad_ap2112_regulator_suggestions/` proves that a native
KiCad schematic can bind an AP2112K-3.3 LDO symbol to the datasheet-backed
`vendor.diodes.ap2112k_3v3` model and carry regulator evidence into automatic
scenario suggestions.

The fixture contains:

- `UREG`, an AP2112K-3.3 with `VIN` on `USB_5V`, `VOUT` on `RAIL_3V3`, `EN`
  tied to `USB_5V`, and `GND` on ground.
- `U1`, a generic MCU powered from `RAIL_3V3`.
- KiCad net metadata mapping `USB_5V` to a powered `5.0 V` rail and
  `RAIL_3V3` to a powered `3.3 V` rail with `power_valid_at_us = 1500`.

After import, `circuitci suggest-scenarios` emits a runnable
`POWER_TREE_VALID` suggestion with `scenario.regulators[]` evidence:

- regulator component `UREG`,
- input pin/net `VIN` / `net_usb_5v`,
- output pin/net `VOUT` / `net_rail_3v3`,
- AP2112K dropout limit `0.4 V`,
- AP2112K output current limit `0.6 A`.

This fixture is intentionally static. It proves dropout/current validation
evidence is available to agents from schematic import, not regulator transient
stability or thermal sign-off.
