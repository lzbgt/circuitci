# FTDI FT232R Model

## Sources

- Official FTDI data sheet URLs and local extraction mirror:
  `docs/research/datasheets/ftdi/ft232r_sources.md`
- Local PDF extraction copy:
  `docs/research/datasheets/ftdi/DS_FT232R_sparkfun_mirror.pdf`

## Modeled Facts

The `vendor.ftdi.ft232r` model captures board-level facts used by common
USB-UART validation:

- `VCC` accepts `4.0 V` to `5.25 V` for the common internal-oscillator mode.
- `VCCIO` accepts `1.8 V` to `5.25 V`.
- `3V3OUT` is modeled as a `3.0 V` to `3.6 V` integrated regulator output with
  `50 mA` maximum external current.
- Normal operating current is represented as `15 mA` typical for static
  power-tree budgeting.
- UART/control outputs `TXD`, `DTR#`, and `RTS#` are represented with
  source-backed 3.3 V output-high minima.
- UART/control inputs `RXD`, `RI#`, `DSR#`, and `DCD#` use the source switching
  threshold range as board-level VIH/VIL screening guidance.
- `RESET#` is available as an active-low input and may be tied to `VCC`.

## Validation Use

`POWER_TREE_VALID` can catch overvoltage on `VCC`, `VCCIO`, or `3V3OUT`.
`UART_BOOTLOADER_SYNC` and `IO_VOLTAGE_COMPATIBLE` can reason about FT232R
UART/control-line wiring without embedding FTDI-specific logic in validators.

The model also has a reduced generated-SPICE observation face,
`CIRCUITCI_FT232R_USB_UART`, backed by
`models/spice/generic/analog_behavioral.lib`. It models a smooth VCC-to-3V3OUT
rail source and VCCIO-referenced TXD, RTS#, and DTR# output-state drivers via
explicit Board IR component parameters:

- `observation_txd_state`
- `observation_rts_n_state`
- `observation_dtr_n_state`

`examples/good_ftdi_ft232r_usb_uart_observation` is registered as the GUI
`FT232R USB-UART` example. It opens with routed schematic metadata, can run the
generated transient observation, and can regenerate model-aware probes and
checks for the placed `UUSB` component through `Create Checks`.

The model is not valid for USB PHY sign-off, USB enumeration, EEPROM/CBUS
programming behavior, UART baud-rate timing, oscillator accuracy, suspend-state
current sign-off, regulator stability, transistor-level modem-line behavior, or
final I/O injection-current and thermal sign-off.
