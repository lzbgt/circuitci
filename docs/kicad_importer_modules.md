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

The split is intentionally conservative. It does not change mapping semantics,
generated Board IR shape, or validation behavior; it only keeps the importer
below the source-file size cap before adding more KiCad coverage.
