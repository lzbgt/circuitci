# Native KiCad Label Conflict Handling

## Purpose

Native `.kicad_sch` import uses labels as net names. A label is not just
decoration: once imported, it can drive model mapping, generated SPICE node
names, assertions, and report evidence. The importer must not silently choose
one label when the schematic contains duplicate or conflicting labels at the
same coordinate.

## Import Contract

The importer accepts local and global labels when each label has:

- a name,
- a valid `(at X Y [ANGLE])` position,
- exact attachment to a wire segment or transformed symbol pin coordinate.

The importer rejects:

- malformed label entries without a name or valid coordinate,
- power symbols without a non-empty `Value` label,
- two labels at the same coordinate with different names,
- two labels at the same coordinate with the same name, because duplicate label
  objects are ambiguous schematic evidence,
- labels on the same connected net group with different names.

Label text rotation and orientation are ignored for connectivity. Only the
label coordinate controls attachment and net naming.

Within the current single-sheet subset, local labels and global labels are both
treated as sheet-local net aliases. Hierarchical sheets are rejected before
import, so global label scope does not cross sheet boundaries yet.

Power-symbol labels are injected from transformed power-symbol pins. They share
the same conflict rules as explicit labels, so an explicit label that conflicts
with or duplicates a power symbol at the same coordinate fails closed.

## Required Coverage

- duplicate same-name labels at the same coordinate fail closed,
- conflicting labels at the same coordinate fail closed,
- malformed labels without coordinates fail closed,
- malformed global labels fail closed,
- local/global labels with different names on one connected net fail closed,
- conflicting explicit and power-symbol labels at one coordinate fail closed,
- duplicate power-symbol labels at one coordinate fail closed,
- power symbols with empty labels fail closed,
- a label attached to a rotated symbol pin remains accepted.
