# TI TPS61023 Source Notes

## Original Documents

| Source | URL or path | Local copy | SHA-256 |
| --- | --- | --- | --- |
| TPS61023 3.7-A Boost Converter With 0.5-V Ultra Low Input Voltage datasheet | <https://www.ti.com/lit/ds/symlink/tps61023.pdf> | `docs/research/datasheets/ti/tps61023.pdf` | `79a66e0a66e98e89b84ced5ba6f4504fca843ec072641c94b1fd73876c43226d` |
| Peer `urine_monitor` LCSC cache for `C1852149` | `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C1852149_TPS61023DRLT.pdf` | peer project cache | `4a9042ba1594d7009a0f4831f881a4f089cc8a2b037c1a25ab76c90d9cf50887` |
| Peer extracted text for `C1852149` | `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C1852149_TPS61023DRLT.extracted.txt` | peer project cache | not copied |

## Modeled Facts

- Input voltage operating range is `0.5 V` to `5.5 V`.
- Startup requires at least `1.8 V` input.
- Output voltage setting range is `2.2 V` to `5.5 V`; this first model is
  the peer-board 5 V boost configuration.
- Valley switch current limit is `3.7 A` typical and `2.7 A` minimum at
  `VIN = 3.6 V`, `VOUT = 5.0 V`.
- The device is designed for inductors from `0.37 uH` to `2.9 uH`; the
  peer design uses the datasheet typical `1 uH` value.
- The datasheet states that a `10 uF` input capacitor is sufficient for most
  applications.
- The datasheet recommends effective ceramic output capacitance from `4 uF`
  to `1000 uF`; the peer design uses `2 x 22 uF`.

## CircuitCI Use

`vendor.ti.tps61023_5v` supports `POWER_TREE_VALID` static checks for:

- VIN and VOUT voltage ranges,
- input and output support capacitance,
- boost input-inductor value between VIN and SW.

The model does not derive allowed output current from operating point,
inductor saturation current, efficiency, thermal conditions, feedback-resistor
tolerance, DCR, ripple current, or loop stability.
