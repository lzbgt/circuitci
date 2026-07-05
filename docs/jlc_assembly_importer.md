# JLC/EasyEDA Assembly Importer

`circuitci import-jlc-assembly` converts a JLC/EasyEDA-style BOM CSV plus
placement/CPL CSV into assembly-evidence Board IR and writes a schema-validated
JSON import manifest beside the generated project. By default, the manifest
uses the output path with a `.json` extension; pass `--manifest` to choose an
explicit path.

```bash
circuitci import-jlc-assembly \
  --bom assembly/bom_STM32_ESP32_V01_2026-04-28.csv \
  --placement assembly/placement_STM32_ESP32_V01_2026-04-28.csv \
  --output out/imported_assembly.project.yaml \
  --manifest out/imported_assembly_manifest.json \
  --name um_stm32_esp32_assembly
```

## Imported Evidence

- one Board IR component per BOM or placement designator,
- `source.format: jlc_assembly`,
- BOM row, quoted designator group, quantity, value/comment, footprint,
  manufacturer part, manufacturer, supplier part, and supplier,
- placement device, footprint, comment/name, pin count, SMD flag,
- raw placement layer, normalized side, side confidence, raw rotation,
  normalized rotation, and orientation confidence,
- `board.layout.placements.<ref>.x_mm`, `y_mm`, `side`, and `rotation_deg`.

The JSON manifest conforms to `schemas/jlc_assembly_import.schema.json` and
records:

- BOM and placement source paths, sizes, SHA-256 hashes, raw column names, and
  data-row counts,
- accepted BOM rows with source row number, row SHA-256 fingerprint,
  designator group, split designators, quantity, manufacturer, supplier,
  value, and footprint fields,
- accepted placement rows with source row number, designator, coordinates,
  row SHA-256 fingerprint, raw layer, normalized side, side confidence, raw
  rotation, normalized rotation, orientation confidence, SMD flag,
  comment/name, and pin count,
- one component join row per generated component showing whether BOM and/or
  placement evidence was present, the source row numbers, selected part number,
  selected footprint, placement coordinates, side confidence, and orientation
  confidence.

The importer validates required headers, quoted CSV fields, duplicate
designators, quantity/designator-count mismatches, non-finite placement
coordinates, invalid rotations, and malformed boolean/integer fields.

## Limits

This importer does not infer nets, electrical pins, power rails, routes, pads,
final assembly polarity/orientation, or schematic intent. `side_confidence` and
`orientation_confidence` only say whether the placement CSV supplied a
recognized side token or rotation value. They are evidence quality markers, not
proof that the footprint origin, package pin 1, and manufacturer assembly view
all agree. The importer intentionally emits low-confidence imported components
with empty pin maps and no scenarios. Use schematic import, PCB import, Gerber
outline/copper import, drill import, or explicit Board IR mapping before
treating the board as electrically validated.

For fabricated JLC/EasyEDA releases, `import-gerber-outline` can be run after
assembly import to add board-outline segment evidence from the release's
`Gerber_BoardOutlineLayer.GKO` file. `import-gerber-copper` can append
anonymous dark flashed copper evidence from top/bottom copper Gerbers when
those files are available. `import-excellon-drill` can then append PTH/NPTH
drill-hit evidence from files such as `Drill_PTH_Through.DRL` and
`Drill_NPTH_Through.DRL`.

The first regression fixture is a small committed extract shaped like the peer
`../urine_monitor` fabricated JLC/EasyEDA Pro release:

- `examples/import_jlc_assembly_peer_extract/bom.csv`
- `examples/import_jlc_assembly_peer_extract/placement.csv`
- `examples/import_jlc_gerber_outline_peer_extract/board_outline.gko`
- `examples/import_jlc_gerber_copper_peer_extract/front_copper.gtl`
- `examples/import_jlc_excellon_drill_peer_extract/pth.drl`
- `examples/import_jlc_excellon_drill_peer_extract/npth.drl`
