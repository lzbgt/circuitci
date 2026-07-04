# STC15W408AS Model

`vendor.stc.stc15w408as` is a source-backed static model for STC's
STC15W408AS 1T 8051-family MCU.

The model covers:

- `VCC` operating range from 2.4 V to 5.5 V.
- `GND` required ground.
- `RST` reset boundary, active high when configured as reset.
- Primary ISP/UART pins `P3_0_RXD` and `P3_1_TXD`.
- Alternate UART pin pairs `P3_6_RXD_2`/`P3_7_TXD_2` and
  `P1_6_RXD_3`/`P1_7_TXD_3`.

The retained source originals are under
`docs/research/datasheets/stc/`. `stc15w408as_sources.md` records hashes,
retrieval date, modeled facts, and the boundary that exact STC ISP sync/ACK
bytes are not modeled from these sources.

The model is intended for static board checks such as supply voltage and reset
polarity. It is not a firmware, flash-programming, ISP protocol, package
variant, oscillator, thermal, or transient current model.
