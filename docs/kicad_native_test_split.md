# Native KiCad Test Split

`tests/kicad_import_cli.rs` now contains native `.kicad_sch` workflow tests:

- basic schematic import,
- explicit mapping into generated SPICE,
- MOSFET/SOA mapped schematic import,
- hierarchy flattening and hierarchy-generated-SPICE coverage.

`tests/kicad_schematic_rules_cli.rs` contains parser-rule and fail-closed
coverage:

- rotations and transformed pins,
- unsupported root hierarchical labels and bus constructs,
- duplicate references and missing pin geometry,
- label and power-symbol conflicts,
- no-connect evidence,
- wire junction semantics.

This keeps the native importer integration coverage split by behavior while
preserving the same CLI paths and fixture semantics. Future native schematic
parser rules should go into `tests/kicad_schematic_rules_cli.rs`; future
end-to-end mapped workflow fixtures should stay in `tests/kicad_import_cli.rs`.
