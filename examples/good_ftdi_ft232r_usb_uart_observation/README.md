# FT232R USB-UART Observation

This fixture opens directly in the GUI as `FT232R USB-UART`. It exercises the
source-backed `vendor.ftdi.ft232r` model through the reduced
`CIRCUITCI_FT232R_USB_UART` generated-SPICE face in
`models/spice/generic/analog_behavioral.lib`.

The observation checks a 5 V `VCC` input, generated 3.3 V `3V3OUT`/`VCCIO`,
idle-high TXD and RTS#, and asserted-low DTR# through explicit external loads.
The model is deliberately limited to preliminary board-design observation. It
does not sign off USB PHY behavior, USB enumeration, EEPROM/CBUS programming,
UART baud timing, oscillator accuracy, suspend current, regulator stability,
transistor-level auto-download circuits, or final I/O injection current.
