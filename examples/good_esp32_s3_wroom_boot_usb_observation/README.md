# ESP32-S3-WROOM Boot/USB Observation

This direct-open GUI example proves the source-backed ESP32-S3-WROOM-1U-N16R8
module can be used in generated SPICE observations.

The fixture uses a 3.3 V module rail, the reduced ESP32-S3-WROOM boot/USB
observation model, EN released high, GPIO0 high for SPI-flash boot, GPIO46 low,
and explicit USB D-/D+ line-state evidence. It is intended for preliminary
rail, reset, boot-strap, and USB pin-identification checks, not RF, firmware,
ROM boot protocol, USB PHY eye/impedance, flash/PSRAM mux, peak-current,
thermal, or EMC sign-off.
