# Bosch BME280 Source Notes

Retrieved on 2026-07-05 from Bosch Sensortec:

- Source URL:
  <https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bme280-ds002.pdf>
- Local original: `docs/research/datasheets/bosch/bme280_datasheet.pdf`
- SHA-256:
  `a2ccdb449fec94380742fe8eec851a11d9bd4142252d332b34682b4deecd7d89`

Facts retained in `vendor.bosch.bme280`:

- BME280 is a combined humidity, pressure, and temperature sensor in a
  2.5 mm x 2.5 mm x 0.93 mm metal-lid LGA package.
- `VDD` is the main sensor supply. The recommended operating range is
  1.71 V to 3.6 V.
- `VDDIO` is the digital-interface supply. The recommended operating range is
  1.2 V to 3.6 V.
- Bosch lists absolute maximum voltage at `VDD` and `VDDIO` as -0.3 V to
  4.25 V, and interface pins as -0.3 V to `VDDIO + 0.3 V`.
- The device supports I2C up to 3.4 MHz and SPI up to 10 MHz.
- `CSB` selects the interface mode: tying `CSB` to `VDDIO` activates I2C, while
  SPI uses chip-select behavior.
- In I2C mode, `SDO` selects the address: low gives `0x76`, high gives `0x77`;
  Bosch notes that `SDO` cannot be left floating.
- Bosch warns that interface pins must not be held high while `VDDIO` is off.
- The generated-SPICE face is intentionally high impedance and only observes
  the surrounding board's rail, pull-up, interface-select, and address-select
  states.

Model boundary:

- The pack is a static board-boundary model for supply-voltage and pin-binding
  review plus a reduced generated-SPICE I2C board-observation model.
- It does not model humidity, pressure, or temperature accuracy; register
  protocol; compensation formulas; I2C/SPI timing; response time; noise;
  condensation; aging; self-heating; or calibration behavior.
