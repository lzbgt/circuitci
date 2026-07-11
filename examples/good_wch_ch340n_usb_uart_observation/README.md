# CH340N USB-UART Observation

This fixture opens directly in the GUI as `CH340N USB-UART`. It exercises the
source-backed `vendor.wch.ch340n` model through the reduced
`CIRCUITCI_CH340N_USB_UART` generated-SPICE face in
`models/spice/wch/ch340n_usb_uart.lib`.

The observation checks a 3.3 V CH340N rail, idle-high TXD, and idle-high RTS#
through explicit external loads. The SOP-8 CH340N model intentionally exposes
RTS# but not DTR# because the CH340DS1 pin table does not assign DTR# to the
SOP-8 member.

The model is deliberately limited to preliminary board-design observation. It
does not sign off USB PHY behavior, USB enumeration, baud-rate timing,
oscillator accuracy, transistor-level modem-line circuits, or final I/O
injection current.
