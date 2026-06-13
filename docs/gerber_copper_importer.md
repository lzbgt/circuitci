# Gerber Copper Importer

`circuitci import-gerber-copper` enriches an existing Board IR project with
anonymous flashed copper feature evidence from a Gerber copper layer.

```bash
circuitci import-gerber-copper fabrication/Gerber_TopLayer.GTL \
  --project out/imported_with_outline_and_drills.project.yaml \
  --output out/imported_with_copper.project.yaml
```

## Imported Evidence

- `board.layout.copper.features[]`,
- feature center coordinates in millimeters,
- layer name from `G04 Layer: ...` when present,
- `polarity: dark`,
- `source_primitive: gerber_flash`,
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
- dark-polarity `D03` flashes.

Linear `D01` draw records are counted in the CLI summary but are not converted
into Board IR copper geometry yet. Clear-polarity flashes are skipped because
they represent copper voids rather than conductive copper.

## Limits

Gerber copper import is fabrication geometry evidence only. It does not infer
nets, components, pad names, annular rings, zones, schematic intent, or
electrical connectivity. Combine it with schematic, PCB, assembly, outline, and
drill imports before using electrical or manufacturability checks.

`DRILL_ANNULAR_RING_VALID` can consume imported dark flash evidence together
with Excellon drill hits for a static annular-ring screen. That rule still
operates on anonymous fabrication geometry; it does not prove net ownership,
thermal relief connectivity, or electrical continuity.
