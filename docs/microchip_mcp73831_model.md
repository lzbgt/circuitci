# Microchip MCP73831 Model

## Source

- Datasheet:
  `docs/research/datasheets/microchip/mcp73831-family-datasheet.pdf`
- Source URL:
  <https://ww1.microchip.com/downloads/en/DeviceDoc/MCP73831-Family-Data-Sheet-DS20001984H.pdf>
- SHA-256:
  `75297f2a5235599368381adfb63df2652cc22d757f171f388147b2ab3d32776f`
- Retrieved: 2026-06-12

## Modeled Facts

The `vendor.microchip.mcp73831_4v2` model captures static board-level facts for
the 4.2 V regulation option:

- `VDD` operating supply range: `3.75 V` to `6.0 V`.
- `VBAT` regulation option: `4.20 V` typical, with the datasheet range
  `4.168 V` to `4.232 V`.
- Programmable fast-charge current range: approximately `15 mA` to `500 mA`.
- `PROG` is represented as a passive programming pin.
- `STAT` is represented as a digital output for status connectivity.

The Board IR component instance must declare:

```yaml
parameters:
  programmed_charge_current_A: 0.1
```

That value should come from the schematic `PROG` resistor or board
configuration.

## Validation Use

`POWER_TREE_VALID` uses the model's `battery_charger` metadata to check:

- programmed charge current is present and finite,
- programmed charge current is inside the modeled charger range,
- programmed charge current does not exceed the input rail
  `supply_current_limit_A`,
- battery-net nominal voltage does not exceed the charger regulation voltage.

This is intentionally static. It does not validate battery chemistry, thermal
foldback, preconditioning behavior, charge termination, USB negotiation,
thermal dissipation, or transient load sharing between charger and system load.
