# Scenario Suggestions

`circuitci suggest-scenarios` inspects a Board IR project and writes an
agent-facing YAML artifact with validation scenarios that should be added or
completed.
The artifact is validated by
`schemas/scenario_suggestion_report.schema.json`.

```bash
circuitci suggest-scenarios project.yaml --output out/scenario_suggestions.yaml
```

The command is conservative:

- It emits runnable `power_tree` suggestions when a project declares power nets
  but has no `POWER_TREE_VALID` scenario.
- If a powered rail is fed by a model with `power_switch`, the power-tree
  suggestion includes the switch control pin and required enabled state. It is
  runnable when the control pin is hard-tied to a declared powered rail for a
  high state or to ground for a low state. Other control nets remain
  `runnable: false` until enable-state evidence is confirmed.
- If a board includes a model with `battery_charger`, the power-tree suggestion
  is runnable when programmed charge current is already a component parameter or
  can be derived from exactly one source-backed PROG/ISET resistor. If neither
  evidence path is present, the suggestion is marked `runnable: false` and
  records the parameter the agent must derive from charger evidence or board
  configuration.
- If a powered output rail is fed by a model with `power_mux` and the selected
  input component parameter is missing, the power-tree suggestion stays runnable
  only when the board state proves exactly one declared mux input rail is
  powered while the output rail is powered. Otherwise it is marked
  `runnable: false` and records the exact parameter plus allowed source names.
- If a model declares `power_conversion`, the power-tree suggestion includes
  `scenario.regulators[]` with the regulator component, input/output pins,
  input/output nets, and declared dropout/current/startup/capacitance limits.
  When a capacitance requirement is declared, the entry also includes measured
  input/output support capacitance and the capacitor component IDs contributing
  to each rail. When output inductance limits are declared, the entry includes
  the converter `switch_pin`, `switch_net`, output inductance limits, measured
  direct switch-to-output support inductance, and the contributing inductor
  component IDs. This gives agents the exact regulator evidence that
  `POWER_TREE_VALID` will execute.
- If a model declares `reset_supervisor`, the power-tree suggestion includes
  `scenario.reset_supervisors[]` with the supervisor component, monitored
  pin/net, reset output pin/net, and threshold range. This points agents at the
  exact threshold check that `POWER_TREE_VALID` will execute.
- It emits runnable `IO_VOLTAGE_COMPATIBLE` suggestions when same-net digital
  output/input pairs have modeled I/O voltage metadata and no existing
  `power_tree` scenario declares that check.
  The suggestion includes `scenario.paths[]` entries with the implicated
  driver, receiver, and net so agents can inspect the exact interfaces the
  static rule will scan.
- It emits reset templates when a component model declares reset behavior, the
  reset pin is connected, and the target power rail declares `power_valid_at_us`.
- Reset suggestions are runnable when an active-low reset net has explicit RC
  evidence: a mapped resistor from the reset net to the target power rail and a
  mapped capacitor from reset to ground. In that case `reset_release_at_us` is
  derived from `-R*C*ln(1 - VIH/Vrail)` plus the rail `power_valid_at_us`.
- Reset suggestions also become runnable when exactly one matching
  `board.runtime.reset_release[]` record supplies explicit reset-release
  timing for the target component and reset pin. This covers oscilloscope,
  simulation, reset-supervisor, or host-control timing evidence that has already
  been reviewed outside the schematic RC heuristic.
- It emits runnable `CONTROL_LINE_RELEASE_SEQUENCE` suggestions from complete
  `board.runtime.control_line_sequences[]` records. Those records must already
  contain the target, required boot mode, timing, reduced control effects, and
  explicit control-line events; CircuitCI does not infer initial line states or
  transistor/diode/RC behavior.
- `BOOT_STRAP_DEFINED` suggestions become runnable when every required strap
  pin is connected and the connected net is directly proven high by a declared
  powered rail or low by ground. Digital nets, resistor-bias-only evidence, and
  other observed states still keep the template non-runnable until explicit
  strap-state evidence is supplied.
- Other reset suggestions are marked `runnable: false` until real
  `timing.reset_release_at_us` evidence is filled from a reset supervisor,
  control-line model, firmware/host trace, or analog waveform.
- It emits GPIO backdrive templates when a powered output-capable pin shares a
  net with an unpowered input-capable pin, model electrical metadata is present,
  and no existing `GPIO_BACKDRIVE` scenario covers that driver/victim path.
- GPIO backdrive templates become runnable only when matching
  `board.runtime.gpio_backdrive[]` evidence confirms the driver can be high
  while the victim rail is unpowered and supplies the actual protection-path
  series resistance.
- It emits interface-protection templates for component models that declare
  `signal_conditioning.channels`, such as level shifters, protection devices,
  series resistors, or bus switches.
- Channel-style interface-protection templates become runnable only when a
  non-generic datasheet-backed model provides complete direction,
  voltage-domain, supply-power-state, supply-constraint, and
  unpowered-isolation metadata. If the declared enable/OE pin is directly tied
  to a powered rail or ground in the disabled state, the suggestion includes
  that pin-state evidence. Generic or incomplete channel models remain
  `runnable: false` review prompts.
- It emits clamp-only interface-protection templates for component models that
  declare `signal_conditioning.protection_clamps`, such as USB ESD arrays. The
  template includes `parameters.clamp` plus `scenario.protection_clamps[]`
  evidence with protected/reference pins and nets, standoff voltage, and line
  capacitance. Ground-referenced clamps such as TPD2EUSB30 and power-referenced
  rail-to-rail clamps such as PRTR5V0U2X are both represented. Agents should
  fill `parameters.max_line_capacitance_F` from the real interface budget when
  capacitance screening is required.
- It emits runnable `USB_CONNECTOR_PROTECTION_VALID` templates for connector
  models that declare `usb_connector` metadata. The template includes
  `scenario.usb_connectors[]` with connector pins/nets and any connected
  `scenario.protection_clamps[]` evidence found on D+, D-, and VBUS. If VBUS is
  connected to a declared power net, the template sets
  `parameters.require_vbus_protection: true` so validation fails closed when no
  VBUS clamp is modeled. If the optional connector shield pin is connected to a
  declared net, the template sets `parameters.require_shield_ground: true` so
  validation checks the simple shield-to-ground case and asks agents to model
  RC, ferrite, chassis-only, or spark-gap strategies explicitly before treating
  the board as EMC-ready.
- It emits `USB_PROTECTION_PLACEMENT_VALID` templates when the USB
  connector and required connected protection components already have finite
  `board.layout.placements` evidence. The template includes connector/clamp
  placement coordinates and `distance_to_target_mm` evidence, but leaves
  `parameters.max_connector_to_protection_distance_mm` as `null` until an agent
  fills the board-specific ESD/layout rule. If
  `board.layout.constraints.usb_connector.max_connector_to_protection_distance_mm`
  is present, the template becomes runnable. CircuitCI does not invent placement
  limits from component coordinates.
- It emits `USB_CONNECTOR_ORIENTATION_VALID` templates when the USB
  connector placement includes imported `rotation_deg` evidence. The template
  includes the measured placement rotation in `scenario.usb_connectors[]`.
  When `board.layout.outline.segments` contains imported board-edge segment
  evidence, the template also reports `nearest_board_edge` evidence and
  pre-fills `expected_connector_rotation_deg` from the nearest edge's inferred
  outward normal minus optional entry-direction offset evidence from KiCad
  footprint properties, KiCad mapping metadata, or the component model's
  `usb_connector.entry_direction_offset_deg`. `nearest_board_edge` keeps both
  raw `outward_normal_deg`, offset-aware `expected_connector_rotation_deg`, and
  `connector_entry_direction_offset_source` evidence. `max_connector_rotation_error_deg`
  remains `null` until an agent supplies a board-specific tolerance or
  `board.layout.constraints.usb_connector.max_connector_rotation_error_deg` is
  present. With both expected rotation and tolerance evidence, the template
  becomes runnable. The inferred expected rotation should be reviewed against
  the connector footprint convention. Without outline evidence,
  both orientation parameters remain manual.
- It emits `USB_CONNECTOR_EDGE_PROXIMITY_VALID` templates when the
  USB connector has finite placement evidence and
  `board.layout.outline.segments` contains usable board-edge segment evidence.
  The template includes `nearest_board_edge.distance_to_connector_mm` and
  `nearest_board_edge.connector_edge_reference` in
  `scenario.usb_connectors[]`. That distance uses supported
  `fabrication`/`courtyard` footprint `fp_line`, `fp_rect`, `fp_poly`,
  `fp_circle`, or `fp_arc` evidence when available and falls back to
  placement-center distance otherwise. Imported KiCad outline segments also
  expose optional `nearest_board_edge.source_primitive`,
  `source_primitive_index`, `sample_index`, `sample_count`, `contour_index`,
  and `boundary_role` evidence so an agent can distinguish native `gr_line`
  edges from sampled `gr_rect`, `gr_poly`, `gr_circle`, or `gr_arc` edges and
  external contours from cutouts. Segments marked `boundary_role: cutout` are
  not selected as USB connector entry edges. When imported footprint drawing evidence exists,
  `scenario.usb_connectors[].footprint` also reports transformed
  `fp_line`/`fp_rect`/`fp_poly`/`fp_circle`/`fp_arc` body, courtyard, or
  silkscreen evidence for mechanical review. The template leaves
  `max_connector_to_board_edge_distance_mm` as `null` until an agent fills the
  connector/enclosure mechanical rule. If
  `board.layout.constraints.usb_connector.max_connector_to_board_edge_distance_mm`
  is present, the template becomes runnable.
- It emits `USB_CONNECTOR_BODY_OVERHANG_VALID` templates when the
  USB connector has finite placement evidence, board-edge outline segment
  evidence, and supported `fabrication`/`courtyard` footprint `fp_line`,
  `fp_rect`, `fp_poly`, `fp_circle`, or `fp_arc` evidence. The template reports
  `nearest_board_edge.connector_body_overhang_mm`,
  `connector_edge_reference`, `footprint_graphic_layer`, and
  `footprint_graphic_kind` in `scenario.usb_connectors[]`. It leaves
  `max_connector_body_overhang_mm` as `null` until an agent fills the
  connector, enclosure, panel, or assembly mechanical limit. If
  `board.layout.constraints.usb_connector.max_connector_body_overhang_mm` is
  present, the template becomes runnable.
- It emits `USB_CONNECTOR_COMPONENT_CLEARANCE_VALID` templates
  when the USB connector has supported footprint evidence and at least one
  other component has placement or footprint evidence. The template includes
  the connector footprint evidence plus
  `scenario.usb_connectors[].nearest_component_clearance`, which reports the
  nearest component, measured 2D clearance, and whether each side used
  footprint or placement-center evidence. It leaves
  `min_connector_to_component_clearance_mm` as `null` until an agent fills the
  connector keepout, cable insertion, enclosure, or assembly clearance rule. If
  `board.layout.constraints.usb_connector.min_connector_to_component_clearance_mm`
  is present, the template becomes runnable.
- It emits `USB_CONNECTOR_ENTRY_CLEARANCE_VALID` templates when the USB
  connector has imported placement rotation and supported
  `fabrication`/`courtyard` footprint evidence. The template copies
  `entry_direction_deg` from imported placement rotation plus optional KiCad
  footprint-property, KiCad mapping, or component-model entry-direction offset
  evidence, includes the connector placement and footprint evidence, and includes
  `scenario.usb_connectors[].entry_clearance` with connector-front projection
  plus `entry_direction_source`, optional `entry_direction_offset_deg`, optional
  `entry_clearance_depth_source`,
  `suggested_min_cable_entry_clearance_depth_mm`, optional
  `entry_clearance_width_source`, optional
  `suggested_cable_entry_clearance_width_mm`, aperture source/front/center
  evidence, optional imported mapping, component-model, or footprint-property
  aperture offsets and width, optional
  `aperture_min_effective_clearance_width_mm`, and the nearest
  forward obstruction candidate when imported component footprint or placement
  evidence is available. Obstruction evidence reports depth in the entry
  direction, lateral offset from the aperture centerline, and whether the
  obstruction came from footprint or placement-center evidence.
  The template is runnable when explicit connector metadata provides both
  `min_cable_entry_clearance_depth_mm` and `cable_entry_clearance_width_mm`.
  Otherwise it stays non-runnable, preserves any available metadata-derived
  value, and leaves missing values as `null` until an agent fills them from
  connector, plug, panel, enclosure, or assembly mechanical drawings.
- It emits `USB_ROUTE_GEOMETRY_VALID` templates when the USB connector, D+/D-
  protection components, placements, and
  `board.layout.routes` evidence are present. The template includes
  `scenario.usb_routes[]` with data-line net, route length, via count, and the
  matching protection component. When imported net rules include route width,
  each route also reports `expected_data_line_width_mm`,
  `measured_data_line_width_mm`, and `data_line_width_delta_mm`. It also
  includes `scenario.usb_route_pairs[]` with computed D+/D- route lengths,
  length mismatch, via counts, via-count delta, and imported
  `expected_data_pair_gap_mm`, `measured_data_pair_gap_mm`, and
  `data_pair_gap_delta_mm` when available. If KiCad PCB import found
  applicable custom DRC `length` or `skew` constraints, the template pre-fills
  `max_data_line_route_length_mm` and
  `max_data_pair_length_mismatch_mm` and becomes runnable. Without both
  imported limits, it remains non-runnable until an agent supplies the missing
  board-specific policy. Via-count, width tolerance, gap tolerance, and ESD
  placement limits remain optional `null` checks until an agent supplies those
  policies. `require_route_pad_contact_evidence` also remains `null` until route
  distance limits are supplied. When imported connector and protection pad
  evidence exists for both data lines, each matching `scenario.usb_routes[]`
  entry reports
  `connector_pad`, `protection_pad`,
  `connector_pad_to_route_distance_mm`,
  `protection_pad_to_route_distance_mm`, and
  `connector_to_protection_pad_route_distance_mm` when the imported pad evidence
  can be matched to the routed net on compatible copper layers. When supported
  pad geometry is present, pad-to-route distance is reported as `0.0` when the
  routed copper touches the pad copper; otherwise it falls back to pad-center
  projection distance. Pad records include center coordinates, layers, and
  optional imported KiCad kind/shape/size/rotation/drill evidence.
- It emits `USB_VBUS_ROUTE_VALID` templates when the USB connector,
  VBUS protection component, placements, and `board.layout.routes` evidence are
  present. The template includes `scenario.usb_routes[]` with VBUS net, route
  length, via count, optional imported `expected_vbus_route_width_mm`, measured
  `measured_vbus_route_width_min_mm`, and the matching protection component.
  If imported net rules include a VBUS `length` constraint, the template
  pre-fills `max_vbus_route_length_mm` and becomes runnable. If imported net
  rules also include `track_width_mm`, the template can pre-fill optional
  `min_vbus_route_width_mm` when no explicit VBUS route-width policy is present.
  Explicit `board.layout.constraints.usb_vbus_route` metadata can pre-fill
  VBUS via-count, minimum width, connector-to-protection route distance,
  component-to-route distance, and pad-contact policy. Without that metadata,
  those optional checks remain `null` until an agent supplies board-specific
  policy. When imported connector VBUS and protection pad evidence exists, the
  VBUS `scenario.usb_routes[]` entry reports
  `connector_pad`, `protection_pad`, pad geometry, pad-to-route distances, and
  `connector_to_protection_pad_route_distance_mm`.
- It emits non-runnable `USB_RETURN_PATH_VALID` templates when USB D+/D-
  `board.layout.routes` evidence and same-layer ground-zone outlines under
  `board.layout.zones` are present. The template includes each data net's
  `unreferenced_route_length_mm` plus `unreferenced_segments[]` midpoint/layer
  evidence from zone outlines. When saved filled-zone evidence exists, it also
  includes `filled_unreferenced_route_length_mm` and
  `filled_unreferenced_segments[]` so agents can compare intended outline
  coverage against actual filled-polygon coverage. It also reports
  `filled_zone_edge_clearance_min_mm` and
  `filled_zone_edge_clearance_segments[]` when filled polygons are present, so
  agents can see the nearest filled-copper edge margin before choosing a
  minimum-clearance policy. If
  `board.layout.constraints.usb_return_path.max_data_line_unreferenced_length_mm`
  is present, it pre-fills `max_data_line_unreferenced_length_mm` and the
  template becomes runnable. Optional fields in the same constraint object can
  pre-fill `max_data_via_to_ground_stitch_distance_mm`,
  `require_filled_zone_coverage`,
  `min_data_line_filled_zone_edge_clearance_mm`, and
  `require_ground_zone_contact_evidence`; otherwise those parameters remain
  `null` until an agent supplies board-specific policy. Each route can include
  `ground_zone_contacts[]` and, when saved filled polygons exist,
  `filled_ground_zone_contacts[]`; these list imported same-net pad or via
  contacts found inside the relevant same-layer ground reference geometry. For
  supported imported pad geometry, suggestions list pad contacts when pad copper
  overlaps the reference geometry, even if the pad center is outside it. In
  filled-zone evidence, contacts are only listed when they share a saved
  `filled_polygon` island with at least one covered route segment midpoint.
- It emits runnable `CLOCK_SOURCE_VALID` templates when a component model
  declares `clock_sources[]`, the oscillator input/output pins are connected to
  distinct nets, and no existing clock scenario covers the component. The
  template includes `scenario.clocks[]` with the oscillator pins, nets, and
  identified crystal/resonator component when one is modeled between the nets.
- It emits boot-strap templates when model boot modes declare required straps
  and the strap pins are connected.
- It emits runnable `BOOT_STRAP_BIAS_VALID` templates when required strap pins
  have explicit resistor bias evidence to declared power or ground nets.
  Imported KiCad schematics can provide this automatically when pull resistors
  are mapped as SPICE resistors with `value_ohm_from: schematic_value`; see
  `examples/import_kicad_bootstrap_bias_suggestions/` and
  `examples/import_kicad_esp32_wroom_32e_suggestions/`.
- Imported KiCad schematics can also provide reset-supervisor evidence when a
  supervisor symbol is mapped to a model with `reset_supervisor` metadata; see
  `examples/import_kicad_tlv803_reset_supervisor_suggestions/`.
- Imported KiCad schematics can provide regulator evidence when a regulator
  symbol is mapped to a model with `power_conversion` metadata; see
  `examples/import_kicad_ap2112_regulator_suggestions/`,
  `examples/import_kicad_ams1117_regulator_suggestions/`, and
  `examples/import_kicad_tps62162_regulator_suggestions/`.
- Imported KiCad schematics can provide clamp-only USB ESD evidence when a
  protection symbol is mapped to a model with
  `signal_conditioning.protection_clamps`; see
  `examples/import_kicad_tpd2eusb30_usb_esd_suggestions/` and
  `examples/import_kicad_prtr5v0u2x_usb_esd_suggestions/`.
- Imported KiCad schematics can provide connector-level USB protection evidence
  when a connector symbol is mapped to a model with `usb_connector` metadata and
  the connected ESD/protection symbols are mapped to clamp models; see
  `examples/import_kicad_usb_connector_protection_suggestions/`.
- The same fixture can be enriched with `import-kicad-pcb` using its
  `board.kicad_pcb`; after enrichment, `suggest-scenarios` emits
  `USB_PROTECTION_PLACEMENT_VALID` with connector-to-protection distance
  evidence.
- Imported fabrication evidence can provide manufacturing suggestions. When
  `board.layout.drills[]` is present, `suggest-scenarios` emits runnable
  `DRILL_DIAMETER_VALID` using
  `fabrication_process: jlcpcb_drill_diameter_range_2026_06`. When
  `board.layout.slots[]` is present, it emits runnable `SLOT_WIDTH_VALID` using
  `fabrication_process: jlcpcb_slot_min_2026_06` and runnable
  `SLOT_ASPECT_RATIO_VALID` using the same slot preset. Drill-to-board-edge and
  slot-to-board-edge templates become runnable when the Board IR carries
  `board.manufacturing.min_drill_edge_clearance_mm` or
  `board.manufacturing.min_slot_edge_clearance_mm`; otherwise they stay
  non-runnable and request those order-specific limits. When drills and copper
  flashes are present, it emits runnable `DRILL_ANNULAR_RING_VALID` using
  `fabrication_process: jlcpcb_double_sided_via_min_2026_06`. When copper and
  routed board-outline evidence are present, it emits runnable
  `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID` using
  `fabrication_process: jlcpcb_routed_edge_copper_clearance_2026_06`. When at
  least one drill is explicitly marked `castellated: true` and board-outline
  evidence exists, it emits runnable `CASTELLATED_HOLE_VALID` using
  `fabrication_process: jlcpcb_castellated_hole_2026_06`. When
  copper evidence has at least two same-layer objects, it emits runnable
  `COPPER_SPACING_VALID` using
  `fabrication_process: jlcpcb_1oz_copper_spacing_2026_06`. When copper
  flashes and solder-mask openings are present, it emits runnable
  `SOLDER_MASK_OPENING_VALID`; when two or more solder-mask openings are
  present, it emits runnable `SOLDER_MASK_DAM_VALID`. Both use
  `fabrication_process: jlcpcb_standard_2026_06`. When solder-paste flash or
  draw evidence is present, it emits runnable
  `SOLDER_PASTE_APERTURE_SIZE_VALID` using
  `fabrication_process: jlcpcb_stencil_aperture_min_2026_06`.
  When any solder-paste opening evidence is present, it emits
  `SOLDER_PASTE_APERTURE_AREA_RATIO_VALID` using
  `fabrication_process: jlcpcb_stencil_area_ratio_2026_06`. The template is
  runnable when `board.manufacturing.stencil_thickness_mm` is present; otherwise
  it remains non-runnable until `stencil_thickness_mm` is supplied because
  stencil release area ratio depends on stencil thickness.
  When copper flashes and solder-paste openings are present,
  `SOLDER_PASTE_OPENING_VALID` becomes runnable if
  `board.manufacturing.min_paste_area_ratio` and
  `board.manufacturing.max_paste_area_ratio` are present and consistent.
  When at least two solder-paste openings are present,
  `SOLDER_PASTE_SPACING_VALID` becomes runnable if
  `board.manufacturing.min_solder_paste_spacing_mm` is present.
  When pad-owned solder-paste flashes for one component show at least two
  repeated gaps matching a discrete source-backed JLC IC pitch row, it emits
  runnable target-scoped `SOLDER_PASTE_IC_PIN_APERTURE_VALID` with the inferred
  `pin_pitch_mm`. Automatic pitch inference is intentionally limited to the
  discrete 0.3, 0.35, 0.4, 0.5, and 0.65 mm rows plus representative exact
  0.8, 1.0, and 1.27 mm pitches inside the source-backed 0.8-1.27 mm IC table
  row. Broad-row candidates require at least three repeated gaps, so a single
  arbitrary pair cannot become a stencil-rule input. When pad-owned
  solder-paste flashes for one component form a two-axis grid with repeated
  horizontal and vertical gaps matching a source-backed JLC BGA pitch row, it
  emits runnable target-scoped `SOLDER_PASTE_BGA_APERTURE_VALID` with the
  inferred `pin_pitch_mm`. The BGA grid suggestion suppresses the IC row
  suggestion for the same target component.
- Manufacturing checks whose thresholds are neither pinned to a named process
  preset nor present as Board IR manufacturing metadata are suggested as
  `runnable: false` with explicit required inputs. This keeps order-specific
  drill-to-edge, slot-to-edge, paste coverage, and paste-spacing limits out of
  generic presets.
- It emits UART bootloader templates when model bootloader metadata declares a
  UART interface. If an output-capable sender pin is already wired to the target
  RX net, the template includes that sender; otherwise it records the missing
  sender as required input.
- It never invents boot strap states, reset-release timestamps, power-good
delays, GPIO pin-state observations, protection-path resistance, strap
current budgets, untied load-switch enable evidence, charger programmed-current
evidence, power-mux selected-source evidence, oscillator startup margin, or
SPICE assertions.

Example output shape:

```yaml
schema_version: 0.1.0
project: scenario_suggestions_power_reset
suggestions:
  - id: power_tree_valid
    kind: power_tree
    confidence: high
    runnable: true
    reason: Project declares power nets but no POWER_TREE_VALID scenario.
    scenario:
      name: scenario_suggestions_power_reset_power_tree
      type: power_tree
      checks:
        - POWER_TREE_VALID
      regulators:
        - component: UREG
          input_pin: VIN
          input_net: usb_5v
          output_pin: VOUT
          output_net: rail_3v3
          dropout_voltage_V: 0.4
          min_output_current_A: 0.01
          max_output_current_A: 0.6
          input_capacitance_min_F: 0.000001
          output_capacitance_min_F: 0.000001
          input_support_capacitance_F: 0.000001
          input_support_capacitors:
            - CIN
          output_support_capacitance_F: 0.000001
          output_support_capacitors:
            - COUT
          switch_pin: SW
          switch_net: buck_sw
          input_inductance_min_H: 0.00000037
          input_support_inductance_H: 0.000001
          input_support_inductors:
            - LIN
          output_inductance_min_H: 0.0000022
          output_support_inductance_H: 0.0000022
          output_support_inductors:
            - L1
          switch_inductor_pin_a: L1
          switch_inductor_net_a: buck_boost_l1
          switch_inductor_pin_b: L2
          switch_inductor_net_b: buck_boost_l2
          switch_inductance_min_H: 0.00000037
          switch_inductance_max_H: 0.00000057
          switch_support_inductance_H: 0.00000047
          switch_support_inductors:
            - LBB
      reset_supervisors:
        - component: USUP
          monitored_pin: VDD
          monitored_net: rail_3v3
          reset_output_pin: RESET
          reset_net: nrst
          threshold_min_V: 2.93
          threshold_max_V: 3.08
  - id: io_voltage_compatible
    kind: power_tree
    confidence: medium
    runnable: true
    reason: Project has same-net digital output/input pairs with modeled I/O voltage metadata but no IO_VOLTAGE_COMPATIBLE check.
    scenario:
      name: scenario_suggestions_power_reset_io_voltage
      type: power_tree
      checks:
        - IO_VOLTAGE_COMPATIBLE
      paths:
        - driver:
            component: U1
            pin: TX
          victim:
            component: U2
            pin: RXD
          net: uart_mcu_tx
          series_resistance_ohm: 0
        - driver:
            component: U2
            pin: TXD
          victim:
            component: U1
            pin: RX
          net: uart_mcu_rx
          series_resistance_ohm: 0
  - id: usb_connector_protection_j1
    kind: interface_protection
    confidence: medium
    runnable: true
    reason: USB connector J1 exposes D+/D-/VBUS nets; add a connector-level protection coverage scenario.
    scenario:
      name: j1_usb_connector_protection
      type: interface_protection
      checks:
        - USB_CONNECTOR_PROTECTION_VALID
      parameters:
        require_vbus_protection: true
        data_working_voltage_min_V: 3.3
        vbus_working_voltage_min_V: 5
      target:
        component: J1
      protection_clamps:
        - component: UESD
          clamp: d1_plus
          protected_pin: D1+
          protected_net: usb_dp
          reference_pin: GND
          reference_net: gnd
          reference: ground
          working_voltage_max_V: 5.5
          line_capacitance_F: 0.0000000000007
      usb_connectors:
        - component: J1
          standard: usb2
          vbus_pin: VBUS
          vbus_net: usb_vbus
          dp_pin: D+
          dp_net: usb_dp
          dm_pin: D-
          dm_net: usb_dm
          gnd_pin: GND
          gnd_net: gnd
  - id: usb_protection_placement_j1
    kind: interface_protection
    confidence: medium
    runnable: false
    reason: USB connector J1 and connected protection components have placement evidence; add a connector-to-protection distance scenario.
    scenario:
      name: j1_usb_protection_placement
      type: interface_protection
      checks:
        - USB_PROTECTION_PLACEMENT_VALID
      parameters:
        require_vbus_protection: true
        max_connector_to_protection_distance_mm: null
      target:
        component: J1
      protection_clamps:
        - component: UESD
          clamp: d1_plus
          protected_pin: D1+
          protected_net: usb_dp
          reference_pin: GND
          reference_net: gnd
          reference: ground
          working_voltage_max_V: 5.5
          line_capacitance_F: 0.0000000000007
          placement:
            x_mm: 1
            y_mm: 0
            side: top
          distance_to_target_mm: 1
      usb_connectors:
        - component: J1
          standard: usb2
          vbus_pin: VBUS
          vbus_net: usb_vbus
          dp_pin: D+
          dp_net: usb_dp
          dm_pin: D-
          dm_net: usb_dm
          gnd_pin: GND
          gnd_net: gnd
          placement:
            x_mm: 0
            y_mm: 0
            side: top
    required_inputs:
      - Fill parameters.max_connector_to_protection_distance_mm from the board's ESD/layout rule or datasheet/layout guidance; do not invent the limit from component coordinates.
      - Use PCB/layout review for routed trace order, via count, return path, shield strategy, and USB differential-pair constraints.
  - id: reset_release_after_power_valid_u1
    kind: reset_boot
    confidence: medium
    runnable: false
    reason: Component U1 has reset behavior and target rail power_valid_at_us, but no RESET_RELEASE_AFTER_POWER_VALID scenario.
    scenario:
      name: u1_reset_release_after_power
      type: reset_boot
      checks:
        - RESET_RELEASE_AFTER_POWER_VALID
      target:
        component: U1
        power_pin: VDD
        reset_pin: NRST
      timing:
        power_valid_at_us: 1500
        reset_release_delay_us: 0
    required_inputs:
      - Fill timing.reset_release_at_us from reset supervisor, RC, control-line, or analog waveform evidence before validation.
      - Keep timing.power_valid_at_us equal to the target rail power_valid_at_us or remove duplicated stale timing.
  - id: reset_release_after_power_valid_u4
    kind: reset_boot
    confidence: medium
    runnable: true
    reason: Component U4 has active-low reset behavior, target rail power_valid_at_us, and explicit RC reset evidence from R4 and C4.
    scenario:
      name: u4_reset_release_after_power
      type: reset_boot
      checks:
        - RESET_RELEASE_AFTER_POWER_VALID
      target:
        component: U4
        power_pin: VDD
        reset_pin: NRST
      timing:
        power_valid_at_us: 1500
        reset_release_delay_us: 931.558
        reset_release_at_us: 2431.558
        boot_sample_at_us: 2531.558
  - id: boot_strap_defined_u1_bootloader
    kind: reset_boot
    confidence: high
    runnable: true
    reason: Component U1 model declares boot mode bootloader, but no BOOT_STRAP_DEFINED scenario covers it.
    scenario:
      name: u1_boot_straps_bootloader
      type: reset_boot
      checks:
        - BOOT_STRAP_DEFINED
      target:
        component: U1
      required_boot_mode: bootloader
      straps:
        - component: U1
          pin: BOOT0
          net: rail_3v3
          actual: high
  - id: gpio_backdrive_u2_txd_to_u1_rx
    kind: gpio_backdrive
    confidence: high
    runnable: true
    reason: Powered output U2.TXD shares net uart_rx with unpowered input U1.RX, but no GPIO_BACKDRIVE scenario covers that path.
    scenario:
      name: u2_to_u1_backdrive
      type: gpio_backdrive
      checks:
        - GPIO_BACKDRIVE
      pin_states:
        - component: U2
          pin: TXD
          mode: output
          state: high
        - component: U1
          pin: RX
          mode: input
      paths:
        - driver: { component: U2, pin: TXD }
          victim: { component: U1, pin: RX }
          net: uart_rx
          series_resistance_ohm: 1000
  - id: interface_protection_u3_ch1
    kind: interface_protection
    confidence: medium
    runnable: false
    reason: Component U3 model declares signal-conditioning channel ch1, but no interface protection review scenario covers it.
    scenario:
      name: u3_ch1_interface_protection
      type: interface_protection
      checks:
        - INTERFACE_PROTECTION_REVIEW
      target:
        component: U3
      conditioning:
        component: U3
        channel: ch1
        kind: level_shifter
        side_a:
          pin: A1
          net: mcu_rx_shifted
          supply_pin: VCCA
          supply_net: mcu_3v3
        side_b:
          pin: B1
          net: usb_uart_tx
          supply_pin: VCCB
          supply_net: usb_uart_3v3
        direction: bidirectional
        unpowered_isolation: false
    required_inputs:
      - Confirm the signal-conditioning part datasheet supports this direction, voltage range, and unpowered-side behavior.
      - Fill enable/OE/reset-state evidence when the part can disconnect or leave either side high impedance.
      - Add analog_transient or GPIO_BACKDRIVE scenarios for any datasheet condition that does not guarantee isolation.
  - id: interface_protection_uesd_d1_plus
    kind: interface_protection
    confidence: medium
    runnable: true
    reason: Component UESD model declares protection clamp d1_plus, but no interface protection review scenario covers it.
    scenario:
      name: uesd_d1_plus_interface_protection
      type: interface_protection
      checks:
        - INTERFACE_PROTECTION_REVIEW
      parameters:
        clamp: d1_plus
      target:
        component: UESD
      protection_clamps:
        - component: UESD
          clamp: d1_plus
          protected_pin: D1+
          protected_net: usb_dp
          reference_pin: GND
          reference_net: gnd
          reference: ground
          working_voltage_max_V: 5.5
          line_capacitance_F: 7.0e-13
    required_inputs:
      - Fill parameters.max_line_capacitance_F from the real interface capacitance budget when capacitance screening is required; do not use the clamp's own capacitance as the budget unless that is the actual design limit.
      - Use layout, signal-integrity, and ESD-pulse validation for USB eye margin, return path, and IEC stress sign-off.
  - id: clock_source_valid_u1
    kind: clock
    confidence: medium
    runnable: true
    reason: Component U1 model declares external clock source metadata, but no CLOCK_SOURCE_VALID scenario covers it.
    scenario:
      name: u1_clock_source
      type: clock
      checks:
        - CLOCK_SOURCE_VALID
      target:
        component: U1
      clocks:
        - component: U1
          name: hse
          input_pin: OSC_IN
          input_net: osc_in
          output_pin: OSC_OUT
          output_net: osc_out
          crystal_component: Y1
  - id: uart_bootloader_sync_u1_uart
    kind: serial_programming
    confidence: high
    runnable: true
    reason: Component U1 model declares bootloader interface uart, but no UART_BOOTLOADER_SYNC scenario covers it.
    scenario:
      name: u1_uart_bootloader_sync
      type: serial_programming
      checks:
        - UART_BOOTLOADER_SYNC
      target:
        component: U1
        power_pin: VDD
        reset_pin: NRST
      timing:
        power_valid_at_us: 1500.0
        reset_release_delay_us: 931.5582043707448
        reset_release_at_us: 2431.558204370745
        boot_sample_at_us: 2531.558204370745
      required_boot_mode: bootloader
      straps:
        - component: U1
          pin: BOOT0
          net: rail_3v3
          actual: high
      bootloader:
        component: U1
        interface: uart
        sync_byte: 127
        expected_response: 121
      events:
        - at_us: 2531.558204370745
          action: uart_send
          from: { component: U2, pin: TXD }
          to: { component: U1, pin: RX }
          bytes: [127]
```

Without matching `board.runtime.gpio_backdrive[]` evidence, the GPIO backdrive
template stays non-runnable and asks for runtime-state proof plus the schematic
series resistance.

UART bootloader suggestions stay non-runnable unless the target has a proven
output-capable sender, explicit reset/boot timing evidence, and, when the model
declares boot modes, exactly one boot mode proven by direct rail/ground strap
state. Standalone boot-strap suggestions use the same direct rail/ground proof
to fill `straps[].actual`; other observed strap evidence should still be
entered explicitly by the user before validation.

This is a planning aid, not validation sign-off. Agents should add runnable
scenarios directly and complete non-runnable templates with measured or modeled
evidence before running `circuitci validate`.
