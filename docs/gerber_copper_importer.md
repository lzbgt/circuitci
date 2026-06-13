# Gerber Copper Importer

`circuitci import-gerber-copper` enriches an existing Board IR project with
anonymous flashed copper feature, circular-aperture linear trace, and
single-contour region evidence from a Gerber copper layer.

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
- `source_primitive: gerber_linear_draw`,
- `source_primitive: gerber_region`,
- `source_primitive_index`,
- aperture code such as `D10`,
- aperture shape: `circle`, `rect`, or `oval`,
- aperture X/Y size in millimeters.

## Supported Gerber Subset

The first implementation intentionally supports a bounded fabrication-copper
subset:

- RS-274X coordinate format declared with `%FSLAX...Y...*%`,
- millimeter units declared with `%MOMM*%`,
- absolute coordinates,
- aperture definitions for `C`, `R`, and `O` shapes,
- bare `Dnn` and `G54Dnn` aperture selection,
- dark-polarity `D03` flashes,
- dark-polarity linear `D01` draws with circular apertures,
- dark-polarity single-contour `G36`/`G37` regions made from linear `D01`
  edges.

Linear `D01` draw records with non-circular apertures are counted as ignored
draw records because their exact swept geometry is not a simple trace-width
segment. Clear-polarity flashes, draws, and regions are skipped/ignored because
they represent copper voids rather than conductive copper. Multi-contour,
nested, open, degenerate, flashed, or arc-interpolated regions fail closed.

## Limits

Gerber copper import is fabrication geometry evidence only. It does not infer
nets, components, pad names, annular rings, zones, schematic intent, or
electrical connectivity. Combine it with schematic, PCB, assembly, outline, and
drill imports before using electrical or manufacturability checks.

`DRILL_ANNULAR_RING_VALID` can consume imported dark flash evidence together
with Excellon drill hits for a static annular-ring screen. That rule still
operates on anonymous fabrication geometry; it does not prove net ownership,
thermal relief connectivity, or electrical continuity.

`COPPER_TO_BOARD_EDGE_CLEARANCE_VALID` can consume imported dark flash,
circular-aperture draw, and region evidence together with board-outline
evidence for a static copper-to-board-edge screen. That rule still operates on
anonymous 2D fabrication geometry; it does not prove net ownership, copper
island connectivity, solder-mask margin, or fab-specific etch compensation.

`COPPER_SPACING_VALID` can consume the same imported dark flash,
circular-aperture draw, and region evidence for a static same-layer
copper-spacing screen. Because Gerber copper import is anonymous, the rule
ignores overlapping or touching copper objects to avoid flagging intentionally
connected copper primitives. Use net-aware PCB evidence for electrical short
sign-off.
