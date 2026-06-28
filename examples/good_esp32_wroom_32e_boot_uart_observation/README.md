# ESP32-WROOM-32E Boot/UART Observation

This direct-open GUI example proves the source-backed ESP32-WROOM-32E module
can be used in generated SPICE observations.

The fixture uses a 3.3 V module rail, the reduced ESP32-WROOM-32E boot/UART
observation model, EN released high, GPIO0 high for SPI-flash boot, GPIO2 low,
and TXD0 idle high with RXD0 present as a high-impedance input. It is intended
for preliminary rail, reset, boot-strap, and UART pin-state checks, not RF,
firmware, ROM serial protocol, flash/PSRAM mux, peak-current, thermal, or EMC
sign-off.
