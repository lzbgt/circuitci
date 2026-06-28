# Silicon Labs CP2102N Model

## Sources

- Official Silicon Labs data sheet:
  `docs/research/datasheets/silabs/cp2102n-datasheet.pdf`
- Research note:
  `docs/research/datasheets/silabs/cp2102n_sources.md`

## Modeled Facts

The `vendor.silabs.cp2102n` model captures board-level facts needed by common
IoT validation:

- `VREGIN` accepts `3.0 V` to `5.25 V`.
- `VDD` is modeled as a `3.0 V` to `3.6 V` power rail.
- `VIO` is modeled as a `1.71 V` to `3.6 V` I/O rail.
- The integrated regulator is represented with `power_conversion` from
  `VREGIN` to `VDD`, `0.8 V` dropout at the 100 mA data-sheet condition, and
  `100 mA` maximum regulator output current.
- GPIO/UART thresholds are referenced to a normal `3.3 V` I/O use case:
  `VIH >= 2.7 V` and `VIL <= 0.6 V`.
- UART and modem-control pins are represented for serial bootloader and
  host-controlled reset/boot-strap validation.
- `RSTb` is available as an input, with the data-sheet pull-up recommendation
  documented in model metadata.

## Validation Use

`POWER_TREE_VALID` can now catch a common CP2102N board error: connecting `VDD`
to a 5 V rail. `VREGIN` may be 5 V, but `VDD` must remain in the 3.0-3.6 V
range. The same model can participate in `UART_BOOTLOADER_SYNC` and preliminary
backdrive screening without embedding Silicon Labs-specific logic in the
validator.

The model also has a reduced generated-SPICE observation face,
`CIRCUITCI_CP2102N_USB_UART`, backed by
`models/spice/generic/analog_behavioral.lib`. It models a smooth VREGIN-to-VDD
3.3 V rail source and VIO-referenced TXD, RTS, and DTR output-state drivers via
explicit Board IR component parameters:

- `observation_txd_state`
- `observation_rts_state`
- `observation_dtr_state`

`examples/good_silabs_cp2102n_usb_uart_observation` is registered as the GUI
`CP2102N USB-UART` example. It opens with routed schematic metadata, can run
the generated transient observation, and can regenerate model-aware probes and
checks for the placed `UUSB` component through `Create Checks`.

The model is not valid for USB PHY sign-off, full USB enumeration behavior,
transistor-level modem-line behavior, regulator transient/stability sign-off, or
final I/O injection-current sign-off.
