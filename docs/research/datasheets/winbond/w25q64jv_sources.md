# Winbond W25Q64JV Source Notes

## Retained Source

| Document | Source URL | Local copy | SHA-256 |
| --- | --- | --- | --- |
| W25Q64JV-DTR Rev. L datasheet | <https://www.winbond.com/resource-files/W25Q64JV_DTR%20RevL%2004272026%20Plus.pdf> | `docs/research/datasheets/winbond/w25q64jv_dtr_rev_l_2026.pdf` | `b408f8a61e7f1d22a3a92ec9a6619f66d31fb8540bd3797b649f407268748bdc` |

Retrieved on 2026-07-05. The official Winbond search result pointed to
`DA00-W25Q64JV`, whose document page resolved to the retained Rev. L PDF.

## Modeled Facts

- The W25Q64JV is a 64 Mbit / 8 Mbyte serial NOR flash device for standard SPI,
  Dual SPI, Quad SPI, QPI, and DTR access.
- The retained datasheet lists single-supply operation from 2.7 V to 3.6 V for
  the 104 MHz class and 3.0 V to 3.6 V for the 133 MHz class. The model uses
  the broad 2.7 V to 3.6 V board-level rail range.
- The 8-pin package pin roles are `/CS`, `DO/IO1`, `/WP/IO2`, `GND`, `DI/IO0`,
  `CLK`, `/HOLD or /RESET/IO3`, and `VCC`.
- The source-backed active write/program/erase current class is 25 mA maximum.
  Standby current is 50 uA maximum and power-down current is 15 uA maximum.
- The model records input/output capacitance, VCC-ratio logic thresholds,
  256-byte page size, 4 KiB sector size, 100k minimum program-erase endurance,
  and 20-year minimum data retention as datasheet metadata.
- The generated-SPICE face is intentionally high impedance and only observes
  the surrounding board's VCC, standby chip-select, write-protect, hold/reset,
  clock, and data-line idle-bias states.

## Model Boundary

`vendor.winbond.w25q64jv` is a board-boundary model. It supports power-tree
voltage/current screening, explicit SPI/QSPI pin binding review, and a reduced
generated-SPICE standby board-observation model.

The model does not emulate SPI commands, JEDEC ID, SFDP tables, erase/program
state machines, write-protect policy, XIP performance, flash content, firmware
boot behavior, retention/endurance lifetime, or signal-integrity/layout timing.
