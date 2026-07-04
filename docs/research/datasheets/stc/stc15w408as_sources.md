# STC15W408AS Source Notes

## Retrieved Sources

- `stc15w408as_features.pdf`
  - Source: <https://www.stcmicro.com/datasheet/STC15W408AS_Features.pdf>
  - SHA-256: `6129da6f667beb9d6491d5984c2d2aa35f641cfa02c069d11376e4b334bc21bf`
  - Retrieved: 2026-07-05
- `stc15f2k60s2_en.pdf`
  - Source: <https://www.stcmicro.com/datasheet/STC15F2K60S2-en.pdf>
  - SHA-256: `c1df3cdb156465acc3f1d07b283b0c599d33cc170a7946a8cb9330aa84273801`
  - Retrieved: 2026-07-05

## Modeled Facts

- The official STC15W408AS material identifies the part as an enhanced 1T
  8051-family MCU and lists an operating voltage range of 2.4 V to 5.5 V.
- The package/pin material identifies `P5.4/RST/SysClkO/CMP-` as the reset
  pin, with reset active high when configured as reset rather than GPIO.
- The ISP application circuit and UART pin table identify the primary serial
  programming UART on `P3.0/RxD` and `P3.1/TxD`.
- The UART switch table also identifies alternate UART placements on
  `P3.6/RxD_2` plus `P3.7/TxD_2`, and `P1.6/RxD_3` plus `P1.7/TxD_3`.

## Modeling Boundary

`vendor.stc.stc15w408as` intentionally models only supply voltage, reset
polarity, and UART board-boundary pins. The retained public STC sources support
UART ISP wiring and cold-reset ISP entry behavior, but they do not provide a
stable byte-level sync/ACK contract in the reviewed source set. The component
model therefore does not declare a `bootloader.interfaces[]` entry; adding one
would make `UART_BOOTLOADER_SYNC` scenarios look source-backed when the byte
protocol is not proven by retained evidence.
