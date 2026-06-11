# Native KiCad Schematic Coverage Extension

## Purpose

The first native `.kicad_sch` importer slice proved single-sheet RC-style
connectivity and mapped generated SPICE. This extension adds coverage for the
analog device case that matters most for the current project direction:
datasheet-backed MOSFET model files and SOA evidence from a native schematic.

## Design

The native MOSFET fixture mirrors the existing KiCad XML MOSFET fixture:

- the schematic contributes only symbols, pin geometry, wires, and labels,
- the mapping file contributes model selection, pin maps, primitive source/load
  metadata, SPICE model files, operating conditions, probes, and assertions,
- validation still emits `SCHEMATIC_IMPORT_ONLY`,
- generated SPICE must use the model's MOSFET body policy rather than importer
  inference.

Additional parser fail-closed cases are covered for duplicate references,
missing library pin geometry, and floating labels. These cases close the main
gaps called out during review of the first native schematic slice.

Cardinal symbol rotation is now covered by
`examples/import_kicad_schematic/rotated_rc.kicad_sch`. That fixture rotates the
resistor by 90 degrees and validates the transformed pin coordinates through the
same mapped generated-SPICE path used by the unrotated RC schematic.

Mirrored symbol pin transforms are covered by native schematic parser-rule
tests. The importer supports `(mirror x)` and `(mirror y)` and rejects malformed
or unsupported mirror tokens.

## Non-Goals

This slice does not add non-cardinal symbol rotations. Passive value parsing is
supported only when the mapping file explicitly requests strict
`schematic_value` parsing for resistor or capacitor SPICE primitive values; no
other value-to-SPICE inference is performed. Later slices added conservative
bus handling, multi-unit symbol pin selection, and hidden `power_in` pin import.
