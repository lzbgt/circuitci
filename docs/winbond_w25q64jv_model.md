# Winbond W25Q64JV SPI NOR Flash Model

`vendor.winbond.w25q64jv` is a source-backed static model for Winbond's
W25Q64JV 3 V 64 Mbit serial NOR flash family.

The retained source is Winbond's official Rev. L datasheet:
`docs/research/datasheets/winbond/w25q64jv_dtr_rev_l_2026.pdf`.

## Encoded Facts

- Power pins: `VCC` and `GND`.
- SPI/QSPI board pins: `CS_N`, `DO_IO1`, `WP_N_IO2`, `DI_IO0`, `CLK`, and
  `HOLD_N_RESET_N_IO3`.
- Static supply range: `2.7 V` to `3.6 V`.
- Active program/erase/write-status supply-current class: `25 mA` maximum.
- Datasheet metadata records 64 Mbit density, 256-byte pages, 4 KiB sectors,
  133 MHz high-voltage SPI clock class, 100k minimum program-erase endurance,
  and 20-year minimum retention.

## Validation Use

The good fixture powers the flash from a 3.3 V rail and binds every SPI/QSPI
pin to a named board net. The bad fixture powers the same part from a 5 V rail,
which fails `POWER_TREE_VALID` against the source-backed 3.6 V maximum.

## Boundary

This is not a flash-controller or firmware model. It does not prove SPI command
timing, JEDEC ID, SFDP contents, erase/program sequencing, XIP performance,
write-protection policy, flash image contents, firmware boot behavior,
retention/endurance lifetime, or high-speed layout/signal integrity.
