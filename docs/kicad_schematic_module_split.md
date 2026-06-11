# KiCad Schematic Importer Module Split

The native KiCad schematic importer is split by parser responsibility:

- `src/importers/kicad_sch.rs`: file orchestration, hierarchy flattening,
  wires, junctions, buses, labels, no-connect validation, and Board IR net
  construction.
- `src/importers/kicad_sch/sexp.rs`: generic S-expression tokenization and
  list access helpers.
- `src/importers/kicad_sch/symbols.rs`: library symbol pin geometry,
  property-only `extends` inheritance, multi-unit symbol selection, hidden
  power pins, symbol rotation/mirroring, and schematic symbol instance parsing.

This keeps the schematic importer under the project line-count cap while
preserving the conservative fail-closed behavior added for hierarchy, buses,
symbol inheritance, multi-unit symbols, and hidden power pins.
