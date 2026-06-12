# KiCad Root Hierarchical Labels

## Purpose

KiCad hierarchical labels normally define a child sheet interface: a
hierarchical label in a child sheet connects to a matching sheet pin in the
parent sheet. The root sheet has no parent sheet, but root-level hierarchical
labels may still appear in real schematics.

CircuitCI imports root hierarchical labels as root-sheet net labels instead of
rejecting the schematic. This is intentionally conservative:

- a root hierarchical label names only the attached root net,
- it does not create an implicit parent-sheet interface,
- it follows the same attachment and conflict rules as local/global labels,
- child hierarchical labels still must exactly match parent sheet pins.

## Source Basis

The saved KiCad 9 schematic editor docs state that hierarchical labels connect
to hierarchical sheet pins and are used to connect child sheets to their parent
sheet. The same section also states that labels with the same name connect
within the same sheet regardless of label type, and that net naming can come
from the highest hierarchy level where a local or hierarchical label appears.

Local source copy:

- `docs/research/kicad/kicad_9_eeschema.html`

## Fail-Closed Behavior

Root hierarchical labels fail closed when they:

- have no label text,
- have malformed or missing coordinates,
- float without attachment to a wire or symbol pin,
- duplicate another label at the same coordinate,
- conflict with another label on the same connected net.

Child hierarchical labels keep the existing stricter interface contract:
duplicates are rejected and every child label must match exactly one parent
sheet pin.
