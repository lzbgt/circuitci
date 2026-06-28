# CH347 Source Notes

Retrieved: 2026-06-28

## Official WCH Metadata

- English endpoint:
  <https://www.wch-ic.com/api/official/website/common/relationFiles?fileName=CH347DS1_PDF.html>
- Chinese endpoint:
  <https://www.wch.cn/api/official/website/common/relationFiles?fileName=CH347DS1_PDF.html>
The English endpoint identifies file `CH347DS1.PDF`, version `1.4`, file id
`348`, scope `CH347`, upload date `2025-06-26`, and describes CH347 as a
480 Mbps high-speed USB bridge for UART, I2C, SPI, and JTAG use cases including
CPU debuggers, FPGA downloaders, and programming devices. The Chinese endpoint
identifies version `1.5`, file id `391`, and upload date `2025-10-30`.

The official binary endpoint
`/api/official/website/common/downloadFile?fileName=CH347DS1.PDF` returned a
browser-session refresh JSON during automated retrieval, so this repo stores
the official metadata/page files and a separate public PDF mirror for local
text extraction.

## Local PDF Text Extraction Copy

- Mirror URL:
  <http://bitsavers.informatik.uni-stuttgart.de/components/wch/_dataSheets/CH347DS1.PDF>
- Local path: `docs/research/datasheets/wch/ch347ds1_bitsavers_mirror.pdf`
- SHA-256:
  `28be5d556d5a14feba88b32210c12eb798bbc9ba91ec9a9e61d76196a3c5a93c`

Extracted facts used in `vendor.wch.ch347`:

- CH347T is a TSSOP-20 device.
- VCC requires a 3.3 V supply; the operating VCC range is `3.0 V` to `3.6 V`.
- Normal operating supply current is `50 mA` maximum.
- Absolute maximum VCC is `4.0 V`.
- Pins marked `FT` withstand up to `5.6 V` as inputs; other I/O pins are
  limited to `VCC + 0.3 V`.
- Input thresholds are `VIL <= 0.8 V` and `VIH >= 2.0 V`.
- Output levels are `VOL <= 0.4 V` while sinking 8 mA and `VOH >= VCC - 0.4 V`
  while sourcing 8 mA. The reduced model uses `2.9 V` at 3.3 V VCC and a
  conservative `50 ohm` source impedance from `0.4 V / 8 mA`.
- Working mode 3 exposes UART1 plus a JTAG interface. JTAG pins are TMS, TCK,
  TDI, TDO, and optional TRST.
- In mode 3, TDI, TCK, TMS, and TRST are outputs; TDO is a 5 V-tolerant input
  with a built-in pull-up resistor.
- TXD1 idles high, and RXD1 is a 5 V-tolerant input with an integrated pull-up.
