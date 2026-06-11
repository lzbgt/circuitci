# KiCad Schematic Bus Handling

Native `.kicad_sch` import remains wire-level for electrical connectivity.
KiCad bus constructs are not scalar wires and are not treated as ordinary
connectivity.

## Current Contract

The importer accepts bus graphics only when scalar connectivity is explicit:

- `bus` polylines are parsed and validated as horizontal/vertical graphics,
- `bus_entry` markers are parsed and validated,
- every scalar wire touching a `bus_entry` endpoint must carry an explicit
  scalar label,
- `bus_alias` declarations are parsed when every member is an explicit scalar
  label.

The importer does not infer a net name from a bus line or bus entry. This means
a labelled wire such as `DATA0` can be imported even when it visually enters a
bus, but an unlabeled wire entering a bus fails closed.

When one or more `bus_alias` declarations are present, each scalar label on a
wire entering a bus entry must be declared by exactly one alias member set.
Duplicate members across aliases are rejected. Range-like members such as
`DATA[0..7]` are rejected until the importer has a dedicated range-expansion
contract.

This is intentionally stricter than ignoring unknown S-expression sections.
For analog simulation and board-fix agents, silently expanding a bus can remove
or merge reset, boot, enable, address, or data nets incorrectly.

## Future Support

Future bus support should expand KiCad range aliases into individual scalar
nets before Board IR generation. It should still fail closed for unresolved
ranges, duplicate members, conflicting member labels, and bus entries that
cannot be associated with a scalar net.
