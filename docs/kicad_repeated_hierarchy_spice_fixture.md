# KiCad Repeated Hierarchy SPICE Fixture

`examples/import_kicad_repeated_hierarchy_spice/` proves that repeated native
KiCad child-sheet instances can drive generated SPICE validation.

The root schematic instantiates the same `filter.kicad_sch` child twice. Both
children use local references `R1` and `C1`, so import namespaces them to:

- `left_filter__R1`
- `left_filter__C1`
- `right_filter__R1`
- `right_filter__C1`

The mapping file declares the generated SPICE scenario with those flattened
component IDs. This is intentional: a mapping that needs per-instance physical
simulation must target the post-flattening Board IR IDs rather than the local
child-sheet references.

The scenario runs a transient RC charge simulation and asserts both repeated
outputs exceed 2.5 V at 2 ms.
