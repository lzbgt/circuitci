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
- `J1.SHIELD -> gnd`
- `UESD.d1_plus` protects `net_usb_dp`
- `UESD.d1_minus` protects `net_usb_dm`
- `UVBUS.vbus` protects `net_usb_vbus`
- `require_shield_ground: true`

The suggestion remains schematic-level evidence. It does not prove connector
placement, ESD part placement, USB differential impedance, RC/ferrite/chassis
shield strategy, return-path quality, or ESD pulse performance.

The same fixture directory also includes `board.kicad_pcb`. A regression chains:

1. `import-kicad-schematic` for connector and ESD connectivity.
2. `import-kicad-pcb` for `board.layout.placements` and routed USB net
   geometry under `board.layout.routes`, plus ground copper-zone outline
   evidence under `board.layout.zones`.
3. `suggest-scenarios` for `USB_PROTECTION_PLACEMENT_VALID`,
   `USB_ROUTE_GEOMETRY_VALID`, `USB_VBUS_ROUTE_VALID`, and
   `USB_RETURN_PATH_VALID`.

That enriched flow emits connector-to-protection distance evidence:

- `J1 -> UESD`: `1.0 mm` for D+ and D-
- `J1 -> UVBUS`: `1.5 mm` for VBUS

The placement suggestion remains non-runnable until an agent fills
`parameters.max_connector_to_protection_distance_mm` from the board's actual
ESD/layout rule.

The PCB fixture also declares a `USB_HS` net class and a simple custom DRC rule
for USB data length/skew. Import preserves that evidence under
`board.layout.constraints.net_rules`, and the USB route suggestion pre-fills:

- `max_data_line_route_length_mm: 25.0`
- `max_data_pair_length_mismatch_mm: 0.5`

Because the PCB fixture also imports a same-layer GND copper-zone outline over
the USB D+/D- route segment midpoints, the return-path suggestion reports
`unreferenced_route_length_mm: 0.0` for both data lines and leaves
`max_data_line_unreferenced_length_mm: null` for the board-specific
return-path rule. This is outline evidence only; filled-zone connectivity,
plane transitions, stitching vias, and impedance still require richer layout
evidence.
