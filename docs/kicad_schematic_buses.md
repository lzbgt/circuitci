# KiCad Schematic Bus Handling

Native `.kicad_sch` import remains wire-level for electrical connectivity.
KiCad bus constructs are not scalar wires and are not treated as ordinary
connectivity.

## Current Contract

The importer accepts bus graphics only when scalar connectivity is explicit:

- `bus` polylines are parsed and validated as horizontal/vertical graphics,
- `bus_entry` markers are parsed and validated,
- every scalar wire touching a `bus_entry` endpoint must carry an explicit
  scalar label or be resolvable from exactly one bus label on the attached bus
  segment,
- `bus_alias` declarations are parsed when every member is an explicit scalar
  label or one simple decimal range such as `DATA[0..7]`.

The importer does not infer a net name from a bus line or bus entry alone. This
means a labelled wire such as `DATA0` can be imported even when it visually
enters a bus. An unlabeled wire entering a bus can be imported only when the
attached bus segment carries one scalar bus label such as `DATA3`, and `DATA3`
is declared by a `bus_alias` member set. A bus label that resolves to multiple
members, such as alias label `DATA` for `DATA[0..7]`, is ambiguous and fails
closed.

When one or more `bus_alias` declarations are present, each scalar label on a
wire entering a bus entry must be declared by exactly one alias member set.
Duplicate members across aliases are rejected. Range-like members such as
`DATA[0..7]` are expanded into scalar members before this check.

Range expansion is intentionally narrow:

- one bracketed decimal range per member,
- ascending bounds only,
- at most 1024 expanded labels per member,
- deterministic zero padding only when both range bounds use the same width.

Grouped syntax, malformed bounds, descending ranges, bus labels not listed by
an alias, and bus-entry wires that cannot be resolved to one scalar member
remain fail-closed.

This is intentionally stricter than ignoring unknown S-expression sections.
For analog simulation and board-fix agents, silently expanding a bus can remove
or merge reset, boot, enable, address, or data nets incorrectly.

## Future Support

Future bus support may add richer KiCad bus-member syntax. It should still fail
closed for unresolved ranges, duplicate members, conflicting member labels, and
bus entries that cannot be associated with a scalar net.
