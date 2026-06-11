# KiCad Nested Hierarchy SPICE Fixture

`examples/import_kicad_nested_hierarchy_spice/` proves that native KiCad
schematics can flatten a root sheet, an intermediate child sheet, and a
grandchild analog subcircuit into one Board IR graph.

The fixture structure is:

- `root.kicad_sch`: source `V1` and sheet `Analog Frontend`,
- `frontend.kicad_sch`: exported `VIN`/`OUT`/`GND` labels and nested sheet
  `Filter`,
- `filter.kicad_sch`: RC low-pass network.

The mapping file declares a generated SPICE transient over `V1`, `R1`, and
`C1`, then asserts the nested output net exceeds 2.5 V at 2 ms. This keeps the
contract explicit: hierarchy import only provides connectivity, while the
mapping file chooses physical components, stimulus, probes, and assertions.
