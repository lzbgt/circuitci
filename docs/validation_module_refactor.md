# Validation Module Refactor

## Purpose

`src/validation/mod.rs` is now close to the project file-size ceiling after the
control-line release rule. The next acceptance features should not grow that
file past 2000 lines. This slice splits validation by rule family without
changing report semantics, scenario semantics, schemas, or example behavior.

## Current Problem

- `src/validation/mod.rs` contains orchestration, limitation emission, all rule
  implementations, report construction helpers, and shared board/model helpers.
- The file is still under the limit, but it is large enough that the next rule
  would likely violate the repo-local 2000-line source-file rule.
- Rule families already have natural boundaries:
  - GPIO backdrive
  - reset and boot straps
  - UART ROM bootloader sync
  - resident bootloader protocol traces
  - abstract control-line release sequences

## Refactor Shape

Keep the public API unchanged:

```rust
pub fn validate(bound: &BoundBoard<'_>) -> (Vec<Finding>, Vec<Limitation>)
```

Split implementation into private modules under `src/validation/`:

- `mod.rs`: top-level orchestration, supported scenario/check dispatch, model
  quality limitations, and once-per-run rule limitation de-duplication. Keeping
  limitation emission centralized preserves report order and avoids duplicate
  honesty-boundary limitations.
- `common.rs`: pure board/model lookup helpers plus shared diagnostic
  constructors whose messages are already part of the report contract. It must
  not become a rule-policy dumping ground.
- `target_contract.rs`: shared target/reset/boot contract checks used by more
  than one rule family. This owns reset target assertions and model boot strap
  comparison logic without owning rule-specific finding builders.
- `backdrive.rs`: `GPIO_BACKDRIVE`.
- `reset_boot.rs`: `RESET_RELEASE_AFTER_POWER_VALID` and `BOOT_STRAP_DEFINED`.
- `uart_bootloader.rs`: `UART_BOOTLOADER_SYNC`.
- `resident_protocol.rs`: `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`.
- `control_line.rs`: `CONTROL_LINE_RELEASE_SEQUENCE`.

The split should use `pub(super)` only where needed by sibling modules. Helpers
must stay internal to `validation`; no new crate-level API is required.

## Shared Ownership Rules

- `validate_sender_endpoint` belongs in `common.rs` because both UART ROM sync
  and resident protocol validation depend on the same transport endpoint
  contract.
- Reset target assertions belong in `target_contract.rs` because reset/boot and
  control-line rules both need the same target/model consistency check.
- Boot strap comparison belongs in `target_contract.rs` with the public
  `BOOT_STRAP_DEFINED` rule wrapper in `reset_boot.rs`; UART sync can reuse the
  same contract check without depending on reset-rule internals.
- Rule-specific finding builders remain local to their rule modules.
- `missing_electrical` remains local to `backdrive.rs`.

## Behavior Preservation Requirements

- Generated reports for existing fixtures must remain semantically identical.
- Rule IDs, finding severity, limitation IDs, suggested fixes, and JSON fields
  must stay stable unless a test exposes a real pre-existing bug.
- Existing scenario type and check mismatch behavior must stay in `mod.rs`.
- Schema files and component models should not change in this refactor.
- The refactor must not introduce STM32-, CH340-, UM-, or C51-specific branches.
- Module names and helper APIs must stay scenario/model-contract based. No rule
  module may reach into fixture-specific assumptions, paths, or acceptance-board
  names.

## Verification

Run:

```sh
circuitci validate <example> --out <baseline-dir>
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
circuitci validate <example> --out <after-dir>
diff -ru <baseline-dir> <after-dir>
```

Then clean build artifacts with `cargo clean` before committing.

## Acceptance

- Every Rust source file remains below 2000 lines.
- All existing tests pass.
- Full generated `report.json` files for all existing example fixtures match the
  pre-refactor baseline exactly.
- `git diff --stat` shows a mostly mechanical move into focused modules.
