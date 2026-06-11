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
- sanitized sheet-name prefixes must be unique.

These rules are intentional. CircuitCI currently flattens a sheet pin into a
root-net alias. Duplicate non-ground sheet-pin names on different root sheet
instances could otherwise merge unrelated child interfaces by name. Distinct
sheet-pin names may share a root net only when an explicit root label provides
the canonical flattened net name. Agents should use explicit interface names,
such as `FILTER_OUT` and `SENSE_IN`, and explicit root labels when connecting
interfaces together until instance-scoped hierarchical net names are
implemented.

Child-local unlabeled nets are still prefixed with the sheet name. Nets that
match declared sheet pins, and ground aliases such as `GND`, are merged into
the flattened Board IR graph.
