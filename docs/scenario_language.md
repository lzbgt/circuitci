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
when `parameters.require_vbus_protection` is true.

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
12. If a component model declares `power_switch`, the declared input and output
    pins must name distinct `electrical_power` ports, the control pin must be a
    digital input/IO port, and a powered output rail must have matching
    scenario `pin_states` evidence for the required enabled state.
13. If `power_switch.max_output_current_A` is declared, every switched-output
    rail load must declare `max_supply_current_A`, and the summed worst-case
    output load must not exceed the switch limit.
14. If a component model declares `reset_supervisor`, the monitored pin must be
    an `electrical_power` port connected to a power rail, and the reset output
    must be a digital output/IO port connected to a net.
15. The monitored rail nominal voltage must be above the supervisor
    `threshold_max_V`, and `threshold_min_V` must not be below the highest
    powered-load `operating_voltage_min_V` on that rail.
16. If a component model declares `battery_charger`, the declared input and
    battery pins must name distinct `electrical_power` ports and be connected
    to rails. Invalid charger metadata fails closed.
17. If `battery_charger.charge_current_parameter` is declared, the component
    instance must provide that numeric parameter. The programmed current must
    fit `min_charge_current_A` / `max_charge_current_A` when present.
18. If the charger input rail declares `supply_current_limit_A`, the programmed
    charge current must fit that input-source budget.
19. If `battery_charger.regulation_voltage_V` is declared and the battery net
    has `nominal_voltage`, the battery net may not exceed the regulation
    voltage.
20. If a component model declares `power_mux`, the output and all input pins
    must name `electrical_power` ports and be connected to rails.
21. If `power_mux.selected_input_parameter` is declared, the component instance
    must provide that string parameter, and the selected input must match one
    of the model input names.
22. If the mux output rail is powered, the selected input rail must be powered.
23. If the mux output rail is powered and an inactive input rail is unpowered,
    that inactive input must declare `reverse_blocking: true`.
24. If `power_mux.max_output_current_A` is declared, every load on the mux
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
