# KiCad PRTR5V0U2X USB ESD Suggestion Fixture

`examples/import_kicad_prtr5v0u2x_usb_esd_suggestions/` proves that a native
KiCad schematic can map a rail-to-rail USB ESD protection part to
`vendor.nexperia.prtr5v0u2x` and produce clamp-only interface-protection
suggestions.

The fixture imports:

- `UESD.IO1 -> net_usb_dp`,
- `UESD.IO2 -> net_usb_dm`,
- `UESD.VCC -> net_usb_vbus`,
- `UESD.GND -> gnd`.

After import, `circuitci suggest-scenarios` emits:

- `interface_protection_uesd_io1_to_vcc` for `net_usb_dp`,
- `interface_protection_uesd_io2_to_vcc` for `net_usb_dm`.

Each suggestion carries:

- `reference: power`,
- `reference_pin: VCC`,
- `reference_net: net_usb_vbus`,
- `working_voltage_max_V: 5.5`,
- `line_capacitance_F: 1.5e-12`.

The suggestions remain static screening evidence. Agents still need real USB
line capacitance budgets, PCB layout review, and any ESD-pulse or eye-margin
validation required for sign-off.
