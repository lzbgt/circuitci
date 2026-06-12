# KiCad Importer Modules

The KiCad importer has grown into several independent contracts. Keep new code
inside the smallest module that owns the behavior:

- `src/importers/kicad.rs` owns the public XML import entry point, shared Board
  IR construction, mapping-file validation, analog scenario construction, and
  helper types used by the native schematic importer.
- `src/importers/kicad/passive_values.rs` owns strict opt-in schematic `Value`
  parsing for mapped resistor and capacitor SPICE primitive values.
- `src/importers/kicad_sch.rs` owns native `.kicad_sch` S-expression
  connectivity extraction before handing the parsed netlist to the shared KiCad
  builder.
- `src/importers/kicad_pcb.rs` owns the `import-kicad-pcb` orchestration,
  project enrichment, shared footprint placement/pad coordinate transforms,
  routed geometry, zones, and KiCad routing-rule import.
- `src/importers/kicad_pcb/footprints.rs` owns matched footprint drawing
  evidence import and serialization for `fp_line`, `fp_rect`, `fp_poly`,
  `fp_circle`, and `fp_arc` items.
- `src/importers/kicad_pcb/outline.rs` owns Edge.Cuts board-outline import,
  bounded curve sampling, closed-contour cutout classification, and outline
  evidence serialization.

The split is intentionally conservative. It does not change mapping semantics,
generated Board IR shape, or validation behavior; it only keeps the importer
below the source-file size cap before adding more KiCad coverage.
