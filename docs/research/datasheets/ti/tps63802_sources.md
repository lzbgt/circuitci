# TI TPS63802 Source Notes

## Original Documents

| Source | URL or path | Local copy | SHA-256 |
| --- | --- | --- | --- |
| TPS63802 2-A, High-efficient, Low IQ Buck-boost Converter in DFN Package datasheet | <https://www.ti.com/lit/ds/symlink/tps63802.pdf> | `docs/research/datasheets/ti/tps63802.pdf` | `be07deca1231c5493957bc64c2dc1cad5543bff330e49ae41caf3286fd80dca6` |
| Peer `urine_monitor` ESP32-S3 and STM32L4 fresh-design notes | `../urine_monitor/docs/fresh_design/projects/` | peer project docs | not copied |

## Modeled Facts

- Input voltage operating range is `1.3 V` to `5.5 V`.
- Device input voltage must be above `1.8 V` for start-up.
- Adjustable output voltage range is `1.8 V` to `5.2 V`; this first model is
  the peer-board `3V3` buck-boost configuration.
- The datasheet lists `2 A` output current for `VI >= 2.3 V`, `VO = 3.3 V`.
- Effective input capacitance connected to VIN is `4 uF` minimum and `5 uF`
  nominal.
- Effective output capacitance connected to VOUT is `7 uF` minimum and
  `8.2 uF` nominal for `VO > 2.3 V`.
- Effective inductance between `L1` and `L2` is `0.37 uH` minimum,
  `0.47 uH` nominal, and `0.57 uH` maximum.

## CircuitCI Use

`vendor.ti.tps63802_3v3` supports `POWER_TREE_VALID` static checks for:

- VIN and configured 3.3 V VOUT voltage ranges,
- output load current against the 2 A 3.3 V datasheet condition,
- input and output support capacitance,
- buck-boost switch inductor value between L1 and L2.

The model does not sign off startup from deeply discharged cells, current
limit behavior outside the stated operating point, inductor saturation,
current ripple, DCR loss, loop stability, thermal behavior, feedback tolerance,
or PCB layout.
