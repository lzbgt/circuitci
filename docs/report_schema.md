# Report Schema

CircuitCI reports are built for both AI agents and engineers.

## JSON Report

```json
{
  "schema_version": "0.1.0",
  "project": "bad_backdrive_board",
  "profile": "iot_basic_v0",
  "result": "fail",
  "summary": {
    "critical": 1,
    "warning": 0,
    "info": 1
  },
  "failures": [],
  "warnings": [],
  "infos": [],
  "waveforms": [],
  "artifacts": [],
  "limitations": [],
  "suggested_next_actions": [],
  "reproduction": {
    "command": "circuitci validate examples/bad_backdrive_board/project.yaml --output out/"
  }
}
```

## Finding Object

```json
{
  "id": "GPIO_BACKDRIVE",
  "severity": "critical",
  "scenario": "usb_hot_plug_mcu_unpowered",
  "message": "Powered component U2.TXD drives unpowered component U1.RX on net uart_rx.",
  "component": "U1",
  "net": "uart_rx",
  "endpoints": {
    "driver": { "component": "U2", "pin": "TXD" },
    "victim": { "component": "U1", "pin": "RX" }
  },
  "measured": {
    "injection_current_A": 0.0012
  },
  "limit": {
    "injection_current_A": 0.0001
  },
  "suggested_fixes": [
    "Add a series resistor sized to keep injection current below the receiving pin limit.",
    "Add a bus switch or isolation device.",
    "Ensure both components are in the same powered domain before driving the net."
  ]
}
```

## Result Semantics

- `fail`: at least one critical finding.
- `pass`: no critical finding.

The MVP result is exactly:

```text
fail iff summary.critical > 0, otherwise pass
```

Warnings and limitations remain visible in the report but do not change `result` in schema version `0.1.0`.

## Limitation Object

```json
{
  "id": "UNSUPPORTED_SCENARIO",
  "scope": "scenario:reset_boot",
  "confidence": "low",
  "blocking": true,
  "message": "Scenario type reset_boot is documented but not implemented in this runtime."
}
```

Unsupported `iot_basic_v0` checks are blocking for fabrication readiness even when the executable subset has no critical finding.

Projects using `generic`, `estimated`, or `low` confidence component models must include non-blocking `LOW_CONFIDENCE_MODEL` limitations scoped to the component and model.

## Additional Rule Findings

Reset/boot/download rules use the same finding object. Required IDs:

- `RESET_RELEASE_AFTER_POWER_VALID`
- `BOOT_STRAP_DEFINED`
- `UART_BOOTLOADER_SYNC`
- `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`
- `CONTROL_LINE_RELEASE_SEQUENCE`
- `SPICE_OPERATING_LIMIT`

Reports must include `scenario`, `component` when applicable, measured timing values in `measured`, limits or expected states in `limit`, and concrete suggested fixes.

Stable rule detail keys:

- `RESET_RELEASE_AFTER_POWER_VALID.measured`: `power_valid_at_us`, `reset_release_at_us`, `margin_us`.
- `RESET_RELEASE_AFTER_POWER_VALID.limit`: `reset_release_not_before_power_valid: true`.
- `BOOT_STRAP_DEFINED.measured`: `required_boot_mode`, `observed_<pin>`.
- `BOOT_STRAP_DEFINED.limit`: `required_<pin>`.
- `UART_BOOTLOADER_SYNC.measured`: `interface`, `sync_event_found`, `event_at_us`.
- `UART_BOOTLOADER_SYNC.limit`: `sync_byte`, `expected_response`, `rx_pin`, `required_boot_mode`.

`RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` reports must include a non-blocking `ABSTRACT_PROTOCOL_TRACE` limitation because the rule validates declared transaction traces rather than raw firmware execution, raw-frame CRC recomputation, flash emulation, or HIL behavior.

`CONTROL_LINE_RELEASE_SEQUENCE` reports must include a non-blocking `ABSTRACT_CONTROL_LINE_MODEL` limitation because the rule validates declared line effects and release delays rather than transistor-level or RC waveform behavior.

`SPICE_OPERATING_LIMIT` reports are emitted by physical analog validation when
generated Board IR MOSFET/BJT operating probes exceed datasheet absolute
maximum ratings. Stable measured keys include `component`, `rating`,
`quantity`, `expression`, `max_abs`, `time_of_max_us`, and `unit`; stable limit
keys include `rating`, `rating_value`, `max_abs`, and `unit`. `rating_value`
preserves the signed datasheet value while `max_abs` is the absolute comparison
limit. If a generated semiconductor model lacks the required absolute-maximum
metadata, the same rule id is emitted with measured `component`, `model`,
`quantity`, `missing_rating`, and `unit` keys.

Declared executable checks with missing required inputs must produce a critical `VALIDATION_INPUT_MISSING` finding so the report cannot pass by skipping validation.

## Markdown Report

Markdown reports must include:

1. Executive summary.
2. Pass/fail table.
3. Critical failures.
4. Warnings.
5. Suggested fixes.
6. Unmodeled or low-confidence areas.
7. Reproduction command.
