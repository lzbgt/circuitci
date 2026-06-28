# FTDI FT232R Sources

Retrieved on 2026-06-28.

## Primary Source

- Official FTDI data-sheet URL:
  <https://ftdichip.com/wp-content/uploads/2020/08/DS_FT232R.pdf>
- Legacy official FTDI data-sheet URL:
  <https://www.ftdichip.com/Support/Documents/DataSheets/ICs/DS_FT232R.pdf>

The official FTDI URLs were visible through browser search/open flows but
returned HTTP 403 to automated binary retrieval from this environment. For
local extraction, the repo stores a public PDF mirror of the same FTDI data
sheet:

- Mirror URL:
  <https://cdn.sparkfun.com/datasheets/BreakoutBoards/DS_FT232R.pdf>
- Local copy:
  `docs/research/datasheets/ftdi/DS_FT232R_sparkfun_mirror.pdf`
- SHA-256:
  `0f4e94f51a392d253b28ee92e71b4c2f7636c7e82780af69ce4d00035f4e730c`

## Extracted Facts Used In `vendor.ftdi.ft232r`

- FT232R is a USB-to-UART bridge with integrated EEPROM, USB termination
  resistors, clock generation, USB transceiver, UART controller, and CBUS
  configuration pins.
- For common internal-oscillator operation, `VCC` operating supply is `4.0 V`
  to `5.25 V`. Operation down to `3.3 V` requires an external crystal.
- `VCCIO` supplies the UART interface and CBUS pins from `1.8 V` to `5.25 V`.
  In bus-powered designs it may be tied to `3V3OUT` for 3.3 V logic or to
  `VCC` for 5 V CMOS logic.
- Normal operating supply current is listed as `15 mA` typical, and USB suspend
  current as `70 uA` typical with `100 uA` maximum.
- `3V3OUT` is the integrated 3.3 V LDO output. It is specified as `3.0 V` to
  `3.6 V` and can supply up to `50 mA` to external logic.
- UART outputs include `TXD`, `DTR#`, and `RTS#`; UART/modem inputs include
  `RXD`, `RI#`, `DSR#`, and `DCD#`.
- `RESET#` is active low and may be tied to `VCC` or left unconnected when not
  used.
- At `VCCIO = 3.3 V`, standard-drive UART/CBUS output high is at least `2.2 V`
  at `1 mA`; output low is at most `0.5 V` at `2 mA`. At `VCCIO = 5.0 V`,
  output high is at least `3.2 V` at `2 mA`, and output low is at most `0.6 V`
  at `2 mA`.
- UART/CBUS input switching threshold is listed as `1.0 V` minimum, `1.2 V`
  typical, and `1.5 V` maximum.
- Absolute maximum ratings include `VCC` up to `6.0 V`, USB pins up to `3.8 V`,
  other inputs/bidirectional pins up to `VCC + 0.5 V`, output current up to
  `24 mA`, and `500 mW` power dissipation at `VCC = 5.25 V`.

## Modeling Notes

The current model is board-level. It is useful for power-tree, USB-UART
connectivity, UART bootloader, and preliminary control-line/backdrive
screening. It is not a USB PHY model, not USB enumeration or driver-stack
simulation, not EEPROM/CBUS programming behavior, not oscillator accuracy
sign-off, and not final I/O injection-current or thermal sign-off.
