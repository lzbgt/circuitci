# TI BQ24075 Source Notes

## Original Documents

| Source | URL or path | Local copy | SHA-256 |
| --- | --- | --- | --- |
| BQ2407x Standalone 1-Cell 1.5-A Linear Battery Chargers with Power Path datasheet | <https://www.ti.com/lit/ds/symlink/bq24074.pdf> | `docs/research/datasheets/ti/bq24074.pdf` | `19bd875e3fada54e74efaca074335c13eb87d0537f895c3234bf77387cbf1dd8` |
| Peer `urine_monitor` LCSC cache for `C15464` | `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C15464_BQ24075RGTR.pdf` | peer project cache | `627fb3c9cfed26cbf19fa043734b97b04980fe34572f5368ed19b84553abd2e7` |
| Peer extracted text for `C15464` | `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C15464_BQ24075RGTR.extracted.txt` | peer project cache | not copied |

## Modeled Facts

- BQ24075 input operating range is `4.35 V` to `6.4 V`.
- BQ24075 BAT regulation voltage is `4.20 V` typical, with `4.16 V` minimum
  and `4.23 V` maximum in the table.
- BQ24075 OUT regulation is `5.5 V` typical, with `5.4 V` minimum and `5.6 V`
  maximum.
- Fast-charge current range is `150 mA` to `1.5 A` in ISET mode.
- Fast-charge current is programmed by a resistor from `ISET` to `VSS`; the
  datasheet gives `ICHG = KISET / RISET`, with typical `KISET = 890 A*Ohm`.
- EN1/EN2 select USB100, USB500, or ILIM-programmed input current.
- ILIM can program maximum input current; the current CircuitCI model expects
  board policy to declare input-rail `supply_current_limit_A` rather than
  deriving it from ILIM/EN states.

## CircuitCI Use

`vendor.ti.bq24075` supports `POWER_TREE_VALID` static checks for:

- input-rail voltage range,
- battery-regulation voltage ceiling,
- programmed fast-charge current range,
- explicit or resistor-derived programmed fast-charge current,
- programmed charge current versus input source-current budget.

It does not sign off thermal regulation, DPPM, battery supplement mode,
charging timers, NTC limits, USB enumeration, ILIM/EN-derived input-current
limits, or power-path transient sharing.
