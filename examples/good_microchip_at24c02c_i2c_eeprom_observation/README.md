# Microchip AT24C02C I2C EEPROM Observation

This fixture exercises the source-backed `vendor.microchip.at24c02c` model with
the reduced high-impedance generated-SPICE observation face in
`models/spice/microchip/at24c02c_i2c_eeprom_observation.lib`.

The scenario checks a 3.3 V `VCC` rail, idle-high `SDA`/`SCL`, address pins
`A0`/`A1`/`A2` tied low for the base I2C address, and `WP` tied low for normal
write operation. It intentionally does not emulate EEPROM transactions,
acknowledge polling, memory contents, write-cycle timing, retention, endurance,
or I2C signal integrity.
