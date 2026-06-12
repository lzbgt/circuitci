# USB ESD Clamp Protection Contract

`INTERFACE_PROTECTION_REVIEW` supports clamp-only protection devices through
`signal_conditioning.protection_clamps`. This is intended for USB ESD arrays and
similar parts that do not translate between two powered domains.

Example model metadata:

```yaml
signal_conditioning:
  protection_clamps:
    - name: dp
      protected_pin: DP
      reference_pin: GND
      reference: ground
      working_voltage_max_V: 5.5
      line_capacitance_F: 1.0e-12
```

Example scenario:

```yaml
scenarios:
  - name: usb_dp_esd_review
    type: interface_protection
    checks:
      - INTERFACE_PROTECTION_REVIEW
    target:
      component: UESD
    parameters:
      clamp: dp
      max_line_capacitance_F: 2.0e-12
```

The rule checks:

- the protected pin and reference pin are connected,
- the reference pin connects to the declared reference net kind,
- protected-net `nominal_voltage` does not exceed `working_voltage_max_V`,
- declared `line_capacitance_F` fits scenario `max_line_capacitance_F`.

This is static board-validation evidence. It does not prove ESD pulse behavior,
dynamic clamp current, USB eye margin, trace impedance, return path quality, or
connector-layout correctness.

`circuitci suggest-scenarios` emits clamp review templates automatically for
connected models with `signal_conditioning.protection_clamps`. The suggestions
include `parameters.clamp` and `scenario.protection_clamps[]` evidence, but
agents still need to fill `parameters.max_line_capacitance_F` from the actual
interface budget when capacitance screening is part of the sign-off.

`circuitci suggest-scenarios` also emits connector-level
`USB_CONNECTOR_PROTECTION_VALID` templates for connector models that declare
`usb_connector` metadata. Those suggestions include `scenario.usb_connectors[]`
with exposed D+/D-/VBUS nets and any connected `scenario.protection_clamps[]`
evidence. When VBUS is connected to a declared power net, the template sets
`parameters.require_vbus_protection: true` so missing VBUS protection becomes an
executable validation failure.

When `board.layout.placements` has finite coordinates for the USB connector and
the required connected protection components, `suggest-scenarios` also emits a
non-runnable `USB_PROTECTION_PLACEMENT_VALID` template. The suggestion includes
placement coordinates and connector-to-protection `distance_to_target_mm`
evidence, but leaves `parameters.max_connector_to_protection_distance_mm` as
`null` until an agent fills the board-specific ESD/layout rule.

Current fixtures:

- `examples/good_usb_esd_protection`
- `examples/bad_usb_esd_reference`
- `examples/bad_usb_esd_standoff`
- `examples/bad_usb_esd_line_capacitance`
- `examples/good_usb_connector_protection`
- `examples/bad_usb_connector_missing_data_protection`
- `examples/bad_usb_connector_missing_vbus_protection`
- `examples/good_usb_connector_protection_placement`
- `examples/bad_usb_connector_protection_placement_distance`
- `examples/scenario_suggestions_usb_connector_protection`
- `examples/import_kicad_usb_connector_protection_suggestions`

Connector-level validation:

- `USB_CONNECTOR_PROTECTION_VALID` targets a connector component whose model
  declares `usb_connector` pins.
- The rule requires clamp-only protection on D+ and D-.
- VBUS protection is required when the scenario declares
  `parameters.require_vbus_protection: true`.
- `parameters.data_working_voltage_min_V` and
  `parameters.vbus_working_voltage_min_V` optionally enforce minimum clamp
  reverse-standoff voltage.
- This check proves schematic coverage only. It does not prove ESD pulse
  robustness, connector placement, shield strategy, return-path quality, trace
  impedance, or USB eye margin.

Placement-distance validation:

- `USB_PROTECTION_PLACEMENT_VALID` targets the same connector component and
  uses `board.layout.placements` evidence.
- The scenario must declare
  `parameters.max_connector_to_protection_distance_mm`.
- The rule checks D+ and D- protection placement, and also checks VBUS when
  `parameters.require_vbus_protection: true`.
- Connector protection can also require the optional USB shield pin to resolve
  to a declared ground net with `parameters.require_shield_ground: true`. This
  is only a static net-kind guard; it does not validate RC, ferrite,
  chassis-only, spark-gap, or EMC shield-bonding strategy.
- Each required protected net must have clamp-only protection on the same net
  with a valid reference net kind, and at least one matching protection
  component must have finite placement coordinates.
- The nearest matching protection component must be no farther from the
  connector placement than the declared maximum distance.
- This check is a first-order component placement guard. It does not inspect
  routed trace order, trace length, via count, parasitic inductance, shield
  bonding, return path continuity, differential impedance, ESD pulse waveform,
  or USB eye margin.

Connector-orientation validation:

- `USB_CONNECTOR_ORIENTATION_VALID` targets the same connector component and
  uses `board.layout.placements.<connector>.rotation_deg` evidence.
- The scenario must declare `parameters.expected_connector_rotation_deg` and
  `parameters.max_connector_rotation_error_deg` from the board-edge or enclosure
  mechanical rule.
- Scenario suggestions can use imported straight `Edge.Cuts` outline segments
  to report nearest-board-edge evidence and prefill the expected rotation from
  the inferred outward normal, but this is still only a starting point for the
  footprint-specific mechanical convention.
- The rule compares rotations modulo `360 deg`, using the smallest angular
  error. It is a static footprint-orientation screen; it does not prove
  enclosure clearance, cable insertion clearance, connector keepouts, or
  mechanical robustness.

Connector edge-proximity validation:

- `USB_CONNECTOR_EDGE_PROXIMITY_VALID` targets the same connector component and
  uses `board.layout.placements` plus straight
  `board.layout.outline.segments` evidence.
- The scenario must declare
  `parameters.max_connector_to_board_edge_distance_mm` from the connector,
  enclosure, or panel-entry mechanical rule.
- The rule measures the nearest supported `fabrication`/`courtyard` footprint
  drawing extent to a straight board-edge segment when imported footprint
  evidence is available. It falls back to connector-center distance for older
  Board IR without footprint graphics.
- It does not prove connector body overhang, keepout, insertion clearance,
  panel alignment, or outline features not captured as straight segments.

Route-geometry validation:

- `USB_ROUTE_GEOMETRY_VALID` targets the same connector component and uses
  `board.layout.routes` evidence.
- The scenario must declare `parameters.max_data_line_route_length_mm`,
  `parameters.max_data_line_via_count`,
  `parameters.max_connector_to_protection_route_distance_mm`, and
  `parameters.max_component_to_route_distance_mm`,
  `parameters.max_data_pair_length_mismatch_mm`, and
  `parameters.max_data_pair_via_count_delta`.
- The rule checks D+ and D- only. VBUS route validation should use a separate
  `USB_VBUS_ROUTE_VALID` rule because its constraints are different from
  data-line geometry.
- The rule also checks D+/D- length mismatch and via-count symmetry using the
  imported route evidence.
- When `parameters.max_data_line_width_delta_mm` is present, it checks data-line
  segment widths against imported `diff_pair_width_mm` or `track_width_mm`.
- When `parameters.max_data_pair_gap_delta_mm` is present, it checks the
  edge-to-edge gap of overlapping parallel D+/D- segments against imported
  `diff_pair_gap_mm`.
- When `parameters.require_route_pad_contact_evidence` is true, connector to
  protection route-order checks use imported same-net connector/protection pad
  centers from `board.layout.pads`, and each pad center must project onto a
  matching route layer within `parameters.max_component_to_route_distance_mm`.
  When supported imported pad shape/size evidence exists, the route must touch
  the pad copper extent instead of only projecting near the pad center. Without
  that parameter, the rule keeps the older component-placement projection
  behavior for hand-authored route evidence.
- Scenario suggestions expose route and pair evidence in `scenario.usb_routes[]`
  and `scenario.usb_route_pairs[]` so agents can inspect measured length,
  via-count, imported expected data-line width, measured line width, line-width
  delta, measured pair mismatch, via-count delta, imported expected pair gap,
  measured pair gap, and pair-gap delta before choosing board-specific limits.
  When same-net connector/protection pad centers are available, each
  `scenario.usb_routes[]` entry also reports the pad records and measured
  connector-to-protection pad route distance used by
  `require_route_pad_contact_evidence`. When supported pad geometry exists,
  pad-to-route distance is reported as `0.0` for route copper touching pad
  copper; incomplete or unsupported pad geometry falls back to center
  projection. Pad records may include imported KiCad kind, shape, size,
  rotation, and drill evidence in addition to center/layer data.
- The rule projects connector/protection placements onto the imported routed
  segments and then computes distance along the route graph, not straight-line
  distance.
- This check is still static layout evidence. It does not prove USB eye margin,
  impedance, skew, return-path continuity, shield bonding, or ESD pulse
  robustness.

- `USB_RETURN_PATH_VALID` targets the same connector component and uses
  `board.layout.routes` plus `board.layout.zones` evidence.
- The scenario must declare
  `parameters.max_data_line_unreferenced_length_mm`.
- The scenario may declare
  `parameters.max_data_via_to_ground_stitch_distance_mm` to require nearby
  ground-net stitching vias for USB data route vias.
- The scenario may declare `parameters.require_filled_zone_coverage: true` to
  use saved `filled_polygons` instead of intended zone outlines for data-route
  midpoint coverage.
- The scenario may declare
  `parameters.min_data_line_filled_zone_edge_clearance_mm` to require each
  D+/D- route segment midpoint to sit at least that far from the nearest
  same-layer filled ground-copper polygon edge.
- The scenario may declare `parameters.require_ground_zone_contact_evidence:
  true` to require the same-layer ground zone to contain imported same-net pad
  or route-via contact evidence before it counts as a return-path reference.
  For supported imported pad shapes (`rect`, `circle`, `oval`), pad contact is
  checked against the pad copper extent; incomplete or unsupported pad geometry
  falls back to pad-center containment.
- Scenario suggestions expose candidate same-net pad/via contacts in
  `scenario.usb_routes[].ground_zone_contacts[]` and, when filled polygons are
  present, `filled_ground_zone_contacts[]`.
- The rule checks D+ and D- only. A routed segment is treated as statically
  referenced when its midpoint is inside a same-layer ground-zone outline.
  In filled-zone mode, that midpoint must be inside a same-layer filled
  polygon. In contact-evidence mode, that polygon or outline must also contain
  same-net pad or via evidence on the same layer. When filled-zone mode and
  contact-evidence mode are both enabled, pad copper or via contact evidence
  must overlap the same saved `filled_polygon` island as the route midpoint.
- When stitching-via distance is enabled, a data via passes only if a ground
  via whose layer list covers the same transition is within the declared
  distance.
- This check is still first-order layout evidence. It does not prove zone fill
  island connectivity, adjacent-plane reference, stitching-via inductance or
  density, impedance, eye margin, or EMC behavior.

- `USB_VBUS_ROUTE_VALID` targets the same connector component and uses
  `board.layout.routes` evidence for the connector VBUS net.
- The scenario must declare `parameters.max_vbus_route_length_mm`,
  `parameters.max_vbus_via_count`,
  `parameters.max_connector_to_vbus_protection_route_distance_mm`, and
  `parameters.max_component_to_route_distance_mm`.
- When `parameters.min_vbus_route_width_mm` is present, every imported VBUS
  route segment must be at least that wide.
- By default, the rule projects connector/protection placements onto the VBUS
  route graph and checks the routed distance from connector VBUS to the nearest
  valid VBUS protection component. When
  `parameters.require_vbus_route_pad_contact_evidence` is true, the same
  route-order check uses imported same-net connector VBUS and protection pad
  evidence from `board.layout.pads` instead of component placement centers.
  When supported imported pad shape/size evidence exists, the route must touch
  the pad copper extent instead of only projecting near the pad center.
- Scenario suggestions expose VBUS route evidence in `scenario.usb_routes[]`,
  including measured length, via count, optional imported expected VBUS route
  width, measured minimum VBUS route width, and matching protection component.
  When imported pad evidence is available, suggestions also expose connector
  and protection pad records plus measured connector-to-protection pad route
  distance. Pad records may include imported KiCad kind, shape, size, rotation,
  and drill evidence in addition to center/layer data.
- This check is still static layout evidence. It does not prove VBUS ampacity,
  fuse trip behavior, inrush current, voltage drop under load, temperature
  rise, or ESD pulse robustness.

Datasheet-backed model pack:

- `libs/vendor/ti/protection/tpd2eusb30.model.yaml`
- `docs/ti_tpd2eusb30_model.md`
- `examples/good_ti_tpd2eusb30_usb_esd`
- `examples/bad_ti_tpd2eusb30_usb_esd_standoff`
- `examples/bad_ti_tpd2eusb30_usb_esd_capacitance`
- `libs/vendor/nexperia/protection/prtr5v0u2x.model.yaml`
- `docs/nexperia_prtr5v0u2x_model.md`
- `examples/good_nexperia_prtr5v0u2x_usb_esd`
- `examples/bad_nexperia_prtr5v0u2x_usb_esd_reference`
- `examples/bad_nexperia_prtr5v0u2x_usb_esd_capacitance`
- `examples/import_kicad_prtr5v0u2x_usb_esd_suggestions`
- `docs/kicad_usb_connector_protection_suggestion_fixture.md`
