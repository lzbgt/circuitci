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

## Non-Goals

This slice does not add hierarchy, buses, rotated symbol instances, hidden
power pins, or value-to-SPICE inference. Those remain unsupported until they
can be modeled without guessing connectivity or physics.
