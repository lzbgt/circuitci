# Native KiCad Test Split

`tests/kicad_import_cli.rs` now contains native `.kicad_sch` mapped workflow
tests:

- basic schematic import,
- explicit mapping into generated SPICE,
- MOSFET/SOA mapped schematic import,
- generated scenario suggestions from mapped schematic passives and mapped
  datasheet-backed component metadata.

`tests/kicad_hierarchy_import_cli.rs` contains hierarchy workflow coverage:

- one-level, multi-child, repeated, and nested hierarchy flattening,
- hierarchy-generated-SPICE fixtures,
- hierarchy aliasing and repeated-sheet namespacing,
- fail-closed hierarchy contracts around sheet pins, cycles, duplicate sheet
  names, alias collisions, and ambiguous root nets.

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
