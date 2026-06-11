# Native KiCad Junction Handling

## Purpose

KiCad junction dots are explicit connectivity evidence. In the native
`.kicad_sch` importer they influence which wire segments are connected before
the project is converted into Board IR. A malformed or meaningless junction
must not be ignored, because that can turn an ambiguous drawing into a silent
connectivity assumption.

## Source Basis

The saved KiCad schematic S-expression reference under
`docs/research/kicad/sexpr-schematic.html` defines the `junction` token as a
positioned schematic object with a `POSITION_IDENTIFIER`.

## Import Contract

The importer accepts a junction only when:

- the junction has valid coordinates,
- no other junction already exists at the same coordinate,
- the junction coordinate lies on at least two wire segments.

The importer rejects:

- malformed junctions without valid coordinates,
- duplicate junctions at the same coordinate,
- floating junctions that lie on no wire,
- redundant junctions that touch only one wire segment,
- wire crossings where both wires pass through the crossing point and no
  explicit junction is present.

Endpoint touches remain valid without a junction because the endpoint itself is
already represented as a wire graph node. This includes endpoint-to-endpoint
touches and endpoint-to-midspan T-touches. Mid-span crossings where neither wire
ends at the crossing point require an explicit junction to avoid guessing
whether the schematic intended a connection or a visual crossing.

Junctions that touch two or more wire segments are accepted even when they are
redundant, such as a two-segment corner or collinear overlap. They do not create
new topology beyond the existing wire graph, but they are valid KiCad schematic
objects and are not silently dropped.

## Required Coverage

- a mid-span wire crossing without junction fails closed,
- the same crossing with a valid junction imports successfully,
- malformed, duplicate, floating, and one-segment junctions fail closed,
- endpoint-to-endpoint and endpoint-to-midspan touches without junction still
  import successfully,
- two-segment corner and collinear junctions are accepted as redundant explicit
  evidence.
