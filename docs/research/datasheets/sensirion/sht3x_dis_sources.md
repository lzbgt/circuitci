# Sensirion SHT3x-DIS Source Notes

Retrieved on 2026-07-13 from Sensirion:

- Source URL:
  <https://sensirion.com/media/documents/213E6A3B/63A5A569/Datasheet_SHT3x_DIS.pdf>
- Local original: `docs/research/datasheets/sensirion/sht3x_dis_datasheet.pdf`
- SHA-256:
  `095b1853e7f4328f5897c9ca6c392a7dd8b0202eda66b0a2629f9cb840dd496d`

Facts retained in `vendor.sensirion.sht31_dis`:

- SHT3x-DIS is a digital humidity and temperature sensor family. The SHT31-DIS
  member is the standard accuracy variant in that family.
- The retained datasheet is Sensirion's December 2022 Version 7
  `Datasheet SHT3x-DIS`.
- The package footprint is 2.5 mm x 2.5 mm with 0.9 mm height.
- The supply voltage range is 2.15 V to 5.5 V, with 3.3 V typical.
- The I2C interface supports communication speeds up to 1 MHz and two
  selectable 7-bit addresses.
- `ADDR` selects the I2C address: logic low selects default address `0x44`,
  logic high selects address `0x45`, and the pin must not be left floating.
- The board-facing pins are `SDA`, `SCL`, `ADDR`, `ALERT`, `nRESET`, `VDD`, and
  `VSS`, plus the center die pad connected to `VSS`.
- `ALERT` may be left floating when unused. It switches high when configured
  alert conditions are met, but alert-limit behavior is outside the reduced
  model.
- `nRESET` may be left floating or connected to `VDD` through a series resistor
  of at least 2 kOhm when unused; the datasheet notes a typical internal
  50 kOhm pull-up.
- Electrical examples include 0.2 uA typical idle current in single-shot mode
  at 25 C, 45 uA typical idle current in periodic mode, 600 uA typical and
  1500 uA maximum measuring current, and 1.7 uA typical average current for one
  low-repeatability measurement per second.

Model boundary:

- The pack is a static board-boundary model for supply-voltage and pin-binding
  review plus a reduced generated-SPICE I2C board-observation model.
- It does not model humidity or temperature accuracy; compensation formulas;
  register protocol; I2C transaction timing; clock stretching; alert threshold
  logic; heater behavior; response time; contamination; drift; self-heating; or
  calibration behavior.
