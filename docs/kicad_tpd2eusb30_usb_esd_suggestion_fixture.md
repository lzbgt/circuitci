# KiCad TPD2EUSB30 USB ESD Suggestion Fixture

`examples/import_kicad_tpd2eusb30_usb_esd_suggestions/` proves that a native
KiCad schematic can bind a TPD2EUSB30 USB ESD symbol to the datasheet-backed
`vendor.ti.tpd2eusb30` model and produce clamp-only interface-protection
scenario suggestions.

The schematic contains:

- `UESD`, a TPD2EUSB30 with `D1+` on `USB_DP`, `D1-` on `USB_DM`, and `GND` on
  ground.

The mapping marks `USB_DP` and `USB_DM` as `digital_or_analog` nets with
`nominal_voltage: 3.3` and maps pins `1`, `2`, and `3` to the model's `D1+`,
`D1-`, and `GND` pins.

After import, `circuitci suggest-scenarios` emits runnable
`INTERFACE_PROTECTION_REVIEW` templates:

- `interface_protection_uesd_d1_plus` for `net_usb_dp`,
- `interface_protection_uesd_d1_minus` for `net_usb_dm`,
- standoff limit `5.5 V`,
- line capacitance `0.7 pF`.

The suggestions still require an agent to fill `parameters.max_line_capacitance_F`
from the real USB interface budget when capacitance screening is part of the
sign-off.
