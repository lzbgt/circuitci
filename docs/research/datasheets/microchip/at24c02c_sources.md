# Microchip AT24C02C Source Notes

## Retained Source

| Document | Source URL | Local copy | SHA-256 |
| --- | --- | --- | --- |
| AT24C01C/AT24C02C data sheet DS20006111A | <https://ww1.microchip.com/downloads/en/DeviceDoc/AT24C01C-AT24C02C-I2C-Compatible-Two-Wire-Serial-EEPROM-1Kbit-2Kbit-20006111A.pdf> | `docs/research/datasheets/microchip/at24c01c_at24c02c_datasheet_ds20006111a.pdf` | `b6466f4cce6d4ed5bdaaf3854c7bd2eb10f9258a678d09ccdf233f6b3f2fa424` |

Retrieved on 2026-07-13 from Microchip's official `ww1.microchip.com`
document host. A local text extraction is retained at
`docs/research/datasheets/microchip/at24c01c_at24c02c_datasheet_ds20006111a.txt`
for agent-side source review.

## Modeled Facts

- AT24C02C is a 2 Kbit I2C-compatible two-wire serial EEPROM organized as
  256 x 8 bits.
- The retained datasheet lists low-voltage operation from 1.7 V to 5.5 V.
- The 8-lead SOIC pinout is `A0`, `A1`, `A2`, `GND`, `SDA`, `SCL`, `WP`, and
  `VCC`.
- `SDA` is open-drain and requires an external pull-up not exceeding 10 kOhm.
  `SCL` must be high while the bus is idle or pulled high externally.
- Address pins `A0`/`A1`/`A2` and `WP` have internal pull-down behavior, but
  Microchip recommends connecting them to a known state.
- `WP` at `VCC` inhibits writes to the full array; `WP` at `GND` allows normal
  write operations.
- The model records the 3 mA maximum active write-current class, 6 uA maximum
  5.5 V standby current class, VCC-ratio input thresholds, 8-byte page writes,
  5 ms maximum self-timed write cycle, 1,000,000 write-cycle endurance, and
  100-year data-retention metadata.

## Model Boundary

`vendor.microchip.at24c02c` is a board-boundary model. It supports power-tree
voltage/current screening, explicit I2C EEPROM pin binding review, and a
reduced generated-SPICE board-observation model for VCC, pull-ups,
address-select pins, and write-protect state.

The model does not emulate I2C transactions, acknowledge polling, device
address scanning, EEPROM contents, write-cycle timing, write-protect policy,
retention/endurance lifetime, or high-speed signal-integrity/layout timing.
