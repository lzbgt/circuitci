# Resident Bootloader Protocol Design

This slice extends CircuitCI from ROM-entry validation to resident firmware-update protocol validation. It remains generic: concrete protocols such as UM `UMBL`, ESP serial loaders, STC ISP, or custom factory bootloaders are component-library data.

## Product Boundary

The runtime may validate:

- a named protocol declared by a component model,
- frame/header constants and payload-size limits,
- request/response operation order,
- package size, chunk coverage, and digest-field consistency,
- expected state transitions declared by the model,
- the already-modeled UART path between host adapter and target.

The runtime must not:

- execute target firmware,
- emulate flash erase/program at byte level,
- hardcode `UMBL`, STM32, CH340, `.umfw`, or W25Q32,
- claim HIL proof for upload/activate/confirm/recover flows.

## Model Contract

Add optional `behavior.protocols` to component models:

```yaml
behavior:
  protocols:
    umbl_resident_update:
      transport_interface: usart1_rom
      frame:
        magic: [85, 77, 66, 76]
        version: 1
        request_type: 1
        response_type: 2
        crc: crc32_ieee
        max_payload_len: 1030
        ok_result: 0
      operations:
        status: { opcode: 1 }
        begin:
          opcode: 2
          role: start_transfer
          payload:
            kind: begin_size_sha256_overwrite
            min_len: 36
            max_len: 37
        data:
          opcode: 3
          role: data_chunk
          payload:
            kind: offset_len_chunk
            overhead_len: 6
        finish:
          opcode: 4
          role: finish_transfer
          payload:
            kind: finish_size_sha256
            len: 36
        activate:
          opcode: 5
          role: activate
          payload:
            kind: enum_u8
            values:
              next: 0
              now: 1
        confirm:
          opcode: 6
          role: confirm
        read_log: { opcode: 9 }
      flows:
        upload_activate_next_log:
          phases:
            - operation: status
            - operation: begin
            - operation: data
              repeat: one_or_more
            - operation: finish
            - operation: status
            - operation: activate
            - operation: read_log
          final_state: activate_pending
```

The first implementation only needs enough typed Rust to validate the fields above. Unknown protocol metadata should deserialize but unsupported options should produce limitations instead of pretending to pass.

## Scenario Contract

Add `firmware_update` scenarios with `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`:

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
      package_sha256: "000102..."
      chunk_size_bytes: 1024
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
      - at_us: 14000
        action: protocol_request
        operation: data
        offset: 1024
        chunk_len: 1024
        payload_len: 1030
        result_code: 0
      - at_us: 16000
        action: protocol_request
        operation: finish
        payload_len: 36
        result_code: 0
      - at_us: 18000
        action: protocol_request
        operation: status
        result_code: 0
        state: staged_ready
      - at_us: 20000
        action: protocol_request
        operation: activate
        activate_mode: next
        payload_len: 1
        result_code: 0
      - at_us: 22000
        action: protocol_request
        operation: read_log
        result_code: 0
        state: activate_pending
```

Protocol events are abstract host/device transactions, not raw bytes. The raw byte framing is represented in component-model metadata. Later versions can add raw-frame fixtures and CRC recomputation.

`CONFIRM` is declared in model metadata but intentionally outside the first pass fixture. In the peer protocol, normal confirmation is application-side after health checks; a host-side confirm operation is a service or lab override. This slice validates resident upload and `ACTIVATE next` progression to an activation-pending journal state while the resident command loop remains available, not application health confirmation.

## Validation Algorithm

`RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` should:

1. Resolve `target.component`.
2. Resolve `scenario.protocol.name` from `component.behavior.protocols`.
3. If `transport_interface` is present, reuse the bootloader interface pins and require the scenario protocol sender endpoint to connect to the target RX net.
4. Require every event action to be `protocol_request` and every operation to exist in the model.
5. Require each event result code to equal model `frame.ok_result` for the pass path.
6. Check operation payload lengths against the model operation payload constraints.
7. Match operation order using model flow phases. `repeat: one_or_more` consumes one or more consecutive events for that operation; `repeat: zero_or_more` consumes any consecutive events.
8. For operations with role `data_chunk`, require `payload_len == overhead_len + chunk_len`, `payload_len <= max_payload_len`, non-overlapping offsets, and complete coverage of `package_size_bytes`. Offset order may be arbitrary; the scenario proves coverage, not firmware write scheduling.
9. Require operations with role `start_transfer` and `finish_transfer` to have package metadata in the scenario. This first slice checks lengths and coverage, not hash recomputation.
10. Require operation order to match the selected model flow.
11. Require the final observed state to match `expected_final_state` when declared.

## Initial Fixtures

Add:

- `examples/um_stm32l4_resident_update_activate`: pass, uses the UM board path and `upload_activate_next_log` flow.
- `examples/um_stm32l4_resident_update_missing_finish`: fail with `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`.
- `examples/um_stm32l4_resident_update_oversize_chunk`: fail because a data payload exceeds max payload.
- `examples/um_stm32l4_resident_update_wrong_sender`: fail because the protocol sender endpoint is not the target RX net driver.

## Definition Of Done

- Design and research docs are committed with the implementation.
- Schemas cover protocol model metadata and scenario protocol fields.
- Rust models deserialize protocol metadata without chip-specific branches.
- CLI tests cover pass, missing-finish fail, and oversize-chunk fail.
- `cargo clippy --all-targets -- -D warnings` and `cargo test` pass.
