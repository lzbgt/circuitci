# Bosch BME280 Model

`vendor.bosch.bme280` is a source-backed static board-boundary model for the
Bosch Sensortec BME280 humidity, pressure, and temperature sensor.

The model is intended for low-risk board checks:

- `POWER_TREE_VALID` screens `VDD` against the 1.71 V to 3.6 V operating range.
- `POWER_TREE_VALID` screens `VDDIO` against the 1.2 V to 3.6 V operating range.
- The port list captures the board-facing I2C/SPI pins: `CSB`, `SDI`, `SDO`,
  and `SCK`.
- Datasheet metadata records Bosch's low-power use-current examples, package
  dimensions, I2C/SPI maximum clock classes, and SDO-selected I2C addresses.

The model deliberately does not validate humidity, pressure, or temperature
accuracy; compensation algorithms; register protocol behavior; I2C/SPI timing;
sensor noise; response time; condensation; drift; self-heating; or calibration.

## Evidence

The official Bosch PDF is retained at
`docs/research/datasheets/bosch/bme280_datasheet.pdf`. Source notes and hashes
are recorded in `docs/research/datasheets/bosch/bme280_sources.md`.
