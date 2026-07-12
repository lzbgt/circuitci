# Bosch BME280 Model

`vendor.bosch.bme280` is a source-backed board-boundary model for the Bosch
Sensortec BME280 humidity, pressure, and temperature sensor. It includes static
power/pin metadata plus a reduced generated-SPICE I2C observation face.

The model is intended for low-risk board checks:

- `POWER_TREE_VALID` screens `VDD` against the 1.71 V to 3.6 V operating range.
- `POWER_TREE_VALID` screens `VDDIO` against the 1.2 V to 3.6 V operating range.
- The port list captures the board-facing I2C/SPI pins: `CSB`, `SDI`, `SDO`,
  and `SCK`.
- Datasheet metadata records Bosch's low-power use-current examples, package
  dimensions, I2C/SPI maximum clock classes, and SDO-selected I2C addresses.
- `CIRCUITCI_BME280_I2C_OBSERVATION` checks board-level rail, SDA/SCL pull-up,
  `CSB` I2C-select, and `SDO` address-select states while leaving all
  interface pins high impedance.

The model deliberately does not validate humidity, pressure, or temperature
accuracy; compensation algorithms; register protocol behavior; I2C/SPI timing;
sensor noise; response time; condensation; drift; self-heating; or calibration.

`examples/good_bosch_bme280_i2c_observation/project.yaml` is a direct-open GUI
fixture that runs the reduced observation model with 3.3 V `VDD`/`VDDIO`,
4.7 kOhm I2C pull-ups, `CSB` high for I2C mode, and `SDO` low for address
`0x76`.

## Evidence

The official Bosch PDF is retained at
`docs/research/datasheets/bosch/bme280_datasheet.pdf`. Source notes and hashes
are recorded in `docs/research/datasheets/bosch/bme280_sources.md`.
