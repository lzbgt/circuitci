# Board Power Test Split

`tests/board_power_cli.rs` owns executable board-power validation coverage:

- static `POWER_TREE_VALID` fixtures,
- regulator dropout, output-current, metadata, and startup timing checks,
- interface-protection channel checks,
- datasheet-backed USB-UART and level-shifter power-limit regressions.

These tests used to live in `tests/backdrive_cli.rs`, which also contains
behavioral, firmware, schema-walk, and suite-runner coverage. Moving the
power-oriented tests keeps the integration crates below the repository's
2000-line source-file limit and gives future component-pack work a clear home.
