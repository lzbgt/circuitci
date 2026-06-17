# Load Budget Validation Split

`src/validation/load_budget.rs` owns load connector and cable checks:

- `LOAD_CONNECTOR_CURRENT_VALID`
- `LOAD_CABLE_CURRENT_VALID`
- `LOAD_CABLE_THERMAL_DERATING_VALID`
- `LOAD_CABLE_VOLTAGE_DROP_VALID`

It also owns shared load-budget parameter parsing and
`VALIDATION_INPUT_MISSING` finding construction used by sibling load-budget
modules.

`src/validation/load_budget_power_switch.rs` owns selected power-switch checks:

- `POWER_SWITCH_BUDGET_VALID`
- `POWER_SWITCH_REVERSE_CURRENT_VALID`
- `POWER_SWITCH_INRUSH_VALID`

This keeps PMU/e-stop switch sign-off logic separate from connector and harness
rules while preserving the same `load_budget` scenario type and report
contract. New eFuse, load-switch, or MOSFET-path checks should go in the
power-switch module unless they are genuinely connector or cable rules.
