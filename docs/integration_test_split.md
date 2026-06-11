# Integration Test Split

## Purpose

`tests/backdrive_cli.rs` grew to the repository line-count limit while it was
accumulating CLI acceptance, analog SPICE, importer, and suite tests. This
refactor keeps the same integration coverage but splits tests by workflow so
future physics/import coverage can be added without violating the 2000-line
source-file rule.

## Split Contract

- Shared CLI helpers live in `tests/common/mod.rs`.
- Existing non-import CLI and acceptance tests stay in `tests/backdrive_cli.rs`.
- Native KiCad schematic workflow tests live in `tests/kicad_import_cli.rs`.
- Native KiCad schematic parser-rule tests live in
  `tests/kicad_schematic_rules_cli.rs`.
- KiCad XML netlist importer tests live in `tests/kicad_xml_import_cli.rs`.
- The split is mechanical: no fixture semantics, report assertions, CLI
  arguments, or schema checks change.
- Each integration file remains executable by Cargo as an independent test
  crate and imports helpers with `mod common;`.

## Review Checklist

- No helper writes generated project files outside temp dirs or `out/`.
- Schema validation remains active for imported Board IR and reports.
- Importer negative tests still assert specific diagnostics where the failure
  mode matters.
- Source-file line counts stay comfortably below 2000 after the split.
