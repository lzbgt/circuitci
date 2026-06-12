# KiCad USB Connector Protection Suggestion Fixture

`examples/import_kicad_usb_connector_protection_suggestions/` proves that a
native KiCad schematic can feed connector-level USB protection validation
suggestions.

The fixture maps:

- `J1`, a USB2 connector, to `generic.connector.usb2`.
- `UESD`, a TPD2EUSB30 data-line ESD device, to `vendor.ti.tpd2eusb30`.
- `UVBUS`, a VBUS clamp, to `generic.protection.vbus_esd_basic`.

After import, `suggest-scenarios` emits a runnable
`USB_CONNECTOR_PROTECTION_VALID` template:

- `J1.D+ -> net_usb_dp`
- `J1.D- -> net_usb_dm`
- `J1.VBUS -> net_usb_vbus`
- `UESD.d1_plus` protects `net_usb_dp`
- `UESD.d1_minus` protects `net_usb_dm`
- `UVBUS.vbus` protects `net_usb_vbus`

The suggestion remains schematic-level evidence. It does not prove connector
placement, ESD part placement, USB differential impedance, shield strategy,
return-path quality, or ESD pulse performance.

The same fixture directory also includes `board.kicad_pcb`. A regression chains:

1. `import-kicad-schematic` for connector and ESD connectivity.
2. `import-kicad-pcb` for `board.layout.placements`.
3. `suggest-scenarios` for `USB_PROTECTION_PLACEMENT_VALID`.

That enriched flow emits connector-to-protection distance evidence:

- `J1 -> UESD`: `1.0 mm` for D+ and D-
- `J1 -> UVBUS`: `1.5 mm` for VBUS

The placement suggestion remains non-runnable until an agent fills
`parameters.max_connector_to_protection_distance_mm` from the board's actual
ESD/layout rule.
