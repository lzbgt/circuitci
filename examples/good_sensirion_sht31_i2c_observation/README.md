# SHT31-DIS I2C Observation

This direct-open fixture exercises the reduced generated-SPICE face for the
Sensirion SHT31-DIS humidity and temperature sensor.

The fixture models a 3.3 V SHT31-DIS I2C design with 4.7 kOhm pull-ups on
`SDA`/`SCL`, `ADDR` tied low for address `0x44`, `NRESET` pulled high, and
`ALERT` held in an idle low state through a weak external pull-down.

The reduced model is intentionally high impedance. It validates observable
board-level rail and line-state assumptions only; it does not emulate
measurements, register transactions, I2C timing, clock stretching, alert
thresholds, heater behavior, drift, or calibration.
