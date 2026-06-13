# Microchip MCP73831 Source Notes

## Original Documents

| Source | URL or path | Local copy | SHA-256 |
| --- | --- | --- | --- |
| MCP73831/2 Miniature Single-Cell Li-Ion/Li-Polymer Charge Management Controllers | <https://ww1.microchip.com/downloads/en/DeviceDoc/MCP73831-Family-Data-Sheet-DS20001984H.pdf> | `docs/research/datasheets/microchip/mcp73831-family-datasheet.pdf` | `75297f2a5235599368381adfb63df2652cc22d757f171f388147b2ab3d32776f` |

## Modeled Facts

- MCP73831 `VDD` operating range is `3.75 V` to `6.0 V`.
- The `-2` charger option regulates `VBAT` to `4.20 V` typical, with the
  modeled datasheet range `4.168 V` to `4.232 V`.
- Fast-charge current is programmed by a resistor from `PROG` to `VSS`.
- The datasheet equation in section 5.1.2 gives `IREG = 1000 V / RPROG` when
  `RPROG` is in ohms and `IREG` is in amperes. This matches the table points
  `10 kOhm -> 0.1 A` and `2 kOhm -> about 0.5 A`.
- The modeled static charge-current range is `0.015 A` to `0.5 A`.

## CircuitCI Use

`vendor.microchip.mcp73831_4v2` supports `POWER_TREE_VALID` static checks for:

- input-rail voltage range,
- battery-regulation voltage ceiling,
- explicit or resistor-derived programmed charge-current range,
- programmed charge current versus input source-current budget.

The resistor-derived current path is accepted only when Board IR contains one
positive resistor between the charger `PROG` net and `VSS` net. Ambiguous,
missing, or invalid resistor evidence falls back to the explicit
`programmed_charge_current_A` parameter requirement.
