# KiCad PCB Placement Importer

`circuitci import-kicad-pcb` enriches an existing Board IR project with
component placement evidence from a KiCad `.kicad_pcb` file:

```bash
circuitci import-kicad-pcb board.kicad_pcb \
  --project imported_schematic.project.yaml \
  --output imported_with_layout.project.yaml
```

The importer reads KiCad `footprint` entries, extracts the footprint reference
from `property "Reference"` or `fp_text reference`, reads the footprint `(at x
y ...)` position in millimeters, and maps footprint layer prefixes to Board IR
placement side:

- `F.*` -> `top`
- `B.*` -> `bottom`
- other layers -> side omitted

Only footprint references that match existing `board.components` are written to
`board.layout.placements`. Extra mechanical footprints are ignored. Duplicate
footprint references, missing references, invalid coordinates, files without
footprints, and PCB files with no matching Board IR components fail closed.
When the enriched project is written to a different directory, relative
`libraries` entries are rewritten to absolute paths so follow-up
`validate`/`suggest-scenarios` commands still resolve the same model packs.

This is placement evidence only. The importer does not extract routed traces,
vias, differential-pair geometry, shield bonding, copper pours, clearances,
footprint pad geometry, or pin-1/BOM/PNP alignment.

Fixture coverage:

- `examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb`
- `tests/kicad_pcb_import_cli.rs`

The regression imports the matching native KiCad schematic, enriches it with
PCB placements, and proves `suggest-scenarios` emits a
`USB_PROTECTION_PLACEMENT_VALID` template with connector-to-protection distance
evidence.
