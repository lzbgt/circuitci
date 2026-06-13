# JLC/EasyEDA Gerber Outline Import Notes

## Sources

- Official Gerber layer-format specification saved locally:
  `docs/research/gerber/ucamco_gerber_layer_format_specification_2024_05.pdf`
  from
  <https://www.ucamco.com/files/downloads/file_en/456/gerber-layer-format-specification-revision-2024-05_en.pdf>
- Peer fabricated release:
  `../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip`
- Observed board-outline member:
  `Gerber_BoardOutlineLayer.GKO`

## Observed Peer Header

The peer outline layer declares:

```text
G04 Layer: BoardOutlineLayer*
G04 EasyEDA Pro v3.2.117, 2026-04-28 20:08:16*
G04 Dimensions in millimeters*
G04 Leading zeros omitted, absolute positions, 4 integers and 5 decimals*
%FSLAX45Y45*%
%MOMM*%
```

The file contains only linear `G01` centerline outline draws using `D02` moves
and `D01` draws. The observed geometry is one external 120 mm x 142 mm board
rectangle plus three internal rectangular slots/cutouts.

## CircuitCI Import Contract

CircuitCI imports this subset into `board.layout.outline.segments` with
`source_primitive: gerber_linear`. It preserves the observed Gerber layer name
as `layer: BoardOutlineLayer` and classifies closed contours as `external` or
`cutout`.

Unsupported Gerber constructs fail closed instead of being approximated:
inch units, incremental coordinates, arcs, flashes, and missing format/unit
declarations.
