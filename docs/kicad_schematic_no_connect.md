# Native KiCad No-Connect Evidence

## Purpose

Native `.kicad_sch` import now has enough symbol geometry to see pins that do
not appear in any exported net. Treating those pins as auto-generated nets is
unsafe: it can make a floating schematic pin look like real connectivity.

This slice makes intentional open pins explicit through KiCad `no_connect`
markers and rejects accidental floating pins before Board IR is generated.

## Import Contract

The parser accepts top-level schematic `no_connect` markers only when the
marker point matches exactly one importable symbol instance pin coordinate.
Library pin electrical type `no_connect` is not accepted as schematic evidence;
the explicit schematic marker must be present.

The parser rejects:

- malformed `no_connect` markers without valid coordinates,
- `no_connect` markers that are not attached to any symbol pin,
- `no_connect` markers whose coordinate matches more than one symbol pin,
- `no_connect` markers attached to a pin that is also connected by a wire or
  label,
- symbol instance pins that are not connected by a wire/label and do not have
  an attached `no_connect` marker,
- importable components with all pins marked `no_connect`.

Pins marked `no_connect` are not emitted as Board IR net nodes. They are
import-time evidence only. That keeps the generated Board IR compatible with
the XML netlist path while preventing silent fake connectivity.

## Connectivity Rules

A symbol pin is considered connected when its endpoint is on a wire segment or
has a local/global/power label at the exact pin coordinate. Connected pins flow
through the existing union-find net extraction and KiCad mapping pipeline.

This no-connect slice did not add hierarchy, buses, or hidden power-pin support.
Those features were added in later focused importer slices with their own
tested semantic mappings.

## Acceptance Fixtures

Required coverage:

- a schematic with an explicit `no_connect` marker imports successfully,
- a floating pin without `no_connect` fails closed,
- a floating `no_connect` marker fails closed,
- a malformed `no_connect` marker fails closed,
- a library pin type `no_connect` without a schematic marker still fails,
- a `no_connect` marker on a connected pin fails closed,
- a `no_connect` marker matching multiple pins fails closed,
- a component with all pins marked `no_connect` fails closed.
