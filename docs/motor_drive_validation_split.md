# Motor Drive Validation Split

`src/validation/motor_drive.rs` is intentionally kept as the motor-drive
orchestration module for bridge budget, SOA, regeneration, route-current, and
current-sense checks.

Focused bridge electrical screens live in
`src/validation/motor_drive_bridge.rs`:

- `MOTOR_LOAD_SUPPLY_VALID`
- `MOTOR_BRIDGE_LOSS_THERMAL_VALID`
- `MOTOR_BRIDGE_SWITCHING_VALID`

Shared parsing, motor-load evidence resolution, geometry helpers, and common
finding builders remain in `src/validation/motor_drive_common.rs`.

When adding new motor-drive rules, place them in the smallest module that owns
the rule family. Add another focused module before growing
`motor_drive.rs` back toward the 2000-line source guard.
