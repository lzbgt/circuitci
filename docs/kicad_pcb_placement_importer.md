# KiCad PCB Layout Evidence Importer

`circuitci import-kicad-pcb` enriches an existing Board IR project with
component placement, footprint drawing, pad, routed-net, zone, and routing-rule
evidence from a KiCad `.kicad_pcb` file:

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
Placement evidence includes component center coordinates, side when it can be
derived from the footprint layer, and footprint `rotation_deg` from the KiCad
`(at x y rotation)` tuple.

The importer also reads footprint `fp_line`, `fp_rect`, and `fp_poly` drawing
items and writes transformed drawing evidence under `board.layout.footprints`
for matching Board IR components. Imported footprint drawing evidence includes:

- component reference,
- transformed start/end coordinates for lines and rectangles or transformed
  point lists for polygons, in millimeters,
- source layer,
- a bounded semantic kind derived from the layer: `fabrication`, `courtyard`,
  `silkscreen`, or `other`.

This is drawing evidence, not a full mechanical body solver. Rectangles are
stored as their transformed opposite corners; rotated rectangles should be
treated as evidence for follow-up rules, not as exact polygonal body sign-off.

The importer also reads connected footprint `pad` entries and writes pad
evidence under `board.layout.pads` when the footprint reference and pad net both
map to existing Board IR objects. Unconnected pads are skipped. Imported pad
evidence includes:

- component reference,
- pad name,
- Board IR net,
- pad center in millimeters after footprint translation/rotation,
- KiCad pad kind and shape when present,
- pad size in millimeters,
- pad rotation in degrees when non-zero,
- scalar pad drill diameter in millimeters when present,
- pad layer list when present.

The importer also reads KiCad `gr_line`, `net`, `segment`, `via`, and `zone`
entries. Straight board-edge `gr_line` items on `Edge.Cuts` are written under
`board.layout.outline.segments`. Routed geometry is written under
`board.layout.routes`; copper-zone outlines and saved `filled_polygon` geometry
are written under `board.layout.zones` only when the PCB net can be matched to
an existing Board IR net. The importer does not create new schematic nets from
PCB data. Net matching tries exact names, lowercase names, common ground
aliases, native KiCad import names such as `net_usb_dp`, and a deterministic
sanitized-name match. Ambiguous net matches fail closed.

Imported route evidence currently includes:

- copper segment `start`/`end` points in millimeters,
- segment `width_mm`,
- segment `layer`,
- via `at` point,
- via `size_mm`,
- via `drill_mm`,
- via layer stack when present.

Imported zone evidence includes:

- zone net,
- copper layer,
- polygon outline points in millimeters.
- saved filled-polygon points in millimeters when present.

Imported outline evidence includes:

- straight `Edge.Cuts` segment start/end points in millimeters,
- source layer, currently `Edge.Cuts`.

When the enriched project is written to a different directory, relative
`libraries` entries are rewritten to absolute paths so follow-up
`validate`/`suggest-scenarios` commands still resolve the same model packs.

This is bounded layout evidence, not a full PCB layout solver. The importer
extracts component center placements, routed `segment`/`via` geometry, net-class
route/differential-pair defaults, simple custom DRC `length`/`skew`
constraints whose conditions name a net class or explicit net, and copper-zone
outlines plus saved filled polygons. It also extracts matched footprint drawing
items and connected pad center, kind, shape, size, rotation, scalar drill, net,
and layer evidence. It does not solve arcs or non-line board edges, exact
rotated-body polygons, filled-copper island connectivity, pad-to-zone
connectivity, thermal relief behavior, solder-mask expansion, shield bonding,
return paths, impedance calculations, arbitrary DRC rule semantics, or pin-1/BOM/PNP
alignment.

Fixture coverage:

- `examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb`
- `tests/kicad_pcb_import_cli.rs`

The regression imports the matching native KiCad schematic, enriches it with
PCB placements, footprint drawing evidence, connected pad geometry, routed USB
net geometry, copper-zone outline/fill evidence, and routing-rule evidence, then proves
`suggest-scenarios` emits USB placement, route, and return-path templates with
measured layout evidence.
