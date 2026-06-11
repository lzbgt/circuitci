# KiCad Schematic Hierarchy

Native `.kicad_sch` import supports a first strict hierarchy slice:

- one root schematic may instantiate one child sheet,
- the child sheet file path must be declared by the sheet `Sheetfile` property,
- each parent sheet `pin` name must have an identically named child
  `hierarchical_label`,
- each child `hierarchical_label` must have a matching parent sheet pin,
- nested sheets are rejected,
- duplicate component references across root and child sheets are rejected,
- unsupported buses remain rejected.

The importer flattens the child sheet into the same Board IR graph. Parent sheet
pins are treated as labels at the sheet-pin coordinates. Child hierarchical
labels are treated as labels in the child schematic. Nets with matching names
are merged; unlabeled child-local nets are prefixed with the sheet name so they
cannot collide with root-local auto names.

This first slice intentionally avoids multi-instance hierarchy. Multiple sheet
instances of the same child require reference and local-net namespacing before
they can be represented safely for board/simulation agents.
