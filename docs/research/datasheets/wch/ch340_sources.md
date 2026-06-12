# CH340 Source Notes

Retrieved: 2026-06-12

## Official WCH Metadata

- English endpoint:
  <https://www.wch-ic.com/api/official/website/common/relationFiles?fileName=CH340DS1_PDF.html>
- Chinese endpoint:
  <https://www.wch.cn/api/official/website/common/relationFiles?fileName=CH340DS1_PDF.html>

Both endpoints identify WCH file id `79`, name `CH340DS1.PDF`, version `3.4`,
and scope `CH340`. The English endpoint reports upload date `2025-03-12`; the
Chinese endpoint reports upload date `2025-03-03`.

The official binary endpoint
`/api/official/website/common/downloadFile?fileName=CH340DS1.PDF` returned a
browser-session refresh error during automated retrieval, so this repo stores
the official metadata JSON and a separate public PDF mirror for local text
extraction.

## Local PDF Text Extraction Copy

- Mirror URL: <https://cdn.sparkfun.com/assets/5/0/a/8/5/CH340DS1.PDF>
- Local path: `docs/research/datasheets/wch/ch340ds1_sparkfun_mirror.pdf`
- SHA-256:
  `f4b76c52222358bec25f328517d9801036e606d57648e62e2ac59e3027a6c050`

Extracted facts used in `vendor.wch.ch340c`:

- CH340C/N/K/E/X/B have integrated clock generator and need no external
  crystal.
- CH340 supports 5 V and 3.3 V supply.
- In 5 V mode, VCC range is `4.0 V` to `5.3 V`, and CH340G/C/N/K/E/X/T/R
  operating current maximum is `20 mA`.
- In 3.3 V mode for CH340C/N/K/E/X/B, VCC range is `3.1 V` to `3.6 V`.
- In 3.3 V mode, `VIL <= 0.8 V`, `VIH >= 1.9 V`, and `VOH >= VCC - 0.6 V` at
  2 mA source current.
- `TXD` stays high when UART transmission is idle, and `RXD` stays high when
  UART reception is idle.
- `DTR#` and `RTS#` are active-low MODEM output signals.
- WCH reference text warns that shared CH340/MCU power avoids I/O current
  between CH340 and MCU; separate supply designs must avoid bidirectional
  current poured backwards.
