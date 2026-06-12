# Component Model Contract

Each component model is a YAML document with a stable `component_id`, version, pin declarations, optional model faces, rules, and quality metadata.

## Minimal Model

```yaml
component_id: generic.mcu.basic
version: 0.1.0
category: mcu

ports:
  VDD:
    kind: electrical_power
    required: true
    electrical:
      operating_voltage_min_V: 2.7
      operating_voltage_max_V: 3.6
      max_supply_current_A: 0.03
  GND:
    kind: electrical_ground
    required: true
  RX:
    kind: digital_electrical_input
    required: false
    electrical:
      vih_min_V: 2.0
      vil_max_V: 0.8
      injection_current_limit_A: 0.0001

model_faces:
  electrical_pins:
    status: simple_behavioral

rules:
  - GPIO_BACKDRIVE

model_quality:
  source: generic
  confidence: low
  intended_use:
    - power_sequence
    - leakage
  not_valid_for:
    - rf
    - high_speed_signal_integrity
    - transistor_level_mcu_behavior
```

## Port Kinds

| Kind | Meaning |
| --- | --- |
| `electrical_power` | Supply input or source rail. |
| `electrical_ground` | Ground reference. |
| `digital_electrical_input` | Digital input with electrical limits. |
| `digital_electrical_output` | Driven output with voltage/current metadata. |
| `digital_electrical_io` | Bidirectional GPIO. |
| `passive` | Passive two-terminal behavior. |

## Electrical Metadata

Inputs should declare:

- `vih_min_V`
- `vil_max_V`
- `injection_current_limit_A`

Outputs should declare:

- `drive_high_voltage_V`
- `source_impedance_ohm`
- optional `powered_behavior`

Power ports should declare when known:

- `operating_voltage_min_V`
- `operating_voltage_max_V`
- `max_supply_current_A`

`POWER_TREE_VALID` uses these values to check that a component is connected to
a powered rail inside its allowed operating range and that declared rail current
budgets are not exceeded. Generic models may use conservative screening values;
datasheet-backed packs should cite their source documents.

The first back-drive approximation computes injection current as:

```text
max(0, driver_high_voltage - victim_power_voltage - diode_drop) / source_resistance
```

The behavioral `GPIO_BACKDRIVE` rule defaults diode drop to `0.3 V` and
combines the output source impedance with any scenario-declared series
resistance. Physical voltage/current proof belongs in `analog_transient`
scenarios, where generated or file-backed SPICE decks provide waveform evidence
and generated semiconductor models can be checked against datasheet operating
limits.

## GPIO_BACKDRIVE Rule

Normative first-slice behavior:

- Rule ID: `GPIO_BACKDRIVE`.
- Severity: `critical` when measured current is greater than the victim limit.
- Comparison: fail iff `injection_current_A > injection_current_limit_A`.
- Default diode drop: `0.3 V`, overridable by scenario `parameters.diode_drop_V`.
- Missing output `source_impedance_ohm`: binding warning and skip that path.
- Missing output `drive_high_voltage_V`: binding warning and skip that path.
- Missing input `injection_current_limit_A`: binding warning and skip that path.
- `digital_electrical_io` direction comes from scenario `pin_states`.
- Victim rail voltage follows Board IR power semantics.

Formula:

```text
effective_resistance = driver.source_impedance_ohm + path.series_resistance_ohm
injection_current_A =
  max(0, driver.drive_high_voltage_V - victim_rail_voltage_V - diode_drop_V)
  / effective_resistance
```

`effective_resistance <= 0` is invalid model/scenario data and must produce a warning finding instead of division by zero.

## Reset/Boot Model Metadata

MCU-like models can declare reset and boot behavior without making the engine chip-specific:

```yaml
behavior:
  reset:
    pin: NRST
    active: low
    min_assert_us: 20
  boot:
    sample_time_after_reset_release_us: 100
    modes:
      bootloader:
        straps:
          - pin: BOOT0
            required_state: high
      application:
        straps:
          - pin: BOOT0
            required_state: low
  bootloader:
    interfaces:
      uart:
        rx_pin: RX
        tx_pin: TX
        sync_byte: 0x7F
        ack_byte: 0x79
```

This metadata can represent STM32-like boot flows, ESP32-like EN/IO0 flows, STM8/C51/STC serial entry flows, or simpler generic boot selectors. Vendor packs provide concrete values; the validation engine reads only the generic contract.

MCU models should remain functional black boxes. A stronger MCU model may run
firmware and expose correct peripheral/pin behavior, reset causes, boot-ROM
entry, pin modes, thresholds, clamp/leakage limits, and timing at the board
boundary. It should not attempt transistor-level modeling of the MCU core or
internal silicon implementation.

## Resident Protocol Metadata

Firmware-specific models can declare resident update protocols without changing the engine:

```yaml
behavior:
  protocols:
    resident_update:
      transport_interface: uart
      frame:
        magic: [85, 77, 66, 76]
        version: 1
        request_type: 1
        response_type: 2
        crc: crc32_ieee
        max_payload_len: 1030
        ok_result: 0
      operations:
        begin:
          opcode: 2
          role: start_transfer
          payload:
            min_len: 36
            max_len: 37
        data:
          opcode: 3
          role: data_chunk
          payload:
            overhead_len: 6
        finish:
          opcode: 4
          role: finish_transfer
          payload:
            len: 36
      flows:
        upload:
          phases:
            - operation: begin
            - operation: data
              repeat: one_or_more
            - operation: finish
```

Operation names are model-local. Generic validation keys off operation metadata such as `role`, payload limits, and flow phases, not chip or protocol names.

## Reset/Boot Rules

`RESET_RELEASE_AFTER_POWER_VALID`:

- Severity: `critical`.
- Fail iff `reset_release_at_us < power_valid_at_us`.
- Missing target or timing fields produce critical `VALIDATION_INPUT_MISSING` findings.

`BOOT_STRAP_DEFINED`:

- Severity: `critical`.
- Resolve required straps from `behavior.boot.modes[scenario.required_boot_mode]`.
- Fail iff a required strap observation is missing, `floating`, `undefined`, or does not equal the model-required state.
- Required and actual values are compared as lowercase symbolic states.

`UART_BOOTLOADER_SYNC`:

- Severity: `critical`.
- Fail if the model lacks the requested bootloader interface.
- Fail if scenario-declared sync/ACK bytes conflict with the model.
- Fail if no scenario event sends exactly `[sync_byte]` to the model's bootloader RX pin.
- Fail if the event sender is missing, unresolved, not output-capable, or not connected to the target RX net.
- Fail if the event target is not the target component and model RX pin.
- Fail if event time is before `boot_sample_at_us` when that timing is declared.
- Pass/fail is abstract protocol behavior, not full firmware execution.

`RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`:

- Severity: `critical`.
- Resolve the named protocol from `behavior.protocols`.
- Fail if the protocol sender does not resolve to an output-capable pin on the target RX net.
- Fail if operation order does not match the named flow phases.
- Fail if any event result code differs from `frame.ok_result`.
- Fail if payload lengths exceed `frame.max_payload_len` or operation payload limits.
- Fail if `data_chunk` roles do not cover the declared package size exactly.
- Pass/fail is abstract trace validation, not full firmware execution or flash emulation.

`CONTROL_LINE_RELEASE_SEQUENCE`:

- Severity: `critical`.
- Uses scenario `control_effects`; no component-model protocol extension is required.
- Fail if a control source is unresolved, unconnected, or not output-capable.
- Fail if a control target is unresolved, unconnected, not input-capable, or not on the target component.
- Fail if an evaluated effect has no explicit prior `control_line` event.
- Fail if derived reset state is not released at reset release or boot sample time.
- Fail if derived boot strap states do not match the required boot mode.
- Pass/fail is abstract line-effect timing, not transistor-level or RC waveform simulation.

## Quality Policy

Every model must declare model quality. Reports emit `LOW_CONFIDENCE_MODEL` limitations for `generic`, `estimated`, or `low` confidence models so users do not over-trust behavioral library metadata.
