# KiCad Import Test Split

## Purpose

`tests/kicad_import_cli.rs` grew close to the 2000-line source limit while
native `.kicad_sch` coverage expanded for rotations, labels, no-connect
markers, and junctions. Future importer work needs room for more fixtures
without making a single integration test file hard to scan.

## Split Boundary

Keep test files aligned with user-visible import commands:

- `tests/kicad_xml_import_cli.rs` owns `circuitci import-kicad-netlist`.
- `tests/kicad_import_cli.rs` continues to own
  `circuitci import-kicad-schematic`.

Shared schema/report helpers remain in `tests/common/mod.rs`. Command-specific
negative-test helpers stay local to their owning test file because XML mapping
failures and native schematic parsing failures exercise different CLI commands
and fixtures.

## Verification Contract

The split is mechanical. It must not change importer behavior or fixture
content. Verification requires:

- focused XML importer tests,
- focused native schematic importer tests,
- full `cargo test`,
- `cargo clippy --all-targets -- -D warnings`,
- existing behavioral and physical acceptance suites.
