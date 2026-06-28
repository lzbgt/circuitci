# DRV8323 Gate-Driver Observation

This fixture proves the source-backed `vendor.ti.drv8323` model can be placed
from the GUI example picker and used in generated SPICE observations.

The generated deck drives:

- `VM` from a 24 V motor bus source.
- `DVDD` and `ENABLE` from 3.3 V sources.
- `nFAULT` released high through the reduced model.
- `SDO` low through the reduced model.
- `SOA`, `SOB`, and `SOC` at nominal 1.65 V observation points.

The model is intentionally limited to preliminary board-facing checks for
supply wiring, enable state, digital output state, and current-sense output
presence. It is not a MOSFET gate-drive, SPI/protection, motor-control,
current-sense accuracy, layout, EMI, or thermal sign-off model.
