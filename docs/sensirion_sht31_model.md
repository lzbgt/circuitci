# Sensirion SHT31-DIS Model

`vendor.sensirion.sht31_dis` is a source-backed board-boundary model for the
Sensirion SHT31-DIS humidity and temperature sensor. It includes static
power/pin metadata plus a reduced generated-SPICE I2C observation face.

The model is intended for low-risk board checks:

- `POWER_TREE_VALID` screens `VDD` against the 2.15 V to 5.5 V operating range.
- The port list captures the board-facing I2C pins: `SDA`, `SCL`, `ADDR`,
  `NRESET`, and `ALERT`.
- Datasheet metadata records the 1 MHz I2C interface class, `ADDR`-selected
  `0x44`/`0x45` addresses, package dimensions, and current classes.
- `CIRCUITCI_SHT31_I2C_OBSERVATION` checks board-level rail, SDA/SCL pull-up,
  `ADDR` address-select, `nRESET` pull-up, and idle `ALERT` state while leaving
  all signal pins high impedance.

The model deliberately does not validate humidity or temperature accuracy;
compensation algorithms; register protocol behavior; I2C timing or clock
stretching; alert threshold logic; heater behavior; response time;
contamination; drift; self-heating; or calibration.

`examples/good_sensirion_sht31_i2c_observation/project.yaml` is a direct-open
GUI fixture that runs the reduced observation model with 3.3 V `VDD`, 4.7 kOhm
I2C pull-ups, `ADDR` low for address `0x44`, `NRESET` pulled high, and `ALERT`
left in a low idle state through an external weak pull-down.

## Evidence

The official Sensirion PDF is retained at
`docs/research/datasheets/sensirion/sht3x_dis_datasheet.pdf`. Source notes and
hashes are recorded in
`docs/research/datasheets/sensirion/sht3x_dis_sources.md`.
