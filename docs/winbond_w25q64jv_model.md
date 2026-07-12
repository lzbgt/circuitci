# Winbond W25Q64JV SPI NOR Flash Model

`vendor.winbond.w25q64jv` is a source-backed board-boundary model for Winbond's
W25Q64JV 3 V 64 Mbit serial NOR flash family. It includes static power/pin
metadata plus a reduced generated-SPICE SPI standby observation face.

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
- `CIRCUITCI_W25Q64JV_SPI_OBSERVATION` checks board-level VCC, `/CS`, `/WP`,
  `/HOLD or /RESET`, `CLK`, `DI/IO0`, and high-impedance `DO/IO1` standby
  states while leaving all SPI/QSPI pins high impedance.

## Validation Use

The good fixture powers the flash from a 3.3 V rail and binds every SPI/QSPI
pin to a named board net. The bad fixture powers the same part from a 5 V rail,
which fails `POWER_TREE_VALID` against the source-backed 3.6 V maximum.

`examples/good_winbond_w25q64jv_spi_flash_observation/project.yaml` is a
direct-open GUI fixture that runs the reduced observation model with 3.3 V
`VCC`, `/CS` high, `/WP` high, `/HOLD or /RESET` high, and explicit low idle
bias on `CLK`, `DI/IO0`, and high-impedance `DO/IO1`.

## Boundary

This is not a flash-controller or firmware model. It does not prove SPI command
timing, JEDEC ID, SFDP contents, erase/program sequencing, XIP performance,
write-protection policy, flash image contents, firmware boot behavior,
retention/endurance lifetime, or high-speed layout/signal integrity.
