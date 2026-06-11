# KiCad Multi-Sheet Import

Native `.kicad_sch` import now supports multiple one-level child sheets in the
root schematic. The importer still rejects nested sheets and repeated
unannotated component references.

The conservative multi-sheet contract is:

- every root sheet must declare `Sheetname`, `Sheetfile`, and at least one pin,
- each child sheet must have `hierarchical_label`s that exactly match its parent
  sheet pins,
- child sheets must not contain nested sheets,
- component references must remain globally unique after flattening,
- non-ground sheet-pin names must be unique across root sheet instances,
- distinct non-ground sheet-pin names must not be wired onto the same root net,
- sanitized sheet-name prefixes must be unique.

These rules are intentional. CircuitCI currently flattens a sheet pin into a
net label at the pin coordinate. Duplicate non-ground sheet-pin names on
different root sheet instances could otherwise merge unrelated child interfaces
by name, while distinct sheet-pin names wired onto one root net would create an
ambiguous multi-label net. Agents should use explicit unique independent
interface names, such as `FILTER_OUT` and `SENSE_IN`, until instance-scoped
hierarchical net names are implemented.

Child-local unlabeled nets are still prefixed with the sheet name. Nets that
match declared sheet pins, and ground aliases such as `GND`, are merged into
the flattened Board IR graph.
