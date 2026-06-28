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
- `manufacturing`
- `analog_transient`
- `analog_ac`
- `analog_dc`
- `motor_drive`
- `load_budget`
- `model_quality`

Unsupported scenario types must produce an explicit low-confidence limitation or informational finding, not a crash.

## Scenario Resolution

For the first Rust implementation:

1. The CLI loads the requested profile name for report metadata and profile
   coverage annotation.
2. Project-declared scenarios are the executable source of truth.
3. A scenario runs each check in its `checks` list once, preserving file order.
4. Duplicate checks in one scenario are de-duplicated with first occurrence winning.
5. Unsupported checks produce `UNSUPPORTED_CHECK` limitations.
6. Unsupported scenario types produce `UNSUPPORTED_SCENARIO` limitations.
7. `iot_basic_v0` reports a non-blocking `PROFILE_COVERAGE_PARTIAL`
   limitation when the project does not declare the core executable checks
   needed for full-profile sign-off.

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
- `BUS_TERMINATION_VALID`
- `BUS_PROTECTION_PLACEMENT_VALID`
- `CLOCK_SOURCE_VALID`
- `POWER_TREE_VALID`
- `MOTOR_BRIDGE_BUDGET_VALID`
- `MOTOR_LOAD_SUPPLY_VALID`
- `MOTOR_BRIDGE_LOSS_THERMAL_VALID`
- `MOTOR_BRIDGE_SWITCHING_VALID`
- `MOTOR_BRIDGE_SOA_VALID`
- `MOTOR_REGEN_CLAMP_VALID`
- `MOTOR_ROUTE_CURRENT_VALID`
- `MOTOR_CURRENT_SENSE_ACCURACY_VALID`
- `MOTOR_CURRENT_SENSE_PLACEMENT_VALID`
- `MODEL_QUALITY_REQUIRED`
- `POWER_SWITCH_BUDGET_VALID`
- `POWER_SWITCH_REVERSE_CURRENT_VALID`
- `POWER_SWITCH_INRUSH_VALID`
- `LOAD_CABLE_CURRENT_VALID`
- `LOAD_CABLE_THERMAL_DERATING_VALID`
- `LOAD_CABLE_VOLTAGE_DROP_VALID`
- `DRILL_DIAMETER_VALID`
- `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID`
- `SLOT_TO_BOARD_EDGE_CLEARANCE_VALID`
- `SLOT_WIDTH_VALID`
- `SLOT_ASPECT_RATIO_VALID`
- `DRILL_ANNULAR_RING_VALID`
- `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID`
- `COPPER_SPACING_VALID`
- `CONDUCTOR_CREEPAGE_CLEARANCE_VALID`
- `CONTROLLED_IMPEDANCE_GEOMETRY_VALID`
- `ADJACENT_PLANE_RETURN_PATH_VALID`
- `REFERENCE_PLANE_SLOT_CROSSING_VALID`
- `SOLDER_MASK_OPENING_VALID`
- `SOLDER_MASK_DAM_VALID`
- `SOLDER_PASTE_OPENING_VALID`
- `SOLDER_PASTE_APERTURE_SIZE_VALID`
- `SOLDER_PASTE_APERTURE_AREA_RATIO_VALID`
- `SOLDER_PASTE_IC_PIN_APERTURE_VALID`
- `SOLDER_PASTE_BGA_APERTURE_VALID`
- `SOLDER_PASTE_SPACING_VALID`
- `ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID`
- `PIN_1_ORIENTATION_VALID`
- `IO_VOLTAGE_COMPATIBLE`
- `SPICE_TRANSIENT_ANALYSIS`
- `SPICE_AC_ANALYSIS`
- `SPICE_DC_ANALYSIS`
- `SPICE_NOISE_ANALYSIS`

`SPICE_OPERATING_LIMIT` is not declared as a separate scenario check. It is an
automatic critical finding emitted by `SPICE_TRANSIENT_ANALYSIS` when generated
Board IR device waveforms exceed datasheet absolute maximum ratings.

## Model Quality Sign-Off

Use `model_quality` scenarios with `MODEL_QUALITY_REQUIRED` when a board is
being reviewed for fabrication and named components must not rely on generic,
estimated, or low-confidence envelopes.

```yaml
scenarios:
  - name: selected_load_evidence_gate
    type: model_quality
    checks:
      - MODEL_QUALITY_REQUIRED
    parameters:
      components: [M1, REGEN1]
      allowed_sources: [datasheet, measured]
      min_confidence: medium
```

The check:

1. Requires `parameters.components` or `target.component`.
2. Requires non-empty `parameters.allowed_sources`.
3. Requires `parameters.min_confidence` to be `low`, `medium`, or `high`.
4. Fails if any named board component is missing, unbound, bound to a missing
   model, has `model_quality.source` outside the allowed list, or has
   confidence below the threshold.

`LOW_CONFIDENCE_MODEL` limitations remain non-blocking report context. Use
`MODEL_QUALITY_REQUIRED` for selected components that must block fabrication
sign-off.

## Load Cable Current

Use `load_budget` scenarios with `LOAD_CABLE_CURRENT_VALID` when a load current
must be checked against a selected cable or harness assembly, not only the PCB
connector.

```yaml
scenarios:
  - name: wheel_actuator_bus_cable_budget
    type: load_budget
    checks:
      - LOAD_CABLE_CURRENT_VALID
    target:
      component: PWR_STAGE
      power_pin: VM
    parameters:
      cable_component: WHEEL_CABLE1
      min_cable_current_margin_ratio: 1.5
```

The check derives load current from `target.power_pin.max_supply_current_A`.
Cable current can come from `parameters.cable_current_rating_A` or from a
`parameters.cable_component` model with `cable_assembly.current_rating_A`.
`parameters.cable_voltage_rating_V` or `cable_assembly.voltage_rating_V` can
also screen nominal load voltage. Missing cable evidence is a critical
`VALIDATION_INPUT_MISSING` finding so schematic/CAD bridges cannot imply
fabrication readiness without a selected harness.

## Load Cable Thermal Derating

Use `load_budget` scenarios with `LOAD_CABLE_THERMAL_DERATING_VALID` when a
load current must be checked against selected cable or harness
temperature-rise evidence.

```yaml
scenarios:
  - name: wheel_actuator_bus_cable_thermal_derating
    type: load_budget
    checks:
      - LOAD_CABLE_THERMAL_DERATING_VALID
    target:
      component: PWR_STAGE
      power_pin: VM
    parameters:
      cable_component: WHEEL_CABLE1
      thermal_current_margin_ratio: 1.5
```

The check derives load current from `target.power_pin.max_supply_current_A`,
multiplies it by `thermal_current_margin_ratio` when present, and estimates
temperature rise by I^2 scaling from a declared test point:
`rise_at_test_current * (thermal_current / test_current)^2`.
`cable_temperature_rise_test_current_A`,
`cable_temperature_rise_at_test_current_C`, and
`max_cable_temperature_rise_C` can be explicit scenario parameters, or can come
from a `parameters.cable_component` model with matching `cable_assembly`
metadata. Missing thermal evidence is a critical `VALIDATION_INPUT_MISSING`
finding.

## Load Cable Voltage Drop

Use `load_budget` scenarios with `LOAD_CABLE_VOLTAGE_DROP_VALID` when a load
current must be checked against selected cable or harness loop resistance,
voltage-drop, and optional power-loss limits.

```yaml
scenarios:
  - name: wheel_actuator_bus_cable_voltage_drop
    type: load_budget
    checks:
      - LOAD_CABLE_VOLTAGE_DROP_VALID
    target:
      component: PWR_STAGE
      power_pin: VM
    parameters:
      cable_component: WHEEL_CABLE1
      max_cable_voltage_drop_V: 0.3
      max_cable_power_loss_W: 2.0
      drop_current_margin_ratio: 1.5
```

The check derives load current from `target.power_pin.max_supply_current_A`,
multiplies it by `drop_current_margin_ratio` when present, and computes
`voltage_drop = current * loop_resistance` and
`power_loss = current^2 * loop_resistance`. Loop resistance can come from
`parameters.cable_loop_resistance_ohm` or `cable_assembly.loop_resistance_ohm`.
Voltage-drop and power-loss limits can be scenario parameters or selected
`cable_assembly` metadata. Missing loop-resistance or voltage-drop evidence is
a critical `VALIDATION_INPUT_MISSING` finding.

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

Bus termination review uses `BUS_TERMINATION_VALID` with explicit topology
metadata. It does not infer that every CAN/RS485 node should be terminated.
Endpoint role, resistor component, expected resistance, and tolerance must be
declared by the scenario.

```yaml
scenarios:
  - name: can_endpoint_termination
    type: interface_protection
    checks:
      - BUS_TERMINATION_VALID
    parameters:
      line_a_net: robot_canh
      line_b_net: robot_canl
      board_is_bus_endpoint: true
      termination_component: RT_CAN
      expected_termination_ohm: 120.0
      termination_tolerance_percent: 5.0
```

Bus termination algorithm:

1. Require `line_a_net`, `line_b_net`, `board_is_bus_endpoint`,
   `expected_termination_ohm`, and `termination_tolerance_percent`.
2. Require both bus nets to be declared and distinct.
3. When `board_is_bus_endpoint` is true, require
   `termination_component`.
4. Resolve that component, require `spice.primitive: resistor`, require
   positive `spice.value_ohm`, and require the resistor to connect directly
   across the two declared bus nets.
5. Require the resistor value to fit `expected_termination_ohm` within
   `termination_tolerance_percent`.
6. When `board_is_bus_endpoint` is false, a supplied termination component
   connected across the two nets is a critical finding, because non-endpoint
   nodes must not add local termination in a two-endpoint bus topology.

Bus protection placement uses `BUS_PROTECTION_PLACEMENT_VALID` with explicit
layout evidence. It is intended for CAN/RS485-style bus TVS and termination
placement checks where a project has a chosen layout policy, not for generic
signal-integrity sign-off.

```yaml
scenarios:
  - name: can_esd_route_placement
    type: interface_protection
    checks:
      - BUS_PROTECTION_PLACEMENT_VALID
    parameters:
      line_a_net: robot_canh
      line_b_net: robot_canl
      reference_component: JACT1
      checked_component: UCAN_ESD1
      max_reference_to_checked_route_distance_mm: 5.0
      max_component_to_route_distance_mm: 0.25
```

Bus placement algorithm:

1. Require `line_a_net`, `line_b_net`, `reference_component`,
   `checked_component`, `max_reference_to_checked_route_distance_mm`, and
   `max_component_to_route_distance_mm`.
2. Require the two line nets to be declared and distinct.
3. Require `board.layout.placements` entries for the reference and checked
   components with finite coordinates.
4. Require `board.layout.routes` entries for both line nets. Each route must
   have positive-width segments, non-empty layers, finite endpoints, and an
   ordered continuous polyline.
5. Project both component coordinates onto each line route. Both projections
   must be within `max_component_to_route_distance_mm`.
6. Require the route distance between the projected reference and checked
   component on both lines to fit
   `max_reference_to_checked_route_distance_mm`.

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

Scenario `parameters.max_connector_to_protection_distance_mm` takes precedence.
If it is omitted or left `null`, `board.layout.constraints.usb_connector` can
provide `max_connector_to_protection_distance_mm` from an explicit board
ESD/layout rule.

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
edge and using its outward normal. If
`board.layout.constraints.usb_connector.max_connector_rotation_error_deg` is
also present, the suggestion becomes runnable. The inferred direction should
still be checked against the footprint's connector-entry rotation convention.

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

Scenario `parameters.max_connector_to_board_edge_distance_mm` takes precedence.
If it is omitted or left `null`, `board.layout.constraints.usb_connector` can
provide the connector/enclosure mechanical rule.

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

Scenario `parameters.max_connector_body_overhang_mm` takes precedence. If it is
omitted or left `null`, `board.layout.constraints.usb_connector` can provide the
connector/enclosure overhang rule.

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

Scenario `parameters.min_connector_to_component_clearance_mm` takes precedence.
If it is omitted or left `null`, `board.layout.constraints.usb_connector` can
provide the connector keepout or assembly clearance rule.

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

Manufacturing scenarios may use `parameters.fabrication_process` to fill
source-backed defaults for supported numeric limits. Explicit numeric
parameters always override the preset. `fabrication_process` may be one
preset string or a list of preset strings. The supported presets are documented
in `docs/fabrication_process_presets.md`. Current JLCPCB presets cover selected
mask expansion/dam, via annular ring, slot width/aspect ratio, drill diameter,
castellated-hole, copper spacing, routed-edge copper clearance, stencil
aperture size, and stencil aperture area-ratio limits where the source
condition is narrow enough to encode.

Drill-diameter validation uses `DRILL_DIAMETER_VALID` when the Board IR
includes circular fabrication drill evidence under `board.layout.drills`.

```yaml
scenarios:
  - name: drill_diameter
    type: manufacturing
    checks:
      - DRILL_DIAMETER_VALID
    parameters:
      fabrication_process: jlcpcb_drill_diameter_range_2026_06
```

Drill-diameter algorithm:

1. Require `parameters.min_drill_diameter_mm` and
   `parameters.max_drill_diameter_mm`, either explicitly or from a fabrication
   process preset.
2. Require finite `board.layout.drills[]` entries with positive `drill_mm`.
3. Check every imported circular drill hit against the selected diameter range.
4. Fail when a drill diameter is smaller than `min_drill_diameter_mm` or larger
   than `max_drill_diameter_mm`.

This is a static circular-drill process screen. It does not model routed slots,
drill wander, plating thickness, tolerance classes, or special-order drill
processes. Routed slots are checked separately by `SLOT_WIDTH_VALID`.

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

1. Resolve `min_drill_edge_clearance_mm` from the scenario parameter first,
   then from any selected process preset, then from
   `board.manufacturing.min_drill_edge_clearance_mm`.
2. Require finite `board.layout.drills[]` entries with positive `drill_mm`.
3. Require finite `board.layout.outline.segments[]` entries.
4. Measure each drill center to the nearest outline segment and subtract drill
   radius.
5. Fail when any drill edge-to-outline clearance is below
   `min_drill_edge_clearance_mm`.

External board-outline segments, cutout segments, and unknown outline segments
all count as board edges for this check. This is a static 2D centerline
fabrication screen; it does not model drill wander, plating
barrel tolerances, panel tabs, fab-specific minimums, or copper-to-hole
clearance.

Slot-to-board-edge clearance uses `SLOT_TO_BOARD_EDGE_CLEARANCE_VALID` when the
Board IR includes routed-slot evidence under `board.layout.slots` and
board-outline segment evidence under `board.layout.outline.segments`.

```yaml
scenarios:
  - name: slot_to_board_edge_clearance
    type: manufacturing
    checks:
      - SLOT_TO_BOARD_EDGE_CLEARANCE_VALID
    parameters:
      min_slot_edge_clearance_mm: 0.5
```

Slot-to-board-edge algorithm:

1. Resolve `min_slot_edge_clearance_mm` from the scenario parameter first, then
   from any selected process preset, then from
   `board.manufacturing.min_slot_edge_clearance_mm`.
2. Require finite `board.layout.slots[]` entries with positive `width_mm` and
   non-zero start/end centerline length.
3. Require finite `board.layout.outline.segments[]` entries.
4. Measure each slot centerline to the nearest outline segment and subtract
   half the routed slot width.
5. Fail when any slot edge-to-outline clearance is below
   `min_slot_edge_clearance_mm`.

External board-outline segments, cutout segments, and unknown outline segments
all count as board edges for this check. This is a static 2D routed-slot
capsule screen; it does not model tool runout, route overcut, plating
tolerances, panel tabs, fab-specific minimums, or 3D mechanical fit.

Slot-width validation uses `SLOT_WIDTH_VALID` when the Board IR includes routed
slot evidence under `board.layout.slots`.

```yaml
scenarios:
  - name: slot_width
    type: manufacturing
    checks:
      - SLOT_WIDTH_VALID
    parameters:
      fabrication_process: jlcpcb_slot_min_2026_06
```

Slot-width algorithm:

1. Require `parameters.min_plated_slot_width_mm` and
   `parameters.min_non_plated_slot_width_mm`, either explicitly or from a
   fabrication process preset.
2. Require finite `board.layout.slots[]` entries with positive `width_mm` and
   non-zero start/end centerline length.
3. Check `plated` slots against `min_plated_slot_width_mm`.
4. Check `non_plated` slots against `min_non_plated_slot_width_mm`.
5. Check `unknown` plating slots against the stricter of the plated and
   non-plated limits because the imported drill file did not prove which
   process applies.
6. Fail when any slot width is below the selected process limit.

This is a static routed-slot process screen. It does not model route-tool
runout, plating thickness, slot end-shape tolerance, milling compensation, or
mechanical fit.

Slot aspect-ratio validation uses `SLOT_ASPECT_RATIO_VALID` when the Board IR
includes routed slot evidence under `board.layout.slots`.

```yaml
scenarios:
  - name: slot_aspect_ratio
    type: manufacturing
    checks:
      - SLOT_ASPECT_RATIO_VALID
    parameters:
      fabrication_process: jlcpcb_slot_min_2026_06
```

Slot aspect-ratio algorithm:

1. Require `parameters.min_slot_aspect_ratio`, either explicitly or from a
   fabrication process preset.
2. Require finite `board.layout.slots[]` entries with positive `width_mm` and
   non-zero start/end centerline length.
3. Compute each slot's centerline length divided by `width_mm`.
4. Fail when any routed slot aspect ratio is below `min_slot_aspect_ratio`.

The JLCPCB slot preset currently supplies `min_slot_aspect_ratio: 2.5` from the
saved via-design source. This check does not replace slot-width or slot-edge
clearance screening; it catches very short routed slots that are difficult to
process even when their width is otherwise supported.

Castellated-hole validation uses `CASTELLATED_HOLE_VALID` when the Board IR
includes explicit castellated drill evidence under `board.layout.drills` and
board-outline segment evidence under `board.layout.outline.segments`.

```yaml
scenarios:
  - name: castellated_hole
    type: manufacturing
    checks:
      - CASTELLATED_HOLE_VALID
    parameters:
      fabrication_process: jlcpcb_castellated_hole_2026_06
```

Castellated-hole algorithm:

1. Require `parameters.min_castellated_hole_diameter_mm` and
   `parameters.min_castellated_hole_edge_clearance_mm`, and
   `parameters.min_castellated_hole_to_hole_spacing_mm`, either explicitly or
   from a fabrication process preset.
2. Require at least one finite `board.layout.drills[]` entry with
   `castellated: true`.
3. Require finite `board.layout.outline.segments[]` entries.
4. Check only the explicitly castellated drill entries.
5. Fail when a castellated drill diameter is smaller than
   `min_castellated_hole_diameter_mm`.
6. Measure each castellated drill center to the nearest outline segment,
   subtract drill radius, and fail when the hole-edge-to-board-edge clearance is
   below `min_castellated_hole_edge_clearance_mm`.
7. Measure every pair of castellated drill centers and subtract both drill
   radii. Fail when adjacent hole-edge spacing is below
   `min_castellated_hole_to_hole_spacing_mm`.

This is a condition-specific static 2D screen for castellated-hole evidence. It
does not change generic `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID` behavior and does
not infer castellated holes from anonymous Excellon drill geometry.

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

For JLCPCB double-sided or multilayer via minimum annular ring, the raw
`min_annular_ring_mm` parameter may be replaced by:

```yaml
parameters:
  fabrication_process:
    - jlcpcb_standard_2026_06
    - jlcpcb_double_sided_via_min_2026_06
```

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
      fabrication_process: jlcpcb_routed_edge_copper_clearance_2026_06
```

Copper-to-board-edge algorithm:

1. Require `parameters.min_copper_edge_clearance_mm` or a fabrication process
   preset that provides it.
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

For JLCPCB routed board-edge and routed-slot copper clearance, the raw
`min_copper_edge_clearance_mm` parameter may be replaced by:

```yaml
parameters:
  fabrication_process: jlcpcb_routed_edge_copper_clearance_2026_06
```

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
      fabrication_process: jlcpcb_1oz_copper_spacing_2026_06
```

Copper spacing algorithm:

1. Require `parameters.min_copper_spacing_mm`, or a `fabrication_process`
   preset that supplies it.
2. Require at least two finite copper features, copper segments, or copper
   regions.
3. Compare same-layer copper feature/feature, feature/segment,
   feature/region, segment/segment, segment/region, and region/region pairs.
4. Use supported `circle`, `rect`, and axis-aligned `oval` flash geometry plus
   circular-aperture trace segment width and simple region polygon
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

Conductor creepage/clearance validation uses
`CONDUCTOR_CREEPAGE_CLEARANCE_VALID` when the Board IR includes same-layer
imported copper objects with explicit `net` ownership. The scenario must
declare the compared net pairs and both distance limits; CircuitCI does not
infer safety requirements from net names, voltages, or board metadata.

```yaml
scenarios:
  - name: hv_to_ground_spacing
    type: manufacturing
    checks:
      - CONDUCTOR_CREEPAGE_CLEARANCE_VALID
    parameters:
      net_pairs:
        - first_net: HV
          second_net: GND
          min_clearance_mm: 0.25
          min_creepage_mm: 0.30
```

Conductor creepage/clearance algorithm:

1. Require `parameters.net_pairs[]` entries with `first_net`, `second_net`,
   `min_clearance_mm`, and `min_creepage_mm`.
2. Require both named nets to exist under `board.nets`.
3. Compare only same-layer copper features, segments, or regions whose explicit
   `net` values match the declared pair.
4. Use the existing imported-copper 2D spacing geometry for supported flash,
   trace-segment, and simple region evidence.
5. Report a failure when the planar spacing is below either declared limit.
6. Fail closed when no same-layer imported copper evidence exists for a
   declared pair.

The measured `clearance_distance_mm` and `creepage_distance_mm` are currently
the same planar same-layer copper spacing. This screen does not model slots,
barriers, conformal coating, board-edge paths, layer-to-layer insulation,
pollution degree, material group, altitude, or electric fields. Use it as an
explicit geometry-evidence gate, not as final safety-standard certification.

Controlled-impedance geometry validation uses
`CONTROLLED_IMPEDANCE_GEOMETRY_VALID` when Board IR includes imported
`board.layout.routes` evidence and the scenario declares reviewed target
geometry from a stackup calculator, fabrication table, coupon requirement, or
other explicit source. CircuitCI does not calculate impedance from dielectric
constants or infer impedance targets from net names.

```yaml
scenarios:
  - name: controlled_impedance_geometry
    type: manufacturing
    checks:
      - CONTROLLED_IMPEDANCE_GEOMETRY_VALID
    parameters:
      nets:
        - net: RF
          source: fab_stackup_table_rev_a
          target_impedance_ohm: 50
          expected_width_mm: 0.20
          max_width_error_mm: 0.03
      differential_pairs:
        - first_net: DP
          second_net: DM
          source: fab_stackup_table_rev_a
          target_differential_impedance_ohm: 90
          expected_width_mm: 0.15
          expected_gap_mm: 0.20
          max_width_error_mm: 0.02
          max_gap_error_mm: 0.03
```

Controlled-impedance geometry algorithm:

1. Require at least one `parameters.nets[]` or
   `parameters.differential_pairs[]` rule.
2. Require every rule to declare a non-empty `source`, explicit impedance
   target, expected route geometry, and geometry tolerance.
3. Require every named net to exist under `board.nets` and have finite
   `board.layout.routes` segment evidence.
4. For single-ended rules, compare each route segment width against
   `expected_width_mm` and fail on the worst width error when it exceeds
   `max_width_error_mm`.
5. For differential-pair rules, compare both route widths and the worst
   same-layer parallel-overlap gap against `expected_width_mm` and
   `expected_gap_mm`.
6. Fail closed when a differential pair has no parallel overlapping same-layer
   route evidence for gap measurement.

This is a geometry-to-reviewed-target check. It does not solve characteristic
or differential impedance, model stackup materials, prove copper thickness or
etch compensation, account for solder mask, or replace a field solver,
fabricator coupon, or SI review.

Adjacent-plane return-path validation uses
`ADJACENT_PLANE_RETURN_PATH_VALID` when Board IR includes explicit
`board.layout.stackup.layers`, route segments, and reference-plane zone
polygons. It does not infer reference nets from layer names or calculate
return current; it only screens sampled route evidence against declared plane
coverage and reviewed limits.

```yaml
scenarios:
  - name: adjacent_plane_return_path
    type: manufacturing
    checks:
      - ADJACENT_PLANE_RETURN_PATH_VALID
    parameters:
      routes:
        - net: USB_D+
          reference_net: GND
          max_unreferenced_length_mm: 2.0
          reference_layer: In1.Cu
```

Adjacent-plane return-path algorithm:

1. Require non-empty `parameters.routes[]` with `net`, `reference_net`, and
   finite non-negative `max_unreferenced_length_mm`.
2. Require both nets to exist under `board.nets`.
3. Require finite non-zero `board.layout.routes.<net>.segments[]` evidence.
4. Resolve the reference plane from explicit `reference_layer` or from the
   nearest conductive stackup layer above/below the route layer, skipping
   dielectric layers. The resolved layer must be `kind: plane` and declare the
   requested `reference_net`.
5. Require `board.layout.zones.<reference_net>[]` polygons on the resolved
   plane layer.
6. Sample each route segment at start, midpoint, and end. A segment is counted
   as unreferenced unless all samples fall inside reference-plane zone
   polygons on the resolved layer.
7. Fail when total unreferenced length exceeds
   `max_unreferenced_length_mm`.

Reference-plane slot-crossing validation uses
`REFERENCE_PLANE_SLOT_CROSSING_VALID` when Board IR includes explicit
`board.layout.stackup.layers`, route segments, and reference-plane zone
polygons. It detects route centerlines that leave one reference-plane zone and
re-enter another zone on the same adjacent plane layer, which is a bounded
split-plane/slot evidence screen. It does not infer reference nets from names
or model return current.

```yaml
scenarios:
  - name: reference_plane_slot_crossing
    type: manufacturing
    checks:
      - REFERENCE_PLANE_SLOT_CROSSING_VALID
    parameters:
      routes:
        - net: USB_D+
          reference_net: GND
          reference_layer: In1.Cu
          max_slot_crossings: 0
```

Reference-plane slot-crossing algorithm:

1. Require non-empty `parameters.routes[]` with `net`, `reference_net`, and
   integer `max_slot_crossings`.
2. Require both nets to exist under `board.nets`.
3. Require finite non-zero `board.layout.routes.<net>.segments[]` evidence.
4. Resolve the reference plane from explicit `reference_layer` or from the
   nearest conductive stackup layer above/below the route layer, skipping
   dielectric layers. The resolved layer must be `kind: plane` and declare the
   requested `reference_net`.
5. Require `board.layout.zones.<reference_net>[]` polygons on the resolved
   plane layer.
6. Compute route centerline coverage intervals from segment/polygon
   intersections on the reference plane.
7. Count each internal uncovered gap between two covered intervals as one
   slot crossing.
8. Fail when the count exceeds `max_slot_crossings`.

Solder-mask opening validation uses `SOLDER_MASK_OPENING_VALID` when the Board
IR includes Gerber copper flash evidence under `board.layout.copper.features`
and Gerber solder-mask opening evidence under `board.layout.solder_mask`.
Supported mask openings include flash features, circular-aperture draw
segments, and simple regions.

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

For JLCPCB-style default mask expansion, the raw `min_mask_expansion_mm`
parameter may be replaced by:

```yaml
parameters:
  fabrication_process: jlcpcb_standard_2026_06
```

Solder-mask opening algorithm:

1. Require `parameters.min_mask_expansion_mm`.
2. Require finite Gerber copper flash features and solder-mask features,
   segments, or regions.
3. Map `F.Cu` copper to `F.Mask` openings and `B.Cu` copper to `B.Mask`
   openings.
4. For each copper flash, find the same-layer mask opening within
   `max_copper_to_mask_center_offset_mm` that gives the largest minimum
   boundary expansion.
5. Fail when no co-located opening exists.
6. Fail when the opening expands the copper flash by less than
   `min_mask_expansion_mm`.

This is a static 2D solder-mask aperture screen. It checks Gerber mask flash,
circular-aperture draw, and simple region openings. It does not solve nested or
overlapping mask-region holes, fab-specific mask swell, paste stencil behavior,
or package-specific mask rules.

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
   axis-aligned `oval`, circular-aperture linear or sampled arc draw, and
   simple region geometry.
4. Ignore different-layer opening pairs.
5. Fail when the measured opening-to-opening gap is below
   `min_solder_mask_dam_mm`.

For JLCPCB-style default mask dam width, the raw
`min_solder_mask_dam_mm` parameter may be replaced by:

```yaml
parameters:
  fabrication_process: jlcpcb_standard_2026_06
```

This is a static 2D mask web screen. It can detect thin or missing mask dams
between imported flash, linear/arc draw, and region openings, but it does not
yet evaluate nested or overlapping mask-region holes, fab-specific mask bridge
exceptions, package-specific no-dam rules, or paste stencil behavior.

Solder-paste opening validation uses `SOLDER_PASTE_OPENING_VALID` when the
Board IR includes Gerber copper flash evidence under
`board.layout.copper.features` and Gerber solder-paste opening evidence under
`board.layout.solder_paste.features`, `board.layout.solder_paste.segments`, or
`board.layout.solder_paste.regions`.

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

1. Resolve finite `min_paste_area_ratio` and `max_paste_area_ratio` from
   scenario parameters first, then process presets, then
   `board.manufacturing.min_paste_area_ratio` and
   `board.manufacturing.max_paste_area_ratio`.
2. Require `max_paste_area_ratio >= min_paste_area_ratio`.
3. Require finite Gerber copper flash features and solder-paste feature,
   segment, or region openings.
4. Skip copper features explicitly owned by vias.
5. Map `F.Cu` copper to `F.Paste` openings and `B.Cu` copper to `B.Paste`
   openings.
6. For each checked copper flash, collect same-layer paste openings whose
   feature center, segment midpoint, or region centroid is within
   `max_copper_to_paste_center_offset_mm`.
7. Fail when no co-located opening exists.
8. Sum all co-located paste-opening areas for that copper flash.
9. Fail when `paste_area_mm2 / copper_area_mm2` is outside the configured
   inclusive area-ratio range. Reports identify the nearest contributing
   opening as representative evidence and include `solder_paste_opening_count`.

This is a static 2D stencil aperture screen. It checks flash, circular-aperture
draw, and simple region area-ratio evidence, including aggregate area for
multiple co-located window apertures. It does not yet evaluate nested or
overlapping paste-region holes, step-stencil thickness, paste volume, or
package-specific paste reductions.

Solder-paste aperture-size validation uses `SOLDER_PASTE_APERTURE_SIZE_VALID`
when the Board IR includes Gerber solder-paste flash or circular-aperture draw
evidence under `board.layout.solder_paste.features` or
`board.layout.solder_paste.segments`.

```yaml
scenarios:
  - name: solder_paste_aperture_size
    type: manufacturing
    checks:
      - SOLDER_PASTE_APERTURE_SIZE_VALID
    parameters:
      fabrication_process: jlcpcb_stencil_aperture_min_2026_06
```

Solder-paste aperture-size algorithm:

1. Require `parameters.min_solder_paste_aperture_size_mm`, or a
   `fabrication_process` preset that supplies it.
2. Require solder-paste flash or circular-aperture draw evidence.
3. For flash openings, measure the smaller of `size.x_mm` and `size.y_mm`.
4. For draw openings, measure the circular aperture draw width.
5. Fail when the measured aperture size is less than or equal to the configured
   minimum, matching JLCPCB's source wording that stencil apertures must be
   greater than the minimum size.

This is a static 2D stencil manufacturability screen. It intentionally does not
apply package-specific paste reductions, paste volume rules, or arbitrary region
minimum-width approximations.

Solder-paste aperture area-ratio validation uses
`SOLDER_PASTE_APERTURE_AREA_RATIO_VALID` when the Board IR includes Gerber
solder-paste flash, circular-aperture draw, or simple region evidence.
It checks stencil release area ratio as
`opening_area_mm2 / (opening_perimeter_mm * stencil_thickness_mm)`. With
`fabrication_process: jlcpcb_stencil_area_ratio_2026_06`, the minimum area
ratio defaults to `0.66` from the saved JLCPCB/IPC-7525 source. Stencil
thickness may be supplied as `parameters.stencil_thickness_mm` or as
`board.manufacturing.stencil_thickness_mm`. The scenario parameter takes
precedence. Stencil thickness remains explicit board/order metadata because
Gerber paste layers do not encode the physical stencil thickness.

```yaml
scenarios:
  - name: solder_paste_aperture_area_ratio
    type: manufacturing
    checks:
      - SOLDER_PASTE_APERTURE_AREA_RATIO_VALID
    parameters:
      fabrication_process: jlcpcb_stencil_area_ratio_2026_06
      stencil_thickness_mm: 0.10
```

IC pin solder-paste aperture validation uses
`SOLDER_PASTE_IC_PIN_APERTURE_VALID` when the Board IR includes pad-owned Gerber
solder-paste opening evidence and the scenario declares a source-backed IC pin
pitch.

```yaml
scenarios:
  - name: solder_paste_ic_pin_aperture
    type: manufacturing
    checks:
      - SOLDER_PASTE_IC_PIN_APERTURE_VALID
    target:
      component: U1
    parameters:
      pin_pitch_mm: 0.5
```

IC pin solder-paste aperture algorithm:

1. Require `parameters.pin_pitch_mm`.
2. Map the pitch to the saved JLCPCB stencil opening standard:
   0.8-1.27 mm pitch uses aperture width 45%-60% of pitch;
   0.635-0.65 mm pitch uses 0.30-0.33 mm width and 1.00 mm length;
   0.5 mm uses 0.24 mm; 0.4 mm uses 0.19 mm; 0.35 mm uses 0.17 mm;
   and 0.3 mm uses 0.16 mm. For the 0.5 mm row, owner-matched copper pad
   evidence shorter than 1.5 mm additionally requires paste length at least the
   copper pad length plus 0.1 mm extension at each end.
3. If `target.component` is present, check only pad-owned solder-paste evidence
   for that component. Without a target, check all pad-owned paste evidence.
4. Require matching pad-owned solder-paste feature, segment, or region evidence.
5. For flash openings, measure the smaller of `size.x_mm` and `size.y_mm`.
6. For draw openings, measure the circular aperture draw width.
7. For simple regions, measure the smaller bounding-box dimension.
8. Fail when a pad-owned paste opening is outside the pitch-conditioned width
   range.
9. For source rows with an explicit length, fail when the measured aperture
   length does not match the pitch-conditioned length.
10. For source rows with a condition-scoped extension and unique owner-matched
    copper pad evidence, fail when the measured paste aperture length does not
    satisfy the copper-pad-length-derived minimum.

This is not a generic stencil capability preset. It represents JLCPCB's
package-specific IC stencil optimization table and should only be used for the
IC pin group whose pitch is declared by the scenario. The 0.5 mm row's
length-extension text is interpreted only with unique owner-matched copper pad
geometry, so it is not applied to anonymous paste evidence or unrelated pads.
`suggest-scenarios` can
infer a target-scoped `pin_pitch_mm` from repeated pad-owned paste flashes for
selected discrete source rows. Representative exact pitches inside the broad
0.8-1.27 mm source row require stronger repeated-gap evidence than the narrow
rows. The validator itself does not infer package class, step-stencil
thickness, or order remarks automatically.

Solder-paste BGA aperture validation uses
`SOLDER_PASTE_BGA_APERTURE_VALID` when the Board IR includes pad-owned Gerber
solder-paste flash evidence and the scenario declares a source-backed BGA ball
pitch.

```yaml
scenarios:
  - name: solder_paste_bga_aperture
    type: manufacturing
    checks:
      - SOLDER_PASTE_BGA_APERTURE_VALID
    target:
      component: U1
    parameters:
      pin_pitch_mm: 0.5
```

BGA solder-paste aperture algorithm:

1. Require `parameters.pin_pitch_mm`.
2. Map the pitch to the saved JLCPCB stencil opening standard BGA rows:
   0.4 mm pitch opens 0.23 mm square with rounded corners; 0.45 mm opens
   0.26 mm; 0.5 mm opens 0.30 mm; 0.65 mm opens 0.35 mm; 0.8 mm opens
   0.45 mm; 1.0 mm opens 0.55 mm; and 1.27 mm opens 0.65 mm.
3. If `target.component` is present, check only pad-owned solder-paste flash
   evidence for that component. Without a target, check all pad-owned paste
   flash evidence.
4. Require matching pad-owned solder-paste feature evidence.
5. Measure the smaller of `size.x_mm` and `size.y_mm`.
6. Fail when a pad-owned paste feature differs from the pitch-conditioned BGA
   opening size.
7. Require at least four pad-owned paste features and at least two horizontal
   plus two vertical same-pitch gaps matching `pin_pitch_mm`, so the selected
   paste evidence proves a two-axis BGA grid for the declared pitch.

This is not a generic paste area-ratio or paste-spacing rule. It represents the
BGA package rows in JLCPCB's stencil opening table. `suggest-scenarios` may
infer a target-scoped BGA `pin_pitch_mm` only when pad-owned solder-paste
flashes for one component show repeated same-pitch gaps in both horizontal and
vertical axes. That grid evidence prevents a BGA from also being suggested as a
one-dimensional IC lead row for the same component.

Solder-paste spacing validation uses `SOLDER_PASTE_SPACING_VALID` when the
Board IR includes at least two Gerber solder-paste opening objects under
`board.layout.solder_paste.features`, `board.layout.solder_paste.segments`, or
`board.layout.solder_paste.regions`.

```yaml
scenarios:
  - name: solder_paste_spacing
    type: manufacturing
    checks:
      - SOLDER_PASTE_SPACING_VALID
    parameters:
      min_solder_paste_spacing_mm: 0.15
```

Solder-paste spacing algorithm:

1. Resolve `min_solder_paste_spacing_mm` from the scenario parameter first,
   then process presets, then `board.manufacturing.min_solder_paste_spacing_mm`.
2. Require at least two finite solder-paste opening features, segments, or
   regions.
3. Compare same-layer opening pairs using supported `circle`, `rect`,
   axis-aligned `oval`, circular-aperture linear or sampled arc draw, and
   simple region geometry.
4. Ignore different-layer opening pairs.
5. Fail when the measured opening-to-opening gap is below
   `min_solder_paste_spacing_mm`.

This is a static 2D stencil web screen. It can detect merged or too-close paste
apertures between imported flash, linear/arc draw, and region openings, but it
does not model paste slump, stencil fabrication tolerance, solder wetting, or
component self-alignment.

Assembly footprint alignment validation uses
`ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID` when Board IR contains JLC/EasyEDA
assembly source metadata and imported KiCad PCB footprint or placement
evidence for the same components.
`suggest-scenarios` emits runnable target-scoped templates automatically for
components with comparable JLC assembly source evidence and imported KiCad PCB
footprint-property or source-explicit placement evidence.

```yaml
scenarios:
  - name: assembly_footprint_alignment
    type: manufacturing
    checks:
      - ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID
    parameters:
      components: [U1, R3]
      rotation_tolerance_deg: 0.01
```

Assembly footprint alignment algorithm:

1. Use `parameters.components` when provided, otherwise `scenario.target`, then
   all components whose `source.format` is `jlc_assembly`.
2. Compare assembly `source.footprint` and `source.placement_footprint` against
   imported KiCad footprint property values under
   `board.layout.footprints.<ref>.properties` using a normalized package-token
   comparison.
3. Compare assembly supplier or manufacturer part fields only when the KiCad
   footprint has comparable explicit part-number properties such as
   `JLCPCB Part`, `LCSC Part`, `Supplier Part`, `MPN`, or
   `Manufacturer Part Number`.
4. Compare explicit placement side and rotation evidence when
   `placement_side_confidence` or `placement_orientation_confidence` is
   `source_explicit` and imported layout placement evidence exists.
5. Fail closed with `VALIDATION_INPUT_MISSING` when the declared scenario has
   no comparable assembly/footprint evidence.

This is a source-consistency screen. It detects contradictions between BOM/CPL
and KiCad PCB evidence, but it does not prove final assembly polarity, visual
pin-1 interpretation, footprint land-pattern correctness, or manufacturer
package compatibility.

Pin-1 orientation validation uses `PIN_1_ORIENTATION_VALID` when Board IR
contains imported footprint `semantics.body_bounds` and `semantics.pin_1`
evidence for a target component, and the scenario supplies an explicit expected
pin-1 direction from a package or assembly drawing.
`suggest-scenarios` emits a non-runnable target-scoped template when imported
body-bounds and pad-1 marker evidence exists, but it does not guess the
expected direction.

```yaml
scenarios:
  - name: pin_1_orientation
    type: manufacturing
    checks:
      - PIN_1_ORIENTATION_VALID
    target:
      component: U1
    parameters:
      expected_pin_1_direction_deg: 180.0
      max_pin_1_direction_error_deg: 5.0
```

The measured direction is the angle from the imported footprint body center to
the imported pin-1 marker in Board IR layout coordinates. A pass only means the
imported pin-1 marker is on the expected side of the imported body within the
declared tolerance; it does not prove package polarity, assembly rotation,
land-pattern correctness, or visual silkscreen polarity.

USB route geometry uses `USB_ROUTE_GEOMETRY_VALID` when the Board IR includes
`board.layout.routes` evidence imported from PCB data. The rule always checks
D+ and D- route length plus route length mismatch. Via-count, width, gap,
pad-contact, and routed connector-to-protection distance checks run only when
their corresponding optional policy parameters are declared in the scenario or
in explicit `board.layout.constraints.usb_route` metadata.

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

For optional USB route parameters, explicit scenario values take precedence.
When a field is omitted or null, the validator uses matching
`board.layout.constraints.usb_route` metadata if present. Net-class route
width/gap values are not tolerances by themselves; they provide expected
geometry only after a tolerance such as `max_data_line_width_delta_mm` or
`max_data_pair_gap_delta_mm` has been supplied.

USB route geometry algorithm:

1. Resolve `target.component` and its `usb_connector` metadata.
2. Resolve D+ and D- nets from connector pin connectivity.
3. Require `board.layout.routes` entries for both data nets.
4. Sum routed segment lengths and require each data net to stay within
   `max_data_line_route_length_mm`.
5. If `max_data_line_via_count` is declared, count vias in each net route and
   require the count to stay within that limit.
6. If `max_data_line_width_delta_mm` is declared, resolve
   `board.layout.constraints.net_rules` for each data net and require every
   segment width to match `diff_pair_width_mm` or `track_width_mm` within that
   tolerance.
7. Require the D+/D- route length mismatch to stay within
   `max_data_pair_length_mismatch_mm`.
8. If `max_data_pair_via_count_delta` is declared, require the D+/D- via-count
   delta to stay within that limit.
9. If `max_data_pair_gap_delta_mm` is declared, resolve
   `diff_pair_gap_mm`, find overlapping parallel D+/D- routed segments, and
   require edge-to-edge gap to match within that tolerance.
10. If `max_connector_to_protection_route_distance_mm` and
   `max_component_to_route_distance_mm` are declared, project connector and
   protection component placements onto the routed net. When
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

For optional VBUS route parameters, explicit scenario values take precedence.
When a field is omitted or null, the validator uses matching
`board.layout.constraints.usb_vbus_route` metadata if present. Net-class
`track_width_mm` can still prefill route-width evidence in suggestions, but
`usb_vbus_route.min_vbus_route_width_mm` is the explicit sign-off threshold
when a board/order rule supplies one.

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
      max_vbus_via_count: 0       # optional
      min_vbus_route_width_mm: 0.30   # optional
      max_connector_to_vbus_protection_route_distance_mm: 2.0 # optional
      max_component_to_route_distance_mm: 0.2 # optional
      require_vbus_route_pad_contact_evidence: true # optional
```

USB VBUS route algorithm:

1. Resolve `target.component` and its `usb_connector` metadata.
2. Resolve the connector VBUS net and require a `board.layout.routes` entry.
3. Sum routed segment lengths and require the net to stay within
   `max_vbus_route_length_mm`.
4. If `max_vbus_via_count` is declared, count vias and require the count to
   stay within that limit.
5. If `min_vbus_route_width_mm` is declared, require every VBUS segment to be at
   least that wide.
6. If both route-distance limits are declared, project connector and VBUS
   protection component placements onto the routed net within
   `max_component_to_route_distance_mm`. When
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
a ground-net zone outline on the same copper layer. Scenario parameters take
precedence, but `board.layout.constraints.usb_return_path` can provide explicit
board-level defaults when the scenario omits them.

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
    `suggest-scenarios` may generate that evidence from a direct rail/ground tie
    or exactly one positive-valued pull resistor to the matching direct
    rail/ground state; ambiguous pulls remain manual.
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
    instance must provide that numeric parameter or the model must declare a
    source-backed `charge_current_programming` equation with exactly one
    positive programming resistor in Board IR. The programmed current must fit
    `min_charge_current_A` / `max_charge_current_A` when present.
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

## Motor Drive Scenario Shape

`motor_drive` scenarios validate explicit motor-controller and bridge budget
inputs. They are deterministic schematic-budget checks for the first design
pass, not FOC simulation, MOSFET SOA, thermal, or layout sign-off.

```yaml
scenarios:
  - name: wheel_bridge_budget
    type: motor_drive
    checks:
      - MOTOR_BRIDGE_BUDGET_VALID
    target:
      component: PWR_STAGE
    parameters:
      motor_component: M1
      bridge_reference_current_A: 15.0
      bridge_device_current_class_A: 40.0
      phase_shunt_resistance_ohm: 0.005
      phase_shunt_power_rating_W: 1.0
      min_shunt_power_margin_ratio: 2.0
      max_shunt_sense_voltage_V: 0.15
      motor_connector_current_rating_A: 8.0
      gate_resistor_ohm: 10.0
      dead_time_ns: 200.0
      pwm_frequency_Hz: 20000.0
```

`MOTOR_LOAD_SUPPLY_VALID` checks that the selected motor supply envelope covers
the declared motor bus voltage window:

```yaml
scenarios:
  - name: wheel_motor_supply_voltage
    type: motor_drive
    checks:
      - MOTOR_LOAD_SUPPLY_VALID
    target:
      component: PWR_STAGE
    parameters:
      motor_component: M1
      bus_voltage_min_V: 6.0
      bus_voltage_max_V: 12.6
```

The motor supply range may be explicit scenario parameters
`motor_supply_voltage_min_V` and `motor_supply_voltage_max_V`, or model-derived
from `parameters.motor_component` bound to `motor_load.supply_voltage_min_V`
and `motor_load.supply_voltage_max_V`. Scenario values override model values
for what-if checks. Missing or inverted ranges fail closed with
`VALIDATION_INPUT_MISSING`. This rule only checks static supply compatibility;
it does not prove torque, speed, stall, thermal, or control-loop behavior.

`MOTOR_BRIDGE_BUDGET_VALID` checks:

1. `target.component` names an existing bridge or power-stage component.
2. The target component is bound to a component model, so the scenario is not
   checking an anonymous schematic placeholder.
3. Motor current evidence comes from explicit scenario parameters
   `motor_phase_peak_current_A`, `motor_phase_rms_current_A`, and
   `max_regen_current_A`, or from `parameters.motor_component` bound to a
   component model with `motor_load.phase_peak_current_A`,
   `motor_load.phase_rms_current_A`, and `motor_load.max_regen_current_A`.
   Scenario numeric parameters override motor-component model evidence.
4. `motor_phase_rms_current_A` may not exceed
   `motor_phase_peak_current_A`.
5. `motor_phase_rms_current_A` must fit the declared
   `bridge_reference_current_A`.
6. `motor_phase_peak_current_A` must fit the declared
   `bridge_device_current_class_A`.
7. Motor phase RMS current and maximum regeneration current must fit
   `motor_connector_current_rating_A`.
8. Phase shunt dissipation is computed as
   `motor_phase_rms_current_A^2 * phase_shunt_resistance_ohm`; multiplied by
   `min_shunt_power_margin_ratio`, it must not exceed
   `phase_shunt_power_rating_W`.
9. If `max_shunt_sense_voltage_V` is supplied, peak shunt sense voltage is
   computed as `motor_phase_peak_current_A * phase_shunt_resistance_ohm` and
   must fit that range.
10. `gate_resistor_ohm`, `dead_time_ns`, and `pwm_frequency_Hz` must be finite
   positive design inputs so the schematic has an explicit gate-timing budget.

Missing or non-finite required values produce critical
`VALIDATION_INPUT_MISSING` findings. This rule is intended to catch first-order
robot motor-board mistakes such as an undersized shunt, connector, or bridge
current class before schematic capture. It does not replace gate-driver
datasheet timing review, switching-loss calculation, current-sense accuracy,
regen clamp design, thermal modeling, or PCB copper-temperature validation.

`MOTOR_BRIDGE_LOSS_THERMAL_VALID` checks a first-pass bridge voltage/current
rating and scaled reference-loss thermal budget:

```yaml
scenarios:
  - name: wheel_bridge_loss_thermal
    type: motor_drive
    checks:
      - MOTOR_BRIDGE_LOSS_THERMAL_VALID
    target:
      component: PWR_STAGE
    parameters:
      motor_component: M1
      bus_voltage_max_V: 12.6
      max_total_bridge_loss_W: 2.0
      min_loss_margin_ratio: 2.0
```

Required evidence:

- `target.component` must bind to a component model with `motor_bridge`.
- `motor_bridge.voltage_rating_V` and `motor_bridge.current_rating_A` provide
  the static device class.
- `motor_bridge.reference_loss_W`,
  `motor_bridge.reference_current_A`, and `motor_bridge.reference_loss_scope`
  provide a source-backed reference loss point. `per_half_bridge` reference
  loss additionally requires `motor_bridge.switching_devices`.
- Motor current evidence comes from explicit scenario parameters or from
  `parameters.motor_component` bound to a model with `motor_load`.
- `bus_voltage_max_V`, `max_total_bridge_loss_W`, and
  `min_loss_margin_ratio` are explicit board design inputs.

The estimated loss is
`reference_loss_W * (motor_phase_rms_current_A / reference_current_A)^2`,
multiplied by `switching_devices` when the reference loss is per half-bridge.
That estimate times `min_loss_margin_ratio` must fit
`max_total_bridge_loss_W`. This is a source-backed screening calculation; it
does not replace MOSFET SOA curves, switching transition loss, gate-charge
timing, thermal impedance, regeneration energy, or measured board temperature.

`MOTOR_BRIDGE_SWITCHING_VALID` checks a first-pass transition-loss and
average gate-charge budget from source-backed bridge switching metadata:

```yaml
scenarios:
  - name: wheel_bridge_switching_budget
    type: motor_drive
    checks:
      - MOTOR_BRIDGE_SWITCHING_VALID
    target:
      component: PWR_STAGE
    parameters:
      motor_component: M1
      bus_voltage_max_V: 12.6
      pwm_frequency_Hz: 20000.0
      gate_drive_voltage_V: 10.0
      switching_events_per_pwm_cycle: 6.0
      gate_charge_events_per_pwm_cycle: 6.0
      max_total_switching_loss_W: 0.5
      min_switching_loss_margin_ratio: 2.0
      max_average_gate_drive_current_A: 0.02
```

Required evidence:

- `target.component` must bind to a component model with `motor_bridge`.
- `motor_bridge.gate_charge_total_C`, `motor_bridge.rise_time_s`, and
  `motor_bridge.fall_time_s` must be positive source-backed metadata.
- Motor peak current comes from explicit scenario parameters or from
  `parameters.motor_component` bound to a model with `motor_load`.
- `bus_voltage_max_V`, `pwm_frequency_Hz`, `gate_drive_voltage_V`,
  `switching_events_per_pwm_cycle`, `gate_charge_events_per_pwm_cycle`,
  `max_total_switching_loss_W`, `min_switching_loss_margin_ratio`, and
  `max_average_gate_drive_current_A` are explicit board/gate-drive inputs.

The transition-loss estimate is
`0.5 * bus_voltage_max_V * motor_phase_peak_current_A *
(rise_time_s + fall_time_s) * pwm_frequency_Hz *
switching_events_per_pwm_cycle`. That value times
`min_switching_loss_margin_ratio` must fit `max_total_switching_loss_W`.
Average gate-drive charge current is
`gate_charge_total_C * pwm_frequency_Hz * gate_charge_events_per_pwm_cycle`;
it must fit `max_average_gate_drive_current_A`. This is a static screening
calculation. It does not prove peak gate source/sink current, Miller behavior,
dead-time, diode reverse recovery, switch-node ringing, PWM sampling, MOSFET
SOA, or measured switching temperature.

`MOTOR_BRIDGE_SOA_VALID` checks a static motor bridge stress point against
source-backed SOA metadata. For motor power blocks whose datasheets publish
current-versus-temperature system SOA curves, prefer
`motor_bridge.system_soa.output_current_temperature_curves`:

```yaml
scenarios:
  - name: wheel_bridge_soa_static
    type: motor_drive
    checks:
      - MOTOR_BRIDGE_SOA_VALID
    target:
      component: PWR_STAGE
    parameters:
      motor_component: M1
      current_source: phase_peak
      board_temperature_C: 115.0
      min_soa_current_margin_ratio: 2.0
```

Required evidence:

- `target.component` must bind to a component model with `motor_bridge`.
- For system SOA, the bridge model must declare
  `motor_bridge.system_soa.output_current_temperature_curves` with positive,
  strictly increasing temperature points, positive current limits, source
  document, source figure, and digitization metadata.
- `current_source` chooses `phase_peak`, `phase_rms`, or `output_current`.
  Current comes from explicit `soa_output_current_A` or from
  `parameters.motor_component` bound to a model with `motor_load`.
- `board_temperature_C` and `min_soa_current_margin_ratio` are explicit
  board/control inputs.

The system-SOA path linearly interpolates allowable output current versus
temperature and requires
`output_current_limit_A / output_current_A >= min_soa_current_margin_ratio`.
It also fails if the requested board temperature is beyond the curve range.

If a bridge model does not declare system SOA, the validator falls back to
classic `datasheet.safe_operating_area.vds_id_curves`: it selects the shortest
curve whose pulse width covers `pulse_width_us`, uses log-log VDS/ID
interpolation, and checks `bus_voltage_max_V`, `pulse_duty_cycle`, and current
margin. Missing or invalid SOA metadata remains a critical fail-closed finding,
not a warning. Both paths are static screens; final sign-off needs selected
motor evidence, measured switch-node/current waveforms, transient thermal
analysis, and board temperature validation.

`MOTOR_REGEN_CLAMP_VALID` checks a declared single-event regeneration absorber
budget:

```yaml
scenarios:
  - name: wheel_regen_clamp_budget
    type: motor_drive
    checks:
      - MOTOR_REGEN_CLAMP_VALID
    target:
      component: PWR_STAGE
    parameters:
      motor_component: M1
      clamp_component: REGEN1
      regen_energy_J: 1.0
      bus_capacitance_F: 0.001
      bus_voltage_nominal_V: 12.6
      clamp_voltage_V: 16.0
      max_bus_voltage_V: 18.0
      min_clamp_current_margin_ratio: 1.5
      min_regen_energy_margin_ratio: 1.5
```

Required evidence:

- `target.component` names the motor bridge or power stage being protected.
- `clamp_component` names an existing, model-bound regeneration absorber,
  brake, clamp, or upstream energy-sink component.
- Maximum regeneration current comes from explicit `max_regen_current_A`, or
  from `parameters.motor_component` bound to a model with
  `motor_load.max_regen_current_A`.
- `regen_energy_J` is explicit single-event energy evidence. The validator does
  not infer rotor inertia, speed, or firmware braking behavior.
- `bus_capacitance_F`, `bus_voltage_nominal_V`, `clamp_voltage_V`, and
  `max_bus_voltage_V` define the bus energy window.
- Absorber current and energy ratings come from explicit
  `clamp_current_rating_A` / `clamp_energy_rating_J` parameters, or from the
  named `clamp_component` model's `regen_absorber.clamp_current_rating_A` /
  `regen_absorber.clamp_energy_rating_J` metadata.
- `min_clamp_current_margin_ratio` and `min_regen_energy_margin_ratio` define
  the required margins.

The validator requires `clamp_voltage_V > bus_voltage_nominal_V`, checks
`clamp_voltage_V <= max_bus_voltage_V`, checks regeneration current with the
declared current margin, and computes bus-capacitor absorption as
`0.5 * bus_capacitance_F * (clamp_voltage_V^2 - bus_voltage_nominal_V^2)`.
That bus absorption plus explicit or model-derived `clamp_energy_rating_J` must cover
`regen_energy_J * min_regen_energy_margin_ratio`. This is a static single-event
screen; it does not prove repeated-pulse heating, brake-resistor temperature
rise, active clamp stability, firmware regeneration control, or motor SOA.

`MOTOR_ROUTE_CURRENT_VALID` checks imported or explicit motor-drive route
width evidence against an explicit current-density policy:

```yaml
scenarios:
  - name: wheel_phase_route_current
    type: motor_drive
    checks:
      - MOTOR_ROUTE_CURRENT_VALID
    target:
      component: PWR_STAGE
    parameters:
      motor_component: M1
      current_source: phase_rms
      route_nets: [phase_u, phase_v, phase_w]
      max_current_density_A_per_mm: 5.0
```

Required parameters:

- `route_nets`: non-empty list of routed motor or power nets.
- `max_current_density_A_per_mm`: explicit board/layout policy. It should be
  tied to copper weight, temperature-rise, board stackup, and product margin
  evidence.
- Either `route_current_A`, or `motor_component` plus `current_source`.

When `route_current_A` is omitted, `current_source` must be one of:

- `phase_rms`
- `phase_peak`
- `max_regen`

The validator uses the selected current evidence to compute
`required_route_width_mm = current / max_current_density_A_per_mm`, then checks
the minimum imported `board.layout.routes.<net>.segments[].width_mm` for every
named route. This is a deterministic route-width guard; it does not model
MOSFET SOA, switching loss, copper temperature rise, thermal vias, pour sharing,
or transient regeneration behavior.

`MOTOR_CURRENT_SENSE_PLACEMENT_VALID` checks imported or explicit phase-shunt,
phase-route, and current-sense route placement evidence:

```yaml
scenarios:
  - name: wheel_current_sense_placement
    type: motor_drive
    checks:
      - MOTOR_CURRENT_SENSE_PLACEMENT_VALID
    target:
      component: PWR_STAGE
    parameters:
      reference_component: PWR_STAGE
      shunt_components: [RSHUNT_U, RSHUNT_V, RSHUNT_W]
      phase_route_nets: [phase_u, phase_v, phase_w]
      sense_route_nets: [cur_u, cur_v, cur_w]
      max_shunt_to_reference_distance_mm: 6.0
      max_shunt_to_phase_route_distance_mm: 0.5
      max_shunt_to_sense_route_distance_mm: 0.2
      max_sense_route_length_mm: 24.0
```

Required parameters:

- `reference_component`: bridge, gate-driver, or current-sense reference
  component used as the local placement origin.
- `shunt_components`: non-empty list of phase shunt component references.
- `phase_route_nets`: same-length list of routed phase nets beside each shunt.
- `sense_route_nets`: same-length list of routed shunt sense nets.
- `max_shunt_to_reference_distance_mm`
- `max_shunt_to_phase_route_distance_mm`
- `max_shunt_to_sense_route_distance_mm`
- `max_sense_route_length_mm`

The validator requires matching component placements and route polylines in
`board.layout`. It measures Euclidean shunt-to-reference distance, shortest
shunt-to-route distance for the paired phase and sense nets, and total
current-sense route length. This is a layout evidence guard for keeping shunts
and Kelvin/sense traces compact; it does not prove current-sense gain,
offset/noise, ADC range, copper temperature rise, or PWM common-mode rejection.

`MOTOR_CURRENT_SENSE_ACCURACY_VALID` checks a declared shunt/gain/ADC accuracy
budget:

```yaml
scenarios:
  - name: wheel_current_sense_accuracy
    type: motor_drive
    checks:
      - MOTOR_CURRENT_SENSE_ACCURACY_VALID
    target:
      component: PWR_STAGE
    parameters:
      motor_component: M1
      phase_shunt_resistance_ohm: 0.005
      shunt_tolerance_ratio: 0.01
      sense_gain_V_per_V: 20.0
      gain_error_ratio: 0.005
      input_offset_voltage_V: 0.0001
      adc_reference_voltage_V: 3.3
      adc_input_max_voltage_V: 3.0
      adc_resolution_bits: 12
      min_current_measurement_A: 0.5
      min_adc_counts_at_min_current: 20.0
      max_total_current_error_A: 0.25
```

Required evidence:

- `target.component` names the motor bridge or current-sense owner.
- Motor peak/RMS current comes from explicit scenario parameters, or from
  `parameters.motor_component` bound to a model with `motor_load`.
- `phase_shunt_resistance_ohm`, `shunt_tolerance_ratio`,
  `sense_gain_V_per_V`, `gain_error_ratio`, `input_offset_voltage_V`,
  `adc_reference_voltage_V`, `adc_input_max_voltage_V`, and
  `adc_resolution_bits` define the measurement chain.
- `min_current_measurement_A` and `min_adc_counts_at_min_current` define the
  minimum current resolution policy.
- `max_total_current_error_A` defines the allowed worst-case static current
  error.

The validator checks peak current output voltage against the usable ADC input
range, checks ADC counts at the declared minimum measurable current, and sums
worst-case quantization, input-offset, shunt-tolerance, and gain errors at the
declared RMS current. This is a static budget screen; it does not prove PWM
sample timing, common-mode rejection, amplifier bandwidth/slew, ADC aperture
jitter, calibration implementation, thermal drift, or firmware filtering.

## Load Budget Scenario Shape

`load_budget` scenarios validate explicit load-to-connector current,
load-to-switch current/thermal/inrush/reverse-current budget, cable current,
cable temperature-rise, cable voltage-drop/power-loss, and optional voltage
budgets. They are
deterministic schematic-budget checks for payload connectors and harness
interfaces, not crimp-process, bundle-derating, flex-life, or transient-load
sign-off.

```yaml
scenarios:
  - name: servo0_connector_current_budget
    type: load_budget
    checks:
      - LOAD_CONNECTOR_CURRENT_VALID
    target:
      component: SV0
      power_pin: VCC
    parameters:
      connector_component: JSV0
      min_connector_current_margin_ratio: 1.5
```

`LOAD_CONNECTOR_CURRENT_VALID` checks:

1. `target.component` names an existing load component.
2. `target.power_pin` names an `electrical_power` port on the load component
   model.
3. The target power port declares finite positive `max_supply_current_A`.
4. Connector current rating comes from explicit
   `parameters.connector_current_rating_A`, or from
   `parameters.connector_component` bound to a component model with
   `connector.current_rating_A`.
5. `min_connector_current_margin_ratio` is optional and defaults to `1.0`;
   when declared it must be finite and at least `1.0`.
6. The load current multiplied by the margin must fit the connector current
   rating.
7. If the load rail nominal voltage and connector voltage rating are both
   available, the rail voltage must fit the connector voltage rating.

Missing or non-finite required values produce critical
`VALIDATION_INPUT_MISSING` findings. This rule catches first-order mistakes
such as using a signal connector for a servo or actuator current path. It does
not prove contact temperature rise, crimp quality, wire gauge, vibration
retention, duty-cycle heating, surge current, or regeneration behavior.

`POWER_SWITCH_BUDGET_VALID` checks:

1. `target.component` and `target.power_pin` name the switched load and
   electrical power pin behind the selected switch.
2. `parameters.switch_component` names an existing component bound to a model
   with `power_switch` metadata.
3. The switch output pin feeds the same net as the targeted load power pin.
4. The switch input/output power-pin voltage ratings cover the connected rail
   nominal voltages.
5. `power_switch.max_output_current_A` covers the load current multiplied by
   `min_switch_current_margin_ratio`.
6. `power_switch.current_limit_A` covers the load current multiplied by
   `min_current_limit_margin_ratio`.
7. `power_switch.on_resistance_ohm`,
   `power_switch.thermal_resistance_junction_to_ambient_C_per_W`,
   `power_switch.max_junction_temperature_C`, and
   `parameters.ambient_temperature_C` define a static conduction thermal
   estimate at `thermal_current_margin_ratio`.
8. `max_junction_temperature_margin_C` is optional and defaults to `0 C`.

Missing selected-switch ratings produce critical `VALIDATION_INPUT_MISSING`
findings. This rule catches using a placeholder e-stop policy box where a real
eFuse, load switch, or MOSFET path is required. It does not model turn-on ramp,
inrush energy, short-circuit waveform, reverse current, repeated surge, or PCB
copper temperature.

`POWER_SWITCH_REVERSE_CURRENT_VALID` checks that a selected switch model
declares a reverse-current blocking mode that satisfies the scenario
requirement. `reverse_current_blocking_mode_required` can be `always`,
`when_disabled`, or `none`; if it is omitted, legacy
`reverse_current_blocking_required: true` means `always`. A model with
`power_switch.reverse_current_blocking_mode: always` satisfies both `always`
and `when_disabled`; `when_disabled` only satisfies off-state isolation
requirements. Legacy `power_switch.reverse_current_blocking: true` is treated
as `always`, and `false` is treated as `none`. Missing selected-switch
reverse-current data produces a critical `VALIDATION_INPUT_MISSING` finding.
This is a static capability gate for e-stop rails and does not prove the
reverse-current transient waveform or upstream energy absorption.

`POWER_SWITCH_INRUSH_VALID` estimates first-order capacitive turn-on current as
`switched_capacitance_F * rail_voltage / soft_start_time`. It requires
`power_switch.max_inrush_current_A`, `power_switch.soft_start_time_us`, and
`parameters.switched_capacitance_F`; `min_inrush_current_margin_ratio` is
optional and defaults to `1.0`. This is a deterministic soft-start budget gate,
not a substitute for measured turn-on waveform, upstream droop, eFuse fault
response, or thermal pulse validation.

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
      sweeps:
        - name: supply_and_pullup
          parameters:
            - name: VCC_VALUE
              values: [3.0, 3.3, 3.6]
            - name: R_PULLUP
              values: [9000.0, 10000.0, 11000.0]
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

The optional `analog.sweeps` list runs the same deck and assertions across a
bounded Cartesian product of explicit SPICE parameter values, generated
component value inputs, vendor model-library sections, and/or deterministic
Monte Carlo component tolerances. Parameter names must be valid SPICE `.param`
identifiers. Generated component value inputs use `component_values[]` entries
with a Board IR component id, one of `value_ohm`, `value_f`, `value_h`, `dc_v`,
or `dc_a`, and a bounded value list. When the scenario uses
`generated_from_board`, CircuitCI emits a nominal `.param` for the selected
component field and uses that parameter in the generated primitive line, so load
resistance, capacitance, inductance, supply voltage, or current source corners
can be edited without hand-authoring SPICE parameter names. Model sections use
`model_sections[].path` plus a non-empty `sections` list; each selected corner
emits an ngspice `.lib "path" section` line.

Monte Carlo sweeps use a `monte_carlo` block with `samples`, optional `seed`,
and `component_values[]` entries containing `component`, `field`, `nominal`,
`tolerance_percent`, and optional `distribution`. Supported distributions are
`uniform` and `normal`; `uniform` samples linearly within
`nominal * (1 +/- tolerance_percent/100)`, while `normal` treats the same
tolerance as +/-3 sigma and clamps sampled z-scores to that range so generated
passive values remain bounded. A Monte Carlo block may also declare `criteria`
with optional `min_yield_percent`,
`min_p1_margin`, `min_p5_margin`, `min_p50_margin`, and `min_p95_margin`
limits. The sampler is deterministic: the same seed, sample count, and target
list produce the same sampled component values, corner names, artifacts, and
worst-corner summaries on every run. Monte Carlo samples are still ordinary
sweep corners, so they work with transient, AC/Bode, DC operating-point, and
noise observations. For every assertion evaluated across a Monte Carlo sweep,
the report also emits an `ANALOG_MONTE_CARLO_YIELD_SUMMARY` finding with
evaluated sample count, pass/fail counts, yield percent, mean margin, margin
standard deviation, and the limiting sampled corner. The same finding includes
linearly interpolated sampled-margin percentiles (`p1_margin`, `p5_margin`,
`p50_margin`, and `p95_margin`) so distribution tails and median margin are
visible without opening every sampled artifact. Summaries are informational when
no criteria are declared, remain informational when all declared criteria pass,
and become critical when any declared yield or percentile margin limit fails.
When criteria are declared, individual sampled assertion failures are retained
as tagged evidence findings instead of directly failing the run; backend,
netlist, solver, and non-assertion limit failures remain critical.

Sweep execution is capped to keep GUI and CI runs predictable. Each corner
writes separate waveform/artifact outputs and tags assertion findings with the
sweep name, corner name, parameter values, component value inputs, and selected
model sections. For every assertion that is evaluated across a sweep, the report
also emits an `ANALOG_SWEEP_MARGIN_SUMMARY` info finding that points at the
worst margin corner, measured value, limit, relation, parameters, component
values, and model sections.
GUI Run Inputs expose presets that persist as this same `analog.sweeps`
structure: supply (`SUPPLY_V`), load (`LOAD_OHM`), temperature (`TEMP_C`),
model-selector (`MODEL_CORNER`), and RC tolerance (`RIN_VALUE` x
`COUT_VALUE`). Temperature presets are special-cased by the ngspice wrapper:
`TEMP_C` or `TEMPERATURE_C` remains available as a `.param` and also becomes
the corner's `.temp` card.

## Analog AC Scenario Shape

`analog_ac` scenarios use the same Board IR binding and model-file contract as
`analog_transient`, but run a small-signal AC sweep, export Bode observation
artifacts, and evaluate frequency-domain assertions:

```yaml
scenarios:
  - name: rc_filter_bode
    type: analog_ac
    checks:
      - SPICE_AC_ANALYSIS
    analog:
      backend: auto
      netlist_source: file
      netlist: deck_ac.cir
      model_files: []
      node_bindings:
        - { node: "0", net: gnd }
        - { node: input, net: input }
        - { node: filtered, net: filtered }
      pin_bindings:
        - { node: input, endpoint: { component: RIN, pin: A } }
        - { node: filtered, endpoint: { component: RIN, pin: B } }
      analysis:
        type: ac
        start_frequency_hz: 10.0
        stop_frequency_hz: 100000.0
        points_per_decade: 20
      stimuli:
        - name: small_signal_input
          description: 1 V AC source defined in deck_ac.cir.
      sweeps: []
      probes:
        - name: input
          expression: V(input)
        - name: filtered
          expression: V(filtered)
      assertions:
        - name: filtered_gain_at_1khz_below_minus_1db
          probe: filtered
          aggregation: gain_db_at_frequency
          relation: below
          at_hz: 1000.0
          threshold_db: -1.0
        - name: filtered_phase_at_1khz_below_minus_20deg
          probe: filtered
          aggregation: phase_deg_at_frequency
          relation: below
          at_hz: 1000.0
          threshold_deg: -20.0
        - name: filtered_cutoff_above_1_4khz
          probe: filtered
          aggregation: falling_gain_crossing_frequency
          relation: above
          threshold_db: -3.0
          frequency_limit_hz: 1400.0
```

File-backed decks must contain an AC-capable source such as `VIN in 0 AC 1`;
transient `SIN(...)` sources alone do not define small-signal AC magnitude for
ngspice. Generated-from-board `analog_ac` scenarios add a unity small-signal
`AC 1` suffix to independent voltage/current source primitives while retaining
their declared DC or pulse operating point.
Each run writes `bode.csv` with `frequency_hz`, `{probe}_mag_db`,
`{probe}_phase_deg`, and `{probe}_mag` columns. `analog.sweeps` work the same
way as transient sweeps and create one Bode artifact per corner. Supported AC
assertions are:

- `gain_db_at_frequency`: requires `at_hz` and `threshold_db`; compares the
  selected probe's `{probe}_mag_db` at the interpolated frequency.
- `phase_deg_at_frequency`: requires `at_hz` and `threshold_deg`; compares
  `{probe}_phase_deg` at the interpolated frequency.
- `rising_gain_crossing_frequency` and `falling_gain_crossing_frequency`:
  require `threshold_db` and `frequency_limit_hz`; compare the first
  interpolated gain crossing frequency against the frequency limit.
- `phase_margin_deg`: requires `threshold_deg`; finds the first falling
  0 dB gain crossing, interpolates `{probe}_phase_deg` at that frequency, and
  compares `180 + phase_deg` as the phase margin.
- `gain_margin_db`: requires `threshold_db`; finds the first falling
  -180 degree phase crossing, interpolates `{probe}_mag_db` at that frequency,
  and compares `-gain_db` as the gain margin.

## Analog DC Operating-Point Scenario Shape

`analog_dc` scenarios use the same Board IR binding, model-file, generated
netlist, and run-input sweep contract as transient and AC/Bode scenarios, but
run an ngspice `.op` analysis and export one `operating_point.csv` row per
corner:

```yaml
scenarios:
  - name: divider_dc_bias
    type: analog_dc
    checks:
      - SPICE_DC_ANALYSIS
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1, R2]
      model_files: []
      node_bindings:
        - { node: "0", net: gnd }
        - { node: vin, net: vin }
        - { node: midpoint, net: midpoint }
      pin_bindings:
        - { node: vin, endpoint: { component: V1, pin: P } }
        - { node: "0", endpoint: { component: V1, pin: N } }
      analysis:
        type: op
      stimuli:
        - name: dc_divider_bias
          description: V1 biases a resistor divider from 5 V.
      sweeps:
        - name: divider_tolerance
          component_values:
            - component: R1
              field: value_ohm
              values: [9500.0, 10000.0, 10500.0]
      probes:
        - name: midpoint
          expression: V(midpoint)
      assertions:
        - name: midpoint_above_2_35v
          probe: midpoint
          aggregation: operating_point
          relation: above
          threshold_v: 2.35
```

`SPICE_DC_ANALYSIS` currently uses external ngspice export. It emits
`operating_point_raw.csv`, normalized `operating_point.csv`, wrapper deck, and
solver log artifacts. `operating_point` assertions compare the checked probe's
DC value against the probe-unit threshold (`threshold_v`, `threshold_a`, or
`threshold_w`) without time or frequency fields. DC scenarios participate in
the same `analog.sweeps` expansion and `ANALOG_SWEEP_MARGIN_SUMMARY` worst
corner reports as transient and AC/Bode scenarios. The GUI run-setup editor
can create generated-from-board `analog_dc` observations directly, and its
check editor can add either individual `operating_point` checks or preset
3.3 V rail, 5 V rail, and 2.5 V midpoint bias windows.

Noise observations use the same model-file, node-binding, pin-binding, and
run-input sweep contract, but run ngspice `.noise` and export both spectral
density and integrated RMS totals:

```yaml
scenarios:
  - name: divider_output_noise
    type: analog_noise
    checks:
      - SPICE_NOISE_ANALYSIS
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1, R2]
      model_files: []
      node_bindings:
        - { node: "0", net: gnd }
        - { node: vin, net: vin }
        - { node: midpoint, net: midpoint }
      pin_bindings:
        - { node: vin, endpoint: { component: V1, pin: P } }
        - { node: "0", endpoint: { component: V1, pin: N } }
      analysis:
        type: noise
        start_frequency_hz: 10.0
        stop_frequency_hz: 100000.0
        points_per_decade: 20
        noise_output_node: midpoint
        noise_input_source: V1
      probes:
        - name: onoise
          expression: V(midpoint)
        - name: inoise
          expression: V(vin)
      assertions:
        - name: output_density_1khz_below_10nv
          probe: onoise
          aggregation: output_noise_density_at_frequency
          relation: below
          at_hz: 1000.0
          threshold_v_per_sqrt_hz: 1.0e-8
        - name: output_rms_noise_below_3_5uv
          probe: onoise
          aggregation: integrated_output_noise
          relation: below
          threshold_v: 3.5e-6
```

`SPICE_NOISE_ANALYSIS` currently uses external ngspice export. It emits
`noise_spectrum_raw.csv`, normalized `noise_spectrum.csv`, `noise_total_raw.csv`,
normalized `noise_total.csv`, wrapper deck, and solver log artifacts. Density
assertions use `output_noise_density_at_frequency` or
`input_noise_density_at_frequency` with `at_hz` and
`threshold_v_per_sqrt_hz`. Integrated RMS checks use
`integrated_output_noise` or `integrated_input_noise` with `threshold_v`. The
GUI run-setup editor can create generated-from-board noise observations by
choosing the output net, input source, and frequency band. Its check presets can
append output-density/output-RMS and input-referred-density/input-RMS assertions
as ordinary `analog.assertions` rows. Scopes plots the normalized density
artifact while showing integrated totals in a compact table.

Analog waveform assertions can also use window aggregations for executable
design measurements. `min`, `max`, `mean`, `rms`, `integral`, and `energy`
require `start_us` and `end_us`. `integral` performs signed trapezoidal
integration over the interpolated waveform window and compares voltage probes
with `threshold_vs`, current probes with `threshold_c`, and power probes with
`threshold_j`. `energy` is power-probe-only and also compares with
`threshold_j`.

Timing and transient-quality assertions use interpolated waveform events.
`rising_crossing_time`, `falling_crossing_time`, `min_high_pulse_width`, and
`min_low_pulse_width` require `start_us`, `end_us`, `time_limit_us`, and the
probe-unit decision threshold (`threshold_v`, `threshold_a`, or `threshold_w`).
`duty_cycle` uses `duty_limit_percent`; crossing-count checks use
`count_limit`. `settling_time` uses a target band instead of a threshold:
provide `target_v`/`target_a`/`target_w`, matching
`tolerance_v`/`tolerance_a`/`tolerance_w`, and `time_limit_us`. It measures the
last time the waveform leaves or crosses the target band within the window.
`overshoot_percent` uses `target_v`/`target_a`/`target_w` plus
`overshoot_limit_percent` and reports the peak positive excursion above the
target as a percent of the target magnitude. `rising_phase_delay` and
`falling_phase_delay` compare two probes: provide `reference_probe`, one
`reference_threshold_v`/`reference_threshold_a`/`reference_threshold_w`, the
checked probe threshold, and `time_limit_us`. The measured value is the delay
from the reference probe's first matching threshold crossing to the checked
probe's first matching threshold crossing after it.

Setup/hold timing checks also compare a checked probe against a reference
probe. `rising_setup_time`, `rising_hold_time`, `falling_setup_time`, and
`falling_hold_time` use the reference threshold to find matching reference
edges and the checked probe threshold to find any checked-signal transition.
Setup time is the minimum time from the previous checked transition to a
reference edge; hold time is the minimum time from a reference edge to the next
checked transition. Use `relation: above` with `time_limit_us` for a minimum
setup or hold margin.
