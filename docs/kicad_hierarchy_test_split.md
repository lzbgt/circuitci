# KiCad Hierarchy Test Split

`tests/kicad_hierarchy_import_cli.rs` owns native KiCad hierarchy workflow
coverage that used to live in `tests/kicad_import_cli.rs`.

The split is mechanical. It keeps the same CLI paths, fixtures, assertions, and
schema checks, but separates hierarchy-heavy tests from mapped single-sheet
workflow tests so future KiCad import automation can be added without pushing a
single Rust test file toward the 2000-line cap.

`tests/kicad_import_cli.rs` remains focused on basic native schematic import,
mapped generated-SPICE fixtures, mapped MOSFET/SOA coverage, rotation fixtures,
and imported schematic scenario-suggestion automation.
