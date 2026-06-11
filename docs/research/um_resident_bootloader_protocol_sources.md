# UM Resident Bootloader Protocol Source Notes

This note records local evidence from the peer `../urine_monitor` repository for the next CircuitCI acceptance slice: validating a resident bootloader firmware-update protocol without hardcoding STM32 or UM-specific behavior in the engine.

## Peer Source Files

- `../urine_monitor/firmware_stm32l431_bootloader/README.md`
- `../urine_monitor/firmware_stm32l431_bootloader/app/um_stm32_boot_proto.h`
- `../urine_monitor/firmware_stm32l431_bootloader/app/um_stm32_boot_proto.c`
- `../urine_monitor/firmware_stm32l431_bootloader/app/main.c`
- `../urine_monitor/tools/um_stm32_bootloader_client.py`

## Extracted Facts

- The resident bootloader runs from internal flash and speaks over the populated `CH340C -> USART1` path.
- The execute application slot is documented as `0x0800C000..0x0803FFFF`; `.umfw` packages are staged in W25Q32 external flash.
- The protocol magic is ASCII `UMBL`; protocol version is `0x01`.
- Frame types are request `0x01`, response `0x02`, and event `0x03`.
- Supported request opcodes are `STATUS`, `BEGIN`, `DATA`, `FINISH`, `ACTIVATE`, `CONFIRM`, `ABORT`, `RECOVER_LAST_GOOD`, and `READ_LOG`.
- Maximum protocol payload is `1030` bytes.
- The wire frame is:
  - magic `UMBL`,
  - version,
  - frame type,
  - opcode,
  - flags,
  - little-endian `u16` sequence,
  - little-endian `u16` payload length,
  - payload,
  - little-endian IEEE CRC32 over header bytes after magic plus payload.
- A response payload starts with little-endian `u16 result_code`, little-endian `u16 detail_len`, then `detail`.
- Success result code is `0x0000`.
- `BEGIN` payload is `size:u32_le + sha256:32 bytes`, optionally followed by `overwrite_flag:1 byte`; current host sends the overwrite byte.
- `DATA` payload is `offset:u32_le + chunk_len:u16_le + chunk bytes`.
- `FINISH` payload is `size:u32_le + sha256:32 bytes`.
- `ACTIVATE` payload is one byte: `0` for next reset, `1` for now. For `now`, the resident image sends the success response before resetting.
- The host client sends upload chunks in increasing offset order: `BEGIN`, zero or more `DATA` chunks, `FINISH`, `STATUS`; activation flow is `ACTIVATE`, then later reset/status/log/confirm behavior depending on activation mode and app health.
- The host self-test covers `STATUS`, upload, `ACTIVATE now`, `STATUS`, and `READ_LOG`; it expects staged-ready after upload and waiting-confirm after activate-now, but that is PTY mock behavior rather than proof that the real resident command loop remains available after `ACTIVATE now`.
- Boot states named by the host include `RECOVERY_IDLE`, `USB_UPLOAD`, `STAGED_READY`, `ACTIVATE_PENDING`, `WAITING_CONFIRM`, `LAST_GOOD_ONLY`, and `APP_UPLOAD`.
- On reset, `ACTIVATE_PENDING` copies staged package into the execute slot, marks `WAITING_CONFIRM`, then jumps to app when the vector is valid.
- If `WAITING_CONFIRM` persists without confirm, reset handling attempts rollback from last-good; when unavailable it returns to recovery idle with confirm-missing reason.
- Real-board HIL proof for full upload/activate/confirm/recover flows is still pending in the peer README.

## Modeling Limits

- CircuitCI should validate declared protocol metadata and a scenario trace. It should not execute STM32 firmware, emulate flash, or prove `.umfw` relocation correctness in this slice.
- The protocol should be component-library metadata, not engine branches for `UMBL`, STM32, or CH340.
- A pass means the modeled board path and declared host/device protocol trace are internally consistent with the component model. It does not mean the physical fabricated board has completed HIL upload/activation.
