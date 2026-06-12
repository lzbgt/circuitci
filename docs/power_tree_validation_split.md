# Power Tree Validation Split

## Purpose

`src/validation/power_tree.rs` was approaching the repository 2000-line source
limit after adding regulator, load-switch, charger, power-mux, I/O-voltage,
and rail-budget checks. The split keeps behavior unchanged while creating room
for more board-validation logic.

## Module Ownership

- `src/validation/power_tree.rs` owns orchestration, rail scanning, generic
  rail voltage/current checks, regulator, load-switch, charger, and I/O-voltage
  checks.
- `src/validation/power_tree/power_mux.rs` owns `power_mux` validation:
  source-selection evidence, selected-input powered checks, inactive-input
  reverse-blocking checks, output-current limits, and power-mux metadata
  diagnostics.

## Review Checklist

- Power-mux behavior changes should add focused fixtures under `examples/` and
  assertions in `tests/board_power_cli.rs`.
- Keep shared helpers in `power_tree.rs` only when they are used by multiple
  power-tree submodules.
- Do not add new power-validation logic to a file that would push it near the
  2000-line guard.
