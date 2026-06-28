# CH340C USB-UART Observation

This fixture opens directly in the GUI as `CH340C USB-UART`. It exercises the
source-backed `vendor.wch.ch340c` model through the reduced
`CIRCUITCI_CH340C_USB_UART` generated-SPICE face in
`models/spice/generic/analog_behavioral.lib`.

The observation checks a 3.3 V CH340C rail, idle-high TXD, asserted-low DTR#,
and idle-high RTS# through explicit external loads. The model is deliberately
limited to preliminary board-design observation. It does not sign off USB PHY
behavior, USB enumeration, baud-rate timing, oscillator accuracy,
transistor-level auto-download circuits, or final I/O injection current.
