# Microchip ATECC608A Source Notes

## Retained Source

| Document | Source URL | Local copy | SHA-256 |
| --- | --- | --- | --- |
| ATECC608A Microchip CryptoAuthentication Device data sheet summary DS40001977A | <https://ww1.microchip.com/downloads/en/DeviceDoc/40001977A.pdf> | `docs/research/datasheets/microchip/atecc608a_datasheet_summary_40001977a.pdf` | `3e971d6d284ebc64fc81d168788b9abd32da81a90ce1e0bb56037b0ca16a4a26` |

Retrieved on 2026-07-13 from Microchip's official `ww1.microchip.com`
document host. A local text extraction is retained at
`docs/research/datasheets/microchip/atecc608a_datasheet_summary_40001977a.txt`
for agent-side source review.

## Modeled Facts

- ATECC608A is a Microchip CryptoAuthentication secure element with protected
  key/data storage, ECDSA/ECDH/SHA/HMAC/AES support, RNG, and secure-boot use
  cases.
- The retained summary identifies the I2C option as a 1 MHz standard I2C
  interface.
- The retained summary lists 1.8 V to 5.5 V I/O levels and 2.0 V to 5.5 V
  supply voltage.
- The 8-lead SOIC top-view pinout uses pins 1, 2, 3, and 7 as no-connect, pin
  4 as `GND`, pin 5 as `SDA`, pin 6 as `SCL`, and pin 8 as `VCC`.
- The DC table records 3 mA maximum active current while waiting for I/O or
  non-ECC command execution, 14 mA maximum active current during ECC command
  execution with clock divider `0x0`, 800 uA typical idle current, and 150 nA
  maximum sleep current for VCC <= 3.6 V and TA <= 55 C.
- The I2C timing table records 1 MHz maximum SCK frequency and an example
  1.2 kOhm SDA pull-up condition for VCC from 2.0 V to 5.0 V.
- The DC table records output-low voltage up to 0.4 V and output-low current
  up to 4 mA when active over VCC = 2.5 V to 5.5 V.

## Model Boundary

`vendor.microchip.atecc608a` is a board-boundary model. It supports power-tree
voltage/current screening, explicit I2C pin binding review, and a reduced
generated-SPICE board-observation model for VCC and idle I2C pull-ups.

The model does not emulate cryptographic commands, secure key storage,
provisioning state, RNG behavior, secure boot policy, wake/sleep timing,
single-wire operation, I2C transaction content, package authenticity, firmware,
or signal-integrity timing.
