# JLC/EasyEDA Assembly Importer

`circuitci import-jlc-assembly` converts a JLC/EasyEDA-style BOM CSV plus
placement/CPL CSV into assembly-evidence Board IR.

```bash
circuitci import-jlc-assembly \
  --bom assembly/bom_STM32_ESP32_V01_2026-04-28.csv \
  --placement assembly/placement_STM32_ESP32_V01_2026-04-28.csv \
  --output out/imported_assembly.project.yaml \
  --name um_stm32_esp32_assembly
```

## Imported Evidence

- one Board IR component per BOM or placement designator,
- `source.format: jlc_assembly`,
- BOM row, quoted designator group, quantity, value/comment, footprint,
  manufacturer part, manufacturer, supplier part, and supplier,
- placement device, footprint, comment/name, pin count, SMD flag,
- `board.layout.placements.<ref>.x_mm`, `y_mm`, `side`, and `rotation_deg`.

The importer validates required headers, quoted CSV fields, duplicate
designators, quantity/designator-count mismatches, non-finite placement
coordinates, invalid rotations, and malformed boolean/integer fields.

## Limits

This importer does not infer nets, electrical pins, power rails, routes, pads,
or schematic intent. It intentionally emits low-confidence imported components
with empty pin maps and no scenarios. Use schematic import, PCB import, Gerber
import, or explicit Board IR mapping before treating the board as electrically
validated.

The first regression fixture is a small committed extract shaped like the peer
`../urine_monitor` fabricated JLC/EasyEDA Pro release:

- `examples/import_jlc_assembly_peer_extract/bom.csv`
- `examples/import_jlc_assembly_peer_extract/placement.csv`
