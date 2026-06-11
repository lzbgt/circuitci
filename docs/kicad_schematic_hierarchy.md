# KiCad Schematic Hierarchy

Native `.kicad_sch` import supports a strict recursive hierarchy slice:

- one root schematic may instantiate one or more child sheets,
- child sheets may instantiate additional child sheets,
- the child sheet file path must be declared by the sheet `Sheetfile` property,
- each parent sheet `pin` name must have an identically named child
  `hierarchical_label`,
- each child `hierarchical_label` must have a matching parent sheet pin,
- hierarchy cycles are rejected,
- duplicate component references across root and child sheets, or across child
  sheet instances, are made instance-scoped with the sanitized sheet name,
- distinct non-ground sheet-pin names wired onto one root net require an
  explicit root label as the canonical flattened net name,
- sheet names that sanitize to the same child-local net prefix are rejected,
- unsupported buses remain rejected.

The importer flattens children bottom-up into the same Board IR graph. Parent
sheet pins are treated as labels at the sheet-pin coordinates. Child
hierarchical labels are treated as labels in the child schematic. Nets with
matching names are merged; unlabeled child-local nets are prefixed with the
sheet name so they cannot collide with parent-local auto names.

Repeated child sheet instances are represented with deterministic component
instance names when their local references would collide. For example, child
reference `R1` in sheet `Filter A` becomes `filter_a__R1` only when that child
reference conflicts with another flattened component. Unique child references
remain unchanged for stable imports. See `docs/kicad_schematic_multi_sheet.md`
for the current multi-sheet contract and
`docs/kicad_nested_hierarchy_spice_fixture.md` for an end-to-end generated
SPICE proof.
