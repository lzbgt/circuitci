# Gerber Copper Importer

`circuitci import-gerber-copper` enriches an existing Board IR project with
anonymous flashed copper feature, circular-aperture trace, and single-contour
region evidence from a Gerber copper layer.

```bash
circuitci import-gerber-copper fabrication/Gerber_TopLayer.GTL \
  --project out/imported_with_outline_and_drills.project.yaml \
  --output out/imported_with_copper.project.yaml
```

## Imported Evidence

- `board.layout.copper.features[]`,
- `board.layout.copper.segments[]`,
- `board.layout.copper.regions[]`,
- feature center coordinates in millimeters,
- segment start/end coordinates and trace width in millimeters,
- region polygon points in millimeters,
- layer name from `G04 Layer: ...` when present,
- `polarity: dark`,
- `source_primitive: gerber_flash`,
- `source_primitive: gerber_linear_draw` or `gerber_arc_draw`,
- `source_primitive: gerber_region`,
- `source_primitive_index`,
- aperture code such as `D10`,
- aperture shape: `circle`, `rect`, or `oval`; observed EasyEDA `RoundRect`
  aperture macros are imported as bounded `rect` apertures,
- aperture X/Y size in millimeters,
- optional `net` and `island_id` when existing Board IR layout evidence gives
  exactly one owner,
- optional `owner_kind`, `component`, `pin`, and `via_index` on flash evidence
  when a unique pad or via owner can be correlated.

For ownership matching, EasyEDA layer names `TopLayer` and `BottomLayer` are
treated as aliases of Board IR copper layers `F.Cu` and `B.Cu`.
Pad ownership matching honors imported pad `rotation_deg` for rectangular,
oval, and polygon-style bounding geometry.

## Supported Gerber Subset

The first implementation intentionally supports a bounded fabrication-copper
subset:

- RS-274X coordinate format declared with `%FSLAX...Y...*%`,
- millimeter units declared with `%MOMM*%`,
- absolute coordinates,
- aperture definitions for `C`, `R`, `O`, and observed EasyEDA `RoundRect`
  apertures,
- bare `Dnn` and `G54Dnn` aperture selection,
- dark-polarity `D03` flashes,
- dark-polarity linear `D01` draws with circular apertures,
- dark-polarity `G02`/`G03` circular-aperture arc draws sampled into bounded
  segment evidence,
- dark-polarity single-contour `G36`/`G37` regions made from linear or sampled
  arc edges.

Draw records with non-circular apertures are counted as ignored
draw records because their exact swept geometry is not a simple trace-width
segment. Clear-polarity flashes, draws, and regions are skipped/ignored because
they represent copper voids rather than conductive copper. Multi-contour,
nested, open, degenerate, or flashed regions fail closed.

## Limits

Gerber copper import is fabrication geometry evidence. When the input Board IR
already contains PCB layout evidence, the importer can annotate imported copper
with ownership from exactly one matching owner:

- `net` plus pad `owner_kind`/`component`/`pin` from pad overlap in
  `board.layout.pads`,
- `net` plus via `owner_kind`/`via_index` from via overlap in
  `board.layout.routes`,
- `net` from route overlap in `board.layout.routes`,
- `net` plus zone-derived `island_id` from zone containment in
  `board.layout.zones`.

It does not infer annular rings, schematic intent, or electrical connectivity.
Ambiguous or missing ownership evidence leaves the imported copper anonymous.
Combine it with schematic, PCB, assembly, outline, and drill imports before
using electrical or manufacturability checks.

`DRILL_ANNULAR_RING_VALID` can consume imported dark flash evidence together
with Excellon drill hits for a static annular-ring screen. When drill and
copper ownership is available, the rule rejects conflicting owner nets, and
scenarios can require matching flash evidence on explicit copper layers such
as `F.Cu` and `B.Cu`. It still does not prove thermal relief connectivity or
full electrical continuity.

`COPPER_TO_BOARD_EDGE_CLEARANCE_VALID` can consume imported dark flash,
circular-aperture draw, and region evidence together with board-outline
evidence for a static copper-to-board-edge screen. It can use the
`jlcpcb_routed_edge_copper_clearance_2026_06` fabrication preset when the board
outline represents routed board edges or routed slots. The rule still operates
on anonymous 2D fabrication geometry; it does not prove net ownership, copper
island connectivity, solder-mask margin, V-cut panel clearance, or fab-specific
etch compensation.

`COPPER_SPACING_VALID` can consume the same imported dark flash,
circular-aperture draw, and region evidence for a static same-layer
copper-spacing screen. When imported copper has `net` or `island_id` ownership
evidence, the rule can distinguish same-owner contact from conflicting-owner
overlap. Anonymous touching copper is still ignored to avoid flagging
intentionally connected Gerber primitives without ownership evidence.
