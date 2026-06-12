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
- Scenario suggestions expose route and pair evidence in `scenario.usb_routes[]`
  and `scenario.usb_route_pairs[]` so agents can inspect measured length,
  via-count, imported expected data-line width, measured line width, line-width
  delta, measured pair mismatch, via-count delta, imported expected pair gap,
  measured pair gap, and pair-gap delta before choosing board-specific limits.
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
- Scenario suggestions expose candidate same-net pad/via contacts in
  `scenario.usb_routes[].ground_zone_contacts[]` and, when filled polygons are
  present, `filled_ground_zone_contacts[]`.
- The rule checks D+ and D- only. A routed segment is treated as statically
  referenced when its midpoint is inside a same-layer ground-zone outline.
  In filled-zone mode, that midpoint must be inside a same-layer filled
  polygon. In contact-evidence mode, that polygon or outline must also contain
  same-net pad or via evidence on the same layer.
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
- The rule projects connector/protection placements onto the VBUS route graph
  and checks the routed distance from connector VBUS to the nearest valid VBUS
  protection component.
- Scenario suggestions expose VBUS route evidence in `scenario.usb_routes[]`,
  including measured length, via count, optional imported expected VBUS route
  width, measured minimum VBUS route width, and matching protection component.
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
