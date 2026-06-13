# TI TPS63802 3.3 V Buck-Boost Model

## Source

- Datasheet:
  `docs/research/datasheets/ti/tps63802.pdf`
- Peer evidence:
  `../urine_monitor/docs/fresh_design/projects/`
- Source note:
  `docs/research/datasheets/ti/tps63802_sources.md`
- Retrieved: 2026-06-13

## Modeled Facts

The `vendor.ti.tps63802_3v3` model captures static board-level facts for the
TPS63802 used as a peer-board 3.3 V buck-boost stage:

- `VIN` operating range: `1.3 V` to `5.5 V`.
- Startup input voltage: greater than `1.8 V`.
- Configured 3.3 V `VOUT` accepted range: `3.201 V` to `3.399 V`.
- Maximum output current screen: `2 A`, matching the datasheet condition
  `VI >= 2.3 V`, `VO = 3.3 V`.
- Required effective input capacitance: at least `4 uF`.
- Required effective output capacitance: at least `7 uF`.
- Required effective switch inductance: direct `L1` to `L2` inductance between
  `0.37 uH` and `0.57 uH`.

## Validation Use

`POWER_TREE_VALID` uses this model through `power_conversion` metadata:

- rail voltage limits come from the `VIN` and `VOUT` power ports,
- support capacitance is checked against Board IR capacitors to ground,
- buck-boost switch inductance is checked as the direct inductance between the
  `L1` and `L2` nets.

The check is a static screen. It does not sign off inductor saturation,
current ripple, DCR loss, loop stability, thermal margin, feedback tolerance,
startup from deeply depleted cells, or PCB layout.
