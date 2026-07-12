# Microchip AT24C02C I2C EEPROM Model

`vendor.microchip.at24c02c` is a source-backed board-boundary model for
Microchip's AT24C02C 2 Kbit I2C-compatible serial EEPROM. It includes static
power/pin metadata plus a reduced generated-SPICE I2C standby observation face.

The retained source is Microchip's official DS20006111A datasheet:
`docs/research/datasheets/microchip/at24c01c_at24c02c_datasheet_ds20006111a.pdf`.

## Encoded Facts

- Power pins: `VCC` and `GND`.
- I2C and configuration pins: `A0`, `A1`, `A2`, `SDA`, `SCL`, and `WP`.
- Static supply range: `1.7 V` to `5.5 V`.
- Active write-current class: `3 mA` maximum.
- Datasheet metadata records 2 Kbit density, 256 x 8 organization, 8-byte
  page writes, 1 MHz Fast Mode Plus at 2.5 V to 5.5 V, 5 ms maximum write
  cycle, 1,000,000 write-cycle endurance, and 100-year retention.
- `CIRCUITCI_AT24C02C_I2C_EEPROM_OBSERVATION` checks board-level VCC, I2C
  pull-ups, `A0`/`A1`/`A2` address-select states, and `WP` write-protect state
  while leaving all signal pins high impedance.

## Validation Use

The good fixture powers the EEPROM from a 3.3 V rail, pulls `SDA`/`SCL` high,
ties `A0`/`A1`/`A2` low for address `0x50`, and ties `WP` low for normal write
operation. The bad fixture powers the same part from a 6 V rail, which fails
`POWER_TREE_VALID` against the source-backed 5.5 V maximum.

`examples/good_microchip_at24c02c_i2c_eeprom_observation/project.yaml` is a
direct-open GUI fixture that runs the reduced observation model with 3.3 V
`VCC`, idle high `SDA`/`SCL`, low address pins, and low `WP`.

## Boundary

This is not an EEPROM protocol or firmware model. It does not prove I2C
transactions, acknowledge polling, address scanning, EEPROM contents, write
cycle timing, write-protect policy, retention/endurance lifetime, or high-speed
layout/signal integrity.
