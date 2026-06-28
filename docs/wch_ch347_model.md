# WCH CH347 Model

## Sources

- Official WCH metadata and product page:
  `docs/research/datasheets/wch/ch347_sources.md`
- Local PDF extraction copy:
  `docs/research/datasheets/wch/ch347ds1_bitsavers_mirror.pdf`

## Modeled Facts

The `vendor.wch.ch347` model captures board-level facts needed for common
USB-JTAG/debug-bridge validation:

- CH347T is modeled as a TSSOP-20 high-speed USB bridge.
- `VCC` accepts `3.0 V` to `3.6 V`, with `50 mA` maximum normal operating
  current.
- Absolute maximum `VCC` is recorded as `4.0 V`.
- `RXD1` and `TDO` are represented as 5 V-tolerant inputs with `VIH >= 2.0 V`
  and `VIL <= 0.8 V`.
- `TXD1`, `TMS`, `TCK`, `TDI`, and `TRST` are represented as digital outputs.
- The 3.3 V output-high minimum is conservatively modeled as `2.9 V`, from the
  datasheet `VCC - 0.4 V` output-high condition.
- Output `source_impedance_ohm` is conservatively set to `50 ohm` from the
  `0.4 V / 8 mA` output-high condition.

## Validation Use

`POWER_TREE_VALID` can catch CH347 VCC rails outside the source-backed 3.3 V
range, and `IO_VOLTAGE_COMPATIBLE` can use the port directions and thresholds
for debug-header wiring checks when enough board evidence is present.

The model also has a reduced generated-SPICE observation face,
`CIRCUITCI_CH347_USB_JTAG_BRIDGE`, backed by
`models/spice/generic/analog_behavioral.lib`. It models VCC-referenced TXD1 and
JTAG line-state drivers via explicit Board IR component parameters:

- `observation_txd1_state`
- `observation_tms_state`
- `observation_tck_state`
- `observation_tdi_state`
- `observation_trst_state`

`examples/good_wch_ch347_usb_jtag_observation` is registered as the GUI
`CH347 USB-JTAG` example. It opens with routed schematic metadata, can run the
generated transient observation, and can regenerate model-aware probes and
checks for the placed `UDBG` component through `Create Checks`.

The model is not valid for USB PHY sign-off, USB enumeration, driver mode
selection, UART baud-rate timing, SPI/I2C/JTAG protocol behavior, TAP state
sign-off, external clock accuracy, or final I/O injection-current and timing
sign-off.
