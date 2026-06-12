# Silicon Labs CP2102N Sources

Retrieved on 2026-06-12.

## Primary Source

- Official Silicon Labs data sheet:
  <https://www.silabs.com/documents/public/data-sheets/cp2102n-datasheet.pdf>
- Local copy:
  `docs/research/datasheets/silabs/cp2102n-datasheet.pdf`
- SHA-256:
  `32fbab0ba17f394ab76fbbd8129bddad228c1c486a1693d96ea5905936e437fe`

## Extracted Facts Used In `vendor.silabs.cp2102n`

- CP2102N is a USB-to-UART bridge with integrated USB full-speed controller,
  USB transceiver, oscillator, and UART.
- Recommended operating voltages:
  - `VDD`: `3.0 V` to `3.6 V`.
  - `VIO`: `1.71 V` to `VDD`.
  - `VREGIN`: `3.0 V` to `5.25 V`.
- Normal operating current is listed as `9.5 mA` typical at 115200 baud and
  `13.7 mA` typical at 3 Mbaud. The model uses `14 mA` as a static board-level
  current estimate for power-tree budgeting, not a worst-case silicon limit.
- The integrated 5 V regulator output on `VDD` is `3.1 V` to `3.6 V` in
  regulation range, with up to `100 mA` total output current and `0.8 V`
  dropout at `100 mA`.
- GPIO/UART input threshold behavior is referenced to `VIO`:
  - `VIH >= VIO - 0.6 V`.
  - `VIL <= 0.6 V`.
  - GPIO levels are undefined whenever `VIO < 1 V`.
- Absolute maximum ratings include:
  - `VDD` and `VIO`: `-0.3 V` to `4.2 V`.
  - `VREGIN`: `-0.3 V` to `5.8 V`.
  - UART/GPIO/VBUS/RSTb and other non-power non-USB pins:
    `-0.3 V` to `5.8 V` when `VIO > 3.3 V`, or `-0.3 V` to `VIO + 2.5 V`
    when `VIO < 3.3 V`.
- QFN24 pin definitions include `VIO`, `VDD`, `VREGIN`, `VBUS`, `RSTb`,
  `D+`, `D-`, `TXD`, `RXD`, `RTS`, `CTS`, `DTR`, `DSR`, `DCD`, `RI`, and
  `GND`.
- The data sheet recommends a `1 kOhm` pull-up on `RSTb` to `VIO`, or to `VDD`
  when `VIO` is tied to `VDD`.

## Modeling Notes

The current model is intentionally board-level. It is useful for power-tree,
UART bootloader, and preliminary backdrive screening. It is not a USB PHY model,
not a detailed USB-host/software model, and not final I/O injection-current
sign-off.
