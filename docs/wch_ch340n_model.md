# WCH CH340N Model

## Sources

- Official WCH English metadata:
  `docs/research/datasheets/wch/ch340ds1_wch_ic_metadata.json`
- Official WCH Chinese metadata:
  `docs/research/datasheets/wch/ch340ds1_wch_cn_metadata.json`
- Local text-extraction copy:
  `docs/research/datasheets/wch/ch340ds1_sparkfun_mirror.pdf`

The official WCH metadata identifies file `CH340DS1.PDF`, version `3.4`, file
id `79`, and scope `CH340`. The official binary download endpoint requires a
browser refresh/session token, so the repo keeps the official metadata JSON and
an inspectable public PDF mirror for local text extraction.

## Modeled Facts

The `vendor.wch.ch340n` model captures board-level facts for the SOP-8 CH340N:

- CH340N supports 5 V and 3.3 V supply modes.
- The model accepts `VCC` from `3.1 V` to `5.3 V`, matching the datasheet's
  CH340N 3.3 V mode minimum and 5 V mode maximum.
- `max_supply_current_A` is `0.02 A`, from the CH340G/C/N/K/E/X/T/R 5 V
  operating current maximum.
- 3.3 V-mode input thresholds are `VIH >= 1.9 V` and `VIL <= 0.8 V`.
- 3.3 V-mode output high is conservatively modeled as `2.7 V` from
  `VCC - 0.6 V` at 2 mA source current.
- `source_impedance_ohm` is conservatively set to `300 ohm` from the same
  `0.6 V / 2 mA` output-high condition.
- CH340N has an integrated clock generator, so the model does not require an
  external crystal for board validation.
- The CH340DS1 package table assigns CH340N to SOP-8 and maps SOP-8 pins to
  `VCC`, `GND`, `V3`, `UD+`, `UD-`, `TXD`, `RXD`, and `RTS#`. It does not map
  `DTR#` to the SOP-8 member.
- The generated-SPICE face uses `CIRCUITCI_CH340N_USB_UART` with pin order
  `[VCC, GND, TXD, RXD, RTS_N]`. `TXD_STATE` and `RTS_N_STATE` are fed from
  Board IR component parameters `observation_txd_state` and
  `observation_rts_n_state`, defaulting high when omitted.

## Validation Use

`POWER_TREE_VALID` can reject rails outside the datasheet-backed CH340N VCC
range. `UART_BOOTLOADER_SYNC` and control-line scenarios can use the CH340N as
a compact USB-UART bridge when the board uses RTS# but not DTR# from the SOP-8
package.

`examples/good_wch_ch340n_usb_uart_observation/project.yaml` is a direct-open
GUI fixture that exercises the reduced generated-SPICE face with a 3.3 V rail,
idle-high TXD, and idle-high RTS#.

The model is not valid for USB PHY sign-off, USB enumeration, baud-rate timing,
oscillator accuracy, transistor-level modem-line behavior, auto-download
transistor networks, or final I/O injection-current sign-off. Use
`GPIO_BACKDRIVE` or explicit `analog_transient` scenarios when CH340N and the
target MCU can be powered from separate rails.
