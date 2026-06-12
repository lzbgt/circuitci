# KiCad Reset RC Suggestion Fixture

`examples/import_kicad_reset_rc_suggestions/` proves that native KiCad import can feed automatic reset-release timing setup.

The fixture imports a generic MCU reset pin with an RC network:

- `R1 = 10k` from `RAIL_3V3` to `NRST`
- `C1 = 100n` from `NRST` to `GND`
- `RAIL_3V3.power_valid_at_us = 1500`

The mapping parses both passive values from schematic text. `suggest-scenarios` then recognizes the active-low reset pin, the target rail timing, and the explicit RC evidence. It emits a runnable `RESET_RELEASE_AFTER_POWER_VALID` template with `reset_release_delay_us` derived from:

```text
t = -R * C * ln(1 - VIH / Vrail)
```

This keeps the generated timestamp tied to schematic evidence instead of a hand-written reset-release time.
