# KiCad Nested Hierarchy

Native `.kicad_sch` import flattens nested sheets recursively. Each hierarchy
edge uses the same contract:

- the parent sheet `pin` set must exactly match the child schematic
  `hierarchical_label` set,
- sheet pins must attach to real connectivity,
- unlabeled local nets are prefixed with the sheet name at the boundary where
  they are flattened,
- colliding component references are prefixed with the sanitized sheet name,
- recursive sheet-file cycles fail closed.

The implementation flattens from leaves toward the root. This lets an
intermediate child schematic export `VIN`/`OUT`/`GND` labels to its parent while
internally mapping those labels to another nested sheet.

`examples/import_kicad_nested_hierarchy_spice/` proves the importer can flatten
root -> child -> grandchild connectivity and then run a generated SPICE
transient from the mapped Board IR.
