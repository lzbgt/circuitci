# Scenario Language

Scenarios describe validation conditions applied to a bound board. CircuitCI
supports deterministic behavioral checks, time-ordered scenario events for
protocol/control-line validation, functional-MCU firmware-in-loop contracts,
and solver-backed `analog_transient` waveforms for physical voltage/current
checks.

## Behavioral Scenario Example

```yaml
scenarios:
  - name: usb_hot_plug_mcu_unpowered
    type: gpio_backdrive
    checks:
      - GPIO_BACKDRIVE
    parameters:
      diode_drop_V: 0.3
    pin_states:
      - component: U2
        pin: TXD
        mode: output
        state: high
      - component: U1
        pin: RX
        mode: input
    paths:
      - driver:
          component: U2
          pin: TXD
        victim:
          component: U1
          pin: RX
        series_resistance_ohm: 0
```

## Event Form

```yaml
events:
  - at: 0ms
    action: set_source
    source: usb_vbus
    voltage: 5.0
  - at: 10ms
    action: serial_open
    device: U2
    baud: 115200
```

## Scenario Types

- `power_up`
- `power_down`
- `usb_hot_plug`
- `reset_boot`
- `serial_programming`
- `gpio_backdrive`
- `i2c_bus`
- `sleep_current`
- `brownout`
- `tolerance_sweep`

Executable scenario types:

- `gpio_backdrive`
- `reset_boot`
- `serial_programming`
- `firmware_update`
- `control_line_sequence`
- `firmware_in_loop`
- `interface_protection`
- `clock`
- `power_tree`
- `analog_transient`

Unsupported scenario types must produce an explicit low-confidence limitation or informational finding, not a crash.

## Scenario Resolution

For the first Rust implementation:

1. The CLI loads the requested profile name for report metadata.
2. Project-declared scenarios are the executable source of truth.
3. A scenario runs each check in its `checks` list once, preserving file order.
4. Duplicate checks in one scenario are de-duplicated with first occurrence winning.
5. Unsupported checks produce `UNSUPPORTED_CHECK` limitations.
6. Unsupported scenario types produce `UNSUPPORTED_SCENARIO` limitations.

Canonical executable check IDs:

- `GPIO_BACKDRIVE`
- `RESET_RELEASE_AFTER_POWER_VALID`
- `BOOT_STRAP_DEFINED`
- `BOOT_STRAP_BIAS_VALID`
- `UART_BOOTLOADER_SYNC`
- `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`
- `CONTROL_LINE_RELEASE_SEQUENCE`
- `FUNCTIONAL_MCU_FIRMWARE`
- `INTERFACE_PROTECTION_REVIEW`
- `CLOCK_SOURCE_VALID`
- `POWER_TREE_VALID`
- `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID`
- `DRILL_ANNULAR_RING_VALID`
- `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID`
- `COPPER_SPACING_VALID`
- `SOLDER_MASK_OPENING_VALID`
- `SOLDER_MASK_DAM_VALID`
- `SOLDER_PASTE_OPENING_VALID`
- `IO_VOLTAGE_COMPATIBLE`
- `SPICE_TRANSIENT_ANALYSIS`

`SPICE_OPERATING_LIMIT` is not declared as a separate scenario check. It is an
automatic critical finding emitted by `SPICE_TRANSIENT_ANALYSIS` when generated
Board IR device waveforms exceed datasheet absolute maximum ratings.

## Reset/Boot Scenario Shape

`reset_boot` scenarios use explicit timing metadata until analog waveform extraction exists:

```yaml
scenarios:
  - name: reset_boot_valid
    type: reset_boot
    target:
      component: U1
      power_pin: VDD
      reset_pin: NRST
    checks:
      - RESET_RELEASE_AFTER_POWER_VALID
      - BOOT_STRAP_DEFINED
    timing:
      power_valid_at_us: 1200
      reset_release_delay_us: 500
      reset_release_at_us: 5000
      boot_sample_at_us: 5100
    straps:
      - component: U1
        pin: BOOT0
        net: boot0
        actual: low
    required_boot_mode: application
```

Timing semantics:

- `power_valid_at_us`: first time the component's operating rail is valid.
- `reset_release_delay_us`: optional reset-supervisor, power-good, or RC delay
  after the operating rail is valid. Defaults to `0`.
- `reset_release_at_us`: first time reset is deasserted.
- `boot_sample_at_us`: time boot straps are sampled.

`target.component` is required for `reset_boot`. `target.power_pin` and `target.reset_pin` are optional scenario assertions; if present, they must match the component model behavior and board pin map.

`RESET_RELEASE_AFTER_POWER_VALID` fails when reset releases before power is
valid plus any declared `reset_release_delay_us`. When `target.power_pin`
resolves to a rail with `power_valid_at_us`, the rule uses that rail timing and
fails closed if it conflicts with duplicated scenario `timing.power_valid_at_us`.
Missing target/timing data for this declared check is a critical
`VALIDATION_INPUT_MISSING` finding.

`BOOT_STRAP_DEFINED` resolves required strap states from
`component.behavior.boot.modes[required_boot_mode]`. It fails when any required
strap is missing from scenario observations, observed as `floating` or
`undefined`, or not equal to the model-required state. The scenario may not
invent the required strap state.

`BOOT_STRAP_BIAS_VALID` is the static resistor-network companion for
schematic-derived strap checks. It resolves each required boot strap pin to its
board net, finds explicit resistor primitives connected from that net to
declared power or ground nets, and computes the DC strap voltage:

```text
strap_voltage = sum(source_voltage / resistor_ohm) / sum(1 / resistor_ohm)
strap_bias_current = sum(max(0, source_voltage - strap_voltage) / resistor_ohm)
```

The rule supports pull-up-only, pull-down-only, and divider networks. Power
source nets must declare `powered` and `nominal_voltage`; unpowered rails
contribute `0 V`. The target strap model pin must declare `vih_min_V` for a
required `high` state or `vil_max_V` for a required `low` state. A strap with
no resistor bias to power or ground fails as floating. A divider voltage inside
the undefined region fails. If the scenario declares
`parameters.max_strap_bias_current_A`, the computed divider current must not
exceed that limit.

```yaml
scenarios:
  - name: bootloader_boot0_bias
    type: reset_boot
    target: { component: U1, power_pin: VDD }
    checks:
      - BOOT_STRAP_BIAS_VALID
    required_boot_mode: bootloader
    parameters:
      max_strap_bias_current_A: 0.0001
```

## Serial Programming Scenario Shape

`serial_programming` scenarios model an abstract bootloader sync handshake:

```yaml
scenarios:
  - name: stm32_like_uart_bootloader
    type: serial_programming
    target:
      component: U1
    checks:
      - UART_BOOTLOADER_SYNC
    required_boot_mode: bootloader
    bootloader:
      component: U1
      interface: uart
      sync_byte: 0x7F
      expected_response: 0x79
    events:
      - at_us: 10000
        action: uart_send
        from:
          component: U2
          pin: TXD
        to:
          component: U1
          pin: RX
        bytes: [0x7F]
```

`UART_BOOTLOADER_SYNC` algorithm:

1. Resolve `target.component`.
2. Resolve `bootloader.interface` from `component.behavior.bootloader.interfaces`.
3. Require scenario `bootloader.sync_byte` and `expected_response` to match the model interface when provided.
4. Require `required_boot_mode` to exist in `component.behavior.boot.modes`.
5. If the same scenario declares strap observations, verify they match the required boot mode before checking sync.
6. Find an event with `action: uart_send`, `to.component == target.component`, `to.pin == model_interface.rx_pin`, `at_us >= boot_sample_at_us` when `boot_sample_at_us` exists, and `bytes` exactly equal to `[model_interface.sync_byte]`.
7. Require the event `from` endpoint to resolve to an output-capable board pin.
8. Require the event `from` endpoint and target RX endpoint to share the same board net.
9. ACK is abstract in this slice: matching the sync event, sender connectivity, and model `ack_byte` is enough to report sync-capable pass. No firmware is executed.

Missing required model/scenario data for this declared check is a critical `VALIDATION_INPUT_MISSING` finding.

## Interface Protection Scenario Shape

`interface_protection` scenarios review declared signal-conditioning channels
such as level shifters, series resistors, or bus switches. They can also review
clamp-only protection devices such as USB ESD arrays.

```yaml
scenarios:
  - name: level_shifter_channel_review
    type: interface_protection
    checks:
      - INTERFACE_PROTECTION_REVIEW
    target:
      component: U3
    parameters:
      channel: ch1
```

Channel review algorithm:

1. Resolve `target.component`.
2. Resolve `parameters.channel` from the target model's
   `signal_conditioning.channels`.
3. Require both side pins to be connected.
4. Require each side supply pin to resolve to a declared power net with a
   `powered` state.
5. Check model `signal_conditioning.supply_constraints` whenever both
   constrained rails are powered. For `less_than_or_equal`, the lower rail's
   nominal voltage must not exceed the upper rail's nominal voltage.
6. If both side supplies have the same powered state, the static isolation
   review passes.
7. If one side is powered and the other is unpowered, the channel must declare
   `unpowered_isolation: true`, or the scenario must observe the channel's
   declared `enable_pin` in its `disabled_state`; otherwise the check fails
   critically.

Clamp review uses `parameters.clamp` instead of `parameters.channel`:

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

Clamp review algorithm:

1. Resolve `target.component`.
2. Resolve `parameters.clamp` from the target model's
   `signal_conditioning.protection_clamps`.
3. Require the clamp protected pin and reference pin to be connected.
4. Require the reference pin's net kind to match the model reference
   (`ground` or `power`).
5. If the model declares `working_voltage_max_V` and the protected net declares
   finite `nominal_voltage`, require the net voltage to be no higher than the
   standoff limit.
6. If the model declares `line_capacitance_F` and the scenario declares
   `max_line_capacitance_F`, require the clamp capacitance to fit the interface
   budget.

USB connector coverage uses `USB_CONNECTOR_PROTECTION_VALID` against a connector
model that declares `usb_connector` pin metadata. The rule verifies that D+ and
D- have connected clamp-only protection on the same nets. It also verifies VBUS
when `parameters.require_vbus_protection` is true, and verifies the optional
shield pin is connected to a declared ground net when
`parameters.require_shield_ground` is true.

```yaml
scenarios:
  - name: usb_connector_protection
    type: interface_protection
    checks:
      - USB_CONNECTOR_PROTECTION_VALID
    target:
      component: J1
    parameters:
      require_vbus_protection: true
      require_shield_ground: true
      data_working_voltage_min_V: 3.6
      vbus_working_voltage_min_V: 5.5
```

Connector protection algorithm:

1. Resolve `target.component`.
2. Resolve `usb_connector` metadata from the target component model.
3. Resolve connector D+, D-, GND, and optional VBUS nets from board
   connectivity.
4. For each required protected net, find a different component with
   `signal_conditioning.protection_clamps` whose protected pin is on the same
   net and whose reference pin is connected to the declared reference kind.
5. If `data_working_voltage_min_V` or `vbus_working_voltage_min_V` is declared,
   require the found clamp standoff voltage to meet that minimum.
6. If `require_shield_ground` is true, require `usb_connector.shield_pin` to be
   connected to a declared `ground` net. This is a static schematic check only;
   RC, ferrite, chassis-only, or spark-gap shield strategies need explicit
   future modeling instead of this simplified parameter.

USB protection placement uses `USB_PROTECTION_PLACEMENT_VALID` to add explicit
layout-distance evidence to the same connector/clamp model contract. The rule
does not infer trace routing from the schematic; it requires
`board.layout.placements` for the connector and matching protection components.

```yaml
board:
  layout:
    placements:
      J1: { x_mm: 0.0, y_mm: 0.0, side: top }
      UESD: { x_mm: 1.0, y_mm: 0.0, side: top }

scenarios:
  - name: usb_protection_placement
    type: interface_protection
    checks:
      - USB_PROTECTION_PLACEMENT_VALID
    target:
      component: J1
    parameters:
      require_vbus_protection: true
      max_connector_to_protection_distance_mm: 2.0
```

Connector-to-protection placement algorithm:

1. Resolve `target.component`.
2. Resolve `usb_connector` metadata from the target component model.
3. Require finite placement coordinates for the connector.
4. For D+, D-, and VBUS when `require_vbus_protection` is true, find
   clamp-only protection on the same net with a valid reference net kind.
5. Require at least one matching protection component for each protected net to
   have finite placement coordinates.
6. Compute center-to-center distance in millimeters and require the nearest
   matching protection component to be no farther than
   `parameters.max_connector_to_protection_distance_mm`.

USB connector orientation uses `USB_CONNECTOR_ORIENTATION_VALID` when
`board.layout.placements.<connector>.rotation_deg` evidence is present and a
mechanical/layout rule declares the expected entry direction.

```yaml
scenarios:
  - name: usb_connector_orientation
    type: interface_protection
    checks:
      - USB_CONNECTOR_ORIENTATION_VALID
    target:
      component: J1
    parameters:
      expected_connector_rotation_deg: 0.0
      max_connector_rotation_error_deg: 5.0
```

Connector-orientation algorithm:

1. Resolve `target.component` and its `usb_connector` metadata.
2. Require finite placement coordinates and finite `rotation_deg` evidence for
   the connector.
3. Normalize actual and expected rotations modulo `360 deg`.
4. Compute the smallest angular error, so `359 deg` is `1 deg` from `0 deg`.
5. Require the error to be no greater than
   `parameters.max_connector_rotation_error_deg`.

`suggest-scenarios` can prefill `expected_connector_rotation_deg` from imported
`board.layout.outline.segments` evidence by finding the nearest board
edge and using its outward normal. That suggestion is still non-runnable until a
layout-specific tolerance is supplied, and the inferred direction must be
checked against the footprint's connector-entry rotation convention.

USB connector edge proximity uses `USB_CONNECTOR_EDGE_PROXIMITY_VALID` when
the Board IR includes connector placement evidence and board-edge outline
segment evidence.

```yaml
scenarios:
  - name: usb_connector_edge_proximity
    type: interface_protection
    checks:
      - USB_CONNECTOR_EDGE_PROXIMITY_VALID
    target:
      component: J1
    parameters:
      max_connector_to_board_edge_distance_mm: 0.5
```

Connector-to-board-edge algorithm:

1. Resolve `target.component` and its `usb_connector` metadata.
2. Require finite placement coordinates for the connector.
3. Require at least one usable segment under
   `board.layout.outline.segments`. KiCad curved Edge.Cuts graphics are
   imported as sampled segments.
4. If `board.layout.footprints.<component>` contains transformed
   `fabrication` or `courtyard` `fp_line`, `fp_rect`, `fp_poly`,
   `fp_circle`, or `fp_arc` evidence, measure the nearest supported footprint
   graphic to each board-edge segment.
5. If no usable footprint drawing evidence is available, project the connector
   placement point to each segment and use that fallback distance.
6. Require the nearest distance to be no greater than
   `parameters.max_connector_to_board_edge_distance_mm`.

USB connector body overhang uses `USB_CONNECTOR_BODY_OVERHANG_VALID` when
the Board IR includes board-edge outline segment evidence and imported
connector `fabrication` or `courtyard` footprint graphics.

```yaml
scenarios:
  - name: usb_connector_body_overhang
    type: interface_protection
    checks:
      - USB_CONNECTOR_BODY_OVERHANG_VALID
    target:
      component: J1
    parameters:
      max_connector_body_overhang_mm: 0.2
```

Connector-body overhang algorithm:

1. Resolve `target.component` and its `usb_connector` metadata.
2. Require finite connector placement evidence and at least one usable segment
   under `board.layout.outline.segments`. KiCad curved Edge.Cuts graphics are
   imported as sampled segments.
3. Require imported connector footprint `fabrication` or `courtyard`
   `fp_line`, `fp_rect`, `fp_poly`, `fp_circle`, or `fp_arc` evidence.
4. Find the nearest supported body/courtyard graphic to the board edge.
5. Infer the edge outward normal from the board outline centroid.
6. Measure the maximum supported footprint point protrusion past that edge
   along the outward normal.
7. Require the measured `connector_body_overhang_mm` to be no greater than
   `parameters.max_connector_body_overhang_mm`.

This is a static 2D board/footprint drawing guard. It does not sign off 3D
connector shell volume, panel cutouts, arcs, enclosure interference, or cable
insertion clearance. Curved footprint graphics are sampled into bounded
polylines for distance and overhang measurements.

USB connector component clearance uses
`USB_CONNECTOR_COMPONENT_CLEARANCE_VALID` when the Board IR includes connector
`fabrication` or `courtyard` footprint graphics and nearby component placement
or footprint evidence.

```yaml
scenarios:
  - name: usb_connector_component_clearance
    type: interface_protection
    checks:
      - USB_CONNECTOR_COMPONENT_CLEARANCE_VALID
    target:
      component: J1
    parameters:
      min_connector_to_component_clearance_mm: 0.5
```

Connector component-clearance algorithm:

1. Resolve `target.component` and its `usb_connector` metadata.
2. Require imported connector footprint `fabrication` or `courtyard`
   `fp_line`, `fp_rect`, `fp_poly`, `fp_circle`, or `fp_arc` evidence.
3. Convert supported connector and nearby component footprint graphics into 2D
   line segments; when a nearby component has no usable footprint graphics,
   fall back to its finite placement center.
4. Measure the minimum 2D clearance between the connector evidence and each
   other component's evidence.
5. Require every measured clearance to be at least
   `parameters.min_connector_to_component_clearance_mm`.

This is a static 2D component keepout screen. It does not prove 3D connector
shell clearance, cable insertion clearance, panel/enclosure clearance, or
assembly stack-up tolerances.

USB connector cable-entry clearance uses
`USB_CONNECTOR_ENTRY_CLEARANCE_VALID` when the Board IR includes USB connector
metadata, imported connector placement rotation, and supported
`fabrication`/`courtyard` footprint graphics.

```yaml
scenarios:
  - name: usb_connector_entry_clearance
    type: interface_protection
    checks:
      - USB_CONNECTOR_ENTRY_CLEARANCE_VALID
    target:
      component: J1
    parameters:
      min_cable_entry_clearance_depth_mm: 8.0
      cable_entry_clearance_width_mm: 6.0
```

Connector entry-clearance algorithm:

1. Resolve `target.component` and its `usb_connector` metadata.
2. Use `parameters.entry_direction_deg` when declared. Otherwise compute the
   cable insertion direction from imported connector `rotation_deg` plus
   optional KiCad footprint property `CircuitCI_EntryDirectionOffsetDeg`; if no
   footprint property is present, use KiCad mapping
   `layout.entry_direction_offset_deg`; if no mapping override is present, use
   component-model
   `usb_connector.entry_direction_offset_deg`. Normalize the result into
   `[0, 360)`.
3. Find the connector body's front projection from supported
   `fabrication`/`courtyard` `fp_line`, `fp_rect`, `fp_poly`, `fp_circle`, or
   `fp_arc` footprint evidence.
4. Apply optional aperture metadata from imported footprint properties, KiCad
   mapping metadata, or the component model. Footprint properties take
   precedence over mapping metadata, and both take precedence over component
   model defaults. Front offset shifts the corridor front, lateral offset shifts
   the corridor centerline perpendicular to entry direction, and aperture width
   becomes the minimum checked width when it is larger than
   `parameters.cable_entry_clearance_width_mm`.
   `CircuitCI_EntryClearanceDepthMM` / `CircuitCI_EntryClearanceWidthMM`,
   KiCad mapping `layout.entry_clearance_depth_mm` /
   `layout.entry_clearance_width_mm`, or component-model
   `usb_connector.entry_clearance_depth_mm` /
   `usb_connector.entry_clearance_width_mm` can prefill the entry-clearance
   parameters in suggestions, but executable validation still uses the scenario
   parameter values.
5. Build a 2D rectangular corridor extending
   `parameters.min_cable_entry_clearance_depth_mm` forward from that entry
   front with the effective checked width.
6. Convert nearby component footprint graphics into 2D line segments, falling
   back to finite placement centers when footprint graphics are unavailable.
7. Fail when any other component evidence intersects the cable-entry corridor.

This is a static 2D entry corridor screen. It does not prove connector shell
volume, plug shape, cable bend radius, panel cutout, enclosure interference, or
assembly stack-up tolerances.

## Manufacturing Scenario Shape

Drill-to-board-edge clearance uses `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID` when
the Board IR includes fabrication drill evidence under `board.layout.drills`
and board-outline segment evidence under `board.layout.outline.segments`.

```yaml
scenarios:
  - name: drill_to_board_edge_clearance
    type: manufacturing
    checks:
      - DRILL_TO_BOARD_EDGE_CLEARANCE_VALID
    parameters:
      min_drill_edge_clearance_mm: 0.5
```

Drill-to-board-edge algorithm:

1. Require `parameters.min_drill_edge_clearance_mm`.
2. Require finite `board.layout.drills[]` entries with positive `drill_mm`.
3. Require finite `board.layout.outline.segments[]` entries.
4. Measure each drill center to the nearest outline segment and subtract drill
   radius.
5. Fail when any drill edge-to-outline clearance is below
   `min_drill_edge_clearance_mm`.

External board-outline segments, cutout segments, and unknown outline segments
all count as board edges for this check. This is a static 2D centerline
fabrication screen; it does not model drill wander, routed-slot width, plating
barrel tolerances, panel tabs, fab-specific minimums, or copper-to-hole
clearance.

Drill annular-ring screening uses `DRILL_ANNULAR_RING_VALID` when the Board IR
includes fabrication drill evidence under `board.layout.drills` and Gerber
copper flash evidence under `board.layout.copper.features`.

```yaml
scenarios:
  - name: drill_annular_ring
    type: manufacturing
    checks:
      - DRILL_ANNULAR_RING_VALID
    parameters:
      min_annular_ring_mm: 0.2
      max_drill_to_copper_center_offset_mm: 0.05
      required_copper_layers: [F.Cu, B.Cu]
```

Drill annular-ring algorithm:

1. Require `parameters.min_annular_ring_mm`.
2. Optionally accept `parameters.max_drill_to_copper_center_offset_mm`;
   default is `0.1` mm.
3. Optionally accept `parameters.required_copper_layers` as a non-empty list
   of copper-layer names. When omitted, the rule requires one matching flash
   on any copper layer. When provided, every listed layer must have its own
   matching flash.
4. Require finite `board.layout.drills[]` entries with positive `drill_mm`.
5. Require finite `board.layout.copper.features[]` entries with positive
   aperture sizes.
6. Skip `non_plated` drills. Check `plated` and `unknown` drills.
7. Match co-located copper flashes within the center-offset limit.
8. Reject a co-located flash as annular-ring evidence when both drill and
   copper carry conflicting owner evidence: different `net` values, different
   pad owners, different via owners, or a pad/via kind mismatch.
9. Compute the best annular ring from supported `circle`, `rect`, or
   axis-aligned `oval` copper flash geometry.
10. Fail when no matching same/unknown-owner copper flash exists on the
    required layer, when only owner-mismatched copper exists, or when the best
    ring is below `min_annular_ring_mm`.

This is a static 2D fabrication screen. It does not model copper draws,
thermal reliefs, plating tolerance, drill wander distributions, solder mask,
fab-specific compensation, or electrical continuity beyond explicit imported
owner metadata.

Copper-to-board-edge clearance uses `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID`
when the Board IR includes board-outline segment evidence under
`board.layout.outline.segments` and anonymous Gerber copper evidence under
`board.layout.copper.features`, `board.layout.copper.segments`, or
`board.layout.copper.regions`.

```yaml
scenarios:
  - name: copper_to_board_edge_clearance
    type: manufacturing
    checks:
      - COPPER_TO_BOARD_EDGE_CLEARANCE_VALID
    parameters:
      min_copper_edge_clearance_mm: 0.25
```

Copper-to-board-edge algorithm:

1. Require `parameters.min_copper_edge_clearance_mm`.
2. Require finite `board.layout.outline.segments[]` entries.
3. Require at least one finite copper feature, copper segment, or copper
   region.
4. Measure each supported flash shape to the nearest board-outline or cutout
   segment.
5. Measure each imported copper segment centerline to the nearest board-outline
   or cutout segment and subtract half the trace width.
6. Measure each imported copper region polygon to the nearest board-outline or
   cutout segment.
7. Fail when any copper edge-to-outline clearance is below
   `min_copper_edge_clearance_mm`.

External board-outline segments, cutout segments, and unknown outline segments
all count as board edges for this check. This is a static 2D fabrication
screen; it does not model solder mask, copper etch compensation, fab-specific
clearance compensation, panelization tabs, copper island connectivity, or net
ownership.

Copper spacing uses `COPPER_SPACING_VALID` when the Board IR includes at least
two anonymous Gerber copper objects under `board.layout.copper.features`,
`board.layout.copper.segments`, or `board.layout.copper.regions`.

```yaml
scenarios:
  - name: copper_spacing
    type: manufacturing
    checks:
      - COPPER_SPACING_VALID
    parameters:
      min_copper_spacing_mm: 0.25
```

Copper spacing algorithm:

1. Require `parameters.min_copper_spacing_mm`.
2. Require at least two finite copper features, copper segments, or copper
   regions.
3. Compare same-layer copper feature/feature, feature/segment,
   feature/region, segment/segment, segment/region, and region/region pairs.
4. Use supported `circle`, `rect`, and axis-aligned `oval` flash geometry plus
   circular-aperture trace segment width and single-contour region polygon
   boundaries.
5. Ignore different-layer pairs.
6. If both copper objects declare the same `net`, or no net and the same
   `island_id`, treat touching or close copper as intentional ownership and
   skip the spacing pair.
7. If both copper objects declare different `net` values, or no net and
   different `island_id` values, report overlapping/touching copper as a
   zero-clearance spacing failure.
8. If ownership is unknown, ignore overlapping or touching anonymous copper
   because Gerber copper alone has no net ownership or island connectivity
   evidence.
9. Fail when separated same-layer copper spacing is below
   `min_copper_spacing_mm`.

This is a static 2D fabrication screen. It can find too-tight same-layer copper
spacing in Gerber evidence, but it cannot prove shorts, same-net intent,
copper-island connectivity, solder-mask margin, etch compensation, or
fab-specific spacing rules without richer PCB/net evidence.

Solder-mask opening validation uses `SOLDER_MASK_OPENING_VALID` when the Board
IR includes Gerber copper flash evidence under `board.layout.copper.features`
and Gerber solder-mask flash-opening evidence under
`board.layout.solder_mask.features`.

```yaml
scenarios:
  - name: solder_mask_openings
    type: manufacturing
    checks:
      - SOLDER_MASK_OPENING_VALID
    parameters:
      min_mask_expansion_mm: 0.05
      max_copper_to_mask_center_offset_mm: 0.05 # optional, defaults to 0.1
```

Solder-mask opening algorithm:

1. Require `parameters.min_mask_expansion_mm`.
2. Require finite Gerber copper flash features and solder-mask flash features.
3. Map `F.Cu` copper to `F.Mask` openings and `B.Cu` copper to `B.Mask`
   openings.
4. For each copper flash, find the same-layer mask opening within
   `max_copper_to_mask_center_offset_mm` that gives the largest minimum X/Y
   expansion.
5. Fail when no co-located opening exists.
6. Fail when the opening expands the copper flash by less than
   `min_mask_expansion_mm` on either axis.

This is a static 2D solder-mask aperture screen. It checks flash-to-flash
opening evidence and does not yet solve mask regions, mask dams between pads,
fab-specific mask swell, paste stencil behavior, or package-specific mask
rules.

Solder-mask dam validation uses `SOLDER_MASK_DAM_VALID` when the Board IR
includes at least two Gerber solder-mask openings under
`board.layout.solder_mask.features`, `board.layout.solder_mask.segments`, or
`board.layout.solder_mask.regions`.

```yaml
scenarios:
  - name: solder_mask_dams
    type: manufacturing
    checks:
      - SOLDER_MASK_DAM_VALID
    parameters:
      min_solder_mask_dam_mm: 0.15
```

Solder-mask dam algorithm:

1. Require `parameters.min_solder_mask_dam_mm`.
2. Require at least two finite solder-mask opening features, segments, or
   regions.
3. Compare same-layer opening pairs using supported `circle`, `rect`,
   axis-aligned `oval`, circular-aperture linear draw, and single-contour
   region geometry.
4. Ignore different-layer opening pairs.
5. Fail when the measured opening-to-opening gap is below
   `min_solder_mask_dam_mm`.

This is a static 2D mask web screen. It can detect thin or missing mask dams
between imported flash, linear draw, and region openings, but it does not yet
evaluate multi-contour mask regions, fab-specific mask bridge exceptions,
package-specific no-dam rules, or paste stencil behavior.

Solder-paste opening validation uses `SOLDER_PASTE_OPENING_VALID` when the
Board IR includes Gerber copper flash evidence under
`board.layout.copper.features` and Gerber solder-paste flash-opening evidence
under `board.layout.solder_paste.features`.

```yaml
scenarios:
  - name: solder_paste_openings
    type: manufacturing
    checks:
      - SOLDER_PASTE_OPENING_VALID
    parameters:
      min_paste_area_ratio: 0.7
      max_paste_area_ratio: 1.0
      max_copper_to_paste_center_offset_mm: 0.05 # optional, defaults to 0.1
```

Solder-paste opening algorithm:

1. Require finite `parameters.min_paste_area_ratio` and
   `parameters.max_paste_area_ratio`.
2. Require `max_paste_area_ratio >= min_paste_area_ratio`.
3. Require finite Gerber copper flash features and solder-paste flash features.
4. Skip copper features explicitly owned by vias.
5. Map `F.Cu` copper to `F.Paste` openings and `B.Cu` copper to `B.Paste`
   openings.
6. For each checked copper flash, find the nearest same-layer paste opening
   within `max_copper_to_paste_center_offset_mm`.
7. Fail when no co-located opening exists.
8. Fail when `paste_area_mm2 / copper_area_mm2` is outside the configured
   inclusive area-ratio range.

This is a static 2D stencil aperture screen. It checks flash-to-flash area
ratio evidence and does not yet evaluate drawn or region paste apertures,
windowed exposed-pad stencils, step-stencil thickness, paste volume, or
package-specific paste reductions.

USB route geometry uses `USB_ROUTE_GEOMETRY_VALID` when the Board IR includes
`board.layout.routes` evidence imported from PCB data. The rule checks D+ and
D- route length, via count, and the routed distance from the connector to the
nearest valid protection component.

```yaml
scenarios:
  - name: usb_route_geometry
    type: interface_protection
    checks:
      - USB_ROUTE_GEOMETRY_VALID
    target:
      component: J1
    parameters:
      max_data_line_route_length_mm: 25.0
      max_data_line_via_count: 0
      max_connector_to_protection_route_distance_mm: 2.0
      max_component_to_route_distance_mm: 0.2
      max_data_pair_length_mismatch_mm: 0.5
      max_data_pair_via_count_delta: 0
      max_data_line_width_delta_mm: 0.01      # optional
      max_data_pair_gap_delta_mm: 0.01        # optional
      require_route_pad_contact_evidence: true # optional
```

USB route geometry algorithm:

1. Resolve `target.component` and its `usb_connector` metadata.
2. Resolve D+ and D- nets from connector pin connectivity.
3. Require `board.layout.routes` entries for both data nets.
4. Sum routed segment lengths and require each data net to stay within
   `max_data_line_route_length_mm`.
5. Count vias in each net route and require the count to stay within
   `max_data_line_via_count`.
6. If `max_data_line_width_delta_mm` is declared, resolve
   `board.layout.constraints.net_rules` for each data net and require every
   segment width to match `diff_pair_width_mm` or `track_width_mm` within that
   tolerance.
7. Require the D+/D- route length mismatch to stay within
   `max_data_pair_length_mismatch_mm`.
8. Require the D+/D- via-count delta to stay within
   `max_data_pair_via_count_delta`.
9. If `max_data_pair_gap_delta_mm` is declared, resolve
   `diff_pair_gap_mm`, find overlapping parallel D+/D- routed segments, and
   require edge-to-edge gap to match within that tolerance.
10. By default, project connector and protection component placements onto the
   routed net within `max_component_to_route_distance_mm`. When
   `require_route_pad_contact_evidence` is true, use imported
   `board.layout.pads` for the connector signal pin and matching protection
   pad instead; each pad must be on the same net and on a route layer within
   `max_component_to_route_distance_mm`. When imported pad shape and size are
   available for supported KiCad shapes (`rect`, `circle`, `oval`), the route
   must touch the pad copper extent; otherwise the check falls back to pad
   center projection.
11. Compute graph distance along the routed segments and require the nearest
   valid protection component or protection pad to be within
   `max_connector_to_protection_route_distance_mm`.

USB VBUS route geometry uses `USB_VBUS_ROUTE_VALID` when the Board IR includes
`board.layout.routes` evidence for the connector VBUS net. This rule is
separate from D+/D- route geometry because VBUS route policy is power-entry and
protection-order focused rather than differential-pair focused.

```yaml
scenarios:
  - name: usb_vbus_route
    type: interface_protection
    checks:
      - USB_VBUS_ROUTE_VALID
    target:
      component: J1
    parameters:
      max_vbus_route_length_mm: 20.0
      max_vbus_via_count: 0
      min_vbus_route_width_mm: 0.30   # optional
      max_connector_to_vbus_protection_route_distance_mm: 2.0
      max_component_to_route_distance_mm: 0.2
      require_vbus_route_pad_contact_evidence: true # optional
```

USB VBUS route algorithm:

1. Resolve `target.component` and its `usb_connector` metadata.
2. Resolve the connector VBUS net and require a `board.layout.routes` entry.
3. Sum routed segment lengths and require the net to stay within
   `max_vbus_route_length_mm`.
4. Count vias and require the count to stay within `max_vbus_via_count`.
5. If `min_vbus_route_width_mm` is declared, require every VBUS segment to be at
   least that wide.
6. By default, project connector and VBUS protection component placements onto
   the routed net within `max_component_to_route_distance_mm`. When
   `require_vbus_route_pad_contact_evidence` is true, use imported
   `board.layout.pads` for the connector VBUS pin and matching protection pad
   instead; each pad must be on the same net and on a route layer within
   `max_component_to_route_distance_mm`. When imported pad shape and size are
   available for supported KiCad shapes (`rect`, `circle`, `oval`), the route
   must touch the pad copper extent; otherwise the check falls back to pad
   center projection.
7. Compute graph distance along the routed VBUS segments and require the nearest
   valid VBUS protection component or protection pad to be within
   `max_connector_to_vbus_protection_route_distance_mm`.
8. Use a separate power-path/current-capacity or thermal review for VBUS copper
   ampacity, fuse behavior, inrush, or temperature-rise sign-off.

USB return-path validation uses `USB_RETURN_PATH_VALID` when the Board IR
includes USB D+/D- `board.layout.routes` evidence and same-layer ground-zone
outline evidence under `board.layout.zones`. This rule is a static layout guard:
it treats a data route segment as referenced when the segment midpoint is inside
a ground-net zone outline on the same copper layer.

```yaml
scenarios:
  - name: usb_return_path
    type: interface_protection
    checks:
      - USB_RETURN_PATH_VALID
    target:
      component: J1
    parameters:
      max_data_line_unreferenced_length_mm: 0.0
      max_data_via_to_ground_stitch_distance_mm: 0.5
      require_filled_zone_coverage: true
      min_data_line_filled_zone_edge_clearance_mm: 0.25
      require_ground_zone_contact_evidence: true
```

USB return-path algorithm:

1. Resolve `target.component` and its `usb_connector` metadata.
2. Resolve D+ and D- nets from connector pin connectivity.
3. Require `board.layout.routes` entries for both data nets.
4. Find `board.layout.zones` entries whose net is declared `kind: ground`.
5. For each D+/D- route segment, require the segment midpoint to fall inside a
   same-layer ground-zone polygon. By default this uses the zone outline. When
   `require_filled_zone_coverage` is `true`, this uses saved
   `filled_polygons` evidence instead.
6. Sum unreferenced segment length and require each data net to stay within
   `max_data_line_unreferenced_length_mm`.
7. If `max_data_via_to_ground_stitch_distance_mm` is declared, require each
   USB data route via to have a ground-net via within that distance whose
   layer list covers the data-via layer transition.
8. If `min_data_line_filled_zone_edge_clearance_mm` is declared, require each
   D+/D- segment midpoint to be inside same-layer filled ground copper and at
   least that far from the nearest filled-polygon edge.
9. If `require_ground_zone_contact_evidence` is `true`, a ground zone only
   counts when imported pad or route-via evidence shows same-net contact on the
   same layer. Imported pads come from `board.layout.pads`; stitching vias come
   from same-net `board.layout.routes` via evidence. When supported pad
   shape/size evidence is available, pad contact is checked against the pad
   copper extent; otherwise it falls back to pad-center containment. When
   filled-zone coverage is required, the pad copper or via contact point must
   overlap the same saved `filled_polygon` as the route segment midpoint.
10. Treat this as an early layout screen only. Filled-polygon containment plus
   same-net pad/via contact evidence is stronger than outline containment but
   still does not prove zone island
   connectivity, adjacent-plane return paths, stitching-via inductance,
   impedance, or USB eye margin.

For controlled level shifters, declare the disabled control state in the
component model and prove it in the scenario:

```yaml
signal_conditioning:
  supply_constraints:
    - name: vcca_lte_vccb
      relation: less_than_or_equal
      lower_supply_pin: VCCA
      upper_supply_pin: VCCB
  channels:
    - name: ch1
      kind: level_shifter
      side_a_pin: A1
      side_b_pin: B1
      side_a_supply_pin: VCCA
      side_b_supply_pin: VCCB
      direction: bidirectional
      unpowered_isolation: false
      enable_pin: OE
      disabled_state: low

scenarios:
  - name: level_shifter_unpowered_side
    type: interface_protection
    target: { component: U3 }
    parameters: { channel: ch1 }
    checks:
      - INTERFACE_PROTECTION_REVIEW
    pin_states:
      - component: U3
        pin: OE
        mode: input
        state: low
```

This is a static datasheet-contract check. It does not prove propagation delay,
edge rate, leakage, dynamic clamp current, ESD pulse behavior, USB eye margin,
or analog waveform margin. Those still need datasheet-backed component models and
`analog_transient` scenarios where relevant.

## Clock Source Scenario Shape

`clock` scenarios validate external crystal support networks declared by
component models. This is a static schematic check, not oscillator startup
simulation.

```yaml
scenarios:
  - name: hse_crystal_support
    type: clock
    target:
      component: U1
    checks:
      - CLOCK_SOURCE_VALID
```

`CLOCK_SOURCE_VALID` checks:

1. The target component model declares `clock_sources[]` with distinct
   oscillator input/output pins.
2. Those pins are connected to distinct nets.
3. A component whose model declares `crystal` is connected between the two
   oscillator nets.
4. Each oscillator net has a positive-valued Board IR capacitor to ground.
5. Effective load capacitance is computed as
   `C1*C2/(C1+C2) + stray_capacitance_F`.
6. The effective load capacitance must fall within the crystal model's
   `load_capacitance_F ± load_capacitance_tolerance_F`. If no explicit
   tolerance is declared, the rule uses ±20% as a conservative screen.

The rule catches common schematic errors such as missing load capacitors or
support capacitors sized for the wrong crystal CL. It does not prove negative
resistance, startup time, ESR margin, drive level, temperature stability, ppm
accuracy, or layout parasitics.

## Power Tree Scenario Shape

`power_tree` scenarios validate declared rail metadata, model power-port
requirements, and explicit static regulator conversion metadata. This is a
deterministic board-rule check, not a full regulator or SMPS transient
simulation.

```yaml
scenarios:
  - name: power_tree_nominal
    type: power_tree
    checks:
      - POWER_TREE_VALID
      - IO_VOLTAGE_COMPATIBLE
```

`POWER_TREE_VALID` checks:

1. Component model ports with `kind: electrical_power` resolve to declared
   `kind: power` nets.
2. The rail is declared `powered: true` for this scenario.
3. The rail has a finite positive `nominal_voltage`.
4. If the model power port declares `operating_voltage_min_V` or
   `operating_voltage_max_V`, the rail nominal voltage must be inside that
   range.
5. If a rail declares `supply_current_limit_A`, every non-source component load
   on that rail must declare `max_supply_current_A`, and the summed worst-case
   current must not exceed the limit.
6. If a component model declares `power_conversion`, the declared input and
   output pins must name distinct `electrical_power` model ports and be
   connected to rails. Invalid conversion metadata fails closed.
7. If `power_conversion.dropout_voltage_V` is declared, the nominal input minus
   output voltage must meet that dropout margin.
8. If `power_conversion.min_output_current_A` is declared, output-rail loads
   must prove enough always-on current with `min_supply_current_A`.
9. If `power_conversion.max_output_current_A` is declared, every output-rail
   load must declare `max_supply_current_A`, and the summed worst-case output
   load must not exceed the regulator limit.
10. If `power_conversion.startup_delay_us` is declared, input and output rails
   must declare `power_valid_at_us`, and the output rail may not become valid
   before `input_power_valid_at_us + startup_delay_us`.
11. If `power_conversion.input_capacitance_min_F` or
    `power_conversion.output_capacitance_min_F` is declared, the corresponding
    regulator rail must have at least that much explicit Board IR capacitance
    to ground.
12. If `power_conversion.input_inductance_min_H` or
    `power_conversion.input_inductance_max_H` is declared, the model must also
    declare `switch_pin`, the board must connect it to a switch net, and the
    board must have direct modeled Board IR inductance between the regulator
    input rail and that switch net within the declared range.
13. If `power_conversion.output_inductance_min_H` or
    `power_conversion.output_inductance_max_H` is declared, the model must also
    declare `switch_pin`, the board must connect it to a switch net, and the
    board must have direct modeled Board IR inductance between that switch net
    and the regulator output rail within the declared range.
14. If `power_conversion.switch_inductance_min_H` or
    `power_conversion.switch_inductance_max_H` is declared, the model must also
    declare `switch_inductor_pin_a` and `switch_inductor_pin_b`, the board must
    connect both pins, and the board must have direct modeled Board IR
    inductance between those two switch-pin nets within the declared range.
15. If a component model declares `power_switch`, the declared input and output
    pins must name distinct `electrical_power` ports, the control pin must be a
    digital input/IO port, and a powered output rail must have matching
    scenario `pin_states` evidence for the required enabled state.
16. If `power_switch.max_output_current_A` is declared, every switched-output
    rail load must declare `max_supply_current_A`, and the summed worst-case
    output load must not exceed the switch limit.
17. If a component model declares `reset_supervisor`, the monitored pin must be
    an `electrical_power` port connected to a power rail, and the reset output
    must be a digital output/IO port connected to a net.
18. The monitored rail nominal voltage must be above the supervisor
    `threshold_max_V`, and `threshold_min_V` must not be below the highest
    powered-load `operating_voltage_min_V` on that rail.
19. If a component model declares `battery_charger`, the declared input and
    battery pins must name distinct `electrical_power` ports and be connected
    to rails. Invalid charger metadata fails closed.
20. If `battery_charger.charge_current_parameter` is declared, the component
    instance must provide that numeric parameter. The programmed current must
    fit `min_charge_current_A` / `max_charge_current_A` when present.
21. If the charger input rail declares `supply_current_limit_A`, the programmed
    charge current must fit that input-source budget.
22. If `battery_charger.regulation_voltage_V` is declared and the battery net
    has `nominal_voltage`, the battery net may not exceed the regulation
    voltage.
23. If a component model declares `power_mux`, the output and all input pins
    must name `electrical_power` ports and be connected to rails.
24. If `power_mux.selected_input_parameter` is declared, the component instance
    must provide that string parameter, and the selected input must match one
    of the model input names.
25. If the mux output rail is powered, the selected input rail must be powered.
26. If the mux output rail is powered and an inactive input rail is unpowered,
    that inactive input must declare `reverse_blocking: true`.
27. If `power_mux.max_output_current_A` is declared, every load on the mux
    output rail must declare `max_supply_current_A`, and the summed load must
    not exceed the mux output-current limit.

`IO_VOLTAGE_COMPATIBLE` can be declared on the same `power_tree` scenario. It
checks same-net digital output/input pairs when both sides have enough
component-model metadata:

1. If an output declares `drive_high_voltage_V` and an input declares
   `vih_min_V`, the output high level must meet the receiver VIH threshold.
2. If an output declares `drive_high_voltage_V` and `source_impedance_ohm`, and
   the input declares `injection_current_limit_A`, the rule estimates clamp
   current against the receiver's powered rail:
   `max(0, driver_high_voltage_V - receiver_rail_voltage_V - diode_drop_V) /
   source_impedance_ohm`.
3. `parameters.diode_drop_V` defaults to `0.3`.

When Board IR components include imported
`source.board_pin_electrical_types`, the scan also applies that schematic
evidence. A model output participates as a driver only if the imported KiCad
pin type is output-capable, and a model input participates as a receiver only
if the imported KiCad pin type is input-capable. Missing imported pin-type
metadata keeps the model-only behavior.

This rule is intended to catch common IoT mistakes such as a 3.3 V MCU tied to
5 V, an unpowered rail marked as valid for logic checks, or an undersized
regulator budget. The I/O compatibility companion check catches common
logic-level mistakes such as a 1.8 V interrupt driving a 3.3 V input with a high
VIH, or a 5 V output overdriving a lower-voltage receiver clamp. Load-transient
stability, inrush, load-dependent dropout, loop stability, thermal behavior,
and real ramp waveform shape still require datasheet-backed dynamic models or
`analog_transient` scenarios.

## Firmware Update Scenario Shape

`firmware_update` scenarios model abstract host/device resident-bootloader transactions:

```yaml
scenarios:
  - name: resident_update_upload_activate
    type: firmware_update
    target:
      component: U1
    checks:
      - RESIDENT_BOOTLOADER_UPDATE_SEQUENCE
    protocol:
      component: U1
      name: umbl_resident_update
      flow: upload_activate_next_log
      sender:
        component: U5
        pin: TXD
      package_size_bytes: 2048
      package_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
      expected_final_state: activate_pending
    events:
      - at_us: 8000
        action: protocol_request
        operation: status
        result_code: 0
        state: recovery_idle
      - at_us: 10000
        action: protocol_request
        operation: begin
        payload_len: 37
        result_code: 0
      - at_us: 12000
        action: protocol_request
        operation: data
        offset: 0
        chunk_len: 1024
        payload_len: 1030
        result_code: 0
```

`RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` validates the declared trace against `component.behavior.protocols[protocol.name]`:

1. Resolve the target component, protocol, and named flow.
2. Check protocol sender connectivity to the target RX pin when `transport_interface` is declared.
3. Require all protocol events to use declared operations and success result codes.
4. Check payload lengths against operation metadata and global max payload.
5. Match model flow phases, including repeat phases such as one-or-more data chunks.
6. For operations with role `data_chunk`, require non-overlapping chunk coverage of `package_size_bytes`.
7. Require the final observed state to match `expected_final_state` when declared.

This is an abstract protocol-trace check. It does not execute firmware, decode raw serial frames, recompute CRCs, emulate flash, or prove HIL behavior.

## Control-Line Sequence Scenario Shape

`control_line_sequence` scenarios model semantic host control-line effects:

```yaml
scenarios:
  - name: derived_app_boot_release
    type: control_line_sequence
    target:
      component: U1
      reset_pin: NRST
    checks:
      - CONTROL_LINE_RELEASE_SEQUENCE
    required_boot_mode: application
    timing:
      power_valid_at_us: 1200
      reset_release_at_us: 5000
      boot_sample_at_us: 5100
    control_effects:
      - name: boot_select
        source: { component: U5, pin: DTR_N }
        target: { component: U1, pin: BOOT0 }
        asserted_state: high
        released_state: low
        release_delay_us: 400
      - name: reset
        source: { component: U5, pin: RTS_N }
        target: { component: U1, pin: NRST }
        asserted_state: low
        released_state: high
        release_delay_us: 0
    events:
      - at_us: 0
        action: control_line
        line: boot_select
        asserted: true
      - at_us: 4900
        action: control_line
        line: boot_select
        asserted: false
```

`CONTROL_LINE_RELEASE_SEQUENCE` validates reduced line effects:

1. Resolve the target component, boot mode, and reset behavior.
2. Validate effect source pins as output-capable and effect target pins as input-capable on the target component.
3. Require explicit `control_line` events before reset and boot sample times; no defaults are inferred.
4. Derive reset at `reset_release_at_us` and `boot_sample_at_us`.
5. Derive boot straps at `boot_sample_at_us`.
6. Compare derived states with reset polarity and required boot-mode straps.

This is an abstract control-line timing check. It does not solve transistor storage, hidden RC networks, or physical CH340 modem-pin voltage truth tables.

## Functional MCU Firmware Scenario Shape

`firmware_in_loop` scenarios describe a functional black-box MCU check. The
runtime boundary is the firmware-visible MCU plus board-facing pins: reset/boot
state, peripheral effects, pin modes, logic states, timing, thresholds, clamps,
leakage, and other pin behavior visible to the surrounding board. It is
explicitly not a transistor-level MCU model.

```yaml
scenarios:
  - name: application_pin_behavior
    type: firmware_in_loop
    target:
      component: U1
    checks:
      - FUNCTIONAL_MCU_FIRMWARE
    firmware:
      backend: qemu
      image: firmware/app.elf
      machine: stm32l4_functional
      build:
        command: ["../urine_monitor/tools/build_stm32l431_node.sh", "--board", "um-stm32l4-v1"]
        working_dir: .
        outputs:
          - ../urine_monitor/firmware_stm32l431_node/build/stm32l431_node.elf
        timeout_ms: 120000
      qemu:
        executable: qemu-system-arm
        timeout_ms: 5000
        extra_args: []
      expected_pin_states:
        - component: U1
          pin: TX
          mode: output
          state: high
```

If `firmware.build` is present, CircuitCI runs it before checking
`firmware.image`. `build.command` is an explicit argv array, not a shell string;
`build.working_dir` defaults to the project directory; `build.outputs` are
verified as files and recorded as artifacts; and `build.timeout_ms` bounds the
build. This lets a scenario invoke repo-local MCU build scripts such as the
peer `../urine_monitor` STM32 wrappers without assuming the compiler is
globally on `PATH`.

The QEMU backend runs `qemu-system-arm` by default with `-M <machine>`,
`-kernel <image>`, `-nographic`, and `-semihosting`; `qemu.extra_args` are
appended as explicit argv entries. `qemu.executable` can point to a specific
QEMU binary, `qemu.timeout_ms` bounds execution, and `qemu.pin_trace_prefix`
overrides the default `CIRCUITCI_PIN ` trace prefix. `backend: auto` selects
QEMU only when a machine is declared and QEMU is available.

A passing firmware-in-loop result must come from executing the functional model
and observing declared pin behavior. The QEMU run must emit one line per
observed pin using this format:

```text
CIRCUITCI_PIN U1.TX mode=output state=high
```

Valid modes are `input`, `output`, and `high_z`; valid states are `high`,
`low`, and `z`. Missing, malformed, conflicting, or mismatched observations
produce `FUNCTIONAL_MCU_FIRMWARE` critical findings. Renode remains fail-closed
until a Renode adapter is integrated. Firmware-in-loop pass/fail must not be
inferred from a transistor-level MCU substitute or from a generic "firmware
present" marker.

## Analog Transient Scenario Shape

`analog_transient` scenarios require a SPICE-compatible deck, model artifacts,
board-to-SPICE node bindings, and quantitative waveform assertions:

```yaml
scenarios:
  - name: q2_q3_downloader_release_transient
    type: analog_transient
    checks:
      - SPICE_TRANSIENT_ANALYSIS
    analog:
      backend: auto
      netlist: downloader_q2_q3.cir
      model_files:
        - path: models/downloader_common.lib
      node_bindings:
        - node: boot0
          net: boot0
      pin_bindings:
        - node: boot0
          endpoint:
            component: U1
            pin: BOOT0
      analysis:
        type: tran
        stop_time_us: 8000
        max_step_us: 1
      stimuli:
        - name: host_control_release
          description: DTR_N and RTS_N release sequence encoded in the deck.
      probes:
        - name: boot0
          expression: V(boot0)
      assertions:
        - name: boot0_low_before_app_sample
          probe: boot0
          at_us: 5100
          relation: below
          threshold_v: 0.99
          suggested_fixes:
            - Rework the BOOT0 driver so the measured waveform meets the declared threshold.
```

This scenario type is the physical analog path. If no SPICE-class backend is
available, or if the runtime cannot execute the deck and evaluate waveforms, the
scenario must fail with a critical analog finding.
