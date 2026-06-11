# KiCad Schematic Bus Handling

Native `.kicad_sch` import remains wire-level only. KiCad bus constructs are
not scalar wires and must not be treated as ordinary connectivity until the
importer can expand bus members with explicit labels and deterministic mapping.

## Current Contract

The importer fails closed when a schematic contains any top-level bus construct:

- `bus`
- `bus_entry`
- `bus_alias`

The diagnostic is explicit so an agent does not mistake an imported partial
schematic for complete connectivity.

This is intentionally stricter than ignoring unknown S-expression sections.
For analog simulation and board-fix agents, silently dropping a bus can remove
reset, boot, enable, address, or data nets from the generated Board IR.

## Future Support

Future bus support should parse KiCad bus aliases and member labels into
individual scalar nets before Board IR generation. It should still fail closed
for unresolved ranges, duplicate members, conflicting member labels, and bus
entries that cannot be associated with a scalar net.
