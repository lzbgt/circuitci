# Bosch BME280 I2C Observation

This fixture opens directly in the GUI as `BME280 I2C Sensor`. It exercises the
`CIRCUITCI_BME280_I2C_OBSERVATION` generated-SPICE face in
`models/spice/bosch/bme280_i2c_observation.lib`.

The observation checks a 3.3 V `VDD`/`VDDIO` rail, I2C pull-ups on `SDI`/`SCK`,
`CSB` tied high for I2C mode, and `SDO` pulled low for the `0x76` address. The
model is intentionally high impedance and does not emulate BME280 measurements,
registers, compensation formulas, bus transactions, timing, noise, or
calibration behavior.
