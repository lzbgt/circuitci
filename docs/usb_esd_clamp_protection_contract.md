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
  power-path/layout rule because its constraints are different from data-line
  geometry.
- The rule also checks D+/D- length mismatch and via-count symmetry using the
  imported route evidence.
- When `parameters.max_data_line_width_delta_mm` is present, it checks data-line
  segment widths against imported `diff_pair_width_mm` or `track_width_mm`.
- When `parameters.max_data_pair_gap_delta_mm` is present, it checks the
  edge-to-edge gap of overlapping parallel D+/D- segments against imported
  `diff_pair_gap_mm`.
- Scenario suggestions expose the same pair evidence in
  `scenario.usb_route_pairs[]` so agents can inspect the measured mismatch and
  via-count delta before choosing board-specific limits.
- The rule projects connector/protection placements onto the imported routed
  segments and then computes distance along the route graph, not straight-line
  distance.
- This check is still static layout evidence. It does not prove USB eye margin,
  impedance, skew, return-path continuity, shield bonding, or ESD pulse
  robustness.

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
