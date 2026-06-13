# Gerber Outline Importer

`circuitci import-gerber-outline` enriches an existing Board IR project with
board-outline segment evidence from a Gerber outline layer.

```bash
circuitci import-gerber-outline fabrication/Gerber_BoardOutlineLayer.GKO \
  --project out/imported_assembly.project.yaml \
  --output out/imported_with_outline.project.yaml
```

## Imported Evidence

- `board.layout.outline.segments[]`
- segment start/end coordinates in millimeters,
- outline layer name from `G04 Layer: ...` when present,
- `source_primitive: gerber_linear`,
- `source_primitive_index`, `sample_index: 0`, `sample_count: 1`,
- closed-contour `contour_index`,
- `boundary_role: external`, `cutout`, or `unknown`.

The importer classifies enclosed closed contours as cutouts, matching the
existing Board IR outline role contract used by USB connector edge-selection
checks.

## Supported Gerber Subset

The first implementation intentionally supports the narrow, observed
JLC/EasyEDA Pro board-outline subset:

- RS-274X coordinate format declared with `%FSLAX...Y...*%`,
- millimeter units declared with `%MOMM*%`,
- absolute coordinates,
- linear `G01` draw records using `D02` moves and `D01` draws,
- aperture selections and aperture definitions ignored for centerline outline
  geometry.

It fails closed for inches, incremental coordinates, arc interpolation, flashes,
missing units/format declarations, and non-linear outline geometry.

## Limits

Gerber outline import is fabrication-outline evidence only. It does not import
copper, pads, drills, net connectivity, schematic intent, routes, solder mask,
or assembly placement. Combine it with schematic import, PCB import,
JLC/EasyEDA assembly import, or explicit Board IR before using electrical
validation scenarios.
