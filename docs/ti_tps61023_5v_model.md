# TI TPS61023 5 V Boost Model

## Source

- Datasheet:
  `docs/research/datasheets/ti/tps61023.pdf`
- Peer evidence:
  `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/C1852149_TPS61023DRLT.pdf`
- Source note:
  `docs/research/datasheets/ti/tps61023_sources.md`
- Retrieved: 2026-06-13

## Modeled Facts

The `vendor.ti.tps61023_5v` model captures static board-level facts for the
TPS61023 used as the peer-board `VEXT_5V` boost stage:

- `VIN` operating range: `0.5 V` to `5.5 V`.
- 5 V configured `VOUT` accepted range: `4.9 V` to `5.1 V`.
- `SW` is the boost switch node.
- Required input capacitance: at least `10 uF` to ground.
- Required effective output capacitance: at least `4 uF` to ground.
- Required boost inductor: direct `VIN` to `SW` inductance between `0.37 uH`
  and `2.9 uH`.

The model intentionally does not encode a fixed maximum output current. The
datasheet gives a valley switch-current limit, while actual output current
depends on VIN, VOUT, efficiency, ripple current, inductor value, saturation
current, thermal conditions, and layout.

## Validation Use

`POWER_TREE_VALID` uses this model through `power_conversion` metadata:

- rail voltage limits come from the `VIN` and `VOUT` power ports,
- support capacitance is checked against Board IR capacitors to ground,
- boost input inductance is checked as the direct inductance between the input
  rail and `SW`.

The check is a static screen. It does not sign off inductor saturation,
current ripple, DCR loss, loop stability, thermal margin, feedback tolerance,
startup from deeply depleted cells, or PCB layout.
