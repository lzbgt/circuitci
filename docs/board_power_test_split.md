# Board Power Test Split

`tests/board_power_cli.rs` owns executable board-power validation coverage:

- static `POWER_TREE_VALID` fixtures,
- static `IO_VOLTAGE_COMPATIBLE` fixtures,
- regulator dropout, output-current, support-capacitance, support-inductance,
  metadata, and startup timing checks,
- reset-supervisor threshold checks,
- datasheet-backed USB-UART and level-shifter power-limit regressions.

`tests/board_interface_protection_cli.rs` owns the interface-focused coverage
split out of the same historical file:

- interface-protection channel checks,
- USB ESD clamp static review,
- USB connector placement/orientation/edge/entry checks,
- USB data/VBUS route geometry and return-path checks.

These tests used to live in `tests/backdrive_cli.rs`, which also contains
behavioral, firmware, schema-walk, and suite-runner coverage. Moving the
power-oriented tests keeps the integration crates below the repository's
2000-line source-file limit and gives future component-pack work a clear home.
