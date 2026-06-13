# TI TPS2121 Source Notes

## Original Documents

| Source | URL or path | Local copy | SHA-256 |
| --- | --- | --- | --- |
| TPS2120/TPS2121 2.8-V to 22-V Priority Power Mux with Seamless Switchover datasheet | <https://www.ti.com/lit/ds/symlink/tps2121.pdf> | `docs/research/datasheets/ti/tps2121.pdf` | `b2f5950f596dc2c4ca33e4ebac27fd35e4dcc68ea63e173466123e1118e91a06` |
| Peer `urine_monitor` LCSC cache for `C2156025` | `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C2156025_TPS2121RUXT.pdf` | peer project cache | `733f43f25a8c8dcba28b9cdd321ead4a604ae901a820f4fc881e8bb19c77b348` |
| Peer extracted text for `C2156025` | `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C2156025_TPS2121RUXT.extracted.txt` | peer project cache | not copied |

## Modeled Facts

- TPS2121 input voltage operating range is `2.8 V` to `22 V`.
- TPS2121 output voltage operating range is `0 V` to `22 V`.
- TPS2121 continuous input current is `4.5 A`.
- TPS2121 current-limit setting with `RILM = 22.1 kOhm` is `4.5 A` typical,
  with `4 A` minimum and `5 A` maximum in the table.
- TPS2121 has fast reverse-current blocking and a typical fast switchover time
  of `5 us`.

## CircuitCI Use

`vendor.ti.tps2121` supports `POWER_TREE_VALID` static checks for:

- input/output rail voltage ranges,
- selected source powered state,
- inactive unpowered input reverse-blocking evidence,
- output load current versus a modeled `4.5 A` current capability.

It does not sign off switchover droop, reverse-current magnitude, ILIM
resistor-derived current limit, priority divider thresholds, soft-start timing,
thermal behavior, or PCB layout.
