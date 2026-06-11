# Scenario Language

Scenarios describe validation conditions applied to a bound board. The MVP supports static scenario metadata and selected checks. Later versions add time-ordered events and solver-backed waveforms.

## MVP Scenario

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

## Future Event Form

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

## Required MVP Scenario Types

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

Executable first-stage scenario types:

- `gpio_backdrive`
- `reset_boot`
- `serial_programming`
- `firmware_update`
- `control_line_sequence`
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

Canonical executable check IDs in the first stage:

- `GPIO_BACKDRIVE`
- `RESET_RELEASE_AFTER_POWER_VALID`
- `BOOT_STRAP_DEFINED`
- `UART_BOOTLOADER_SYNC`
- `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`
- `CONTROL_LINE_RELEASE_SEQUENCE`
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
- `reset_release_at_us`: first time reset is deasserted.
- `boot_sample_at_us`: time boot straps are sampled.

`target.component` is required for `reset_boot`. `target.power_pin` and `target.reset_pin` are optional scenario assertions; if present, they must match the component model behavior and board pin map.

`RESET_RELEASE_AFTER_POWER_VALID` fails when reset releases before power is valid. Missing target/timing data for this declared check is a critical `VALIDATION_INPUT_MISSING` finding.

`BOOT_STRAP_DEFINED` resolves required strap states from `component.behavior.boot.modes[required_boot_mode]`. It fails when any required strap is missing from scenario observations, observed as `floating` or `undefined`, or not equal to the model-required state. The scenario may not invent the required strap state.

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
```

This scenario type is the physical analog path. If no SPICE-class backend is
available, or if the runtime cannot execute the deck and evaluate waveforms, the
scenario must fail with a critical analog finding.
