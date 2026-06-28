# CP2102N USB-UART Observation

This fixture opens directly in the GUI as `CP2102N USB-UART`. It exercises the
source-backed `vendor.silabs.cp2102n` model through the reduced
`CIRCUITCI_CP2102N_USB_UART` generated-SPICE face in
`models/spice/generic/analog_behavioral.lib`.

The observation checks a 5 V `VREGIN` input, the generated 3.3 V `VDD`/`VIO`
rail, idle-high TXD and RTS, and asserted-low DTR through explicit external
loads. The model is deliberately limited to preliminary board-design
observation. It does not sign off USB PHY behavior, USB enumeration, UART baud
timing, oscillator accuracy, regulator stability, transistor-level
auto-download circuits, or final I/O injection current.
