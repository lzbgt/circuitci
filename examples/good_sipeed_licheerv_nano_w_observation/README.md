# LicheeRV-Nano-W Module Observation

This example validates a source-backed reduced generated-SPICE face for the
Sipeed LicheeRV-Nano-W module. It checks the board-facing 5 V module rail,
UART0 TX/RX line states, the motion-enable GPIO output, and a fault IRQ input.

The model is intentionally limited to preliminary rail and low-speed line-state
evidence. It does not model Linux boot current transients, internal SoC rails,
firmware behavior, MIPI/USB/high-speed interfaces, thermal behavior, or exact
header numbering beyond the reviewed project-facing nets.
