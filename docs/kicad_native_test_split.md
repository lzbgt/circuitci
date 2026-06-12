# Native KiCad Test Split

`tests/kicad_import_cli.rs` now contains native `.kicad_sch` workflow tests:

- basic schematic import,
- explicit mapping into generated SPICE,
- MOSFET/SOA mapped schematic import,
- hierarchy flattening and hierarchy-generated-SPICE coverage.

`tests/kicad_schematic_rules_cli.rs` contains schematic-geometry parser-rule
and fail-closed coverage:

- rotations and transformed pins,
- root hierarchical labels and inferred bus construct guards,
- label and power-symbol conflicts,
- no-connect evidence,
- wire junction semantics.

`tests/kicad_symbol_rules_cli.rs` contains symbol-specific parser-rule and
fail-closed coverage:

- duplicate references,
- `on_board`, `in_bom`, selected unit, and KiCad instance metadata,
- missing pin geometry,
- `extends` inheritance,
- multi-unit symbol pin selection,
- hidden power-pin import and hidden-pin fail-closed cases.

This keeps the native importer integration coverage split by behavior while
preserving the same CLI paths and fixture semantics. Future native schematic
parser rules should go into the matching rules target; future end-to-end mapped
workflow fixtures should stay in `tests/kicad_import_cli.rs`.
