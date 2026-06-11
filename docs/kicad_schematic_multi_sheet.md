# KiCad Multi-Sheet Import

Native `.kicad_sch` import now supports multiple child sheets and recursive
nesting. Repeated child component references are represented with deterministic
sheet-instance namespacing when needed.

The conservative multi-sheet contract is:

- every root sheet must declare `Sheetname`, `Sheetfile`, and at least one pin,
- each child sheet must have `hierarchical_label`s that exactly match its parent
  sheet pins,
- nested child sheets must satisfy the same sheet-pin and hierarchical-label
  contract,
- hierarchy cycles are rejected,
- component references that collide after flattening are prefixed with the
  sanitized sheet name, such as `filter_a__R1`,
- repeated non-ground sheet-pin names are allowed only when root connectivity
  and labels disambiguate each sheet instance,
- sanitized sheet-name prefixes must be unique.

These rules are intentional. CircuitCI flattens a sheet pin into a root-net
alias keyed by sheet instance and pin name. Distinct sheet-pin names may share a
root net only when an explicit root label provides the canonical flattened net
name. Repeated sheet-pin names may be reused across instances, but disconnected
root groups that would resolve to the same non-ground alias still fail closed.
Agents should use explicit root labels when connecting or separating repeated
interfaces.

Child-local unlabeled nets are still prefixed with the sheet name at each
flattening boundary. Nets that match declared sheet pins, and ground aliases
such as `GND`, are merged into the flattened Board IR graph.
