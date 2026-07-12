# Microchip ATECC608A I2C Secure Element Model

`vendor.microchip.atecc608a` is a source-backed board-boundary model for
Microchip's ATECC608A I2C CryptoAuthentication secure element. It includes
static power/pin metadata plus a reduced generated-SPICE I2C observation face.

The retained source is Microchip's official DS40001977A data sheet summary:
`docs/research/datasheets/microchip/atecc608a_datasheet_summary_40001977a.pdf`.

## Encoded Facts

- Power pins: `VCC` and `GND`.
- I2C pins: `SDA` and `SCL`.
- SOIC no-connect package pins are intentionally omitted from the electrical
  model.
- Static supply range: `2.0 V` to `5.5 V`.
- Datasheet metadata records 1 MHz I2C class, 3 mA non-ECC active current,
  14 mA ECC active current, 800 uA idle current, 150 nA sleep current, and
  4 mA active output-low current.
- `CIRCUITCI_ATECC608A_I2C_SECURE_ELEMENT_OBSERVATION` checks board-level VCC
  and idle-high I2C pull-ups while leaving all pins high impedance.

## Validation Use

The good fixture powers the secure element from a 3.3 V rail and pulls
`SDA`/`SCL` high. The bad fixture powers the same part from a 6 V rail, which
fails `POWER_TREE_VALID` against the source-backed 5.5 V maximum.

`examples/good_microchip_atecc608a_i2c_secure_element_observation/project.yaml`
is a direct-open GUI fixture that runs the reduced observation model.

## Boundary

This is not a cryptographic protocol, provisioning, firmware, secure-boot, or
anti-cloning sign-off model. It does not prove command contents, key storage,
RNG quality, slot policy, wake/sleep timing, single-wire operation, I2C bus
timing margins, or signal integrity.
