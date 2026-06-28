# CH347 USB-JTAG Observation

This fixture opens directly in the GUI as `CH347 USB-JTAG`. It exercises the
reduced `CIRCUITCI_CH347_USB_JTAG_BRIDGE` generated-SPICE face using the
source-backed WCH CH347 model.

The observation is intentionally limited to board-level line states:

- `VCC` is a 3.3 V rail inside the source-backed CH347 operating range.
- `TXD1`, `TMS`, `TDI`, and `TRST` are driven high.
- `TCK` is held low as an idle clock-state snapshot.
- `TDO` is represented as a pulled-up input.

This is not a USB, UART, JTAG TAP, CPU-debug, external-clock, or timing sign-off
model. It is a quick executable check that the board-level debug bridge rails
and default line states are plausible before deeper protocol validation.
