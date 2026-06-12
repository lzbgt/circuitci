# KiCad Bootstrap Bias Suggestion Fixture

`examples/import_kicad_bootstrap_bias_suggestions/` proves that native KiCad import can feed automatic boot-strap validation setup.

The fixture imports a generic MCU with `BOOT0` biased by real schematic resistors:

- `RUP = 100k` from `RAIL_3V3` to `BOOT0`
- `RDN = 10k` from `BOOT0` to `GND`

The KiCad mapping uses `value_ohm_from: schematic_value`, so the imported Board IR contains numeric resistor values derived from the schematic text. After import, `suggest-scenarios` recognizes the MCU boot metadata and the explicit resistor bias network, then emits a runnable `BOOT_STRAP_BIAS_VALID` template for application boot.

This is intentionally an agent-facing automation fixture: an agent can import a common IoT-board schematic and receive a concrete validation scenario instead of hand-writing the strap divider check.
