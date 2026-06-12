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
2. `import-kicad-pcb` for `board.layout.placements`, Edge.Cuts
   outline segment evidence under `board.layout.outline`, connected pad geometry
   evidence under `board.layout.pads`, routed USB net geometry under
   `board.layout.routes`, plus ground copper-zone outline and saved
   filled-polygon evidence under `board.layout.zones`.
3. `suggest-scenarios` for `USB_PROTECTION_PLACEMENT_VALID`,
   `USB_CONNECTOR_ORIENTATION_VALID`,
   `USB_CONNECTOR_EDGE_PROXIMITY_VALID`,
   `USB_CONNECTOR_BODY_OVERHANG_VALID`,
   `USB_CONNECTOR_COMPONENT_CLEARANCE_VALID`,
   `USB_CONNECTOR_ENTRY_CLEARANCE_VALID`, `USB_ROUTE_GEOMETRY_VALID`,
   `USB_VBUS_ROUTE_VALID`, and `USB_RETURN_PATH_VALID`.

That enriched flow emits connector-to-protection distance evidence:

- `J1 -> UESD`: `1.0 mm` for D+ and D-
- `J1 -> UVBUS`: `1.5 mm` for VBUS

The placement suggestion remains non-runnable until an agent fills
`parameters.max_connector_to_protection_distance_mm` from the board's actual
ESD/layout rule.

Because the PCB fixture also imports `Edge.Cuts` board-outline segment
evidence, the orientation and edge-proximity suggestions include nearest-edge
evidence. The fixture's imported `F.Fab` polygon crosses the nearest board-edge
segment, so `nearest_board_edge.distance_to_connector_mm` is `0.0` with
`connector_edge_reference: footprint_polygon`. The body-overhang suggestion
uses the same polygon evidence and reports `connector_body_overhang_mm: 0.4`.
The fixture also imports one `fp_circle` and one `fp_arc` so
`scenario.usb_connectors[].footprint` exposes curved footprint drawing evidence
for mechanical review. Placement-center distance is still available as the
fallback behavior when no supported footprint drawing evidence exists.

`project_checks.yaml` in the same fixture directory exercises the imported PCB
evidence through `validate`, not just through suggestions. After
`import-kicad-pcb` enriches that project, its
`USB_CONNECTOR_COMPONENT_CLEARANCE_VALID` scenario uses the imported `J1`
fabrication polygon and the imported `UESD` fabrication rectangle. The executable
check reports the measured footprint-to-footprint clearance as `0.5 mm` and
fails against the fixture's
`0.7 mm` minimum connector-to-component clearance limit.

The same imported connector rotation and footprint evidence also emits a
non-runnable `USB_CONNECTOR_ENTRY_CLEARANCE_VALID` template. It computes
`entry_direction_deg: 0.0` from the KiCad placement rotation plus the generic
connector model's `entry_direction_offset_deg: 0.0`, then leaves cable entry
corridor depth/width as explicit mechanical policy inputs. Its
`scenario.usb_connectors[].entry_clearance` evidence reports the nearest
forward obstruction candidate from imported footprint evidence, which is the
`UESD` fabrication rectangle in this fixture.
`project_checks.yaml` also executes that rule with a `0.8 mm` deep by `1.0 mm`
wide corridor, proving the imported `UESD` fabrication rectangle is reported as
an entry-corridor obstruction.

The PCB fixture also declares a `USB_HS` net class and a simple custom DRC rule
for USB data length/skew. Import preserves that evidence under
`board.layout.constraints.net_rules`, and the USB route suggestion pre-fills:

- `max_data_line_route_length_mm: 25.0`
- `max_data_pair_length_mismatch_mm: 0.5`

Because the PCB fixture also imports a same-layer GND copper-zone outline over
the USB D+/D- route segment midpoints, the return-path suggestion reports
`unreferenced_route_length_mm: 0.0` and
`filled_unreferenced_route_length_mm: 0.0` for both data lines, then leaves
`max_data_line_unreferenced_length_mm: null` for the board-specific return-path
rule. It also leaves
`max_data_via_to_ground_stitch_distance_mm: null` so an agent can enable
nearby ground-stitch checks when data vias cross layers, and
`require_filled_zone_coverage: null` so an agent can choose saved
`filled_polygons` over the intended zone outline. When filled polygons are
present, each data route also reports `filled_zone_edge_clearance_min_mm` and
`filled_zone_edge_clearance_segments[]`, while
`min_data_line_filled_zone_edge_clearance_mm: null` remains for the
board-specific filled-copper edge-margin policy. The template also leaves
`require_ground_zone_contact_evidence: null` so an agent can decide whether the
imported same-net pad/via evidence must prove that the ground zone is connected
before it counts as return-path coverage. The route evidence includes
`ground_zone_contacts[]` and `filled_ground_zone_contacts[]`; in this fixture
those lists include imported ground pads such as `J1.GND` inside the same-layer
GND copper. Filled-zone contact evidence is listed only when the imported
contact shares the same saved `filled_polygon` island as the covered route
midpoint. This is geometry, pad-contact, and via-proximity evidence only;
unmodeled filled-zone island connectivity, controlled plane transitions,
stitching-via inductance, and impedance still require richer layout evidence.
